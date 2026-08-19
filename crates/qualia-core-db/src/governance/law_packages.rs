//! Law packages as signed content — provenance scaffold (T72).
//!
//! Law packages need signed content: who authored the dissolve rate;
//! under what licence; whether it is physical or fictional. Provenance
//! must travel with the law.

use serde::{Deserialize, Serialize};

/// Whether a law is grounded in physics or is fictional (narrative/game/
/// simulation only).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LawNature {
    /// Grounded in physics — the law reflects real-world behaviour.
    Physical,
    /// Fictional — the law is for narrative, game, or simulation only.
    Fictional,
}

/// A signed law package (T72). Provenance travels with the law.
///
/// The package contains the law's identity, authorship, licence, nature
/// (physical vs fictional), content hash, and an Ed25519 signature by
/// the author. The signature verifies that the author did indeed
/// author this law package and that its content has not been tampered with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LawPackage {
    /// The law's identifier (hash of its content).
    pub law_id: String,
    /// Human-readable name.
    pub name: String,
    /// Author DID (who authored this law).
    pub author_did: String,
    /// Licence under which this law is published.
    pub licence: String,
    /// Whether this law is physical (grounded in physics) or
    /// fictional (narrative/game/simulation only).
    pub nature: LawNature,
    /// Content hash of the law's formal definition.
    pub content_hash: String,
    /// Ed25519 signature by the author over the content hash.
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>,
    /// Unix timestamp when the law was authored.
    pub authored_at_unix: u64,
}

impl LawPackage {
    /// Create a new law package with the given metadata.
    /// The signature should be set via `set_signature`.
    pub fn new(
        law_id: &str,
        name: &str,
        author_did: &str,
        licence: &str,
        nature: LawNature,
        content_hash: &str,
        authored_at_unix: u64,
    ) -> Self {
        Self {
            law_id: law_id.to_string(),
            name: name.to_string(),
            author_did: author_did.to_string(),
            licence: licence.to_string(),
            nature,
            content_hash: content_hash.to_string(),
            signature: Vec::new(),
            authored_at_unix,
        }
    }

    /// The message that the signature covers: the content hash bytes.
    /// The signature is over the content_hash string as UTF-8 bytes.
    fn signature_message(&self) -> &[u8] {
        self.content_hash.as_bytes()
    }

