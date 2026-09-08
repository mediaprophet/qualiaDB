//! Hybrid qpr-pq-1 handshake: ML-KEM-768 then X25519 into HKDF-SHA-384.

use crate::crypto::network::digest::sha384;
use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::kdf::hybrid_shared_secret;
use crate::crypto::network::kem::{encapsulate, MlKem768Secret};
use crate::crypto::network::transcript::Transcript;
use crate::crypto::network::types::{
    AEAD_KEY_LEN, ML_KEM_768_CT_LEN, ML_KEM_768_PK_LEN, X25519_LEN,
};
use crate::crypto::network::x25519::X25519Secret;
use crate::net::qdnf::types::StrongDigest;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandshakeState {
    Idle = 0,
    Reachability = 1,
    SharesExchanged = 2,
    HandshakeKeys = 3,
    Proofs = 4,
    Finished = 5,
    Traffic = 6,
}

pub const FORBIDDEN_ZERO_RTT: bool = true;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct InitiatorShare {
    pub ml_kem_pk: [u8; ML_KEM_768_PK_LEN],
    pub x25519_pk: [u8; X25519_LEN],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ResponderShare {
    pub ml_kem_ct: [u8; ML_KEM_768_CT_LEN],
    pub x25519_pk: [u8; X25519_LEN],
}

pub struct HandshakeKeys {
    pub initiator_to_responder: [u8; AEAD_KEY_LEN],
    pub responder_to_initiator: [u8; AEAD_KEY_LEN],
    pub transcript_digest: StrongDigest,
}

/// Derive directional handshake keys after shares. Proofs are encrypted after this.
pub fn derive_handshake_keys(
    ml_kem_ss: &[u8; 32],
    x25519_ss: &[u8; 32],
    transcript: &Transcript,
) -> Result<HandshakeKeys, CryptoError> {
    let digest = transcript.digest();
    let mut i2r = [0u8; AEAD_KEY_LEN];
    let mut r2i = [0u8; AEAD_KEY_LEN];
    hybrid_shared_secret(ml_kem_ss, x25519_ss, b"qpr-pq-1/qlink/i2r", &mut i2r)?;
    hybrid_shared_secret(ml_kem_ss, x25519_ss, b"qpr-pq-1/qlink/r2i", &mut r2i)?;
    Ok(HandshakeKeys {
        initiator_to_responder: i2r,
        responder_to_initiator: r2i,
        transcript_digest: digest,
    })
}

pub fn initiator_share(
    x25519_secret: &[u8; 32],
    ml_kem_pk: &[u8; ML_KEM_768_PK_LEN],
) -> InitiatorShare {
    InitiatorShare {
        ml_kem_pk: *ml_kem_pk,
        x25519_pk: X25519Secret::from_bytes(*x25519_secret).public(),
    }
}

pub fn responder_complete(
    initiator: &InitiatorShare,
    x25519_secret: &[u8; 32],
) -> Result<(ResponderShare, [u8; 32], [u8; 32]), CryptoError> {
    let (kem_ss, ct) = encapsulate(&initiator.ml_kem_pk)?;
    let x_ss = X25519Secret::from_bytes(*x25519_secret).diffie_hellman(&initiator.x25519_pk)?;
    let share = ResponderShare {
        ml_kem_ct: ct,
        x25519_pk: X25519Secret::from_bytes(*x25519_secret).public(),
    };
    Ok((share, kem_ss, x_ss))
}

pub fn initiator_complete(
    ml_kem_sk: &MlKem768Secret,
    x25519_secret: &[u8; 32],
    responder: &ResponderShare,
) -> Result<([u8; 32], [u8; 32]), CryptoError> {
    let kem_ss = ml_kem_sk.decapsulate(&responder.ml_kem_ct)?;
    let x_ss = X25519Secret::from_bytes(*x25519_secret).diffie_hellman(&responder.x25519_pk)?;
    Ok((kem_ss, x_ss))
}

pub fn finished_mac(transcript_digest: &StrongDigest, role: &[u8]) -> StrongDigest {
    let mut buf = [0u8; 64];
    buf[..48].copy_from_slice(&transcript_digest.0);
    let role_len = role.len().min(16);
    buf[48..48 + role_len].copy_from_slice(&role[..role_len]);
    sha384(&buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::kem::MlKem768Secret;

    #[test]
    fn hybrid_handshake_agrees() {
        let (sk, pk) = MlKem768Secret::generate().unwrap();
        let i_x = [11u8; 32];
        let r_x = [12u8; 32];
        let ishare = initiator_share(&i_x, &pk);
        let (rshare, r_kem, r_dh) = responder_complete(&ishare, &r_x).unwrap();
        let (i_kem, i_dh) = initiator_complete(&sk, &i_x, &rshare).unwrap();
        assert_eq!(i_kem, r_kem);
        assert_eq!(i_dh, r_dh);
        let mut t = Transcript::new();
        t.append(b"suite", b"qpr-pq-1").unwrap();
        let k1 = derive_handshake_keys(&i_kem, &i_dh, &t).unwrap();
        let k2 = derive_handshake_keys(&r_kem, &r_dh, &t).unwrap();
        assert_eq!(k1.initiator_to_responder, k2.initiator_to_responder);
        assert_ne!(k1.initiator_to_responder, k1.responder_to_initiator);
        assert!(FORBIDDEN_ZERO_RTT);
        assert_eq!(HandshakeState::Idle as u8, 0);
    }
}
