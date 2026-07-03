//! Sanctuary lane cryptography.
//!
//! This module derives 48 bytes of key material from a user PIN, splits it into
//! a 32-byte AEAD key plus a 16-byte volume tweak, and performs zero-heap
//! encryption and decryption using deterministic, domain-separated nonces.

use core::fmt;

use aes_gcm::aead::{AeadInOut, KeyInit};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const SANCTUARY_CIPHER_KEY_BYTES: usize = 32;
pub const SANCTUARY_TWEAK_BYTES: usize = 16;
pub const SANCTUARY_KEY_MATERIAL_BYTES: usize = SANCTUARY_CIPHER_KEY_BYTES + SANCTUARY_TWEAK_BYTES;
pub const SANCTUARY_TAG_BYTES: usize = 16;
pub const SANCTUARY_GCM_NONCE_BYTES: usize = 12;
pub const SANCTUARY_XCHACHA_NONCE_BYTES: usize = 24;
pub const DEFAULT_PBKDF2_ITERATIONS: u32 = 310_000;

const AES_GCM_DOMAIN: [u8; 4] = *b"QGCM";
const CHACHA20_DOMAIN: [u8; 4] = *b"QCHA";
const XCHACHA20_HEAD_DOMAIN: [u8; 8] = *b"Q42XCH1!";
const XCHACHA20_TAIL_DOMAIN: [u8; 8] = *b"Q42XCH2!";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SanctuaryAeadAlgorithm {
    Aes256Gcm,
    ChaCha20Poly1305,
    XChaCha20Poly1305,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SanctuaryCryptoError {
    OutputBufferTooSmall,
    EncryptionFailed,
    DecryptionFailed,
}

/// Derived sanctuary key material.
///
/// The debug representation is intentionally redacted to avoid leaking secrets
/// into logs or test output.
#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct SanctuaryKeyMaterial {
    /// 32-byte cipher key for AEAD encryption.
    pub cipher_key: [u8; SANCTUARY_CIPHER_KEY_BYTES],
    /// 16-byte volume root tweak for nonce derivation.
    pub volume_tweak: [u8; SANCTUARY_TWEAK_BYTES],
}

impl fmt::Debug for SanctuaryKeyMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "SanctuaryKeyMaterial { cipher_key: [REDACTED; 32], volume_tweak: [REDACTED; 16] }",
        )
    }
}

/// Derive 48 bytes of sanctuary key material from a PIN and salt.
///
/// Layout:
/// `bytes[0..32]`   -> AEAD cipher key
/// `bytes[32..48]`  -> volume tweak used for deterministic nonce derivation
pub fn derive_sanctuary_key_material(
    pin: &[u8],
    salt: &[u8],
    iterations: u32,
) -> SanctuaryKeyMaterial {
    let mut key_material = [0u8; SANCTUARY_KEY_MATERIAL_BYTES];
    pbkdf2_hmac::<Sha256>(pin, salt, iterations, &mut key_material);

    let mut cipher_key = [0u8; SANCTUARY_CIPHER_KEY_BYTES];
    let mut volume_tweak = [0u8; SANCTUARY_TWEAK_BYTES];
    cipher_key.copy_from_slice(&key_material[..SANCTUARY_CIPHER_KEY_BYTES]);
    volume_tweak.copy_from_slice(&key_material[SANCTUARY_CIPHER_KEY_BYTES..]);
    key_material.zeroize();

    SanctuaryKeyMaterial {
        cipher_key,
        volume_tweak,
    }
}

/// Convenience wrapper for call sites that only need the 32-byte cipher key.
pub fn derive_lane_cipher_key(pin: &[u8], salt: &[u8], iterations: u32) -> [u8; 32] {
    let key_material = derive_sanctuary_key_material(pin, salt, iterations);
    key_material.cipher_key
}

