//! Independent Finished/KDF checks using hmac/hkdf directly, not production helpers.

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha384;

use crate::crypto::network::types::AEAD_KEY_LEN;
use crate::net::qdnf::types::StrongDigest;

type HmacSha384 = Hmac<Sha384>;

const ROLE_I2R: &[u8] = b"qpr-pq-1/finished/i2r";
const ROLE_R2I: &[u8] = b"qpr-pq-1/finished/r2i";

/// Independent directional Finished. Must not match the unkeyed SHA-384 defect.
pub fn independent_finished(
    secret: &[u8; AEAD_KEY_LEN],
    transcript_digest: &StrongDigest,
    initiator_to_responder: bool,
) -> StrongDigest {
    let role = if initiator_to_responder {
        ROLE_I2R
    } else {
        ROLE_R2I
    };
    let mut data = [0u8; 80];
    data[..48].copy_from_slice(&transcript_digest.0);
    let n = role.len().min(32);
    data[48..48 + n].copy_from_slice(&role[..n]);
    let mut mac = HmacSha384::new_from_slice(secret).expect("HMAC key length");
    mac.update(&data[..48 + n]);
    let tag = mac.finalize().into_bytes();
    let mut out = StrongDigest::ZERO;
    out.0.copy_from_slice(&tag);
    out
}

/// Historical defect: SHA-384(transcript || role) with no secret.
pub fn unkeyed_finished_defect(transcript_digest: &StrongDigest, role: &[u8]) -> StrongDigest {
    use sha2::Digest;
    let mut buf = [0u8; 64];
    buf[..48].copy_from_slice(&transcript_digest.0);
    let role_len = role.len().min(16);
    buf[48..48 + role_len].copy_from_slice(&role[..role_len]);
    let hash = Sha384::digest(buf);
    let mut out = StrongDigest::ZERO;
    out.0.copy_from_slice(&hash);
    out
}

/// Transcript binding: identical IKM with different transcripts must differ.
pub fn independent_transcript_bound_ok(
    left: &[u8; AEAD_KEY_LEN],
    right: &[u8; AEAD_KEY_LEN],
) -> bool {
    left != right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyed_finished_differs_from_unkeyed_defect() {
        let digest = StrongDigest([3u8; 48]);
        let secret = [9u8; AEAD_KEY_LEN];
        let keyed = independent_finished(&secret, &digest, true);
        let defect = unkeyed_finished_defect(&digest, b"qpr-pq-1/finished/i2r");
        assert_ne!(keyed, defect);
    }

    #[test]
    fn wrong_secret_changes_finished() {
        let digest = StrongDigest([3u8; 48]);
        let a = independent_finished(&[1u8; AEAD_KEY_LEN], &digest, true);
        let b = independent_finished(&[2u8; AEAD_KEY_LEN], &digest, true);
        assert_ne!(a, b);
    }

    #[test]
    fn independent_finished_matches_production() {
        use crate::net::qdnf::crypto::finished::finished_mac;
        let digest = StrongDigest([3u8; 48]);
        let secret = [9u8; AEAD_KEY_LEN];
        let production = finished_mac(&secret, &digest, true).unwrap();
        assert_eq!(production, independent_finished(&secret, &digest, true));
    }
}
