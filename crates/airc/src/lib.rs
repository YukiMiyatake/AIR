//! AIR toolchain library (parse / check / run).

pub mod check;
pub mod diag;
pub mod interp;
pub mod parse;

pub use check::typecheck_module;
pub use diag::{emit_diags, Diagnostic, DiagMode};
pub use interp::{print_value, run_module, value_to_exit_code, AirValue};

pub use parse::{parse_module_json, Module};
