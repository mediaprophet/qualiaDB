//! Type definitions (structs and enums) for the machine learning library.
//!
//! Split out of the former monolithic `machine_learning.rs` (pure code motion).
//! Private fields were widened to `pub(super)` so the `impl` blocks in sibling
//! submodules retain the exact field access they had when co-located.

#[allow(unused_imports)]
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use std::collections::HashMap;

/// Machine Learning Library Manager
pub struct MachineLearningLibrary {
    pub(super) model_manager: ModelManager,
    pub(super) inference_engine: InferenceEngine,
    pub(super) training_engine: TrainingEngine,
    pub(super) optimization_engine: MLOptimizationEngine,
    pub(super) performance_monitor: MLPerformanceMonitor,
    pub(super) request_count: u64,
}

/// Model manager for neural network models
pub struct ModelManager {
    pub(super) model_storage: ModelStorage,
    pub(super) model_loader: ModelLoader,
    pub(super) model_converter: ModelConverter,
    pub(super) model_cache: ModelCache,
}

/// Model storage using ZNS for efficient model storage
pub struct ModelStorage {
    pub(super) zones: HashMap<String, ModelZone>,
    pub(super) model_catalog: ModelCatalog,
    pub(super) compression_engine: ModelCompression,
    pub(super) version_control: ModelVersionControl,
    pub(super) model_store: HashMap<String, Model>,
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
    pub(super) models: HashMap<String, ModelMetadata>,
    pub(super) relationships: HashMap<String, Vec<ModelRelationship>>,
    pub(super) tags: HashMap<String, Vec<String>>,
    pub(super) search_index: ModelSearchIndex,
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
    pub(super) index_entries: HashMap<String, ModelIndexEntry>,
    pub(super) search_engine: ModelSearchEngine,
    /// Whether `initialize()` has actually configured the index (the search methods are
    /// only valid once this is `true`).
    pub(super) initialized: bool,
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
    pub(super) engine_type: SearchEngineType,
    pub(super) indexing_strategy: IndexingStrategy,
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
    pub(super) compression_algorithms: HashMap<String, CompressionAlgorithm>,
    pub(super) compression_statistics: CompressionStatistics,
    pub(super) quality_metrics: CompressionQualityMetrics,
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
    pub(super) versions: HashMap<String, ModelVersion>,
    pub(super) branches: HashMap<String, Vec<String>>,
    pub(super) tags: HashMap<String, Vec<String>>,
    /// Whether `initialize()` has actually configured the controller.
    pub(super) initialized: bool,
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
    pub(super) loading_strategies: HashMap<String, LoadingStrategy>,
    pub(super) format_converters: HashMap<String, FormatConverter>,
    pub(super) loading_cache: LoadingCache,
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
    pub(super) cache_entries: HashMap<String, CacheEntry>,
    pub(super) cache_policy: CachePolicy,
    pub(super) cache_stats: CacheStats,
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
    pub(super) conversion_pipelines: HashMap<String, ConversionPipeline>,
    pub(super) optimization_strategies: HashMap<String, OptimizationStrategy>,
    pub(super) validation_engine: ValidationEngine,
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
    pub(super) validators: HashMap<String, Validator>,
    pub(super) validation_rules: Vec<ValidationRule>,
    pub(super) test_suite: TestSuite,
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
    pub(super) cache_entries: HashMap<String, ModelCacheEntry>,
    pub(super) cache_policy: ModelCachePolicy,
    pub(super) cache_stats: ModelCacheStats,
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
    pub(super) inference_backends: HashMap<String, InferenceBackend>,
    pub(super) request_scheduler: RequestScheduler,
    pub(super) batch_processor: BatchProcessor,
    pub(super) performance_optimizer: InferenceOptimizer,
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
    pub(super) scheduling_policy: SchedulingPolicy,
    pub(super) queue_manager: QueueManager,
    pub(super) load_balancer: LoadBalancer,
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
    pub(super) pending_requests: Vec<InferenceRequest>,
    pub(super) running_requests: HashMap<String, RunningRequest>,
    pub(super) completed_requests: Vec<CompletedRequest>,
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
    pub(super) balancing_strategy: LoadBalancingStrategy,
    pub(super) backend_metrics: HashMap<String, BackendMetrics>,
    pub(super) health_checker: HealthChecker,
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
    pub(super) health_checks: HashMap<String, HealthCheck>,
    pub(super) check_interval: u64,
    pub(super) timeout: u64,
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
    pub(super) batching_strategy: BatchingStrategy,
    pub(super) batch_size: usize,
    pub(super) batch_timeout: u64,
    pub(super) batch_optimizer: BatchOptimizer,
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
    pub(super) optimization_algorithms: HashMap<String, BatchOptimizationAlgorithm>,
    pub(super) optimization_metrics: BatchOptimizationMetrics,
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
    pub(super) optimization_strategies: Vec<InferenceOptimizationStrategy>,
    pub(super) performance_analyzer: PerformanceAnalyzer,
    pub(super) auto_tuner: AutoTuner,
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
    pub(super) analysis_methods: Vec<AnalysisMethod>,
    pub(super) performance_profiles: HashMap<String, PerformanceProfile>,
    pub(super) bottleneck_detector: BottleneckDetector,
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
    pub(super) detection_algorithms: Vec<BottleneckDetectionAlgorithm>,
    pub(super) detection_thresholds: DetectionThresholds,
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
    pub(super) tuning_algorithms: HashMap<String, TuningAlgorithm>,
    pub(super) tuning_objectives: Vec<TuningObjective>,
    pub(super) tuning_history: TuningHistory,
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
    pub(super) tuning_records: Vec<TuningRecord>,
    pub(super) best_configurations: HashMap<String, TuningConfiguration>,
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
    pub(super) training_backends: HashMap<String, TrainingBackend>,
    pub(super) training_scheduler: TrainingScheduler,
    pub(super) data_pipeline: DataPipeline,
    pub(super) training_optimizer: TrainingOptimizer,
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
    pub(super) scheduling_policy: TrainingSchedulingPolicy,
    pub(super) resource_manager: ResourceManager,
    pub(super) progress_tracker: ProgressTracker,
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
    pub(super) resources: HashMap<String, Resource>,
    pub(super) allocation_strategy: AllocationStrategy,
    pub(super) utilization_tracker: UtilizationTracker,
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
    pub(super) utilization_history: HashMap<String, Vec<UtilizationRecord>>,
    pub(super) current_utilization: HashMap<String, f64>,
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
    pub(super) training_jobs: HashMap<String, TrainingJob>,
    pub(super) progress_metrics: ProgressMetrics,
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
    pub(super) data_sources: HashMap<String, DataSource>,
    pub(super) data_transformers: HashMap<String, DataTransformer>,
    pub(super) data_loaders: HashMap<String, DataLoader>,
    pub(super) data_augmenters: HashMap<String, DataAugmenter>,
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
    pub(super) optimization_algorithms: HashMap<String, TrainingOptimizationAlgorithm>,
    pub(super) hyperparameter_tuner: HyperparameterTuner,
    pub(super) early_stopping: EarlyStopping,
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
    pub(super) tuning_space: TuningSpace,
    pub(super) tuning_algorithm: TuningAlgorithm,
    pub(super) tuning_history: TuningHistory,
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
    pub(super) stopping_criteria: StoppingCriteria,
    pub(super) patience: u32,
    pub(super) min_delta: f64,
    pub(super) restore_best_weights: bool,
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
    pub(super) optimization_algorithms: HashMap<String, MLOptimizationAlgorithm>,
    pub(super) optimization_objectives: Vec<OptimizationObjective>,
    pub(super) optimization_constraints: Vec<OptimizationConstraint>,
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
    pub(super) inference_metrics: InferenceMetrics,
    pub(super) training_metrics: TrainingMetrics,
    pub(super) system_metrics: SystemMetrics,
    pub(super) model_metrics: ModelMetrics,
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
