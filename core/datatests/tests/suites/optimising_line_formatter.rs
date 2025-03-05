use std::{error::Error, fs::read_to_string, path::Path, sync::LazyLock};

use itertools::Itertools;
use pretty_assertions::assert_str_eq;

use crate::suites::get_input_output;

use super::ErrorString;
use pasfmt_core::prelude::*;

struct ErrorLogger;
impl log::Log for ErrorLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if matches!(record.level(), log::Level::Error) {
            panic!("{:?} logged an error:\n{}", record.file(), record.args());
        }
    }

    fn flush(&self) {}
}
static LOGGER: ErrorLogger = ErrorLogger {};
static SET_LOGGER: LazyLock<()> = LazyLock::new(|| {
    log::set_logger(&LOGGER).expect("logger should not already be configured");
    log::set_max_level(log::LevelFilter::Trace);
});

fn get_specified_line_length(input: &str) -> Option<u32> {
    const WRAP_COLUMN_PAT: &str = "// wrap_column=";
    let index = WRAP_COLUMN_PAT.len() + input.find(WRAP_COLUMN_PAT)?;
    let ending_index = index
        + input[index..]
            .find(|c: char| c.is_ascii_whitespace())
            .unwrap_or(input.len());
    Some(
        input[index..ending_index]
            .parse()
            .expect("`wrap_column=` should precede a valid integer"),
    )
}

fn run_test(input: &str) -> datatest_stable::Result<()> {
    *SET_LOGGER;

    // The default max_line_length is small in tests to make writing tests
    // more simple and more concise.
    const DEFAULT_MAX_LINE_LENGTH: u32 = 30;
    let max_line_length = get_specified_line_length(input).unwrap_or(DEFAULT_MAX_LINE_LENGTH);

    let reconstruction_settings = ReconstructionSettings::new(LineEnding::Lf, TabKind::Soft, 2, 4);
    let formatter = Formatter::builder()
        .lexer(DelphiLexer {})
        .parser(DelphiLogicalLineParser {})
        .token_consolidator(DistinguishGenericTypeParamsConsolidator {})
        .lines_consolidator(ConditionalDirectiveConsolidator {})
        .file_formatter(TokenSpacing {})
        .file_formatter(OptimisingLineFormatter::new(
            OptimisingLineFormatterSettings {
                max_line_length,
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
