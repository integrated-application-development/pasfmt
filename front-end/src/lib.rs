use pasfmt_core::prelude::*;
use pasfmt_orchestrator::predule::*;
use serde_derive::Deserialize;

#[derive(Deserialize, Default, Debug)]
pub struct FormattingSettings {
    reconstruction: Reconstruction,
}
impl From<Reconstruction> for ReconstructionSettings {
    fn from(val: Reconstruction) -> Self {
        ReconstructionSettings::new(
            val.eol,
            val.indentation.clone(),
            val.continuation.unwrap_or(val.indentation),
        )
    }
}

#[derive(Deserialize, Debug)]
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
            continuation: Some("  ".to_owned()),
        }
    }
}

pub fn format_with_settings(formatting_settings: FormattingSettings, config: PasFmtConfiguration) {
    let uses_clause_formatter = &UsesClauseFormatter {};
    let eof_newline_formatter = &EofNewline {};
    let formatter = FileFormatter::new(
        Formatter::builder()
            .lexer(DelphiLexer {})
            .token_consolidator(DistinguishGenericTypeParamsConsolidator {})
            .parser(DelphiLogicalLineParser {})
            .lines_consolidator(UsesClauseConsolidator {})
            .file_formatter(FormattingToggler {})
            .file_formatter(OperatorSpacing {})
            .line_formatter(RemoveRepeatedNewlines {})
            .line_formatter(FormatterSelector::new(
                |logical_line_type| match logical_line_type {
                    LogicalLineType::UsesClause => Some(uses_clause_formatter),
                    LogicalLineType::Eof => Some(eof_newline_formatter),
                    _ => None,
                },
            ))
            .reconstructor(DelphiLogicalLinesReconstructor::new(
                formatting_settings.reconstruction.into(),
            ))
            .build(),
        encoding_rs::WINDOWS_1252,
    );
    FormattingOrchestrator::run(formatter, config);
}
