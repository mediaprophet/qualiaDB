//! Cryptographic operations — hashing, AEAD, KDF, signing (T-crypto).
//!
//! This module defines the VibeScript crypto namespace. The actual
//! crypto implementations live in the host (qualia-core-db's
//! cryptographic_library), so all operations dispatch through the
//! Host ABI. vibe itself has no crypto dependencies — it
//! defines the types and the dispatch contract.
//!
//! ## Namespace
//!
//! ```vibe
//! import "crypto" as crypto;
//!
//! fn main() {
//!   let h = crypto.sha256("hello");
//!   let sig = crypto.sign("key:ed25519:0", h);
//!   let ok = crypto.verify("key:ed25519:0", h, sig);
//! }
//! ```
//!
//! ## Operations
//!
//! - **Hashing:** `crypto.sha256`, `crypto.sha512`, `crypto.blake3`
//!   — pure, deterministic. Input: String or hex. Output: Record
//!   `{ algorithm, hex, bytes }`.
//! - **KDF:** `crypto.hkdf_sha256(ikm, info, length)` — key derivation.
//! - **AEAD encrypt:** `crypto.aead_encrypt(algorithm, key, nonce,
//!   plaintext, aad)` — returns Record `{ ciphertext, tag, nonce }`.
//! - **AEAD decrypt:** `crypto.aead_decrypt(algorithm, key, nonce,
//!   ciphertext, tag, aad)` — returns plaintext or error.
//! - **Signing:** `crypto.sign(key_id, data)` — returns signature.
//! - **Verification:** `crypto.verify(key_id, data, signature)` —
//!   returns Bool.
//!
//! ## Fail-closed
//!
//! Default Host behavior is E702 (no crypto provider on this host).
//! A host with a crypto library overrides these methods.
//!
//! Reference: qualia-core-db `specialized_libs/cryptographic_library/`.

use crate::span::Span;
use crate::value::Value;
use std::collections::BTreeMap;

/// Build a hash result Record value.
/// `{ algorithm: String, hex: String, bytes: U64 }`
pub fn hash_result_value(algorithm: &str, hex: &str, bytes: usize) -> Value {
    let mut rec = BTreeMap::new();
    rec.insert("algorithm".into(), Value::String(algorithm.into()));
    rec.insert("hex".into(), Value::String(hex.into()));
    rec.insert("bytes".into(), Value::U64(bytes as u64));
    Value::Record(rec)
}

/// Build an encrypted data Record value.
/// `{ ciphertext_hex: String, tag_hex: String, nonce_hex: String, algorithm: String }`
pub fn encrypted_data_value(
    algorithm: &str,
    ciphertext_hex: &str,
    tag_hex: &str,
    nonce_hex: &str,
) -> Value {
    let mut rec = BTreeMap::new();
    rec.insert("algorithm".into(), Value::String(algorithm.into()));
    rec.insert(
        "ciphertext_hex".into(),
        Value::String(ciphertext_hex.into()),
    );
    rec.insert("tag_hex".into(), Value::String(tag_hex.into()));
    rec.insert("nonce_hex".into(), Value::String(nonce_hex.into()));
    Value::Record(rec)
}

/// Build a signature Record value.
/// `{ key_id: String, signature_hex: String, algorithm: String }`
pub fn signature_value(key_id: &str, signature_hex: &str, algorithm: &str) -> Value {
    let mut rec = BTreeMap::new();
    rec.insert("key_id".into(), Value::String(key_id.into()));
    rec.insert("signature_hex".into(), Value::String(signature_hex.into()));
    rec.insert("algorithm".into(), Value::String(algorithm.into()));
    Value::Record(rec)
}

/// Extract a String argument from args, or return a diagnostic.
pub fn extract_string_arg(
    args: &[Value],
    index: usize,
    name: &str,
    span: Span,
) -> Result<String, crate::error::Diagnostic> {
    match args.get(index) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(crate::error::Diagnostic::new(
            crate::error::DiagCode::E100,
            span,
            format!("crypto: {name} expects a string argument at position {index}"),
        )),
    }
}

/// Extract a U64 argument from args, or return a diagnostic.
pub fn extract_u64_arg(
    args: &[Value],
    index: usize,
    name: &str,
    span: Span,
) -> Result<u64, crate::error::Diagnostic> {
    match args.get(index) {
        Some(Value::U64(n)) => Ok(*n),
        Some(Value::I64(n)) => Ok((*n).max(0) as u64),
        _ => Err(crate::error::Diagnostic::new(
            crate::error::DiagCode::E100,
            span,
            format!("crypto: {name} expects a number at position {index}"),
        )),
    }
}

