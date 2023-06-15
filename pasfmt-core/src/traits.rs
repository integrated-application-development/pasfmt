use crate::lang::*;

pub trait Lexer {
    fn lex<'a>(&self, input: &'a str) -> Vec<Token<'a>>;
}

pub trait LogicalLineParser {
    fn parse<'a>(&self, input: Vec<Token<'a>>) -> LogicalLines<'a>;
}
