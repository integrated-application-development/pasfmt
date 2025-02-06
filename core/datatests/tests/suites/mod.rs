use core::fmt;
use std::error::Error;

pub mod logical_line_parser;
pub mod optimising_line_formatter;

struct ErrorString(String);
impl fmt::Debug for ErrorString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl fmt::Display for ErrorString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl Error for ErrorString {}

pub const FILE_SEPARATOR: &str = "!#################################!";

struct InputOutput<'i> {
    input: &'i str,
    output: Option<&'i str>,
}
fn get_input_output(input: &str) -> InputOutput {
    match input.split_once(FILE_SEPARATOR) {
        Some((input, output)) => InputOutput {
            input,
            output: Some(output),
        },
        None => InputOutput {
            input,
            output: None,
        },
    }
}
