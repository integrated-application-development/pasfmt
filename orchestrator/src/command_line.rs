use std::{
    borrow::{Borrow, Cow},
    error::Error,
    fmt::Display,
    fs::read_to_string,
    path::{Path, PathBuf},
    str::FromStr,
};

use anstyle::AnsiColor;
use anyhow::Context;
pub use clap::{self, error::ErrorKind, CommandFactory, Parser};
use clap::{
    builder::{PossibleValuesParser, StyledStr, Styles, TypedValueParser},
    Args, ValueEnum,
};

use config::{Config, File, FileFormat};
use log::{debug, LevelFilter};

use crate::formatting_orchestrator::FormatterConfiguration;

const DEFAULT_CONFIG_FILE_NAME: &str = "pasfmt.toml";

#[derive(Debug)]
pub enum CliError {
    Clap(clap::Error),
    ConfigHelp,
}

impl Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Clap(error) => error.fmt(f),
            CliError::ConfigHelp => f.write_str("ConfigHelp"),
        }
    }
}

impl Error for CliError {}

pub trait CliWrapper: clap::Parser {
    type Config: Configuration;
    fn inner(&self) -> &PasFmtConfiguration<Self::Config>;
    fn into_inner(self) -> PasFmtConfiguration<Self::Config>;
}

pub trait CliWrapperImpl: CliWrapper {
    fn validate(self) -> Result<PasFmtConfiguration<Self::Config>, CliError>;
    fn try_create() -> Result<PasFmtConfiguration<Self::Config>, CliError>;
    fn create() -> PasFmtConfiguration<Self::Config>;
}

