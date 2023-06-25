use crate::{lang::*, traits::LogicalLinesReconstructor};

pub struct DelphiLogicalLinesReconstructor {
    reconstruction_settings: ReconstructionSettings,
}
#[allow(dead_code)]
impl DelphiLogicalLinesReconstructor {
    pub fn new(reconstruction_settings: ReconstructionSettings) -> Self {
        DelphiLogicalLinesReconstructor {
            reconstruction_settings,
        }
    }
}
impl LogicalLinesReconstructor for DelphiLogicalLinesReconstructor {
    fn reconstruct(&self, formatted_tokens: FormattedTokens) -> String {
        let (mut tokens, token_formatting_data) = formatted_tokens.own_data();

        tokens.sort_by_key(|token_a| token_a.get_index());

        tokens
            .iter()
            .fold("".to_owned(), |acc: String, token: &Token| {
                let possible_formatting_data = token_formatting_data
                    .iter()
                    .find(|formatting_data| formatting_data.get_token_index() == token.get_index());

                let leading_whitespace = match possible_formatting_data {
                    None => token.get_leading_whitespace().to_string(),
                    Some(formatting_data) => format!(
                        "{}{}{}{}",
                        self.reconstruction_settings
                            .get_newline_str()
                            .repeat(formatting_data.get_newlines_before()),
                        self.reconstruction_settings
                            .get_indentation_str()
                            .repeat(formatting_data.get_indentations_before()),
                        self.reconstruction_settings
                            .get_continuation_str()
                            .repeat(formatting_data.get_continuations_before()),
                        " ".repeat(formatting_data.get_spaces_before())
                    ),
                };

                format!("{}{}{}", acc, leading_whitespace, token.get_content())
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn all_tokens_with_data_from_token() {
        run_test(
            FormattedTokens::new(
                vec![
                    Token::RefToken(RefToken::new(0, "\n\n  ", "token1", TokenType::Unknown)),
                    Token::RefToken(RefToken::new(1, " ", "token2", TokenType::Unknown)),
                ],
                vec![
                    FormattingData::from(0, "\n\n  "),
                    FormattingData::from(1, " "),
                ],
            ),
            "\n\n  token1 token2",
        );
    }

    #[test]
    fn all_tokens_with_data_constructed_spaces() {
        let mut formatting_data1 = FormattingData::new(0);
        *formatting_data1.get_spaces_before_mut() = 3;
        let mut formatting_data2 = FormattingData::new(1);
        *formatting_data2.get_spaces_before_mut() = 1;
        run_test(
            FormattedTokens::new(
                vec![
                    Token::RefToken(RefToken::new(0, "", "token1", TokenType::Unknown)),
                    Token::RefToken(RefToken::new(1, "", "token2", TokenType::Unknown)),
                ],
                vec![formatting_data1, formatting_data2],
            ),
            "   token1 token2",
        );
    }

    #[test]
    fn all_tokens_with_data_constructed_indentations() {
        let mut formatting_data1 = FormattingData::new(0);
        *formatting_data1.get_indentations_before_mut() = 3;
        let mut formatting_data2 = FormattingData::new(1);
        *formatting_data2.get_indentations_before_mut() = 1;
        run_test(
            FormattedTokens::new(
                vec![
                    Token::RefToken(RefToken::new(0, "", "token1", TokenType::Unknown)),
                    Token::RefToken(RefToken::new(1, "", "token2", TokenType::Unknown)),
                ],
                vec![formatting_data1, formatting_data2],
            ),
            "      token1  token2",
        );
    }

    #[test]
    fn all_tokens_with_data_constructed_continuations() {
        let mut formatting_data1 = FormattingData::new(0);
        *formatting_data1.get_continuations_before_mut() = 3;
        let mut formatting_data2 = FormattingData::new(1);
        *formatting_data2.get_continuations_before_mut() = 1;
        run_test(
            FormattedTokens::new(
                vec![
                    Token::RefToken(RefToken::new(0, "", "token1", TokenType::Unknown)),
                    Token::RefToken(RefToken::new(1, "", "token2", TokenType::Unknown)),
                ],
                vec![formatting_data1, formatting_data2],
            ),
            "      token1  token2",
        );
    }

    #[test]
    fn some_tokens_with_data() {
        let mut formatting_data1 = FormattingData::new(0);
        *formatting_data1.get_newlines_before_mut() = 3;
        run_test(
            FormattedTokens::new(
                vec![
                    Token::RefToken(RefToken::new(0, " ", "token1", TokenType::Unknown)),
                    Token::RefToken(RefToken::new(1, "\n   ", "token2", TokenType::Unknown)),
                ],
                vec![formatting_data1],
            ),
            "\n\n\ntoken1\n   token2",
        );
        let mut formatting_data2 = FormattingData::new(1);
        *formatting_data2.get_newlines_before_mut() = 3;
        run_test(
            FormattedTokens::new(
                vec![
                    Token::RefToken(RefToken::new(0, " ", "token1", TokenType::Unknown)),
                    Token::RefToken(RefToken::new(1, "\n   ", "token2", TokenType::Unknown)),
                ],
                vec![formatting_data2],
            ),
            " token1\n\n\ntoken2",
        );
    }

    #[test]
    fn no_tokens_with_data() {
        run_test(
            FormattedTokens::new(
                vec![
                    Token::RefToken(RefToken::new(0, " ", "token1", TokenType::Unknown)),
                    Token::RefToken(RefToken::new(1, "\n   ", "token2", TokenType::Unknown)),
                ],
                vec![],
            ),
            " token1\n   token2",
        );
    }

    #[test]
    fn trailing_newlines() {
        run_test(
            FormattedTokens::new(
                vec![
                    Token::RefToken(RefToken::new(0, "", "token1", TokenType::Unknown)),
                    Token::RefToken(RefToken::new(1, "\n", "", TokenType::Eof)),
                ],
                vec![],
            ),
            "token1\n",
        );
    }

    #[test]
    fn unrepresentable_leading_whitespace() {
        run_test(
            FormattedTokens::new(
                vec![Token::RefToken(RefToken::new(
                    0,
                    "\n \n\t",
                    "token1",
                    TokenType::Unknown,
                ))],
                vec![],
            ),
            "\n \n\ttoken1",
        );
    }

    #[test]
    fn tokens_out_of_order() {
        run_test(
            FormattedTokens::new(
                vec![
                    Token::RefToken(RefToken::new(1, " ", "token2", TokenType::Unknown)),
                    Token::RefToken(RefToken::new(0, "", "token1", TokenType::Unknown)),
                ],
                vec![],
            ),
            "token1 token2",
        );
    }
}
