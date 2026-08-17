//! SHA-256 of a string. Future seam: `crypto/` (already a folder in core-db).

use super::super::args;
use poet_vibe::{Diagnostic, Span, Value};
use sha2::{Digest, Sha256};

pub fn digest(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let s = args::as_str(args_v).ok_or_else(|| args::bad(span, "sha256 needs a string"))?;
    let hash = Sha256::digest(s.as_bytes());
    Ok(Value::String(hex_lower(&hash)))
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
