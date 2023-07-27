use crate::lang::*;

pub trait Lexer {
    fn lex<'a>(&self, input: &'a str) -> Vec<Token<'a>>;
}

pub trait TokenConsolidator {
    fn consolidate<'a>(&self, tokens: Vec<Token<'a>>) -> Vec<Token<'a>>;
}

pub trait LogicalLineParser {
    fn parse<'a>(&self, input: Vec<Token<'a>>) -> LogicalLines<'a>;
}

pub trait LogicalLinesConsolidator {
    fn consolidate<'a>(&self, input: LogicalLines<'a>) -> LogicalLines<'a>;
}
pub trait LogicalLineFormatter {
    fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &LogicalLine);
}
pub trait LogicalLineFileFormatter {
    fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &[LogicalLine]);
}

pub trait LogicalLinesReconstructor {
    fn reconstruct(&self, formatted_tokens: FormattedTokens) -> String;
}
