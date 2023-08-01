use log::*;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
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

    fn get_valid_files<S: AsRef<str>>(&self, paths: &[S]) -> Vec<String> {
        let mut valid_paths = vec![];
        paths.iter().map(AsRef::as_ref).for_each(|path_str| {
            let path = Path::new(path_str);
            if path.is_dir() {
                valid_paths.extend(WalkDir::new(path_str).into_iter().filter_map(|entry| {
                    let file = match entry {
                        Ok(file) => file,
                        Err(_) => return None,
                    };
                    let file_path = file.path().to_str().unwrap().to_string();
                    match formattable_file_path(&file_path) {
                        true => Some(file_path),
                        false => None,
                    }
                }));
            } else if path.is_file() {
                valid_paths.push(path_str.to_string());
            } else {
                match glob(path_str) {
                    Err(_) => self.warn_invalid_glob(path_str),
                    Ok(glob) => glob.for_each(|entry| match entry {
                        Err(_) => self.warn_invalid_file(path_str),
                        Ok(path) => valid_paths.push(path.into_os_string().into_string().unwrap()),
                    }),
                }
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
        T: Fn(&mut File, &str, &str, String) -> Result<(), String> + Sync,
    {
        open_options.read(true);

        let valid_paths: Vec<_> = self.get_valid_files(paths);

        valid_paths.into_par_iter().for_each(|file_path| {
            let mut file = match open_options.open(&file_path) {
                Ok(file) => file,
                Err(e) => {
                    error!("Failed to open '{}', {}", &file_path, e);
                    return;
                }
            };

            let file_contents = match self
                .get_file_contents(&mut file)
                .map_err(|e| format!("Failed to read '{}', {}", file_path, e))
            {
                Ok(contents) => contents,
                Err(error) => {
                    error!("{}", error);
                    return;
                }
            };

            let formatted_file = self.formatter.format(&file_contents);
            if let Err(e) = result_operation(&mut file, &file_path, &file_contents, formatted_file)
            {
                error!("{e}");
            }
        });
    }

    pub fn format_files<S: AsRef<str>>(&self, paths: &[S]) {
        self.exec_format(
            paths,
            OpenOptions::new().write(true).to_owned(),
            |file, file_path, _, formatted_output| {
                let encoded_output = self.encoding.encode(&formatted_output).0;
                file.set_len(0)
                    .map_err(|e| format!("Failed to truncate file: {file_path}, {e}"))?;
                file.seek(SeekFrom::End(0))
                    .map_err(|e| format!("Failed to seek to start of file: {file_path}, {e}"))?;
                file.write_all(&encoded_output)
                    .map_err(|e| format!("Failed to write to '{file_path}', {e}"))?;
                Ok(())
            },
        )
    }
    pub fn format_stdin_to_stdout(&self, input: &str) {
        let formatted_input = self.formatter.format(input);
        println!("{}", formatted_input);
    }
    pub fn format_files_to_stdout<S: AsRef<str>>(&self, paths: &[S]) {
        self.exec_format(
            paths,
            OpenOptions::new(),
            |_, file_path, _, formatted_output| {
                println!("{}:\n{}", file_path, formatted_output);
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
                    println!("VERIFY: '{}' has different formatting", file_path);
                }
                Ok(())
            },
        )
    }
}

fn formattable_file_path(path: &str) -> bool {
    path.ends_with(".pas") || path.ends_with(".dpr") || path.ends_with(".dpk")
}
