use std::borrow::Cow;

use crate::{command_line::FormatMode, file_formatter::FileFormatter, predule::ErrHandler};
use log::LevelFilter;

pub trait FormatterConfiguration {
    fn is_stdin(&self) -> bool;
    fn get_paths(&self) -> anyhow::Result<Cow<[String]>>;
    fn log_level(&self) -> LevelFilter;
    fn mode(&self) -> FormatMode;
    fn cursors(&self) -> &[u32];
}

pub struct FormattingOrchestrator;
impl FormattingOrchestrator {
    pub fn run(
        file_formatter: FileFormatter,
        config: impl FormatterConfiguration,
        err_handler: impl ErrHandler,
    ) {
        match config.mode() {
            FormatMode::Check if config.is_stdin() => file_formatter.check_stdin(err_handler),
            FormatMode::Stdout if config.is_stdin() => {
                file_formatter.format_stdin_to_stdout(err_handler, config.cursors())
            }
            mode => match config.get_paths() {
                Ok(paths) => match mode {
                    FormatMode::Check => file_formatter.check_files(&paths, err_handler),
                    FormatMode::Files => {
                        file_formatter.format_files(&paths, err_handler, config.cursors())
                    }
                    FormatMode::Stdout => {
                        file_formatter.format_files_to_stdout(&paths, err_handler, config.cursors())
                    }
                },
                Err(e) => err_handler(e),
            },
        };
    }
}
