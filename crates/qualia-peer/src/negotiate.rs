//! E20.3 capability/version negotiation, anti-rollback, signed update bytes.
//!
//! A boolean `signed=true` is not an API and is not accepted as proof.

use qualia_core_db::crypto::network::digest::sha384;
use qualia_core_db::crypto::network::transcript::Transcript;
use qualia_core_db::net::qdnf::errors::QdnfError;
use qualia_core_db::net::qdnf::types::{Generation, StrongDigest};

/// Pinned capability identity: SHA-384 transcript digest plus generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapabilityVersion {
    pub digest: StrongDigest,
    pub generation: Generation,
}

/// Pin a capability to a version digest. Empty id or generation 0 is rejected.
pub fn pin_capability_version(
    capability: &[u8],
    generation: Generation,
) -> Result<CapabilityVersion, QdnfError> {
    if capability.is_empty() {
        return Err(QdnfError::Malformed);
    }
    if generation == Generation::ZERO {
        return Err(QdnfError::Unauthorized);
    }
    let mut t = Transcript::new();
    t.append(b"qdnf-cap-version", capability)?;
    t.append(b"generation", &generation.0.to_be_bytes())?;
    Ok(CapabilityVersion {
        digest: t.digest(),
        generation,
    })
}

/// Anti-rollback: an offered generation older than `min` is StaleGeneration.
pub fn admit_capability_generation(offered: Generation, min: Generation) -> Result<(), QdnfError> {
    if min == Generation::ZERO || offered == Generation::ZERO {
        return Err(QdnfError::Unauthorized);
    }
    if offered.0 < min.0 {
        return Err(QdnfError::StaleGeneration);
    }
    Ok(())
}

/// Admit a peer only when generation is current and the pinned digest matches.
pub fn negotiate_capability(
    local: CapabilityVersion,
    peer_generation: Generation,
    peer_digest: StrongDigest,
) -> Result<StrongDigest, QdnfError> {
    if local.digest.is_zero() || peer_digest.is_zero() {
        return Err(QdnfError::Malformed);
    }
    admit_capability_generation(peer_generation, local.generation)?;
    if peer_digest != local.digest {
        return Err(QdnfError::Conflict);
    }
    Ok(local.digest)
}

/// Signed update metadata: exact SHA-384 of the bytes. No `signed` flag.
pub fn verify_update_bytes(expected: StrongDigest, bytes: &[u8]) -> Result<(), QdnfError> {
    if bytes.is_empty() || expected.is_zero() {
        return Err(QdnfError::Malformed);
    }
    let got = sha384(bytes);
    if got != expected {
        return Err(QdnfError::Conflict);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_is_digest_of_capability_and_generation() {
        let a = pin_capability_version(b"qpr-v1", Generation(3)).unwrap();
        let b = pin_capability_version(b"qpr-v1", Generation(3)).unwrap();
        let other_id = pin_capability_version(b"qpr-v2", Generation(3)).unwrap();
        let other_gen = pin_capability_version(b"qpr-v1", Generation(4)).unwrap();
        assert_eq!(a.digest, b.digest);
        assert_ne!(a.digest, StrongDigest::ZERO);
        assert_ne!(a.digest, other_id.digest);
        assert_ne!(a.digest, other_gen.digest);
        assert_eq!(
            pin_capability_version(b"", Generation(1)),
            Err(QdnfError::Malformed)
        );
        assert_eq!(
            pin_capability_version(b"qpr-v1", Generation::ZERO),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn older_generation_is_stale_not_admitted() {
        let min = Generation(5);
        assert_eq!(
            admit_capability_generation(Generation(4), min),
            Err(QdnfError::StaleGeneration)
        );
        assert!(admit_capability_generation(Generation(5), min).is_ok());
        assert!(admit_capability_generation(Generation(6), min).is_ok());
        assert_eq!(
            admit_capability_generation(Generation::ZERO, min),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn negotiate_requires_matching_digest_and_current_generation() {
        let local = pin_capability_version(b"qpr-v1", Generation(2)).unwrap();
        assert_eq!(
            negotiate_capability(local, Generation(1), local.digest),
            Err(QdnfError::StaleGeneration)
        );
        let other = pin_capability_version(b"qpr-v2", Generation(2)).unwrap();
        assert_eq!(
            negotiate_capability(local, Generation(2), other.digest),
            Err(QdnfError::Conflict)
        );
        assert_eq!(
            negotiate_capability(local, Generation(2), local.digest).unwrap(),
            local.digest
        );
    }

    #[test]
    fn update_metadata_requires_exact_bytes_hash() {
        let bytes = b"qdnf-update-artifact-v1";
        let digest = sha384(bytes);
        assert!(verify_update_bytes(digest, bytes).is_ok());
        assert_eq!(
            verify_update_bytes(digest, b"tampered-artifact"),
            Err(QdnfError::Conflict)
        );
        assert_eq!(verify_update_bytes(digest, b""), Err(QdnfError::Malformed));
        assert_eq!(
            verify_update_bytes(StrongDigest::ZERO, bytes),
            Err(QdnfError::Malformed)
        );
        let matching_wrong = sha384(b"other-bytes");
        assert_eq!(
            verify_update_bytes(matching_wrong, bytes),
            Err(QdnfError::Conflict)
        );
    }
}
