//! Exact-byte digest checks. A matching digest without the bytes is not a signature.

use crate::crypto::network::digest::sha384;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Verify that `original_bytes` are present and hash to `digest`.
///
/// Empty input is [`QdnfError::Malformed`] (an attacker cannot pass a digest
/// without the bytes). A zero digest is malformed. A live hash mismatch is
/// [`QdnfError::Conflict`] (forged digest).
pub fn verify_exact_bytes(digest: StrongDigest, original_bytes: &[u8]) -> Result<(), QdnfError> {
    if original_bytes.is_empty() {
        return Err(QdnfError::Malformed);
    }
    if digest.is_zero() {
        return Err(QdnfError::Malformed);
    }
    let got = sha384(original_bytes);
    if got != digest {
        return Err(QdnfError::Conflict);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;

    #[test]
    fn empty_bytes_are_malformed_even_with_matching_digest() {
        let digest = sha384(b"contract-bytes");
        assert_eq!(verify_exact_bytes(digest, b""), Err(QdnfError::Malformed));
    }

    #[test]
    fn zero_digest_is_malformed() {
        assert_eq!(
            verify_exact_bytes(StrongDigest::ZERO, b"present"),
            Err(QdnfError::Malformed)
        );
    }

    #[test]
    fn hash_mismatch_is_conflict() {
        let digest = sha384(b"original");
        assert_eq!(
            verify_exact_bytes(digest, b"tampered"),
            Err(QdnfError::Conflict)
        );
    }

    #[test]
    fn matching_bytes_verify() {
        let bytes = b"pinned-artifact";
        assert!(verify_exact_bytes(sha384(bytes), bytes).is_ok());
    }
}
