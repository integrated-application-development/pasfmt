use crate::lang::ConditionalDirectiveKind::*;
use crate::lang::IdentifierOrKeywordKind::*;
use crate::lang::NumberLiteralKind::*;
use crate::lang::OperatorKind::*;
use crate::lang::PureKeywordKind::*;
use crate::lang::TokenType::*;
use crate::lang::*;
use crate::traits::Lexer;

use log::*;
use nom::branch::*;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::multi::*;
use nom::sequence::*;
use nom::*;

pub struct DelphiLexer {}
impl Lexer for DelphiLexer {
    fn lex<'a>(&self, input: &'a str) -> Vec<Token<'a>> {
        parse_file(input)
    }
}

type ContentAndTokenType<'a> = (&'a str, TokenType);
type WhitespaceAndToken<'a> = (&'a str, ContentAndTokenType<'a>);

fn get_word_token_type(input: &str) -> TokenType {
    match input.to_lowercase().as_str() {
        "absolute" => IdentifierOrKeyword(Absolute),
        "abstract" => IdentifierOrKeyword(Abstract),
        "add" => IdentifierOrKeyword(Add),
        "align" => IdentifierOrKeyword(Align),
        "and" => Op(And),
        "array" => Keyword(Array),
        "as" => Op(As),
        "assembler" => IdentifierOrKeyword(Assembler),
        "at" => IdentifierOrKeyword(At),
        "automated" => IdentifierOrKeyword(Automated),
        "begin" => Keyword(Begin),
        "case" => Keyword(Case),
        "cdecl" => IdentifierOrKeyword(Cdecl),
        "class" => Keyword(Class),
        "const" => Keyword(Const),
        "constructor" => Keyword(Constructor),
        "contains" => IdentifierOrKeyword(Contains),
        "default" => IdentifierOrKeyword(Default),
        "delayed" => IdentifierOrKeyword(Delayed),
        "deprecated" => IdentifierOrKeyword(Deprecated),
        "destructor" => Keyword(Destructor),
        "dispid" => IdentifierOrKeyword(Dispid),
        "dispinterface" => Keyword(Dispinterface),
        "div" => Op(Div),
        "do" => Keyword(Do),
        "downto" => Keyword(Downto),
        "dynamic" => IdentifierOrKeyword(Dynamic),
        "else" => Keyword(PureKeywordKind::Else),
        "end" => Keyword(End),
        "except" => Keyword(Except),
        "experimental" => IdentifierOrKeyword(Experimental),
        "export" => IdentifierOrKeyword(Export),
        "exports" => Keyword(Exports),
        "external" => IdentifierOrKeyword(External),
        "far" => IdentifierOrKeyword(Far),
        "file" => Keyword(File),
        "final" => IdentifierOrKeyword(Final),
        "finalization" => Keyword(Finalization),
        "finally" => Keyword(Finally),
        "for" => Keyword(For),
        "forward" => IdentifierOrKeyword(Forward),
        "function" => Keyword(Function),
        "goto" => Keyword(Goto),
        "helper" => IdentifierOrKeyword(Helper),
        "if" => Keyword(PureKeywordKind::If),
        "implementation" => Keyword(Implementation),
        "implements" => IdentifierOrKeyword(Implements),
        "in" => Op(In),
        "index" => IdentifierOrKeyword(Index),
        "inherited" => Keyword(Inherited),
        "initialization" => Keyword(Initialization),
        "inline" => Keyword(Inline),
        "interface" => Keyword(Interface),
        "is" => Op(Is),
        "label" => Keyword(Label),
        "library" => Keyword(Library),
        "local" => IdentifierOrKeyword(Local),
        "message" => IdentifierOrKeyword(Message),
        "mod" => Op(Mod),
        "name" => IdentifierOrKeyword(Name),
        "near" => IdentifierOrKeyword(Near),
        "nil" => Keyword(Nil),
        "nodefault" => IdentifierOrKeyword(NoDefault),
        "not" => Op(Not),
        "object" => Keyword(Object),
        "of" => Keyword(Of),
        "on" => IdentifierOrKeyword(On),
        "operator" => IdentifierOrKeyword(Operator),
        "or" => Op(Or),
        "out" => IdentifierOrKeyword(Out),
        "overload" => IdentifierOrKeyword(Overload),
        "override" => IdentifierOrKeyword(Override),
        "package" => IdentifierOrKeyword(Package),
        "packed" => Keyword(Packed),
        "pascal" => IdentifierOrKeyword(Pascal),
        "platform" => IdentifierOrKeyword(Platform),
        "private" => IdentifierOrKeyword(Private),
        "procedure" => Keyword(Procedure),
        "program" => Keyword(Program),
        "property" => Keyword(Property),
        "protected" => IdentifierOrKeyword(Protected),
        "public" => IdentifierOrKeyword(Public),
        "published" => IdentifierOrKeyword(Published),
        "raise" => Keyword(Raise),
        "read" => IdentifierOrKeyword(Read),
        "readonly" => IdentifierOrKeyword(Readonly),
        "record" => Keyword(Record),
        "reference" => IdentifierOrKeyword(Reference),
        "register" => IdentifierOrKeyword(Register),
        "reintroduce" => IdentifierOrKeyword(Reintroduce),
        "remove" => IdentifierOrKeyword(Remove),
        "repeat" => Keyword(Repeat),
        "requires" => IdentifierOrKeyword(Requires),
        "resident" => IdentifierOrKeyword(Resident),
        "resourcestring" => Keyword(ResourceString),
        "safecall" => IdentifierOrKeyword(SafeCall),
        "sealed" => IdentifierOrKeyword(Sealed),
        "set" => Keyword(Set),
        "shl" => Op(Shl),
        "shr" => Op(Shr),
        "static" => IdentifierOrKeyword(Static),
        "stdcall" => IdentifierOrKeyword(StdCall),
        "stored" => IdentifierOrKeyword(Stored),
        "strict" => IdentifierOrKeyword(Strict),
        "then" => Keyword(Then),
        "threadvar" => Keyword(ThreadVar),
        "to" => Keyword(To),
        "try" => Keyword(Try),
        "type" => Keyword(Type),
        "unit" => Keyword(Unit),
        "unsafe" => IdentifierOrKeyword(Unsafe),
        "until" => Keyword(Until),
        "uses" => Keyword(Uses),
        "var" => Keyword(Var),
        "varargs" => IdentifierOrKeyword(Varargs),
        "variant" => IdentifierOrKeyword(Variant),
        "virtual" => IdentifierOrKeyword(Virtual),
        "while" => Keyword(While),
        "with" => Keyword(With),
        "write" => IdentifierOrKeyword(Write),
        "writeonly" => IdentifierOrKeyword(WriteOnly),
        "xor" => Op(Xor),
        _ => Identifier,
    }
}