#[macro_export]
macro_rules! pasfmt_config {
    ($(#[$attr: meta])* $type_name:ident <$config_type_name:ident>) => {
        #[derive($crate::command_line::clap::Parser, Debug)]
        #[command(author, about, version, long_about = None)]
        #[clap(max_term_width = 120)]
        $(#[$attr])*
        struct $type_name {
            #[command(flatten)]
            config: $crate::command_line::PasFmtConfiguration<$config_type_name>,
        }

        impl $crate::command_line::CliWrapper for $type_name {
            type Config = $config_type_name;

            #[inline]
            fn inner(&self) -> &$crate::command_line::PasFmtConfiguration<Self::Config> {
                &self.config
            }

            #[inline]
            fn into_inner(self) -> $crate::command_line::PasFmtConfiguration<Self::Config> {
                self.config
            }
        }
    };
}

pub use pasfmt_config;

impl<T: CliWrapper> CliWrapperImpl for T {
    fn validate(self) -> Result<PasFmtConfiguration<Self::Config>, CliError> {
        if self.inner().config_help_requested() {
            return Err(CliError::ConfigHelp);
        }

        if matches!(self.inner().mode(), FormatMode::Files) && self.inner().is_stdin() {
            return Err(CliError::Clap(Self::command().error(
                ErrorKind::ArgumentConflict,
                "files mode not supported when reading from stdin",
            )));
        }

        Ok(self.into_inner())
    }

    fn try_create() -> Result<PasFmtConfiguration<Self::Config>, CliError> {
        Self::try_parse().map_err(CliError::Clap)?.validate()
    }

    fn create() -> PasFmtConfiguration<Self::Config> {
        match Self::try_create() {
            Ok(config) => config,
            Err(CliError::Clap(err)) => err.exit(),
            Err(CliError::ConfigHelp) => {
                // This method seems a bit convoluted, but it makes sure that the output
                // stream and exit code used is consistent with the inbuilt --help option.
                // It would be simpler if we could create an error via Command::error
                // that looks just like the one used for --help, but the required functions
                // are not public.
                let _ = Self::command()
                    .override_help(PasFmtConfiguration::<Self::Config>::config_help())
                    .print_help();
                std::process::exit(
                    Self::command()
                        .error(ErrorKind::DisplayHelp, "")
                        .exit_code(),
                );
            }
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum FormatMode {
    /// format files in-place
    Files,
    /// print formatted files to stdout
    Stdout,
    /// exit zero if input is formatted correctly, otherwise exit non-zero and
    /// list the erroneous files
    Check,
}

fn parse_override(s: &str) -> Result<ConfigOverride, Box<dyn Error + Send + Sync + 'static>> {
    if s.eq_ignore_ascii_case("help") {
        return Ok(ConfigOverride::Help);
    }

    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{s}`"))?;
    Ok(ConfigOverride::Set {
        key: s[..pos].parse()?,
        val: s[pos + 1..].parse()?,
    })
}

#[derive(Debug, Clone)]
enum ConfigOverride {
    Help,
    Set { key: String, val: String },
}

#[derive(Args, Debug)]
#[command(styles = get_styles())]
pub struct PasFmtConfiguration<C: Configuration> {
    #[clap(skip)]
    marker: std::marker::PhantomData<C>,

    /// Paths that will be formatted. Can be a path/dir/glob. If no paths are
    /// specified, stdin is read.
    #[arg(index = 1, num_args = 0..)]
    paths: Vec<String>,

    /// A file containing paths to operate on. Newline separated list of
    /// path/dir/glob.
    #[arg(short, long)]
    files_from: Option<PathBuf>,

    /// Override the configuration file. By default working directory will be
    /// traversed until a `pasfmt.toml` file is found.
    #[arg(long)]
    config_file: Option<PathBuf>,

    /// Override one configuration option using KEY=VALUE. This takes
    /// precedence over `--config-file`.
    ///
    /// To list available options, use `-C help`.
    #[arg(short = 'C', value_parser = parse_override, value_name = "KEY=VALUE")]
    overrides: Vec<ConfigOverride>,

    /// The mode of operation
    ///
    /// The default is `files`, unless data is being read from stdin, in which
    /// case the default is `stdout`.
    ///
    /// When the mode is `stdout` and data is being read from files, all output
    /// is concatenated in a human-readable format.
    #[arg(short, long, value_enum)]
    mode: Option<FormatMode>,

    /// Print (to stderr) where cursors at the given UTF-8 byte positions move
    /// to after formatting
    ///
    /// Cursors provided can be comma-separated. After formatting, a single
    /// line of the form `CURSOR=<LIST>` will be printed to stderr, where
    /// `<LIST>` is a comma-separated list of the new UTF-8 byte positions for
    /// the input positions. The order of this list matches the provided order
    /// of the cursors.
    #[arg(long, value_delimiter = ',', num_args = 1..)]
    cursor: Vec<u32>,

    /// Increase logging verbosity (can be repeated).
    #[arg(short, long, action = clap::ArgAction::Count, conflicts_with = "log_level")]
    verbose: u8,

    /// Only show log messages at least this severe.
    #[arg(short, long, default_value_t = LevelFilter::Warn, value_parser = log_level_parser())]
    log_level: LevelFilter,
}

fn log_level_parser() -> impl TypedValueParser {
    PossibleValuesParser::new(valid_log_levels())
        .map(|level| LevelFilter::from_str(&level).unwrap())
}

fn valid_log_levels() -> Vec<&'static str> {
    LevelFilter::iter()
        .map(|level| level.as_str())
        .collect::<Vec<_>>()
}

// LevelFilter doesn't expose enough to make this possible without duplicating the integer values a bit.
fn log_level_from_usize(u: usize) -> Option<LevelFilter> {
    match u {
        0 => Some(LevelFilter::Off),
        1 => Some(LevelFilter::Error),
        2 => Some(LevelFilter::Warn),
        3 => Some(LevelFilter::Info),
        4 => Some(LevelFilter::Debug),
        5 => Some(LevelFilter::Trace),
        _ => None,
    }
}

fn get_styles() -> Styles {
    Styles::styled()
        .usage(AnsiColor::White.on_default().bold().underline())
        .header(AnsiColor::White.on_default().bold().underline())
        .literal(AnsiColor::BrightCyan.on_default())
        .invalid(AnsiColor::Red.on_default().bold())
        .error(AnsiColor::Red.on_default().bold())
        .valid(AnsiColor::Green.on_default().bold())
        .placeholder(AnsiColor::White.on_default())
}

pub struct ConfigItem {
    pub name: &'static str,
    pub description: &'static str,
    pub hint: &'static str,
    pub default: String,
}

pub trait Configuration: for<'de> ::serde::Deserialize<'de> {
    fn docs() -> impl IntoIterator<Item = ConfigItem>;
}

impl<C: Configuration> PasFmtConfiguration<C> {
    fn find_config_file(search_dir: PathBuf) -> Option<PathBuf> {
        let mut path = search_dir;
        loop {
            path.push(DEFAULT_CONFIG_FILE_NAME);

            if path.is_file() {
                return Some(path);
            }

            if !(path.pop() && path.pop()) {
                debug!(
                    "{} file not found in any parent directory",
                    DEFAULT_CONFIG_FILE_NAME
                );
                return None;
            }
        }
    }

    fn config_help_requested(&self) -> bool {
        self.overrides
            .iter()
            .any(|o| matches!(o, ConfigOverride::Help))
    }

    fn config_help() -> StyledStr {
        use std::fmt::Write;

        let cyan = AnsiColor::Cyan.on_default();
        let yellow = AnsiColor::Yellow.on_default();
        let reset = anstyle::Reset;
        let italic = anstyle::Style::new().italic();

        let mut out = String::new();
        out.push_str("Available configuration options:\n\n");
        for item in C::docs() {
            writeln!(
                out,
                "{cyan}{}{reset} {italic}{}{reset} (default: {yellow}{}{reset})",
                item.name, item.hint, item.default
            )
            .unwrap();

            for line in item.description.lines() {
                writeln!(out, "  {line}").unwrap();
            }

            writeln!(out).unwrap();
        }

        out.into()
    }

    fn get_config_object_from_file(&self, config_file: Option<Cow<Path>>) -> anyhow::Result<C> {
        let mut builder = Config::builder();

        if let Some(f) = config_file {
            debug!("Using config file: {}", f.display());
            builder = builder.add_source(File::from(f.borrow()).format(FileFormat::Toml));
        }

        for item in &self.overrides {
            match item {
                ConfigOverride::Set { key, val } => {
                    builder = builder.set_override(key, val.as_ref())?;
                }
                ConfigOverride::Help => {
                    // Do nothing; this is handled in CliWrapperImpl::validate
                }
            }
        }

        builder
            .build()?
            .try_deserialize::<C>()
            .context("failed to construct configuration")
    }

    pub fn get_config_object(&self) -> anyhow::Result<C> {
        let config_file = match &self.config_file {
            Some(file) => Some(Cow::Borrowed(file.as_path())),
            None => Self::find_config_file(std::env::current_dir()?).map(Cow::Owned),
        };
        self.get_config_object_from_file(config_file)
    }
}

impl<C: Configuration> FormatterConfiguration for PasFmtConfiguration<C> {
    fn get_paths(&self) -> anyhow::Result<Cow<[String]>> {
        let mut paths = Cow::Borrowed(&self.paths[..]);
        if let Some(arg_file) = &self.files_from {
            paths.to_mut().extend(
                read_to_string(arg_file)
                    .with_context(|| {
                        format!("failed to read `--files-from` path: {}", arg_file.display())
                    })?
                    .lines()
                    .map(String::from)
                    .collect::<Vec<_>>(),
            );
        }
        Ok(paths)
    }
    fn log_level(&self) -> LevelFilter {
        log_level_from_usize((self.verbose as usize) + (self.log_level as usize))
            .unwrap_or(LevelFilter::max())
    }
    fn mode(&self) -> FormatMode {
        self.mode.unwrap_or_else(|| {
            if self.is_stdin() {
                FormatMode::Stdout
            } else {
                FormatMode::Files
            }
        })
    }

    fn is_stdin(&self) -> bool {
        self.paths.is_empty() && self.files_from.is_none()
    }

    fn cursors(&self) -> &[u32] {
        &self.cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::{prelude::*, TempDir};
    use serde::Deserialize;
    use spectral::prelude::*;

    pasfmt_config!(Config<Settings>);

    #[derive(Deserialize, PartialEq, Eq, Debug)]
    enum SettingEnum {
        A,
        B,
    }

    #[derive(Deserialize, PartialEq, Eq, Default, Debug)]
    #[serde(deny_unknown_fields)]
    struct Nested {
        #[serde(default)]
        bar: i32,
        #[serde(default)]
        baz: Option<SettingEnum>,
    }

    #[derive(Deserialize, Debug, PartialEq, Eq, Default)]
    #[serde(deny_unknown_fields)]
    struct Settings {
        #[serde(default)]
        foo: String,
        #[serde(default)]
        bar: i32,
        #[serde(default)]
        baz: Option<SettingEnum>,
        #[serde(default)]
        nested: Nested,
    }

    impl Configuration for Settings {
        fn docs() -> impl IntoIterator<Item = ConfigItem> {
            []
        }
    }

    fn config(args: &[&str]) -> Result<PasFmtConfiguration<Settings>, CliError> {
        Config::try_parse_from(args)
            .map_err(CliError::Clap)?
            .validate()
    }

    #[test]
    fn log_level() -> Result<(), Box<dyn Error>> {
        assert_eq!(config(&[""])?.log_level(), LevelFilter::Warn);
        assert_eq!(config(&["", "-v"])?.log_level(), LevelFilter::Info);
        assert_eq!(config(&["", "-v", "-v"])?.log_level(), LevelFilter::Debug);
        assert_eq!(config(&["", "-vv"])?.log_level(), LevelFilter::Debug);
        assert_eq!(config(&["", "-vv", "-v"])?.log_level(), LevelFilter::Trace);
        assert_eq!(config(&["", "-vvvvvvvv"])?.log_level(), LevelFilter::Trace);

        for level in [
            LevelFilter::Off,
            LevelFilter::Error,
            LevelFilter::Warn,
            LevelFilter::Info,
            LevelFilter::Debug,
            LevelFilter::Trace,
        ] {
            assert_eq!(
                config(&["", "--log-level", level.as_str()])?.log_level(),
                level
            );
        }

        Ok(())
    }

    #[test]
    fn log_level_conflicts_with_verbose() -> Result<(), Box<dyn Error>> {
        let err = config(&["", "--log-level=ERROR", "-v"]).unwrap_err();
        assert_that(&err.to_string()).contains(
            "error: the argument '--log-level <LOG_LEVEL>' cannot be used with '--verbose...'",
        );

        Ok(())
    }

    #[test]
    fn is_stdin() -> Result<(), Box<dyn Error>> {
        assert!(config(&[""])?.is_stdin());
        assert!(!config(&["", "foo.pas"])?.is_stdin());
        assert!(!config(&["", "--files-from=asdf"])?.is_stdin());

        Ok(())
    }

    #[test]
    fn mode() -> Result<(), Box<dyn Error>> {
        assert_eq!(config(&[""])?.mode(), FormatMode::Stdout);
        assert_eq!(config(&["", "--mode=check"])?.mode(), FormatMode::Check);

        assert_eq!(config(&["", "foo.pas"])?.mode(), FormatMode::Files);
        assert_eq!(
            config(&["", "foo.pas", "--mode=stdout"])?.mode(),
            FormatMode::Stdout
        );
        assert_eq!(
            config(&["", "foo.pas", "--mode=check"])?.mode(),
            FormatMode::Check
        );

        assert_eq!(
            config(&["", "--files-from=asdf"])?.mode(),
            FormatMode::Files
        );
        assert_eq!(
            config(&["", "--files-from=asdf", "--mode=stdout"])?.mode(),
            FormatMode::Stdout
        );
        assert_eq!(
            config(&["", "--files-from=asdf", "--mode=check"])?.mode(),
            FormatMode::Check
        );

        Ok(())
    }

    #[test]
    fn incompatible_file_mode_and_stdin() -> Result<(), Box<dyn Error>> {
        let config = config(&["", "--mode=files"]);
        assert_that(&config).is_err();
        // Unfortunately cannot be chained in one assertion because spectral insists on asserting
        // only on references and never on values, and mapping with `to_string()` would produce
        // a temporary value we cannot return a reference to.
        assert_that(&config.unwrap_err().to_string())
            .starts_with("error: files mode not supported when reading from stdin");

        Ok(())
    }

    #[test]
    fn files_from() -> Result<(), Box<dyn Error>> {
        let tmp = TempDir::new()?;
        let files_from = tmp.child("files_from.txt");
        let files_from_path = files_from.to_string_lossy().into_owned();
        files_from.write_str("a\nb/c\nd").unwrap();

        {
            let config = config(&["", "--files-from", &files_from_path])?;
            let get_paths = config.get_paths().unwrap();
            assert_eq!(get_paths.as_ref(), &["a", "b/c", "d"]);
        }

        {
            let config = config(&["", "outside_file", "--files-from", &files_from_path])?;
            let get_paths = config.get_paths().unwrap();
            assert_eq!(get_paths.as_ref(), &["outside_file", "a", "b/c", "d"]);
        }

        drop(tmp);
        {
            let config = config(&["", "--files-from", &files_from_path])?;
            let result = config.get_paths();
            assert_that(&result).is_err();
            assert_that(&result.unwrap_err().to_string())
                .starts_with("failed to read `--files-from` path:");
        }

        Ok(())
    }

    mod cfg {
        use super::*;
        use indoc::indoc;

        #[test]
        fn config_file_cannot_be_directory() -> Result<(), Box<dyn Error>> {
            let config: anyhow::Result<Settings> =
                config(&["", "--config-file=."])?.get_config_object();
            assert_that(&config).is_err();
            assert_eq!(
                format!("{:#}", config.unwrap_err()),
                "configuration file \".\" not found"
            );

            Ok(())
        }

        #[test]
        fn config_file_must_exist() -> Result<(), Box<dyn Error>> {
            let tmp = TempDir::new()?;
            let config_path = tmp.join("does_not_exit");
            let config: anyhow::Result<Settings> =
                config(&["", "--config-file", &config_path.to_string_lossy()])?.get_config_object();

            assert_that(&config).is_err();

            // The whole message is a bit hard to assert on, because it contains an OS error, which could vary.
            let msg = format!("configuration file \"{}\" not found", config_path.display());
            assert_that(&format!("{:?}", config.unwrap_err())).starts_with(&*msg);

            Ok(())
        }

        #[test]
        fn config_obj_from_file() -> Result<(), Box<dyn Error>> {
            let tmp = TempDir::new()?;
            let config_file = &tmp.child("custom_settings.toml");

            let cases = [
                (
                    r#"
                    foo = "foo!"
                    bar = -1
                    [nested]
                    bar = 1
                    "#,
                    Settings {
                        foo: "foo!".to_owned(),
                        bar: -1,
                        baz: None,
                        nested: Nested { bar: 1, baz: None },
                    },
                ),
                (
                    r#"
                    [nested]
                    baz = "A"
                    "#,
                    Settings {
                        foo: "".to_owned(),
                        bar: 0,
                        baz: None,
                        nested: Nested {
                            bar: 0,
                            baz: Some(SettingEnum::A),
                        },
                    },
                ),
            ];

            for (contents, expected) in &cases {
                config_file.write_str(contents)?;

                let config = config(&["", "--config-file", &config_file.to_string_lossy()])?;
                let obj: Settings = config.get_config_object()?;

                assert_eq!(&obj, expected);
            }

            Ok(())
        }

        #[test]
        fn config_obj_from_overrides() -> Result<(), Box<dyn Error>> {
            let config = config(&["", "-C", "foo=bar", "-Cbar=-1", "-Cnested.baz=B"])?;
            let obj: Settings = config.get_config_object_from_file(None)?;

            assert_eq!(
                obj,
                Settings {
                    foo: "bar".to_string(),
                    bar: -1,
                    baz: None,
                    nested: Nested {
                        bar: 0,
                        baz: Some(SettingEnum::B)
                    }
                }
            );

            Ok(())
        }

        #[test]
        fn config_override_syntax_errors() -> Result<(), Box<dyn Error>> {
            for (arg, msg) in [
                (
                    "a..=",
                    indoc! {"
                        a..
                          ^
                        invalid identifier
                        expected ASCII alphanumeric, `_`, `-`\
                    "},
                ),
                (
                    ".=",
                    indoc! {"
                        .
                        ^
                        invalid identifier
                        expected ASCII alphanumeric, `_`, `-`\
                    "},
                ),
                (
                    "!=",
                    indoc! {"
                        !
                        ^
                        invalid identifier
                        expected ASCII alphanumeric, `_`, `-`\
                    "},
                ),
                (
                    "a[=",
                    indoc! { "
                        a[
                          ^
                        invalid subscript
                        expected integer\
                    "},
                ),
            ] {
                let config = config(&["", "-C", arg])?;
                let err = config.get_config_object_from_file(None).unwrap_err();

                assert_eq!(
                    format!("{}", err),
                    msg,
                    "parsing {arg} gave the wrong error message"
                );
            }

            Ok(())
        }

        #[test]
        fn config_overrides_have_greater_precedence_than_file() -> Result<(), Box<dyn Error>> {
            let tmp = TempDir::new()?;
            let config_file = &tmp.child("custom_settings.toml");

            config_file.write_str("bar = 1000")?;

            let args = &["", "--config-file", &config_file.to_string_lossy()];
            let obj: Settings = config(args)?.get_config_object()?;
            assert_eq!(obj.bar, 1000);

            let obj: Settings =
                config(&[&args[..], &["-Cbar=-1"]].concat())?.get_config_object()?;
            assert_eq!(obj.bar, -1);

            Ok(())
        }

        #[test]
        fn overrides_with_escaped_values_not_supported() -> Result<(), Box<dyn Error>> {
            let cases = [
                ("backslash sequences should not be unescaped", r#"\n\\"#),
                ("quotes should not be trimmed", r#""a""#),
                ("spaces should not be trimmed", " "),
            ];

            for (desc, val) in cases {
                let obj: Settings =
                    config(&["", &format!("-Cfoo={val}")])?.get_config_object_from_file(None)?;
                assert_eq!(obj.foo, val, "{}", desc);
            }

            Ok(())
        }

        #[test]
        fn key_val_missing_equals() -> Result<(), Box<dyn Error>> {
            let obj = config(&["", "-Casdf"]);
            assert_that(&obj).is_err();
            assert_eq!(
                format!("{}", obj.unwrap_err()),
                "error: invalid value 'asdf' for '-C <KEY=VALUE>': \
                invalid KEY=value: no `=` found in `asdf`\n\nFor more information, try '--help'.\n"
            );

            Ok(())
        }

        #[test]
        fn config_help() {
            for args in [
                &["", "-Chelp"][..],
                &["", "-C", "help"],
                &["", "-Cfoo=a", "-Cbar=0", "-Chelp"],
            ] {
                let cfg = config(args);
                assert!(matches!(cfg.unwrap_err(), CliError::ConfigHelp));
            }
        }
    }
}
