use assert_fs::prelude::*;
use predicates::prelude::*;

use crate::utils::*;

#[test]
fn files_mode_conflicts_with_stdin() -> TestResult {
    const ERR_MSG: &str = "error: files mode not supported when reading from stdin";

    pasfmt()?
        .arg("--mode=files")
        .assert()
        .failure()
        .stderr(predicates::str::starts_with(ERR_MSG));

    Ok(())
}

#[test]
fn invalid_option() -> TestResult {
    const ERR_MSG: &str = "error: unexpected argument '--asdf' found";

    pasfmt()?
        .arg("--asdf")
        .assert()
        .failure()
        .stderr(predicates::str::starts_with(ERR_MSG));

    Ok(())
}

#[test]
fn invalid_keys_in_toml_raise_error() -> TestResult {
    let config = assert_fs::NamedTempFile::new("pasfmt.toml")?;
    config.write_str("asdf = 0")?;

    pasfmt()?
        .write_stdin("a")
        .arg("--config-file")
        .arg(config.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "ERROR failed to construct configuration",
        ))
        .stderr(predicates::str::contains("unknown field `asdf`, expected "));

    Ok(())
}

#[test]
fn config_file_found_in_current_dir() -> TestResult {
    let tmp = assert_fs::TempDir::new()?;
    let config = tmp.child("pasfmt.toml");
    config.write_str(r#"line_ending = "crlf""#)?;

    pasfmt()?
        .write_stdin("a := b;\na")
        .current_dir(tmp.path())
        .assert()
        .stdout(predicate::eq("a := b;\r\na\r\n"))
        .success();

    Ok(())
}

#[test]
fn config_file_found_in_parent_dir() -> TestResult {
    let tmp = assert_fs::TempDir::new()?;
    let child_dir = tmp.child("child/inner_child/");
    child_dir.create_dir_all()?;
    let config = tmp.child("pasfmt.toml");
    config.write_str(r#"line_ending = "crlf""#)?;

    pasfmt()?
        .write_stdin("a := b;\na")
        .current_dir(child_dir)
        .assert()
        .stdout(predicate::eq("a := b;\r\na\r\n"))
        .success();

    Ok(())
}
