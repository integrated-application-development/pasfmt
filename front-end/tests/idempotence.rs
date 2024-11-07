use assert_fs::{prelude::*, TempDir};
use predicates::prelude::*;

use crate::utils::windows::fmt_with_lock;
use crate::utils::*;

#[test]
fn file_is_only_written_when_necessary() -> TestResult {
    let tmp = TempDir::new()?;

    let to_leave = tmp.child("to_leave.pas");
    to_leave.write_str("a;")?;
    fmt(&*to_leave)?.success();

    let to_change = tmp.child("to_change.pas");
    to_change.write_str("a ;")?;

    fmt_with_lock(&to_leave, 0)?
        .success()
        .stderr(predicate::str::contains(format!(
            "DEBUG skipping writing to '{}' because it is already formatted",
            to_leave.display()
        )));

    fmt_with_lock(&to_change, 0)?
        .failure()
        .stderr(predicate::str::contains(format!(
            "ERROR failed to write to '{}'",
            to_change.display()
        )));

    Ok(())
}
