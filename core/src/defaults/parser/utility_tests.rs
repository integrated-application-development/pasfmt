use spectral::prelude::*;

use super::*;
use crate::prelude::*;

#[yare::parameterized(
    valid = {
        indoc::indoc! {"
            function Bar(
            {$ifdef A}
              {$ifdef A}
              A
              {$elseif}
                {$ifdef A}
              B
                {$elseif}
              C
                {$elseif}
              D
                {$endif}
              {$endif}
            {$endif}
            );"
        },
        &[0, 1, 2],
    },
    no_end = {
        indoc::indoc! {"
            function Bar(
            {$ifdef A}
              {$ifdef A}
              A
              {$elseif}
                {$ifdef A}
              B
                {$elseif}
              C
                {$elseif}
              D
            );"
        },
        &[0, 1, 2],
    },
    adjacent_bigger_first = {
        indoc::indoc! {"
            function Bar(
            {$ifdef A}
              {$ifdef A}
              A
              {$else}
              B
              {$endif}
              {$ifdef A}
              C
              {$endif}
            {$endif}
            );"
        },
        &[0, 1],
    },
    adjacent_bigger_second = {
        indoc::indoc! {"
            function Bar(
            {$ifdef A}
              {$ifdef A}
              A
              {$endif}
              {$ifdef A}
              B
              {$else}
              C
              {$endif}
            {$endif}
            );"
        },
        &[0, 1],
    },
    no_branches = {
        indoc::indoc! {"
            function Bar(
            {$ifdef A}
              {$ifdef A}
              A
                {$ifdef A}
              B
                {$endif}
              {$endif}
            {$endif}
            );"
        },
        &[0, 0, 0],
    },
    naked_else = { "function Bar({$elseif});", &[0] },
    no_directives = { "function Bar();", &[0] },
    no_else = { "{$ifdef}function Bar();{$endif}", &[0] },
)]
fn directive_level_counts(input: &str, expected_levels: &[usize]) {
    let raw_tokens = DelphiLexer {}.lex(input);
    let conditional_branches = get_conditional_branches_per_directive(&raw_tokens);
    assert_that(&get_directive_level_last_indices(&conditional_branches).as_slice())
        .is_equal_to(expected_levels);
}

mod passes {
    use super::*;

    fn token_indices_to_string(tokens: &[RawToken], indices: &[usize]) -> String {
        let mut result = String::new();
        for &index in indices {
            if let Some(token) = tokens.get(index) {
                result.push_str(token.get_leading_whitespace());
                result.push_str(token.get_content());
            }
        }
        result
    }

    const IF: &str = "{$ifdef}";
    const ELSEIF: &str = "{$elseif}";
    const END: &str = "{$endif}";

    #[yare::parameterized(
        no_directives = {
            "foo(a, b, c);".to_owned(),
            &["foo(a, b, c);"],
        },
        if_around_all = {
            format!("{IF}foo(a, b, c);{END}"),
            &["foo(a, b, c);"],
        },
        if_else_around_all = {
            format!("{IF}foo(a, b, c){ELSEIF}bar(d, e, f){END}"),
            &[
                "foo(a, b, c)",
                "bar(d, e, f)",
            ],
        },
        if_else_else_around_all = {
            format!("{IF}foo(a, b, c){ELSEIF}bar(d, e, f){ELSEIF}baz(g, h, i){END}"),
            &[
                "foo(a, b, c)",
                "bar(d, e, f)",
                "baz(g, h, i)",
            ],
        },
        if_internal = {
            format!("foo({IF}a{END});"),
            &["foo(a);"],
        },
        if_else_internal = {
            format!("foo({IF}a{ELSEIF}b{END});"),
            &[
                "foo(a);",
                "foo(b);",
            ],
        },
        if_else_else_internal = {
            format!("foo({IF}a{ELSEIF}b{ELSEIF}c{END});"),
            &[
                "foo(a);",
                "foo(b);",
                "foo(c);",
            ],
        },
        max_branch = {
            format!("foo({IF}a{ELSEIF}b{END}, {IF}d{ELSEIF}e{ELSEIF}f{END});"),
            &[
                "foo(a,d);",
                "foo(b,e);",
                "foo(b,f);",
            ],
        },
        max_branch_nested = {
            indoc::formatdoc!("
                foo(
                  {IF}
                    {IF}a{ELSEIF}b{END}, {IF}d{ELSEIF}e{ELSEIF}f{END},
                  {END}
                  {IF}
                    {IF}g{ELSEIF}h{END}, {IF}i{ELSEIF}j{ELSEIF}k{END}
                  {ELSEIF}
                    {IF}l{ELSEIF}m{END}, {IF}n{ELSEIF}o{ELSEIF}p{END}
                  {END});"
            ),
            &[
                "foo(a,d,g,i);",
                "foo(b,e,h,j);",
                "foo(b,f,h,k);",
                "foo(a,d,l,n);",
                "foo(b,e,m,o);",
                "foo(b,f,m,p);",
            ],
        },
    )]
    fn token_views(input: String, expected_pass_strings: &[&str]) {
        let raw_tokens = DelphiLexer {}.lex(&input);
        let conditional_branches = get_conditional_branches_per_directive(&raw_tokens);
        let mut pass_tokens = Vec::new();
        let pass_strings = get_all_conditional_branch_paths(&conditional_branches)
            .into_iter()
            .map(|branch| {
                get_pass_tokens(
                    &raw_tokens,
                    &branch,
                    &conditional_branches,
                    &mut pass_tokens,
                );
                token_indices_to_string(&raw_tokens, &pass_tokens)
            })
            .collect_vec();
        let pass_strings = pass_strings.iter().map(String::as_str).collect_vec();
        let expected_pass_strings = expected_pass_strings.iter().collect_vec();
        assert_that(&pass_strings).contains_all_of(&expected_pass_strings);
    }
}

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
    assert_that(&parser.pass_index).is_equal_to(token_count);
    assert_that(&parser.brack_level).is_equal_to(0);
    assert_that(&parser.paren_level).is_equal_to(0);
    assert_that(&parser.current_line.len()).is_equal_to(original_line_count);
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
    assert_that(&lines).has_length(1);
    assert_that(lines[0].get_tokens()).has_length(tokens_len);
    assert_that(&tokens_len).is_equal_to(consolidated_tokens.len());
}
