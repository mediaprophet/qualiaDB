//! SHA-256 / SHA-512 / BLAKE3 of a string or hex blob.
//! Future seam: `crypto/` (already a folder in core-db).

use super::super::args;
use vibe::{Diagnostic, Span, Value};
use sha2::{Digest, Sha256, Sha512};

pub fn digest(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let s = args::as_str(args_v).ok_or_else(|| args::bad(span, "sha256 needs a string"))?;
    let hash = Sha256::digest(s.as_bytes());
    Ok(Value::String(hex_lower(&hash)))
}

/// SHA-512 of a `text` (string) or `hex` (hex-encoded string) record.
/// Output: record `{ algorithm: "SHA-512", hex: string, bytes: usize }`.
pub fn sha512(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let bytes = extract_bytes(args_v, span, "sha512")?;
    let hash = Sha512::digest(&bytes);
    let hex = hex_lower(&hash);
    let n = hex.len() / 2;
    Ok(args::record([
        ("algorithm", Value::String("SHA-512".into())),
        ("hex", Value::String(hex)),
        ("bytes", Value::U64(n as u64)),
    ]))
}

/// BLAKE3 of a `text` (string) or `hex` (hex-encoded string) record.
/// Output: record `{ algorithm: "BLAKE3", hex: string, bytes: usize }`.
pub fn blake3(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let bytes = extract_bytes(args_v, span, "blake3")?;
    let hash = blake3::hash(&bytes);
    let hex = hex_lower(hash.as_bytes());
    let n = hex.len() / 2;
    Ok(args::record([
        ("algorithm", Value::String("BLAKE3".into())),
        ("hex", Value::String(hex)),
        ("bytes", Value::U64(n as u64)),
    ]))
}

/// Resolve the input bytes for `sha512`/`blake3`: a record with `text` (string)
/// or `hex` (hex-encoded string). `text` wins when both are present.
fn extract_bytes(args_v: &Value, span: Span, what: &str) -> Result<Vec<u8>, Diagnostic> {
    if let Some(text) = args::rec_str(args_v, "text") {
        return Ok(text.as_bytes().to_vec());
    }
    if let Some(hex) = args::rec_str(args_v, "hex") {
        return hex_decode(hex).ok_or_else(|| args::bad(span, format!("{what} needs valid hex")));
    }
    Err(args::bad(
        span,
        format!("{what} needs a record with `text` or `hex`"),
    ))
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

/// Decode a lowercase/uppercase hex string into bytes. `None` on odd length or
/// non-hex characters.
fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    let bytes = hex.as_bytes();
    if bytes.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i < bytes.len() {
        let hi = hex_nibble(bytes[i])?;
        let lo = hex_nibble(bytes[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Some(out)
}

fn hex_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn empty_sha256() {
        let v = digest(&Value::String(String::new()), Span { start: 0, end: 0 }).unwrap();
        assert_eq!(
            v,
            Value::String(
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into()
            )
        );
    }

    #[test]
    fn sha512_of_text() {
        let mut m = BTreeMap::new();
        m.insert("text".into(), Value::String("abc".into()));
        let v = sha512(&Value::Record(m), Span { start: 0, end: 0 }).unwrap();
        let rec = match v {
            Value::Record(r) => r,
            other => panic!("expected record, got {other:?}"),
        };
        assert_eq!(rec.get("algorithm"), Some(&Value::String("SHA-512".into())));
        assert_eq!(rec.get("bytes"), Some(&Value::U64(64)));
        // Known SHA-512("abc") prefix.
        let hex = match rec.get("hex") {
            Some(Value::String(s)) => s.as_str(),
            other => panic!("expected hex string, got {other:?}"),
        };
        assert!(hex.starts_with("ddaf35a193617abacc417349ae20413112e6fa4e89a97ea2"));
    }

    #[test]
    fn sha512_of_hex() {
        let mut m = BTreeMap::new();
        m.insert("hex".into(), Value::String("616263".into())); // "abc"
        let v = sha512(&Value::Record(m), Span { start: 0, end: 0 }).unwrap();
        let rec = match v {
            Value::Record(r) => r,
            other => panic!("expected record, got {other:?}"),
        };
        let hex = match rec.get("hex") {
            Some(Value::String(s)) => s.as_str(),
            other => panic!("expected hex string, got {other:?}"),
        };
        assert!(hex.starts_with("ddaf35a193617abacc417349ae20413112e6fa4e89a97ea2"));
    }

    #[test]
    fn sha512_rejects_bad_input() {
        let v = Value::String("abc".into());
        assert!(sha512(&v, Span { start: 0, end: 0 }).is_err());

        let mut m = BTreeMap::new();
        m.insert("hex".into(), Value::String("zz".into()));
        assert!(sha512(&Value::Record(m), Span { start: 0, end: 0 }).is_err());
    }

    #[test]
    fn blake3_of_text() {
        let mut m = BTreeMap::new();
        m.insert("text".into(), Value::String("abc".into()));
        let v = blake3(&Value::Record(m), Span { start: 0, end: 0 }).unwrap();
        let rec = match v {
            Value::Record(r) => r,
            other => panic!("expected record, got {other:?}"),
        };
        assert_eq!(rec.get("algorithm"), Some(&Value::String("BLAKE3".into())));
        assert_eq!(rec.get("bytes"), Some(&Value::U64(32)));
        // BLAKE3 default hash is 32 bytes → 64 hex chars.
        let hex = match rec.get("hex") {
            Some(Value::String(s)) => s,
            other => panic!("expected hex string, got {other:?}"),
        };
        assert_eq!(hex.len(), 64);
        // Deterministic: same input → same output.
        let mut m2 = BTreeMap::new();
        m2.insert("text".into(), Value::String("abc".into()));
        let v2 = blake3(&Value::Record(m2), Span { start: 0, end: 0 }).unwrap();
        if let Value::Record(r2) = v2 {
            if let Some(Value::String(h2)) = r2.get("hex") {
                assert_eq!(h2, hex);
            }
        }
    }

    #[test]
    fn blake3_empty_text() {
        let mut m = BTreeMap::new();
        m.insert("text".into(), Value::String(String::new()));
        let v = blake3(&Value::Record(m), Span { start: 0, end: 0 }).unwrap();
        let rec = match v {
            Value::Record(r) => r,
            other => panic!("expected record, got {other:?}"),
        };
        let hex = match rec.get("hex") {
            Some(Value::String(s)) => s,
            other => panic!("expected hex string, got {other:?}"),
        };
        // BLAKE3("") — 32 bytes → 64 hex chars, all lowercase hex.
        assert_eq!(hex.len(), 64);
        assert!(hex
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }
}
