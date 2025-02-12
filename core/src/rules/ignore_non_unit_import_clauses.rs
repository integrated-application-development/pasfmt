use itertools::Itertools;

use crate::prelude::*;

use crate::prelude::KeywordKind::*;

pub struct IgnoreNonUnitImportClauses;
impl TokenIgnorer for IgnoreNonUnitImportClauses {
    fn ignore_tokens(
        &self,
        (tokens, lines): (&[Token], &[LogicalLine]),
        token_marker: &mut TokenMarker,
    ) {
        if let Some(TokenType::Keyword(Program | Library | Package)) = tokens
            .iter()
            .find(|token| {
                !matches!(
                    token.get_token_type(),
                    TokenType::Comment(_)
                        | TokenType::CompilerDirective
                        | TokenType::ConditionalDirective(_)
                )
            })
            .map(Token::get_token_type)
        {
            lines
                .iter()
                .filter(|line| line.get_line_type() == LogicalLineType::ImportClause)
                .for_each(|line| self.ignore_clause_line(line, token_marker));
        };
    }
}
impl IgnoreNonUnitImportClauses {
    fn ignore_clause_line(&self, line: &LogicalLine, token_marker: &mut TokenMarker) {
        let Some((&start, &end)) = line.get_tokens().iter().minmax().into_option() else {
            return;
        };
        for token in start..=end {
            token_marker.mark(token);
        }
    }
}

#[cfg(test)]
mod tests {
    use paste::paste;
    use spectral::prelude::*;

    use crate::prelude::*;
    use crate::test_utils::formatter_test_group;

    struct AddSpaceAfterComma {}
    impl LogicalLineFileFormatter for AddSpaceAfterComma {
        fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, _input: &[LogicalLine]) {
            for token_index in 0..formatted_tokens.get_tokens().len() {
                if let Some(formatting_data) = formatted_tokens
                    .get_token_type_for_index(token_index)
                    .filter(|token_type| token_type == &TokenType::Op(OperatorKind::Comma))
                    .and_then(|_| formatted_tokens.get_formatting_data_mut(token_index + 1))
                {
                    formatting_data.spaces_before += 1;
                }
            }
        }
    }

    fn formatter() -> Formatter {
        Formatter::builder()
            .lexer(DelphiLexer {})
            .parser(DelphiLogicalLineParser {})
            .token_ignorer(IgnoreNonUnitImportClauses {})
            .file_formatter(AddSpaceAfterComma {})
            .reconstructor(default_test_reconstructor())
            .build()
    }

    macro_rules! ignored_file_type_tests {
        ($typ:ident) => {
            paste! {
                formatter_test_group!(
                    [<ignored_uses_clauses_ $typ>],
                    nothing_before = {
                        indoc::concatdoc!(
                            stringify!($typ),
                            "
                             foo;
                            uses a,b,c;
                            (a,b,c);
                            "
                        ),
                        indoc::concatdoc!(
                            stringify!($typ),
                            "
                             foo;
                            uses a,b,c;
                            (a, b, c);
                            "
                        )
                    },
                    comments_and_directives_before = {
                        indoc::concatdoc!(
                            "
                            {}//
                            {$R}
                            {$ifdef X}
                            ",
                            stringify!($typ),
                            "
                             foo;
                            {$endif}
                            uses a,b,c;
                            (a,b,c);
                            "
                        ),
                        indoc::concatdoc!(
                            "
                            {}//
                            {$R}
                            {$ifdef X}
                            ",
                            stringify!($typ),
                            "
                             foo;
                            {$endif}
                            uses a,b,c;
                            (a, b, c);
                            "
                        ),
                    },
                    two_imports = {
                        indoc::concatdoc!(
                            stringify!($typ),
                            "
                             foo;
                            uses a,b,c;
                            (a,b,c);
                            uses a,b,c;
                            "
                        ),
                        indoc::concatdoc!(
                            stringify!($typ),
                            "
                             foo;
                            uses a,b,c;
                            (a, b, c);
                            uses a,b,c;
                            "
                        )
                    },
                    containing_conditional_directives = {
                        indoc::concatdoc!(
                            stringify!($typ),
                            "
                             foo;
                            uses a, {$ifdef A},  b  {$endif};
                            (a,b,c);
                            "
                        ),
                        indoc::concatdoc!(
                            stringify!($typ),
                            "
                             foo;
                            uses a, {$ifdef A},  b  {$endif};
                            (a, b, c);
                            "
                        )
                    },
                );
            }
        };
    }

    ignored_file_type_tests!(program);

    /*
        Libraries and packages don't technically have 'uses' clauses, but they do have other kinds
        of import clauses that we intend on parsing properly at a later date. For now, we can pretend
        that they can contain uses clauses.
    */
    ignored_file_type_tests!(library);
    ignored_file_type_tests!(package);

    formatter_test_group!(
        not_ignored_uses_clauses,
        nothing_before_unit = {
            indoc::indoc! {"
                unit foo;
                uses a,b,c;
                (a,b,c);
            "},
            indoc::indoc! {"
                unit foo;
                uses a, b, c;
                (a, b, c);
            "}
        },
        comments_and_directives_before_unit = {
            indoc::indoc! {"
                {}//
                {$R}
                {$ifdef X}
                unit foo;
                uses a,b,c;
                (a,b,c);
            "},
            indoc::indoc! {"
                {}//
                {$R}
                {$ifdef X}
                unit foo;
                uses a, b, c;
                (a, b, c);
            "}
        },
        two_imports = {
            indoc::indoc! {"
                unit foo;
                uses a,b,c;
                (a,b,c);
                uses a,b,c;
            "},
            indoc::indoc! {"
                unit foo;
                uses a, b, c;
                (a, b, c);
                uses a, b, c;
            "}
        }
    );
}
