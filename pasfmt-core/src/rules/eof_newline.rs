use crate::{lang::*, traits::LogicalLineFormatter};

pub struct EofNewline {}
impl LogicalLineFormatter for EofNewline {
    fn format<'a>(
        &self,
        mut formatted_tokens: FormattedTokens<'a>,
        input: &LogicalLine,
    ) -> FormattedTokens<'a> {
        let eof_tokens: Vec<_> = input
            .get_tokens()
            .iter()
            .filter_map(|&token_index| {
                match formatted_tokens.get_token_type_for_index(token_index) {
                    Some(TokenType::Eof) => Some(token_index),
                    _ => None,
                }
            })
            .collect();
        eof_tokens.into_iter().for_each(|eof_token| {
            if let Some(token_formatting_data) =
                formatted_tokens.get_or_create_formatting_data_mut(eof_token)
            {
                *token_formatting_data.get_newlines_before_mut() = 1;
                *token_formatting_data.get_spaces_before_mut() = 0;
                *token_formatting_data.get_indentations_before_mut() = 0;
                *token_formatting_data.get_continuations_before_mut() = 0;
            }
        });

        formatted_tokens
    }
}

#[cfg(test)]
mod tests {
    use spectral::prelude::*;

    use super::*;
    use crate::{
        defaults::lexer::DelphiLexer, defaults::parser::DelphiLogicalLineParser,
        defaults::reconstructor::DelphiLogicalLinesReconstructor, formatter::Formatter,
    };

    fn run_test(input: &'static str, output: &'static str) {
        let formatter = Formatter::new(
            Box::new(DelphiLexer {}),
            vec![],
            Box::new(DelphiLogicalLineParser {}),
            vec![],
            vec![Box::new(EofNewline {})],
            Box::new(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_string(), "  ".to_string(), "  ".to_string()),
            )),
        );

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
