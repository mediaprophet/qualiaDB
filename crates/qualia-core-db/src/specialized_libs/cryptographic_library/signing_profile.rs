//! Explicit raw-message profiles. Existing contextual/prehashed APIs are unchanged.
//! This is a primitive surface, not key custody, consent, or publisher authority.
use crate::crypto::network::{errors::CryptoError, types::*};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use fips204::{
    ml_dsa_65,
    traits::{SerDes, Signer as _, Verifier as _},
};

/// Borrowed provider keys, confined to internal adapters and fixtures.
pub(crate) struct RawSigningKeys<'a> {
    pub ml_dsa_65: &'a ml_dsa_65::PrivateKey,
    pub ed25519: &'a SigningKey,
}

/// FIPS-204 ordinary ML-DSA-65, raw message and empty primitive context.
pub(crate) fn sign_mldsa65_empty_context_into(
    key: &ml_dsa_65::PrivateKey,
    message: &[u8],
    out: &mut [u8; ML_DSA_65_SIG_LEN],
) -> Result<(), CryptoError> {
    *out = key
        .try_sign(message, &[])
        .map_err(|_| CryptoError::CryptoFailure)?;
    Ok(())
}

pub fn verify_mldsa65_empty_context(
    key: &[u8; ML_DSA_65_PK_LEN],
    message: &[u8],
    signature: &[u8; ML_DSA_65_SIG_LEN],
) -> Result<(), CryptoError> {
    let key = ml_dsa_65::PublicKey::try_from_bytes(*key).map_err(|_| CryptoError::Unauthorized)?;
    if key.verify(message, signature, &[]) {
        Ok(())
    } else {
        Err(CryptoError::Unauthorized)
    }
}

pub(crate) fn sign_ed25519_raw_into(
    key: &SigningKey,
    message: &[u8],
    out: &mut [u8; ED25519_SIG_LEN],
) {
    *out = key.sign(message).to_bytes();
}

pub fn verify_ed25519_raw_strict(
    key: &[u8; ED25519_PK_LEN],
    message: &[u8],
    signature: &[u8; ED25519_SIG_LEN],
) -> Result<(), CryptoError> {
    let key = VerifyingKey::from_bytes(key).map_err(|_| CryptoError::Unauthorized)?;
    key.verify_strict(message, &Signature::from_bytes(signature))
        .map_err(|_| CryptoError::Unauthorized)
}
