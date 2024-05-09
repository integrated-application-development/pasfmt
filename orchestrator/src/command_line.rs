use std::{error::Error, fs::read_to_string, path::PathBuf, str::FromStr};

use anstyle::AnsiColor;
use anyhow::Context;
pub use clap::{self, error::ErrorKind, CommandFactory, Parser};
use clap::{
    builder::PossibleValuesParser, builder::Styles, builder::TypedValueParser, Args, ValueEnum,
};

use figment::{
    providers::{Format, Serialized, Toml},
    value::{Dict, Map},
    Figment, Metadata, Profile, Provider,
};
use log::{debug, warn, LevelFilter};
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
            pub fn create() -> PasFmtConfiguration {
                let parsed = Self::parse();
                let mut cmd = Self::command();
                if matches!(parsed.config.mode(), FormatMode::Files) && parsed.config.is_stdin() {
                    cmd.error(
                        ErrorKind::ArgumentConflict,
                        "Files mode not supported when reading from stdin.",
                    )
                    .exit();
                }

                parsed.config
            }
        }
    };
}
pub use pasfmt_config;

#[derive(Debug, Clone, Copy, ValueEnum)]
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

impl Default for PasFmtConfiguration {
    fn default() -> Self {
        Self {
            paths: Default::default(),
            files_from: Default::default(),
            config_file: Default::default(),
            verbose: Default::default(),
            config: Default::default(),
            log_level: LevelFilter::Warn,
            mode: None,
        }
    }
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

impl PasFmtConfiguration {
    pub fn get_config_object<T>(&self) -> anyhow::Result<T>
    where
        T: for<'de> Deserialize<'de> + Serialize + Default,
    {
        // A relative path will be searched for in the current directory and all parent directories.
        // We want the user-provided path to only be resolved against the working directory, so we
        // canonicalize it. We do want the default config path to be searched for in parent
        // directories, so that path is left relative.
        let config_file = match &self.config_file {
            Some(file) => file.canonicalize().with_context(|| {
                format!("Failed to resolve config file path: '{}'", file.display())
            })?,
            None => PathBuf::from(DEFAULT_CONFIG_FILE_NAME),
        };
        debug!("Using config file: {}", config_file.display());

        let default_provider = Serialized::defaults(T::default()).erase_with_name("<default>");
        let mut config = Figment::from(&default_provider).merge(Toml::file(config_file));

        // apply overrides
        config = self.config.iter().fold(config, |config, (key, val)| {
            if config.find_metadata(key).is_none() {
                warn!("Ingoring unknown configuration key '{key}'");
                config
            } else {
                config
                    .merge(Serialized::default(key, val).erase_with_name("command-line overrides"))
            }
        });

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
    fn get_paths(&self) -> Vec<String> {
        let mut paths = self.paths.clone();
        if let Some(arg_file) = &self.files_from {
            paths.extend(
                read_to_string(arg_file)
                    .unwrap()
                    .lines()
                    .map(String::from)
                    .collect::<Vec<_>>(),
            );
        }
        paths
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
