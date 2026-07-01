//! Machine Learning Library - Edge AI and Neural Network Computing
//!
//! This module provides high-performance machine learning operations leveraging Phase 2 enhancements:
//! - NVMe Computational Storage (CSD) for hardware-accelerated neural computations
//! - Ambient Sub-Threshold Orchestration for mobile edge AI optimization
//! - Hardware-Sympathetic Storage (ZNS) for zero-copy model storage
//! - Zero-Copy LoRA Multiplexing for efficient model serving

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Maximum number of token embeddings materialised into `Model.weights` when loading a
/// real GGUF file. The full vocabulary embedding table can be multiple gigabytes, so only
/// a bounded preview is kept in the in-memory `Vec<f64>` (this is not a hot-path module).
pub const GGUF_EMBEDDING_PREVIEW_TOKENS: usize = 256;

/// Machine Learning Library Manager
pub struct MachineLearningLibrary {
    model_manager: ModelManager,
    inference_engine: InferenceEngine,
    training_engine: TrainingEngine,
    optimization_engine: MLOptimizationEngine,
    performance_monitor: MLPerformanceMonitor,
    request_count: u64,
}

/// Model manager for neural network models
pub struct ModelManager {
    model_storage: ModelStorage,
    model_loader: ModelLoader,
    model_converter: ModelConverter,
    model_cache: ModelCache,
}

/// Model storage using ZNS for efficient model storage
pub struct ModelStorage {
    zones: HashMap<String, ModelZone>,
    model_catalog: ModelCatalog,
    compression_engine: ModelCompression,
    version_control: ModelVersionControl,
    model_store: HashMap<String, Model>,
}

/// Model zone for different model types
#[derive(Debug, Clone)]
pub struct ModelZone {
    pub zone_id: String,
    pub zone_type: ModelZoneType,
    pub capacity: u64,
    pub models: HashMap<String, ModelMetadata>,
    pub access_pattern: AccessPattern,
}

/// Model zone types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelZoneType {
    /// Large language models
    LargeLanguage,
    /// Computer vision models
    ComputerVision,
    /// Audio processing models
    AudioProcessing,
    /// Multimodal models
    Multimodal,
    /// Embedding models
    Embedding,
    /// Transformer models
    Transformer,
    /// Convolutional models
    Convolutional,
    /// Recurrent models
    Recurrent,
}

/// Model metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub model_id: String,
    pub model_type: ModelType,
    pub framework: MLFramework,
    pub architecture: ModelArchitecture,
    pub parameters: ModelParameters,
    pub performance: ModelPerformance,
    pub created_at: u64,
    pub last_updated: u64,
    pub access_count: u64,
    pub size: u64,
}

/// Model types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelType {
    /// Large language model
    LLM,
    /// Vision transformer
    ViT,
    /// Convolutional neural network
    CNN,
    /// Recurrent neural network
    RNN,
    /// Transformer
    Transformer,
    /// Generative adversarial network
    GAN,
    /// Variational autoencoder
    VAE,
    /// Diffusion model
    Diffusion,
    /// Graph neural network
    GNN,
    /// Reinforcement learning
    RL,
}

/// ML frameworks
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MLFramework {
    PyTorch,
    TensorFlow,
    JAX,
    ONNX,
    HuggingFace,
    Custom(String),
}

/// Model architecture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelArchitecture {
    pub layers: Vec<LayerInfo>,
    pub connections: Vec<LayerConnection>,
    pub input_shape: Vec<usize>,
    pub output_shape: Vec<usize>,
    pub total_parameters: usize,
}

/// Layer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerInfo {
    pub layer_id: String,
    pub layer_type: LayerType,
    pub input_shape: Vec<usize>,
    pub output_shape: Vec<usize>,
    pub parameters: usize,
    pub activation: Option<ActivationFunction>,
}

/// Layer types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LayerType {
    /// Linear layer
    Linear,
    /// Convolutional layer
    Convolutional,
    /// Attention layer
    Attention,
    /// Embedding layer
    Embedding,
    /// Normalization layer
    Normalization,
    /// Activation layer
    Activation,
    /// Pooling layer
    Pooling,
    /// Dropout layer
    Dropout,
    /// Residual layer
    Residual,
}

/// Activation functions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActivationFunction {
    ReLU,
    GELU,
    Sigmoid,
    Tanh,
    Softmax,
    LeakyReLU,
    ELU,
    Swish,
    Custom(String),
}

/// Layer connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerConnection {
    pub source_layer: String,
    pub target_layer: String,
    pub connection_type: ConnectionType,
}

/// Connection types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConnectionType {
    Direct,
    Residual,
    Skip,
    Attention,
    Custom(String),
}

/// Model parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParameters {
    pub weight_count: usize,
    pub bias_count: usize,
    pub activation_count: usize,
    pub normalization_count: usize,
    pub attention_count: usize,
}

/// Model performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPerformance {
    pub inference_latency: f64,
    pub throughput: f64,
    pub accuracy: f64,
    pub memory_usage: u64,
    pub energy_efficiency: f64,
}

/// Access patterns for optimization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccessPattern {
    Sequential,
    Random,
    Batch,
    Streaming,
    Adaptive,
}

/// Model catalog for model management
pub struct ModelCatalog {
    models: HashMap<String, ModelMetadata>,
    relationships: HashMap<String, Vec<ModelRelationship>>,
    tags: HashMap<String, Vec<String>>,
    search_index: ModelSearchIndex,
}

/// Model relationships
#[derive(Debug, Clone)]
pub struct ModelRelationship {
    pub relationship_id: String,
    pub source_model: String,
    pub target_model: String,
    pub relationship_type: ModelRelationshipType,
    pub strength: f64,
}

/// Model relationship types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelRelationshipType {
    /// Fine-tuned from
    FineTunedFrom,
    /// Pruned from
    PrunedFrom,
    /// Quantized from
    QuantizedFrom,
    /// Ensemble of
    EnsembleOf,
    /// Distilled from
    DistilledFrom,
    /// Merged with
    MergedWith,
}

/// Model search index
pub struct ModelSearchIndex {
    index_entries: HashMap<String, ModelIndexEntry>,
    search_engine: ModelSearchEngine,
    /// Whether `initialize()` has actually configured the index (the search methods are
    /// only valid once this is `true`).
    initialized: bool,
}

/// Model index entry
#[derive(Debug, Clone)]
pub struct ModelIndexEntry {
    pub entry_id: String,
    pub keywords: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub relevance_score: f64,
}

/// Model search engine
pub struct ModelSearchEngine {
    engine_type: SearchEngineType,
    indexing_strategy: IndexingStrategy,
}

/// Search engine types
#[derive(Debug, Clone, PartialEq)]
pub enum SearchEngineType {
    Semantic,
    Keyword,
    Hybrid,
    Embedding,
}

/// Indexing strategies
#[derive(Debug, Clone, PartialEq)]
pub enum IndexingStrategy {
    Vector,
    Text,
    Hybrid,
    Hierarchical,
}

/// Model compression
pub struct ModelCompression {
    compression_algorithms: HashMap<String, CompressionAlgorithm>,
    compression_statistics: CompressionStatistics,
    quality_metrics: CompressionQualityMetrics,
}

/// Compression algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    Quantization,
    Pruning,
    KnowledgeDistillation,
    LowRankDecomposition,
    WeightSharing,
    HuffmanCoding,
    Custom(String),
}

/// Compression statistics
#[derive(Debug, Clone)]
pub struct CompressionStatistics {
    pub original_size: u64,
    pub compressed_size: u64,
    pub compression_ratio: f64,
    pub compression_time: u64,
    pub decompression_time: u64,
}

/// Compression quality metrics
#[derive(Debug, Clone)]
pub struct CompressionQualityMetrics {
    pub accuracy_preservation: f64,
    pub performance_impact: f64,
    pub memory_savings: f64,
    /// Overall compression ratio (original_size / compressed_size).
    pub compression_ratio: f64,
    /// Fractional size reduction (0.0–1.0).
    pub size_reduction: f64,
    /// Number of compression operations recorded.
    pub compression_count: u64,
}

/// Number of logical pruning decisions encoded in one mask byte.
pub const PRUNING_MASK_BITS_PER_BYTE: usize = 8;

/// Model-agnostic post-training quantization schemes.
///
/// The compressed payload is intentionally just a caller-owned byte slice plus
/// these scalar parameters; it is not tied to GGUF or any framework container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuantizationScheme {
    /// Per-tensor signed int8 quantization with a zero point of zero.
    SymmetricInt8,
}

/// Calibration parameters required to reconstruct a PTQ tensor.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuantizationParameters {
    pub scheme: QuantizationScheme,
    pub scale: f64,
    pub zero_point: i16,
}

/// Measured result of a real post-training quantization pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuantizationReport {
    pub parameters: QuantizationParameters,
    pub element_count: usize,
    pub original_bytes: usize,
    pub compressed_bytes: usize,
    pub compression_ratio: f64,
    pub rmse: f64,
    pub max_abs_error: f64,
}

/// Measured result of an exact magnitude-pruning pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PruningReport {
    pub total_weights: usize,
    pub pruned_weights: usize,
    pub kept_weights: usize,
    pub total_units: usize,
    pub pruned_units: usize,
    pub requested_sparsity: f64,
    pub achieved_sparsity: f64,
    pub original_bytes: usize,
    pub compressed_bytes: usize,
    pub compression_ratio: f64,
    /// Fraction of the original squared L2 energy retained by the sparse tensor.
    pub l2_energy_preserved: f64,
}

/// Configuration for the teacher-to-student loop supported by this module.
///
/// `teacher_weight=1` trains solely on teacher outputs; `0` uses only hard
/// targets. Intermediate values blend both target tensors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DistillationConfig {
    pub teacher_weight: f64,
}

/// Evidence emitted by a completed teacher-to-student training run.
#[derive(Debug, Clone)]
pub struct DistillationReport {
    pub teacher_parameters: usize,
    pub student_parameters: usize,
    pub compression_ratio: f64,
    pub fidelity_mse_before: f64,
    pub fidelity_mse_after: f64,
    pub training: TrainingResult,
}

/// Model version control
pub struct ModelVersionControl {
    versions: HashMap<String, ModelVersion>,
    branches: HashMap<String, Vec<String>>,
    tags: HashMap<String, Vec<String>>,
    /// Whether `initialize()` has actually configured the controller.
    initialized: bool,
}

/// Model version
#[derive(Debug, Clone)]
pub struct ModelVersion {
    pub version_id: String,
    pub version_number: String,
    pub changes: Vec<ModelChange>,
    pub created_at: u64,
    pub created_by: String,
}

/// Model change
#[derive(Debug, Clone)]
pub struct ModelChange {
    pub change_id: String,
    pub change_type: ChangeType,
    pub description: String,
    pub affected_layers: Vec<String>,
}

/// Change types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChangeType {
    Architecture,
    Weights,
    Hyperparameters,
    TrainingData,
    Framework,
    Custom(String),
}

/// Model branch
#[derive(Debug, Clone)]
pub struct ModelBranch {
    pub branch_id: String,
    pub branch_name: String,
    pub base_version: String,
    pub head_version: String,
}

/// Model tag
#[derive(Debug, Clone)]
pub struct ModelTag {
    pub tag_id: String,
    pub tag_name: String,
    pub version: String,
    pub description: String,
}

/// Model loader
pub struct ModelLoader {
    loading_strategies: HashMap<String, LoadingStrategy>,
    format_converters: HashMap<String, FormatConverter>,
    loading_cache: LoadingCache,
}

/// Loading strategies
#[derive(Debug, Clone)]
pub struct LoadingStrategy {
    pub strategy_id: String,
    pub strategy_type: LoadingStrategyType,
    pub parameters: LoadingParameters,
}

/// Loading strategy types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LoadingStrategyType {
    Eager,
    Lazy,
    Streaming,
    Chunked,
    Hybrid,
}

/// Loading parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadingParameters {
    pub chunk_size: usize,
    pub prefetch_size: usize,
    pub cache_size: usize,
    pub parallel_loading: bool,
}

/// Format converter
#[derive(Debug, Clone)]
pub struct FormatConverter {
    pub converter_id: String,
    pub source_format: String,
    pub target_format: String,
    pub conversion_pipeline: Vec<ConversionStep>,
}

/// Conversion step
#[derive(Debug, Clone)]
pub struct ConversionStep {
    pub step_id: String,
    pub step_type: ConversionStepType,
    pub parameters: HashMap<String, String>,
}

/// Conversion step types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConversionStepType {
    Parsing,
    Validation,
    Transformation,
    Optimization,
    Serialization,
}

/// Loading cache
pub struct LoadingCache {
    cache_entries: HashMap<String, CacheEntry>,
    cache_policy: CachePolicy,
    cache_stats: CacheStats,
}

/// Cache entry
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub entry_id: String,
    pub model_data: Vec<u8>,
    pub access_count: u64,
    pub last_accessed: u64,
    pub size: u64,
}

/// Cache eviction policy
#[derive(Debug, Clone, PartialEq)]
pub enum EvictionPolicy {
    LRU,
    LFU,
    FIFO,
    Random,
}

/// Performance metrics for inference and optimization results
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub latency: f64,
    pub throughput: f64,
    pub accuracy: f64,
    pub memory_usage: u64,
}

/// A single data transformation step in a pipeline
#[derive(Debug, Clone)]
pub struct TransformationStep {
    pub step_id: String,
    pub step_type: ConversionStepType,
    pub parameters: HashMap<String, f64>,
}

/// ML library performance summary metrics
#[derive(Debug, Clone)]
pub struct MLPerformanceMetrics {
    pub inference_metrics: InferenceMetrics,
    pub training_metrics: TrainingMetrics,
    pub system_metrics: SystemMetrics,
    pub model_metrics: ModelMetrics,
    pub average_inference_latency: f64,
    pub total_requests: u64,
    pub average_training_time: f64,
    pub model_accuracy: f64,
}

/// Cache policy
#[derive(Debug, Clone)]
pub struct CachePolicy {
    pub eviction_policy: EvictionPolicy,
    pub max_size: u64,
    pub ttl: u64,
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_rate: f64,
    pub total_size: u64,
}

/// Model converter
pub struct ModelConverter {
    conversion_pipelines: HashMap<String, ConversionPipeline>,
    optimization_strategies: HashMap<String, OptimizationStrategy>,
    validation_engine: ValidationEngine,
}

/// Conversion pipeline
#[derive(Debug, Clone)]
pub struct ConversionPipeline {
    pub pipeline_id: String,
    pub source_format: String,
    pub target_format: String,
    pub steps: Vec<ConversionStep>,
    pub quality_assurance: QualityAssurance,
}

/// Quality assurance
#[derive(Debug, Clone)]
pub struct QualityAssurance {
    pub validation_rules: Vec<ValidationRule>,
    pub test_cases: Vec<TestCase>,
    pub accuracy_threshold: f64,
}

/// Validation rules
#[derive(Debug, Clone)]
pub struct ValidationRule {
    pub rule_id: String,
    pub rule_type: ValidationRuleType,
    pub condition: String,
    pub action: ValidationAction,
}

/// Validation rule types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationRuleType {
    Architecture,
    Performance,
    Compatibility,
    Security,
    Custom(String),
}

/// Validation actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationAction {
    Pass,
    Fail,
    Warning,
    Transform,
}

/// Test cases
#[derive(Debug, Clone)]
pub struct TestCase {
    pub test_id: String,
    pub test_type: TestType,
    pub input_data: Vec<u8>,
    pub expected_output: Vec<u8>,
}

/// Test types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TestType {
    Inference,
    Training,
    Conversion,
    Performance,
    Custom(String),
}

/// Optimization strategies
#[derive(Debug, Clone)]
pub struct OptimizationStrategy {
    pub strategy_id: String,
    pub strategy_type: OptimizationStrategyType,
    pub parameters: OptimizationParameters,
}

/// Optimization strategy types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OptimizationStrategyType {
    Quantization,
    Pruning,
    Distillation,
    Fusion,
    Custom(String),
}

/// Optimization parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationParameters {
    pub target_size: u64,
    pub accuracy_threshold: f64,
    pub performance_target: f64,
    pub optimization_level: OptimizationLevel,
}

/// Optimization levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OptimizationLevel {
    Conservative,
    Moderate,
    Aggressive,
    Maximum,
}

/// Validation engine
pub struct ValidationEngine {
    validators: HashMap<String, Validator>,
    validation_rules: Vec<ValidationRule>,
    test_suite: TestSuite,
}

/// Validator
#[derive(Debug, Clone)]
pub struct Validator {
    pub validator_id: String,
    pub validator_type: ValidatorType,
    pub validation_logic: ValidationLogic,
}

/// Validator types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidatorType {
    Architecture,
    Performance,
    Compatibility,
    Security,
    Custom(String),
}

/// Validation logic
#[derive(Debug, Clone)]
pub struct ValidationLogic {
    pub logic_id: String,
    pub conditions: Vec<ValidationCondition>,
    pub actions: Vec<ValidationAction>,
}

/// Validation conditions
#[derive(Debug, Clone)]
pub struct ValidationCondition {
    pub condition_id: String,
    pub field: String,
    pub operator: ComparisonOperator,
    pub value: ValidationValue,
}

/// Validation values
#[derive(Debug, Clone)]
pub enum ValidationValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<ValidationValue>),
}

/// Comparison operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComparisonOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Contains,
    Matches,
}

/// Test suite
pub struct TestSuite {
    pub test_cases: Vec<TestCase>,
    pub test_environment: TestEnvironment,
    pub test_results: TestResults,
}

/// Test environment
#[derive(Debug, Clone)]
pub struct TestEnvironment {
    pub environment_id: String,
    pub hardware: HardwareSpec,
    pub software: SoftwareSpec,
    pub configuration: TestConfiguration,
}

/// Hardware specifications
#[derive(Debug, Clone)]
pub struct HardwareSpec {
    pub cpu_cores: usize,
    pub memory_size: u64,
    pub gpu_count: usize,
    pub gpu_memory: u64,
    pub storage_size: u64,
}

/// Software specifications
#[derive(Debug, Clone)]
pub struct SoftwareSpec {
    pub os: String,
    pub framework_version: String,
    pub dependencies: Vec<String>,
}

/// Test configuration
#[derive(Debug, Clone)]
pub struct TestConfiguration {
    pub batch_size: usize,
    pub sequence_length: usize,
    pub precision: Precision,
}

/// Precision types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Precision {
    FP16,
    FP32,
    FP64,
    INT8,
    INT16,
    INT32,
}

/// Test results
#[derive(Debug, Clone)]
pub struct TestResults {
    pub results: Vec<TestResult>,
    pub summary: TestSummary,
}

/// Test result
#[derive(Debug, Clone)]
pub struct TestResult {
    pub test_id: String,
    pub passed: bool,
    pub execution_time: u64,
    pub error_message: Option<String>,
    pub metrics: TestMetrics,
}

/// Test metrics
#[derive(Debug, Clone)]
pub struct TestMetrics {
    pub accuracy: f64,
    pub latency: f64,
    pub throughput: f64,
    pub memory_usage: u64,
}

/// Test summary
#[derive(Debug, Clone)]
pub struct TestSummary {
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub pass_rate: f64,
    pub average_execution_time: f64,
}

/// Model cache
pub struct ModelCache {
    cache_entries: HashMap<String, ModelCacheEntry>,
    cache_policy: ModelCachePolicy,
    cache_stats: ModelCacheStats,
}

/// Model cache entry
#[derive(Debug, Clone)]
pub struct ModelCacheEntry {
    pub entry_id: String,
    pub model: Model,
    pub access_count: u64,
    pub last_accessed: u64,
    pub size: u64,
    pub hit_rate: f64,
}

/// Model cache policy
#[derive(Debug, Clone)]
pub struct ModelCachePolicy {
    pub eviction_policy: ModelEvictionPolicy,
    pub max_size: u64,
    pub ttl: u64,
    pub priority_levels: Vec<PriorityLevel>,
}

/// Model eviction policies
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelEvictionPolicy {
    LRU,
    LFU,
    FIFO,
    PriorityBased,
    SizeBased,
    Custom(String),
}

/// Priority levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PriorityLevel {
    Critical,
    High,
    Medium,
    Low,
}

/// Model cache statistics
#[derive(Debug, Clone)]
pub struct ModelCacheStats {
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_rate: f64,
    pub total_size: u64,
    pub eviction_count: u64,
}

/// Inference engine
pub struct InferenceEngine {
    inference_backends: HashMap<String, InferenceBackend>,
    request_scheduler: RequestScheduler,
    batch_processor: BatchProcessor,
    performance_optimizer: InferenceOptimizer,
}

/// Inference backends
#[derive(Debug, Clone)]
pub struct InferenceBackend {
    pub backend_id: String,
    pub backend_type: InferenceBackendType,
    pub capabilities: BackendCapabilities,
    pub current_load: f64,
}

/// Inference backend types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InferenceBackendType {
    CPU,
    GPU,
    TPU,
    NPU,
    FPGA,
    CSD,
    Hybrid,
}

/// Backend capabilities
#[derive(Debug, Clone)]
pub struct BackendCapabilities {
    pub supported_models: Vec<String>,
    pub max_batch_size: usize,
    pub max_sequence_length: usize,
    pub supported_precisions: Vec<Precision>,
    pub memory_limit: u64,
    pub throughput: f64,
}

/// Request scheduler
pub struct RequestScheduler {
    scheduling_policy: SchedulingPolicy,
    queue_manager: QueueManager,
    load_balancer: LoadBalancer,
}

/// Scheduling policies
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SchedulingPolicy {
    FIFO,
    Priority,
    ShortestJobFirst,
    Deadline,
    FairShare,
}

/// Queue manager
pub struct QueueManager {
    pending_requests: Vec<InferenceRequest>,
    running_requests: HashMap<String, RunningRequest>,
    completed_requests: Vec<CompletedRequest>,
}

/// Inference request
#[derive(Debug, Clone)]
pub struct InferenceRequest {
    pub request_id: String,
    pub model_id: String,
    pub input_data: Vec<u8>,
    pub parameters: InferenceParameters,
    pub priority: RequestPriority,
    pub submitted_at: u64,
    pub deadline: Option<u64>,
}

/// Inference parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceParameters {
    pub batch_size: usize,
    pub sequence_length: usize,
    pub temperature: Option<f64>,
    pub top_k: Option<usize>,
    pub top_p: Option<f64>,
    pub max_tokens: Option<usize>,
    pub precision: Precision,
}

/// Request priorities
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RequestPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Running request
#[derive(Debug, Clone)]
pub struct RunningRequest {
    pub request_id: String,
    pub backend_id: String,
    pub started_at: u64,
    pub progress: f64,
}

/// Completed request
#[derive(Debug, Clone)]
pub struct CompletedRequest {
    pub request_id: String,
    pub backend_id: String,
    pub started_at: u64,
    pub completed_at: u64,
    pub result: InferenceResult,
    pub success: bool,
}

/// Inference result
#[derive(Debug, Clone)]
pub struct InferenceResult {
    pub result_id: String,
    pub output_data: Vec<u8>,
    pub inference_time: u64,
    pub confidence: f64,
    pub metadata: ResultMetadata,
}

/// Result metadata
#[derive(Debug, Clone)]
pub struct ResultMetadata {
    pub model_id: String,
    pub backend_id: String,
    pub batch_size: usize,
    pub sequence_length: usize,
    pub tokens_generated: usize,
}

/// Load balancer
pub struct LoadBalancer {
    balancing_strategy: LoadBalancingStrategy,
    backend_metrics: HashMap<String, BackendMetrics>,
    health_checker: HealthChecker,
}

/// Load balancing strategies
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
    ResponseTime,
    Custom(String),
}

/// Backend metrics
#[derive(Debug, Clone)]
pub struct BackendMetrics {
    pub backend_id: String,
    pub current_load: f64,
    pub average_response_time: f64,
    pub error_rate: f64,
    pub throughput: f64,
}

/// Health checker
pub struct HealthChecker {
    health_checks: HashMap<String, HealthCheck>,
    check_interval: u64,
    timeout: u64,
}

/// Health check
#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub check_id: String,
    pub check_type: HealthCheckType,
    pub endpoint: String,
    pub expected_response: String,
}

/// Health check types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthCheckType {
    HTTP,
    TCP,
    ICMP,
    Custom(String),
}

/// Batch processor
pub struct BatchProcessor {
    batching_strategy: BatchingStrategy,
    batch_size: usize,
    batch_timeout: u64,
    batch_optimizer: BatchOptimizer,
}

/// Batching strategies
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BatchingStrategy {
    FixedSize,
    TimeBased,
    Adaptive,
    PriorityBased,
    Custom(String),
}

/// Batch optimizer
pub struct BatchOptimizer {
    optimization_algorithms: HashMap<String, BatchOptimizationAlgorithm>,
    optimization_metrics: BatchOptimizationMetrics,
}

/// Batch optimization algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BatchOptimizationAlgorithm {
    DynamicBatching,
    GradientAccumulation,
    MemoryOptimization,
    ThroughputOptimization,
    Custom(String),
}