    /// Set the signature (typically after signing with the author's
    /// private key).
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.signature = signature;
    }

    /// Verify the Ed25519 signature against the author's public key.
    ///
    /// Returns `true` if the signature is valid for this package's
    /// content hash, `false` otherwise (including if no signature is
    /// set).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn verify_signature(&self, public_key: &[u8]) -> bool {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};
        if self.signature.is_empty() || public_key.len() != 32 {
            return false;
        }
        let pk_bytes: &[u8; 32] = public_key.try_into().unwrap();
        let pk = match VerifyingKey::from_bytes(pk_bytes) {
            Ok(pk) => pk,
            Err(_) => return false,
        };
        let sig = match Signature::from_slice(&self.signature) {
            Ok(s) => s,
            Err(_) => return false,
        };
        pk.verify(self.signature_message(), &sig).is_ok()
    }

    /// Verify the signature (WASM stub — no ed25519 on WASM).
    #[cfg(target_arch = "wasm32")]
    pub fn verify_signature(&self, _public_key: &[u8]) -> bool {
        false
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{SigningKey, Signer};

    /// Generate a signing key from a fixed seed for deterministic tests.
    fn test_signing_key(seed: u8) -> SigningKey {
        let mut bytes = [0u8; 32];
        bytes[0] = seed;
        SigningKey::from_bytes(&bytes)
    }

    #[test]
    fn law_package_construction() {
        let pkg = LawPackage::new(
            "law:crush:001",
            "Crush Law",
            "did:example:author",
            "CC-BY-4.0",
            LawNature::Physical,
            "sha256:abc123",
            1700000000,
        );
        assert_eq!(pkg.law_id, "law:crush:001");
        assert_eq!(pkg.name, "Crush Law");
        assert_eq!(pkg.author_did, "did:example:author");
        assert_eq!(pkg.licence, "CC-BY-4.0");
        assert_eq!(pkg.nature, LawNature::Physical);
        assert_eq!(pkg.content_hash, "sha256:abc123");
        assert!(pkg.signature.is_empty());
        assert_eq!(pkg.authored_at_unix, 1700000000);
    }

    #[test]
    fn law_nature_physical_vs_fictional() {
        assert_ne!(LawNature::Physical, LawNature::Fictional);
        assert_eq!(LawNature::Physical, LawNature::Physical);
        assert_eq!(LawNature::Fictional, LawNature::Fictional);
    }

    #[test]
    fn json_roundtrip() {
        let mut pkg = LawPackage::new(
            "law:001",
            "Test Law",
            "did:example:author",
            "MIT",
            LawNature::Fictional,
            "sha256:deadbeef",
            1700000000,
        );
        pkg.set_signature(vec![1, 2, 3, 4]);
        let json = pkg.to_json().unwrap();
        let restored = LawPackage::from_json(&json).unwrap();
        assert_eq!(restored.law_id, "law:001");
        assert_eq!(restored.name, "Test Law");
        assert_eq!(restored.author_did, "did:example:author");
        assert_eq!(restored.licence, "MIT");
        assert_eq!(restored.nature, LawNature::Fictional);
        assert_eq!(restored.content_hash, "sha256:deadbeef");
        assert_eq!(restored.signature, vec![1, 2, 3, 4]);
        assert_eq!(restored.authored_at_unix, 1700000000);
    }

    #[test]
    fn verify_signature_with_correct_key() {
        let signing_key = test_signing_key(1);
        let verifying_key = signing_key.verifying_key();

        let content_hash = "sha256:abc123def456";
        let signature = signing_key.sign(content_hash.as_bytes());

        let mut pkg = LawPackage::new(
            "law:001",
            "Test Law",
            "did:example:author",
            "MIT",
            LawNature::Physical,
            content_hash,
            1700000000,
        );
        pkg.set_signature(signature.to_bytes().to_vec());

        assert!(
            pkg.verify_signature(&verifying_key.to_bytes()),
            "signature should verify with correct public key"
        );
    }

    #[test]
    fn verify_signature_with_wrong_key_fails() {
        let signing_key = test_signing_key(1);
        let wrong_key = test_signing_key(2);
        let wrong_verifying_key = wrong_key.verifying_key();

        let content_hash = "sha256:abc123def456";
        let signature = signing_key.sign(content_hash.as_bytes());

        let mut pkg = LawPackage::new(
            "law:001",
            "Test Law",
            "did:example:author",
            "MIT",
            LawNature::Physical,
            content_hash,
            1700000000,
        );
        pkg.set_signature(signature.to_bytes().to_vec());

        assert!(
            !pkg.verify_signature(&wrong_verifying_key.to_bytes()),
            "signature should NOT verify with wrong public key"
        );
    }

    #[test]
    fn verify_signature_empty_signature_fails() {
        let pkg = LawPackage::new(
            "law:001",
            "Test Law",
            "did:example:author",
            "MIT",
            LawNature::Physical,
            "sha256:abc",
            1700000000,
        );
        assert!(!pkg.verify_signature(&[0u8; 32]), "empty signature should fail");
    }

    #[test]
    fn verify_signature_tampered_content_fails() {
        let signing_key = test_signing_key(1);
        let verifying_key = signing_key.verifying_key();

        let content_hash = "sha256:abc123def456";
        let signature = signing_key.sign(content_hash.as_bytes());

        let mut pkg = LawPackage::new(
            "law:001",
            "Test Law",
            "did:example:author",
            "MIT",
            LawNature::Physical,
            "sha256:TAMPERED",
            1700000000,
        );
        pkg.set_signature(signature.to_bytes().to_vec());

        assert!(
            !pkg.verify_signature(&verifying_key.to_bytes()),
            "tampered content hash should fail verification"
        );
    }
}
