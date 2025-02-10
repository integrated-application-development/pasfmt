pub static TESTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests");

pub fn pasfmt() -> Result<assert_cmd::Command, assert_cmd::cargo::CargoError> {
    let mut pasfmt = assert_cmd::Command::cargo_bin("pasfmt")?;
    pasfmt.current_dir(TESTS_DIR);
    Ok(pasfmt)
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

#[cfg(windows)]
pub mod windows {
    use super::*;

    use std::os::windows::io::AsRawHandle;
    use std::path::Path;
    use windows_sys::Win32::{
        Storage::FileSystem::{LockFileEx, LOCK_FILE_FLAGS},
        System::IO::OVERLAPPED,
    };

    pub fn fmt_with_lock(path: &Path, flags: LOCK_FILE_FLAGS) -> AssertResult {
        let handle = std::fs::OpenOptions::new().write(true).open(path)?;
        unsafe {
            let mut overlapped: OVERLAPPED = std::mem::zeroed();
            let ret = LockFileEx(handle.as_raw_handle(), flags, 0, !0, !0, &mut overlapped);

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
}
