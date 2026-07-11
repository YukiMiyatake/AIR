//! Binary `.airb` sketch: closed syntax tags + interned symbol table.
//!
//! Layout (version 1):
//! ```text
//! magic "AIRB" | u8 version=1
//! u32be sym_count | (u16be len + utf8)* 
//! value…
//! ```
//!
//! Values:
//! - `0x00` null
//! - `0x01` false / `0x02` true
//! - `0x03` i64be number
//! - `0x04` u32be symbol id (string)
//! - `0x05` array: u32be len + values
//! - `0x40` tagged array: u8 known-tag index + u32be rest_len + values
//!   (first JSON element is the tag string; not repeated as a SYM)

use crate::diag::{err, Diagnostic};
use serde_json::{Number, Value};

const MAGIC: &[u8; 4] = b"AIRB";
const VERSION: u8 = 1;

/// Closed set of syntax / type tags (not user type names).
pub const KNOWN_TAGS: &[&str] = &[
    "mod",
    "fn",
    "let",
    "set!",
    "if",
    "loop",
    "break",
    "return",
    "call",
    "lit",
    "var",
    "seq",
    "match",
    "array_lit",
    "cap",
    "borrow",
    "move",
    "as",
    "ok",
    "err",
    "struct",
    "enum",
    "ref",
    "array",
    "result",
    "named",
    "ptr",
    "slice",
    "struct_lit",
    "field",
    "variant",
    "variant_lit",
];

const V_NULL: u8 = 0x00;
const V_FALSE: u8 = 0x01;
const V_TRUE: u8 = 0x02;
const V_I64: u8 = 0x03;
const V_SYM: u8 = 0x04;
const V_ARRAY: u8 = 0x05;
const V_TAGGED: u8 = 0x40;

fn tag_index(name: &str) -> Option<u8> {
    KNOWN_TAGS
        .iter()
        .position(|t| *t == name)
        .map(|i| i as u8)
}

/// Pack an AST value tree into `.airb` bytes.
pub fn pack_airb(v: &Value) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let mut syms: Vec<String> = Vec::new();
    let mut map = std::collections::HashMap::<String, u32>::new();
    intern_collect(v, &mut syms, &mut map);
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    write_u32(&mut out, syms.len() as u32);
    for s in &syms {
        if s.len() > u16::MAX as usize {
            return Err(vec![err("pack.sym", "symbol too long")]);
        }
        write_u16(&mut out, s.len() as u16);
        out.extend_from_slice(s.as_bytes());
    }
    encode_value(v, &map, &mut out)?;
    Ok(out)
}

/// Unpack `.airb` bytes to an AST value tree.
pub fn unpack_airb(bytes: &[u8]) -> Result<Value, Vec<Diagnostic>> {
    let mut r = Reader { bytes, i: 0 };
    let magic = r.read_exact(4)?;
    if magic != MAGIC {
        return Err(vec![err("pack.magic", "not an AIRB file")]);
    }
    let ver = r.u8()?;
    if ver != VERSION {
        return Err(vec![err(
            "pack.version",
            format!("unsupported airb version {ver}"),
        )]);
    }
    let nsym = r.u32()? as usize;
    let mut syms = Vec::with_capacity(nsym);
    for _ in 0..nsym {
        let n = r.u16()? as usize;
        let b = r.read_exact(n)?;
        let s = std::str::from_utf8(b)
            .map_err(|_| vec![err("pack.utf8", "symbol not utf-8")])?
            .to_string();
        syms.push(s);
    }
    decode_value(&mut r, &syms)
}

fn intern_collect(
    v: &Value,
    syms: &mut Vec<String>,
    map: &mut std::collections::HashMap<String, u32>,
) {
    match v {
        Value::String(s) => {
            if !map.contains_key(s) {
                let id = syms.len() as u32;
                map.insert(s.clone(), id);
                syms.push(s.clone());
            }
        }
        Value::Array(items) => {
            for it in items {
                intern_collect(it, syms, map);
            }
        }
        _ => {}
    }
}

