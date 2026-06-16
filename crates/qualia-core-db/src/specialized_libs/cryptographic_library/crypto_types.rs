//! Cryptographic Type Definitions
//!
//! This module contains all type definitions for the cryptographic library.
//! These are zero-heap compatible types used throughout the library.

use serde::{Deserialize, Serialize};

/// Compact identifier view for key catalogs in zero-heap call paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyHandle {
    pub key_id_hash: u64,
    pub key_type_tag: u8,
    pub algorithm_tag: u8,
    pub security_level_tag: u8,
}

/// Layout metadata for caller-owned encryption buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InlineEncryptionLayout {
    pub ciphertext_len: usize,
    pub iv_len: usize,
    pub tag_len: usize,
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// Encryption algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    AES256GCM,
    ChaCha20Poly1305,
    XChaCha20Poly1305,
    Custom(String),
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
    PCI_DSS,
    Custom(String),
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

/// Recommendation severity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecommendationSeverity {
    Low,
    Medium,
    High,
    Critical,
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

/// Rotation types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RotationType {
    Automatic,
    Manual,
    Emergency,
    Compliance,
    Compromise,
}

/// Rotation priority
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RotationPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Recovery methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecoveryMethod {
    ShamirSecretSharing,
    KeyEscrow,
    BiometricRecovery,
    TrustedParty,
    HardwareToken,
    MultiFactor,
}

/// Encryption modes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EncryptionMode {
    ECB,
    CBC,
    CTR,
    GCM,
    CCM,
    XTS,
    Custom(String),
}

/// Encryption padding
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EncryptionPadding {
    PKCS7,
    ISO9797_1,
    ISO10126,
    ANSIX923,
    ZeroPadding,
    NoPadding,
}

/// Derivation functions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DerivationFunction {
    HKDF_SHA256,
    HKDF_SHA512,
    PBKDF2,
    Argon2,
    Scrypt,
    Custom(String),
}

/// Hash operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HashOperation {
    Compute,
    Verify,
    Update,
    Delete,
}

/// Hash optimization strategies
#[derive(Debug, Clone, PartialEq)]
pub enum HashOptimizationStrategy {
    BatchHashing,
    ParallelProcessing,
    HardwareAcceleration,
    Caching,
    MemoryOptimization,
}

/// Proof system types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProofSystemType {
    zkSNARKs,
    zkSTARKs,
    Bulletproofs,
    SigmaProtocols,
    Custom(String),
}

/// Circuit types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CircuitType {
    Arithmetic,
    Boolean,
    Hash,
    Signature,
    Custom(String),
}

/// Constraint types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintType {
    Equality,
    Inequality,
    Range,
    Boolean,
    Custom(String),
}

/// Expression types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExpressionType {
    Variable,
    Constant,
    Addition,
    Subtraction,
    Multiplication,
    Division,
    Custom(String),
}

/// Variable types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VariableType {
    Public,
    Private,
    Constant,
    Custom(String),
}

/// Prover types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProverType {
    Groth16,
    PLONK,
    Sonic,
    Marlin,
    Custom(String),
}

/// Verifier types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VerifierType {
    Groth16,
    PLONK,
    Sonic,
    Marlin,
    Custom(String),
}

/// Proof storage types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProofStorageType {
    Local,
    Distributed,
    Encrypted,
    Custom(String),
}

/// Verification priorities
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VerificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Verification optimization strategies
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationOptimizationStrategy {
    BatchVerification,
    ParallelProcessing,
    Caching,
    HardwareAcceleration,
}

/// Proof optimization strategies
#[derive(Debug, Clone, PartialEq)]
pub enum ProofOptimizationStrategy {
    ParallelProving,
    CircuitOptimization,
    Precomputation,
    HardwareAcceleration,
}

/// Threat types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ThreatType {
    MaliciousKey,
    CompromisedCertificate,
    WeakAlgorithm,
    SideChannelAttack,
    TimingAttack,
    Custom(String),
}

/// Threat severity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ThreatSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Anomaly types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnomalyType {
    Statistical,
    Behavioral,
    Network,
    Resource,
    Custom(String),
}

/// Anomaly severity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Compliance frameworks
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComplianceFramework {
    HIPAA,
    GDPR,
    SOX,
    PCI_DSS,
    FIPS140_2,
    FIPS140_3,
    ISO27001,
    Custom(String),
}

/// Event types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventType {
    KeyOperation,
    SignatureOperation,
    EncryptionOperation,
    ProofOperation,
    SecurityEvent,
    ComplianceEvent,
}

/// Audit results
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuditResult {
    Success,
    Failure,
    Warning,
    Error,
}

/// Content generators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContentGenerator {
    Static,
    Dynamic,
    Template,
    Custom(String),
}

/// Report formats
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReportFormat {
    PDF,
    HTML,
    JSON,
    XML,
    CSV,
    Custom(String),
}

/// Schedule types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScheduleType {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
    OnDemand,
    Custom(String),
}

/// Delivery channels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeliveryChannel {
    Email,
    SFTP,
    API,
    Webhook,
    Custom(String),
}

/// Backoff strategies
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Fixed,
    Linear,
    Exponential,
    Custom(String),
}

/// Delivery final status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeliveryFinalStatus {
    Delivered,
    Failed,
    Pending,
    Cancelled,
}

/// Compliance status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComplianceStatus {
    Compliant,
    NonCompliant,
    PartiallyCompliant,
    Unknown,
}

/// Signature operation types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SignatureOperation {
    Sign,
    Verify,
    BatchSign,
    BatchVerify,
}

/// Signature optimization strategies
#[derive(Debug, Clone, PartialEq)]
pub enum SignatureOptimizationStrategy {
    Precomputation,
    Caching,
    ParallelProcessing,
    HardwareAcceleration,
}

/// Encryption optimization strategies
#[derive(Debug, Clone, PartialEq)]
pub enum EncryptionOptimizationStrategy {
    BatchEncryption,
    ParallelProcessing,
    HardwareAcceleration,
    MemoryOptimization,
}

/// Anomaly detection algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnomalyDetectionAlgorithm {
    Statistical,
    MachineLearning,
    DeepLearning,
    Ensemble,
    Custom(String),
}

/// Model types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelType {
    Statistical,
    NeuralNetwork,
    DecisionTree,
    Custom(String),
}

/// Control types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ControlType {
    Preventive,
    Detective,
    Corrective,
    Compensating,
}

/// Implementation status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImplementationStatus {
    Implemented,
    PartiallyImplemented,
    NotImplemented,
    NotApplicable,
}

/// Detection rule types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DetectionRuleType {
    Signature,
    Heuristic,
    Behavioral,
    Statistical,
    Custom(String),
}

/// Comparison operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComparisonOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    Contains,
    Matches,
}

/// Detection action types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DetectionActionType {
    Alert,
    Block,
    Quarantine,
    Log,
    Custom(String),
}

/// Security alert types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SecurityAlertType {
    Threat,
    Anomaly,
    Compliance,
    System,
    Custom(String),
}

/// Alert severity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Alert channels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertChannel {
    Email,
    SMS,
    Webhook,
    Slack,
    PagerDuty,
    Custom(String),
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