/// Extract an f64 argument from args, or return a diagnostic.
pub fn extract_f64_arg(
    args: &[Value],
    index: usize,
    name: &str,
    span: Span,
) -> Result<f64, crate::error::Diagnostic> {
    match args.get(index) {
        Some(Value::F64(n)) => Ok(*n),
        Some(Value::I64(n)) => Ok(*n as f64),
        Some(Value::U64(n)) => Ok(*n as f64),
        _ => Err(crate::error::Diagnostic::new(
            crate::error::DiagCode::E100,
            span,
            format!("{name} expects a number at position {index}"),
        )),
    }
}

/// Convert bytes to a hex string.
pub fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        s.push_str(&format!("{byte:02x}"));
    }
    s
}

/// Convert a hex string to bytes. Returns None on invalid hex.
pub fn from_hex(hex: &str) -> Option<Vec<u8>> {
    let hex = hex.trim();
    if hex.len() % 2 != 0 {
        return None;
    }
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for i in (0..hex.len()).step_by(2) {
        bytes.push(u8::from_str_radix(&hex[i..i + 2], 16).ok()?);
    }
    Some(bytes)
}

/// The list of crypto namespace capability IDs.
pub const CRYPTO_CAPABILITIES: &[&str] = &[
    "crypto.sha256",
    "crypto.sha512",
    "crypto.blake3",
    "crypto.hkdf_sha256",
    "crypto.aead_encrypt",
    "crypto.aead_decrypt",
    "crypto.sign",
    "crypto.verify",
    "crypto.generate_key",
];

/// Supported AEAD algorithms.
pub const AEAD_ALGORITHMS: &[&str] = &["AES-256-GCM", "ChaCha20-Poly1305", "XChaCha20-Poly1305"];

/// Supported hash algorithms.
pub const HASH_ALGORITHMS: &[&str] = &["SHA-256", "SHA-512", "BLAKE3"];

/// Supported signing algorithms.
pub const SIGNING_ALGORITHMS: &[&str] = &["Ed25519", "ML-DSA-65"];

// ── ZK proofs (zk-SNARKs) ─────────────────────────────────────────────────────

/// The list of ZK namespace capability IDs.
pub const ZK_CAPABILITIES: &[&str] = &[
    "zk.prove_threshold",
    "zk.verify_threshold",
    "zk.prove_range",
    "zk.verify_range",
    "zk.prove_matmul",
    "zk.verify_matmul",
    "zk.list_circuits",
];

/// Build a ZK proof Record value.
/// `{ proof_hex: String, vk_hex: String, proof_id: String, circuit_id: String }`
pub fn zk_proof_value(proof_hex: &str, vk_hex: &str, proof_id: &str, circuit_id: &str) -> Value {
    let mut rec = BTreeMap::new();
    rec.insert("proof_hex".into(), Value::String(proof_hex.into()));
    rec.insert("vk_hex".into(), Value::String(vk_hex.into()));
    rec.insert("proof_id".into(), Value::String(proof_id.into()));
    rec.insert("circuit_id".into(), Value::String(circuit_id.into()));
    Value::Record(rec)
}

/// Build a ZK verification result Record value.
/// `{ valid: Bool, proof_id: String, verification_time_ms: U64 }`
pub fn zk_verification_value(valid: bool, proof_id: &str, verification_time_ms: u64) -> Value {
    let mut rec = BTreeMap::new();
    rec.insert("valid".into(), Value::Bool(valid));
    rec.insert("proof_id".into(), Value::String(proof_id.into()));
    rec.insert(
        "verification_time_ms".into(),
        Value::U64(verification_time_ms),
    );
    Value::Record(rec)
}

