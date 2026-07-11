//! AIR reference CLI (Rust) — Phase 1.5 parity with tools/airc for check/run.

use airc::{
    emit_diags, parse_module_json, print_value, run_module, typecheck_module, value_to_exit_code,
};
use std::env;
use std::fs;
use std::process::ExitCode;

fn usage() -> &'static str {
    "airc — AIR toolchain (Rust)

Usage:
  airc version
  airc check <file.air.json> [--diag=text|json]
  airc run   <file.air.json> [--diag=text|json]
"
}

fn parse_cli(args: &[String]) -> Result<(String, Option<String>, String), String> {
    let mut cmd = String::new();
    let mut file: Option<String> = None;
    let mut diag = "text".to_string();
    for a in args {
        if a == "-h" || a == "--help" {
            return Ok(("help".into(), None, diag));
        }
        if let Some(v) = a.strip_prefix("--diag=") {
            if v != "text" && v != "json" {
                return Err(format!("invalid --diag value: {v}"));
            }
            diag = v.to_string();
            continue;
        }
        if cmd.is_empty() {
            cmd = a.clone();
            continue;
        }
        if file.is_none() {
            file = Some(a.clone());
            continue;
        }
        return Err(format!("unexpected argument: {a}"));
    }
    Ok((cmd, file, diag))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let (cmd, file, diag) = match parse_cli(&args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            eprint!("{}", usage());
            return ExitCode::from(2);
        }
    };

    if cmd.is_empty() {
        eprint!("{}", usage());
        return ExitCode::from(2);
    }
    if cmd == "help" {
        print!("{}", usage());
        return ExitCode::SUCCESS;
    }
    if cmd == "version" {
        println!("airc {} (rust)", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    if cmd != "check" && cmd != "run" {
        eprintln!("unknown command: {cmd}");
        eprint!("{}", usage());
        return ExitCode::from(2);
    }
    let Some(path) = file else {
        eprintln!("missing file for `{cmd}`");
        eprint!("{}", usage());
        return ExitCode::from(2);
    };

    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::from(1);
        }
    };

    let module = match parse_module_json(&text) {
        Ok(m) => m,
        Err(diags) => {
            emit_diags(&diags, &diag, &path);
            return ExitCode::from(1);
        }
    };

    if let Err(diags) = typecheck_module(&module) {
        emit_diags(&diags, &diag, &path);
        return ExitCode::from(1);
    }

    if cmd == "check" {
        println!("ok: checked module {}", module.name);
        return ExitCode::SUCCESS;
    }

    match run_module(&module) {
        Ok(v) => {
            print_value(&v);
            ExitCode::from(value_to_exit_code(&v))
        }
        Err(e) => {
            emit_diags(
                &[airc::diag::err("runtime.abort", e.to_string())],
                &diag,
                &path,
            );
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use airc::{parse_module_json, run_module, typecheck_module, AirValue};

    fn load(path: &str) -> String {
        std::fs::read_to_string(path)
            .or_else(|_| std::fs::read_to_string(format!("../../{path}")))
            .unwrap_or_else(|_| panic!("missing {path}"))
    }

    fn run_i32(path: &str) -> i32 {
        let module = parse_module_json(&load(path)).expect("parse");
        typecheck_module(&module).expect("check");
        match run_module(&module).expect("run") {
            AirValue::I32(n) => n,
            other => panic!("expected i32, got {other:?}"),
        }
    }

    #[test]
    fn sum_example_is_55() {
        assert_eq!(run_i32("examples/sum.air.json"), 55);
    }

    #[test]
    fn div_by_zero_match_is_minus_one() {
        assert_eq!(run_i32("examples/div.air.json"), -1);
    }

    #[test]
    fn array_sum_is_10() {
        assert_eq!(run_i32("examples/arr.air.json"), 10);
    }

    #[test]
    fn hello_returns_zero() {
        assert_eq!(run_i32("examples/hello.air.json"), 0);
    }

    #[test]
    fn bad_move_is_rejected() {
        let module = parse_module_json(&load("examples/bad_move.air.json")).expect("parse");
        let err = typecheck_module(&module).expect_err("should fail ownership");
        assert!(
            err.iter().any(|d| d.code == "mem.use_after_move"),
            "expected mem.use_after_move, got {err:?}"
        );
    }
}
