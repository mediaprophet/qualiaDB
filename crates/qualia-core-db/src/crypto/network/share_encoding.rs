//! CRY-02.03 **PARTIAL** — freeze responder-share wire encoding and ML-KEM-then-X25519 IKM.
//!
//! Draft until FND-03 freeze: responder share wire bytes are exactly
//! `ml_kem_ct || x25519_pk` with **no version prefix**. Initiator share encoding
//! lives in `pq_handshake`; this module owns the responder half and the hybrid
//! IKM concat used as HKDF input.
//!
//! CRY-02.12 remainder: a **live** ML-KEM-768 encapsulate/decapsulate round-trip
//! is exercised in tests. That ciphertext is **not a frozen hex vector** —
//! `encapsulate` draws OS RNG, so ct bytes change every run. Do not treat live
//! KEM ciphertext as a conformance fixture. Dual proofs, record vectors, and
//! length-delimited transcripts remain open. No security proof is claimed.
//!
//! Crypto stays below `net::qdnf`: this module uses raw arrays only.

use super::errors::CryptoError;
use super::types::{ML_KEM_768_CT_LEN, ML_KEM_SS_LEN, X25519_LEN};

/// Exact responder-share slice length: `ML_KEM_768_CT_LEN || X25519_LEN`.
pub const RESPONDER_SHARE_WIRE_LEN: usize = ML_KEM_768_CT_LEN + X25519_LEN;

const _: () = assert!(ML_KEM_SS_LEN + X25519_LEN == 64);
const _: () = assert!(RESPONDER_SHARE_WIRE_LEN == 1088 + 32);

/// Encode responder share as `ml_kem_ct || x25519_pk` (draft; no version byte).
///
/// Writes exactly [`RESPONDER_SHARE_WIRE_LEN`] bytes. [`CryptoError::Capacity`]
/// if `out` is shorter than that.
pub fn encode_responder_share(
    ml_kem_ct: &[u8; ML_KEM_768_CT_LEN],
    x25519_pk: &[u8; X25519_LEN],
    out: &mut [u8],
) -> Result<usize, CryptoError> {
    if out.len() < RESPONDER_SHARE_WIRE_LEN {
        return Err(CryptoError::Capacity);
    }
    out[..ML_KEM_768_CT_LEN].copy_from_slice(ml_kem_ct);
    out[ML_KEM_768_CT_LEN..RESPONDER_SHARE_WIRE_LEN].copy_from_slice(x25519_pk);
    Ok(RESPONDER_SHARE_WIRE_LEN)
}

/// Decode responder share from `ml_kem_ct || x25519_pk`.
///
/// [`CryptoError::Truncated`] if `src.len() < RESPONDER_SHARE_WIRE_LEN`.
/// Extra bytes are [`CryptoError::Malformed`]. Exact length only.
pub fn decode_responder_share(
    src: &[u8],
) -> Result<([u8; ML_KEM_768_CT_LEN], [u8; X25519_LEN]), CryptoError> {
    if src.len() < RESPONDER_SHARE_WIRE_LEN {
        return Err(CryptoError::Truncated);
    }
    if src.len() != RESPONDER_SHARE_WIRE_LEN {
        return Err(CryptoError::Malformed);
    }
    let mut ct = [0u8; ML_KEM_768_CT_LEN];
    let mut pk = [0u8; X25519_LEN];
    ct.copy_from_slice(&src[..ML_KEM_768_CT_LEN]);
    pk.copy_from_slice(&src[ML_KEM_768_CT_LEN..]);
    Ok((ct, pk))
}

/// Concatenate ML-KEM shared secret then X25519 shared secret (HKDF IKM order).
pub fn hybrid_ikm(
    ml_kem_ss: &[u8; ML_KEM_SS_LEN],
    x25519_ss: &[u8; X25519_LEN],
    out: &mut [u8; 64],
) {
    out[..ML_KEM_SS_LEN].copy_from_slice(ml_kem_ss);
    out[ML_KEM_SS_LEN..].copy_from_slice(x25519_ss);
}

