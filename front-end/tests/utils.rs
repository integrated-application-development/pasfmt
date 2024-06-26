pub static TESTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests");

pub fn pasfmt() -> Result<assert_cmd::Command, assert_cmd::cargo::CargoError> {
    assert_cmd::Command::cargo_bin("pasfmt")
}

pub fn fmt(
    arg: impl AsRef<std::ffi::OsStr>,
) -> Result<assert_cmd::assert::Assert, assert_cmd::cargo::CargoError> {
    Ok(pasfmt()?
        .current_dir(TESTS_DIR)
        .arg("--log-level=DEBUG")
        .arg(arg)
        .assert())
}

pub type DynResult<T> = Result<T, Box<dyn std::error::Error>>;
pub type TestResult = DynResult<()>;
pub type AssertResult = DynResult<assert_cmd::assert::Assert>;
