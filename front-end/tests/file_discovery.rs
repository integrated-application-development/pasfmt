use assert_fs::{prelude::*, TempDir};
use predicates::prelude::*;
use std::path::Path;
use std::path::PathBuf;

use crate::utils::*;

fn file_contents(glob: &str) -> DynResult<Vec<(PathBuf, String)>> {
    let mut out = vec![];
    for f in glob::glob(glob)? {
        let f = f?;
        let metadata = f.metadata()?;
        if metadata.is_file() {
            let contents = std::fs::read_to_string(&f)?;
            out.push((f, contents))
        }
    }

    out.sort();

    Ok(out)
}

fn file_contents_in_dir(dir: &Path) -> DynResult<Vec<(PathBuf, String)>> {
    file_contents(&(dir.to_string_lossy() + "/**/*"))
}

fn assert_all_contents_changed(glob: &str, orig_contents: &[(PathBuf, String)]) -> TestResult {
    for (before, after) in orig_contents.iter().zip(file_contents(glob)?) {
        assert_eq!(
            before.0, after.0,
            "File paths should be unchanged after formatting"
        );
        assert_ne!(
            before.1, after.1,
            "File {:?} is expected to be included in formatting, but its contents didn't change",
            before.0
        );
    }

    Ok(())
}

fn assert_no_contents_changed(glob: &str, orig_contents: &[(PathBuf, String)]) -> TestResult {
    assert_eq!(orig_contents, file_contents(glob)?);

    Ok(())
}

#[test]
fn recursive_directory_search() -> TestResult {
    let tmp = TempDir::new()?;

    tmp.copy_from(TESTS_DIR.to_string() + "/data/ext", &["**/*"])?;

    let included_files_contents = file_contents_in_dir(&tmp.child("included"))?;
    let excluded_files_contents = file_contents_in_dir(&tmp.child("excluded"))?;

    assert_eq!(
        included_files_contents.len(),
        6,
        "Unexpected number of files copied from tests/data/ext/included"
    );
    assert_eq!(
        excluded_files_contents.len(),
        6,
        "Unexpected number of files copied from tests/data/ext/excluded"
    );

    fmt(&*tmp)?;

    assert_no_contents_changed(
        &(tmp.to_string_lossy() + "/excluded/**/*"),
        &excluded_files_contents,
    )?;

    assert_all_contents_changed(
        &(tmp.to_string_lossy() + "/included/**/*"),
        &included_files_contents,
    )?;

    Ok(())
}

#[test]
fn glob() -> TestResult {
    let tmp = TempDir::new()?;

    tmp.copy_from(TESTS_DIR.to_string() + "/data/ext/included", &["**/*"])?;

    let pas_glob = tmp.to_string_lossy() + "/**/*.pas";
    let pas_contents = file_contents(&pas_glob)?;
    assert_eq!(pas_contents.len(), 1);

    let dpr_glob = tmp.to_string_lossy() + "/**/*.dpr";
    let dpr_contents = file_contents(&dpr_glob)?;
    assert_eq!(dpr_contents.len(), 1);

    pasfmt()?
        .current_dir(&tmp)
        .arg("**/*.pas")
        .assert()
        .success();
    assert_all_contents_changed(&pas_glob, &pas_contents)?;
    assert_no_contents_changed(&dpr_glob, &dpr_contents)?;

    let pas_contents = file_contents(&pas_glob)?;
    pasfmt()?
        .current_dir(&tmp)
        .arg("**/*.dpr")
        .assert()
        .success();
    assert_all_contents_changed(&dpr_glob, &dpr_contents)?;
    assert_no_contents_changed(&pas_glob, &pas_contents)?;

    Ok(())
}

#[test]
fn classic_glob_does_not_recurse() -> TestResult {
    let tmp = TempDir::new()?;

    tmp.copy_from(TESTS_DIR.to_string() + "/data/ext/included", &["**/*"])?;

    let pas_glob = tmp.to_string_lossy() + "/**/*.pas";
    let pas_contents = file_contents(&pas_glob)?;
    assert_eq!(pas_contents.len(), 1);

    pasfmt()?.current_dir(&tmp).arg("*.pas").assert().success();
    assert_no_contents_changed(&pas_glob, &pas_contents)?;

    Ok(())
}

#[test]
fn glob_can_include_any_ext() -> TestResult {
    let tmp = TempDir::new()?;

    tmp.copy_from(TESTS_DIR, &["data/ext/excluded/*"])?;

    let excluded_glob = tmp.to_string_lossy() + "/data/ext/excluded/*";
    let excluded_files_contents = file_contents(&excluded_glob)?;
    assert_eq!(excluded_files_contents.len(), 6);

    pasfmt()?
        .current_dir(&tmp)
        .args(["**/*.*pas*", "**/*.*dpr*", "**/*.*dpk*"])
        .assert()
        .success();
    assert_all_contents_changed(&excluded_glob, &excluded_files_contents)?;

    Ok(())
}

#[test]
fn invalid_glob() -> TestResult {
    let err_msg = "ERROR invalid glob expression `***`";
    let err_details =
        "Pattern syntax error near position 2: wildcards are either regular `*` or recursive `**`";
    fmt("***")?
        .failure()
        .stderr(predicate::str::contains(err_msg))
        .stderr(predicate::str::contains(err_details));

    Ok(())
}
