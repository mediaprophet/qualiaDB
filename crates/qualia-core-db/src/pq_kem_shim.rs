// ── Post-Quantum KEM Serialization Shim ───────────────────────────────────────
/// Internal abstraction layer for post-quantum key encapsulation mechanisms.
/// Insulates the database from volatile upstream crate APIs and exposes fixed-size
/// wrappers compatible with the NQuin zero-heap discipline.

#[cfg(feature = "pq-kem")]
use fips203::traits::{Decaps, Encaps, KeyGen, SerDes};

/// Fixed-size KEM ciphertext variants.
#[cfg(feature = "pq-kem")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KemCiphertext {
    Kyber512([u8; 768]),
    Kyber768([u8; 1088]),
    Kyber1024([u8; 1568]),
}

#[cfg(feature = "pq-kem")]
impl KemCiphertext {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            KemCiphertext::Kyber512(bytes) => bytes,
            KemCiphertext::Kyber768(bytes) => bytes,
            KemCiphertext::Kyber1024(bytes) => bytes,
        }
    }

    pub fn from_bytes(variant: KemVariant, bytes: &[u8]) -> Result<Self, KemError> {
        match variant {
            KemVariant::Kyber512 => {
                if bytes.len() != 768 {
                    return Err(KemError::InvalidLength {
                        expected: 768,
                        actual: bytes.len(),
                    });
                }
                let mut arr = [0u8; 768];
                arr.copy_from_slice(bytes);
                Ok(KemCiphertext::Kyber512(arr))
            }
            KemVariant::Kyber768 => {
                if bytes.len() != 1088 {
                    return Err(KemError::InvalidLength {
                        expected: 1088,
                        actual: bytes.len(),
                    });
                }
                let mut arr = [0u8; 1088];
                arr.copy_from_slice(bytes);
                Ok(KemCiphertext::Kyber768(arr))
            }
            KemVariant::Kyber1024 => {
                if bytes.len() != 1568 {
                    return Err(KemError::InvalidLength {
                        expected: 1568,
                        actual: bytes.len(),
                    });
                }
                let mut arr = [0u8; 1568];
                arr.copy_from_slice(bytes);
                Ok(KemCiphertext::Kyber1024(arr))
            }
        }
    }
}

/// KEM variant identifier.
#[cfg(feature = "pq-kem")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KemVariant {
    Kyber512,
    Kyber768,
    Kyber1024,
}

/// Fixed-size KEM public key variants.
#[cfg(feature = "pq-kem")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KemPublicKey {
    Kyber512([u8; 800]),
    Kyber768([u8; 1184]),
    Kyber1024([u8; 1568]),
}

#[cfg(feature = "pq-kem")]
impl KemPublicKey {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            KemPublicKey::Kyber512(bytes) => bytes,
            KemPublicKey::Kyber768(bytes) => bytes,
            KemPublicKey::Kyber1024(bytes) => bytes,
        }
    }

    pub fn from_bytes(variant: KemVariant, bytes: &[u8]) -> Result<Self, KemError> {
        match variant {
            KemVariant::Kyber512 => {
                if bytes.len() != 800 {
                    return Err(KemError::InvalidLength {
                        expected: 800,
                        actual: bytes.len(),
                    });
                }
                let mut arr = [0u8; 800];
                arr.copy_from_slice(bytes);
                Ok(KemPublicKey::Kyber512(arr))
            }
            KemVariant::Kyber768 => {
                if bytes.len() != 1184 {
                    return Err(KemError::InvalidLength {
                        expected: 1184,
                        actual: bytes.len(),
                    });
                }
                let mut arr = [0u8; 1184];
                arr.copy_from_slice(bytes);
                Ok(KemPublicKey::Kyber768(arr))
            }
            KemVariant::Kyber1024 => {
                if bytes.len() != 1568 {
                    return Err(KemError::InvalidLength {
                        expected: 1568,
                        actual: bytes.len(),
                    });
                }
                let mut arr = [0u8; 1568];
                arr.copy_from_slice(bytes);
                Ok(KemPublicKey::Kyber1024(arr))
            }
        }
    }
}

/// Fixed-size KEM secret key variants.
#[cfg(feature = "pq-kem")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KemSecretKey {
    Kyber512([u8; 1632]),
    Kyber768([u8; 2400]),
    Kyber1024([u8; 3168]),
}

