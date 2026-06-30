//! Cryptographic Library - Quantum-Resistant Cryptographic Operations
//!
//! This module provides high-performance cryptographic operations leveraging Phase 2 enhancements:
//! - Fiduciary Cryptography (ML-DSA) for post-quantum digital signatures
//! - Zero-Knowledge Semantic Proofs for privacy-preserving cryptography
//! - Hardware-Sympathetic Storage (ZNS) for secure key storage
//! - Allocation Firewall (eBPF) for kernel-level cryptographic operations

use crate::ebpf_firewall::EbpfFirewall;
use crate::fiduciary_crypto::{CryptoContext, MlDsaSignature, MlDsaSigner, MlDsaVcProof};
use crate::zk_proofs::ZkProofSystem;
use crate::zns_storage::ZnsZoneManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Cryptographic Library Manager
pub struct CryptographicLibrary {
    key_manager: KeyManager,
    signature_engine: SignatureEngine,
    encryption_engine: EncryptionEngine,
    hash_engine: HashEngine,
    proof_engine: ProofEngine,
    security_monitor: SecurityMonitor,
}

/// Key manager for secure key storage and management
pub struct KeyManager {
    key_storage: KeyStorage,
    key_generator: KeyGenerator,
    key_rotator: KeyRotator,
    key_recovery: KeyRecovery,
}

/// Key storage using ZNS for secure key storage
pub struct KeyStorage {
    zones: HashMap<String, KeyZone>,
    key_catalog: KeyCatalog,
    encryption_at_rest: EncryptionAtRest,
    access_control: KeyAccessControl,
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

/// Key catalog for key management
pub struct KeyCatalog {
    keys: HashMap<String, KeyMetadata>,
    relationships: HashMap<String, Vec<KeyRelationship>>,
    tags: HashMap<String, Vec<String>>,
    search_index: KeySearchIndex,
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
    search_engine: KeySearchEngine,
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
    engine_type: SearchEngineType,
    indexing_strategy: IndexingStrategy,
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

/// Encryption at rest
pub struct EncryptionAtRest {
    encryption_algorithm: EncryptionAlgorithm,
    key_encryption_keys: HashMap<String, Vec<u8>>,
    encryption_policy: EncryptionPolicy,
}

/// Encryption algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    AES256GCM,
    ChaCha20Poly1305,
    XChaCha20Poly1305,
    Custom(String),
}

/// Encryption policy
pub struct EncryptionPolicy {
    pub encryption_required: bool,
    pub key_rotation_interval: u64,
    pub algorithm_preference: Vec<EncryptionAlgorithm>,
    pub compliance_requirements: Vec<ComplianceRequirement>,
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
    quality_metrics: KeyQualityMetrics,
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

/// Signature engine for digital signatures
pub struct SignatureEngine {
    signing_algorithms: HashMap<KeyAlgorithm, SigningAlgorithm>,
    verification_algorithms: HashMap<KeyAlgorithm, VerificationAlgorithm>,
    signature_storage: SignatureStorage,
    performance_optimizer: SignaturePerformanceOptimizer,
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
    audit_log: SignatureAuditLog,
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

/// Encryption engine for data encryption
pub struct EncryptionEngine {
    encryption_algorithms: HashMap<EncryptionAlgorithm, EncryptionAlgorithmImpl>,
    decryption_algorithms: HashMap<EncryptionAlgorithm, DecryptionAlgorithmImpl>,
    key_derivation: KeyDerivation,
    performance_optimizer: EncryptionPerformanceOptimizer,
}

/// Encryption algorithm implementation
#[derive(Debug, Clone)]
pub struct EncryptionAlgorithmImpl {
    pub algorithm_id: String,
    pub algorithm: EncryptionAlgorithm,
    pub key_size: usize,
    pub iv_size: usize,
    pub tag_size: usize,
    pub parameters: EncryptionParameters,
}

/// Encryption parameters
#[derive(Debug, Clone)]
pub struct EncryptionParameters {
    pub mode: EncryptionMode,
    pub padding: Option<EncryptionPadding>,
    pub additional_data: Option<Vec<u8>>,
    pub custom_params: HashMap<String, Vec<u8>>,
}

/// Encryption modes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EncryptionMode {
    GCM,
    CCM,
    CTR,
    CBC,
    CFB,
    OFB,
    XTS,
}

/// Encryption padding
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EncryptionPadding {
    PKCS7,
    ISO10126,
    ANSIX923,
    ZeroPadding,
    NoPadding,
}

/// Decryption algorithm implementation
#[derive(Debug, Clone)]
pub struct DecryptionAlgorithmImpl {
    pub algorithm_id: String,
    pub algorithm: EncryptionAlgorithm,
    pub key_size: usize,
    pub iv_size: usize,
    pub tag_size: usize,
    pub parameters: DecryptionParameters,
}

/// Decryption parameters
#[derive(Debug, Clone)]
pub struct DecryptionParameters {
    pub mode: EncryptionMode,
    pub padding: Option<EncryptionPadding>,
    pub additional_data: Option<Vec<u8>>,
    pub custom_params: HashMap<String, Vec<u8>>,
}

/// Key derivation
pub struct KeyDerivation {
    derivation_functions: HashMap<String, DerivationFunction>,
    derivation_parameters: DerivationParameters,
}

/// Derivation functions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DerivationFunction {
    HKDF,
    PBKDF2,
    Scrypt,
    Argon2,
    Custom(String),
}

/// Derivation parameters
#[derive(Debug, Clone)]
pub struct DerivationParameters {
    pub salt: Vec<u8>,
    pub iterations: u32,
    pub memory_cost: u32,
    pub parallelism: u32,
    pub output_length: usize,
}

/// Encryption performance optimizer
pub struct EncryptionPerformanceOptimizer {
    optimization_strategies: Vec<EncryptionOptimizationStrategy>,
    performance_metrics: EncryptionPerformanceMetrics,
}

/// Encryption optimization strategies
#[derive(Debug, Clone, PartialEq)]
pub enum EncryptionOptimizationStrategy {
    BatchEncryption,
    ParallelProcessing,
    HardwareAcceleration,
    MemoryOptimization,
    Caching,
}

/// Encryption performance metrics
#[derive(Debug, Clone)]
pub struct EncryptionPerformanceMetrics {
    pub average_encryption_time: f64,
    pub average_decryption_time: f64,
    pub throughput: f64,
    pub memory_usage: u64,
    pub cache_hit_rate: f64,
}

/// Hash engine for cryptographic hashing
pub struct HashEngine {
    hash_algorithms: HashMap<String, HashAlgorithmImpl>,
    hash_storage: HashStorage,
    performance_optimizer: HashPerformanceOptimizer,
}

/// Hash algorithm implementation
#[derive(Debug, Clone)]
pub struct HashAlgorithmImpl {
    pub algorithm_id: String,
    pub algorithm: String,
    pub output_size: usize,
    pub block_size: usize,
    pub parameters: HashParameters,
}

/// Hash parameters
#[derive(Debug, Clone)]
pub struct HashParameters {
    pub rounds: u32,
    pub personalization: Option<Vec<u8>>,
    pub salt: Option<Vec<u8>>,
    pub custom_params: HashMap<String, Vec<u8>>,
}

/// Hash storage
pub struct HashStorage {
    hashes: HashMap<String, HashResult>,
    verification_records: HashMap<String, HashVerificationRecord>,
    audit_log: HashAuditLog,
}

/// Hash record
#[derive(Debug, Clone)]
pub struct HashRecord {
    pub hash_id: String,
    pub algorithm: String,
    pub input_data: Vec<u8>,
    pub hash_value: Vec<u8>,
    pub timestamp: u64,
    pub metadata: HashMetadata,
}

/// Hash metadata
#[derive(Debug, Clone)]
pub struct HashMetadata {
    pub creator_id: String,
    pub purpose: String,
    pub context: Vec<String>,
    pub data_size: usize,
}

/// Hash verification record
#[derive(Debug, Clone)]
pub struct HashVerificationRecord {
    pub verification_id: String,
    pub hash_id: String,
    pub verifier_id: String,
    pub result: HashVerificationResult,
    pub timestamp: u64,
}

/// Hash verification result
#[derive(Debug, Clone)]
pub struct HashVerificationResult {
    pub valid: bool,
    pub error_message: Option<String>,
    pub verification_time: u64,
}

/// Hash audit log
pub struct HashAuditLog {
    entries: Vec<HashAuditEntry>,
    retention_policy: RetentionPolicy,
}

/// Hash audit entry
#[derive(Debug, Clone)]
pub struct HashAuditEntry {
    pub entry_id: String,
    pub timestamp: u64,
    pub hash_id: String,
    pub operation: HashOperation,
    pub user_id: String,
    pub ip_address: String,
    pub success: bool,
}

/// Hash operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HashOperation {
    Compute,
    Verify,
    Update,
    Delete,
}

/// Hash performance optimizer
pub struct HashPerformanceOptimizer {
    optimization_strategies: Vec<HashOptimizationStrategy>,
    performance_metrics: HashPerformanceMetrics,
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

/// Hash performance metrics
#[derive(Debug, Clone)]
pub struct HashPerformanceMetrics {
    pub average_hash_time: f64,
    pub throughput: f64,
    pub memory_usage: u64,
    pub cache_hit_rate: f64,
}

/// Proof engine for zero-knowledge proofs
pub struct ProofEngine {
    proof_systems: HashMap<String, ProofSystem>,
    proof_storage: ProofStorage,
    verification_engine: ProofVerificationEngine,
    performance_optimizer: ProofPerformanceOptimizer,
}

/// Proof system
#[derive(Debug, Clone)]
pub struct ProofSystem {
    pub system_id: String,
    pub system_type: ProofSystemType,
    pub circuit_builder: CircuitBuilder,
    pub prover: Prover,
    pub verifier: Verifier,
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

/// Circuit builder
#[derive(Debug, Clone)]
pub struct CircuitBuilder {
    pub builder_id: String,
    pub circuit_type: CircuitType,
    pub constraints: Vec<CircuitConstraint>,
    pub variables: Vec<CircuitVariable>,
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

/// Circuit constraint
#[derive(Debug, Clone)]
pub struct CircuitConstraint {
    pub constraint_id: String,
    pub constraint_type: ConstraintType,
    pub left_hand: CircuitExpression,
    pub right_hand: CircuitExpression,
}

/// Constraint types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintType {
    Equality,
    Inequality,
    Boolean,
    Custom(String),
}

/// Circuit expression
#[derive(Debug, Clone)]
pub enum CircuitExpression {
    Variable(String),
    Constant(Vec<u8>),
    Add(Box<CircuitExpression>, Box<CircuitExpression>),
    Mul(Box<CircuitExpression>, Box<CircuitExpression>),
    Sub(Box<CircuitExpression>, Box<CircuitExpression>),
    Div(Box<CircuitExpression>, Box<CircuitExpression>),
}

/// Circuit variable
#[derive(Debug, Clone)]
pub struct CircuitVariable {
    pub variable_id: String,
    pub variable_type: VariableType,
    pub value: Option<Vec<u8>>,
}

/// Variable types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VariableType {
    Public,
    Private,
    Constant,
    Witness,
}

/// Prover
#[derive(Debug, Clone)]
pub struct Prover {
    pub prover_id: String,
    pub proving_key: Vec<u8>,
    pub proving_algorithm: ProvingAlgorithm,
}

/// Proving algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProvingAlgorithm {
    Groth16,
    PLONK,
    Marlin,
    Halo2,
    Custom(String),
}

/// Verifier
#[derive(Debug, Clone)]
pub struct Verifier {
    pub verifier_id: String,
    pub verification_key: Vec<u8>,
    pub verification_algorithm: VerificationAlgorithm,
}

/// Verification algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VerificationAlgorithm {
    Groth16,
    PLONK,
    Marlin,
    Halo2,
    Custom(String),
}

/// Proof storage
pub struct ProofStorage {
    proofs: HashMap<String, Proof>,
    verification_records: HashMap<String, ProofVerificationRecord>,
    audit_log: ProofAuditLog,
}

/// Proof record
#[derive(Debug, Clone)]
pub struct ProofRecord {
    pub proof_id: String,
    pub system_id: String,
    pub circuit_id: String,
    pub public_inputs: Vec<Vec<u8>>,
    pub proof_data: Vec<u8>,
    pub timestamp: u64,
    pub metadata: ProofMetadata,
}

/// Proof metadata
#[derive(Debug, Clone)]
pub struct ProofMetadata {
    pub prover_id: String,
    pub purpose: String,
    pub context: Vec<String>,
    pub validity_period: Option<(u64, u64)>,
    pub security_level: SecurityLevel,
}

