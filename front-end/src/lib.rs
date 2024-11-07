#![deny(clippy::enum_glob_use)]

use encoding_rs::Encoding;
use pasfmt_core::prelude::*;
use pasfmt_orchestrator::predule::*;
use serde_derive::{Deserialize, Serialize};

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
                "Failed to convert system codepage {} to encoding. Defaulting to UTF-8.",
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

#[derive(Serialize, Deserialize, Debug)]
pub struct FormattingSettings {
    reconstruction: Reconstruction,
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

#[derive(Serialize, Deserialize, Debug)]
struct Reconstruction {
    eol: String,
    indentation: String,
    continuation: Option<String>,
}
impl Default for Reconstruction {
    fn default() -> Self {
        Reconstruction {
            eol: "\r\n".to_owned(),
            indentation: "  ".to_owned(),
            continuation: None,
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
