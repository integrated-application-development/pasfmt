use crate::lang::*;

pub trait Lexer {
    fn lex<'a>(&self, input: &'a str) -> Vec<Token<'a>>;
}

pub trait TokenConsolidator {
    fn consolidate(&self, tokens: &mut [Token]);
}

pub trait LogicalLineParser {
    fn parse<'a>(&self, input: &'a [Token<'a>]) -> Vec<LogicalLine>;
}

pub trait LogicalLinesConsolidator {
    fn consolidate(&self, input: (&mut [Token], &mut [LogicalLine]));
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
