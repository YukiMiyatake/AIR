//! Phase 2 native codegen stub (Cranelift planned — see docs/CODEGEN.md).

use crate::diag::{err, Diagnostic};
use crate::parse::Module;

/// After a successful typecheck, attempt native lowering.
/// v0: always reports `codegen.unimplemented` (design sketch only).
pub fn compile_module(_module: &Module) -> Result<(), Vec<Diagnostic>> {
    Err(vec![err(
        "codegen.unimplemented",
        "native codegen not implemented yet; see docs/CODEGEN.md (Cranelift MVP). Use `airc run` for the interpreter.",
    )])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::typecheck_module;
    use crate::parse_module_file;

    #[test]
    fn compile_stub_after_check_on_sum() {
        let text = std::fs::read_to_string("examples/sum.air")
            .or_else(|_| std::fs::read_to_string("../../examples/sum.air"))
            .expect("sum.air");
        let module = parse_module_file("examples/sum.air", &text).expect("parse");
        typecheck_module(&module).expect("check");
        let err = compile_module(&module).expect_err("stub");
        assert!(
            err.iter().any(|d| d.code == "codegen.unimplemented"),
            "{err:?}"
        );
    }
}
