// Part of the cryptographic_library module (split from the former mod.rs monolith
// per CLAUDE.md §11 — pure code motion, no behaviour change).
use super::*;

/// Key manager for secure key storage and management
pub struct KeyManager {
    pub(super) key_storage: KeyStorage,
    pub(super) key_generator: KeyGenerator,
    key_rotator: KeyRotator,
    key_recovery: KeyRecovery,
}

/// Key storage using ZNS for secure key storage
pub struct KeyStorage {
    zones: HashMap<String, KeyZone>,
    pub(super) key_catalog: KeyCatalog,
    encryption_at_rest: EncryptionAtRest,
    pub(super) access_control: KeyAccessControl,
    key_data: HashMap<String, Key>,
}

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

/// Key catalog for key management
pub struct KeyCatalog {
    keys: HashMap<String, KeyMetadata>,
    relationships: HashMap<String, Vec<KeyRelationship>>,
    tags: HashMap<String, Vec<String>>,
    pub(super) search_index: KeySearchIndex,
}

/// Key relationships
#[derive(Debug, Clone)]
pub struct KeyRelationship {
    pub relationship_id: String,
    pub source_key: String,
    pub target_key: String,
    pub relationship_type: KeyRelationshipType,
    pub created_at: u64,
}

/// Key relationship types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KeyRelationshipType {
    /// Public-private key pair
    KeyPair,
    /// Derived from master key
    DerivedFrom,
    /// Backup of original key
    BackupOf,
    /// Rotated version of key
    RotatedFrom,
    /// Shared between parties
    SharedWith,
    /// Hierarchical relationship
    ChildOf,
}

/// Key search index
pub struct KeySearchIndex {
    index_entries: HashMap<String, KeyIndexEntry>,
    pub(super) search_engine: KeySearchEngine,
}

/// Key index entry
#[derive(Debug, Clone)]
pub struct KeyIndexEntry {
    pub entry_id: String,
    pub keywords: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub relevance_score: f64,
}

/// Key search engine
pub struct KeySearchEngine {
    pub(super) engine_type: SearchEngineType,
    pub(super) indexing_strategy: IndexingStrategy,
    /// Indexed key metadata keyed by key id.
    key_metadata: HashMap<String, KeyMetadata>,
    /// Tags attached to each key.
    key_tags: HashMap<String, Vec<String>>,
    /// Purpose assigned to each key.
    key_purposes: HashMap<String, KeyPurpose>,
}

/// Search engine types
#[derive(Debug, Clone, PartialEq)]
pub enum SearchEngineType {
    FullText,
    Semantic,
    Hybrid,
    Encrypted,
}

/// Indexing strategies
#[derive(Debug, Clone, PartialEq)]
pub enum IndexingStrategy {
    Inverted,
    Ngram,
    SkipGram,
    BM25,
    Encrypted,
}

/// Key purposes for search filtering.
///
/// A key's purpose describes the cryptographic role it is intended for
/// (signing, encryption, key exchange, etc.). This is orthogonal to the
/// raw [`KeyAlgorithm`] — e.g. an `AES` key may be used for either
/// `Encryption` or `Decryption`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyPurpose {
    /// Digital signature generation
    Signing,
    /// Signature verification
    Verification,
    /// Data encryption
    Encryption,
    /// Data decryption
    Decryption,
    /// Key exchange / key agreement
    KeyExchange,
    /// Authentication / identity proof
    Authentication,
    /// Key derivation
    Derivation,
    /// Hashing / fingerprinting
    Hashing,
}

/// Structured query against the [`KeySearchIndex`].
///
/// Every field is optional; a `None`/empty field is treated as a wildcard
/// (i.e. it does not constrain the result set). When multiple fields are
/// populated they are combined as a logical AND — only keys satisfying every
/// constraint are returned.
#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    /// Free-form text matched as a case-insensitive substring against the
    /// key id and metadata fields (algorithm, key type, security level, …).
    pub text: Option<String>,
    /// Exact algorithm filter.
    pub algorithm: Option<KeyAlgorithm>,
    /// Exact purpose filter.
    pub purpose: Option<KeyPurpose>,
    /// Tag filter — a key matches if it carries *any* of the listed tags.
    pub tags: Vec<String>,
    /// Inclusive lower bound on the key creation timestamp.
    pub created_after: Option<u64>,
    /// Inclusive upper bound on the key creation timestamp.
    pub created_before: Option<u64>,
}

impl SearchQuery {
    /// Build an empty query (matches everything).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the free-form text filter.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Set the algorithm filter.
    pub fn with_algorithm(mut self, algorithm: KeyAlgorithm) -> Self {
        self.algorithm = Some(algorithm);
        self
    }

    /// Set the purpose filter.
    pub fn with_purpose(mut self, purpose: KeyPurpose) -> Self {
        self.purpose = Some(purpose);
        self
    }

    /// Add a tag to the tag filter.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set the inclusive creation-date lower bound.
    pub fn with_created_after(mut self, ts: u64) -> Self {
        self.created_after = Some(ts);
        self
    }

    /// Set the inclusive creation-date upper bound.
    pub fn with_created_before(mut self, ts: u64) -> Self {
        self.created_before = Some(ts);
        self
    }
}

/// A single search hit returned by [`KeySearchIndex::search`].
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    /// The id of the matching key.
    pub key_id: String,
    /// Aggregate relevance score in the range `0.0..=1.0+`. Higher is more
    /// relevant; exact matches contribute `1.0` and partial matches `0.5`,
    /// with multiple matching fields summed.
    pub relevance_score: f64,
    /// Names of the fields that contributed to the match (e.g. `"key_id"`,
    /// `"algorithm"`, `"tag:production"`).
    pub matched_fields: Vec<String>,
}

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

/// Key access control
pub struct KeyAccessControl {
    access_policies: HashMap<String, AccessPolicy>,
    authentication_methods: Vec<AuthenticationMethod>,
    audit_log: AccessAuditLog,
}

/// Access policies
#[derive(Debug, Clone)]
pub struct AccessPolicy {
    pub policy_id: String,
    pub key_id: String,
    pub allowed_operations: Vec<KeyOperation>,
    pub required_auth: Vec<AuthenticationMethod>,
    pub time_restrictions: TimeRestrictions,
    pub ip_restrictions: Vec<String>,
}

/// Key operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KeyOperation {
    Read,
    Write,
    Delete,
    Sign,
    Verify,
    Encrypt,
    Decrypt,
    Derive,
    Rotate,
    Export,
    Import,
}

/// Authentication methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuthenticationMethod {
    Password,
    Biometric,
    HardwareToken,
    MultiFactor,
    Certificate,
    ZeroKnowledge,
}

/// Time restrictions
#[derive(Debug, Clone)]
pub struct TimeRestrictions {
    pub allowed_hours: Vec<u8>,
    pub allowed_days: Vec<u8>,
    pub start_date: Option<u64>,
    pub end_date: Option<u64>,
}

