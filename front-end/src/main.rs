use pasfmt_core::{
    defaults::{
        lexer::DelphiLexer, parser::DelphiLogicalLineParser,
        reconstructor::DelphiLogicalLinesReconstructor,
    },
    formatter::{Formatter, *},
    formatter_selector::FormatterSelector,
    lang::{LogicalLineType, ReconstructionSettings},
    rules::{
        eof_newline::EofNewline, remove_repeated_newlines::RemoveRepeatedNewlines,
        uses_clause_consolidator::UsesClauseConsolidator,
        uses_clause_formatter::UsesClauseFormatter,
    },
};
use pasfmt_orchestrator::{
    command_line::PasFmtConfiguration, file_formatter::FileFormatter,
    formatting_orchestrator::FormattingOrchestrator,
};
use serde_derive::Deserialize;

#[derive(Deserialize, Default, Debug)]
struct FormattingSettings {
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

fn main() {
    let config = PasFmtConfiguration::new();
    let formatting_settings = config.get_config_object::<FormattingSettings>();

    let uses_clause_formatter = &UsesClauseFormatter {};
    let eof_newline_formatter = &EofNewline {};
    let formatter = FileFormatter::new(
        Formatter::builder()
            .lexer(DelphiLexer {})
            .parser(DelphiLogicalLineParser {})
            .lines_consolidator(UsesClauseConsolidator {})
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
    FormattingOrchestrator::run(formatter);
}
