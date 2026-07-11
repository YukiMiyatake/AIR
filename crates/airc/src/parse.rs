use crate::diag::{err, tag, Diagnostic};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub raw: Value,
}

pub fn parse_module_json(text: &str) -> Result<Module, Vec<Diagnostic>> {
    let data: Value = serde_json::from_str(text).map_err(|e| {
        vec![err("parse.json", format!("JSON parse error: {e}"))]
    })?;
    parse_module(data)
}

/// Parse air-format from JSON or S-expr (`Value` tree).
pub fn parse_module_value(data: Value) -> Result<Module, Vec<Diagnostic>> {
    parse_module(data)
}

pub fn parse_module(data: Value) -> Result<Module, Vec<Diagnostic>> {
    let Some((t, rest)) = tag(&data) else {
        return Err(vec![err("parse.invalid", "root must be [mod, name, items...]")]);
    };
    if t != "mod" {
        return Err(vec![err("parse.invalid", "root must be [mod, name, items...]")]);
    }
    if rest.is_empty() || !rest[0].is_string() {
        return Err(vec![err("parse.invalid", "mod name must be string")]);
    }
    let name = rest[0].as_str().unwrap().to_string();
    for item in &rest[1..] {
        let Some((it, _)) = tag(item) else {
            return Err(vec![err("parse.invalid", "item must be tagged array")]);
        };
        if it != "fn" && it != "struct" && it != "enum" {
            return Err(vec![err("parse.invalid", format!("unknown item tag: {it}"))]);
        }
        if it == "fn" {
            validate_fn(item)?;
        }
    }
    Ok(Module { name, raw: data })
}

fn validate_fn(v: &Value) -> Result<(), Vec<Diagnostic>> {
    let arr = v.as_array().unwrap();
    if arr.len() != 5 || !arr[1].is_string() || !arr[2].is_array() {
        return Err(vec![err("parse.invalid", "fn must be [fn, name, params, ret, body]")]);
    }
    Ok(())
}

pub fn fns_in_module(module: &Module) -> Vec<&Value> {
    let arr = module.raw.as_array().unwrap();
    arr.iter()
        .skip(2)
        .filter(|v| tag(v).map(|(t, _)| t == "fn").unwrap_or(false))
        .collect()
}

pub fn find_fn<'a>(module: &'a Module, name: &str) -> Option<&'a Value> {
    fns_in_module(module).into_iter().find(|f| {
        f.as_array()
            .and_then(|a| a.get(1))
            .and_then(|n| n.as_str())
            == Some(name)
    })
}
