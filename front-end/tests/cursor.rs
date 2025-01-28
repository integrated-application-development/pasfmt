use assert_fs::prelude::FileWriteStr;

use crate::utils::*;

fn test_cursors(input: &str, input_cursors: &str, expected_cursors: &str) -> TestResult {
    let expected_cursor_line = "CURSOR=".to_string() + expected_cursors + "\n";

    // stdin -> stdout
    pasfmt()?
        .write_stdin(input)
        .arg("--cursor")
        .arg(input_cursors)
        .assert()
        .success()
        .stderr(predicates::str::contains(&expected_cursor_line));

    // on file
    let tmp = assert_fs::NamedTempFile::new("foo.pas")?;
    tmp.write_str(input)?;
    pasfmt()?
        .arg(tmp.path())
        .arg("--cursor")
        .arg(input_cursors)
        .assert()
        .success()
        .stderr(predicates::str::contains(&expected_cursor_line));

    Ok(())
}

#[test]
fn cursors_on_stderr_comma_separated() -> TestResult {
    test_cursors(
        "begin a:=b;end;\n",
        "0,1,5,6,7,15,16,100",
        "0,1,5,8,9,20,21,21",
    )?;
    Ok(())
}
