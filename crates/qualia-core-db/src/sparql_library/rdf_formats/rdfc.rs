//! Graph identity digests (UE-013).
//!
//! **RDFC-1.0** (W3C RDF Dataset Canonicalization) is the admitted profile for
//! graph identity and signatures. It is **not yet implemented** in this crate.
//! Callers must not treat any other digest as RDFC-1.0.
//!
//! A clearly labelled **provisional** SPO ordered SHA-256 is provided for
//! package scaffolding only — it is *not* dataset-canonical and must never be
//! written into a receipt field named `rdfc10` / `RDFC-1.0`.

use crate::NQuin;
use sha2::{Digest, Sha256};

/// W3C profile IRI / short name for RDF Dataset Canonicalization 1.0.
pub const RDFC10_PROFILE: &str = "RDFC-1.0";

/// Error when RDFC-1.0 is requested but unavailable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RdfcError {
    /// Full RDFC-1.0 canonicalization is not implemented in this build.
    NotImplemented,
}

impl std::fmt::Display for RdfcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotImplemented => write!(
                f,
                "RDFC-1.0 graph hash is not implemented in Qualia 0.0.39; \
                 use native tooling or wait for a versioned RDFC profile. \
                 Do not hash arbitrary JSON text."
            ),
        }
    }
}
impl std::error::Error for RdfcError {}

/// Always `false` until a conforming RDFC-1.0 implementation ships.
pub fn rdfc10_available() -> bool {
    false
}

/// Fail closed — never returns a digest labelled as RDFC-1.0.
pub fn rdfc10_hash_hex(_quins: &[NQuin]) -> Result<String, RdfcError> {
    Err(RdfcError::NotImplemented)
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
    fn rdfc10_is_unavailable_and_fails_closed() {
        assert!(!rdfc10_available());
        assert_eq!(rdfc10_hash_hex(&[]), Err(RdfcError::NotImplemented));
        let msg = RdfcError::NotImplemented.to_string();
        assert!(msg.contains("RDFC-1.0"));
        assert!(msg.contains("not implemented"));
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
