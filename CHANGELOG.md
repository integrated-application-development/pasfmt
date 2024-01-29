# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `string` as a keyword
- Support for Delphi 12 multi-line string literals
- `-C` CLI option to override configuration options.

### Removed

- `add`, `remove`, and `variant` as keywords
- short version of `--config-file` CLI option

### Changed

- `TokenType::TextLiteral` to contain a `TextLiteralKind`.
- The default encoding on Windows to be the system ANSI codepage.
- The default encoding on non-Windows platforms to be UTF-8.

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
- Incorrect encoding used for writing files with encodings inferred from a BOM.

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
