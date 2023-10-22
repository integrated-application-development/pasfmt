# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `string` as a keyword
- Support for Delphi 12 multi-line string literals
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
- `Formatter::format_into_buf` and `Reconstructor::reconstruct_into_buf` to allow reuse of memory
  allocations.
- Validation on the reconstruction settings. This now ensures that indentation, eol, and continuation
  are all non-empty and only consist of whitespace. Without this validation, the format may not be
  idempotent.
- Colour styling in CLI help.
- `RawTokenType` as a copy of `TokenType` with `IdentifierOrKeyword`

### Removed

- `add`, `remove`, and `variant` as keywords
- short version of `--config-file` CLI option
- `--write`/`-w` CLI option (now accessible via `--mode=files`).
- `--verify` CLI option (replaced by `--mode=check` with some differences).
- `IdentifierOrKeyword` variant of `TokenType` enum

### Changed

- `TokenType::TextLiteral` to contain a `TextLiteralKind`.
- The default encoding on Windows to be the system ANSI codepage.
- The default encoding on non-Windows platforms to be UTF-8.
- Handling of IO errors. Previously any IO error would immediately crash the program.
  Now errors are logged when they occur and cause the program to exit non-zero after
  all other files have been formatted successfully.
- Default continutation to be the same as the configured indentation.
- The intermediate type of tokens between lexing and line parsing from `Token` to `RawToken`.
  This is to allow the logical line parser to consolidate the token types while processing.
  Additionally, formatters and consolidators operating after parsing do not need to worry about
  any identifier/keyword ambiguities.

### Fixed

- Incorrect parsing for generic type param lists containing semicolons.
- Extra trailing newline when formatting stdin to stdout.
- Lexical edge cases:
  - Incorrect lexing of codepoints in the interval [U+00, U+20) as `Unkown`, now lexed as whitespace.
  - Incorrect lexing of codepoint U+3000 as `Unkown`, now lexed as whitespace.
  - Incorrect lexing of non-ascii codepoints as `Unkown`, now lexed as identifiers (excluding U+3000).
  - Incorrect lexing of hex and binary integer literals:
    - Previously not accepted with no digits (`$` and `%`)
    - Previously the classification was reversed (unobservable with the current rules).
    - Previously not accepted with underscore as the first digit (`$_1`)
  - Incorrect lexing of asm blocks
    - Asm labels can now start with just one '@' character instead of two, and they can contain '@' characters.
    - Asm integer literals now supported (e.g. octal `076O`, hex `0FFH`/`$FF`, binary `010B`)
  - Incorrect lexing of `..` tokens used directly after integer literals (e.g. `0..1`).
  - Incorrect lexing of keywords used in qualified names (e.g. `System.String`); now lexed as identifiers.
  - Incorrect token type of unterminated text literal at end of file.
  - Incorrect token type of ampersand-escaped integer literals with multiple ampersands.
  - Incorrect lexing of ampersand-escaped unicode identifiers.
- Incorrect encoding used for writing files with encodings inferred from a BOM.
- Incorrect encoding used in stdin/stdout mode; UTF-8 was always used, but now the configured
  encoding is respected.

## [0.1.0] - 2023-08-28

### Added

- `lang` module containing all common types.
- `traits` module outlining the API.
  - `Lexer`
  - `TokenConsolidator`
  - `LogicalLineParser`
  - `LogicalLineConsolidator`
  - `TokenIgnorer`
  - `TokenRemover`
  - `LogicalLineFormatter`
  - `LogicalLineFileFormatter`
  - `Reconstructor`
- Default implementations of the fundamental traits.
- Ability to recognise and format uses clauses.
  - Package and library imports are not yet parsed.
  - Program uses clauses are ignored.
- Formatter to remove repeated empty new lines.
- Formatter to normalise to a single new line at eof.
- Trailing whitespace removal. This is a built-in feature.
- Orchestrator to handle parallelising the reading, formatting, and writing of files.
- Configuration and command-line frameworks.
- Default command-line interface.
- Formatting toggle comments.
- Benchmarking tool.
- Token spacing rules.
