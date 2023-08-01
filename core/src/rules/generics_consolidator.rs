use crate::{
    lang::{OperatorKind, PureKeywordKind, Token, TokenType},
    traits::TokenConsolidator,
};

pub struct DistinguishGenericTypeParamsConsolidator;
impl TokenConsolidator for DistinguishGenericTypeParamsConsolidator {
    fn consolidate<'a>(&self, mut tokens: Vec<Token<'a>>) -> Vec<Token<'a>> {
        let mut token_idx = 0;
        let mut opening_idxs = vec![];
        while token_idx < tokens.len() {
            if !matches!(
                tokens.get(token_idx).map(Token::get_token_type),
                Some(TokenType::Op(OperatorKind::LessThan))
            ) {
                token_idx += 1;
                continue;
            }

            let mut next_idx = token_idx + 1;
            let mut comma_found = false;
            opening_idxs.clear();
            opening_idxs.push(token_idx);
            while !opening_idxs.is_empty() {
                match tokens.get(next_idx).map(Token::get_token_type) {
                    Some(TokenType::Op(OperatorKind::LessThan)) => {
                        opening_idxs.push(next_idx);
                    }
                    Some(TokenType::Op(OperatorKind::Comma)) => {
                        comma_found = true;
                    }
                    // all the other tokens you can include in a generic type parameter list
                    Some(
                        TokenType::Identifier
                        | TokenType::IdentifierOrKeyword(_)
                        | TokenType::Op(OperatorKind::Colon)
                        | TokenType::CompilerDirective
                        | TokenType::Comment(_)
                        | TokenType::ConditionalDirective(_)
                        | TokenType::Keyword(
                            PureKeywordKind::Class
                            | PureKeywordKind::Record
                            | PureKeywordKind::Constructor,
                        ),
                    ) => {}
                    Some(TokenType::Op(OperatorKind::GreaterThan)) => {
                        if comma_found {
                            if let Some(
                                TokenType::Identifier
                                | TokenType::IdentifierOrKeyword(_)
                                | TokenType::Op(OperatorKind::AddressOf),
                            ) = tokens.get(next_idx + 1).map(Token::get_token_type)
                            {
                                // cases where it could still be a comparison:
                                //   Foo(X < Y, U > V)
                                //   Foo(X < Y, U > +V)   (or any other operator that isn't a left bracket)
                                //   Foo(X < Y, U > @V)
                                // In some other cases it's still ambiguous but we would prefer to treat it like generics
                                break;
                            }
                        }

                        let old_idx = opening_idxs.pop().unwrap();
                        tokens[old_idx].set_token_type(TokenType::Op(OperatorKind::LGeneric));

                        tokens[next_idx].set_token_type(TokenType::Op(OperatorKind::RGeneric));
                    }
                    _ => break,
                }
                next_idx += 1;
            }
            token_idx = next_idx;
        }
        tokens
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use spectral::prelude::*;

    use crate::lang::TokenType::*;
    use crate::prelude::*;

    fn to_tokens(types: &[TokenType]) -> Vec<Token> {
        types
            .iter()
            .enumerate()
            .map(|(i, &t)| Token::RefToken(RefToken::new(i, "", "", t)))
            .collect()
    }

    fn run_test(tokens: &[TokenType], expected_tokens: &[TokenType]) {
        let consolidator = &DistinguishGenericTypeParamsConsolidator {};
        assert_that(
            &consolidator
                .consolidate(to_tokens(tokens))
                .iter()
                .map(|t| t.get_token_type())
                .collect_vec(),
        )
        .is_equal_to(
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
    const LP: TokenType = Op(OperatorKind::LParen);
    const RP: TokenType = Op(OperatorKind::RParen);
    const SEMI: TokenType = Op(OperatorKind::Semicolon);
    const COL: TokenType = Op(OperatorKind::Colon);
    const COM: TokenType = Op(OperatorKind::Comma);
    const LT: TokenType = Op(OperatorKind::LessThan);
    const GT: TokenType = Op(OperatorKind::GreaterThan);
    const GE: TokenType = Op(OperatorKind::GreaterEqual);
    const LG: TokenType = Op(OperatorKind::LGeneric);
    const RG: TokenType = Op(OperatorKind::RGeneric);
    const AND: TokenType = Op(OperatorKind::And);

    const CLASS: TokenType = TokenType::Keyword(PureKeywordKind::Class);

    #[test]
    fn non_generics_are_unchanged() {
        run_test_unchanged(&[ID]);
        run_test_unchanged(&[ID, AND, CompilerDirective]);
        run_test_unchanged(&[ID, GT, ID]);
        run_test_unchanged(&[ID, LT, LP, ID, GT, ID, RP]);
        run_test_unchanged(&[ID, LT, SEMI, ID, GT, ID]);
    }

    #[test]
    fn multi_element_type_list() {
        // A<B, C>
        run_test(&[ID, LT, ID, COM, ID, GT], &[ID, LG, ID, COM, ID, RG]);
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
    fn type_constraints() {
        const REC: TokenType = TokenType::Keyword(PureKeywordKind::Record);
        const CON: TokenType = TokenType::Keyword(PureKeywordKind::Constructor);

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
    fn followed_by_ident() {
        const IDKW: TokenType = TokenType::IdentifierOrKeyword(IdentifierOrKeywordKind::SafeCall);

        // A<B> C
        run_test(&[ID, LT, ID, GT, ID], &[ID, LG, ID, RG, ID]);
        run_test(&[ID, LT, ID, GT, IDKW], &[ID, LG, ID, RG, IDKW]);

        // A<B, B2> C
        run_test_unchanged(&[ID, LT, ID, COM, ID, GT, ID]);
        run_test_unchanged(&[ID, LT, ID, COM, ID, GT, IDKW]);
    }

    #[test]
    fn followed_by_address_op() {
        const ADDR: TokenType = TokenType::Op(OperatorKind::AddressOf);
        // A<B> @
        run_test(&[ID, LG, ID, RG, ADDR], &[ID, LG, ID, RG, ADDR]);

        // A<B, C> @
        run_test_unchanged(&[ID, LT, ID, COM, ID, GT, ADDR]);
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
