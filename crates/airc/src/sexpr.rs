//! Canonical S-expr encoding of air-format AST (`serde_json::Value` trees).

use crate::diag::{err, Diagnostic};
use serde_json::{Number, Value};

/// Parse a module from S-expr text into the same JSON-shaped AST tree.
pub fn parse_sexpr_value(text: &str) -> Result<Value, Vec<Diagnostic>> {
    let mut lx = Lexer::new(text);
    let v = parse_value(&mut lx)?;
    lx.skip_ws();
    if lx.peek_char().is_some() {
        return Err(vec![err("parse.sexpr", "trailing tokens after value")]);
    }
    Ok(v)
}

/// Print a value as normalized S-expr (pretty, 2-space indent).
pub fn print_sexpr(v: &Value) -> String {
    let mut out = String::new();
    write_pretty(v, 0, &mut out);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

struct Lexer<'a> {
    src: &'a str,
    i: usize,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Self { src, i: 0 }
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek_char() {
            if c == ';' {
                while let Some(c2) = self.peek_char() {
                    self.bump();
                    if c2 == '\n' {
                        break;
                    }
                }
            } else if c.is_whitespace() {
                self.bump();
            } else {
                break;
            }
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.src[self.i..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let mut it = self.src[self.i..].chars();
        let c = it.next()?;
        self.i += c.len_utf8();
        Some(c)
    }

    fn next(&mut self) -> Result<Option<Tok<'a>>, Vec<Diagnostic>> {
        self.skip_ws();
        if self.i >= self.src.len() {
            return Ok(None);
        }
        let c = self.peek_char().unwrap();
        if c == '(' {
            self.bump();
            return Ok(Some(Tok::LParen));
        }
        if c == ')' {
            self.bump();
            return Ok(Some(Tok::RParen));
        }
        if c == '"' {
            return Ok(Some(Tok::Str(self.read_string()?)));
        }
        Ok(Some(Tok::Atom(self.read_atom()?)))
    }

    fn read_atom(&mut self) -> Result<&'a str, Vec<Diagnostic>> {
        let start = self.i;
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() || c == '(' || c == ')' || c == ';' || c == '"' {
                break;
            }
            self.bump();
        }
        if self.i == start {
            return Err(vec![err("parse.sexpr", "expected atom")]);
        }
        Ok(&self.src[start..self.i])
    }

    fn read_string(&mut self) -> Result<String, Vec<Diagnostic>> {
        self.bump(); // "
        let mut s = String::new();
        while let Some(c) = self.bump() {
            if c == '"' {
                return Ok(s);
            }
            if c == '\\' {
                let e = self
                    .bump()
                    .ok_or_else(|| vec![err("parse.sexpr", "unterminated escape")])?;
                match e {
                    'n' => s.push('\n'),
                    't' => s.push('\t'),
                    'r' => s.push('\r'),
                    '\\' | '"' => s.push(e),
                    _ => {
                        return Err(vec![err(
                            "parse.sexpr",
                            format!("bad escape \\{e}"),
                        )])
                    }
                }
            } else {
                s.push(c);
            }
        }
        Err(vec![err("parse.sexpr", "unterminated string")])
    }
}

enum Tok<'a> {
    LParen,
    RParen,
    Atom(&'a str),
    Str(String),
}

fn parse_value(lx: &mut Lexer<'_>) -> Result<Value, Vec<Diagnostic>> {
    match lx.next()?.ok_or_else(|| vec![err("parse.sexpr", "unexpected eof")])? {
        Tok::LParen => {
            let mut items = Vec::new();
            loop {
                lx.skip_ws();
                if lx.peek_char() == Some(')') {
                    lx.bump();
                    break;
                }
                if lx.peek_char().is_none() {
                    return Err(vec![err("parse.sexpr", "unclosed list")]);
                }
                items.push(parse_value(lx)?);
            }
            Ok(Value::Array(items))
        }
        Tok::RParen => Err(vec![err("parse.sexpr", "unexpected )")]),
        Tok::Atom(a) => Ok(atom_to_value(a)),
        Tok::Str(s) => Ok(Value::String(s)),
    }
}

