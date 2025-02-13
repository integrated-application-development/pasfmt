# Changelog

This file documents notable changes to the `pasfmt-core` crate.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Removed

- `LGeneric` and `RGeneric` token type in favour of `ChevronKind`.
- `OperatorKind::Pointer` in favour of `OperatorKind::Caret`.
- `RemoveRepeatedNewlines`, `ImportClauseConsolidator`, and `UsesClauseFormatter` in favour of `OptimisingLineFormatter`
- `Formatter::format_into_buf` function.
- `Reconstructor::reconstruct_into_buf` trait method (renamed).

### Changed

- Parsing of variant records. Variant fields are now child lines of the record tag.
- Parsing of `uses`, `contains`, `requires`, and `exports`. Now parsed as heading and body lines.
- Parameters to the `Formatter::format` function, allowing per-file options to be passed in
  (e.g. cursors to track).
- `Reconstructor::reconstruct` to have the signiture of the old `Reconstructor::reconstruct_into_buf`.
- Renamed `FormattingSettings` to `FormattingConfig`.
- Parsing of `while`, `with`, `on`, `if`, case arm and `for` statements to use
  child lines for their nested statements.
- Signature of `ReconstructionSettings::new`.
  Instead of allowing the user to provide the three strings, it is now abstracted over:
  - `LineEnding`
  - `TabKind`
  - a count of tabs for continuation
  - a count of tabs for indentation

### Added

- `ChevronKind` for distinguishing greater-than and less-than operators.
- `EqKind` for distinguishing declaration and binary equal operators.
- `InKind` for distinguishing import, for loop, and binary `In` keywords.
- `CaretKind` for distinguishing pointer type, and variable dereference operators.
- `OperatorKind::Caret(CaretKind)` to represent `^` tokens.
- `DeclKind` for distinguishing kinds of `var` and `const` keywords.
- `LogicalLineType::CaseArm` to represent the arms of a case statement.
- `LogicalLineType::Declaration` to represent `label`, `type`, `const` etc. declarations.
- `OptimisingLineFormatter` to reflow all kinds of logical lines.
- `IgnoreAsmIstructions` to ignore all ASM instruction lines.
- Method `process_cursor` to the `LogicalLinesReconstructor` trait.
- Trait `CursorMetadata`.

### Fixed

- Lexing of conditional directive expressions containing compiler directives, comments, or strings.
- Lexing of compiler directives similar to conditional directives (e.g. `{$if_}`).
- Lexing of unterminated asm text literals at EOF.
- Lexing of sequential conditionally-compiled asm keywords.
- Parsing of repeated label definitions.
- Consolidation of `in` as identifier in `class operator In` construct.
- Duplicated line attribution of mid-line compiler directives.
- Parsing of `class helper`s with parent a type.
- Parsing of generics parameters in routine headers.
- Block comment kind for multiline comments on their own line.
- Parsing of anonymous routines with trailing semicolons.
- Parent line attribution of parent lines whose line number changes with conditional compilation.
- `LogicalLineType` of inline assembly instructions without leading new line.
- `LogicalLineType::Declaration` attribution for declarations in a visibility section.
- Inline declaration parsing in various statement lists.

## [0.3.0] - 2024-05-29

### Removed

- `RefToken` and `OwningToken` types.

### Changed

- `Token` type to be a struct containing a `std::borrow::Cow` for the token content instead of
  an enum of `RefToken` and `OwningToken`.
- `FormattingData` fields from `usize` to `u16`.

### Fixed

- Missing `winapi` keyword.

## [0.2.0] - 2024-05-07

### Added

- `string` as a keyword.
- Support for Delphi 12 multi-line string literals.
- `Formatter::format_into_buf` and `Reconstructor::reconstruct_into_buf` to allow reuse of memory
  allocations.
- Validation on the reconstruction settings. This now ensures that indentation, eol, and continuation
  are all non-empty and only consist of whitespace. Without this validation, the format may not be
  idempotent.
- `RawTokenType` as a copy of `TokenType` with `IdentifierOrKeyword`.

### Removed

- `add`, `remove`, and `variant` as keywords.
- `IdentifierOrKeyword` variant of `TokenType` enum.

### Changed

- `TokenType::TextLiteral` to contain a `TextLiteralKind`.
- Handling of unterminated comments and text literals.
- The intermediate type of tokens between lexing and line parsing from `Token` to `RawToken`.
  This is to allow the logical line parser to consolidate the token types while processing.
  Additionally, formatters and consolidators operating after parsing do not need to worry about
  any identifier/keyword ambiguities.
- Logical line parsing:
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

### Fixed

- Incorrect parsing for generic type param lists containing semicolons.
- Extra trailing newline when formatting stdin to stdout.
- Lexical edge cases:
  - Codepoints in `[U+00, U+20)` and `U+3000` are now lexed as whitespace instead of `Unknown`.
  - Non-ascii codepoints (excluding `U+3000`) are now lexed as identifiers instead of `Unknown`.
  - Hex and binary integer literals token types were reversed (unobservable with the current rules).
  - Binary literals now can contain underscore (e.g. `%_1`).
  - Asm labels now can contain `@` characters.
  - Asm integer literals are now supported (e.g. octal `076O`, hex `0FFH`/`$FF`, binary `010B`).
  - Keywords used in qualified names are now lexed as identifiers (e.g. `System.String`).
  - Integer literals can now be escaped with multiple ampersands (e.g. `&&0`).
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
- Trailing whitespace removal. This is a built-in feature.
- `rules` module containing dedicated formatters
  - `UsesClauseFormatter`
  - `RemoveRepeatedNewlines`
  - `EofNewline`
  - `FormattingToggler`
  - `TokenSpacing`
