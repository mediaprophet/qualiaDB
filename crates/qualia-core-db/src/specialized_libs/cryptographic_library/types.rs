// Part of the cryptographic_library module (split from the former mod.rs monolith
// per CLAUDE.md §11 — pure code motion, no behaviour change).
use super::*;

/// Cryptographic operation result
#[derive(Debug, Clone)]
pub struct CryptographicResult<T> {
    pub result: T,
    pub execution_time: u64,
    pub memory_usage: u64,
    pub security_level: SecurityLevel,
    pub compliance_status: ComplianceStatus,
}

/// Compliance status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComplianceStatus {
    Compliant,
    NonCompliant,
    PartiallyCompliant,
    Unknown,
}

/// Key representation
#[derive(Debug, Clone)]
pub struct Key {
    pub key_id: String,
    pub key_type: KeyType,
    pub key_algorithm: KeyAlgorithm,
    pub key_data: Vec<u8>,
    pub metadata: KeyMetadata,
}

/// Signature representation
#[derive(Debug, Clone)]
pub struct Signature {
    pub signature_id: String,
    pub key_id: String,
    pub algorithm: KeyAlgorithm,
    pub data: Vec<u8>,
    pub signature: Vec<u8>,
    pub timestamp: u64,
}

/// Encrypted data
#[derive(Debug, Clone)]
pub struct EncryptedData {
    pub data_id: String,
    pub algorithm: EncryptionAlgorithm,
    pub ciphertext: Vec<u8>,
    pub iv: Vec<u8>,
    pub tag: Vec<u8>,
    pub aad: Vec<u8>,
    pub metadata: EncryptionMetadata,
}

/// Encryption metadata
#[derive(Debug, Clone)]
pub struct EncryptionMetadata {
    pub key_id: String,
    pub algorithm: EncryptionAlgorithm,
    pub mode: EncryptionMode,
    pub padding: Option<EncryptionPadding>,
    pub created_at: u64,
}

/// Hash result
#[derive(Debug, Clone)]
pub struct HashResult {
    pub hash_id: String,
    pub algorithm: String,
    pub input_data: Vec<u8>,
    pub hash_value: Vec<u8>,
    pub timestamp: u64,
}

/// Proof representation
#[derive(Debug, Clone)]
pub struct Proof {
    pub proof_id: String,
    pub system_id: String,
    pub circuit_id: String,
    pub public_inputs: Vec<Vec<u8>>,
    pub proof_data: Vec<u8>,
    pub timestamp: u64,
}
