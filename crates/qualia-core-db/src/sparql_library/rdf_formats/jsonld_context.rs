//! Pinned JSON-LD 1.1 context for Qualia package receipts (UE-012).
//!
//! Callers embed this context (or an equivalent pin) so Node/Civics packages
//! never fetch remote `@context` URLs at admission time. Digests are SHA-256
//! over the exact UTF-8 bytes of [`QUALIA_JSONLD_CONTEXT_V1`].

use sha2::{Digest, Sha256};

/// Qualia / Civics decision-evidence JSON-LD 1.1 context (v1).
///
/// Compact, reviewable, and sufficient for observation / evidence / receipt
/// graphs. Extend only by bumping the version string and adding a new const.
pub const QUALIA_JSONLD_CONTEXT_V1: &str = r#"{
  "@context": {
    "@version": 1.1,
    "@vocab": "https://webizen.org/ns/qualia#",
    "xsd": "http://www.w3.org/2001/XMLSchema#",
    "rdf": "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
    "rdfs": "http://www.w3.org/2000/01/rdf-schema#",
    "owl": "http://www.w3.org/2002/07/owl#",
    "sh": "http://www.w3.org/ns/shacl#",
    "q42": "https://webizen.org/ns/q42#",
    "id": "@id",
    "type": "@type",
    "Observation": "q42:Observation",
    "Evidence": "q42:Evidence",
    "Receipt": "q42:CalculationReceipt",
    "observedAt": { "@id": "q42:observedAt", "@type": "xsd:dateTime" },
    "value": { "@id": "q42:value", "@type": "xsd:decimal" },
    "subject": { "@id": "q42:subject", "@type": "@id" },
    "predicate": { "@id": "q42:predicate", "@type": "@id" },
    "object": { "@id": "q42:object" },
    "engineVersion": "q42:engineVersion",
    "algorithm": "q42:algorithm",
    "receiptHash": "q42:receiptHash",
    "contextDigest": "q42:contextDigest",
    "conforms": { "@id": "sh:conforms", "@type": "xsd:boolean" }
  },
  "qualia:contextVersion": "1"
}
"#;

/// Label for package manifests / receipts.
pub const QUALIA_JSONLD_CONTEXT_ID: &str = "qualia-jsonld-context-v1";

/// UTF-8 bytes of the pinned v1 context.
pub fn pinned_context_bytes() -> &'static [u8] {
    QUALIA_JSONLD_CONTEXT_V1.as_bytes()
}

/// SHA-256 of [`QUALIA_JSONLD_CONTEXT_V1`] as lowercase hex (64 chars).
pub fn context_digest_hex() -> String {
    digest_hex(pinned_context_bytes())
}

/// SHA-256 of arbitrary context bytes as lowercase hex.
pub fn digest_hex(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in hash {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_digest_is_stable_sha256() {
        let d = context_digest_hex();
        assert_eq!(d.len(), 64);
        assert!(d.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')));
        assert_eq!(d, digest_hex(QUALIA_JSONLD_CONTEXT_V1.as_bytes()));
        assert_eq!(d, context_digest_hex());
    }

    #[test]
    fn digest_changes_when_context_changes() {
        let a = digest_hex(b"{\"@context\":{}}");
        let b = digest_hex(b"{\"@context\":{\"a\":1}}");
        assert_ne!(a, b);
    }

    #[test]
    fn pinned_bytes_match_const_str() {
        assert_eq!(pinned_context_bytes(), QUALIA_JSONLD_CONTEXT_V1.as_bytes());
        assert!(QUALIA_JSONLD_CONTEXT_V1.contains("@version"));
        assert!(QUALIA_JSONLD_CONTEXT_V1.contains("q42:"));
    }
}
