//! ML-DSA-65 adapter with length-delimited network context.
//!
//! Historical fiduciary_crypto context hashing concatenates domain and purpose
//! without lengths. This adapter uses an unambiguous length-delimited context
//! so old signatures cannot silently acquire the new interpretation.

use fips204::ml_dsa_65;
use fips204::traits::{SerDes, Signer, Verifier};
use sha2::{Digest, Sha384};

use super::errors::CryptoError;
use super::types::{ML_DSA_65_PK_LEN, ML_DSA_65_SIG_LEN, ML_DSA_65_SK_LEN};

const NETWORK_CTX_LABEL: &[u8] = b"QDNF-MLDSA-CTX-V1";

pub fn generate_keypair() -> Result<([u8; ML_DSA_65_SK_LEN], [u8; ML_DSA_65_PK_LEN]), CryptoError> {
    let (pk, sk) = ml_dsa_65::try_keygen().map_err(|_| CryptoError::CryptoFailure)?;
    Ok((sk.into_bytes(), pk.into_bytes()))
}

/// Compress a length-delimited context into a FIPS-204 ctx (<= 255 bytes).
fn network_ctx(context: &[u8]) -> [u8; 48] {
    let mut hasher = Sha384::new();
    hasher.update(&(NETWORK_CTX_LABEL.len() as u16).to_be_bytes());
    hasher.update(NETWORK_CTX_LABEL);
    hasher.update(&(context.len() as u32).to_be_bytes());
    hasher.update(context);
    let out = hasher.finalize();
    let mut ctx = [0u8; 48];
    ctx.copy_from_slice(&out);
    ctx
}

pub fn sign(
    secret: &[u8; ML_DSA_65_SK_LEN],
    message: &[u8],
    context: &[u8],
    sig_out: &mut [u8; ML_DSA_65_SIG_LEN],
) -> Result<(), CryptoError> {
    let sk = ml_dsa_65::PrivateKey::try_from_bytes(*secret).map_err(|_| CryptoError::CryptoFailure)?;
    let ctx = network_ctx(context);
    let sig = sk
        .try_sign(message, &ctx)
        .map_err(|_| CryptoError::CryptoFailure)?;
    sig_out.copy_from_slice(&sig);
    Ok(())
}

pub fn verify(
    public: &[u8; ML_DSA_65_PK_LEN],
    message: &[u8],
    context: &[u8],
    signature: &[u8; ML_DSA_65_SIG_LEN],
) -> Result<(), CryptoError> {
    let pk = ml_dsa_65::PublicKey::try_from_bytes(*public).map_err(|_| CryptoError::CryptoFailure)?;
    let ctx = network_ctx(context);
    if pk.verify(message, signature, &ctx) {
        Ok(())
    } else {
        Err(CryptoError::Unauthorized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_context_is_length_delimited() {
        let (sk, pk) = generate_keypair().unwrap();
        let mut sig = [0u8; ML_DSA_65_SIG_LEN];
        sign(&sk, b"record", b"qlink", &mut sig).unwrap();
        assert!(verify(&pk, b"record", b"qlink", &sig).is_ok());
        assert_eq!(
            verify(&pk, b"record", b"qsession", &sig),
            Err(CryptoError::Unauthorized)
        );
    }
}
