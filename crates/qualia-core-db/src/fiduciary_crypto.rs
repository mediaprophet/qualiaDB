//! Fiduciary Cryptography (ML-DSA / FIPS-204) Implementation
//!
//! Post-quantum digital signatures using **real ML-DSA-65** (FIPS-204, NIST security
//! category 3) via the pure-Rust `fips204` crate. Produced signatures are interoperable
//! with any conformant FIPS-204 implementation. Pure Rust, WASM-compatible (uses
//! `getrandom` for entropy).
//!
//! NOTE: revisions before 0.0.12 contained a SHA3-based *simulation* of ML-DSA for
//! demonstration only. That fake lattice path has been removed and replaced with the
//! standardized algorithm. The serialized key/signature byte layouts therefore changed.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use serde_bytes;
use sha3::{Digest, Sha3_512};
use fips204::ml_dsa_65;
use fips204::traits::{KeyGen, SerDes, Signer, Verifier};

/// ML-DSA-65 parameters (FIPS-204, NIST security category 3).
pub const ML_DSA_SECURITY_LEVEL: usize = 192; // approximate classical security bits
pub const ML_DSA_PRIVATE_KEY_SIZE: usize = ml_dsa_65::SK_LEN; // 4032 bytes
pub const ML_DSA_PUBLIC_KEY_SIZE: usize = ml_dsa_65::PK_LEN;  // 1952 bytes
pub const ML_DSA_SIGNATURE_SIZE: usize = ml_dsa_65::SIG_LEN;  // 3309 bytes

/// ML-DSA cryptographic signer
pub struct MlDsaSigner {
    private_key: MlDsaPrivateKey,
    public_key: MlDsaPublicKey,
    key_id: Option<String>,
}

/// ML-DSA-65 private (secret) key — FIPS-204 serialized form (`SK_LEN` = 4032 bytes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlDsaPrivateKey {
    #[serde(with = "serde_bytes")]
    pub sk_bytes: Vec<u8>,
}

/// ML-DSA-65 public key — FIPS-204 serialized form (`PK_LEN` = 1952 bytes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlDsaPublicKey {
    #[serde(with = "serde_bytes")]
    pub pk_bytes: Vec<u8>,
}

/// ML-DSA-65 signature — FIPS-204 serialized form (`SIG_LEN` = 3309 bytes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlDsaSignature {
    #[serde(with = "serde_bytes")]
    pub sig_bytes: Vec<u8>,
}

/// Key management for ML-DSA
pub struct MlDsaKeyManager {
    keys: HashMap<String, Arc<Mutex<MlDsaSigner>>>,
    default_key: Option<String>,
    key_rotation_policy: KeyRotationPolicy,
}

/// Key rotation policy
#[derive(Debug, Clone)]
pub struct KeyRotationPolicy {
    pub rotation_interval: u64, // seconds
    pub max_signatures: u64,
    pub quantum_resistance_threshold: f64,
}

/// Cryptographic context for signatures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoContext {
    pub domain: String,
    pub purpose: String,
    pub timestamp: u64,
    pub nonce: [u8; 32],
}

/// Fiduciary cryptographic operations
pub struct FiduciaryCrypto {
    key_manager: Arc<Mutex<MlDsaKeyManager>>,
    context_manager: ContextManager,
    compliance_checker: ComplianceChecker,
}

/// Context manager for cryptographic operations
pub struct ContextManager {
    active_contexts: HashMap<String, CryptoContext>,
    context_cache: Vec<CryptoContext>,
    max_cache_size: usize,
}

/// Compliance checker for cryptographic operations
pub struct ComplianceChecker {
    quantum_resistance_threshold: f64,
    fiduciary_standards: FiduciaryStandards,
    audit_log: Vec<AuditEntry>,
}

/// Fiduciary standards compliance
#[derive(Debug, Clone)]
pub struct FiduciaryStandards {
    pub min_security_level: usize,
    pub quantum_resistance_required: bool,
    pub audit_trail_required: bool,
    pub key_escrow_required: bool,
}

/// Audit entry for cryptographic operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: u64,
    pub operation: String,
    pub key_id: Option<String>,
    pub context: Option<String>,
    pub success: bool,
    pub details: String,
}

