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
