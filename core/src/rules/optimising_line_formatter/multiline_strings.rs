use crate::prelude::*;

pub(super) struct StringFormatter<'a> {
    pub(super) recon_settings: &'a ReconstructionSettings,
}

fn lines_custom(input: &str) -> impl Iterator<Item = &str> {
    let mut skip_next_nl = false;
    input
        .split_inclusive(move |c| {
            if skip_next_nl && c == '\n' {
                skip_next_nl = false;
                return false;
            }
            skip_next_nl = c == '\r';
            matches!(c, '\r' | '\n')
        })
        .map(|line| line.trim_matches(['\n', '\r']))
}

impl StringFormatter<'_> {
    /// Formats multiline strings in the provided logical line.
    ///
    /// 1. Normalises indentation of all internal lines to match the opening quotes.
    /// 2. Normalises trailing line seperators.
    ///
    /// Returns true if and only if a token is mutated.
    pub(super) fn format_multiline_strings(
        &self,
        line: &LogicalLine,
        tokens: &mut FormattedTokens,
    ) -> bool {
        let mut changed = false;

        for &idx in line.get_tokens() {
            let (tok, fmt) = tokens.get_token_mut(idx).unwrap();
            let tok = match tok.map(|tok| (tok.get_token_type(), tok)) {
                Ok((TokenType::TextLiteral(TextLiteralKind::MultiLine), tok)) => tok,
                _ => continue,
            };

            let last_line = tok.get_content().lines().last().unwrap();
            let base_indentation = &last_line[0..count_leading_whitespace(last_line)];

            if base_indentation.len() != last_line.trim_end_matches('\'').len() {
                log::warn!(
                    "Last line of multiline string contains non-whitespace before trailing quote: {last_line:?}"
                );
                continue;
            };

            if let Some(new_string_contents) =
                self.try_rewrite_string(tok.get_content(), fmt, base_indentation)
            {
                if new_string_contents != tok.get_content() {
                    tok.set_content(new_string_contents);
                    changed = true
                }
            }
        }
        changed
    }

    fn try_rewrite_string(
        &self,
        original: &str,
        indent: &FormattingData,
        base_indentation: &str,
    ) -> Option<String> {
        let mut contents = String::with_capacity(original.len());

        // Inside a multiline string, all of LF, CR, and CRLF will terminate the internal line.
        // The case of a lone CR is tricky to handle, and requires a custom lines iterator.
        let mut lines = lines_custom(original);

        contents.extend(lines.next());

        for line in lines {
            contents.push_str(self.recon_settings.get_newline_str());

            let Some(stripped_line) = line.strip_prefix(base_indentation) else {
                if base_indentation.starts_with(line) {
                    // it's permitted for the internal lines to only match a prefix of the trailing quote's whitespace
                    // so long as there is nothing after that
                    continue;
                }

                log::warn!(
                    "Whitespace inside line of multiline string does not match whitespace before trailing quote: {line:?}"
                );
                return None;
            };

            if !stripped_line.is_empty() {
                (0..indent.indentations_before)
                    .for_each(|_| contents.push_str(self.recon_settings.get_indentation_str()));
                (0..indent.continuations_before)
                    .for_each(|_| contents.push_str(self.recon_settings.get_continuation_str()));
                contents.push_str(stripped_line);
            }
        }

        Some(contents)
    }
}
