// Part of the cryptographic_library module (split from the former mod.rs monolith
// per CLAUDE.md §11 — pure code motion, no behaviour change).
use super::*;

/// Signature engine for digital signatures
pub struct SignatureEngine {
    signing_algorithms: HashMap<KeyAlgorithm, SigningAlgorithm>,
    verification_algorithms: HashMap<KeyAlgorithm, VerificationAlgorithm>,
    pub(super) signature_storage: SignatureStorage,
    pub(super) performance_optimizer: SignaturePerformanceOptimizer,
}

/// Signing algorithms
#[derive(Debug, Clone)]
pub struct SigningAlgorithm {
    pub algorithm_id: String,
    pub key_algorithm: KeyAlgorithm,
    pub hash_function: String,
    pub parameters: SigningParameters,
}

/// Signing parameters
#[derive(Debug, Clone)]
pub struct SigningParameters {
    pub padding: Option<String>,
    pub salt_length: Option<usize>,
    pub deterministic: bool,
    pub custom_params: HashMap<String, Vec<u8>>,
}

/// Verification algorithm configuration
#[derive(Debug, Clone)]
pub struct VerificationAlgorithmConfig {
    pub algorithm_id: String,
    pub key_algorithm: KeyAlgorithm,
    pub hash_function: String,
    pub parameters: VerificationParameters,
}

/// Verification parameters
#[derive(Debug, Clone)]
pub struct VerificationParameters {
    pub strict_verification: bool,
    pub allow_weak_hashes: bool,
    pub custom_params: HashMap<String, Vec<u8>>,
}

/// Signature storage
pub struct SignatureStorage {
    signatures: HashMap<String, Signature>,
    verification_records: HashMap<String, VerificationRecord>,
    pub(super) audit_log: SignatureAuditLog,
}

/// Signature record
#[derive(Debug, Clone)]
pub struct SignatureRecord {
    pub signature_id: String,
    pub key_id: String,
    pub algorithm: KeyAlgorithm,
    pub data_hash: Vec<u8>,
    pub signature: Vec<u8>,
    pub timestamp: u64,
    pub metadata: SignatureMetadata,
}

/// Signature metadata
#[derive(Debug, Clone)]
pub struct SignatureMetadata {
    pub signer_id: String,
    pub purpose: String,
    pub context: Vec<String>,
    pub validity_period: Option<(u64, u64)>,
}

/// Verification record
#[derive(Debug, Clone)]
pub struct VerificationRecord {
    pub verification_id: String,
    pub signature_id: String,
    pub verifier_id: String,
    pub result: VerificationResult,
    pub timestamp: u64,
}

/// Verification result
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub valid: bool,
    pub error_message: Option<String>,
    pub verification_time: u64,
    pub confidence: f64,
}

/// Signature audit log
pub struct SignatureAuditLog {
    entries: Vec<SignatureAuditEntry>,
    retention_policy: RetentionPolicy,
}

/// Signature audit entry
#[derive(Debug, Clone)]
pub struct SignatureAuditEntry {
    pub entry_id: String,
    pub timestamp: u64,
    pub signature_id: String,
    pub operation: SignatureOperation,
    pub user_id: String,
    pub ip_address: String,
    pub success: bool,
}

/// Signature operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SignatureOperation {
    Sign,
    Verify,
    Revoke,
    Renew,
}

/// Signature performance optimizer
pub struct SignaturePerformanceOptimizer {
    optimization_strategies: Vec<SignatureOptimizationStrategy>,
    performance_metrics: SignaturePerformanceMetrics,
}

/// Signature optimization strategies
#[derive(Debug, Clone, PartialEq)]
pub enum SignatureOptimizationStrategy {
    BatchSigning,
    Precomputation,
    ParallelVerification,
    Caching,
    HardwareAcceleration,
}

