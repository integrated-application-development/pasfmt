use assert_fs::{prelude::*, TempDir};
use predicates::prelude::*;
use std::path::Path;
use std::{path::PathBuf, time::SystemTime};

use crate::utils::*;

fn file_timestamps(glob: &str) -> DynResult<Vec<(PathBuf, SystemTime)>> {
    let mut out = vec![];
    for f in glob::glob(glob)? {
        let f = f?;
        let metadata = f.metadata()?;
        if metadata.is_file() {
            out.push((f, metadata.modified()?))
        }
    }

    out.sort();

    Ok(out)
}

fn file_timestamps_in_dir(dir: &Path) -> DynResult<Vec<(PathBuf, SystemTime)>> {
    file_timestamps(&(dir.to_string_lossy() + "/**/*"))
}

fn assert_all_timestamps_changed(
    glob: &str,
    orig_timestamps: &[(PathBuf, SystemTime)],
) -> TestResult {
    for (before, after) in orig_timestamps.iter().zip(file_timestamps(glob)?) {
        assert_eq!(
            before.0, after.0,
            "File paths should be unchanged after formatting"
        );
        assert_ne!(
            before.1, after.1,
            "File {:?} is expected to be included in formatting, but the last modified time didn't change",
            before.0
        );
    }

    Ok(())
}

fn assert_no_timestamps_changed(
    glob: &str,
    orig_timestamps: &[(PathBuf, SystemTime)],
) -> TestResult {
    assert_eq!(orig_timestamps, file_timestamps(glob)?);

    Ok(())
}

#[test]
fn recursive_directory_search() -> TestResult {
    let tmp = TempDir::new()?;

    tmp.copy_from(TESTS_DIR.to_string() + "/data/ext", &["**/*"])?;

    let included_files_timestamps = file_timestamps_in_dir(&tmp.child("included"))?;
    let excluded_files_timestamps = file_timestamps_in_dir(&tmp.child("excluded"))?;

    assert_eq!(
        included_files_timestamps.len(),
        6,
        "Unexpected number of files copied from tests/data/ext/included"
    );
    assert_eq!(
        excluded_files_timestamps.len(),
        6,
        "Unexpected number of files copied from tests/data/ext/excluded"
    );

    fmt(&*tmp)?;

    assert_no_timestamps_changed(
        &(tmp.to_string_lossy() + "/excluded/**/*"),
        &excluded_files_timestamps,
    )?;

    assert_all_timestamps_changed(
        &(tmp.to_string_lossy() + "/included/**/*"),
        &included_files_timestamps,
    )?;

    Ok(())
}

#[test]
fn glob() -> TestResult {
    let tmp = TempDir::new()?;

    tmp.copy_from(TESTS_DIR.to_string() + "/data/ext/included", &["**/*"])?;

    let pas_glob = tmp.to_string_lossy() + "/**/*.pas";
    let pas_timestamps = file_timestamps(&pas_glob)?;
    assert_eq!(pas_timestamps.len(), 1);

    let dpr_glob = tmp.to_string_lossy() + "/**/*.dpr";
    let dpr_timestamps = file_timestamps(&dpr_glob)?;
    assert_eq!(dpr_timestamps.len(), 1);

    pasfmt()?
        .current_dir(&tmp)
        .arg("**/*.pas")
        .assert()
        .success();
    assert_all_timestamps_changed(&pas_glob, &pas_timestamps)?;
    assert_no_timestamps_changed(&dpr_glob, &dpr_timestamps)?;

    let pas_timestamps = file_timestamps(&pas_glob)?;
    pasfmt()?
        .current_dir(&tmp)
        .arg("**/*.dpr")
        .assert()
        .success();
    assert_all_timestamps_changed(&dpr_glob, &dpr_timestamps)?;
    assert_no_timestamps_changed(&pas_glob, &pas_timestamps)?;

    Ok(())
}

#[test]
fn classic_glob_does_not_recurse() -> TestResult {
    let tmp = TempDir::new()?;

    tmp.copy_from(TESTS_DIR.to_string() + "/data/ext/included", &["**/*"])?;

    let pas_glob = tmp.to_string_lossy() + "/**/*.pas";
    let pas_timestamps = file_timestamps(&pas_glob)?;
    assert_eq!(pas_timestamps.len(), 1);

    pasfmt()?.current_dir(&tmp).arg("*.pas").assert().success();
    assert_no_timestamps_changed(&pas_glob, &pas_timestamps)?;

    Ok(())
}

#[test]
fn glob_can_include_any_ext() -> TestResult {
    let tmp = TempDir::new()?;

    tmp.copy_from(TESTS_DIR, &["data/ext/excluded/*"])?;

    let excluded_glob = tmp.to_string_lossy() + "/data/ext/excluded/*";
    let excluded_files_timestamps = file_timestamps(&excluded_glob)?;
    assert_eq!(excluded_files_timestamps.len(), 6);

    pasfmt()?
        .current_dir(&tmp)
        .args(["**/*.*pas*", "**/*.*dpr*", "**/*.*dpk*"])
        .assert()
        .success();
    assert_all_timestamps_changed(&excluded_glob, &excluded_files_timestamps)?;

    Ok(())
}

#[test]
fn invalid_glob() -> TestResult {
    fmt("***")?
        .success()
        .stderr(predicate::str::contains("WARN '***' is not a valid glob"));

    Ok(())
}