fn get_operator_token_type(input: &str) -> TokenType {
    match input {
        "+" => Op(Plus),
        "-" => Op(Minus),
        "*" => Op(Star),
        "/" => Op(Slash),
        ":=" => Op(Assign),
        "," => Op(Comma),
        ";" => Op(Semicolon),
        ":" => Op(Colon),
        "=" => Op(Equal),
        "<>" => Op(NotEqual),
        "<" => Op(LessThan),
        "<=" => Op(LessEqual),
        ">" => Op(GreaterThan),
        ">=" => Op(GreaterEqual),
        "(." => Op(LBrack),
        ".)" => Op(RBrack),
        "[" => Op(LBrack),
        "]" => Op(RBrack),
        "(" => Op(LParen),
        ")" => Op(RParen),
        "^" => Op(Pointer),
        "@" => Op(AddressOf),
        "." => Op(Dot),
        ".." => Op(DotDot),

        _ => panic!("Unknown operator token {}", input),
    }
}

fn is_newline(input: char) -> bool {
    input == '\n' || input == '\r'
}

fn get_final_token_type(
    token_index: usize,
    token_type: TokenType,
    leading_whitespace: &str,
) -> TokenType {
    match token_type {
        Comment(CommentKind::Block) => {
            match leading_whitespace.contains('\n') || token_index == 0 {
                false => Comment(CommentKind::InlineBlock),
                true => Comment(CommentKind::IndividualBlock),
            }
        }
        Comment(CommentKind::Line) => match leading_whitespace.contains('\n') || token_index == 0 {
            false => Comment(CommentKind::InlineLine),
            true => Comment(CommentKind::IndividualLine),
        },
        other => other,
    }
}

// Fragments

fn take_whitespace(input: &str) -> IResult<&str, &str> {
    take_while(char::is_whitespace)(input)
}

fn scale_factor(input: &str) -> IResult<&str, &str> {
    recognize(tuple((
        tag_no_case("e"),
        opt(alt((char('+'), char('-')))),
        digit1,
    )))(input)
}

fn binary_digit(input: &str) -> IResult<&str, char> {
    alt((char('0'), char('1')))(input)
}

fn binary_digit_sequence(input: &str) -> IResult<&str, &str> {
    recognize(many0(alt((binary_digit, char('_')))))(input)
}

fn digit_sequence(input: &str) -> IResult<&str, &str> {
    recognize(tuple((digit1, many0(alt((digit1, tag("_")))))))(input)
}

fn escaped_character(input: &str) -> IResult<&str, &str> {
    recognize(tuple((
        char('#'),
        alt((
            digit_sequence,
            recognize(tuple((char('%'), hex_digit0))),
            recognize(tuple((char('$'), binary_digit_sequence))),
        )),
    )))(input)
}

