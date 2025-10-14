use itertools::Itertools;

use crate::prelude::*;

pub struct CommentFormatter {}

fn format_line_comment(tok: &mut Token) {
    fn comment_is_separator(comment: &str) -> bool {
        let comment = comment.trim_ascii_end();
        comment.len() >= 10
            && comment.chars().next().is_some_and(|b| !b.is_alphanumeric())
            && comment.chars().all_equal()
    }

    let content = tok.get_content();
    let mut new_content: Option<String> = None;
    let Some(mut comment) = content.strip_prefix("//") else {
        return;
    };

    // doc comments have an extra slash
    comment = comment.strip_prefix('/').unwrap_or(comment);

    if comment
        .bytes()
        .next()
        .is_some_and(|b| !b.is_ascii_whitespace())
        && !comment_is_separator(comment)
    {
        let mut str = String::with_capacity(content.len() + 1);
        str.push_str(&content[..content.len() - comment.len()]);
        str.push(' ');
        str.push_str(comment);
        new_content = Some(str);
    }

    let trimmed = content.trim_ascii_end();
    if trimmed.len() != content.len() {
        let new_content = new_content.get_or_insert_with(|| content.to_string());
        new_content.truncate(new_content.trim_ascii_end().len());
    }

    if let Some(new_content) = new_content {
        tok.set_content(new_content);
    }
}

fn format_compiler_directive(tok: &mut Token) {
    let content = tok.get_content();

    let Some(stripped) = content
        .strip_prefix("{$")
        .or_else(|| content.strip_prefix("(*$"))
    else {
        return;
    };

    enum State {
        Before,
        AfterPlusMinus,
        AfterDigit,
        AfterComma,
        AfterLetter,
        AfterWord,
    }
    let mut is_switch = false;
    let mut state = State::Before;
    let mut directive_len = 0;
    for b in stripped.bytes() {
        match (state, b) {
            (State::Before | State::AfterComma, b'a'..=b'z' | b'A'..=b'Z') => {
                state = State::AfterLetter;
            }

            (State::AfterLetter, b'+' | b'-') => {
                state = State::AfterPlusMinus;
                is_switch = true;
            }

            (State::AfterPlusMinus | State::AfterDigit, b',') => {
                state = State::AfterComma;
            }

            (State::AfterLetter | State::AfterDigit, b'0'..=b'9') => {
                state = State::AfterDigit;
                is_switch = true;
            }

            (
                State::AfterLetter | State::AfterWord,
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_',
            ) if !is_switch => {
                state = State::AfterWord;
            }

            // A comma after a letter is only valid in the `AfterWord` case
            (State::AfterLetter, b',') => return,

            // In a switch directive, commas and letters must be followed by something else
            (State::AfterComma | State::AfterLetter, _) => return,

            _ => break,
        };

        directive_len += 1;
    }

    let directive = &stripped[..directive_len];

    if directive.bytes().any(|b| b.is_ascii_lowercase()) {
        let mut str = String::with_capacity(content.len());
        let prefix = &content[..content.len() - stripped.len()];
        str.push_str(prefix);
        str.extend(directive.chars().map(|c| c.to_ascii_uppercase()));
        let rest = &stripped[directive.len()..];
        str.push_str(rest);

        tok.set_content(str);
    }
}

