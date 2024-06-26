use std::{borrow::Cow, error::Error, fs::read_to_string, path::PathBuf, str::FromStr};

use anstyle::AnsiColor;
use anyhow::{bail, Context};
pub use clap::{self, error::ErrorKind, CommandFactory, Parser};
use clap::{
    builder::PossibleValuesParser, builder::Styles, builder::TypedValueParser, Args, ValueEnum,
};

use figment::{
    providers::{Format, Serialized, Toml},
    value::{Dict, Map, Value},
    Figment, Metadata, Profile, Provider,
};
use log::{debug, LevelFilter};
use serde::{Deserialize, Serialize};

use crate::formatting_orchestrator::FormatterConfiguration;

const DEFAULT_CONFIG_FILE_NAME: &str = "pasfmt.toml";

#[macro_export]
macro_rules! pasfmt_config {
    ($(#[$attr: meta])* $type_name: ident) => {
        #[derive(clap::Parser, Debug)]
        #[command(author, about, version, long_about = None)]
        $(#[$attr])*
        struct $type_name {
            #[command(flatten)]
            config: PasFmtConfiguration,
        }
        impl $type_name {
            #[allow(unused)]
            pub fn validate(self) -> Result<PasFmtConfiguration, clap::Error> {
                if matches!(self.config.mode(), FormatMode::Files) && self.config.is_stdin() {
                    return Err(Self::command().error(
                        ErrorKind::ArgumentConflict,
                        "Files mode not supported when reading from stdin.",
                    ));
                }

                Ok(self.config)
            }

            #[allow(unused)]
            pub fn try_create() -> Result<PasFmtConfiguration, clap::Error> {
                Self::try_parse()?.validate()
            }

            #[allow(unused)]
            pub fn create() -> PasFmtConfiguration {
                match Self::try_create() {
                    Ok(config) => config,
                    Err(err) => err.exit(),
                }
            }
        }
    };
}
pub use pasfmt_config;

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

fn parse_key_val<K, V>(s: &str) -> Result<(K, V), Box<dyn Error + Send + Sync + 'static>>
where
    K: std::str::FromStr,
    K::Err: Error + Send + Sync + 'static,
    V: std::str::FromStr,
    V::Err: Error + Send + Sync + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{s}`"))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}

#[derive(Args, Debug)]
#[command(styles = get_styles())]
pub struct PasFmtConfiguration {
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
    #[arg(short = 'C', value_parser = parse_key_val::<String, String>, value_name = "KEY=VALUE")]
    config: Vec<(String, String)>,

    /// The mode of operation
    ///
    /// The default is `files`, unless data is being read from stdin, in which
    /// case the default is `stdout`.
    #[arg(short, long, value_enum)]
    mode: Option<FormatMode>,

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

// By default, some providers save the source code location as their 'source'.
// This struct and corresponding trait provide a way to erase that while also
// providing a new name for the metadata.
struct ErasedLocation<P: Provider> {
    name: &'static str,
    delegate: P,
}

trait EraseLocation<P: Provider> {
    fn erase_with_name(self, name: &'static str) -> ErasedLocation<P>;
}

impl<P: Provider> Provider for ErasedLocation<P> {
    fn metadata(&self) -> Metadata {
        Metadata::named(self.name)
    }

    fn data(&self) -> Result<Map<Profile, Dict>, figment::Error> {
        self.delegate.data()
    }
}

impl<P: Provider> EraseLocation<P> for P {
    fn erase_with_name(self, name: &'static str) -> ErasedLocation<P> {
        ErasedLocation {
            delegate: self,
            name,
        }
    }
}

const COMMAND_LINE_OVERRIDES_METADATA_NAME: &str = "command-line overrides";

struct UnknownKeyErr<'a> {
    key: &'a str,
}

impl<'a> Provider for UnknownKeyErr<'a> {
    fn metadata(&self) -> Metadata {
        Metadata::named(COMMAND_LINE_OVERRIDES_METADATA_NAME)
    }

    fn data(&self) -> Result<Map<Profile, Dict>, figment::Error> {
        Err(figment::Error::from(format!(
            "Unknown configuration key {:?}",
            self.key
        )))
    }
}

impl PasFmtConfiguration {
    fn apply_overrides(&self, config: Figment) -> Figment {
        self.config.iter().fold(config, |config, (key, val)| {
            if config.find_metadata(key).is_none() {
                config.merge(UnknownKeyErr { key })
            } else {
                let value: Result<_, std::convert::Infallible> = Value::from_str(val);
                let value = value.unwrap(); // unwrapping is fine because the Err is Infallible
                config.merge(
                    Serialized::default(key, value)
                        .erase_with_name(COMMAND_LINE_OVERRIDES_METADATA_NAME),
                )
            }
        })
    }

    pub fn get_config_object<T>(&self) -> anyhow::Result<T>
    where
        T: for<'de> Deserialize<'de> + Serialize + Default,
    {
        // A relative path will be searched for in the current directory and all parent directories.
        // We want the user-provided path to only be resolved against the working directory, so we
        // canonicalize it. We do want the default config path to be searched for in parent
        // directories, so that path is left relative.
        let config_file = match &self.config_file {
            Some(file) => {
                if file.is_dir() {
                    bail!(
                        "Config file path cannot be a directory: '{}'",
                        file.display()
                    );
                }
                file.canonicalize().with_context(|| {
                    format!("Failed to resolve config file path: '{}'", file.display())
                })?
            }
            None => PathBuf::from(DEFAULT_CONFIG_FILE_NAME),
        };
        debug!("Using config file: {}", config_file.display());

        let default_provider = Serialized::defaults(T::default()).erase_with_name("<default>");
        let mut config = Figment::from(&default_provider).merge(Toml::file(config_file));

        config = self.apply_overrides(config);

        let obj = config
            .extract::<T>()
            .context("Failed to construct configuration")?;

        debug!(
            "Configuration:\n{}",
            toml::to_string_pretty(&obj)
                .unwrap_or("Failed to serialize config object.".to_string())
        );

        Ok(obj)
    }
}

impl FormatterConfiguration for PasFmtConfiguration {
    fn get_paths(&self) -> anyhow::Result<Cow<[String]>> {
        let mut paths = Cow::Borrowed(&self.paths[..]);
        if let Some(arg_file) = &self.files_from {
            paths.to_mut().extend(
                read_to_string(arg_file)
                    .with_context(|| {
                        format!("Failed to read `--files-from` path: {}", arg_file.display())
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::{prelude::*, TempDir};
    use indoc::indoc;
    use serde_derive::{Deserialize, Serialize};
    use spectral::prelude::*;

    pasfmt_config!(Config);

    fn config(args: &[&str]) -> Result<PasFmtConfiguration, clap::Error> {
        Config::try_parse_from(args)?.validate()
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
            .starts_with("error: Files mode not supported when reading from stdin.");

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
                .starts_with("Failed to read `--files-from` path:");
        }

        Ok(())
    }

    mod cfg {
        use super::*;

        #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
        enum SettingEnum {
            A,
            B,
        }

        #[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq)]
        struct Nested {
            bar: i32,
            baz: Option<SettingEnum>,
        }

        #[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq)]
        struct Settings {
            foo: String,
            bar: i32,
            baz: Option<SettingEnum>,
            nested: Nested,
        }

        #[test]
        fn config_file_cannot_be_directory() -> Result<(), Box<dyn Error>> {
            let config: anyhow::Result<Settings> =
                config(&["", "--config-file=."])?.get_config_object();
            assert_that(&config).is_err();
            assert_eq!(
                format!("{:?}", config.unwrap_err()),
                "Config file path cannot be a directory: '.'"
            );

            Ok(())
        }

        #[test]
        fn config_file_must_exist() -> Result<(), Box<dyn Error>> {
            let tmp = TempDir::new()?;
            let config: anyhow::Result<Settings> = config(&[
                "",
                "--config-file",
                &tmp.join("does_not_exit").to_string_lossy(),
            ])?
            .get_config_object();

            assert_that(&config).is_err();

            // The whole message is a bit hard to assert on, because it contains an OS error, which could vary.
            assert_that(&format!("{:?}", config.unwrap_err()))
                .starts_with("Failed to resolve config file path:");

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
            let obj: Settings = config.get_config_object()?;

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
        fn unknown_config_key_throws_error() -> Result<(), Box<dyn Error>> {
            let obj: anyhow::Result<Settings, _> = config(&["", "-Casdf=0"])?.get_config_object();
            assert_that(&obj).is_err();
            assert_eq!(
                format!("{:?}", obj.unwrap_err()),
                indoc!(
                    r#"
                    Failed to construct configuration

                    Caused by:
                        Unknown configuration key "asdf" in command-line overrides"#
                )
            );

            Ok(())
        }

        #[test]
        fn overrides_with_escaped_values() -> Result<(), Box<dyn Error>> {
            let cases = [
                (
                    "quoted backslash sequences should be unescaped",
                    r#""\n""#,
                    "\n",
                ),
                (
                    "unquoted backslash sequences should not be unescaped",
                    r#"\n"#,
                    "\\n",
                ),
                (
                    "quoted backslash sequences should be unescaped",
                    r#""\\""#,
                    "\\",
                ),
                (
                    "unquoted backslash sequences should not be unescaped",
                    r#"\\"#,
                    "\\\\",
                ),
                ("quoted spaces should be preserved", r#"" ""#, " "),
                ("unquoted spaces should be trimmed", " ", ""),
            ];

            for (desc, escaped, unescaped) in cases {
                let obj: Settings =
                    config(&["", &format!("-Cfoo={escaped}")])?.get_config_object()?;
                assert_eq!(obj.foo, unescaped, "{}", desc);
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
    }
}
