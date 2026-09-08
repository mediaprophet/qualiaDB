//! ChaCha20-Poly1305 over caller buffers.

use aead::{AeadInOut, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, Tag};

use super::errors::CryptoError;
use super::types::{AEAD_KEY_LEN, AEAD_NONCE_LEN, AEAD_TAG_LEN};

pub fn encrypt_in_place(
    key: &[u8; AEAD_KEY_LEN],
    nonce: &[u8; AEAD_NONCE_LEN],
    aad: &[u8],
    buffer: &mut [u8],
    tag_out: &mut [u8; AEAD_TAG_LEN],
) -> Result<(), CryptoError> {
    let cipher = ChaCha20Poly1305::new(&Key::try_from(key.as_slice()).map_err(|_| CryptoError::CryptoFailure)?);
    let tag = cipher
        .encrypt_inout_detached(
            &Nonce::try_from(nonce.as_slice()).map_err(|_| CryptoError::CryptoFailure)?,
            aad,
            buffer.into(),
        )
        .map_err(|_| CryptoError::CryptoFailure)?;
    tag_out.copy_from_slice(tag.as_slice());
    Ok(())
}

pub fn decrypt_in_place(
    key: &[u8; AEAD_KEY_LEN],
    nonce: &[u8; AEAD_NONCE_LEN],
    aad: &[u8],
    buffer: &mut [u8],
    tag: &[u8; AEAD_TAG_LEN],
) -> Result<(), CryptoError> {
    let cipher = ChaCha20Poly1305::new(&Key::try_from(key.as_slice()).map_err(|_| CryptoError::CryptoFailure)?);
    let tag = Tag::try_from(tag.as_slice()).map_err(|_| CryptoError::CryptoFailure)?;
    cipher
        .decrypt_inout_detached(
            &Nonce::try_from(nonce.as_slice()).map_err(|_| CryptoError::CryptoFailure)?,
            aad,
            buffer.into(),
            &tag,
        )
        .map_err(|_| CryptoError::CryptoFailure)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_aad_mismatch() {
        let key = [7u8; 32];
        let nonce = [9u8; 12];
        let mut buf = *b"hello-qdnf";
        let mut tag = [0u8; 16];
        encrypt_in_place(&key, &nonce, b"aad", &mut buf, &mut tag).unwrap();
        let mut good = buf;
        decrypt_in_place(&key, &nonce, b"aad", &mut good, &tag).unwrap();
        assert_eq!(&good, b"hello-qdnf");
        let mut bad = buf;
        assert!(decrypt_in_place(&key, &nonce, b"nope", &mut bad, &tag).is_err());
    }
}
