//! Existing generator's explicit X25519 dispatch; no second generator or vault.
use crate::crypto::network::{entropy::fill_os, x25519::public_from_secret};
use crate::specialized_libs::cryptographic_library::CryptographicError;
use zeroize::Zeroizing;
pub(super) fn generate() -> Result<Vec<u8>, CryptographicError> {
    let mut bytes = Zeroizing::new([0; 32]);
    fill_os(&mut bytes[..])
        .map_err(|_| CryptographicError::InvalidKey("OS entropy unavailable".into()))?;
    if *bytes == [0; 32] {
        return Err(CryptographicError::InvalidKey("empty X25519 scalar".into()));
    }
    Ok(bytes.to_vec())
}
pub(super) fn public(bytes: &[u8]) -> Result<Vec<u8>, CryptographicError> {
    let scalar: &[u8; 32] = bytes
        .try_into()
        .map_err(|_| CryptographicError::InvalidKey("X25519 scalar must be 32 bytes".into()))?;
    if *scalar == [0; 32] {
        return Err(CryptographicError::InvalidKey("empty X25519 scalar".into()));
    }
    Ok(public_from_secret(scalar).to_vec())
}
