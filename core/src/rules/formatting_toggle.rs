use crate::lang::*;
use crate::prelude::*;

use log::warn;

enum FormattingToggle {
    On,
    Off,
}

fn count_prefix_bytes(input: &str, f: impl Fn(&u8) -> bool) -> usize {
    input.bytes().take_while(f).count()
}

fn strip_prefix_bytes(input: &str, f: impl Fn(&u8) -> bool) -> &str {
    &input[count_prefix_bytes(input, f)..]
}

fn strip_prefix_bytes1(input: &str, f: impl Fn(&u8) -> bool) -> Option<&str> {
    match count_prefix_bytes(input, f) {
        0 => None,
        count => Some(&input[count..]),
    }
}

fn starts_with_icase(input: &str, prefix: &str) -> bool {
    input.is_char_boundary(prefix.len()) && input[..prefix.len()].eq_ignore_ascii_case(prefix)
}

// `core::str::strip_prefix` has no case-insensitive equivalent in core, unfortunately
fn strip_prefix_icase<'a>(input: &'a str, prefix: &str) -> Option<&'a str> {
    if input.len() < prefix.len() || !starts_with_icase(input, prefix) {
        return None;
    }
    Some(&input[prefix.len()..])
}

fn parse_pasfmt_toggle(input: &str) -> Option<FormattingToggle> {
    let word = &input[..count_prefix_bytes(input, u8::is_ascii_alphanumeric)];
    if word.eq_ignore_ascii_case("on") {
        Some(FormattingToggle::On)
    } else if word.eq_ignore_ascii_case("off") {
        Some(FormattingToggle::Off)
    } else {
        warn!("pasfmt directive comment found but '{word}' is neither 'on' nor 'off'");
        None
    }
}

fn parse_pasfmt_directive_comment_contents(input: &str) -> Option<FormattingToggle> {
    let input = strip_prefix_bytes(input, u8::is_ascii_whitespace);
    let input = strip_prefix_icase(input, "pasfmt")?;
    let input = strip_prefix_bytes1(input, u8::is_ascii_whitespace)?;
    parse_pasfmt_toggle(input)
}

fn parse_toggle(content: &str) -> Option<FormattingToggle> {
    let content = content
        .strip_prefix("//")
        .or_else(|| content.strip_prefix("(*"))
        .or_else(|| content.strip_prefix('{'))?;

    parse_pasfmt_directive_comment_contents(content)
}

