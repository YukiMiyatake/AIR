//! AIR toolchain library (parse / check / run).

pub mod airb;
pub mod check;
pub mod codegen;
pub mod diag;
pub mod hash;
pub mod interp;
pub mod parse;
pub mod sexpr;

pub use airb::{pack_airb, unpack_airb, KNOWN_TAGS};
pub use check::typecheck_module;
pub use codegen::{compile_module, CompileOptions, CompileOutcome, CompileOutputKind};
pub use diag::{emit_diags, Diagnostic, DiagMode};
pub use hash::{ast_digest_hex, ast_eq};
pub use interp::{print_value, run_module, value_to_exit_code, with_stdout_capture, AirValue};
pub use parse::{parse_module_json, parse_module_value, Module};
pub use sexpr::{normalize_lit_digits, parse_sexpr_value, print_sexpr};

/// Load a module from `.airb` bytes (unpack → air-format AST).
pub fn parse_module_airb(bytes: &[u8]) -> Result<Module, Vec<Diagnostic>> {
    let v = unpack_airb(bytes)?;
    parse_module_value(v)
}

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

/// Load a module from a filesystem path (`.air`, `.air.json`, or `.airb`).
pub fn load_module_path(path: &str) -> Result<Module, Vec<Diagnostic>> {
    if path.ends_with(".airb") {
        let bytes = std::fs::read(path).map_err(|e| {
            vec![crate::diag::err(
                "parse.io",
                format!("failed to read {path}: {e}"),
            )]
        })?;
        return parse_module_airb(&bytes);
    }
    let text = std::fs::read_to_string(path).map_err(|e| {
        vec![crate::diag::err(
            "parse.io",
            format!("failed to read {path}: {e}"),
        )]
    })?;
    parse_module_file(path, &text)
}
