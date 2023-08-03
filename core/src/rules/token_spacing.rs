use crate::lang::OperatorKind::*;
use crate::lang::*;
use crate::prelude::*;

pub struct TokenSpacing {}
impl LogicalLineFileFormatter for TokenSpacing {
    fn format(&self, formatted_tokens: &mut FormattedTokens, _input: &[LogicalLine]) {
        for token_index in 0..formatted_tokens.get_tokens().len() {
            let (spaces_before, spaces_after) = match formatted_tokens
                .get_token_type_for_index(token_index)
            {
                Some(TokenType::Op(operator)) => {
                    space_operator(operator, token_index, formatted_tokens)
                }
                Some(
                    TokenType::Comment(_)
                    | TokenType::CompilerDirective
                    | TokenType::ConditionalDirective(_),
                ) => one_space_before(token_index, formatted_tokens),
                Some(TokenType::Keyword(_)) => one_space_either_side(token_index, formatted_tokens),
                _ => max_one_either_side(token_index, formatted_tokens),
            };

            if let Some(spaces_before) = spaces_before {
                if let Some(formatting_data) = formatted_tokens.get_formatting_data_mut(token_index)
                {
                    // TODO will not play nice with the optimising line formatter
                    if formatting_data.newlines_before == 0 {
                        formatting_data.spaces_before = spaces_before;
                    }
                }
            }

            if let Some(spaces_after) = spaces_after {
                let next_idx = token_index + 1;
                let next_token_type = formatted_tokens.get_token_type_for_index(next_idx);
                if let Some(formatting_data) = formatted_tokens.get_formatting_data_mut(next_idx) {
                    // TODO will not play nice with the optimising line formatter
                    if formatting_data.newlines_before == 0
                        && next_token_type != Some(TokenType::Eof)
                    {
                        formatting_data.spaces_before = spaces_after;
                    }
                }
            }
        }
    }
}

fn max_one_either_side(
    token_index: usize,
    formatted_tokens: &mut FormattedTokens<'_>,
) -> (Option<usize>, Option<usize>) {
    (
        formatted_tokens
            .get_formatting_data(token_index)
            .map(|data| data.spaces_before.min(1)),
        formatted_tokens
            .get_formatting_data(token_index + 1)
            .map(|data| data.spaces_before.min(1)),
    )
}

fn one_space_either_side(
    token_index: usize,
    formatted_tokens: &mut FormattedTokens<'_>,
) -> (Option<usize>, Option<usize>) {
    (
        match formatted_tokens.get_token_type_for_index(token_index.wrapping_sub(1)) {
            None => None,
            Some(TokenType::Op(LBrack | LParen)) => Some(0),
            _ => Some(1),
        },
        match formatted_tokens.get_token_type_for_index(token_index + 1) {
            None => None,
            Some(TokenType::Op(RBrack | RParen)) => Some(0),
            _ => Some(1),
        },
    )
}

fn one_space_before(
    token_index: usize,
    formatted_tokens: &mut FormattedTokens<'_>,
) -> (Option<usize>, Option<usize>) {
    (
        match formatted_tokens.get_token_type_for_index(token_index.wrapping_sub(1)) {
            None => None,
            Some(TokenType::Op(LBrack | LParen)) => Some(0),
            _ => Some(1),
        },
        Some(0),
    )
}

