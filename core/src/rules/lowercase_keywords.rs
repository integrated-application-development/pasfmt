use crate::prelude::*;

pub struct LowercaseKeywords {}

impl LogicalLineFileFormatter for LowercaseKeywords {
    fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, _input: &[LogicalLine]) {
        for (tok, _) in formatted_tokens.tokens_mut() {
            let Ok(tok) = tok else { continue };
            if matches!(tok.get_token_type(), TokenType::Keyword(_))
                && tok.get_content().bytes().any(|b| b.is_ascii_uppercase())
            {
                tok.set_content(tok.get_content().to_ascii_lowercase());
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
            .file_formatter(LowercaseKeywords {})
            .reconstructor(default_test_reconstructor())
            .build()
    }

    formatter_test_group!(
        tests,
        full_uppercase = {
            "BEGIN END",
            "begin end"
        },
        partial_uppercase = {
            "begIn enD",
            "begin end"
        },
        lowercase = {
            "begin end",
            "begin end"
        },
        impure_keyword_is_ignored = {
            "ABSOLUTE := 0",
            "ABSOLUTE := 0",
        },
        impure_keyword_is_formatted = {
            "var a: b ABSOLUTE c",
            "var a: b absolute c",
        },
        ignored_tokens = {
            "{pasfmt off} BEGIN {pasfmt on} END",
            "{pasfmt off} BEGIN {pasfmt on} end",
        }
    );
}
