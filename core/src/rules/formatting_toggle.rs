use crate::lang::*;
use crate::traits::LogicalLineFileFormatter;

use log::warn;
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
    let (input, word) = alphanumeric1(input)?;
    if word.eq_ignore_ascii_case("on") {
        Ok((input, FormattingToggle::On))
    } else if word.eq_ignore_ascii_case("off") {
        Ok((input, FormattingToggle::Off))
    } else {
        warn!("pasfmt directive comment found but '{word}' is neither 'on' nor 'off'.");
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
    fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, _input: &[LogicalLine]) {
        let mut ignored = false;
        for index in 0..formatted_tokens.get_tokens().len() {
            let mut on_toggle_comment = false;
            if let Some((token, _)) = formatted_tokens.get_token(index) {
                if let TokenType::Comment(_) = token.get_token_type() {
                    let parse_result: IResult<_, _> = delimited(
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
            .file_formatter(AddSpaceBeforeEverything {})
            .file_formatter(FormattingToggler {})
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
        }
    );
}
