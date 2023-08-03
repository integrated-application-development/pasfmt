use crate::lang::*;
use crate::traits::LogicalLineFileFormatter;

use nom::branch::*;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::sequence::*;
use nom::*;

enum FormattingToggle {
    On,
    Off,
}

fn parse_pasfmt_toggle(input: &str) -> IResult<&str, FormattingToggle> {
    if input.eq_ignore_ascii_case("off") {
        Ok(("", FormattingToggle::Off))
    } else if input.eq_ignore_ascii_case("on") {
        Ok(("", FormattingToggle::On))
    } else {
        fail(input)
    }
}

fn parse_pasfmt_directive_comment_contents(input: &str) -> IResult<&str, FormattingToggle> {
    delimited(
        tuple((multispace0, tag_no_case("pasfmt"), multispace1)),
        parse_pasfmt_toggle,
        success(0),
    )(input)
}

pub struct FormattingToggler {}
impl LogicalLineFileFormatter for FormattingToggler {
    fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &[LogicalLine]) {
        let mut ignored = false;
        for line in input {
            for &index in line.get_tokens() {
                let mut on_toggle_comment = false;
                if let Some((token, _)) = formatted_tokens.get_token(index) {
                    if let TokenType::Comment(_) = token.get_token_type() {
                        let parse_result: IResult<_, _> =
                            delimited(
                                alt((tag("{"), tag("(*"), tag("//"))),
                                parse_pasfmt_directive_comment_contents,
                                success(0),
                            )(token.get_content());
                        match parse_result {
                            Ok((_, FormattingToggle::Off)) => ignored = true,
                            Ok((_, FormattingToggle::On)) => ignored = false,
                            _ => {}
                        };
                        on_toggle_comment = parse_result.is_ok();
                    }
                }

                if ignored | on_toggle_comment {
                    if let Some(formatting_data) = formatted_tokens.get_formatting_data_mut(index) {
                        formatting_data.ignored = true;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use spectral::prelude::*;

    use crate::prelude::*;
    use crate::rules::test_utils::formatter_test_group;
    use crate::traits::LogicalLineFormatter;

    struct AddSpaceBeforeEverything {}
    impl LogicalLineFormatter for AddSpaceBeforeEverything {
        fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &LogicalLine) {
            for &token_index in input.get_tokens() {
                if let Some(formatting_data) = formatted_tokens
                    .get_token_type_for_index(token_index)
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
            .line_formatter(AddSpaceBeforeEverything {})
            .file_formatter(FormattingToggler {})
            .reconstructor(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_string(), "  ".to_string(), "  ".to_string()),
            ))
            .build()
    }

    formatter_test_group!(
        formatting_toggle,
        not_disabled = {"Foo(Bar + Baz)", " Foo ( Bar  +  Baz ) "},
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
        }
    );
}
