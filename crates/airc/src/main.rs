//! AIR primary CLI (Rust) — Phase 1.5+ toolchain (`tools/airc` is the TS oracle).

use airc::{
    ast_digest_hex, ast_eq, compile_module, emit_diags, load_module_path, pack_airb, print_sexpr,
    print_value, run_module, typecheck_module, unpack_airb, value_to_exit_code,
};
use std::env;
use std::fs;
use std::process::ExitCode;

fn usage() -> &'static str {
    "airc — AIR toolchain (Rust)

Usage:
  airc version
  airc fmt     <file.air|.airb>    # print canonical S-expr (.air.json legacy)
  airc hash    <file.air|.airb>    # SHA-256 of structural AST
  airc eq      <fileA> <fileB>     # exit 0 if same AST
  airc pack    <file.air> <out.airb>
  airc unpack  <file.airb>         # print S-expr
  airc check   <file.air|.airb> [--diag=text|json]
  airc run     <file.air|.airb> [--diag=text|json]
  airc compile <file.air|.airb> [--diag=text|json]  # Phase 2 Cranelift JIT (sum-class)

Default text encoding is .air (S-expr). .airb is accepted for check/run/fmt/hash/eq/compile.
.air.json remains accepted for legacy parity.
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
    match load_module_path(path) {
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
        "fmt" | "hash" | "check" | "run" | "unpack" | "compile" => {
            if cli.files.len() != 1 {
                eprintln!("`{}` needs exactly one file", cli.cmd);
                eprint!("{}", usage());
                return ExitCode::from(2);
            }
        }
        "eq" | "pack" => {
            if cli.files.len() != 2 {
                eprintln!("`{}` needs exactly two files", cli.cmd);
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

    if cli.cmd == "unpack" {
        let path = &cli.files[0];
        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("{path}: {e}");
                return ExitCode::from(1);
            }
        };
        let v = match unpack_airb(&bytes) {
            Ok(v) => v,
            Err(diags) => {
                emit_diags(&diags, &cli.diag, path);
                return ExitCode::from(1);
            }
        };
        print!("{}", print_sexpr(&v));
        return ExitCode::SUCCESS;
    }

    if cli.cmd == "pack" {
        let module = match load_module(&cli.files[0], &cli.diag) {
            Ok(m) => m,
            Err(c) => return c,
        };
        let bytes = match pack_airb(&module.raw) {
            Ok(b) => b,
            Err(diags) => {
                emit_diags(&diags, &cli.diag, &cli.files[0]);
                return ExitCode::from(1);
            }
        };
        if let Err(e) = fs::write(&cli.files[1], &bytes) {
            eprintln!("{}: {e}", cli.files[1]);
            return ExitCode::from(1);
        }
        println!("wrote {} ({} bytes)", cli.files[1], bytes.len());
        return ExitCode::SUCCESS;
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

    if cli.cmd == "compile" {
        match compile_module(&module) {
            Ok(out) => {
                match out.main {
                    Some(v) => println!("ok: compiled module {} (jit main => {v})", module.name),
                    None => println!("ok: compiled module {}", module.name),
                }
                ExitCode::SUCCESS
            }
            Err(diags) => {
                emit_diags(&diags, &cli.diag, path);
                ExitCode::from(1)
            }
        }
    } else {
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
}

#[cfg(test)]
mod tests {
    use airc::{
        ast_digest_hex, load_module_path, pack_airb, parse_module_airb, parse_module_file,
        parse_module_json, run_module, typecheck_module, with_stdout_capture, AirValue,
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
    fn point_struct_field_sum_is_seven() {
        assert_eq!(run_i32("examples/point.air"), 7);
    }

    #[test]
    fn option_variant_match_is_forty_two() {
        assert_eq!(run_i32("examples/option.air"), 42);
    }

    #[test]
    fn pair_tuple_variant_sum_is_seven() {
        assert_eq!(run_i32("examples/pair.air"), 7);
    }

    #[test]
    fn fset_then_field_is_ten() {
        assert_eq!(run_i32("examples/fset.air"), 10);
    }

    #[test]
    fn packed_airb_sum_checks_and_runs() {
        let module =
            parse_module_file("examples/sum.air", &load("examples/sum.air")).expect("parse");
        typecheck_module(&module).expect("check .air");
        let bytes = pack_airb(&module.raw).expect("pack");
        let from_bytes = parse_module_airb(&bytes).expect("parse airb bytes");
        assert_eq!(module.raw, from_bytes.raw);
        typecheck_module(&from_bytes).expect("check airb");
        match run_module(&from_bytes).expect("run airb") {
            AirValue::I32(n) => assert_eq!(n, 55),
            other => panic!("{other:?}"),
        }
        let dir = std::env::temp_dir();
        let path = dir.join("air_sum_test.airb");
        std::fs::write(&path, &bytes).expect("write airb");
        let loaded = load_module_path(path.to_str().unwrap()).expect("load path");
        assert_eq!(module.raw, loaded.raw);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn bad_enum_match_is_rejected() {
        let module = parse_module_file(
            "examples/bad_enum_match.air",
            &load("examples/bad_enum_match.air"),
        )
        .expect("parse");
        let err = typecheck_module(&module).expect_err("should fail exhaustiveness");
        assert!(
            err.iter().any(|d| d.code == "type.match"),
            "expected type.match, got {err:?}"
        );
    }

    #[test]
    fn air_matches_json_ast_for_suite() {
        use airc::{normalize_lit_digits, pack_airb, parse_sexpr_value, print_sexpr, unpack_airb};
        let names = [
            "sum", "div", "arr", "hello", "overflow", "aset", "borrow_ok", "bad_move", "bad_borrow",
            "point", "option", "bad_enum_match", "pair", "fset",
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
            let packed = pack_airb(&air_mod.raw).expect("pack");
            let unpacked = unpack_airb(&packed).expect("unpack");
            assert_eq!(air_mod.raw, unpacked, "{name}: airb round-trip");
        }
    }
}
