use crate::diag::{err, tag, Diagnostic};
use crate::parse::{collect_enum_defs, collect_struct_defs, find_fn, Module, EnumVariants, StructFields};
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
type StructDefs = HashMap<String, StructFields>;
type EnumDefs = HashMap<String, EnumVariants>;

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
    let structs = collect_struct_defs(module)?;
    let enums = collect_enum_defs(module)?;
    for name in structs.keys() {
        if enums.contains_key(name) {
            return Err(vec![err(
                "parse.duplicate",
                format!("struct and enum both named `{name}`"),
            )]);
        }
    }
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
        match check_expr(body, &mut env, &fns, &structs, &enums, &mut break_ty, &mut diags) {
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

fn is_copy(t: &Value, structs: &StructDefs, enums: &EnumDefs) -> bool {
    if let Some(s) = t.as_str() {
        return s != "never";
    }
    if let Some(arr) = t.as_array() {
        match arr.first().and_then(|x| x.as_str()) {
            Some("ref") if arr.get(1).and_then(|x| x.as_str()) == Some("shared") => true,
            Some("array") if arr.len() >= 2 => is_copy(&arr[1], structs, enums),
            Some("named") if arr.len() >= 2 => {
                let Some(name) = arr[1].as_str() else {
                    return false;
                };
                if let Some(fields) = structs.get(name) {
                    return fields.iter().all(|(_, ft)| is_copy(ft, structs, enums));
                }
                if let Some(vars) = enums.get(name) {
                    return vars.iter().all(|v| {
                        v.payloads.iter().all(|t| is_copy(t, structs, enums))
                    });
                }
                false
            }
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
    structs: &StructDefs,
    enums: &EnumDefs,
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
            if !is_copy(&ty, structs, enums) {
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
                last = check_expr(x, env, fns, structs, enums, break_ty, diags)?;
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
                let it = check_expr(init, &mut child, fns, structs, enums, break_ty, diags)?;
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
            let out = check_expr(&rest[1], &mut child, fns, structs, enums, break_ty, diags);
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
            let it = check_expr(&rest[1], env, fns, structs, enums, break_ty, diags)?;
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
            let c = check_expr(&rest[0], env, fns, structs, enums, break_ty, diags)?;
            if c != Value::String("bool".into()) {
                diags.push(err("type.mismatch", "if cond must be bool"));
                return None;
            }
            let mut env_then = env.clone();
            let th = check_expr(&rest[1], &mut env_then, fns, structs, enums, break_ty, diags)?;
            let mut env_else = env.clone();
            let el = check_expr(&rest[2], &mut env_else, fns, structs, enums, break_ty, diags)?;
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
            check_expr(&rest[0], env, fns, structs, enums, &mut inner, diags)?;
            match inner {
                Some(ty) => Some(ty),
                None => {
                    diags.push(err("type.loop", "loop needs break with value"));
                    None
                }
            }
        }
        "break" => {
            let ty = check_expr(&rest[0], env, fns, structs, enums, break_ty, diags)?;
            if let Some(prev) = break_ty {
                if !ty_eq(prev, &ty) {
                    diags.push(err("type.mismatch", "break types disagree"));
                    return None;
                }
            }
            *break_ty = Some(ty);
            Some(Value::String("never".into()))
        }
        "return" => check_expr(&rest[0], env, fns, structs, enums, break_ty, diags),
        "call" => {
            let callee = rest[0].as_str()?;
            let mut arg_tys = Vec::new();
            for a in &rest[1..] {
                arg_tys.push(check_expr(a, env, fns, structs, enums, break_ty, diags)?);
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
                "fset" => {
                    if arg_tys.len() != 3 {
                        diags.push(err("type.call", "fset(struct, field, value)"));
                        return None;
                    }
                    let (pt, prest) = tag(&rest[1]).unwrap_or(("", &[]));
                    if pt != "var" {
                        diags.push(err(
                            "type.call",
                            "v0 fset place must be [\"var\", name]",
                        ));
                        return None;
                    }
                    let Some(fname) = rest[2].as_str() else {
                        diags.push(err("type.call", "fset field must be a string name"));
                        return None;
                    };
                    let Some(place_arr) = arg_tys[0].as_array() else {
                        diags.push(err("type.call", "fset of non-struct"));
                        return None;
                    };
                    if place_arr.first().and_then(|x| x.as_str()) != Some("named")
                        || place_arr.len() < 2
                    {
                        diags.push(err("type.call", "fset of non-named type"));
                        return None;
                    }
                    let ty_name = place_arr[1].as_str().unwrap();
                    let Some(fields) = structs.get(ty_name) else {
                        diags.push(err(
                            "type.unbound",
                            format!("unknown struct `{ty_name}`"),
                        ));
                        return None;
                    };
                    let Some((_, fty)) = fields.iter().find(|(n, _)| n == fname) else {
                        diags.push(err(
                            "type.field",
                            format!("unknown field `{fname}` on `{ty_name}`"),
                        ));
                        return None;
                    };
                    if !ty_eq(fty, &arg_tys[2]) {
                        diags.push(err(
                            "type.mismatch",
                            format!("fset field `{fname}` type mismatch"),
                        ));
                        return None;
                    }
                    let _ = prest;
                    Some(Value::String("i32".into()))
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
            let scr = check_expr(&rest[0], env, fns, structs, enums, break_ty, diags)?;
            let mut out: Option<Value> = None;
            let mut arm_envs: Vec<Env> = Vec::new();
            let mut covered: Vec<String> = Vec::new();
            let mut has_wildcard = false;
            let mut is_result = false;
            let mut enum_name: Option<String> = None;
            if let Some(arr) = scr.as_array() {
                match arr.first().and_then(|x| x.as_str()) {
                    Some("result") => is_result = true,
                    Some("named") if arr.len() >= 2 => {
                        if let Some(n) = arr[1].as_str() {
                            if enums.contains_key(n) {
                                enum_name = Some(n.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
            for arm in &rest[1..] {
                let aa = arm.as_array()?;
                if aa.len() != 2 {
                    diags.push(err("type.match", "arm must be [pattern, expr]"));
                    return None;
                }
                let mut child = env.clone();
                if aa[0].as_str() == Some("_") {
                    has_wildcard = true;
                } else if let Some(pat) = aa[0].as_array() {
                    if pat.len() >= 2 {
                        if let (Some("ok"), Some(name)) = (pat[0].as_str(), pat[1].as_str()) {
                            covered.push("ok".into());
                            if let Some(scr_arr) = scr.as_array() {
                                if scr_arr.first().and_then(|x| x.as_str()) == Some("result") {
                                    child.insert(name.to_string(), slot_new(scr_arr[1].clone()));
                                }
                            }
                        } else if let (Some("err"), Some(name)) =
                            (pat[0].as_str(), pat[1].as_str())
                        {
                            covered.push("err".into());
                            if let Some(scr_arr) = scr.as_array() {
                                if scr_arr.first().and_then(|x| x.as_str()) == Some("result") {
                                    child.insert(name.to_string(), slot_new(scr_arr[2].clone()));
                                }
                            }
                        } else if pat[0].as_str() == Some("variant") && pat.len() >= 3 {
                            let ename = pat[1].as_str()?;
                            let vname = pat[2].as_str()?;
                            covered.push(vname.to_string());
                            if enum_name.as_deref() != Some(ename) {
                                diags.push(err(
                                    "type.match",
                                    format!("variant pattern enum `{ename}` != scrutinee"),
                                ));
                                return None;
                            }
                            let Some(vars) = enums.get(ename) else {
                                diags.push(err(
                                    "type.unbound",
                                    format!("unknown enum `{ename}`"),
                                ));
                                return None;
                            };
                            let Some(var) = vars.iter().find(|v| v.name == vname) else {
                                diags.push(err(
                                    "type.match",
                                    format!("unknown variant `{vname}` on `{ename}`"),
                                ));
                                return None;
                            };
                            let binds = &pat[3..];
                            if binds.len() != var.payloads.len() {
                                diags.push(err(
                                    "type.match",
                                    format!(
                                        "variant `{vname}` expects {} payload bind(s), got {}",
                                        var.payloads.len(),
                                        binds.len()
                                    ),
                                ));
                                return None;
                            }
                            for (i, bind_v) in binds.iter().enumerate() {
                                let Some(bind) = bind_v.as_str() else {
                                    diags.push(err(
                                        "type.match",
                                        "variant payload binds must be names",
                                    ));
                                    return None;
                                };
                                child.insert(
                                    bind.to_string(),
                                    slot_new(var.payloads[i].clone()),
                                );
                            }
                        }
                    }
                }
                let bt = check_expr(&aa[1], &mut child, fns, structs, enums, break_ty, diags)?;
                arm_envs.push(child);
                if let Some(prev) = &out {
                    if !ty_eq(prev, &bt) {
                        diags.push(err("type.mismatch", "match arms type mismatch"));
                        return None;
                    }
                }
                out = Some(bt);
            }
            if is_result && !has_wildcard {
                if !(covered.iter().any(|c| c == "ok") && covered.iter().any(|c| c == "err")) {
                    diags.push(err("type.match", "Result match not exhaustive"));
                    return None;
                }
            }
            if let Some(ename) = &enum_name {
                if !has_wildcard {
                    let vars = enums.get(ename).unwrap();
                    for v in vars {
                        if !covered.iter().any(|c| c == &v.name) {
                            diags.push(err(
                                "type.match",
                                format!("enum `{ename}` match not exhaustive (missing `{}`)", v.name),
                            ));
                            return None;
                        }
                    }
                }
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
                let t = check_expr(el, env, fns, structs, enums, break_ty, diags)?;
                if !ty_eq(&t, elem_ty) {
                    diags.push(err("type.mismatch", "array_lit element type mismatch"));
                    return None;
                }
            }
            Some(serde_json::json!(["array", elem_ty, rest.len() - 1]))
        }
        "struct_lit" => {
            if rest.is_empty() || !rest[0].is_string() {
                diags.push(err("type.struct", "struct_lit needs type name"));
                return None;
            }
            let ty_name = rest[0].as_str()?;
            let Some(fields) = structs.get(ty_name) else {
                diags.push(err(
                    "type.unbound",
                    format!("unknown struct `{ty_name}`"),
                ));
                return None;
            };
            if rest.len() - 1 != fields.len() {
                diags.push(err(
                    "type.struct",
                    format!("struct_lit `{ty_name}` field count mismatch"),
                ));
                return None;
            }
            let mut seen = HashMap::<String, ()>::new();
            for pair in &rest[1..] {
                let pa = pair.as_array()?;
                if pa.len() != 2 || !pa[0].is_string() {
                    diags.push(err("type.struct", "struct_lit field must be [name, expr]"));
                    return None;
                }
                let fname = pa[0].as_str()?.to_string();
                if seen.insert(fname.clone(), ()).is_some() {
                    diags.push(err(
                        "type.struct",
                        format!("duplicate field `{fname}` in struct_lit"),
                    ));
                    return None;
                }
                let Some((_, expected)) = fields.iter().find(|(n, _)| n == &fname) else {
                    diags.push(err(
                        "type.struct",
                        format!("unknown field `{fname}` on `{ty_name}`"),
                    ));
                    return None;
                };
                let got = check_expr(&pa[1], env, fns, structs, enums, break_ty, diags)?;
                if !ty_eq(expected, &got) {
                    diags.push(err(
                        "type.mismatch",
                        format!("struct_lit field `{fname}` type mismatch"),
                    ));
                    return None;
                }
            }
            for (fname, _) in fields {
                if !seen.contains_key(fname) {
                    diags.push(err(
                        "type.struct",
                        format!("missing field `{fname}` in struct_lit `{ty_name}`"),
                    ));
                    return None;
                }
            }
            Some(serde_json::json!(["named", ty_name]))
        }
        "variant_lit" => {
            if rest.len() < 2 || !rest[0].is_string() || !rest[1].is_string() {
                diags.push(err(
                    "type.enum",
                    "variant_lit must be [variant_lit, enum, variant, payload?]",
                ));
                return None;
            }
            let ename = rest[0].as_str()?;
            let vname = rest[1].as_str()?;
            let Some(vars) = enums.get(ename) else {
                diags.push(err("type.unbound", format!("unknown enum `{ename}`")));
                return None;
            };
            let Some(var) = vars.iter().find(|v| v.name == vname) else {
                diags.push(err(
                    "type.enum",
                    format!("unknown variant `{vname}` on `{ename}`"),
                ));
                return None;
            };
            let args = &rest[2..];
            if args.len() != var.payloads.len() {
                diags.push(err(
                    "type.enum",
                    format!(
                        "variant `{vname}` expects {} payload(s), got {}",
                        var.payloads.len(),
                        args.len()
                    ),
                ));
                return None;
            }
            for (i, arg) in args.iter().enumerate() {
                let got = check_expr(arg, env, fns, structs, enums, break_ty, diags)?;
                if !ty_eq(&var.payloads[i], &got) {
                    diags.push(err(
                        "type.mismatch",
                        format!("variant `{vname}` payload {i} type mismatch"),
                    ));
                    return None;
                }
            }
            Some(serde_json::json!(["named", ename]))
        }
        "field" => {
            if rest.len() != 2 || !rest[1].is_string() {
                diags.push(err("type.field", "field must be [field, place, name]"));
                return None;
            }
            let place_ty = check_expr(&rest[0], env, fns, structs, enums, break_ty, diags)?;
            let fname = rest[1].as_str()?;
            let Some(arr) = place_ty.as_array() else {
                diags.push(err("type.field", "field of non-struct"));
                return None;
            };
            if arr.first().and_then(|x| x.as_str()) != Some("named") || arr.len() < 2 {
                diags.push(err("type.field", "field of non-named type"));
                return None;
            }
            let ty_name = arr[1].as_str()?;
            let Some(fields) = structs.get(ty_name) else {
                diags.push(err(
                    "type.unbound",
                    format!("unknown struct `{ty_name}`"),
                ));
                return None;
            };
            let Some((_, fty)) = fields.iter().find(|(n, _)| n == fname) else {
                diags.push(err(
                    "type.field",
                    format!("unknown field `{fname}` on `{ty_name}`"),
                ));
                return None;
            };
            Some(fty.clone())
        }
        "as" => {
            check_expr(&rest[1], env, fns, structs, enums, break_ty, diags)?;
            Some(rest[0].clone())
        }
        "cap" => {
            for a in &rest[1..] {
                check_expr(a, env, fns, structs, enums, break_ty, diags)?;
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
