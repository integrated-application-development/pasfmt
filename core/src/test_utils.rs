#![cfg(test)]

use crate::prelude::*;

macro_rules! formatter_test_group {
    ($i:ident, $($case_name: ident = {$($e: expr),* $(,)?}),* $(,)?) => {
        #[yare::parameterized(
          $($case_name = { $($e),* }),*
        )]
        fn $i(input: &str, output: &str) {
            let formatter: Formatter = formatter();
            let formatted_output = formatter.format(input);
            assert_that(&formatted_output).is_equal_to(output.to_string());
        }
    };
}

pub(crate) use formatter_test_group;

pub fn default_test_reconstructor() -> DelphiLogicalLinesReconstructor {
    DelphiLogicalLinesReconstructor::new(ReconstructionSettings::new(
        "\n".to_owned(),
        "  ".to_owned(),
        "  ".to_owned(),
    ))
}

pub fn new_token<'a>(whitespace: &'a str, content: &'a str, token_type: TokenType) -> Token<'a> {
    Token::RefToken(RefToken::new(whitespace, content, token_type))
}

pub fn new_owning_token<'a>(
    whitespace: String,
    content: String,
    token_type: TokenType,
) -> Token<'a> {
    Token::OwningToken(OwningToken::new(whitespace, content, token_type))
}