/// Access audit log
pub struct AccessAuditLog {
    entries: Vec<AccessLogEntry>,
    retention_policy: RetentionPolicy,
}

/// Access log entry
#[derive(Debug, Clone)]
pub struct AccessLogEntry {
    pub entry_id: String,
    pub timestamp: u64,
    pub key_id: String,
    pub operation: KeyOperation,
    pub user_id: String,
    pub ip_address: String,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Retention policy
pub struct RetentionPolicy {
    pub retention_days: u32,
    pub auto_delete: bool,
    pub archive_before_delete: bool,
}

/// Key generator
pub struct KeyGenerator {
    generation_algorithms: HashMap<KeyAlgorithm, GenerationAlgorithm>,
    entropy_sources: Vec<EntropySource>,
    selected_entropy_source: Option<EntropySource>,
    pub(super) quality_metrics: KeyQualityMetrics,
}

/// Generation algorithms
#[derive(Debug, Clone)]
pub struct GenerationAlgorithm {
    pub algorithm_id: String,
    pub algorithm: KeyAlgorithm,
    pub parameters: GenerationParameters,
    pub security_level: SecurityLevel,
}

/// Generation parameters
#[derive(Debug, Clone)]
pub struct GenerationParameters {
    pub key_size: usize,
    pub curve: Option<String>,
    pub hash_function: Option<String>,
    pub custom_params: HashMap<String, Vec<u8>>,
}

/// Entropy sources
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EntropySource {
    HardwareRNG,
    OSRandom,
    UserInput,
    Network,
    Quantum,
    Hybrid,
}

/// Key quality metrics
pub struct KeyQualityMetrics {
    pub entropy_score: f64,
    pub randomness_test_results: Vec<RandomnessTestResult>,
    pub security_assessment: SecurityAssessment,
}

/// Randomness test results
#[derive(Debug, Clone)]
pub struct RandomnessTestResult {
    pub test_name: String,
    pub test_type: RandomnessTestType,
    pub p_value: f64,
    pub passed: bool,
}

/// Randomness test types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RandomnessTestType {
    Frequency,
    BlockFrequency,
    Runs,
    LongestRun,
    Serial,
    CUSUM,
    Custom(String),
}

/// Security assessment
#[derive(Debug, Clone)]
pub struct SecurityAssessment {
    pub vulnerability_score: f64,
    pub compliance_score: f64,
    pub recommendations: Vec<SecurityRecommendation>,
}

/// Security recommendations
#[derive(Debug, Clone)]
pub struct SecurityRecommendation {
    pub recommendation_id: String,
    pub severity: RecommendationSeverity,
    pub description: String,
    pub action_required: bool,
}

/// Recommendation severity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecommendationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Key rotator
pub struct KeyRotator {
    rotation_policies: HashMap<KeyAlgorithm, RotationPolicy>,
    rotation_schedule: RotationSchedule,
    rotation_history: RotationHistory,
}

/// Rotation policies
#[derive(Debug, Clone)]
pub struct RotationPolicy {
    pub policy_id: String,
    pub algorithm: KeyAlgorithm,
    pub rotation_interval: u64,
    pub grace_period: u64,
    pub automatic_rotation: bool,
    pub notification_settings: NotificationSettings,
}

/// Notification settings
#[derive(Debug, Clone)]
pub struct NotificationSettings {
    pub notify_before_rotation: bool,
    pub notification_days: u32,
    pub notification_channels: Vec<NotificationChannel>,
}

/// Notification channels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email,
    SMS,
    Webhook,
    Slack,
    Custom(String),
}

/// Rotation schedule
pub struct RotationSchedule {
    pub scheduled_rotations: Vec<ScheduledRotation>,
    pub rotation_queue: Vec<QueuedRotation>,
    pub completed_rotations: Vec<CompletedRotation>,
}

/// Scheduled rotation
#[derive(Debug, Clone)]
pub struct ScheduledRotation {
    pub rotation_id: String,
    pub key_id: String,
    pub scheduled_time: u64,
    pub rotation_type: RotationType,
}

/// Rotation types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RotationType {
    Automatic,
    Manual,
    Emergency,
    Compliance,
}

/// Queued rotation
#[derive(Debug, Clone)]
pub struct QueuedRotation {
    pub rotation_id: String,
    pub key_id: String,
    pub queued_at: u64,
    pub priority: RotationPriority,
}

/// Rotation priorities
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RotationPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Completed rotation
#[derive(Debug, Clone)]
pub struct CompletedRotation {
    pub rotation_id: String,
    pub key_id: String,
    pub old_key_id: String,
    pub new_key_id: String,
    pub completed_at: u64,
    pub success: bool,
}

/// Rotation history
pub struct RotationHistory {
    entries: Vec<RotationHistoryEntry>,
    retention_policy: RetentionPolicy,
}

/// Rotation history entry
#[derive(Debug, Clone)]
pub struct RotationHistoryEntry {
    pub entry_id: String,
    pub timestamp: u64,
    pub key_id: String,
    pub rotation_type: RotationType,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Key recovery
pub struct KeyRecovery {
    recovery_methods: Vec<RecoveryMethod>,
    recovery_policies: RecoveryPolicies,
    recovery_attempts: RecoveryAttempts,
}

/// Recovery methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecoveryMethod {
    ShamirSecretSharing,
    EncryptedBackup,
    HardwareToken,
    BiometricRecovery,
    SocialRecovery,
    CloudBackup,
}

/// Recovery policies
pub struct RecoveryPolicies {
    pub minimum_shares: usize,
    pub total_shares: usize,
    pub recovery_threshold: f64,
    pub time_lock: u64,
    pub geo_restrictions: Vec<String>,
}

/// Recovery attempts
pub struct RecoveryAttempts {
    pub attempts: Vec<RecoveryAttempt>,
    pub lockout_policy: LockoutPolicy,
}