fn identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        alt((alpha1, tag("_"))),
        many0_count(alt((alphanumeric1, tag("_")))),
    ))(input)
}

fn identifier_or_keyword(input: &str) -> IResult<&str, &str> {
    alt((
        identifier,
        recognize(tuple((char('&'), identifier))),
        recognize(tuple((tag("&&"), opt(identifier)))),
    ))(input)
}

fn quoted_string(input: &str) -> IResult<&str, &str> {
    delimited(
        tag("'"),
        recognize(many0(alt((is_not("'"), tag("''"))))),
        tag("'"),
    )(input)
}
// Tokens

fn unknown(input: &str) -> IResult<&str, ContentAndTokenType> {
    map(take(1usize), |token| (token, Unknown))(input)
}

fn eof_token(input: &str) -> IResult<&str, WhitespaceAndToken> {
    map(take_whitespace, |whitespace| (whitespace, ("", Eof)))(input)
}

fn asm_label(input: &str) -> IResult<&str, ContentAndTokenType> {
    map(recognize(tuple((many1(char('@')), identifier))), |text| {
        (text, TokenType::Identifier)
    })(input)
}

fn quoted_asm_string(input: &str) -> IResult<&str, &str> {
    delimited(
        tag("\""),
        // inline assembly escapes double quotes using backslashes
        recognize(many0(alt((tag("\\\""), is_not("\""))))),
        tag("\""),
    )(input)
}

fn asm_string_literal(input: &str) -> IResult<&str, ContentAndTokenType> {
    map(recognize(quoted_asm_string), |text| {
        (text, TokenType::TextLiteral)
    })(input)
}

fn asm_identifier(input: &str) -> IResult<&str, ContentAndTokenType> {
    let (input, ident) = identifier(input)?;

    if ident.eq_ignore_ascii_case("end") {
        fail(input)
    } else {
        Ok((input, (ident, TokenType::Identifier)))
    }
}

fn asm_block(input: &str) -> IResult<&str, Vec<WhitespaceAndToken>> {
    let (mut input, asm) = tuple((
        take_whitespace,
        tag_no_case("asm").map(|text| (text, Keyword(Asm))),
    ))(input)?;
    let mut tokens = vec![asm];

    while let Ok((next_input, next)) = pair(
        take_whitespace,
        alt((
            asm_identifier,
            asm_label,
            line_comment,
            compiler_directive,
            block_comment,
            operator,
            number_literal,
            text_literal,
            asm_string_literal,
        )),
    )(input)
    {
        input = next_input;
        tokens.push(next);
    }

    let (input, end) = tuple((
        take_whitespace,
        tag_no_case("end").map(|text| (text, Keyword(End))),
    ))(input)?;
    tokens.push(end);

    Ok((input, tokens))
}

fn identifier_or_keyword_and_type(input: &str) -> IResult<&str, ContentAndTokenType> {
    map(identifier_or_keyword, |token| {
        (token, get_word_token_type(token))
    })(input)
}

fn text_literal(input: &str) -> IResult<&str, ContentAndTokenType> {
    map(
        alt((
            recognize(tuple((
                quoted_string,
                many0(tuple((many1(escaped_character), quoted_string))),
                many0(escaped_character),
            ))),
            recognize(tuple((
                many1(escaped_character),
                many0(tuple((quoted_string, many1(escaped_character)))),
                opt(quoted_string),
            ))),
        )),
        |value: &str| (value, TextLiteral),
    )(input)
}

fn number_literal(input: &str) -> IResult<&str, ContentAndTokenType> {
    alt((
        map(
            recognize(tuple((
                opt(char('&')),
                digit_sequence,
                opt(pair(char('.'), digit_sequence)),
                opt(scale_factor),
            ))),
            |value: &str| (value, NumberLiteral(Decimal)),
        ),
        map(
            recognize(tuple((opt(char('&')), char('%'), hex_digit0))),
            |value: &str| (value, NumberLiteral(Hex)),
        ),
        map(
            recognize(tuple((opt(char('&')), char('$'), binary_digit_sequence))),
            |value: &str| (value, NumberLiteral(Binary)),
        ),
    ))(input)
}

fn compiler_directive_identifier(input: &str) -> IResult<&str, ContentAndTokenType> {
    let (remaining, text) = alt((
        tag_no_case("ifdef"),
        tag_no_case("ifndef"),
        tag_no_case("ifopt"),
        tag_no_case("elseif"),
        tag_no_case("else"),
        tag_no_case("ifend"),
        tag_no_case("endif"),
        tag_no_case("if"),
        identifier_or_keyword,
    ))(input)?;
    let token_type = match text.to_lowercase().as_str() {
        "if" => ConditionalDirective(ConditionalDirectiveKind::If),
        "ifdef" => ConditionalDirective(Ifdef),
        "ifndef" => ConditionalDirective(Ifndef),
        "ifopt" => ConditionalDirective(Ifopt),
        "elseif" => ConditionalDirective(Elseif),
        "else" => ConditionalDirective(ConditionalDirectiveKind::Else),
        "ifend" => ConditionalDirective(Ifend),
        "endif" => ConditionalDirective(Endif),
        _ => CompilerDirective,
    };
    Ok((remaining, (text, token_type)))
}

