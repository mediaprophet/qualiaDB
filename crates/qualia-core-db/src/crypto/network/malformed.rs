//! CRY-02.13 **PARTIAL** — fail-closed classifier for malformed / truncated /
//! noncanonical / stripped / reordered network crypto.
//!
//! Thin wrappers over caller buffers. No heap. No reimplementation of KEM, DH,
//! or dual-sign. Handshake skip/reorder is `pq_handshake::transition()`.
//! Duplicate chunk identity is [`CryptoError::Replay`] in `chunks::ChunkTable`.
//!
//! | Defect | `CryptoError` |
//! | Truncated slice (`got < need`) | Truncated |
//! | Extra trailing bytes after exact share | Malformed |
//! | Unknown critical tag (nonzero reserved) | CriticalExtension |
//! | Stripped all-zero dual-sig half | Unauthorized |
//! | All-zero DH peer public | CryptoFailure |
//! | KEM ciphertext shorter than ML-KEM-768 | Truncated |
//! | KEM ciphertext longer than ML-KEM-768 | Malformed |
//! | Skip-to-Traffic / 0-RTT | Downgrade |
//! | Reverse handshake flight | Unauthorized |
//! | ChunkTable duplicate append | Replay (`chunks.rs`) |
//!
//! Draft until FND-03 freeze. Does **not** invent a frozen COSE_Sign1 object.
//! Does **not** claim the CRY-02.13 package complete. No security proof.

use crate::crypto::network::dual_sign::DualProof;
use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::pq_handshake::transition;
use crate::crypto::network::types::ML_KEM_768_CT_LEN;
use crate::net::qdnf::crypto::handshake::HandshakeState;

/// Truncation: slice shorter than expected length.
#[inline]
pub fn reject_truncated(got: usize, need: usize) -> Result<(), CryptoError> {
    if got < need {
        Err(CryptoError::Truncated)
    } else {
        Ok(())
    }
}

/// Noncanonical: extra trailing bytes after an exact-length share.
///
/// Overlong QUIC-style varint lives in `session/packet.rs`. Here, a share
/// buffer must be exactly `need` bytes. [`CryptoError::Malformed`] if `got != need`.
#[inline]
pub fn reject_noncanonical_share(got: usize, need: usize) -> Result<(), CryptoError> {
    if got != need {
        Err(CryptoError::Malformed)
    } else {
        Ok(())
    }
}

/// Unknown critical extension tag (nonzero reserved). Tag `0` is the empty set.
#[inline]
pub fn reject_unknown_critical(critical_tag: u16) -> Result<(), CryptoError> {
    if critical_tag != 0 {
        Err(CryptoError::CriticalExtension)
    } else {
        Ok(())
    }
}

#[inline]
fn is_all_zero(bytes: &[u8]) -> bool {
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != 0 {
            return false;
        }
        i += 1;
    }
    true
}

/// Signature stripping: all-zero ML-DSA or Ed25519 half is Unauthorized.
#[inline]
pub fn reject_stripped_dual(proof: &DualProof) -> Result<(), CryptoError> {
    if is_all_zero(&proof.mldsa_sig) || is_all_zero(&proof.ed25519_sig) {
        Err(CryptoError::Unauthorized)
    } else {
        Ok(())
    }
}

/// All-zero DH peer public. Same outcome as [`crate::crypto::network::x25519::X25519Secret`].
#[inline]
pub fn reject_all_zero_dh(peer_pk: &[u8; 32]) -> Result<(), CryptoError> {
    if is_all_zero(peer_pk) {
        Err(CryptoError::CryptoFailure)
    } else {
        Ok(())
    }
}

/// Malformed KEM: wrong ciphertext length.
#[inline]
pub fn reject_malformed_kem_ct(len: usize) -> Result<(), CryptoError> {
    if len < ML_KEM_768_CT_LEN {
        Err(CryptoError::Truncated)
    } else if len > ML_KEM_768_CT_LEN {
        Err(CryptoError::Malformed)
    } else {
        Ok(())
    }
}

/// Reordered handshake flights: fail-closed via `pq_handshake::transition()`.
///
/// Skip-to-Traffic / 0-RTT → [`CryptoError::Downgrade`]. Reverse →
/// [`CryptoError::Unauthorized`]. Other illegal steps → [`CryptoError::Malformed`].
#[inline]
pub fn reject_reordered_flight(
    from: HandshakeState,
    to: HandshakeState,
) -> Result<(), CryptoError> {
    transition(from, to).map(|_| ())
}