#[cfg(feature = "pq-kem")]
impl KemSecretKey {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            KemSecretKey::Kyber512(bytes) => bytes,
            KemSecretKey::Kyber768(bytes) => bytes,
            KemSecretKey::Kyber1024(bytes) => bytes,
        }
    }

    pub fn from_bytes(variant: KemVariant, bytes: &[u8]) -> Result<Self, KemError> {
        match variant {
            KemVariant::Kyber512 => {
                if bytes.len() != 1632 {
                    return Err(KemError::InvalidLength {
                        expected: 1632,
                        actual: bytes.len(),
                    });
                }
                let mut arr = [0u8; 1632];
                arr.copy_from_slice(bytes);
                Ok(KemSecretKey::Kyber512(arr))
            }
            KemVariant::Kyber768 => {
                if bytes.len() != 2400 {
                    return Err(KemError::InvalidLength {
                        expected: 2400,
                        actual: bytes.len(),
                    });
                }
                let mut arr = [0u8; 2400];
                arr.copy_from_slice(bytes);
                Ok(KemSecretKey::Kyber768(arr))
            }
            KemVariant::Kyber1024 => {
                if bytes.len() != 3168 {
                    return Err(KemError::InvalidLength {
                        expected: 3168,
                        actual: bytes.len(),
                    });
                }
                let mut arr = [0u8; 3168];
                arr.copy_from_slice(bytes);
                Ok(KemSecretKey::Kyber1024(arr))
            }
        }
    }
}

/// Fixed-size shared secret (ML-KEM SSK_LEN = 32 for all parameter sets).
#[cfg(feature = "pq-kem")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KemSharedSecret(pub [u8; 32]);

/// KEM-specific errors.
#[cfg(feature = "pq-kem")]
#[derive(Debug, thiserror::Error)]
pub enum KemError {
    #[error("Invalid length: expected {expected}, got {actual}")]
    InvalidLength { expected: usize, actual: usize },

    #[error("Invalid variant")]
    InvalidVariant,

    #[error("KEM operation failed: {0}")]
    OperationFailed(String),
}

/// Post-quantum serialization trait.
#[cfg(feature = "pq-kem")]
pub trait PostQuantumSerialize {
    fn to_fixed_bytes(&self) -> &[u8];
    fn from_fixed_bytes(variant: KemVariant, bytes: &[u8]) -> Result<Self, KemError>
    where
        Self: Sized;
}

#[cfg(feature = "pq-kem")]
impl PostQuantumSerialize for KemCiphertext {
    fn to_fixed_bytes(&self) -> &[u8] {
        self.as_bytes()
    }

    fn from_fixed_bytes(variant: KemVariant, bytes: &[u8]) -> Result<Self, KemError> {
        Self::from_bytes(variant, bytes)
    }
}

#[cfg(feature = "pq-kem")]
impl PostQuantumSerialize for KemPublicKey {
    fn to_fixed_bytes(&self) -> &[u8] {
        self.as_bytes()
    }

    fn from_fixed_bytes(variant: KemVariant, bytes: &[u8]) -> Result<Self, KemError> {
        Self::from_bytes(variant, bytes)
    }
}

#[cfg(feature = "pq-kem")]
impl PostQuantumSerialize for KemSecretKey {
    fn to_fixed_bytes(&self) -> &[u8] {
        self.as_bytes()
    }

    fn from_fixed_bytes(variant: KemVariant, bytes: &[u8]) -> Result<Self, KemError> {
        Self::from_bytes(variant, bytes)
    }
}

/// Generate an ML-KEM-768 keypair (Kyber768).
#[cfg(feature = "pq-kem")]
pub fn generate_kyber768_keypair() -> Result<(KemPublicKey, KemSecretKey), KemError> {
    use fips203::ml_kem_768;
    let (ek, dk) = ml_kem_768::KG::try_keygen()
        .map_err(|e| KemError::OperationFailed(e.to_string()))?;
    let ek_bytes = ek.into_bytes();
    let dk_bytes = dk.into_bytes();
    Ok((
        KemPublicKey::Kyber768(ek_bytes),
        KemSecretKey::Kyber768(dk_bytes),
    ))
}

