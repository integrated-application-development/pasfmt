#![deny(clippy::enum_glob_use)]

use encoding_rs::Encoding;
use pasfmt_core::prelude::*;
use pasfmt_orchestrator::predule::*;
use serde::Deserialize;

#[cfg(windows)]
fn get_windows_default_encoding() -> &'static Encoding {
    // SAFETY: yes it's a foreign function, but it's a simple one from the WinAPI that we
    // can assume to be safe.
    let ansi_codepage = unsafe { windows_sys::Win32::Globalization::GetACP() };
    let encoding = ansi_codepage
        .try_into()
        .ok()
        .and_then(|cp: u16| codepage::to_encoding(cp))
        .unwrap_or_else(|| {
            log::warn!(
                "failed to convert system codepage {} to encoding, defaulting to UTF-8",
                ansi_codepage
            );
            encoding_rs::UTF_8
        });
    log::debug!("encoding from system ANSI codepage: {}", encoding.name());
    encoding
}

fn default_encoding() -> &'static Encoding {
    #[cfg(windows)]
    {
        use std::sync::LazyLock;

        static WINDOWS_DEFAULT_ENCODING: LazyLock<&'static Encoding> =
            LazyLock::new(get_windows_default_encoding);

        *WINDOWS_DEFAULT_ENCODING
    }
    #[cfg(not(windows))]
    encoding_rs::UTF_8
}

#[cfg_attr(feature = "__demo", derive(serde::Serialize))]
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct FormattingSettings {
    #[serde(default = "Reconstruction::default")]
    reconstruction: Reconstruction,
    #[serde(default = "Olf::default")]
    olf: Olf,
    #[serde(default = "default_encoding")]
    encoding: &'static Encoding,
}

impl FormattingSettings {
    #[cfg(feature = "__demo")]
    pub fn max_line_length(&self) -> u32 {
        self.olf.max_line_length
    }
}

impl Default for FormattingSettings {
    fn default() -> Self {
        Self {
            reconstruction: Default::default(),
            encoding: default_encoding(),
            olf: Olf::default(),
        }
    }
}

impl TryFrom<Reconstruction> for ReconstructionSettings {
    type Error = InvalidReconstructionSettingsError;

    fn try_from(val: Reconstruction) -> Result<Self, Self::Error> {
        ReconstructionSettings::new(
            val.eol,
            val.indentation.clone(),
            val.continuation.unwrap_or(val.indentation),
        )
    }
}

#[cfg_attr(feature = "__demo", derive(serde::Serialize))]
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
struct Reconstruction {
    #[serde(default = "default_eol")]
    eol: String,
    #[serde(default = "default_indentation")]
    indentation: String,
    #[serde(default = "default_continuation")]
    continuation: Option<String>,
}

fn default_eol() -> String {
    "\r\n".to_owned()
}

fn default_indentation() -> String {
    "  ".to_owned()
}
fn default_continuation() -> Option<String> {
    Some("    ".to_owned())
}

impl Default for Reconstruction {
    fn default() -> Self {
        Reconstruction {
            eol: default_eol(),
            indentation: default_indentation(),
            continuation: default_continuation(),
        }
    }
}

#[cfg_attr(feature = "__demo", derive(serde::Serialize))]
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
struct Olf {
    #[serde(default = "default_max_line_length")]
    max_line_length: u32,
}
fn default_max_line_length() -> u32 {
    120
}

impl From<Olf> for OptimisingLineFormatterSettings {
    fn from(value: Olf) -> Self {
        Self {
            max_line_length: value.max_line_length,
            iteration_max: 20_000,
        }
    }
}
impl Default for Olf {
    fn default() -> Self {
        Olf {
            max_line_length: default_max_line_length(),
        }
    }
}

pub fn format(config: PasFmtConfiguration, err_handler: impl ErrHandler) {
    let formatting_settings = match config.get_config_object::<FormattingSettings>() {
        Ok(formatting_settings) => formatting_settings,
        Err(e) => {
            err_handler(e);
            return;
        }
    };
    log::debug!("Configuration:\n{:#?}", formatting_settings);

    let encoding = formatting_settings.encoding;
    let formatter = match make_formatter(&formatting_settings) {
        Ok(f) => f,
        Err(e) => {
            err_handler(anyhow::Error::from(e));
            return;
        }
    };

    let file_formatter = FileFormatter::new(formatter, encoding);
    FormattingOrchestrator::run(file_formatter, config, err_handler)
}

pub fn make_formatter(
    settings: &FormattingSettings,
) -> Result<Formatter, InvalidReconstructionSettingsError> {
    let reconstruction_settings: ReconstructionSettings =
        settings.reconstruction.clone().try_into()?;

    let eof_newline_formatter = &EofNewline {};

    Ok(Formatter::builder()
        .lexer(DelphiLexer {})
        .parser(DelphiLogicalLineParser {})
        .token_consolidator(DistinguishGenericTypeParamsConsolidator {})
        .token_ignorer(FormattingToggler {})
        .token_ignorer(IgnoreNonUnitImportClauses {})
        .token_ignorer(IgnoreAsmIstructions {})
        .file_formatter(TokenSpacing {})
        .line_formatter(FormatterSelector::new(
            |logical_line_type| match logical_line_type {
                LogicalLineType::Eof => Some(eof_newline_formatter),
                _ => None,
            },
        ))
        .file_formatter(OptimisingLineFormatter::new(
            settings.olf.clone().into(),
            reconstruction_settings.clone(),
        ))
        .reconstructor(DelphiLogicalLinesReconstructor::new(
            reconstruction_settings,
        ))
        .build())
}
