use std::io::Read;

use crate::{command_line::PasFmtConfiguration, file_formatter::FileFormatter};

pub struct FormattingOrchestrator;
impl FormattingOrchestrator {
    pub fn run(file_formatter: FileFormatter) {
        let config = PasFmtConfiguration::new();
        let files = config.get_paths();
        let file_refs: Vec<_> = files.iter().map(|string| string.as_str()).collect();

        if file_refs.is_empty() {
            let mut input = String::new();
            let mut stdin = std::io::stdin().lock();

            stdin.read_to_string(&mut input).unwrap();
            file_formatter.format_stdin_to_stdout(&input);
        } else if config.is_verify() {
            file_formatter.check_files(file_refs);
        } else if config.is_write() {
            file_formatter.format_files(file_refs);
        } else {
            file_formatter.format_files_to_stdout(file_refs);
        }
    }
}
