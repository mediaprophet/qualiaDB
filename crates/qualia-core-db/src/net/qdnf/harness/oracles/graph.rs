//! Independent digest oracle over sha2, not `crypto::network::digest`.

use sha2::{Digest, Sha384};

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

pub fn independent_sha384(data: &[u8]) -> StrongDigest {
    let hash = Sha384::digest(data);
    let mut out = StrongDigest::ZERO;
    out.0.copy_from_slice(&hash);
    out
}

pub fn zero_digest_is_not_identity(digest: StrongDigest) -> Result<(), QdnfError> {
    if digest.is_zero() {
        Err(QdnfError::Unauthorized)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;

    #[test]
    fn independent_digest_matches_production_sha384() {
        let data = b"did:q42:oracle";
        assert_eq!(independent_sha384(data), sha384(data));
    }

    #[test]
    fn zero_digest_is_rejected_as_identity() {
        assert_eq!(
            zero_digest_is_not_identity(StrongDigest::ZERO),
            Err(QdnfError::Unauthorized)
        );
        assert!(zero_digest_is_not_identity(independent_sha384(b"x")).is_ok());
    }
}