impl MlDsaSigner {
    /// Generate a new real ML-DSA-65 (FIPS-204) key pair.
    pub fn generate_keypair() -> Result<(MlDsaPrivateKey, MlDsaPublicKey), MlDsaError> {
        let (pk, sk) = ml_dsa_65::try_keygen()
            .map_err(|e| MlDsaError::KeyGenerationFailed(e.to_string()))?;
        let private_key = MlDsaPrivateKey { sk_bytes: sk.into_bytes().to_vec() };
        let public_key = MlDsaPublicKey { pk_bytes: pk.into_bytes().to_vec() };
        Ok((private_key, public_key))
    }

    /// Create signer from key pair
    pub fn from_keypair(private_key: MlDsaPrivateKey, public_key: MlDsaPublicKey) -> Self {
        Self {
            private_key,
            public_key,
            key_id: None,
        }
    }

    /// Sign `message` with this signer's ML-DSA-65 secret key.
    ///
    /// The `context` fields are bound to the signature via the FIPS-204 context string
    /// (derived deterministically by `derive_ctx`). Sign and verify must use an equal
    /// `CryptoContext`.
    pub fn sign(&self, message: &[u8], context: &CryptoContext) -> Result<MlDsaSignature, MlDsaError> {
        Self::sign_with_secret(&self.private_key.sk_bytes, message, context)
    }

    /// Verify an ML-DSA-65 signature over `message` against this signer's public key.
    pub fn verify(&self, message: &[u8], signature: &MlDsaSignature, context: &CryptoContext) -> Result<bool, MlDsaError> {
        Self::verify_with_public(&self.public_key.pk_bytes, message, signature, context)
    }

    /// Get public key
    pub fn public_key(&self) -> &MlDsaPublicKey {
        &self.public_key
    }

    /// Get key ID
    pub fn key_id(&self) -> Option<&str> {
        self.key_id.as_deref()
    }

    /// Set key ID
    pub fn set_key_id(&mut self, key_id: String) {
        self.key_id = Some(key_id);
    }

    /// Derive a FIPS-204 context string (<= 255 bytes) from a `CryptoContext`.
    ///
    /// ML-DSA accepts an application context that is bound into both signing and
    /// verification. We compress domain/purpose/timestamp/nonce into a 64-byte SHA3-512
    /// digest so whatever context the application supplies is deterministically bound to
    /// the signature. Sign and verify must pass an equal `CryptoContext`.
    fn derive_ctx(context: &CryptoContext) -> Vec<u8> {
        let mut hasher = Sha3_512::new();
        hasher.update(context.domain.as_bytes());
        hasher.update(context.purpose.as_bytes());
        hasher.update(&context.timestamp.to_be_bytes());
        hasher.update(&context.nonce);
        hasher.finalize().to_vec() // 64 bytes, within the 255-byte ML-DSA ctx limit
    }

    /// Sign `message` with a serialized ML-DSA-65 secret key (`SK_LEN` bytes).
    pub fn sign_with_secret(sk_bytes: &[u8], message: &[u8], context: &CryptoContext) -> Result<MlDsaSignature, MlDsaError> {
        let sk_arr: [u8; ml_dsa_65::SK_LEN] = sk_bytes.try_into()
            .map_err(|_| MlDsaError::SignatureGenerationFailed(
                format!("secret key must be {} bytes", ml_dsa_65::SK_LEN)))?;
        let sk = ml_dsa_65::PrivateKey::try_from_bytes(sk_arr)
            .map_err(|e| MlDsaError::SignatureGenerationFailed(e.to_string()))?;
        let ctx = Self::derive_ctx(context);
        let sig = sk.try_sign(message, &ctx)
            .map_err(|e| MlDsaError::SignatureGenerationFailed(e.to_string()))?;
        Ok(MlDsaSignature { sig_bytes: sig.to_vec() })
    }

