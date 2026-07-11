//! AIR toolchain library (parse / check / run).

pub mod check;
pub mod diag;
pub mod interp;
pub mod parse;
pub mod sexpr;

pub use check::typecheck_module;
pub use diag::{emit_diags, Diagnostic, DiagMode};
pub use interp::{print_value, run_module, value_to_exit_code, with_stdout_capture, AirValue};
pub use parse::{parse_module_json, parse_module_value, Module};
pub use sexpr::{normalize_lit_digits, parse_sexpr_value, print_sexpr};

/// Load a module from `.air.json` (JSON) or `.air` (S-expr).
pub fn parse_module_file(path: &str, text: &str) -> Result<Module, Vec<Diagnostic>> {
    if path.ends_with(".air.json") {
        return parse_module_json(text);
    }
    if path.ends_with(".air") {
        let mut v = parse_sexpr_value(text)?;
        normalize_lit_digits(&mut v);
        return parse_module_value(v);
    }
    match parse_module_json(text) {
        Ok(m) => Ok(m),
        Err(_) => {
            let mut v = parse_sexpr_value(text)?;
            normalize_lit_digits(&mut v);
            parse_module_value(v)
        }
    }
}