/// Signature performance metrics
#[derive(Debug, Clone)]
pub struct SignaturePerformanceMetrics {
    pub average_signing_time: f64,
    pub average_verification_time: f64,
    pub throughput: f64,
    pub error_rate: f64,
    pub cache_hit_rate: f64,
}
impl SignatureEngine {
    pub fn new() -> Self {
        Self {
            signing_algorithms: HashMap::new(),
            verification_algorithms: HashMap::new(),
            signature_storage: SignatureStorage::new(),
            performance_optimizer: SignaturePerformanceOptimizer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.signature_storage.initialize()?;
        self.performance_optimizer.initialize()?;
        Ok(())
    }

    /// Register a signing algorithm configuration.
    pub fn add_signing_algorithm(&mut self, algorithm: SigningAlgorithm) {
        self.signing_algorithms
            .insert(algorithm.key_algorithm, algorithm);
    }

    /// Look up a signing algorithm by key algorithm.
    pub fn get_signing_algorithm(&self, algorithm: &KeyAlgorithm) -> Option<&SigningAlgorithm> {
        self.signing_algorithms.get(algorithm)
    }

    /// Iterate over all registered signing algorithms.
    pub fn list_signing_algorithms(&self) -> impl Iterator<Item = &SigningAlgorithm> {
        self.signing_algorithms.values()
    }

    /// Register a verification algorithm configuration.
    pub fn add_verification_algorithm(
        &mut self,
        key_algorithm: KeyAlgorithm,
        algorithm: VerificationAlgorithm,
    ) {
        self.verification_algorithms
            .insert(key_algorithm, algorithm);
    }

    /// Look up a verification algorithm by key algorithm.
    pub fn get_verification_algorithm(
        &self,
        algorithm: &KeyAlgorithm,
    ) -> Option<&VerificationAlgorithm> {
        self.verification_algorithms.get(algorithm)
    }

    /// Iterate over all registered verification algorithms.
    pub fn list_verification_algorithms(&self) -> impl Iterator<Item = &VerificationAlgorithm> {
        self.verification_algorithms.values()
    }

    /// Deterministic ML-DSA context used for fiduciary sign/verify in this library.
    /// Sign and verify must use the same context, so it is fixed here.
    fn fiduciary_context() -> CryptoContext {
        CryptoContext {
            domain: "qualia.fiduciary".to_string(),
            purpose: "sign".to_string(),
            timestamp: 0,
            nonce: [0u8; 32],
        }
    }

    pub fn sign_data(
        &mut self,
        private_key: &Key,
        data: &[u8],
    ) -> Result<Signature, CryptographicError> {
        let start_time = std::time::Instant::now();

        let signature_data = match private_key.key_algorithm {
            KeyAlgorithm::MLDSA => {
                // Real ML-DSA signs the message directly (it hashes internally); no
                // SHA-256 prehash, and the key material is the full FIPS-204 secret key.
                let ctx = Self::fiduciary_context();
                let sig = MlDsaSigner::sign_with_secret(&private_key.key_data, data, &ctx)
                    .map_err(|e| {
                        CryptographicError::SignatureError(format!("ML-DSA sign failed: {e}"))
                    })?;
                sig.sig_bytes
            }
            KeyAlgorithm::SPHINCS => {
                use fips205::slh_dsa_sha2_256s;
                use fips205::traits::{SerDes, Signer};
                let sk_arr: [u8; slh_dsa_sha2_256s::SK_LEN] =
                    private_key.key_data.as_slice().try_into().map_err(|_| {
                        CryptographicError::InvalidKey(format!(
                            "SPHINCS secret key must be {} bytes",
                            slh_dsa_sha2_256s::SK_LEN
                        ))
                    })?;
                let sk = slh_dsa_sha2_256s::PrivateKey::try_from_bytes(&sk_arr)
                    .map_err(|e| CryptographicError::SignatureError(e.to_string()))?;
                let sig = sk
                    .try_sign(data, b"", true)
                    .map_err(|e| CryptographicError::SignatureError(e.to_string()))?;
                sig.to_vec()
            }
            KeyAlgorithm::ECDSA => {
                #[cfg(feature = "interop-crypto")]
                {
                    use crate::fiduciary_crypto::InteropEcdsaSigner;
                    let signer = InteropEcdsaSigner::from_secret_key(&private_key.key_data)
                        .map_err(|e| CryptographicError::SignatureError(e.to_string()))?;
                    let sig = signer
                        .sign(data)
                        .map_err(|e| CryptographicError::SignatureError(e.to_string()))?;
                    sig.sig_bytes
                }
                #[cfg(not(feature = "interop-crypto"))]
                {
                    return Err(CryptographicError::UnsupportedAlgorithm(
                        "ECDSA requires interop-crypto feature".to_string(),
                    ));
                }
            }
            KeyAlgorithm::RSA => {
                #[cfg(feature = "interop-crypto")]
                {
                    use rsa::pkcs1v15::SigningKey;
                    use rsa::sha2::Sha256;
                    use rsa::signature::{SignatureEncoding, Signer};
                    use rsa::{pkcs8::DecodePrivateKey, RsaPrivateKey};
                    let priv_key = RsaPrivateKey::from_pkcs8_der(&private_key.key_data)
                        .map_err(|e| CryptographicError::SignatureError(e.to_string()))?;
                    let signing_key = SigningKey::<Sha256>::new(priv_key);
                    signing_key.sign(data).to_bytes().to_vec()
                }
                #[cfg(not(feature = "interop-crypto"))]
                {
                    return Err(CryptographicError::UnsupportedAlgorithm(
                        "RSA requires interop-crypto feature".to_string(),
                    ));
                }
            }
            _ => {
                // Ed25519 over a SHA-256 digest of the data.
                let hash = self.compute_data_hash(data)?;
                self.sign_hash(&private_key, &hash)?
            }
        };

        let signature = Signature {
            signature_id: format!(
                "sig_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
            key_id: private_key.key_id.clone(),
            algorithm: private_key.key_algorithm.clone(),
            data: data.to_vec(),
            signature: signature_data,
            timestamp: start_time.elapsed().as_millis() as u64,
        };

        // Store signature
        self.signature_storage.store_signature(signature.clone())?;

        // Audit log the signing operation
        self.signature_storage.audit_log.log_entry(
            &signature.signature_id,
            SignatureOperation::Sign,
            "system",
            true,
        );

        // Record performance metrics
        self.performance_optimizer
            .record_signing_time(start_time.elapsed().as_millis() as f64);

        Ok(signature)
    }

    pub fn verify_signature(
        &mut self,
        public_key: &Key,
        signature: &Signature,
        data: &[u8],
    ) -> Result<bool, CryptographicError> {
        let start_time = std::time::Instant::now();

        let is_valid = match public_key.key_algorithm {
            KeyAlgorithm::MLDSA => {
                let ctx = Self::fiduciary_context();
                let sig = MlDsaSignature {
                    sig_bytes: signature.signature.clone(),
                };
                MlDsaSigner::verify_with_public(&public_key.key_data, data, &sig, &ctx).map_err(
                    |e| CryptographicError::SignatureError(format!("ML-DSA verify failed: {e}")),
                )?
            }
            KeyAlgorithm::SPHINCS => {
                use fips205::slh_dsa_sha2_256s;
                use fips205::traits::{SerDes, Verifier};
                let pk_arr: [u8; slh_dsa_sha2_256s::PK_LEN] =
                    public_key.key_data.as_slice().try_into().map_err(|_| {
                        CryptographicError::InvalidKey(format!(
                            "SPHINCS public key must be {} bytes",
                            slh_dsa_sha2_256s::PK_LEN
                        ))
                    })?;
                let pk = slh_dsa_sha2_256s::PublicKey::try_from_bytes(&pk_arr)
                    .map_err(|e| CryptographicError::SignatureError(e.to_string()))?;
                if signature.signature.len() != slh_dsa_sha2_256s::SIG_LEN {
                    return Ok(false);
                }
                let mut sig_arr = [0u8; slh_dsa_sha2_256s::SIG_LEN];
                sig_arr.copy_from_slice(&signature.signature);
                pk.verify(data, &sig_arr, b"")
            }
            KeyAlgorithm::ECDSA => {
                #[cfg(feature = "interop-crypto")]
                {
                    use crate::fiduciary_crypto::{InteropEcdsaSignature, InteropEcdsaSigner};
                    let signer = InteropEcdsaSigner::from_public_key(&public_key.key_data)
                        .map_err(|e| CryptographicError::SignatureError(e.to_string()))?;
                    let sig = InteropEcdsaSignature {
                        sig_bytes: signature.signature.clone(),
                    };
                    signer
                        .verify(data, &sig)
                        .map_err(|e| CryptographicError::SignatureError(e.to_string()))?
                }
                #[cfg(not(feature = "interop-crypto"))]
                {
                    return Err(CryptographicError::UnsupportedAlgorithm(
                        "ECDSA requires interop-crypto feature".to_string(),
                    ));
                }
            }
            KeyAlgorithm::RSA => {
                #[cfg(feature = "interop-crypto")]
                {
                    use rsa::pkcs1v15::{Signature as RsaSignature, VerifyingKey};
                    use rsa::sha2::Sha256;
                    use rsa::signature::Verifier;
                    use rsa::{pkcs8::DecodePublicKey, RsaPublicKey};
                    let pub_key = RsaPublicKey::from_public_key_der(&public_key.key_data)
                        .map_err(|e| CryptographicError::SignatureError(e.to_string()))?;
                    let verifying_key = VerifyingKey::<Sha256>::new(pub_key);
                    let sig = RsaSignature::try_from(signature.signature.as_slice())
                        .map_err(|e| CryptographicError::SignatureError(e.to_string()))?;
                    verifying_key.verify(data, &sig).is_ok()
                }
                #[cfg(not(feature = "interop-crypto"))]
                {
                    return Err(CryptographicError::UnsupportedAlgorithm(
                        "RSA requires interop-crypto feature".to_string(),
                    ));
                }
            }
            _ => {
                let hash = self.compute_data_hash(data)?;
                self.verify_hash_signature(&public_key, &signature.signature, &hash)?
            }
        };

        // Store verification record
        let verification_record = VerificationRecord {
            verification_id: format!(
                "verif_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
            signature_id: signature.signature_id.clone(),
            verifier_id: "system".to_string(),
            result: VerificationResult {
                valid: is_valid,
                error_message: None,
                verification_time: start_time.elapsed().as_millis() as u64,
                confidence: 1.0,
            },
            timestamp: start_time.elapsed().as_millis() as u64,
        };

        self.signature_storage
            .store_verification_record(verification_record)?;

        // Audit log the verification operation
        self.signature_storage.audit_log.log_entry(
            &signature.signature_id,
            SignatureOperation::Verify,
            "system",
            is_valid,
        );

        // Record performance metrics
        self.performance_optimizer
            .record_verification_time(start_time.elapsed().as_millis() as f64);
        if !is_valid {
            self.performance_optimizer.record_error();
        }

        Ok(is_valid)
    }

    fn compute_data_hash(&self, data: &[u8]) -> Result<Vec<u8>, CryptographicError> {
        // Compute SHA-256 hash
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        Ok(hasher.finalize().to_vec())
    }

    fn sign_hash(&self, private_key: &Key, hash: &[u8]) -> Result<Vec<u8>, CryptographicError> {
        use ed25519_dalek::{Signer, SigningKey};
        if private_key.key_data.len() < 32 {
            return Err(CryptographicError::InvalidKey(
                "Private key too short for signing".to_string(),
            ));
        }
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&private_key.key_data[..32]);
        let signing_key = SigningKey::from_bytes(&seed);
        let sig = signing_key.sign(hash);
        Ok(sig.to_bytes().to_vec())
    }

    fn verify_hash_signature(
        &self,
        public_key: &Key,
        signature: &[u8],
        hash: &[u8],
    ) -> Result<bool, CryptographicError> {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};
        if public_key.key_data.len() < 32 {
            return Err(CryptographicError::InvalidKey(
                "Public key too short for verification".to_string(),
            ));
        }
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&public_key.key_data[..32]);
        let verifying_key = VerifyingKey::from_bytes(&key_bytes)
            .map_err(|e| CryptographicError::InvalidKey(e.to_string()))?;
        if signature.len() != 64 {
            return Ok(false);
        }
        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(signature);
        let sig = Signature::from_bytes(&sig_bytes);
        Ok(verifying_key.verify(hash, &sig).is_ok())
    }
}