/// Draft encoding has no version prefix on responder-share wire bytes.
pub fn responder_share_has_version_prefix() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::kem::{encapsulate, MlKem768Secret};

    fn patterned_ct() -> [u8; ML_KEM_768_CT_LEN] {
        let mut ct = [0u8; ML_KEM_768_CT_LEN];
        let mut i = 0usize;
        while i < ML_KEM_768_CT_LEN {
            ct[i] = (i as u8).wrapping_add(0xA5);
            i += 1;
        }
        ct[0] = 0xC1;
        ct[ML_KEM_768_CT_LEN - 1] = 0xC2;
        ct
    }

    fn patterned_x25519_pk() -> [u8; X25519_LEN] {
        let mut pk = [0u8; X25519_LEN];
        let mut i = 0usize;
        while i < X25519_LEN {
            pk[i] = (i as u8).wrapping_add(0x5A);
            i += 1;
        }
        pk[0] = 0x11;
        pk[X25519_LEN - 1] = 0xEE;
        pk
    }

    #[test]
    fn encode_decode_patterned_round_trip() {
        let ct = patterned_ct();
        let pk = patterned_x25519_pk();
        let mut wire = [0u8; RESPONDER_SHARE_WIRE_LEN];
        let n = encode_responder_share(&ct, &pk, &mut wire).unwrap();
        assert_eq!(n, RESPONDER_SHARE_WIRE_LEN);
        assert_eq!(&wire[..ML_KEM_768_CT_LEN], &ct);
        assert_eq!(&wire[ML_KEM_768_CT_LEN..], &pk);

        let (decoded_ct, decoded_pk) = decode_responder_share(&wire).unwrap();
        assert_eq!(decoded_ct, ct);
        assert_eq!(decoded_pk, pk);
    }

    #[test]
    fn truncated_malformed_and_capacity() {
        let ct = patterned_ct();
        let pk = patterned_x25519_pk();
        let mut short = [0u8; 8];
        assert_eq!(
            encode_responder_share(&ct, &pk, &mut short),
            Err(CryptoError::Capacity)
        );

        let mut wire = [0u8; RESPONDER_SHARE_WIRE_LEN];
        encode_responder_share(&ct, &pk, &mut wire).unwrap();
        match decode_responder_share(&wire[..RESPONDER_SHARE_WIRE_LEN - 1]) {
            Err(e) => assert_eq!(e, CryptoError::Truncated),
            Ok(_) => panic!("truncated responder share decoded"),
        }
        let mut extra = [0u8; RESPONDER_SHARE_WIRE_LEN + 1];
        extra[..RESPONDER_SHARE_WIRE_LEN].copy_from_slice(&wire);
        match decode_responder_share(&extra) {
            Err(e) => assert_eq!(e, CryptoError::Malformed),
            Ok(_) => panic!("oversize responder share decoded"),
        }
    }

    #[test]
    fn hybrid_ikm_kem_then_x25519() {
        let mut kem = [0u8; ML_KEM_SS_LEN];
        let mut x = [0u8; X25519_LEN];
        kem[0] = 0xAA;
        kem[ML_KEM_SS_LEN - 1] = 0xAB;
        x[0] = 0x55;
        x[X25519_LEN - 1] = 0x56;
        let mut out = [0u8; 64];
        hybrid_ikm(&kem, &x, &mut out);
        assert_eq!(out[0], 0xAA);
        assert_eq!(out[ML_KEM_SS_LEN - 1], 0xAB);
        assert_eq!(out[ML_KEM_SS_LEN], 0x55);
        assert_eq!(out[63], 0x56);
        assert_eq!(&out[..ML_KEM_SS_LEN], &kem);
        assert_eq!(&out[ML_KEM_SS_LEN..], &x);
    }

    #[test]
    fn no_version_prefix() {
        assert!(!responder_share_has_version_prefix());
    }

    /// Live encapsulate uses RNG. Ciphertext is not a frozen CRY-02.12 vector.
    #[test]
    fn live_ml_kem_encapsulate_then_encode_recovers_ct() {
        let (sk, pk) = MlKem768Secret::generate().unwrap();
        let (ss_enc, ct) = encapsulate(&pk).unwrap();
        let ss_dec = sk.decapsulate(&ct).unwrap();
        assert_eq!(ss_enc, ss_dec);

        let dummy_x25519 = patterned_x25519_pk();
        let mut wire = [0u8; RESPONDER_SHARE_WIRE_LEN];
        let n = encode_responder_share(&ct, &dummy_x25519, &mut wire).unwrap();
        assert_eq!(n, RESPONDER_SHARE_WIRE_LEN);

        let (decoded_ct, decoded_pk) = decode_responder_share(&wire).unwrap();
        assert_eq!(decoded_ct, ct);
        assert_eq!(decoded_pk, dummy_x25519);
        // Live ct is RNG-dependent: patterned fixture must not match this run.
        assert_ne!(decoded_ct, patterned_ct());
    }
}
