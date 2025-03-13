use crate::prelude::*;

pub struct IgnoreAsmIstructions;
impl TokenIgnorer for IgnoreAsmIstructions {
    fn ignore_tokens(
        &self,
        (_tokens, lines): (&[Token], &[LogicalLine]),
        token_marker: &mut TokenMarker,
    ) {
        lines
            .iter()
            .filter(|line| line.get_line_type() == LogicalLineType::AsmInstruction)
            .for_each(|line| {
                line.get_tokens().iter().for_each(|token| {
                    token_marker.mark(*token);
                })
            });
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::test_utils::formatter_test_group;

    struct OneSpaceBetweenLineTokens {}
    impl LogicalLineFileFormatter for OneSpaceBetweenLineTokens {
        fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, _input: &[LogicalLine]) {
            for token_index in 1..formatted_tokens.get_tokens().len() {
                if let Some(formatting_data) = formatted_tokens.get_formatting_data_mut(token_index)
                {
                    if formatting_data.newlines_before == 0 {
                        formatting_data.spaces_before = 1;
                    }
                }
            }
        }
    }

    fn formatter() -> Formatter {
        Formatter::builder()
            .lexer(DelphiLexer {})
            .parser(DelphiLogicalLineParser {})
            .token_ignorer(IgnoreAsmIstructions {})
            .file_formatter(OneSpaceBetweenLineTokens {})
            .reconstructor(default_test_reconstructor())
            .build()
    }

    formatter_test_group!(
        asm_instructions,
        instruction_on_same_line = {
            indoc::indoc! {"
                begin
                  A+B+C;
                  asm  mov a,b   end;
                  A+B+C;
                end
            "},
            // The `end` is not part of the logical line
            indoc::indoc! {"
                begin
                  A + B + C ;
                  asm  mov a,b end ;
                  A + B + C ;
                end
            "}
        },
        individual_lines = {
            indoc::indoc! {"
                A+B+C;
                asm
                mov A,B
                 mov C , D
                  mov   C ,   D
                end;
                A+B+C;
            "},
            indoc::indoc! {"
                A + B + C ;
                asm
                mov A,B
                 mov C , D
                  mov   C ,   D
                end ;
                A + B + C ;
            "}
        },
    );
}
