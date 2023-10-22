use crate::{lang::*, traits::LogicalLinesConsolidator};

pub struct ImportClauseConsolidator {}
impl LogicalLinesConsolidator for ImportClauseConsolidator {
    fn consolidate(&self, (tokens, lines): (&mut [Token<'_>], &mut [LogicalLine])) {
        let uses_token_indices: Vec<usize> = tokens
            .iter()
            .enumerate()
            .filter_map(|(i, token)| match token.get_token_type() {
                TokenType::Keyword(KeywordKind::Uses) => Some(i),
                _ => None,
            })
            .collect();

        for &uses_token_index in uses_token_indices.iter().rev() {
            let (uses_line_index, uses_line_ending_token_index) = lines
                .iter()
                .enumerate()
                .filter_map(
                    |(index, line)| match line.get_tokens().contains(&uses_token_index) {
                        true => line.get_tokens().last().map(|token| (index, token)),
                        false => None,
                    },
                )
                .max_by(|a, b| a.1.cmp(b.1))
                .unwrap();

            let mut uses_line_ending_token_index = *uses_line_ending_token_index;

            if tokens[uses_token_index..=uses_line_ending_token_index]
                .iter()
                .any(|token| {
                    matches!(
                        token.get_token_type(),
                        TokenType::ConditionalDirective(ConditionalDirectiveKind::Ifdef)
                    )
                })
                && lines.iter().any(|line| {
                    let next_token_index = uses_line_ending_token_index + 1;
                    line.get_tokens().contains(&next_token_index)
                        && matches!(line.get_line_type(), LogicalLineType::ConditionalDirective)
                        && matches!(
                            line.get_tokens().first().map(|token_index| tokens
                                .get(*token_index)
                                .unwrap()
                                .get_token_type()),
                            Some(TokenType::ConditionalDirective(
                                ConditionalDirectiveKind::Endif
                            ))
                        )
                })
            {
                uses_line_ending_token_index += 1;
            }

            let new_token_line_index_and_token_index: Vec<(usize, usize)> = (uses_token_index
                ..=uses_line_ending_token_index)
                .filter_map(|token_index| {
                    match lines
                        .get(uses_line_index)
                        .unwrap()
                        .get_tokens()
                        .contains(&token_index)
                    {
                        true => None,
                        false => lines
                            .iter()
                            .enumerate()
                            .find(|(_, line)| line.get_tokens().contains(&token_index))
                            .map(|(line_index, _)| (line_index, token_index)),
                    }
                })
                .collect();

            let (mut new_token_line_indices, new_token_indices): (Vec<usize>, Vec<usize>) =
                new_token_line_index_and_token_index.into_iter().unzip();

            new_token_line_indices.sort();
            new_token_line_indices.dedup();

            lines
                .get_mut(uses_line_index)
                .unwrap()
                .get_tokens_mut()
                .extend(new_token_indices);
            lines
                .get_mut(uses_line_index)
                .unwrap()
                .get_tokens_mut()
                .sort();
            new_token_line_indices
                .iter()
                .rev()
                .for_each(|conditional_line_index| {
                    if let Some(logical_line) = lines.get_mut(*conditional_line_index) {
                        logical_line.void_and_drain();
                    }
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use spectral::prelude::*;

    use super::*;
    use crate::prelude::*;
    use indoc::indoc;

    fn run_test(input: &str, expected_lines: Vec<LogicalLine>) {
        println!("input:\n{}", input);

        let lexer = DelphiLexer {};
        let parser = DelphiLogicalLineParser {};
        let consolidator = ImportClauseConsolidator {};

        let tokens = lexer.lex(input);
        let (mut lines, mut tokens) = parser.parse(tokens);
        consolidator.consolidate((&mut tokens, &mut lines));

        let lines = lines
            .iter()
            .filter(|line| {
                !matches!(
                    line.get_line_type(),
                    LogicalLineType::Eof | LogicalLineType::Voided
                )
            })
            .collect_vec();

        assert_that(&lines).has_length(expected_lines.len());
        expected_lines.iter().for_each(|expected_line| {
            assert_that(&lines).contains(expected_line);
        });
    }

    #[test]
    fn uses_clause_with_ifdef_after() {
        run_test(
            indoc! {"
                uses
                    Unit1
                  , Unit2
                  , Unit3
                  ;
                {$ifdef A}
                {$endif}"
            },
            vec![
                LogicalLine::new(
                    None,
                    0,
                    vec![0, 1, 2, 3, 4, 5, 6],
                    LogicalLineType::ImportClause,
                ),
                LogicalLine::new(None, 0, vec![7], LogicalLineType::ConditionalDirective),
                LogicalLine::new(None, 0, vec![8], LogicalLineType::ConditionalDirective),
            ],
        );
    }

    #[test]
    fn uses_clause_with_ifelsedef_around_semicolon() {
        run_test(
            indoc! {"
                uses
                    Unit1
                {$ifdef A}
                  , Unit2
                  ;
                {$else}
                  , Unit3
                  ;
                {$endif}"
            },
            vec![LogicalLine::new(
                None,
                0,
                vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
                LogicalLineType::ImportClause,
            )],
        );
    }

    #[test]
    fn uses_clause_with_central_ifelsedef() {
        run_test(
            indoc! {"
                uses
                    Unit1
                {$ifdef A}
                  , Unit2
                {$else}
                  , Unit3
                {$endif}
                  , Unit4
                  ;"
            },
            vec![LogicalLine::new(
                None,
                0,
                vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
                LogicalLineType::ImportClause,
            )],
        );
    }

    #[test]
    fn uses_clause_with_central_ifdef() {
        run_test(
            indoc! {"
                uses
                    Unit1
                {$ifdef}
                  , Unit2
                {$endif}
                  , Unit3
                  ;"
            },
            vec![LogicalLine::new(
                None,
                0,
                vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
                LogicalLineType::ImportClause,
            )],
        );
    }

    #[test]
    fn uses_clause_with_start_and_end_ifdef() {
        run_test(
            indoc! {"
                uses
                {$ifdef}
                    Unit1
                  , Unit2
                  , Unit3
                {$endif}
                  ;"
            },
            vec![LogicalLine::new(
                None,
                0,
                vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
                LogicalLineType::ImportClause,
            )],
        );
    }

    #[test]
    fn uses_clause_with_ifdef_outside() {
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
            vec![
                LogicalLine::new(None, 0, vec![0], LogicalLineType::ConditionalDirective),
                LogicalLine::new(
                    None,
                    0,
                    vec![1, 2, 3, 4, 5, 6, 7],
                    LogicalLineType::ImportClause,
                ),
                LogicalLine::new(None, 0, vec![8], LogicalLineType::ConditionalDirective),
            ],
        );
    }
}
