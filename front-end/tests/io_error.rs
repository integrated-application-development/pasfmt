use assert_fs::prelude::*;
use predicates::prelude::*;

use crate::utils::*;

#[test]
fn non_existent() -> TestResult {
    fmt("foo.pas")?
        .failure()
        .stderr(predicate::str::contains("ERROR Failed to open 'foo.pas'"))
        .stderr(predicate::str::contains("Error: 1 file with errors"));

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
        .stderr(predicate::str::contains("ERROR Failed to open 'foo.pas'"))
        .stderr(predicate::str::contains("Error: 1 file with errors"));

    child.assert("a;\n");

    Ok(())
}

#[cfg(windows)]
mod windows {
    use super::*;

    use std::os::windows::io::AsRawHandle;
    use std::path::Path;
    use windows_sys::Win32::{
        Storage::FileSystem::{LockFileEx, LOCKFILE_EXCLUSIVE_LOCK, LOCK_FILE_FLAGS},
        System::IO::OVERLAPPED,
    };

    fn fmt_with_lock(path: &Path, flags: LOCK_FILE_FLAGS) -> AssertResult {
        let handle = std::fs::OpenOptions::new().write(true).open(path)?;
        unsafe {
            let mut overlapped: OVERLAPPED = std::mem::zeroed();
            let ret = LockFileEx(
                handle.as_raw_handle() as isize,
                flags,
                0,
                !0,
                !0,
                &mut overlapped,
            );

            if ret == 0 {
                return Err(Box::new(std::io::Error::last_os_error()));
            }
        }
        // I don't think a guard to unlock the file is necessary, because the Microsoft docs say
        //   If a process terminates with a portion of a file locked or closes a file that has
        //   outstanding locks, the locks are unlocked by the operating system.
        // And the file handle is closed at the end of this function.

        Ok(fmt(path)?)
    }

    const LOCKED_FILE_ERR_MSG: &str = "The process cannot access the file because another process has locked a portion of the file.";

    #[test]
    fn read_fail() -> TestResult {
        let child = assert_fs::NamedTempFile::new("foo.pas")?;
        child.write_str("a ;")?;

        fmt_with_lock(child.path(), LOCKFILE_EXCLUSIVE_LOCK)?
            .failure()
            .stderr(predicate::str::contains("ERROR Failed to read "))
            .stderr(predicate::str::contains(LOCKED_FILE_ERR_MSG));

        Ok(())
    }

    #[test]
    fn write_fail() -> TestResult {
        let child = assert_fs::NamedTempFile::new("foo.pas")?;
        child.write_str("a ;")?;

        fmt_with_lock(child.path(), 0)?
            .failure()
            .stderr(predicate::str::contains("ERROR Failed to write to "))
            .stderr(predicate::str::contains(LOCKED_FILE_ERR_MSG));

        Ok(())
    }
}
