use assert_fs::{prelude::*, TempDir};
use predicates::prelude::*;
use std::fs::read_to_string;
use std::path::Path;

use crate::utils::*;

#[test]
fn format_file() -> TestResult {
    let tmp = TempDir::new()?;

    let child = tmp.child("foo.pas");
    child.write_str("a ;")?;
    fmt(&*child)?.success();

    assert_eq!(read_to_string(child)?, "a;\n",);

    Ok(())
}

#[test]
fn format_dir() -> TestResult {
    let tmp = TempDir::new()?;

    let child = tmp.child("foo.pas");
    child.write_str("a ;")?;

    let child2 = tmp.child("bar.pas");
    child2.write_str("b ;")?;

    fmt(&*tmp)?.success();

    assert_eq!(read_to_string(child)?, "a;\n",);
    assert_eq!(read_to_string(child2)?, "b;\n",);

    Ok(())
}

#[test]
fn format_stdin() -> TestResult {
    let mut cmd = pasfmt()?;

    cmd.write_stdin("a:=b;")
        .current_dir(TESTS_DIR)
        .assert()
        .success()
        .stdout(predicate::eq("a := b;\n"));

    Ok(())
}

#[test]
fn format_files_to_stdout() -> TestResult {
    let tmp = TempDir::new()?;

    let child = tmp.child("foo.pas");
    child.write_str("a ;")?;
    let child2 = tmp.child("bar.pas");
    child2.write_str("b ;")?;

    pasfmt()?
        .arg("--mode=stdout")
        .arg(tmp.as_os_str())
        .current_dir(TESTS_DIR)
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "{}:\na;\n",
            child.display()
        )))
        .stdout(predicate::str::contains(format!(
            "{}:\nb;\n",
            child2.display()
        )));

    assert_eq!(
        read_to_string(child)?,
        "a ;",
        "Files should not be written when formatting to stdout."
    );
    assert_eq!(
        read_to_string(child2)?,
        "b ;",
        "Files should not be written when formatting to stdout."
    );

    Ok(())
}

fn run_check_test(path: &Path) -> AssertResult {
    Ok(pasfmt()?
        .arg("--mode=check")
        .arg(path)
        .current_dir(TESTS_DIR)
        .assert())
}

#[test]
fn check_file_fail() -> TestResult {
    let child = assert_fs::NamedTempFile::new("foo.pas")?;
    child.write_str("a ;")?;

    run_check_test(child.path())?
        .failure()
        .stderr(predicate::str::contains(format!(
            "ERROR CHECK: '{}' has incorrect formatting\n",
            child.display()
        )));

    Ok(())
}

#[test]
fn check_file_pass() -> TestResult {
    let child = assert_fs::NamedTempFile::new("foo.pas")?;
    child.write_str("a ;")?;
    // get it into a correctly-formatted state
    fmt(&*child)?;

    run_check_test(child.path())?.success().stderr(
        predicate::str::contains(format!(
            "ERROR CHECK: '{}' has incorrect formatting\n",
            child.display()
        ))
        .not(),
    );

    Ok(())
}

#[test]
fn check_files() -> TestResult {
    let tmp = TempDir::new()?;

    let formatted_child = tmp.child("foo.pas");
    formatted_child.write_str("a;")?;
    // get it into a correctly-formatted state
    fmt(&*formatted_child)?;
    let formatted_child_orig_contents = read_to_string(&formatted_child)?;

    let unformatted_child = tmp.child("bar.pas");
    unformatted_child.write_str("b ;")?;

    run_check_test(tmp.path())?
        .failure()
        .stderr(predicate::str::contains(format!(
            "ERROR CHECK: '{}' has incorrect formatting\n",
            unformatted_child.display()
        )))
        .stderr(
            predicate::str::contains(format!(
                "ERROR CHECK: '{}' has incorrect formatting\n",
                formatted_child.display()
            ))
            .not(),
        );

    assert_eq!(
        read_to_string(formatted_child)?,
        formatted_child_orig_contents,
        "Files should not be written when in check mode."
    );
    assert_eq!(
        read_to_string(unformatted_child)?,
        "b ;",
        "Files should not be written when in check mode."
    );

    Ok(())
}

#[test]
fn check_stdin() -> TestResult {
    let mut cmd = pasfmt()?;

    cmd.write_stdin("a:=b;")
        .current_dir(TESTS_DIR)
        .arg("--mode=check")
        .assert()
        .failure()
        .stderr(predicate::str::starts_with(
            "Error: <stdin> is incorrectly formatted\n",
        ));

    Ok(())
}
