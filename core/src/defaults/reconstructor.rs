use std::cmp::Ordering;

use crate::{
    formatter::Cursor,
    lang::*,
    traits::{CursorTracker, LogicalLinesReconstructor},
};

pub struct DelphiLogicalLinesReconstructor {
    reconstruction_settings: ReconstructionSettings,
}
impl DelphiLogicalLinesReconstructor {
    pub fn new(reconstruction_settings: ReconstructionSettings) -> Self {
        DelphiLogicalLinesReconstructor {
            reconstruction_settings,
        }
    }
}

struct InternalCursor<'cursor> {
    cursor: &'cursor mut Cursor,
    tok_idx: usize,
    tok_pos: TokPos,
}

struct CursorTrackerImpl<'cursor> {
    reconstructor: &'cursor DelphiLogicalLinesReconstructor,
    cursors: Vec<InternalCursor<'cursor>>,
}

enum TokPos {
    /// Position of a cursor inside a token
    InContent {
        /// Byte offset relative to the start of the token contents (not whitespace)
        offset: u32,
    },
    /// Position of a cursor in the whitespace before a token
    InWhitespace {
        /// Column of the cursor (in bytes), measured from the previous line break
        col: u16,
        /// The number of line breaks between the cursor and the subsequent token
        newlines_after_cursor: u16,
    },
}

impl LogicalLinesReconstructor for DelphiLogicalLinesReconstructor {
    fn reconstruct(&self, formatted_tokens: FormattedTokens, buf: &mut String) {
        formatted_tokens
            .get_tokens()
            .iter()
            .for_each(|(token, formatting_data)| {
                if formatting_data.is_ignored() {
                    buf.push_str(token.get_leading_whitespace());
                } else {
                    (0..formatting_data.newlines_before)
                        .for_each(|_| buf.push_str(self.reconstruction_settings.get_newline_str()));
                    (0..formatting_data.indentations_before).for_each(|_| {
                        buf.push_str(self.reconstruction_settings.get_indentation_str())
                    });
                    (0..formatting_data.continuations_before).for_each(|_| {
                        buf.push_str(self.reconstruction_settings.get_continuation_str())
                    });
                    (0..formatting_data.spaces_before).for_each(|_| buf.push(' '));
                };

                buf.push_str(token.get_content());
            });
    }

    fn process_cursors<'cursor>(
        &'cursor self,
        cursors: &'cursor mut [Cursor],
        tokens: &[RawToken],
    ) -> Box<dyn CursorTracker + 'cursor> {
        let mut cursor_positions: Vec<(i64, &mut Cursor, Option<usize>)> =
            cursors.iter_mut().map(|c| (c.0 as i64, c, None)).collect();

        for (idx, tok) in tokens.iter().enumerate() {
            let next_len = tok.get_str().len() as i64;

            let mut all_found = true;
            for (cursor_rem, _, cursor_idx) in &mut cursor_positions {
                if cursor_idx.is_none() {
                    // including the case where these are equal causes a cursor to 'stick' to the previous token
                    if *cursor_rem <= next_len {
                        *cursor_rem -= tok.get_leading_whitespace().len() as i64;
                        *cursor_idx = Some(idx)
                    } else {
                        all_found = false;
                        *cursor_rem -= next_len;
                    }
                }
            }

            if all_found {
                break;
            }
        }

        let cursors = cursor_positions
            .into_iter()
            .map(|(tok_pos, cursor, tok_idx)| {
                let tok_idx = tok_idx.unwrap_or(tokens.len());

                let tok_pos = if tok_pos >= 0 {
                    TokPos::InContent {
                        offset: tok_pos as u32,
                    }
                } else {
                    let tok = &tokens[tok_idx];
                    let leading_ws = tok.get_leading_whitespace();
                    let (ws_before_cursor, ws_after_cursor) =
                        &leading_ws.split_at(
                            (leading_ws.len() as u64).saturating_add_signed(tok_pos) as usize,
                        );
                    let newlines_after_cursor = (ws_after_cursor.split('\n').count() - 1) as u16;

                    let col = if let Some(pos) = ws_before_cursor.rfind('\n') {
                        ws_before_cursor.len() - 1 - pos
                    } else {
                        ws_before_cursor.len()
                            + Self::col_for_token_end_pre_fmt(tokens, tok_idx.wrapping_sub(1))
                    };

                    TokPos::InWhitespace {
                        col: col as u16,
                        newlines_after_cursor,
                    }
                };

                InternalCursor {
                    cursor,
                    tok_idx,
                    tok_pos,
                }
            })
            .collect();

        Box::new(CursorTrackerImpl {
            reconstructor: self,
            cursors,
        })
    }
}

