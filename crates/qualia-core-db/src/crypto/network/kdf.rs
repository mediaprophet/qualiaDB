//! HKDF-SHA-384 and HMAC-SHA-384. Domain-separated; no secret XOR combiners.

use hkdf::Hkdf;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha384;
use zeroize::Zeroize;

use super::errors::CryptoError;

type HmacSha384 = Hmac<Sha384>;

pub fn hkdf_sha384(
    salt: Option<&[u8]>,
    ikm: &[u8],
    info: &[u8],
    okm: &mut [u8],
) -> Result<(), CryptoError> {
    if okm.is_empty() {
        return Err(CryptoError::Range);
    }
    let hk = Hkdf::<Sha384>::new(salt, ikm);
    hk.expand(info, okm).map_err(|_| CryptoError::CryptoFailure)
}

pub fn hmac_sha384(key: &[u8], data: &[u8], out: &mut [u8; 48]) -> Result<(), CryptoError> {
    let mut mac = HmacSha384::new_from_slice(key).map_err(|_| CryptoError::CryptoFailure)?;
    mac.update(data);
    let result = mac.finalize().into_bytes();
    out.copy_from_slice(&result);
    Ok(())
}

/// Combine ML-KEM and X25519 secrets: HKDF(IKM = kem || x25519). Never XOR.
pub fn hybrid_shared_secret(
    ml_kem_ss: &[u8; 32],
    x25519_ss: &[u8; 32],
    info: &[u8],
    out: &mut [u8],
) -> Result<(), CryptoError> {
    let mut ikm = [0u8; 64];
    ikm[..32].copy_from_slice(ml_kem_ss);
    ikm[32..].copy_from_slice(x25519_ss);
    let result = hkdf_sha384(Some(b"qpr-pq-1"), &ikm, info, out);
    ikm.zeroize();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hybrid_combiner_is_not_xor() {
        let kem = [1u8; 32];
        let x = [1u8; 32];
        let mut out = [0u8; 32];
        hybrid_shared_secret(&kem, &x, b"test", &mut out).unwrap();
        assert_ne!(out, [0u8; 32]);
        assert_ne!(out, [1u8; 32]);
    }
}
