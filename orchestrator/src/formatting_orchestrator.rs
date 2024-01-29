use crate::file_formatter::FileFormatter;
use log::LevelFilter;

pub trait FormatterConfiguration {
    fn get_paths(&self) -> Vec<String>;
    fn log_level(&self) -> LevelFilter;
    fn is_write(&self) -> bool;
    fn is_verify(&self) -> bool;
}

pub struct FormattingOrchestrator;
impl FormattingOrchestrator {
    pub fn run<T: FormatterConfiguration>(file_formatter: FileFormatter, config: T) {
        let files = config.get_paths();
        if files.is_empty() {
            file_formatter.format_stdin_to_stdout();
        } else if config.is_verify() {
            file_formatter.check_files(&files);
        } else if config.is_write() {
            file_formatter.format_files(&files);
        } else {
            file_formatter.format_files_to_stdout(&files);
        }
    }
}