/// Fail closed on any classified wire defect. Always true.
#[inline]
pub fn fail_closed_on_malformed() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::dual_sign::{sign_dual, verify_dual};
    use crate::crypto::network::ed25519::public_from_seed;
    use crate::crypto::network::mldsa::generate_keypair;
    use crate::crypto::network::pq_handshake::{
        application_data_allowed, decode_initiator_share, INITIATOR_SHARE_WIRE_LEN,
    };
    use crate::crypto::network::share_encoding::{
        decode_responder_share, encode_responder_share, RESPONDER_SHARE_WIRE_LEN,
    };
    use crate::crypto::network::types::{
        ED25519_SIG_LEN, ML_DSA_65_SIG_LEN, ML_KEM_768_CT_LEN, X25519_LEN,
    };
    use crate::crypto::network::x25519::X25519Secret;

    #[test]
    fn truncated_initiator_share_is_truncated() {
        let need = INITIATOR_SHARE_WIRE_LEN;
        let got = need - 1;
        assert_eq!(reject_truncated(got, need), Err(CryptoError::Truncated));
        let short = [0u8; INITIATOR_SHARE_WIRE_LEN - 1];
        match decode_initiator_share(&short) {
            Err(e) => assert_eq!(e, CryptoError::Truncated),
            Ok(_) => panic!("truncated initiator share decoded"),
        }
        assert_eq!(reject_truncated(need, need), Ok(()));
    }

    #[test]
    fn extra_trailing_byte_on_exact_share_is_malformed() {
        let mut ct = [0u8; ML_KEM_768_CT_LEN];
        let mut pk = [0u8; X25519_LEN];
        ct[0] = 0xC1;
        pk[0] = 0x11;
        let mut wire = [0u8; RESPONDER_SHARE_WIRE_LEN];
        let n = encode_responder_share(&ct, &pk, &mut wire).unwrap();
        assert_eq!(n, RESPONDER_SHARE_WIRE_LEN);
        assert_eq!(
            reject_noncanonical_share(n, RESPONDER_SHARE_WIRE_LEN),
            Ok(())
        );

        let mut extra = [0u8; RESPONDER_SHARE_WIRE_LEN + 1];
        extra[..RESPONDER_SHARE_WIRE_LEN].copy_from_slice(&wire);
        extra[RESPONDER_SHARE_WIRE_LEN] = 0x99;
        assert_eq!(
            reject_noncanonical_share(extra.len(), RESPONDER_SHARE_WIRE_LEN),
            Err(CryptoError::Malformed)
        );
        assert_eq!(decode_responder_share(&extra), Err(CryptoError::Malformed));

        let mut init_extra = [0u8; INITIATOR_SHARE_WIRE_LEN + 1];
        init_extra[INITIATOR_SHARE_WIRE_LEN] = 0xAA;
        match decode_initiator_share(&init_extra) {
            Err(e) => assert_eq!(e, CryptoError::Malformed),
            Ok(_) => panic!("oversize initiator share decoded"),
        }
    }

    #[test]
    fn unknown_critical_tag() {
        assert_eq!(reject_unknown_critical(0), Ok(()));
        assert_eq!(
            reject_unknown_critical(0x99),
            Err(CryptoError::CriticalExtension)
        );
    }

    #[test]
    fn stripped_ed25519_half_is_unauthorized() {
        let mut proof = DualProof {
            mldsa_sig: [0u8; ML_DSA_65_SIG_LEN],
            ed25519_sig: [0u8; ED25519_SIG_LEN],
        };
        proof.mldsa_sig[0] = 0x01;
        assert_eq!(reject_stripped_dual(&proof), Err(CryptoError::Unauthorized));

        let (sk, pk) = generate_keypair().unwrap();
        let seed = [7u8; 32];
        let ed_pk = public_from_seed(&seed);
        sign_dual(&sk, &seed, b"qdnf-malformed", b"qlink", &mut proof).unwrap();
        assert_eq!(reject_stripped_dual(&proof), Ok(()));
        assert!(verify_dual(&pk, &ed_pk, b"qdnf-malformed", b"qlink", &proof).is_ok());

        proof.ed25519_sig = [0u8; ED25519_SIG_LEN];
        assert_eq!(reject_stripped_dual(&proof), Err(CryptoError::Unauthorized));
        assert_eq!(
            verify_dual(&pk, &ed_pk, b"qdnf-malformed", b"qlink", &proof),
            Err(CryptoError::Unauthorized)
        );
    }

    #[test]
    fn all_zero_dh_pk_is_crypto_failure() {
        let zero = [0u8; 32];
        assert_eq!(reject_all_zero_dh(&zero), Err(CryptoError::CryptoFailure));
        let sk = X25519Secret::from_bytes([1u8; 32]);
        assert_eq!(sk.diffie_hellman(&zero), Err(CryptoError::CryptoFailure));

        let peer = X25519Secret::from_bytes([2u8; 32]).public();
        assert_eq!(reject_all_zero_dh(&peer), Ok(()));
        assert!(sk.diffie_hellman(&peer).is_ok());
    }

    #[test]
    fn malformed_kem_ct_length() {
        assert_eq!(reject_malformed_kem_ct(0), Err(CryptoError::Truncated));
        assert_eq!(
            reject_malformed_kem_ct(ML_KEM_768_CT_LEN + 1),
            Err(CryptoError::Malformed)
        );
        assert_eq!(reject_malformed_kem_ct(ML_KEM_768_CT_LEN), Ok(()));
    }

    #[test]
    fn idle_to_traffic_is_downgrade() {
        assert_eq!(
            reject_reordered_flight(HandshakeState::Idle, HandshakeState::Traffic),
            Err(CryptoError::Downgrade)
        );
        assert_eq!(
            transition(HandshakeState::Idle, HandshakeState::Traffic),
            Err(CryptoError::Downgrade)
        );
    }

    #[test]
    fn proofs_to_reachability_is_unauthorized() {
        assert_eq!(
            reject_reordered_flight(HandshakeState::Proofs, HandshakeState::Reachability),
            Err(CryptoError::Unauthorized)
        );
        assert_eq!(
            transition(HandshakeState::Proofs, HandshakeState::Reachability),
            Err(CryptoError::Unauthorized)
        );
    }

    #[test]
    fn fail_closed_on_malformed_is_true() {
        assert!(fail_closed_on_malformed());
    }

    #[test]
    fn zero_rtt_idle_application_data_forbidden() {
        assert!(!application_data_allowed(HandshakeState::Idle));
    }
}