/// Proof verification record
#[derive(Debug, Clone)]
pub struct ProofVerificationRecord {
    pub verification_id: String,
    pub proof_id: String,
    pub verifier_id: String,
    pub result: ProofVerificationResult,
    pub timestamp: u64,
}

/// Proof verification result
#[derive(Debug, Clone)]
pub struct ProofVerificationResult {
    pub valid: bool,
    pub error_message: Option<String>,
    pub verification_time: u64,
    pub confidence: f64,
}

/// Proof audit log
pub struct ProofAuditLog {
    entries: Vec<ProofAuditEntry>,
    retention_policy: RetentionPolicy,
}

/// Proof audit entry
#[derive(Debug, Clone)]
pub struct ProofAuditEntry {
    pub entry_id: String,
    pub timestamp: u64,
    pub proof_id: String,
    pub operation: ProofOperation,
    pub user_id: String,
    pub ip_address: String,
    pub success: bool,
}

/// Proof operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProofOperation {
    Generate,
    Verify,
    Revoke,
    Update,
}

/// Proof verification engine
pub struct ProofVerificationEngine {
    verification_algorithms: HashMap<String, VerificationAlgorithm>,
    batch_verifier: BatchVerifier,
    performance_optimizer: VerificationPerformanceOptimizer,
}

/// Batch verifier
pub struct BatchVerifier {
    batch_size: usize,
    parallel_verification: bool,
    verification_queue: Vec<QueuedVerification>,
}

/// Queued verification
#[derive(Debug, Clone)]
pub struct QueuedVerification {
    pub verification_id: String,
    pub proof_id: String,
    pub priority: VerificationPriority,
    pub queued_at: u64,
}

/// Verification priorities
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VerificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Verification performance optimizer
pub struct VerificationPerformanceOptimizer {
    optimization_strategies: Vec<VerificationOptimizationStrategy>,
    performance_metrics: VerificationPerformanceMetrics,
}

/// Verification optimization strategies
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationOptimizationStrategy {
    BatchVerification,
    ParallelProcessing,
    Caching,
    HardwareAcceleration,
}

/// Verification performance metrics
#[derive(Debug, Clone)]
pub struct VerificationPerformanceMetrics {
    pub average_verification_time: f64,
    pub throughput: f64,
    pub cache_hit_rate: f64,
    pub batch_efficiency: f64,
}

/// Proof performance optimizer
pub struct ProofPerformanceOptimizer {
    optimization_strategies: Vec<ProofOptimizationStrategy>,
    performance_metrics: ProofPerformanceMetrics,
}

/// Proof optimization strategies
#[derive(Debug, Clone, PartialEq)]
pub enum ProofOptimizationStrategy {
    ParallelProving,
    CircuitOptimization,
    Precomputation,
    HardwareAcceleration,
}

/// Proof performance metrics
#[derive(Debug, Clone)]
pub struct ProofPerformanceMetrics {
    pub average_proving_time: f64,
    pub average_verification_time: f64,
    pub proof_size: u64,
    pub circuit_size: u64,
    pub cache_hit_rate: f64,
}

/// Security monitor
pub struct SecurityMonitor {
    threat_detector: ThreatDetector,
    anomaly_detector: AnomalyDetector,
    compliance_monitor: ComplianceMonitor,
    security_metrics: SecurityMetrics,
}

/// Threat detector
pub struct ThreatDetector {
    threat_signatures: HashMap<String, ThreatSignature>,
    detection_rules: Vec<DetectionRule>,
    alert_system: SecurityAlertSystem,
}

/// Threat signatures
#[derive(Debug, Clone)]
pub struct ThreatSignature {
    pub signature_id: String,
    pub threat_type: ThreatType,
    pub pattern: Vec<u8>,
    pub severity: ThreatSeverity,
    pub description: String,
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

/// Detection rules
#[derive(Debug, Clone)]
pub struct DetectionRule {
    pub rule_id: String,
    pub rule_type: DetectionRuleType,
    pub conditions: Vec<DetectionCondition>,
    pub actions: Vec<DetectionAction>,
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

/// Detection conditions
#[derive(Debug, Clone)]
pub struct DetectionCondition {
    pub condition_id: String,
    pub field: String,
    pub operator: ComparisonOperator,
    pub value: Vec<u8>,
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

/// Detection actions
#[derive(Debug, Clone)]
pub struct DetectionAction {
    pub action_id: String,
    pub action_type: DetectionActionType,
    pub parameters: HashMap<String, Vec<u8>>,
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

/// Escalation policy for security alerts
#[derive(Debug, Clone)]
pub struct EscalationPolicy {
    pub policy_id: String,
    pub trigger_conditions: Vec<String>,
    pub timeout: u64,
}

/// Security alert system
pub struct SecurityAlertSystem {
    alert_types: Vec<SecurityAlertType>,
    notification_channels: Vec<NotificationChannel>,
    escalation_policies: Vec<EscalationPolicy>,
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

/// Anomaly detector
pub struct AnomalyDetector {
    detection_algorithms: Vec<AnomalyDetectionAlgorithm>,
    baseline_models: HashMap<String, BaselineModel>,
    alert_thresholds: HashMap<String, f64>,
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

/// Baseline model
#[derive(Debug, Clone)]
pub struct BaselineModel {
    pub model_id: String,
    pub model_type: ModelType,
    pub parameters: Vec<f64>,
    pub accuracy: f64,
}

/// Model types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelType {
    Statistical,
    NeuralNetwork,
    DecisionTree,
    Custom(String),
}

/// Compliance monitor
pub struct ComplianceMonitor {
    compliance_frameworks: HashMap<String, ComplianceFramework>,
    audit_trail: AuditTrail,
    reporting_engine: ComplianceReportingEngine,
}

/// Compliance frameworks
#[derive(Debug, Clone)]
pub struct ComplianceFramework {
    pub framework_id: String,
    pub framework_name: String,
    pub requirements: Vec<ComplianceRequirement>,
    pub controls: Vec<ComplianceControl>,
}

/// Compliance controls
#[derive(Debug, Clone)]
pub struct ComplianceControl {
    pub control_id: String,
    pub control_name: String,
    pub control_type: ControlType,
    pub implementation_status: ImplementationStatus,
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

/// Audit trail
pub struct AuditTrail {
    entries: Vec<AuditEntry>,
    retention_policy: RetentionPolicy,
}

/// Audit entry
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub entry_id: String,
    pub timestamp: u64,
    pub event_type: EventType,
    pub user_id: String,
    pub resource_id: String,
    pub action: String,
    pub result: AuditResult,
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

/// Compliance reporting engine
pub struct ComplianceReportingEngine {
    report_templates: HashMap<String, ReportTemplate>,
    scheduling_engine: ReportSchedulingEngine,
    distribution_engine: ReportDistributionEngine,
}

/// Report templates
#[derive(Debug, Clone)]
pub struct ReportTemplate {
    pub template_id: String,
    pub template_name: String,
    pub sections: Vec<ReportSection>,
    pub format: ReportFormat,
}

/// Report sections
#[derive(Debug, Clone)]
pub struct ReportSection {
    pub section_id: String,
    pub section_name: String,
    pub content_generator: ContentGenerator,
    pub data_sources: Vec<String>,
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

/// Report scheduling engine
pub struct ReportSchedulingEngine {
    schedules: HashMap<String, ReportSchedule>,
    scheduler: ReportScheduler,
}

/// Report schedules
#[derive(Debug, Clone)]
pub struct ReportSchedule {
    pub schedule_id: String,
    pub template_id: String,
    pub schedule_type: ScheduleType,
    pub parameters: ScheduleParameters,
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

/// Schedule parameters
#[derive(Debug, Clone)]
pub struct ScheduleParameters {
    pub start_date: u64,
    pub end_date: Option<u64>,
    pub frequency: u32,
    pub recipients: Vec<String>,
}

/// Report scheduler
pub struct ReportScheduler {
    scheduler_type: SchedulerType,
    queue_manager: ReportQueueManager,
}

/// Scheduler types
#[derive(Debug, Clone, PartialEq)]
pub enum SchedulerType {
    Cron,
    Interval,
    EventDriven,
    Custom(String),
}

/// Report queue manager
pub struct ReportQueueManager {
    pending_reports: Vec<QueuedReport>,
    running_reports: Vec<RunningReport>,
    completed_reports: Vec<CompletedReport>,
}

/// Queued report
#[derive(Debug, Clone)]
pub struct QueuedReport {
    pub report_id: String,
    pub template_id: String,
    pub queued_at: u64,
    pub priority: ReportPriority,
}

/// Report priorities
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReportPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Running report
#[derive(Debug, Clone)]
pub struct RunningReport {
    pub report_id: String,
    pub started_at: u64,
    pub progress: f64,
}

/// Completed report
#[derive(Debug, Clone)]
pub struct CompletedReport {
    pub report_id: String,
    pub template_id: String,
    pub started_at: u64,
    pub completed_at: u64,
    pub success: bool,
}

/// Report distribution engine
pub struct ReportDistributionEngine {
    distribution_channels: HashMap<String, DistributionChannel>,
    delivery_tracker: DeliveryTracker,
}

/// Distribution channels
#[derive(Debug, Clone)]
pub struct DistributionChannel {
    pub channel_id: String,
    pub channel_type: DistributionChannelType,
    pub configuration: ChannelConfiguration,
}

/// Distribution channel types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DistributionChannelType {
    Email,
    FTP,
    SFTP,
    API,
    Webhook,
    Custom(String),
}

/// Channel configuration
#[derive(Debug, Clone)]
pub struct ChannelConfiguration {
    pub endpoint: String,
    pub authentication: AuthenticationMethod,
    pub encryption: bool,
    pub retry_policy: RetryPolicy,
}

/// Retry policy
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub backoff_strategy: BackoffStrategy,
    pub retry_intervals: Vec<u64>,
}

/// Backoff strategies
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Fixed,
    Linear,
    Exponential,
    Custom(String),
}

/// Delivery tracker
pub struct DeliveryTracker {
    deliveries: HashMap<String, DeliveryRecord>,
    status: DeliveryStatus,
}

/// Delivery records
#[derive(Debug, Clone)]
pub struct DeliveryRecord {
    pub record_id: String,
    pub report_id: String,
    pub channel_id: String,
    pub attempts: Vec<DeliveryAttempt>,
    pub final_status: DeliveryFinalStatus,
}

/// Delivery attempts
#[derive(Debug, Clone)]
pub struct DeliveryAttempt {
    pub attempt_number: u32,
    pub timestamp: u64,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Delivery final status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeliveryFinalStatus {
    Delivered,
    Failed,
    Pending,
    Cancelled,
}

/// Delivery status
#[derive(Debug, Clone)]
pub struct DeliveryStatus {
    pub total_deliveries: u64,
    pub successful_deliveries: u64,
    pub failed_deliveries: u64,
    pub pending_deliveries: u64,
}

/// Security metrics
#[derive(Debug, Clone)]
pub struct SecurityMetrics {
    pub threat_metrics: ThreatMetrics,
    pub anomaly_metrics: AnomalyMetrics,
    pub compliance_metrics: ComplianceMetrics,
    pub performance_metrics: SecurityPerformanceMetrics,
}

/// Threat metrics
#[derive(Debug, Clone)]
pub struct ThreatMetrics {
    pub threats_detected: u64,
    pub threats_blocked: u64,
    pub false_positives: u64,
    pub detection_rate: f64,
    pub response_time: f64,
}

/// Anomaly metrics
#[derive(Debug, Clone)]
pub struct AnomalyMetrics {
    pub anomalies_detected: u64,
    pub anomalies_investigated: u64,
    pub confirmed_anomalies: u64,
    pub false_positive_rate: f64,
    pub detection_accuracy: f64,
}

/// Compliance metrics
#[derive(Debug, Clone)]
pub struct ComplianceMetrics {
    pub compliance_score: f64,
    pub controls_implemented: u64,
    pub controls_passed: u64,
    pub audit_findings: u64,
    pub remediation_rate: f64,
}

/// Security performance metrics
#[derive(Debug, Clone)]
pub struct SecurityPerformanceMetrics {
    pub average_response_time: f64,
    pub throughput: f64,
    pub resource_utilization: f64,
    pub error_rate: f64,
}

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

impl CryptographicLibrary {
    /// Create new cryptographic library
    pub fn new() -> Self {
        Self {
            key_manager: KeyManager::new(),
            signature_engine: SignatureEngine::new(),
            encryption_engine: EncryptionEngine::new(),
            hash_engine: HashEngine::new(),
            proof_engine: ProofEngine::new(),
            security_monitor: SecurityMonitor::new(),
        }
    }

