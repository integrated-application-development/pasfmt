use std::env;
use std::fs::File;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};

mod generators;
use generators::logical_line_parser;
use generators::optimising_line_formatter;
use walkdir::DirEntry;

fn main() {
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    let staging_test_root_dir = Path::new(&crate_dir).join("generated-staging");
    let test_root_dir = Path::new(&crate_dir).join("generated");
    remove_dir_if_present(&staging_test_root_dir);

    logical_line_parser::generate_test_files(&staging_test_root_dir.join("logical_line_test"));
    optimising_line_formatter::generate_test_files(
        &staging_test_root_dir.join("optimising_line_formatter"),
    );
    if !test_root_dir.is_dir() {
        std::fs::create_dir(&test_root_dir).unwrap_or_else(|e| {
            panic!(
                "failed to create directory '{}', {e}",
                &test_root_dir.display()
            )
        })
    }

    println!("cargo:rerun-if-changed=generated/");
    println!("cargo:rerun-if-changed=generators/");
    println!("cargo:rerun-if-changed=tests/");

    let staged_files = get_dir_entries(&staging_test_root_dir);
    let existing_files = get_dir_entries(&test_root_dir);

    if replace_existing_tests(staged_files, existing_files) {
        remove_dir_if_present(&test_root_dir);
        std::fs::rename(&staging_test_root_dir, &test_root_dir).unwrap_or_else(|e| {
            panic!(
                "unable to rename '{}' to '{}', {e}",
                &staging_test_root_dir.display(),
                &test_root_dir.display()
            )
        });
    } else {
        remove_dir_if_present(&staging_test_root_dir);
    }
}

fn remove_dir_if_present(path: &PathBuf) {
    match std::fs::remove_dir_all(path) {
        Ok(()) => {}
        Err(e) if matches!(e.kind(), ErrorKind::NotFound) => {}
        Err(e) => panic!("unable to remove {}, {}", &path.display(), e),
    };
}

fn replace_existing_tests(staged_files: Vec<DirEntry>, existing_files: Vec<DirEntry>) -> bool {
    if staged_files.len() != existing_files.len() {
        return true;
    }

    staged_files
        .iter()
        .zip(existing_files)
        .filter(|(staged, existing)| staged.file_type().is_file() && existing.file_type().is_file())
        .any(|(staged, existing)| read_file_bytes(&existing) != read_file_bytes(staged))
}

fn get_dir_entries(root: &PathBuf) -> Vec<DirEntry> {
    walkdir::WalkDir::new(root)
        .sort_by_file_name()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

fn read_file_bytes(file: &DirEntry) -> Vec<u8> {
    let mut contents = vec![];
    File::open(file.clone().into_path())
        .unwrap_or_else(|e| panic!("failed to open '{}', {e}", &file.path().display()))
        .read_to_end(&mut contents)
        .unwrap_or_else(|e| panic!("failed to read '{}', {e}", &file.path().display()));
    contents
}