impl LogicalLineFileFormatter for CommentFormatter {
    fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, _input: &[LogicalLine]) {
        for (tok, _) in formatted_tokens.tokens_mut() {
            let Ok(tok) = tok else { continue };
            match tok.get_token_type() {
                TokenType::CompilerDirective | TokenType::ConditionalDirective(_) => {
                    format_compiler_directive(tok)
                }
                TokenType::Comment(
                    CommentKind::InlineBlock
                    | CommentKind::IndividualBlock
                    | CommentKind::MultilineBlock,
                ) => {
                    // do nothing, currently
                }
                TokenType::Comment(CommentKind::InlineLine | CommentKind::IndividualLine) => {
                    format_line_comment(tok)
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn formatter() -> Formatter {
        Formatter::builder()
            .lexer(DelphiLexer {})
            .parser(DelphiLogicalLineParser {})
            .token_ignorer(FormattingToggler {})
            .file_formatter(CommentFormatter {})
            .reconstructor(default_test_reconstructor())
            .build()
    }

    formatter_test_group!(
        block_comments,
        not_yet_supported = {
            "{       a        }",
            "{       a        }",
        },
    );

    formatter_test_group!(
        line_comments,
        no_space_at_start = {
            "//a",
            "// a",
        },
        empty = {
            "//",
            "//",
        },
        multiple_spaces_at_start_remain = {
            "//  a",
            "//  a",
        },
        any_ascii_whitespace_at_start_remains = {
            "
            //\x09a
            //\x0Cb
            ",
            "
            //\x09a
            //\x0Cb
            ",
        },
        ascii_whitespace_at_end_is_trimmed = {
            "// a\u{A0}\x20\x09\x0C",
            "// a\u{A0}",
        },
        insert_at_start_and_trim_at_end = {
            "//a ",
            "// a",
        },
        empty_doc = {
            "///",
            "///",
        },
        no_space_at_start_doc = {
            "///a",
            "/// a",
        },
        slash_after_doc = {
            "////",
            "/// /",
        },
        separators_ignored = {
            "
            //----------
            //----------\x20\x09
            // ----------
            //--------------------
            //++++++++++
            //,,,,,,,,,,
            //__________
            //##########
            //[[[[[[[[[[
            ",
            "
            //----------
            //----------
            // ----------
            //--------------------
            //++++++++++
            //,,,,,,,,,,
            //__________
            //##########
            //[[[[[[[[[[
            ",
        },
        not_quite_separators_not_ignored = {
            "
            //---------
            //---------+
            //++-+++++++
            //OOOOOOOOOO
            ",
            "
            // ---------
            // ---------+
            // ++-+++++++
            // OOOOOOOOOO
            ",
        },
    );

    formatter_test_group!(
        compiler_directives,
        brace_style = {
            "{$define foo}",
            "{$DEFINE foo}",
        },
        paren_star_style = {
            "(*$define foo*)",
            "(*$DEFINE foo*)",
        },
        already_uppercase = {
            "{$DEFINE foo}",
            "{$DEFINE foo}",
        },
        nested_style = {
            "(*$message '}'*){$message '*)'}",
            "(*$MESSAGE '}'*){$MESSAGE '*)'}",
        },
        invalid_space_at_start = {
            "(*$ define foo*)",
            "(*$ define foo*)",
        },
        no_space_after = {
            "{$message''}",
            "{$MESSAGE''}",
        },
        switch_directives = {
            "{$o+}{$r-}{$z2}{$a16}",
            "{$O+}{$R-}{$Z2}{$A16}",
        },
        batched_switch_directives = {
            "{$o+,r-,b+,a+,a1,a2,a4,a8,a16,z1,z2,z4}{$a1,b+}",
            "{$O+,R-,B+,A+,A1,A2,A4,A8,A16,Z1,Z2,Z4}{$A1,B+}",
        },
        fake_batched_directives = {
            "{$if,comment}",
            "{$IF,comment}",
        },
        unknown_directive_names = {
            "{$asdf}{$fdsa}{$as_df}{$as09df}{$asdf09}{$zyxw}",
            "{$ASDF}{$FDSA}{$AS_DF}{$AS09DF}{$ASDF09}{$ZYXW}",
        },
        invalid_directive_names_ignored = {
            "{$0asdf}{$,a}{$a,b}",
            "{$0asdf}{$,a}{$a,b}",
        },
        incomplete_switch_directive_ignored = {
            "{$a}{$a+,b}{$a+,}",
            "{$a}{$a+,b}{$a+,}",
        },
        unusual_word_breaks = {
            "{$a1a}{$a+!a}{$aa-a}{$a_b=a}",
            "{$A1a}{$A+!a}{$AA-a}{$A_B=a}",
        },
        ignored_tokens = {
            "{$r+}{pasfmt off}{$r+}",
            "{$R+}{pasfmt off}{$r+}",
        },
        trailing_whitespace_ignored = {
            "{$DEFINE foo  }",
            "{$DEFINE foo  }",
        },
    );

    formatter_test_group!(
        conditional_directives,
        simple_expressions = {
            "{$if foo}{$elseif bar}{$endif}",
            "{$IF foo}{$ELSEIF bar}{$ENDIF}",
        },
        complex_expressions_not_formatted = {
            "{$if  (foo>bar  ) and true   }",
            "{$IF  (foo>bar  ) and true   }",
        },
        all_directives = {
            "{$ifdef FOo}{$ifndef fOo}{$if fOo}{$elseif fOo}{$else}{$endif}{$ifend}",
            "{$IFDEF FOo}{$IFNDEF fOo}{$IF fOo}{$ELSEIF fOo}{$ELSE}{$ENDIF}{$IFEND}",
        },
        nested_directives_not_formatted = {
            "{$if {$include foo.inc}}",
            "{$IF {$include foo.inc}}",
        },
        tricky_quoting = {
            "{$if foo = '}'}{$if bar}",
            "{$IF foo = '}'}{$IF bar}",
        },
    );
}