/// Derive a 96-bit nonce for AES-256-GCM.
pub fn derive_chunk_nonce(volume_tweak: &[u8; 16], chunk_index_or_offset: u64) -> [u8; 12] {
    derive_compact_nonce(volume_tweak, chunk_index_or_offset, AES_GCM_DOMAIN)
}

/// Derive a 96-bit nonce for ChaCha20-Poly1305.
pub fn derive_chacha_nonce(volume_tweak: &[u8; 16], chunk_index_or_offset: u64) -> [u8; 12] {
    derive_compact_nonce(volume_tweak, chunk_index_or_offset, CHACHA20_DOMAIN)
}

/// Derive a 192-bit nonce for XChaCha20-Poly1305.
pub fn derive_xchacha_nonce(volume_tweak: &[u8; 16], chunk_index_or_offset: u64) -> [u8; 24] {
    let index_bytes = chunk_index_or_offset.to_le_bytes();
    let mut nonce = [0u8; SANCTUARY_XCHACHA_NONCE_BYTES];

    for i in 0..8 {
        nonce[i] = volume_tweak[i] ^ index_bytes[i] ^ XCHACHA20_HEAD_DOMAIN[i];
        nonce[8 + i] = volume_tweak[8 + i] ^ XCHACHA20_TAIL_DOMAIN[i];
        nonce[16 + i] = volume_tweak[15 - i] ^ index_bytes[i] ^ XCHACHA20_TAIL_DOMAIN[i];
    }

    nonce
}

/// Encrypt the caller-owned buffer in place without heap allocation.
pub fn encrypt_sanctuary_chunk_in_place(
    algorithm: SanctuaryAeadAlgorithm,
    key_material: &SanctuaryKeyMaterial,
    chunk_index: u64,
    buffer: &mut [u8],
    additional_data: &[u8],
    tag_out: &mut [u8; SANCTUARY_TAG_BYTES],
) -> Result<usize, SanctuaryCryptoError> {
    match algorithm {
        SanctuaryAeadAlgorithm::Aes256Gcm => {
            let nonce = derive_chunk_nonce(&key_material.volume_tweak, chunk_index);
            let key = <&aes_gcm::Key<aes_gcm::Aes256Gcm>>::try_from(&key_material.cipher_key[..])
                .map_err(|_| SanctuaryCryptoError::EncryptionFailed)?;
            let cipher = aes_gcm::Aes256Gcm::new(key);
            let nonce = <&aes_gcm::aead::Nonce<aes_gcm::Aes256Gcm>>::try_from(&nonce[..])
                .map_err(|_| SanctuaryCryptoError::EncryptionFailed)?;
            let tag = cipher
                .encrypt_inout_detached(nonce, additional_data, buffer.into())
                .map_err(|_| SanctuaryCryptoError::EncryptionFailed)?;
            tag_out.copy_from_slice(tag.as_slice());
        }
        SanctuaryAeadAlgorithm::ChaCha20Poly1305 => {
            let nonce = derive_chacha_nonce(&key_material.volume_tweak, chunk_index);
            let key = <&chacha20poly1305::Key>::try_from(&key_material.cipher_key[..])
                .map_err(|_| SanctuaryCryptoError::EncryptionFailed)?;
            let cipher = chacha20poly1305::ChaCha20Poly1305::new(key);
            let nonce = <&chacha20poly1305::Nonce>::try_from(&nonce[..])
                .map_err(|_| SanctuaryCryptoError::EncryptionFailed)?;
            let tag = cipher
                .encrypt_inout_detached(nonce, additional_data, buffer.into())
                .map_err(|_| SanctuaryCryptoError::EncryptionFailed)?;
            tag_out.copy_from_slice(tag.as_slice());
        }
        SanctuaryAeadAlgorithm::XChaCha20Poly1305 => {
            let nonce = derive_xchacha_nonce(&key_material.volume_tweak, chunk_index);
            let key = <&chacha20poly1305::Key>::try_from(&key_material.cipher_key[..])
                .map_err(|_| SanctuaryCryptoError::EncryptionFailed)?;
            let cipher = chacha20poly1305::XChaCha20Poly1305::new(key);
            let nonce = <&chacha20poly1305::XNonce>::try_from(&nonce[..])
                .map_err(|_| SanctuaryCryptoError::EncryptionFailed)?;
            let tag = cipher
                .encrypt_inout_detached(nonce, additional_data, buffer.into())
                .map_err(|_| SanctuaryCryptoError::EncryptionFailed)?;
            tag_out.copy_from_slice(tag.as_slice());
        }
    }

    Ok(buffer.len())
}

