use crate::lang::KeywordKind::*;
use crate::lang::TokenType::*;
use crate::prelude::*;

const PROPERTY_DECLARATION_KEYWORDS: [KeywordKind; 10] = [
    Default, DispId, Implements, Index, NoDefault, Read, ReadOnly, Stored, Write, WriteOnly,
];

/*
    The purpose of this consolidator is to reassign the token type of the
    identifiers that may be keywords in the property declaration context to be
    keywords. While this is not always correct it is good enough for our purposes
    without actual language parsing.

    e.g.
    property Index;
    property Foo: Integer Index 0 read Bar;

    Index will be reassigned to be a keyword in both cases despite it being an
    identifier in the first case.
*/

pub struct PropertyDeclarationConsolidator {}
impl LogicalLinesConsolidator for PropertyDeclarationConsolidator {
    fn consolidate<'a>(&self, input: LogicalLines<'a>) -> LogicalLines<'a> {
        let (mut tokens, mut lines) = input.into();
        lines
            .iter_mut()
            .filter(|line| line.get_line_type() == LogicalLineType::PropertyDeclaration)
            .flat_map(|line| line.get_tokens())
            .for_each(|&token_index| {
                let token = match tokens.get_mut(token_index) {
                    Some(token) => token,
                    _ => return,
                };

                if let IdentifierOrKeyword(keyword_kind) = token.get_token_type() {
                    if PROPERTY_DECLARATION_KEYWORDS.contains(&keyword_kind) {
                        token.set_token_type(Keyword(keyword_kind));
                    };
                }
            });
        LogicalLines::new(tokens, lines)
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use spectral::prelude::*;

    use crate::prelude::*;

    fn to_tokens(types: &[TokenType]) -> Vec<Token> {
        types
            .iter()
            .enumerate()
            .map(|(i, &t)| Token::RefToken(RefToken::new(i, "", "", t)))
            .collect()
    }

    fn to_logical_lines(tokens: Vec<Token>) -> LogicalLines {
        let lines = vec![LogicalLine::new(
            None,
            0,
            tokens.iter().map(|token| token.get_index()).collect(),
            LogicalLineType::PropertyDeclaration,
        )];
        LogicalLines::new(tokens, lines)
    }

    fn run_test(tokens: &[TokenType], expected_tokens: &[TokenType]) {
        let consolidator = &PropertyDeclarationConsolidator {};
        assert_that(
            &consolidator
                .consolidate(to_logical_lines(to_tokens(tokens)))
                .get_tokens()
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

    const PROP: TokenType = TokenType::Keyword(KeywordKind::Property);
    const ID: TokenType = TokenType::Identifier;
    const COLON: TokenType = TokenType::Op(OperatorKind::Colon);
    const READ_I: TokenType = TokenType::IdentifierOrKeyword(KeywordKind::Read);
    const READ_P: TokenType = TokenType::Keyword(KeywordKind::Read);
    const WRITE_I: TokenType = TokenType::IdentifierOrKeyword(KeywordKind::Write);
    const WRITE_P: TokenType = TokenType::Keyword(KeywordKind::Write);
    const DEFAULT_I: TokenType = TokenType::IdentifierOrKeyword(KeywordKind::Default);
    const DEFAULT_P: TokenType = TokenType::Keyword(KeywordKind::Default);
    const SEMI: TokenType = TokenType::Op(OperatorKind::Semicolon);

    #[test]
    fn no_change_to_types() {
        run_test_unchanged(&[PROP, SEMI]);
        run_test_unchanged(&[PROP, ID, SEMI]);
        run_test_unchanged(&[PROP, ID, COLON, ID, SEMI]);
    }

    #[test]
    fn lone_read_specifier() {
        // property Foo: TBar read FFoo;
        run_test(
            &[PROP, ID, COLON, ID, READ_I, ID, SEMI],
            &[PROP, ID, COLON, ID, READ_P, ID, SEMI],
        );
    }

    #[test]
    fn lone_read_specifier_with_default() {
        // property Foo: TBar read FFoo default A;
        run_test(
            &[PROP, ID, COLON, ID, READ_I, ID, DEFAULT_I, ID, SEMI],
            &[PROP, ID, COLON, ID, READ_P, ID, DEFAULT_P, ID, SEMI],
        );
    }

    #[test]
    fn lone_write_specifier() {
        // property Foo: TBar write FFoo;
        run_test(
            &[PROP, ID, COLON, ID, WRITE_I, ID, SEMI],
            &[PROP, ID, COLON, ID, WRITE_P, ID, SEMI],
        );
    }

    #[test]
    fn lone_write_specifier_with_default() {
        // property Foo: TBar write FFoo default A;
        run_test(
            &[PROP, ID, COLON, ID, WRITE_I, ID, DEFAULT_I, ID, SEMI],
            &[PROP, ID, COLON, ID, WRITE_P, ID, DEFAULT_P, ID, SEMI],
        );
    }

    #[test]
    fn read_write_specifiers() {
        // property Foo: TBar read FFoo write FFoo;
        run_test(
            &[PROP, ID, COLON, ID, READ_I, ID, WRITE_I, ID, SEMI],
            &[PROP, ID, COLON, ID, READ_P, ID, WRITE_P, ID, SEMI],
        );
    }

    #[test]
    fn read_write_specifiers_with_default() {
        // property Foo: TBar read FFoo write FFoo default A;
        run_test(
            &[
                PROP, ID, COLON, ID, READ_I, ID, WRITE_I, ID, DEFAULT_I, ID, SEMI,
            ],
            &[
                PROP, ID, COLON, ID, READ_P, ID, WRITE_P, ID, DEFAULT_P, ID, SEMI,
            ],
        );
    }

    #[test]
    fn all_property_specifiers() {
        const CLASS: TokenType = TokenType::Keyword(KeywordKind::Class);
        const DISPID_I: TokenType = TokenType::IdentifierOrKeyword(KeywordKind::DispId);
        const DISPID_P: TokenType = TokenType::Keyword(KeywordKind::DispId);
        const READONLY_I: TokenType = TokenType::IdentifierOrKeyword(KeywordKind::ReadOnly);
        const READONLY_P: TokenType = TokenType::Keyword(KeywordKind::ReadOnly);
        const WRITEONLY_I: TokenType = TokenType::IdentifierOrKeyword(KeywordKind::WriteOnly);
        const WRITEONLY_P: TokenType = TokenType::Keyword(KeywordKind::WriteOnly);
        const IMPL_I: TokenType = TokenType::IdentifierOrKeyword(KeywordKind::Implements);
        const IMPL_P: TokenType = TokenType::Keyword(KeywordKind::Implements);
        const INDEX_I: TokenType = TokenType::IdentifierOrKeyword(KeywordKind::Index);
        const INDEX_P: TokenType = TokenType::Keyword(KeywordKind::Index);
        const NODEFAULT_I: TokenType = TokenType::IdentifierOrKeyword(KeywordKind::NoDefault);
        const NODEFAULT_P: TokenType = TokenType::Keyword(KeywordKind::NoDefault);
        const STORED_I: TokenType = TokenType::IdentifierOrKeyword(KeywordKind::Stored);
        const STORED_P: TokenType = TokenType::Keyword(KeywordKind::Stored);

        // class property Foo: TBar read FFoo write FFoo dispid B readonly writeonly
        //   implements I index V nodefault stored true default A;
        run_test(
            &[
                CLASS,
                PROP,
                ID,
                COLON,
                ID,
                READ_I,
                ID,
                WRITE_I,
                ID,
                DISPID_I,
                ID,
                READONLY_I,
                WRITEONLY_I,
                IMPL_I,
                ID,
                INDEX_I,
                ID,
                NODEFAULT_I,
                STORED_I,
                DEFAULT_I,
                ID,
                SEMI,
            ],
            &[
                CLASS,
                PROP,
                ID,
                COLON,
                ID,
                READ_P,
                ID,
                WRITE_P,
                ID,
                DISPID_P,
                ID,
                READONLY_P,
                WRITEONLY_P,
                IMPL_P,
                ID,
                INDEX_P,
                ID,
                NODEFAULT_P,
                STORED_P,
                DEFAULT_P,
                ID,
                SEMI,
            ],
        );
    }
}