fn space_operator(
    operator: OperatorKind,
    token_index: usize,
    formatted_tokens: &mut FormattedTokens<'_>,
) -> (Option<usize>, Option<usize>) {
    let token_type_by_idx = |token_idx: usize| formatted_tokens.get_token_type_for_index(token_idx);

    let binary_op_spacing = (Some(1), Some(1));

    match operator {
        // always binary operators
        Star | Slash | Assign | Equal | NotEqual | LessEqual | GreaterEqual | LessThan
        | GreaterThan => binary_op_spacing,

        // maybe unary operators
        Plus | Minus => {
            let prev = token_type_by_idx(token_index.wrapping_sub(1));
            match prev {
                // unary after keyword
                Some(TokenType::Keyword(_)) => (Some(1), Some(0)),
                // unary after opening bracket or start of line
                None | Some(TokenType::Op(LParen | LBrack)) => (Some(0), Some(0)),
                // binary after closing bracket or closing generics
                Some(TokenType::Op(RBrack | RParen | RGeneric)) => binary_op_spacing,
                // unary after any other operator
                Some(TokenType::Op(_)) => (Some(1), Some(0)),
                // default to binary
                _ => binary_op_spacing,
            }
        }
        Comma | Colon => (Some(0), Some(1)),
        RBrack | RParen => match token_type_by_idx(token_index + 1) {
            Some(
                TokenType::Identifier | TokenType::IdentifierOrKeyword(_) | TokenType::Keyword(_),
            ) => (Some(0), Some(1)),
            _ => (Some(0), Some(0)),
        },
        LBrack | LParen => match token_type_by_idx(token_index.wrapping_sub(1)) {
            Some(TokenType::Identifier) | Some(TokenType::IdentifierOrKeyword(_)) => {
                (Some(0), Some(0))
            }
            Some(TokenType::Keyword(
                KeywordKind::Class
                | KeywordKind::Interface
                | KeywordKind::Function
                | KeywordKind::Procedure
                | KeywordKind::Array,
            )) => (Some(0), Some(0)),
            Some(TokenType::Keyword(_)) => (Some(1), Some(0)),
            _ => (None, Some(0)),
        },
        Pointer => {
            match (
                token_type_by_idx(token_index.wrapping_sub(1)),
                token_type_by_idx(token_index + 1),
            ) {
                // ident|)|]|^ before ^ (e.g. foo^, foo^^, foo()^)
                (
                    Some(
                        TokenType::Identifier
                        | TokenType::IdentifierOrKeyword(_)
                        | TokenType::Op(RBrack | RParen | Pointer),
                    ),
                    token_after,
                ) => (
                    Some(0),
                    match token_after {
                        Some(TokenType::Op(RBrack | RParen | LBrack | LParen)) => Some(0),
                        _ => Some(1),
                    },
                ),
                // ident|(|[ after ^ (e.g. ^foo, foo^(), foo[0]^, foo()^())
                (
                    token_before,
                    Some(
                        TokenType::Identifier
                        | TokenType::IdentifierOrKeyword(_)
                        | TokenType::Op(LBrack | LParen),
                    ),
                ) => (
                    match token_before {
                        Some(TokenType::Op(RBrack | RParen | LBrack | LParen)) => Some(0),
                        _ => Some(1),
                    },
                    Some(0),
                ),
                _ => (None, None),
            }
        }
        Dot | DotDot => (Some(0), Some(0)),
        LGeneric => (Some(0), Some(0)),
        RGeneric => (
            Some(0),
            match token_type_by_idx(token_index + 1) {
                Some(TokenType::Op(_)) => Some(0),
                _ => Some(1),
            },
        ),
        AddressOf => one_space_before(token_index, formatted_tokens),
        Semicolon => (
            Some(0),
            if token_type_by_idx(token_index + 1).is_some() {
                Some(1)
            } else {
                None
            },
        ),
    }
}

#[cfg(test)]
mod tests {
    use crate::{prelude::*, rules::test_utils::formatter_test_group};
    use spectral::prelude::*;

