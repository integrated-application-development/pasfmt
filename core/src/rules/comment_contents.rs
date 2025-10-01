use crate::prelude::*;

pub struct CommentFormatter {}

fn format_line_comment(tok: &mut Token) {
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

impl LogicalLineFileFormatter for CommentFormatter {
    fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, _input: &[LogicalLine]) {
        for (tok, _) in formatted_tokens.tokens_mut() {
            let Ok(tok) = tok else { continue };
            match tok.get_token_type() {
                TokenType::CompilerDirective => {
                    // do nothing, currently
                }
                TokenType::ConditionalDirective(_) => {
                    // do nothing, currently
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
    );
}
