//! AIR reference CLI (Rust). Phase 1.5 scaffold — parity with `tools/airc` TBD.

use std::env;
use std::process::ExitCode;

fn usage() -> &'static str {
    "airc — AIR toolchain (Rust)

Usage:
  airc version
  airc check <file.air.json>   (TODO: parity with tools/airc)
  airc run   <file.air.json>   (TODO: parity with tools/airc)
"
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        None => {
            eprint!("{}", usage());
            ExitCode::from(2)
        }
        Some("-h") | Some("--help") => {
            print!("{}", usage());
            ExitCode::SUCCESS
        }
        Some("version") => {
            println!("airc {} (rust-scaffold)", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("check") | Some("run") => {
            eprintln!(
                "not implemented yet in Rust airc; use TypeScript tools/airc or wait for Phase 1.5 parity"
            );
            ExitCode::from(1)
        }
        Some(other) => {
            eprintln!("unknown command: {other}");
            eprint!("{}", usage());
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_constant_nonempty() {
        assert!(!env!("CARGO_PKG_VERSION").is_empty());
    }
}
