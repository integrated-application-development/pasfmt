#![forbid(unsafe_code)]

pub mod command_line;
pub mod file_formatter;
pub mod formatting_orchestrator;

pub mod predule {
    pub use crate::command_line::{pasfmt_config, *};
    pub use crate::file_formatter::*;
    pub use crate::formatting_orchestrator::*;
}