pub struct FormattingToggler {}
impl TokenIgnorer for FormattingToggler {
    fn ignore_tokens(&self, input: (&[Token], &[LogicalLine]), token_marker: &mut TokenMarker) {
        let mut ignored = false;
        for (i, token) in input.0.iter().enumerate() {
            let mut on_toggle_comment = false;
            if let TokenType::Comment(_) = token.get_token_type() {
                on_toggle_comment = true;
                match parse_toggle(token.get_content()) {
                    Some(FormattingToggle::Off) => ignored = true,
                    Some(FormattingToggle::On) => ignored = false,
                    None => on_toggle_comment = false,
                };
            }

            if ignored | on_toggle_comment {
                token_marker.mark(i);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use spectral::prelude::*;

    use crate::prelude::*;
    use crate::test_utils::formatter_test_group;

    struct AddSpaceBeforeEverything {}
    impl LogicalLineFileFormatter for AddSpaceBeforeEverything {
        fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, _input: &[LogicalLine]) {
            for token_index in 0..formatted_tokens.get_tokens().len() {
                if let Some(formatting_data) = formatted_tokens
                    .get_token_type_for_index(token_index)
                    .filter(|token_type| token_type != &TokenType::Eof)
                    .and_then(|_| formatted_tokens.get_formatting_data_mut(token_index))
                {
                    formatting_data.spaces_before += 1;
                }
            }
        }
    }

    fn formatter() -> Formatter {
        Formatter::builder()
            .lexer(DelphiLexer {})
            .parser(DelphiLogicalLineParser {})
            .token_ignorer(FormattingToggler {})
            .file_formatter(AddSpaceBeforeEverything {})
            .reconstructor(default_test_reconstructor())
            .build()
    }

    formatter_test_group!(
        formatting_toggle,
        not_disabled = {"Foo(Bar + Baz)", " Foo ( Bar  +  Baz )"},
        disabled_single_line = {
            indoc! {
              "
              Foo(Bar);
              // pasfmt off
              Foo(Bar);
              "
            },
            indoc! {
              "
               Foo ( Bar ) ;
              // pasfmt off
              Foo(Bar);
              "
            },
        },
        case_insensitive = {
            indoc! {
              "
              Foo(Bar);
              // PASFMT off
              Foo(Bar);
              // PaSFmt on
              Foo(Bar);
              "
            },
            indoc! {
              "
               Foo ( Bar ) ;
              // PASFMT off
              Foo(Bar);
              // PaSFmt on
               Foo ( Bar ) ;
              "
            },
        },
        disabled_and_reenabled = {
            indoc! {
              "
              Foo(Bar);
              // pasfmt off
              Foo(Bar);
              Foo(Bar);
              // pasfmt on
              Foo(Bar);
              // pasfmt off
              Foo(Bar);
              "
            },
            indoc! {
              "
               Foo ( Bar ) ;
              // pasfmt off
              Foo(Bar);
              Foo(Bar);
              // pasfmt on
               Foo ( Bar ) ;
              // pasfmt off
              Foo(Bar);
              "
            },
        },
        disabled_with_block_comment = {
            indoc! {
              "
              Foo(Bar);
              { pasfmt off }
              Foo(Bar);
              "
            },
            indoc! {
              "
               Foo ( Bar ) ;
              { pasfmt off }
              Foo(Bar);
              "
            },
        },
        disabled_with_block_comment_alt = {
            indoc! {
              "
              Foo(Bar);
              (* pasfmt off *)
              Foo(Bar);
              "
            },
            indoc! {
              "
               Foo ( Bar ) ;
              (* pasfmt off *)
              Foo(Bar);
              "
            },
        },
        whitespace_at_start_of_toggle = {
            indoc! {
              "
              Foo(Bar);
              {
                pasfmt off
              }
              Foo(Bar);
              "
            },
            indoc! {
              "
               Foo ( Bar ) ;
              {
                pasfmt off
              }
              Foo(Bar);
              "
            },
        },
        no_whitespace_around_toggle = {
            indoc! {
              "
              Foo(Bar);
              {pasfmt off}
              Foo(Bar);
              "
            },
            indoc! {
              "
               Foo ( Bar ) ;
              {pasfmt off}
              Foo(Bar);
              "
            },
        },
        trailing_content_in_toggle = {
            indoc! {
              "
              Foo(Bar);
              // pasfmt off (because of xyz)
              Foo(Bar);
              {
                pasfmt on
                (because of zyx)
              }
              Foo(Bar);
              "
            },
            indoc! {
              "
               Foo ( Bar ) ;
              // pasfmt off (because of xyz)
              Foo(Bar);
              {
                pasfmt on
                (because of zyx)
              }
               Foo ( Bar ) ;
              "
            },
        },
        word_break_after_off_on = {
            indoc! {
              "
              Foo(Bar);
              // pasfmt offa
              Foo(Bar);
              // pasfmt off1
              Foo(Bar);
              // pasfmt off!
              Foo(Bar);
              // pasfmt onA
              Foo(Bar);
              // pasfmt on)
              Foo(Bar);
              "
            },
            indoc! {
              "
               Foo ( Bar ) ;
               // pasfmt offa
               Foo ( Bar ) ;
               // pasfmt off1
               Foo ( Bar ) ;
              // pasfmt off!
              Foo(Bar);
              // pasfmt onA
              Foo(Bar);
              // pasfmt on)
               Foo ( Bar ) ;
              "
            },
        },
        ignore_through_conditional_directives = {
            indoc! {
              "
              Foo(
                a,
                {$ifdef A}
                // pasfmt off
                b+b1,
                {$endif}
                // pasfmt on
                c
              )
              // pasfmt off
              "
            },
            indoc! {
              "
               Foo (
                 a ,
                 {$ifdef A}
                // pasfmt off
                b+b1,
                {$endif}
                // pasfmt on
                 c
               )
              // pasfmt off
              "
            },
        },
        non_ascii = {
            indoc! {
              "
              Foo(Bar);
              // pasfmx off
              Foo(Bar);
              // pasfm£ off
              Foo(Bar);
              // pasfm† off
              Foo(Bar);
              // pasfm🚀 off
              Foo(Bar);
              // pasfmt off
              Foo(Bar);
              "
            },
            indoc! {
              "
               Foo ( Bar ) ;
               // pasfmx off
               Foo ( Bar ) ;
               // pasfm£ off
               Foo ( Bar ) ;
               // pasfm† off
               Foo ( Bar ) ;
               // pasfm🚀 off
               Foo ( Bar ) ;
              // pasfmt off
              Foo(Bar);
              "
            }
        }
    );
}
