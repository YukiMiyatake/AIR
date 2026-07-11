//! AIR reference CLI (Rust) — Phase 1.5 parity with tools/airc for check/run.

use airc::{
    ast_digest_hex, ast_eq, emit_diags, parse_module_file, print_sexpr, print_value, run_module,
    typecheck_module, value_to_exit_code,
};
use std::env;
use std::fs;
use std::process::ExitCode;

fn usage() -> &'static str {
    "airc — AIR toolchain (Rust)

Usage:
  airc version
  airc fmt   <file.air|.air.json>           # print canonical S-expr
  airc hash  <file.air|.air.json>           # SHA-256 of structural AST
  airc eq    <fileA> <fileB>                # exit 0 if same AST
  airc check <file.air|.air.json> [--diag=text|json]
  airc run   <file.air|.air.json> [--diag=text|json]
"
}

struct Cli {
    cmd: String,
    files: Vec<String>,
    diag: String,
}

fn parse_cli(args: &[String]) -> Result<Cli, String> {
    let mut cmd = String::new();
    let mut files = Vec::new();
    let mut diag = "text".to_string();
    for a in args {
        if a == "-h" || a == "--help" {
            return Ok(Cli {
                cmd: "help".into(),
                files,
                diag,
            });
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
        files.push(a.clone());
    }
    Ok(Cli { cmd, files, diag })
}

fn load_module(path: &str, diag: &str) -> Result<airc::Module, ExitCode> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{path}: {e}");
            return Err(ExitCode::from(1));
        }
    };
    match parse_module_file(path, &text) {
        Ok(m) => Ok(m),
        Err(diags) => {
            emit_diags(&diags, diag, path);
            Err(ExitCode::from(1))
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let cli = match parse_cli(&args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            eprint!("{}", usage());
            return ExitCode::from(2);
        }
    };

    if cli.cmd.is_empty() {
        eprint!("{}", usage());
        return ExitCode::from(2);
    }
    if cli.cmd == "help" {
        print!("{}", usage());
        return ExitCode::SUCCESS;
    }
    if cli.cmd == "version" {
        println!("airc {} (rust)", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    match cli.cmd.as_str() {
        "fmt" | "hash" | "check" | "run" => {
            if cli.files.len() != 1 {
                eprintln!("`{}` needs exactly one file", cli.cmd);
                eprint!("{}", usage());
                return ExitCode::from(2);
            }
        }
        "eq" => {
            if cli.files.len() != 2 {
                eprintln!("`eq` needs exactly two files");
                eprint!("{}", usage());
                return ExitCode::from(2);
            }
        }
        _ => {
            eprintln!("unknown command: {}", cli.cmd);
            eprint!("{}", usage());
            return ExitCode::from(2);
        }
    }

    if cli.cmd == "eq" {
        let a = match load_module(&cli.files[0], &cli.diag) {
            Ok(m) => m,
            Err(c) => return c,
        };
        let b = match load_module(&cli.files[1], &cli.diag) {
            Ok(m) => m,
            Err(c) => return c,
        };
        if ast_eq(&a.raw, &b.raw) {
            println!("equal");
            return ExitCode::SUCCESS;
        }
        println!("not equal");
        return ExitCode::from(1);
    }

    let path = &cli.files[0];
    let module = match load_module(path, &cli.diag) {
        Ok(m) => m,
        Err(c) => return c,
    };

    if cli.cmd == "fmt" {
        print!("{}", print_sexpr(&module.raw));
        return ExitCode::SUCCESS;
    }
    if cli.cmd == "hash" {
        println!("{}", ast_digest_hex(&module.raw));
        return ExitCode::SUCCESS;
    }

    if let Err(diags) = typecheck_module(&module) {
        emit_diags(&diags, &cli.diag, path);
        return ExitCode::from(1);
    }

    if cli.cmd == "check" {
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
                &cli.diag,
                path,
            );
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use airc::{
        ast_digest_hex, parse_module_file, parse_module_json, run_module, typecheck_module,
        with_stdout_capture, AirValue,
    };

    fn load(path: &str) -> String {
        std::fs::read_to_string(path)
            .or_else(|_| std::fs::read_to_string(format!("../../{path}")))
            .unwrap_or_else(|_| panic!("missing {path}"))
    }

    fn run_i32(path: &str) -> i32 {
        let module = parse_module_file(path, &load(path)).expect("parse");
        typecheck_module(&module).expect("check");
        match run_module(&module).expect("run") {
            AirValue::I32(n) => n,
            other => panic!("expected i32, got {other:?}"),
        }
    }

    #[test]
    fn sum_example_is_55() {
        assert_eq!(run_i32("examples/sum.air"), 55);
    }

    #[test]
    fn div_by_zero_match_is_minus_one() {
        assert_eq!(run_i32("examples/div.air"), -1);
    }

    #[test]
    fn array_sum_is_10() {
        assert_eq!(run_i32("examples/arr.air"), 10);
    }

    #[test]
    fn hello_returns_zero() {
        assert_eq!(run_i32("examples/hello.air"), 0);
    }

    #[test]
    fn hello_prints_to_stdout() {
        let module =
            parse_module_file("examples/hello.air", &load("examples/hello.air")).expect("parse");
        typecheck_module(&module).expect("check");
        let (v, lines) = with_stdout_capture(|| run_module(&module).expect("run"));
        assert_eq!(v, AirValue::I32(0));
        assert_eq!(lines, vec!["hello".to_string()]);
    }

    #[test]
    fn bad_move_is_rejected() {
        let module =
            parse_module_file("examples/bad_move.air", &load("examples/bad_move.air")).expect("parse");
        let err = typecheck_module(&module).expect_err("should fail ownership");
        assert!(
            err.iter().any(|d| d.code == "mem.use_after_move"),
            "expected mem.use_after_move, got {err:?}"
        );
    }

    #[test]
    fn checked_add_overflow_is_minus_one() {
        assert_eq!(run_i32("examples/overflow.air"), -1);
    }

    #[test]
    fn checked_add_ok_path() {
        let src = r#"[
          "mod", "t",
          ["fn", "main", [], "i32",
            ["match",
              ["call", "checked_add", ["lit", "i32", "20"], ["lit", "i32", "22"]],
              [["ok", "v"], ["var", "v"]],
              [["err", "e"], ["lit", "i32", "-1"]]]]]
        "#;
        let module = parse_module_json(src).expect("parse");
        typecheck_module(&module).expect("check");
        match run_module(&module).expect("run") {
            AirValue::I32(n) => assert_eq!(n, 42),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn aset_then_aget_is_nine() {
        assert_eq!(run_i32("examples/aset.air"), 9);
    }

    #[test]
    fn bad_borrow_is_rejected() {
        let module =
            parse_module_file("examples/bad_borrow.air", &load("examples/bad_borrow.air"))
                .expect("parse");
        let err = typecheck_module(&module).expect_err("should fail borrow");
        assert!(
            err.iter().any(|d| d.code == "mem.borrow_conflict"),
            "expected mem.borrow_conflict, got {err:?}"
        );
    }

    #[test]
    fn borrow_ok_returns_seven() {
        assert_eq!(run_i32("examples/borrow_ok.air"), 7);
    }

    #[test]
    fn air_matches_json_ast_for_suite() {
        use airc::{normalize_lit_digits, parse_sexpr_value, print_sexpr};
        let names = [
            "sum", "div", "arr", "hello", "overflow", "aset", "borrow_ok", "bad_move", "bad_borrow",
        ];
        for name in names {
            let json_path = format!("examples/{name}.air.json");
            let air_path = format!("examples/{name}.air");
            let json_mod = parse_module_file(&json_path, &load(&json_path)).expect("json");
            let air_mod = parse_module_file(&air_path, &load(&air_path)).expect("air");
            assert_eq!(
                json_mod.raw, air_mod.raw,
                "{name}: .air AST != .air.json AST"
            );
            let printed = print_sexpr(&json_mod.raw);
            let mut back = parse_sexpr_value(&printed).expect("reparse");
            normalize_lit_digits(&mut back);
            assert_eq!(json_mod.raw, back, "{name}: fmt round-trip");
            assert_eq!(
                ast_digest_hex(&json_mod.raw),
                ast_digest_hex(&air_mod.raw),
                "{name}: hash mismatch"
            );
        }
    }
}
