use crate::{
    lang::{ChevronKind, KeywordKind, OperatorKind, Token, TokenData, TokenType},
    traits::TokenConsolidator,
};

struct TypeParamState {
    open_idx: usize,
    brack_count: u32,
}

pub struct DistinguishGenericTypeParamsConsolidator;
impl TokenConsolidator for DistinguishGenericTypeParamsConsolidator {
    fn consolidate(&self, tokens: &mut [Token]) {
        let mut token_idx = 0;
        let mut state: Vec<TypeParamState> = vec![];
        while token_idx < tokens.len() {
            if !matches!(
                tokens.get(token_idx).map(TokenData::get_token_type),
                Some(TokenType::Op(OperatorKind::LessThan(_)))
            ) {
                token_idx += 1;
                continue;
            }

            let mut next_idx = token_idx + 1;
            let mut comma_found = false;
            let mut prev_was_string = false;
            let mut brack_count = 0;

            state.clear();
            state.push(TypeParamState {
                open_idx: token_idx,
                brack_count: 0,
            });
            while !state.is_empty() {
                let token_type = tokens.get(next_idx).map(TokenData::get_token_type);
                match token_type {
                    Some(TokenType::Op(OperatorKind::LessThan(_))) => {
                        state.push(TypeParamState {
                            open_idx: next_idx,
                            brack_count,
                        });
                    }
                    Some(TokenType::Op(OperatorKind::Comma)) => {
                        comma_found = true;
                    }
                    // all the other tokens you can include in a generic type parameter list
                    Some(
                        TokenType::Identifier
                        | TokenType::Op(
                            OperatorKind::Dot | OperatorKind::Colon | OperatorKind::Semicolon,
                        )
                        | TokenType::CompilerDirective
                        | TokenType::Comment(_)
                        | TokenType::ConditionalDirective(_)
                        | TokenType::Keyword(
                            KeywordKind::Class
                            | KeywordKind::Record
                            | KeywordKind::Constructor
                            | KeywordKind::String
                            | KeywordKind::Array
                            | KeywordKind::Set
                            | KeywordKind::Of,
                        ),
                    ) => {}
                    Some(TokenType::Op(OperatorKind::GreaterThan(_))) => {
                        if comma_found {
                            if let Some(
                                TokenType::Identifier
                                | TokenType::Op(OperatorKind::AddressOf)
                                | TokenType::Keyword(KeywordKind::Not),
                            ) = tokens.get(next_idx + 1).map(TokenData::get_token_type)
                            {
                                // cases where it cannot be generics
                                //   Foo(X < Y, U > V)
                                //   Foo(X < Y, U > @V)
                                //   Foo(X < Y, U > not V)

                                // cases where it is ambiguous but we prefer to treat it as generics
                                //   Foo(X < Y, U > +V)
                                //   Foo(X < Y, U > -V)
                                //   Foo(X < Y, U > (V))
                                //   Foo(X < Y, U > [V])

                                // cases where it must be generics
                                //   Foo(X < Y, U > /V)   (or any other binary operator)
                                break;
                            }
                        }

                        let closed_state = state.pop().unwrap();
                        brack_count = closed_state.brack_count;

                        use ChevronKind as CK;
                        tokens[closed_state.open_idx]
                            .set_token_type(TokenType::Op(OperatorKind::LessThan(CK::Generic)));

                        tokens[next_idx]
                            .set_token_type(TokenType::Op(OperatorKind::GreaterThan(CK::Generic)));
                    }

                    // support for 'short string' types as generic args: `A<string[10]>`
                    Some(TokenType::Op(OperatorKind::LBrack))
                        if prev_was_string || brack_count > 0 =>
                    {
                        brack_count += 1;
                    }
                    Some(TokenType::Op(OperatorKind::RBrack)) if brack_count > 0 => {
                        while let Some(prev) = state.pop() {
                            if prev.brack_count < brack_count {
                                brack_count -= 1;
                                state.push(prev);
                                break;
                            }
                        }
                    }

                    // support for expressions as the short string length: `A<string[1 shl 7]>`
                    Some(
                        TokenType::TextLiteral(_) | TokenType::NumberLiteral(_) | TokenType::Op(_),
                    ) if brack_count > 0 => {}
                    Some(TokenType::Keyword(kk)) if brack_count > 0 && kk.is_numeric_operator() => {
                    }

                    _ => break,
                }

                if token_type.is_some_and(|t| !t.is_comment_or_directive()) {
                    prev_was_string =
                        matches!(token_type, Some(TokenType::Keyword(KeywordKind::String)));
                }

                next_idx += 1;
            }
            token_idx = next_idx;
        }
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use spectral::prelude::*;

    use crate::lang::TokenType as TT;
    use crate::prelude::*;

    fn to_tokens(types: &[TokenType]) -> Vec<Token> {
        types.iter().map(|&t| new_token("", t)).collect()
    }

    fn run_test(tokens: &[TokenType], expected_tokens: &[TokenType]) {
        let consolidator = &DistinguishGenericTypeParamsConsolidator {};
        let mut tokens = to_tokens(tokens);
        consolidator.consolidate(&mut tokens);
        assert_that(&tokens.iter().map(|t| t.get_token_type()).collect_vec()).is_equal_to(
            to_tokens(expected_tokens)
                .iter()
                .map(|t| t.get_token_type())
                .collect_vec(),
        );
    }

    fn run_test_unchanged(tokens: &[TokenType]) {
        run_test(tokens, tokens);
    }

    const ID: TokenType = TokenType::Identifier;

    const LP: TokenType = TT::Op(OperatorKind::LParen);
    const RP: TokenType = TT::Op(OperatorKind::RParen);
    const LB: TokenType = TT::Op(OperatorKind::LBrack);
    const RB: TokenType = TT::Op(OperatorKind::RBrack);
    const SEMI: TokenType = TT::Op(OperatorKind::Semicolon);
    const COL: TokenType = TT::Op(OperatorKind::Colon);
    const COM: TokenType = TT::Op(OperatorKind::Comma);
    const LT: TokenType = TT::Op(OperatorKind::LessThan(ChevronKind::Comp));
    const GT: TokenType = TT::Op(OperatorKind::GreaterThan(ChevronKind::Comp));
    const GE: TokenType = TT::Op(OperatorKind::GreaterEqual);
    const LG: TokenType = TT::Op(OperatorKind::LessThan(ChevronKind::Generic));
    const RG: TokenType = TT::Op(OperatorKind::GreaterThan(ChevronKind::Generic));
    const AND: TokenType = TT::Keyword(KeywordKind::And);
    const DOT: TokenType = TT::Op(OperatorKind::Dot);
    const ADDR: TokenType = TT::Op(OperatorKind::AddressOf);
    const NOT: TokenType = TT::Keyword(KeywordKind::Not);
    const PLUS: TokenType = TT::Op(OperatorKind::Plus);
    const SHL: TokenType = TT::Keyword(KeywordKind::Shl);

    const STRING: TokenType = TokenType::Keyword(KeywordKind::String);
    const SET: TokenType = TokenType::Keyword(KeywordKind::Set);
    const ARRAY: TokenType = TokenType::Keyword(KeywordKind::Array);
    const OF: TokenType = TokenType::Keyword(KeywordKind::Of);
    const CLASS: TokenType = TokenType::Keyword(KeywordKind::Class);
    const REC: TokenType = TokenType::Keyword(KeywordKind::Record);
    const CON: TokenType = TokenType::Keyword(KeywordKind::Constructor);

    const NUM: TokenType = TokenType::NumberLiteral(NumberLiteralKind::Decimal);
    const TEXT: TokenType = TokenType::TextLiteral(TextLiteralKind::SingleLine);
    const MTEXT: TokenType = TokenType::TextLiteral(TextLiteralKind::MultiLine);

    #[test]
    fn non_generics_are_unchanged() {
        run_test_unchanged(&[ID]);
        run_test_unchanged(&[ID, AND, TT::CompilerDirective]);
        run_test_unchanged(&[ID, GT, ID]);
        run_test_unchanged(&[ID, LT, LP, ID, GT, ID, RP]);
        // A < B, C > D
        run_test_unchanged(&[ID, LT, ID, COM, ID, GT, ID]);
    }

    #[test]
    fn multi_element_type_list() {
        // A<B, C>
        run_test(&[ID, LT, ID, COM, ID, GT], &[ID, LG, ID, COM, ID, RG]);
    }

    #[test]
    fn type_list_with_dots() {
        // A<B.C>
        run_test(&[ID, LT, ID, DOT, ID, GT], &[ID, LG, ID, DOT, ID, RG])
    }

    #[test]
    fn nested_type_list() {
        // Foo<Bar<Baz>>
        run_test(&[ID, LT, ID, LT, ID, GT, GT], &[ID, LG, ID, LG, ID, RG, RG]);
    }

    #[test]
    fn nested_multi_element_list() {
        // A1<B1<C1, C2>, B2, B3<C3>>
        run_test(
            &[
                ID, LT, ID, LT, ID, COM, ID, GT, COM, ID, COM, ID, LT, ID, GT, GT,
            ],
            &[
                ID, LG, ID, LG, ID, COM, ID, RG, COM, ID, COM, ID, LG, ID, RG, RG,
            ],
        );
    }

    #[test]
    fn multi_element_type_list_declaration() {
        // A<B: C; D: E>
        run_test(
            &[ID, LT, ID, COL, ID, SEMI, ID, COL, ID, GT],
            &[ID, LG, ID, COL, ID, SEMI, ID, COL, ID, RG],
        );
    }

    #[test]
    fn type_constraints() {
        // A<B: C>
        run_test(&[ID, LT, ID, COL, ID, GT], &[ID, LG, ID, COL, ID, RG]);
        // A<B: class>
        run_test(&[ID, LT, ID, COL, CLASS, GT], &[ID, LG, ID, COL, CLASS, RG]);
        // A<B: record>
        run_test(&[ID, LT, ID, COL, REC, GT], &[ID, LG, ID, COL, REC, RG]);
        // A<B: constructor>
        run_test(&[ID, LT, ID, COL, CON, GT], &[ID, LG, ID, COL, CON, RG]);
    }

    #[test]
    fn string_keyword() {
        // A<String>
        run_test(&[ID, LT, STRING, GT], &[ID, LG, STRING, RG])
    }

    #[test]
    fn composite_types() {
        // A<set of B>
        run_test(&[ID, LT, SET, OF, ID, GT], &[ID, LG, SET, OF, ID, RG]);
        // A<array of B>
        run_test(&[ID, LT, ARRAY, OF, ID, GT], &[ID, LG, ARRAY, OF, ID, RG]);
        // A<array of set of B>
        run_test(
            &[ID, LT, ARRAY, OF, SET, OF, ID, GT],
            &[ID, LG, ARRAY, OF, SET, OF, ID, RG],
        );
        // A<string[10]>
        run_test(
            &[ID, LT, STRING, LB, NUM, RB, GT],
            &[ID, LG, STRING, LB, NUM, RB, RG],
        );
        // A<string[10 + B<string[1 shl 2]>]>
        run_test(
            &[
                ID, LT, STRING, LB, NUM, PLUS, ID, LT, STRING, LB, NUM, SHL, NUM, RB, GT, RB, GT,
            ],
            &[
                ID, LG, STRING, LB, NUM, PLUS, ID, LG, STRING, LB, NUM, SHL, NUM, RB, RG, RB, RG,
            ],
        );
        // A<string[SizeOf('a' + '''\n a \n''')]>
        run_test(
            &[ID, LT, STRING, LB, ID, LP, TEXT, PLUS, MTEXT, RP, RB, GT],
            &[ID, LG, STRING, LB, ID, LP, TEXT, PLUS, MTEXT, RP, RB, RG],
        );
    }

    #[test]
    fn followed_by_ident() {
        // A<B> C
        run_test(&[ID, LT, ID, GT, ID], &[ID, LG, ID, RG, ID]);

        // A<B, B2> C
        run_test_unchanged(&[ID, LT, ID, COM, ID, GT, ID]);

        // A(B < C, D > E)
        run_test_unchanged(&[ID, LP, ID, LT, ID, COM, ID, GT, ID, RP]);
    }

    #[test]
    fn followed_by_address_op() {
        // A<B> @
        run_test(&[ID, LG, ID, RG, ADDR], &[ID, LG, ID, RG, ADDR]);

        // A<B, C> @
        run_test_unchanged(&[ID, LT, ID, COM, ID, GT, ADDR]);

        // Foo(A<B, C> @B)
        run_test_unchanged(&[ID, LP, ID, LT, ID, COM, ID, GT, ADDR, ID, RP]);
    }

    #[test]
    fn followed_by_not_op() {
        // A<B> not C
        run_test(&[ID, LT, ID, GT, NOT, ID], &[ID, LG, ID, RG, NOT, ID]);

        // A<B, C> not D
        run_test_unchanged(&[ID, LT, ID, COM, ID, GT, NOT, ID]);

        // Foo(A < B, C > not D)
        run_test_unchanged(&[ID, LP, ID, LT, ID, COM, ID, GT, NOT, ID, RP]);
    }

    #[test]
    fn followed_by_maybe_unary_op() {
        // A<B> + C
        run_test(&[ID, LT, ID, GT, PLUS, ID], &[ID, LG, ID, RG, PLUS, ID]);

        // A<B, C> + D
        run_test(
            &[ID, LT, ID, COM, ID, GT, PLUS, ID],
            &[ID, LG, ID, COM, ID, RG, PLUS, ID],
        );

        // Foo(A < B, C > + D)
        run_test(
            &[ID, LP, ID, LT, ID, COM, ID, GT, PLUS, ID, RP],
            &[ID, LP, ID, LG, ID, COM, ID, RG, PLUS, ID, RP],
        );
    }

    #[test]
    fn followed_by_parens() {
        // A<B> (C)
        run_test(&[ID, LT, ID, GT, LP, ID, RP], &[ID, LG, ID, RG, LP, ID, RP]);
        // A<B> [C]
        run_test(&[ID, LT, ID, GT, LB, ID, RB], &[ID, LG, ID, RG, LB, ID, RB]);

        // A<B, C> (D)
        run_test(
            &[ID, LT, ID, COM, ID, GT, LP, ID, RP],
            &[ID, LG, ID, COM, ID, RG, LP, ID, RP],
        );
        // A<B, C> [D]
        run_test(
            &[ID, LT, ID, COM, ID, GT, LB, ID, RB],
            &[ID, LG, ID, COM, ID, RG, LB, ID, RB],
        );

        // ambiguous, but treated like generics
        // Foo(A < B, C > (D))
        run_test(
            &[ID, LP, ID, LT, ID, COM, ID, GT, LP, ID, RP, RP],
            &[ID, LP, ID, LG, ID, COM, ID, RG, LP, ID, RP, RP],
        );
        // Foo(A < B, C > [D])
        run_test(
            &[ID, LP, ID, LT, ID, COM, ID, GT, LB, ID, RB, RP],
            &[ID, LP, ID, LG, ID, COM, ID, RG, LB, ID, RB, RP],
        );
    }

    #[test]
    fn ambiguous_greater_equal() {
        // in these cases we could do better

        // A<B>= C
        run_test_unchanged(&[ID, LT, ID, GE, ID]);
        // A<B>= class
        run_test_unchanged(&[ID, LT, ID, GE, CLASS]);
        // Foo(A<B>=C)
        run_test_unchanged(&[ID, LP, ID, LT, ID, GE, ID, RP]);
        // Foo(A<B, C>=D)
        run_test_unchanged(&[ID, LP, ID, LT, ID, COM, ID, GE, ID, RP]);
    }
}
