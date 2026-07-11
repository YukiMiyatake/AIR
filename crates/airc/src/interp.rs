use crate::parse::{find_fn, Module};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static STDOUT_CAPTURE: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Run `f` while capturing lines written by `cap.print` (not CLI `print_value`).
pub fn with_stdout_capture<R>(f: impl FnOnce() -> R) -> (R, Vec<String>) {
    STDOUT_CAPTURE.with(|c| {
        *c.borrow_mut() = Some(Vec::new());
    });
    let result = f();
    let lines = STDOUT_CAPTURE.with(|c| c.borrow_mut().take().unwrap_or_default());
    (result, lines)
}

fn host_print_line(line: &str) {
    STDOUT_CAPTURE.with(|c| {
        if let Some(buf) = c.borrow_mut().as_mut() {
            buf.push(line.to_string());
        } else {
            println!("{line}");
        }
    });
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AirValue {
    I32(i32),
    Bool(bool),
    Str(String),
    Ok(Box<AirValue>),
    Err(Box<AirValue>),
    Array(Vec<AirValue>),
}

#[derive(Debug)]
pub enum RuntimeError {
    Message(String),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::Message(m) => write!(f, "{m}"),
        }
    }
}

#[derive(Debug)]
enum EvalErr {
    Break(AirValue),
    Msg(String),
}

fn tag<'a>(v: &'a Value) -> Option<(&'a str, &'a [Value])> {
    let arr = v.as_array()?;
    let t = arr.first()?.as_str()?;
    Some((t, &arr[1..]))
}

fn as_i32(v: &AirValue) -> Result<i32, EvalErr> {
    match v {
        AirValue::I32(n) => Ok(*n),
        _ => Err(EvalErr::Msg("runtime.type: expected i32".into())),
    }
}

fn lit_value(width: &str, digits: &str) -> Result<AirValue, EvalErr> {
    if width == "bool" {
        return Ok(AirValue::Bool(digits == "true"));
    }
    if width == "str" {
        return Ok(AirValue::Str(digits.to_string()));
    }
    let n: i32 = digits
        .parse()
        .map_err(|_| EvalErr::Msg(format!("runtime.lit: {digits}")))?;
    Ok(AirValue::I32(n))
}