struct NonBreakingWs {
    len: usize,
    break_found: bool,
}

impl DelphiLogicalLinesReconstructor {
    fn ws_len(&self, token: &(&Token, FormattingData)) -> usize {
        if token.1.is_ignored() {
            token.0.get_leading_whitespace().len()
        } else {
            let settings = &self.reconstruction_settings;
            self.nonbreaking_ws_len(token).len
                + token.1.newlines_before as usize * settings.get_newline_str().len()
        }
    }

    fn nonbreaking_ws_len(&self, token: &(&Token, FormattingData)) -> NonBreakingWs {
        if token.1.is_ignored() {
            let leading_ws = token.0.get_leading_whitespace();
            let nl_pos = leading_ws.rfind('\n');
            let len = leading_ws.len() - nl_pos.map(|p| p + 1).unwrap_or(0);
            NonBreakingWs {
                len,
                break_found: nl_pos.is_some(),
            }
        } else {
            let settings = &self.reconstruction_settings;
            let len = token.1.spaces_before as usize
                + token.1.continuations_before as usize * settings.get_continuation_str().len()
                + token.1.indentations_before as usize * settings.get_indentation_str().len();
            NonBreakingWs {
                len,
                break_found: token.1.newlines_before > 0,
            }
        }
    }

    fn nl_len(&self) -> usize {
        self.reconstruction_settings.get_newline_str().len()
    }

    fn col_for_token_end_pre_fmt(tokens: &[RawToken], mut idx: usize) -> usize {
        let mut col = 0;
        while let Some(tok) = tokens.get(idx) {
            idx = idx.wrapping_sub(1);
            col += tok.get_str().len();
            if let Some(pos) = tok.get_str().rfind('\n') {
                col -= pos + 1;
                break;
            }
        }
        col
    }

    fn col_for_token_end_post_fmt(
        &self,
        tokens: &[(&Token, FormattingData)],
        mut idx: usize,
    ) -> usize {
        let mut col = 0;
        while let Some(token) = tokens.get(idx) {
            idx = idx.wrapping_sub(1);
            let content = token.0.get_content();
            col += content.len();
            if let Some(pos) = content.rfind('\n') {
                col -= pos + 1;
                break;
            }

            let ws_len = self.nonbreaking_ws_len(token);
            col += ws_len.len;
            if ws_len.break_found {
                break;
            }
        }
        col
    }

    fn offset_for_token(&self, formatted_tokens: &FormattedTokens, token_idx: usize) -> usize {
        let mut pos = 0;
        for (idx, token) in formatted_tokens.get_tokens().iter().enumerate() {
            pos += self.ws_len(token);
            if idx >= token_idx {
                break;
            }
            pos += token.0.get_content().len();
        }
        pos
    }
}

