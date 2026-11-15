//! Hardware key-provider backend. No software fallback across this boundary.

use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::hardware;
use crate::crypto::network::types::{
    AEAD_KEY_LEN, AEAD_NONCE_LEN, AEAD_TAG_LEN, ED25519_SIG_LEN, ML_DSA_65_SIG_LEN,
    ML_DSA_65_SK_LEN, ML_KEM_768_CT_LEN, ML_KEM_768_SK_LEN, ML_KEM_SS_LEN, X25519_LEN,
};

use super::grant::ProviderGrant;

pub struct HardwareBackend;

impl HardwareBackend {
    /// Reserved no-arg sign entry point — always `PlatformUnsupported`; use `sign_ed25519`/`sign_mldsa`.
    pub fn sign() -> Result<(), CryptoError> {
        Err(CryptoError::PlatformUnsupported)
    }

    /// Ed25519 signs `message`+`context` into `sig_out`, delegating to the hardware layer; grant fields accepted for interface parity. Fails closed `PlatformUnsupported` where no hardware backend exists.
    pub fn sign_ed25519(
        grant: &ProviderGrant,
        seed: &[u8; 32],
        message: &[u8],
        context: &[u8],
        now_unix: u64,
        controller: u64,
        authority_generation: u64,
        sig_out: &mut [u8; ED25519_SIG_LEN],
    ) -> Result<(), CryptoError> {
        let _ = (grant, now_unix, controller, authority_generation);
        hardware::sign_ed25519(seed, message, context, sig_out)
    }

    /// ML-DSA-65 signs `message`+`context` into `sig_out` via the hardware layer; `PlatformUnsupported` on platforms without one.
    pub fn sign_mldsa(
        secret: &[u8; ML_DSA_65_SK_LEN],
        message: &[u8],
        context: &[u8],
        sig_out: &mut [u8; ML_DSA_65_SIG_LEN],
    ) -> Result<(), CryptoError> {
        hardware::sign_mldsa(secret, message, context, sig_out)
    }

    /// Derives the X25519 public key for `secret` via the hardware layer; `PlatformUnsupported` where unavailable.
    pub fn x25519_public(secret: &[u8; X25519_LEN]) -> Result<[u8; X25519_LEN], CryptoError> {
        hardware::x25519_public(secret)
    }

    /// X25519 Diffie-Hellman of `secret`×`peer` via the hardware layer; `PlatformUnsupported` where unavailable.
    pub fn x25519_dh(
        secret: &[u8; X25519_LEN],
        peer: &[u8; X25519_LEN],
    ) -> Result<[u8; X25519_LEN], CryptoError> {
        hardware::x25519_dh(secret, peer)
    }

    /// Reserved ephemeral-keygen entry point — always `PlatformUnsupported`.
    pub fn generate_ephemeral() -> Result<(), CryptoError> {
        Err(CryptoError::PlatformUnsupported)
    }

    /// Fills `out` with hardware entropy; `PlatformUnsupported` where no entropy source exists.
    pub fn fill_entropy(out: &mut [u8]) -> Result<(), CryptoError> {
        hardware::fill_entropy(out)
    }

    /// ML-KEM-768 decapsulates `ciphertext` under `secret` via the hardware layer; `PlatformUnsupported` where unavailable.
    pub fn mlkem_decaps(
        secret: &[u8; ML_KEM_768_SK_LEN],
        ciphertext: &[u8; ML_KEM_768_CT_LEN],
    ) -> Result<[u8; ML_KEM_SS_LEN], CryptoError> {
        hardware::mlkem_decaps(secret, ciphertext)
    }

    /// AEAD-encrypts `buffer` in place under `(key, nonce, aad)`, emitting `tag_out`; `PlatformUnsupported` where unavailable.
    pub fn aead_encrypt(
        key: &[u8; AEAD_KEY_LEN],
        nonce: &[u8; AEAD_NONCE_LEN],
        aad: &[u8],
        buffer: &mut [u8],
        tag_out: &mut [u8; AEAD_TAG_LEN],
    ) -> Result<(), CryptoError> {
        hardware::aead_encrypt(key, nonce, aad, buffer, tag_out)
    }

    /// AEAD-decrypts `buffer` in place, verifying `tag`; `PlatformUnsupported`/tag-failure errors fail closed.
    pub fn aead_decrypt(
        key: &[u8; AEAD_KEY_LEN],
        nonce: &[u8; AEAD_NONCE_LEN],
        aad: &[u8],
        buffer: &mut [u8],
        tag: &[u8; AEAD_TAG_LEN],
    ) -> Result<(), CryptoError> {
        hardware::aead_decrypt(key, nonce, aad, buffer, tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::types::{KeyEpoch, KeyPurpose};

    fn grant() -> ProviderGrant {
        ProviderGrant {
            epoch: KeyEpoch {
                purpose: KeyPurpose::ControllerSign,
                epoch: 1,
                controller: 9,
            },
            cell_epoch: 1,
            authority_generation: 1,
            deadline_unix: 100,
            revoked: false,
        }
    }

    #[test]
    fn all_ops_are_platform_unsupported() {
        assert_eq!(
            HardwareBackend::sign(),
            Err(CryptoError::PlatformUnsupported)
        );
        let mut sig = [0u8; ED25519_SIG_LEN];
        assert_eq!(
            HardwareBackend::sign_ed25519(&grant(), &[3u8; 32], b"m", b"c", 10, 9, 1, &mut sig),
            Err(CryptoError::PlatformUnsupported)
        );
        assert_eq!(sig, [0u8; ED25519_SIG_LEN]);
        let mut buf = [0xAAu8; 8];
        assert_eq!(
            HardwareBackend::fill_entropy(&mut buf),
            Err(CryptoError::PlatformUnsupported)
        );
        assert_eq!(buf, [0xAAu8; 8]);
        assert_eq!(
            HardwareBackend::generate_ephemeral(),
            Err(CryptoError::PlatformUnsupported)
        );
        assert_eq!(
            HardwareBackend::x25519_public(&[5u8; 32]),
            Err(CryptoError::PlatformUnsupported)
        );
        assert_eq!(
            HardwareBackend::x25519_dh(&[5u8; 32], &[6u8; 32]),
            Err(CryptoError::PlatformUnsupported)
        );
        let mut mldsa_sig = [0u8; ML_DSA_65_SIG_LEN];
        assert_eq!(
            HardwareBackend::sign_mldsa(&[0u8; ML_DSA_65_SK_LEN], b"m", b"c", &mut mldsa_sig),
            Err(CryptoError::PlatformUnsupported)
        );
        assert_eq!(
            HardwareBackend::mlkem_decaps(&[0u8; ML_KEM_768_SK_LEN], &[0u8; ML_KEM_768_CT_LEN]),
            Err(CryptoError::PlatformUnsupported)
        );
        let mut aead_buf = *b"hello-qdnf";
        let mut tag = [0u8; AEAD_TAG_LEN];
        assert_eq!(
            HardwareBackend::aead_encrypt(&[7u8; 32], &[9u8; 12], b"aad", &mut aead_buf, &mut tag),
            Err(CryptoError::PlatformUnsupported)
        );
        assert_eq!(&aead_buf, b"hello-qdnf");
        assert_eq!(
            HardwareBackend::aead_decrypt(&[7u8; 32], &[9u8; 12], b"aad", &mut aead_buf, &tag),
            Err(CryptoError::PlatformUnsupported)
        );
    }
}
