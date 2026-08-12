use crate::formatter::{Cursor, TokenMarker};
use crate::lang::*;

/// Formatting stage that reads a code string into a vector of raw tokens.
pub trait Lexer {
    fn lex<'a>(&self, input: &'a str) -> Vec<RawToken<'a>>;
}

/// Formatting stage that adjusts raw tokens.
pub trait RawTokenConsolidator {
    fn consolidate(&self, tokens: &mut [RawToken]);
}

/// Formatting stage that adjusts tokens.
pub trait TokenConsolidator {
    fn consolidate(&self, tokens: &mut [Token]);
}

/// Formatting stage that parses a stream of tokens into logical lines.
pub trait LogicalLineParser {
    fn parse<'a>(&self, input: Vec<RawToken<'a>>) -> (Vec<LogicalLine>, Vec<Token<'a>>);
}

/// Formatting stage that converts or consolidates logical lines into a different representation.
pub trait LogicalLinesConsolidator {
    fn consolidate(&self, input: (&mut [Token], &mut [LogicalLine]));
}

/// Formatting stage that marks certain tokens to be ignored during formatting.
pub trait TokenIgnorer {
    fn ignore_tokens(&self, input: (&[Token], &[LogicalLine]), token_marker: &mut TokenMarker);
}
/// Formatting stage that marks certain tokens to be removed during formatting.
pub trait TokenRemover {
    fn remove_tokens(&self, input: (&[Token], &[LogicalLine]), token_marker: &mut TokenMarker);
}

/// Formatting stage that adjusts token formatting metadata for a single logical line.
pub trait LogicalLineFormatter {
    fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &LogicalLine);
}
/// Formatting stage that adjusts token formatting metadata for all logical lines.
pub trait LogicalLineFileFormatter {
    fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &[LogicalLine]);
}

pub trait CursorTracker {
    fn relocate_cursors(&mut self, formatted_tokens: &FormattedTokens);
    fn notify_token_deleted(&mut self, deleted_token: usize);
}

/// Formatting stage that reconstructs a formatted token stream back into a Delphi code string.
pub trait LogicalLinesReconstructor {
    fn reconstruct(&self, formatted_tokens: FormattedTokens, out: &mut String);

    fn process_cursors<'cursor>(
        &'cursor self,
        cursors: &'cursor mut [Cursor],
        tokens: &[RawToken],
    ) -> Box<dyn CursorTracker + 'cursor>;
}