/// Recovery attempt
#[derive(Debug, Clone)]
pub struct RecoveryAttempt {
    pub attempt_id: String,
    pub timestamp: u64,
    pub key_id: String,
    pub method: RecoveryMethod,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Lockout policy
pub struct LockoutPolicy {
    pub max_attempts: u32,
    pub lockout_duration: u64,
    pub exponential_backoff: bool,
}
impl KeyManager {
    pub fn new() -> Self {
        Self {
            key_storage: KeyStorage::new(),
            key_generator: KeyGenerator::new(),
            key_rotator: KeyRotator::new(),
            key_recovery: KeyRecovery::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.key_storage.initialize()?;
        self.key_generator.initialize()?;
        self.key_rotator.initialize()?;
        self.key_recovery.initialize()?;
        Ok(())
    }

    pub fn generate_key_pair(
        &mut self,
        key_id: String,
        _key_type: KeyType,
        algorithm: KeyAlgorithm,
        security_level: SecurityLevel,
    ) -> Result<(Key, Key), CryptographicError> {
        // Generate private key
        let private_key = self.key_generator.generate_key(
            format!("{}_private", key_id),
            KeyType::Private,
            algorithm,
            security_level,
        )?;

        // Generate public key from private key
        let public_key = self
            .key_generator
            .derive_public_key(&private_key, format!("{}_public", key_id))?;

        Ok((private_key, public_key))
    }

    pub fn store_key(&mut self, key: Key) -> Result<(), CryptographicError> {
        self.key_storage.store_key(key)
    }

    pub fn get_key(&self, key_id: &str) -> Result<Key, CryptographicError> {
        self.key_storage.get_key(key_id)
    }

    pub fn rotate_key(&mut self, old_key: &Key) -> Result<Key, CryptographicError> {
        self.key_rotator.rotate_key(old_key)
    }

    pub fn list_keys(&self) -> Vec<String> {
        self.key_storage.list_keys()
    }

    pub fn get_key_metadata(&self, key_id: &str) -> Option<KeyMetadata> {
        self.key_storage.get_key_metadata(key_id)
    }
}

impl KeyStorage {
    pub fn new() -> Self {
        Self {
            zones: HashMap::new(),
            key_catalog: KeyCatalog::new(),
            encryption_at_rest: EncryptionAtRest::new(),
            access_control: KeyAccessControl::new(),
            key_data: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.create_zones()?;
        self.key_catalog.initialize()?;
        self.encryption_at_rest.initialize()?;
        self.access_control.initialize()?;
        Ok(())
    }

    fn create_zones(&mut self) -> Result<(), CryptographicError> {
        let zones = vec![
            ("mldsa", KeyZoneType::MLDSA),
            ("traditional", KeyZoneType::Traditional),
            ("symmetric", KeyZoneType::Symmetric),
            ("keyexchange", KeyZoneType::KeyExchange),
            ("session", KeyZoneType::Session),
            ("backup", KeyZoneType::Backup),
            ("hsm", KeyZoneType::HSM),
        ];

        for (name, zone_type) in zones {
            let zone = KeyZone {
                zone_id: name.to_string(),
                zone_type,
                capacity: 1024 * 1024 * 1024, // 1GB
                keys: HashMap::new(),
                access_pattern: AccessPattern::Frequent,
            };
            self.zones.insert(name.to_string(), zone);
        }

        Ok(())
    }

    pub fn store_key(&mut self, key: Key) -> Result<(), CryptographicError> {
        // Determine best zone for this key
        let zone_id = self.select_best_zone(&key)?;

        // Store in zone
        let zone = self
            .zones
            .get_mut(&zone_id)
            .ok_or_else(|| CryptographicError::StorageError("Zone not found".to_string()))?;

        zone.keys.insert(key.key_id.clone(), key.metadata.clone());

        // Register in catalog
        self.key_catalog.register_key(key.metadata.clone());

        // Store actual key data
        self.key_data.insert(key.key_id.clone(), key);

        Ok(())
    }

    pub fn get_key(&self, key_id: &str) -> Result<Key, CryptographicError> {
        self.key_data
            .get(key_id)
            .cloned()
            .ok_or_else(|| CryptographicError::StorageError(format!("Key not found: {}", key_id)))
    }

    /// Get a key with access control enforcement. Returns an error if the
    /// operation is not permitted by any registered access policy.
    /// Deny-by-default when policies exist but none match.
    pub fn get_key_with_access(
        &mut self,
        key_id: &str,
        operation: KeyOperation,
        user_id: &str,
    ) -> Result<Key, CryptographicError> {
        // If policies are registered, enforce them
        if self.access_control.policy_count() > 0 {
            if !self
                .access_control
                .check_permission(key_id, operation.clone())
            {
                self.access_control
                    .log_access(key_id, operation.clone(), user_id, false);
                return Err(CryptographicError::AccessDenied(format!(
                    "Access denied for operation {:?} on key {}",
                    operation, key_id
                )));
            }
        }
        let key = self.get_key(key_id)?;
        self.access_control
            .log_access(key_id, operation, user_id, true);
        Ok(key)
    }

    pub fn get_key_metadata(&self, key_id: &str) -> Option<KeyMetadata> {
        for zone in self.zones.values() {
            if let Some(metadata) = zone.keys.get(key_id) {
                return Some(metadata.clone());
            }
        }
        None
    }

    pub fn list_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        for zone in self.zones.values() {
            keys.extend(zone.keys.keys().cloned());
        }
        keys
    }

    fn select_best_zone(&self, key: &Key) -> Result<String, CryptographicError> {
        // Simple selection logic - in real implementation would be more sophisticated
        match key.key_algorithm {
            KeyAlgorithm::MLDSA => Ok("mldsa".to_string()),
            KeyAlgorithm::AES | KeyAlgorithm::ChaCha20 => Ok("symmetric".to_string()),
            _ => Ok("traditional".to_string()),
        }
    }
}

impl KeyCatalog {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            relationships: HashMap::new(),
            tags: HashMap::new(),
            search_index: KeySearchIndex::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.search_index.initialize()?;
        Ok(())
    }

    /// Register a relationship between two keys (e.g. KeyPair, RotatedFrom, DerivedFrom).
    pub fn add_relationship(
        &mut self,
        source_key: &str,
        target_key: &str,
        rel_type: KeyRelationshipType,
    ) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let rel = KeyRelationship {
            relationship_id: format!("rel_{}_{}_{}", source_key, target_key, now),
            source_key: source_key.to_string(),
            target_key: target_key.to_string(),
            relationship_type: rel_type,
            created_at: now,
        };
        self.relationships
            .entry(source_key.to_string())
            .or_default()
            .push(rel);
    }

    /// Get all relationships for a given key (as source).
    pub fn get_relationships(&self, key_id: &str) -> &[KeyRelationship] {
        match self.relationships.get(key_id) {
            Some(rels) => rels,
            None => &[],
        }
    }

    /// Find the related key of a given type (e.g. find the public key paired with a private key).
    pub fn find_related(
        &self,
        key_id: &str,
        rel_type: KeyRelationshipType,
    ) -> Option<&KeyRelationship> {
        self.relationships
            .get(key_id)
            .and_then(|rels| rels.iter().find(|r| r.relationship_type == rel_type))
    }

    /// Register key metadata in the catalog.
    pub fn register_key(&mut self, metadata: KeyMetadata) {
        // Populate the search index so the key is discoverable by keyword/metadata.
        let mut index_metadata = HashMap::new();
        index_metadata.insert("key_type".to_string(), format!("{:?}", metadata.key_type));
        index_metadata.insert(
            "algorithm".to_string(),
            format!("{:?}", metadata.key_algorithm),
        );
        index_metadata.insert(
            "security_level".to_string(),
            format!("{:?}", metadata.security_level),
        );
        index_metadata.insert("key_size".to_string(), metadata.key_size.to_string());

        let entry = KeyIndexEntry {
            entry_id: metadata.key_id.clone(),
            keywords: vec![
                metadata.key_id.clone(),
                format!("{:?}", metadata.key_algorithm),
                format!("{:?}", metadata.key_type),
                format!("{:?}", metadata.security_level),
            ],
            metadata: index_metadata,
            relevance_score: 1.0,
        };
        self.search_index.index(entry);

        self.keys.insert(metadata.key_id.clone(), metadata);
    }