/// Decrypt the caller-owned buffer in place without heap allocation.
pub fn decrypt_sanctuary_chunk_in_place(
    algorithm: SanctuaryAeadAlgorithm,
    key_material: &SanctuaryKeyMaterial,
    chunk_index: u64,
    buffer: &mut [u8],
    tag: &[u8; SANCTUARY_TAG_BYTES],
    additional_data: &[u8],
) -> Result<usize, SanctuaryCryptoError> {
    match algorithm {
        SanctuaryAeadAlgorithm::Aes256Gcm => {
            let nonce = derive_chunk_nonce(&key_material.volume_tweak, chunk_index);
            let key = <&aes_gcm::Key<aes_gcm::Aes256Gcm>>::try_from(&key_material.cipher_key[..])
                .map_err(|_| SanctuaryCryptoError::DecryptionFailed)?;
            let cipher = aes_gcm::Aes256Gcm::new(key);
            let nonce = <&aes_gcm::aead::Nonce<aes_gcm::Aes256Gcm>>::try_from(&nonce[..])
                .map_err(|_| SanctuaryCryptoError::DecryptionFailed)?;
            let tag = <&aes_gcm::aead::Tag<aes_gcm::Aes256Gcm>>::try_from(&tag[..])
                .map_err(|_| SanctuaryCryptoError::DecryptionFailed)?;
            cipher
                .decrypt_inout_detached(nonce, additional_data, buffer.into(), tag)
                .map_err(|_| SanctuaryCryptoError::DecryptionFailed)?;
        }
        SanctuaryAeadAlgorithm::ChaCha20Poly1305 => {
            let nonce = derive_chacha_nonce(&key_material.volume_tweak, chunk_index);
            let key = <&chacha20poly1305::Key>::try_from(&key_material.cipher_key[..])
                .map_err(|_| SanctuaryCryptoError::DecryptionFailed)?;
            let cipher = chacha20poly1305::ChaCha20Poly1305::new(key);
            let nonce = <&chacha20poly1305::Nonce>::try_from(&nonce[..])
                .map_err(|_| SanctuaryCryptoError::DecryptionFailed)?;
            let tag = <&chacha20poly1305::Tag>::try_from(&tag[..])
                .map_err(|_| SanctuaryCryptoError::DecryptionFailed)?;
            cipher
                .decrypt_inout_detached(nonce, additional_data, buffer.into(), tag)
                .map_err(|_| SanctuaryCryptoError::DecryptionFailed)?;
        }
        SanctuaryAeadAlgorithm::XChaCha20Poly1305 => {
            let nonce = derive_xchacha_nonce(&key_material.volume_tweak, chunk_index);
            let key = <&chacha20poly1305::Key>::try_from(&key_material.cipher_key[..])
                .map_err(|_| SanctuaryCryptoError::DecryptionFailed)?;
            let cipher = chacha20poly1305::XChaCha20Poly1305::new(key);
            let nonce = <&chacha20poly1305::XNonce>::try_from(&nonce[..])
                .map_err(|_| SanctuaryCryptoError::DecryptionFailed)?;
            let tag = <&chacha20poly1305::Tag>::try_from(&tag[..])
                .map_err(|_| SanctuaryCryptoError::DecryptionFailed)?;
            cipher
                .decrypt_inout_detached(nonce, additional_data, buffer.into(), tag)
                .map_err(|_| SanctuaryCryptoError::DecryptionFailed)?;
        }
    }

    Ok(buffer.len())
}

