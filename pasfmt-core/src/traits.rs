use crate::lang::*;

pub trait Lexer {
    fn lex<'a>(&self, input: &'a str) -> Vec<Token<'a>>;
}

pub trait LogicalLineParser {
    fn parse<'a>(&self, input: Vec<Token<'a>>) -> LogicalLines<'a>;
}

pub trait LogicalLinesConsolidator {
    fn consolidate<'a>(&self, input: LogicalLines<'a>) -> LogicalLines<'a>;
}
