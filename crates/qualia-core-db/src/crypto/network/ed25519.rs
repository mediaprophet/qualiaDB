//! Ed25519 adapter with versioned length-delimited network context.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use super::errors::CryptoError;
use super::types::{ED25519_PK_LEN, ED25519_SIG_LEN};

const CONTEXT_V1: &[u8] = b"QDNF-ED25519-CTX-V1";

/// Bind a versioned context: u16be(len) || label || u32be(len) || value.
pub fn sign_with_context(
    seed: &[u8; 32],
    message: &[u8],
    context: &[u8],
    sig_out: &mut [u8; ED25519_SIG_LEN],
) -> Result<(), CryptoError> {
    let key = SigningKey::from_bytes(seed);
    let mut bound = [0u8; 512];
    let n = bind_context(CONTEXT_V1, context, message, &mut bound)?;
    let sig = key.sign(&bound[..n]);
    sig_out.copy_from_slice(&sig.to_bytes());
    Ok(())
}

pub fn verify_with_context(
    public: &[u8; ED25519_PK_LEN],
    message: &[u8],
    context: &[u8],
    signature: &[u8; ED25519_SIG_LEN],
) -> Result<(), CryptoError> {
    let key = VerifyingKey::from_bytes(public).map_err(|_| CryptoError::CryptoFailure)?;
    let mut bound = [0u8; 512];
    let n = bind_context(CONTEXT_V1, context, message, &mut bound)?;
    let sig = Signature::from_bytes(signature);
    key.verify(&bound[..n], &sig)
        .map_err(|_| CryptoError::Unauthorized)
}

pub fn public_from_seed(seed: &[u8; 32]) -> [u8; 32] {
    SigningKey::from_bytes(seed).verifying_key().to_bytes()
}

fn bind_context(
    label: &[u8],
    context: &[u8],
    message: &[u8],
    out: &mut [u8],
) -> Result<usize, CryptoError> {
    let need = 2 + label.len() + 4 + context.len() + 4 + message.len();
    if out.len() < need {
        return Err(CryptoError::Capacity);
    }
    let mut i = 0;
    let label_len: u16 = label.len().try_into().map_err(|_| CryptoError::Range)?;
    out[i..i + 2].copy_from_slice(&label_len.to_be_bytes());
    i += 2;
    out[i..i + label.len()].copy_from_slice(label);
    i += label.len();
    let ctx_len: u32 = context.len().try_into().map_err(|_| CryptoError::Range)?;
    out[i..i + 4].copy_from_slice(&ctx_len.to_be_bytes());
    i += 4;
    out[i..i + context.len()].copy_from_slice(context);
    i += context.len();
    let msg_len: u32 = message.len().try_into().map_err(|_| CryptoError::Range)?;
    out[i..i + 4].copy_from_slice(&msg_len.to_be_bytes());
    i += 4;
    out[i..i + message.len()].copy_from_slice(message);
    i += message.len();
    Ok(i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_mismatch_fails() {
        let seed = [3u8; 32];
        let pk = public_from_seed(&seed);
        let mut sig = [0u8; 64];
        sign_with_context(&seed, b"msg", b"ctx-a", &mut sig).unwrap();
        assert!(verify_with_context(&pk, b"msg", b"ctx-a", &sig).is_ok());
        assert_eq!(
            verify_with_context(&pk, b"msg", b"ctx-b", &sig),
            Err(CryptoError::Unauthorized)
        );
    }
}