fn compiler_directive(input: &str) -> IResult<&str, ContentAndTokenType> {
    let mut parts = alt((
        tuple((
            tag("{$"),
            compiler_directive_identifier,
            take_until("}"),
            tag("}"),
        )),
        tuple((
            tag("(*$"),
            compiler_directive_identifier,
            take_until("*)"),
            tag("*)"),
        )),
    ));
    let (_, (_, (_, token_type), _, _)) = parts(input)?;
    let (remaining, token_text) = recognize(parts)(input)?;
    Ok((remaining, (token_text, token_type)))
}

fn block_comment(input: &str) -> IResult<&str, ContentAndTokenType> {
    map(
        recognize(alt((
            tuple((tag("{"), take_until("}"), tag("}"))),
            tuple((tag("(*"), take_until("*)"), tag("*)"))),
        ))),
        |result: &str| {
            (
                result,
                match result.contains('\n') {
                    false => Comment(CommentKind::Block),
                    true => Comment(CommentKind::MultilineBlock),
                },
            )
        },
    )(input)
}

fn line_comment(input: &str) -> IResult<&str, ContentAndTokenType> {
    map(
        recognize(tuple((tag("//"), take_till(is_newline)))),
        |result| (result, Comment(CommentKind::Line)),
    )(input)
}

fn operator(input: &str) -> IResult<&str, ContentAndTokenType> {
    map(
        alt((
            alt((
                tag("+"),
                tag("-"),
                tag("*"),
                tag("/"),
                tag(":="),
                tag(","),
                tag(";"),
                tag(":"),
                tag("="),
                tag("<>"),
                tag("<="),
                tag("<"),
                tag(">="),
            )),
            alt((
                tag(">"),
                tag("(."),
                tag(".)"),
                tag("["),
                tag("]"),
                tag("("),
                tag(")"),
                tag("^"),
                tag("@"),
                tag(".."),
                tag("."),
            )),
        )),
        |token| (token, get_operator_token_type(token)),
    )(input)
}

fn whitespace_and_token(input: &str) -> IResult<&str, WhitespaceAndToken> {
    pair(
        take_whitespace,
        alt((
            compiler_directive,
            block_comment,
            line_comment,
            text_literal,
            number_literal,
            identifier_or_keyword_and_type,
            operator,
            unknown,
        )),
    )(input)
}

fn parse_delphi_file(mut input: &str) -> IResult<&str, Vec<WhitespaceAndToken>> {
    let mut result = vec![];
    loop {
        if let Ok((remaining, new_result)) = asm_block(input) {
            result.extend(new_result);
            input = remaining;
        } else if let Ok((remaining, new_result)) = whitespace_and_token(input) {
            result.push(new_result);
            input = remaining;
        } else {
            break;
        }
    }
    Ok((input, result))
}

