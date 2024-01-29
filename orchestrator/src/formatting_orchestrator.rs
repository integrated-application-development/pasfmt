use crate::file_formatter::FileFormatter;
use anyhow::anyhow;
use log::LevelFilter;

pub trait FormatterConfiguration {
    fn is_stdin(&self) -> bool;
    fn get_paths(&self) -> Vec<String>;
    fn log_level(&self) -> LevelFilter;
    fn is_write(&self) -> bool;
    fn is_verify(&self) -> bool;
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

fn verify<T: FormatterConfiguration>(
    config: &T,
    file_formatter: &FileFormatter,
) -> anyhow::Result<()> {
    let check_results = file_formatter.check_files(&config.get_paths());

    let mut fail_count = 0;
    for err in check_results.iter().filter_map(|r| r.as_ref().err()) {
        fail_count += 1;
        println!("{err}");
    }

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
        let files = config.get_paths();
        if files.is_empty() {
            file_formatter.format_stdin_to_stdout()
        } else if config.is_verify() {
            verify(&config, &file_formatter)
        } else if config.is_write() {
            coalesce_err(&file_formatter.format_files(&files))
        } else {
            coalesce_err(&file_formatter.format_files_to_stdout(&files))
        }
    }
}