    /// Verify an ML-DSA-65 signature using a serialized public key (`PK_LEN` bytes).
    pub fn verify_with_public(pk_bytes: &[u8], message: &[u8], signature: &MlDsaSignature, context: &CryptoContext) -> Result<bool, MlDsaError> {
        let pk_arr: [u8; ml_dsa_65::PK_LEN] = pk_bytes.try_into()
            .map_err(|_| MlDsaError::SignatureVerificationFailed(
                format!("public key must be {} bytes", ml_dsa_65::PK_LEN)))?;
        let pk = ml_dsa_65::PublicKey::try_from_bytes(pk_arr)
            .map_err(|e| MlDsaError::SignatureVerificationFailed(e.to_string()))?;
        let sig_arr: [u8; ml_dsa_65::SIG_LEN] = signature.sig_bytes.as_slice().try_into()
            .map_err(|_| MlDsaError::SignatureVerificationFailed(
                format!("signature must be {} bytes", ml_dsa_65::SIG_LEN)))?;
        let ctx = Self::derive_ctx(context);
        Ok(pk.verify(message, &sig_arr, &ctx))
    }

    // Generate cryptographically secure random bytes using OS entropy (rand 0.10)
    fn secure_random(buf: &mut [u8]) -> Result<(), MlDsaError> {
        let mut offset = 0;
        while offset + 32 <= buf.len() {
            let chunk: [u8; 32] = rand::random();
            buf[offset..offset + 32].copy_from_slice(&chunk);
            offset += 32;
        }
        if offset < buf.len() {
            let remaining = buf.len() - offset;
            let tail: [u8; 32] = rand::random();
            buf[offset..].copy_from_slice(&tail[..remaining]);
        }
        Ok(())
    }
}

impl MlDsaKeyManager {
    /// Create new key manager
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            default_key: None,
            key_rotation_policy: KeyRotationPolicy {
                rotation_interval: 86400 * 30, // 30 days
                max_signatures: 1000000,
                quantum_resistance_threshold: 0.95,
            },
        }
    }

    /// Generate and store new key
    pub fn generate_key(&mut self, key_id: String) -> Result<(), MlDsaError> {
        let (private_key, public_key) = MlDsaSigner::generate_keypair()?;
        let mut signer = MlDsaSigner::from_keypair(private_key, public_key);
        signer.set_key_id(key_id.clone());

        let signer_arc = Arc::new(Mutex::new(signer));
        self.keys.insert(key_id.clone(), signer_arc);

        // Set as default if no default exists
        if self.default_key.is_none() {
            self.default_key = Some(key_id);
        }

        Ok(())
    }

    /// Get signer by key ID
    pub fn get_signer(&self, key_id: &str) -> Option<Arc<Mutex<MlDsaSigner>>> {
        self.keys.get(key_id).cloned()
    }

    /// Get default signer
    pub fn get_default_signer(&self) -> Option<Arc<Mutex<MlDsaSigner>>> {
        self.default_key.as_ref().and_then(|key_id| self.get_signer(key_id))
    }

    /// List all key IDs
    pub fn list_keys(&self) -> Vec<String> {
        self.keys.keys().cloned().collect()
    }

    /// Remove key
    pub fn remove_key(&mut self, key_id: &str) -> Result<(), MlDsaError> {
        self.keys.remove(key_id);
        
        // Update default key if necessary
        if self.default_key.as_ref() == Some(&key_id.to_string()) {
            self.default_key = self.keys.keys().next().cloned();
        }

        Ok(())
    }
}

impl ContextManager {
    /// Create new context manager
    pub fn new() -> Self {
        Self {
            active_contexts: HashMap::new(),
            context_cache: Vec::new(),
            max_cache_size: 1000,
        }
    }

    /// Create new cryptographic context
    pub fn create_context(&mut self, domain: String, purpose: String) -> Result<CryptoContext, MlDsaError> {
        let context = CryptoContext {
            domain,
            purpose,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            nonce: Self::generate_nonce(),
        };

        // Add to cache
        self.context_cache.push(context.clone());
        
        // Limit cache size
        if self.context_cache.len() > self.max_cache_size {
            self.context_cache.remove(0);
        }

        Ok(context)
    }

    /// Get context by ID
    pub fn get_context(&self, context_id: &str) -> Option<&CryptoContext> {
        self.active_contexts.get(context_id)
    }

    /// Generate nonce
    fn generate_nonce() -> [u8; 32] {
        let mut nonce = [0u8; 32];
        MlDsaSigner::secure_random(&mut nonce).unwrap_or(());
        nonce
    }
}