impl SignatureStorage {
    pub fn new() -> Self {
        Self {
            signatures: HashMap::new(),
            verification_records: HashMap::new(),
            audit_log: SignatureAuditLog::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    pub fn store_signature(&mut self, signature: Signature) -> Result<(), CryptographicError> {
        self.signatures
            .insert(signature.signature_id.clone(), signature);
        Ok(())
    }

    pub fn store_verification_record(
        &mut self,
        record: VerificationRecord,
    ) -> Result<(), CryptographicError> {
        self.verification_records
            .insert(record.verification_id.clone(), record);
        Ok(())
    }
}

impl SignatureAuditLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            retention_policy: RetentionPolicy {
                retention_days: 365,
                auto_delete: true,
                archive_before_delete: true,
            },
        }
    }

    /// Record a signature operation (sign, verify, revoke, renew).
    pub fn log_entry(
        &mut self,
        signature_id: &str,
        operation: SignatureOperation,
        user_id: &str,
        success: bool,
    ) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let entry = SignatureAuditEntry {
            entry_id: format!("sig_{}_{}", timestamp, self.entries.len()),
            timestamp,
            signature_id: signature_id.to_string(),
            operation,
            user_id: user_id.to_string(),
            ip_address: String::new(),
            success,
        };
        self.entries.push(entry);
        let cutoff =
            timestamp.saturating_sub((self.retention_policy.retention_days as u64) * 86400);
        self.entries.retain(|e| e.timestamp >= cutoff);
    }

    /// Number of logged entries.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Iterate over entries.
    pub fn entries(&self) -> &[SignatureAuditEntry] {
        &self.entries
    }
}

