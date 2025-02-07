use crate::formatter::{Cursor, TokenMarker};
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

pub trait CursorTracker {
    fn relocate_cursors(&mut self, formatted_tokens: &FormattedTokens);
    fn notify_token_deleted(&mut self, deleted_token: usize);
}

pub trait LogicalLinesReconstructor {
    fn reconstruct(&self, formatted_tokens: FormattedTokens, out: &mut String);

    fn process_cursors<'cursor>(
        &'cursor self,
        cursors: &'cursor mut [Cursor],
        tokens: &[RawToken],
    ) -> Box<dyn CursorTracker + 'cursor>;
}