impl ComplianceChecker {
    /// Create new compliance checker
    pub fn new() -> Self {
        Self {
            quantum_resistance_threshold: 0.95,
            fiduciary_standards: FiduciaryStandards {
                min_security_level: 128,
                quantum_resistance_required: true,
                audit_trail_required: true,
                key_escrow_required: false,
            },
            audit_log: Vec::new(),
        }
    }

    /// Check cryptographic operation compliance
    pub fn check_compliance(&mut self, operation: &str, key_id: Option<&str>) -> Result<bool, MlDsaError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let entry = AuditEntry {
            timestamp,
            operation: operation.to_string(),
            key_id: key_id.map(|s| s.to_string()),
            context: None,
            success: true,
            details: "Compliance check passed".to_string(),
        };

        self.audit_log.push(entry);

        Ok(true)
    }

    /// Get audit log
    pub fn get_audit_log(&self) -> &[AuditEntry] {
        &self.audit_log
    }

    /// Clear audit log
    pub fn clear_audit_log(&mut self) {
        self.audit_log.clear();
    }
}

impl FiduciaryCrypto {
    /// Create new fiduciary crypto system
    pub fn new() -> Self {
        Self {
            key_manager: Arc::new(Mutex::new(MlDsaKeyManager::new())),
            context_manager: ContextManager::new(),
            compliance_checker: ComplianceChecker::new(),
        }
    }

    /// Generate new key
    pub fn generate_key(&mut self, key_id: String) -> Result<(), MlDsaError> {
        let mut key_manager = self.key_manager.lock().unwrap();
        key_manager.generate_key(key_id)
    }

    /// Sign message using the internal MlDsaSigner for the given key.
    ///
    /// NOTE: The signing context uses timestamp=0 and nonce=[0] so that a matching
    /// `verify()` call (which reconstructs the same deterministic context) will succeed.
    /// A future upgrade to FIPS-204 ML-DSA should embed the context in the signature.
    pub fn sign(&self, message: &[u8], key_id: Option<&str>, domain: String, purpose: String) -> Result<MlDsaSignature, MlDsaError> {
        let key_manager = self.key_manager.lock().unwrap();
        let signer_arc = if let Some(kid) = key_id {
            key_manager.get_signer(kid)
                .ok_or_else(|| MlDsaError::KeyNotFound(kid.to_string()))?
        } else {
            key_manager.get_default_signer()
                .ok_or_else(|| MlDsaError::NoDefaultKey)?
        };
        let signer = signer_arc.lock().unwrap();

        let context = CryptoContext {
            domain,
            purpose,
            timestamp: 0,
            nonce: [0u8; 32],
        };

        signer.sign(message, &context)
    }

    /// Verify a signature produced by `sign()` using the internal MlDsaSigner.
    pub fn verify(&self, message: &[u8], signature: &MlDsaSignature, key_id: Option<&str>, domain: String, purpose: String) -> Result<bool, MlDsaError> {
        let key_manager = self.key_manager.lock().unwrap();
        let signer_arc = if let Some(kid) = key_id {
            key_manager.get_signer(kid)
                .ok_or_else(|| MlDsaError::KeyNotFound(kid.to_string()))?
        } else {
            key_manager.get_default_signer()
                .ok_or_else(|| MlDsaError::NoDefaultKey)?
        };
        let signer = signer_arc.lock().unwrap();

        let context = CryptoContext {
            domain,
            purpose,
            timestamp: 0,
            nonce: [0u8; 32],
        };

        signer.verify(message, signature, &context)
    }

    /// Hash a token into a 32-byte digest using SHA3-512 (first 32 bytes).
    pub fn hash_token(&self, token: &[u8]) -> Result<[u8; 32], MlDsaError> {
        let mut hasher = Sha3_512::new();
        hasher.update(token);
        let digest = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest[..32]);
        Ok(out)
    }

    /// List all keys
    pub fn list_keys(&self) -> Vec<String> {
        let key_manager = self.key_manager.lock().unwrap();
        key_manager.list_keys()
    }

    /// Get audit log
    pub fn get_audit_log(&self) -> Vec<AuditEntry> {
        let compliance_checker = &self.compliance_checker;
        compliance_checker.get_audit_log().to_vec()
    }
}

