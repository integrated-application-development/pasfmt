use super::*;
use crate::prelude::*;
use pretty_assertions::assert_eq;

#[yare::parameterized(
    single_ident = { "A" },
    addition = { "A + B" },
    shr = { "A shr B" },
    dereference = { "A^" },
    empty_parens = { "()" },
    parens = { "(A)" },
    parens_binary_op = { "(A) + B" },
    nested_parens = { "((A))" },
    nested_parens_binary_op = { "((A)) + B" },
    empty_bracks = { "[]" },
    bracks = { "[A]" },
    nested_bracks = { "[[A]]" },
    array = { "[A, [B]]" },
    qualified_name = { "A.B.C" },
    qualified_name_in_expr = { "1 + A.B.C" },
    array_access = { "A[1]" },
    qualified_array_access = { "A.B[1]" },
    nested_generics_access = { "A<T, S<T>>.Bar()" },
    non_generics = { "A < B" },
)]
fn expression_parsing(input: &str) {
    test_expression_parsing(input, None);
}

#[yare::parameterized(
    invalid_binary = { "A > ;", 2 },
)]
fn invalid_expression_parsing(input: &str, token_count: usize) {
    test_expression_parsing(input, Some(token_count));
}

fn test_expression_parsing(input: &str, token_count: Option<usize>) {
    let lexer = &DelphiLexer {};
    // The token `other` is added to test that the expression parser isn't
    // stopping because of EOF
    let input_str = input.to_owned() + " other";
    let mut tokens = lexer.lex(&input_str);
    // Asserting that the all the tokens have been consumed, minus the EOF
    // token, and the `other` token if not otherwise specified
    let token_count = token_count.unwrap_or(tokens.len() - 2);

    eprintln!("input:\n  {input}\ntokens:");
    for token in tokens.iter() {
        eprintln!("  {token:?}");
    }
    let token_indices = (0..tokens.len()).collect_vec();
    let mut attributed_directives = FxHashSet::default();
    let mut parser = InternalDelphiLogicalLineParser::new(
        &mut tokens,
        &token_indices,
        &mut attributed_directives,
    );
    let original_line_count = parser.current_line.len();
    parser.parse_expression();
    assert_eq!(parser.pass_index, token_count);
    assert_eq!(parser.brack_level, 0);
    assert_eq!(parser.paren_level, 0);
    assert_eq!(parser.current_line.len(), original_line_count);
}

#[test]
fn no_eof() {
    // If there is erroneously no EOF token, the parser should still work
    let tokens = vec![
        RawToken::new("unit", 0, TT::Keyword(KK::Unit)),
        RawToken::new(" foo", 1, TT::Identifier),
        RawToken::new(";", 0, TT::Op(OK::Semicolon)),
    ];
    let tokens_len = tokens.len();

    let (lines, consolidated_tokens) = DelphiLogicalLineParser {}.parse(tokens);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].get_tokens().len(), tokens_len);
    assert_eq!(tokens_len, consolidated_tokens.len());
}

#[yare::parameterized(
    backward_2_from_real = { 6, -2, Some(0) },
    backward_2_from_filtered = { 5, -2, Some(0) },
    backward_2_to_none = { 3, -2, None },
    backward_1_from_real = { 6, -1, Some(3) },
    backward_1_from_filtered = { 5, -1, Some(3) },
    backward_1_to_none = { 0, -1, None },


    current_from_real = { 0, 0, Some(0) },
    current_from_filtered = { 1, 0, Some(1) },
    current_on_eof = { 7, 0, None },
    current_on_oob = { 8, 0, None },

    forward_1_from_real = { 0, 1, Some(3) },
    forward_1_from_filtered = { 1, 1, Some(3) },
    forward_1_to_none = { 6, 1, None },
    forward_2_from_real = { 0, 2, Some(6) },
    forward_2_from_filtered = { 1, 2, Some(6) },
    forward_2_to_none = { 3, 2, None },
)]
fn run_get_token_test(pass_index: usize, offset: isize, expected_token_index: Option<usize>) {
    let mut tokens = vec![
        RawToken::new("A", 0, TT::Identifier),
        RawToken::new("{1}", 0, TT::Comment(CK::InlineBlock)),
        RawToken::new("{2}", 0, TT::Comment(CK::InlineBlock)),
        RawToken::new("B", 0, TT::Identifier),
        RawToken::new("{3}", 0, TT::Comment(CK::InlineBlock)),
        RawToken::new("{4}", 0, TT::Comment(CK::InlineBlock)),
        RawToken::new("C", 0, TT::Identifier),
        RawToken::new("", 0, TT::Eof),
    ];
    let mut directives = Default::default();
    let pass_indices = (0..tokens.len()).collect_vec();
    let mut parser =
        InternalDelphiLogicalLineParser::new(&mut tokens, &pass_indices, &mut directives);
    parser.pass_index = pass_index;
    let offset_index = match offset {
        -2 => parser.get_token_index::<-2>(),
        -1 => parser.get_token_index::<-1>(),
        0 => parser.get_token_index::<0>(),
        1 => parser.get_token_index::<1>(),
        2 => parser.get_token_index::<2>(),
        _ => panic!("offset {offset} not mapped"),
    };
    pretty_assertions::assert_eq!(offset_index, expected_token_index);
}