fn parse_file(input: &str) -> Vec<Token> {
    let (remaining, mut token_data) = parse_delphi_file(input).unwrap();
    let (remaining, eof_token_data) = eof_token(remaining).unwrap();
    token_data.push(eof_token_data);

    assert!(remaining.is_empty(), "Failed to lex the entire file");
    token_data
        .iter()
        .filter(|(_, (_, token_type))| token_type == &Unknown)
        .for_each(|(_, (content, _))| warn!("Found unknown token '{}'", content));

    token_data
        .into_iter()
        .enumerate()
        .map(|(index, (leading_whitespace, (content, token_type)))| {
            Token::RefToken(RefToken::new(
                index,
                leading_whitespace,
                content,
                get_final_token_type(index, token_type, leading_whitespace),
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use spectral::prelude::*;

    fn run_test(input: &str, expected_token_types: Vec<ContentAndTokenType>) {
        let lexer = DelphiLexer {};
        let tokens = lexer.lex(input);
        let token_types: Vec<_> = tokens
            .iter()
            .filter(|token| token.get_token_type() != Eof)
            .map(|token| (token.get_content(), token.get_token_type()))
            .collect();

        assert_that(&token_types).is_equal_to(&expected_token_types);
    }

    fn alternating_case(input: &str) -> String {
        input
            .char_indices()
            .map(|(index, char)| match index % 2 {
                0 => char.to_ascii_uppercase(),
                _ => char.to_ascii_lowercase(),
            })
            .collect::<String>()
    }

    fn run_casing_test((input, expected_token_type): ContentAndTokenType) {
        let lowercase = input.to_ascii_lowercase();
        let uppercase = input.to_ascii_uppercase();
        let alternating = alternating_case(input);
        let input = format!("{} {} {}", lowercase, uppercase, alternating);
        run_test(
            input.as_str(),
            vec![
                (lowercase.as_str(), expected_token_type),
                (uppercase.as_str(), expected_token_type),
                (alternating.as_str(), expected_token_type),
            ],
        )
    }

    #[test]
    fn parse_block_comment_types() {
        run_test(
            indoc! {"
                {block comment} {.$fake compiler directive} \
                (*star block comment*) (*.$fake compiler star directive*) {*)} (*{*) \
                {
                    Multiline block comment
                }"
            },
            vec![
                ("{block comment}", Comment(CommentKind::IndividualBlock)),
                (
                    "{.$fake compiler directive}",
                    Comment(CommentKind::InlineBlock),
                ),
                ("(*star block comment*)", Comment(CommentKind::InlineBlock)),
                (
                    "(*.$fake compiler star directive*)",
                    Comment(CommentKind::InlineBlock),
                ),
                ("{*)}", Comment(CommentKind::InlineBlock)),
                ("(*{*)", Comment(CommentKind::InlineBlock)),
                (
                    indoc! {"
                        {
                            Multiline block comment
                        }"
                    },
                    Comment(CommentKind::MultilineBlock),
                ),
            ],
        );
    }

    #[test]
    fn parse_block_comments() {
        run_test(
            indoc! {"
                {individual block}
                {individual block}
                ; {inline block}"
            },
            vec![
                ("{individual block}", Comment(CommentKind::IndividualBlock)),
                ("{individual block}", Comment(CommentKind::IndividualBlock)),
                (";", Op(Semicolon)),
                ("{inline block}", Comment(CommentKind::InlineBlock)),
            ],
        );
    }

    #[test]
    fn parse_line_comments() {
        run_test(
            indoc! {"
                // Individual line comment 1
                // Individual line comment 2
                ; // Inline line comment"
            },
            vec![
                (
                    "// Individual line comment 1",
                    Comment(CommentKind::IndividualLine),
                ),
                (
                    "// Individual line comment 2",
                    Comment(CommentKind::IndividualLine),
                ),
                (";", Op(Semicolon)),
                ("// Inline line comment", Comment(CommentKind::InlineLine)),
            ],
        );
    }

    #[test]
    fn parse_compiler_directives() {
        run_test(
            "(*$message*) {$foo *) } (*$bar aa {}*)",
            vec![
                ("(*$message*)", CompilerDirective),
                ("{$foo *) }", CompilerDirective),
                ("(*$bar aa {}*)", CompilerDirective),
            ],
        );
        [
            ("{$if}", ConditionalDirective(ConditionalDirectiveKind::If)),
            ("{$ifdef}", ConditionalDirective(Ifdef)),
            ("{$ifndef}", ConditionalDirective(Ifndef)),
            ("{$ifopt}", ConditionalDirective(Ifopt)),
            ("{$elseif}", ConditionalDirective(Elseif)),
            (
                "{$else}",
                ConditionalDirective(ConditionalDirectiveKind::Else),
            ),
            ("{$ifend}", ConditionalDirective(Ifend)),
            ("{$endif}", ConditionalDirective(Endif)),
            (
                "(*$if*)",
                ConditionalDirective(ConditionalDirectiveKind::If),
            ),
            ("(*$ifdef*)", ConditionalDirective(Ifdef)),
            ("(*$ifndef*)", ConditionalDirective(Ifndef)),
            ("(*$ifopt*)", ConditionalDirective(Ifopt)),
            ("(*$elseif*)", ConditionalDirective(Elseif)),
            (
                "(*$else*)",
                ConditionalDirective(ConditionalDirectiveKind::Else),
            ),
            ("(*$ifend*)", ConditionalDirective(Ifend)),
            ("(*$endif*)", ConditionalDirective(Endif)),
        ]
        .into_iter()
        .for_each(run_casing_test);
    }

    #[test]
    fn parse_string_literals() {
        run_test(
            "'string' 'string''part2' 'ab''''cd' 'abc'#13#10 'after escaped stuff'",
            vec![
                ("'string'", TextLiteral),
                ("'string''part2'", TextLiteral),
                ("'ab''''cd'", TextLiteral),
                ("'abc'#13#10", TextLiteral),
                ("'after escaped stuff'", TextLiteral),
            ],
        );
    }

    #[test]
    fn parse_decimal_number_literals() {
        run_test(
            "0 0.0 10 1_000 1_000.00 1_111_111.11",
            vec![
                ("0", NumberLiteral(Decimal)),
                ("0.0", NumberLiteral(Decimal)),
                ("10", NumberLiteral(Decimal)),
                ("1_000", NumberLiteral(Decimal)),
                ("1_000.00", NumberLiteral(Decimal)),
                ("1_111_111.11", NumberLiteral(Decimal)),
            ],
        );
    }

    #[test]
    fn parse_hex_number_literal() {
        run_test(
            "% %00 %FF",
            vec![
                ("%", NumberLiteral(Hex)),
                ("%00", NumberLiteral(Hex)),
                ("%FF", NumberLiteral(Hex)),
            ],
        );
    }

    #[test]
    fn parse_binary_number_literals() {
        run_test(
            "$ $0 $1 $1111_0000",
            vec![
                ("$", NumberLiteral(Binary)),
                ("$0", NumberLiteral(Binary)),
                ("$1", NumberLiteral(Binary)),
                ("$1111_0000", NumberLiteral(Binary)),
            ],
        );
    }

    #[test]
    fn parse_ampersand_integer_literals() {
        // Only the &0 case is valid according to our compiler, not that it makes any sense, but we figure that
        // the other cases should be lexed in the same way (even if they are invalid).
        run_test(
            "&%FF &$0 &0",
            vec![
                ("&%FF", NumberLiteral(Hex)),
                ("&$0", NumberLiteral(Binary)),
                ("&0", NumberLiteral(Decimal)),
            ],
        );
    }

    #[test]
    fn parse_identifiers() {
        run_test(
            "Foo _Foo _1Foo &begin &&op_Addition &&",
            vec![
                ("Foo", Identifier),
                ("_Foo", Identifier),
                ("_1Foo", Identifier),
                ("&begin", Identifier),
                ("&&op_Addition", Identifier),
                // You can't actually use this as an identifier, but in some contexts it's valid yet ignored.
                ("&&", Identifier),
            ],
        );
    }

    #[test]
    fn parse_operators() {
        run_test(
            "+-*/:=,;=:<><<=>=>[](..)()^@... mod div shl shr and as in or xor not is",
            vec![
                ("+", Op(Plus)),
                ("-", Op(Minus)),
                ("*", Op(Star)),
                ("/", Op(Slash)),
                (":=", Op(Assign)),
                (",", Op(Comma)),
                (";", Op(Semicolon)),
                ("=", Op(Equal)),
                (":", Op(Colon)),
                ("<>", Op(NotEqual)),
                ("<", Op(LessThan)),
                ("<=", Op(LessEqual)),
                (">=", Op(GreaterEqual)),
                (">", Op(GreaterThan)),
                ("[", Op(LBrack)),
                ("]", Op(RBrack)),
                ("(.", Op(LBrack)),
                (".)", Op(RBrack)),
                ("(", Op(LParen)),
                (")", Op(RParen)),
                ("^", Op(Pointer)),
                ("@", Op(AddressOf)),
                ("..", Op(DotDot)),
                (".", Op(Dot)),
                ("mod", Op(Mod)),
                ("div", Op(Div)),
                ("shl", Op(Shl)),
                ("shr", Op(Shr)),
                ("and", Op(And)),
                ("as", Op(As)),
                ("in", Op(In)),
                ("or", Op(Or)),
                ("xor", Op(Xor)),
                ("not", Op(Not)),
                ("is", Op(Is)),
            ],
        );
    }

    #[test]
    fn parse_keywords() {
        [
            ("absolute", IdentifierOrKeyword(Absolute)),
            ("abstract", IdentifierOrKeyword(Abstract)),
            ("add", IdentifierOrKeyword(Add)),
            ("align", IdentifierOrKeyword(Align)),
            ("array", Keyword(Array)),
            ("assembler", IdentifierOrKeyword(Assembler)),
            ("at", IdentifierOrKeyword(At)),
            ("automated", IdentifierOrKeyword(Automated)),
            ("begin", Keyword(Begin)),
            ("case", Keyword(Case)),
            ("cdecl", IdentifierOrKeyword(Cdecl)),
            ("class", Keyword(Class)),
            ("const", Keyword(Const)),
            ("constructor", Keyword(Constructor)),
            ("contains", IdentifierOrKeyword(Contains)),
            ("default", IdentifierOrKeyword(Default)),
            ("delayed", IdentifierOrKeyword(Delayed)),
            ("deprecated", IdentifierOrKeyword(Deprecated)),
            ("destructor", Keyword(Destructor)),
            ("dispid", IdentifierOrKeyword(Dispid)),
            ("dispinterface", Keyword(Dispinterface)),
            ("do", Keyword(Do)),
            ("downto", Keyword(Downto)),
            ("dynamic", IdentifierOrKeyword(Dynamic)),
            ("else", Keyword(PureKeywordKind::Else)),
            ("end", Keyword(End)),
            ("except", Keyword(Except)),
            ("experimental", IdentifierOrKeyword(Experimental)),
            ("export", IdentifierOrKeyword(Export)),
            ("exports", Keyword(Exports)),
            ("external", IdentifierOrKeyword(External)),
            ("far", IdentifierOrKeyword(Far)),
            ("file", Keyword(File)),
            ("final", IdentifierOrKeyword(Final)),
            ("finalization", Keyword(Finalization)),
            ("finally", Keyword(Finally)),
            ("for", Keyword(For)),
            ("forward", IdentifierOrKeyword(Forward)),
            ("function", Keyword(Function)),
            ("goto", Keyword(Goto)),
            ("helper", IdentifierOrKeyword(Helper)),
            ("if", Keyword(PureKeywordKind::If)),
            ("implementation", Keyword(Implementation)),
            ("implements", IdentifierOrKeyword(Implements)),
            ("index", IdentifierOrKeyword(Index)),
            ("inherited", Keyword(Inherited)),
            ("initialization", Keyword(Initialization)),
            ("inline", Keyword(Inline)),
            ("interface", Keyword(Interface)),
            ("label", Keyword(Label)),
            ("library", Keyword(Library)),
            ("local", IdentifierOrKeyword(Local)),
            ("message", IdentifierOrKeyword(Message)),
            ("name", IdentifierOrKeyword(Name)),
            ("near", IdentifierOrKeyword(Near)),
            ("nil", Keyword(Nil)),
            ("nodefault", IdentifierOrKeyword(NoDefault)),
            ("object", Keyword(Object)),
            ("of", Keyword(Of)),
            ("on", IdentifierOrKeyword(On)),
            ("operator", IdentifierOrKeyword(Operator)),
            ("out", IdentifierOrKeyword(Out)),
            ("overload", IdentifierOrKeyword(Overload)),
            ("override", IdentifierOrKeyword(Override)),
            ("package", IdentifierOrKeyword(Package)),
            ("packed", Keyword(Packed)),
            ("pascal", IdentifierOrKeyword(Pascal)),
            ("platform", IdentifierOrKeyword(Platform)),
            ("private", IdentifierOrKeyword(Private)),
            ("procedure", Keyword(Procedure)),
            ("program", Keyword(Program)),
            ("property", Keyword(Property)),
            ("protected", IdentifierOrKeyword(Protected)),
            ("public", IdentifierOrKeyword(Public)),
            ("published", IdentifierOrKeyword(Published)),
            ("raise", Keyword(Raise)),
            ("read", IdentifierOrKeyword(Read)),
            ("readonly", IdentifierOrKeyword(Readonly)),
            ("record", Keyword(Record)),
            ("reference", IdentifierOrKeyword(Reference)),
            ("register", IdentifierOrKeyword(Register)),
            ("reintroduce", IdentifierOrKeyword(Reintroduce)),
            ("remove", IdentifierOrKeyword(Remove)),
            ("repeat", Keyword(Repeat)),
            ("requires", IdentifierOrKeyword(Requires)),
            ("resident", IdentifierOrKeyword(Resident)),
            ("resourcestring", Keyword(ResourceString)),
            ("safecall", IdentifierOrKeyword(SafeCall)),
            ("sealed", IdentifierOrKeyword(Sealed)),
            ("set", Keyword(Set)),
            ("static", IdentifierOrKeyword(Static)),
            ("stdcall", IdentifierOrKeyword(StdCall)),
            ("stored", IdentifierOrKeyword(Stored)),
            ("strict", IdentifierOrKeyword(Strict)),
            ("then", Keyword(Then)),
            ("threadvar", Keyword(ThreadVar)),
            ("to", Keyword(To)),
            ("try", Keyword(Try)),
            ("type", Keyword(Type)),
            ("unit", Keyword(Unit)),
            ("unsafe", IdentifierOrKeyword(Unsafe)),
            ("until", Keyword(Until)),
            ("uses", Keyword(Uses)),
            ("var", Keyword(Var)),
            ("varargs", IdentifierOrKeyword(Varargs)),
            ("variant", IdentifierOrKeyword(Variant)),
            ("virtual", IdentifierOrKeyword(Virtual)),
            ("while", Keyword(While)),
            ("with", Keyword(With)),
            ("write", IdentifierOrKeyword(Write)),
            ("writeonly", IdentifierOrKeyword(WriteOnly)),
        ]
        .into_iter()
        .for_each(run_casing_test);
    }

    #[test]
    fn parse_function_declaration() {
        run_test(
            "function Foo(Arg1:String;Arg2:Bar);stdcall;",
            vec![
                ("function", Keyword(Function)),
                ("Foo", Identifier),
                ("(", Op(LParen)),
                ("Arg1", Identifier),
                (":", Op(Colon)),
                ("String", Identifier),
                (";", Op(Semicolon)),
                ("Arg2", Identifier),
                (":", Op(Colon)),
                ("Bar", Identifier),
                (")", Op(RParen)),
                (";", Op(Semicolon)),
                ("stdcall", IdentifierOrKeyword(StdCall)),
                (";", Op(Semicolon)),
            ],
        );
    }

    #[test]
    fn parse_invalid_code() {
        run_test(
            "? ? ?",
            vec![("?", Unknown), ("?", Unknown), ("?", Unknown)],
        );
    }

    #[test]
    fn inline_assembly_with_end_in_label() {
        run_test(
            indoc! {"
            asm
            @@end:
                XOR RBX, RBX
            end
            "},
            vec![
                ("asm", Keyword(Asm)),
                ("@@end", Identifier),
                (":", Op(Colon)),
                ("XOR", Identifier),
                ("RBX", Identifier),
                (",", Op(Comma)),
                ("RBX", Identifier),
                ("end", Keyword(End)),
            ],
        );
    }
    #[test]
    fn inline_assembly_with_end_in_ifdef() {
        run_test(
            indoc! {"
            asm
                XOR RBX, RBX {$ifdef End}
            end
            "},
            vec![
                ("asm", Keyword(Asm)),
                ("XOR", Identifier),
                ("RBX", Identifier),
                (",", Op(Comma)),
                ("RBX", Identifier),
                ("{$ifdef End}", ConditionalDirective(Ifdef)),
                ("end", Keyword(End)),
            ],
        );
    }
    #[test]
    fn inline_assembly_with_end_in_comment() {
        run_test(
            indoc! {"
            asm
                XOR RBX, RBX // End
            end
            "},
            vec![
                ("asm", Keyword(Asm)),
                ("XOR", Identifier),
                ("RBX", Identifier),
                (",", Op(Comma)),
                ("RBX", Identifier),
                ("// End", Comment(CommentKind::InlineLine)),
                ("end", Keyword(End)),
            ],
        );
    }
    #[test]
    fn inline_assembly_with_end_in_keyword() {
        run_test(
            indoc! {"
            asm
                XOR RBX, IfEnd
            end
            "},
            vec![
                ("asm", Keyword(Asm)),
                ("XOR", Identifier),
                ("RBX", Identifier),
                (",", Op(Comma)),
                ("IfEnd", Identifier),
                ("end", Keyword(End)),
            ],
        );
    }
    #[test]
    fn inline_assembly_with_labels() {
        run_test(
            indoc! {"
            asm
            @@ClearRBX:
                XOR RBX, RBX
            end
            "},
            vec![
                ("asm", Keyword(Asm)),
                ("@@ClearRBX", Identifier),
                (":", Op(Colon)),
                ("XOR", Identifier),
                ("RBX", Identifier),
                (",", Op(Comma)),
                ("RBX", Identifier),
                ("end", Keyword(End)),
            ],
        );
    }
    #[test]
    fn inline_assembly_with_double_quotes() {
        run_test(
            indoc! {"
            asm
                CMP AL,\"'\"
                XOR RBX, RBX
            end
            "},
            vec![
                ("asm", Keyword(Asm)),
                ("CMP", Identifier),
                ("AL", Identifier),
                (",", Op(Comma)),
                ("\"'\"", TextLiteral),
                ("XOR", Identifier),
                ("RBX", Identifier),
                (",", Op(Comma)),
                ("RBX", Identifier),
                ("end", Keyword(End)),
            ],
        );
    }

    #[test]
    fn inline_assembly_with_escaped_double_quotes() {
        run_test(
            indoc! {r#"
            asm
                CMP AL,"\""
            end
            "#},
            vec![
                ("asm", Keyword(Asm)),
                ("CMP", Identifier),
                ("AL", Identifier),
                (",", Op(Comma)),
                (r#""\"""#, TextLiteral),
                ("end", Keyword(End)),
            ],
        );
    }

    #[test]
    fn inline_assembly_with_comments() {
        run_test(
            indoc! {"
            asm
                MOV RAX, 0 // comment
                XOR RBX, RBX
            end
            "},
            vec![
                ("asm", Keyword(Asm)),
                ("MOV", Identifier),
                ("RAX", Identifier),
                (",", Op(Comma)),
                ("0", NumberLiteral(Decimal)),
                ("// comment", Comment(CommentKind::InlineLine)),
                ("XOR", Identifier),
                ("RBX", Identifier),
                (",", Op(Comma)),
                ("RBX", Identifier),
                ("end", Keyword(End)),
            ],
        );
    }
    #[test]
    fn inline_assembly() {
        run_test(
            indoc! {"
            asm
                MOV RAX, 0
                XOR RBX, RBX
            end
            "},
            vec![
                ("asm", Keyword(Asm)),
                ("MOV", Identifier),
                ("RAX", Identifier),
                (",", Op(Comma)),
                ("0", NumberLiteral(Decimal)),
                ("XOR", Identifier),
                ("RBX", Identifier),
                (",", Op(Comma)),
                ("RBX", Identifier),
                ("end", Keyword(End)),
            ],
        );
    }
}
