use encoding_rs::Encoding;
use log::*;
use std::{
    borrow::Cow,
    fs::{File, OpenOptions},
    io::{IsTerminal, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use glob::glob;
use pasfmt_core::formatter::Formatter;
use rayon::prelude::*;
use walkdir::WalkDir;

struct DecodedFile<'a> {
    contents: Cow<'a, str>,
    encoding: &'static Encoding,
    replacements: bool,
}

pub struct FileFormatter {
    formatter: Formatter,
    encoding: &'static encoding_rs::Encoding,
}
impl FileFormatter {
    pub fn new(formatter: Formatter, encoding: &'static encoding_rs::Encoding) -> Self {
        FileFormatter {
            formatter,
            encoding,
        }
    }

    fn warn_invalid_glob(&self, path: &str) {
        warn!("'{}' is not a valid glob", path);
    }

    fn warn_invalid_file(&self, path: &str) {
        warn!("'{}' is not a valid file path/glob", path);
    }

    fn get_valid_files<S: AsRef<str>>(&self, paths: &[S]) -> Vec<PathBuf> {
        let mut valid_paths = vec![];
        paths.iter().map(AsRef::as_ref).for_each(|path_str| {
            let path = Path::new(path_str);
            match path.metadata() {
                Ok(metadata) if metadata.is_dir() => {
                    valid_paths.extend(WalkDir::new(path_str).into_iter().filter_map(|entry| {
                        let entry = entry.ok()?;
                        let file_path = entry.path();
                        match formattable_file_path(file_path) {
                            true => Some(file_path.to_path_buf()),
                            false => None,
                        }
                    }));
                }
                Ok(metadata) if metadata.is_file() => {
                    valid_paths.push(path.to_path_buf());
                }
                _ => match glob(path_str) {
                    Err(_) => self.warn_invalid_glob(path_str),
                    Ok(glob) => glob.for_each(|entry| match entry {
                        Err(_) => self.warn_invalid_file(path_str),
                        Ok(path) => valid_paths.push(path),
                    }),
                },
            }
        });

        valid_paths
    }

    fn decode_file<'a>(
        &self,
        file: &mut impl Read,
        buf: &'a mut Vec<u8>,
    ) -> std::io::Result<DecodedFile<'a>> {
        file.read_to_end(buf)?;

        let (contents, encoding, replacements) = self.encoding.decode(buf);

        Ok(DecodedFile {
            contents,
            encoding,
            replacements,
        })
    }

    fn exec_format<S: AsRef<str>, T>(
        &self,
        paths: &[S],
        mut open_options: OpenOptions,
        result_operation: T,
    ) where
        T: Fn(&mut File, &Path, &DecodedFile, String) -> Result<(), String> + Sync,
    {
        open_options.read(true);

        let valid_paths: Vec<_> = self.get_valid_files(paths);

        valid_paths.into_par_iter().for_each(|file_path| {
            let mut file = match open_options.open(&file_path) {
                Ok(file) => file,
                Err(e) => panic!("Failed to open '{}', {}", file_path.display(), e),
            };

            let mut buf = vec![];
            let decoded_file = match self
                .decode_file(&mut file, &mut buf)
                .map_err(|e| format!("Failed to read '{}', {}", file_path.display(), e))
            {
                Ok(contents) => contents,
                Err(error) => {
                    panic!("{}", error);
                }
            };

            if decoded_file.replacements {
                panic!(
                    "File '{}' had malformed sequences (in encoding '{}')",
                    file_path.display(),
                    decoded_file.encoding.name()
                );
            }

            let formatted_file = self.formatter.format(&decoded_file.contents);
            if let Err(e) = result_operation(&mut file, &file_path, &decoded_file, formatted_file) {
                panic!("{e}");
            }
        });
    }

    pub fn format_files<S: AsRef<str>>(&self, paths: &[S]) {
        self.exec_format(
            paths,
            OpenOptions::new().write(true).to_owned(),
            |file, file_path, decoded_file, formatted_output| {
                if decoded_file.contents.eq(&formatted_output) {
                    debug!(
                        "Skipping writing to '{}' because it is already formatted.",
                        file_path.display()
                    );
                    return Ok(());
                }
                let encoded_output = decoded_file.encoding.encode(&formatted_output).0;
                file.seek(SeekFrom::Start(0)).map_err(|e| {
                    format!(
                        "Failed to seek to start of file: '{}', {}",
                        file_path.display(),
                        e
                    )
                })?;
                file.write_all(&encoded_output)
                    .map_err(|e| format!("Failed to write to '{}', {}", file_path.display(), e))?;
                file.set_len(encoded_output.len() as u64).map_err(|e| {
                    format!(
                        "Failed to set file length: '{}', {}",
                        file_path.display(),
                        e
                    )
                })?;
                Ok(())
            },
        )
    }
    fn decode_stdin<'a>(&self, buf: &'a mut Vec<u8>) -> std::io::Result<DecodedFile<'a>> {
        if std::io::stdin().is_terminal() {
            eprintln!("waiting for stdin...");
        }
        let mut stdin = std::io::stdin().lock();

        self.decode_file(&mut stdin, buf)
    }
    pub fn format_stdin_to_stdout(&self) {
        let mut buf = vec![];
        let decoded_stdin = self
            .decode_stdin(&mut buf)
            .expect("Failed to read from stdin");
        let formatted_input = self.formatter.format(&decoded_stdin.contents);
        std::io::stdout()
            .write_all(&decoded_stdin.encoding.encode(&formatted_input).0)
            .expect("Failed to write to stdout");
    }
    pub fn format_files_to_stdout<S: AsRef<str>>(&self, paths: &[S]) {
        self.exec_format(
            paths,
            OpenOptions::new(),
            |_, file_path, _, formatted_output| {
                // Append a newline to the formatted output deliberately because makes it more
                // human-readable and no less machine-readable.
                println!("{}:\n{}", file_path.display(), formatted_output);
                Ok(())
            },
        )
    }
    pub fn check_files<S: AsRef<str>>(&self, paths: &[S]) {
        self.exec_format(
            paths,
            OpenOptions::new(),
            |_, file_path, decoded_file, formatted_output| {
                if formatted_output != decoded_file.contents {
                    println!("VERIFY: '{}' has different formatting", file_path.display());
                }
                Ok(())
            },
        )
    }
}

