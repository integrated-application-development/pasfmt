use std::{borrow::Cow, error::Error, fmt::Display, vec::Drain};

use crate::prelude::*;

/// Used to distinguish the semantic meaning of `in` keywords
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum InKind {
    /// As used in a `for` loop
    ///
    /// E.g., ```for A in B do```
    ForLoop,
    /// As used in an expression
    ///
    /// E.g., ```if A in B then```
    Op,
    /// As used in an import clause
    ///
    /// E.g., ```uses A in 'A.pas';```
    Import,
}

/// Used to distinguish the semantic meaning of `var` and `const` keywords
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum DeclKind {
    /// As used to start a block
    ///
    /// E.g., ```const A = B;```
    Section,
    /// As used to declare inline
    ///
    /// E.g., ```const A: B = C;```
    Inline,
    /// As used to specify a parameter type
    ///
    /// E.g., ```procedure A(const B: C);```
    Param,
    /// As used to start an anonymous section
    ///
    /// E.g., ```A := procedure(B: C) const D = 1```
    AnonSection,
    /// As used to other usages
    ///
    /// E.g., ```type A = array of const```
    Other,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[cfg_attr(
    feature = "_lang_types_from_str",
    derive(strum_macros::EnumDiscriminants),
    strum_discriminants(derive(strum_macros::EnumString)),
    strum_discriminants(strum(ascii_case_insensitive))
)]
pub enum KeywordKind {
    // Pure keywords
    And,
    Array,
    As,
    Asm,
    Begin,
    Case,
    Class,
    Const(DeclKind),
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
    In(InKind),
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
    Var(DeclKind),
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
    WinApi,
    Write,
    WriteOnly,
}
impl KeywordKind {
    pub fn is_method_directive(&self) -> bool {
        matches!(
            self,
            KeywordKind::Overload
                | KeywordKind::Reintroduce
                | KeywordKind::Message
                | KeywordKind::Static
                | KeywordKind::Dynamic
                | KeywordKind::Override
                | KeywordKind::Virtual
                | KeywordKind::Abstract
                | KeywordKind::Final
                | KeywordKind::Inline
                | KeywordKind::Assembler
                | KeywordKind::Cdecl
                | KeywordKind::Pascal
                | KeywordKind::Register
                | KeywordKind::SafeCall
                | KeywordKind::StdCall
                | KeywordKind::WinApi
                | KeywordKind::Export
                | KeywordKind::Deprecated
                | KeywordKind::Experimental
                | KeywordKind::Platform
                | KeywordKind::Library
                | KeywordKind::Far
                | KeywordKind::Local
                | KeywordKind::Near
                | KeywordKind::DispId
                | KeywordKind::VarArgs
                | KeywordKind::Unsafe
                | KeywordKind::External
                | KeywordKind::Forward
        )
    }
    pub fn is_property_directive(&self) -> bool {
        matches!(
            self,
            KeywordKind::Default
                | KeywordKind::Read
                | KeywordKind::Write
                | KeywordKind::ReadOnly
                | KeywordKind::WriteOnly
                | KeywordKind::DispId
                | KeywordKind::Implements
                | KeywordKind::Index
                | KeywordKind::NoDefault
                | KeywordKind::Stored
        )
    }
}

/// Used to distinguish the semantic meanings of `=`
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum EqKind {
    /// As used in const, type, etc. declarations
    ///
    /// E.g., ```const A = 1;```
    Decl,
    /// As used in boolean expressions
    ///
    /// E.g., ```if A = B then```
    Comp,
}

/// Used to distinguish the semantic meanings of `<` and `>`
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum ChevronKind {
    /// As used in generic parameters
    ///
    /// E.g., ```Foo<Bar>()```
    Generic,
    /// As used in boolean expressions
    ///
    /// E.g., ```if A < B then```
    Comp,
}

/// Used to distinguish the semantic meanings of `^`
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum CaretKind {
    /// As used in pointer types
    ///
    /// E.g., ```PFoo = ^TFoo```
    Type,
    /// As used in expressions
    ///
    /// E.g., ```A := B^```
    Deref,
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
    Equal(EqKind),
    NotEqual,
    LessThan(ChevronKind),
    LessEqual,
    GreaterThan(ChevronKind),
    GreaterEqual,
    LBrack,
    RBrack,
    LParen,
    RParen,
    Caret(CaretKind),
    AddressOf,
    Dot,
    DotDot,
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
pub enum TextLiteralKind {
    SingleLine,
    MultiLine,
    Asm,
    Unterminated,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum RawTokenType {
    Op(OperatorKind),
    Identifier,
    IdentifierOrKeyword(KeywordKind),
    Keyword(KeywordKind),
    TextLiteral(TextLiteralKind),
    NumberLiteral(NumberLiteralKind),
    ConditionalDirective(ConditionalDirectiveKind),
    CompilerDirective,
    Comment(CommentKind),
    Eof,
    Unknown,
}

impl RawTokenType {
    pub(crate) fn is_comment_or_directive(&self) -> bool {
        matches!(
            self,
            Self::Comment(_) | Self::CompilerDirective | Self::ConditionalDirective(_)
        )
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum TokenType {
    Op(OperatorKind),
    Identifier,
    Keyword(KeywordKind),
    TextLiteral(TextLiteralKind),
    NumberLiteral(NumberLiteralKind),
    ConditionalDirective(ConditionalDirectiveKind),
    CompilerDirective,
    Comment(CommentKind),
    Eof,
    Unknown,
}

impl TokenType {
    pub(crate) fn is_comment_or_directive(&self) -> bool {
        matches!(
            self,
            Self::Comment(_) | Self::CompilerDirective | Self::ConditionalDirective(_)
        )
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
#[cfg_attr(
    feature = "_lang_types_from_str",
    derive(strum_macros::EnumString),
    strum(ascii_case_insensitive)
)]
pub enum LogicalLineType {
    Assignment,
    ConditionalDirective,
    CompilerDirective,
    ForLoop,
    Eof,
    ImportClause,
    AsmInstruction,
    PropertyDeclaration,
    RoutineHeader,
    InlineDeclaration,
    Guid,
    Attribute,
    CaseHeader,
    CaseArm,
    Declaration,
    Unknown,
    Voided,
}
#[derive(Hash, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LineParent {
    pub line_index: usize,
    pub global_token_index: usize,
}
#[derive(Debug, PartialEq, Eq)]
pub struct LogicalLine {
    parent: Option<LineParent>,
    level: u16,
    tokens: Vec<usize>,
    line_type: LogicalLineType,
}
impl LogicalLine {
    pub fn new(
        parent: Option<LineParent>,
        level: u16,
        tokens: Vec<usize>,
        line_type: LogicalLineType,
    ) -> Self {
        LogicalLine {
            parent,
            level,
            tokens,
            line_type,
        }
    }
    pub fn get_parent(&self) -> Option<LineParent> {
        self.parent
    }
    pub fn get_level(&self) -> u16 {
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

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidReconstructionSettingsError {
    msg: String,
}

impl Display for InvalidReconstructionSettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid reconstruction settings: {}", self.msg)
    }
}

impl Error for InvalidReconstructionSettingsError {}

impl InvalidReconstructionSettingsError {
    fn new<S: Into<String>>(msg: S) -> Self {
        InvalidReconstructionSettingsError { msg: msg.into() }
    }
}

pub struct ReconstructionSettings {
    newline_str: String,
    indentation_str: String,
    continuation_str: String,
}
impl ReconstructionSettings {
    pub fn new<S: Into<String> + AsRef<str>>(
        newline_str: S,
        indentation_str: S,
        continuation_str: S,
    ) -> Result<Self, InvalidReconstructionSettingsError> {
        for (name, val) in &[
            ("newline", newline_str.as_ref()),
            ("indentation", indentation_str.as_ref()),
            ("continuation", continuation_str.as_ref()),
        ] {
            if val.is_empty() {
                return Err(InvalidReconstructionSettingsError::new(format!(
                    "{name} sequence cannot be blank"
                )));
            } else if count_leading_whitespace(val) != val.len() {
                return Err(InvalidReconstructionSettingsError::new(format!(
                    "{name} sequence must be all whitespace (was {val:?})"
                )));
            }
        }

        Ok(ReconstructionSettings {
            newline_str: newline_str.into(),
            indentation_str: indentation_str.into(),
            continuation_str: continuation_str.into(),
        })
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

pub trait TokenData {
    type TokenType;
    fn get_leading_whitespace(&self) -> &str;
    fn get_content(&self) -> &str;
    fn get_token_type(&self) -> Self::TokenType;
    fn set_token_type(&mut self, typ: Self::TokenType);
}

#[derive(Debug, PartialEq, Eq)]
pub struct RawToken<'a> {
    content: &'a str,
    ws_len: u32,
    token_type: RawTokenType,
}
impl<'a> RawToken<'a> {
    pub fn new(content: &'a str, ws_len: u32, token_type: RawTokenType) -> RawToken<'a> {
        RawToken {
            content,
            ws_len,
            token_type,
        }
    }
}
impl<'a> TokenData for RawToken<'a> {
    type TokenType = RawTokenType;
    fn get_leading_whitespace(&self) -> &str {
        &self.content[..self.ws_len as usize]
    }
    fn get_content(&self) -> &str {
        &self.content[self.ws_len as usize..]
    }
    fn get_token_type(&self) -> RawTokenType {
        self.token_type
    }
    fn set_token_type(&mut self, typ: RawTokenType) {
        self.token_type = typ;
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Token<'a> {
    content: Cow<'a, str>,
    ws_len: u32,
    token_type: TokenType,
}
impl<'a> Token<'a> {
    pub fn new_ref(content: &'a str, ws_len: u32, token_type: TokenType) -> Self {
        Self {
            content: Cow::Borrowed(content),
            ws_len,
            token_type,
        }
    }

    pub fn new_owned(content: String, ws_len: u32, token_type: TokenType) -> Self {
        Self {
            content: Cow::Owned(content),
            ws_len,
            token_type,
        }
    }
}

impl<'a> TokenData for Token<'a> {
    type TokenType = TokenType;
    fn get_leading_whitespace(&self) -> &str {
        &self.content[..self.ws_len as usize]
    }
    fn get_content(&self) -> &str {
        &self.content[self.ws_len as usize..]
    }
    fn get_token_type(&self) -> TokenType {
        self.token_type
    }

    fn set_token_type(&mut self, typ: TokenType) {
        self.token_type = typ
    }
}
impl<'a> From<RawToken<'a>> for Token<'a> {
    fn from(val: RawToken<'a>) -> Self {
        use RawTokenType as RTT;
        use TokenType as TT;
        Token::new_ref(
            val.content,
            val.ws_len,
            match val.token_type {
                RTT::IdentifierOrKeyword(_) => TT::Identifier,
                RTT::Op(op_kind) => TT::Op(op_kind),
                RTT::Identifier => TT::Identifier,
                RTT::Keyword(kind) => TT::Keyword(kind),
                RTT::TextLiteral(kind) => TT::TextLiteral(kind),
                RTT::NumberLiteral(kind) => TT::NumberLiteral(kind),
                RTT::ConditionalDirective(kind) => TT::ConditionalDirective(kind),
                RTT::CompilerDirective => TT::CompilerDirective,
                RTT::Comment(kind) => TT::Comment(kind),
                RTT::Eof => TT::Eof,
                RTT::Unknown => TT::Unknown,
            },
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    mod reconstruction_validation {
        use super::*;

        #[test]
        fn normal_whitespace() {
            assert!(ReconstructionSettings::new("\n", "\t", "    ",).is_ok());
        }

        #[test]
        fn weird_whitespace() {
            assert!(ReconstructionSettings::new("\0", "\x1F", "\u{3000}",).is_ok());
        }

        #[test]
        fn borrowed_or_owned() {
            assert!(ReconstructionSettings::new("\n", " ", " ").is_ok());
            assert!(
                ReconstructionSettings::new("\n".to_owned(), " ".to_owned(), " ".to_owned())
                    .is_ok()
            );
        }

        #[test]
        fn invalid_empty() {
            use super::InvalidReconstructionSettingsError as E;

            assert_eq!(
                ReconstructionSettings::new("", "", "").err(),
                Some(E::new("newline sequence cannot be blank"))
            );
            assert_eq!(
                ReconstructionSettings::new("\n", "", "").err(),
                Some(E::new("indentation sequence cannot be blank"))
            );
            assert_eq!(
                ReconstructionSettings::new("\n", " ", "").err(),
                Some(E::new("continuation sequence cannot be blank"))
            );
        }

        #[test]
        fn invalid_non_whitespace() {
            use super::InvalidReconstructionSettingsError as E;

            assert_eq!(
                ReconstructionSettings::new("a", " ", " ").err(),
                Some(E::new(
                    "newline sequence must be all whitespace (was \"a\")"
                ))
            );
            assert_eq!(
                ReconstructionSettings::new("\n", "a", " ").err(),
                Some(E::new(
                    "indentation sequence must be all whitespace (was \"a\")"
                ))
            );
            assert_eq!(
                ReconstructionSettings::new("\n", " ", "a").err(),
                Some(E::new(
                    "continuation sequence must be all whitespace (was \"a\")"
                ))
            );
        }

        #[test]
        fn display() {
            assert_eq!(
                ReconstructionSettings::new("", " ", " ")
                    .err()
                    .unwrap()
                    .to_string(),
                "Invalid reconstruction settings: newline sequence cannot be blank"
            );
        }
    }
}
