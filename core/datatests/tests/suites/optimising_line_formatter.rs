use std::{error::Error, fs::read_to_string, path::Path};

use itertools::Itertools;
use pretty_assertions::assert_str_eq;

use crate::suites::get_input_output;

use super::ErrorString;
use pasfmt_core::prelude::*;

fn run_test(input: &str) -> datatest_stable::Result<()> {
    let reconstruction_settings = ReconstructionSettings::new(LineEnding::Lf, TabKind::Soft, 2, 4);
    let formatter = Formatter::builder()
        .lexer(DelphiLexer {})
        .parser(DelphiLogicalLineParser {})
        .token_consolidator(DistinguishGenericTypeParamsConsolidator {})
        .file_formatter(TokenSpacing {})
        .file_formatter(OptimisingLineFormatter::new(
            OptimisingLineFormatterSettings {
                // The max_line_length is small in tests to make writing tests
                // more simple and more concise.
                max_line_length: 30,
                iteration_max: 20_000,
                break_before_begin: false,
            },
            reconstruction_settings.clone(),
        ))
        .reconstructor(DelphiLogicalLinesReconstructor::new(
            reconstruction_settings,
        ))
        .build();

    let io = get_input_output(input);
    let input = trim_string(io.input)?;
    let expected_output = io
        .output
        .map(trim_string)
        .unwrap_or_else(|| Ok(input.clone()))?;

    let formatted_input = formatter.format(&input, FileOptions::new());
    assert_str_eq!(expected_output, formatted_input);
    Ok(())
}

fn trim_string(input: &str) -> Result<String, Box<dyn Error>> {
    let mut lines = input.lines();
    let first_line = lines.next();
    if first_line != Some("") {
        return Ok(input.to_string());
    }

    let leading_whitespace = input
        .lines()
        .find_or_first(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .unwrap_or(0);
    Ok(lines
        .map(|line| {
            let all_whitespace = line.chars().all(char::is_whitespace);
            match line.split_at_checked(leading_whitespace) {
                None if all_whitespace => Ok(line.trim()),
                None => Err(Box::new(ErrorString(
                    "Line has less leading whitespace than the first".into(),
                ))),
                Some((whitespace, content)) if whitespace.chars().all(char::is_whitespace) => {
                    Ok(content)
                }
                _ => Err(Box::new(ErrorString(
                    "Line has less leading whitespace than the first".into(),
                ))),
            }
        })
        .collect::<Result<Vec<_>, _>>()?
        .join("\n"))
}

pub fn test_file(path: &Path) -> datatest_stable::Result<()> {
    let input = read_to_string(path)?;
    run_test(&input)
}
