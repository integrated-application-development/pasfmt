use pasfmt_core::{
    defaults::{
        lexer::DelphiLexer, parser::DelphiLogicalLineParser,
        reconstructor::DelphiLogicalLinesReconstructor,
    },
    formatter::Formatter,
    formatter_selector::FormatterSelector,
    lang::{LogicalLineType, ReconstructionSettings},
    rules::{
        remove_repeated_newlines::RemoveRepeatedNewlines,
        remove_trailing_whitespace::RemoveTrailingWhitespace,
        uses_clause_consolidator::UsesClauseConsolidator,
        uses_clause_formatter::UsesClauseFormatter,
    },
};
use pasfmt_orchestrator::{
    command_line::PasFmtConfiguration, formatting_orchestrator::FormattingOrchestrator,
};
use serde_derive::Deserialize;

#[derive(Deserialize, Default, Debug)]
struct FormattingSettings {
    reconstruction: Reconstruction,
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
    let formatter = Formatter::new(
        Box::new(DelphiLexer {}),
        vec![],
        Box::new(DelphiLogicalLineParser {}),
        vec![Box::new(UsesClauseConsolidator {})],
        vec![
            Box::new(RemoveTrailingWhitespace {}),
            Box::new(RemoveRepeatedNewlines {}),
            Box::new(FormatterSelector::new(
                |logical_line_type| match logical_line_type {
                    LogicalLineType::UsesClause => Some(uses_clause_formatter),
                    _ => None,
                },
            )),
        ],
        Box::new(DelphiLogicalLinesReconstructor::new(
            ReconstructionSettings::new(
                formatting_settings.reconstruction.eol,
                formatting_settings.reconstruction.indentation.clone(),
                formatting_settings
                    .reconstruction
                    .continuation
                    .unwrap_or(formatting_settings.reconstruction.indentation),
            ),
        )),
    );
    FormattingOrchestrator::run(formatter);
}
