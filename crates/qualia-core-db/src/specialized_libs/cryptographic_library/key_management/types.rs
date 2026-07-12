// Part of the cryptographic_library::key_management module (split per CLAUDE.md
// §11 — pure code motion, no behaviour change).
//
// Shared key-model value types used across the key-management submodules
// (zones, metadata, algorithm/level enums) plus the generic retention policy.
use super::*;

/// Key zone for different key types
#[derive(Debug, Clone)]
pub struct KeyZone {
    pub zone_id: String,
    pub zone_type: KeyZoneType,
    pub capacity: u64,
    pub keys: HashMap<String, KeyMetadata>,
    pub access_pattern: AccessPattern,
}

/// Key zone types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KeyZoneType {
    /// ML-DSA keys for post-quantum signatures
    MLDSA,
    /// Traditional keys for compatibility
    Traditional,
    /// Symmetric keys for encryption
    Symmetric,
    /// Key exchange keys
    KeyExchange,
    /// Temporary keys for sessions
    Session,
    /// Backup keys for recovery
    Backup,
    /// Hardware security module keys
    HSM,
}

/// Key metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    pub key_id: String,
    pub key_type: KeyType,
    pub key_algorithm: KeyAlgorithm,
    pub key_size: usize,
    pub created_at: u64,
    pub expires_at: u64,
    pub last_used: u64,
    pub usage_count: u64,
    pub security_level: SecurityLevel,
    pub access_level: AccessLevel,
}

/// Key types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KeyType {
    Private,
    Public,
    Symmetric,
    Shared,
    Master,
    Derived,
}

/// Key algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyAlgorithm {
    /// Post-quantum algorithms
    MLDSA,
    Kyber,
    NTRU,
    SPHINCS,
    /// Traditional algorithms
    RSA,
    ECDSA,
    EdDSA,
    /// Symmetric algorithms
    AES,
    ChaCha20,
    /// Hash algorithms
    SHA256,
    SHA512,
    BLAKE3,
}

/// Security levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SecurityLevel {
    Low,
    Medium,
    High,
    Critical,
    TopSecret,
}

/// Access levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccessLevel {
    Public,
    Internal,
    Confidential,
    Secret,
    TopSecret,
}

/// Access patterns for optimization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccessPattern {
    Frequent,
    Occasional,
    Rare,
    Emergency,
    Batch,
}

/// Retention policy
pub struct RetentionPolicy {
    pub retention_days: u32,
    pub auto_delete: bool,
    pub archive_before_delete: bool,
}
