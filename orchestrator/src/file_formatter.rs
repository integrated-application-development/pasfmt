use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
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
        eprintln!("WARNING: '{}' is not a valid glob", path);
    }

    fn warn_invalid_file(&self, path: &str) {
        eprintln!("WARNING: '{}' is not a valid file path/glob", path);
    }

    fn get_valid_files(&self, paths: Vec<&str>) -> Vec<String> {
        let mut valid_paths = vec![];
        paths.iter().for_each(|path_str| {
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

    fn get_file_contents(&self, file_path: &str) -> Result<String, String> {
        let mut file = File::open(file_path)
            .map_err(|e| format!("ERROR: Failed to open '{}', {}", file_path, e))?;

        let mut file_bytes = Vec::new();
        file.read_to_end(&mut file_bytes)
            .map_err(|e| format!("ERROR: Failed to read '{}', {}", file_path, e))?;

        Ok(self.encoding.decode(&file_bytes[..]).0.into_owned())
    }

    fn exec_format<T>(&self, paths: Vec<&str>, result_operation: T)
    where
        T: Fn(&str, &str, String) + Sync,
    {
        let valid_paths: Vec<_> = self.get_valid_files(paths);

        valid_paths.into_par_iter().for_each(|file_path| {
            let file_contents = match self.get_file_contents(&file_path) {
                Ok(contents) => contents,
                Err(error) => {
                    eprintln!("{}", error);
                    return;
                }
            };

            let formatted_file = self.formatter.format(&file_contents);
            result_operation(&file_path, &file_contents, formatted_file);
        });
    }

    pub fn format_files(&self, paths: Vec<&str>) {
        self.exec_format(paths, |file_path, _, formatted_output| {
            let mut new_file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(file_path)
                .unwrap();

            let encoded_output = self.encoding.encode(&formatted_output).0;
            match new_file.write_all(&encoded_output) {
                Ok(_) => {}
                Err(error) => eprintln!("ERROR: Failed to write to '{}', {}", file_path, error),
            }
        })
    }
    pub fn format_stdin_to_stdout(&self, input: &str) {
        let formatted_input = self.formatter.format(input);
        println!("{}", formatted_input);
    }
    pub fn format_files_to_stdout(&self, paths: Vec<&str>) {
        self.exec_format(paths, |file_path, _, formatted_output| {
            println!("{}:", file_path);
            println!("{}", formatted_output);
        })
    }
    pub fn check_files(&self, paths: Vec<&str>) {
        self.exec_format(paths, |file_path, file_contents, formatted_output| {
            if formatted_output != file_contents {
                eprintln!("VERIFY: '{}' has different formatting", file_path);
            }
        })
    }
}

fn formattable_file_path(path: &str) -> bool {
    path.ends_with(".pas") || path.ends_with(".dpr") || path.ends_with(".dpk")
}
