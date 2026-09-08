//! CRY-02 PQ handshake transitions over the existing `HandshakeState` FSM.
//!
//! Draft until FND-03 freeze: initiator share wire bytes are exactly
//! `ml_kem_pk || x25519_pk` with no version prefix. Responder share encoding,
//! length-delimited transcripts, dual proofs, and CRY-02.12 independent
//! vectors remain open. No security proof is claimed. Live KEM encapsulate
//! is not exercised here (RNG/provider); encodings + FSM + 0-RTT only.
//!
//! Allowed successive states:
//! `Idle → Reachability → SharesExchanged → HandshakeKeys → Proofs → Finished → Traffic`

use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::types::{ML_KEM_768_PK_LEN, X25519_LEN};
use crate::net::qdnf::crypto::handshake::{
    HandshakeState, InitiatorShare, FORBIDDEN_ZERO_RTT,
};

/// Exact initiator-share slice length: `ML_KEM_768_PK_LEN || X25519_LEN`.
pub const INITIATOR_SHARE_WIRE_LEN: usize = ML_KEM_768_PK_LEN + X25519_LEN;

const _: () = assert!(FORBIDDEN_ZERO_RTT);

/// QLink HKDF/domain label. Distinct from [`qsession_domain`].
pub fn qlink_domain() -> &'static [u8] {
    b"qpr-pq-1/qlink"
}

/// QSession HKDF/domain label. Distinct from [`qlink_domain`].
pub fn qsession_domain() -> &'static [u8] {
    b"qpr-pq-1/qsession"
}

/// CRY-02.06 partial: classical-only controller/delegation is never sufficient.
pub fn classical_only_authority_is_enough() -> bool {
    false
}

/// Application data is admitted only in `Traffic`. Initial-profile 0-RTT is false.
pub fn application_data_allowed(state: HandshakeState) -> bool {
    debug_assert!(FORBIDDEN_ZERO_RTT);
    state == HandshakeState::Traffic
}

/// Advance `from` to `to` only when `to` is the immediate successor.
///
/// Forbidden: skip, reverse, stay, `Idle → Traffic`, or any `→ Traffic` before `Finished`.
/// Skip-to-traffic / 0-RTT attempts map to [`CryptoError::Downgrade`]. Reverse maps to
/// [`CryptoError::Unauthorized`]. Other illegal steps map to [`CryptoError::Malformed`].
pub fn transition(
    from: HandshakeState,
    to: HandshakeState,
) -> Result<HandshakeState, CryptoError> {
    if successor(from) == Some(to) {
        return Ok(to);
    }
    if to == HandshakeState::Traffic {
        return Err(CryptoError::Downgrade);
    }
    if (to as u8) < (from as u8) {
        return Err(CryptoError::Unauthorized);
    }
    Err(CryptoError::Malformed)
}

#[inline]
fn successor(from: HandshakeState) -> Option<HandshakeState> {
    match from {
        HandshakeState::Idle => Some(HandshakeState::Reachability),
        HandshakeState::Reachability => Some(HandshakeState::SharesExchanged),
        HandshakeState::SharesExchanged => Some(HandshakeState::HandshakeKeys),
        HandshakeState::HandshakeKeys => Some(HandshakeState::Proofs),
        HandshakeState::Proofs => Some(HandshakeState::Finished),
        HandshakeState::Finished => Some(HandshakeState::Traffic),
        HandshakeState::Traffic => None,
    }
}

/// Encode initiator share as `ml_kem_pk || x25519_pk` (draft; no version byte).
pub fn encode_initiator_share(
    share: &InitiatorShare,
    out: &mut [u8],
) -> Result<usize, CryptoError> {
    if out.len() < INITIATOR_SHARE_WIRE_LEN {
        return Err(CryptoError::Capacity);
    }
    out[..ML_KEM_768_PK_LEN].copy_from_slice(&share.ml_kem_pk);
    out[ML_KEM_768_PK_LEN..INITIATOR_SHARE_WIRE_LEN].copy_from_slice(&share.x25519_pk);
    Ok(INITIATOR_SHARE_WIRE_LEN)
}

