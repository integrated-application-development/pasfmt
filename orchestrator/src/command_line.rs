use std::{
    env,
    fs::{self, read_to_string},
    path::{Path, PathBuf},
};

use clap::Parser;

const DEFAULT_CONFIG_FILE_NAME: &str = "pasfmt.toml";

#[derive(Parser, Default, Debug)]
#[command(author, version, about, long_about = None)]
pub struct PasFmtConfiguration {
    /// Paths that will be formatted. Can be a path/dir/glob. If no paths are
    /// specified, stdin is read.
    #[arg(index = 1, num_args = 0..)]
    paths: Vec<String>,

    /// A file containing paths to operate on. Newline separated list of
    /// path/dir/glob.
    #[arg(short, long)]
    files_file: Option<String>,

    /// Override the configuration file. By default working directory will be
    /// traversed until a `pasfmt.toml` file is found.
    #[arg(short, long)]
    config_file: Option<String>,

    /// Whether to format the files and update their contents in-place.
    #[arg(short, long, conflicts_with = "verify")]
    write: bool,

    /// Whether to check the correctness of the formatting of the files. It will
    /// list the files that are different.
    #[arg(short, long, conflicts_with = "write")]
    verify: bool,
}
impl PasFmtConfiguration {
    pub fn new() -> Self {
        PasFmtConfiguration::parse()
    }
    pub fn get_paths(&self) -> Vec<String> {
        let mut paths = self.paths.clone();
        if let Some(arg_file) = &self.files_file {
            paths.extend(
                read_to_string(arg_file)
                    .unwrap()
                    .lines()
                    .map(String::from)
                    .collect::<Vec<_>>(),
            );
        }
        paths
    }
    pub fn is_write(&self) -> bool {
        self.write
    }
    pub fn is_verify(&self) -> bool {
        self.verify
    }

    fn get_config_file(&self) -> Option<String> {
        if self.config_file.is_some() {
            return self.config_file.clone();
        }
        let current_dir = env::current_dir();
        if current_dir.is_err() {
            return None;
        }

        let mut path: PathBuf = current_dir.unwrap();
        let file = Path::new(DEFAULT_CONFIG_FILE_NAME);

        loop {
            path.push(file);

            if path.is_file() {
                break path.into_os_string().into_string().ok();
            }

            if !(path.pop() && path.pop()) {
                break None;
            }
        }
    }
    pub fn get_config_object<T>(&self) -> T
    where
        T: for<'de> toml::macros::Deserialize<'de> + Default,
    {
        let config_file = self.get_config_file();
        if config_file.is_none() {
            return T::default();
        }

        let config_file_contents = fs::read_to_string(self.get_config_file().unwrap());
        if config_file_contents.is_err() {
            return T::default();
        }

        toml::from_str::<T>(&config_file_contents.unwrap()).unwrap_or_default()
    }
}