/// Copy plaintext into a caller-supplied output buffer, then encrypt in place.
pub fn encrypt_sanctuary_chunk(
    algorithm: SanctuaryAeadAlgorithm,
    key_material: &SanctuaryKeyMaterial,
    chunk_index: u64,
    plaintext: &[u8],
    ciphertext_out: &mut [u8],
    tag_out: &mut [u8; SANCTUARY_TAG_BYTES],
    additional_data: &[u8],
) -> Result<usize, SanctuaryCryptoError> {
    if ciphertext_out.len() < plaintext.len() {
        return Err(SanctuaryCryptoError::OutputBufferTooSmall);
    }

    let ciphertext = &mut ciphertext_out[..plaintext.len()];
    ciphertext.copy_from_slice(plaintext);
    encrypt_sanctuary_chunk_in_place(
        algorithm,
        key_material,
        chunk_index,
        ciphertext,
        additional_data,
        tag_out,
    )
}

/// Copy ciphertext into a caller-supplied output buffer, then decrypt in place.
pub fn decrypt_sanctuary_chunk(
    algorithm: SanctuaryAeadAlgorithm,
    key_material: &SanctuaryKeyMaterial,
    chunk_index: u64,
    ciphertext: &[u8],
    tag: &[u8; SANCTUARY_TAG_BYTES],
    plaintext_out: &mut [u8],
    additional_data: &[u8],
) -> Result<usize, SanctuaryCryptoError> {
    if plaintext_out.len() < ciphertext.len() {
        return Err(SanctuaryCryptoError::OutputBufferTooSmall);
    }

    let plaintext = &mut plaintext_out[..ciphertext.len()];
    plaintext.copy_from_slice(ciphertext);
    decrypt_sanctuary_chunk_in_place(
        algorithm,
        key_material,
        chunk_index,
        plaintext,
        tag,
        additional_data,
    )
}