fn atom_to_value(a: &str) -> Value {
    if a == "true" {
        return Value::Bool(true);
    }
    if a == "false" {
        return Value::Bool(false);
    }
    if let Ok(n) = a.parse::<i64>() {
        return Value::Number(Number::from(n));
    }
    Value::String(a.to_string())
}

fn write_pretty(v: &Value, indent: usize, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => {
            if is_int_digits(s) {
                out.push_str(s);
            } else if is_bare_atom(s) {
                out.push_str(s);
            } else {
                push_quoted(s, out);
            }
        }
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str("()");
                return;
            }
            // (lit str "hello") — always quote lit payload when width is str
            if items.first().and_then(|x| x.as_str()) == Some("lit")
                && items.len() == 3
                && items[1].as_str() == Some("str")
            {
                out.push_str("(lit str ");
                if let Value::String(s) = &items[2] {
                    push_quoted(s, out);
                } else {
                    write_pretty(&items[2], indent, out);
                }
                out.push(')');
                return;
            }
            out.push('(');
            let multiline = should_multiline(items);
            if !multiline {
                for (i, it) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(' ');
                    }
                    write_pretty(it, indent, out);
                }
                out.push(')');
                return;
            }
            out.push('\n');
            for it in items {
                push_indent(indent + 1, out);
                write_pretty(it, indent + 1, out);
                out.push('\n');
            }
            push_indent(indent, out);
            out.push(')');
        }
        Value::Object(_) => out.push_str("#<object>"),
    }
}

fn should_multiline(items: &[Value]) -> bool {
    if items.len() <= 1 {
        return false;
    }
    // Keep short call-like forms inline when compact.
    let approx: usize = items.iter().map(approx_len).sum::<usize>() + items.len();
    if approx <= 72 && items.iter().all(|x| !matches!(x, Value::Array(a) if a.len() > 3)) {
        return false;
    }
    items.iter().any(|x| matches!(x, Value::Array(a) if !a.is_empty()))
}

fn approx_len(v: &Value) -> usize {
    match v {
        Value::String(s) => s.len(),
        Value::Number(n) => n.to_string().len(),
        Value::Bool(_) => 5,
        Value::Array(a) => a.iter().map(approx_len).sum::<usize>() + a.len() + 2,
        _ => 4,
    }
}

fn is_int_digits(s: &str) -> bool {
    let s = s.as_bytes();
    if s.is_empty() {
        return false;
    }
    let (start, rest) = if s[0] == b'-' {
        if s.len() == 1 {
            return false;
        }
        (1, &s[1..])
    } else {
        (0, s)
    };
    let _ = start;
    !rest.is_empty() && rest.iter().all(|c| c.is_ascii_digit())
}

fn is_bare_atom(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !(first.is_ascii_alphabetic() || "_!+-*/%<>=.".contains(first)) {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || "_!+-*/%<>=.".contains(c))
}

fn push_quoted(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out.push('"');
}

fn push_indent(n: usize, out: &mut String) {
    for _ in 0..n {
        out.push_str("  ");
    }
}

/// After S-expr parse, normalize `lit` digit atoms: `(lit i32 0)` → `["lit","i32","0"]`.
pub fn normalize_lit_digits(v: &mut Value) {
    let Value::Array(items) = v else {
        return;
    };
    if items.first().and_then(|x| x.as_str()) == Some("lit") && items.len() == 3 {
        if let Value::Number(n) = &items[2] {
            items[2] = Value::String(n.to_string());
        }
    }
    for child in items.iter_mut() {
        normalize_lit_digits(child);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_sum_shape() {
        let json = r#"["mod","sum",["fn","main",[],"i32",["lit","i32","0"]]]"#;
        let v: Value = serde_json::from_str(json).unwrap();
        let text = print_sexpr(&v);
        let mut back = parse_sexpr_value(&text).unwrap();
        normalize_lit_digits(&mut back);
        assert_eq!(v, back);
    }
}
