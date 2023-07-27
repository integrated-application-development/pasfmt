use crate::lang::*;
use crate::traits::*;

#[allow(dead_code)]
pub struct FormatterSelector<'a, T>
where
    T: Fn(LogicalLineType) -> Option<&'a dyn LogicalLineFormatter>,
{
    selector: T,
}
#[allow(dead_code)]
impl<'a, T> FormatterSelector<'a, T>
where
    T: Fn(LogicalLineType) -> Option<&'a dyn LogicalLineFormatter>,
{
    pub fn new(selector: T) -> Self {
        FormatterSelector { selector }
    }
}
#[allow(dead_code)]
impl<'a, T> LogicalLineFormatter for FormatterSelector<'a, T>
where
    T: Fn(LogicalLineType) -> Option<&'a dyn LogicalLineFormatter>,
{
    fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &LogicalLine) {
        if let Some(formatter) = (self.selector)(input.get_line_type()) {
            formatter.format(formatted_tokens, input)
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        defaults::{
            lexer::DelphiLexer, parser::DelphiLogicalLineParser,
            reconstructor::DelphiLogicalLinesReconstructor,
        },
        formatter::*,
    };

    use super::*;
    use spectral::prelude::*;

    fn run_test(formatter: Formatter, input: &str, expected_output: &str) {
        let output = formatter.format(input);
        assert_that(&output).is_equal_to(expected_output.to_string());
    }

    struct Add1Indentation;
    impl LogicalLineFormatter for Add1Indentation {
        fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &LogicalLine) {
            let first_token = *input.get_tokens().first().unwrap();
            if let Some(formatting_data) = formatted_tokens.get_formatting_data_mut(first_token) {
                formatting_data.indentations_before += 1;
            }
        }
    }

    struct Add1Continuation;
    impl LogicalLineFormatter for Add1Continuation {
        fn format(&self, formatted_tokens: &mut FormattedTokens<'_>, input: &LogicalLine) {
            let first_token = *input.get_tokens().first().unwrap();
            if let Some(formatting_data) = formatted_tokens.get_formatting_data_mut(first_token) {
                formatting_data.continuations_before += 1;
            }
        }
    }

    #[test]
    fn optional_formatter_selector() {
        let add_1_indentation = &Add1Indentation {};
        let add_1_continuation = &Add1Continuation {};
        let formatter = Formatter::builder()
            .lexer(DelphiLexer {})
            .parser(DelphiLogicalLineParser {})
            .line_formatter(FormatterSelector {
                selector: |line_type| match line_type {
                    LogicalLineType::Unknown => Some(add_1_indentation),
                    LogicalLineType::Eof => Some(add_1_continuation),
                    _ => None,
                },
            })
            .reconstructor(DelphiLogicalLinesReconstructor::new(
                ReconstructionSettings::new("\n".to_owned(), " i".to_owned(), " c".to_owned()),
            ))
            .build();
        run_test(formatter, "a;", " ia; c");
    }
}
