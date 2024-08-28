use crate::lang::OperatorKind as OK;
use crate::lang::TokenType as TT;
use crate::lang::*;
use crate::prelude::*;

pub struct TokenSpacing {}
impl LogicalLineFileFormatter for TokenSpacing {
    fn format(&self, formatted_tokens: &mut FormattedTokens, _input: &[LogicalLine]) {
        for token_index in 0..formatted_tokens.get_tokens().len() {
            let (spaces_before, spaces_after) = match formatted_tokens
                .get_token_type_for_index(token_index)
            {
                Some(TT::Op(operator)) => space_operator(operator, token_index, formatted_tokens),
                Some(TT::Comment(CommentKind::InlineLine)) => (Some(1), None),
                Some(
                    TT::Comment(_)
                    | TT::CompilerDirective
                    | TT::ConditionalDirective(_)
                    | TT::Keyword(_),
                ) => one_space_either_side(token_index, formatted_tokens),
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
                    if formatting_data.newlines_before == 0 && next_token_type != Some(TT::Eof) {
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

fn spaces_before(token_type: Option<TokenType>, spaces: usize) -> Option<usize> {
    match token_type {
        None => None,
        Some(TT::Op(OK::LBrack | OK::LParen | OK::LessThan(ChevronKind::Generic))) => Some(0),
        _ => Some(spaces),
    }
}

fn spaces_after(token_type: Option<TokenType>, spaces: usize) -> Option<usize> {
    match token_type {
        None => None,
        Some(TT::Op(OK::RBrack | OK::RParen | OK::GreaterThan(ChevronKind::Generic))) => Some(0),
        _ => Some(spaces),
    }
}

fn one_space_either_side(
    token_index: usize,
    formatted_tokens: &mut FormattedTokens<'_>,
) -> (Option<usize>, Option<usize>) {
    (
        spaces_before(
            formatted_tokens.get_token_type_for_index(token_index.wrapping_sub(1)),
            1,
        ),
        spaces_after(
            formatted_tokens.get_token_type_for_index(token_index + 1),
            1,
        ),
    )
}

fn one_space_before(
    token_index: usize,
    formatted_tokens: &mut FormattedTokens<'_>,
) -> (Option<usize>, Option<usize>) {
    (
        spaces_before(
            formatted_tokens.get_token_type_for_index(token_index.wrapping_sub(1)),
            1,
        ),
        Some(0),
    )
}

fn space_operator(
    operator: OperatorKind,
    token_index: usize,
    formatted_tokens: &mut FormattedTokens<'_>,
) -> (Option<usize>, Option<usize>) {
    let token_type_by_idx = |token_idx: usize| formatted_tokens.get_token_type_for_index(token_idx);
    let prev_real_token_type = |token_idx: usize| {
        formatted_tokens
            .get_tokens()
            .iter()
            .rev()
            .skip(formatted_tokens.get_tokens().len() - token_idx)
            .find_map(|token| {
                Some(token.0.get_token_type()).filter(|t| !t.is_comment_or_directive())
            })
    };

    let binary_op_spacing = (Some(1), Some(1));

    match operator {
        // always binary operators
        OK::Star
        | OK::Slash
        | OK::Assign
        | OK::Equal(_)
        | OK::NotEqual
        | OK::LessEqual
        | OK::GreaterEqual
        | OK::LessThan(ChevronKind::Comp)
        | OK::GreaterThan(ChevronKind::Comp) => binary_op_spacing,

        // maybe unary operators
        OK::Plus | OK::Minus => {
            let prev = prev_real_token_type(token_index);
            match prev {
                // binary after closing bracket or closing generics or special keywords
                Some(
                    TT::Op(OK::RBrack | OK::RParen | OK::GreaterThan(ChevronKind::Generic))
                    | TT::Keyword(KeywordKind::Inherited | KeywordKind::Nil),
                ) => binary_op_spacing,
                /*
                    unary after:
                    - other keywords
                    - start of line
                    - any other operator
                    - comments/directives
                */
                None
                | Some(
                    TT::Op(_)
                    | TT::Keyword(_)
                    | TT::Comment(_)
                    | TT::CompilerDirective
                    | TT::ConditionalDirective(_),
                ) => (None, Some(0)),
                // default to binary
                _ => binary_op_spacing,
            }
        }
        OK::Comma | OK::Colon => (Some(0), Some(1)),
        OK::RBrack | OK::RParen => match token_type_by_idx(token_index + 1) {
            Some(TT::Identifier | TT::Keyword(_)) => (Some(0), Some(1)),
            _ => (Some(0), Some(0)),
        },
        OK::LBrack | OK::LParen => match token_type_by_idx(token_index.wrapping_sub(1)) {
            Some(
                TT::Identifier
                | TT::Keyword(
                    KeywordKind::Class
                    | KeywordKind::Abstract
                    | KeywordKind::Sealed
                    | KeywordKind::Interface
                    | KeywordKind::Function
                    | KeywordKind::Procedure
                    | KeywordKind::Array
                    | KeywordKind::String,
                ),
            ) => (Some(0), Some(0)),
            Some(TT::Keyword(_)) => (Some(1), Some(0)),
            _ => (None, Some(0)),
        },
        OK::Pointer => {
            match (
                token_type_by_idx(token_index.wrapping_sub(1)),
                token_type_by_idx(token_index + 1),
            ) {
                /*
                           | matching the operator in this column
                           v

                       foo ^
                      foo^ ^
                     foo() ^
                    foo[0] ^
                */
                (
                    Some(TT::Identifier | TT::Op(OK::RBrack | OK::RParen | OK::Pointer)),
                    token_after,
                ) => (
                    Some(0),
                    match token_after {
                        Some(TT::Identifier | TT::Op(OK::LBrack | OK::LParen)) => Some(0),
                        _ => Some(1),
                    },
                ),
                // just ^foo
                (_, Some(TT::Identifier)) => (None, Some(0)),
                _ => (None, None),
            }
        }
        OK::Dot | OK::DotDot => (Some(0), Some(0)),
        OK::LessThan(ChevronKind::Generic) => (Some(0), Some(0)),
        OK::GreaterThan(ChevronKind::Generic) => (
            Some(0),
            match token_type_by_idx(token_index + 1) {
                Some(TT::Op(_)) => Some(0),
                _ => Some(1),
            },
        ),
        OK::AddressOf => one_space_before(token_index, formatted_tokens),
        OK::Semicolon => (Some(0), Some(1)),
    }
}

#[cfg(test)]
mod tests {
    use crate::{prelude::*, test_utils::formatter_test_group};
    use spectral::prelude::*;

    fn formatter() -> Formatter {
        Formatter::builder()
            .lexer(DelphiLexer {})
            .parser(DelphiLogicalLineParser {})
            .token_consolidator(DistinguishGenericTypeParamsConsolidator {})
            .file_formatter(TokenSpacing {})
            .reconstructor(default_test_reconstructor())
            .build()
    }

    formatter_test_group!(
        parens_after_special_pure_keywords,
        class_before_parens = {"class ( TObject);", "class(TObject);"},
        abstract_class_before_parens = {"class abstract ( TObject);", "class abstract(TObject);"},
        sealed_class_before_parens = {"class sealed ( TObject);", "class sealed(TObject);"},
        interface_before_parens = {"interface ( IInterface);", "interface(IInterface);"},
        procedure_before_parens = {"procedure ();", "procedure();"},
        function_before_parens = {"function  ();", "function();"},
        string_before_parens = {"string  (str);", "string(str);"},
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
        keyword_after_right_bracket = {"if Foo(0 )then", "if Foo(0) then"},
        maybe_keyword_after_right_bracket = {"Foo(0 )read", "Foo(0) read"},
        ident_after_right_bracket = {"Foo(0 )bar", "Foo(0) bar"},
    );

    formatter_test_group!(
        square_bracket,
        before_left_square_bracket = {"Foo(0,[])", "Foo(0, [])"},
        not_between_left_parens_left_square = {"Foo([])", "Foo([])"},
        after_comma_before_left_square = {"Foo(0,  [])", "Foo(0, [])"},
        not_between_array_left_square = {"array [0..2]", "array[0..2]"},
        not_between_string_left_square = {"string [255]", "string[255]"},
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
        double_pointer_before_brackets = {"Foo^ ^ [Bar]", "Foo^^[Bar]"},
        double_pointer_after_parens = {"Foo()  ^  ^", "Foo()^^"},
        double_pointer_after_brackets = {"Foo[0]   ^ ^", "Foo[0]^^"},
        double_pointer_before_ident = {"type Foo = ^ ^ Bar", "type Foo = ^^Bar"},
        triple_pointer_before_ident = {"type Foo = ^ ^ ^ Bar", "type Foo = ^^^Bar"},
        triple_pointer_after_ident = {"Foo  ^ ^ ^", "Foo^^^"},
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

        one_generic_param_in_property_type = {"property Foo: TFoo < A >", "property Foo: TFoo<A>"},
        two_generic_params_in_property_type = {"property Foo: TFoo < A , B >", "property Foo: TFoo<A, B>"},
        one_generic_param_in_property_type_before_read = {"property Foo : TFoo < A > read FFoo", "property Foo: TFoo<A> read FFoo"},
        two_generic_params_in_property_type_before_read = {"property Foo : TFoo < A ,  B > read FFoo", "property Foo: TFoo<A, B> read FFoo"},
        one_generic_param_in_property_type_before_write = {"property Foo : TFoo < A > write FFoo", "property Foo: TFoo<A> write FFoo"},
        two_generic_params_in_property_type_before_write = {"property Foo : TFoo < A ,  B > write FFoo", "property Foo: TFoo<A, B> write FFoo"},
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
        eol_line_comment_after_parens = {"foo(//\n)", "foo( //\n)"},
        eol_line_comment_inside_generics = {"Foo<//\nA>", "Foo< //\nA>"},
        eol_block_comment_after_parens = {"foo( {}\n)", "foo({}\n)"},
        eol_block_comment_inside_generics = {"Foo< {}\nA>", "Foo<{}\nA>"},
        block_comment_after_parens = {"foo( {})", "foo({})"},
        block_comment_before_parens = {"foo(\n{} )", "foo(\n{})"},
        block_comment_inside_generics = {"Foo< {}A, B{} >", "Foo<{} A, B {}>"},
        ifdef_inside_parens = {"foo( {$ifdef A}'a'{$else}'b'{$ifend} )", "foo({$ifdef A} 'a' {$else} 'b' {$ifend})"},
        ifdef_inside_generics = {"Foo< {$ifdef A}A{$else}B{$ifend} >", "Foo<{$ifdef A} A {$else} B {$ifend}>"},
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

    formatter_test_group!(
        binary_operations_with_keyword,
        binary_plus_with_inherited_left = {"inherited +1", "inherited + 1"},
        binary_plus_with_inherited_right = {"1 +inherited", "1 + inherited"},
        binary_plus_with_inherited_twice = {"inherited +inherited", "inherited + inherited"},
        binary_plus_with_inherited_named = {"inherited Foo +1", "inherited Foo + 1"},
        binary_plus_with_inherited_args = {"inherited Foo(0) +1", "inherited Foo(0) + 1"},
        binary_plus_with_nil_left = {"nil +''", "nil + ''"},
        binary_plus_with_nil_right = {"'' +nil", "'' + nil"},
        binary_plus_with_nil_twice = {"nil +nil", "nil + nil"},
    );

    formatter_test_group!(
        unary_operations_after_keyword,
        unary_plus_with_while = {"while + 1", "while +1"},
        unary_plus_with_if = {"if + 1", "if +1"},
    );

    formatter_test_group!(
        comment_and_directives,
        bof_then_comment_before_unary_op = {"{}+2", "{} +2"},
        keyword_then_comment_before_unary_op = {"if {}+2", "if {} +2"},
        literal_then_comment_before_binary_op = {"1{}+2", "1 {} + 2"},
        directive_before_unary_op = {"{$R}+2", "{$R} +2"},
        literal_then_directive_before_binary_op = {"1{$R}+2", "1 {$R} + 2"},
        several_directives_and_comments_before_unary_op = {"{}{$R}{}+2", "{} {$R} {} +2"},
        several_directives_and_comments_before_binary_op = {"1{}{$R}{}+2", "1 {} {$R} {} + 2"},
    );
}
