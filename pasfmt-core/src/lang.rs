#[derive(Debug, PartialEq, Eq, Clone)]
pub enum IdentifierOrKeywordKind {
    Absolute,
    Abstract,
    Add,
    Align,
    Assembler,
    At,
    Automated,
    Cdecl,
    Contains,
    Default,
    Delayed,
    Deprecated,
    Dispid,
    Dynamic,
    Experimental,
    Export,
    External,
    Far,
    Final,
    Forward,
    Helper,
    Implements,
    Index,
    Local,
    Message,
    Name,
    Near,
    NoDefault,
    On,
    Operator,
    Out,
    Overload,
    Override,
    Package,
    Pascal,
    Platform,
    Private,
    Protected,
    Public,
    Published,
    Read,
    Readonly,
    Reference,
    Register,
    Reintroduce,
    Remove,
    Requires,
    Resident,
    SafeCall,
    Sealed,
    Static,
    StdCall,
    Stored,
    Strict,
    Unsafe,
    Varargs,
    Variant,
    Virtual,
    Write,
    WriteOnly,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PureKeywordKind {
    And,
    Array,
    As,
    Asm,
    Begin,
    Case,
    Class,
    Const,
    Constructor,
    Destructor,
    Dispinterface,
    Div,
    Do,
    Downto,
    Else,
    End,
    Except,
    Exports,
    File,
    Finalization,
    Finally,
    For,
    Function,
    Goto,
    If,
    Implementation,
    In,
    Inherited,
    Initialization,
    Inline,
    Interface,
    Is,
    Label,
    Library,
    Mod,
    Nil,
    Not,
    Object,
    Of,
    Or,
    Packed,
    Procedure,
    Program,
    Property,
    Raise,
    Record,
    Repeat,
    ResourceString,
    Set,
    Shl,
    Shr,
    Then,
    ThreadVar,
    To,
    Try,
    Type,
    Unit,
    Until,
    Uses,
    Var,
    While,
    With,
    Xor,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum OperatorKind {
    Plus,
    Minus,
    Star,
    Slash,
    Assign,
    Comma,
    Semicolon,
    Colon,
    Equal,
    NotEqual,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
    LBrack,
    RBrack,
    LParen,
    RParen,
    Pointer,
    AddressOf,
    Dot,
    DotDot,
    Ampersand,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum NumberLiteralKind {
    Decimal,
    Hex,
    Binary,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CommentKind {
    Block,
    InlineBlock,
    IndividualBlock,
    MultilineBlock,
    Line,
    InlineLine,
    IndividualLine,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ConditionalDirectiveKind {
    If,
    Ifdef,
    Ifndef,
    Ifopt,
    Elseif,
    Else,
    Ifend,
    Endif,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TokenType {
    Op(OperatorKind),
    Identifier,
    IdentifierOrKeyword(IdentifierOrKeywordKind),
    Keyword(PureKeywordKind),
    TextLiteral,
    NumberLiteral(NumberLiteralKind),
    ConditionalDirective(ConditionalDirectiveKind),
    CompilerDirective,
    Comment(CommentKind),
    Eof,
    Unknown,
}

pub struct RefToken<'a> {
    index: usize,
    original_leading_whitespace: &'a str,
    content: &'a str,
    token_type: TokenType,
}
impl<'a> RefToken<'a> {
    pub fn new(
        index: usize,
        leading_whitespace: &'a str,
        content: &'a str,
        token_type: TokenType,
    ) -> RefToken<'a> {
        RefToken {
            index,
            original_leading_whitespace: leading_whitespace,
            content,
            token_type,
        }
    }
}

pub enum Token<'a> {
    RefToken(RefToken<'a>),
}
#[allow(dead_code)]
impl<'a> Token<'a> {
    pub fn get_index(&self) -> usize {
        match &self {
            Token::RefToken(token) => token.index,
        }
    }
    pub fn get_leading_whitespace(&self) -> &str {
        match &self {
            Token::RefToken(token) => token.original_leading_whitespace,
        }
    }
    pub fn get_content(&self) -> &str {
        match &self {
            Token::RefToken(token) => token.content,
        }
    }
    pub fn get_token_type(&self) -> TokenType {
        match &self {
            Token::RefToken(token) => token.token_type.clone(),
        }
    }
}
