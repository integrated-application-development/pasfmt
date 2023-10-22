use crate::formatter::TokenMarker;
use crate::lang::*;

pub trait Lexer {
    fn lex<'a>(&self, input: &'a str) -> Vec<RawToken<'a>>;
}

pub trait RawTokenConsolidator {
    fn consolidate(&self, tokens: &mut [RawToken]);
}
pub trait TokenConsolidator {
    fn consolidate(&self, tokens: &mut [Token]);
}

pub trait LogicalLineParser {
    fn parse<'a>(&self, input: Vec<RawToken<'a>>) -> (Vec<LogicalLine>, Vec<Token<'a>>);
}

pub trait LogicalLinesConsolidator {
    fn consolidate(&self, input: (&mut [Token], &mut [LogicalLine]));
}

pub trait TokenIgnorer {
    fn ignore_tokens(&self, input: (&[Token], &[LogicalLine]), token_marker: &mut TokenMarker);
}

pub trait TokenRemover {
    fn remove_tokens(&self, input: (&[Token], &[LogicalLine]), token_marker: &mut TokenMarker);
}

pub trait LogicalLineFormatter {
    fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &LogicalLine);
}
pub trait LogicalLineFileFormatter {
    fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &[LogicalLine]);
}

pub trait LogicalLinesReconstructor {
    fn reconstruct_into_buf(&self, formatted_tokens: FormattedTokens, out: &mut String);
    fn reconstruct(&self, formatted_tokens: FormattedTokens) -> String {
        let mut out = String::new();
        self.reconstruct_into_buf(formatted_tokens, &mut out);
        out
    }
}
