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

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum LogicalLineType {
    ConditionalDirective,
    Eof,
    UsesClause,
    Unknown,
}
#[derive(Debug, PartialEq, Eq)]
pub struct LogicalLine {
    parent_token: Option<usize>,
    level: usize,
    tokens: Vec<usize>,
    line_type: LogicalLineType,
}
#[allow(dead_code)]
impl LogicalLine {
    pub fn new(
        parent_token: Option<usize>,
        level: usize,
        tokens: Vec<usize>,
        line_type: LogicalLineType,
    ) -> Self {
        LogicalLine {
            parent_token,
            level,
            tokens,
            line_type,
        }
    }
    pub fn get_parent_token(&self) -> Option<usize> {
        self.parent_token
    }
    pub fn get_level(&self) -> usize {
        self.level
    }
    pub fn get_tokens(&self) -> &Vec<usize> {
        &self.tokens
    }
    pub fn get_tokens_mut(&mut self) -> &mut Vec<usize> {
        &mut self.tokens
    }
    pub fn get_line_type(&self) -> LogicalLineType {
        self.line_type.clone()
    }
}
#[allow(dead_code)]
pub struct LogicalLines<'a> {
    tokens: Vec<Token<'a>>,
    lines: Vec<LogicalLine>,
}
#[allow(dead_code)]
impl<'a> LogicalLines<'a> {
    pub fn new(tokens: Vec<Token<'a>>, lines: Vec<LogicalLine>) -> Self {
        LogicalLines { tokens, lines }
    }
    pub fn get_tokens(&self) -> &Vec<Token<'a>> {
        &self.tokens
    }
    pub fn get_lines(&self) -> &Vec<LogicalLine> {
        &self.lines
    }
    pub fn own_data(self) -> (Vec<Token<'a>>, Vec<LogicalLine>) {
        (self.tokens, self.lines)
    }
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
