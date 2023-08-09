use std::{fs::read_to_string, path::Path};

use itertools::Itertools;
use pretty_assertions::assert_str_eq;

use super::ErrorString;
use pasfmt_core::prelude::*;

fn run_test(input: &str) -> datatest_stable::Result<()> {
    let reconstruction_settings =
        ReconstructionSettings::new("\n".to_owned(), "  ".to_owned(), "    ".to_owned()).unwrap();
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
            },
            reconstruction_settings.clone(),
        ))
        .reconstructor(DelphiLogicalLinesReconstructor::new(
            reconstruction_settings,
        ))
        .build();

    let leading_whitespace = input
        .lines()
        .find_or_first(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .unwrap_or(0);
    let trimmed_input = input
        .lines()
        .enumerate()
        .filter(|&(index, line)| !(index == 0 && line.trim().is_empty()))
        .map(|(_, line)| {
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
        .join("\n");
    let formatted_input = formatter.format(&trimmed_input);

    assert_str_eq!(trimmed_input, formatted_input);
    Ok(())
}

pub fn test_file(path: &Path) -> datatest_stable::Result<()> {
    let input = read_to_string(path)?;
    run_test(&input)
}
