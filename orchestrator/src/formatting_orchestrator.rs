use std::io::Read;

use crate::{command_line::PasFmtConfiguration, file_formatter::FileFormatter};

pub struct FormattingOrchestrator;
impl FormattingOrchestrator {
    pub fn run(file_formatter: FileFormatter, config: PasFmtConfiguration) {
        let files = config.get_paths();
        if files.is_empty() {
            let mut input = String::new();
            let mut stdin = std::io::stdin().lock();

            stdin.read_to_string(&mut input).unwrap();
            file_formatter.format_stdin_to_stdout(&input);
        } else if config.is_verify() {
            file_formatter.check_files(&files);
        } else if config.is_write() {
            file_formatter.format_files(&files);
        } else {
            file_formatter.format_files_to_stdout(&files);
        }
    }
}
