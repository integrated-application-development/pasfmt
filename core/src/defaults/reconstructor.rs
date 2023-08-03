use itertools::Itertools;

use crate::{lang::*, traits::LogicalLinesReconstructor};

pub struct DelphiLogicalLinesReconstructor {
    reconstruction_settings: ReconstructionSettings,
}
impl DelphiLogicalLinesReconstructor {
    pub fn new(reconstruction_settings: ReconstructionSettings) -> Self {
        DelphiLogicalLinesReconstructor {
            reconstruction_settings,
        }
    }
}
impl LogicalLinesReconstructor for DelphiLogicalLinesReconstructor {
    fn reconstruct(&self, formatted_tokens: FormattedTokens) -> String {
        let mut out = String::new();
        formatted_tokens
            .get_tokens()
            .iter()
            .sorted_by_key(|(token, _)| token.get_index())
            .for_each(|(token, formatting_data)| {
                if formatting_data.ignored {
                    out.push_str(token.get_leading_whitespace());
                } else {
                    (0..formatting_data.newlines_before)
                        .for_each(|_| out.push_str(self.reconstruction_settings.get_newline_str()));
                    (0..formatting_data.indentations_before).for_each(|_| {
                        out.push_str(self.reconstruction_settings.get_indentation_str())
                    });
                    (0..formatting_data.continuations_before).for_each(|_| {
                        out.push_str(self.reconstruction_settings.get_continuation_str())
                    });
                    (0..formatting_data.spaces_before).for_each(|_| out.push(' '));
                };

                out.push_str(token.get_content());
            });
        out
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use spectral::prelude::*;

    fn run_test(input: FormattedTokens, expected_output: &str) {
        let reconstructor = DelphiLogicalLinesReconstructor {
            reconstruction_settings: ReconstructionSettings::new(
                "\n".to_owned(),
                "  ".to_owned(),
                "  ".to_owned(),
            ),
        };
        let output = reconstructor.reconstruct(input);
        assert_that(&output).is_equal_to(expected_output.to_string());
    }

    fn ignored_formatting_data() -> FormattingData {
        FormattingData {
            ignored: true,
            ..Default::default()
        }
    }

    #[test]
    fn all_tokens_with_data_from_token() {
        run_test(
            FormattedTokens::new(vec![
                (
                    Token::RefToken(RefToken::new(0, "\n\n  ", "token1", TokenType::Unknown)),
                    FormattingData::from("\n\n  "),
                ),
                (
                    Token::RefToken(RefToken::new(1, " ", "token2", TokenType::Unknown)),
                    FormattingData::from(" "),
                ),
            ]),
            "\n\n  token1 token2",
        );
    }

    #[test]
    fn all_tokens_with_data_constructed_spaces() {
        let formatting_data1 = FormattingData {
            spaces_before: 3,
            ..Default::default()
        };
        let formatting_data2 = FormattingData {
            spaces_before: 1,
            ..Default::default()
        };
        run_test(
            FormattedTokens::new(vec![
                (
                    Token::RefToken(RefToken::new(0, "", "token1", TokenType::Unknown)),
                    formatting_data1,
                ),
                (
                    Token::RefToken(RefToken::new(1, "", "token2", TokenType::Unknown)),
                    formatting_data2,
                ),
            ]),
            "   token1 token2",
        );
    }

    #[test]
    fn all_tokens_with_data_constructed_indentations() {
        let formatting_data1 = FormattingData {
            indentations_before: 3,
            ..Default::default()
        };
        let formatting_data2 = FormattingData {
            indentations_before: 1,
            ..Default::default()
        };
        run_test(
            FormattedTokens::new(vec![
                (
                    Token::RefToken(RefToken::new(0, "", "token1", TokenType::Unknown)),
                    formatting_data1,
                ),
                (
                    Token::RefToken(RefToken::new(1, "", "token2", TokenType::Unknown)),
                    formatting_data2,
                ),
            ]),
            "      token1  token2",
        );
    }

    #[test]
    fn all_tokens_with_data_constructed_continuations() {
        let formatting_data1 = FormattingData {
            continuations_before: 3,
            ..Default::default()
        };
        let formatting_data2 = FormattingData {
            continuations_before: 1,
            ..Default::default()
        };
        run_test(
            FormattedTokens::new(vec![
                (
                    Token::RefToken(RefToken::new(0, "", "token1", TokenType::Unknown)),
                    formatting_data1,
                ),
                (
                    Token::RefToken(RefToken::new(1, "", "token2", TokenType::Unknown)),
                    formatting_data2,
                ),
            ]),
            "      token1  token2",
        );
    }

    #[test]
    fn some_tokens_with_data() {
        let formatting_data1 = FormattingData {
            newlines_before: 3,
            ..Default::default()
        };
        run_test(
            FormattedTokens::new(vec![
                (
                    Token::RefToken(RefToken::new(0, " ", "token1", TokenType::Unknown)),
                    formatting_data1,
                ),
                (
                    Token::RefToken(RefToken::new(1, "\n   ", "token2", TokenType::Unknown)),
                    ignored_formatting_data(),
                ),
            ]),
            "\n\n\ntoken1\n   token2",
        );
        let formatting_data2 = FormattingData {
            newlines_before: 3,
            ..Default::default()
        };
        run_test(
            FormattedTokens::new(vec![
                (
                    Token::RefToken(RefToken::new(0, " ", "token1", TokenType::Unknown)),
                    ignored_formatting_data(),
                ),
                (
                    Token::RefToken(RefToken::new(1, "\n   ", "token2", TokenType::Unknown)),
                    formatting_data2,
                ),
            ]),
            " token1\n\n\ntoken2",
        );
    }

    #[test]
    fn no_tokens_with_data() {
        run_test(
            FormattedTokens::new(vec![
                (
                    Token::RefToken(RefToken::new(0, " ", "token1", TokenType::Unknown)),
                    ignored_formatting_data(),
                ),
                (
                    Token::RefToken(RefToken::new(1, "\n   ", "token2", TokenType::Unknown)),
                    ignored_formatting_data(),
                ),
            ]),
            " token1\n   token2",
        );
    }

    #[test]
    fn trailing_newlines() {
        run_test(
            FormattedTokens::new(vec![
                (
                    Token::RefToken(RefToken::new(0, "", "token1", TokenType::Unknown)),
                    ignored_formatting_data(),
                ),
                (
                    Token::RefToken(RefToken::new(1, "\n", "", TokenType::Eof)),
                    ignored_formatting_data(),
                ),
            ]),
            "token1\n",
        );
    }

    #[test]
    fn unrepresentable_leading_whitespace() {
        run_test(
            FormattedTokens::new(vec![(
                Token::RefToken(RefToken::new(0, "\n \n\t", "token1", TokenType::Unknown)),
                ignored_formatting_data(),
            )]),
            "\n \n\ttoken1",
        );
    }

    #[test]
    fn tokens_out_of_order() {
        run_test(
            FormattedTokens::new(vec![
                (
                    Token::RefToken(RefToken::new(1, " ", "token2", TokenType::Unknown)),
                    ignored_formatting_data(),
                ),
                (
                    Token::RefToken(RefToken::new(0, "", "token1", TokenType::Unknown)),
                    ignored_formatting_data(),
                ),
            ]),
            "token1 token2",
        );
    }
}
