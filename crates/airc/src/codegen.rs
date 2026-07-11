//! Phase 2 Cranelift codegen — `sum`-class i32 / control-flow subset.
//!
//! See docs/CODEGEN.md. Interpreter (`airc run`) remains the general execution path;
//! `airc compile` JIT-runs parameterless `main` when present.

use crate::diag::{err, tag, Diagnostic};
use crate::parse::{fns_in_module, Module};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{types, AbiParam, BlockArg, InstBuilder, UserFuncName};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::verifier::verify_function;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Linkage, Module as ClifModule};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Result of a successful `compile` (native / JIT path).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileOutcome {
    /// Value returned by JIT-calling parameterless `main`, if the module defines one.
    pub main: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AirTy {
    I32,
    Bool,
}

impl AirTy {
    fn clif(self) -> types::Type {
        match self {
            AirTy::I32 => types::I32,
            AirTy::Bool => types::I8,
        }
    }
}

#[derive(Clone, Copy)]
struct Local {
    var: Variable,
    ty: AirTy,
}

struct LoopCtx {
    exit: cranelift_codegen::ir::Block,
}

enum Lowered {
    Value(cranelift_codegen::ir::Value, AirTy),
    Unreachable,
}

/// After a successful typecheck, lower supported functions to Cranelift IR,
/// JIT-compile them, and call parameterless `main` when present.
pub fn compile_module(module: &Module) -> Result<CompileOutcome, Vec<Diagnostic>> {
    let isa = host_isa()?;
    let mut jit = JITModule::new(JITBuilder::with_isa(isa, default_libcall_names()));
    let fns = fns_in_module(module);
    if fns.is_empty() {
        return Err(vec![err(
            "codegen.unsupported",
            "module has no functions to compile",
        )]);
    }

    let mut fb_ctx = FunctionBuilderContext::new();
    let mut ctx = jit.make_context();
    let mut main_id = None;

    for f in &fns {
        let arr = f.as_array().unwrap();
        let name = arr[1].as_str().unwrap();
        let params = arr[2].as_array().unwrap();
        let ret_ty = parse_simple_ty(&arr[3])?;
        if ret_ty != AirTy::I32 {
            return Err(vec![err(
                "codegen.unsupported",
                format!("fn `{name}`: only i32 return is supported in Cranelift MVP"),
            )]);
        }

        let mut sig = jit.make_signature();
        for p in params {
            let pa = p.as_array().ok_or_else(|| {
                vec![err(
                    "codegen.unsupported",
                    format!("fn `{name}`: bad param"),
                )]
            })?;
            let pty = parse_simple_ty(&pa[1])?;
            if pty != AirTy::I32 {
                return Err(vec![err(
                    "codegen.unsupported",
                    format!("fn `{name}`: only i32 params in Cranelift MVP"),
                )]);
            }
            sig.params.push(AbiParam::new(types::I32));
        }
        sig.returns.push(AbiParam::new(types::I32));

        let id = jit
            .declare_function(name, Linkage::Export, &sig)
            .map_err(|e| vec![err("codegen.error", format!("declare `{name}`: {e}"))])?;
        if name == "main" && params.is_empty() {
            main_id = Some(id);
        }

        ctx.func.signature = sig;
        ctx.func.name = UserFuncName::user(0, id.as_u32());
        lower_fn_body(&mut ctx.func, &mut fb_ctx, name, params, &arr[4])?;

        verify_function(&ctx.func, jit.isa()).map_err(|e| {
            vec![err(
                "codegen.error",
                format!("verify `{name}`: {e}"),
            )]
        })?;

        jit.define_function(id, &mut ctx)
            .map_err(|e| vec![err("codegen.error", format!("define `{name}`: {e}"))])?;
        jit.clear_context(&mut ctx);
    }

    jit.finalize_definitions()
        .map_err(|e| vec![err("codegen.error", format!("finalize: {e}"))])?;

    let main = if let Some(id) = main_id {
        let ptr = jit.get_finalized_function(id);
        // SAFETY: signature is `() -> i32`, matching declare/define above; ptr is live
        // for the lifetime of `jit`, which outlives this call.
        let f: extern "C" fn() -> i32 = unsafe { std::mem::transmute(ptr) };
        Some(f())
    } else {
        None
    };

    Ok(CompileOutcome { main })
}

fn host_isa() -> Result<Arc<dyn cranelift_codegen::isa::TargetIsa>, Vec<Diagnostic>> {
    let mut flag_builder = settings::builder();
    flag_builder
        .set("is_pic", "false")
        .map_err(|e| vec![err("codegen.error", format!("cranelift flags: {e}"))])?;
    flag_builder
        .set("use_colocated_libcalls", "false")
        .map_err(|e| vec![err("codegen.error", format!("cranelift flags: {e}"))])?;
    let flags = settings::Flags::new(flag_builder);
    let isa_builder = cranelift_native::builder().map_err(|e| {
        vec![err(
            "codegen.error",
            format!("host ISA unavailable: {e}"),
        )]
    })?;
    isa_builder
        .finish(flags)
        .map_err(|e| vec![err("codegen.error", format!("ISA finish: {e}"))])
}

fn lower_fn_body(
    func: &mut cranelift_codegen::ir::Function,
    fb_ctx: &mut FunctionBuilderContext,
    name: &str,
    params: &[Value],
    body: &Value,
) -> Result<(), Vec<Diagnostic>> {
    let mut builder = FunctionBuilder::new(func, fb_ctx);
    let entry = builder.create_block();
    builder.append_block_params_for_function_params(entry);
    builder.switch_to_block(entry);
    builder.seal_block(entry);

    let mut env = HashMap::new();
    let block_params: Vec<_> = builder.block_params(entry).to_vec();
    for (i, p) in params.iter().enumerate() {
        let pname = p.as_array().unwrap()[0].as_str().unwrap().to_string();
        let var = builder.declare_var(types::I32);
        builder.def_var(var, block_params[i]);
        env.insert(
            pname,
            Local {
                var,
                ty: AirTy::I32,
            },
        );
    }

    match lower_expr(&mut builder, &mut env, body, None)? {
        Lowered::Value(v, ty) => {
            if ty != AirTy::I32 {
                return Err(vec![err(
                    "codegen.unsupported",
                    format!("fn `{name}`: body must yield i32"),
                )]);
            }
            builder.ins().return_(&[v]);
        }
        Lowered::Unreachable => {
            return Err(vec![err(
                "codegen.unsupported",
                format!("fn `{name}`: body does not return"),
            )]);
        }
    }
    builder.finalize();
    Ok(())
}

fn parse_simple_ty(v: &Value) -> Result<AirTy, Vec<Diagnostic>> {
    match v.as_str() {
        Some("i32") => Ok(AirTy::I32),
        Some("bool") => Ok(AirTy::Bool),
        _ => Err(vec![err(
            "codegen.unsupported",
            format!("unsupported type in Cranelift MVP: {v}"),
        )]),
    }
}

fn fork_env(
    builder: &mut FunctionBuilder<'_>,
    parent: &HashMap<String, Local>,
) -> HashMap<String, Local> {
    let mut child = HashMap::new();
    for (name, loc) in parent {
        let var = builder.declare_var(loc.ty.clif());
        let val = builder.use_var(loc.var);
        builder.def_var(var, val);
        child.insert(
            name.clone(),
            Local {
                var,
                ty: loc.ty,
            },
        );
    }
    child
}

fn lower_expr(
    builder: &mut FunctionBuilder<'_>,
    env: &mut HashMap<String, Local>,
    e: &Value,
    loop_ctx: Option<&LoopCtx>,
) -> Result<Lowered, Vec<Diagnostic>> {
    if let Some(b) = e.as_bool() {
        let v = builder.ins().iconst(types::I8, if b { 1 } else { 0 });
        return Ok(Lowered::Value(v, AirTy::Bool));
    }
    if let Some(n) = e.as_i64() {
        let v = builder.ins().iconst(types::I32, n);
        return Ok(Lowered::Value(v, AirTy::I32));
    }

    let (t, rest) = tag(e).ok_or_else(|| {
        vec![err(
            "codegen.unsupported",
            format!("unsupported expr in Cranelift MVP: {e}"),
        )]
    })?;

    match t {
        "lit" => {
            let ty = rest[0].as_str().unwrap();
            let raw = rest[1].as_str().unwrap();
            match ty {
                "i32" => {
                    let n: i32 = raw.parse().map_err(|_| {
                        vec![err("codegen.error", format!("bad i32 lit: {raw}"))]
                    })?;
                    let v = builder.ins().iconst(types::I32, i64::from(n));
                    Ok(Lowered::Value(v, AirTy::I32))
                }
                "bool" => {
                    let b = raw == "true";
                    let v = builder.ins().iconst(types::I8, if b { 1 } else { 0 });
                    Ok(Lowered::Value(v, AirTy::Bool))
                }
                other => Err(vec![err(
                    "codegen.unsupported",
                    format!("lit type `{other}` not in Cranelift MVP"),
                )]),
            }
        }
        "var" => {
            let name = rest[0].as_str().unwrap();
            let loc = *env.get(name).ok_or_else(|| {
                vec![err(
                    "codegen.error",
                    format!("unbound var `{name}` during codegen"),
                )]
            })?;
            let v = builder.use_var(loc.var);
            Ok(Lowered::Value(v, loc.ty))
        }
        "seq" => {
            let mut last = Lowered::Value(builder.ins().iconst(types::I32, 0), AirTy::I32);
            for x in rest {
                match lower_expr(builder, env, x, loop_ctx)? {
                    u @ Lowered::Unreachable => return Ok(u),
                    v => last = v,
                }
            }
            Ok(last)
        }
        "let" => {
            let mut child = fork_env(builder, env);
            for b in rest[0].as_array().unwrap() {
                let ba = b.as_array().unwrap();
                let name = ba[0].as_str().unwrap().to_string();
                let init = if ba.len() == 2 { &ba[1] } else { &ba[2] };
                let (val, ty) = match lower_expr(builder, &mut child, init, loop_ctx)? {
                    Lowered::Value(v, t) => (v, t),
                    Lowered::Unreachable => return Ok(Lowered::Unreachable),
                };
                let var = builder.declare_var(ty.clif());
                builder.def_var(var, val);
                child.insert(name, Local { var, ty });
            }
            lower_expr(builder, &mut child, &rest[1], loop_ctx)
        }
        "set!" => {
            let name = rest[0].as_str().unwrap();
            let (val, ty) = match lower_expr(builder, env, &rest[1], loop_ctx)? {
                Lowered::Value(v, t) => (v, t),
                Lowered::Unreachable => return Ok(Lowered::Unreachable),
            };
            let loc = env.get(name).copied().ok_or_else(|| {
                vec![err(
                    "codegen.error",
                    format!("set! unbound `{name}`"),
                )]
            })?;
            if loc.ty != ty {
                return Err(vec![err(
                    "codegen.error",
                    format!("set! type mismatch for `{name}`"),
                )]);
            }
            builder.def_var(loc.var, val);
            Ok(Lowered::Value(val, ty))
        }
        "if" => lower_if(builder, env, &rest[0], &rest[1], &rest[2], loop_ctx),
        "loop" => lower_loop(builder, env, &rest[0]),
        "break" => {
            let ctx = loop_ctx.ok_or_else(|| {
                vec![err("codegen.unsupported", "break outside loop")]
            })?;
            let (val, ty) = match lower_expr(builder, env, &rest[0], loop_ctx)? {
                Lowered::Value(v, t) => (v, t),
                Lowered::Unreachable => return Ok(Lowered::Unreachable),
            };
            if ty != AirTy::I32 {
                return Err(vec![err(
                    "codegen.unsupported",
                    "break value must be i32 in Cranelift MVP",
                )]);
            }
            builder.ins().jump(ctx.exit, &[BlockArg::from(val)]);
            Ok(Lowered::Unreachable)
        }
        "return" => {
            let (val, ty) = match lower_expr(builder, env, &rest[0], loop_ctx)? {
                Lowered::Value(v, t) => (v, t),
                Lowered::Unreachable => return Ok(Lowered::Unreachable),
            };
            if ty != AirTy::I32 {
                return Err(vec![err(
                    "codegen.unsupported",
                    "return must be i32 in Cranelift MVP",
                )]);
            }
            builder.ins().return_(&[val]);
            Ok(Lowered::Unreachable)
        }
        "call" => lower_call(builder, env, rest, loop_ctx),
        other => Err(vec![err(
            "codegen.unsupported",
            format!("expr `{other}` not in Cranelift MVP (sum-class subset)"),
        )]),
    }
}

fn lower_if(
    builder: &mut FunctionBuilder<'_>,
    env: &mut HashMap<String, Local>,
    cond_e: &Value,
    then_e: &Value,
    else_e: &Value,
    loop_ctx: Option<&LoopCtx>,
) -> Result<Lowered, Vec<Diagnostic>> {
    let (cond, cty) = match lower_expr(builder, env, cond_e, loop_ctx)? {
        Lowered::Value(v, t) => (v, t),
        Lowered::Unreachable => return Ok(Lowered::Unreachable),
    };
    if cty != AirTy::Bool {
        return Err(vec![err("codegen.error", "if condition must be bool")]);
    }

    let then_b = builder.create_block();
    let else_b = builder.create_block();
    let merge = builder.create_block();

    builder.ins().brif(cond, then_b, &[], else_b, &[]);

    builder.switch_to_block(then_b);
    builder.seal_block(then_b);
    let then_ty = match lower_expr(builder, env, then_e, loop_ctx)? {
        Lowered::Value(v, ty) => {
            ensure_merge_param(builder, merge, ty);
            builder.ins().jump(merge, &[BlockArg::from(v)]);
            Some(ty)
        }
        Lowered::Unreachable => None,
    };

    builder.switch_to_block(else_b);
    builder.seal_block(else_b);
    let else_ty = match lower_expr(builder, env, else_e, loop_ctx)? {
        Lowered::Value(v, ty) => {
            ensure_merge_param(builder, merge, ty);
            if let Some(tt) = then_ty {
                if tt != ty {
                    return Err(vec![err(
                        "codegen.error",
                        "if branches have different types",
                    )]);
                }
            }
            builder.ins().jump(merge, &[BlockArg::from(v)]);
            Some(ty)
        }
        Lowered::Unreachable => None,
    };

    let ty = match (then_ty, else_ty) {
        (None, None) => return Ok(Lowered::Unreachable),
        (Some(t), None) | (None, Some(t)) | (Some(t), Some(_)) => t,
    };

    builder.switch_to_block(merge);
    builder.seal_block(merge);
    let out = builder.block_params(merge)[0];
    Ok(Lowered::Value(out, ty))
}

fn ensure_merge_param(
    builder: &mut FunctionBuilder<'_>,
    merge: cranelift_codegen::ir::Block,
    ty: AirTy,
) {
    if builder.block_params(merge).is_empty() {
        builder.append_block_param(merge, ty.clif());
    }
}

fn lower_loop(
    builder: &mut FunctionBuilder<'_>,
    env: &mut HashMap<String, Local>,
    body: &Value,
) -> Result<Lowered, Vec<Diagnostic>> {
    let header = builder.create_block();
    let exit = builder.create_block();
    builder.append_block_param(exit, types::I32);

    builder.ins().jump(header, &[]);

    builder.switch_to_block(header);
    let ctx = LoopCtx { exit };
    match lower_expr(builder, env, body, Some(&ctx))? {
        Lowered::Value(..) => {
            builder.ins().jump(header, &[]);
        }
        Lowered::Unreachable => {}
    }
    builder.seal_block(header);

    builder.switch_to_block(exit);
    builder.seal_block(exit);
    let v = builder.block_params(exit)[0];
    Ok(Lowered::Value(v, AirTy::I32))
}

fn lower_call(
    builder: &mut FunctionBuilder<'_>,
    env: &mut HashMap<String, Local>,
    rest: &[Value],
    loop_ctx: Option<&LoopCtx>,
) -> Result<Lowered, Vec<Diagnostic>> {
    let callee = rest[0].as_str().unwrap();
    let mut args = Vec::new();
    for a in &rest[1..] {
        match lower_expr(builder, env, a, loop_ctx)? {
            Lowered::Value(v, t) => args.push((v, t)),
            Lowered::Unreachable => return Ok(Lowered::Unreachable),
        }
    }

    match callee {
        "+" | "-" | "*" => {
            if args.len() != 2 || args[0].1 != AirTy::I32 || args[1].1 != AirTy::I32 {
                return Err(vec![err(
                    "codegen.error",
                    format!("`{callee}` expects two i32"),
                )]);
            }
            let v = match callee {
                "+" => builder.ins().iadd(args[0].0, args[1].0),
                "-" => builder.ins().isub(args[0].0, args[1].0),
                "*" => builder.ins().imul(args[0].0, args[1].0),
                _ => unreachable!(),
            };
            Ok(Lowered::Value(v, AirTy::I32))
        }
        "/" => {
            if args.len() != 2 || args[0].1 != AirTy::I32 || args[1].1 != AirTy::I32 {
                return Err(vec![err("codegen.error", "`/` expects two i32")]);
            }
            let v = builder.ins().sdiv(args[0].0, args[1].0);
            Ok(Lowered::Value(v, AirTy::I32))
        }
        "<=" | "<" | ">" | ">=" | "==" | "!=" => {
            if args.len() != 2 || args[0].1 != AirTy::I32 || args[1].1 != AirTy::I32 {
                return Err(vec![err(
                    "codegen.error",
                    format!("`{callee}` expects two i32 in Cranelift MVP"),
                )]);
            }
            let cc = match callee {
                "<=" => IntCC::SignedLessThanOrEqual,
                "<" => IntCC::SignedLessThan,
                ">" => IntCC::SignedGreaterThan,
                ">=" => IntCC::SignedGreaterThanOrEqual,
                "==" => IntCC::Equal,
                "!=" => IntCC::NotEqual,
                _ => unreachable!(),
            };
            let v = builder.ins().icmp(cc, args[0].0, args[1].0);
            Ok(Lowered::Value(v, AirTy::Bool))
        }
        other => Err(vec![err(
            "codegen.unsupported",
            format!("call `{other}` not in Cranelift MVP (no user fns / caps yet)"),
        )]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::typecheck_module;
    use crate::parse_module_file;

    #[test]
    fn compile_sum_with_cranelift() {
        let text = std::fs::read_to_string("examples/sum.air")
            .or_else(|_| std::fs::read_to_string("../../examples/sum.air"))
            .expect("sum.air");
        let module = parse_module_file("examples/sum.air", &text).expect("parse");
        typecheck_module(&module).expect("check");
        let out = compile_module(&module).expect("compile");
        assert_eq!(out.main, Some(55));
    }

    #[test]
    fn compile_rejects_cap_print() {
        let text = std::fs::read_to_string("examples/hello.air")
            .or_else(|_| std::fs::read_to_string("../../examples/hello.air"))
            .expect("hello.air");
        let module = parse_module_file("examples/hello.air", &text).expect("parse");
        typecheck_module(&module).expect("check");
        let err = compile_module(&module).expect_err("cap.print unsupported");
        assert!(
            err.iter().any(|d| d.code == "codegen.unsupported"),
            "{err:?}"
        );
    }
}
