//! Hardware token / HSM primitive surface.
//!
//! Every operation returns `PlatformUnsupported`. Callers must not fall back to
//! the software adapters across this trust boundary (including WASM).

use super::errors::CryptoError;
use super::types::{
    AEAD_KEY_LEN, AEAD_NONCE_LEN, AEAD_TAG_LEN, ED25519_SIG_LEN, ML_DSA_65_SIG_LEN,
    ML_DSA_65_SK_LEN, ML_KEM_768_CT_LEN, ML_KEM_768_SK_LEN, ML_KEM_SS_LEN, X25519_LEN,
};

/// Hardware CSPRNG / token RNG. Unsupported in this build.
pub fn fill_entropy(_out: &mut [u8]) -> Result<(), CryptoError> {
    Err(CryptoError::PlatformUnsupported)
}

pub fn sign_ed25519(
    _seed: &[u8; 32],
    _message: &[u8],
    _context: &[u8],
    _sig_out: &mut [u8; ED25519_SIG_LEN],
) -> Result<(), CryptoError> {
    Err(CryptoError::PlatformUnsupported)
}

pub fn sign_mldsa(
    _secret: &[u8; ML_DSA_65_SK_LEN],
    _message: &[u8],
    _context: &[u8],
    _sig_out: &mut [u8; ML_DSA_65_SIG_LEN],
) -> Result<(), CryptoError> {
    Err(CryptoError::PlatformUnsupported)
}

pub fn x25519_public(_secret: &[u8; X25519_LEN]) -> Result<[u8; X25519_LEN], CryptoError> {
    Err(CryptoError::PlatformUnsupported)
}

pub fn x25519_dh(
    _secret: &[u8; X25519_LEN],
    _peer: &[u8; X25519_LEN],
) -> Result<[u8; X25519_LEN], CryptoError> {
    Err(CryptoError::PlatformUnsupported)
}

pub fn mlkem_decaps(
    _secret: &[u8; ML_KEM_768_SK_LEN],
    _ciphertext: &[u8; ML_KEM_768_CT_LEN],
) -> Result<[u8; ML_KEM_SS_LEN], CryptoError> {
    Err(CryptoError::PlatformUnsupported)
}

pub fn aead_encrypt(
    _key: &[u8; AEAD_KEY_LEN],
    _nonce: &[u8; AEAD_NONCE_LEN],
    _aad: &[u8],
    _buffer: &mut [u8],
    _tag_out: &mut [u8; AEAD_TAG_LEN],
) -> Result<(), CryptoError> {
    Err(CryptoError::PlatformUnsupported)
}

pub fn aead_decrypt(
    _key: &[u8; AEAD_KEY_LEN],
    _nonce: &[u8; AEAD_NONCE_LEN],
    _aad: &[u8],
    _buffer: &mut [u8],
    _tag: &[u8; AEAD_TAG_LEN],
) -> Result<(), CryptoError> {
    Err(CryptoError::PlatformUnsupported)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::ed25519;
    use crate::crypto::network::x25519::X25519Secret;

    #[test]
    fn fill_entropy_is_platform_unsupported_without_software_fill() {
        let mut buf = [0xAAu8; 16];
        assert_eq!(fill_entropy(&mut buf), Err(CryptoError::PlatformUnsupported));
        assert_eq!(buf, [0xAAu8; 16]);
    }

    #[test]
    fn sign_ed25519_does_not_fall_back_to_software() {
        let seed = [3u8; 32];
        let mut sig = [0u8; ED25519_SIG_LEN];
        assert_eq!(
            sign_ed25519(&seed, b"msg", b"ctx", &mut sig),
            Err(CryptoError::PlatformUnsupported)
        );
        assert_eq!(sig, [0u8; ED25519_SIG_LEN]);
        let mut software = [0u8; ED25519_SIG_LEN];
        ed25519::sign_with_context(&seed, b"msg", b"ctx", &mut software).unwrap();
        assert_ne!(software, [0u8; ED25519_SIG_LEN]);
    }

    #[test]
    fn x25519_does_not_compute_in_software() {
        let secret = [5u8; 32];
        assert_eq!(
            x25519_public(&secret),
            Err(CryptoError::PlatformUnsupported)
        );
        let peer = X25519Secret::from_bytes([6u8; 32]).public();
        assert_eq!(
            x25519_dh(&secret, &peer),
            Err(CryptoError::PlatformUnsupported)
        );
        let expected = X25519Secret::from_bytes(secret).public();
        assert_ne!(expected, [0u8; 32]);
    }

    #[test]
    fn remaining_ops_are_platform_unsupported() {
        let mut mldsa_sig = [0u8; ML_DSA_65_SIG_LEN];
        assert_eq!(
            sign_mldsa(&[0u8; ML_DSA_65_SK_LEN], b"m", b"c", &mut mldsa_sig),
            Err(CryptoError::PlatformUnsupported)
        );
        assert_eq!(mldsa_sig, [0u8; ML_DSA_65_SIG_LEN]);
        assert_eq!(
            mlkem_decaps(&[0u8; ML_KEM_768_SK_LEN], &[0u8; ML_KEM_768_CT_LEN]),
            Err(CryptoError::PlatformUnsupported)
        );
        let mut buf = *b"hello-qdnf";
        let mut tag = [0u8; AEAD_TAG_LEN];
        assert_eq!(
            aead_encrypt(&[7u8; 32], &[9u8; 12], b"aad", &mut buf, &mut tag),
            Err(CryptoError::PlatformUnsupported)
        );
        assert_eq!(&buf, b"hello-qdnf");
        assert_eq!(
            aead_decrypt(&[7u8; 32], &[9u8; 12], b"aad", &mut buf, &tag),
            Err(CryptoError::PlatformUnsupported)
        );
    }
}