fn derive_compact_nonce(
    volume_tweak: &[u8; 16],
    chunk_index_or_offset: u64,
    domain: [u8; 4],
) -> [u8; 12] {
    let index_bytes = chunk_index_or_offset.to_le_bytes();
    let mut nonce = [0u8; SANCTUARY_GCM_NONCE_BYTES];

    for i in 0..4 {
        nonce[i] = volume_tweak[i] ^ volume_tweak[12 + i] ^ domain[i];
    }
    for i in 0..8 {
        nonce[4 + i] = volume_tweak[4 + i] ^ index_bytes[i];
    }

    nonce
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_ITERATIONS: u32 = 1_000;

    #[test]
    fn test_sanctuary_key_derivation() {
        let result =
            derive_sanctuary_key_material(b"test_pin_123", b"test_salt_456", TEST_ITERATIONS);
        let result2 =
            derive_sanctuary_key_material(b"test_pin_123", b"test_salt_456", TEST_ITERATIONS);

        assert_eq!(result.cipher_key.len(), SANCTUARY_CIPHER_KEY_BYTES);
        assert_eq!(result.volume_tweak.len(), SANCTUARY_TWEAK_BYTES);
        assert_ne!(result.cipher_key[..16], result.volume_tweak);
        assert_eq!(result, result2);
    }

    #[test]
    fn test_aes_nonce_derivation_uses_full_chunk_index() {
        let tweak = [1u8; SANCTUARY_TWEAK_BYTES];
        let low = derive_chunk_nonce(&tweak, 0);
        let high = derive_chunk_nonce(&tweak, 1u64 << 40);

        assert_ne!(low, high);
    }

    #[test]
    fn test_nonce_domains_are_distinct() {
        let tweak = [7u8; SANCTUARY_TWEAK_BYTES];
        let chunk_index = 42u64;

        let aes_nonce = derive_chunk_nonce(&tweak, chunk_index);
        let chacha_nonce = derive_chacha_nonce(&tweak, chunk_index);
        let xchacha_nonce = derive_xchacha_nonce(&tweak, chunk_index);

        assert_ne!(aes_nonce, chacha_nonce);
        assert_ne!(aes_nonce[..], xchacha_nonce[..SANCTUARY_GCM_NONCE_BYTES]);
        assert_eq!(xchacha_nonce.len(), SANCTUARY_XCHACHA_NONCE_BYTES);
    }

    #[test]
    fn test_nonce_uniqueness_across_volume() {
        let tweak = [3u8; SANCTUARY_TWEAK_BYTES];
        let mut nonces = std::collections::HashSet::new();

        for index in 0..1_000u64 {
            let nonce = derive_chunk_nonce(&tweak, index);
            assert!(nonces.insert(nonce), "duplicate nonce at chunk {index}");
        }
    }

    #[test]
    fn test_zero_heap_encrypt_decrypt_aes_gcm() {
        let key_material =
            derive_sanctuary_key_material(b"test_pin", b"test_salt", TEST_ITERATIONS);
        let plaintext = b"Hello, zero-heap world!";
        let mut ciphertext = [0u8; 128];
        let mut tag = [0u8; SANCTUARY_TAG_BYTES];

        let written = encrypt_sanctuary_chunk(
            SanctuaryAeadAlgorithm::Aes256Gcm,
            &key_material,
            0,
            plaintext,
            &mut ciphertext,
            &mut tag,
            b"",
        )
        .unwrap();

        let mut decrypted = [0u8; 128];
        let read = decrypt_sanctuary_chunk(
            SanctuaryAeadAlgorithm::Aes256Gcm,
            &key_material,
            0,
            &ciphertext[..written],
            &tag,
            &mut decrypted,
            b"",
        )
        .unwrap();

        assert_eq!(&decrypted[..read], plaintext);
    }

    #[test]
    fn test_zero_heap_encrypt_decrypt_xchacha() {
        let key_material = derive_sanctuary_key_material(b"pin", b"salt", TEST_ITERATIONS);
        let mut buffer = *b"Secret message in place";
        let original = buffer;
        let mut tag = [0u8; SANCTUARY_TAG_BYTES];

        encrypt_sanctuary_chunk_in_place(
            SanctuaryAeadAlgorithm::XChaCha20Poly1305,
            &key_material,
            9,
            &mut buffer,
            b"context",
            &mut tag,
        )
        .unwrap();

        assert_ne!(buffer, original);

        decrypt_sanctuary_chunk_in_place(
            SanctuaryAeadAlgorithm::XChaCha20Poly1305,
            &key_material,
            9,
            &mut buffer,
            &tag,
            b"context",
        )
        .unwrap();

        assert_eq!(buffer, original);
    }

    #[test]
    fn test_decrypt_rejects_wrong_aad() {
        let key_material = derive_sanctuary_key_material(b"pin", b"salt", TEST_ITERATIONS);
        let plaintext = b"Bound to aad";
        let mut ciphertext = [0u8; 64];
        let mut tag = [0u8; SANCTUARY_TAG_BYTES];

        let written = encrypt_sanctuary_chunk(
            SanctuaryAeadAlgorithm::ChaCha20Poly1305,
            &key_material,
            3,
            plaintext,
            &mut ciphertext,
            &mut tag,
            b"correct",
        )
        .unwrap();

        let mut decrypted = [0u8; 64];
        let result = decrypt_sanctuary_chunk(
            SanctuaryAeadAlgorithm::ChaCha20Poly1305,
            &key_material,
            3,
            &ciphertext[..written],
            &tag,
            &mut decrypted,
            b"wrong",
        );

        assert_eq!(result, Err(SanctuaryCryptoError::DecryptionFailed));
    }
}