    /// Add a tag to a key for searchability.
    pub fn add_tag(&mut self, key_id: &str, tag: &str) {
        self.tags
            .entry(key_id.to_string())
            .or_default()
            .push(tag.to_string());
    }

    /// Get tags for a key.
    pub fn get_tags(&self, key_id: &str) -> &[String] {
        match self.tags.get(key_id) {
            Some(tags) => tags,
            None => &[],
        }
    }

    /// Search keys by keyword, tag, or metadata (case-insensitive substring).
    /// Returns the matching key IDs.
    pub fn search(&self, query: &str) -> Vec<String> {
        let q = query.to_lowercase();
        let mut matches = std::collections::HashSet::new();

        // 1. Match against registered key metadata (key_id, algorithm, type, level).
        for (key_id, metadata) in &self.keys {
            if key_id.to_lowercase().contains(&q)
                || format!("{:?}", metadata.key_algorithm)
                    .to_lowercase()
                    .contains(&q)
                || format!("{:?}", metadata.key_type)
                    .to_lowercase()
                    .contains(&q)
                || format!("{:?}", metadata.security_level)
                    .to_lowercase()
                    .contains(&q)
            {
                matches.insert(key_id.clone());
            }
        }

        // 2. Match against tags (case-insensitive).
        for (key_id, tags) in &self.tags {
            if tags.iter().any(|t| t.to_lowercase().contains(&q)) {
                matches.insert(key_id.clone());
            }
        }

        // 3. Match against the search index entries.
        for entry in self.search_index.search_by_keyword(query) {
            matches.insert(entry.entry_id.clone());
        }

        matches.into_iter().collect()
    }

    /// Find all keys with a given tag (case-insensitive).
    pub fn get_by_tag(&self, tag: &str) -> Vec<String> {
        let t = tag.to_lowercase();
        self.tags
            .iter()
            .filter(|(_, tags)| tags.iter().any(|x| x.to_lowercase() == t))
            .map(|(key_id, _)| key_id.clone())
            .collect()
    }

    /// Number of registered keys.
    pub fn key_count(&self) -> usize {
        self.keys.len()
    }

    /// Number of tracked relationships.
    pub fn relationship_count(&self) -> usize {
        self.relationships.values().map(|v| v.len()).sum()
    }
}

