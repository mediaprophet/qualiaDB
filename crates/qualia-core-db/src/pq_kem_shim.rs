// ── Post-Quantum KEM Serialization Shim ───────────────────────────────────────
/// Internal abstraction layer for post-quantum key encapsulation mechanisms
/// Insulates the database from volatile upstream crate APIs
/// 
/// This module provides fixed-size, zero-heap compatible wrappers for
/// post-quantum KEM operations, ensuring the NQuin architecture remains
/// insulated from API changes in upstream crates like pqcrypto or fips203.

#[cfg(feature = "pq-kem")]
use serde::{Serialize, Deserialize};

/// Fixed-size KEM ciphertext variants
/// These provide compile-time memory layout guarantees for zero-heap discipline
#[cfg(feature = "pq-kem")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KemCiphertext {
    /// Kyber512 ciphertext (768 bytes)
    Kyber512([u8; 768]),
    
    /// Kyber768 ciphertext (1088 bytes)
    Kyber768([u8; 1088]),
    
    /// Kyber1024 ciphertext (1568 bytes)
    Kyber1024([u8; 1568]),
}

#[cfg(feature = "pq-kem")]
impl KemCiphertext {
    /// Get the byte representation of the ciphertext
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            KemCiphertext::Kyber512(bytes) => bytes,
            KemCiphertext::Kyber768(bytes) => bytes,
            KemCiphertext::Kyber1024(bytes) => bytes,
        }
    }
    
    /// Create ciphertext from bytes
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

/// KEM variant identifier
#[cfg(feature = "pq-kem")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KemVariant {
    Kyber512,
    Kyber768,
    Kyber1024,
}

/// Fixed-size KEM public key variants
#[cfg(feature = "pq-kem")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KemPublicKey {
    /// Kyber512 public key (800 bytes)
    Kyber512([u8; 800]),
    
    /// Kyber768 public key (1184 bytes)
    Kyber768([u8; 1184]),
    
    /// Kyber1024 public key (1568 bytes)
    Kyber1024([u8; 1568]),
}

#[cfg(feature = "pq-kem")]
impl KemPublicKey {
    /// Get the byte representation of the public key
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            KemPublicKey::Kyber512(bytes) => bytes,
            KemPublicKey::Kyber768(bytes) => bytes,
            KemPublicKey::Kyber1024(bytes) => bytes,
        }
    }
    
    /// Create public key from bytes
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

/// Fixed-size KEM secret key variants
#[cfg(feature = "pq-kem")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KemSecretKey {
    /// Kyber512 secret key (1632 bytes)
    Kyber512([u8; 1632]),
    
    /// Kyber768 secret key (2400 bytes)
    Kyber768([u8; 2400]),
    
    /// Kyber1024 secret key (3168 bytes)
    Kyber1024([u8; 3168]),
}

#[cfg(feature = "pq-kem")]
impl KemSecretKey {
    /// Get the byte representation of the secret key
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            KemSecretKey::Kyber512(bytes) => bytes,
            KemSecretKey::Kyber768(bytes) => bytes,
            KemSecretKey::Kyber1024(bytes) => bytes,
        }
    }
    
    /// Create secret key from bytes
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

/// KEM-specific errors
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

/// Post-quantum serialization trait
/// Enforces uniform serialization boundary across all KEM implementations
#[cfg(feature = "pq-kem")]
pub trait PostQuantumSerialize {
    /// Convert to fixed-size byte array
    fn to_fixed_bytes(&self) -> &[u8];
    
    /// Convert from fixed-size byte array
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
}