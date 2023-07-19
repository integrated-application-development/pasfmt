use crate::{lang::*, traits::LogicalLineFormatter};

pub struct RemoveTrailingWhitespace {}
impl LogicalLineFormatter for RemoveTrailingWhitespace {
    fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &LogicalLine) {
        // The action of creating formatting data for a token will remove
        // trailing whitespace. Trailing whitespace is unrepresentable using the
        // `FormattingData` struct.

        for &token_index in input.get_tokens() {
            formatted_tokens.get_or_create_formatting_data_mut(token_index);
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
            vec![Box::new(RemoveTrailingWhitespace {})],
            Box::new(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_string(), "  ".to_string(), "  ".to_string()),
            )),
        );

        let formatted_output = formatter.format(input);
        assert_that(&formatted_output).is_equal_to(output.to_string());
    }

    #[test]
    fn trailing_whitespace() {
        // The newline indicates the eof
        run_test("Foo \n", "Foo\n");
        run_test("Foo  \n", "Foo\n");
        run_test("Foo\t\n", "Foo\n");
        run_test("Foo\t\n", "Foo\n");
    }

    #[test]
    fn trailing_whitespace_in_leading_whitespace() {
        run_test(" \nFoo", "\nFoo");
        run_test("\n \nFoo", "\n\nFoo");
        run_test("\t\nFoo", "\nFoo");
        run_test("\n\t\nFoo", "\n\nFoo");
    }
}