fn encode_value(
    v: &Value,
    map: &std::collections::HashMap<String, u32>,
    out: &mut Vec<u8>,
) -> Result<(), Vec<Diagnostic>> {
    match v {
        Value::Null => out.push(V_NULL),
        Value::Bool(false) => out.push(V_FALSE),
        Value::Bool(true) => out.push(V_TRUE),
        Value::Number(n) => {
            let i = n
                .as_i64()
                .ok_or_else(|| vec![err("pack.num", "number must fit i64")])?;
            out.push(V_I64);
            out.extend_from_slice(&i.to_be_bytes());
        }
        Value::String(s) => {
            let id = *map
                .get(s)
                .ok_or_else(|| vec![err("pack.sym", "missing symbol")])?;
            out.push(V_SYM);
            write_u32(out, id);
        }
        Value::Array(items) => {
            if let Some(Value::String(tag)) = items.first() {
                if let Some(ti) = tag_index(tag) {
                    out.push(V_TAGGED);
                    out.push(ti);
                    write_u32(out, (items.len() - 1) as u32);
                    for it in &items[1..] {
                        encode_value(it, map, out)?;
                    }
                    return Ok(());
                }
            }
            out.push(V_ARRAY);
            write_u32(out, items.len() as u32);
            for it in items {
                encode_value(it, map, out)?;
            }
        }
        Value::Object(_) => {
            return Err(vec![err("pack.obj", "objects not supported in airb v1")])
        }
    }
    Ok(())
}

fn decode_value(r: &mut Reader<'_>, syms: &[String]) -> Result<Value, Vec<Diagnostic>> {
    match r.u8()? {
        V_NULL => Ok(Value::Null),
        V_FALSE => Ok(Value::Bool(false)),
        V_TRUE => Ok(Value::Bool(true)),
        V_I64 => {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(r.read_exact(8)?);
            Ok(Value::Number(Number::from(i64::from_be_bytes(buf))))
        }
        V_SYM => {
            let id = r.u32()? as usize;
            let s = syms
                .get(id)
                .ok_or_else(|| vec![err("pack.sym", format!("bad symbol id {id}"))])?;
            Ok(Value::String(s.clone()))
        }
        V_ARRAY => {
            let n = r.u32()? as usize;
            let mut items = Vec::with_capacity(n);
            for _ in 0..n {
                items.push(decode_value(r, syms)?);
            }
            Ok(Value::Array(items))
        }
        V_TAGGED => {
            let ti = r.u8()? as usize;
            let tag = KNOWN_TAGS
                .get(ti)
                .ok_or_else(|| vec![err("pack.tag", format!("bad tag index {ti}"))])?;
            let n = r.u32()? as usize;
            let mut items = Vec::with_capacity(n + 1);
            items.push(Value::String((*tag).to_string()));
            for _ in 0..n {
                items.push(decode_value(r, syms)?);
            }
            Ok(Value::Array(items))
        }
        other => Err(vec![err(
            "pack.value",
            format!("unknown value tag 0x{other:02x}"),
        )]),
    }
}

fn write_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_be_bytes());
}

fn write_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_be_bytes());
}

struct Reader<'a> {
    bytes: &'a [u8],
    i: usize,
}

impl<'a> Reader<'a> {
    fn read_exact(&mut self, n: usize) -> Result<&'a [u8], Vec<Diagnostic>> {
        if self.i + n > self.bytes.len() {
            return Err(vec![err("pack.eof", "truncated airb")]);
        }
        let s = &self.bytes[self.i..self.i + n];
        self.i += n;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8, Vec<Diagnostic>> {
        Ok(self.read_exact(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, Vec<Diagnostic>> {
        let mut b = [0u8; 2];
        b.copy_from_slice(self.read_exact(2)?);
        Ok(u16::from_be_bytes(b))
    }

    fn u32(&mut self) -> Result<u32, Vec<Diagnostic>> {
        let mut b = [0u8; 4];
        b.copy_from_slice(self.read_exact(4)?);
        Ok(u32::from_be_bytes(b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn roundtrip_uses_tagged_opcode_for_mod() {
        let v = json!(["mod", "sum", ["fn", "main", [], "i32", ["lit", "i32", "0"]]]);
        let bytes = pack_airb(&v).unwrap();
        assert_eq!(&bytes[0..4], b"AIRB");
        let back = unpack_airb(&bytes).unwrap();
        assert_eq!(v, back);
        // user name "sum" is a symbol, not a known-tag expansion of the tag enum
        assert!(KNOWN_TAGS.iter().all(|t| *t != "sum"));
    }
}
