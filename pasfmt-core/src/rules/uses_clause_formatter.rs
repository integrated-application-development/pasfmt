use std::cmp::max;

use crate::{lang::*, traits::LogicalLineFormatter};

pub struct UsesClauseFormatter {}
impl LogicalLineFormatter for UsesClauseFormatter {
    fn format<'a>(
        &self,
        mut formatted_tokens: FormattedTokens<'a>,
        input: &LogicalLine,
    ) -> FormattedTokens<'a> {
        let mut conditional_depth: isize = 0;
        for &token_index in input.get_tokens() {
            {
                // Reset formatting for the token
                let token_formatting_data =
                    formatted_tokens.get_or_create_formatting_data_mut(token_index);
                *token_formatting_data.get_spaces_before_mut() = 0;
            }
            if !matches!(
                formatted_tokens.get_token_type_for_index(token_index),
                TokenType::Keyword(PureKeywordKind::Uses)
                    | TokenType::Comment(CommentKind::IndividualLine)
                    | TokenType::Comment(CommentKind::IndividualBlock)
                    | TokenType::Comment(CommentKind::MultilineBlock)
            ) {
                // Reset newlines for tokens other than uses
                *formatted_tokens
                    .get_or_create_formatting_data_mut(token_index)
                    .get_newlines_before_mut() = 0;
            }

            if matches!(
                formatted_tokens.get_token_type_for_index(token_index),
                TokenType::Keyword(PureKeywordKind::Uses)
            ) {
                let token_formatting_data =
                    formatted_tokens.get_or_create_formatting_data_mut(token_index);
                *token_formatting_data.get_spaces_before_mut() = 0;
                if token_index > 0 {
                    *token_formatting_data.get_newlines_before_mut() =
                        max(token_formatting_data.get_newlines_before(), 1);
                }
            }

            if matches!(
                formatted_tokens.get_token_type_for_index(token_index),
                TokenType::ConditionalDirective(_)
            ) {
                conditional_depth += match formatted_tokens.get_token_type_for_index(token_index) {
                    TokenType::ConditionalDirective(ConditionalDirectiveKind::Endif)
                    | TokenType::ConditionalDirective(ConditionalDirectiveKind::Else)
                    | TokenType::ConditionalDirective(ConditionalDirectiveKind::Ifend) => -1,
                    _ => 0,
                };

                let token_formatting_data =
                    formatted_tokens.get_or_create_formatting_data_mut(token_index);
                *token_formatting_data.get_newlines_before_mut() = 1;
                *token_formatting_data.get_spaces_before_mut() = 0;
                *token_formatting_data.get_indentations_before_mut() =
                    conditional_depth.unsigned_abs();

                conditional_depth += match formatted_tokens.get_token_type_for_index(token_index) {
                    TokenType::ConditionalDirective(ConditionalDirectiveKind::If)
                    | TokenType::ConditionalDirective(ConditionalDirectiveKind::Else)
                    | TokenType::ConditionalDirective(ConditionalDirectiveKind::Ifdef)
                    | TokenType::ConditionalDirective(ConditionalDirectiveKind::Ifopt)
                    | TokenType::ConditionalDirective(ConditionalDirectiveKind::Ifndef)
                    | TokenType::ConditionalDirective(ConditionalDirectiveKind::Elseif) => 1,
                    _ => 0,
                }
            } else if matches!(
                formatted_tokens.get_token_type_for_index(token_index),
                TokenType::Keyword(PureKeywordKind::In)
                    | TokenType::TextLiteral
                    | TokenType::Comment(_)
            ) || token_index > 1
                && matches!(
                    formatted_tokens.get_token_type_for_index(token_index - 1),
                    TokenType::Op(OperatorKind::Comma)
                )
            {
                let token_formatting_data =
                    formatted_tokens.get_or_create_formatting_data_mut(token_index);
                *token_formatting_data.get_spaces_before_mut() = 1;
            }

            if matches!(
                formatted_tokens.get_token_type_for_index(token_index),
                TokenType::Op(OperatorKind::Comma)
                    | TokenType::Op(OperatorKind::Semicolon)
                    | TokenType::Comment(CommentKind::IndividualLine)
                    | TokenType::Comment(CommentKind::IndividualBlock)
                    | TokenType::Comment(CommentKind::MultilineBlock)
            ) {
                let token_formatting_data =
                    formatted_tokens.get_or_create_formatting_data_mut(token_index);
                *token_formatting_data.get_spaces_before_mut() = 2;
                *token_formatting_data.get_newlines_before_mut() = 1;
            }

            if matches!(
                formatted_tokens.get_token_type_for_index(token_index),
                TokenType::Identifier
            ) && token_index > 0
                && matches!(
                    formatted_tokens.get_token_type_for_index(token_index - 1),
                    TokenType::Keyword(PureKeywordKind::Uses)
                        | TokenType::ConditionalDirective(_)
                        | TokenType::Comment(CommentKind::IndividualLine)
                        | TokenType::Comment(CommentKind::InlineLine)
                )
            {
                let token_formatting_data =
                    formatted_tokens.get_or_create_formatting_data_mut(token_index);
                *token_formatting_data.get_spaces_before_mut() = 4;
                *token_formatting_data.get_newlines_before_mut() = 1;
            }
        }

        formatted_tokens
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use spectral::prelude::*;

    use super::*;
    use crate::{
        defaults::lexer::DelphiLexer,
        defaults::parser::DelphiLogicalLineParser,
        rules::uses_clause_consolidator::UsesClauseConsolidator,
        traits::LogicalLineParser,
        traits::{Lexer, LogicalLinesConsolidator},
    };

    fn run_test(input: &'static str, expected_output: &'static str) {
        let lexer = DelphiLexer {};
        let parser = DelphiLogicalLineParser {};
        let line_consolidator = UsesClauseConsolidator {};
        let formatter = UsesClauseFormatter {};

        let formatted_tokens = Some(input)
            .map(|input| lexer.lex(input))
            .map(|tokens| parser.parse(tokens))
            .map(|lines| line_consolidator.consolidate(lines).own_data())
            .map(|(tokens, lines)| {
                formatter.format(
                    FormattedTokens::new(tokens, vec![]),
                    lines
                        .iter()
                        .find(|line| line.get_line_type() == LogicalLineType::UsesClause)
                        .unwrap(),
                )
            })
            .unwrap();

        let formatted_output = formatted_tokens.get_tokens().iter().fold(
            "".to_owned(),
            |acc: String, token: &Token| {
                let possible_formatting_data = formatted_tokens
                    .get_formatting_data()
                    .iter()
                    .find(|formatting_data| formatting_data.get_token_index() == token.get_index());

                let leading_whitespace = match possible_formatting_data {
                    None => token.get_leading_whitespace().to_string(),
                    Some(formatting_data) => format!(
                        "{}{}{}",
                        "\n".repeat(formatting_data.get_newlines_before()),
                        "  ".repeat(formatting_data.get_indentations_before()),
                        " ".repeat(formatting_data.get_spaces_before())
                    ),
                };

                format!("{}{}{}", acc, leading_whitespace, token.get_content())
            },
        );
        println!("{}", formatted_output);
        assert_that(&formatted_output).is_equal_to(expected_output.to_string());
    }

    fn run_match_test(input: &'static str) {
        run_test(input, input);
    }

    #[test]
    fn indented_uses_keyword() {
        run_test(
            indoc! {"
                {$ifdef}
                    uses
                    Unit1
                  , Unit2
                  , Unit3
                  ;
                {$endif}"
            },
            indoc! {"
                {$ifdef}
                uses
                    Unit1
                  , Unit2
                  , Unit3
                  ;
                {$endif}"
            },
        );
    }

    #[test]
    fn indented_uses_keyword_formatted_differently() {
        run_test(
            indoc! {"
                {$ifdef}
                uses
                  Unit1,
                  Unit2,
                  Unit3;
                {$endif}"
            },
            indoc! {"
                {$ifdef}
                uses
                    Unit1
                  , Unit2
                  , Unit3
                  ;
                {$endif}"
            },
        );
    }

    #[test]
    fn uses_clause_with_wonky_conditionals() {
        run_match_test(indoc! {"
            uses
            {$ifdef}
                Unit1
              , Unit2
              ,
            {$endif}
                Unit3
              ;"
        });
    }

    #[test]
    fn uses_clause_with_nested_conditionals() {
        run_match_test(indoc! {"
            uses
                Unit1
            {$ifdef A}
              , Unit2
              {$ifdef B}
              , Unit3
              {$else}
                {$ifdef C}
              , Unit4
                {$endif}
              {$endif}
            {$endif}
              , Unit4
              ;"
        });
    }

    #[test]
    fn uses_clause_with_conditionals() {
        run_match_test(indoc! {"
            uses
            {$ifdef}
                Unit1
              , Unit2
            {$endif}
              , Unit3
              ;"
        });
    }

    #[test]
    fn uses_clause_no_conditionals() {
        run_match_test(indoc! {"
            uses
                Unit1
              , Unit2
              , Unit3
              ;"
        });
    }

    #[test]
    fn units_with_namespaces() {
        run_match_test(indoc! {"
            uses
                Unit1.Unit1
              , Unit2.Unit2
              , Unit3.Unit3
              ;"
        });
    }

    #[test]
    fn units_with_namespaces_and_directives() {
        run_match_test(indoc! {"
            uses
                Unit1.Unit1
            {$ifdef}
              , Unit2.Unit2
            {$endif}
              , Unit3.Unit3
              ;"
        });
    }

    #[test]
    fn units_with_in() {
        run_match_test(indoc! {"
            uses
                Unit1 in 'a/b.pas'
              , Unit2 in 'x/y.pas'
              , Unit3 in 'w/x.pas'
              ;"
        });
    }

    #[test]
    fn units_with_in_and_directives() {
        run_match_test(indoc! {"
            uses
                Unit1 in 'a/b.pas'
            {$ifdef}
              , Unit2 in 'x/y.pas'
            {$endif}
              , Unit3 in 'w/x.pas'
              ;"
        });
    }

    #[test]
    fn units_with_in_namespaces_and_directives() {
        run_match_test(indoc! {"
            uses
                Unit1.Unit1 in 'a/b.pas'
            {$ifdef}
              , Unit2.Unit2 in 'x/y.pas'
            {$endif}
              , Unit3.Unit3 in 'w/x.pas'
              ;"
        });
    }

    #[test]
    fn newline_before_uses() {
        run_test(
            indoc! {"
                Foo;uses
                    Unit1
                  , Unit2
                  , Unit3
                  ;"
            },
            indoc! {"
                Foo;
                uses
                    Unit1
                  , Unit2
                  , Unit3
                  ;"
            },
        );
        run_match_test(indoc! {"
            Foo;
            uses
                Unit1
              , Unit2
              , Unit3
              ;"
        });
        run_match_test(indoc! {"
            Foo;

            uses
                Unit1
              , Unit2
              , Unit3
              ;"
        });
        run_match_test(indoc! {"
            Foo;


            uses
                Unit1
              , Unit2
              , Unit3
              ;"
        });
    }
}
