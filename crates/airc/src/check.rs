use crate::diag::{err, tag, Diagnostic};
use crate::parse::{find_fn, Module};
use serde_json::Value;
use std::collections::HashMap;

pub fn typecheck_module(module: &Module) -> Result<(), Vec<Diagnostic>> {
    let main = find_fn(module, "main").ok_or_else(|| vec![err("type.main", "missing main")])?;
    let mut diags = Vec::new();
    let mut fns = HashMap::new();
    for f in crate::parse::fns_in_module(module) {
        let name = f.as_array().unwrap()[1].as_str().unwrap().to_string();
        fns.insert(name, f);
    }
    for (name, f) in &fns {
        let arr = f.as_array().unwrap();
        let ret = &arr[3];
        let body = &arr[4];
        let mut env = HashMap::new();
        for p in arr[2].as_array().unwrap() {
            let pa = p.as_array().unwrap();
            env.insert(pa[0].as_str().unwrap().to_string(), pa[1].clone());
        }
        let mut break_ty: Option<Value> = None;
        match check_expr(body, &mut env, &fns, &mut break_ty, &mut diags) {
            Some(body_ty) => {
                if body_ty != Value::String("never".into()) && &body_ty != ret {
                    diags.push(err(
                        "type.mismatch",
                        format!("fn {name} body type mismatch"),
                    ));
                }
            }
            None => {}
        }
    }
    let _ = main;
    if diags.is_empty() {
        Ok(())
    } else {
        Err(diags)
    }
}

fn ty_eq(a: &Value, b: &Value) -> bool {
    a == b
}

fn is_int_ty(t: &Value) -> bool {
    matches!(
        t.as_str(),
        Some("i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "usize" | "isize")
    )
}

fn check_expr(
    e: &Value,
    env: &mut HashMap<String, Value>,
    fns: &HashMap<String, &Value>,
    break_ty: &mut Option<Value>,
    diags: &mut Vec<Diagnostic>,
) -> Option<Value> {
    if e.is_boolean() {
        return Some(Value::String("bool".into()));
    }
    if e.is_number() {
        return Some(Value::String("i32".into()));
    }
    if e.is_string() {
        return Some(Value::String("str".into()));
    }
    let (t, rest) = tag(e)?;
    match t {
        "lit" => {
            if rest.len() != 2 || !rest[0].is_string() || !rest[1].is_string() {
                diags.push(err("type.lit", "bad lit"));
                return None;
            }
            Some(rest[0].clone())
        }
        "var" => {
            let name = rest[0].as_str()?;
            match env.get(name) {
                Some(ty) => Some(ty.clone()),
                None => {
                    diags.push(err("type.unbound", format!("unknown variable {name}")));
                    None
                }
            }
        }
        "seq" => {
            let mut last = Value::String("i32".into());
            for x in rest {
                last = check_expr(x, env, fns, break_ty, diags)?;
            }
            Some(last)
        }
        "let" => {
            if rest.len() != 2 {
                diags.push(err("type.let", "bad let"));
                return None;
            }
            let mut child = env.clone();
            for b in rest[0].as_array()? {
                let ba = b.as_array()?;
                let name = ba[0].as_str()?.to_string();
                let (ty_anno, init) = if ba.len() == 2 {
                    (None, &ba[1])
                } else {
                    (Some(&ba[1]), &ba[2])
                };
                let it = check_expr(init, &mut child, fns, break_ty, diags)?;
                if let Some(anno) = ty_anno {
                    if !ty_eq(anno, &it) {
                        diags.push(err("type.mismatch", format!("let {name} type mismatch")));
                        return None;
                    }
                    child.insert(name, anno.clone());
                } else {
                    child.insert(name, it);
                }
            }
            check_expr(&rest[1], &mut child, fns, break_ty, diags)
        }
        "set!" => {
            let name = rest[0].as_str()?;
            let slot = env.get(name).cloned();
            let Some(slot) = slot else {
                diags.push(err("type.unbound", format!("set! unknown {name}")));
                return None;
            };
            let it = check_expr(&rest[1], env, fns, break_ty, diags)?;
            if !ty_eq(&slot, &it) {
                diags.push(err("type.mismatch", format!("set! type mismatch for {name}")));
                return None;
            }
            Some(slot)
        }
        "if" => {
            if rest.len() != 3 {
                diags.push(err("type.if", "bad if"));
                return None;
            }
            let c = check_expr(&rest[0], env, fns, break_ty, diags)?;
            if c != Value::String("bool".into()) {
                diags.push(err("type.mismatch", "if cond must be bool"));
                return None;
            }
            let th = check_expr(&rest[1], env, fns, break_ty, diags)?;
            let el = check_expr(&rest[2], env, fns, break_ty, diags)?;
            if th == Value::String("never".into()) {
                return Some(el);
            }
            if el == Value::String("never".into()) {
                return Some(th);
            }
            if !ty_eq(&th, &el) {
                diags.push(err("type.mismatch", "if branches must match"));
                return None;
            }
            Some(th)
        }
        "loop" => {
            let mut inner: Option<Value> = None;
            check_expr(&rest[0], env, fns, &mut inner, diags)?;
            match inner {
                Some(ty) => Some(ty),
                None => {
                    diags.push(err("type.loop", "loop needs break with value"));
                    None
                }
            }
        }
        "break" => {
            let ty = check_expr(&rest[0], env, fns, break_ty, diags)?;
            if let Some(prev) = break_ty {
                if !ty_eq(prev, &ty) {
                    diags.push(err("type.mismatch", "break types disagree"));
                    return None;
                }
            }
            *break_ty = Some(ty);
            Some(Value::String("never".into()))
        }
        "return" => check_expr(&rest[0], env, fns, break_ty, diags),
        "call" => {
            let callee = rest[0].as_str()?;
            let mut arg_tys = Vec::new();
            for a in &rest[1..] {
                arg_tys.push(check_expr(a, env, fns, break_ty, diags)?);
            }
            match callee {
                "+" | "-" | "*" | "/" | "%" => {
                    if arg_tys.len() == 2 && ty_eq(&arg_tys[0], &arg_tys[1]) && is_int_ty(&arg_tys[0])
                    {
                        Some(arg_tys[0].clone())
                    } else {
                        diags.push(err("type.mismatch", format!("arith {callee}")));
                        None
                    }
                }
                "<" | "<=" | ">" | ">=" | "==" | "!=" => {
                    if arg_tys.len() == 2 && ty_eq(&arg_tys[0], &arg_tys[1]) {
                        Some(Value::String("bool".into()))
                    } else {
                        diags.push(err("type.mismatch", format!("compare {callee}")));
                        None
                    }
                }
                other => {
                    let f = fns.get(other);
                    let Some(f) = f else {
                        diags.push(err("type.unbound", format!("unknown function {other}")));
                        return None;
                    };
                    let ret = f.as_array().unwrap()[3].clone();
                    Some(ret)
                }
            }
        }
        "as" => {
            check_expr(&rest[1], env, fns, break_ty, diags)?;
            Some(rest[0].clone())
        }
        "cap" => {
            for a in &rest[1..] {
                check_expr(a, env, fns, break_ty, diags)?;
            }
            Some(Value::String("i32".into()))
        }
        other => {
            diags.push(err("type.unsupported", format!("unsupported expr {other}")));
            None
        }
    }
}
