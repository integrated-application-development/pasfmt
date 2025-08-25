use itertools::Itertools;

use crate::{
    lang::{
        CommentKind, LogicalLine, LogicalLineType, OperatorKind, TextLiteralKind, Token, TokenData,
        TokenType,
    },
    traits::LogicalLinesConsolidator,
};
use TokenType as TT;

pub struct ConditionalDirectiveConsolidator {}
impl ConditionalDirectiveConsolidator {
    fn is_allowed_token(tokens: &[Token], token_index: usize) -> bool {
        matches!(
            tokens.get(token_index).map(Token::get_token_type),
            Some(
                TT::Identifier
                    | TT::NumberLiteral(_)
                    | TT::TextLiteral(TextLiteralKind::SingleLine)
                    | TT::Comment(
                        CommentKind::IndividualBlock
                            | CommentKind::IndividualLine
                            | CommentKind::InlineBlock
                            | CommentKind::InlineLine
                    )
                    | TT::Op(OperatorKind::Dot),
            )
        )
    }

    fn expand_line(tokens: &[Token], line: &mut LogicalLine) -> Vec<usize> {
        let line_tokens = line.get_tokens();
        let (Some(&first_token), Some(&last_token)) = (line_tokens.first(), line_tokens.last())
        else {
            return vec![];
        };

        if last_token - first_token + 1 == line_tokens.len() {
            // if this equality holds, the tokens are sequential and there is
            // nothing to do
            return vec![];
        }

        enum ConditionalState {
            Outside,
            AfterIf,
            AfterElse,
        }
        use ConditionalState as CS;

        let mut state = ConditionalState::Outside;
        let mut directives: Vec<usize> = Vec::with_capacity(3);
        let mut new_line_tokens = vec![first_token];

        for (&prev, &current) in line.get_tokens().iter().tuple_windows() {
            if current - prev > 1 {
                let gap_start_tok = prev + 1;
                let gap_end_tok = current - 1;
                match (
                    tokens.get(gap_start_tok).map(Token::get_token_type),
                    tokens.get(gap_end_tok).map(Token::get_token_type),
                ) {
                    (
                        Some(TT::ConditionalDirective(b_kind)),
                        Some(TT::ConditionalDirective(e_kind)),
                    ) => match &state {
                        CS::Outside if b_kind.is_if() => {
                            state = match e_kind {
                                _ if gap_start_tok == gap_end_tok => CS::AfterIf,
                                k if k.is_else() => CS::AfterElse,
                                _ => return vec![],
                            };
                        }
                        CS::AfterIf if b_kind.is_else() => {
                            state = match e_kind {
                                _ if gap_start_tok == gap_end_tok => CS::AfterElse,
                                k if k.is_end() => CS::Outside,
                                _ => return vec![],
                            };
                        }
                        CS::AfterIf | CS::AfterElse
                            if b_kind.is_end() && gap_start_tok == gap_end_tok =>
                        {
                            state = CS::Outside;
                        }
                        // the gap is an excluded conditional directive pattern
                        _ => return vec![],
                    },
                    // the gap is not conditionally excluded code
                    _ => return vec![],
                }

                directives.push(gap_start_tok);
                new_line_tokens.push(gap_start_tok);

                for gap_token in (gap_start_tok..gap_end_tok).skip(1) {
                    if Self::is_allowed_token(tokens, gap_token) {
                        new_line_tokens.push(gap_token);
                    } else {
                        return vec![];
                    }
                }

                if gap_end_tok != gap_start_tok {
                    directives.push(gap_end_tok);
                    new_line_tokens.push(gap_end_tok);
                }
            }

            if !(matches!(state, CS::Outside) || Self::is_allowed_token(tokens, current)) {
                return vec![];
            }
            new_line_tokens.push(current);
        }

        if directives.is_empty() || !matches!(state, ConditionalState::Outside) {
            // there were no or incomplete directives in the line, ignore
            return vec![];
        }

        // Add all non-conditional tokens, merging the lines
        *line.get_tokens_mut() = new_line_tokens;
        directives
    }
}
impl LogicalLinesConsolidator for ConditionalDirectiveConsolidator {
    fn consolidate(&self, (tokens, lines): (&mut [Token], &mut [LogicalLine])) {
        let mut directive_tokens: Vec<usize> = Vec::new();
        for line in lines.iter_mut() {
            directive_tokens.extend(Self::expand_line(tokens, line));
        }

        // In theory, it could be more efficient later to deduplicate the
        // expanded lines. This is more complicated than it is worth to ensure
        // all child lines' parent remain correct.

        let first_token =
            |line: &&mut LogicalLine| line.get_tokens().first().copied().unwrap_or_default();

        directive_tokens.sort();
        directive_tokens.dedup();

        let mut directive_lines = lines
            .iter_mut()
            .filter(|line| line.get_line_type() == LogicalLineType::ConditionalDirective)
            .sorted_by_key(first_token)
            .collect_vec();

        for directive in &directive_tokens {
            if let Ok(directive_line) = directive_lines.binary_search_by_key(directive, first_token)
            {
                directive_lines[directive_line].void_and_drain();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::prelude::*;

    use super::*;

    fn consolidate_n_times(tokens: &mut [Token], lines: &mut [LogicalLine], n: u8) {
        for _ in 0..n {
            ConditionalDirectiveConsolidator {}.consolidate((tokens, lines));
        }
    }

    fn format_input(input: &str, n: u8) -> String {
        let tokens = DelphiLexer {}.lex(input);
        let (mut lines, mut tokens) = DelphiLogicalLineParser {}.parse(tokens);
        consolidate_n_times(&mut tokens, &mut lines, n);
        let mut formatted_tokens =
            FormattedTokens::new_from_tokens(&mut tokens, &TokenMarker::default());
        for line in lines {
            if let Some(&first) = line.get_tokens().first() {
                let data = formatted_tokens
                    .get_formatting_data_mut(first)
                    .expect("Formatting data should exist for the first token");
                data.spaces_before = 0;
                data.newlines_before = if first == 0
                    || matches!(
                        line.get_line_type(),
                        LogicalLineType::Eof | LogicalLineType::Voided
                    ) {
                    0
                } else {
                    1
                };
            }
            for &token in line.get_tokens().iter().skip(1) {
                let data = formatted_tokens
                    .get_formatting_data_mut(token)
                    .expect("Formatting data should exist for the line token");
                data.spaces_before = 1;
                data.newlines_before = 0;
            }
        }

        let mut out = String::new();
        DelphiLogicalLinesReconstructor::new(default_test_reconstruction_settings())
            .reconstruct(formatted_tokens, &mut out);
        out
    }

    fn assert_unchanged(input: &str) {
        assert_eq!(input, format_input(input, 1), "Input should be unchanged");
    }

    fn assert_unchanged_repeated_consolidation(input: &str) {
        assert_eq!(input, format_input(input, 2), "Input should be unchanged");
    }

    #[test]
    fn no_conditional_directives() {
        assert_unchanged("A := B ;");
        assert_unchanged("A := B + C ;");
    }

    #[test]
    fn simple_if_conditional_directives() {
        assert_unchanged("A := {$ifdef A} B {$endif} + C ;");
        assert_unchanged("A := {$ifndef A} B {$endif} + C ;");
        assert_unchanged("A := {$ifopt A} B {$endif} + C ;");
        assert_unchanged("A := {$if A} B {$ifend} + C ;");
        assert_unchanged(
            "\
            A := B\n\
            {$ifdef} + B\n\
            {$endif} ;",
        );
    }

    #[test]
    fn simple_if_else_conditional_directives() {
        assert_unchanged("A := {$ifdef A} B1 {$else} B2 {$endif} + C ;");
        assert_unchanged("A := {$ifndef A} B1 {$elseif} B2 {$endif} + C ;");
        assert_unchanged("A := {$ifopt A} B1 {$else} B2 {$endif} + C ;");
        assert_unchanged("A := {$if A} B1 {$elseif} B2 {$ifend} + C ;");
        assert_unchanged("A := {$if A} B1 {} {$else} B2 {} {$ifend} + C ;");
        assert_unchanged(
            "\
            A := B\n\
            {$ifdef} + B1\n\
            {$else} + B2\n\
            {$endif} ;",
        );
    }

    #[test]
    fn repeated_consolidation() {
        assert_unchanged_repeated_consolidation("A := {$ifdef A} B1 {$else} B2 {$endif} + C ;");
        assert_unchanged_repeated_consolidation(
            "\
            A := B\n\
            {$ifdef} + B1\n\
            {$else} + B2\n\
            {$endif} ;",
        );
    }

    #[test]
    fn simple_child_line_if_else_conditional_directives() {
        assert_unchanged(
            "\
            if A then\n\
            A := {$ifdef A} B1 {$else} B2 {$endif} + C ;",
        );
        assert_unchanged(
            "\
            for A in b do\n\
            A := {$ifndef A} B1 {$elseif} B2 {$endif} + C ;",
        );
        assert_unchanged(
            "\
            while A do\n\
            A := {$ifopt A} B1 {$else} B2 {$endif} + C ;",
        );
        assert_unchanged(
            "\
            with A do\n\
            A := {$if A} B1 {$elseif} B2 {$ifend} + C ;",
        );
        assert_unchanged(
            "\
            if A then\n\
            A := {$if A} B1 {} {$else} B2 {} {$ifend} + C ;",
        );
        assert_unchanged(
            "\
            while A do\n\
            A := B\n\
            {$ifdef} + B1\n\
            {$else} + B2\n\
            {$endif} ;",
        );
    }

    #[test]
    fn identifier_chaining_if_else_conditional_directives() {
        assert_unchanged("A := {$ifdef A} B1 . B2 {$else} B1 . B2 {$endif} + C ;");
        assert_unchanged("A := {$ifdef A} B1 . B2 . B3 {$else} B1 . B2 . B3 {$endif} + C ;");
    }

    #[test]
    fn if_else_on_whole_child_lines() {
        assert_unchanged(
            "\
            if A then\n\
            begin\n\
            {$ifdef A}\n\
            A := B ;\n\
            {$else}\n\
            C := D . E ;\n\
            {$endif}\n\
            end ;",
        );
    }

    #[test]
    fn ignored_cases() {
        assert_unchanged(
            "\
            A :=\n\
            {$ifdef A} A + B\n\
            {$else} C + D\n\
            {$endif} + E ;",
        );
        assert_unchanged(
            "\
            A :=\n\
            {$ifndef A} A ( )\n\
            {$else} B ( )\n\
            {$endif} ;",
        );
        assert_unchanged(
            "\
            A := A (\n\
            {$ifopt A} )\n\
            {$else} )\n\
            {$endif} ;",
        );
        assert_unchanged(
            "\
            type\n\
            A = packed record\n\
            {$if} //\n\
            F : F ;\n\
            end ;",
        );
        assert_unchanged(
            "\
            A := procedure begin\n\
            {$if A}\n\
            B1 ; end\n\
            {$else}\n\
            B2 ; end\n\
            {$endif} ;",
        );
        assert_unchanged(
            "\
            A := procedure begin\n\
            B1 ; end ;\n\
            {$if}\n\
            {$endif}",
        );
        assert_unchanged(
            "\
            A :=\n\
            {$if} {
                Multiline comment
            }\n\
            {$endif}",
        );
        assert_unchanged(
            "\
            A :=\n\
            {$ifdef A}\n\
            {$elseif} A\n\
            {$else} A\n\
            {$endif} + E ;",
        );
    }

    #[test]
    fn ignored_nested_directive_cases() {
        assert_unchanged(
            "\
            A :=\n\
            {$ifdef A}\n\
            {$if} A\n\
            {$endif}\n\
            {$else}\n\
            {$if} A\n\
            {$endif}\n\
            {$endif} + E ;",
        );
        assert_unchanged(
            "\
            A :=\n\
            {$ifdef A}\n\
            {$if} A\n\
            {$endif}\n\
            {$endif} + E ;",
        );
        assert_unchanged(
            "\
            {$if}\n\
            A :=\n\
            {$if} A\n\
            {$else} B\n\
            {$endif}\n\
            {$endif} ;",
        );
    }
}
