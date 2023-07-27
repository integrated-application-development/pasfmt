use crate::lang::*;
use crate::traits::*;
use itertools::Itertools;

#[allow(dead_code)]
pub struct Formatter {
    lexer: Box<dyn Lexer + Sync>,
    token_consolidators: Vec<Box<dyn TokenConsolidator + Sync>>,
    logical_line_parser: Box<dyn LogicalLineParser + Sync>,
    logical_line_consolidators: Vec<Box<dyn LogicalLinesConsolidator + Sync>>,
    logical_line_formatters: Vec<Box<dyn LogicalLineFormatter + Sync>>,
    reconstructor: Box<dyn LogicalLinesReconstructor + Sync>,
}
#[allow(dead_code)]
impl Formatter {
    pub fn new(
        lexer: Box<dyn Lexer + Sync>,
        token_consolidators: Vec<Box<dyn TokenConsolidator + Sync>>,
        logical_line_parser: Box<dyn LogicalLineParser + Sync>,
        logical_line_consolidators: Vec<Box<dyn LogicalLinesConsolidator + Sync>>,
        logical_line_formatters: Vec<Box<dyn LogicalLineFormatter + Sync>>,
        reconstructor: Box<dyn LogicalLinesReconstructor + Sync>,
    ) -> Self {
        Formatter {
            lexer,
            token_consolidators,
            logical_line_parser,
            logical_line_consolidators,
            logical_line_formatters,
            reconstructor,
        }
    }
    pub fn format(&self, input: &str) -> String {
        Some(input)
            .map(|input| self.lexer.lex(input))
            .map(|tokens| {
                self.token_consolidators
                    .iter()
                    .fold(tokens, |tokens, consolidator| {
                        consolidator
                            .consolidate(tokens)
                            .into_iter()
                            .sorted_by_key(|token| token.get_index())
                            .collect::<Vec<Token>>()
                    })
            })
            .map(|tokens| self.logical_line_parser.parse(tokens))
            .map(|logical_lines| {
                self.logical_line_consolidators
                    .iter()
                    .fold(logical_lines, |logical_lines, consolidator| {
                        consolidator.consolidate(logical_lines)
                    })
                    .into()
            })
            .map(|(tokens, logical_lines)| {
                self.logical_line_formatters.iter().fold(
                    FormattedTokens::new_from_tokens(tokens),
                    |formatted_tokens, formatter| {
                        logical_lines.iter().fold(
                            formatted_tokens,
                            |mut formatted_tokens, logical_line| {
                                formatter.format(&mut formatted_tokens, logical_line);
                                formatted_tokens
                            },
                        )
                    },
                )
            })
            .map(|formatted_tokens| self.reconstructor.reconstruct(formatted_tokens))
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::defaults::{
        lexer::DelphiLexer, parser::DelphiLogicalLineParser,
        reconstructor::DelphiLogicalLinesReconstructor,
    };

    use super::*;
    use indoc::indoc;
    use spectral::prelude::*;

    fn run_test(formatter: Formatter, input: &str, expected_output: &str) {
        let output = formatter.format(input);
        assert_that(&output).is_equal_to(expected_output.to_string());
    }

    struct AddNumberTokenToPrevious;
    impl TokenConsolidator for AddNumberTokenToPrevious {
        fn consolidate<'a>(&self, mut tokens: Vec<Token<'a>>) -> Vec<Token<'a>> {
            for token_index in (1..tokens.len()).rev() {
                if tokens.get(token_index).unwrap().get_token_type()
                    == TokenType::NumberLiteral(NumberLiteralKind::Decimal)
                {
                    let second_token = tokens.remove(token_index);
                    let first_token = tokens.remove(token_index - 1);

                    tokens.push(Token::OwningToken(OwningToken::new(
                        first_token.get_index(),
                        first_token.get_leading_whitespace().to_owned(),
                        first_token.get_content().to_owned() + second_token.get_content(),
                        first_token.get_token_type(),
                    )))
                }
            }
            tokens
        }
    }

    struct Append1ToAllTokens;
    impl TokenConsolidator for Append1ToAllTokens {
        fn consolidate<'a>(&self, mut tokens: Vec<Token<'a>>) -> Vec<Token<'a>> {
            let tokens_to_remove: Vec<_> = (0..tokens.len() - 1)
                .map(|token_index| {
                    println!("{}", token_index);
                    let token = tokens.get(token_index).unwrap();

                    tokens.push(Token::OwningToken(OwningToken::new(
                        token.get_index(),
                        token.get_leading_whitespace().to_owned(),
                        token.get_content().to_owned() + "1",
                        token.get_token_type(),
                    )));

                    token_index
                })
                .collect();
            tokens_to_remove.into_iter().rev().for_each(|index| {
                tokens.remove(index);
            });
            tokens
        }
    }

    #[test]
    fn no_optional_stages() {
        let formatter = Formatter::new(
            Box::new(DelphiLexer {}),
            vec![],
            Box::new(DelphiLogicalLineParser {}),
            vec![],
            vec![],
            Box::new(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), "  ".to_owned(), "  ".to_owned()),
            )),
        );
        run_test(formatter, "a 1 b 2 c 3", "a 1 b 2 c 3");
    }

    #[test]
    fn single_token_consolidator() {
        let formatter = Formatter::new(
            Box::new(DelphiLexer {}),
            vec![Box::new(AddNumberTokenToPrevious {})],
            Box::new(DelphiLogicalLineParser {}),
            vec![],
            vec![],
            Box::new(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), "  ".to_owned(), "  ".to_owned()),
            )),
        );
        run_test(formatter, "a 1 b 2 c 3", "a1 b2 c3");
    }

    #[test]
    fn multiple_token_consolidators() {
        let formatter = Formatter::new(
            Box::new(DelphiLexer {}),
            vec![
                Box::new(AddNumberTokenToPrevious {}),
                Box::new(Append1ToAllTokens {}),
            ],
            Box::new(DelphiLogicalLineParser {}),
            vec![],
            vec![],
            Box::new(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), "  ".to_owned(), "  ".to_owned()),
            )),
        );
        run_test(formatter, "a 1 b 2 c 3", "a11 b21 c31");
    }

    struct CombineFirst2Lines;
    impl LogicalLinesConsolidator for CombineFirst2Lines {
        fn consolidate<'a>(&self, input: LogicalLines<'a>) -> LogicalLines<'a> {
            let (tokens, mut lines) = input.into();
            let second_line = lines.remove(1);
            let second_line_tokens = second_line.get_tokens();
            lines
                .get_mut(0)
                .unwrap()
                .get_tokens_mut()
                .extend(second_line_tokens);

            LogicalLines::new(tokens, lines)
        }
    }

    struct LogicalLinesOnNewLines;
    impl LogicalLineFormatter for LogicalLinesOnNewLines {
        fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &LogicalLine) {
            let first_token = *input.get_tokens().first().unwrap();
            if first_token != 0 && first_token != formatted_tokens.get_tokens().len() - 1 {
                if let Some(formatting_data) = formatted_tokens.get_formatting_data_mut(first_token)
                {
                    *formatting_data.get_spaces_before_mut() = 0;
                    *formatting_data.get_newlines_before_mut() = 1;
                }
            }
        }
    }

    #[test]
    fn single_line_consolidator() {
        let formatter = Formatter::new(
            Box::new(DelphiLexer {}),
            vec![],
            Box::new(DelphiLogicalLineParser {}),
            vec![Box::new(CombineFirst2Lines {})],
            vec![Box::new(LogicalLinesOnNewLines {})],
            Box::new(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), "  ".to_owned(), "  ".to_owned()),
            )),
        );
        run_test(
            formatter,
            "a; b; c; d;",
            indoc! {"
                a; b;
                c;
                d;"
            },
        );
    }

    #[test]
    fn multiple_line_consolidators() {
        let formatter = Formatter::new(
            Box::new(DelphiLexer {}),
            vec![],
            Box::new(DelphiLogicalLineParser {}),
            vec![
                Box::new(CombineFirst2Lines {}),
                Box::new(CombineFirst2Lines {}),
            ],
            vec![Box::new(LogicalLinesOnNewLines {})],
            Box::new(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), "  ".to_owned(), "  ".to_owned()),
            )),
        );
        run_test(
            formatter,
            "a; b; c; d;",
            indoc! {"
                a; b; c;
                d;"
            },
        );
    }

    struct SpaceBeforeSemiColon;
    impl LogicalLineFormatter for SpaceBeforeSemiColon {
        fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &LogicalLine) {
            let semicolon_indices: Vec<_> = input
                .get_tokens()
                .iter()
                .filter(|&&token_index| {
                    formatted_tokens.get_token_type_for_index(token_index)
                        == Some(TokenType::Op(OperatorKind::Semicolon))
                })
                .collect();

            semicolon_indices.iter().for_each(|&&semicolon_index| {
                if let Some(semicolon_formatting_data) =
                    formatted_tokens.get_formatting_data_mut(semicolon_index)
                {
                    *semicolon_formatting_data.get_spaces_before_mut() = 1;
                }
            });
        }
    }

    #[test]
    fn single_line_formatter() {
        let formatter = Formatter::new(
            Box::new(DelphiLexer {}),
            vec![],
            Box::new(DelphiLogicalLineParser {}),
            vec![],
            vec![Box::new(LogicalLinesOnNewLines {})],
            Box::new(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), "  ".to_owned(), "  ".to_owned()),
            )),
        );
        run_test(
            formatter,
            "a; b; c; d;",
            indoc! {"
                a;
                b;
                c;
                d;"
            },
        );
    }
    #[test]
    fn multiple_line_formatters() {
        let formatter = Formatter::new(
            Box::new(DelphiLexer {}),
            vec![Box::new(AddNumberTokenToPrevious {})],
            Box::new(DelphiLogicalLineParser {}),
            vec![],
            vec![
                Box::new(LogicalLinesOnNewLines {}),
                Box::new(SpaceBeforeSemiColon {}),
            ],
            Box::new(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), "  ".to_owned(), "  ".to_owned()),
            )),
        );
        run_test(
            formatter,
            "a; b; c; d;",
            indoc! {"
                a ;
                b ;
                c ;
                d ;"
            },
        );
    }
}