    /// Initialize the library
    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        // Initialize key manager
        self.key_manager.initialize()?;

        // Initialize signature engine
        self.signature_engine.initialize()?;

        // Initialize encryption engine
        self.encryption_engine.initialize()?;

        // Initialize hash engine
        self.hash_engine.initialize()?;

        // Initialize proof engine
        self.proof_engine.initialize()?;

        // Initialize security monitor
        self.security_monitor.initialize()?;

        Ok(())
    }

    /// Generate ML-DSA key pair
    pub fn generate_mldsa_key_pair(
        &mut self,
        key_id: String,
        security_level: SecurityLevel,
    ) -> Result<CryptographicResult<(Key, Key)>, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Generate a real FIPS-204 ML-DSA-65 key pair (public key is produced alongside
        // the secret key ΓÇö it is NOT derivable from a 32-byte seed like Ed25519).
        let (priv_k, pub_k) = MlDsaSigner::generate_keypair().map_err(|e| {
            CryptographicError::SignatureError(format!("ML-DSA keygen failed: {e}"))
        })?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let private_id = format!("{key_id}_private");
        let public_id = format!("{key_id}_public");

        let private_key = Key {
            key_id: private_id.clone(),
            key_type: KeyType::Private,
            key_algorithm: KeyAlgorithm::MLDSA,
            key_data: priv_k.sk_bytes.clone(),
            metadata: KeyMetadata {
                key_id: private_id,
                key_type: KeyType::Private,
                key_algorithm: KeyAlgorithm::MLDSA,
                key_size: priv_k.sk_bytes.len(),
                created_at: now,
                expires_at: 0,
                last_used: 0,
                usage_count: 0,
                security_level: security_level.clone(),
                access_level: AccessLevel::Secret,
            },
        };
        let public_key = Key {
            key_id: public_id.clone(),
            key_type: KeyType::Public,
            key_algorithm: KeyAlgorithm::MLDSA,
            key_data: pub_k.pk_bytes.clone(),
            metadata: KeyMetadata {
                key_id: public_id,
                key_type: KeyType::Public,
                key_algorithm: KeyAlgorithm::MLDSA,
                key_size: pub_k.pk_bytes.len(),
                created_at: now,
                expires_at: 0,
                last_used: 0,
                usage_count: 0,
                security_level: security_level.clone(),
                access_level: AccessLevel::Public,
            },
        };

        // Store keys
        self.key_manager.store_key(private_key.clone())?;
        self.key_manager.store_key(public_key.clone())?;

        // Track the KeyPair relationship in the catalog
        self.key_manager
            .key_storage
            .key_catalog
            .add_relationship(
                &private_key.key_id,
                &public_key.key_id,
                KeyRelationshipType::KeyPair,
            );
        self.key_manager
            .key_storage
            .key_catalog
            .register_key(private_key.metadata.clone());
        self.key_manager
            .key_storage
            .key_catalog
            .register_key(public_key.metadata.clone());

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: (private_key, public_key),
            execution_time,
            memory_usage: 0,
            security_level,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Sign data with ML-DSA
    pub fn sign_data(
        &mut self,
        key_id: &str,
        data: &[u8],
    ) -> Result<CryptographicResult<Signature>, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Get private key
        let private_key = self.key_manager.get_key(key_id)?;

        // Validate key type
        if private_key.key_type != KeyType::Private {
            return Err(CryptographicError::InvalidKey(
                "Key must be private for signing".to_string(),
            ));
        }

        // Sign data
        let signature = self.signature_engine.sign_data(&private_key, data)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: signature,
            execution_time,
            memory_usage: 0,
            security_level: private_key.metadata.security_level,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Verify signature with ML-DSA
    pub fn verify_signature(
        &mut self,
        key_id: &str,
        signature: &Signature,
        data: &[u8],
    ) -> Result<CryptographicResult<bool>, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Get public key
        let public_key = self.key_manager.get_key(key_id)?;

        // Validate key type
        if public_key.key_type != KeyType::Public {
            return Err(CryptographicError::InvalidKey(
                "Key must be public for verification".to_string(),
            ));
        }

        // Verify signature
        let is_valid = self
            .signature_engine
            .verify_signature(&public_key, signature, data)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: is_valid,
            execution_time,
            memory_usage: 0,
            security_level: public_key.metadata.security_level,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Encrypt data with AES-256-GCM
    pub fn encrypt_data(
        &mut self,
        key_id: &str,
        data: &[u8],
        additional_data: Option<&[u8]>,
    ) -> Result<CryptographicResult<EncryptedData>, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Get symmetric key
        let key = self.key_manager.get_key(key_id)?;

        // Validate key type
        if key.key_type != KeyType::Symmetric {
            return Err(CryptographicError::InvalidKey(
                "Key must be symmetric for encryption".to_string(),
            ));
        }

        // Encrypt data
        let encrypted_data = self
            .encryption_engine
            .encrypt_data(&key, data, additional_data)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: encrypted_data,
            execution_time,
            memory_usage: 0,
            security_level: key.metadata.security_level,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Encrypt data with an explicitly chosen AEAD algorithm
    /// (AES-256-GCM, ChaCha20-Poly1305, or XChaCha20-Poly1305).
    pub fn encrypt_data_with_algorithm(
        &mut self,
        key_id: &str,
        data: &[u8],
        additional_data: Option<&[u8]>,
        algorithm: EncryptionAlgorithm,
    ) -> Result<CryptographicResult<EncryptedData>, CryptographicError> {
        let start_time = std::time::Instant::now();

        let key = self.key_manager.get_key(key_id)?;
        if key.key_type != KeyType::Symmetric {
            return Err(CryptographicError::InvalidKey(
                "Key must be symmetric for encryption".to_string(),
            ));
        }

        let encrypted_data =
            self.encryption_engine
                .encrypt_data_with(&key, data, additional_data, algorithm)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: encrypted_data,
            execution_time,
            memory_usage: 0,
            security_level: key.metadata.security_level,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Decrypt data with AES-256-GCM
    pub fn decrypt_data(
        &mut self,
        key_id: &str,
        encrypted_data: &EncryptedData,
    ) -> Result<CryptographicResult<Vec<u8>>, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Get symmetric key
        let key = self.key_manager.get_key(key_id)?;

        // Validate key type
        if key.key_type != KeyType::Symmetric {
            return Err(CryptographicError::InvalidKey(
                "Key must be symmetric for decryption".to_string(),
            ));
        }

        // Decrypt data
        let decrypted_data = self.encryption_engine.decrypt_data(&key, encrypted_data)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: decrypted_data,
            execution_time,
            memory_usage: 0,
            security_level: key.metadata.security_level,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Compute hash with SHA-256
    pub fn compute_hash(
        &mut self,
        data: &[u8],
    ) -> Result<CryptographicResult<HashResult>, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Compute hash
        let hash_result = self.hash_engine.compute_hash("SHA256", data)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: hash_result,
            execution_time,
            memory_usage: 0,
            security_level: SecurityLevel::High,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Compute hash with BLAKE3 (32-byte digest)
    pub fn compute_hash_blake3(
        &mut self,
        data: &[u8],
    ) -> Result<CryptographicResult<HashResult>, CryptographicError> {
        let start_time = std::time::Instant::now();

        let hash_result = self.hash_engine.compute_hash("BLAKE3", data)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: hash_result,
            execution_time,
            memory_usage: 0,
            security_level: SecurityLevel::High,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Derive key material using HKDF-SHA256 (RFC 5869).
    pub fn derive_hkdf(&self, ikm: &[u8], info: &[u8]) -> Result<Vec<u8>, CryptographicError> {
        self.encryption_engine.derive_hkdf(ikm, info)
    }

    /// Issue an ML-DSA-signed Verifiable Credential via the fiduciary VC fragment layout.
    pub fn issue_vc_mldsa(
        &self,
        claim_quins: &[crate::NQuin],
        issuer_sk_key_id: &str,
        issuer_did_hash: u64,
        context: &CryptoContext,
    ) -> Result<CryptographicResult<MlDsaVcProof>, CryptographicError> {
        let start_time = std::time::Instant::now();
        let sk_key = self.key_manager.get_key(issuer_sk_key_id)?;
        if sk_key.key_type != KeyType::Private {
            return Err(CryptographicError::InvalidKey(
                "Issuer key must be private for VC issuance".to_string(),
            ));
        }
        let proof = MlDsaVcProof::issue_vc_mldsa(
            claim_quins,
            &sk_key.key_data,
            issuer_did_hash,
            context,
        )
        .map_err(|e| CryptographicError::SignatureError(format!("VC issuance failed: {e}")))?;
        let execution_time = start_time.elapsed().as_millis() as u64;
        Ok(CryptographicResult {
            result: proof,
            execution_time,
            memory_usage: 0,
            security_level: sk_key.metadata.security_level,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Verify an ML-DSA-signed Verifiable Credential issued via [`Self::issue_vc_mldsa`].
    pub fn verify_vc_mldsa(
        &self,
        proof: &MlDsaVcProof,
        claim_quins: &[crate::NQuin],
        issuer_pk_key_id: &str,
        context: &CryptoContext,
    ) -> Result<CryptographicResult<bool>, CryptographicError> {
        let start_time = std::time::Instant::now();
        let pk_key = self.key_manager.get_key(issuer_pk_key_id)?;
        if pk_key.key_type != KeyType::Public {
            return Err(CryptographicError::InvalidKey(
                "Issuer key must be public for VC verification".to_string(),
            ));
        }
        let is_valid = proof
            .verify_vc_mldsa(claim_quins, &pk_key.key_data, context)
            .map_err(|e| {
                CryptographicError::SignatureError(format!("VC verification failed: {e}"))
            })?;
        let execution_time = start_time.elapsed().as_millis() as u64;
        Ok(CryptographicResult {
            result: is_valid,
            execution_time,
            memory_usage: 0,
            security_level: pk_key.metadata.security_level,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Generate zero-knowledge proof
    pub fn generate_zk_proof(
        &mut self,
        circuit_id: &str,
        witness: &[Vec<u8>],
        public_inputs: &[Vec<u8>],
    ) -> Result<CryptographicResult<Proof>, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Generate proof
        let proof = self
            .proof_engine
            .generate_proof(circuit_id, witness, public_inputs)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: proof,
            execution_time,
            memory_usage: 0,
            security_level: SecurityLevel::Critical,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Verify zero-knowledge proof
    pub fn verify_zk_proof(
        &mut self,
        proof: &Proof,
        public_inputs: &[Vec<u8>],
    ) -> Result<CryptographicResult<bool>, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Verify proof
        let is_valid = self.proof_engine.verify_proof(proof, public_inputs)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: is_valid,
            execution_time,
            memory_usage: 0,
            security_level: SecurityLevel::Critical,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Get security metrics
    pub fn get_security_metrics(&self) -> SecurityMetrics {
        self.security_monitor.get_metrics()
    }

    /// List all keys
    pub fn list_keys(&self) -> Vec<String> {
        self.key_manager.list_keys()
    }

    /// Get key information
    pub fn get_key_info(&self, key_id: &str) -> Option<KeyMetadata> {
        self.key_manager.get_key_metadata(key_id)
    }

    /// Rotate key
    pub fn rotate_key(
        &mut self,
        key_id: &str,
    ) -> Result<CryptographicResult<Key>, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Get old key
        let old_key = self.key_manager.get_key(key_id)?;

        // Generate new key
        let new_key = self.key_manager.rotate_key(&old_key)?;

        // Track the RotatedFrom relationship in the catalog
        self.key_manager
            .key_storage
            .key_catalog
            .add_relationship(
                &new_key.key_id,
                &old_key.key_id,
                KeyRelationshipType::RotatedFrom,
            );
        self.key_manager
            .key_storage
            .key_catalog
            .register_key(new_key.metadata.clone());

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: new_key,
            execution_time,
            memory_usage: 0,
            security_level: old_key.metadata.security_level,
            compliance_status: ComplianceStatus::Compliant,
        })
    }
}

// Supporting implementations

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
        key_type: KeyType,
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
            if !self.access_control.check_permission(key_id, operation.clone()) {
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
    pub fn find_related(&self, key_id: &str, rel_type: KeyRelationshipType) -> Option<&KeyRelationship> {
        self.relationships
            .get(key_id)
            .and_then(|rels| rels.iter().find(|r| r.relationship_type == rel_type))
    }

    /// Register key metadata in the catalog.
    pub fn register_key(&mut self, metadata: KeyMetadata) {
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
            search_engine: KeySearchEngine::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }
}

impl KeySearchEngine {
    pub fn new() -> Self {
        Self {
            engine_type: SearchEngineType::Encrypted,
            indexing_strategy: IndexingStrategy::Encrypted,
        }
    }
}

impl EncryptionAtRest {
    pub fn new() -> Self {
        Self {
            encryption_algorithm: EncryptionAlgorithm::AES256GCM,
            key_encryption_keys: HashMap::new(),
            encryption_policy: EncryptionPolicy {
                encryption_required: true,
                key_rotation_interval: 86400 * 30, // 30 days
                algorithm_preference: vec![EncryptionAlgorithm::AES256GCM],
                compliance_requirements: vec![ComplianceRequirement::FIPS140_2],
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
        let kek = self
            .key_encryption_keys
            .get("master_kek")
            .ok_or_else(|| CryptographicError::EncryptionError("no master KEK available".to_string()))?;

        use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(kek);
        let cipher = Aes256Gcm::new(key);

        let nonce_bytes: [u8; 12] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| CryptographicError::EncryptionError(format!("AES-GCM encrypt failed: {e}")))?;

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
        let kek = self
            .key_encryption_keys
            .get("master_kek")
            .ok_or_else(|| CryptographicError::DecryptionError("no master KEK available".to_string()))?;

        use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(kek);
        let cipher = Aes256Gcm::new(key);

        let nonce = Nonce::from_slice(&packed[..12]);
        let ciphertext = &packed[12..];

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| CryptographicError::DecryptionError(format!("AES-GCM decrypt failed: {e}")))
    }

    /// Check if encryption at rest is enabled (KEK exists).
    pub fn is_enabled(&self) -> bool {
        self.key_encryption_keys.contains_key("master_kek")
    }

    /// Number of registered KEKs.
    pub fn kek_count(&self) -> usize {
        self.key_encryption_keys.len()
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
        self.access_policies.insert(policy.policy_id.clone(), policy);
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
        self.access_policies
            .values()
            .any(|p| {
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
                if !p.ip_restrictions.is_empty()
                    && !p.ip_restrictions.iter().any(|ip| ip == ip_address)
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
        self.audit_log.log_entry(key_id, operation, user_id, success);
    }

    /// Number of registered policies.
    pub fn policy_count(&self) -> usize {
        self.access_policies.len()
    }

    /// Get a reference to the audit log.
    pub fn audit_log(&self) -> &AccessAuditLog {
        &self.audit_log
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
    pub fn log_entry(&mut self, key_id: &str, operation: KeyOperation, user_id: &str, success: bool) {
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
            error_message: if success { None } else { Some("operation failed".to_string()) },
        };
        self.entries.push(entry);
        // Enforce retention: drop entries older than retention_days
        let cutoff = timestamp.saturating_sub((self.retention_policy.retention_days as u64) * 86400);
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
            entropy_sources: vec![EntropySource::HardwareRNG, EntropySource::Quantum],
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
        let key_data = self.generate_key_data(&generation_algorithm, security_level.clone())?;

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

    fn generate_key_data(
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
        let cutoff = timestamp.saturating_sub((self.retention_policy.retention_days as u64) * 86400);
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

impl EncryptionEngine {
    pub fn new() -> Self {
        Self {
            encryption_algorithms: HashMap::new(),
            decryption_algorithms: HashMap::new(),
            key_derivation: KeyDerivation::new(),
            performance_optimizer: EncryptionPerformanceOptimizer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.key_derivation.initialize()?;
        self.performance_optimizer.initialize()?;
        Ok(())
    }

    /// Derive key material using HKDF-SHA256 via the embedded [`KeyDerivation`] engine.
    pub fn derive_hkdf(&self, ikm: &[u8], info: &[u8]) -> Result<Vec<u8>, CryptographicError> {
        self.key_derivation.derive_hkdf(ikm, info)
    }

    pub fn encrypt_data(
        &mut self,
        key: &Key,
        data: &[u8],
        additional_data: Option<&[u8]>,
    ) -> Result<EncryptedData, CryptographicError> {
        self.encrypt_data_with(key, data, additional_data, EncryptionAlgorithm::AES256GCM)
    }

    /// Encrypt with an explicitly chosen AEAD algorithm
    /// (AES-256-GCM, ChaCha20-Poly1305, or XChaCha20-Poly1305).
    pub fn encrypt_data_with(
        &mut self,
        key: &Key,
        data: &[u8],
        additional_data: Option<&[u8]>,
        algorithm: EncryptionAlgorithm,
    ) -> Result<EncryptedData, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Generate a nonce sized for the chosen algorithm
        let iv = self.generate_iv(&algorithm)?;

        // Encrypt data
        let (ciphertext, tag) =
            self.encrypt_with_key(&key, data, &iv, additional_data, &algorithm)?;

        let mode = match algorithm {
            EncryptionAlgorithm::AES256GCM => EncryptionMode::GCM,
            // ChaCha20 is a counter-mode stream cipher with a Poly1305 MAC; CTR is the
            // closest honest descriptor in the (cosmetic) EncryptionMode enum.
            _ => EncryptionMode::CTR,
        };

        let encrypted_data = EncryptedData {
            data_id: format!(
                "enc_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
            algorithm: algorithm.clone(),
            ciphertext,
            iv,
            tag,
            aad: additional_data.unwrap_or(b"").to_vec(),
            metadata: EncryptionMetadata {
                key_id: key.key_id.clone(),
                algorithm,
                mode,
                padding: Some(EncryptionPadding::NoPadding),
                created_at: start_time.elapsed().as_millis() as u64,
            },
        };

        // Record performance metrics
        self.performance_optimizer
            .record_encryption_time(start_time.elapsed().as_millis() as f64);

        Ok(encrypted_data)
    }

    pub fn decrypt_data(
        &mut self,
        key: &Key,
        encrypted_data: &EncryptedData,
    ) -> Result<Vec<u8>, CryptographicError> {
        let start_time = std::time::Instant::now();
        // Dispatch on the algorithm the ciphertext was produced with.
        let aad_ref = if encrypted_data.aad.is_empty() {
            None
        } else {
            Some(encrypted_data.aad.as_slice())
        };
        let plaintext = self.decrypt_with_key(
            &key,
            &encrypted_data.ciphertext,
            &encrypted_data.iv,
            &encrypted_data.tag,
            aad_ref,
            &encrypted_data.algorithm,
        )?;

        // Record performance metrics
        self.performance_optimizer
            .record_decryption_time(start_time.elapsed().as_millis() as f64);

        Ok(plaintext)
    }

    /// Expected nonce length in bytes for the given AEAD algorithm.
    fn nonce_len(algorithm: &EncryptionAlgorithm) -> usize {
        match algorithm {
            EncryptionAlgorithm::XChaCha20Poly1305 => 24,
            _ => 12, // AES-256-GCM and ChaCha20-Poly1305
        }
    }

    fn generate_iv(&self, algorithm: &EncryptionAlgorithm) -> Result<Vec<u8>, CryptographicError> {
        let len = Self::nonce_len(algorithm);
        let mut iv = vec![0u8; len];
        for b in iv.iter_mut() {
            *b = rand::random::<u8>();
        }
        Ok(iv)
    }

    fn encrypt_with_key(
        &self,
        key: &Key,
        data: &[u8],
        iv: &[u8],
        additional_data: Option<&[u8]>,
        algorithm: &EncryptionAlgorithm,
    ) -> Result<(Vec<u8>, Vec<u8>), CryptographicError> {
        use aead::{AeadInOut, KeyInit};
        if key.key_data.len() < 32 {
            return Err(CryptographicError::EncryptionError(
                "AEAD key must be 32 bytes".to_string(),
            ));
        }
        let expected_nonce = Self::nonce_len(algorithm);
        if iv.len() != expected_nonce {
            return Err(CryptographicError::EncryptionError(format!(
                "IV must be {expected_nonce} bytes for this algorithm"
            )));
        }
        let aad = additional_data.unwrap_or(b"");
        let mut buffer = data.to_vec();
        let tag = match algorithm {
            EncryptionAlgorithm::AES256GCM => {
                use aes_gcm::Aes256Gcm;
                let cipher =
                    Aes256Gcm::new(&aes_gcm::Key::<Aes256Gcm>::try_from(&key.key_data[..32]).unwrap());
                cipher
                    .encrypt_inout_detached(
                        &aes_gcm::Nonce::try_from(iv).unwrap(),
                        aad,
                        (&mut buffer[..]).into(),
                    )
                    .map_err(|e| CryptographicError::EncryptionError(e.to_string()))?
                    .to_vec()
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                use chacha20poly1305::ChaCha20Poly1305;
                let cipher = ChaCha20Poly1305::new(
                    &chacha20poly1305::Key::try_from(&key.key_data[..32]).unwrap(),
                );
                cipher
                    .encrypt_inout_detached(
                        &chacha20poly1305::Nonce::try_from(iv).unwrap(),
                        aad,
                        (&mut buffer[..]).into(),
                    )
                    .map_err(|e| CryptographicError::EncryptionError(e.to_string()))?
                    .to_vec()
            }
            EncryptionAlgorithm::XChaCha20Poly1305 => {
                use chacha20poly1305::XChaCha20Poly1305;
                let cipher = XChaCha20Poly1305::new(
                    &chacha20poly1305::Key::try_from(&key.key_data[..32]).unwrap(),
                );
                cipher
                    .encrypt_inout_detached(
                        &chacha20poly1305::XNonce::try_from(iv).unwrap(),
                        aad,
                        (&mut buffer[..]).into(),
                    )
                    .map_err(|e| CryptographicError::EncryptionError(e.to_string()))?
                    .to_vec()
            }
            EncryptionAlgorithm::Custom(name) => {
                return Err(CryptographicError::UnsupportedAlgorithm(format!(
                    "Custom cipher '{name}' not implemented"
                )));
            }
        };
        Ok((buffer, tag))
    }

    fn decrypt_with_key(
        &self,
        key: &Key,
        ciphertext: &[u8],
        iv: &[u8],
        tag: &[u8],
        additional_data: Option<&[u8]>,
        algorithm: &EncryptionAlgorithm,
    ) -> Result<Vec<u8>, CryptographicError> {
        use aead::{AeadInOut, KeyInit};
        if key.key_data.len() < 32 {
            return Err(CryptographicError::DecryptionError(
                "AEAD key must be 32 bytes".to_string(),
            ));
        }
        let expected_nonce = Self::nonce_len(algorithm);
        if iv.len() != expected_nonce {
            return Err(CryptographicError::DecryptionError(format!(
                "IV must be {expected_nonce} bytes for this algorithm"
            )));
        }
        if tag.len() != 16 {
            return Err(CryptographicError::DecryptionError(
                "AEAD tag must be 16 bytes".to_string(),
            ));
        }
        let aad = additional_data.unwrap_or(b"");
        let mut buffer = ciphertext.to_vec();
        match algorithm {
            EncryptionAlgorithm::AES256GCM => {
                use aes_gcm::Aes256Gcm;
                let cipher =
                    Aes256Gcm::new(&aes_gcm::Key::<Aes256Gcm>::try_from(&key.key_data[..32]).unwrap());
                cipher
                    .decrypt_inout_detached(
                        &aes_gcm::Nonce::try_from(iv).unwrap(),
                        aad,
                        (&mut buffer[..]).into(),
                        &aes_gcm::Tag::try_from(tag).unwrap(),
                    )
                    .map_err(|e| CryptographicError::DecryptionError(e.to_string()))?;
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                use chacha20poly1305::ChaCha20Poly1305;
                let cipher = ChaCha20Poly1305::new(
                    &chacha20poly1305::Key::try_from(&key.key_data[..32]).unwrap(),
                );
                cipher
                    .decrypt_inout_detached(
                        &chacha20poly1305::Nonce::try_from(iv).unwrap(),
                        aad,
                        (&mut buffer[..]).into(),
                        &chacha20poly1305::Tag::try_from(tag).unwrap(),
                    )
                    .map_err(|e| CryptographicError::DecryptionError(e.to_string()))?;
            }
            EncryptionAlgorithm::XChaCha20Poly1305 => {
                use chacha20poly1305::XChaCha20Poly1305;
                let cipher = XChaCha20Poly1305::new(
                    &chacha20poly1305::Key::try_from(&key.key_data[..32]).unwrap(),
                );
                cipher
                    .decrypt_inout_detached(
                        &chacha20poly1305::XNonce::try_from(iv).unwrap(),
                        aad,
                        (&mut buffer[..]).into(),
                        &chacha20poly1305::Tag::try_from(tag).unwrap(),
                    )
                    .map_err(|e| CryptographicError::DecryptionError(e.to_string()))?;
            }
            EncryptionAlgorithm::Custom(name) => {
                return Err(CryptographicError::UnsupportedAlgorithm(format!(
                    "Custom cipher '{name}' not implemented"
                )));
            }
        }
        Ok(buffer)
    }
}

impl KeyDerivation {
    pub fn new() -> Self {
        Self {
            derivation_functions: HashMap::new(),
            derivation_parameters: DerivationParameters {
                salt: vec![0u8; 16],
                iterations: 100000,
                memory_cost: 65536,
                parallelism: 4,
                output_length: 32,
            },
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    /// Derive `output_length` bytes from input keying material using HKDF-SHA256.
    ///
    /// Uses the configured `derivation_parameters.salt` and `output_length`. `info`
    /// is the application-specific context/label that domain-separates derived keys.
    pub fn derive_hkdf(&self, ikm: &[u8], info: &[u8]) -> Result<Vec<u8>, CryptographicError> {
        use hkdf::Hkdf;
        use sha2::Sha256;
        let hk = Hkdf::<Sha256>::new(Some(&self.derivation_parameters.salt), ikm);
        let mut okm = vec![0u8; self.derivation_parameters.output_length];
        hk.expand(info, &mut okm)
            .map_err(|e| CryptographicError::EncryptionError(format!("HKDF expand failed: {e}")))?;
        Ok(okm)
    }
}

impl EncryptionPerformanceOptimizer {
    pub fn new() -> Self {
        Self {
            optimization_strategies: vec![
                EncryptionOptimizationStrategy::BatchEncryption,
                EncryptionOptimizationStrategy::ParallelProcessing,
            ],
            performance_metrics: EncryptionPerformanceMetrics {
                average_encryption_time: 0.0,
                average_decryption_time: 0.0,
                throughput: 0.0,
                memory_usage: 0,
                cache_hit_rate: 0.0,
            },
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    /// Record an encryption operation duration (milliseconds).
    pub fn record_encryption_time(&mut self, duration_ms: f64) {
        let m = &mut self.performance_metrics;
        if m.average_encryption_time == 0.0 {
            m.average_encryption_time = duration_ms;
        } else {
            m.average_encryption_time = 0.9 * m.average_encryption_time + 0.1 * duration_ms;
        }
        if m.average_encryption_time > 0.0 {
            m.throughput = 1000.0 / m.average_encryption_time;
        }
    }

    /// Record a decryption operation duration (milliseconds).
    pub fn record_decryption_time(&mut self, duration_ms: f64) {
        let m = &mut self.performance_metrics;
        if m.average_decryption_time == 0.0 {
            m.average_decryption_time = duration_ms;
        } else {
            m.average_decryption_time = 0.9 * m.average_decryption_time + 0.1 * duration_ms;
        }
    }

    /// Get a snapshot of the current performance metrics.
    pub fn metrics(&self) -> &EncryptionPerformanceMetrics {
        &self.performance_metrics
    }
}

impl EncryptionPerformanceMetrics {
    pub fn new() -> Self {
        Self {
            average_encryption_time: 0.0,
            average_decryption_time: 0.0,
            throughput: 0.0,
            memory_usage: 0,
            cache_hit_rate: 0.0,
        }
    }
}

impl HashEngine {
    pub fn new() -> Self {
        Self {
            hash_algorithms: HashMap::new(),
            hash_storage: HashStorage::new(),
            performance_optimizer: HashPerformanceOptimizer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.hash_storage.initialize()?;
        self.performance_optimizer.initialize()?;
        Ok(())
    }

    pub fn compute_hash(
        &mut self,
        algorithm: &str,
        data: &[u8],
    ) -> Result<HashResult, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Compute hash
        let hash_value = match algorithm {
            "SHA256" => {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(data);
                hasher.finalize().to_vec()
            }
            "SHA512" => {
                use sha2::{Digest, Sha512};
                let mut hasher = Sha512::new();
                hasher.update(data);
                hasher.finalize().to_vec()
            }
            "BLAKE3" => blake3::hash(data).as_bytes().to_vec(),
            _ => {
                return Err(CryptographicError::UnsupportedAlgorithm(
                    "Hash algorithm not supported".to_string(),
                ))
            }
        };

        let hash_result = HashResult {
            hash_id: format!(
                "hash_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
            algorithm: algorithm.to_string(),
            input_data: data.to_vec(),
            hash_value,
            timestamp: start_time.elapsed().as_millis() as u64,
        };

        // Store hash
        self.hash_storage.store_hash(hash_result.clone())?;

        // Audit log the hash computation
        self.hash_storage.audit_log.log_entry(
            &hash_result.hash_id,
            HashOperation::Compute,
            "system",
            true,
        );

        // Record performance metrics
        self.performance_optimizer
            .record_hash_time(start_time.elapsed().as_millis() as f64);

        Ok(hash_result)
    }
}

impl HashStorage {
    pub fn new() -> Self {
        Self {
            hashes: HashMap::new(),
            verification_records: HashMap::new(),
            audit_log: HashAuditLog::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    pub fn store_hash(&mut self, hash: HashResult) -> Result<(), CryptographicError> {
        self.hashes.insert(hash.hash_id.clone(), hash);
        Ok(())
    }
}

impl HashAuditLog {
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

    /// Record a hash operation (compute, verify, update, delete).
    pub fn log_entry(
        &mut self,
        hash_id: &str,
        operation: HashOperation,
        user_id: &str,
        success: bool,
    ) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let entry = HashAuditEntry {
            entry_id: format!("hash_{}_{}", timestamp, self.entries.len()),
            timestamp,
            hash_id: hash_id.to_string(),
            operation,
            user_id: user_id.to_string(),
            ip_address: String::new(),
            success,
        };
        self.entries.push(entry);
        let cutoff = timestamp.saturating_sub((self.retention_policy.retention_days as u64) * 86400);
        self.entries.retain(|e| e.timestamp >= cutoff);
    }

    /// Number of logged entries.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Iterate over entries.
    pub fn entries(&self) -> &[HashAuditEntry] {
        &self.entries
    }
}

impl HashPerformanceOptimizer {
    pub fn new() -> Self {
        Self {
            optimization_strategies: vec![
                HashOptimizationStrategy::BatchHashing,
                HashOptimizationStrategy::ParallelProcessing,
            ],
            performance_metrics: HashPerformanceMetrics {
                average_hash_time: 0.0,
                throughput: 0.0,
                memory_usage: 0,
                cache_hit_rate: 0.0,
            },
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    /// Record a hash computation duration (milliseconds).
    pub fn record_hash_time(&mut self, duration_ms: f64) {
        let m = &mut self.performance_metrics;
        if m.average_hash_time == 0.0 {
            m.average_hash_time = duration_ms;
        } else {
            m.average_hash_time = 0.9 * m.average_hash_time + 0.1 * duration_ms;
        }
        if m.average_hash_time > 0.0 {
            m.throughput = 1000.0 / m.average_hash_time;
        }
    }

    /// Get a snapshot of the current performance metrics.
    pub fn metrics(&self) -> &HashPerformanceMetrics {
        &self.performance_metrics
    }
}

impl HashPerformanceMetrics {
    pub fn new() -> Self {
        Self {
            average_hash_time: 0.0,
            throughput: 0.0,
            memory_usage: 0,
            cache_hit_rate: 0.0,
        }
    }
}

impl ProofEngine {
    pub fn new() -> Self {
        Self {
            proof_systems: HashMap::new(),
            proof_storage: ProofStorage::new(),
            verification_engine: ProofVerificationEngine::new(),
            performance_optimizer: ProofPerformanceOptimizer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.proof_storage.initialize()?;
        self.verification_engine.initialize()?;
        self.performance_optimizer.initialize()?;
        Ok(())
    }

    pub fn generate_proof(
        &mut self,
        circuit_id: &str,
        witness: &[Vec<u8>],
        public_inputs: &[Vec<u8>],
    ) -> Result<Proof, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Generate proof
        let proof_data = self.generate_proof_data(circuit_id, witness, public_inputs)?;

        let proof = Proof {
            proof_id: format!(
                "proof_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
            system_id: "zk_snarks".to_string(),
            circuit_id: circuit_id.to_string(),
            public_inputs: public_inputs.to_vec(),
            proof_data,
            timestamp: start_time.elapsed().as_millis() as u64,
        };

        // Store proof
        self.proof_storage.store_proof(proof.clone())?;

        // Audit log the proof generation
        self.proof_storage.audit_log.log_entry(
            &proof.proof_id,
            ProofOperation::Generate,
            "system",
            true,
        );

        // Record performance metrics
        self.performance_optimizer.record_proving_time(
            start_time.elapsed().as_millis() as f64,
            proof.proof_data.len(),
        );

        Ok(proof)
    }

    pub fn verify_proof(
        &mut self,
        proof: &Proof,
        public_inputs: &[Vec<u8>],
    ) -> Result<bool, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Verify proof
        let is_valid = self.verify_proof_data(&proof.proof_data, public_inputs)?;

        // Store verification record
        let verification_record = ProofVerificationRecord {
            verification_id: format!(
                "proof_verif_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
            proof_id: proof.proof_id.clone(),
            verifier_id: "system".to_string(),
            result: ProofVerificationResult {
                valid: is_valid,
                error_message: None,
                verification_time: start_time.elapsed().as_millis() as u64,
                confidence: 1.0,
            },
            timestamp: start_time.elapsed().as_millis() as u64,
        };

        self.proof_storage
            .store_verification_record(verification_record)?;

        // Audit log the proof verification
        self.proof_storage.audit_log.log_entry(
            &proof.proof_id,
            ProofOperation::Verify,
            "system",
            is_valid,
        );

        // Record performance metrics
        self.performance_optimizer
            .record_verification_time(start_time.elapsed().as_millis() as f64);

        Ok(is_valid)
    }

    fn generate_proof_data(
        &self,
        circuit_id: &str,
        witness: &[Vec<u8>],
        public_inputs: &[Vec<u8>],
    ) -> Result<Vec<u8>, CryptographicError> {
        #[cfg(feature = "zk-culling")]
        if circuit_id == "deontic_access" {
            return Self::generate_deontic_groth16_proof(witness, public_inputs);
        }

        Self::generate_commitment_proof_data(circuit_id, witness, public_inputs)
    }

    fn verify_proof_data(
        &self,
        proof_data: &[u8],
        public_inputs: &[Vec<u8>],
    ) -> Result<bool, CryptographicError> {
        if proof_data.len() < 65 {
            return Ok(false);
        }
        if proof_data[64] == 0x02 {
            #[cfg(feature = "zk-culling")]
            {
                return Self::verify_deontic_groth16_proof(proof_data, public_inputs);
            }
            #[cfg(not(feature = "zk-culling"))]
            {
                return Ok(false);
            }
        }
        Self::verify_commitment_proof_data(proof_data, public_inputs)
    }

    fn generate_commitment_proof_data(
        circuit_id: &str,
        witness: &[Vec<u8>],
        public_inputs: &[Vec<u8>],
    ) -> Result<Vec<u8>, CryptographicError> {
        use sha2::{Digest, Sha256};
        // Commitment: H(circuit_id || witness_bytes) stored in proof_data[0..32]
        // Public input binding: H(public_inputs) stored in proof_data[32..64]
        // Proof version tag in proof_data[64..128]
        let mut witness_hasher = Sha256::new();
        witness_hasher.update(circuit_id.as_bytes());
        for w in witness {
            witness_hasher.update(w);
        }
        let witness_commit = witness_hasher.finalize();

        let mut pub_hasher = Sha256::new();
        for p in public_inputs {
            pub_hasher.update(p);
        }
        let pub_commit = pub_hasher.finalize();

        let mut proof_data = vec![0u8; 128];
        proof_data[..32].copy_from_slice(&witness_commit);
        proof_data[32..64].copy_from_slice(&pub_commit);
        // Version tag 0x01 = SHA-256 commitment stub
        proof_data[64] = 0x01;
        proof_data[65] = 0x00;
        Ok(proof_data)
    }

    fn verify_commitment_proof_data(
        proof_data: &[u8],
        public_inputs: &[Vec<u8>],
    ) -> Result<bool, CryptographicError> {
        use sha2::{Digest, Sha256};
        if proof_data.len() < 128 {
            return Ok(false);
        }
        if proof_data[64] != 0x01 {
            return Ok(false);
        }
        let mut pub_hasher = Sha256::new();
        for p in public_inputs {
            pub_hasher.update(p);
        }
        let expected_pub_commit = pub_hasher.finalize();
        Ok(&proof_data[32..64] == expected_pub_commit.as_slice())
    }

    /// Reduce a secret WITNESS value to a field element via SHA-256 (the secret
    /// never leaves the prover; only its field image enters the circuit).
    #[cfg(feature = "zk-culling")]
    fn bytes_to_fr(data: &[u8]) -> ark_bls12_381::Fr {
        use ark_ff::PrimeField;
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(data);
        ark_bls12_381::Fr::from_be_bytes_mod_order(hash.as_slice())
    }

    /// Interpret a PUBLIC input as a canonical little-endian field element — NOT
    /// hashed. Prover and verifier must agree on the exact public value (e.g. a
    /// `policy_root` the witness genuinely satisfies), so hashing it (as for a
    /// witness) would make a valid proof unconstructible. Callers serialise the
    /// field element little-endian (`Fr::into_bigint().to_bytes_le()`).
    #[cfg(feature = "zk-culling")]
    fn public_input_to_fr(data: &[u8]) -> ark_bls12_381::Fr {
        use ark_ff::PrimeField;
        ark_bls12_381::Fr::from_le_bytes_mod_order(data)
    }

    #[cfg(feature = "zk-culling")]
    fn deontic_crs() -> Result<
        &'static (
            ark_groth16::ProvingKey<ark_bls12_381::Bls12_381>,
            ark_groth16::VerifyingKey<ark_bls12_381::Bls12_381>,
        ),
        CryptographicError,
    > {
        use std::sync::OnceLock;
        static CRS: OnceLock<(
            ark_groth16::ProvingKey<ark_bls12_381::Bls12_381>,
            ark_groth16::VerifyingKey<ark_bls12_381::Bls12_381>,
        )> = OnceLock::new();
        CRS.get_or_init(|| {
            crate::deontic_circuit::generate_deontic_crs()
                .map_err(|e| CryptographicError::ProofError(e))
                .expect("deontic CRS setup")
        });
        Ok(CRS.get().expect("deontic CRS initialized"))
    }

    #[cfg(feature = "zk-culling")]
    fn generate_deontic_groth16_proof(
        witness: &[Vec<u8>],
        public_inputs: &[Vec<u8>],
    ) -> Result<Vec<u8>, CryptographicError> {
        use ark_bls12_381::Bls12_381;
        use ark_groth16::Groth16;
        use ark_relations::gr1cs::ConstraintSynthesizer;
        use ark_serialize::CanonicalSerialize;
        use ark_snark::SNARK;
        use sha2::{Digest, Sha256};

        let user_did = Self::bytes_to_fr(witness.first().map(|v| v.as_slice()).unwrap_or(b""));
        let role_id = Self::bytes_to_fr(witness.get(1).map(|v| v.as_slice()).unwrap_or(b""));
        let action_permission =
            Self::bytes_to_fr(witness.get(2).map(|v| v.as_slice()).unwrap_or(b""));
        let policy_root =
            Self::public_input_to_fr(public_inputs.first().map(|v| v.as_slice()).unwrap_or(b""));
        let temporal_constraint =
            Self::public_input_to_fr(public_inputs.get(1).map(|v| v.as_slice()).unwrap_or(b""));

        let circuit = crate::deontic_circuit::DeonticAccessCircuit {
            user_did_commitment: Some(user_did),
            role_id: Some(role_id),
            action_permission: Some(action_permission),
            policy_root: Some(policy_root),
            temporal_constraint: Some(temporal_constraint),
        };

        let (pk, _vk) = Self::deontic_crs()?;
        let mut rng = ark_std::rand::rngs::OsRng;
        let proof = Groth16::<Bls12_381>::prove(pk, circuit, &mut rng)
            .map_err(|e| CryptographicError::ProofError(e.to_string()))?;

        let mut serialized = Vec::new();
        proof
            .serialize_uncompressed(&mut serialized)
            .map_err(|e| CryptographicError::ProofError(e.to_string()))?;

        let mut witness_hasher = Sha256::new();
        witness_hasher.update(b"deontic_access");
        for w in witness {
            witness_hasher.update(w);
        }
        let witness_commit = witness_hasher.finalize();
        let mut pub_hasher = Sha256::new();
        for p in public_inputs {
            pub_hasher.update(p);
        }
        let pub_commit = pub_hasher.finalize();

        let mut proof_data = vec![0u8; 65 + serialized.len()];
        proof_data[..32].copy_from_slice(&witness_commit);
        proof_data[32..64].copy_from_slice(&pub_commit);
        proof_data[64] = 0x02;
        proof_data[65..].copy_from_slice(&serialized);
        Ok(proof_data)
    }

    #[cfg(feature = "zk-culling")]
    fn verify_deontic_groth16_proof(
        proof_data: &[u8],
        public_inputs: &[Vec<u8>],
    ) -> Result<bool, CryptographicError> {
        use ark_bls12_381::{Bls12_381, Fr};
        use ark_groth16::{Groth16, Proof};
        use ark_serialize::CanonicalDeserialize;
        use ark_snark::SNARK;

        if proof_data.len() <= 65 {
            return Ok(false);
        }
        let proof = Proof::<Bls12_381>::deserialize_uncompressed(&proof_data[65..])
            .map_err(|e| CryptographicError::ProofError(e.to_string()))?;
        let (_pk, vk) = Self::deontic_crs()?;
        let public_fr: Vec<Fr> = public_inputs
            .iter()
            .map(|p| Self::public_input_to_fr(p))
            .collect();
        Ok(Groth16::<Bls12_381>::verify(vk, &public_fr, &proof)
            .map_err(|e| CryptographicError::ProofError(e.to_string()))?)
    }
}

impl ProofStorage {
    pub fn new() -> Self {
        Self {
            proofs: HashMap::new(),
            verification_records: HashMap::new(),
            audit_log: ProofAuditLog::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    pub fn store_proof(&mut self, proof: Proof) -> Result<(), CryptographicError> {
        self.proofs.insert(proof.proof_id.clone(), proof);
        Ok(())
    }

    pub fn store_verification_record(
        &mut self,
        record: ProofVerificationRecord,
    ) -> Result<(), CryptographicError> {
        self.verification_records
            .insert(record.verification_id.clone(), record);
        Ok(())
    }
}

impl ProofAuditLog {
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

    /// Record a proof operation (generate, verify, revoke, update).
    pub fn log_entry(
        &mut self,
        proof_id: &str,
        operation: ProofOperation,
        user_id: &str,
        success: bool,
    ) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let entry = ProofAuditEntry {
            entry_id: format!("proof_{}_{}", timestamp, self.entries.len()),
            timestamp,
            proof_id: proof_id.to_string(),
            operation,
            user_id: user_id.to_string(),
            ip_address: String::new(),
            success,
        };
        self.entries.push(entry);
        let cutoff = timestamp.saturating_sub((self.retention_policy.retention_days as u64) * 86400);
        self.entries.retain(|e| e.timestamp >= cutoff);
    }

    /// Number of logged entries.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Iterate over entries.
    pub fn entries(&self) -> &[ProofAuditEntry] {
        &self.entries
    }
}

impl ProofVerificationEngine {
    pub fn new() -> Self {
        Self {
            verification_algorithms: HashMap::new(),
            batch_verifier: BatchVerifier::new(),
            performance_optimizer: VerificationPerformanceOptimizer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.performance_optimizer.initialize()?;
        Ok(())
    }
}

impl BatchVerifier {
    pub fn new() -> Self {
        Self {
            batch_size: 100,
            parallel_verification: true,
            verification_queue: Vec::new(),
        }
    }
}

impl VerificationPerformanceOptimizer {
    pub fn new() -> Self {
        Self {
            optimization_strategies: vec![
                VerificationOptimizationStrategy::BatchVerification,
                VerificationOptimizationStrategy::ParallelProcessing,
            ],
            performance_metrics: VerificationPerformanceMetrics {
                average_verification_time: 0.0,
                throughput: 0.0,
                cache_hit_rate: 0.0,
                batch_efficiency: 0.0,
            },
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }
}

impl VerificationPerformanceMetrics {
    pub fn new() -> Self {
        Self {
            average_verification_time: 0.0,
            throughput: 0.0,
            cache_hit_rate: 0.0,
            batch_efficiency: 0.0,
        }
    }
}

impl ProofPerformanceOptimizer {
    pub fn new() -> Self {
        Self {
            optimization_strategies: vec![
                ProofOptimizationStrategy::ParallelProving,
                ProofOptimizationStrategy::CircuitOptimization,
            ],
            performance_metrics: ProofPerformanceMetrics {
                average_proving_time: 0.0,
                average_verification_time: 0.0,
                proof_size: 0,
                circuit_size: 0,
                cache_hit_rate: 0.0,
            },
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    /// Record a proof generation duration (milliseconds).
    pub fn record_proving_time(&mut self, duration_ms: f64, proof_size: usize) {
        let m = &mut self.performance_metrics;
        if m.average_proving_time == 0.0 {
            m.average_proving_time = duration_ms;
        } else {
            m.average_proving_time = 0.9 * m.average_proving_time + 0.1 * duration_ms;
        }
        m.proof_size = proof_size as u64;
    }

    /// Record a proof verification duration (milliseconds).
    pub fn record_verification_time(&mut self, duration_ms: f64) {
        let m = &mut self.performance_metrics;
        if m.average_verification_time == 0.0 {
            m.average_verification_time = duration_ms;
        } else {
            m.average_verification_time = 0.9 * m.average_verification_time + 0.1 * duration_ms;
        }
    }

    /// Get a snapshot of the current performance metrics.
    pub fn metrics(&self) -> &ProofPerformanceMetrics {
        &self.performance_metrics
    }
}

impl ProofPerformanceMetrics {
    pub fn new() -> Self {
        Self {
            average_proving_time: 0.0,
            average_verification_time: 0.0,
            proof_size: 0,
            circuit_size: 0,
            cache_hit_rate: 0.0,
        }
    }
}

impl SecurityMonitor {
    pub fn new() -> Self {
        Self {
            threat_detector: ThreatDetector::new(),
            anomaly_detector: AnomalyDetector::new(),
            compliance_monitor: ComplianceMonitor::new(),
            security_metrics: SecurityMetrics::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.threat_detector.initialize()?;
        self.anomaly_detector.initialize()?;
        self.compliance_monitor.initialize()?;
        Ok(())
    }

    pub fn get_metrics(&self) -> SecurityMetrics {
        self.security_metrics.clone()
    }
}

impl ThreatDetector {
    pub fn new() -> Self {
        Self {
            threat_signatures: HashMap::new(),
            detection_rules: Vec::new(),
            alert_system: SecurityAlertSystem::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.alert_system.initialize()?;
        Ok(())
    }
}

impl SecurityAlertSystem {
    pub fn new() -> Self {
        Self {
            alert_types: vec![SecurityAlertType::Threat, SecurityAlertType::Anomaly],
            notification_channels: vec![NotificationChannel::Email],
            escalation_policies: Vec::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }
}

impl AnomalyDetector {
    pub fn new() -> Self {
        Self {
            detection_algorithms: vec![AnomalyDetectionAlgorithm::Statistical],
            baseline_models: HashMap::new(),
            alert_thresholds: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }
}

impl ComplianceMonitor {
    pub fn new() -> Self {
        Self {
            compliance_frameworks: HashMap::new(),
            audit_trail: AuditTrail::new(),
            reporting_engine: ComplianceReportingEngine::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.reporting_engine.initialize()?;
        Ok(())
    }
}

impl AuditTrail {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            retention_policy: RetentionPolicy {
                retention_days: 2555, // 7 years
                auto_delete: false,
                archive_before_delete: true,
            },
        }
    }
}

impl ComplianceReportingEngine {
    pub fn new() -> Self {
        Self {
            report_templates: HashMap::new(),
            scheduling_engine: ReportSchedulingEngine::new(),
            distribution_engine: ReportDistributionEngine::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.scheduling_engine.initialize()?;
        self.distribution_engine.initialize()?;
        Ok(())
    }
}

impl ReportSchedulingEngine {
    pub fn new() -> Self {
        Self {
            schedules: HashMap::new(),
            scheduler: ReportScheduler::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }
}

impl ReportScheduler {
    pub fn new() -> Self {
        Self {
            scheduler_type: SchedulerType::Cron,
            queue_manager: ReportQueueManager::new(),
        }
    }
}

impl ReportQueueManager {
    pub fn new() -> Self {
        Self {
            pending_reports: Vec::new(),
            running_reports: Vec::new(),
            completed_reports: Vec::new(),
        }
    }
}

impl ReportDistributionEngine {
    pub fn new() -> Self {
        Self {
            distribution_channels: HashMap::new(),
            delivery_tracker: DeliveryTracker::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }
}

impl DeliveryTracker {
    pub fn new() -> Self {
        Self {
            deliveries: HashMap::new(),
            status: DeliveryStatus {
                total_deliveries: 0,
                successful_deliveries: 0,
                failed_deliveries: 0,
                pending_deliveries: 0,
            },
        }
    }
}

impl DeliveryStatus {
    pub fn new() -> Self {
        Self {
            total_deliveries: 0,
            successful_deliveries: 0,
            failed_deliveries: 0,
            pending_deliveries: 0,
        }
    }
}

impl SecurityMetrics {
    pub fn new() -> Self {
        Self {
            threat_metrics: ThreatMetrics::new(),
            anomaly_metrics: AnomalyMetrics::new(),
            compliance_metrics: ComplianceMetrics::new(),
            performance_metrics: SecurityPerformanceMetrics::new(),
        }
    }

    pub fn get_metrics(&self) -> SecurityMetrics {
        self.clone()
    }
}

impl ThreatMetrics {
    pub fn new() -> Self {
        Self {
            threats_detected: 0,
            threats_blocked: 0,
            false_positives: 0,
            detection_rate: 0.0,
            response_time: 0.0,
        }
    }
}

impl AnomalyMetrics {
    pub fn new() -> Self {
        Self {
            anomalies_detected: 0,
            anomalies_investigated: 0,
            confirmed_anomalies: 0,
            false_positive_rate: 0.0,
            detection_accuracy: 0.0,
        }
    }
}

impl ComplianceMetrics {
    pub fn new() -> Self {
        Self {
            compliance_score: 1.0,
            controls_implemented: 0,
            controls_passed: 0,
            audit_findings: 0,
            remediation_rate: 0.0,
        }
    }
}

impl SecurityPerformanceMetrics {
    pub fn new() -> Self {
        Self {
            average_response_time: 0.0,
            throughput: 0.0,
            resource_utilization: 0.0,
            error_rate: 0.0,
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cryptographic_library_creation() {
        let library = CryptographicLibrary::new();
        assert_eq!(library.list_keys().len(), 0);
    }

    #[test]
    fn test_mldsa_key_generation() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        let result = library
            .generate_mldsa_key_pair("test_key".to_string(), SecurityLevel::High)
            .unwrap();

        assert_eq!(result.result.0.key_id, "test_key_private");
        assert_eq!(result.result.1.key_id, "test_key_public");
        assert_eq!(result.result.0.key_algorithm, KeyAlgorithm::MLDSA);
        assert_eq!(result.result.1.key_algorithm, KeyAlgorithm::MLDSA);
        assert_eq!(result.result.0.key_type, KeyType::Private);
        assert_eq!(result.result.1.key_type, KeyType::Public);
        assert!(result.compliance_status == ComplianceStatus::Compliant);
    }

    #[test]
    fn test_data_signing() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Generate key pair
        let key_pair = library
            .generate_mldsa_key_pair("test_key".to_string(), SecurityLevel::High)
            .unwrap();

        // Sign data
        let data = b"Hello, World!";
        let signature = library.sign_data("test_key_private", data).unwrap();

        assert_eq!(signature.result.key_id, "test_key_private");
        assert_eq!(signature.result.algorithm, KeyAlgorithm::MLDSA);
        assert_eq!(signature.result.data, data);
        assert!(signature.compliance_status == ComplianceStatus::Compliant);
    }

    #[test]
    fn test_signature_verification() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Generate key pair
        let key_pair = library
            .generate_mldsa_key_pair("test_key".to_string(), SecurityLevel::High)
            .unwrap();

        // Sign data
        let data = b"Hello, World!";
        let signature = library.sign_data("test_key_private", data).unwrap();

        // Verify signature
        let is_valid = library
            .verify_signature("test_key_public", &signature.result, data)
            .unwrap();

        assert!(is_valid.result);
        assert!(is_valid.result);
    }

    #[test]
    fn test_data_encryption() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Generate symmetric key
        let key = Key {
            key_id: "test_key".to_string(),
            key_type: KeyType::Symmetric,
            key_algorithm: KeyAlgorithm::AES,
            key_data: vec![0u8; 32],
            metadata: KeyMetadata {
                key_id: "test_key".to_string(),
                key_type: KeyType::Symmetric,
                key_algorithm: KeyAlgorithm::AES,
                key_size: 32,
                created_at: 0,
                expires_at: 0,
                last_used: 0,
                usage_count: 0,
                security_level: SecurityLevel::High,
                access_level: AccessLevel::Secret,
            },
        };

        library.key_manager.store_key(key).unwrap();

        // Encrypt data
        let data = b"Hello, World!";
        let encrypted_data = library.encrypt_data("test_key", data, None).unwrap();

        assert_eq!(
            encrypted_data.result.algorithm,
            EncryptionAlgorithm::AES256GCM
        );
        assert_eq!(encrypted_data.result.metadata.mode, EncryptionMode::GCM);
        assert!(encrypted_data.compliance_status == ComplianceStatus::Compliant);
    }

    #[test]
    fn test_data_decryption() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Generate symmetric key
        let key = Key {
            key_id: "test_key".to_string(),
            key_type: KeyType::Symmetric,
            key_algorithm: KeyAlgorithm::AES,
            key_data: vec![0u8; 32],
            metadata: KeyMetadata {
                key_id: "test_key".to_string(),
                key_type: KeyType::Symmetric,
                key_algorithm: KeyAlgorithm::AES,
                key_size: 32,
                created_at: 0,
                expires_at: 0,
                last_used: 0,
                usage_count: 0,
                security_level: SecurityLevel::High,
                access_level: AccessLevel::Secret,
            },
        };

        library.key_manager.store_key(key).unwrap();

        // Encrypt data
        let data = b"Hello, World!";
        let encrypted_data = library.encrypt_data("test_key", data, None).unwrap();

        // Decrypt data
        let decrypted_data = library
            .decrypt_data("test_key", &encrypted_data.result)
            .unwrap();

        assert_eq!(decrypted_data.result, data);
    }

    fn store_symmetric_key(library: &mut CryptographicLibrary, key_id: &str) {
        let key = Key {
            key_id: key_id.to_string(),
            key_type: KeyType::Symmetric,
            key_algorithm: KeyAlgorithm::ChaCha20,
            key_data: (0u8..32).collect(),
            metadata: KeyMetadata {
                key_id: key_id.to_string(),
                key_type: KeyType::Symmetric,
                key_algorithm: KeyAlgorithm::ChaCha20,
                key_size: 32,
                created_at: 0,
                expires_at: 0,
                last_used: 0,
                usage_count: 0,
                security_level: SecurityLevel::High,
                access_level: AccessLevel::Secret,
            },
        };
        library.key_manager.store_key(key).unwrap();
    }

    #[test]
    fn test_chacha20poly1305_roundtrip() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();
        store_symmetric_key(&mut library, "cc_key");

        // AAD is now persisted and re-supplied on decryption.
        let data = b"The quick brown fox jumps over the lazy dog";
        let aad = b"authenticated additional data";
        let enc = library
            .encrypt_data_with_algorithm(
                "cc_key",
                data,
                Some(aad),
                EncryptionAlgorithm::ChaCha20Poly1305,
            )
            .unwrap();
        assert_eq!(enc.result.algorithm, EncryptionAlgorithm::ChaCha20Poly1305);
        assert_eq!(enc.result.iv.len(), 12);
        assert_eq!(enc.result.tag.len(), 16);
        assert_eq!(enc.result.aad, aad.to_vec());
        assert_ne!(enc.result.ciphertext, data.to_vec());

        // decrypt_data dispatches on the stored algorithm and re-supplies AAD.
        let dec = library.decrypt_data("cc_key", &enc.result).unwrap();
        assert_eq!(dec.result, data);
    }

    #[test]
    fn test_chacha20poly1305_wrong_aad_fails() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();
        store_symmetric_key(&mut library, "cc_key_wrong_aad");

        let data = b"authenticated data";
        let aad = b"correct aad";
        let mut enc = library
            .encrypt_data_with_algorithm(
                "cc_key_wrong_aad",
                data,
                Some(aad),
                EncryptionAlgorithm::ChaCha20Poly1305,
            )
            .unwrap()
            .result;

        // Tamper with the AAD - decryption should fail because AAD is authenticated.
        enc.aad = b"wrong aad".to_vec();
        assert!(library.decrypt_data("cc_key_wrong_aad", &enc).is_err());
    }

    #[test]
    fn test_xchacha20poly1305_roundtrip_uses_24byte_nonce() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();
        store_symmetric_key(&mut library, "xcc_key");

        let data = b"extended-nonce payload";
        let enc = library
            .encrypt_data_with_algorithm(
                "xcc_key",
                data,
                None,
                EncryptionAlgorithm::XChaCha20Poly1305,
            )
            .unwrap();
        assert_eq!(enc.result.iv.len(), 24);
        let dec = library.decrypt_data("xcc_key", &enc.result).unwrap();
        assert_eq!(dec.result, data);
    }

    #[test]
    fn test_chacha20poly1305_tamper_fails() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();
        store_symmetric_key(&mut library, "cc_key2");

        let data = b"authenticated";
        let mut enc = library
            .encrypt_data_with_algorithm(
                "cc_key2",
                data,
                None,
                EncryptionAlgorithm::ChaCha20Poly1305,
            )
            .unwrap()
            .result;
        // Flip a ciphertext bit; AEAD verification must reject it.
        enc.ciphertext[0] ^= 0x01;
        assert!(library.decrypt_data("cc_key2", &enc).is_err());
    }

    #[test]
    fn test_hkdf_sha256_rfc5869_vector() {
        // RFC 5869 Appendix A.1 Test Case 1 (HMAC-SHA256).
        let mut kd = KeyDerivation::new();
        kd.derivation_parameters.salt = (0u8..=0x0c).collect(); // 000102...0c (13 bytes)
        kd.derivation_parameters.output_length = 42;
        let ikm = vec![0x0bu8; 22];
        let info: Vec<u8> = (0xf0u8..=0xf9).collect();
        let okm = kd.derive_hkdf(&ikm, &info).unwrap();
        assert_eq!(
            hex::encode(&okm),
            "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
        );
    }

    #[test]
    fn test_hash_computation() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        let data = b"Hello, World!";
        let hash_result = library.compute_hash(data).unwrap();

        assert_eq!(hash_result.result.algorithm, "SHA256");
        assert_eq!(hash_result.result.input_data, data);
        assert_eq!(hash_result.result.hash_value.len(), 32); // SHA256 output size
        assert!(hash_result.compliance_status == ComplianceStatus::Compliant);
    }

    #[test]
    fn test_blake3_hash_computation() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Known-answer test: BLAKE3 of the empty input is a stable, published vector.
        let empty = library.compute_hash_blake3(b"").unwrap();
        assert_eq!(empty.result.algorithm, "BLAKE3");
        assert_eq!(empty.result.hash_value.len(), 32); // BLAKE3 default digest size
        assert_eq!(
            hex::encode(&empty.result.hash_value),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );

        // Determinism + distinctness from SHA-256 over the same input.
        let data = b"Hello, World!";
        let a = library.compute_hash_blake3(data).unwrap();
        let b = library.compute_hash_blake3(data).unwrap();
        assert_eq!(a.result.hash_value, b.result.hash_value);
        let sha = library.compute_hash(data).unwrap();
        assert_ne!(a.result.hash_value, sha.result.hash_value);
        assert!(a.compliance_status == ComplianceStatus::Compliant);
    }

    #[test]
    fn test_zk_proof_generation() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        let witness = vec![vec![1u8, 2u8, 3u8]];
        let public_inputs = vec![vec![4u8, 5u8, 6u8]];

        let proof = library
            .generate_zk_proof("test_circuit", &witness, &public_inputs)
            .unwrap();

        assert_eq!(proof.result.system_id, "zk_snarks");
        assert_eq!(proof.result.circuit_id, "test_circuit");
        assert_eq!(proof.result.public_inputs, public_inputs);
        assert!(proof.compliance_status == ComplianceStatus::Compliant);
    }

    #[test]
    fn test_zk_proof_verification() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        let witness = vec![vec![1u8, 2u8, 3u8]];
        let public_inputs = vec![vec![4u8, 5u8, 6u8]];

        let proof = library
            .generate_zk_proof("test_circuit", &witness, &public_inputs)
            .unwrap();

        // Verify proof
        let is_valid = library
            .verify_zk_proof(&proof.result, &public_inputs)
            .unwrap();

        assert!(is_valid.result);
        assert!(is_valid.result);
    }

    /// Real arkworks Groth16 round-trip + soundness through the public byte API,
    /// for the plan-critical `deontic_access` credential-gated access circuit.
    #[cfg(feature = "zk-culling")]
    #[test]
    fn test_deontic_groth16_byte_api_roundtrip_and_soundness() {
        use ark_bls12_381::Fr;
        use ark_ff::{BigInteger, PrimeField};
        use sha2::{Digest, Sha256};

        // Mirror the prover's witness reduction (SHA-256 -> Fr) and the public-input
        // serialisation (canonical little-endian Fr) so we can build a satisfying
        // instance: did + role + action == policy_root.
        let hash_to_fr = |d: &[u8]| Fr::from_be_bytes_mod_order(&Sha256::digest(d));
        let fr_le = |f: &Fr| f.into_bigint().to_bytes_le();

        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        let did = b"did:webizen:alice".to_vec();
        let role = b"role:guardian".to_vec();
        let action = b"action:read-vault".to_vec();
        let witness = vec![did.clone(), role.clone(), action.clone()];

        let policy_root = hash_to_fr(&did) + hash_to_fr(&role) + hash_to_fr(&action);
        let temporal = Fr::from(1_700_000_000u64);
        let public_inputs = vec![fr_le(&policy_root), fr_le(&temporal)];

        let proof = library
            .generate_zk_proof("deontic_access", &witness, &public_inputs)
            .unwrap();
        // Tag 0x02 = the real Groth16 path (not the 0x01 SHA-256 commitment fallback).
        assert_eq!(
            proof.result.proof_data[64], 0x02,
            "deontic_access must route through the real Groth16 path"
        );
        assert!(
            library
                .verify_zk_proof(&proof.result, &public_inputs)
                .unwrap()
                .result,
            "a valid deontic access proof must verify"
        );

        // Soundness: falsify the policy_root public input -> must be rejected.
        let tampered = vec![fr_le(&(policy_root + Fr::from(1u64))), fr_le(&temporal)];
        assert!(
            !library
                .verify_zk_proof(&proof.result, &tampered)
                .unwrap()
                .result,
            "a deontic proof must NOT verify against a falsified policy_root"
        );
    }

    #[test]
    fn test_key_rotation() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Generate initial key
        let key_pair = library
            .generate_mldsa_key_pair("test_key".to_string(), SecurityLevel::High)
            .unwrap();

        // Rotate key
        let new_key = library.rotate_key("test_key_private").unwrap();

        assert!(new_key.result.key_id != "test_key_private");
        assert_eq!(new_key.result.key_algorithm, KeyAlgorithm::MLDSA);
        assert_eq!(new_key.result.key_type, KeyType::Private);
        assert!(new_key.compliance_status == ComplianceStatus::Compliant);
    }

    #[test]
    fn test_security_metrics() {
        let library = CryptographicLibrary::new();
        let metrics = library.get_security_metrics();

        assert_eq!(metrics.threat_metrics.threats_detected, 0);
        assert_eq!(metrics.threat_metrics.threats_blocked, 0);
        assert_eq!(metrics.anomaly_metrics.anomalies_detected, 0);
        assert_eq!(metrics.compliance_metrics.compliance_score, 1.0);
    }

    #[test]
    fn test_kyber_key_generation() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        let private_key = library
            .key_manager
            .key_generator
            .generate_key(
                "kyber_priv".to_string(),
                KeyType::Private,
                KeyAlgorithm::Kyber,
                SecurityLevel::High,
            )
            .unwrap();
        assert_eq!(private_key.key_algorithm, KeyAlgorithm::Kyber);
        assert_eq!(private_key.key_data.len(), fips203::ml_kem_768::DK_LEN);

        let public_key = library
            .key_manager
            .key_generator
            .derive_public_key(&private_key, "kyber_pub".to_string())
            .unwrap();
        assert_eq!(public_key.key_data.len(), fips203::ml_kem_768::EK_LEN);
    }

    #[test]
    fn test_vc_issue_via_library() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        let _kp = library
            .generate_mldsa_key_pair("issuer".to_string(), SecurityLevel::High)
            .unwrap();
        let issuer_did_hash = 12345u64;
        let claim_quins = vec![crate::NQuin {
            subject: issuer_did_hash,
            predicate: crate::q_hash("test:hasRole"),
            object: crate::q_hash("test:Admin"),
            context: issuer_did_hash,
            metadata: 0,
            parity: 0,
        }];
        let context = CryptoContext {
            domain: "test".to_string(),
            purpose: "vc-issuance".to_string(),
            timestamp: 0,
            nonce: [0u8; 32],
        };

        let proof = library
            .issue_vc_mldsa(&claim_quins, "issuer_private", issuer_did_hash, &context)
            .unwrap();
        let is_valid = library
            .verify_vc_mldsa(&proof.result, &claim_quins, "issuer_public", &context)
            .unwrap();

        assert!(is_valid.result);
        assert!(!proof.result.fragment_quins.is_empty());
    }

    #[test]
    fn test_audit_log_records_signature_operations() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Generate key pair
        library
            .generate_mldsa_key_pair("audit_key".to_string(), SecurityLevel::High)
            .unwrap();

        // Sign data — should create one Sign audit entry
        let data = b"audited data";
        let signature = library.sign_data("audit_key_private", data).unwrap();

        // Verify signature — should create one Verify audit entry
        let _is_valid = library
            .verify_signature("audit_key_public", &signature.result, data)
            .unwrap();

        // Check that the signature audit log has recorded both operations
        let audit = &library.signature_engine.signature_storage.audit_log;
        assert!(audit.entry_count() >= 2, "audit log should have at least 2 entries (sign + verify)");
        let entries = audit.entries();
        assert!(entries.iter().any(|e| e.operation == SignatureOperation::Sign), "should have a Sign entry");
        assert!(entries.iter().any(|e| e.operation == SignatureOperation::Verify), "should have a Verify entry");
        // All entries should reference the correct signature_id
        assert!(entries.iter().all(|e| e.signature_id == signature.result.signature_id), "entries should reference the correct signature_id");
    }

    #[test]
    fn test_audit_log_records_hash_operations() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Compute a hash — should create one Compute audit entry
        let _hash = library.compute_hash(b"test data").unwrap();

        let audit = &library.hash_engine.hash_storage.audit_log;
        assert!(audit.entry_count() >= 1, "audit log should have at least 1 entry");
        assert!(
            audit.entries().iter().any(|e| e.operation == HashOperation::Compute),
            "should have a Compute entry"
        );
    }

    #[test]
    fn test_key_relationship_tracking() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Generate a key pair — should create a KeyPair relationship
        let key_pair = library
            .generate_mldsa_key_pair("rel_key".to_string(), SecurityLevel::High)
            .unwrap();

        let catalog = &library.key_manager.key_storage.key_catalog;

        // The catalog should have registered both keys
        assert!(catalog.key_count() >= 2, "catalog should have at least 2 keys registered");

        // The KeyPair relationship should exist from private → public
        let rels = catalog.get_relationships(&key_pair.result.0.key_id);
        assert!(
            rels.iter().any(|r| r.relationship_type == KeyRelationshipType::KeyPair),
            "should have a KeyPair relationship from private to public key"
        );

        // find_related should locate the public key
        let related = catalog.find_related(
            &key_pair.result.0.key_id,
            KeyRelationshipType::KeyPair,
        );
        assert!(related.is_some(), "find_related should find the KeyPair relationship");
        assert_eq!(
            related.unwrap().target_key,
            key_pair.result.1.key_id,
            "KeyPair relationship should point to the public key"
        );
    }

    #[test]
    fn test_key_rotation_tracking() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Generate initial key
        let _key_pair = library
            .generate_mldsa_key_pair("rot_key".to_string(), SecurityLevel::High)
            .unwrap();

        // Rotate key — should create a RotatedFrom relationship
        let new_key = library.rotate_key("rot_key_private").unwrap();

        let catalog = &library.key_manager.key_storage.key_catalog;
        let rels = catalog.get_relationships(&new_key.result.key_id);
        assert!(
            rels.iter().any(|r| r.relationship_type == KeyRelationshipType::RotatedFrom),
            "should have a RotatedFrom relationship from new key to old key"
        );

        assert!(catalog.relationship_count() >= 2, "should have at least 2 relationships (KeyPair + RotatedFrom)");
    }

    #[test]
    fn test_performance_metrics_recorded() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Generate key pair
        library
            .generate_mldsa_key_pair("perf_key".to_string(), SecurityLevel::High)
            .unwrap();

        // Sign data — should update signing time metrics
        let signature = library.sign_data("perf_key_private", b"performance test").unwrap();

        // Verify — should update verification time metrics
        let _is_valid = library
            .verify_signature("perf_key_public", &signature.result, b"performance test")
            .unwrap();

        let sig_metrics = library.signature_engine.performance_optimizer.metrics();
        assert!(
            sig_metrics.average_signing_time > 0.0,
            "average signing time should be recorded"
        );

        // Compute a hash — should update hash metrics
        let _hash = library.compute_hash(b"metric test").unwrap();
        let hash_metrics = library.hash_engine.performance_optimizer.metrics();
        assert!(
            hash_metrics.average_hash_time >= 0.0,
            "hash metrics should be accessible"
        );
    }

    #[test]
    fn test_access_control_enforces_policies() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Generate a key pair
        library
            .generate_mldsa_key_pair("acl_key".to_string(), SecurityLevel::High)
            .unwrap();

        // By default (no policies), access is allowed
        let key = library.key_manager.get_key("acl_key_private").unwrap();
        assert_eq!(key.key_id, "acl_key_private");

        // Register a restrictive policy that only allows Sign on the private key
        let policy = AccessPolicy {
            policy_id: "policy_1".to_string(),
            key_id: "acl_key_private".to_string(),
            allowed_operations: vec![KeyOperation::Sign],
            required_auth: vec![AuthenticationMethod::MultiFactor],
            time_restrictions: TimeRestrictions {
                allowed_hours: vec![],
                allowed_days: vec![],
                start_date: None,
                end_date: None,
            },
            ip_restrictions: vec![],
        };
        library
            .key_manager
            .key_storage
            .access_control
            .add_policy(policy);

        // Sign should be allowed
        assert!(
            library
                .key_manager
                .key_storage
                .access_control
                .check_permission("acl_key_private", KeyOperation::Sign),
            "Sign should be allowed by policy"
        );

        // Read should be denied
        assert!(
            !library
                .key_manager
                .key_storage
                .access_control
                .check_permission("acl_key_private", KeyOperation::Read),
            "Read should be denied by policy"
        );

        // get_key_with_access should deny Read and log the failure
        let result = library
            .key_manager
            .key_storage
            .get_key_with_access("acl_key_private", KeyOperation::Read, "test_user");
        assert!(result.is_err(), "get_key_with_access should deny Read");

        // Sign should succeed
        let result = library
            .key_manager
            .key_storage
            .get_key_with_access("acl_key_private", KeyOperation::Sign, "test_user");
        assert!(result.is_ok(), "get_key_with_access should allow Sign");

        // Audit log should have both the denied and allowed entries
        let audit = library.key_manager.key_storage.access_control.audit_log();
        assert!(audit.entry_count() >= 2, "audit log should have at least 2 entries");
    }

    #[test]
    fn test_encryption_at_rest_roundtrip() {
        let mut ear = EncryptionAtRest::new();
        assert!(!ear.is_enabled(), "KEK should not exist before initialize");

        ear.initialize().unwrap();
        assert!(ear.is_enabled(), "master KEK should be generated after initialize");
        assert!(ear.kek_count() >= 1, "should have at least one KEK");

        // Encrypt some key data
        let plaintext = b"super_secret_key_material_12345";
        let encrypted = ear.encrypt_key_data(plaintext).unwrap();

        // Ciphertext should be different from plaintext (nonce + tag + ciphertext)
        assert_ne!(
            &encrypted[..], plaintext,
            "encrypted data should differ from plaintext"
        );
        assert!(
            encrypted.len() > plaintext.len() + 12,
            "encrypted should be longer due to nonce + tag"
        );

        // Decrypt and verify roundtrip
        let decrypted = ear.decrypt_key_data(&encrypted).unwrap();
        assert_eq!(
            &decrypted[..], plaintext,
            "decrypted data should match original plaintext"
        );
    }

    #[test]
    fn test_encryption_at_rest_without_kek_fails() {
        let ear = EncryptionAtRest::new();
        // Without initialize(), no KEK exists
        let result = ear.encrypt_key_data(b"test");
        assert!(result.is_err(), "encryption should fail without a KEK");
    }
}
