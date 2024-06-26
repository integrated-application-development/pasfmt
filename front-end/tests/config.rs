use assert_fs::fixture::FileWriteStr;

use crate::utils::*;

#[test]
fn invalid_reconstruction_settings() -> TestResult {
    const ERR_MSG: &str = "Error: Invalid reconstruction settings: newline sequence must be all whitespace (was \"a\")";

    pasfmt()?
        .arg("-Creconstruction.eol=a")
        .assert()
        .failure()
        .stderr(predicates::str::starts_with(ERR_MSG));

    Ok(())
}

#[test]
fn files_mode_conflicts_with_stdin() -> TestResult {
    const ERR_MSG: &str = "error: Files mode not supported when reading from stdin.";

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
fn invalid_keys_in_toml_are_ignored() -> TestResult {
    let config = assert_fs::NamedTempFile::new("pasfmt.toml")?;
    // While we'd probably prefer to get a warning for invalid keys in the config file,
    // The framework we're using doesn't support that.
    config.write_str(
        "\
        asdf = 0\n\
        [reconstruction]\n\
        foo = \" \"\n\
        eol = \" \"\n\
        ",
    )?;

    pasfmt()?
        .write_stdin("a")
        .arg("--config-file")
        .arg(config.path())
        .assert()
        .success()
        .stdout("a ");

    Ok(())
}
