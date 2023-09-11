use log::*;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use glob::glob;
use pasfmt_core::formatter::Formatter;
use rayon::prelude::*;
use walkdir::WalkDir;

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

    fn get_file_contents(&self, file: &mut File) -> std::io::Result<String> {
        let mut file_bytes = Vec::new();
        file.read_to_end(&mut file_bytes)?;

        Ok(self.encoding.decode(&file_bytes[..]).0.into_owned())
    }

    fn exec_format<S: AsRef<str>, T>(
        &self,
        paths: &[S],
        mut open_options: OpenOptions,
        result_operation: T,
    ) where
        T: Fn(&mut File, &Path, &str, String) -> Result<(), String> + Sync,
    {
        open_options.read(true);

        let valid_paths: Vec<_> = self.get_valid_files(paths);

        valid_paths.into_par_iter().for_each(|file_path| {
            let mut file = match open_options.open(&file_path) {
                Ok(file) => file,
                Err(e) => panic!("Failed to open '{}', {}", &file_path.to_string_lossy(), e),
            };

            let file_contents = match self
                .get_file_contents(&mut file)
                .map_err(|e| format!("Failed to read '{}', {}", file_path.to_string_lossy(), e))
            {
                Ok(contents) => contents,
                Err(error) => {
                    panic!("{}", error);
                }
            };

            let formatted_file = self.formatter.format(&file_contents);
            if let Err(e) = result_operation(&mut file, &file_path, &file_contents, formatted_file)
            {
                panic!("{e}");
            }
        });
    }

    pub fn format_files<S: AsRef<str>>(&self, paths: &[S]) {
        self.exec_format(
            paths,
            OpenOptions::new().write(true).to_owned(),
            |file, file_path, _, formatted_output| {
                let path_str = || file_path.to_string_lossy();
                let encoded_output = self.encoding.encode(&formatted_output).0;
                file.seek(SeekFrom::Start(0)).map_err(|e| {
                    format!("Failed to seek to start of file: {}, {}", path_str(), e)
                })?;
                file.write_all(&encoded_output)
                    .map_err(|e| format!("Failed to write to '{}', {}", path_str(), e))?;
                file.set_len(encoded_output.len() as u64)
                    .map_err(|e| format!("Failed to set file length: {}, {}", path_str(), e))?;
                Ok(())
            },
        )
    }
    pub fn format_stdin_to_stdout(&self, input: &str) {
        let formatted_input = self.formatter.format(input);
        print!("{}", formatted_input);
    }
    pub fn format_files_to_stdout<S: AsRef<str>>(&self, paths: &[S]) {
        self.exec_format(
            paths,
            OpenOptions::new(),
            |_, file_path, _, formatted_output| {
                // Append a newline to the formatted output deliberately because makes it more
                // human-readable and no less machine-readable.
                println!("{}:\n{}", file_path.to_string_lossy(), formatted_output);
                Ok(())
            },
        )
    }
    pub fn check_files<S: AsRef<str>>(&self, paths: &[S]) {
        self.exec_format(
            paths,
            OpenOptions::new(),
            |_, file_path, file_contents, formatted_output| {
                if formatted_output != file_contents {
                    println!(
                        "VERIFY: '{}' has different formatting",
                        file_path.to_string_lossy()
                    );
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
