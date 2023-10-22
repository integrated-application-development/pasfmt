pub mod eof_newline;
pub mod formatting_toggle;
pub mod generics_consolidator;
pub mod ignore_non_unit_import_clauses;
pub mod import_clause_consolidator;
pub mod remove_repeated_newlines;
pub mod token_spacing;
pub mod uses_clause_formatter;

pub use eof_newline::*;
pub use formatting_toggle::*;
pub use generics_consolidator::*;
pub use ignore_non_unit_import_clauses::*;
pub use import_clause_consolidator::*;
pub use remove_repeated_newlines::*;
pub use token_spacing::*;
pub use uses_clause_formatter::*;
