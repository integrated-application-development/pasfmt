use std::vec::Drain;

use crate::prelude::*;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum KeywordKind {
    // Pure keywords
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
    DispInterface,
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
    String,
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

    // Impure keywords
    Absolute,
    Abstract,
    Align,
    Assembler,
    At,
    Automated,
    Cdecl,
    Contains,
    Default,
    Delayed,
    Deprecated,
    DispId,
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
    ReadOnly,
    Reference,
    Register,
    Reintroduce,
    Requires,
    Resident,
    SafeCall,
    Sealed,
    Static,
    StdCall,
    Stored,
    Strict,
    Unsafe,
    VarArgs,
    Virtual,
    Write,
    WriteOnly,
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
    LGeneric,
    RGeneric,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum NumberLiteralKind {
    Decimal,
    Octal,
    Hex,
    Binary,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum CommentKind {
    InlineBlock,
    IndividualBlock,
    MultilineBlock,
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
    IdentifierOrKeyword(KeywordKind),
    Keyword(KeywordKind),
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
    ImportClause,
    AsmInstruction,
    PropertyDeclaration,
    Unknown,
    Voided,
}
#[derive(Debug, PartialEq, Eq)]
pub struct LogicalLine {
    parent_token: Option<usize>,
    level: usize,
    tokens: Vec<usize>,
    line_type: LogicalLineType,
}
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
    pub fn void_and_drain(&mut self) -> Drain<usize> {
        self.line_type = LogicalLineType::Voided;
        self.tokens.drain(0..)
    }
}

#[derive(Default)]
pub struct FormattingData {
    ignored: bool,
    pub newlines_before: usize,
    pub indentations_before: usize,
    pub continuations_before: usize,
    pub spaces_before: usize,
}

impl From<&str> for FormattingData {
    fn from(leading_whitespace: &str) -> Self {
        (leading_whitespace, false).into()
    }
}

impl From<(&str, bool)> for FormattingData {
    fn from((leading_whitespace, ignored): (&str, bool)) -> Self {
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
            ignored,
            newlines_before,
            indentations_before: 0,
            continuations_before: 0,
            spaces_before: last_line.len() - last_line.trim_start().len(),
        }
    }
}

impl FormattingData {
    pub fn is_ignored(&self) -> bool {
        self.ignored
    }
}

pub struct FormattedTokens<'a> {
    tokens: Vec<(&'a Token<'a>, FormattingData)>,
}
impl<'a> FormattedTokens<'a> {
    pub fn new_from_tokens(tokens: &'a [Token<'a>], ignored_tokens: &TokenMarker) -> Self {
        FormattedTokens {
            tokens: tokens
                .iter()
                .enumerate()
                .map(|(i, token)| {
                    let formatting_data = FormattingData::from((
                        token.get_leading_whitespace(),
                        ignored_tokens.is_marked(&i),
                    ));
                    (token, formatting_data)
                })
                .collect(),
        }
    }
    pub fn new(tokens: Vec<(&'a Token<'a>, FormattingData)>) -> Self {
        FormattedTokens { tokens }
    }
    pub fn get_tokens(&self) -> &Vec<(&'a Token<'a>, FormattingData)> {
        &self.tokens
    }

    pub fn get_token(&self, index: usize) -> Option<&(&'a Token<'a>, FormattingData)> {
        self.tokens.get(index)
    }

    pub fn get_token_mut(&mut self, index: usize) -> Option<&mut (&'a Token<'a>, FormattingData)> {
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

pub struct LogicalLines<'a> {
    tokens: &'a mut [Token<'a>],
    lines: Vec<LogicalLine>,
}
impl<'a> LogicalLines<'a> {
    pub fn new(tokens: &'a mut [Token<'a>], lines: Vec<LogicalLine>) -> Self {
        LogicalLines { tokens, lines }
    }
    pub fn get_tokens(&'a self) -> &'a [Token<'a>] {
        self.tokens
    }
    pub fn get_tokens_mut(&'a mut self) -> &'a mut [Token<'a>] {
        self.tokens
    }
    pub fn get_lines(&self) -> &[LogicalLine] {
        &self.lines
    }
    pub fn get_lines_mut(&mut self) -> &mut [LogicalLine] {
        &mut self.lines
    }
}
impl<'a> From<LogicalLines<'a>> for (&'a [Token<'a>], Vec<LogicalLine>) {
    fn from(val: LogicalLines<'a>) -> Self {
        (val.tokens, val.lines)
    }
}

pub struct ReconstructionSettings {
    newline_str: String,
    indentation_str: String,
    continuation_str: String,
}
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
    content: &'a str,
    ws_len: u32,
    token_type: TokenType,
}
impl<'a> RefToken<'a> {
    pub fn new(content: &'a str, ws_len: u32, token_type: TokenType) -> RefToken<'a> {
        RefToken {
            content,
            ws_len,
            token_type,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct OwningToken {
    content: Box<str>,
    ws_len: u32,
    token_type: TokenType,
}
impl OwningToken {
    pub fn new(content: String, ws_len: u32, token_type: TokenType) -> OwningToken {
        OwningToken {
            content: content.into(),
            ws_len,
            token_type,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Token<'a> {
    RefToken(RefToken<'a>),
    OwningToken(OwningToken),
}
impl<'a> Token<'a> {
    pub fn get_leading_whitespace(&self) -> &str {
        match &self {
            Token::RefToken(token) => &token.content[..token.ws_len as usize],
            Token::OwningToken(token) => &token.content[..token.ws_len as usize],
        }
    }
    pub fn get_content(&self) -> &str {
        match &self {
            Token::RefToken(token) => &token.content[token.ws_len as usize..],
            Token::OwningToken(token) => &token.content[token.ws_len as usize..],
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
impl LogicalLineFileFormatter for FormatterKind {
    fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &[LogicalLine]) {
        match self {
            FormatterKind::LineFormatter(formatter) => input
                .iter()
                .for_each(|logical_line| formatter.format(formatted_tokens, logical_line)),
            FormatterKind::FileFormatter(formatter) => formatter.format(formatted_tokens, input),
        }
    }
}
