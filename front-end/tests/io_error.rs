use assert_fs::prelude::*;
use predicates::prelude::*;

use crate::utils::*;

#[test]
fn non_existent() -> TestResult {
    fmt("foo.pas")?
        .failure()
        .stderr(predicate::str::contains("ERROR failed to open 'foo.pas'"));

    Ok(())
}

#[test]
fn io_error_does_not_stop_formatting_other_files() -> TestResult {
    let child = assert_fs::NamedTempFile::new("foo.pas")?;
    child.write_str("a ;")?;

    pasfmt()?
        .arg(child.path())
        .arg("foo.pas")
        .current_dir(TESTS_DIR)
        .assert()
        .failure()
        .stderr(predicate::str::contains("ERROR failed to open 'foo.pas'"));

    child.assert("a;\n");

    Ok(())
}

#[cfg(windows)]
mod windows {
    use super::*;

    use windows_sys::Win32::Storage::FileSystem::LOCKFILE_EXCLUSIVE_LOCK;

    use crate::utils::windows::fmt_with_lock;

    const LOCKED_FILE_ERR_MSG: &str = "The process cannot access the file because another process has locked a portion of the file.";

    #[test]
    fn read_fail() -> TestResult {
        let child = assert_fs::NamedTempFile::new("foo.pas")?;
        child.write_str("a ;")?;

        fmt_with_lock(child.path(), LOCKFILE_EXCLUSIVE_LOCK)?
            .failure()
            .stderr(predicate::str::contains("ERROR failed to read "))
            .stderr(predicate::str::contains(LOCKED_FILE_ERR_MSG));

        Ok(())
    }

    #[test]
    fn write_fail() -> TestResult {
        let child = assert_fs::NamedTempFile::new("foo.pas")?;
        child.write_str("a ;")?;

        fmt_with_lock(child.path(), 0)?
            .failure()
            .stderr(predicate::str::contains("ERROR failed to write to "))
            .stderr(predicate::str::contains(LOCKED_FILE_ERR_MSG));

        Ok(())
    }
}
