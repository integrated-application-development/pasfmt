#![cfg(test)]

use crate::prelude::*;

macro_rules! formatter_test_group {
    ($i:ident, $($case_name: ident = {$($e: expr),* $(,)?}),* $(,)?) => {
        #[yare::parameterized(
          $($case_name = { $($e),* }),*
        )]
        fn $i(input: &str, output: &str) {
            let formatter: Formatter = formatter();
            let formatted_output = formatter.format(input, FileOptions::new());
            ::pretty_assertions::assert_eq!(&formatted_output, output);
        }
    };
}

pub(crate) use formatter_test_group;

pub fn default_test_reconstructor() -> DelphiLogicalLinesReconstructor {
    DelphiLogicalLinesReconstructor::new(default_test_reconstruction_settings())
}

pub fn default_test_reconstruction_settings() -> ReconstructionSettings {
    ReconstructionSettings::new(LineEnding::Lf, TabKind::Soft, 2, 2)
}

fn ws_len(whitespace_and_content: &str) -> u32 {
    whitespace_and_content
        .bytes()
        .take_while(|b| b.is_ascii_whitespace())
        .count() as u32
}

pub fn new_token(whitespace_and_content: &str, token_type: TokenType) -> Token {
    Token::new_ref(
        whitespace_and_content,
        ws_len(whitespace_and_content),
        token_type,
    )
}

pub fn new_owning_token<'a>(whitespace_and_content: String, token_type: TokenType) -> Token<'a> {
    let ws_len = ws_len(&whitespace_and_content);
    Token::new_owned(whitespace_and_content, ws_len, token_type)
}
