# Changelog

All notable changes to `pasfmt` will be documented in this file.
More detailed changelog files for some of the underlying crates can be found in their respective directories.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Fixed

- Fixed spacing around generics with composite type parameters (e.g. `array of`, `set of`, `string[10]`).
- Fixed fields being merged with comments in variant records.
- Fixed end of line comment placement after compiler directives.
- Fixed infinite loop on unterminated parameter list.

### Changed

- Allow case arm statements to be formatted inline.
- Allow simple conditionally compiled code to be formatted inline.

## [0.4.0-rc1] - 2025-02-20

### Fixed

- Fixed lexing of conditional directive expressions containing compiler directives, comments, or strings.
- Fixed lexing of compiler directives similar to conditional directives (e.g. `{$if_}`).
- Fixed lexing of unterminated asm text literals at EOF.
- Fixed lexing of sequential conditionally-compiled asm keywords.
- `--config-file` no longer erroneously accepts a path to a directory and searches from it for a
  `pasfmt.toml` file. It is now an error to provide a path to a directory.
- Fixed detection of decoding errors when reading from stdin.
- Stopped ignoring non-existent paths (they were being treated as globs matching no files).
- Fixed errors when writing non-ASCII data in an ANSI encoding to a Windows Console. Now written as Unicode.

### Changed

- Changed error message when `--mode=check` fails on stdin, now reads `'<stdin>' has incorrect formatting`.
- Started using simple struct-level debug formatting for logging of the current configuration.
  Previously it was in TOML format.
- Started wrapping CLI help at the smallest of the terminal width and 120 characters.
- Changed whitespace configuration
  - `line_ending` now controls the end-of-line sequence (previously `reconstruction.eol`). Default "native".
  - `use_tabs` now controls the character used for indentation. Default false.
  - `tab_width` now controls the number of spaces per indentation. Default 2.
  - `continuation_indents` now controls the number of indentations per continuation. Default 2.
- Changed indentation level of comments in various edge cases.

### Added

- Added line wrapping to reflow all kinds of logical lines.
  - Configuration option `wrap_column` controls the line length.
  - Configuration option `begin_style` controls the location of `begin` in control flow statements.
- Added cursor tracking, via the `--cursor` CLI option.
  - Expects a comma-separated list of UTF-8 byte offsets into the file. After formatting, a line
    with the structure `CURSOR=1,2,3,4` is printed to stderr, indicating the new locations of the
    provided cursors.
- Added debug logging for time taken to format each file.
- Added name for platform-dependent encoding configuration option, "native".
  This has the behaviour of the previous implicit default.

### Removed

- Stopped formatting inline assembly instructions. The lines are ignored.

## [0.3.0] - 2024-05-29

### Fixed

- Fixed erroneous removal of BOM.
- Fixed erroneous transcoding of UTF-16 input to UTF-8.
- Removed unwanted use of the replacement character when encoding. This now results in an error.

### Changed

- Renamed `--files-file` parameter to `--files-from`.

## [0.2.0] - 2024-05-07

### Fixed

- Fixed incorrect parsing for generic type param lists containing semicolons.
- Removed extra trailing newline when formatting stdin to stdout.
- Fixed lexical edge cases with unusual whitespace.
- Fixed incorrect encoding used for writing files with encodings inferred from a BOM.
- Fixed incorrect encoding used in stdin/stdout mode; UTF-8 was always used, but now the configured
  encoding is respected.

### Changed

- Changed handling of unterminated comments and text literals.
- Changed the default encodings
  - Windows: system ANSI codepage.
  - non-Windows: UTF-8.
- Improved handling of IO errors. Previously any IO error would immediately crash the program.
  Now errors are logged when they occur and cause the program to exit non-zero after
  all other files have been formatted successfully.

### Added

- Added support for Delphi 12 multi-line string literals.
- Added `-C` CLI option to override configuration options.
- Added `--mode` CLI option which replaces the `--write` and `--verify` options.
  The default for this option is dynamic; when files to format are provided the default is `files`,
  otherwise (when formatting stdin) it's `stdout`. This differs from the previous default behaviour
  which was equivalent to `stdout`. <br/>
  `check` mode replaces `--verify` with some differences:
  - Non-zero exit code in the case of incorrectly formatted files.
  - Can be used in stdin/stdout mode.
  - Messages for incorrectly formatted files prefixed with 'CHECK' instead of 'VERIFY', and are
    sent to stderr, not stdout.
- Added validation to the reconstruction settings. This now ensures that indentation, eol, and continuation
  are all non-empty and only consist of whitespace. Without this validation, the format may not be
  idempotent.
- Added colour styling in CLI help.

### Removed

- Removed short version of `--config-file` CLI option.
- Removed `--write`/`-w` CLI option (now accessible via `--mode=files`).
- Removed `--verify` CLI option (replaced by `--mode=check` with some differences).

## [0.1.0] - 2023-08-28

### Added

- Format uses clauses.
  - Program uses clauses are ignored.
- Remove repeated empty new lines.
- Normalise to a single new line at eof.
- Trim trailing whitespace.
- Parallel reading, formatting, and writing of files.
- Command-line interface.
- Support a special comment syntax to disable formatting
  ```delphi
  // pasfmt off
  Foo ( );
  // pasfmt on
  ```
- Format spaces between all tokens (on separate lines)
