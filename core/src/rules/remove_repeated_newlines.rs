use std::cmp::min;

use crate::{lang::*, traits::LogicalLineFormatter};

pub struct RemoveRepeatedNewlines {}
impl LogicalLineFormatter for RemoveRepeatedNewlines {
    fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &LogicalLine) {
        for &token_index in input.get_tokens() {
            if let Some(token_formatting_data) =
                formatted_tokens.get_formatting_data_mut(token_index)
            {
                *token_formatting_data.get_newlines_before_mut() =
                    min(token_formatting_data.get_newlines_before(), 2);
            }
        }
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
            vec![Box::new(RemoveRepeatedNewlines {})],
            Box::new(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_string(), "  ".to_string(), "  ".to_string()),
            )),
        );

        let formatted_output = formatter.format(input);
        assert_that(&formatted_output).is_equal_to(output.to_string());
    }

    #[test]
    fn no_new_lines_before() {
        run_test("Foo; Bar;", "Foo; Bar;");
    }

    #[test]
    fn one_new_line_before() {
        run_test("Foo;\nBar;", "Foo;\nBar;");
    }

    #[test]
    fn two_new_lines_before() {
        run_test("Foo;\n\nBar;", "Foo;\n\nBar;");
    }

    #[test]
    fn three_new_lines_before() {
        run_test("Foo;\n\n\nBar;", "Foo;\n\nBar;");
    }

    #[test]
    fn four_new_lines_before() {
        run_test("Foo;\n\n\n\nBar;", "Foo;\n\nBar;");
    }
}
