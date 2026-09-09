//! Share-exchange: initiator ML-KEM public + X25519, responder ciphertext + X25519.

use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::kem::{encapsulate, MlKem768Secret};
use crate::crypto::network::malformed::{reject_all_zero_dh, reject_malformed_kem_ct};
use crate::crypto::network::types::{ML_KEM_768_CT_LEN, ML_KEM_768_PK_LEN, X25519_LEN};
use crate::crypto::network::x25519::X25519Secret;

use super::identity::reject_reflected_share;

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

fn reject_zero(ss: &[u8; 32]) -> Result<(), CryptoError> {
    if ss.iter().all(|b| *b == 0) {
        Err(CryptoError::CryptoFailure)
    } else {
        Ok(())
    }
}

/// Fail-closed checks on the complete path: all-zero peer DH public, KEM
/// ciphertext length, and reflected X25519 publics.
fn reject_complete_failures(
    local_pk: &[u8; 32],
    peer_pk: &[u8; 32],
    kem_ct_len: usize,
) -> Result<(), CryptoError> {
    reject_all_zero_dh(peer_pk)?;
    reject_malformed_kem_ct(kem_ct_len)?;
    reject_reflected_share(local_pk, peer_pk)?;
    Ok(())
}

pub fn initiator_share(
    x25519_secret: &[u8; 32],
    ml_kem_pk: &[u8; ML_KEM_768_PK_LEN],
) -> Result<InitiatorShare, CryptoError> {
    reject_zero(x25519_secret)?;
    let x25519_pk = X25519Secret::from_bytes(*x25519_secret).public();
    reject_all_zero_dh(&x25519_pk)?;
    Ok(InitiatorShare {
        ml_kem_pk: *ml_kem_pk,
        x25519_pk,
    })
}

pub fn responder_complete(
    initiator: &InitiatorShare,
    x25519_secret: &[u8; 32],
) -> Result<(ResponderShare, [u8; 32], [u8; 32]), CryptoError> {
    reject_zero(x25519_secret)?;
    let local_pk = X25519Secret::from_bytes(*x25519_secret).public();
    reject_complete_failures(&local_pk, &initiator.x25519_pk, ML_KEM_768_CT_LEN)?;
    let (kem_ss, ct) = encapsulate(&initiator.ml_kem_pk)?;
    reject_malformed_kem_ct(ct.len())?;
    reject_zero(&kem_ss)?;
    let x_ss = X25519Secret::from_bytes(*x25519_secret).diffie_hellman(&initiator.x25519_pk)?;
    reject_zero(&x_ss)?;
    let share = ResponderShare {
        ml_kem_ct: ct,
        x25519_pk: local_pk,
    };
    Ok((share, kem_ss, x_ss))
}

pub fn initiator_complete(
    ml_kem_sk: &MlKem768Secret,
    x25519_secret: &[u8; 32],
    responder: &ResponderShare,
) -> Result<([u8; 32], [u8; 32]), CryptoError> {
    reject_zero(x25519_secret)?;
    let local_pk = X25519Secret::from_bytes(*x25519_secret).public();
    reject_complete_failures(&local_pk, &responder.x25519_pk, responder.ml_kem_ct.len())?;
    let kem_ss = ml_kem_sk.decapsulate(&responder.ml_kem_ct)?;
    reject_zero(&kem_ss)?;
    let x_ss = X25519Secret::from_bytes(*x25519_secret).diffie_hellman(&responder.x25519_pk)?;
    reject_zero(&x_ss)?;
    Ok((kem_ss, x_ss))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::kem::MlKem768Secret;
    use crate::crypto::network::transcript::Transcript;
    use crate::net::qdnf::crypto::handshake::{derive_handshake_keys, HandshakeKeys};

    #[test]
    fn hybrid_handshake_agrees() {
        let (sk, pk) = MlKem768Secret::generate().unwrap();
        let i_x = [11u8; 32];
        let r_x = [12u8; 32];
        let ishare = initiator_share(&i_x, &pk).unwrap();
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
        let _ = HandshakeKeys {
            initiator_to_responder: k1.initiator_to_responder,
            responder_to_initiator: k1.responder_to_initiator,
            transcript_digest: k1.transcript_digest,
        };
    }

    #[test]
    fn zero_x25519_secret_is_rejected() {
        let (_sk, pk) = MlKem768Secret::generate().unwrap();
        assert!(initiator_share(&[0u8; 32], &pk).is_err());
    }
}
