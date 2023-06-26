use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

use glob::glob;
use pasfmt_core::formatter::Formatter;

pub struct FileFormatter {
    formatter: Formatter,
}
impl FileFormatter {
    pub fn new(formatter: Formatter) -> Self {
        FileFormatter { formatter }
    }

    fn get_valid_files(&self, paths: Vec<&str>) -> Vec<String> {
        let (valid_paths, invalid_paths): (Vec<Option<String>>, Vec<Option<String>>) = paths
            .iter()
            .flat_map(|path_str| {
                let path = Path::new(path_str);

                if path.is_dir() {
                    return visit_dirs(path)
                        .iter()
                        .map(|file| (Some(file.clone()), None))
                        .collect::<Vec<_>>();
                }

                if path.is_file() {
                    return vec![(Some(path_str.to_string()), None)];
                }

                match glob(path_str) {
                    Err(_) => vec![(None, Some(path_str.to_string()))],
                    Ok(glob) => glob
                        .map(|entry| match entry {
                            Ok(path) => (Some(path.into_os_string().into_string().unwrap()), None),
                            Err(_) => (None, Some(path_str.to_string())),
                        })
                        .collect(),
                }
            })
            .unzip();

        invalid_paths
            .into_iter()
            .flatten()
            .for_each(|invalid_path| {
                eprintln!("WARNING: '{}' is not a valid file path/glob", invalid_path);
            });

        valid_paths.into_iter().flatten().collect()
    }
    fn get_file_contents(&self, file_path: &str) -> Result<String, String> {
        let mut file_contents = String::new();

        let file = File::open(file_path);
        if file.is_err() {
            return Err(format!(
                "ERROR: Failed to open '{}', {}",
                file_path,
                file.unwrap_err()
            ));
        }

        let mut file = file.unwrap();
        let file_read_result = file.read_to_string(&mut file_contents);
        if file_read_result.is_err() {
            return Err(format!(
                "ERROR: Failed to read '{}', {}",
                file_path,
                file_read_result.unwrap_err()
            ));
        }

        Ok(file_contents)
    }

    pub fn format_files(&self, paths: Vec<&str>) {
        let valid_paths: Vec<_> = self.get_valid_files(paths);

        valid_paths.into_iter().for_each(|file_path| {
            let file_contents = self.get_file_contents(&file_path);
            if file_contents.is_err() {
                eprintln!("{}", file_contents.unwrap_err());
                return;
            }

            let file_contents = file_contents.unwrap();
            let formatted_file = self.formatter.format(&file_contents);

            let mut new_file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&file_path)
                .unwrap();
            let file_write_result = new_file.write_all(formatted_file.as_bytes());
            if file_write_result.is_err() {
                eprintln!(
                    "ERROR: Failed to write to '{}', {}",
                    file_path,
                    file_write_result.unwrap_err()
                );
            }
        });
    }
    pub fn format_stdin_to_stdout(&self, input: &str) {
        let formatted_input = self.formatter.format(input);
        println!("{}", formatted_input);
    }
    pub fn format_files_to_stdout(&self, paths: Vec<&str>) {
        let valid_paths: Vec<_> = self.get_valid_files(paths);

        valid_paths.into_iter().for_each(|file_path| {
            let file_contents = self.get_file_contents(&file_path);
            if file_contents.is_err() {
                eprintln!("{}", file_contents.unwrap_err());
                return;
            }

            let file_contents = file_contents.unwrap();
            let formatted_file = self.formatter.format(&file_contents);
            println!("{}", formatted_file);
        });
    }
    pub fn check_files(&self, paths: Vec<&str>) {
        let valid_paths: Vec<_> = self.get_valid_files(paths);

        valid_paths.into_iter().for_each(|file_path| {
            let file_contents = self.get_file_contents(&file_path);
            if file_contents.is_err() {
                eprintln!("{}", file_contents.unwrap_err());
                return;
            }

            let file_contents = file_contents.unwrap();
            let formatted_file = self.formatter.format(&file_contents);

            if formatted_file != file_contents {
                eprintln!("VERIFY: {:?} has different formatting", file_path);
            }
        });
    }
}

fn formattable_file_path(path: &str) -> bool {
    path.ends_with(".pas") || path.ends_with(".dpr") || path.ends_with(".dpk")
}

fn visit_dirs_rec(directory: &Path, paths: &mut Vec<String>) {
    let paths = paths;
    if directory.is_dir() {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                visit_dirs_rec(&path, paths);
            } else if formattable_file_path(path.to_str().unwrap()) {
                paths.push(path.as_os_str().to_str().unwrap().to_owned());
            }
        }
    }
}
fn visit_dirs(directory: &Path) -> Vec<String> {
    let mut paths = Vec::new();
    visit_dirs_rec(directory, &mut paths);
    paths
}
