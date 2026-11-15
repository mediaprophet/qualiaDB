//! Directional secret-derived Finished confirmation (E02.2).

use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::kdf::{hmac_sha384, hmac_sha384_verify};
use crate::crypto::network::types::AEAD_KEY_LEN;
use crate::net::qdnf::types::StrongDigest;

pub const ROLE_I2R: &[u8] = b"qpr-pq-1/finished/i2r";
pub const ROLE_R2I: &[u8] = b"qpr-pq-1/finished/r2i";

fn finished_data(transcript_digest: &StrongDigest, role: &[u8], out: &mut [u8; 80]) -> usize {
    out[..48].copy_from_slice(&transcript_digest.0);
    let n = role.len().min(32);
    out[48..48 + n].copy_from_slice(&role[..n]);
    48 + n
}

/// Finished MAC over the transcript.
///
/// Rejects all-zero secrets and unauthenticated zero transcript digests (`R03`).
pub fn finished_mac(
    secret: &[u8; AEAD_KEY_LEN],
    transcript_digest: &StrongDigest,
    initiator_to_responder: bool,
) -> Result<StrongDigest, CryptoError> {
    if secret == &[0u8; AEAD_KEY_LEN] || *transcript_digest == StrongDigest::ZERO {
        return Err(CryptoError::CryptoFailure);
    }
    let role = if initiator_to_responder {
        ROLE_I2R
    } else {
        ROLE_R2I
    };
    let mut data = [0u8; 80];
    let len = finished_data(transcript_digest, role, &mut data);
    let mut out = [0u8; 48];
    hmac_sha384(secret, &data[..len], &mut out)?;
    Ok(StrongDigest(out))
}

/// Verifies `finished`.
pub fn verify_finished(
    secret: &[u8; AEAD_KEY_LEN],
    transcript_digest: &StrongDigest,
    initiator_to_responder: bool,
    tag: &StrongDigest,
) -> Result<(), CryptoError> {
    if secret == &[0u8; AEAD_KEY_LEN]
        || *transcript_digest == StrongDigest::ZERO
        || *tag == StrongDigest::ZERO
    {
        return Err(CryptoError::CryptoFailure);
    }
    let role = if initiator_to_responder {
        ROLE_I2R
    } else {
        ROLE_R2I
    };
    let mut data = [0u8; 80];
    let len = finished_data(transcript_digest, role, &mut data);
    hmac_sha384_verify(secret, &data[..len], &tag.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_secret_fails_confirmation() {
        let digest = StrongDigest([7u8; 48]);
        let secret_a = [1u8; AEAD_KEY_LEN];
        let secret_b = [2u8; AEAD_KEY_LEN];
        let tag = finished_mac(&secret_a, &digest, true).unwrap();
        assert!(verify_finished(&secret_a, &digest, true, &tag).is_ok());
        assert!(verify_finished(&secret_b, &digest, true, &tag).is_err());
    }

    #[test]
    fn equal_transcripts_wrong_role_fail() {
        let digest = StrongDigest([7u8; 48]);
        let secret = [9u8; AEAD_KEY_LEN];
        let tag = finished_mac(&secret, &digest, true).unwrap();
        assert!(verify_finished(&secret, &digest, false, &tag).is_err());
        let other = finished_mac(&secret, &digest, false).unwrap();
        assert_ne!(tag, other);
    }

    #[test]
    fn changed_transcript_fails() {
        let secret = [9u8; AEAD_KEY_LEN];
        let tag = finished_mac(&secret, &StrongDigest([1u8; 48]), true).unwrap();
        assert!(verify_finished(&secret, &StrongDigest([2u8; 48]), true, &tag).is_err());
    }

    #[test]
    fn r03_zero_secret_or_zero_transcript_digest_is_rejected() {
        let digest = StrongDigest([7u8; 48]);
        let zero_secret = [0u8; AEAD_KEY_LEN];
        let valid_secret = [5u8; AEAD_KEY_LEN];
        assert!(finished_mac(&zero_secret, &digest, true).is_err());
        assert!(finished_mac(&valid_secret, &StrongDigest::ZERO, true).is_err());
        let tag = finished_mac(&valid_secret, &digest, true).unwrap();
        assert!(verify_finished(&zero_secret, &digest, true, &tag).is_err());
        assert!(verify_finished(&valid_secret, &StrongDigest::ZERO, true, &tag).is_err());
        assert!(verify_finished(&valid_secret, &digest, true, &StrongDigest::ZERO).is_err());
    }
}
