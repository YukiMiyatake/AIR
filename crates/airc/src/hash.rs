//! Structural AST identity: encoding-independent hash and equality.

use serde_json::Value;
use sha2::{Digest, Sha256};

/// SHA-256 (hex) of the compact JSON encoding of the AST value tree.
///
/// Same AST from `.air` and `.air.json` must hash equal. Whitespace / pretty
/// printing do not affect the hash because hashing is over `serde_json::Value`,
/// not source text.
pub fn ast_digest_hex(v: &Value) -> String {
    let bytes = serde_json::to_vec(v).expect("AST Value is always serializable");
    let hash = Sha256::digest(bytes);
    hex::encode(hash)
}

pub fn ast_eq(a: &Value, b: &Value) -> bool {
    a == b
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn hash_stable_for_same_tree() {
        let a = json!(["lit", "i32", "1"]);
        let b = json!(["lit", "i32", "1"]);
        assert_eq!(ast_digest_hex(&a), ast_digest_hex(&b));
    }

    #[test]
    fn hash_differs_when_child_differs() {
        let a = json!(["lit", "i32", "1"]);
        let b = json!(["lit", "i32", "2"]);
        assert_ne!(ast_digest_hex(&a), ast_digest_hex(&b));
    }
}
