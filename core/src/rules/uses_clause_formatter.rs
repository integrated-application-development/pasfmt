use std::cmp::max;

use crate::{lang::*, traits::LogicalLineFormatter};

pub struct UsesClauseFormatter {}
impl LogicalLineFormatter for UsesClauseFormatter {
    fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &LogicalLine) {
        let mut conditional_depth: isize = 0;
        for &token_index in input.get_tokens() {
            {
                // Reset formatting for the token
                if let Some(token_formatting_data) =
                    formatted_tokens.get_formatting_data_mut(token_index)
                {
                    token_formatting_data.spaces_before = 0;
                }
            }
            if !matches!(
                formatted_tokens.get_token_type_for_index(token_index),
                Some(
                    TokenType::Keyword(KeywordKind::Uses)
                        | TokenType::Comment(
                            CommentKind::IndividualLine
                                | CommentKind::IndividualBlock
                                | CommentKind::MultilineBlock
                        )
                )
            ) {
                // Reset newlines for tokens other than uses
                if let Some(formatting_data) = formatted_tokens.get_formatting_data_mut(token_index)
                {
                    formatting_data.newlines_before = 0;
                }
            }

            if matches!(
                formatted_tokens.get_token_type_for_index(token_index),
                Some(TokenType::Keyword(KeywordKind::Uses))
            ) {
                if let Some(token_formatting_data) =
                    formatted_tokens.get_formatting_data_mut(token_index)
                {
                    token_formatting_data.spaces_before = 0;
                    if token_index > 0 {
                        token_formatting_data.newlines_before =
                            max(token_formatting_data.newlines_before, 1);
                    }
                }
            }

            if matches!(
                formatted_tokens.get_token_type_for_index(token_index),
                Some(TokenType::ConditionalDirective(_) | TokenType::CompilerDirective)
            ) {
                conditional_depth += match formatted_tokens.get_token_type_for_index(token_index) {
                    Some(TokenType::ConditionalDirective(
                        ConditionalDirectiveKind::Endif
                        | ConditionalDirectiveKind::Else
                        | ConditionalDirectiveKind::Ifend,
                    )) => -1,
                    _ => 0,
                };

                if let Some(token_formatting_data) =
                    formatted_tokens.get_formatting_data_mut(token_index)
                {
                    token_formatting_data.newlines_before = 1;
                    token_formatting_data.spaces_before = 0;
                    token_formatting_data.indentations_before = conditional_depth.unsigned_abs();
                }

                conditional_depth += match formatted_tokens.get_token_type_for_index(token_index) {
                    Some(TokenType::ConditionalDirective(
                        ConditionalDirectiveKind::If
                        | ConditionalDirectiveKind::Else
                        | ConditionalDirectiveKind::Ifdef
                        | ConditionalDirectiveKind::Ifopt
                        | ConditionalDirectiveKind::Ifndef
                        | ConditionalDirectiveKind::Elseif,
                    )) => 1,
                    _ => 0,
                }
            } else if matches!(
                formatted_tokens.get_token_type_for_index(token_index),
                Some(
                    TokenType::Keyword(KeywordKind::In(InKind::Import))
                        | TokenType::TextLiteral(_)
                        | TokenType::Comment(_)
                )
            ) || token_index > 1
                && matches!(
                    formatted_tokens.get_token_type_for_index(token_index - 1),
                    Some(TokenType::Op(OperatorKind::Comma))
                )
            {
                if let Some(token_formatting_data) =
                    formatted_tokens.get_formatting_data_mut(token_index)
                {
                    token_formatting_data.spaces_before = 1;
                }
            }

            if matches!(
                formatted_tokens.get_token_type_for_index(token_index),
                Some(
                    TokenType::Op(OperatorKind::Comma | OperatorKind::Semicolon)
                        | TokenType::Comment(
                            CommentKind::IndividualLine
                                | CommentKind::IndividualBlock
                                | CommentKind::MultilineBlock
                        )
                )
            ) {
                if let Some(token_formatting_data) =
                    formatted_tokens.get_formatting_data_mut(token_index)
                {
                    token_formatting_data.spaces_before = 2;
                    token_formatting_data.newlines_before = 1;
                }
            }

            if matches!(
                formatted_tokens.get_token_type_for_index(token_index),
                Some(TokenType::Identifier)
            ) && token_index > 0
                && matches!(
                    formatted_tokens.get_token_type_for_index(token_index - 1),
                    Some(
                        TokenType::Keyword(KeywordKind::Uses)
                            | TokenType::ConditionalDirective(_)
                            | TokenType::CompilerDirective
                            | TokenType::Comment(
                                CommentKind::IndividualLine | CommentKind::InlineLine
                            )
                    )
                )
            {
                if let Some(token_formatting_data) =
                    formatted_tokens.get_formatting_data_mut(token_index)
                {
                    token_formatting_data.spaces_before = 4;
                    token_formatting_data.newlines_before = 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use indoc::indoc;
    use spectral::prelude::*;

    fn run_test(input: &'static str, expected_output: &'static str) {
        let uses_line_consolidator = ImportClauseConsolidator {};
        let uses_formatter = &UsesClauseFormatter {};
        let formatter = Formatter::builder()
            .lexer(DelphiLexer {})
            .parser(DelphiLogicalLineParser {})
            .lines_consolidator(uses_line_consolidator)
            .line_formatter(FormatterSelector::new(|line_type| match line_type {
                LogicalLineType::ImportClause => Some(uses_formatter),
                _ => None,
            }))
            .reconstructor(default_test_reconstructor())
            .build();

        let formatted_output = formatter.format(input);
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

    #[test]
    fn region_in_uses() {
        run_match_test(indoc! {"
            uses
            {$REGION 'a'}
                Unit1
              , Unit2
              , Unit3
            {$ENDREGION}
              ;"
        });
        run_match_test(indoc! {"
            uses
                Unit1
            {$REGION 'a'}
              , Unit2
            {$ENDREGION}
              , Unit3
              ;"
        });
    }
    #[test]
    fn region_partially_in_uses() {
        run_match_test(indoc! {"
            {$REGION 'a'}
            uses
                Unit1
              , Unit2
              , Unit3
            {$ENDREGION}
              ;"
        });
        run_match_test(indoc! {"
            uses
            {$REGION 'a'}
                Unit1
              , Unit2
              , Unit3
              ;
            {$ENDREGION}"
        });
        run_match_test(indoc! {"
            uses
                Unit1
            {$REGION 'a'}
              , Unit2
              , Unit3
              ;
            {$ENDREGION}"
        });
    }
    #[test]
    fn region_outside() {
        run_match_test(indoc! {"
            {$REGION 'a'}
            uses
                Unit1
              , Unit2
              , Unit3
              ;
            {$ENDREGION}"
        });
    }
}