impl SignaturePerformanceOptimizer {
    pub fn new() -> Self {
        Self {
            optimization_strategies: vec![
                SignatureOptimizationStrategy::BatchSigning,
                SignatureOptimizationStrategy::Caching,
            ],
            performance_metrics: SignaturePerformanceMetrics {
                average_signing_time: 0.0,
                average_verification_time: 0.0,
                throughput: 0.0,
                error_rate: 0.0,
                cache_hit_rate: 0.0,
            },
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    /// Get the configured optimization strategies.
    pub fn optimization_strategies(&self) -> &[SignatureOptimizationStrategy] {
        &self.optimization_strategies
    }

    /// Add an optimization strategy if not already present.
    pub fn add_optimization_strategy(&mut self, strategy: SignatureOptimizationStrategy) {
        if !self.optimization_strategies.contains(&strategy) {
            self.optimization_strategies.push(strategy);
        }
    }

    /// Record a signing operation duration (milliseconds) and update running averages.
    pub fn record_signing_time(&mut self, duration_ms: f64) {
        let m = &mut self.performance_metrics;
        if m.average_signing_time == 0.0 {
            m.average_signing_time = duration_ms;
        } else {
            // Exponential moving average for lightweight online tracking
            m.average_signing_time = 0.9 * m.average_signing_time + 0.1 * duration_ms;
        }
        if duration_ms > 0.0 {
            m.throughput = 1000.0 / m.average_signing_time;
        }
    }

    /// Record a verification operation duration (milliseconds).
    pub fn record_verification_time(&mut self, duration_ms: f64) {
        let m = &mut self.performance_metrics;
        if m.average_verification_time == 0.0 {
            m.average_verification_time = duration_ms;
        } else {
            m.average_verification_time = 0.9 * m.average_verification_time + 0.1 * duration_ms;
        }
    }

    /// Record an error (failed sign/verify).
    pub fn record_error(&mut self) {
        let m = &mut self.performance_metrics;
        // Simple error rate approximation — incrementally adjusted
        m.error_rate = 0.95 * m.error_rate + 0.05;
    }

    /// Get a snapshot of the current performance metrics.
    pub fn metrics(&self) -> &SignaturePerformanceMetrics {
        &self.performance_metrics
    }
}

impl SignaturePerformanceMetrics {
    pub fn new() -> Self {
        Self {
            average_signing_time: 0.0,
            average_verification_time: 0.0,
            throughput: 0.0,
            error_rate: 0.0,
            cache_hit_rate: 0.0,
        }
    }
}