fn formattable_file_path(path: &Path) -> bool {
    match path.extension() {
        Some(ext) => {
            ext.eq_ignore_ascii_case("pas")
                || ext.eq_ignore_ascii_case("dpr")
                || ext.eq_ignore_ascii_case("dpk")
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use yare::parameterized;

    use crate::file_formatter::formattable_file_path;
    use std::path::PathBuf;

    #[parameterized(
        pas_lower = {"a.pas"},
        pas_upper = {"a.PAS"},
        pas_mixed = {"a.Pas"},
        dpr_lower = {"b.dpr"},
        dpr_upper = {"b.DPR"},
        dpr_mixed = {"b.Dpr"},
        dpk_lower = {"c.dpk"},
        dpk_upper = {"c.DPK"},
        dpk_mixed = {"c.Dpk"},
    )]
    fn formattable_file_paths(path: &str) {
        assert!(formattable_file_path(&PathBuf::from(path)));
    }

    #[parameterized(
        no_dot_pas = {"pas"},
        starts_pas = {"a.pas1"},
        ends_pas = {"a.unpas"},
        contains_dot_pas = {"a.pas.x"},
        no_dot_dpr = {"dpr"},
        starts_dpr = {"a.dpr1"},
        ends_dpr = {"a.undpr"},
        contains_dot_dpr = {"a.dpr.x"},
        no_dot_dpk = {"dpk"},
        starts_dpk = {"a.dpk1"},
        ends_dpk = {"a.undpk"},
        contains_dot_dpk = {"a.dpk.x"},
        only_dot_pas = {".pas"},
        only_dot_dpr = {".dpr"},
        only_dot_dpk = {".dpk"},
    )]
    fn non_formattable_file_paths(path: &str) {
        assert!(!formattable_file_path(&PathBuf::from(path)));
    }
}
