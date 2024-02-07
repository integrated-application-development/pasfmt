use crate::{command_line::FormatMode, file_formatter::FileFormatter};
use anyhow::anyhow;
use log::LevelFilter;

pub trait FormatterConfiguration {
    fn is_stdin(&self) -> bool;
    fn get_paths(&self) -> Vec<String>;
    fn log_level(&self) -> LevelFilter;
    fn mode(&self) -> FormatMode;
}

fn plural<'a>(val: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if val == 1 {
        singular
    } else {
        plural
    }
}

fn coalesce_err(results: &[anyhow::Result<()>]) -> anyhow::Result<()> {
    match results.iter().filter(|r| r.is_err()).count() {
        0 => Ok(()),
        c => Err(anyhow!("{c} {} with errors", plural(c, "file", "files"))),
    }
}

fn check<T: FormatterConfiguration>(
    config: &T,
    file_formatter: &FileFormatter,
) -> anyhow::Result<()> {
    let check_results = if config.is_stdin() {
        vec![file_formatter.check_stdin()]
    } else {
        file_formatter.check_files(&config.get_paths())
    };

    let fail_count = check_results.iter().filter(|r| r.is_err()).count();
    if fail_count > 0 {
        Err(anyhow!(
            "{fail_count} {} incorrectly formatted",
            plural(fail_count, "file is", "files are")
        ))
    } else {
        Ok(())
    }
}

pub struct FormattingOrchestrator;
impl FormattingOrchestrator {
    pub fn run<T: FormatterConfiguration>(
        file_formatter: FileFormatter,
        config: T,
    ) -> anyhow::Result<()> {
        match config.mode() {
            FormatMode::Check => check(&config, &file_formatter),
            FormatMode::Files => coalesce_err(&file_formatter.format_files(&config.get_paths())),
            FormatMode::Stdout => {
                if config.is_stdin() {
                    file_formatter.format_stdin_to_stdout()
                } else {
                    coalesce_err(&file_formatter.format_files_to_stdout(&config.get_paths()))
                }
            }
        }
    }
}
