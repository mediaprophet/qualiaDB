//! CRY-02.05 **PARTIAL** — dual ML-DSA-65 + Ed25519 proof over one payload.
//!
//! Both algorithms sign and verify the **same** caller `message` and `context`
//! bytes. Binding is the existing length-delimited adapters
//! ([`crate::crypto::network::mldsa`] SHA-384-compressed FIPS-204 ctx, and
//! [`crate::crypto::network::ed25519`] versioned `u16be||label||u32be||ctx||u32be||msg`).
//! This wave does **not** invent a deterministic-CBOR COSE_Sign1 object.
//! [`cose_sign1_frozen`] is false: the CRY-02.05 COSE_Sign1 object freeze
//! remains open.
//!
//! Dual proof: **both** signatures must verify. ML-DSA-only or Ed25519-only
//! (including a stripped all-zero counterpart) is [`CryptoError::Unauthorized`].
//! [`single_algorithm_is_enough`] is false.
//!
//! Draft until FND-03 freeze. No security proof is claimed.

use crate::crypto::network::ed25519::{sign_with_context, verify_with_context};
use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::mldsa;
use crate::crypto::network::types::{
    ED25519_PK_LEN, ED25519_SIG_LEN, ML_DSA_65_PK_LEN, ML_DSA_65_SIG_LEN, ML_DSA_65_SK_LEN,
};

/// Paired signatures over one bound payload. Both fields are required.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DualProof {
    pub mldsa_sig: [u8; ML_DSA_65_SIG_LEN],
    pub ed25519_sig: [u8; ED25519_SIG_LEN],
}

/// Sign `message` under `context` with both ML-DSA-65 and Ed25519.
///
/// Same `message` and same `context` bytes are passed to both adapters.
/// A failed call may leave `out` partially written; the caller must not
/// treat it as a valid proof.
pub fn sign_dual(
    mldsa_sk: &[u8; ML_DSA_65_SK_LEN],
    ed25519_seed: &[u8; 32],
    message: &[u8],
    context: &[u8],
    out: &mut DualProof,
) -> Result<(), CryptoError> {
    mldsa::sign(mldsa_sk, message, context, &mut out.mldsa_sig)?;
    sign_with_context(ed25519_seed, message, context, &mut out.ed25519_sig)?;
    Ok(())
}

/// Verify ML-DSA-65 then Ed25519 over the same `message` and `context`.
///
/// Either signature failing is [`CryptoError::Unauthorized`]. This does not
/// short-circuit into `Ok` after a single-algorithm success. Returning
/// `Unauthorized` on the first failure (without attempting the second) is
/// permitted; ML-DSA is checked first.
pub fn verify_dual(
    mldsa_pk: &[u8; ML_DSA_65_PK_LEN],
    ed25519_pk: &[u8; ED25519_PK_LEN],
    message: &[u8],
    context: &[u8],
    proof: &DualProof,
) -> Result<(), CryptoError> {
    mldsa::verify(mldsa_pk, message, context, &proof.mldsa_sig)?;
    verify_with_context(ed25519_pk, message, context, &proof.ed25519_sig)?;
    Ok(())
}

/// One algorithm is never sufficient for a dual proof.
pub fn single_algorithm_is_enough() -> bool {
    false
}

/// Honest: the CRY-02.05 COSE_Sign1 / deterministic-CBOR object is not frozen.
pub fn cose_sign1_frozen() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::ed25519::public_from_seed;
    use crate::crypto::network::mldsa::generate_keypair;

    const MSG: &[u8] = b"qdnf-dual-payload";
    const CTX: &[u8] = b"qlink";
    const SEED: [u8; 32] = [7u8; 32];

    fn fresh_signed() -> ([u8; ML_DSA_65_PK_LEN], [u8; ED25519_PK_LEN], DualProof) {
        let (sk, pk) = generate_keypair().unwrap();
        let ed_pk = public_from_seed(&SEED);
        let mut proof = DualProof {
            mldsa_sig: [0u8; ML_DSA_65_SIG_LEN],
            ed25519_sig: [0u8; ED25519_SIG_LEN],
        };
        sign_dual(&sk, &SEED, MSG, CTX, &mut proof).unwrap();
        (pk, ed_pk, proof)
    }

    #[test]
    fn generate_sign_verify_ok() {
        let (mldsa_pk, ed_pk, proof) = fresh_signed();
        assert!(verify_dual(&mldsa_pk, &ed_pk, MSG, CTX, &proof).is_ok());
    }

    #[test]
    fn wrong_ed25519_sig_is_unauthorized() {
        let (mldsa_pk, ed_pk, mut proof) = fresh_signed();
        proof.ed25519_sig[0] ^= 0x01;
        assert_eq!(
            verify_dual(&mldsa_pk, &ed_pk, MSG, CTX, &proof),
            Err(CryptoError::Unauthorized)
        );
        assert!(mldsa::verify(&mldsa_pk, MSG, CTX, &proof.mldsa_sig).is_ok());
    }

    #[test]
    fn wrong_context_is_unauthorized() {
        let (mldsa_pk, ed_pk, proof) = fresh_signed();
        assert_eq!(
            verify_dual(&mldsa_pk, &ed_pk, MSG, b"qsession", &proof),
            Err(CryptoError::Unauthorized)
        );
    }

    #[test]
    fn single_algorithm_is_not_enough() {
        assert!(!single_algorithm_is_enough());
    }

    #[test]
    fn cose_sign1_is_not_frozen() {
        assert!(!cose_sign1_frozen());
    }

    #[test]
    fn stripped_ed25519_is_unauthorized() {
        let (mldsa_pk, ed_pk, mut proof) = fresh_signed();
        proof.ed25519_sig = [0u8; ED25519_SIG_LEN];
        assert_eq!(
            verify_dual(&mldsa_pk, &ed_pk, MSG, CTX, &proof),
            Err(CryptoError::Unauthorized)
        );
        assert!(mldsa::verify(&mldsa_pk, MSG, CTX, &proof.mldsa_sig).is_ok());
    }
}