/// Encapsulate a shared secret against a public key.
#[cfg(feature = "pq-kem")]
pub fn encapsulate(
    public_key: &KemPublicKey,
) -> Result<(KemSharedSecret, KemCiphertext), KemError> {
    match public_key {
        KemPublicKey::Kyber512(pk_bytes) => {
            use fips203::ml_kem_512;
            let ek = ml_kem_512::EncapsKey::try_from_bytes(*pk_bytes)
                .map_err(|e| KemError::OperationFailed(e.to_string()))?;
            let (ssk, ct) = ek
                .try_encaps()
                .map_err(|e| KemError::OperationFailed(e.to_string()))?;
            Ok((
                KemSharedSecret(ssk.into_bytes()),
                KemCiphertext::Kyber512(ct.into_bytes()),
            ))
        }
        KemPublicKey::Kyber768(pk_bytes) => {
            use fips203::ml_kem_768;
            let ek = ml_kem_768::EncapsKey::try_from_bytes(*pk_bytes)
                .map_err(|e| KemError::OperationFailed(e.to_string()))?;
            let (ssk, ct) = ek
                .try_encaps()
                .map_err(|e| KemError::OperationFailed(e.to_string()))?;
            Ok((
                KemSharedSecret(ssk.into_bytes()),
                KemCiphertext::Kyber768(ct.into_bytes()),
            ))
        }
        KemPublicKey::Kyber1024(pk_bytes) => {
            use fips203::ml_kem_1024;
            let ek = ml_kem_1024::EncapsKey::try_from_bytes(*pk_bytes)
                .map_err(|e| KemError::OperationFailed(e.to_string()))?;
            let (ssk, ct) = ek
                .try_encaps()
                .map_err(|e| KemError::OperationFailed(e.to_string()))?;
            Ok((
                KemSharedSecret(ssk.into_bytes()),
                KemCiphertext::Kyber1024(ct.into_bytes()),
            ))
        }
    }
}

/// Decapsulate a shared secret from a ciphertext using a secret key.
#[cfg(feature = "pq-kem")]
pub fn decapsulate(
    secret_key: &KemSecretKey,
    ciphertext: &KemCiphertext,
) -> Result<KemSharedSecret, KemError> {
    match (secret_key, ciphertext) {
        (KemSecretKey::Kyber512(sk_bytes), KemCiphertext::Kyber512(ct_bytes)) => {
            use fips203::ml_kem_512;
            let dk = ml_kem_512::DecapsKey::try_from_bytes(*sk_bytes)
                .map_err(|e| KemError::OperationFailed(e.to_string()))?;
            let ct = ml_kem_512::CipherText::try_from_bytes(*ct_bytes)
                .map_err(|e| KemError::OperationFailed(e.to_string()))?;
            let ssk = dk
                .try_decaps(&ct)
                .map_err(|e| KemError::OperationFailed(e.to_string()))?;
            Ok(KemSharedSecret(ssk.into_bytes()))
        }
        (KemSecretKey::Kyber768(sk_bytes), KemCiphertext::Kyber768(ct_bytes)) => {
            use fips203::ml_kem_768;
            let dk = ml_kem_768::DecapsKey::try_from_bytes(*sk_bytes)
                .map_err(|e| KemError::OperationFailed(e.to_string()))?;
            let ct = ml_kem_768::CipherText::try_from_bytes(*ct_bytes)
                .map_err(|e| KemError::OperationFailed(e.to_string()))?;
            let ssk = dk
                .try_decaps(&ct)
                .map_err(|e| KemError::OperationFailed(e.to_string()))?;
            Ok(KemSharedSecret(ssk.into_bytes()))
        }
        (KemSecretKey::Kyber1024(sk_bytes), KemCiphertext::Kyber1024(ct_bytes)) => {
            use fips203::ml_kem_1024;
            let dk = ml_kem_1024::DecapsKey::try_from_bytes(*sk_bytes)
                .map_err(|e| KemError::OperationFailed(e.to_string()))?;
            let ct = ml_kem_1024::CipherText::try_from_bytes(*ct_bytes)
                .map_err(|e| KemError::OperationFailed(e.to_string()))?;
            let ssk = dk
                .try_decaps(&ct)
                .map_err(|e| KemError::OperationFailed(e.to_string()))?;
            Ok(KemSharedSecret(ssk.into_bytes()))
        }
        _ => Err(KemError::InvalidVariant),
    }
}

#[cfg(test)]
#[cfg(feature = "pq-kem")]
mod tests {
    use super::*;

    #[test]
    fn test_kem_ciphertext_roundtrip() {
        let data = vec![42u8; 768];
        let ciphertext = KemCiphertext::from_bytes(KemVariant::Kyber512, &data).unwrap();
        assert_eq!(ciphertext.as_bytes(), &data[..]);
    }

    #[test]
    fn test_kem_public_key_roundtrip() {
        let data = vec![99u8; 800];
        let pubkey = KemPublicKey::from_bytes(KemVariant::Kyber512, &data).unwrap();
        assert_eq!(pubkey.as_bytes(), &data[..]);
    }

    #[test]
    fn test_invalid_length() {
        let data = vec![42u8; 100];
        let result = KemCiphertext::from_bytes(KemVariant::Kyber512, &data);
        assert!(result.is_err());
    }

    #[test]
    fn test_kyber768_encaps_decaps_roundtrip() {
        let (pk, sk) = generate_kyber768_keypair().unwrap();
        let (ssk_enc, ct) = encapsulate(&pk).unwrap();
        let ssk_dec = decapsulate(&sk, &ct).unwrap();
        assert_eq!(ssk_enc.0, ssk_dec.0);
    }
}