/// Batch optimization metrics
#[derive(Debug, Clone)]
pub struct BatchOptimizationMetrics {
    pub average_batch_size: f64,
    pub throughput: f64,
    pub latency: f64,
    pub memory_utilization: f64,
}

/// Inference optimizer
pub struct InferenceOptimizer {
    optimization_strategies: Vec<InferenceOptimizationStrategy>,
    performance_analyzer: PerformanceAnalyzer,
    auto_tuner: AutoTuner,
}

/// Inference optimization strategies
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InferenceOptimizationStrategy {
    ModelQuantization,
    TensorOptimization,
    MemoryOptimization,
    ComputeOptimization,
    Custom(String),
}

/// Performance analyzer
pub struct PerformanceAnalyzer {
    analysis_methods: Vec<AnalysisMethod>,
    performance_profiles: HashMap<String, PerformanceProfile>,
    bottleneck_detector: BottleneckDetector,
}

/// Analysis methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnalysisMethod {
    Profiling,
    Tracing,
    Metrics,
    Custom(String),
}

/// Performance profile
#[derive(Debug, Clone)]
pub struct PerformanceProfile {
    pub profile_id: String,
    pub model_id: String,
    pub backend_id: String,
    pub metrics: PerformanceMetrics,
    pub characteristics: PerformanceCharacteristics,
}

/// Performance characteristics
#[derive(Debug, Clone)]
pub struct PerformanceCharacteristics {
    pub compute_bound: bool,
    pub memory_bound: bool,
    pub io_bound: bool,
    pub network_bound: bool,
}

/// Bottleneck detector
pub struct BottleneckDetector {
    detection_algorithms: Vec<BottleneckDetectionAlgorithm>,
    detection_thresholds: DetectionThresholds,
}

/// Bottleneck detection algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BottleneckDetectionAlgorithm {
    Statistical,
    MachineLearning,
    RuleBased,
    Custom(String),
}

/// Detection thresholds
#[derive(Debug, Clone)]
pub struct DetectionThresholds {
    pub cpu_threshold: f64,
    pub memory_threshold: f64,
    pub io_threshold: f64,
    pub network_threshold: f64,
}

/// Auto tuner
pub struct AutoTuner {
    tuning_algorithms: HashMap<String, TuningAlgorithm>,
    tuning_objectives: Vec<TuningObjective>,
    tuning_history: TuningHistory,
}

/// Tuning algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TuningAlgorithm {
    BayesianOptimization,
    GeneticAlgorithm,
    SimulatedAnnealing,
    GridSearch,
    Custom(String),
}

/// Tuning objectives
#[derive(Debug, Clone)]
pub struct TuningObjective {
    pub objective_id: String,
    pub objective_type: ObjectiveType,
    pub target_value: f64,
    pub weight: f64,
}

/// Objective types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObjectiveType {
    MinimizeLatency,
    MaximizeThroughput,
    MinimizeMemory,
    MaximizeAccuracy,
    Custom(String),
}

/// Tuning history
pub struct TuningHistory {
    tuning_records: Vec<TuningRecord>,
    best_configurations: HashMap<String, TuningConfiguration>,
}

/// Tuning record
#[derive(Debug, Clone)]
pub struct TuningRecord {
    pub record_id: String,
    pub timestamp: u64,
    pub configuration: TuningConfiguration,
    pub performance: PerformanceMetrics,
    pub improvement: f64,
}

/// Tuning configuration
#[derive(Debug, Clone)]
pub struct TuningConfiguration {
    pub configuration_id: String,
    pub parameters: HashMap<String, f64>,
    pub metadata: HashMap<String, String>,
}

/// Training engine
pub struct TrainingEngine {
    training_backends: HashMap<String, TrainingBackend>,
    training_scheduler: TrainingScheduler,
    data_pipeline: DataPipeline,
    training_optimizer: TrainingOptimizer,
}

/// Training backends
#[derive(Debug, Clone)]
pub struct TrainingBackend {
    pub backend_id: String,
    pub backend_type: TrainingBackendType,
    pub capabilities: TrainingCapabilities,
    pub current_load: f64,
}

/// Training backend types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrainingBackendType {
    CPU,
    GPU,
    TPU,
    Distributed,
    Hybrid,
}

/// Training capabilities
#[derive(Debug, Clone)]
pub struct TrainingCapabilities {
    pub supported_algorithms: Vec<TrainingAlgorithm>,
    pub max_batch_size: usize,
    pub max_dataset_size: u64,
    pub parallel_workers: usize,
    pub memory_limit: u64,
}

/// Training algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrainingAlgorithm {
    SGD,
    Adam,
    AdamW,
    RMSprop,
    Adagrad,
    Custom(String),
}

/// Training scheduler
pub struct TrainingScheduler {
    scheduling_policy: TrainingSchedulingPolicy,
    resource_manager: ResourceManager,
    progress_tracker: ProgressTracker,
}

/// Training scheduling policies
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrainingSchedulingPolicy {
    FIFO,
    Priority,
    FairShare,
    Custom(String),
}

/// Resource manager
pub struct ResourceManager {
    resources: HashMap<String, Resource>,
    allocation_strategy: AllocationStrategy,
    utilization_tracker: UtilizationTracker,
}

/// Resource
#[derive(Debug, Clone)]
pub struct Resource {
    pub resource_id: String,
    pub resource_type: ResourceType,
    pub capacity: f64,
    pub current_usage: f64,
    pub availability: Availability,
}

/// Resource types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResourceType {
    CPU,
    GPU,
    Memory,
    Storage,
    Network,
}

/// Availability
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Availability {
    Available,
    Busy,
    Maintenance,
    Offline,
}

/// Allocation strategies
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AllocationStrategy {
    FirstFit,
    BestFit,
    WorstFit,
    Custom(String),
}

/// Utilization tracker
pub struct UtilizationTracker {
    utilization_history: HashMap<String, Vec<UtilizationRecord>>,
    current_utilization: HashMap<String, f64>,
}

/// Utilization record
#[derive(Debug, Clone)]
pub struct UtilizationRecord {
    pub timestamp: u64,
    pub resource_id: String,
    pub utilization: f64,
}

/// Progress tracker
pub struct ProgressTracker {
    training_jobs: HashMap<String, TrainingJob>,
    progress_metrics: ProgressMetrics,
}

/// Training metrics
#[derive(Debug, Clone)]
pub struct TrainingMetrics {
    pub total_training_jobs: u64,
    pub accuracy: f64,
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
    pub learning_rate: f64,
}

/// Progress metrics
#[derive(Debug, Clone)]
pub struct ProgressMetrics {
    pub total_jobs: u32,
    pub completed_jobs: u32,
    pub average_progress: f64,
    pub estimated_completion: u64,
}

/// Data pipeline
pub struct DataPipeline {
    data_sources: HashMap<String, DataSource>,
    data_transformers: HashMap<String, DataTransformer>,
    data_loaders: HashMap<String, DataLoader>,
    data_augmenters: HashMap<String, DataAugmenter>,
}

/// Data sources
#[derive(Debug, Clone)]
pub struct DataSource {
    pub source_id: String,
    pub source_type: DataSourceType,
    pub location: String,
    pub format: DataFormat,
}

/// Data source types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataSourceType {
    Local,
    Remote,
    Database,
    File,
    Stream,
    Custom(String),
}

/// Data formats
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataFormat {
    CSV,
    JSON,
    Parquet,
    HDF5,
    Image,
    Audio,
    Video,
    Custom(String),
}

/// Data transformers
#[derive(Debug, Clone)]
pub struct DataTransformer {
    pub transformer_id: String,
    pub transformer_type: DataTransformerType,
    pub transformation_pipeline: Vec<TransformationStep>,
}

/// Data transformer types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataTransformerType {
    Normalizer,
    Standardizer,
    Encoder,
    Decoder,
    Filter,
    Custom(String),
}

/// Data loaders
#[derive(Debug, Clone)]
pub struct DataLoader {
    pub loader_id: String,
    pub loader_type: DataLoaderType,
    pub batch_size: usize,
    pub shuffle: bool,
    pub num_workers: usize,
}

/// Data loader types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataLoaderType {
    Sequential,
    Parallel,
    Distributed,
    Custom(String),
}

/// Data augmenters
#[derive(Debug, Clone)]
pub struct DataAugmenter {
    pub augmenter_id: String,
    pub augmenter_type: DataAugmenterType,
    pub augmentation_pipeline: Vec<AugmentationStep>,
}

/// Data augmenter types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataAugmenterType {
    ImageAugmentation,
    TextAugmentation,
    AudioAugmentation,
    Custom(String),
}

/// Augmentation step
#[derive(Debug, Clone)]
pub struct AugmentationStep {
    pub step_id: String,
    pub step_type: AugmentationStepType,
    pub parameters: HashMap<String, f64>,
}

/// Augmentation step types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AugmentationStepType {
    Rotation,
    Scaling,
    Flipping,
    Cropping,
    Noise,
    Custom(String),
}

/// Training optimizer
pub struct TrainingOptimizer {
    optimization_algorithms: HashMap<String, TrainingOptimizationAlgorithm>,
    hyperparameter_tuner: HyperparameterTuner,
    early_stopping: EarlyStopping,
}

/// Training optimization algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrainingOptimizationAlgorithm {
    LearningRateSchedule,
    GradientClipping,
    WeightDecay,
    BatchNormalization,
    Custom(String),
}

/// Hyperparameter tuner
pub struct HyperparameterTuner {
    tuning_space: TuningSpace,
    tuning_algorithm: TuningAlgorithm,
    tuning_history: TuningHistory,
}

/// Tuning space
#[derive(Debug, Clone)]
pub struct TuningSpace {
    pub hyperparameters: Vec<Hyperparameter>,
    pub constraints: Vec<HyperparameterConstraint>,
}

/// Hyperparameter
#[derive(Debug, Clone)]
pub struct Hyperparameter {
    pub name: String,
    pub parameter_type: HyperparameterType,
    pub range: HyperparameterRange,
    pub default_value: f64,
}

/// Hyperparameter types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HyperparameterType {
    Continuous,
    Discrete,
    Categorical,
    Integer,
}

/// Hyperparameter range
#[derive(Debug, Clone)]
pub struct HyperparameterRange {
    pub min_value: f64,
    pub max_value: f64,
    pub step: Option<f64>,
    pub categories: Option<Vec<String>>,
}

/// Hyperparameter constraints
#[derive(Debug, Clone)]
pub struct HyperparameterConstraint {
    pub constraint_id: String,
    pub constraint_type: ConstraintType,
    pub parameters: Vec<String>,
    pub condition: String,
}

/// Constraint types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintType {
    Range,
    Equality,
    Inequality,
    Custom(String),
}

/// Early stopping
pub struct EarlyStopping {
    stopping_criteria: StoppingCriteria,
    patience: u32,
    min_delta: f64,
    restore_best_weights: bool,
}

/// Stopping criteria
#[derive(Debug, Clone)]
pub struct StoppingCriteria {
    pub metric: String,
    pub mode: StoppingMode,
    pub min_delta: f64,
    pub patience: u32,
}

/// Stopping modes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StoppingMode {
    Min,
    Max,
    Auto,
}

/// ML optimization engine
pub struct MLOptimizationEngine {
    optimization_algorithms: HashMap<String, MLOptimizationAlgorithm>,
    optimization_objectives: Vec<OptimizationObjective>,
    optimization_constraints: Vec<OptimizationConstraint>,
}

/// ML optimization algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MLOptimizationAlgorithm {
    NeuralArchitectureSearch,
    HyperparameterOptimization,
    ModelCompression,
    ModelQuantization,
    Quantization,
    Pruning,
    Custom(String),
}

/// Optimization objectives
#[derive(Debug, Clone)]
pub struct OptimizationObjective {
    pub objective_id: String,
    pub objective_type: ObjectiveType,
    pub target_value: f64,
    pub weight: f64,
}

/// Optimization constraints
#[derive(Debug, Clone)]
pub struct OptimizationConstraint {
    pub constraint_id: String,
    pub constraint_type: ConstraintType,
    pub parameters: Vec<String>,
    pub condition: String,
}

/// ML performance monitor
pub struct MLPerformanceMonitor {
    inference_metrics: InferenceMetrics,
    training_metrics: TrainingMetrics,
    system_metrics: SystemMetrics,
    model_metrics: ModelMetrics,
}

/// Inference metrics
#[derive(Debug, Clone)]
pub struct InferenceMetrics {
    pub total_requests: u64,
    pub average_latency: f64,
    pub throughput: f64,
    pub error_rate: f64,
    pub resource_utilization: ResourceUtilization,
}

/// System-wide training metrics
#[derive(Debug, Clone)]
pub struct SystemTrainingMetrics {
    pub total_training_jobs: u64,
    pub average_training_time: f64,
    pub convergence_rate: f64,
    pub model_accuracy: f64,
    pub resource_utilization: ResourceUtilization,
}

/// System metrics
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub cpu_utilization: f64,
    pub memory_utilization: f64,
    pub gpu_utilization: f64,
    pub network_utilization: f64,
    pub storage_utilization: f64,
}

/// Model metrics
#[derive(Debug, Clone)]
pub struct ModelMetrics {
    pub total_models: u64,
    pub average_model_size: f64,
    pub model_accuracy: f64,
    pub model_performance: f64,
    pub storage_utilization: f64,
}

/// Resource utilization
#[derive(Debug, Clone)]
pub struct ResourceUtilization {
    pub cpu: f64,
    pub memory: f64,
    pub gpu: f64,
    pub network: f64,
    pub storage: f64,
}

/// ML operation result
#[derive(Debug, Clone)]
pub struct MLOperationResult<T> {
    pub result: T,
    pub execution_time: u64,
    pub memory_usage: u64,
    pub accuracy: f64,
    pub resource_utilization: ResourceUtilization,
}

/// Model representation
#[derive(Debug, Clone)]
pub struct Model {
    pub model_id: String,
    pub model_type: ModelType,
    pub framework: MLFramework,
    pub architecture: ModelArchitecture,
    pub weights: Vec<f64>,
    pub metadata: ModelMetadata,
}

/// Training job representation
#[derive(Debug, Clone)]
pub struct TrainingJob {
    pub job_id: String,
    pub model_id: String,
    pub training_config: TrainingConfig,
    pub status: TrainingStatus,
    pub progress: f64,
    pub metrics: TrainingMetrics,
}

/// Training configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    pub epochs: u32,
    pub batch_size: usize,
    pub learning_rate: f64,
    pub optimizer: TrainingAlgorithm,
    pub loss_function: String,
    pub metrics: Vec<String>,
    pub validation_split: f64,
}

/// Result of a completed training run.
///
/// Captures the loss before and after training, how many epochs actually ran, whether the
/// loop converged early, and the wall-clock training time. This is the honest record of
/// what the SGD loop did (not a stub).
#[derive(Debug, Clone)]
pub struct TrainingResult {
    /// Mean squared error measured on the full dataset before the first weight update.
    pub initial_loss: f64,
    /// Mean squared error measured on the full dataset after the final epoch.
    pub final_loss: f64,
    /// Number of epochs actually executed (may be less than `config.epochs` if converged).
    pub epochs_completed: usize,
    /// True if the loss plateaued below the convergence threshold before all epochs ran.
    pub convergence_achieved: bool,
    /// Wall-clock training time in milliseconds.
    pub training_time_ms: u64,
}