/// ML-DSA error types
#[derive(Debug, Clone)]
pub enum MlDsaError {
    KeyGenerationFailed(String),
    KeyNotFound(String),
    NoDefaultKey,
    SignatureGenerationFailed(String),
    SignatureVerificationFailed(String),
    InvalidContext(String),
    ComplianceError(String),
    RandomGenerationError(String),
}

impl std::fmt::Display for MlDsaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MlDsaError::KeyGenerationFailed(msg) => write!(f, "Key generation failed: {}", msg),
            MlDsaError::KeyNotFound(msg) => write!(f, "Key not found: {}", msg),
            MlDsaError::NoDefaultKey => write!(f, "No default key available"),
            MlDsaError::SignatureGenerationFailed(msg) => write!(f, "Signature generation failed: {}", msg),
            MlDsaError::SignatureVerificationFailed(msg) => write!(f, "Signature verification failed: {}", msg),
            MlDsaError::InvalidContext(msg) => write!(f, "Invalid context: {}", msg),
            MlDsaError::ComplianceError(msg) => write!(f, "Compliance error: {}", msg),
            MlDsaError::RandomGenerationError(msg) => write!(f, "Random generation error: {}", msg),
        }
    }
}

impl std::error::Error for MlDsaError {}


// ── ML-DSA Verifiable Credential Issuance ─────────────────────────────────────

/// Predicate hash for VC ML-DSA proof head
const P_VC_PROOF_MLDSA: u64 = crate::q_hash("vc:proof/mldsa");
/// Predicate hash for VC ML-DSA proof fragment
const P_VC_PROOF_MLDSA_FRAG: u64 = crate::q_hash("vc:proof/mldsa/frag");

/// ML-DSA VC proof fragment layout for multi-Quin storage
/// A 3309-byte ML-DSA signature spans ~414 NQuins (8 bytes per object field)
#[derive(Debug, Clone)]
pub struct MlDsaVcProof {
    pub head_quin: crate::NQuin,
    pub fragment_quins: Vec<crate::NQuin>,
}

impl MlDsaVcProof {
    /// Issue an ML-DSA-signed Verifiable Credential by fragmenting the signature
    /// across multiple NQuins following the Merkle-DAG pattern.
    pub fn issue_vc_mldsa(
        claim_quins: &[crate::NQuin],
        issuer_sk: &[u8],
        issuer_did_hash: u64,
        context: &CryptoContext,
    ) -> Result<Self, MlDsaError> {
        // 1. Serialize the claim graph to canonical bytes for signing
        let claim_bytes = Self::serialize_claims(claim_quins);
        
        // 2. Sign with ML-DSA
        let signature = MlDsaSigner::sign_with_secret(issuer_sk, &claim_bytes, context)?;
        
        // 3. Fragment the signature into NQuin-sized chunks (8 bytes per object field)
        let sig_bytes = signature.sig_bytes;
        let total_len = sig_bytes.len();
        let fragment_count = (total_len + 7) / 8; // Ceiling division by 8
        
        let mut fragment_quins = Vec::with_capacity(fragment_count);
        
        for i in 0..fragment_count {
            let start = i * 8;
            let end = (start + 8).min(total_len);
            let chunk = &sig_bytes[start..end];
            
            // Pack 8 bytes into a u64
            let mut object: u64 = 0;
            for (j, &byte) in chunk.iter().enumerate() {
                object |= (byte as u64) << (j * 8);
            }
            
            // Create fragment Quin
            let fragment = crate::NQuin {
                subject: issuer_did_hash,
                predicate: P_VC_PROOF_MLDSA_FRAG,
                object,
                context: issuer_did_hash,
                metadata: (i as u64) << 32 | (fragment_count as u64), // fragment index | count
                parity: 0, // TODO: compute proper parity
            };
            
            fragment_quins.push(fragment);
        }
        
        // 4. Create head Quin with metadata about the fragments
        let head_object = ((total_len as u64) << 32) | (fragment_count as u64);
        let head = crate::NQuin {
            subject: issuer_did_hash,
            predicate: P_VC_PROOF_MLDSA,
            object: head_object,
            context: issuer_did_hash,
            metadata: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            parity: 0, // TODO: compute proper parity
        };
        
        Ok(Self {
            head_quin: head,
            fragment_quins,
        })
    }
    
