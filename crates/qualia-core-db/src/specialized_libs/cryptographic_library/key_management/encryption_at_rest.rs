// Part of the cryptographic_library::key_management module (split per CLAUDE.md
// §11 — pure code motion, no behaviour change).
//
// Encryption-at-rest for stored key material (AES-256-GCM under a master KEK)
// plus the encryption-policy model and validation engine.
use super::*;

/// Encryption at rest
pub struct EncryptionAtRest {
    encryption_algorithm: EncryptionAlgorithm,
    key_encryption_keys: HashMap<String, Vec<u8>>,
    encryption_policy: EncryptionPolicy,
}

/// Encryption algorithms
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    AES256GCM,
    ChaCha20Poly1305,
    XChaCha20Poly1305,
    Custom(String),
}

/// Encryption policy
///
/// A named policy that defines the cryptographic constraints a key or
/// encryption-at-rest configuration must satisfy. Policies are registered
/// with [`EncryptionPolicyEngine`] and validated against keys and storage
/// configurations.
#[derive(Debug, Clone)]
pub struct EncryptionPolicy {
    /// Unique identifier for the policy.
    pub policy_id: String,
    /// Human-readable policy name.
    pub name: String,
    /// Minimum acceptable key size in bits.
    pub min_key_size: u32,
    /// Algorithms that are permitted by this policy.
    pub required_algorithms: Vec<KeyAlgorithm>,
    /// Compliance standards the policy enforces.
    pub compliance_standards: Vec<ComplianceStandard>,
    /// Maximum age (in days) a key may reach before requiring rotation.
    pub key_rotation_interval_days: u32,
    /// Whether encryption at rest is mandatory under this policy.
    pub require_encryption_at_rest: bool,
}

/// Compliance standards enforced by an [`EncryptionPolicy`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComplianceStandard {
    FIPS140,
    HIPAA,
    GDPR,
    SOC2,
    PciDss,
    ISO27001,
}

/// Compliance requirements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComplianceRequirement {
    FIPS140_2,
    FIPS140_3,
    CommonCriteria,
    HIPAA,
    GDPR,
    SOX,
    PciDss,
    Custom(String),
}

/// Error returned by the encryption policy engine.
#[derive(Debug, Clone, PartialEq)]
pub enum PolicyError {
    /// No policy registered with the requested id.
    UnknownPolicy(String),
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyError::UnknownPolicy(id) => write!(f, "Unknown policy: {}", id),
        }
    }
}

impl std::error::Error for PolicyError {}

/// Severity of a policy violation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Warning,
    Critical,
}

/// A single policy rule violation.
#[derive(Debug, Clone)]
pub struct PolicyViolation {
    pub rule: String,
    pub detail: String,
    pub severity: ViolationSeverity,
}

/// Result of validating a key or configuration against a policy.
#[derive(Debug, Clone)]
pub struct PolicyValidationResult {
    pub policy_id: String,
    pub passed: bool,
    pub violations: Vec<PolicyViolation>,
    pub checked_at: u64,
}

impl PolicyValidationResult {
    /// Convenience: returns true when there are no violations.
    pub fn is_compliant(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Engine that registers encryption policies and validates keys/configurations
/// against them.
pub struct EncryptionPolicyEngine {
    policies: HashMap<String, EncryptionPolicy>,
}

impl EncryptionPolicyEngine {
    /// Create a new empty policy engine.
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
        }
    }

    /// Register a policy. Replaces any existing policy with the same id.
    pub fn add_policy(&mut self, policy: EncryptionPolicy) {
        self.policies.insert(policy.policy_id.clone(), policy);
    }

    /// Look up a policy by id, returning an error if unknown.
    fn get_policy(&self, policy_id: &str) -> Result<&EncryptionPolicy, PolicyError> {
        self.policies
            .get(policy_id)
            .ok_or_else(|| PolicyError::UnknownPolicy(policy_id.to_string()))
    }

    /// Current unix timestamp in seconds.
    fn now_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Validate a key against a policy.
    ///
    /// Checks:
    /// - key size >= `min_key_size`
    /// - key algorithm is in `required_algorithms`
    /// - key age (days since `created_at`) < `key_rotation_interval_days`
    pub fn validate_key(
        &self,
        key: &Key,
        policy_id: &str,
    ) -> Result<PolicyValidationResult, PolicyError> {
        let policy = self.get_policy(policy_id)?;
        let now = Self::now_secs();
        let mut violations: Vec<PolicyViolation> = Vec::new();

        // Key size check (metadata.key_size is in bits).
        if (key.metadata.key_size as u32) < policy.min_key_size {
            violations.push(PolicyViolation {
                rule: "min_key_size".to_string(),
                detail: format!(
                    "key size {} bits is below minimum {} bits",
                    key.metadata.key_size, policy.min_key_size
                ),
                severity: ViolationSeverity::Critical,
            });
        }

        // Algorithm check.
        if !policy.required_algorithms.contains(&key.key_algorithm) {
            violations.push(PolicyViolation {
                rule: "required_algorithms".to_string(),
                detail: format!(
                    "key algorithm {:?} is not in the required set",
                    key.key_algorithm
                ),
                severity: ViolationSeverity::Critical,
            });
        }

        // Key age check (days since created_at).
        let age_seconds = now.saturating_sub(key.metadata.created_at);
        let age_days = age_seconds / 86_400;
        if age_days >= policy.key_rotation_interval_days as u64 {
            violations.push(PolicyViolation {
                rule: "key_rotation_interval_days".to_string(),
                detail: format!(
                    "key age {} days meets/exceeds rotation interval {} days",
                    age_days, policy.key_rotation_interval_days
                ),
                severity: ViolationSeverity::Warning,
            });
        }

        Ok(Self::build_result(policy_id, violations, now))
    }

