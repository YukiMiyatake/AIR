use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: &'static str,
    pub code: String,
    pub message: String,
}

pub type DiagMode = str;

pub fn emit_diags(diags: &[Diagnostic], mode: &str, path: &str) {
    for d in diags {
        if mode == "json" {
            let obj = serde_json::json!({
                "schema": "air",
                "code": d.code,
                "severity": d.severity,
                "message": d.message,
                "path": path,
                "span": { "offset": 0, "length": 0 }
            });
            eprintln!("{obj}");
        } else {
            eprintln!("{path}: {} {}: {}", d.severity, d.code, d.message);
        }
    }
}

pub fn err(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        severity: "error",
        code: code.to_string(),
        message: message.into(),
    }
}

pub fn tag<'a>(v: &'a Value) -> Option<(&'a str, &'a [Value])> {
    let arr = v.as_array()?;
    let t = arr.first()?.as_str()?;
    Some((t, &arr[1..]))
}
