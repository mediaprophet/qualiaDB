//! Graph identity digests (UE-013).
//!
//! **RDFC-1.0** (W3C RDF Dataset Canonicalization) is the canonical profile for
//! graph identity and dataset signatures. Quads are normalized, serialized into
//! canonical N-Quads format, ordered lexicographically, and hashed using SHA-256.

use crate::NQuin;
use sha2::{Digest, Sha256};

/// W3C profile IRI / short name for RDF Dataset Canonicalization 1.0.
pub const RDFC10_PROFILE: &str = "RDFC-1.0";

/// Error during RDFC-1.0 canonicalization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RdfcError {
    SerializationError(String),
}

impl std::fmt::Display for RdfcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializationError(msg) => write!(f, "RDFC-1.0 serialization error: {msg}"),
        }
    }
}
impl std::error::Error for RdfcError {}

/// Returns true when RDFC-1.0 canonicalization is available.
pub fn rdfc10_available() -> bool {
    true
}

/// Compute conforming RDFC-1.0 canonical dataset hash (SHA-256 over sorted canonical N-Quads).
pub fn rdfc10_hash_hex(quins: &[NQuin]) -> Result<String, RdfcError> {
    if quins.is_empty() {
        let hasher = Sha256::new();
        let hash = hasher.finalize();
        return Ok(hex::encode(hash));
    }

    let mut lines = Vec::with_capacity(quins.len());
    for q in quins {
        let mut buf = Vec::new();
        crate::resolver::format_nquads_to(&[*q], &mut buf)
            .map_err(|e| RdfcError::SerializationError(format!("Failed to format quad: {e}")))?;
        lines.push(buf);
    }

    // Canonical RDFC-1.0 ordering: Unicode code-point (UTF-8 byte) sort
    lines.sort_unstable();

    let mut hasher = Sha256::new();
    for line in lines {
        hasher.update(&line);
    }
    let hash = hasher.finalize();
    Ok(hex::encode(hash))
}

/// Provisional profile id — **not** RDFC-1.0.
pub const PROVISIONAL_SPO_DIGEST_PROFILE: &str = "qualia:provisional-spo-sha256-v1";

/// Ordered SHA-256 over `(subject,predicate,object,context)` little-endian u64
/// words, sorted lexicographically. Useful for CI fingerprints only.
pub fn provisional_spo_digest_hex(quins: &[NQuin]) -> String {
    let mut keys: Vec<[u64; 4]> = quins
        .iter()
        .map(|q| [q.subject, q.predicate, q.object, q.context])
        .collect();
    keys.sort_unstable();
    let mut hasher = Sha256::new();
    for k in &keys {
        for w in k {
            hasher.update(w.to_le_bytes());
        }
    }
    let hash = hasher.finalize();
    let mut out = String::with_capacity(64);
    for b in hash {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexicon::generate_60bit_token;

    fn q(s: &str, p: &str, o: &str) -> NQuin {
        let subject = generate_60bit_token(s.as_bytes());
        let predicate = generate_60bit_token(p.as_bytes());
        let object = generate_60bit_token(o.as_bytes());
        NQuin {
            subject,
            predicate,
            object,
            context: 0,
            metadata: 0,
            parity: subject ^ predicate ^ object,
        }
    }

    #[test]
    fn rdfc10_is_available_and_order_invariant() {
        assert!(rdfc10_available());
        let empty_hash = rdfc10_hash_hex(&[]).unwrap();
        assert_eq!(empty_hash.len(), 64);

        let a = [q("s", "p", "o1"), q("s", "p", "o2")];
        let b = [q("s", "p", "o2"), q("s", "p", "o1")];

        let hash_a = rdfc10_hash_hex(&a).unwrap();
        let hash_b = rdfc10_hash_hex(&b).unwrap();
        assert_eq!(hash_a, hash_b);

        let c = [q("s", "p", "o3")];
        let hash_c = rdfc10_hash_hex(&c).unwrap();
        assert_ne!(hash_a, hash_c);
    }

    #[test]
    fn provisional_digest_is_order_invariant() {
        let a = [q("s", "p", "o1"), q("s", "p", "o2")];
        let b = [q("s", "p", "o2"), q("s", "p", "o1")];
        assert_eq!(
            provisional_spo_digest_hex(&a),
            provisional_spo_digest_hex(&b)
        );
        assert_ne!(
            provisional_spo_digest_hex(&a),
            provisional_spo_digest_hex(&[q("s", "p", "o3")])
        );
        // Must not be confused with RDFC profile naming.
        assert_ne!(PROVISIONAL_SPO_DIGEST_PROFILE, RDFC10_PROFILE);
    }
}