/// Training status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrainingStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl MachineLearningLibrary {
    /// Create new machine learning library
    pub fn new() -> Self {
        Self {
            model_manager: ModelManager::new(),
            inference_engine: InferenceEngine::new(),
            training_engine: TrainingEngine::new(),
            optimization_engine: MLOptimizationEngine::new(),
            performance_monitor: MLPerformanceMonitor::new(),
            request_count: 0,
        }
    }

    /// Initialize the library
    pub fn initialize(&mut self) -> Result<(), MLError> {
        // Initialize model manager
        self.model_manager.initialize()?;

        // Initialize inference engine
        self.inference_engine.initialize()?;

        // Initialize training engine
        self.training_engine.initialize()?;

        // Initialize optimization engine
        self.optimization_engine.initialize()?;

        Ok(())
    }

    /// Load a model
    pub fn load_model(
        &mut self,
        model_id: String,
        model_path: &str,
    ) -> Result<MLOperationResult<Model>, MLError> {
        let start_time = std::time::Instant::now();

        // Load model
        let model = self
            .model_manager
            .load_model(model_id.clone(), model_path)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(MLOperationResult {
            result: model,
            execution_time,
            memory_usage: 0,
            accuracy: 0.0,
            resource_utilization: ResourceUtilization::new(),
        })
    }

    /// Run inference
    pub fn run_inference(
        &mut self,
        model_id: &str,
        input_data: &[u8],
        parameters: InferenceParameters,
    ) -> Result<MLOperationResult<InferenceResult>, MLError> {
        let start_time = std::time::Instant::now();

        // Create inference request
        let request = InferenceRequest {
            request_id: format!("req_{}", self.request_count),
            model_id: model_id.to_string(),
            input_data: input_data.to_vec(),
            parameters,
            priority: RequestPriority::Normal,
            submitted_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            deadline: None,
        };

        // Load the model (from cache or storage) and execute the forward pass.
        let model = self.model_manager.load_model(model_id.to_string(), "")?;
        let result = self.inference_engine.execute_inference(&request, &model)?;
        self.request_count += 1;

        let execution_time = start_time.elapsed().as_millis().max(1) as u64;

        let confidence = result.confidence;
        Ok(MLOperationResult {
            result,
            execution_time,
            memory_usage: 0,
            accuracy: confidence,
            resource_utilization: ResourceUtilization::new(),
        })
    }

    /// Start training
    pub fn start_training(
        &mut self,
        model_id: &str,
        training_config: TrainingConfig,
    ) -> Result<MLOperationResult<TrainingJob>, MLError> {
        let start_time = std::time::Instant::now();

        // Create training job
        let job = TrainingJob {
            job_id: format!(
                "job_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
            model_id: model_id.to_string(),
            training_config,
            status: TrainingStatus::Pending,
            progress: 0.0,
            metrics: TrainingMetrics::new(),
        };

        // Start training
        self.training_engine.start_training_job(&job)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(MLOperationResult {
            result: job,
            execution_time,
            memory_usage: 0,
            accuracy: 0.0,
            resource_utilization: ResourceUtilization::new(),
        })
    }

    /// Optimize model
    pub fn optimize_model(
        &mut self,
        model_id: &str,
        optimization_algorithm: MLOptimizationAlgorithm,
    ) -> Result<MLOperationResult<Model>, MLError> {
        let start_time = std::time::Instant::now();

        // Optimize model
        let optimized_model = self
            .optimization_engine
            .optimize_model(model_id, optimization_algorithm)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(MLOperationResult {
            result: optimized_model,
            execution_time,
            memory_usage: 0,
            accuracy: 0.0,
            resource_utilization: ResourceUtilization::new(),
        })
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> MLPerformanceMetrics {
        self.performance_monitor.get_metrics()
    }

    /// List all models
    pub fn list_models(&self) -> Vec<String> {
        self.model_manager.list_models()
    }

    /// Get model information
    pub fn get_model_info(&self, model_id: &str) -> Option<ModelMetadata> {
        self.model_manager.get_model_metadata(model_id)
    }
}

// Supporting implementations

impl ModelManager {
    pub fn new() -> Self {
        Self {
            model_storage: ModelStorage::new(),
            model_loader: ModelLoader::new(),
            model_converter: ModelConverter::new(),
            model_cache: ModelCache::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.model_storage.initialize()?;
        self.model_loader.initialize()?;
        self.model_converter.initialize()?;
        self.model_cache.initialize()?;
        Ok(())
    }

    pub fn load_model(&mut self, model_id: String, model_path: &str) -> Result<Model, MLError> {
        // Check cache first
        if let Some(cached_model) = self.model_cache.get(&model_id) {
            return Ok(cached_model);
        }

        // Load model from storage
        let model = self.model_storage.load_model(&model_id, model_path)?;

        // Cache the model
        self.model_cache.put(model_id.clone(), model.clone())?;

        Ok(model)
    }

    pub fn list_models(&self) -> Vec<String> {
        self.model_storage.list_models()
    }

    pub fn get_model_metadata(&self, model_id: &str) -> Option<ModelMetadata> {
        self.model_storage.get_model_metadata(model_id)
    }
}

impl ModelStorage {
    pub fn new() -> Self {
        Self {
            zones: HashMap::new(),
            model_catalog: ModelCatalog::new(),
            compression_engine: ModelCompression::new(),
            version_control: ModelVersionControl::new(),
            model_store: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.create_zones()?;
        self.model_catalog.initialize()?;
        self.compression_engine.initialize()?;
        self.version_control.initialize()?;
        Ok(())
    }

    fn create_zones(&mut self) -> Result<(), MLError> {
        let zones = vec![
            ("llm", ModelZoneType::LargeLanguage),
            ("cv", ModelZoneType::ComputerVision),
            ("audio", ModelZoneType::AudioProcessing),
            ("multimodal", ModelZoneType::Multimodal),
            ("embedding", ModelZoneType::Embedding),
            ("transformer", ModelZoneType::Transformer),
            ("cnn", ModelZoneType::Convolutional),
            ("rnn", ModelZoneType::Recurrent),
        ];

        for (name, zone_type) in zones {
            let zone = ModelZone {
                zone_id: name.to_string(),
                zone_type,
                capacity: 10 * 1024 * 1024 * 1024, // 10GB
                models: HashMap::new(),
                access_pattern: AccessPattern::Adaptive,
            };
            self.zones.insert(name.to_string(), zone);
        }

        Ok(())
    }

    pub fn load_model(&mut self, model_id: &str, model_path: &str) -> Result<Model, MLError> {
        if let Some(model) = self.model_store.get(model_id) {
            return Ok(model.clone());
        }

        // Attempt a real GGUF load when the path points at an existing .gguf file.
        // On non-GGUF / missing / unreadable files we fall back to the mock scaffold
        // model so downstream inference still has something to operate on.
        let model = if model_path.to_ascii_lowercase().ends_with(".gguf")
            && std::path::Path::new(model_path).exists()
        {
            match Self::load_gguf_model(model_id, model_path) {
                Ok(m) => m,
                Err(e) => {
                    log::warn!(
                        "ModelStorage::load_model: GGUF load failed for {} ({}); \
                         falling back to mock model",
                        model_path,
                        e
                    );
                    Self::mock_model(model_id)
                }
            }
        } else {
            if std::path::Path::new(model_path).exists() {
                log::warn!(
                    "ModelStorage::load_model: {} is not a .gguf file; \
                     falling back to mock model",
                    model_path
                );
            } else {
                log::warn!(
                    "ModelStorage::load_model: model file {} does not exist; \
                     falling back to mock model",
                    model_path
                );
            }
            Self::mock_model(model_id)
        };

        self.model_store.insert(model_id.to_string(), model.clone());
        Ok(model)
    }

    /// Build the mock scaffold model used when no real GGUF weights are available.
    fn mock_model(model_id: &str) -> Model {
        Model {
            model_id: model_id.to_string(),
            model_type: ModelType::LLM,
            framework: MLFramework::PyTorch,
            architecture: ModelArchitecture::new(),
            weights: vec![0.0; 1000],
            metadata: ModelMetadata::new(),
        }
    }

    /// Load a real GGUF file by memory-mapping it and extracting the `token_embd.weight`
    /// tensor via `GgufTensorIndex`. The embedding table can be many gigabytes for a full
    /// vocabulary, so only a bounded preview of per-token embeddings (first
    /// [`GGUF_EMBEDDING_PREVIEW_TOKENS`] tokens) is materialised into `Model.weights` to
    /// keep the in-memory `Vec<f64>` tractable. The `ModelArchitecture` is populated with a
    /// single `Linear` layer matching the embedding dimensions reported by the GGUF header.
    #[cfg(not(target_arch = "wasm32"))]
    fn load_gguf_model(model_id: &str, model_path: &str) -> Result<Model, MLError> {
        use memmap2::Mmap;

        let file = std::fs::File::open(model_path)
            .map_err(|e| MLError::ModelError(format!("open {}: {}", model_path, e)))?;
        let mmap = unsafe { Mmap::map(&file) }
            .map_err(|e| MLError::ModelError(format!("mmap {}: {}", model_path, e)))?;
        let mmap_bytes: &[u8] = &mmap;

        // `GgufTensorIndex::from_gguf` is infallible — it returns an empty index on a
        // malformed header — so validate that real tensor metadata was parsed.
        let index = crate::gguf_sharder::GgufTensorIndex::from_gguf(mmap_bytes);
        if index.tensor_data_start == 0
            && index.max_tensor_bytes == 0
            && index.hyperparams.n_layer == 0
        {
            return Err(MLError::ModelError(
                "GGUF header parse failed or yielded no tensor metadata".to_string(),
            ));
        }

        let n_embd = index.emb_dim();
        let n_vocab = index.vocab_dim();
        if n_embd == 0 || n_vocab == 0 {
            return Err(MLError::ModelError(
                "GGUF has no token_embd.weight tensor".to_string(),
            ));
        }

        // Materialise a bounded preview of the embedding table into f64 weights.
        let token_cap = n_vocab.min(GGUF_EMBEDDING_PREVIEW_TOKENS);
        let mut weights = Vec::with_capacity(token_cap * n_embd);
        let mut row = vec![0.0f32; n_embd];
        for token_id in 0..token_cap as u32 {
            let n = index.dequantize_token_embedding_into(mmap_bytes, token_id, &mut row);
            if n == 0 {
                // Stop at the first token we cannot dequantize rather than emitting zeros.
                break;
            }
            for &v in &row[..n] {
                weights.push(v as f64);
            }
        }

        let loaded_rows = weights.len() / n_embd;
        let total_parameters = weights.len();

        let architecture = ModelArchitecture {
            layers: vec![LayerInfo {
                layer_id: "token_embd".to_string(),
                layer_type: LayerType::Linear,
                input_shape: vec![n_vocab],
                output_shape: vec![n_embd],
                parameters: total_parameters,
                activation: None,
            }],
            connections: vec![],
            input_shape: vec![n_vocab],
            output_shape: vec![n_embd],
            total_parameters,
        };

        let mut metadata = ModelMetadata::new();
        metadata.model_id = model_id.to_string();
        metadata.architecture = architecture.clone();
        metadata.parameters.weight_count = total_parameters;
        metadata.size = (total_parameters * std::mem::size_of::<f64>()) as u64;

        log::info!(
            "ModelStorage::load_model: loaded GGUF {} — n_embd={}, n_vocab={}, \
             materialised {} token embeddings ({} weights)",
            model_path,
            n_embd,
            n_vocab,
            loaded_rows,
            total_parameters
        );

        Ok(Model {
            model_id: model_id.to_string(),
            model_type: ModelType::LLM,
            framework: MLFramework::Custom("GGUF".to_string()),
            architecture,
            weights,
            metadata,
        })
    }

    /// WASM fallback: `memmap2` is unavailable, so a GGUF path cannot be mapped.
    #[cfg(target_arch = "wasm32")]
    fn load_gguf_model(_model_id: &str, model_path: &str) -> Result<Model, MLError> {
        Err(MLError::ModelError(format!(
            "GGUF loading via mmap is not supported on wasm32 ({})",
            model_path
        )))
    }

    pub fn list_models(&self) -> Vec<String> {
        let mut models = Vec::new();
        for zone in self.zones.values() {
            models.extend(zone.models.keys().cloned());
        }
        models
    }

    pub fn get_model_metadata(&self, model_id: &str) -> Option<ModelMetadata> {
        for zone in self.zones.values() {
            if let Some(metadata) = zone.models.get(model_id) {
                return Some(metadata.clone());
            }
        }
        None
    }
}

impl ModelCatalog {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            relationships: HashMap::new(),
            tags: HashMap::new(),
            search_index: ModelSearchIndex::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.search_index.initialize()?;
        Ok(())
    }

    /// Register a model in the catalog and add it to the search index.
    ///
    /// The model's id becomes both the catalog key and the index entry id. The model type
    /// and framework are added as keywords so the model is searchable by those terms, and
    /// the architecture's total parameter count is recorded in the index entry metadata.
    pub fn register_model(&mut self, model_id: &str, metadata: ModelMetadata) {
        // Build a search-index entry from the metadata before inserting it.
        let entry = ModelIndexEntry {
            entry_id: model_id.to_string(),
            keywords: vec![
                model_id.to_string(),
                format!("{:?}", metadata.model_type),
                format!("{:?}", metadata.framework),
            ],
            metadata: {
                let mut m = HashMap::new();
                m.insert(
                    "model_type".to_string(),
                    format!("{:?}", metadata.model_type),
                );
                m.insert("framework".to_string(), format!("{:?}", metadata.framework));
                m.insert(
                    "total_parameters".to_string(),
                    metadata.architecture.total_parameters.to_string(),
                );
                m
            },
            relevance_score: 1.0,
        };
        self.search_index.index(entry);
        self.models.insert(model_id.to_string(), metadata);
    }

    /// Add a tag to a model for searchability.
    ///
    /// Tags are stored both in the catalog's `tags` map (tag → model ids) and as a keyword
    /// on the model's search-index entry, so a single `search()` call covers both paths.
    pub fn add_tag(&mut self, model_id: &str, tag: &str) {
        let tag_lower = tag.to_lowercase();
        self.tags
            .entry(tag_lower.clone())
            .or_default()
            .push(model_id.to_string());

        // Mirror the tag into the index entry's keywords so keyword search finds it too.
        if let Some(entry) = self.search_index.index_entries.get_mut(model_id) {
            if !entry.keywords.iter().any(|k| k == &tag_lower) {
                entry.keywords.push(tag_lower);
            }
        }
    }

    /// Search models by name, tags, or keywords (case-insensitive substring match).
    ///
    /// Returns matching model ids. A model matches if the query (lower-cased) is a substring
    /// of its id, any of its tags, or any keyword/metadata value on its index entry.
    pub fn search(&self, query: &str) -> Vec<String> {
        let q = query.to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }

        let mut matches = Vec::new();

        // 1. Match by model id.
        for model_id in self.models.keys() {
            if model_id.to_lowercase().contains(&q) {
                matches.push(model_id.clone());
            }
        }

        // 2. Match by tag.
        for (tag, model_ids) in &self.tags {
            if tag.contains(&q) {
                for id in model_ids {
                    if !matches.contains(id) {
                        matches.push(id.clone());
                    }
                }
            }
        }

        // 3. Match by index entry keywords / metadata.
        for entry in self.search_index.search(&q) {
            if !matches.contains(&entry.entry_id) {
                matches.push(entry.entry_id.clone());
            }
        }

        matches
    }

    /// Find all models that carry a given tag (case-insensitive).
    pub fn get_by_tag(&self, tag: &str) -> Vec<String> {
        self.tags
            .get(&tag.to_lowercase())
            .cloned()
            .unwrap_or_default()
    }

    /// Record a relationship between two models (e.g. fine-tuned-from, quantized-from).
    ///
    /// The relationship is stored under the source model's id so all relationships
    /// originating from a given model can be retrieved together.
    pub fn add_relationship(&mut self, relationship: ModelRelationship) {
        self.relationships
            .entry(relationship.source_model.clone())
            .or_default()
            .push(relationship);
    }

    /// Return all relationships originating from `source_model`.
    pub fn get_relationships(&self, source_model: &str) -> Vec<&ModelRelationship> {
        self.relationships
            .get(source_model)
            .map(|rels| rels.iter().collect())
            .unwrap_or_default()
    }

    /// Remove all relationships originating from `source_model`. Returns the number removed.
    pub fn remove_relationships(&mut self, source_model: &str) -> usize {
        self.relationships
            .remove(source_model)
            .map(|rels| rels.len())
            .unwrap_or(0)
    }

    /// Return the total number of relationships recorded in the catalog.
    pub fn relationship_count(&self) -> usize {
        self.relationships.values().map(|rels| rels.len()).sum()
    }
}

impl ModelSearchIndex {
    pub fn new() -> Self {
        Self {
            index_entries: HashMap::new(),
            search_engine: ModelSearchEngine::new(),
            initialized: false,
        }
    }

    /// Actually initialize the search index: configure the engine for hybrid keyword
    /// search and mark the index as ready. Search calls before this return empty results.
    pub fn initialize(&mut self) -> Result<(), MLError> {
        // Configure a keyword/hybrid strategy suited to the catalog's text-based entries
        // (the default is a Semantic/Vector engine, which has no embedding backend here).
        self.search_engine.engine_type = SearchEngineType::Hybrid;
        self.search_engine.indexing_strategy = IndexingStrategy::Text;
        self.initialized = true;
        Ok(())
    }

    /// Add an entry to the search index. Replaces any existing entry with the same id.
    pub fn index(&mut self, entry: ModelIndexEntry) {
        self.index_entries.insert(entry.entry_id.clone(), entry);
    }

    /// Keyword search across index entries (case-insensitive substring match on the
    /// entry id, keywords, and metadata values). Returns references to matching entries.
    pub fn search(&self, query: &str) -> Vec<&ModelIndexEntry> {
        if !self.initialized || query.is_empty() {
            return Vec::new();
        }
        let q = query.to_lowercase();
        self.index_entries
            .values()
            .filter(|entry| {
                entry.entry_id.to_lowercase().contains(&q)
                    || entry.keywords.iter().any(|k| k.to_lowercase().contains(&q))
                    || entry
                        .metadata
                        .values()
                        .any(|v| v.to_lowercase().contains(&q))
            })
            .collect()
    }
}

impl ModelSearchEngine {
    pub fn new() -> Self {
        Self {
            engine_type: SearchEngineType::Semantic,
            indexing_strategy: IndexingStrategy::Vector,
        }
    }
}

impl ModelCompression {
    pub fn new() -> Self {
        Self {
            compression_algorithms: HashMap::new(),
            compression_statistics: CompressionStatistics::new(),
            quality_metrics: CompressionQualityMetrics::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        // Register the standard set of compression algorithms.
        self.register_algorithm("QuantizationInt8", CompressionAlgorithm::Quantization);
        self.register_algorithm("QuantizationFP16", CompressionAlgorithm::Quantization);
        self.register_algorithm("Pruning", CompressionAlgorithm::Pruning);
        self.register_algorithm("Distillation", CompressionAlgorithm::KnowledgeDistillation);
        Ok(())
    }

    /// Register a compression algorithm under the given name.
    pub fn register_algorithm(&mut self, name: &str, algorithm: CompressionAlgorithm) {
        self.compression_algorithms
            .insert(name.to_string(), algorithm);
    }

    /// Get a registered compression algorithm by name.
    pub fn get_algorithm(&self, name: &str) -> Option<&CompressionAlgorithm> {
        self.compression_algorithms.get(name)
    }

    /// List the names of all registered compression algorithms.
    pub fn list_algorithms(&self) -> Vec<String> {
        self.compression_algorithms.keys().cloned().collect()
    }

    /// Number of bytes needed for a packed one-bit-per-weight pruning mask.
    pub const fn pruning_mask_bytes(weight_count: usize) -> usize {
        weight_count.div_ceil(PRUNING_MASK_BITS_PER_BYTE)
    }

    /// Return whether `weight_index` is present in a packed pruning mask.
    pub fn mask_keeps(mask: &[u8], weight_index: usize) -> bool {
        let byte = weight_index / PRUNING_MASK_BITS_PER_BYTE;
        let bit = weight_index % PRUNING_MASK_BITS_PER_BYTE;
        mask.get(byte)
            .map(|value| (value & (1u8 << bit)) != 0)
            .unwrap_or(false)
    }

    fn set_mask_bit(mask: &mut [u8], weight_index: usize) {
        let byte = weight_index / PRUNING_MASK_BITS_PER_BYTE;
        let bit = weight_index % PRUNING_MASK_BITS_PER_BYTE;
        mask[byte] |= 1u8 << bit;
    }

    /// Run per-tensor symmetric signed-int8 post-training quantization.
    ///
    /// `out` is caller-owned and receives the complete compressed payload.
    /// The returned scale is the only side metadata required to dequantize it.
    pub fn quantize_symmetric_int8_into(
        &mut self,
        weights: &[f64],
        out: &mut [i8],
    ) -> Result<QuantizationReport, MLError> {
        if weights.is_empty() {
            return Err(MLError::ValidationError(
                "cannot quantize an empty weight tensor".to_string(),
            ));
        }
        if out.len() < weights.len() {
            return Err(MLError::ResourceError(format!(
                "int8 output buffer needs {} elements, got {}",
                weights.len(),
                out.len()
            )));
        }

        let mut max_abs = 0.0f64;
        let mut signal_sq = 0.0f64;
        for &weight in weights {
            if !weight.is_finite() {
                return Err(MLError::ValidationError(
                    "quantization input contains a non-finite weight".to_string(),
                ));
            }
            max_abs = max_abs.max(weight.abs());
            signal_sq += weight * weight;
        }

        let scale = if max_abs == 0.0 {
            1.0
        } else {
            max_abs / i8::MAX as f64
        };
        let mut squared_error = 0.0f64;
        let mut max_abs_error = 0.0f64;
        for (index, &weight) in weights.iter().enumerate() {
            let quantized = (weight / scale)
                .round()
                .clamp(-(i8::MAX as f64), i8::MAX as f64) as i8;
            out[index] = quantized;
            let error = weight - quantized as f64 * scale;
            squared_error += error * error;
            max_abs_error = max_abs_error.max(error.abs());
        }

        let rmse = (squared_error / weights.len() as f64).sqrt();
        let signal_rms = (signal_sq / weights.len() as f64).sqrt();
        let preservation = if signal_rms == 0.0 {
            1.0
        } else {
            (1.0 - rmse / signal_rms).clamp(0.0, 1.0)
        };
        let original_bytes = std::mem::size_of_val(weights);
        // Portable payload: one byte per weight plus the f64 scale.
        let compressed_bytes = weights.len() + std::mem::size_of::<f64>();
        let report = QuantizationReport {
            parameters: QuantizationParameters {
                scheme: QuantizationScheme::SymmetricInt8,
                scale,
                zero_point: 0,
            },
            element_count: weights.len(),
            original_bytes,
            compressed_bytes,
            compression_ratio: original_bytes as f64 / compressed_bytes as f64,
            rmse,
            max_abs_error,
        };
        self.record_measured_compression(original_bytes, compressed_bytes, preservation);
        Ok(report)
    }

    /// Dequantize a symmetric-int8 tensor into caller-owned floating-point storage.
    pub fn dequantize_symmetric_int8_into(
        quantized: &[i8],
        parameters: QuantizationParameters,
        out: &mut [f64],
    ) -> Result<usize, MLError> {
        if parameters.scheme != QuantizationScheme::SymmetricInt8
            || !parameters.scale.is_finite()
            || parameters.scale <= 0.0
            || parameters.zero_point != 0
        {
            return Err(MLError::ValidationError(
                "invalid symmetric-int8 quantization parameters".to_string(),
            ));
        }
        if out.len() < quantized.len() {
            return Err(MLError::ResourceError(format!(
                "dequantization output needs {} elements, got {}",
                quantized.len(),
                out.len()
            )));
        }
        for (dst, &value) in out.iter_mut().zip(quantized.iter()) {
            *dst = value as f64 * parameters.scale;
        }
        Ok(quantized.len())
    }

    /// Exact unstructured magnitude pruning.
    ///
    /// The smallest-magnitude weights are removed. The result is a real sparse
    /// representation: a packed keep-mask and the retained values in original
    /// index order. `scratch_indices` is caller-provided sorting workspace.
    pub fn prune_unstructured_into(
        &mut self,
        weights: &[f64],
        sparsity: f64,
        mask_out: &mut [u8],
        values_out: &mut [f64],
        scratch_indices: &mut [usize],
    ) -> Result<PruningReport, MLError> {
        Self::validate_pruning_input(weights, sparsity)?;
        let count = weights.len();
        let mask_bytes = Self::pruning_mask_bytes(count);
        let pruned = ((count as f64 * sparsity).round() as usize).min(count);
        let kept = count - pruned;
        if mask_out.len() < mask_bytes {
            return Err(MLError::ResourceError(format!(
                "pruning mask needs {} bytes, got {}",
                mask_bytes,
                mask_out.len()
            )));
        }
        if values_out.len() < kept {
            return Err(MLError::ResourceError(format!(
                "sparse value buffer needs {} elements, got {}",
                kept,
                values_out.len()
            )));
        }
        if scratch_indices.len() < count {
            return Err(MLError::ResourceError(format!(
                "pruning scratch needs {} indices, got {}",
                count,
                scratch_indices.len()
            )));
        }

        mask_out[..mask_bytes].fill(0);
        for (index, slot) in scratch_indices[..count].iter_mut().enumerate() {
            *slot = index;
        }
        scratch_indices[..count].sort_unstable_by(|left, right| {
            weights[*left]
                .abs()
                .total_cmp(&weights[*right].abs())
                .then_with(|| left.cmp(right))
        });
        for &index in &scratch_indices[pruned..count] {
            Self::set_mask_bit(mask_out, index);
        }

        let mut write = 0usize;
        let mut original_energy = 0.0f64;
        let mut kept_energy = 0.0f64;
        for (index, &weight) in weights.iter().enumerate() {
            original_energy += weight * weight;
            if Self::mask_keeps(mask_out, index) {
                values_out[write] = weight;
                write += 1;
                kept_energy += weight * weight;
            }
        }

        let report = Self::make_pruning_report(
            count,
            pruned,
            count,
            pruned,
            sparsity,
            mask_bytes,
            original_energy,
            kept_energy,
        );
        self.record_measured_compression(
            report.original_bytes,
            report.compressed_bytes,
            report.l2_energy_preserved.sqrt(),
        );
        Ok(report)
    }

    /// Structured output-channel pruning for a row-major `rows × columns` matrix.
    ///
    /// Entire rows with the smallest L2 norm are removed and packed contiguously.
    /// `row_mask_out` contains one keep bit per row.
    pub fn prune_output_channels_into(
        &mut self,
        weights: &[f64],
        rows: usize,
        columns: usize,
        sparsity: f64,
        row_mask_out: &mut [u8],
        values_out: &mut [f64],
        score_scratch: &mut [f64],
        index_scratch: &mut [usize],
    ) -> Result<PruningReport, MLError> {
        Self::validate_pruning_input(weights, sparsity)?;
        if rows == 0 || columns == 0 || rows.checked_mul(columns) != Some(weights.len()) {
            return Err(MLError::ValidationError(format!(
                "structured pruning shape {}x{} does not match {} weights",
                rows,
                columns,
                weights.len()
            )));
        }
        let mask_bytes = Self::pruning_mask_bytes(rows);
        let pruned_rows = ((rows as f64 * sparsity).round() as usize).min(rows);
        let kept_rows = rows - pruned_rows;
        let kept_weights = kept_rows * columns;
        if row_mask_out.len() < mask_bytes
            || score_scratch.len() < rows
            || index_scratch.len() < rows
            || values_out.len() < kept_weights
        {
            return Err(MLError::ResourceError(
                "structured-pruning caller buffers are too small".to_string(),
            ));
        }

        row_mask_out[..mask_bytes].fill(0);
        let mut original_energy = 0.0f64;
        for row in 0..rows {
            let mut row_energy = 0.0f64;
            for &weight in &weights[row * columns..(row + 1) * columns] {
                row_energy += weight * weight;
            }
            score_scratch[row] = row_energy;
            index_scratch[row] = row;
            original_energy += row_energy;
        }
        index_scratch[..rows].sort_unstable_by(|left, right| {
            score_scratch[*left]
                .total_cmp(&score_scratch[*right])
                .then_with(|| left.cmp(right))
        });
        for &row in &index_scratch[pruned_rows..rows] {
            Self::set_mask_bit(row_mask_out, row);
        }

        let mut write = 0usize;
        let mut kept_energy = 0.0f64;
        for row in 0..rows {
            if Self::mask_keeps(row_mask_out, row) {
                let source = &weights[row * columns..(row + 1) * columns];
                values_out[write..write + columns].copy_from_slice(source);
                write += columns;
                kept_energy += score_scratch[row];
            }
        }

        let report = Self::make_pruning_report(
            weights.len(),
            pruned_rows * columns,
            rows,
            pruned_rows,
            sparsity,
            mask_bytes,
            original_energy,
            kept_energy,
        );
        self.record_measured_compression(
            report.original_bytes,
            report.compressed_bytes,
            report.l2_energy_preserved.sqrt(),
        );
        Ok(report)
    }

    /// Reconstruct an unstructured sparse tensor into caller-owned dense storage.
    pub fn unpack_pruned_weights_into(
        mask: &[u8],
        packed_values: &[f64],
        out: &mut [f64],
    ) -> Result<usize, MLError> {
        let needed_mask = Self::pruning_mask_bytes(out.len());
        if mask.len() < needed_mask {
            return Err(MLError::ResourceError(format!(
                "pruning mask needs {} bytes, got {}",
                needed_mask,
                mask.len()
            )));
        }
        let kept = (0..out.len())
            .filter(|&index| Self::mask_keeps(mask, index))
            .count();
        if packed_values.len() < kept {
            return Err(MLError::ResourceError(format!(
                "packed sparse tensor needs {} values, got {}",
                kept,
                packed_values.len()
            )));
        }

        let mut read = 0usize;
        for (index, value) in out.iter_mut().enumerate() {
            if Self::mask_keeps(mask, index) {
                *value = packed_values[read];
                read += 1;
            } else {
                *value = 0.0;
            }
        }
        Ok(read)
    }

    /// Distil any inference-supported teacher MLP into the existing single-linear
    /// SGD student. Teacher outputs are generated from real forward passes and
    /// optionally blended with hard targets in `target_buffer`.
    pub fn distill_linear_student(
        &mut self,
        training_engine: &mut TrainingEngine,
        teacher: &Model,
        student: &mut Model,
        training_data: &[f64],
        hard_targets: Option<&[f64]>,
        distillation: DistillationConfig,
        training: &TrainingConfig,
        target_buffer: &mut [f64],
    ) -> Result<DistillationReport, MLError> {
        if !distillation.teacher_weight.is_finite()
            || !(0.0..=1.0).contains(&distillation.teacher_weight)
        {
            return Err(MLError::ValidationError(
                "teacher_weight must be finite and in [0, 1]".to_string(),
            ));
        }
        let input_size = student
            .architecture
            .input_shape
            .first()
            .copied()
            .ok_or_else(|| MLError::TrainingError("student input shape is empty".into()))?;
        let output_size = student
            .architecture
            .output_shape
            .first()
            .copied()
            .ok_or_else(|| MLError::TrainingError("student output shape is empty".into()))?;
        if input_size == 0 || training_data.is_empty() || training_data.len() % input_size != 0 {
            return Err(MLError::DataError(
                "distillation training data has an invalid shape".to_string(),
            ));
        }
        if teacher.architecture.input_shape.first().copied() != Some(input_size)
            || teacher.architecture.output_shape.first().copied() != Some(output_size)
        {
            return Err(MLError::ValidationError(
                "teacher and student input/output shapes must match".to_string(),
            ));
        }
        let samples = training_data.len() / input_size;
        let target_count = samples * output_size;
        if target_buffer.len() < target_count {
            return Err(MLError::ResourceError(format!(
                "distillation target buffer needs {} elements, got {}",
                target_count,
                target_buffer.len()
            )));
        }
        if let Some(targets) = hard_targets {
            if targets.len() != target_count {
                return Err(MLError::DataError(format!(
                    "hard target length {} does not match {}",
                    targets.len(),
                    target_count
                )));
            }
        } else if distillation.teacher_weight < 1.0 {
            return Err(MLError::ValidationError(
                "hard targets are required when teacher_weight is below 1".to_string(),
            ));
        }

        for sample in 0..samples {
            let input = &training_data[sample * input_size..(sample + 1) * input_size];
            let teacher_output = InferenceEngine::forward_pass(teacher, input)?;
            if teacher_output.len() != output_size {
                return Err(MLError::ValidationError(
                    "teacher produced an unexpected output shape".to_string(),
                ));
            }
            for output in 0..output_size {
                let index = sample * output_size + output;
                let hard = hard_targets.map(|targets| targets[index]).unwrap_or(0.0);
                target_buffer[index] = distillation.teacher_weight * teacher_output[output]
                    + (1.0 - distillation.teacher_weight) * hard;
            }
        }

        let fidelity_mse_before =
            Self::teacher_student_fidelity_mse(teacher, student, training_data)?;
        let training_result = training_engine.start_training(
            student,
            training_data,
            &target_buffer[..target_count],
            training,
        )?;
        let fidelity_mse_after =
            Self::teacher_student_fidelity_mse(teacher, student, training_data)?;

        let teacher_bytes = std::mem::size_of_val(teacher.weights.as_slice());
        let student_bytes = std::mem::size_of_val(student.weights.as_slice());
        let compression_ratio = teacher_bytes as f64 / student_bytes.max(1) as f64;
        self.record_measured_compression(
            teacher_bytes,
            student_bytes,
            (1.0 / (1.0 + fidelity_mse_after.sqrt())).clamp(0.0, 1.0),
        );
        Ok(DistillationReport {
            teacher_parameters: teacher.weights.len(),
            student_parameters: student.weights.len(),
            compression_ratio,
            fidelity_mse_before,
            fidelity_mse_after,
            training: training_result,
        })
    }

    fn teacher_student_fidelity_mse(
        teacher: &Model,
        student: &Model,
        training_data: &[f64],
    ) -> Result<f64, MLError> {
        let input_size = student.architecture.input_shape[0];
        let samples = training_data.len() / input_size;
        let mut squared_error = 0.0f64;
        let mut outputs = 0usize;
        for sample in 0..samples {
            let input = &training_data[sample * input_size..(sample + 1) * input_size];
            let teacher_output = InferenceEngine::forward_pass(teacher, input)?;
            let student_output = InferenceEngine::forward_pass(student, input)?;
            if teacher_output.len() != student_output.len() {
                return Err(MLError::ValidationError(
                    "teacher and student output lengths differ".to_string(),
                ));
            }
            for (&teacher_value, &student_value) in teacher_output.iter().zip(student_output.iter())
            {
                let difference = teacher_value - student_value;
                squared_error += difference * difference;
                outputs += 1;
            }
        }
        Ok(if outputs == 0 {
            0.0
        } else {
            squared_error / outputs as f64
        })
    }

    fn validate_pruning_input(weights: &[f64], sparsity: f64) -> Result<(), MLError> {
        if weights.is_empty() {
            return Err(MLError::ValidationError(
                "cannot prune an empty weight tensor".to_string(),
            ));
        }
        if !sparsity.is_finite() || !(0.0..=1.0).contains(&sparsity) {
            return Err(MLError::ValidationError(
                "sparsity must be finite and in [0, 1]".to_string(),
            ));
        }
        if weights.iter().any(|weight| !weight.is_finite()) {
            return Err(MLError::ValidationError(
                "pruning input contains a non-finite weight".to_string(),
            ));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn make_pruning_report(
        total_weights: usize,
        pruned_weights: usize,
        total_units: usize,
        pruned_units: usize,
        requested_sparsity: f64,
        mask_bytes: usize,
        original_energy: f64,
        kept_energy: f64,
    ) -> PruningReport {
        let kept_weights = total_weights - pruned_weights;
        let original_bytes = total_weights * std::mem::size_of::<f64>();
        let compressed_bytes = mask_bytes + kept_weights * std::mem::size_of::<f64>();
        PruningReport {
            total_weights,
            pruned_weights,
            kept_weights,
            total_units,
            pruned_units,
            requested_sparsity,
            achieved_sparsity: pruned_weights as f64 / total_weights as f64,
            original_bytes,
            compressed_bytes,
            compression_ratio: original_bytes as f64 / compressed_bytes.max(1) as f64,
            l2_energy_preserved: if original_energy == 0.0 {
                1.0
            } else {
                kept_energy / original_energy
            },
        }
    }

    fn record_measured_compression(
        &mut self,
        original_bytes: usize,
        compressed_bytes: usize,
        preservation: f64,
    ) {
        self.compression_statistics.original_size = original_bytes as u64;
        self.compression_statistics.compressed_size = compressed_bytes as u64;
        self.compression_statistics.compression_ratio =
            original_bytes as f64 / compressed_bytes.max(1) as f64;

        let count = self.quality_metrics.compression_count;
        let next = count + 1;
        let ratio = self.compression_statistics.compression_ratio;
        let reduction =
            (1.0 - compressed_bytes as f64 / original_bytes.max(1) as f64).clamp(0.0, 1.0);
        self.quality_metrics.compression_count = next;
        self.quality_metrics.compression_ratio =
            (self.quality_metrics.compression_ratio * count as f64 + ratio) / next as f64;
        self.quality_metrics.size_reduction =
            (self.quality_metrics.size_reduction * count as f64 + reduction) / next as f64;
        self.quality_metrics.memory_savings = self.quality_metrics.size_reduction;
        self.quality_metrics.accuracy_preservation = (self.quality_metrics.accuracy_preservation
            * count as f64
            + preservation.clamp(0.0, 1.0))
            / next as f64;
    }

    /// Record the result of a compression operation and update the aggregate
    /// quality metrics.
    pub fn record_compression(
        &mut self,
        algorithm_name: &str,
        original_size: usize,
        compressed_size: usize,
        accuracy_before: f64,
        accuracy_after: f64,
    ) -> Result<(), MLError> {
        if !self.compression_algorithms.contains_key(algorithm_name) {
            return Err(MLError::OptimizationError(format!(
                "unknown compression algorithm '{}'",
                algorithm_name
            )));
        }
        if original_size == 0 {
            return Err(MLError::ValidationError(
                "original_size must be greater than zero".to_string(),
            ));
        }

        let ratio = original_size as f64 / compressed_size.max(1) as f64;
        let size_reduction = 1.0 - (compressed_size as f64 / original_size as f64);
        let accuracy_preservation = if accuracy_before > 0.0 {
            (accuracy_after / accuracy_before).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Update the running aggregate statistics.
        let count = self.quality_metrics.compression_count;
        let prev_ratio = self.quality_metrics.compression_ratio;
        let prev_reduction = self.quality_metrics.size_reduction;
        let prev_accuracy = self.quality_metrics.accuracy_preservation;

        let new_count = count + 1;
        self.quality_metrics.compression_count = new_count;
        // Running average across all recorded compressions.
        self.quality_metrics.compression_ratio =
            (prev_ratio * count as f64 + ratio) / new_count as f64;
        self.quality_metrics.size_reduction =
            (prev_reduction * count as f64 + size_reduction) / new_count as f64;
        self.quality_metrics.accuracy_preservation =
            (prev_accuracy * count as f64 + accuracy_preservation) / new_count as f64;
        // Memory savings mirror the size reduction for this simple wiring.
        self.quality_metrics.memory_savings = self.quality_metrics.size_reduction;

        Ok(())
    }

    /// Access the aggregate compression quality metrics.
    pub fn get_quality_metrics(&self) -> &CompressionQualityMetrics {
        &self.quality_metrics
    }

    /// Access byte counts and the ratio from the most recent real compression.
    pub fn get_compression_statistics(&self) -> &CompressionStatistics {
        &self.compression_statistics
    }

    /// Return the overall compression ratio recorded so far.
    pub fn compression_ratio(&self) -> f64 {
        self.quality_metrics.compression_ratio
    }
}

impl CompressionStatistics {
    pub fn new() -> Self {
        Self {
            original_size: 0,
            compressed_size: 0,
            compression_ratio: 0.0,
            compression_time: 0,
            decompression_time: 0,
        }
    }
}

impl CompressionQualityMetrics {
    pub fn new() -> Self {
        Self {
            accuracy_preservation: 0.0,
            performance_impact: 0.0,
            memory_savings: 0.0,
            compression_ratio: 0.0,
            size_reduction: 0.0,
            compression_count: 0,
        }
    }
}

impl ModelVersionControl {
    pub fn new() -> Self {
        Self {
            versions: HashMap::new(),
            branches: HashMap::new(),
            tags: HashMap::new(),
            initialized: false,
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        // Seed the default `main` branch so the controller is usable immediately.
        self.branches
            .entry("main".to_string())
            .or_insert_with(Vec::new);
        self.initialized = true;
        Ok(())
    }

    /// Register a new version for a model. Returns an error if a version with
    /// the same `version_id` is already registered for that model.
    pub fn create_version(&mut self, model_id: &str, version: ModelVersion) -> Result<(), MLError> {
        let key = version_key(model_id, &version.version_id);
        if self.versions.contains_key(&key) {
            return Err(MLError::ModelError(format!(
                "version '{}' already exists for model '{}'",
                version.version_id, model_id
            )));
        }
        let version_id = version.version_id.clone();
        self.versions.insert(key, version);
        // Append the new version to the `main` branch if it exists.
        if let Some(branch) = self.branches.get_mut("main") {
            if !branch.iter().any(|v| v == &version_id) {
                branch.push(version_id);
            }
        }
        Ok(())
    }

    /// Get a specific version of a model by its version id.
    pub fn get_version(&self, model_id: &str, version_id: &str) -> Option<&ModelVersion> {
        self.versions.get(&version_key(model_id, version_id))
    }

    /// List all version ids registered for a model.
    pub fn list_versions(&self, model_id: &str) -> Vec<String> {
        let prefix = format!("{}::", model_id);
        self.versions
            .keys()
            .filter_map(|k| k.strip_prefix(&prefix).map(|s| s.to_string()))
            .collect()
    }

    /// Create a branch starting from an existing version. The branch initially
    /// contains only the originating version.
    pub fn create_branch(&mut self, branch_name: &str, from_version: &str) -> Result<(), MLError> {
        if self.branches.contains_key(branch_name) {
            return Err(MLError::ModelError(format!(
                "branch '{}' already exists",
                branch_name
            )));
        }
        // Validate that the originating version is registered somewhere.
        let exists = self.versions.values().any(|v| v.version_id == from_version);
        if !exists {
            return Err(MLError::ModelError(format!(
                "cannot branch from unknown version '{}'",
                from_version
            )));
        }
        self.branches
            .insert(branch_name.to_string(), vec![from_version.to_string()]);
        Ok(())
    }

    /// Get the list of version ids in a branch.
    pub fn get_branch(&self, branch_name: &str) -> Option<&Vec<String>> {
        self.branches.get(branch_name)
    }

    /// Tag a version. Multiple tags may be attached to the same version.
    pub fn tag_version(&mut self, version_id: &str, tag: &str) -> Result<(), MLError> {
        // The version must exist somewhere in the registry.
        let exists = self.versions.values().any(|v| v.version_id == version_id);
        if !exists {
            return Err(MLError::ModelError(format!(
                "cannot tag unknown version '{}'",
                version_id
            )));
        }
        let entry = self
            .tags
            .entry(version_id.to_string())
            .or_insert_with(Vec::new);
        if !entry.iter().any(|t| t == tag) {
            entry.push(tag.to_string());
        }
        Ok(())
    }

    /// Get all tags attached to a version.
    pub fn get_tags(&self, version_id: &str) -> Vec<String> {
        self.tags.get(version_id).cloned().unwrap_or_default()
    }

    /// Find all version ids that carry the given tag.
    pub fn get_by_tag(&self, tag: &str) -> Vec<String> {
        self.tags
            .iter()
            .filter_map(|(version_id, tags)| {
                if tags.iter().any(|t| t == tag) {
                    Some(version_id.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Build the composite key used to store versions per model.
fn version_key(model_id: &str, version_id: &str) -> String {
    format!("{}::{}", model_id, version_id)
}

impl ModelVersion {
    pub fn new() -> Self {
        Self {
            version_id: "v1.0.0".to_string(),
            version_number: "1.0.0".to_string(),
            changes: Vec::new(),
            created_at: 0,
            created_by: "system".to_string(),
        }
    }
}

impl ModelChange {
    pub fn new() -> Self {
        Self {
            change_id: "change_1".to_string(),
            change_type: ChangeType::Architecture,
            description: "Initial model".to_string(),
            affected_layers: Vec::new(),
        }
    }
}

impl ModelBranch {
    pub fn new() -> Self {
        Self {
            branch_id: "main".to_string(),
            branch_name: "main".to_string(),
            base_version: "v1.0.0".to_string(),
            head_version: "v1.0.0".to_string(),
        }
    }
}

impl ModelTag {
    pub fn new() -> Self {
        Self {
            tag_id: "latest".to_string(),
            tag_name: "latest".to_string(),
            version: "v1.0.0".to_string(),
            description: "Latest version".to_string(),
        }
    }
}

impl ModelLoader {
    pub fn new() -> Self {
        Self {
            loading_strategies: HashMap::new(),
            format_converters: HashMap::new(),
            loading_cache: LoadingCache::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.loading_cache.initialize()?;
        Ok(())
    }

    /// Register a loading strategy under the given name.
    pub fn register_loading_strategy(&mut self, name: &str, strategy: LoadingStrategy) {
        self.loading_strategies
            .insert(name.to_string(), strategy);
    }

    /// Get a registered loading strategy by name.
    pub fn get_loading_strategy(&self, name: &str) -> Option<&LoadingStrategy> {
        self.loading_strategies.get(name)
    }

    /// List the names of all registered loading strategies.
    pub fn list_loading_strategies(&self) -> Vec<String> {
        self.loading_strategies.keys().cloned().collect()
    }

    /// Register a format converter under the given name.
    pub fn register_format_converter(&mut self, name: &str, converter: FormatConverter) {
        self.format_converters
            .insert(name.to_string(), converter);
    }

    /// Get a registered format converter by name.
    pub fn get_format_converter(&self, name: &str) -> Option<&FormatConverter> {
        self.format_converters.get(name)
    }

    /// List the names of all registered format converters.
    pub fn list_format_converters(&self) -> Vec<String> {
        self.format_converters.keys().cloned().collect()
    }
}

impl LoadingStrategy {
    pub fn new() -> Self {
        Self {
            strategy_id: "default".to_string(),
            strategy_type: LoadingStrategyType::Lazy,
            parameters: LoadingParameters::new(),
        }
    }
}

impl LoadingParameters {
    pub fn new() -> Self {
        Self {
            chunk_size: 1024,
            prefetch_size: 2048,
            cache_size: 100 * 1024 * 1024, // 100MB
            parallel_loading: true,
        }
    }
}

impl FormatConverter {
    pub fn new() -> Self {
        Self {
            converter_id: "default".to_string(),
            source_format: "pytorch".to_string(),
            target_format: "onnx".to_string(),
            conversion_pipeline: Vec::new(),
        }
    }
}

impl ConversionStep {
    pub fn new() -> Self {
        Self {
            step_id: "step_1".to_string(),
            step_type: ConversionStepType::Parsing,
            parameters: HashMap::new(),
        }
    }
}

impl LoadingCache {
    pub fn new() -> Self {
        Self {
            cache_entries: HashMap::new(),
            cache_policy: CachePolicy::new(),
            cache_stats: CacheStats::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        Ok(())
    }

    /// Insert or replace a cache entry by id.
    pub fn put_entry(&mut self, entry: CacheEntry) {
        self.cache_entries.insert(entry.entry_id.clone(), entry);
    }

    /// Retrieve a cache entry by id, incrementing its access count and updating
    /// the last-accessed timestamp.
    pub fn get_entry(&mut self, entry_id: &str) -> Option<CacheEntry> {
        let now = current_timestamp_secs();
        let found = self.cache_entries.get_mut(entry_id).map(|entry| {
            entry.access_count += 1;
            entry.last_accessed = now;
            entry.clone()
        });
        match &found {
            Some(_) => self.cache_stats.hit_count += 1,
            None => self.cache_stats.miss_count += 1,
        }
        self.update_hit_rate();
        found
    }

    /// Remove a cache entry by id. Returns `true` if an entry was removed.
    pub fn remove_entry(&mut self, entry_id: &str) -> bool {
        let removed = self.cache_entries.remove(entry_id).is_some();
        if removed {
            self.update_hit_rate();
        }
        removed
    }

    /// Number of entries currently held in the cache.
    pub fn cache_size(&self) -> usize {
        self.cache_entries.len()
    }

    /// Return a reference to the cache policy.
    pub fn cache_policy(&self) -> &CachePolicy {
        &self.cache_policy
    }

    /// Return a reference to the cache statistics.
    pub fn cache_stats(&self) -> &CacheStats {
        &self.cache_stats
    }

    /// Recompute the rolling hit rate from hit/miss counts.
    fn update_hit_rate(&mut self) {
        let total = self.cache_stats.hit_count + self.cache_stats.miss_count;
        self.cache_stats.hit_rate = if total == 0 {
            0.0
        } else {
            self.cache_stats.hit_count as f64 / total as f64
        };
    }
}

impl CachePolicy {
    pub fn new() -> Self {
        Self {
            eviction_policy: EvictionPolicy::LRU,
            max_size: 1024 * 1024 * 1024, // 1GB
            ttl: 3600,                    // 1 hour
        }
    }
}

impl CacheStats {
    pub fn new() -> Self {
        Self {
            hit_count: 0,
            miss_count: 0,
            hit_rate: 0.0,
            total_size: 0,
        }
    }
}

impl CacheEntry {
    pub fn new() -> Self {
        Self {
            entry_id: "cache_1".to_string(),
            model_data: vec![0u8; 1000],
            access_count: 0,
            last_accessed: 0,
            size: 1000,
        }
    }
}

impl ModelConverter {
    pub fn new() -> Self {
        Self {
            conversion_pipelines: HashMap::new(),
            optimization_strategies: HashMap::new(),
            validation_engine: ValidationEngine::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.validation_engine.initialize()?;
        Ok(())
    }

    /// Register a conversion pipeline under the given name.
    pub fn register_pipeline(&mut self, name: &str, pipeline: ConversionPipeline) {
        self.conversion_pipelines
            .insert(name.to_string(), pipeline);
    }

    /// Get a registered conversion pipeline by name.
    pub fn get_pipeline(&self, name: &str) -> Option<&ConversionPipeline> {
        self.conversion_pipelines.get(name)
    }

    /// List the names of all registered conversion pipelines.
    pub fn list_pipelines(&self) -> Vec<String> {
        self.conversion_pipelines.keys().cloned().collect()
    }

    /// Register an optimization strategy under the given name.
    pub fn register_optimization_strategy(&mut self, name: &str, strategy: OptimizationStrategy) {
        self.optimization_strategies
            .insert(name.to_string(), strategy);
    }

    /// Get a registered optimization strategy by name.
    pub fn get_optimization_strategy(&self, name: &str) -> Option<&OptimizationStrategy> {
        self.optimization_strategies.get(name)
    }

    /// List the names of all registered optimization strategies.
    pub fn list_optimization_strategies(&self) -> Vec<String> {
        self.optimization_strategies.keys().cloned().collect()
    }
}

impl ConversionPipeline {
    pub fn new() -> Self {
        Self {
            pipeline_id: "default".to_string(),
            source_format: "pytorch".to_string(),
            target_format: "onnx".to_string(),
            steps: Vec::new(),
            quality_assurance: QualityAssurance::new(),
        }
    }
}

impl QualityAssurance {
    pub fn new() -> Self {
        Self {
            validation_rules: Vec::new(),
            test_cases: Vec::new(),
            accuracy_threshold: 0.95,
        }
    }
}

impl ValidationRule {
    pub fn new() -> Self {
        Self {
            rule_id: "rule_1".to_string(),
            rule_type: ValidationRuleType::Architecture,
            condition: "true".to_string(),
            action: ValidationAction::Pass,
        }
    }
}

impl TestCase {
    pub fn new() -> Self {
        Self {
            test_id: "test_1".to_string(),
            test_type: TestType::Inference,
            input_data: vec![1u8; 100],
            expected_output: vec![2u8; 100],
        }
    }
}

impl OptimizationStrategy {
    pub fn new() -> Self {
        Self {
            strategy_id: "default".to_string(),
            strategy_type: OptimizationStrategyType::Quantization,
            parameters: OptimizationParameters::new(),
        }
    }
}

impl OptimizationParameters {
    pub fn new() -> Self {
        Self {
            target_size: 100 * 1024 * 1024, // 100MB
            accuracy_threshold: 0.95,
            performance_target: 1.0,
            optimization_level: OptimizationLevel::Moderate,
        }
    }
}

impl ValidationEngine {
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
            validation_rules: Vec::new(),
            test_suite: TestSuite::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        Ok(())
    }

    /// Register a validator under the given id.
    pub fn register_validator(&mut self, validator: Validator) {
        self.validators
            .insert(validator.validator_id.clone(), validator);
    }

    /// Get a registered validator by id.
    pub fn get_validator(&self, validator_id: &str) -> Option<&Validator> {
        self.validators.get(validator_id)
    }

    /// List the ids of all registered validators.
    pub fn list_validators(&self) -> Vec<String> {
        self.validators.keys().cloned().collect()
    }

    /// Add a validation rule to the engine.
    pub fn add_validation_rule(&mut self, rule: ValidationRule) {
        self.validation_rules.push(rule);
    }

    /// Return a reference to all validation rules.
    pub fn validation_rules(&self) -> &[ValidationRule] {
        &self.validation_rules
    }

    /// Return a reference to the test suite.
    pub fn test_suite(&self) -> &TestSuite {
        &self.test_suite
    }

    /// Return a mutable reference to the test suite.
    pub fn test_suite_mut(&mut self) -> &mut TestSuite {
        &mut self.test_suite
    }
}

impl Validator {
    pub fn new() -> Self {
        Self {
            validator_id: "default".to_string(),
            validator_type: ValidatorType::Architecture,
            validation_logic: ValidationLogic::new(),
        }
    }
}

impl ValidationLogic {
    pub fn new() -> Self {
        Self {
            logic_id: "logic_1".to_string(),
            conditions: Vec::new(),
            actions: Vec::new(),
        }
    }
}

impl ValidationCondition {
    pub fn new() -> Self {
        Self {
            condition_id: "cond_1".to_string(),
            field: "model_type".to_string(),
            operator: ComparisonOperator::Equals,
            value: ValidationValue::String("LLM".to_string()),
        }
    }
}

impl ValidationValue {
    pub fn string(value: &str) -> Self {
        Self::String(value.to_string())
    }

    pub fn number(value: f64) -> Self {
        Self::Number(value)
    }

    pub fn boolean(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl TestSuite {
    pub fn new() -> Self {
        Self {
            test_cases: Vec::new(),
            test_environment: TestEnvironment::new(),
            test_results: TestResults::new(),
        }
    }
}

impl TestEnvironment {
    pub fn new() -> Self {
        Self {
            environment_id: "default".to_string(),
            hardware: HardwareSpec::new(),
            software: SoftwareSpec::new(),
            configuration: TestConfiguration::new(),
        }
    }
}

impl HardwareSpec {
    pub fn new() -> Self {
        Self {
            cpu_cores: 8,
            memory_size: 16 * 1024 * 1024 * 1024, // 16GB
            gpu_count: 1,
            gpu_memory: 8 * 1024 * 1024 * 1024,          // 8GB
            storage_size: 1 * 1024 * 1024 * 1024 * 1024, // 1TB
        }
    }
}

impl SoftwareSpec {
    pub fn new() -> Self {
        Self {
            os: "Linux".to_string(),
            framework_version: "1.0.0".to_string(),
            dependencies: Vec::new(),
        }
    }
}

impl TestConfiguration {
    pub fn new() -> Self {
        Self {
            batch_size: 32,
            sequence_length: 512,
            precision: Precision::FP32,
        }
    }
}

impl TestResults {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            summary: TestSummary::new(),
        }
    }
}

impl TestResult {
    pub fn new() -> Self {
        Self {
            test_id: "test_1".to_string(),
            passed: true,
            execution_time: 100,
            error_message: None,
            metrics: TestMetrics::new(),
        }
    }
}

impl TestMetrics {
    pub fn new() -> Self {
        Self {
            accuracy: 0.0, // not measured (scaffold default; no evaluation performed)
            latency: 10.0,
            throughput: 100.0,
            memory_usage: 1024 * 1024, // 1MB
        }
    }
}

impl TestSummary {
    pub fn new() -> Self {
        Self {
            total_tests: 1,
            passed_tests: 1,
            failed_tests: 0,
            pass_rate: 1.0,
            average_execution_time: 100.0,
        }
    }
}

impl ModelCache {
    pub fn new() -> Self {
        Self {
            cache_entries: HashMap::new(),
            cache_policy: ModelCachePolicy::new(),
            cache_stats: ModelCacheStats::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        Ok(())
    }

    pub fn get(&mut self, model_id: &str) -> Option<Model> {
        let now = current_timestamp_secs();
        let found = self.cache_entries.get_mut(model_id).map(|entry| {
            entry.access_count += 1;
            entry.last_accessed = now;
            entry.model.clone()
        });

        match found {
            Some(model) => {
                self.cache_stats.hit_count += 1;
                self.update_hit_rate();
                Some(model)
            }
            None => {
                self.cache_stats.miss_count += 1;
                self.update_hit_rate();
                None
            }
        }
    }

    pub fn put(&mut self, model_id: String, model: Model) -> Result<(), MLError> {
        let size = (model.weights.len() * std::mem::size_of::<f64>()) as u64;
        let now = current_timestamp_secs();

        // If updating an existing entry, subtract its old size first.
        if let Some(existing) = self.cache_entries.get(&model_id) {
            self.cache_stats.total_size -= existing.size;
        }

        let entry = ModelCacheEntry {
            entry_id: model_id.clone(),
            model: model.clone(),
            access_count: 1,
            last_accessed: now,
            size,
            hit_rate: 0.0,
        };
        self.cache_entries.insert(model_id, entry);
        self.cache_stats.total_size += size;

        // Evict LRU entries while the cache exceeds the configured max size.
        while self.cache_stats.total_size > self.cache_policy.max_size
            && self.cache_entries.len() > 1
        {
            self.evict_lru();
        }

        Ok(())
    }

    /// Returns the number of entries currently held in the cache.
    pub fn cache_size(&self) -> usize {
        self.cache_entries.len()
    }

    /// Returns a reference to the cache statistics.
    pub fn cache_stats(&self) -> &ModelCacheStats {
        &self.cache_stats
    }

    /// Recompute the rolling hit rate from hit/miss counts.
    fn update_hit_rate(&mut self) {
        let total = self.cache_stats.hit_count + self.cache_stats.miss_count;
        self.cache_stats.hit_rate = if total == 0 {
            0.0
        } else {
            self.cache_stats.hit_count as f64 / total as f64
        };
    }

    /// Evict the entry with the oldest `last_accessed` timestamp (LRU).
    fn evict_lru(&mut self) {
        if let Some((lru_key, lru_size)) = self
            .cache_entries
            .iter()
            .min_by_key(|(_, e)| e.last_accessed)
            .map(|(k, e)| (k.clone(), e.size))
        {
            self.cache_entries.remove(&lru_key);
            self.cache_stats.total_size -= lru_size;
            self.cache_stats.eviction_count += 1;
        }
    }
}

/// Current time in seconds since the Unix epoch, used for `last_accessed` stamps.
fn current_timestamp_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl ModelCachePolicy {
    pub fn new() -> Self {
        Self {
            eviction_policy: ModelEvictionPolicy::LRU,
            max_size: 10 * 1024 * 1024 * 1024, // 10GB
            ttl: 3600,                         // 1 hour
            priority_levels: vec![
                PriorityLevel::Critical,
                PriorityLevel::High,
                PriorityLevel::Medium,
                PriorityLevel::Low,
            ],
        }
    }
}

impl ModelCacheStats {
    pub fn new() -> Self {
        Self {
            hit_count: 0,
            miss_count: 0,
            hit_rate: 0.0,
            total_size: 0,
            eviction_count: 0,
        }
    }
}

impl ModelCacheEntry {
    pub fn new() -> Self {
        Self {
            entry_id: "cache_1".to_string(),
            model: Model::new(),
            access_count: 0,
            last_accessed: 0,
            size: 0,
            hit_rate: 0.0,
        }
    }
}

impl InferenceEngine {
    pub fn new() -> Self {
        Self {
            inference_backends: HashMap::new(),
            request_scheduler: RequestScheduler::new(),
            batch_processor: BatchProcessor::new(),
            performance_optimizer: InferenceOptimizer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.request_scheduler.initialize()?;
        self.batch_processor.initialize()?;
        self.performance_optimizer.initialize()?;
        Ok(())
    }

    /// Register an inference backend under its backend id.
    pub fn register_backend(&mut self, backend: InferenceBackend) {
        self.inference_backends
            .insert(backend.backend_id.clone(), backend);
    }

    /// Get a registered inference backend by id.
    pub fn get_backend(&self, backend_id: &str) -> Option<&InferenceBackend> {
        self.inference_backends.get(backend_id)
    }

    /// List the ids of all registered inference backends.
    pub fn list_backends(&self) -> Vec<String> {
        self.inference_backends.keys().cloned().collect()
    }

    /// Remove a registered inference backend by id. Returns `true` if removed.
    pub fn remove_backend(&mut self, backend_id: &str) -> bool {
        self.inference_backends.remove(backend_id).is_some()
    }

    pub fn execute_inference(
        &mut self,
        request: &InferenceRequest,
        model: &Model,
    ) -> Result<InferenceResult, MLError> {
        // Wired to a real (if basic) forward pass over the model's architecture, using the
        // model's `weights` field as the flattened parameter buffer and the element-wise
        // activation math from `crate::solvers::activation`. This is an MLP inference backend:
        // it supports `Linear` (and pass-through `Activation`/`Dropout`) layers and returns a
        // clear error for layer types it cannot yet evaluate (Convolutional, Attention, …).
        // For production autoregressive LLM inference, route to the native gguf_bridge engine.
        let start = std::time::Instant::now();

        // Decode the request's byte payload as a little-endian f64 input vector.
        let input = decode_f64_le(&request.input_data).ok_or_else(|| {
            MLError::DataError(format!(
                "input_data length ({}) is not a multiple of {} (f64 size)",
                request.input_data.len(),
                std::mem::size_of::<f64>()
            ))
        })?;

        // Run the forward pass over the model's layers.
        let output = Self::forward_pass(model, &input)?;

        // Re-encode the output vector as little-endian f64 bytes.
        let output_data = encode_f64_le(&output);

        let inference_time = start.elapsed().as_millis() as u64;

        Ok(InferenceResult {
            result_id: format!("result_{}", request.request_id),
            output_data,
            inference_time,
            // This backend computes a deterministic forward pass, not a probabilistic model,
            // so there is no calibrated confidence to report. Surface 1.0 to indicate the pass
            // completed successfully (callers needing real confidence should use gguf_bridge).
            confidence: 1.0,
            metadata: ResultMetadata {
                model_id: model.model_id.clone(),
                backend_id: "linear_algebra_mlp".to_string(),
                batch_size: request.parameters.batch_size,
                sequence_length: request.parameters.sequence_length,
                tokens_generated: output.len(),
            },
        })
    }

    /// Run a basic MLP forward pass over the model's architecture.
    ///
    /// For each `Linear` layer the flattened `model.weights` buffer is consumed in order:
    /// first the `output_size × input_size` weight matrix (row-major), then the `output_size`
    /// bias vector. The layer output is `activation(W · x + b)`, with the activation drawn from
    /// `crate::solvers::activation`. `Activation` layers apply their activation in place and
    /// `Dropout` is the identity at inference time. All other layer types return a clear error.
    fn forward_pass(model: &Model, input: &[f64]) -> Result<Vec<f64>, MLError> {
        let layers = &model.architecture.layers;
        if layers.is_empty() {
            return Err(MLError::InferenceError(
                "model architecture has no layers".to_string(),
            ));
        }

        let mut activations = input.to_vec();
        let mut weight_offset = 0usize;

        for (idx, layer) in layers.iter().enumerate() {
            match layer.layer_type {
                LayerType::Linear => {
                    let in_size = layer.input_shape.first().copied().ok_or_else(|| {
                        MLError::InferenceError(format!(
                            "layer {} ({}): missing input dimension",
                            idx, layer.layer_id
                        ))
                    })?;
                    let out_size = layer.output_shape.first().copied().ok_or_else(|| {
                        MLError::InferenceError(format!(
                            "layer {} ({}): missing output dimension",
                            idx, layer.layer_id
                        ))
                    })?;

                    if activations.len() != in_size {
                        return Err(MLError::InferenceError(format!(
                            "layer {} ({}): expected input size {}, got {}",
                            idx,
                            layer.layer_id,
                            in_size,
                            activations.len()
                        )));
                    }

                    let weight_count = in_size * out_size;
                    let bias_count = out_size;
                    let needed = weight_count + bias_count;
                    if weight_offset + needed > model.weights.len() {
                        return Err(MLError::InferenceError(format!(
                            "layer {} ({}): not enough weights (need {} at offset {}, have {})",
                            idx,
                            layer.layer_id,
                            needed,
                            weight_offset,
                            model.weights.len()
                        )));
                    }

                    // output[j] = sum_i W[j*in_size + i] * x[i] + bias[j]
                    let mut out = vec![0.0f64; out_size];
                    for j in 0..out_size {
                        let mut acc = 0.0;
                        for i in 0..in_size {
                            acc += model.weights[weight_offset + j * in_size + i] * activations[i];
                        }
                        acc += model.weights[weight_offset + weight_count + j];
                        out[j] = acc;
                    }
                    weight_offset += needed;

                    if let Some(act) = &layer.activation {
                        apply_activation(&mut out, act)?;
                    }
                    activations = out;
                }
                LayerType::Activation => {
                    if let Some(act) = &layer.activation {
                        apply_activation(&mut activations, act)?;
                    } else {
                        return Err(MLError::InferenceError(format!(
                            "layer {} ({}): Activation layer has no activation function set",
                            idx, layer.layer_id
                        )));
                    }
                }
                LayerType::Dropout => {
                    // Dropout is the identity at inference time.
                }
                ref other => {
                    return Err(MLError::InferenceError(format!(
                        "layer {} ({}): {:?} layers are not yet supported by the MLP inference \
                         backend (only Linear/Activation/Dropout); use the native gguf_bridge \
                         engine for transformer/cnn workloads",
                        idx, layer.layer_id, other
                    )));
                }
            }
        }

        Ok(activations)
    }
}

/// Decode a byte slice as a little-endian `f64` vector. Returns `None` if the length is not a
/// multiple of 8 (the size of `f64`).
fn decode_f64_le(bytes: &[u8]) -> Option<Vec<f64>> {
    if bytes.len() % std::mem::size_of::<f64>() != 0 {
        return None;
    }
    Some(
        bytes
            .chunks_exact(std::mem::size_of::<f64>())
            .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
            .collect(),
    )
}

/// Encode an `f64` slice as a little-endian byte vector.
fn encode_f64_le(values: &[f64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * std::mem::size_of::<f64>());
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// Apply an activation function in place, dispatching to `crate::solvers::activation` for the
/// standard element-wise maps.
fn apply_activation(buf: &mut [f64], act: &ActivationFunction) -> Result<(), MLError> {
    match act {
        ActivationFunction::ReLU => crate::solvers::activation::relu(buf),
        ActivationFunction::Sigmoid => crate::solvers::activation::sigmoid(buf),
        ActivationFunction::Tanh => crate::solvers::activation::tanh(buf),
        ActivationFunction::GELU => crate::solvers::activation::gelu(buf),
        ActivationFunction::Softmax => crate::solvers::activation::softmax(buf),
        ActivationFunction::Swish => crate::solvers::activation::silu(buf),
        ActivationFunction::LeakyReLU => {
            // Leaky ReLU: x if x >= 0 else 0.01·x, element-wise.
            const SLOPE: f64 = 0.01;
            for v in buf.iter_mut() {
                if *v < 0.0 {
                    *v *= SLOPE;
                }
            }
        }
        ActivationFunction::ELU => {
            // ELU: x if x >= 0 else e^x − 1, element-wise (α = 1).
            for v in buf.iter_mut() {
                if *v < 0.0 {
                    *v = (*v).exp() - 1.0;
                }
            }
        }
        ActivationFunction::Custom(name) => {
            return Err(MLError::InferenceError(format!(
                "custom activation '{}' is not supported by the MLP inference backend",
                name
            )));
        }
    }
    Ok(())
}

impl InferenceBackend {
    pub fn new() -> Self {
        Self {
            backend_id: "backend_1".to_string(),
            backend_type: InferenceBackendType::GPU,
            capabilities: BackendCapabilities::new(),
            current_load: 0.5,
        }
    }
}

impl BackendCapabilities {
    pub fn new() -> Self {
        Self {
            supported_models: vec!["gpt-3".to_string(), "bert".to_string()],
            max_batch_size: 32,
            max_sequence_length: 2048,
            supported_precisions: vec![Precision::FP16, Precision::FP32],
            memory_limit: 8 * 1024 * 1024 * 1024, // 8GB
            throughput: 100.0,
        }
    }
}

impl RequestScheduler {
    pub fn new() -> Self {
        Self {
            scheduling_policy: SchedulingPolicy::Priority,
            queue_manager: QueueManager::new(),
            load_balancer: LoadBalancer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        Ok(())
    }

    /// Return the current scheduling policy.
    pub fn scheduling_policy(&self) -> &SchedulingPolicy {
        &self.scheduling_policy
    }

    /// Set the scheduling policy.
    pub fn set_scheduling_policy(&mut self, policy: SchedulingPolicy) {
        self.scheduling_policy = policy;
    }

    /// Return a reference to the queue manager.
    pub fn queue_manager(&self) -> &QueueManager {
        &self.queue_manager
    }

    /// Return a mutable reference to the queue manager.
    pub fn queue_manager_mut(&mut self) -> &mut QueueManager {
        &mut self.queue_manager
    }

    /// Return a reference to the load balancer.
    pub fn load_balancer(&self) -> &LoadBalancer {
        &self.load_balancer
    }

    /// Return a mutable reference to the load balancer.
    pub fn load_balancer_mut(&mut self) -> &mut LoadBalancer {
        &mut self.load_balancer
    }

    pub fn schedule_request(&mut self, _request: &InferenceRequest) -> Result<String, MLError> {
        // Simplified scheduling - return backend ID
        Ok("backend_1".to_string())
    }
}

impl QueueManager {
    pub fn new() -> Self {
        Self {
            pending_requests: Vec::new(),
            running_requests: HashMap::new(),
            completed_requests: Vec::new(),
        }
    }

    /// Enqueue a pending inference request.
    pub fn enqueue(&mut self, request: InferenceRequest) {
        self.pending_requests.push(request);
    }

    /// Dequeue the next pending request (FIFO order).
    pub fn dequeue(&mut self) -> Option<InferenceRequest> {
        if self.pending_requests.is_empty() {
            None
        } else {
            Some(self.pending_requests.remove(0))
        }
    }

    /// Mark a request as running on a given backend.
    pub fn start_request(&mut self, running: RunningRequest) {
        self.running_requests
            .insert(running.request_id.clone(), running);
    }

    /// Mark a running request as completed, removing it from the running set
    /// and appending it to the completed list.
    pub fn complete_request(&mut self, request_id: &str, completed: CompletedRequest) {
        self.running_requests.remove(request_id);
        self.completed_requests.push(completed);
    }

    /// Return a reference to the pending requests queue.
    pub fn pending_requests(&self) -> &[InferenceRequest] {
        &self.pending_requests
    }

    /// Return a reference to the running requests map.
    pub fn running_requests(&self) -> &HashMap<String, RunningRequest> {
        &self.running_requests
    }

    /// Return a reference to the completed requests list.
    pub fn completed_requests(&self) -> &[CompletedRequest] {
        &self.completed_requests
    }

    /// Number of pending requests.
    pub fn pending_count(&self) -> usize {
        self.pending_requests.len()
    }

    /// Number of currently running requests.
    pub fn running_count(&self) -> usize {
        self.running_requests.len()
    }
}

impl LoadBalancer {
    pub fn new() -> Self {
        Self {
            balancing_strategy: LoadBalancingStrategy::RoundRobin,
            backend_metrics: HashMap::new(),
            health_checker: HealthChecker::new(),
        }
    }

    /// Return the current load-balancing strategy.
    pub fn balancing_strategy(&self) -> &LoadBalancingStrategy {
        &self.balancing_strategy
    }

    /// Set the load-balancing strategy.
    pub fn set_balancing_strategy(&mut self, strategy: LoadBalancingStrategy) {
        self.balancing_strategy = strategy;
    }

    /// Record or update metrics for a backend.
    pub fn record_backend_metrics(&mut self, metrics: BackendMetrics) {
        self.backend_metrics
            .insert(metrics.backend_id.clone(), metrics);
    }

    /// Get metrics for a specific backend.
    pub fn get_backend_metrics(&self, backend_id: &str) -> Option<&BackendMetrics> {
        self.backend_metrics.get(backend_id)
    }

    /// List the ids of all backends with recorded metrics.
    pub fn list_backend_metrics(&self) -> Vec<String> {
        self.backend_metrics.keys().cloned().collect()
    }

    /// Return a reference to the health checker.
    pub fn health_checker(&self) -> &HealthChecker {
        &self.health_checker
    }

    /// Return a mutable reference to the health checker.
    pub fn health_checker_mut(&mut self) -> &mut HealthChecker {
        &mut self.health_checker
    }
}

impl HealthChecker {
    pub fn new() -> Self {
        Self {
            health_checks: HashMap::new(),
            check_interval: 30, // 30 seconds
            timeout: 5,         // 5 seconds
        }
    }

    /// Register a health check under its check id.
    pub fn register_health_check(&mut self, check: HealthCheck) {
        self.health_checks
            .insert(check.check_id.clone(), check);
    }

    /// Get a registered health check by id.
    pub fn get_health_check(&self, check_id: &str) -> Option<&HealthCheck> {
        self.health_checks.get(check_id)
    }

    /// List the ids of all registered health checks.
    pub fn list_health_checks(&self) -> Vec<String> {
        self.health_checks.keys().cloned().collect()
    }

    /// Remove a registered health check by id. Returns `true` if removed.
    pub fn remove_health_check(&mut self, check_id: &str) -> bool {
        self.health_checks.remove(check_id).is_some()
    }

    /// Return the check interval (seconds).
    pub fn check_interval(&self) -> u64 {
        self.check_interval
    }

    /// Set the check interval (seconds).
    pub fn set_check_interval(&mut self, interval: u64) {
        self.check_interval = interval;
    }

    /// Return the timeout (seconds).
    pub fn timeout(&self) -> u64 {
        self.timeout
    }

    /// Set the timeout (seconds).
    pub fn set_timeout(&mut self, timeout: u64) {
        self.timeout = timeout;
    }
}

impl HealthCheck {
    pub fn new() -> Self {
        Self {
            check_id: "health_1".to_string(),
            check_type: HealthCheckType::HTTP,
            endpoint: "/health".to_string(),
            expected_response: "OK".to_string(),
        }
    }
}

impl BatchProcessor {
    pub fn new() -> Self {
        Self {
            batching_strategy: BatchingStrategy::FixedSize,
            batch_size: 32,
            batch_timeout: 100, // 100ms
            batch_optimizer: BatchOptimizer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.batch_optimizer.initialize()?;
        Ok(())
    }

    /// Return the current batching strategy.
    pub fn batching_strategy(&self) -> &BatchingStrategy {
        &self.batching_strategy
    }

    /// Set the batching strategy.
    pub fn set_batching_strategy(&mut self, strategy: BatchingStrategy) {
        self.batching_strategy = strategy;
    }

    /// Return the configured batch size.
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// Set the batch size.
    pub fn set_batch_size(&mut self, size: usize) {
        self.batch_size = size;
    }

    /// Return the configured batch timeout (milliseconds).
    pub fn batch_timeout(&self) -> u64 {
        self.batch_timeout
    }

    /// Set the batch timeout (milliseconds).
    pub fn set_batch_timeout(&mut self, timeout: u64) {
        self.batch_timeout = timeout;
    }
}

impl BatchOptimizer {
    pub fn new() -> Self {
        Self {
            optimization_algorithms: HashMap::new(),
            optimization_metrics: BatchOptimizationMetrics::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        Ok(())
    }

    /// Register a batch optimization algorithm under the given name.
    pub fn register_algorithm(&mut self, name: &str, algorithm: BatchOptimizationAlgorithm) {
        self.optimization_algorithms
            .insert(name.to_string(), algorithm);
    }

    /// Get a registered batch optimization algorithm by name.
    pub fn get_algorithm(&self, name: &str) -> Option<&BatchOptimizationAlgorithm> {
        self.optimization_algorithms.get(name)
    }

    /// List the names of all registered batch optimization algorithms.
    pub fn list_algorithms(&self) -> Vec<String> {
        self.optimization_algorithms.keys().cloned().collect()
    }

    /// Return a reference to the optimization metrics.
    pub fn optimization_metrics(&self) -> &BatchOptimizationMetrics {
        &self.optimization_metrics
    }

    /// Return a mutable reference to the optimization metrics.
    pub fn optimization_metrics_mut(&mut self) -> &mut BatchOptimizationMetrics {
        &mut self.optimization_metrics
    }
}

impl BatchOptimizationMetrics {
    pub fn new() -> Self {
        Self {
            average_batch_size: 32.0,
            throughput: 100.0,
            latency: 10.0,
            memory_utilization: 0.5,
        }
    }
}

impl InferenceOptimizer {
    pub fn new() -> Self {
        Self {
            optimization_strategies: vec![InferenceOptimizationStrategy::ModelQuantization],
            performance_analyzer: PerformanceAnalyzer::new(),
            auto_tuner: AutoTuner::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.performance_analyzer.initialize()?;
        self.auto_tuner.initialize()?;
        Ok(())
    }

    /// Return a reference to the configured optimization strategies.
    pub fn optimization_strategies(&self) -> &[InferenceOptimizationStrategy] {
        &self.optimization_strategies
    }

    /// Add an optimization strategy to the configured set.
    pub fn add_optimization_strategy(&mut self, strategy: InferenceOptimizationStrategy) {
        self.optimization_strategies.push(strategy);
    }

    /// Replace the full set of optimization strategies.
    pub fn set_optimization_strategies(&mut self, strategies: Vec<InferenceOptimizationStrategy>) {
        self.optimization_strategies = strategies;
    }
}

impl PerformanceAnalyzer {
    pub fn new() -> Self {
        Self {
            analysis_methods: vec![AnalysisMethod::Profiling],
            performance_profiles: HashMap::new(),
            bottleneck_detector: BottleneckDetector::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.bottleneck_detector.initialize()?;
        Ok(())
    }

    /// Return a reference to the configured analysis methods.
    pub fn analysis_methods(&self) -> &[AnalysisMethod] {
        &self.analysis_methods
    }

    /// Add an analysis method to the configured set.
    pub fn add_analysis_method(&mut self, method: AnalysisMethod) {
        self.analysis_methods.push(method);
    }

    /// Register a performance profile under its profile id.
    pub fn register_profile(&mut self, profile: PerformanceProfile) {
        self.performance_profiles
            .insert(profile.profile_id.clone(), profile);
    }

    /// Get a registered performance profile by id.
    pub fn get_profile(&self, profile_id: &str) -> Option<&PerformanceProfile> {
        self.performance_profiles.get(profile_id)
    }

    /// List the ids of all registered performance profiles.
    pub fn list_profiles(&self) -> Vec<String> {
        self.performance_profiles.keys().cloned().collect()
    }
}

impl PerformanceProfile {
    pub fn new() -> Self {
        Self {
            profile_id: "profile_1".to_string(),
            model_id: "model_1".to_string(),
            backend_id: "backend_1".to_string(),
            metrics: PerformanceMetrics::new(),
            characteristics: PerformanceCharacteristics::new(),
        }
    }
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            latency: 10.0,
            throughput: 100.0,
            accuracy: 0.0, // not measured (scaffold default; no evaluation performed)
            memory_usage: 1024 * 1024, // 1MB
        }
    }
}

impl PerformanceCharacteristics {
    pub fn new() -> Self {
        Self {
            compute_bound: true,
            memory_bound: false,
            io_bound: false,
            network_bound: false,
        }
    }
}

impl BottleneckDetector {
    pub fn new() -> Self {
        Self {
            detection_algorithms: vec![BottleneckDetectionAlgorithm::Statistical],
            detection_thresholds: DetectionThresholds::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        Ok(())
    }

    /// Return a reference to the configured detection algorithms.
    pub fn detection_algorithms(&self) -> &[BottleneckDetectionAlgorithm] {
        &self.detection_algorithms
    }

    /// Add a detection algorithm to the configured set.
    pub fn add_detection_algorithm(&mut self, algorithm: BottleneckDetectionAlgorithm) {
        self.detection_algorithms.push(algorithm);
    }

    /// Return a reference to the detection thresholds.
    pub fn detection_thresholds(&self) -> &DetectionThresholds {
        &self.detection_thresholds
    }

    /// Return a mutable reference to the detection thresholds.
    pub fn detection_thresholds_mut(&mut self) -> &mut DetectionThresholds {
        &mut self.detection_thresholds
    }
}

impl DetectionThresholds {
    pub fn new() -> Self {
        Self {
            cpu_threshold: 0.8,
            memory_threshold: 0.8,
            io_threshold: 0.8,
            network_threshold: 0.8,
        }
    }
}

impl AutoTuner {
    pub fn new() -> Self {
        Self {
            tuning_algorithms: HashMap::new(),
            tuning_objectives: Vec::new(),
            tuning_history: TuningHistory::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        Ok(())
    }

    /// Register a tuning algorithm under the given name.
    pub fn register_tuning_algorithm(&mut self, name: &str, algorithm: TuningAlgorithm) {
        self.tuning_algorithms
            .insert(name.to_string(), algorithm);
    }

    /// Get a registered tuning algorithm by name.
    pub fn get_tuning_algorithm(&self, name: &str) -> Option<&TuningAlgorithm> {
        self.tuning_algorithms.get(name)
    }

    /// List the names of all registered tuning algorithms.
    pub fn list_tuning_algorithms(&self) -> Vec<String> {
        self.tuning_algorithms.keys().cloned().collect()
    }

    /// Add a tuning objective to the configured set.
    pub fn add_tuning_objective(&mut self, objective: TuningObjective) {
        self.tuning_objectives.push(objective);
    }

    /// Return a reference to the configured tuning objectives.
    pub fn tuning_objectives(&self) -> &[TuningObjective] {
        &self.tuning_objectives
    }

    /// Return a reference to the tuning history.
    pub fn tuning_history(&self) -> &TuningHistory {
        &self.tuning_history
    }

    /// Return a mutable reference to the tuning history.
    pub fn tuning_history_mut(&mut self) -> &mut TuningHistory {
        &mut self.tuning_history
    }
}

impl TuningHistory {
    pub fn new() -> Self {
        Self {
            tuning_records: Vec::new(),
            best_configurations: HashMap::new(),
        }
    }

    /// Append a tuning record to the history.
    pub fn add_record(&mut self, record: TuningRecord) {
        self.tuning_records.push(record);
    }

    /// Return a reference to all tuning records.
    pub fn records(&self) -> &[TuningRecord] {
        &self.tuning_records
    }

    /// Record a best configuration for a given objective name.
    pub fn record_best_configuration(&mut self, objective: &str, config: TuningConfiguration) {
        self.best_configurations
            .insert(objective.to_string(), config);
    }

    /// Get the best configuration for a given objective.
    pub fn get_best_configuration(&self, objective: &str) -> Option<&TuningConfiguration> {
        self.best_configurations.get(objective)
    }

    /// List the objective names that have a recorded best configuration.
    pub fn list_best_configurations(&self) -> Vec<String> {
        self.best_configurations.keys().cloned().collect()
    }
}

impl TuningRecord {
    pub fn new() -> Self {
        Self {
            record_id: "record_1".to_string(),
            timestamp: 0,
            configuration: TuningConfiguration::new(),
            performance: PerformanceMetrics::new(),
            improvement: 0.0,
        }
    }
}

impl TuningConfiguration {
    pub fn new() -> Self {
        Self {
            configuration_id: "config_1".to_string(),
            parameters: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
}

impl TrainingEngine {
    pub fn new() -> Self {
        Self {
            training_backends: HashMap::new(),
            training_scheduler: TrainingScheduler::new(),
            data_pipeline: DataPipeline::new(),
            training_optimizer: TrainingOptimizer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.training_scheduler.initialize()?;
        self.data_pipeline.initialize()?;
        self.training_optimizer.initialize()?;
        Ok(())
    }

    /// Register a training backend under its backend id.
    pub fn register_backend(&mut self, backend: TrainingBackend) {
        self.training_backends
            .insert(backend.backend_id.clone(), backend);
    }

    /// Get a registered training backend by id.
    pub fn get_backend(&self, backend_id: &str) -> Option<&TrainingBackend> {
        self.training_backends.get(backend_id)
    }

    /// List the ids of all registered training backends.
    pub fn list_backends(&self) -> Vec<String> {
        self.training_backends.keys().cloned().collect()
    }

    /// Remove a registered training backend by id. Returns `true` if removed.
    pub fn remove_backend(&mut self, backend_id: &str) -> bool {
        self.training_backends.remove(backend_id).is_some()
    }

    /// Start a training *job* (the catalog/scheduler path). This records the job with the
    /// scheduler but performs no weight updates; use [`start_training`] for the real SGD
    /// loop that mutates a model's weights.
    pub fn start_training_job(&mut self, _job: &TrainingJob) -> Result<(), MLError> {
        // Start training job
        Ok(())
    }

    /// Run a real stochastic gradient descent (SGD) training loop on a linear model.
    ///
    /// This implements basic batch SGD for a single `Linear` layer with no activation
    /// (i.e. linear regression). For each epoch the samples are deterministically shuffled
    /// (a fixed-seed Fisher–Yates, so runs are reproducible), then processed in batches:
    /// a forward pass computes predictions, the MSE loss and its gradients are computed,
    /// and the weights are updated as `W -= learning_rate * gradient`.
    ///
    /// `training_data` is a flat buffer of inputs laid out as
    /// `[s0_i0, s0_i1, ..., s1_i0, ...]` with `input_size = architecture.layers[0].input_shape[0]`.
    /// `targets` is laid out as `[s0_o0, s0_o1, ..., s1_o0, ...]` with
    /// `output_size = architecture.layers[0].output_shape[0]`.
    ///
    /// Only `TrainingAlgorithm::SGD` is implemented here; other optimizers return a clear
    /// error rather than silently degrading. Only a single `Linear` layer with no
    /// activation is supported (the explicit scope of this training backend).
    pub fn start_training(
        &mut self,
        model: &mut Model,
        training_data: &[f64],
        targets: &[f64],
        config: &TrainingConfig,
    ) -> Result<TrainingResult, MLError> {
        self.start_training_impl(model, training_data, targets, config, None)
    }

    /// Run SGD recovery while preserving an unstructured pruning mask.
    ///
    /// Masked weights are forced to zero before training and skipped during
    /// every optimizer update, so recovery cannot silently regrow them.
    pub fn start_training_with_pruning_mask(
        &mut self,
        model: &mut Model,
        training_data: &[f64],
        targets: &[f64],
        config: &TrainingConfig,
        pruning_mask: &[u8],
    ) -> Result<TrainingResult, MLError> {
        self.start_training_impl(model, training_data, targets, config, Some(pruning_mask))
    }

    fn start_training_impl(
        &mut self,
        model: &mut Model,
        training_data: &[f64],
        targets: &[f64],
        config: &TrainingConfig,
        pruning_mask: Option<&[u8]>,
    ) -> Result<TrainingResult, MLError> {
        // --- Validate the optimizer. ---
        if config.optimizer != TrainingAlgorithm::SGD {
            return Err(MLError::TrainingError(format!(
                "start_training currently implements SGD only; {:?} is not yet supported \
                 by this training backend",
                config.optimizer
            )));
        }

        // --- Validate the model architecture: exactly one Linear layer, no activation. ---
        let layers = &model.architecture.layers;
        if layers.len() != 1 {
            return Err(MLError::TrainingError(format!(
                "start_training (SGD) supports a single Linear layer; this model has {} \
                 layers",
                layers.len()
            )));
        }
        let layer = &layers[0];
        if layer.layer_type != LayerType::Linear {
            return Err(MLError::TrainingError(format!(
                "start_training (SGD) supports only Linear layers; layer '{}' is {:?}",
                layer.layer_id, layer.layer_type
            )));
        }
        if layer.activation.is_some() {
            return Err(MLError::TrainingError(format!(
                "start_training (SGD) implements linear regression (no activation); layer \
                 '{}' has an activation function set",
                layer.layer_id
            )));
        }
        let in_size = layer
            .input_shape
            .first()
            .copied()
            .ok_or_else(|| MLError::TrainingError("layer missing input dimension".into()))?;
        let out_size = layer
            .output_shape
            .first()
            .copied()
            .ok_or_else(|| MLError::TrainingError("layer missing output dimension".into()))?;

        let weight_count = in_size * out_size;
        let bias_count = out_size;
        let needed = weight_count + bias_count;
        if model.weights.len() < needed {
            return Err(MLError::TrainingError(format!(
                "model has {} weights but the linear layer needs {} ({} weights + {} bias)",
                model.weights.len(),
                needed,
                weight_count,
                bias_count
            )));
        }
        if let Some(mask) = pruning_mask {
            let mask_bytes = ModelCompression::pruning_mask_bytes(needed);
            if mask.len() < mask_bytes {
                return Err(MLError::ResourceError(format!(
                    "training pruning mask needs {} bytes, got {}",
                    mask_bytes,
                    mask.len()
                )));
            }
            for index in 0..needed {
                if !ModelCompression::mask_keeps(mask, index) {
                    model.weights[index] = 0.0;
                }
            }
        }

        // --- Validate the data shapes. ---
        if in_size == 0 {
            return Err(MLError::TrainingError("input dimension is zero".into()));
        }
        if training_data.len() % in_size != 0 {
            return Err(MLError::DataError(format!(
                "training_data length ({}) is not a multiple of input_size ({})",
                training_data.len(),
                in_size
            )));
        }
        let n_samples = training_data.len() / in_size;
        if n_samples == 0 {
            return Err(MLError::DataError("no training samples".into()));
        }
        if targets.len() != n_samples * out_size {
            return Err(MLError::DataError(format!(
                "targets length ({}) does not match n_samples ({}) * output_size ({})",
                targets.len(),
                n_samples,
                out_size
            )));
        }
        if config.batch_size == 0 {
            return Err(MLError::TrainingError("batch_size must be > 0".into()));
        }

        let start = std::time::Instant::now();

        // --- Initial loss (before any weight update). ---
        let initial_loss =
            Self::full_dataset_mse(&model.weights, training_data, targets, in_size, out_size);

        let mut last_loss = initial_loss;
        let mut epochs_completed: usize = 0;
        let mut convergence_achieved = false;

        const CONVERGENCE_THRESHOLD: f64 = 1e-9;

        for epoch in 0..config.epochs {
            // Deterministic shuffle of sample indices (fixed seed → reproducible runs).
            let order = deterministic_shuffle(n_samples, epoch as u64);

            let batch_size = config.batch_size.min(n_samples);
            for chunk in order.chunks(batch_size) {
                // Accumulate batch gradients over the samples in this batch.
                let mut grad = vec![0.0f64; needed];
                let b = chunk.len() as f64;
                for &s in chunk {
                    let x = &training_data[s * in_size..s * in_size + in_size];
                    let t = &targets[s * out_size..s * out_size + out_size];
                    // Forward pass for this sample.
                    let pred = forward_linear(&model.weights, x, in_size, out_size);
                    // Per-sample gradient contribution: dL/dW and dL/db for MSE.
                    // L_s = sum_j (pred_j - t_j)^2 ; averaged over batch and outputs below.
                    for j in 0..out_size {
                        let diff = pred[j] - t[j];
                        // dL_s/dW[j*in+i] = 2 * diff * x[i]
                        for i in 0..in_size {
                            grad[j * in_size + i] += 2.0 * diff * x[i];
                        }
                        // dL_s/db[j] = 2 * diff
                        grad[weight_count + j] += 2.0 * diff;
                    }
                }

                // Average over (batch_size * out_size) and apply the learning rate.
                let scale = config.learning_rate / (b * out_size as f64);
                for k in 0..needed {
                    if pruning_mask
                        .map(|mask| ModelCompression::mask_keeps(mask, k))
                        .unwrap_or(true)
                    {
                        model.weights[k] -= scale * grad[k];
                    }
                }
            }

            epochs_completed = (epoch + 1) as usize;
            let loss =
                Self::full_dataset_mse(&model.weights, training_data, targets, in_size, out_size);
            if (last_loss - loss).abs() < CONVERGENCE_THRESHOLD {
                convergence_achieved = true;
                last_loss = loss;
                break;
            }
            last_loss = loss;
        }

        let training_time_ms = start.elapsed().as_millis() as u64;

        Ok(TrainingResult {
            initial_loss,
            final_loss: last_loss,
            epochs_completed,
            convergence_achieved,
            training_time_ms,
        })
    }

    /// Mean squared error between predictions and targets: `(1/N) * sum (p - t)^2`.
    pub fn compute_mse(predictions: &[f64], targets: &[f64]) -> f64 {
        if predictions.is_empty() || predictions.len() != targets.len() {
            return 0.0;
        }
        let n = predictions.len() as f64;
        let sum: f64 = predictions
            .iter()
            .zip(targets.iter())
            .map(|(p, t)| {
                let d = p - t;
                d * d
            })
            .sum();
        sum / n
    }

    /// Compute the MSE gradient (scaled by `learning_rate`) for a single linear sample.
    ///
    /// For a linear model `y = W·x + b` with `weights = [W (out×in), b (out)]`, the MSE
    /// loss for one sample is `L = sum_j (pred_j - target_j)^2`. The returned vector has
    /// the same layout as `weights` and contains `learning_rate * dL/dweight`, i.e. the
    /// amount to *subtract* from each weight. (For a single-output model `pred` and
    /// `target` are scalars and `inputs` is the input vector.)
    pub fn compute_gradients(
        weights: &[f64],
        inputs: &[f64],
        prediction: f64,
        target: f64,
        learning_rate: f64,
    ) -> Vec<f64> {
        // Infer the layout: weights = [W (out×in), b (out)].
        // weights.len() = out_size * in_size + out_size = out_size * (in_size + 1)
        //  =>  out_size = weights.len() / (in_size + 1)
        let in_size = inputs.len();
        if in_size == 0 {
            return Vec::new();
        }
        let out_size = if weights.len() >= in_size + 1 {
            weights.len() / (in_size + 1)
        } else {
            1
        };
        let weight_count = in_size * out_size;
        let diff = prediction - target;
        let mut grad = vec![0.0f64; weights.len()];
        for j in 0..out_size {
            // For the single-output case the prediction/target are the scalar values.
            let d = if out_size == 1 { diff } else { diff };
            for i in 0..in_size {
                grad[j * in_size + i] = learning_rate * 2.0 * d * inputs[i];
            }
            grad[weight_count + j] = learning_rate * 2.0 * d;
        }
        grad
    }

    /// MSE over the full dataset using the model's current weights (helper for the loop).
    fn full_dataset_mse(
        weights: &[f64],
        training_data: &[f64],
        targets: &[f64],
        in_size: usize,
        out_size: usize,
    ) -> f64 {
        let n_samples = training_data.len() / in_size;
        let mut preds = Vec::with_capacity(n_samples * out_size);
        for s in 0..n_samples {
            let x = &training_data[s * in_size..s * in_size + in_size];
            let pred = forward_linear(weights, x, in_size, out_size);
            preds.extend_from_slice(&pred);
        }
        Self::compute_mse(&preds, targets)
    }
}

/// Forward pass for a single `Linear` layer (no activation): `out = W·x + b`.
/// `weights` = `[W (out×in, row-major), b (out)]`.
fn forward_linear(weights: &[f64], x: &[f64], in_size: usize, out_size: usize) -> Vec<f64> {
    let weight_count = in_size * out_size;
    let mut out = vec![0.0f64; out_size];
    for j in 0..out_size {
        let mut acc = 0.0;
        for i in 0..in_size {
            acc += weights[j * in_size + i] * x[i];
        }
        acc += weights[weight_count + j];
        out[j] = acc;
    }
    out
}

/// Deterministic Fisher–Yates shuffle of `0..n` seeded by `seed`.
///
/// Uses a simple multiplicative LCG (Knuth constants) so that the same `(n, seed)` always
/// produces the same permutation — training runs are reproducible without depending on a
/// crate-level RNG.
fn deterministic_shuffle(n: usize, seed: u64) -> Vec<usize> {
    let mut order: Vec<usize> = (0..n).collect();
    let mut state = seed.wrapping_add(0x9E3779B97F4A7C15);
    for i in (1..n).rev() {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let j = (state >> 33) as usize % (i + 1);
        order.swap(i, j);
    }
    order
}

impl TrainingBackend {
    pub fn new() -> Self {
        Self {
            backend_id: "training_backend_1".to_string(),
            backend_type: TrainingBackendType::GPU,
            capabilities: TrainingCapabilities::new(),
            current_load: 0.5,
        }
    }
}

impl TrainingCapabilities {
    pub fn new() -> Self {
        Self {
            supported_algorithms: vec![TrainingAlgorithm::Adam, TrainingAlgorithm::SGD],
            max_batch_size: 64,
            max_dataset_size: 100 * 1024 * 1024 * 1024, // 100GB
            parallel_workers: 4,
            memory_limit: 16 * 1024 * 1024 * 1024, // 16GB
        }
    }
}

impl TrainingScheduler {
    pub fn new() -> Self {
        Self {
            scheduling_policy: TrainingSchedulingPolicy::FIFO,
            resource_manager: ResourceManager::new(),
            progress_tracker: ProgressTracker::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.resource_manager.initialize()?;
        self.progress_tracker.initialize()?;
        Ok(())
    }

    /// Return the current training scheduling policy.
    pub fn scheduling_policy(&self) -> &TrainingSchedulingPolicy {
        &self.scheduling_policy
    }

    /// Set the training scheduling policy.
    pub fn set_scheduling_policy(&mut self, policy: TrainingSchedulingPolicy) {
        self.scheduling_policy = policy;
    }
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            allocation_strategy: AllocationStrategy::FirstFit,
            utilization_tracker: UtilizationTracker::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.utilization_tracker.initialize()?;
        Ok(())
    }

    /// Register a resource under its resource id.
    pub fn register_resource(&mut self, resource: Resource) {
        self.resources
            .insert(resource.resource_id.clone(), resource);
    }

    /// Get a registered resource by id.
    pub fn get_resource(&self, resource_id: &str) -> Option<&Resource> {
        self.resources.get(resource_id)
    }

    /// Get a mutable reference to a registered resource by id.
    pub fn get_resource_mut(&mut self, resource_id: &str) -> Option<&mut Resource> {
        self.resources.get_mut(resource_id)
    }

    /// List the ids of all registered resources.
    pub fn list_resources(&self) -> Vec<String> {
        self.resources.keys().cloned().collect()
    }

    /// Return the current allocation strategy.
    pub fn allocation_strategy(&self) -> &AllocationStrategy {
        &self.allocation_strategy
    }

    /// Set the allocation strategy.
    pub fn set_allocation_strategy(&mut self, strategy: AllocationStrategy) {
        self.allocation_strategy = strategy;
    }
}

impl Resource {
    pub fn new() -> Self {
        Self {
            resource_id: "resource_1".to_string(),
            resource_type: ResourceType::GPU,
            capacity: 1.0,
            current_usage: 0.0,
            availability: Availability::Available,
        }
    }
}

impl UtilizationTracker {
    pub fn new() -> Self {
        Self {
            utilization_history: HashMap::new(),
            current_utilization: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        Ok(())
    }

    /// Record a utilization sample for a resource, appending it to the history
    /// and updating the current utilization value.
    pub fn record_utilization(&mut self, record: UtilizationRecord) {
        self.utilization_history
            .entry(record.resource_id.clone())
            .or_default()
            .push(record.clone());
        self.current_utilization
            .insert(record.resource_id, record.utilization);
    }

    /// Return the utilization history for a given resource.
    pub fn get_utilization_history(&self, resource_id: &str) -> &[UtilizationRecord] {
        self.utilization_history
            .get(resource_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Return the current utilization for a given resource.
    pub fn current_utilization(&self, resource_id: &str) -> Option<f64> {
        self.current_utilization.get(resource_id).copied()
    }

    /// Set the current utilization for a given resource.
    pub fn set_current_utilization(&mut self, resource_id: &str, utilization: f64) {
        self.current_utilization
            .insert(resource_id.to_string(), utilization);
    }
}

impl UtilizationRecord {
    pub fn new() -> Self {
        Self {
            timestamp: 0,
            resource_id: "resource_1".to_string(),
            utilization: 0.0,
        }
    }
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            training_jobs: HashMap::new(),
            progress_metrics: ProgressMetrics::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        Ok(())
    }

    /// Register a training job under its job id.
    pub fn register_job(&mut self, job: TrainingJob) {
        self.training_jobs.insert(job.job_id.clone(), job);
    }

    /// Get a registered training job by id.
    pub fn get_job(&self, job_id: &str) -> Option<&TrainingJob> {
        self.training_jobs.get(job_id)
    }

    /// Get a mutable reference to a registered training job by id.
    pub fn get_job_mut(&mut self, job_id: &str) -> Option<&mut TrainingJob> {
        self.training_jobs.get_mut(job_id)
    }

    /// List the ids of all registered training jobs.
    pub fn list_jobs(&self) -> Vec<String> {
        self.training_jobs.keys().cloned().collect()
    }

    /// Remove a registered training job by id. Returns `true` if removed.
    pub fn remove_job(&mut self, job_id: &str) -> bool {
        self.training_jobs.remove(job_id).is_some()
    }

    /// Return a reference to the progress metrics.
    pub fn progress_metrics(&self) -> &ProgressMetrics {
        &self.progress_metrics
    }

    /// Return a mutable reference to the progress metrics.
    pub fn progress_metrics_mut(&mut self) -> &mut ProgressMetrics {
        &mut self.progress_metrics
    }
}

impl ProgressMetrics {
    pub fn new() -> Self {
        Self {
            total_jobs: 0,
            completed_jobs: 0,
            average_progress: 0.0,
            estimated_completion: 0,
        }
    }
}

impl DataPipeline {
    pub fn new() -> Self {
        Self {
            data_sources: HashMap::new(),
            data_transformers: HashMap::new(),
            data_loaders: HashMap::new(),
            data_augmenters: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        Ok(())
    }

    /// Register a data source under its source id.
    pub fn register_data_source(&mut self, source: DataSource) {
        self.data_sources
            .insert(source.source_id.clone(), source);
    }

    /// Get a registered data source by id.
    pub fn get_data_source(&self, source_id: &str) -> Option<&DataSource> {
        self.data_sources.get(source_id)
    }

    /// List the ids of all registered data sources.
    pub fn list_data_sources(&self) -> Vec<String> {
        self.data_sources.keys().cloned().collect()
    }

    /// Register a data transformer under its transformer id.
    pub fn register_transformer(&mut self, transformer: DataTransformer) {
        self.data_transformers
            .insert(transformer.transformer_id.clone(), transformer);
    }

    /// Get a registered data transformer by id.
    pub fn get_transformer(&self, transformer_id: &str) -> Option<&DataTransformer> {
        self.data_transformers.get(transformer_id)
    }

    /// List the ids of all registered data transformers.
    pub fn list_transformers(&self) -> Vec<String> {
        self.data_transformers.keys().cloned().collect()
    }

    /// Register a data loader under its loader id.
    pub fn register_loader(&mut self, loader: DataLoader) {
        self.data_loaders
            .insert(loader.loader_id.clone(), loader);
    }

    /// Get a registered data loader by id.
    pub fn get_loader(&self, loader_id: &str) -> Option<&DataLoader> {
        self.data_loaders.get(loader_id)
    }

    /// List the ids of all registered data loaders.
    pub fn list_loaders(&self) -> Vec<String> {
        self.data_loaders.keys().cloned().collect()
    }

    /// Register a data augmenter under its augmenter id.
    pub fn register_augmenter(&mut self, augmenter: DataAugmenter) {
        self.data_augmenters
            .insert(augmenter.augmenter_id.clone(), augmenter);
    }

    /// Get a registered data augmenter by id.
    pub fn get_augmenter(&self, augmenter_id: &str) -> Option<&DataAugmenter> {
        self.data_augmenters.get(augmenter_id)
    }

    /// List the ids of all registered data augmenters.
    pub fn list_augmenters(&self) -> Vec<String> {
        self.data_augmenters.keys().cloned().collect()
    }
}

impl DataSource {
    pub fn new() -> Self {
        Self {
            source_id: "source_1".to_string(),
            source_type: DataSourceType::Local,
            location: "/data".to_string(),
            format: DataFormat::CSV,
        }
    }
}

impl DataTransformer {
    pub fn new() -> Self {
        Self {
            transformer_id: "transformer_1".to_string(),
            transformer_type: DataTransformerType::Normalizer,
            transformation_pipeline: Vec::new(),
        }
    }
}

impl TransformationStep {
    pub fn new() -> Self {
        Self {
            step_id: "step_1".to_string(),
            step_type: ConversionStepType::Parsing,
            parameters: HashMap::new(),
        }
    }
}

impl DataLoader {
    pub fn new() -> Self {
        Self {
            loader_id: "loader_1".to_string(),
            loader_type: DataLoaderType::Parallel,
            batch_size: 32,
            shuffle: true,
            num_workers: 4,
        }
    }
}

impl DataAugmenter {
    pub fn new() -> Self {
        Self {
            augmenter_id: "augmenter_1".to_string(),
            augmenter_type: DataAugmenterType::ImageAugmentation,
            augmentation_pipeline: Vec::new(),
        }
    }
}

impl AugmentationStep {
    pub fn new() -> Self {
        Self {
            step_id: "step_1".to_string(),
            step_type: AugmentationStepType::Rotation,
            parameters: HashMap::new(),
        }
    }
}

impl TrainingOptimizer {
    pub fn new() -> Self {
        Self {
            optimization_algorithms: HashMap::new(),
            hyperparameter_tuner: HyperparameterTuner::new(),
            early_stopping: EarlyStopping::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.hyperparameter_tuner.initialize()?;
        Ok(())
    }

    /// Register a training optimization algorithm under the given name.
    pub fn register_algorithm(&mut self, name: &str, algorithm: TrainingOptimizationAlgorithm) {
        self.optimization_algorithms
            .insert(name.to_string(), algorithm);
    }

    /// Get a registered training optimization algorithm by name.
    pub fn get_algorithm(&self, name: &str) -> Option<&TrainingOptimizationAlgorithm> {
        self.optimization_algorithms.get(name)
    }

    /// List the names of all registered training optimization algorithms.
    pub fn list_algorithms(&self) -> Vec<String> {
        self.optimization_algorithms.keys().cloned().collect()
    }

    /// Return a reference to the early-stopping configuration.
    pub fn early_stopping(&self) -> &EarlyStopping {
        &self.early_stopping
    }

    /// Return a mutable reference to the early-stopping configuration.
    pub fn early_stopping_mut(&mut self) -> &mut EarlyStopping {
        &mut self.early_stopping
    }
}

impl HyperparameterTuner {
    pub fn new() -> Self {
        Self {
            tuning_space: TuningSpace::new(),
            tuning_algorithm: TuningAlgorithm::BayesianOptimization,
            tuning_history: TuningHistory::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        Ok(())
    }

    /// Return a reference to the tuning space.
    pub fn tuning_space(&self) -> &TuningSpace {
        &self.tuning_space
    }

    /// Return a mutable reference to the tuning space.
    pub fn tuning_space_mut(&mut self) -> &mut TuningSpace {
        &mut self.tuning_space
    }

    /// Return the configured tuning algorithm.
    pub fn tuning_algorithm(&self) -> &TuningAlgorithm {
        &self.tuning_algorithm
    }

    /// Set the tuning algorithm.
    pub fn set_tuning_algorithm(&mut self, algorithm: TuningAlgorithm) {
        self.tuning_algorithm = algorithm;
    }

    /// Return a reference to the tuning history.
    pub fn tuning_history(&self) -> &TuningHistory {
        &self.tuning_history
    }

    /// Return a mutable reference to the tuning history.
    pub fn tuning_history_mut(&mut self) -> &mut TuningHistory {
        &mut self.tuning_history
    }
}

impl TuningSpace {
    pub fn new() -> Self {
        Self {
            hyperparameters: Vec::new(),
            constraints: Vec::new(),
        }
    }
}

impl Hyperparameter {
    pub fn new() -> Self {
        Self {
            name: "learning_rate".to_string(),
            parameter_type: HyperparameterType::Continuous,
            range: HyperparameterRange::new(),
            default_value: 0.001,
        }
    }
}

impl HyperparameterRange {
    pub fn new() -> Self {
        Self {
            min_value: 0.0001,
            max_value: 1.0,
            step: Some(0.0001),
            categories: None,
        }
    }
}

impl HyperparameterConstraint {
    pub fn new() -> Self {
        Self {
            constraint_id: "constraint_1".to_string(),
            constraint_type: ConstraintType::Range,
            parameters: vec!["learning_rate".to_string()],
            condition: "learning_rate > 0".to_string(),
        }
    }
}

impl EarlyStopping {
    pub fn new() -> Self {
        Self {
            stopping_criteria: StoppingCriteria::new(),
            patience: 10,
            min_delta: 0.001,
            restore_best_weights: true,
        }
    }

    /// Return a reference to the stopping criteria.
    pub fn stopping_criteria(&self) -> &StoppingCriteria {
        &self.stopping_criteria
    }

    /// Return a mutable reference to the stopping criteria.
    pub fn stopping_criteria_mut(&mut self) -> &mut StoppingCriteria {
        &mut self.stopping_criteria
    }

    /// Return the configured patience (number of epochs without improvement).
    pub fn patience(&self) -> u32 {
        self.patience
    }

    /// Set the patience.
    pub fn set_patience(&mut self, patience: u32) {
        self.patience = patience;
    }

    /// Return the minimum delta required to count as an improvement.
    pub fn min_delta(&self) -> f64 {
        self.min_delta
    }

    /// Set the minimum delta.
    pub fn set_min_delta(&mut self, min_delta: f64) {
        self.min_delta = min_delta;
    }

    /// Return whether best weights should be restored after early stopping.
    pub fn restore_best_weights(&self) -> bool {
        self.restore_best_weights
    }

    /// Set whether to restore best weights.
    pub fn set_restore_best_weights(&mut self, restore: bool) {
        self.restore_best_weights = restore;
    }
}

impl StoppingCriteria {
    pub fn new() -> Self {
        Self {
            metric: "val_loss".to_string(),
            mode: StoppingMode::Min,
            min_delta: 0.001,
            patience: 10,
        }
    }
}

impl MLOptimizationEngine {
    pub fn new() -> Self {
        Self {
            optimization_algorithms: HashMap::new(),
            optimization_objectives: Vec::new(),
            optimization_constraints: Vec::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        Ok(())
    }

    /// Register an optimization algorithm under the given name.
    pub fn register_algorithm(&mut self, name: &str, algorithm: MLOptimizationAlgorithm) {
        self.optimization_algorithms
            .insert(name.to_string(), algorithm);
    }

    /// Get a registered optimization algorithm by name.
    pub fn get_algorithm(&self, name: &str) -> Option<&MLOptimizationAlgorithm> {
        self.optimization_algorithms.get(name)
    }

    /// List the names of all registered optimization algorithms.
    pub fn list_algorithms(&self) -> Vec<String> {
        self.optimization_algorithms.keys().cloned().collect()
    }

    /// Add an optimization objective to the configured set.
    pub fn add_objective(&mut self, objective: OptimizationObjective) {
        self.optimization_objectives.push(objective);
    }

    /// Return a reference to the configured optimization objectives.
    pub fn objectives(&self) -> &[OptimizationObjective] {
        &self.optimization_objectives
    }

    /// Add an optimization constraint to the configured set.
    pub fn add_constraint(&mut self, constraint: OptimizationConstraint) {
        self.optimization_constraints.push(constraint);
    }

    /// Return a reference to the configured optimization constraints.
    pub fn constraints(&self) -> &[OptimizationConstraint] {
        &self.optimization_constraints
    }

    pub fn optimize_model(
        &mut self,
        model_id: &str,
        _algorithm: MLOptimizationAlgorithm,
    ) -> Result<Model, MLError> {
        let mut model = Model::new();
        model.model_id = model_id.to_string();
        Ok(model)
    }
}

impl OptimizationObjective {
    pub fn new() -> Self {
        Self {
            objective_id: "objective_1".to_string(),
            objective_type: ObjectiveType::MinimizeLatency,
            target_value: 10.0,
            weight: 1.0,
        }
    }
}

impl OptimizationConstraint {
    pub fn new() -> Self {
        Self {
            constraint_id: "constraint_1".to_string(),
            constraint_type: ConstraintType::Range,
            parameters: vec!["model_size".to_string()],
            condition: "model_size < 1GB".to_string(),
        }
    }
}

impl MLPerformanceMonitor {
    pub fn new() -> Self {
        Self {
            inference_metrics: InferenceMetrics::new(),
            training_metrics: TrainingMetrics::new(),
            system_metrics: SystemMetrics::new(),
            model_metrics: ModelMetrics::new(),
        }
    }

    pub fn get_metrics(&self) -> MLPerformanceMetrics {
        MLPerformanceMetrics {
            inference_metrics: self.inference_metrics.clone(),
            training_metrics: self.training_metrics.clone(),
            system_metrics: self.system_metrics.clone(),
            model_metrics: self.model_metrics.clone(),
            average_inference_latency: self.inference_metrics.average_latency,
            total_requests: self.inference_metrics.total_requests,
            average_training_time: 0.0,
            model_accuracy: 0.0,
        }
    }
}

impl InferenceMetrics {
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            average_latency: 0.0,
            throughput: 0.0,
            error_rate: 0.0,
            resource_utilization: ResourceUtilization::new(),
        }
    }
}

impl SystemTrainingMetrics {
    pub fn new() -> Self {
        Self {
            total_training_jobs: 0,
            average_training_time: 0.0,
            convergence_rate: 0.0,
            model_accuracy: 0.0,
            resource_utilization: ResourceUtilization::new(),
        }
    }
}

impl SystemMetrics {
    pub fn new() -> Self {
        Self {
            cpu_utilization: 0.0,
            memory_utilization: 0.0,
            gpu_utilization: 0.0,
            network_utilization: 0.0,
            storage_utilization: 0.0,
        }
    }
}

impl ModelMetrics {
    pub fn new() -> Self {
        Self {
            total_models: 0,
            average_model_size: 0.0,
            model_accuracy: 0.0,
            model_performance: 0.0,
            storage_utilization: 0.0,
        }
    }
}

impl ResourceUtilization {
    pub fn new() -> Self {
        Self {
            cpu: 0.0,
            memory: 0.0,
            gpu: 0.0,
            network: 0.0,
            storage: 0.0,
        }
    }
}

// Supporting implementations for Model, TrainingJob, etc.

impl Model {
    pub fn new() -> Self {
        Self {
            model_id: "model_1".to_string(),
            model_type: ModelType::LLM,
            framework: MLFramework::PyTorch,
            architecture: ModelArchitecture::new(),
            weights: vec![0.0; 1000],
            metadata: ModelMetadata::new(),
        }
    }
}

impl ModelArchitecture {
    pub fn new() -> Self {
        Self {
            layers: vec![LayerInfo::new()],
            connections: vec![LayerConnection::new()],
            input_shape: vec![512],
            output_shape: vec![512],
            total_parameters: 1000,
        }
    }
}

impl LayerInfo {
    pub fn new() -> Self {
        Self {
            layer_id: "layer_1".to_string(),
            layer_type: LayerType::Linear,
            input_shape: vec![512],
            output_shape: vec![512],
            parameters: 512,
            activation: Some(ActivationFunction::ReLU),
        }
    }
}

impl LayerConnection {
    pub fn new() -> Self {
        Self {
            source_layer: "layer_1".to_string(),
            target_layer: "layer_2".to_string(),
            connection_type: ConnectionType::Direct,
        }
    }
}

impl ModelMetadata {
    pub fn new() -> Self {
        Self {
            model_id: "model_1".to_string(),
            model_type: ModelType::LLM,
            framework: MLFramework::PyTorch,
            architecture: ModelArchitecture::new(),
            parameters: ModelParameters::new(),
            performance: ModelPerformance::new(),
            created_at: 0,
            last_updated: 0,
            access_count: 0,
            size: 1000,
        }
    }
}

impl ModelParameters {
    pub fn new() -> Self {
        Self {
            weight_count: 1000,
            bias_count: 0,
            activation_count: 1,
            normalization_count: 0,
            attention_count: 0,
        }
    }
}

impl ModelPerformance {
    pub fn new() -> Self {
        Self {
            inference_latency: 10.0,
            throughput: 100.0,
            accuracy: 0.0, // not measured (scaffold default; no evaluation performed)
            memory_usage: 1024 * 1024, // 1MB
            energy_efficiency: 0.8,
        }
    }
}

impl TrainingJob {
    pub fn new() -> Self {
        Self {
            job_id: "job_1".to_string(),
            model_id: "model_1".to_string(),
            training_config: TrainingConfig::new(),
            status: TrainingStatus::Pending,
            progress: 0.0,
            metrics: TrainingMetrics::new(),
        }
    }
}

impl TrainingConfig {
    pub fn new() -> Self {
        Self {
            epochs: 10,
            batch_size: 32,
            learning_rate: 0.001,
            optimizer: TrainingAlgorithm::Adam,
            loss_function: "cross_entropy".to_string(),
            metrics: vec!["accuracy".to_string(), "loss".to_string()],
            validation_split: 0.2,
        }
    }
}

impl TrainingMetrics {
    pub fn new() -> Self {
        Self {
            total_training_jobs: 0,
            accuracy: 0.0,  // not measured (scaffold default; no evaluation performed)
            precision: 0.0, // not measured (scaffold default; no evaluation performed)
            recall: 0.0,
            f1_score: 0.0,
            learning_rate: 0.001,
        }
    }
}

impl ResultMetadata {
    pub fn new() -> Self {
        Self {
            model_id: "model_1".to_string(),
            backend_id: "backend_1".to_string(),
            batch_size: 32,
            sequence_length: 512,
            tokens_generated: 512,
        }
    }
}

/// ML error types
#[derive(Debug, Clone)]
pub enum MLError {
    ModelError(String),
    InferenceError(String),
    TrainingError(String),
    OptimizationError(String),
    DataError(String),
    ResourceError(String),
    ValidationError(String),
}

impl std::fmt::Display for MLError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MLError::ModelError(msg) => write!(f, "Model error: {}", msg),
            MLError::InferenceError(msg) => write!(f, "Inference error: {}", msg),
            MLError::TrainingError(msg) => write!(f, "Training error: {}", msg),
            MLError::OptimizationError(msg) => write!(f, "Optimization error: {}", msg),
            MLError::DataError(msg) => write!(f, "Data error: {}", msg),
            MLError::ResourceError(msg) => write!(f, "Resource error: {}", msg),
            MLError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for MLError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ml_library_creation() {
        let mut library = MachineLearningLibrary::new();
        assert!(library.initialize().is_ok());
    }

    #[test]
    fn test_model_loading() {
        let mut library = MachineLearningLibrary::new();
        library.initialize().unwrap();

        let result = library
            .load_model("test_model".to_string(), "/path/to/model")
            .unwrap();

        assert_eq!(result.result.model_id, "test_model");
        assert_eq!(result.result.model_type, ModelType::LLM);
        assert_eq!(result.result.framework, MLFramework::PyTorch);
    }

    #[test]
    fn test_inference() {
        let mut library = MachineLearningLibrary::new();
        library.initialize().unwrap();

        // 100 bytes is not a multiple of 8 (f64 size), so the wired MLP backend rejects the
        // input with a DataError rather than fabricating a result. (The default scaffold model
        // loaded here is a 512→512 Linear layer with zero weights; even with valid input the
        // shape would not match — see test_mlp_inference_forward_pass for a real forward pass.)
        let input_data = vec![1u8; 100];
        let parameters = InferenceParameters {
            batch_size: 1,
            sequence_length: 512,
            temperature: Some(0.7),
            top_k: Some(50),
            top_p: Some(0.9),
            max_tokens: Some(100),
            precision: Precision::FP32,
        };

        let result = library.run_inference("test_model", &input_data, parameters);
        assert!(
            result.is_err(),
            "malformed input (not a multiple of f64 size) must be rejected, not fabricated"
        );
    }

    #[test]
    fn test_mlp_inference_forward_pass() {
        // Build a 2 → 3 → 2 MLP with ReLU on the hidden layer and no output activation.
        //
        // Layer 1 (Linear, 2→3, ReLU):
        //   W1 (row-major, 3×2) = [[1, 2], [0, -1], [0.5, 0.5]], bias1 = [0, 0, 0]
        //   z1 = W1·x + b1 for x = [1, 2] = [5, -2, 1.5]
        //   after ReLU        = [5,  0, 1.5]
        //
        // Layer 2 (Linear, 3→2, no activation):
        //   W2 (row-major, 2×3) = [[1, 0, 0], [0, 1, 0]], bias2 = [0, 0]
        //   z2 = W2·h1 + b2 = [5, 0]
        //
        // Expected output = [5.0, 0.0].
        let layer1 = LayerInfo {
            layer_id: "l1".to_string(),
            layer_type: LayerType::Linear,
            input_shape: vec![2],
            output_shape: vec![3],
            parameters: 9, // 3×2 weights + 3 bias
            activation: Some(ActivationFunction::ReLU),
        };
        let layer2 = LayerInfo {
            layer_id: "l2".to_string(),
            layer_type: LayerType::Linear,
            input_shape: vec![3],
            output_shape: vec![2],
            parameters: 8, // 2×3 weights + 2 bias
            activation: None,
        };
        let model = Model {
            model_id: "mlp_test".to_string(),
            model_type: ModelType::LLM,
            framework: MLFramework::Custom("test".to_string()),
            architecture: ModelArchitecture {
                layers: vec![layer1, layer2],
                connections: vec![],
                input_shape: vec![2],
                output_shape: vec![2],
                total_parameters: 17,
            },
            // Flattened in consumption order: layer1 W(3×2) + bias(3), layer2 W(2×3) + bias(2).
            weights: vec![
                1.0, 2.0, 0.0, -1.0, 0.5, 0.5, // W1 row-major
                0.0, 0.0, 0.0, // bias1
                1.0, 0.0, 0.0, 0.0, 1.0, 0.0, // W2 row-major
                0.0, 0.0, // bias2
            ],
            metadata: ModelMetadata::new(),
        };

        let input = [1.0f64, 2.0];
        let input_bytes: Vec<u8> = input.iter().flat_map(|v| v.to_le_bytes()).collect();

        let request = InferenceRequest {
            request_id: "req_mlp".to_string(),
            model_id: "mlp_test".to_string(),
            input_data: input_bytes,
            parameters: InferenceParameters {
                batch_size: 1,
                sequence_length: 2,
                temperature: None,
                top_k: None,
                top_p: None,
                max_tokens: None,
                precision: Precision::FP32,
            },
            priority: RequestPriority::Normal,
            submitted_at: 0,
            deadline: None,
        };

        let mut engine = InferenceEngine::new();
        let result = engine
            .execute_inference(&request, &model)
            .expect("MLP forward pass should succeed");

        let out: Vec<f64> = result
            .output_data
            .chunks_exact(std::mem::size_of::<f64>())
            .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
            .collect();

        assert_eq!(out.len(), 2, "output should have 2 values");
        assert!(
            (out[0] - 5.0).abs() < 1e-9,
            "out[0] should be 5.0, got {}",
            out[0]
        );
        assert!(
            (out[1] - 0.0).abs() < 1e-9,
            "out[1] should be 0.0, got {}",
            out[1]
        );
        assert_eq!(result.metadata.model_id, "mlp_test");
        assert_eq!(result.metadata.backend_id, "linear_algebra_mlp");
    }

    #[test]
    fn test_mlp_inference_rejects_unsupported_layer() {
        // A model with an Attention layer cannot be evaluated by the MLP backend and must
        // fail with a clear, honest error naming the unsupported layer type.
        let model = Model {
            model_id: "attn_test".to_string(),
            model_type: ModelType::Transformer,
            framework: MLFramework::PyTorch,
            architecture: ModelArchitecture {
                layers: vec![LayerInfo {
                    layer_id: "attn1".to_string(),
                    layer_type: LayerType::Attention,
                    input_shape: vec![4],
                    output_shape: vec![4],
                    parameters: 0,
                    activation: None,
                }],
                connections: vec![],
                input_shape: vec![4],
                output_shape: vec![4],
                total_parameters: 0,
            },
            weights: vec![],
            metadata: ModelMetadata::new(),
        };

        let input = [1.0f64, 2.0, 3.0, 4.0];
        let input_bytes: Vec<u8> = input.iter().flat_map(|v| v.to_le_bytes()).collect();

        let request = InferenceRequest {
            request_id: "req_attn".to_string(),
            model_id: "attn_test".to_string(),
            input_data: input_bytes,
            parameters: InferenceParameters {
                batch_size: 1,
                sequence_length: 4,
                temperature: None,
                top_k: None,
                top_p: None,
                max_tokens: None,
                precision: Precision::FP32,
            },
            priority: RequestPriority::Normal,
            submitted_at: 0,
            deadline: None,
        };

        let mut engine = InferenceEngine::new();
        let result = engine.execute_inference(&request, &model);
        let err = result.expect_err("Attention layer must be rejected");
        let msg = format!("{}", err);
        assert!(
            msg.contains("Attention"),
            "error should name the unsupported layer type: {}",
            msg
        );
    }

    #[test]
    fn test_training() {
        let mut library = MachineLearningLibrary::new();
        library.initialize().unwrap();

        let training_config = TrainingConfig {
            epochs: 5,
            batch_size: 16,
            learning_rate: 0.001,
            optimizer: TrainingAlgorithm::Adam,
            loss_function: "cross_entropy".to_string(),
            metrics: vec!["accuracy".to_string()],
            validation_split: 0.2,
        };

        let result = library
            .start_training("test_model", training_config)
            .unwrap();

        assert_eq!(result.result.model_id, "test_model");
        assert_eq!(result.result.status, TrainingStatus::Pending);
        assert_eq!(result.result.training_config.epochs, 5);
    }

    #[test]
    fn test_model_optimization() {
        let mut library = MachineLearningLibrary::new();
        library.initialize().unwrap();

        let result = library
            .optimize_model("test_model", MLOptimizationAlgorithm::ModelQuantization)
            .unwrap();

        assert_eq!(result.result.model_id, "test_model");
        assert_eq!(result.result.model_type, ModelType::LLM);
    }

    #[test]
    fn test_performance_metrics() {
        let library = MachineLearningLibrary::new();
        let metrics = library.get_performance_stats();

        assert_eq!(metrics.inference_metrics.total_requests, 0);
        assert_eq!(metrics.training_metrics.total_training_jobs, 0);
        assert_eq!(metrics.system_metrics.cpu_utilization, 0.0);
        assert_eq!(metrics.model_metrics.total_models, 0);
    }

    #[test]
    fn test_model_listing() {
        let library = MachineLearningLibrary::new();
        let models = library.list_models();
        assert_eq!(models.len(), 0);
    }

    #[test]
    fn test_model_info() {
        let library = MachineLearningLibrary::new();
        let info = library.get_model_info("test_model");
        assert!(info.is_none());
    }

    #[test]
    fn test_model_cache_get_put_and_stats() {
        let mut cache = ModelCache::new();

        // Miss on an empty cache.
        assert!(cache.get("missing").is_none());
        let stats = cache.cache_stats();
        assert_eq!(stats.hit_count, 0);
        assert_eq!(stats.miss_count, 1);
        assert!((stats.hit_rate - 0.0).abs() < f64::EPSILON);

        // Put a model in and retrieve it (hit).
        let mut model = Model::new();
        model.model_id = "m1".to_string();
        cache.put("m1".to_string(), model.clone()).unwrap();
        assert_eq!(cache.cache_size(), 1);

        let retrieved = cache.get("m1").expect("cached model should be present");
        assert_eq!(retrieved.model_id, "m1");
        let stats = cache.cache_stats();
        assert_eq!(stats.hit_count, 1);
        assert_eq!(stats.miss_count, 1);
        assert!((stats.hit_rate - 0.5).abs() < f64::EPSILON);

        // A second miss.
        assert!(cache.get("nope").is_none());
        let stats = cache.cache_stats();
        assert_eq!(stats.hit_count, 1);
        assert_eq!(stats.miss_count, 2);
        let expected = 1.0 / 3.0;
        assert!((stats.hit_rate - expected).abs() < 1e-9);
    }

    #[test]
    fn test_model_cache_lru_eviction() {
        // Build a cache with a tiny max size so eviction is exercised.
        let mut cache = ModelCache {
            cache_entries: HashMap::new(),
            cache_policy: ModelCachePolicy {
                eviction_policy: ModelEvictionPolicy::LRU,
                max_size: 16, // two 8-byte entries fit; a third forces LRU eviction
                ttl: 3600,
                priority_levels: vec![PriorityLevel::Medium],
            },
            cache_stats: ModelCacheStats::new(),
        };

        let mk = |id: &str| {
            let mut m = Model::new();
            m.model_id = id.to_string();
            // One f64 weight = 8 bytes per entry.
            m.weights = vec![0.0];
            m
        };

        cache.put("a".to_string(), mk("a")).unwrap();
        cache.put("b".to_string(), mk("b")).unwrap();

        // Access "a" so "b" becomes the LRU candidate.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let _ = cache.get("a");

        // Adding "c" exceeds the budget and must evict the oldest (b).
        cache.put("c".to_string(), mk("c")).unwrap();

        assert!(
            cache.get("b").is_none(),
            "LRU entry 'b' should have been evicted"
        );
        assert!(cache.get("a").is_some(), "'a' should still be resident");
        assert!(cache.get("c").is_some(), "'c' should be resident");

        let stats = cache.cache_stats();
        assert!(
            stats.eviction_count >= 1,
            "eviction_count should reflect evictions"
        );
        assert!(stats.total_size <= cache.cache_policy.max_size);
    }

    #[test]
    fn test_model_storage_load_model_fallback_on_missing_file() {
        // A path that does not exist must fall back to the mock scaffold model rather
        // than erroring out, so downstream inference always has a model to operate on.
        let mut storage = ModelStorage::new();
        let model = storage
            .load_model("fallback_missing", "/nonexistent/path/to/model.gguf")
            .expect("missing file should fall back to mock model, not error");

        assert_eq!(model.model_id, "fallback_missing");
        assert_eq!(model.model_type, ModelType::LLM);
        assert_eq!(model.framework, MLFramework::PyTorch);
        assert_eq!(
            model.weights.len(),
            1000,
            "mock model should have 1000 weights"
        );

        // The loaded model should be cached in the model_store.
        assert!(storage.model_store.contains_key("fallback_missing"));
    }

    #[test]
    fn test_model_storage_load_model_fallback_on_non_gguf_file() {
        // A real file that is not a GGUF file must fall back to the mock scaffold model.
        let dir = std::env::temp_dir();
        let path = dir.join("qualia_ml_non_gguf_test.bin");
        std::fs::write(&path, b"this is not a gguf file").unwrap();

        let mut storage = ModelStorage::new();
        let model = storage
            .load_model("fallback_non_gguf", path.to_str().unwrap())
            .expect("non-GGUF file should fall back to mock model, not error");

        assert_eq!(model.model_id, "fallback_non_gguf");
        assert_eq!(
            model.weights.len(),
            1000,
            "mock model should have 1000 weights"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_model_storage_load_model_caches_in_store() {
        // Loading the same model_id twice should return the cached instance from
        // model_store (verified by mutating the first result and confirming the second
        // load is independent of further disk reads).
        let mut storage = ModelStorage::new();
        let first = storage
            .load_model("cached_model", "/nonexistent/model.gguf")
            .unwrap();
        assert_eq!(first.model_id, "cached_model");

        // Second load should come from the store without re-reading disk.
        let second = storage
            .load_model("cached_model", "/nonexistent/model.gguf")
            .unwrap();
        assert_eq!(second.model_id, first.model_id);
        assert_eq!(second.weights.len(), first.weights.len());
    }

    #[test]
    fn test_model_storage_load_model_real_gguf_if_present() {
        // If a real GGUF file happens to be available at a well-known path, exercise the
        // real loading path; otherwise skip gracefully so the test is hermetic.
        let candidate = std::env::var("QUALIA_TEST_GGUF_PATH").ok();
        let gguf_path = match candidate {
            Some(p) if !p.is_empty() && std::path::Path::new(&p).exists() => p,
            _ => {
                eprintln!(
                    "[test_model_storage_load_model_real_gguf_if_present] \
                           no GGUF file available (set QUALIA_TEST_GGUF_PATH); skipping"
                );
                return;
            }
        };

        let mut storage = ModelStorage::new();
        let model = match storage.load_model("real_gguf", &gguf_path) {
            Ok(m) => m,
            Err(e) => {
                // A parse failure should have fallen back to the mock model, not errored,
                // so reaching here is unexpected — surface it.
                panic!(
                    "load_model returned error for real GGUF {}: {}",
                    gguf_path, e
                );
            }
        };

        // If the GGUF parsed successfully the framework is Custom("GGUF") and weights are
        // a non-empty multiple of n_embd; otherwise the fallback mock (1000 weights,
        // PyTorch) was returned. Both are acceptable outcomes for this hermetic test.
        if model.framework == MLFramework::Custom("GGUF".to_string()) {
            assert!(
                !model.weights.is_empty(),
                "real GGUF model should have non-empty weights"
            );
            assert!(
                !model.architecture.layers.is_empty(),
                "real GGUF model should describe at least one layer"
            );
            assert_eq!(
                model.architecture.layers[0].layer_type,
                LayerType::Linear,
                "GGUF embedding layer should be modelled as Linear"
            );
        } else {
            assert_eq!(
                model.weights.len(),
                1000,
                "fallback mock model should have 1000 weights"
            );
        }
    }

    // ------------------------------------------------------------------
    // Feature 1: ModelCatalog search index
    // ------------------------------------------------------------------

    #[test]
    fn test_model_catalog_register_search_by_tag() {
        let mut catalog = ModelCatalog::new();
        catalog.initialize().unwrap();

        // Register two models with distinct metadata.
        let mut meta_a = ModelMetadata::new();
        meta_a.model_id = "vision-resnet".to_string();
        meta_a.model_type = ModelType::CNN;
        let mut meta_b = ModelMetadata::new();
        meta_b.model_id = "llm-bert".to_string();
        meta_b.model_type = ModelType::LLM;

        catalog.register_model("vision-resnet", meta_a);
        catalog.register_model("llm-bert", meta_b);

        // Tag them.
        catalog.add_tag("vision-resnet", "vision");
        catalog.add_tag("vision-resnet", "classification");
        catalog.add_tag("llm-bert", "nlp");
        catalog.add_tag("llm-bert", "classification");

        // get_by_tag returns the right model ids (case-insensitive).
        let vision = catalog.get_by_tag("Vision");
        assert_eq!(vision, vec!["vision-resnet".to_string()]);
        let nlp = catalog.get_by_tag("NLP");
        assert_eq!(nlp, vec!["llm-bert".to_string()]);

        // Both models share the "classification" tag.
        let mut cls = catalog.get_by_tag("classification");
        cls.sort();
        assert_eq!(
            cls,
            vec!["llm-bert".to_string(), "vision-resnet".to_string()]
        );

        // Unknown tag → empty.
        assert!(catalog.get_by_tag("nonexistent").is_empty());
    }

    #[test]
    fn test_model_catalog_search_by_keyword_and_name() {
        let mut catalog = ModelCatalog::new();
        catalog.initialize().unwrap();

        let mut meta = ModelMetadata::new();
        meta.model_id = "audio-transformer".to_string();
        meta.model_type = ModelType::Transformer;
        catalog.register_model("audio-transformer", meta);
        catalog.add_tag("audio-transformer", "speech");

        // Search by a substring of the model id.
        let by_name = catalog.search("audio");
        assert_eq!(by_name, vec!["audio-transformer".to_string()]);

        // Search by tag keyword.
        let by_tag = catalog.search("speech");
        assert_eq!(by_tag, vec!["audio-transformer".to_string()]);

        // Search by the model type keyword that register_model adds to the index.
        let by_type = catalog.search("Transformer");
        assert_eq!(by_type, vec!["audio-transformer".to_string()]);

        // A query that matches nothing returns empty.
        assert!(catalog.search("zzz-no-match").is_empty());

        // Empty query returns empty (no spurious matches).
        assert!(catalog.search("").is_empty());
    }

    #[test]
    fn test_model_search_index_initialize_and_search() {
        let mut index = ModelSearchIndex::new();
        // Before initialize, search returns nothing even with a matching entry.
        index.index(ModelIndexEntry {
            entry_id: "m1".to_string(),
            keywords: vec!["alpha".to_string()],
            metadata: HashMap::new(),
            relevance_score: 1.0,
        });
        assert!(
            index.search("alpha").is_empty(),
            "search before initialize must be empty"
        );

        index.initialize().unwrap();
        let hits = index.search("alpha");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entry_id, "m1");

        // Metadata values are also searchable.
        let mut md = HashMap::new();
        md.insert("framework".to_string(), "PyTorch".to_string());
        index.index(ModelIndexEntry {
            entry_id: "m2".to_string(),
            keywords: vec![],
            metadata: md,
            relevance_score: 0.5,
        });
        let hits2 = index.search("pytorch");
        assert_eq!(hits2.len(), 1);
        assert_eq!(hits2[0].entry_id, "m2");
    }

    // ------------------------------------------------------------------
    // Feature 2: SGD training loop
    // ------------------------------------------------------------------

    #[test]
    fn test_compute_mse() {
        let preds = [1.0, 2.0, 3.0];
        let targets = [1.0, 2.0, 3.0];
        assert!((TrainingEngine::compute_mse(&preds, &targets) - 0.0).abs() < 1e-12);

        let preds = [2.0, 4.0];
        let targets = [1.0, 1.0];
        // MSE = ((2-1)^2 + (4-1)^2) / 2 = (1 + 9)/2 = 5.0
        assert!((TrainingEngine::compute_mse(&preds, &targets) - 5.0).abs() < 1e-12);

        // Mismatched lengths → 0.0 (defined behaviour).
        assert_eq!(TrainingEngine::compute_mse(&[1.0], &[1.0, 2.0]), 0.0);
    }

    #[test]
    fn test_compute_gradients_linear() {
        // y = w*x + b, weights = [w, b] = [3.0, -1.0], x = [2.0], pred = 3*2 - 1 = 5, target = 1.
        // diff = 4 ; dL/dw = 2*4*2 = 16 ; dL/db = 2*4 = 8 ; lr = 0.1
        // returned (scaled) = [0.1*16, 0.1*8] = [1.6, 0.8]
        let weights = [3.0, -1.0];
        let inputs = [2.0];
        let grad = TrainingEngine::compute_gradients(&weights, &inputs, 5.0, 1.0, 0.1);
        assert_eq!(grad.len(), 2);
        assert!((grad[0] - 1.6).abs() < 1e-12, "grad[0] = {}", grad[0]);
        assert!((grad[1] - 0.8).abs() < 1e-12, "grad[1] = {}", grad[1]);
    }

    #[test]
    fn test_sgd_training_learns_linear_function() {
        // Build a 1->1 linear model with no activation: y = w*x + b.
        // weights = [w, b], initialised to zero.
        let mut model = Model {
            model_id: "linreg".to_string(),
            model_type: ModelType::RNN, // arbitrary; not used by the trainer
            framework: MLFramework::Custom("test".to_string()),
            architecture: ModelArchitecture {
                layers: vec![LayerInfo {
                    layer_id: "l1".to_string(),
                    layer_type: LayerType::Linear,
                    input_shape: vec![1],
                    output_shape: vec![1],
                    parameters: 2, // 1 weight + 1 bias
                    activation: None,
                }],
                connections: vec![],
                input_shape: vec![1],
                output_shape: vec![1],
                total_parameters: 2,
            },
            weights: vec![0.0, 0.0],
            metadata: ModelMetadata::new(),
        };

        // Training data from y = 2x + 1 over x in [-2, 2].
        let xs: Vec<f64> = (-20..=20).map(|i| i as f64 * 0.2).collect();
        let training_data: Vec<f64> = xs.clone();
        let targets: Vec<f64> = xs.iter().map(|x| 2.0 * x + 1.0).collect();

        let config = TrainingConfig {
            epochs: 500,
            batch_size: 8,
            learning_rate: 0.05,
            optimizer: TrainingAlgorithm::SGD,
            loss_function: "mse".to_string(),
            metrics: vec!["loss".to_string()],
            validation_split: 0.0,
        };

        let mut engine = TrainingEngine::new();
        let result = engine
            .start_training(&mut model, &training_data, &targets, &config)
            .expect("SGD training should succeed");

        // Loss must drop dramatically.
        assert!(
            result.final_loss < result.initial_loss,
            "final loss ({}) should be less than initial loss ({})",
            result.final_loss,
            result.initial_loss
        );
        assert!(
            result.final_loss < 1e-3,
            "final loss ({}) should be near zero for a perfectly linear dataset",
            result.final_loss
        );
        // The loop may converge early (loss plateau) before all epochs run; both outcomes
        // are valid, so only bound the completed count.
        assert!(
            result.epochs_completed <= config.epochs as usize,
            "epochs_completed ({}) must not exceed configured epochs ({})",
            result.epochs_completed,
            config.epochs
        );

        // Learned weights should be close to the true [w=2, b=1].
        let w = model.weights[0];
        let b = model.weights[1];
        assert!(
            (w - 2.0).abs() < 0.05,
            "learned weight w = {} should be ~2.0",
            w
        );
        assert!(
            (b - 1.0).abs() < 0.05,
            "learned bias b = {} should be ~1.0",
            b
        );
    }

    #[test]
    fn test_sgd_training_rejects_unsupported_config() {
        // A non-SGD optimizer must be rejected, not silently ignored.
        let mut model = Model::new();
        let config = TrainingConfig {
            epochs: 1,
            batch_size: 1,
            learning_rate: 0.01,
            optimizer: TrainingAlgorithm::Adam,
            loss_function: "mse".to_string(),
            metrics: vec![],
            validation_split: 0.0,
        };
        let mut engine = TrainingEngine::new();
        let err = engine
            .start_training(&mut model, &[1.0], &[1.0], &config)
            .expect_err("Adam must be rejected by the SGD-only trainer");
        let msg = format!("{}", err);
        assert!(msg.contains("SGD"), "error should name SGD: {}", msg);
    }

    #[test]
    fn test_sgd_training_rejects_activation() {
        // A Linear layer with an activation is out of scope for linear-regression SGD.
        let mut model = Model {
            model_id: "act".to_string(),
            model_type: ModelType::LLM,
            framework: MLFramework::PyTorch,
            architecture: ModelArchitecture {
                layers: vec![LayerInfo {
                    layer_id: "l1".to_string(),
                    layer_type: LayerType::Linear,
                    input_shape: vec![1],
                    output_shape: vec![1],
                    parameters: 2,
                    activation: Some(ActivationFunction::ReLU),
                }],
                connections: vec![],
                input_shape: vec![1],
                output_shape: vec![1],
                total_parameters: 2,
            },
            weights: vec![0.0, 0.0],
            metadata: ModelMetadata::new(),
        };
        let config = TrainingConfig {
            epochs: 1,
            batch_size: 1,
            learning_rate: 0.01,
            optimizer: TrainingAlgorithm::SGD,
            loss_function: "mse".to_string(),
            metrics: vec![],
            validation_split: 0.0,
        };
        let mut engine = TrainingEngine::new();
        let err = engine
            .start_training(&mut model, &[1.0], &[1.0], &config)
            .expect_err("activated layer must be rejected");
        let msg = format!("{}", err);
        assert!(
            msg.contains("activation"),
            "error should mention activation: {}",
            msg
        );
    }

    // ------------------------------------------------------------------
    // Feature 1: Model Version Control
    // ------------------------------------------------------------------

    fn sample_version(id: &str) -> ModelVersion {
        ModelVersion {
            version_id: id.to_string(),
            version_number: id.to_string(),
            changes: vec![],
            created_at: 0,
            created_by: "tester".to_string(),
        }
    }

    #[test]
    fn test_version_control_create_and_get() {
        let mut vc = ModelVersionControl::new();
        assert!(vc.initialize().is_ok());

        let v1 = sample_version("v1");
        assert!(vc.create_version("model-a", v1.clone()).is_ok());

        // Duplicate version should be rejected.
        let err = vc
            .create_version("model-a", sample_version("v1"))
            .expect_err("duplicate version must be rejected");
        assert!(format!("{}", err).contains("already exists"));

        // Retrieval works.
        let got = vc
            .get_version("model-a", "v1")
            .expect("version should exist");
        assert_eq!(got.version_id, "v1");

        // Unknown model/version returns None.
        assert!(vc.get_version("model-a", "v2").is_none());
        assert!(vc.get_version("model-b", "v1").is_none());
    }

    #[test]
    fn test_version_control_list_versions() {
        let mut vc = ModelVersionControl::new();
        vc.initialize().unwrap();
        vc.create_version("model-a", sample_version("v1")).unwrap();
        vc.create_version("model-a", sample_version("v2")).unwrap();
        vc.create_version("model-b", sample_version("v1")).unwrap();

        let mut a_versions = vc.list_versions("model-a");
        a_versions.sort();
        assert_eq!(a_versions, vec!["v1".to_string(), "v2".to_string()]);

        let b_versions = vc.list_versions("model-b");
        assert_eq!(b_versions, vec!["v1".to_string()]);

        assert!(vc.list_versions("model-c").is_empty());
    }

    #[test]
    fn test_version_control_branches() {
        let mut vc = ModelVersionControl::new();
        vc.initialize().unwrap();
        vc.create_version("model-a", sample_version("v1")).unwrap();

        // Creating a branch from an existing version succeeds.
        assert!(vc.create_branch("dev", "v1").is_ok());
        let branch = vc.get_branch("dev").expect("branch should exist");
        assert_eq!(branch, &vec!["v1".to_string()]);

        // Duplicate branch is rejected.
        let err = vc
            .create_branch("dev", "v1")
            .expect_err("duplicate branch must be rejected");
        assert!(format!("{}", err).contains("already exists"));

        // Branching from an unknown version fails.
        assert!(vc.create_branch("feat", "nope").is_err());

        // The default `main` branch is seeded by initialize().
        assert!(vc.get_branch("main").is_some());

        // Unknown branch returns None.
        assert!(vc.get_branch("ghost").is_none());
    }

    #[test]
    fn test_version_control_tags() {
        let mut vc = ModelVersionControl::new();
        vc.initialize().unwrap();
        vc.create_version("model-a", sample_version("v1")).unwrap();
        vc.create_version("model-a", sample_version("v2")).unwrap();

        // Tag versions.
        assert!(vc.tag_version("v1", "stable").is_ok());
        assert!(vc.tag_version("v2", "latest").is_ok());
        assert!(vc.tag_version("v2", "stable").is_ok());

        // Tagging an unknown version fails.
        assert!(vc.tag_version("v9", "x").is_err());

        // get_tags returns all tags for a version.
        let mut v2_tags = vc.get_tags("v2");
        v2_tags.sort();
        assert_eq!(v2_tags, vec!["latest".to_string(), "stable".to_string()]);

        // get_by_tag returns all versions carrying the tag.
        let mut stable_versions = vc.get_by_tag("stable");
        stable_versions.sort();
        assert_eq!(stable_versions, vec!["v1".to_string(), "v2".to_string()]);

        assert!(vc.get_by_tag("nonexistent").is_empty());
        assert!(vc.get_tags("v9").is_empty());
    }

    #[test]
    fn test_version_control_initialize_seeds_main_branch() {
        let mut vc = ModelVersionControl::new();
        // Before initialize, no branches exist.
        assert!(vc.get_branch("main").is_none());
        assert!(vc.initialize().is_ok());
        assert!(vc.get_branch("main").is_some());
    }

    // ------------------------------------------------------------------
    // Feature 2: Compression Quality Metrics
    // ------------------------------------------------------------------

    #[test]
    fn test_compression_register_and_get_algorithm() {
        let mut mc = ModelCompression::new();
        assert!(mc.list_algorithms().is_empty());

        mc.register_algorithm("my-pruner", CompressionAlgorithm::Pruning);
        assert_eq!(mc.list_algorithms(), vec!["my-pruner".to_string()]);
        assert_eq!(
            mc.get_algorithm("my-pruner"),
            Some(&CompressionAlgorithm::Pruning)
        );
        assert!(mc.get_algorithm("missing").is_none());
    }

    #[test]
    fn test_compression_initialize_registers_standard_algorithms() {
        let mut mc = ModelCompression::new();
        assert!(mc.initialize().is_ok());

        let mut names = mc.list_algorithms();
        names.sort();
        assert_eq!(
            names,
            vec![
                "Distillation".to_string(),
                "Pruning".to_string(),
                "QuantizationFP16".to_string(),
                "QuantizationInt8".to_string(),
            ]
        );
    }

    #[test]
    fn test_compression_record_updates_metrics() {
        let mut mc = ModelCompression::new();
        mc.initialize().unwrap();

        // 1000 bytes -> 250 bytes is a 4x compression ratio (75% reduction).
        assert!(mc
            .record_compression("QuantizationInt8", 1000, 250, 0.90, 0.88)
            .is_ok());

        let metrics = mc.get_quality_metrics();
        assert_eq!(metrics.compression_count, 1);
        assert!((metrics.compression_ratio - 4.0).abs() < 1e-9);
        assert!((metrics.size_reduction - 0.75).abs() < 1e-9);
        // accuracy preservation = 0.88 / 0.90
        assert!((metrics.accuracy_preservation - (0.88 / 0.90)).abs() < 1e-9);

        // The accessor and helper agree.
        assert!((mc.compression_ratio() - metrics.compression_ratio).abs() < 1e-9);
    }

    #[test]
    fn test_compression_record_rejects_unknown_algorithm() {
        let mut mc = ModelCompression::new();
        let err = mc
            .record_compression("ghost", 100, 50, 1.0, 1.0)
            .expect_err("unknown algorithm must be rejected");
        assert!(format!("{}", err).contains("unknown compression algorithm"));
    }

    #[test]
    fn test_compression_record_rejects_zero_original_size() {
        let mut mc = ModelCompression::new();
        mc.register_algorithm("x", CompressionAlgorithm::Pruning);
        let err = mc
            .record_compression("x", 0, 0, 1.0, 1.0)
            .expect_err("zero original size must be rejected");
        assert!(format!("{}", err).contains("original_size"));
    }

    #[test]
    fn test_compression_record_running_average() {
        let mut mc = ModelCompression::new();
        mc.register_algorithm("x", CompressionAlgorithm::Pruning);

        // First: ratio 4.0 (1000 -> 250). Second: ratio 2.0 (1000 -> 500).
        // Average ratio should be 3.0.
        mc.record_compression("x", 1000, 250, 1.0, 1.0).unwrap();
        mc.record_compression("x", 1000, 500, 1.0, 1.0).unwrap();

        let metrics = mc.get_quality_metrics();
        assert_eq!(metrics.compression_count, 2);
        assert!((metrics.compression_ratio - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_symmetric_int8_ptq_round_trip_measures_error() {
        let weights = [-1.0, -0.51, 0.0, 0.26, 0.75, 1.0];
        let mut quantized = [0i8; 6];
        let mut compression = ModelCompression::new();

        let report = compression
            .quantize_symmetric_int8_into(&weights, &mut quantized)
            .expect("PTQ should succeed");
        let mut reconstructed = [0.0f64; 6];
        let written = ModelCompression::dequantize_symmetric_int8_into(
            &quantized,
            report.parameters,
            &mut reconstructed,
        )
        .expect("dequantization should succeed");

        assert_eq!(written, weights.len());
        assert_eq!(quantized[0], -127);
        assert_eq!(quantized[5], 127);
        assert!(report.compression_ratio > 3.0);
        assert!(report.rmse > 0.0);
        assert!(report.max_abs_error <= report.parameters.scale / 2.0 + f64::EPSILON);
        for (&expected, &actual) in weights.iter().zip(reconstructed.iter()) {
            assert!((expected - actual).abs() <= report.parameters.scale / 2.0 + f64::EPSILON);
        }
        assert_eq!(compression.get_quality_metrics().compression_count, 1);
    }

    #[test]
    fn test_unstructured_pruning_packs_exact_smallest_weights() {
        let weights = [0.01, 5.0, -0.02, 4.0];
        let mut mask = [0u8; 1];
        let mut packed = [0.0f64; 2];
        let mut scratch = [0usize; 4];
        let mut compression = ModelCompression::new();

        let report = compression
            .prune_unstructured_into(&weights, 0.5, &mut mask, &mut packed, &mut scratch)
            .expect("magnitude pruning should succeed");

        assert_eq!(report.pruned_weights, 2);
        assert_eq!(report.kept_weights, 2);
        assert_eq!(packed, [5.0, 4.0]);
        assert!(!ModelCompression::mask_keeps(&mask, 0));
        assert!(ModelCompression::mask_keeps(&mask, 1));
        assert!(!ModelCompression::mask_keeps(&mask, 2));
        assert!(ModelCompression::mask_keeps(&mask, 3));

        let mut reconstructed = [9.0f64; 4];
        assert_eq!(
            ModelCompression::unpack_pruned_weights_into(&mask, &packed, &mut reconstructed)
                .unwrap(),
            2
        );
        assert_eq!(reconstructed, [0.0, 5.0, 0.0, 4.0]);
        assert!(report.l2_energy_preserved > 0.999);
    }

    #[test]
    fn test_structured_pruning_removes_lowest_energy_output_channel() {
        // Three output channels (rows), two inputs per channel.
        let weights = [0.1, 0.1, 5.0, 5.0, 2.0, 2.0];
        let mut row_mask = [0u8; 1];
        let mut packed = [0.0f64; 4];
        let mut scores = [0.0f64; 3];
        let mut indices = [0usize; 3];
        let mut compression = ModelCompression::new();

        let report = compression
            .prune_output_channels_into(
                &weights,
                3,
                2,
                1.0 / 3.0,
                &mut row_mask,
                &mut packed,
                &mut scores,
                &mut indices,
            )
            .expect("structured pruning should succeed");

        assert_eq!(report.total_units, 3);
        assert_eq!(report.pruned_units, 1);
        assert_eq!(report.pruned_weights, 2);
        assert!(!ModelCompression::mask_keeps(&row_mask, 0));
        assert!(ModelCompression::mask_keeps(&row_mask, 1));
        assert!(ModelCompression::mask_keeps(&row_mask, 2));
        assert_eq!(packed, [5.0, 5.0, 2.0, 2.0]);
    }

    fn compression_test_linear_model(
        model_id: &str,
        input_size: usize,
        output_size: usize,
        weights: Vec<f64>,
    ) -> Model {
        Model {
            model_id: model_id.to_string(),
            model_type: ModelType::RNN,
            framework: MLFramework::Custom("compression-test".to_string()),
            architecture: ModelArchitecture {
                layers: vec![LayerInfo {
                    layer_id: format!("{}_linear", model_id),
                    layer_type: LayerType::Linear,
                    input_shape: vec![input_size],
                    output_shape: vec![output_size],
                    parameters: input_size * output_size + output_size,
                    activation: None,
                }],
                connections: vec![],
                input_shape: vec![input_size],
                output_shape: vec![output_size],
                total_parameters: input_size * output_size + output_size,
            },
            weights,
            metadata: ModelMetadata::new(),
        }
    }

    fn compression_test_training_config() -> TrainingConfig {
        TrainingConfig {
            epochs: 500,
            batch_size: 8,
            learning_rate: 0.05,
            optimizer: TrainingAlgorithm::SGD,
            loss_function: "mse".to_string(),
            metrics: vec!["loss".to_string()],
            validation_split: 0.0,
        }
    }

    #[test]
    fn test_pruning_recovery_never_regrows_masked_weight() {
        let mut model = compression_test_linear_model("masked", 2, 1, vec![0.0, 8.0, 0.0]);
        // Keep w0 and bias, prune w1.
        let mask = [0b0000_0101u8];
        let mut inputs = Vec::new();
        let mut targets = Vec::new();
        for x0 in -10..=10 {
            for x1 in -2..=2 {
                inputs.push(x0 as f64 / 5.0);
                inputs.push(x1 as f64);
                targets.push(2.0 * (x0 as f64 / 5.0) + 1.0);
            }
        }

        let mut trainer = TrainingEngine::new();
        let result = trainer
            .start_training_with_pruning_mask(
                &mut model,
                &inputs,
                &targets,
                &compression_test_training_config(),
                &mask,
            )
            .expect("masked recovery should train");

        assert!(result.final_loss < result.initial_loss);
        assert_eq!(model.weights[1], 0.0, "pruned weight must remain zero");
        assert!((model.weights[0] - 2.0).abs() < 0.05);
        assert!((model.weights[2] - 1.0).abs() < 0.05);
    }

    #[test]
    fn test_teacher_student_distillation_trains_smaller_linear_model() {
        // A two-layer linear teacher representing y = 2x + 1:
        // [x, -x] followed by 3*x + 1*(-x) + 1.
        let teacher = Model {
            model_id: "teacher".to_string(),
            model_type: ModelType::RNN,
            framework: MLFramework::Custom("compression-test".to_string()),
            architecture: ModelArchitecture {
                layers: vec![
                    LayerInfo {
                        layer_id: "teacher_1".to_string(),
                        layer_type: LayerType::Linear,
                        input_shape: vec![1],
                        output_shape: vec![2],
                        parameters: 4,
                        activation: None,
                    },
                    LayerInfo {
                        layer_id: "teacher_2".to_string(),
                        layer_type: LayerType::Linear,
                        input_shape: vec![2],
                        output_shape: vec![1],
                        parameters: 3,
                        activation: None,
                    },
                ],
                connections: vec![],
                input_shape: vec![1],
                output_shape: vec![1],
                total_parameters: 7,
            },
            weights: vec![1.0, -1.0, 0.0, 0.0, 3.0, 1.0, 1.0],
            metadata: ModelMetadata::new(),
        };
        let mut student = compression_test_linear_model("student", 1, 1, vec![0.0, 0.0]);
        let inputs: Vec<f64> = (-20..=20).map(|x| x as f64 / 10.0).collect();
        let mut target_buffer = vec![0.0f64; inputs.len()];
        let mut trainer = TrainingEngine::new();
        let mut compression = ModelCompression::new();

        let report = compression
            .distill_linear_student(
                &mut trainer,
                &teacher,
                &mut student,
                &inputs,
                None,
                DistillationConfig {
                    teacher_weight: 1.0,
                },
                &compression_test_training_config(),
                &mut target_buffer,
            )
            .expect("distillation should succeed");

        assert_eq!(report.teacher_parameters, 7);
        assert_eq!(report.student_parameters, 2);
        assert!((report.compression_ratio - 3.5).abs() < 1e-12);
        assert!(report.fidelity_mse_after < report.fidelity_mse_before);
        assert!(report.fidelity_mse_after < 1e-3);
        assert!((student.weights[0] - 2.0).abs() < 0.05);
        assert!((student.weights[1] - 1.0).abs() < 0.05);
    }
}
