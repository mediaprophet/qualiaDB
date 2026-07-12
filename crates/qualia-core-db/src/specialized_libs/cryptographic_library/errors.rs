// Part of the cryptographic_library module (split from the former mod.rs monolith
// per CLAUDE.md §11 — pure code motion, no behaviour change).

/// Cryptographic error types
#[derive(Debug, Clone)]
pub enum CryptographicError {
    InvalidKey(String),
    UnsupportedAlgorithm(String),
    StorageError(String),
    EncryptionError(String),
    DecryptionError(String),
    SignatureError(String),
    HashError(String),
    ProofError(String),
    SecurityError(String),
    ComplianceError(String),
    AccessDenied(String),
}

impl std::fmt::Display for CryptographicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptographicError::InvalidKey(msg) => write!(f, "Invalid key: {}", msg),
            CryptographicError::UnsupportedAlgorithm(msg) => {
                write!(f, "Unsupported algorithm: {}", msg)
            }
            CryptographicError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            CryptographicError::EncryptionError(msg) => write!(f, "Encryption error: {}", msg),
            CryptographicError::DecryptionError(msg) => write!(f, "Decryption error: {}", msg),
            CryptographicError::SignatureError(msg) => write!(f, "Signature error: {}", msg),
            CryptographicError::HashError(msg) => write!(f, "Hash error: {}", msg),
            CryptographicError::ProofError(msg) => write!(f, "Proof error: {}", msg),
            CryptographicError::SecurityError(msg) => write!(f, "Security error: {}", msg),
            CryptographicError::ComplianceError(msg) => write!(f, "Compliance error: {}", msg),
            CryptographicError::AccessDenied(msg) => write!(f, "Access denied: {}", msg),
        }
    }
}

impl std::error::Error for CryptographicError {}