/// Build a ZK matmul proof Record value.
/// `{ valid: Bool, result: List<I64> }`
pub fn zk_matmul_result_value(valid: bool, result: &[i128]) -> Value {
    let mut rec = BTreeMap::new();
    rec.insert("valid".into(), Value::Bool(valid));
    rec.insert(
        "result".into(),
        Value::List(result.iter().map(|v| Value::I64(*v as i64)).collect()),
    );
    Value::Record(rec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_result_value_structure() {
        let v = hash_result_value("SHA-256", "abc123", 32);
        let rec = match &v {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        assert_eq!(
            match rec.get("algorithm").unwrap() {
                Value::String(s) => s.as_str(),
                _ => panic!("expected String"),
            },
            "SHA-256"
        );
        assert_eq!(
            match rec.get("hex").unwrap() {
                Value::String(s) => s.as_str(),
                _ => panic!("expected String"),
            },
            "abc123"
        );
        assert_eq!(
            match rec.get("bytes").unwrap() {
                Value::U64(n) => *n,
                _ => panic!("expected U64"),
            },
            32
        );
    }

    #[test]
    fn encrypted_data_value_structure() {
        let v = encrypted_data_value("AES-256-GCM", "ciphertext", "tag", "nonce");
        let rec = match &v {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        assert_eq!(rec.len(), 4);
        assert!(rec.contains_key("ciphertext_hex"));
        assert!(rec.contains_key("tag_hex"));
        assert!(rec.contains_key("nonce_hex"));
    }

    #[test]
    fn signature_value_structure() {
        let v = signature_value("key:ed25519:0", "sig123", "Ed25519");
        let rec = match &v {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        assert_eq!(rec.len(), 3);
        assert!(rec.contains_key("key_id"));
        assert!(rec.contains_key("signature_hex"));
        assert!(rec.contains_key("algorithm"));
    }

    #[test]
    fn to_hex_round_trip() {
        let bytes = vec![0x00, 0xFF, 0xAB, 0xCD];
        let hex = to_hex(&bytes);
        assert_eq!(hex, "00ffabcd");
        let restored = from_hex(&hex).unwrap();
        assert_eq!(restored, bytes);
    }

    #[test]
    fn from_hex_invalid() {
        assert!(from_hex("abc").is_none()); // odd length
        assert!(from_hex("xy").is_none()); // invalid chars
        assert!(from_hex("").is_some()); // empty is valid (empty bytes)
    }

    #[test]
    fn from_hex_valid() {
        assert_eq!(from_hex("48656c6c6f").unwrap(), b"Hello");
        assert_eq!(from_hex("00ff").unwrap(), vec![0x00, 0xFF]);
    }

    #[test]
    fn crypto_capabilities_listed() {
        assert!(CRYPTO_CAPABILITIES.contains(&"crypto.sha256"));
        assert!(CRYPTO_CAPABILITIES.contains(&"crypto.blake3"));
        assert!(CRYPTO_CAPABILITIES.contains(&"crypto.aead_encrypt"));
        assert!(CRYPTO_CAPABILITIES.contains(&"crypto.sign"));
        assert!(CRYPTO_CAPABILITIES.contains(&"crypto.verify"));
    }

    #[test]
    fn aead_algorithms_listed() {
        assert!(AEAD_ALGORITHMS.contains(&"AES-256-GCM"));
        assert!(AEAD_ALGORITHMS.contains(&"ChaCha20-Poly1305"));
        assert!(AEAD_ALGORITHMS.contains(&"XChaCha20-Poly1305"));
    }

    #[test]
    fn hash_algorithms_listed() {
        assert!(HASH_ALGORITHMS.contains(&"SHA-256"));
        assert!(HASH_ALGORITHMS.contains(&"SHA-512"));
        assert!(HASH_ALGORITHMS.contains(&"BLAKE3"));
    }

    #[test]
    fn signing_algorithms_listed() {
        assert!(SIGNING_ALGORITHMS.contains(&"Ed25519"));
        assert!(SIGNING_ALGORITHMS.contains(&"ML-DSA-65"));
    }

    // ── ZK proof value builder tests ─────────────────────────────────

    #[test]
    fn zk_capabilities_listed() {
        assert!(ZK_CAPABILITIES.contains(&"zk.prove_threshold"));
        assert!(ZK_CAPABILITIES.contains(&"zk.verify_threshold"));
        assert!(ZK_CAPABILITIES.contains(&"zk.prove_range"));
        assert!(ZK_CAPABILITIES.contains(&"zk.verify_range"));
        assert!(ZK_CAPABILITIES.contains(&"zk.prove_matmul"));
        assert!(ZK_CAPABILITIES.contains(&"zk.verify_matmul"));
    }

    #[test]
    fn zk_proof_value_structure() {
        let v = zk_proof_value("deadbeef", "vk123", "proof_1", "circuit_42");
        let rec = match &v {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        assert_eq!(rec.len(), 4);
        assert!(rec.contains_key("proof_hex"));
        assert!(rec.contains_key("vk_hex"));
        assert!(rec.contains_key("proof_id"));
        assert!(rec.contains_key("circuit_id"));
    }

    #[test]
    fn zk_verification_value_structure() {
        let v = zk_verification_value(true, "proof_1", 42);
        let rec = match &v {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        assert_eq!(rec.len(), 3);
        assert_eq!(
            match rec.get("valid").unwrap() {
                Value::Bool(b) => *b,
                _ => panic!("expected Bool"),
            },
            true
        );
        assert_eq!(
            match rec.get("verification_time_ms").unwrap() {
                Value::U64(n) => *n,
                _ => panic!("expected U64"),
            },
            42
        );
    }

    #[test]
    fn zk_matmul_result_value_structure() {
        let v = zk_matmul_result_value(true, &[1, 2, 3]);
        let rec = match &v {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        assert_eq!(rec.len(), 2);
        assert_eq!(
            match rec.get("valid").unwrap() {
                Value::Bool(b) => *b,
                _ => panic!("expected Bool"),
            },
            true
        );
        match rec.get("result").unwrap() {
            Value::List(xs) => assert_eq!(xs.len(), 3),
            _ => panic!("expected List"),
        }
    }
}