impl KeySearchIndex {
    pub fn new() -> Self {
        Self {
            index_entries: HashMap::new(),
            search_engine: KeySearchEngine::new(SearchEngineType::Encrypted),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        // Actually configure the search engine rather than returning Ok(()) blindly.
        self.search_engine.engine_type = SearchEngineType::Hybrid;
        self.search_engine.indexing_strategy = IndexingStrategy::Inverted;
        Ok(())
    }

    /// Add an entry to the search index, keyed by its `entry_id`.
    pub fn index(&mut self, entry: KeyIndexEntry) {
        self.index_entries.insert(entry.entry_id.clone(), entry);
    }

    /// Index a key together with its metadata so it becomes discoverable via
    /// [`Self::search`] with a structured [`SearchQuery`].
    pub fn index_key(&mut self, key_id: &str, metadata: &KeyMetadata) {
        self.search_engine.index_key(key_id, metadata);
    }

    /// Attach a tag to an indexed key.
    pub fn add_tag(&mut self, key_id: &str, tag: &str) {
        self.search_engine.add_tag(key_id, tag);
    }

    /// Assign a purpose to an indexed key.
    pub fn set_purpose(&mut self, key_id: &str, purpose: KeyPurpose) {
        self.search_engine.set_purpose(key_id, purpose);
    }

    /// Keyword search across index entries (case-insensitive substring match).
    /// Returns references to every entry whose `entry_id` or any keyword contains
    /// the query substring.
    pub fn search_by_keyword(&self, query: &str) -> Vec<&KeyIndexEntry> {
        let q = query.to_lowercase();
        self.index_entries
            .values()
            .filter(|entry| {
                entry.entry_id.to_lowercase().contains(&q)
                    || entry.keywords.iter().any(|k| k.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// Structured search over the indexed keys. Delegates to the underlying
    /// [`KeySearchEngine`].
    pub fn search(&self, query: &SearchQuery) -> Vec<SearchResult> {
        self.search_engine.search(query)
    }

    /// Number of indexed entries.
    pub fn entry_count(&self) -> usize {
        self.index_entries.len()
    }

    /// Number of keys indexed via [`Self::index_key`].
    pub fn indexed_key_count(&self) -> usize {
        self.search_engine.indexed_key_count()
    }
}

impl KeySearchEngine {
    pub fn new(engine_type: SearchEngineType) -> Self {
        let indexing_strategy = match engine_type {
            SearchEngineType::Encrypted => IndexingStrategy::Encrypted,
            _ => IndexingStrategy::Inverted,
        };
        Self {
            engine_type,
            indexing_strategy,
            key_metadata: HashMap::new(),
            key_tags: HashMap::new(),
            key_purposes: HashMap::new(),
        }
    }

    /// Index a key together with its metadata.
    ///
    /// The metadata is stored verbatim so that subsequent [`Self::search`]
    /// calls can filter on algorithm, creation date, and the textual
    /// representation of the metadata fields.
    pub fn index_key(&mut self, key_id: &str, metadata: &KeyMetadata) {
        self.key_metadata
            .insert(key_id.to_string(), metadata.clone());
    }

    /// Attach a tag to a previously indexed key. Tags are case-sensitive but
    /// compared case-insensitively during search.
    pub fn add_tag(&mut self, key_id: &str, tag: &str) {
        self.key_tags
            .entry(key_id.to_string())
            .or_default()
            .push(tag.to_string());
    }

    /// Assign a purpose to a previously indexed key.
    pub fn set_purpose(&mut self, key_id: &str, purpose: KeyPurpose) {
        self.key_purposes.insert(key_id.to_string(), purpose);
    }

    /// Number of indexed keys.
    pub fn indexed_key_count(&self) -> usize {
        self.key_metadata.len()
    }

    /// Run a structured [`SearchQuery`] against the indexed keys.
    ///
    /// Each populated query field acts as both a hard filter (non-matching
    /// keys are excluded) and a relevance signal. Relevance is accumulated:
    /// an exact match contributes `1.0`, a partial (substring) match
    /// contributes `0.5`. Results are returned sorted by descending
    /// relevance score.
    pub fn search(&self, query: &SearchQuery) -> Vec<SearchResult> {
        let q_text = query.text.as_deref().map(|t| t.trim().to_lowercase());
        let q_text = q_text.filter(|t| !t.is_empty());

        let mut results: Vec<SearchResult> = self
            .key_metadata
            .iter()
            .filter_map(|(key_id, metadata)| {
                let mut score: f64 = 0.0;
                let mut matched_fields: Vec<String> = Vec::new();

                // --- Text filter (substring on key_id + metadata fields) ---
                if let Some(ref q) = q_text {
                    let mut text_matched = false;

                    // key_id
                    let key_id_lc = key_id.to_lowercase();
                    if key_id_lc == *q {
                        score += 1.0;
                        matched_fields.push("key_id".to_string());
                        text_matched = true;
                    } else if key_id_lc.contains(q) {
                        score += 0.5;
                        matched_fields.push("key_id".to_string());
                        text_matched = true;
                    }

                    // algorithm
                    let algo_lc = format!("{:?}", metadata.key_algorithm).to_lowercase();
                    if algo_lc == *q {
                        score += 1.0;
                        matched_fields.push("algorithm".to_string());
                        text_matched = true;
                    } else if algo_lc.contains(q) {
                        score += 0.5;
                        matched_fields.push("algorithm".to_string());
                        text_matched = true;
                    }

                    // key_type
                    let kt_lc = format!("{:?}", metadata.key_type).to_lowercase();
                    if kt_lc == *q {
                        score += 1.0;
                        matched_fields.push("key_type".to_string());
                        text_matched = true;
                    } else if kt_lc.contains(q) {
                        score += 0.5;
                        matched_fields.push("key_type".to_string());
                        text_matched = true;
                    }

                    // security_level
                    let sl_lc = format!("{:?}", metadata.security_level).to_lowercase();
                    if sl_lc == *q {
                        score += 1.0;
                        matched_fields.push("security_level".to_string());
                        text_matched = true;
                    } else if sl_lc.contains(q) {
                        score += 0.5;
                        matched_fields.push("security_level".to_string());
                        text_matched = true;
                    }

                    // tags
                    if let Some(tags) = self.key_tags.get(key_id) {
                        for tag in tags {
                            let tag_lc = tag.to_lowercase();
                            if tag_lc == *q {
                                score += 1.0;
                                matched_fields.push(format!("tag:{}", tag));
                                text_matched = true;
                            } else if tag_lc.contains(q) {
                                score += 0.5;
                                matched_fields.push(format!("tag:{}", tag));
                                text_matched = true;
                            }
                        }
                    }

                    if !text_matched {
                        return None;
                    }
                }

                // --- Algorithm filter (exact match) ---
                if let Some(ref algo) = query.algorithm {
                    if metadata.key_algorithm != *algo {
                        return None;
                    }
                    score += 1.0;
                    matched_fields.push("algorithm".to_string());
                }

                // --- Purpose filter (exact match) ---
                if let Some(ref purpose) = query.purpose {
                    match self.key_purposes.get(key_id) {
                        Some(p) if p == purpose => {
                            score += 1.0;
                            matched_fields.push("purpose".to_string());
                        }
                        _ => return None,
                    }
                }

                // --- Tag filter (any tag matches, case-insensitive) ---
                if !query.tags.is_empty() {
                    let tags = self.key_tags.get(key_id);
                    let mut tag_matched = false;
                    if let Some(tags) = tags {
                        for query_tag in &query.tags {
                            let qt_lc = query_tag.to_lowercase();
                            if tags.iter().any(|t| t.to_lowercase() == qt_lc) {
                                score += 0.5;
                                matched_fields.push(format!("tag:{}", query_tag));
                                tag_matched = true;
                            }
                        }
                    }
                    if !tag_matched {
                        return None;
                    }
                }

                // --- Date range filter (inclusive) ---
                if let Some(after) = query.created_after {
                    if metadata.created_at < after {
                        return None;
                    }
                    matched_fields.push("created_after".to_string());
                }
                if let Some(before) = query.created_before {
                    if metadata.created_at > before {
                        return None;
                    }
                    matched_fields.push("created_before".to_string());
                }

                // A key that satisfied only filter constraints (no text) still
                // gets a baseline score so it is represented in the output.
                if score == 0.0 {
                    score = 1.0;
                }

                Some(SearchResult {
                    key_id: key_id.clone(),
                    relevance_score: score,
                    matched_fields,
                })
            })
            .collect();

        // Sort by descending relevance score, then by key_id for determinism.
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.key_id.cmp(&b.key_id))
        });
        results
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

impl KeyAccessControl {
    pub fn new() -> Self {
        Self {
            access_policies: HashMap::new(),
            authentication_methods: vec![AuthenticationMethod::MultiFactor],
            audit_log: AccessAuditLog::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    /// Register an access policy for a key.
    pub fn add_policy(&mut self, policy: AccessPolicy) {
        self.access_policies
            .insert(policy.policy_id.clone(), policy);
    }

    /// Check whether a given operation is permitted on a key.
    /// Returns true if a policy exists that explicitly allows the operation.
    /// Deny-by-default: no matching policy means denial.
    pub fn check_permission(&self, key_id: &str, operation: KeyOperation) -> bool {
        self.access_policies
            .values()
            .any(|p| p.key_id == key_id && p.allowed_operations.contains(&operation))
    }

    /// Check permission with full context (time restrictions, IP).
    pub fn check_permission_with_context(
        &self,
        key_id: &str,
        operation: KeyOperation,
        current_hour: u8,
        current_day: u8,
        ip_address: &str,
    ) -> bool {
        self.access_policies.values().any(|p| {
            if p.key_id != key_id || !p.allowed_operations.contains(&operation) {
                return false;
            }
            // Check time restrictions
            if !p.time_restrictions.allowed_hours.is_empty()
                && !p.time_restrictions.allowed_hours.contains(&current_hour)
            {
                return false;
            }
            if !p.time_restrictions.allowed_days.is_empty()
                && !p.time_restrictions.allowed_days.contains(&current_day)
            {
                return false;
            }
            // Check IP restrictions
            if !p.ip_restrictions.is_empty() && !p.ip_restrictions.iter().any(|ip| ip == ip_address)
            {
                return false;
            }
            true
        })
    }

    /// Record an access attempt in the audit log.
    pub fn log_access(
        &mut self,
        key_id: &str,
        operation: KeyOperation,
        user_id: &str,
        success: bool,
    ) {
        self.audit_log
            .log_entry(key_id, operation, user_id, success);
    }

    /// Number of registered policies.
    pub fn policy_count(&self) -> usize {
        self.access_policies.len()
    }

    /// Get a reference to the audit log.
    pub fn audit_log(&self) -> &AccessAuditLog {
        &self.audit_log
    }

    /// Get the list of configured authentication methods.
    pub fn authentication_methods(&self) -> &[AuthenticationMethod] {
        &self.authentication_methods
    }

    /// Add an authentication method if not already present.
    pub fn add_authentication_method(&mut self, method: AuthenticationMethod) {
        if !self.authentication_methods.contains(&method) {
            self.authentication_methods.push(method);
        }
    }

    /// Check whether a given authentication method is supported.
    pub fn supports_authentication_method(&self, method: &AuthenticationMethod) -> bool {
        self.authentication_methods.contains(method)
    }
}

impl AccessAuditLog {
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

    /// Record a key access event. Called after every key read/write/sign/verify operation.
    pub fn log_entry(
        &mut self,
        key_id: &str,
        operation: KeyOperation,
        user_id: &str,
        success: bool,
    ) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let entry = AccessLogEntry {
            entry_id: format!("acc_{}_{}", timestamp, self.entries.len()),
            timestamp,
            key_id: key_id.to_string(),
            operation,
            user_id: user_id.to_string(),
            ip_address: String::new(),
            success,
            error_message: if success {
                None
            } else {
                Some("operation failed".to_string())
            },
        };
        self.entries.push(entry);
        // Enforce retention: drop entries older than retention_days
        let cutoff =
            timestamp.saturating_sub((self.retention_policy.retention_days as u64) * 86400);
        self.entries.retain(|e| e.timestamp >= cutoff);
    }

    /// Number of logged entries.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Iterate over entries (newest first).
    pub fn entries(&self) -> &[AccessLogEntry] {
        &self.entries
    }
}

impl KeyGenerator {
    pub fn new() -> Self {
        Self {
            generation_algorithms: HashMap::new(),
            entropy_sources: vec![
                EntropySource::HardwareRNG,
                EntropySource::OSRandom,
                EntropySource::Quantum,
            ],
            selected_entropy_source: None,
            quality_metrics: KeyQualityMetrics::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        for alg in [
            KeyAlgorithm::MLDSA,
            KeyAlgorithm::Kyber,
            KeyAlgorithm::NTRU,
            KeyAlgorithm::SPHINCS,
            KeyAlgorithm::RSA,
            KeyAlgorithm::ECDSA,
            KeyAlgorithm::EdDSA,
            KeyAlgorithm::AES,
            KeyAlgorithm::ChaCha20,
        ] {
            self.generation_algorithms
                .insert(alg.clone(), GenerationAlgorithm::new(alg));
        }
        Ok(())
    }

    /// Set the preferred entropy source for key generation.
    pub fn set_entropy_source(&mut self, source: EntropySource) {
        self.selected_entropy_source = Some(source);
    }

    /// Get the currently selected entropy source, if any.
    pub fn get_entropy_source(&self) -> Option<&EntropySource> {
        self.selected_entropy_source.as_ref()
    }

    /// List the available entropy sources by name.
    pub fn list_entropy_sources(&self) -> Vec<String> {
        self.entropy_sources
            .iter()
            .map(|s| format!("{:?}", s))
            .collect()
    }

    /// Generate random key data of the requested size using the selected
    /// entropy source. For `HardwareRNG`, `OSRandom`, and `Quantum` (used as a
    /// placeholder since a real quantum RNG is not available) the bytes are
    /// filled with `rand::random()`. The generated material is also folded into
    /// the quality metrics as a simple entropy estimate.
    pub fn generate_key_data(&mut self, key_size: usize) -> Result<Vec<u8>, CryptographicError> {
        let source = self
            .selected_entropy_source
            .clone()
            .unwrap_or(EntropySource::OSRandom);

        if !self.entropy_sources.contains(&source) {
            return Err(CryptographicError::SecurityError(format!(
                "selected entropy source {:?} is not available",
                source
            )));
        }

        let mut data = vec![0u8; key_size];
        match source {
            EntropySource::HardwareRNG | EntropySource::OSRandom | EntropySource::Quantum => {
                // rand::random() draws from the OS CSPRNG; for HardwareRNG and
                // Quantum this is a placeholder until dedicated hardware is wired.
                for byte in data.iter_mut() {
                    *byte = rand::random();
                }
            }
            _ => {
                return Err(CryptographicError::SecurityError(format!(
                    "entropy source {:?} not yet supported for raw key generation",
                    source
                )));
            }
        }

        // Update quality metrics with a simple entropy estimate (8 bits/byte ideal).
        self.quality_metrics.entropy_score = if key_size > 0 { 8.0 } else { 0.0 };

        Ok(data)
    }

    pub fn generate_key(
        &mut self,
        key_id: String,
        key_type: KeyType,
        algorithm: KeyAlgorithm,
        security_level: SecurityLevel,
    ) -> Result<Key, CryptographicError> {
        let generation_algorithm = self.generation_algorithms.get(&algorithm).ok_or_else(|| {
            CryptographicError::UnsupportedAlgorithm("Algorithm not supported".to_string())
        })?;

        // Generate key data
        let key_data =
            self.generate_algorithm_key_data(&generation_algorithm, security_level.clone())?;

        // Create metadata
        let metadata = KeyMetadata {
            key_id: key_id.clone(),
            key_type: key_type.clone(),
            key_algorithm: algorithm.clone(),
            key_size: key_data.len(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            expires_at: 0,
            last_used: 0,
            usage_count: 0,
            security_level: security_level.clone(),
            access_level: AccessLevel::Secret,
        };

        Ok(Key {
            key_id,
            key_type,
            key_algorithm: algorithm,
            key_data,
            metadata,
        })
    }

    pub fn derive_public_key(
        &mut self,
        private_key: &Key,
        public_key_id: String,
    ) -> Result<Key, CryptographicError> {
        // Derive public key from private key
        let public_key_data = self.derive_public_key_data(&private_key)?;

        let metadata = KeyMetadata {
            key_id: public_key_id.clone(),
            key_type: KeyType::Public,
            key_algorithm: private_key.key_algorithm.clone(),
            key_size: public_key_data.len(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            expires_at: 0,
            last_used: 0,
            usage_count: 0,
            security_level: private_key.metadata.security_level.clone(),
            access_level: AccessLevel::Public,
        };

        Ok(Key {
            key_id: public_key_id,
            key_type: KeyType::Public,
            key_algorithm: private_key.key_algorithm.clone(),
            key_data: public_key_data,
            metadata,
        })
    }

    fn generate_algorithm_key_data(
        &self,
        algorithm: &GenerationAlgorithm,
        _security_level: SecurityLevel,
    ) -> Result<Vec<u8>, CryptographicError> {
        match &algorithm.algorithm {
            KeyAlgorithm::MLDSA => {
                let (priv_k, _pub_k) = MlDsaSigner::generate_keypair().map_err(|e| {
                    CryptographicError::SignatureError(format!("ML-DSA keygen failed: {e}"))
                })?;
                Ok(priv_k.sk_bytes)
            }
            KeyAlgorithm::Kyber => {
                use fips203::ml_kem_768;
                use fips203::traits::{KeyGen, SerDes};
                let (_ek, dk) = ml_kem_768::KG::try_keygen()
                    .map_err(|e| CryptographicError::SignatureError(e.to_string()))?;
                Ok(dk.into_bytes().to_vec())
            }
            KeyAlgorithm::NTRU => {
                use fips203::ml_kem_512;
                use fips203::traits::{KeyGen, SerDes};
                let (_ek, dk) = ml_kem_512::KG::try_keygen()
                    .map_err(|e| CryptographicError::SignatureError(e.to_string()))?;
                Ok(dk.into_bytes().to_vec())
            }
            KeyAlgorithm::SPHINCS => {
                use fips205::slh_dsa_sha2_256s;
                use fips205::traits::{KeyGen, SerDes};
                let (_pk, sk) = slh_dsa_sha2_256s::KG::try_keygen()
                    .map_err(|e| CryptographicError::SignatureError(e.to_string()))?;
                Ok(sk.into_bytes().to_vec())
            }
            KeyAlgorithm::RSA => {
                #[cfg(feature = "interop-crypto")]
                {
                    use rsa::{pkcs8::EncodePrivateKey, rand_core::OsRng, RsaPrivateKey};
                    let mut rng = OsRng;
                    let priv_key = RsaPrivateKey::new(&mut rng, 2048)
                        .map_err(|e| CryptographicError::SignatureError(e.to_string()))?;
                    Ok(priv_key
                        .to_pkcs8_der()
                        .map_err(|e| CryptographicError::SignatureError(e.to_string()))?
                        .as_bytes()
                        .to_vec())
                }
                #[cfg(not(feature = "interop-crypto"))]
                {
                    Err(CryptographicError::UnsupportedAlgorithm(
                        "RSA requires interop-crypto feature".to_string(),
                    ))
                }
            }
            KeyAlgorithm::ECDSA => {
                #[cfg(feature = "interop-crypto")]
                {
                    use crate::fiduciary_crypto::InteropEcdsaSigner;
                    let signer = InteropEcdsaSigner::generate()
                        .map_err(|e| CryptographicError::SignatureError(e.to_string()))?;
                    signer
                        .export_secret_key()
                        .map_err(|e| CryptographicError::SignatureError(e.to_string()))
                }
                #[cfg(not(feature = "interop-crypto"))]
                {
                    Err(CryptographicError::UnsupportedAlgorithm(
                        "ECDSA requires interop-crypto feature".to_string(),
                    ))
                }
            }
            KeyAlgorithm::AES | KeyAlgorithm::ChaCha20 | KeyAlgorithm::EdDSA => {
                Ok(rand::random::<[u8; 32]>().to_vec())
            }
            other => Err(CryptographicError::UnsupportedAlgorithm(format!(
                "{other:?} key generation not implemented"
            ))),
        }
    }

    fn derive_public_key_data(&self, private_key: &Key) -> Result<Vec<u8>, CryptographicError> {
        match &private_key.key_algorithm {
            KeyAlgorithm::MLDSA => {
                use fips204::ml_dsa_65;
                use fips204::traits::{SerDes, Signer};
                let sk_arr: [u8; ml_dsa_65::SK_LEN] =
                    private_key.key_data.as_slice().try_into().map_err(|_| {
                        CryptographicError::InvalidKey(format!(
                            "ML-DSA secret key must be {} bytes",
                            ml_dsa_65::SK_LEN
                        ))
                    })?;
                let sk = ml_dsa_65::PrivateKey::try_from_bytes(sk_arr)
                    .map_err(|e| CryptographicError::InvalidKey(e.to_string()))?;
                Ok(sk.get_public_key().into_bytes().to_vec())
            }
            KeyAlgorithm::Kyber => {
                use fips203::ml_kem_768;
                const K: usize = 3;
                let len_dk_pke = 384 * K;
                let dk_arr: [u8; ml_kem_768::DK_LEN] =
                    private_key.key_data.as_slice().try_into().map_err(|_| {
                        CryptographicError::InvalidKey(format!(
                            "Kyber decaps key must be {} bytes",
                            ml_kem_768::DK_LEN
                        ))
                    })?;
                Ok(dk_arr[len_dk_pke..len_dk_pke + ml_kem_768::EK_LEN].to_vec())
            }
            KeyAlgorithm::NTRU => {
                use fips203::ml_kem_512;
                const K: usize = 2;
                let len_dk_pke = 384 * K;
                let dk_arr: [u8; ml_kem_512::DK_LEN] =
                    private_key.key_data.as_slice().try_into().map_err(|_| {
                        CryptographicError::InvalidKey(format!(
                            "NTRU decaps key must be {} bytes",
                            ml_kem_512::DK_LEN
                        ))
                    })?;
                Ok(dk_arr[len_dk_pke..len_dk_pke + ml_kem_512::EK_LEN].to_vec())
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
                    .map_err(|e| CryptographicError::InvalidKey(e.to_string()))?;
                Ok(sk.get_public_key().into_bytes().to_vec())
            }
            KeyAlgorithm::RSA => {
                #[cfg(feature = "interop-crypto")]
                {
                    use rsa::{
                        pkcs8::{DecodePrivateKey, EncodePublicKey},
                        RsaPrivateKey, RsaPublicKey,
                    };
                    let priv_key = RsaPrivateKey::from_pkcs8_der(&private_key.key_data)
                        .map_err(|e| CryptographicError::InvalidKey(e.to_string()))?;
                    let pub_key = RsaPublicKey::from(&priv_key);
                    Ok(pub_key
                        .to_public_key_der()
                        .map_err(|e| CryptographicError::InvalidKey(e.to_string()))?
                        .to_vec())
                }
                #[cfg(not(feature = "interop-crypto"))]
                {
                    Err(CryptographicError::UnsupportedAlgorithm(
                        "RSA requires interop-crypto feature".to_string(),
                    ))
                }
            }
            KeyAlgorithm::ECDSA => {
                #[cfg(feature = "interop-crypto")]
                {
                    use crate::fiduciary_crypto::InteropEcdsaSigner;
                    let signer = InteropEcdsaSigner::from_secret_key(&private_key.key_data)
                        .map_err(|e| CryptographicError::InvalidKey(e.to_string()))?;
                    Ok(signer
                        .public_key()
                        .ok_or_else(|| {
                            CryptographicError::InvalidKey("ECDSA public key missing".to_string())
                        })?
                        .to_vec())
                }
                #[cfg(not(feature = "interop-crypto"))]
                {
                    Err(CryptographicError::UnsupportedAlgorithm(
                        "ECDSA requires interop-crypto feature".to_string(),
                    ))
                }
            }
            KeyAlgorithm::EdDSA => {
                use ed25519_dalek::SigningKey;
                if private_key.key_data.len() < 32 {
                    return Err(CryptographicError::InvalidKey(
                        "Private key too short".to_string(),
                    ));
                }
                let mut seed = [0u8; 32];
                seed.copy_from_slice(&private_key.key_data[..32]);
                let signing_key = SigningKey::from_bytes(&seed);
                Ok(signing_key.verifying_key().to_bytes().to_vec())
            }
            other => Err(CryptographicError::UnsupportedAlgorithm(format!(
                "{other:?} public key derivation not supported"
            ))),
        }
    }
}

impl GenerationAlgorithm {
    pub fn new(algorithm: KeyAlgorithm) -> Self {
        Self {
            algorithm_id: format!("gen_{}", format!("{:?}", algorithm).to_lowercase()),
            algorithm,
            parameters: GenerationParameters {
                key_size: 256,
                curve: None,
                hash_function: None,
                custom_params: HashMap::new(),
            },
            security_level: SecurityLevel::High,
        }
    }
}

impl KeyQualityMetrics {
    pub fn new() -> Self {
        Self {
            entropy_score: 0.0,
            randomness_test_results: Vec::new(),
            security_assessment: SecurityAssessment {
                vulnerability_score: 0.0,
                compliance_score: 1.0,
                recommendations: Vec::new(),
            },
        }
    }
}

impl SecurityAssessment {
    pub fn new() -> Self {
        Self {
            vulnerability_score: 0.0,
            compliance_score: 1.0,
            recommendations: Vec::new(),
        }
    }
}

impl KeyRotator {
    pub fn new() -> Self {
        Self {
            rotation_policies: HashMap::new(),
            rotation_schedule: RotationSchedule::new(),
            rotation_history: RotationHistory::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        // Initialize rotation policies
        self.rotation_policies.insert(
            KeyAlgorithm::MLDSA,
            RotationPolicy::new(KeyAlgorithm::MLDSA),
        );
        self.rotation_policies
            .insert(KeyAlgorithm::AES, RotationPolicy::new(KeyAlgorithm::AES));
        Ok(())
    }

    pub fn rotate_key(&mut self, old_key: &Key) -> Result<Key, CryptographicError> {
        let new_key_id = format!(
            "{}_rotated_{}",
            old_key.key_id,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        let mut new_key_data = rand::random::<[u8; 32]>().to_vec();
        new_key_data.resize(old_key.key_data.len(), 0);
        let new_key = Key {
            key_id: new_key_id.clone(),
            key_type: old_key.key_type.clone(),
            key_algorithm: old_key.key_algorithm.clone(),
            key_data: new_key_data,
            metadata: KeyMetadata {
                key_id: new_key_id,
                key_type: old_key.key_type.clone(),
                key_algorithm: old_key.key_algorithm.clone(),
                key_size: old_key.metadata.key_size,
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                expires_at: 0,
                last_used: 0,
                usage_count: 0,
                security_level: old_key.metadata.security_level.clone(),
                access_level: old_key.metadata.access_level.clone(),
            },
        };
        Ok(new_key)
    }

    /// Get a reference to the rotation schedule.
    pub fn rotation_schedule(&self) -> &RotationSchedule {
        &self.rotation_schedule
    }

    /// Get a mutable reference to the rotation schedule.
    pub fn rotation_schedule_mut(&mut self) -> &mut RotationSchedule {
        &mut self.rotation_schedule
    }

    /// Get a reference to the rotation history.
    pub fn rotation_history(&self) -> &RotationHistory {
        &self.rotation_history
    }

    /// Get a mutable reference to the rotation history.
    pub fn rotation_history_mut(&mut self) -> &mut RotationHistory {
        &mut self.rotation_history
    }
}

impl RotationPolicy {
    pub fn new(algorithm: KeyAlgorithm) -> Self {
        Self {
            policy_id: format!("rotation_policy_{:?}", algorithm),
            algorithm,
            rotation_interval: 86400 * 90, // 90 days
            grace_period: 86400 * 7,       // 7 days
            automatic_rotation: true,
            notification_settings: NotificationSettings {
                notify_before_rotation: true,
                notification_days: 7,
                notification_channels: vec![NotificationChannel::Email],
            },
        }
    }
}

impl NotificationSettings {
    pub fn new() -> Self {
        Self {
            notify_before_rotation: true,
            notification_days: 7,
            notification_channels: vec![NotificationChannel::Email],
        }
    }
}

impl RotationSchedule {
    pub fn new() -> Self {
        Self {
            scheduled_rotations: Vec::new(),
            rotation_queue: Vec::new(),
            completed_rotations: Vec::new(),
        }
    }
}

impl RotationHistory {
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

    /// Record a rotation in the history, enforcing retention.
    pub fn add_entry(&mut self, entry: RotationHistoryEntry) {
        let cutoff = entry
            .timestamp
            .saturating_sub((self.retention_policy.retention_days as u64) * 86400);
        self.entries.retain(|e| e.timestamp >= cutoff);
        self.entries.push(entry);
    }

    /// Number of recorded rotation history entries.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Iterate over history entries.
    pub fn entries(&self) -> &[RotationHistoryEntry] {
        &self.entries
    }

    /// Get the retention policy for rotation history.
    pub fn retention_policy(&self) -> &RetentionPolicy {
        &self.retention_policy
    }
}

impl KeyRecovery {
    pub fn new() -> Self {
        Self {
            recovery_methods: vec![
                RecoveryMethod::ShamirSecretSharing,
                RecoveryMethod::EncryptedBackup,
            ],
            recovery_policies: RecoveryPolicies {
                minimum_shares: 3,
                total_shares: 5,
                recovery_threshold: 0.6,
                time_lock: 86400, // 24 hours
                geo_restrictions: Vec::new(),
            },
            recovery_attempts: RecoveryAttempts {
                attempts: Vec::new(),
                lockout_policy: LockoutPolicy {
                    max_attempts: 3,
                    lockout_duration: 3600, // 1 hour
                    exponential_backoff: true,
                },
            },
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    /// Get the list of configured recovery methods.
    pub fn recovery_methods(&self) -> &[RecoveryMethod] {
        &self.recovery_methods
    }

    /// Add a recovery method if not already present.
    pub fn add_recovery_method(&mut self, method: RecoveryMethod) {
        if !self.recovery_methods.contains(&method) {
            self.recovery_methods.push(method);
        }
    }

    /// Get the recovery policies.
    pub fn recovery_policies(&self) -> &RecoveryPolicies {
        &self.recovery_policies
    }

    /// Get a mutable reference to the recovery policies.
    pub fn recovery_policies_mut(&mut self) -> &mut RecoveryPolicies {
        &mut self.recovery_policies
    }

    /// Get a reference to the recovery attempts.
    pub fn recovery_attempts(&self) -> &RecoveryAttempts {
        &self.recovery_attempts
    }

    /// Get a mutable reference to the recovery attempts.
    pub fn recovery_attempts_mut(&mut self) -> &mut RecoveryAttempts {
        &mut self.recovery_attempts
    }
}

impl RecoveryAttempts {
    pub fn new() -> Self {
        Self {
            attempts: Vec::new(),
            lockout_policy: LockoutPolicy {
                max_attempts: 3,
                lockout_duration: 3600,
                exponential_backoff: true,
            },
        }
    }
}

impl LockoutPolicy {
    pub fn new() -> Self {
        Self {
            max_attempts: 3,
            lockout_duration: 3600,
            exponential_backoff: true,
        }
    }
}
