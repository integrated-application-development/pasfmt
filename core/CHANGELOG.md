# Changelog

This file documents notable changes to the `pasfmt-core` crate.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Stopped deleting "voided" lines.

### Added

- Added `ConditionalDirectiveConsolidator` to combine _simple_ conditional statements into a single `LogicalLine`.

## [0.4.0-rc1] - 2025-02-20

### Fixed

- Fixed lexing of conditional directive expressions containing compiler directives, comments, or strings.
- Fixed lexing of compiler directives similar to conditional directives (e.g. `{$if_}`).
- Fixed lexing of unterminated asm text literals at EOF.
- Fixed lexing of sequential conditionally-compiled asm keywords.
- Fixed parsing of repeated label definitions.
- Fixed consolidation of `in` as identifier in `class operator In` construct.
- Fixed duplicated line attribution of mid-line compiler directives.
- Fixed parsing of `class helper`s with parent a type.
- Fixed parsing of generics parameters in routine headers.
- Fixed block comment kind for multiline comments on their own line.
- Fixed parsing of anonymous routines with trailing semicolons.
- Fixed parent line attribution of parent lines whose line number changes with conditional compilation.
- Fixed `LogicalLineType` of inline assembly instructions without leading new line.
- Fixed `LogicalLineType::Declaration` attribution for declarations in a visibility section.
- Fixed inline declaration parsing in various statement lists.

### Changed

- Variant record fields are now parsed as child lines of the record tag.
- `uses`, `contains`, `requires`, and `exports` clauses are now parsed as heading and body lines.
- Added parameter to the `Formatter::format` function, allowing per-file options to be passed in
  (e.g. cursors to track).
- Changed `Reconstructor::reconstruct` to have the signature of the old `Reconstructor::reconstruct_into_buf`.
- Renamed `FormattingSettings` to `FormattingConfig`.
- `while`, `with`, `on`, `if`, case arm and `for` statements are now parsed with child lines for
  their nested statements.
- Changed the signature of `ReconstructionSettings::new`.
  Instead of allowing the user to provide the three strings, it is now abstracted over:
  - `LineEnding`
  - `TabKind`
  - a count of tabs for continuation
  - a count of tabs for indentation

### Added

- Added `ChevronKind` for distinguishing greater-than and less-than operators.
- Added `EqKind` for distinguishing declaration and binary equal operators.
- Added `InKind` for distinguishing import, for loop, and binary `In` keywords.
- Added `CaretKind` for distinguishing pointer type, and variable dereference operators.
- Added `OperatorKind::Caret(CaretKind)` to represent `^` tokens.
- Added `DeclKind` for distinguishing kinds of `var` and `const` keywords.
- Added `LogicalLineType::CaseArm` to represent the arms of a case statement.
- Added `LogicalLineType::Declaration` to represent `label`, `type`, `const` etc. declarations.
- Added `OptimisingLineFormatter` to reflow all kinds of logical lines.
- Added `IgnoreAsmInstructions` to ignore all ASM instruction lines.
- Added trait method `LogicalLinesReconstructor::process_cursor`.
- Added trait `CursorMetadata`.

### Removed

- Removed `LGeneric` and `RGeneric` token type in favour of `ChevronKind`.
- Removed `OperatorKind::Pointer` in favour of `OperatorKind::Caret`.
- Removed `RemoveRepeatedNewlines`, `ImportClauseConsolidator`, and `UsesClauseFormatter` in favour of `OptimisingLineFormatter`
- Removed `Formatter::format_into_buf` function.
- Removed `Reconstructor::reconstruct_into_buf` trait method (renamed).

## [0.3.0] - 2024-05-29

### Fixed

- Added missing `winapi` keyword.

### Changed

- Changed `Token` type to be a struct containing a `std::borrow::Cow` for the token content instead of
  an enum of `RefToken` and `OwningToken`.
- Changed `FormattingData` fields from `usize` to `u16`.

### Removed

- Removed `RefToken` and `OwningToken` types.

## [0.2.0] - 2024-05-07

### Fixed

- Fixed incorrect parsing for generic type param lists containing semicolons.
- Removed extra trailing newline when formatting stdin to stdout.
- Fixed lexical edge cases:
  - Codepoints in `[U+00, U+20)` and `U+3000` are now lexed as whitespace instead of `Unknown`.
  - Non-ascii codepoints (excluding `U+3000`) are now lexed as identifiers instead of `Unknown`.
  - Hex and binary integer literals token types were reversed (unobservable with the current rules).
  - Binary literals now can contain underscore (e.g. `%_1`).
  - Asm labels now can contain `@` characters.
  - Asm integer literals are now supported (e.g. octal `076O`, hex `0FFH`/`$FF`, binary `010B`).
  - Keywords used in qualified names are now lexed as identifiers (e.g. `System.String`).
  - Integer literals can now be escaped with multiple ampersands (e.g. `&&0`).
- Fixed incorrect encoding used for writing files with encodings inferred from a BOM.
- Fixed incorrect encoding used in stdin/stdout mode; UTF-8 was always used, but now the configured
  encoding is respected.

### Changed

- Changed `TokenType::TextLiteral` to contain a `TextLiteralKind`.
- Changed handling of unterminated comments and text literals.
- Changed the intermediate type of tokens between lexing and line parsing from `Token` to `RawToken`.
  This is to allow the logical line parser to consolidate the token types while processing.
  Additionally, formatters and consolidators operating after parsing do not need to worry about
  any identifier/keyword ambiguities.
- Changed logical line parsing:
  - Added more `LogicalLineType` variants.
    - `Assignment`
    - `CompilerDirective`
    - `ForLoop`
    - `RoutineHeader`
    - `InlineDeclaration`
    - `InlineStatement`
    - `Guid`
    - `CaseHeader`
  - Added support for more Delphi structures:
    - Tagged records.
    - `with` statements.
    - `forward` and `external` routine declarations.
  - Consolidated all token types based on context.
  - Improved support for `property` and routine directives.

### Added

- Added `string` as a keyword.
- Added support for Delphi 12 multi-line string literals.
- Added `Formatter::format_into_buf` and `Reconstructor::reconstruct_into_buf` to allow reuse of memory
  allocations.
- Added validation on the reconstruction settings. This now ensures that indentation, eol, and continuation
  are all non-empty and only consist of whitespace. Without this validation, the format may not be
  idempotent.
- Added `RawTokenType` as a copy of `TokenType` with `IdentifierOrKeyword`.

### Removed

- Removed `add`, `remove`, and `variant` as keywords.
- Removed `IdentifierOrKeyword` variant of `TokenType` enum.

## [0.1.0] - 2023-08-28

### Added

- Added `lang` module containing all common types.
- Added `traits` module outlining the API.
  - `Lexer`
  - `TokenConsolidator`
  - `LogicalLineParser`
  - `LogicalLineConsolidator`
  - `TokenIgnorer`
  - `TokenRemover`
  - `LogicalLineFormatter`
  - `LogicalLineFileFormatter`
  - `Reconstructor`
- Added default implementations of the fundamental traits.
- Added trailing whitespace removal.
- Added `rules` module containing dedicated formatters
  - `UsesClauseFormatter`
  - `RemoveRepeatedNewlines`
  - `EofNewline`
  - `FormattingToggler`
  - `TokenSpacing`
