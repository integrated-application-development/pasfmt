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
    use crate::prelude::*;
    use spectral::prelude::*;

    fn run_test(input: &'static str, output: &'static str) {
        let formatter = Formatter::builder()
            .lexer(DelphiLexer {})
            .parser(DelphiLogicalLineParser {})
            .line_formatter(EofNewline {})
            .reconstructor(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), "  ".to_owned(), "  ".to_owned()),
            ))
            .build();

        let formatted_output = formatter.format(input);
        assert_that(&formatted_output).is_equal_to(output.to_string());
    }

    #[test]
    fn trailing_newline_on_single_line_input() {
        run_test("Foo; Bar;", "Foo; Bar;\n");
    }

    #[test]
    fn trailing_newline_on_multiline_input() {
        run_test("Foo;\nBar;", "Foo;\nBar;\n");
    }

    #[test]
    fn trailing_newline_on_multiline_and_trailing_whitespace_input() {
        run_test("Foo;\nBar;  ", "Foo;\nBar;\n");
    }
}
