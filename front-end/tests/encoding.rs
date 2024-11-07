use assert_fs::prelude::*;
use predicates::prelude::*;

use crate::utils::*;

mod bom {
    use super::*;

    fn assert_fmt(input: &'static [u8], expected_out: &'static [u8]) -> TestResult {
        pasfmt()?
            .current_dir(TESTS_DIR)
            .write_stdin(input)
            .assert()
            .success()
            .stdout(predicate::eq(expected_out));

        let tmp = assert_fs::NamedTempFile::new("foo.pas")?;
        tmp.write_binary(input)?;
        pasfmt()?
            .current_dir(TESTS_DIR)
            .arg(tmp.path())
            .assert()
            .success();
        tmp.assert(expected_out);

        Ok(())
    }

    #[test]
    fn utf8() -> TestResult {
        // "😃 ;"
        assert_fmt(
            b"\xEF\xBB\xBF\xF0\x9F\x98\x83 ;\n",
            b"\xEF\xBB\xBF\xF0\x9F\x98\x83;\n",
        )
    }

    #[test]
    fn utf16le() -> TestResult {
        // "😃 ;"
        assert_fmt(
            b"\xFF\xFE\x3D\xD8\x03\xDE \0;\0\n\0",
            b"\xFF\xFE\x3D\xD8\x03\xDE;\0\n\0",
        )
    }

    #[test]
    fn utf16be() -> TestResult {
        // "😃 ;"
        assert_fmt(
            b"\xFE\xFF\xD8\x3D\xDE\x03\0 \0;\0\n",
            b"\xFE\xFF\xD8\x3D\xDE\x03\0;\0\n",
        )
    }

    #[test]
    fn utf32le_unsupported() -> TestResult {
        // "😃 ;"
        // Should be interpreted as UTF-16LE and mangled accordingly
        assert_fmt(
            b"\xFF\xFE\0\0\x03\xF6\x01\0\x20\0\0\0;\0\0\0\n\0\0\0",
            b"\xFF\xFE\x03\xF6;\0\n\0",
        )
    }

    #[test]
    fn utf32be_unsupported() -> TestResult {
        // "😃 ;"
        // Should be interpreted as the configured encoding (UTF-8), since no BOM will be detected.
        // This should trigger a decoding error, because it's not valid UTF-8.
        pasfmt()?
            .current_dir(TESTS_DIR)
            .write_stdin(b"\0\0\xFE\xFF\0\x01\xF6\x03\0\0\0\x20\0\0\0;\0\0\0\n")
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "File '<stdin>' has malformed sequences (in encoding 'UTF-8')",
            ));

        Ok(())
    }
}

mod codepages {
    use super::*;

    /* I'd like to be able to test that the default encoding on Windows is the system
     * ANSI code page (ACP), but that's difficult for two reasons:
     *   1. It's not practical to control the system ACP from a test, so we'd have to rely on
     *      whatever the system has configured.
     *   2. It's impossible to detect (with current formatting rules) that the configured
     *      code page has been used; if the wrong code page is used, the data will be mangled
     *      on the way in, but anti-mangled on the way out with no differences. It would only
     *      be observable if you had formatting rules that behave differently when formatting
     *      tokens with non-ASCII content.
     */

    #[test]
    fn valid() -> TestResult {
        // hardly all of the supported encodings, but some of them
        for cp in [
            "windows-1252",
            "windows-1250",
            "GBK",
            "Big5",
            "IBM866",
            "ISO-2022-JP",
            "gb18030",
        ] {
            pasfmt()?
                .write_stdin("a ;")
                .current_dir(TESTS_DIR)
                .arg("-Cencoding=".to_string() + cp)
                .assert()
                .success()
                .stdout("a;\n");
        }

        Ok(())
    }

    #[test]
    fn invalid() -> TestResult {
        let err_msg =
            "invalid encoding label: cpASDF for key \"default.encoding\" in command-line overrides";

        pasfmt()?
            .write_stdin("a ;")
            .current_dir(TESTS_DIR)
            .arg("-Cencoding=cpASDF")
            .assert()
            .failure()
            .stderr(predicate::str::contains(err_msg));

        Ok(())
    }
}

mod malformed {
    use super::*;

    fn test_malformed_data(
        bom: &'static [u8],
        data: &'static [u8],
        encoding: &'static str,
    ) -> TestResult {
        pasfmt()?
            .write_stdin(data)
            .arg(format!("-Cencoding={encoding}"))
            .assert()
            .failure()
            .stderr(predicate::str::contains(format!(
                "File '<stdin>' has malformed sequences (in encoding '{encoding}')"
            )));

        pasfmt()?
                .write_stdin([bom, data].concat())
                .assert()
                .failure()
                .stderr(predicate::str::contains(format!(
                    "File '<stdin>' has malformed sequences (in encoding '{encoding}' - inferred from BOM)"
                )));

        Ok(())
    }

    #[test]
    fn invalid_utf8() -> TestResult {
        test_malformed_data(b"\xEF\xBB\xBF", b"\xC3\x28", "UTF-8")
    }

    #[test]
    fn invalid_utf16be() -> TestResult {
        test_malformed_data(b"\xFE\xFF", b"\xD8\x00", "UTF-16BE")
    }

    #[test]
    fn invalid_utf16le() -> TestResult {
        test_malformed_data(b"\xFF\xFE", b"\x00\xD8", "UTF-16LE")
    }

    #[test]
    fn invalid_output() -> TestResult {
        // Set the EOL sequence to U+3000 and try to encode it as win1252
        // it should fail to encode
        pasfmt()?
            .write_stdin("")
            .arg("-Creconstruction.eol=\"\u{3000}\"")
            .arg("-Cencoding=windows-1252")
            .assert()
            .failure()
            .stderr(predicate::str::contains("ERROR Failed to write to stdout"))
            .stderr(predicate::str::contains(
                "Formatting result contains data that cannot be encoded as windows-1252",
            ));
        Ok(())
    }
}
