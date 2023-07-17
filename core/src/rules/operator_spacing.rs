use crate::lang::OperatorKind::*;
use crate::{lang::*, traits::LogicalLineFormatter};

pub struct OperatorSpacing {}
impl LogicalLineFormatter for OperatorSpacing {
    fn format(&self, formatted_tokens: &mut FormattedTokens, input: &LogicalLine) {
        for (line_index, &token_index) in input.get_tokens().iter().enumerate() {
            let token_type_by_line_idx = |token_idx: usize| {
                input
                    .get_tokens()
                    .get(token_idx)
                    .and_then(|t| formatted_tokens.get_token_type_for_index(*t))
            };

            if let Some(TokenType::Op(operator)) = formatted_tokens
                .get_token(token_index)
                .map(|(token, _)| token.get_token_type())
            {
                let (spaces_before, spaces_after) = match operator {
                    // always binary operators
                    Star | Slash | Assign | Equal | NotEqual | LessEqual | GreaterEqual
                    | LessThan | GreaterThan | Mod | Div | Shl | Shr | And | As | In | Or | Xor
                    | Is => (Some(1), Some(1)),
                    // maybe unary operators
                    op @ (Plus | Minus | Not) => {
                        let prev = token_type_by_line_idx(line_index.wrapping_sub(1));
                        let unary_trailing_spaces = if op == Not { Some(1) } else { Some(0) };
                        match prev {
                            // unary after keyword
                            Some(TokenType::Keyword(_)) => (Some(1), unary_trailing_spaces),
                            // unary after opening bracket or start of line
                            None | Some(TokenType::Op(LParen | LBrack)) => {
                                (Some(0), unary_trailing_spaces)
                            }
                            // unary not after closing bracket
                            Some(TokenType::Op(typ)) if typ != RBrack && typ != RParen => {
                                (Some(1), unary_trailing_spaces)
                            }
                            // binary operation
                            _ => (Some(1), Some(1)),
                        }
                    }
                    Comma | Colon => (Some(0), Some(1)),
                    RBrack | RParen => match token_type_by_line_idx(line_index + 1) {
                        Some(
                            TokenType::Identifier
                            | TokenType::IdentifierOrKeyword(_)
                            | TokenType::Keyword(_),
                        ) => (Some(0), Some(1)),
                        _ => (Some(0), Some(0)),
                    },
                    LBrack | LParen => match token_type_by_line_idx(line_index.wrapping_sub(1)) {
                        Some(TokenType::Identifier) | Some(TokenType::IdentifierOrKeyword(_)) => {
                            (Some(0), Some(0))
                        }
                        Some(TokenType::Keyword(
                            PureKeywordKind::Class
                            | PureKeywordKind::Interface
                            | PureKeywordKind::Function
                            | PureKeywordKind::Procedure
                            | PureKeywordKind::Array,
                        )) => (Some(0), Some(0)),
                        Some(TokenType::Keyword(_)) => (Some(1), Some(0)),
                        _ => (None, Some(0)),
                    },
                    Pointer => {
                        match (
                            token_type_by_line_idx(line_index.wrapping_sub(1)),
                            token_type_by_line_idx(line_index + 1),
                        ) {
                            // ident|)|] before ^ (e.g. foo^, foo()^)
                            (
                                Some(
                                    TokenType::Identifier
                                    | TokenType::IdentifierOrKeyword(_)
                                    | TokenType::Op(RBrack | RParen),
                                ),
                                token_after,
                            ) => (
                                Some(0),
                                match token_after {
                                    Some(TokenType::Op(RBrack | RParen | LBrack | LParen)) => {
                                        Some(0)
                                    }
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
                                    Some(TokenType::Op(RBrack | RParen | LBrack | LParen)) => {
                                        Some(0)
                                    }
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
                        match token_type_by_line_idx(line_index + 1) {
                            Some(TokenType::Op(_)) => Some(0),
                            _ => Some(1),
                        },
                    ),
                    AddressOf => (
                        match token_type_by_line_idx(line_index.wrapping_sub(1)) {
                            None => None,
                            Some(TokenType::Op(LBrack | LParen)) => Some(0),
                            _ => Some(1),
                        },
                        Some(0),
                    ),
                    Semicolon => (
                        Some(0),
                        if token_type_by_line_idx(line_index + 1).is_some() {
                            Some(1)
                        } else {
                            None
                        },
                    ),
                };

                if let Some(spaces_before) = spaces_before {
                    if let Some(formatting_data) =
                        formatted_tokens.get_formatting_data_mut(token_index)
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
                    if let Some(formatting_data) =
                        formatted_tokens.get_formatting_data_mut(next_idx)
                    {
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
}

#[cfg(test)]
mod tests {
    use spectral::prelude::*;

    use super::*;
    use crate::{
        defaults::lexer::DelphiLexer, defaults::parser::DelphiLogicalLineParser,
        defaults::reconstructor::DelphiLogicalLinesReconstructor, formatter::*,
        rules::generics_consolidator::DistinguishGenericTypeParamsConsolidator,
    };

    fn run_test(input: &'static str, output: &'static str) {
        let formatter = Formatter::builder()
            .lexer(DelphiLexer {})
            .token_consolidator(DistinguishGenericTypeParamsConsolidator {})
            .parser(DelphiLogicalLineParser {})
            .line_formatter(OperatorSpacing {})
            .reconstructor(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_string(), "  ".to_string(), "  ".to_string()),
            ))
            .build();

        let formatted_output = formatter.format(input);
        assert_that(&formatted_output).is_equal_to(output.to_string());
    }

    #[test]
    fn parens_after_special_pure_keywords() {
        run_test("class ( TObject);", "class(TObject);");
        run_test("interface ( IInterface);", "interface(IInterface);");
        run_test("procedure ();", "procedure();");
        run_test("function  ();", "function();");
    }

    #[test]
    fn plus() {
        run_test("Foo+Bar;", "Foo + Bar;");
    }

    #[test]
    fn unary_plus() {
        run_test("Foo( + 1);", "Foo(+1);");
        run_test("while+1;", "while +1;");
        run_test("+ 1;", "+1;");
        run_test(";+ 1;", ";+1;");
    }

    #[test]
    fn unary_not() {
        run_test("Foo( not False);", "Foo(not False);");
        run_test("Foo[ not False];", "Foo[not False];");
        run_test("while  not  False;", "while not False;");
        run_test("Foo  not  False;", "Foo not False;");
        run_test("not(Foo)", "not (Foo)");
        run_test(";not Bar;", ";not Bar;");
    }

    #[test]
    fn other() {
        run_test("Foo(Position+1, 0, 0);", "Foo(Position + 1, 0, 0);")
    }

    #[test]
    fn space_after_right_brackets() {
        run_test("if Foo(0 )then", "if Foo(0) then")
    }

    #[test]
    fn space_before_left_brackets() {
        run_test("Foo(0,[])", "Foo(0, [])");
        run_test("Foo(0,  [])", "Foo(0, [])");
        run_test("Foo([])", "Foo([])");
        run_test("array [0..2]", "array[0..2]");
    }

    #[test]
    fn pointer_before() {
        run_test("if  ^ Foo then", "if ^Foo then");
    }

    #[test]
    fn pointer_after() {
        run_test("if Foo ^ then", "if Foo^ then");
    }

    #[test]
    fn pointer_after_parens() {
        run_test("if Foo() ^ then", "if Foo()^ then");
    }

    #[test]
    fn pointer_after_brackets() {
        run_test("if Foo[] ^then", "if Foo[]^ then");
    }

    #[test]
    fn pointer_before_parens() {
        run_test("if Foo ^ () then", "if Foo^() then");
    }

    #[test]
    fn pointer_before_brackets() {
        run_test("if Foo ^ [] then", "if Foo^[] then");
    }

    #[test]
    fn pointer_before_dot() {
        run_test("if Foo ^ .Bar then", "if Foo^.Bar then");
    }

    #[test]
    fn function_declaration() {
        run_test(
            "function Foo ( Bar :String ;Baz:Integer) :Integer;",
            "function Foo(Bar: String; Baz: Integer): Integer;",
        );
        run_test(
            "procedure Foo ( Bar :String ;Baz:Integer) ;",
            "procedure Foo(Bar: String; Baz: Integer);",
        )
    }

    #[test]
    fn address_operator() {
        run_test("Foo :=@ Bar", "Foo := @Bar");
        run_test("Foo + @ Bar", "Foo + @Bar");
        run_test("Foo( @ Bar)", "Foo(@Bar)");
        run_test("Foo[ @ Bar]", "Foo[@Bar]");
        run_test("@ Foo := Bar", "@Foo := Bar");
    }

    #[test]
    fn always_binary_operators() {
        run_test("Field^:=Self", "Field^ := Self");
        run_test("Field^<>Self", "Field^ <> Self");
        run_test("Field<=  Self", "Field <= Self");
        run_test("Field  >=Self", "Field >= Self");
        run_test("Foo()=Self", "Foo() = Self");
        run_test(" = Self", " = Self");
        run_test("Foo[0]  /Self", "Foo[0] / Self");
        run_test("Foo[0]  *Self", "Foo[0] * Self");
    }

    #[test]
    fn comparison_operators() {
        run_test("", "");
        run_test("a<b", "a < b");
        run_test("a>b", "a > b");
        // this is not what we want, but it would be hard to distinguish this from generics without
        // proper parsing.
        run_test("Foo(Bar < Baz, Baz > Bar)", "Foo(Bar < Baz, Baz > Bar)");
    }

    #[test]
    fn generic_type_params() {
        run_test("Foo < Bar, Baz > ()", "Foo<Bar, Baz>()");
        run_test("class Foo < Bar, Baz > =T", "class Foo<Bar, Baz> = T");
        run_test("Foo < Bar< Baz >, FooBar >", "Foo<Bar<Baz>, FooBar>");
        run_test("Foo(Bar < T, U > (V))", "Foo(Bar<T, U>(V))");
        run_test("[Bar < T > (V)]", "[Bar<T>(V)]");
        run_test("Foo(Bar < T, U > [1])", "Foo(Bar<T, U>[1])");
        run_test("Foo(Bar < T, U > @V)", "Foo(Bar < T, U > @V)");

        run_test("Foo<Bar>  =T", "Foo<Bar> = T");

        // ambiguous >= token, room for improvement
        run_test("Foo<Bar>=T", "Foo < Bar >= T");
        run_test("Foo<Bar, Baz>=T", "Foo < Bar, Baz >= T");
    }
}
