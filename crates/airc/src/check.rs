use crate::diag::{err, tag, Diagnostic};
use crate::parse::{find_fn, Module};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone)]
struct Slot {
    ty: Value,
    moved: bool,
    shared: u32,
    mut_borrowed: bool,
}

type Env = HashMap<String, Slot>;

fn slot_new(ty: Value) -> Slot {
    Slot {
        ty,
        moved: false,
        shared: 0,
        mut_borrowed: false,
    }
}

fn is_borrowed(s: &Slot) -> bool {
    s.shared > 0 || s.mut_borrowed
}

fn acquire_borrow(env: &mut Env, name: &str, kind: &str, diags: &mut Vec<Diagnostic>) -> bool {
    let Some(slot) = env.get_mut(name) else {
        diags.push(err("type.unbound", format!("unknown variable {name}")));
        return false;
    };
    if slot.moved {
        diags.push(err(
            "mem.use_after_move",
            format!("borrow of moved local `{name}`"),
        ));
        return false;
    }
    if kind == "mut" {
        if is_borrowed(slot) {
            diags.push(err(
                "mem.borrow_conflict",
                format!("mut borrow conflicts on `{name}`"),
            ));
            return false;
        }
        slot.mut_borrowed = true;
    } else if kind == "shared" {
        if slot.mut_borrowed {
            diags.push(err(
                "mem.borrow_conflict",
                format!("shared borrow conflicts with mut on `{name}`"),
            ));
            return false;
        }
        slot.shared += 1;
    } else {
        diags.push(err("type.borrow", format!("bad borrow kind {kind}")));
        return false;
    }
    true
}

fn release_borrow(env: &mut Env, name: &str, kind: &str) {
    let Some(slot) = env.get_mut(name) else {
        return;
    };
    if kind == "mut" {
        slot.mut_borrowed = false;
    } else if slot.shared > 0 {
        slot.shared -= 1;
    }
}