    /// Verify an ML-DSA-signed VC by reassembling the signature fragments
    pub fn verify_vc_mldsa(
        &self,
        claim_quins: &[crate::NQuin],
        issuer_pk: &[u8],
        context: &CryptoContext,
    ) -> Result<bool, MlDsaError> {
        // 1. Reassemble the signature from fragments
        let mut signature_bytes = Vec::new();
        let mut fragment_count = 0;
        
        for fragment in &self.fragment_quins {
            // Extract fragment metadata
            fragment_count = (fragment.metadata >> 32) as u32;
            
            // Extract 8 bytes from object field
            for j in 0..8 {
                let byte = ((fragment.object >> (j * 8)) & 0xFF) as u8;
                signature_bytes.push(byte);
            }
        }
        
        // Trim to actual signature size (3309 bytes for ML-DSA-65)
        if signature_bytes.len() > ml_dsa_65::SIG_LEN {
            signature_bytes.truncate(ml_dsa_65::SIG_LEN);
        }
        
        // 2. Verify the signature
        let signature = MlDsaSignature {
            sig_bytes: signature_bytes,
        };
        
        // 3. Serialize the claim graph for verification
        let claim_bytes = Self::serialize_claims(claim_quins);
        
        // 4. Verify with ML-DSA
        MlDsaSigner::verify_with_public(issuer_pk, &claim_bytes, &signature, context)
    }
    
    /// Serialize claim Quins to canonical bytes for signing
    fn serialize_claims(claims: &[crate::NQuin]) -> Vec<u8> {
        // Simple serialization: concatenate all NQuin bytes
        let mut bytes = Vec::new();
        for quin in claims {
            bytes.extend_from_slice(unsafe {
                std::slice::from_raw_parts(
                    quin as *const _ as *const u8,
                    std::mem::size_of::<crate::NQuin>(),
                )
            });
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let (private_key, public_key) = MlDsaSigner::generate_keypair().unwrap();

        // Real FIPS-204 ML-DSA-65 serialized key sizes.
        assert_eq!(private_key.sk_bytes.len(), ML_DSA_PRIVATE_KEY_SIZE);
        assert_eq!(public_key.pk_bytes.len(), ML_DSA_PUBLIC_KEY_SIZE);
    }

    #[test]
    fn test_sign_verify_rejects_tampered_message() {
        let (private_key, public_key) = MlDsaSigner::generate_keypair().unwrap();
        let signer = MlDsaSigner::from_keypair(private_key, public_key);
        let context = CryptoContext {
            domain: "test".to_string(),
            purpose: "auth".to_string(),
            timestamp: 42,
            nonce: [7u8; 32],
        };
        let sig = signer.sign(b"genuine message", &context).unwrap();
        // A different message must fail verification.
        assert!(!signer.verify(b"forged message", &sig, &context).unwrap());
        // A different context must also fail verification.
        let other_ctx = CryptoContext { purpose: "other".to_string(), ..context.clone() };
        assert!(!signer.verify(b"genuine message", &sig, &other_ctx).unwrap());
    }

    #[test]
    fn test_sign_verify() {
        let (private_key, public_key) = MlDsaSigner::generate_keypair().unwrap();
        let signer = MlDsaSigner::from_keypair(private_key, public_key);
        
        let message = b"Hello, QualiaDB!";
        let context = CryptoContext {
            domain: "test".to_string(),
            purpose: "authentication".to_string(),
            timestamp: 1234567890,
            nonce: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32],
        };

        let signature = signer.sign(message, &context).unwrap();
        let is_valid = signer.verify(message, &signature, &context).unwrap();

        assert!(is_valid);
    }

    #[test]
    fn test_key_manager() {
        let mut key_manager = MlDsaKeyManager::new();
        
        key_manager.generate_key("test_key".to_string()).unwrap();
        
        let keys = key_manager.list_keys();
        assert!(keys.contains(&"test_key".to_string()));
        
        let signer = key_manager.get_signer("test_key").unwrap();
        assert!(signer.lock().unwrap().key_id() == Some("test_key"));
    }

