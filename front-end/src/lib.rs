#![deny(clippy::enum_glob_use)]

use encoding_rs::Encoding;
use pasfmt_core::prelude::*;
use pasfmt_orchestrator::predule::*;
use serde_derive::Deserialize;

#[cfg(windows)]
fn windows_default_encoding() -> &'static Encoding {
    use log::warn;

    // SAFETY: yes it's a foreign function, but it's a simple one from the WinAPI that we
    // can assume to be safe.
    let ansi_codepage = unsafe { windows_sys::Win32::Globalization::GetACP() };
    ansi_codepage
        .try_into()
        .ok()
        .and_then(|cp: u16| codepage::to_encoding(cp))
        .unwrap_or_else(|| {
            warn!(
                "failed to convert system codepage {} to encoding, defaulting to UTF-8",
                ansi_codepage
            );
            encoding_rs::UTF_8
        })
}

fn default_encoding() -> &'static Encoding {
    #[cfg(windows)]
    {
        windows_default_encoding()
    }
    #[cfg(not(windows))]
    encoding_rs::UTF_8
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct FormattingSettings {
    #[serde(default = "Reconstruction::default")]
    reconstruction: Reconstruction,
    #[serde(default = "default_encoding")]
    encoding: &'static Encoding,
}

impl Default for FormattingSettings {
    fn default() -> Self {
        Self {
            reconstruction: Default::default(),
            encoding: default_encoding(),
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

#[derive(Deserialize, Debug)]
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
    None
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

pub fn format_with_settings(
    formatting_settings: FormattingSettings,
    config: PasFmtConfiguration,
    err_handler: impl ErrHandler,
) {
    let uses_clause_formatter = &UsesClauseFormatter {};
    let eof_newline_formatter = &EofNewline {};

    let reconstruction_settings: ReconstructionSettings =
        match formatting_settings.reconstruction.try_into() {
            Ok(s) => s,
            Err(e) => {
                err_handler(anyhow::Error::from(e));
                return;
            }
        };

    let formatter = FileFormatter::new(
        Formatter::builder()
            .lexer(DelphiLexer {})
            .parser(DelphiLogicalLineParser {})
            .token_consolidator(DistinguishGenericTypeParamsConsolidator {})
            .lines_consolidator(ImportClauseConsolidator {})
            .token_ignorer(FormattingToggler {})
            .token_ignorer(IgnoreNonUnitImportClauses {})
            .file_formatter(TokenSpacing {})
            .line_formatter(RemoveRepeatedNewlines {})
            .line_formatter(FormatterSelector::new(
                |logical_line_type| match logical_line_type {
                    LogicalLineType::ImportClause => Some(uses_clause_formatter),
                    LogicalLineType::Eof => Some(eof_newline_formatter),
                    _ => None,
                },
            ))
            .reconstructor(DelphiLogicalLinesReconstructor::new(
                reconstruction_settings,
            ))
            .build(),
        formatting_settings.encoding,
    );
    FormattingOrchestrator::run(formatter, config, err_handler);
}
