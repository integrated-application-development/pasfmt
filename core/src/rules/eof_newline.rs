use crate::{lang::*, traits::LogicalLineFormatter};

pub struct EofNewline {}
impl LogicalLineFormatter for EofNewline {
    fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, _input: &LogicalLine) {
        let eof_index = formatted_tokens.get_tokens().len() - 1;
        if let Some(token_formatting_data) = formatted_tokens
            .get_token_type_for_index(eof_index)
            .filter(|typ| matches!(typ, TokenType::Eof))
            .and_then(|_| formatted_tokens.get_formatting_data_mut(eof_index))
        {
            token_formatting_data.newlines_before = 1;
            token_formatting_data.spaces_before = 0;
            token_formatting_data.indentations_before = 0;
            token_formatting_data.continuations_before = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{prelude::*, rules::test_utils::formatter_test_group};
    use spectral::prelude::*;

    fn formatter() -> Formatter {
        Formatter::builder()
            .lexer(DelphiLexer {})
            .parser(DelphiLogicalLineParser {})
            .line_formatter(EofNewline {})
            .reconstructor(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), "  ".to_owned(), "  ".to_owned()),
            ))
            .build()
    }

    formatter_test_group!(
        trailing_newlines,
        single_line = {"Foo; Bar;", "Foo; Bar;\n"},
        multiline = {"Foo;\nBar;", "Foo;\nBar;\n"},
        multiline_and_trailing_whitespace = {"Foo;\nBar;  ", "Foo;\nBar;\n"},
    );
}