    /// Validate whether encryption at rest is present when required by a policy.
    pub fn validate_encryption_at_rest(
        &self,
        encrypted: bool,
        policy_id: &str,
    ) -> Result<PolicyValidationResult, PolicyError> {
        let policy = self.get_policy(policy_id)?;
        let now = Self::now_secs();
        let mut violations: Vec<PolicyViolation> = Vec::new();

        if policy.require_encryption_at_rest && !encrypted {
            violations.push(PolicyViolation {
                rule: "require_encryption_at_rest".to_string(),
                detail: "encryption at rest is required but not present".to_string(),
                severity: ViolationSeverity::Critical,
            });
        }

        Ok(Self::build_result(policy_id, violations, now))
    }

    fn build_result(
        policy_id: &str,
        violations: Vec<PolicyViolation>,
        now: u64,
    ) -> PolicyValidationResult {
        PolicyValidationResult {
            policy_id: policy_id.to_string(),
            passed: violations.is_empty(),
            violations,
            checked_at: now,
        }
    }
}

impl Default for EncryptionPolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl EncryptionAtRest {
    pub fn new() -> Self {
        Self {
            encryption_algorithm: EncryptionAlgorithm::AES256GCM,
            key_encryption_keys: HashMap::new(),
            encryption_policy: EncryptionPolicy {
                policy_id: "default".to_string(),
                name: "Default Encryption Policy".to_string(),
                min_key_size: 256,
                required_algorithms: vec![KeyAlgorithm::AES],
                compliance_standards: vec![ComplianceStandard::FIPS140],
                key_rotation_interval_days: 30,
                require_encryption_at_rest: true,
            },
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        // Generate a default master key encryption key (KEK) if none exists
        if !self.key_encryption_keys.contains_key("master_kek") {
            let kek: [u8; 32] = rand::random();
            self.key_encryption_keys
                .insert("master_kek".to_string(), kek.to_vec());
        }
        Ok(())
    }

    /// Encrypt key data using the master KEK via AES-256-GCM.
    /// Returns (ciphertext, nonce, tag) tuple flattened into a single Vec
    /// with layout: [12-byte nonce | 16-byte tag | ciphertext].
    pub fn encrypt_key_data(&self, plaintext: &[u8]) -> Result<Vec<u8>, CryptographicError> {
        let kek = self.key_encryption_keys.get("master_kek").ok_or_else(|| {
            CryptographicError::EncryptionError("no master KEK available".to_string())
        })?;

        use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
        let key = aes_gcm::Key::<Aes256Gcm>::try_from(kek.as_slice()).unwrap();
        let cipher = Aes256Gcm::new(&key);

        let nonce_bytes: [u8; 12] = rand::random();
        let nonce = Nonce::try_from(nonce_bytes.as_slice()).unwrap();

        let ciphertext = cipher.encrypt(&nonce, plaintext).map_err(|e| {
            CryptographicError::EncryptionError(format!("AES-GCM encrypt failed: {e}"))
        })?;

        // Pack: nonce (12) + ciphertext (includes 16-byte GCM tag appended by aes-gcm)
        let mut packed = Vec::with_capacity(12 + ciphertext.len());
        packed.extend_from_slice(&nonce_bytes);
        packed.extend_from_slice(&ciphertext);
        Ok(packed)
    }

    /// Decrypt key data previously encrypted with `encrypt_key_data`.
    pub fn decrypt_key_data(&self, packed: &[u8]) -> Result<Vec<u8>, CryptographicError> {
        if packed.len() < 12 {
            return Err(CryptographicError::DecryptionError(
                "packed data too short".to_string(),
            ));
        }
        let kek = self.key_encryption_keys.get("master_kek").ok_or_else(|| {
            CryptographicError::DecryptionError("no master KEK available".to_string())
        })?;

        use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
        let key = aes_gcm::Key::<Aes256Gcm>::try_from(kek.as_slice()).unwrap();
        let cipher = Aes256Gcm::new(&key);

        let nonce = Nonce::try_from(&packed[..12]).unwrap();
        let ciphertext = &packed[12..];

        cipher.decrypt(&nonce, ciphertext).map_err(|e| {
            CryptographicError::DecryptionError(format!("AES-GCM decrypt failed: {e}"))
        })
    }

    /// Check if encryption at rest is enabled (KEK exists).
    pub fn is_enabled(&self) -> bool {
        self.key_encryption_keys.contains_key("master_kek")
    }

    /// Number of registered KEKs.
    pub fn kek_count(&self) -> usize {
        self.key_encryption_keys.len()
    }

    /// Get the encryption algorithm used at rest.
    pub fn encryption_algorithm(&self) -> &EncryptionAlgorithm {
        &self.encryption_algorithm
    }

    /// Set the encryption algorithm used at rest.
    pub fn set_encryption_algorithm(&mut self, algorithm: EncryptionAlgorithm) {
        self.encryption_algorithm = algorithm;
    }

    /// Get the encryption policy governing encryption at rest.
    pub fn encryption_policy(&self) -> &EncryptionPolicy {
        &self.encryption_policy
    }

    /// Set the encryption policy governing encryption at rest.
    pub fn set_encryption_policy(&mut self, policy: EncryptionPolicy) {
        self.encryption_policy = policy;
    }
}
