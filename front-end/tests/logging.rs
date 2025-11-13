use assert_fs::{TempDir, prelude::*};
use predicates::prelude::*;

use crate::utils::*;

/// The trace logging done by the [pasfmt_core::rules::optimising_line_formatter]
/// is quite complex, and has the potential to crash due to a bug.
///
/// While not covering all cases, this test will ensure that the basic
/// functionality of the logging works without crashing.
#[test]
fn trace_logging_does_not_crash() -> TestResult {
    let tmp = TempDir::new()?;

    let file = tmp.child("a.pas");
    file.write_str(
        "
if True then
  Foo;

A :=
    procedure
    begin
      var B :=
          procedure
          begin
            Foo;
          end;
    end;
",
    )?;

    pasfmt()?
        .arg("--log-level=TRACE")
        .arg(&*file)
        .assert()
        .success()
        .stderr(predicate::str::contains("TRACE"));

    Ok(())
}
