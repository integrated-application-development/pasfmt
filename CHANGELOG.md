# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `lang` module containing all common types.
- `traits` module outlining the API.
  - `Lexer`
  - `TokenConsolidator`
  - `LogicalLineParser`
  - `LogicalLineConsolidator`
  - `LogicalLineFormatter`
  - `LogicalLineFileFormatter`
  - `Reconstructor`
- Delphi Lexer.
- Basic Delphi logical line parsing.
- Token reconstructor.
- Formatter selector.
- Ability to recognise and format uses clauses.
- Ability to parse and recognise asm blocks.
- Formatter to remove repeated empty new lines.
- Formatter to normalise to a single new line at eof.
- Trailing whitespace removal. This is a built-in feature.
- FileFormatter to handle reading and writing to files.
- Configuration and command-line frameworks.
- Default command-line interface.
- Formatting toggle comments.
- Benchmarking tool.
- Token spacing rules.
