use assert_fs::{prelude::*, TempDir};
use predicates::prelude::*;

use crate::utils::*;

#[test]
fn file_is_only_written_when_necessary() -> TestResult {
    let tmp = TempDir::new()?;

    let to_leave = tmp.child("to_leave.pas");
    to_leave.write_str("a;")?;
    fmt(&*to_leave)?.success();
    let to_leave_last_written = to_leave.metadata()?.modified()?;

    let to_change = tmp.child("to_change.pas");
    to_change.write_str("a ;")?;
    let to_change_last_written = to_change.metadata()?.modified()?;

    fmt(&*tmp)?
        .success()
        .stderr(predicate::str::contains(format!(
            "DEBUG Skipping writing to '{}' because it is already formatted.",
            to_leave.display()
        )));

    assert_eq!(
        to_leave.metadata()?.modified()?,
        to_leave_last_written,
        "File already in formatted state should not be written to"
    );

    assert_ne!(
        to_change.metadata()?.modified()?,
        to_change_last_written,
        "File not already in formatted state should be written to"
    );

    Ok(())
}
