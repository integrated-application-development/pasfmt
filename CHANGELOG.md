# Changelog

All notable changes to `pasfmt` will be documented in this file.
More detailed changelog files for some of the underlying crates can be found in their respective directories.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Removed

- Ability to override configuration on the CLI with escaped values eg (`-Ckey="\uFF"`).
- Support for inline assembly instruction formatting. Instructions lines are ignored.

### Changed

- Error message when `--mode=check` fails on stdin, now reads `'<stdin>' has incorrect formatting`.
- Style of the (debug-level) logging of the current configuration. Previously it was in TOML format,
  now it's a simpler struct-level debug formatting.
- Wrap CLI help at the smallest of the terminal width and 120 characters.
- Whitespace configuration
  - `line_ending` now controls the end-of-line sequence (previously `reconstruction.eol`).
  - `use_tabs` now controls the character used for indentation.
  - `tab_width` now controls the number of spaces per indentation.
  - `continuation_indents` now controls the number of indentations per continuation.
- Indentation level of comments in various edge cases.

### Added

- Line wrapping to reflow all kinds of logical lines.
  - Configuration option `wrap_column` controls the line length.
  - Configuration option `begin_style` controls the location of `begin` in control flow statements.
- Cursor tracking, via the `--cursor` CLI option.
  - Expects a comma-separated list of UTF-8 byte offsets into the file. After formatting, a line
    with the structure `CURSOR=1,2,3,4` is printed to stderr, indicating the new locations of the
    provided cursors.
- Debug logging for time taken to format each file.

### Fixed

- Lexing of conditional directive expressions containing compiler directives, comments, or strings.
- Lexing of compiler directives similar to conditional directives (e.g. `{$if_}`).
- Lexing of unterminated asm text literals at EOF.
- Lexing of sequential conditionally-compiled asm keywords.
- `--config-file` no longer erroneously accepts a path to a directory and searches from it for a
  `pasfmt.toml` file. It is now an error to provide a path to a directory.
- Detection of decoding errors when reading from stdin.
- Silent ignoral of non-existent paths (were being treated as globs matching no files).
- Errors on writing non-ASCII data in an ANSI encoding to a Windows Console. Now written as Unicode.

## [0.3.0] - 2024-05-29

### Changed

- `--files-file` parameter to `--files-from`.

### Fixed

- Erroneous removal of BOM.
- Erroneous transcoding of UTF-16 input to UTF-8.
- Unwanted use of the replacement character when encoding. This now results in an error.

## [0.2.0] - 2024-05-07

### Added

- Support for Delphi 12 multi-line string literals.
- `-C` CLI option to override configuration options.
- `--mode` CLI option which replaces the `--write` and `--verify` options.
  The default for this option is dynamic; when files to format are provided the default is `files`,
  otherwise (when formatting stdin) it's `stdout`. This differs from the previous default behaviour
  which was equivalent to `stdout`. <br/>
  `check` mode replaces `--verify` with some differences:
  - Non-zero exit code in the case of incorrectly formatted files.
  - Can be used in stdin/stdout mode.
  - Messages for incorrectly formatted files prefixed with 'CHECK' instead of 'VERIFY', and are
    sent to stderr, not stdout.
- Validation on the reconstruction settings. This now ensures that indentation, eol, and continuation
  are all non-empty and only consist of whitespace. Without this validation, the format may not be
  idempotent.
- Colour styling in CLI help.

### Removed

- short version of `--config-file` CLI option.
- `--write`/`-w` CLI option (now accessible via `--mode=files`).
- `--verify` CLI option (replaced by `--mode=check` with some differences).

### Changed

- Handling of unterminated comments and text literals.
- The default encoding on Windows to be the system ANSI codepage.
- The default encoding on non-Windows platforms to be UTF-8.
- Handling of IO errors. Previously any IO error would immediately crash the program.
  Now errors are logged when they occur and cause the program to exit non-zero after
  all other files have been formatted successfully.
- Default continuation to be the same as the configured indentation.

### Fixed

- Incorrect parsing for generic type param lists containing semicolons.
- Extra trailing newline when formatting stdin to stdout.
- Lexical edge cases with unusual whitespace.
- Incorrect encoding used for writing files with encodings inferred from a BOM.
- Incorrect encoding used in stdin/stdout mode; UTF-8 was always used, but now the configured
  encoding is respected.

## [0.1.0] - 2023-08-28

### Added

- Ability to recognise and format uses clauses.
  - Package and library imports are not yet parsed.
  - Program uses clauses are ignored.
- Formatter to remove repeated empty new lines.
- Formatter to normalise to a single new line at eof.
- Trailing whitespace removal. This is a built-in feature.
- Parallel reading, formatting, and writing of files.
- Command-line interface.
- Support for special comment syntax to disable formatting
  ```delphi
  // pasfmt off
  Foo ( );
  // pasfmt on
  ```
- Formatter for spaces between all tokens (on separate lines)
