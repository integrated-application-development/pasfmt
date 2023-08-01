use crate::traits::{LogicalLineFileFormatter, LogicalLineFormatter};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
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

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum PureKeywordKind {
    Array,
    Asm,
    Begin,
    Case,
    Class,
    Const,
    Constructor,
    Destructor,
    Dispinterface,
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
    Inherited,
    Initialization,
    Inline,
    Interface,
    Label,
    Library,
    Nil,
    Object,
    Of,
    Packed,
    Procedure,
    Program,
    Property,
    Raise,
    Record,
    Repeat,
    ResourceString,
    Set,
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
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
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
    Mod,
    Div,
    Shl,
    Shr,
    And,
    As,
    In,
    Or,
    Xor,
    Not,
    Is,
    LGeneric,
    RGeneric,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum NumberLiteralKind {
    Decimal,
    Hex,
    Binary,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum CommentKind {
    Block,
    InlineBlock,
    IndividualBlock,
    MultilineBlock,
    Line,
    InlineLine,
    IndividualLine,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
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

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
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

#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub enum LogicalLineType {
    ConditionalDirective,
    Eof,
    UsesClause,
    AsmInstruction,
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
        self.line_type
    }
}

#[allow(dead_code)]
#[derive(Default)]
pub struct FormattingData {
    pub ignored: bool,
    pub newlines_before: usize,
    pub indentations_before: usize,
    pub continuations_before: usize,
    pub spaces_before: usize,
}
#[allow(dead_code)]
impl FormattingData {
    pub fn from(leading_whitespace: &str) -> Self {
        let newlines_before = leading_whitespace
            .chars()
            .filter(|char| char.eq(&'\n'))
            .count();

        // Rusts .lines() fn doesn't treat a trailing newline as creating
        // another line.
        let last_line = leading_whitespace
            .split('\n')
            .last()
            .map(|line| line.trim_end_matches('\r'))
            .unwrap_or_default();

        FormattingData {
            ignored: false,
            newlines_before,
            indentations_before: 0,
            continuations_before: 0,
            spaces_before: last_line.len() - last_line.trim_start().len(),
        }
    }
}

#[allow(dead_code)]
pub struct FormattedTokens<'a> {
    tokens: Vec<(Token<'a>, FormattingData)>,
}
#[allow(dead_code)]
impl<'a> FormattedTokens<'a> {
    pub fn new_from_tokens(tokens: Vec<Token<'a>>) -> Self {
        FormattedTokens {
            tokens: tokens
                .into_iter()
                .map(|token| {
                    let formatting_data = FormattingData::from(token.get_leading_whitespace());
                    (token, formatting_data)
                })
                .collect(),
        }
    }
    pub fn new(tokens: Vec<(Token<'a>, FormattingData)>) -> Self {
        FormattedTokens { tokens }
    }
    pub fn get_tokens(&self) -> &Vec<(Token<'a>, FormattingData)> {
        &self.tokens
    }

    pub fn get_token(&self, index: usize) -> Option<&(Token<'a>, FormattingData)> {
        self.tokens.get(index)
    }

    pub fn get_token_mut(&mut self, index: usize) -> Option<&mut (Token<'a>, FormattingData)> {
        self.tokens.get_mut(index)
    }

    pub fn get_formatting_data(&self, index: usize) -> Option<&FormattingData> {
        self.tokens.get(index).map(|t| &t.1)
    }

    pub fn get_formatting_data_mut(&mut self, index: usize) -> Option<&mut FormattingData> {
        self.tokens.get_mut(index).map(|t| &mut t.1)
    }
    pub fn get_token_type_for_index(&self, index: usize) -> Option<TokenType> {
        self.tokens.get(index).map(|t| t.0.get_token_type())
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
    pub fn get_lines(&self) -> &[LogicalLine] {
        &self.lines
    }
}
impl<'a> From<LogicalLines<'a>> for (Vec<Token<'a>>, Vec<LogicalLine>) {
    fn from(val: LogicalLines<'a>) -> Self {
        (val.tokens, val.lines)
    }
}

pub struct ReconstructionSettings {
    newline_str: String,
    indentation_str: String,
    continuation_str: String,
}
#[allow(dead_code)]
impl ReconstructionSettings {
    pub fn new(newline_str: String, indentation_str: String, continuation_str: String) -> Self {
        ReconstructionSettings {
            newline_str,
            indentation_str,
            continuation_str,
        }
    }
    pub fn get_newline_str(&self) -> &str {
        &self.newline_str
    }
    pub fn get_indentation_str(&self) -> &str {
        &self.indentation_str
    }
    pub fn get_continuation_str(&self) -> &str {
        &self.continuation_str
    }
}

#[derive(Debug, PartialEq, Eq)]
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

#[derive(Debug, PartialEq, Eq)]
pub struct OwningToken {
    index: usize,
    original_leading_whitespace: String,
    content: String,
    token_type: TokenType,
}
#[allow(dead_code)]
impl OwningToken {
    pub fn new(
        index: usize,
        leading_whitespace: String,
        content: String,
        token_type: TokenType,
    ) -> OwningToken {
        OwningToken {
            index,
            original_leading_whitespace: leading_whitespace,
            content,
            token_type,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Token<'a> {
    RefToken(RefToken<'a>),
    #[allow(dead_code)]
    OwningToken(OwningToken),
}
impl<'a> Token<'a> {
    pub fn get_index(&self) -> usize {
        match &self {
            Token::RefToken(token) => token.index,
            Token::OwningToken(token) => token.index,
        }
    }
    pub fn get_leading_whitespace(&self) -> &str {
        match &self {
            Token::RefToken(token) => token.original_leading_whitespace,
            Token::OwningToken(token) => token.original_leading_whitespace.as_str(),
        }
    }
    pub fn get_content(&self) -> &str {
        match &self {
            Token::RefToken(token) => token.content,
            Token::OwningToken(token) => &token.content,
        }
    }
    pub fn get_token_type(&self) -> TokenType {
        match &self {
            Token::RefToken(token) => token.token_type,
            Token::OwningToken(token) => token.token_type,
        }
    }

    pub fn set_token_type(&mut self, typ: TokenType) {
        match self {
            Token::RefToken(token) => token.token_type = typ,
            Token::OwningToken(token) => token.token_type = typ,
        }
    }
}

pub enum FormatterKind {
    LineFormatter(Box<dyn LogicalLineFormatter + Sync>),
    FileFormatter(Box<dyn LogicalLineFileFormatter + Sync>),
}