fn direct_borrow_of(e: &Value) -> Option<(String, String)> {
    let (t, rest) = tag(e)?;
    if t != "borrow" || rest.len() < 2 {
        return None;
    }
    let kind = rest[0].as_str()?.to_string();
    let (pt, prest) = tag(&rest[1])?;
    if pt != "var" {
        return None;
    }
    Some((prest[0].as_str()?.to_string(), kind))
}

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
        let mut env = Env::new();
        for p in arr[2].as_array().unwrap() {
            let pa = p.as_array().unwrap();
            env.insert(
                pa[0].as_str().unwrap().to_string(),
                slot_new(pa[1].clone()),
            );
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

fn is_copy(t: &Value) -> bool {
    if let Some(s) = t.as_str() {
        return s != "never";
    }
    if let Some(arr) = t.as_array() {
        match arr.first().and_then(|x| x.as_str()) {
            Some("ref") if arr.get(1).and_then(|x| x.as_str()) == Some("shared") => true,
            Some("array") if arr.len() >= 2 => is_copy(&arr[1]),
            _ => false,
        }
    } else {
        false
    }
}

fn merge_moved(dst: &mut Env, a: &Env, b: &Env) {
    for (name, slot) in dst.iter_mut() {
        let ma = a.get(name).map(|s| s.moved).unwrap_or(slot.moved);
        let mb = b.get(name).map(|s| s.moved).unwrap_or(slot.moved);
        slot.moved = ma || mb;
        let sa = a.get(name).map(|s| s.shared).unwrap_or(slot.shared);
        let sb = b.get(name).map(|s| s.shared).unwrap_or(slot.shared);
        slot.shared = sa.max(sb);
        let mua = a.get(name).map(|s| s.mut_borrowed).unwrap_or(slot.mut_borrowed);
        let mub = b.get(name).map(|s| s.mut_borrowed).unwrap_or(slot.mut_borrowed);
        slot.mut_borrowed = mua || mub;
    }
}

fn check_expr(
    e: &Value,
    env: &mut Env,
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
            let Some(slot) = env.get_mut(name) else {
                diags.push(err("type.unbound", format!("unknown variable {name}")));
                return None;
            };
            if slot.moved {
                diags.push(err(
                    "mem.use_after_move",
                    format!("use of moved local `{name}`"),
                ));
                return None;
            }
            let ty = slot.ty.clone();
            if !is_copy(&ty) {
                if is_borrowed(slot) {
                    diags.push(err(
                        "mem.borrow_conflict",
                        format!("move of borrowed local `{name}`"),
                    ));
                    return None;
                }
                slot.moved = true;
            }
            Some(ty)
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
            let mut held: Vec<(String, String)> = Vec::new();
            for b in rest[0].as_array()? {
                let ba = b.as_array()?;
                let name = ba[0].as_str()?.to_string();
                let (ty_anno, init) = if ba.len() == 2 {
                    (None, &ba[1])
                } else {
                    (Some(&ba[1]), &ba[2])
                };
                if let Some((place, kind)) = direct_borrow_of(init) {
                    held.push((place, kind));
                }
                let it = check_expr(init, &mut child, fns, break_ty, diags)?;
                if let Some(anno) = ty_anno {
                    if !ty_eq(anno, &it) {
                        diags.push(err("type.mismatch", format!("let {name} type mismatch")));
                        return None;
                    }
                    child.insert(name, slot_new(anno.clone()));
                } else {
                    child.insert(name, slot_new(it));
                }
            }
            let out = check_expr(&rest[1], &mut child, fns, break_ty, diags);
            for (place, kind) in held {
                release_borrow(&mut child, &place, &kind);
            }
            out
        }
        "set!" => {
            let name = rest[0].as_str()?;
            let Some(slot) = env.get(name) else {
                diags.push(err("type.unbound", format!("set! unknown {name}")));
                return None;
            };
            if is_borrowed(slot) {
                diags.push(err(
                    "mem.borrow_conflict",
                    format!("set! of borrowed local `{name}`"),
                ));
                return None;
            }
            let slot_ty = slot.ty.clone();
            let it = check_expr(&rest[1], env, fns, break_ty, diags)?;
            if !ty_eq(&slot_ty, &it) {
                diags.push(err(
                    "mem.type_mismatch",
                    format!("set! type mismatch for {name}"),
                ));
                return None;
            }
            if let Some(slot) = env.get_mut(name) {
                slot.moved = false;
            }
            Some(slot_ty)
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
            let mut env_then = env.clone();
            let th = check_expr(&rest[1], &mut env_then, fns, break_ty, diags)?;
            let mut env_else = env.clone();
            let el = check_expr(&rest[2], &mut env_else, fns, break_ty, diags)?;
            merge_moved(env, &env_then, &env_else);
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
                "ok" => {
                    if arg_tys.len() == 1 {
                        Some(serde_json::json!(["result", arg_tys[0], "str"]))
                    } else {
                        diags.push(err("type.call", "ok takes one arg"));
                        None
                    }
                }
                "err" => {
                    if arg_tys.len() == 1 {
                        Some(serde_json::json!(["result", "i32", "str"]))
                    } else {
                        diags.push(err("type.call", "err takes one arg"));
                        None
                    }
                }
                "checked_add" | "checked_sub" | "checked_mul" | "checked_div" => {
                    if arg_tys.len() == 2
                        && ty_eq(&arg_tys[0], &arg_tys[1])
                        && is_int_ty(&arg_tys[0])
                    {
                        Some(serde_json::json!(["result", arg_tys[0], "str"]))
                    } else {
                        diags.push(err(
                            "type.mismatch",
                            format!("{callee} needs two same integer args"),
                        ));
                        None
                    }
                }
                "aget" => {
                    if arg_tys.len() == 2 {
                        if let Some(arr) = arg_tys[0].as_array() {
                            if arr.first().and_then(|x| x.as_str()) == Some("array") && arr.len() >= 2
                            {
                                return Some(arr[1].clone());
                            }
                        }
                    }
                    diags.push(err("type.call", "aget(array, idx)"));
                    None
                }
                "aset" => {
                    if arg_tys.len() == 3 {
                        if let Some(arr) = arg_tys[0].as_array() {
                            if arr.first().and_then(|x| x.as_str()) == Some("array")
                                && arr.len() >= 2
                                && arg_tys[1] == Value::String("i32".into())
                                && ty_eq(&arr[1], &arg_tys[2])
                            {
                                return Some(Value::String("i32".into()));
                            }
                        }
                    }
                    diags.push(err("type.call", "aset(array, idx, value)"));
                    None
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
        "match" => {
            if rest.is_empty() {
                diags.push(err("type.match", "bad match"));
                return None;
            }
            let scr = check_expr(&rest[0], env, fns, break_ty, diags)?;
            let mut out: Option<Value> = None;
            let mut arm_envs: Vec<Env> = Vec::new();
            for arm in &rest[1..] {
                let aa = arm.as_array()?;
                if aa.len() != 2 {
                    diags.push(err("type.match", "arm must be [pattern, expr]"));
                    return None;
                }
                let mut child = env.clone();
                if let Some(pat) = aa[0].as_array() {
                    if pat.len() >= 2 {
                        if let (Some("ok"), Some(name)) = (pat[0].as_str(), pat[1].as_str()) {
                            if let Some(scr_arr) = scr.as_array() {
                                if scr_arr.first().and_then(|x| x.as_str()) == Some("result") {
                                    child.insert(name.to_string(), slot_new(scr_arr[1].clone()));
                                }
                            }
                        } else if let (Some("err"), Some(name)) =
                            (pat[0].as_str(), pat[1].as_str())
                        {
                            if let Some(scr_arr) = scr.as_array() {
                                if scr_arr.first().and_then(|x| x.as_str()) == Some("result") {
                                    child.insert(name.to_string(), slot_new(scr_arr[2].clone()));
                                }
                            }
                        }
                    }
                }
                let bt = check_expr(&aa[1], &mut child, fns, break_ty, diags)?;
                arm_envs.push(child);
                if let Some(prev) = &out {
                    if !ty_eq(prev, &bt) {
                        diags.push(err("type.mismatch", "match arms type mismatch"));
                        return None;
                    }
                }
                out = Some(bt);
            }
            if let (Some(a), Some(b)) = (arm_envs.first(), arm_envs.get(1)) {
                merge_moved(env, a, b);
                for extra in arm_envs.iter().skip(2) {
                    let snap = env.clone();
                    merge_moved(env, &snap, extra);
                }
            }
            out
        }
        "array_lit" => {
            if rest.is_empty() {
                diags.push(err("type.array", "array_lit needs element type"));
                return None;
            }
            let elem_ty = &rest[0];
            for el in &rest[1..] {
                let t = check_expr(el, env, fns, break_ty, diags)?;
                if !ty_eq(&t, elem_ty) {
                    diags.push(err("type.mismatch", "array_lit element type mismatch"));
                    return None;
                }
            }
            Some(serde_json::json!(["array", elem_ty, rest.len() - 1]))
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
        "borrow" => {
            if rest.len() < 2 {
                diags.push(err("type.borrow", "bad borrow"));
                return None;
            }
            let kind = rest[0].as_str()?;
            let place = &rest[1];
            let Some((pt, prest)) = tag(place) else {
                diags.push(err("type.borrow", "borrow place must be tagged"));
                return None;
            };
            if pt != "var" {
                diags.push(err(
                    "type.borrow",
                    "v0 borrow place must be [\"var\", name]",
                ));
                return None;
            }
            let name = prest[0].as_str()?;
            let Some(slot) = env.get(name) else {
                diags.push(err("type.unbound", format!("unknown variable {name}")));
                return None;
            };
            let inner_ty = slot.ty.clone();
            if !acquire_borrow(env, name, kind, diags) {
                return None;
            }
            Some(serde_json::json!(["ref", kind, inner_ty]))
        }
        "move" => {
            if rest.is_empty() {
                diags.push(err("type.move", "bad move"));
                return None;
            }
            let place = &rest[0];
            let Some((pt, prest)) = tag(place) else {
                diags.push(err("type.move", "move place must be tagged"));
                return None;
            };
            if pt != "var" {
                diags.push(err("type.move", "v0 move place must be [\"var\", name]"));
                return None;
            }
            let name = prest[0].as_str()?;
            let Some(slot) = env.get_mut(name) else {
                diags.push(err("type.unbound", format!("unknown variable {name}")));
                return None;
            };
            if slot.moved {
                diags.push(err(
                    "mem.use_after_move",
                    format!("use of moved local `{name}`"),
                ));
                return None;
            }
            if is_borrowed(slot) {
                diags.push(err(
                    "mem.borrow_conflict",
                    format!("move of borrowed local `{name}`"),
                ));
                return None;
            }
            let ty = slot.ty.clone();
            slot.moved = true;
            Some(ty)
        }
        other => {
            diags.push(err("type.unsupported", format!("unsupported expr {other}")));
            None
        }
    }
}