/// Decode initiator share from `ml_kem_pk || x25519_pk`. Truncated `src` fails.
/// Extra bytes are [`CryptoError::Malformed`]. Does not call `initiator_share`.
pub fn decode_initiator_share(src: &[u8]) -> Result<InitiatorShare, CryptoError> {
    if src.len() < INITIATOR_SHARE_WIRE_LEN {
        return Err(CryptoError::Truncated);
    }
    if src.len() != INITIATOR_SHARE_WIRE_LEN {
        return Err(CryptoError::Malformed);
    }
    let mut share = InitiatorShare {
        ml_kem_pk: [0u8; ML_KEM_768_PK_LEN],
        x25519_pk: [0u8; X25519_LEN],
    };
    share.ml_kem_pk.copy_from_slice(&src[..ML_KEM_768_PK_LEN]);
    share.x25519_pk.copy_from_slice(&src[ML_KEM_768_PK_LEN..]);
    Ok(share)
}

/// Concatenate ML-KEM shared secret then X25519 shared secret. Order is frozen.
pub fn ml_kem_then_x25519_secret_input(
    ml_kem_ss: &[u8; 32],
    x25519_ss: &[u8; 32],
    out: &mut [u8; 64],
) {
    out[..32].copy_from_slice(ml_kem_ss);
    out[32..].copy_from_slice(x25519_ss);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::x25519::X25519Secret;
    use crate::net::qdnf::crypto::handshake::initiator_share;

    /// Fixed test-only ML-KEM public bytes. Not a production key; not generated.
    fn test_ml_kem_pk() -> [u8; ML_KEM_768_PK_LEN] {
        let mut pk = [0u8; ML_KEM_768_PK_LEN];
        let mut i = 0usize;
        while i < ML_KEM_768_PK_LEN {
            pk[i] = (i as u8).wrapping_add(0x5A);
            i += 1;
        }
        pk[0] = 0x11;
        pk[ML_KEM_768_PK_LEN - 1] = 0xEE;
        pk
    }

    const TEST_X25519_SK: [u8; 32] = [
        0x42, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
        0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B,
        0x1C, 0x1D, 0x1E, 0x1F,
    ];

    fn test_share_from_fixed_keys() -> InitiatorShare {
        InitiatorShare {
            ml_kem_pk: test_ml_kem_pk(),
            x25519_pk: X25519Secret::from_bytes(TEST_X25519_SK).public(),
        }
    }

    #[test]
    fn successive_transitions_ok() {
        let chain = [
            (HandshakeState::Idle, HandshakeState::Reachability),
            (HandshakeState::Reachability, HandshakeState::SharesExchanged),
            (HandshakeState::SharesExchanged, HandshakeState::HandshakeKeys),
            (HandshakeState::HandshakeKeys, HandshakeState::Proofs),
            (HandshakeState::Proofs, HandshakeState::Finished),
            (HandshakeState::Finished, HandshakeState::Traffic),
        ];
        let mut i = 0usize;
        while i < chain.len() {
            let (from, to) = chain[i];
            assert_eq!(transition(from, to), Ok(to));
            i += 1;
        }
    }

    #[test]
    fn skip_shares_to_proofs_forbidden() {
        assert_eq!(
            transition(HandshakeState::SharesExchanged, HandshakeState::Proofs),
            Err(CryptoError::Malformed)
        );
    }

    #[test]
    fn reverse_and_skip_to_traffic_forbidden() {
        assert_eq!(
            transition(HandshakeState::Proofs, HandshakeState::HandshakeKeys),
            Err(CryptoError::Unauthorized)
        );
        assert_eq!(
            transition(HandshakeState::Idle, HandshakeState::Traffic),
            Err(CryptoError::Downgrade)
        );
        assert_eq!(
            transition(HandshakeState::SharesExchanged, HandshakeState::Traffic),
            Err(CryptoError::Downgrade)
        );
        assert_eq!(
            transition(HandshakeState::Proofs, HandshakeState::Traffic),
            Err(CryptoError::Downgrade)
        );
        assert_eq!(
            transition(HandshakeState::Idle, HandshakeState::Idle),
            Err(CryptoError::Malformed)
        );
    }

    #[test]
    fn zero_rtt_application_data_forbidden() {
        assert!(FORBIDDEN_ZERO_RTT);
        assert!(!application_data_allowed(HandshakeState::Idle));
        assert!(!application_data_allowed(HandshakeState::Reachability));
        assert!(!application_data_allowed(HandshakeState::SharesExchanged));
        assert!(!application_data_allowed(HandshakeState::HandshakeKeys));
        assert!(!application_data_allowed(HandshakeState::Proofs));
        assert!(!application_data_allowed(HandshakeState::Finished));
        assert!(application_data_allowed(HandshakeState::Traffic));
    }

    #[test]
    fn encode_decode_share_round_trip_independent_of_initiator_share() {
        let share = test_share_from_fixed_keys();
        let mut wire = [0u8; INITIATOR_SHARE_WIRE_LEN];
        let n = encode_initiator_share(&share, &mut wire).unwrap();
        assert_eq!(n, INITIATOR_SHARE_WIRE_LEN);
        assert_eq!(&wire[..ML_KEM_768_PK_LEN], &share.ml_kem_pk);
        assert_eq!(&wire[ML_KEM_768_PK_LEN..], &share.x25519_pk);

        let decoded = decode_initiator_share(&wire).unwrap();
        assert_eq!(decoded.ml_kem_pk, share.ml_kem_pk);
        assert_eq!(decoded.x25519_pk, share.x25519_pk);

        let from_helper = initiator_share(&TEST_X25519_SK, &share.ml_kem_pk);
        assert_eq!(from_helper.x25519_pk, share.x25519_pk);
        assert_eq!(from_helper.ml_kem_pk, share.ml_kem_pk);
    }

    #[test]
    fn encode_capacity_and_decode_truncated() {
        let share = test_share_from_fixed_keys();
        let mut short = [0u8; 8];
        assert_eq!(
            encode_initiator_share(&share, &mut short),
            Err(CryptoError::Capacity)
        );
        let mut wire = [0u8; INITIATOR_SHARE_WIRE_LEN];
        encode_initiator_share(&share, &mut wire).unwrap();
        match decode_initiator_share(&wire[..INITIATOR_SHARE_WIRE_LEN - 1]) {
            Err(e) => assert_eq!(e, CryptoError::Truncated),
            Ok(_) => panic!("truncated share decoded"),
        }
        let mut extra = [0u8; INITIATOR_SHARE_WIRE_LEN + 1];
        extra[..INITIATOR_SHARE_WIRE_LEN].copy_from_slice(&wire);
        match decode_initiator_share(&extra) {
            Err(e) => assert_eq!(e, CryptoError::Malformed),
            Ok(_) => panic!("oversize share decoded"),
        }
    }

    #[test]
    fn ml_kem_then_x25519_order() {
        let kem = [0xAAu8; 32];
        let x = [0x55u8; 32];
        let mut out = [0u8; 64];
        ml_kem_then_x25519_secret_input(&kem, &x, &mut out);
        assert_eq!(&out[..32], &kem);
        assert_eq!(&out[32..], &x);
        assert_ne!(&out[..32], &out[32..]);
    }

    #[test]
    fn domains_are_distinct() {
        assert_ne!(qlink_domain(), qsession_domain());
        assert_eq!(qlink_domain(), b"qpr-pq-1/qlink");
        assert_eq!(qsession_domain(), b"qpr-pq-1/qsession");
    }

    #[test]
    fn classical_only_authority_rejected() {
        assert!(!classical_only_authority_is_enough());
    }
}