    fn formatter() -> Formatter {
        Formatter::builder()
            .lexer(DelphiLexer {})
            .token_consolidator(DistinguishGenericTypeParamsConsolidator {})
            .parser(DelphiLogicalLineParser {})
            .file_formatter(TokenSpacing {})
            .reconstructor(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_string(), "  ".to_string(), "  ".to_string()),
            ))
            .build()
    }

    formatter_test_group!(
        parens_after_special_pure_keywords,
        class_before_parens = {"class ( TObject);", "class(TObject);"},
        interface_before_parens = {"interface ( IInterface);", "interface(IInterface);"},
        procedure_before_parens = {"procedure ();", "procedure();"},
        function_before_parens = {"function  ();", "function();"},
    );

    formatter_test_group!(
        plus,
        plus_between_idents = {"Foo+Bar;", "Foo + Bar;"},
    );

    formatter_test_group!(
        unary_plus,
        plus_after_parens = {"Foo( + 1);", "Foo(+1);"},
        plus_after_square = {"Foo[ + 1];", "Foo[+1];"},
        plus_after_keyword = {"while+1;", "while +1;"},
        plus_at_start_of_input = {"+ 1;", "+1;"},
    );

    formatter_test_group!(
        unary_not,
        not_after_parens = {"Foo( not False);", "Foo(not False);"},
        not_after_square_bracket = {"Foo[ not False];", "Foo[not False];"},
        not_after_keyword = {"while  not  False;", "while not False;"},
        not_after_ident = {"Foo  not  False;", "Foo not False;"},
        not_before_parens = {"not(Foo)", "not (Foo)"},
        not_before_ident = {"not Bar;", "not Bar;"},
    );

    formatter_test_group!(
        parens,
        after_right_bracket = {"if Foo(0 )then", "if Foo(0) then"},
    );

    formatter_test_group!(
        square_bracket,
        before_left_square_bracket = {"Foo(0,[])", "Foo(0, [])"},
        not_between_left_parens_left_square = {"Foo([])", "Foo([])"},
        after_comma_before_left_square = {"Foo(0,  [])", "Foo(0, [])"},
        not_between_array_left_square = {"array [0..2]", "array[0..2]"},
    );

    formatter_test_group!(
        pointer,
        pointer_before = {"if  ^ Foo then", "if ^Foo then"},
        pointer_after = {"if Foo ^ then", "if Foo^ then"},
        pointer_after_parens = {"if Foo() ^ then", "if Foo()^ then"},
        pointer_after_brackets = {"if Foo[] ^then", "if Foo[]^ then"},
        pointer_before_parens = {"if Foo ^ () then", "if Foo^() then"},
        pointer_before_brackets = {"if Foo ^ [] then", "if Foo^[] then"},
        pointer_before_dot = {"if Foo ^ .Bar then", "if Foo^.Bar then"},
        double_pointer_before_dot = {"Foo^ ^ .Bar", "Foo^^.Bar"},
        double_pointer_before_parens = {"Foo^ ^ (Bar)", "Foo^^(Bar)"},
    );

    formatter_test_group!(
        routine_declaration,
        function_declaration = {
            "function Foo ( Bar :String ;Baz:Integer) :Integer;",
            "function Foo(Bar: String; Baz: Integer): Integer;",
        },
        procedure_declaration = {
            "procedure Foo ( Bar :String ;Baz:Integer) ;",
            "procedure Foo(Bar: String; Baz: Integer);",
        }
    );

    formatter_test_group!(
        address_operator,
        assign_address_ident = {"Foo :=@ Bar", "Foo := @Bar"},
        plus_address_ident = {"Foo + @ Bar", "Foo + @Bar"},
        parens_address_ident = {"Foo( @ Bar)", "Foo(@Bar)"},
        square_address_ident = {"Foo[ @ Bar]", "Foo[@Bar]"},
        left_address_ident_assign = {"@ Foo := Bar", "@Foo := Bar"},
    );

    macro_rules! binary_op_cases {
        ($(($name: ident, $symbol: expr)),* $(,)?)=> {
            formatter_test_group!(
                binary_operators,
                $($name = {concat!("A  ", $symbol, "  B"), concat!("A ", $symbol, " B")}),*
            );
        };
    }

    binary_op_cases!(
        (star, "*"),
        (slash, "/"),
        (assign, ":="),
        (equal, "="),
        (not_equal, "<>"),
        (less_equal, "<="),
        (greater_equal, ">="),
        (less_than, "<"),
        (greater_than, ">"),
        (r#mod, "mod"),
        (div, "div"),
        (shl, "shl"),
        (shr, "shr"),
        (and, "and"),
        (r#as, "as"),
        (r#in, "in"),
        (or, "or"),
        (xor, "xor"),
        (is, "is")
    );

    formatter_test_group!(
        generic_type_params,
        two_type_param_function_call = {"Foo < Bar, Baz > ()", "Foo<Bar, Baz>()"},
        two_type_param_type_def = {"class Foo < Bar, Baz > =T", "class Foo<Bar, Baz> = T"},
        single_type_param_type_def = {"class Foo < Bar > =T", "class Foo<Bar> = T"},
        nested_multi_generic_param_list = {"Foo < Bar< Baz >, FooBar >", "Foo<Bar<Baz>, FooBar>"},
        multi_type_param_list_nested_function_call = {"Foo(Bar < T, U > (V))", "Foo(Bar<T, U>(V))"},
        multi_type_param_list_nested_array_index = {"Foo(Bar < T, U > [1])", "Foo(Bar<T, U>[1])"},
        multi_type_param_list_nested_before_address_op = {"Foo(Bar < T, U > @V)", "Foo(Bar < T, U > @V)"},
        type_params_inside_array_literal = {"[Bar < T > (V)]", "[Bar<T>(V)]"},
        single_type_param_equality = {"Foo<Bar>  =T", "Foo<Bar> = T"},

        identifier_after_multi_type_param_list = {"Foo(Bar < Baz, Baz > Bar)", "Foo(Bar < Baz, Baz > Bar)"},
        address_op_after_multi_type_param_list = {"Foo(Bar < Baz, Baz > @Bar)", "Foo(Bar < Baz, Baz > @Bar)"},
        not_op_after_multi_type_param_list = {"Foo(Bar < Baz, Baz > not Bar)", "Foo(Bar < Baz, Baz > not Bar)"},
        maybe_binary_op_after_multi_type_param_list = {"Foo(Bar < Baz, Baz > + Bar)", "Foo(Bar<Baz, Baz> + Bar)"},
        binary_op_after_multi_type_param_list = {"Foo(Bar < Baz, Baz > / Bar)", "Foo(Bar<Baz, Baz> / Bar)"},

        // ambiguous >= token, room for improvement
        ambiguous_greater_equal_single_type_param_type_def = {"Foo<Bar>=T", "Foo < Bar >= T"},
        ambiguous_greater_equal_multi_type_param_type_def = {"Foo<Bar, Baz>=T", "Foo < Bar, Baz >= T"},
    );

    formatter_test_group!(
        after_semicolon,
        semicolon_at_start = {";foo", "; foo"},
        semicolon_in_middle = {"1;2", "1; 2"},
        semicolon_at_end = {"1;", "1;"},
    );

    formatter_test_group!(
        space_before_comments_and_directives,
        eol_line_comment = {"foo//", "foo //"},
        eol_line_comment_double_space = {"foo  //", "foo //"},
        eol_block_comment_no_space = {"foo{}", "foo {}"},
        eol_block_comment_double_space = {"foo  {}", "foo {}"},
        ifdef = {"foo{$ifdef A}{$endif}", "foo {$ifdef A} {$endif}"},
        directive = {"foo{$Message 'A'}", "foo {$Message 'A'}"},
    );

    formatter_test_group!(
        consecutive_keywords,
        begin_end = {"begin   end", "begin end"},
        repeat_until = {"repeat  until", "repeat until"},
        then_begin_end = {"then  begin end", "then begin end"},
    );

    formatter_test_group!(
        fallback_spacing,
        adjacent_text_literals = {"'a'  'a'", "'a' 'a'"},
        adjacent_number_literals = {"0   1", "0 1"},
        adjacent_unknown_tokens_without_space = {"!!", "!!"},
        adjacent_unknown_tokens_with_space = {"!  !", "! !"}
    );

    formatter_test_group!(
        ambiguous_keyword,
        max_one_space_around_ambiguous_keywords = {"public  private   ReadOnly Message", "public private ReadOnly Message"},
    );
}
