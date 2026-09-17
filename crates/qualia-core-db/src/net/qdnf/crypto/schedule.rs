//! Transcript-bound handshake key schedule (E02.1).

use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::kdf::hkdf_sha384;
use crate::crypto::network::transcript::Transcript;
use crate::crypto::network::types::AEAD_KEY_LEN;
use crate::net::qdnf::types::StrongDigest;
use zeroize::Zeroize;

pub const INFO_I2R: &[u8] = b"qpr-pq-1/qlink/i2r/transcript";
pub const INFO_R2I: &[u8] = b"qpr-pq-1/qlink/r2i/transcript";

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HandshakeKeys {
    pub initiator_to_responder: [u8; AEAD_KEY_LEN],
    pub responder_to_initiator: [u8; AEAD_KEY_LEN],
    pub transcript_digest: StrongDigest,
}

/// Derive directional keys. Transcript digest is the HKDF salt (binding).
pub fn derive_handshake_keys(
    ml_kem_ss: &[u8; 32],
    x25519_ss: &[u8; 32],
    transcript: &Transcript,
) -> Result<HandshakeKeys, CryptoError> {
    if ml_kem_ss.iter().all(|b| *b == 0) || x25519_ss.iter().all(|b| *b == 0) {
        return Err(CryptoError::CryptoFailure);
    }
    let digest = transcript.digest();
    let mut ikm = [0u8; 64];
    ikm[..32].copy_from_slice(ml_kem_ss);
    ikm[32..].copy_from_slice(x25519_ss);
    let mut i2r = [0u8; AEAD_KEY_LEN];
    let mut r2i = [0u8; AEAD_KEY_LEN];
    let i2r_res = hkdf_sha384(Some(&digest.0), &ikm, INFO_I2R, &mut i2r);
    let r2i_res = hkdf_sha384(Some(&digest.0), &ikm, INFO_R2I, &mut r2i);
    ikm.zeroize();
    i2r_res?;
    r2i_res?;
    if i2r == r2i || i2r == [0u8; AEAD_KEY_LEN] || r2i == [0u8; AEAD_KEY_LEN] {
        i2r.zeroize();
        r2i.zeroize();
        return Err(CryptoError::CryptoFailure);
    }
    Ok(HandshakeKeys {
        initiator_to_responder: i2r,
        responder_to_initiator: r2i,
        transcript_digest: digest,
    })
}

pub const INFO_TRAFFIC_SEND: &[u8] = b"qpr-pq-1/traffic-update/send";
pub const INFO_TRAFFIC_RECV: &[u8] = b"qpr-pq-1/traffic-update/recv";

/// Derive the next traffic keys from the current pair. Not a hybrid recovery.
pub fn derive_traffic_update(
    send: &[u8; AEAD_KEY_LEN],
    recv: &[u8; AEAD_KEY_LEN],
    next_generation: u64,
) -> Result<([u8; AEAD_KEY_LEN], [u8; AEAD_KEY_LEN]), CryptoError> {
    if send == &[0u8; AEAD_KEY_LEN] || recv == &[0u8; AEAD_KEY_LEN] || send == recv {
        return Err(CryptoError::CryptoFailure);
    }
    if next_generation == 0 {
        return Err(CryptoError::Unauthorized);
    }
    let mut salt = [0u8; 8];
    salt.copy_from_slice(&next_generation.to_be_bytes());
    let mut ikm = [0u8; 64];
    ikm[..32].copy_from_slice(send);
    ikm[32..].copy_from_slice(recv);
    let mut next_send = [0u8; AEAD_KEY_LEN];
    let mut next_recv = [0u8; AEAD_KEY_LEN];
    let send_res = hkdf_sha384(Some(&salt), &ikm, INFO_TRAFFIC_SEND, &mut next_send);
    let recv_res = hkdf_sha384(Some(&salt), &ikm, INFO_TRAFFIC_RECV, &mut next_recv);
    ikm.zeroize();
    send_res?;
    recv_res?;
    if next_send == next_recv
        || next_send == *send
        || next_recv == *recv
        || next_send == [0u8; AEAD_KEY_LEN]
        || next_recv == [0u8; AEAD_KEY_LEN]
    {
        next_send.zeroize();
        next_recv.zeroize();
        return Err(CryptoError::CryptoFailure);
    }
    Ok((next_send, next_recv))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::transcript::Transcript;

    #[test]
    fn changed_transcript_changes_keys() {
        let kem = [3u8; 32];
        let x = [4u8; 32];
        let mut t1 = Transcript::new();
        t1.append(b"suite", b"qpr-pq-1").unwrap();
        t1.append(b"id", b"a").unwrap();
        let mut t2 = Transcript::new();
        t2.append(b"suite", b"qpr-pq-1").unwrap();
        t2.append(b"id", b"b").unwrap();
        let k1 = derive_handshake_keys(&kem, &x, &t1).unwrap();
        let k2 = derive_handshake_keys(&kem, &x, &t2).unwrap();
        assert_ne!(k1.initiator_to_responder, k2.initiator_to_responder);
        assert_ne!(k1.transcript_digest, k2.transcript_digest);
    }

    #[test]
    fn all_zero_dh_is_rejected() {
        let mut t = Transcript::new();
        t.append(b"suite", b"qpr-pq-1").unwrap();
        assert!(derive_handshake_keys(&[0u8; 32], &[1u8; 32], &t).is_err());
        assert!(derive_handshake_keys(&[1u8; 32], &[0u8; 32], &t).is_err());
    }

    #[test]
    fn traffic_update_changes_keys_and_is_not_identity() {
        let send = [5u8; AEAD_KEY_LEN];
        let recv = [6u8; AEAD_KEY_LEN];
        let (s2, r2) = derive_traffic_update(&send, &recv, 2).unwrap();
        assert_ne!(s2, send);
        assert_ne!(r2, recv);
        assert_ne!(s2, r2);
        let (s3, r3) = derive_traffic_update(&s2, &r2, 3).unwrap();
        assert_ne!(s3, s2);
        assert_ne!(r3, r2);
    }
}