fn eval2(
    e: &Value,
    env: &mut HashMap<String, AirValue>,
    fns: &HashMap<String, &Value>,
) -> Result<AirValue, EvalErr> {
    if let Some(b) = e.as_bool() {
        return Ok(AirValue::Bool(b));
    }
    if let Some(n) = e.as_i64() {
        return Ok(AirValue::I32(n as i32));
    }
    if let Some(s) = e.as_str() {
        return Ok(AirValue::Str(s.to_string()));
    }
    let (t, rest) = tag(e).ok_or_else(|| EvalErr::Msg("runtime.expr".into()))?;
    match t {
        "lit" => lit_value(rest[0].as_str().unwrap(), rest[1].as_str().unwrap()),
        "var" => {
            let name = rest[0].as_str().unwrap();
            env.get(name)
                .cloned()
                .ok_or_else(|| EvalErr::Msg(format!("runtime.unbound: {name}")))
        }
        "seq" => {
            let mut last = AirValue::I32(0);
            for x in rest {
                last = eval2(x, env, fns)?;
            }
            Ok(last)
        }
        "let" => {
            let mut child = env.clone();
            for b in rest[0].as_array().unwrap() {
                let ba = b.as_array().unwrap();
                let name = ba[0].as_str().unwrap().to_string();
                let init = if ba.len() == 2 { &ba[1] } else { &ba[2] };
                let v = eval2(init, &mut child, fns)?;
                child.insert(name, v);
            }
            eval2(&rest[1], &mut child, fns)
        }
        "set!" => {
            let name = rest[0].as_str().unwrap().to_string();
            let v = eval2(&rest[1], env, fns)?;
            env.insert(name, v.clone());
            Ok(v)
        }
        "if" => match eval2(&rest[0], env, fns)? {
            AirValue::Bool(true) => eval2(&rest[1], env, fns),
            AirValue::Bool(false) => eval2(&rest[2], env, fns),
            _ => Err(EvalErr::Msg("runtime.type: if cond".into())),
        },
        "loop" => loop {
            match eval2(&rest[0], env, fns) {
                Err(EvalErr::Break(v)) => return Ok(v),
                Err(e) => return Err(e),
                Ok(_) => continue,
            }
        },
        "break" => Err(EvalErr::Break(eval2(&rest[0], env, fns)?)),
        "return" => eval2(&rest[0], env, fns),
        "call" => {
            let callee = rest[0].as_str().unwrap();
            let mut args = Vec::new();
            for a in &rest[1..] {
                args.push(eval2(a, env, fns)?);
            }
            match callee {
                "+" => Ok(AirValue::I32(
                    as_i32(&args[0])?.wrapping_add(as_i32(&args[1])?),
                )),
                "-" => Ok(AirValue::I32(
                    as_i32(&args[0])?.wrapping_sub(as_i32(&args[1])?),
                )),
                "*" => Ok(AirValue::I32(
                    as_i32(&args[0])?.wrapping_mul(as_i32(&args[1])?),
                )),
                "/" => {
                    let b = as_i32(&args[1])?;
                    if b == 0 {
                        return Err(EvalErr::Msg("runtime.div0".into()));
                    }
                    Ok(AirValue::I32(as_i32(&args[0])? / b))
                }
                "<=" => Ok(AirValue::Bool(as_i32(&args[0])? <= as_i32(&args[1])?)),
                "<" => Ok(AirValue::Bool(as_i32(&args[0])? < as_i32(&args[1])?)),
                ">" => Ok(AirValue::Bool(as_i32(&args[0])? > as_i32(&args[1])?)),
                ">=" => Ok(AirValue::Bool(as_i32(&args[0])? >= as_i32(&args[1])?)),
                "==" => match (&args[0], &args[1]) {
                    (AirValue::I32(a), AirValue::I32(b)) => Ok(AirValue::Bool(a == b)),
                    (AirValue::Bool(a), AirValue::Bool(b)) => Ok(AirValue::Bool(a == b)),
                    (AirValue::Str(a), AirValue::Str(b)) => Ok(AirValue::Bool(a == b)),
                    _ => Ok(AirValue::Bool(false)),
                },
                "ok" => Ok(AirValue::Ok(Box::new(args[0].clone()))),
                "err" => Ok(AirValue::Err(Box::new(args[0].clone()))),
                "aget" => {
                    let idx = as_i32(&args[1])?;
                    match &args[0] {
                        AirValue::Array(elems) => {
                            if idx < 0 || idx as usize >= elems.len() {
                                return Err(EvalErr::Msg("runtime.oob".into()));
                            }
                            Ok(elems[idx as usize].clone())
                        }
                        _ => Err(EvalErr::Msg("runtime.aget".into())),
                    }
                }
                other => {
                    let f = fns
                        .get(other)
                        .ok_or_else(|| EvalErr::Msg(format!("runtime.unbound fn {other}")))?;
                    let arr = f.as_array().unwrap();
                    let mut frame = HashMap::new();
                    for (i, p) in arr[2].as_array().unwrap().iter().enumerate() {
                        let name = p.as_array().unwrap()[0].as_str().unwrap().to_string();
                        frame.insert(name, args[i].clone());
                    }
                    eval2(&arr[4], &mut frame, fns)
                }
            }
        }
        "match" => {
            let scr = eval2(&rest[0], env, fns)?;
            for arm in &rest[1..] {
                let aa = arm.as_array().unwrap();
                let pat = aa[0].as_array().unwrap();
                let body = &aa[1];
                let mut child = env.clone();
                match (pat[0].as_str(), &scr) {
                    (Some("ok"), AirValue::Ok(v)) => {
                        let name = pat[1].as_str().unwrap().to_string();
                        child.insert(name, (**v).clone());
                        return eval2(body, &mut child, fns);
                    }
                    (Some("err"), AirValue::Err(v)) => {
                        let name = pat[1].as_str().unwrap().to_string();
                        child.insert(name, (**v).clone());
                        return eval2(body, &mut child, fns);
                    }
                    _ => continue,
                }
            }
            Err(EvalErr::Msg("runtime.match".into()))
        }
        "array_lit" => {
            let mut elems = Vec::new();
            for el in &rest[1..] {
                elems.push(eval2(el, env, fns)?);
            }
            Ok(AirValue::Array(elems))
        }
        "as" => eval2(&rest[1], env, fns),
        "cap" => {
            if rest[0].as_str() == Some("print") {
                let v = eval2(&rest[1], env, fns)?;
                match v {
                    AirValue::Str(s) => host_print_line(&s),
                    other => host_print_line(&format!("{other:?}")),
                }
            }
            Ok(AirValue::I32(0))
        }
        other => Err(EvalErr::Msg(format!("runtime.unsupported {other}"))),
    }
}

pub fn run_module(module: &Module) -> Result<AirValue, RuntimeError> {
    let mut fns = HashMap::new();
    for f in crate::parse::fns_in_module(module) {
        let name = f.as_array().unwrap()[1].as_str().unwrap().to_string();
        fns.insert(name, f);
    }
    let main =
        find_fn(module, "main").ok_or_else(|| RuntimeError::Message("runtime.main".into()))?;
    let arr = main.as_array().unwrap();
    let mut env = HashMap::new();
    match eval2(&arr[4], &mut env, &fns) {
        Ok(v) => Ok(v),
        Err(EvalErr::Break(v)) => Ok(v),
        Err(EvalErr::Msg(m)) => Err(RuntimeError::Message(m)),
    }
}

pub fn value_to_exit_code(v: &AirValue) -> u8 {
    match v {
        AirValue::I32(n) => *n as u8,
        _ => 0,
    }
}

pub fn print_value(v: &AirValue) {
    match v {
        AirValue::I32(n) => println!("{n}"),
        AirValue::Bool(b) => println!("{b}"),
        AirValue::Str(s) => println!("{s}"),
        other => println!("{other:?}"),
    }
}