impl CursorTracker for CursorTrackerImpl<'_> {
    fn notify_token_deleted(&mut self, deleted_token: usize) {
        for cursor in &mut self.cursors {
            match deleted_token.cmp(&cursor.tok_idx) {
                Ordering::Less => cursor.tok_idx -= 1,
                Ordering::Equal => cursor.tok_pos = TokPos::InContent { offset: 0 },
                Ordering::Greater => {}
            }
        }
    }

    fn relocate_cursors(&mut self, formatted_tokens: &FormattedTokens) {
        for cursor in &mut self.cursors {
            let token = match formatted_tokens.get_token(cursor.tok_idx) {
                Some(t) => t,
                None => {
                    // cursor is out of bounds, set it to the last position in the file, if possible
                    if let Some(t) = formatted_tokens.get_tokens().last() {
                        cursor.tok_pos = TokPos::InContent {
                            offset: t.0.get_content().len() as u32,
                        };
                        t
                    } else {
                        return;
                    }
                }
            };

            let (tok, fmt) = token;

            let new_token_offset = self
                .reconstructor
                .offset_for_token(formatted_tokens, cursor.tok_idx);

            cursor.cursor.0 = match cursor.tok_pos {
                TokPos::InContent { offset } => {
                    // offset into the token content, but don't go over the end of the token if its length has changed
                    new_token_offset as u32 + offset.min(tok.get_content().len() as u32)
                }
                TokPos::InWhitespace {
                    col,
                    newlines_after_cursor,
                } => {
                    let mut lines_back = newlines_after_cursor.min(fmt.newlines_before);
                    if lines_back > 0 {
                        // The cursor was on a blank line. Keep it there at column 0.
                        if fmt.newlines_before <= newlines_after_cursor && fmt.newlines_before > 1 {
                            // the line that the cursor was on is no longer there, move it to the next line
                            lines_back -= 1;
                        }

                        (new_token_offset
                            + (self.reconstructor.nl_len()
                                * fmt.newlines_before.saturating_sub(lines_back) as usize)
                            - self.reconstructor.ws_len(token)) as u32
                    } else {
                        // Either no newlines after cursor before formatting, or no newlines before token now
                        // in either case, the cursor should go onto the same line as the token, but we
                        // can try to keep it at the same column if possible (so long as that doesn't
                        // move it between tokens).

                        let col_end = self.reconstructor.col_for_token_end_post_fmt(
                            formatted_tokens.get_tokens(),
                            cursor.tok_idx,
                        );
                        let col_start = col_end - tok.get_content().len();
                        let col_ws_start =
                            col_start - self.reconstructor.nonbreaking_ws_len(token).len;

                        (new_token_offset
                            - (col_start - (col as usize).clamp(col_ws_start, col_start)))
                            as u32
                    }
                }
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use spectral::prelude::*;

    fn run_test(input: FormattedTokens, expected_output: &str) {
        let reconstructor = DelphiLogicalLinesReconstructor {
            reconstruction_settings: ReconstructionSettings::new(
                LineEnding::Lf,
                TabKind::Soft,
                2,
                2,
            ),
        };
        let mut output = String::new();
        reconstructor.reconstruct(input, &mut output);
        assert_that(&output).is_equal_to(expected_output.to_string());
    }

    fn ignored_formatting_data() -> FormattingData {
        FormattingData::from(("", true))
    }

    #[test]
    fn all_tokens_with_data_from_token() {
        run_test(
            FormattedTokens::new(vec![
                (
                    &new_token("\n\n  token1", TokenType::Unknown),
                    FormattingData::from("\n\n  "),
                ),
                (
                    &new_token(" token2", TokenType::Unknown),
                    FormattingData::from(" "),
                ),
            ]),
            "\n\n  token1 token2",
        );
    }

    #[test]
    fn all_tokens_with_data_constructed_spaces() {
        let mut formatting_data1 = FormattingData::default();
        formatting_data1.spaces_before = 3;
        let mut formatting_data2 = FormattingData::default();
        formatting_data2.spaces_before = 1;
        run_test(
            FormattedTokens::new(vec![
                (&new_token("token1", TokenType::Unknown), formatting_data1),
                (&new_token("token2", TokenType::Unknown), formatting_data2),
            ]),
            "   token1 token2",
        );
    }

    #[test]
    fn all_tokens_with_data_constructed_indentations() {
        let mut formatting_data1 = FormattingData::default();
        formatting_data1.indentations_before = 3;
        let mut formatting_data2 = FormattingData::default();
        formatting_data2.indentations_before = 1;
        run_test(
            FormattedTokens::new(vec![
                (&new_token("token1", TokenType::Unknown), formatting_data1),
                (&new_token("token2", TokenType::Unknown), formatting_data2),
            ]),
            "      token1  token2",
        );
    }

    #[test]
    fn all_tokens_with_data_constructed_continuations() {
        let mut formatting_data1 = FormattingData::default();
        formatting_data1.continuations_before = 3;
        let mut formatting_data2 = FormattingData::default();
        formatting_data2.continuations_before = 1;
        run_test(
            FormattedTokens::new(vec![
                (&new_token("token1", TokenType::Unknown), formatting_data1),
                (&new_token("token2", TokenType::Unknown), formatting_data2),
            ]),
            "      token1  token2",
        );
    }

    #[test]
    fn some_tokens_with_data() {
        let mut formatting_data1 = FormattingData::default();
        formatting_data1.newlines_before = 3;
        run_test(
            FormattedTokens::new(vec![
                (&new_token(" token1", TokenType::Unknown), formatting_data1),
                (
                    &new_token("\n   token2", TokenType::Unknown),
                    ignored_formatting_data(),
                ),
            ]),
            "\n\n\ntoken1\n   token2",
        );
        let mut formatting_data2 = FormattingData::default();
        formatting_data2.newlines_before = 3;
        run_test(
            FormattedTokens::new(vec![
                (
                    &new_token(" token1", TokenType::Unknown),
                    ignored_formatting_data(),
                ),
                (
                    &new_token("\n   token2", TokenType::Unknown),
                    formatting_data2,
                ),
            ]),
            " token1\n\n\ntoken2",
        );
    }

    #[test]
    fn no_tokens_with_data() {
        run_test(
            FormattedTokens::new(vec![
                (
                    &new_token(" token1", TokenType::Unknown),
                    ignored_formatting_data(),
                ),
                (
                    &new_token("\n   token2", TokenType::Unknown),
                    ignored_formatting_data(),
                ),
            ]),
            " token1\n   token2",
        );
    }

    #[test]
    fn trailing_newlines() {
        run_test(
            FormattedTokens::new(vec![
                (
                    &new_token("token1", TokenType::Unknown),
                    ignored_formatting_data(),
                ),
                (&new_token("\n", TokenType::Eof), ignored_formatting_data()),
            ]),
            "token1\n",
        );
    }

    #[test]
    fn unrepresentable_leading_whitespace() {
        run_test(
            FormattedTokens::new(vec![(
                &new_token("\n \n\ttoken1", TokenType::Unknown),
                ignored_formatting_data(),
            )]),
            "\n \n\ttoken1",
        );
    }

    mod cursor {
        use super::*;

        struct IgnoreUnknownTokens;
        impl TokenIgnorer for IgnoreUnknownTokens {
            fn ignore_tokens(
                &self,
                input: (&[Token], &[LogicalLine]),
                token_marker: &mut TokenMarker,
            ) {
                for (idx, token) in input.0.iter().enumerate() {
                    if matches!(token.get_token_type(), TokenType::Unknown) {
                        token_marker.mark(idx);
                    }
                }
            }
        }

        struct DeleteInlineBlockComments;
        impl TokenRemover for DeleteInlineBlockComments {
            fn remove_tokens(
                &self,
                input: (&[Token], &[LogicalLine]),
                token_marker: &mut TokenMarker,
            ) {
                for (idx, token) in input.0.iter().enumerate() {
                    if matches!(
                        token.get_token_type(),
                        TokenType::Comment(CommentKind::InlineBlock)
                    ) {
                        token_marker.mark(idx);
                    }
                }
            }
        }

        struct AddSpaces;
        impl LogicalLineFileFormatter for AddSpaces {
            fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, _: &[LogicalLine]) {
                for idx in 0..formatted_tokens.get_tokens().len() {
                    let (tok, fmt) = formatted_tokens.get_token_mut(idx).unwrap();
                    if matches!(tok.get_token_type(), TokenType::Identifier,)
                        && tok.get_content() == "Spaces"
                    {
                        fmt.spaces_before = 6;
                    }
                }
            }
        }

        fn formatter() -> Formatter {
            let recon = ReconstructionSettings::new(LineEnding::Lf, TabKind::Soft, 2, 4);
            Formatter::builder()
                .lexer(DelphiLexer {})
                .parser(DelphiLogicalLineParser {})
                .token_ignorer(IgnoreUnknownTokens)
                .token_remover(DeleteInlineBlockComments)
                .file_formatter(TokenSpacing {})
                .file_formatter(AddSpaces)
                .file_formatter(OptimisingLineFormatter::new(
                    OptimisingLineFormatterSettings {
                        max_line_length: 30,
                        iteration_max: 1000,
                        break_before_begin: false,
                    },
                    recon.clone(),
                ))
                .reconstructor(DelphiLogicalLinesReconstructor::new(recon))
                .build()
        }

        fn parse_cursors(input: &str) -> (String, Vec<Cursor>) {
            let input_cursors = input.match_indices('|').enumerate().fold(
                vec![],
                |mut cursors, (match_idx, (cursor_pos, _))| {
                    cursors.push(Cursor((cursor_pos - match_idx) as u32));
                    cursors
                },
            );
            let input = input.replace('|', "");
            (input, input_cursors)
        }

        fn assert_cursor(input: &str, output: &str) {
            let (input, mut cursors) = parse_cursors(input);
            let (expected_output, expected_cursors) = parse_cursors(output);

            if cursors.is_empty() {
                panic!("input should use '|' to indicate the initial cursor positions");
            }

            let output = formatter().format(&input, FileOptions::new().with_cursors(&mut cursors));

            pretty_assertions::assert_eq!(expected_output, output);
            pretty_assertions::assert_eq!(expected_cursors, cursors, "input was {:?}", &input);
        }

        #[test]
        fn sticks_to_next_token() {
            assert_cursor("|a :=  b;", "|a := b;");
            assert_cursor("a |:=  b;", "a |:= b;");
            assert_cursor("a :=  |b;", "a := |b;");
            assert_cursor("a :=  b|;", "a := b|;");
        }

        #[test]
        fn sticks_to_prev_token() {
            assert_cursor("a|   :=|    b|", "a| :=| b|");
            assert_cursor("unit A;begin a| end;", "unit A;\nbegin\n  a|\nend;");
        }

        #[test]
        fn sticks_to_prev_over_next_token() {
            assert_cursor("a|*|b|+|c", "a| *| b| +| c");
            assert_cursor("begin a|;|end;", "begin\n  a|;|\nend;");
        }

        #[test]
        fn sticks_to_margin() {
            assert_cursor("a;\n|\nb;", "a;\n|\nb;");
            assert_cursor("begin\n|\na;end", "begin\n|\n  a;\nend");
            assert_cursor("begin\n| |   a;end", "begin\n| | a;\nend");
            assert_cursor("begin\na;\n|\n|\n|a;end;", "begin\n  a;\n||\n  |a;\nend;");
            assert_cursor(";\n|\n  |\n| a", ";\n||\n|a");
            assert_cursor("a;\n|\n|\na;", "a;\n||\na;");
        }

        #[test]
        fn unwrapped_lines() {
            assert_cursor("(\n|\na)", "(|a)");
            assert_cursor("a\n| | |\na", "a|| |a");
        }

        #[test]
        fn on_changed_whitespace() {
            assert_cursor("a |   | |   :=b;", "a |||:= b;");
            assert_cursor("a |  | |    :=   |    b;", "a |||:= |b;");
        }

        #[test]
        fn mid_token() {
            assert_cursor("a      :=b|b|b|b;", "a := b|b|b|b;");
        }

        #[test]
        fn around_ignored_tokens() {
            assert_cursor(
                "\
const a=b;
c=d   | | | ! |     |asdf|;",
                "\
const
  a = b;
  c = d|| |     !| |asdf|;",
            );
        }

        #[test]
        fn around_deleted_tokens() {
            assert_cursor("a | |{|} b  {}   {}|", "a |||b|");
        }

        #[test]
        fn multiline_tokens() {
            assert_cursor(
                "\
A = 1 + {

} | | | | | | | | A;

A = 1     + '''
''' + | | | | Spaces;",
                "\
A
    = 1
        +
        {

}
  | | | | | | ||A;

A = 1 + '''
''' + | | | |  Spaces;",
            );
        }
    }
}