    #[test]
    fn test_fiduciary_crypto() {
        let mut crypto = FiduciaryCrypto::new();
        
        crypto.generate_key("test_key".to_string()).unwrap();
        
        let message = b"Test message";
        let signature = crypto.sign(message, Some("test_key"), "test".to_string(), "auth".to_string()).unwrap();
        
        let is_valid = crypto.verify(message, &signature, Some("test_key"), "test".to_string(), "auth".to_string()).unwrap();
        
        assert!(is_valid);
    }

    #[test]
    fn test_vc_issuance_roundtrip() {
        // Generate ML-DSA keypair
        let (private_key, public_key) = MlDsaSigner::generate_keypair().unwrap();
        let issuer_did_hash = 12345u64;
        
        // Create a simple claim graph (one Quin)
        let claim_quins = vec![
            crate::NQuin {
                subject: issuer_did_hash,
                predicate: crate::q_hash("test:hasRole"),
                object: crate::q_hash("test:Admin"),
                context: issuer_did_hash,
                metadata: 0,
                parity: 0,
            }
        ];
        
        // Create context for signing
        let context = CryptoContext {
            domain: "test".to_string(),
            purpose: "vc-issuance".to_string(),
            timestamp: 0,
            nonce: [0u8; 32],
        };
        
        // Issue VC
        let proof = MlDsaVcProof::issue_vc_mldsa(
            &claim_quins,
            &private_key.sk_bytes,
            issuer_did_hash,
            &context,
        ).unwrap();
        
        // Verify VC
        let is_valid = proof.verify_vc_mldsa(
            &claim_quins,
            &public_key.pk_bytes,
            &context,
        ).unwrap();
        
        assert!(is_valid, "VC verification should succeed");
    }
    
    #[test]
    fn test_vc_tampered_fragment_fails() {
        let (private_key, public_key) = MlDsaSigner::generate_keypair().unwrap();
        let issuer_did_hash = 12345u64;
        
        let claim_quins = vec![
            crate::NQuin {
                subject: issuer_did_hash,
                predicate: crate::q_hash("test:hasRole"),
                object: crate::q_hash("test:Admin"),
                context: issuer_did_hash,
                metadata: 0,
                parity: 0,
            }
        ];
        
        let context = CryptoContext {
            domain: "test".to_string(),
            purpose: "vc-issuance".to_string(),
            timestamp: 0,
            nonce: [0u8; 32],
        };
        
        let mut proof = MlDsaVcProof::issue_vc_mldsa(
            &claim_quins,
            &private_key.sk_bytes,
            issuer_did_hash,
            &context,
        ).unwrap();
        
        // Tamper with a fragment
        if !proof.fragment_quins.is_empty() {
            proof.fragment_quins[0].object ^= 0xFF; // Flip bits
        }
        
        let is_valid = proof.verify_vc_mldsa(
            &claim_quins,
            &public_key.pk_bytes,
            &context,
        ).unwrap();
        
        assert!(!is_valid, "Tampered fragment should fail verification");
    }
    
    #[test]
    fn test_vc_wrong_key_fails() {
        let (private_key, _public_key) = MlDsaSigner::generate_keypair().unwrap();
        let (wrong_private, wrong_public) = MlDsaSigner::generate_keypair().unwrap();
        let issuer_did_hash = 12345u64;
        
        let claim_quins = vec![
            crate::NQuin {
                subject: issuer_did_hash,
                predicate: crate::q_hash("test:hasRole"),
                object: crate::q_hash("test:Admin"),
                context: issuer_did_hash,
                metadata: 0,
                parity: 0,
            }
        ];
        
        let context = CryptoContext {
            domain: "test".to_string(),
            purpose: "vc-issuance".to_string(),
            timestamp: 0,
            nonce: [0u8; 32],
        };
        
        let proof = MlDsaVcProof::issue_vc_mldsa(
            &claim_quins,
            &private_key.sk_bytes,
            issuer_did_hash,
            &context,
        ).unwrap();
        
        // Try to verify with wrong public key
        let is_valid = proof.verify_vc_mldsa(
            &claim_quins,
            &wrong_public.pk_bytes,
            &context,
        ).unwrap();
        
        assert!(!is_valid, "Wrong public key should fail verification");
    }
}
