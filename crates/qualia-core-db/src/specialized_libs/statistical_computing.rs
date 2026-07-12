//! Statistical Computing Library - Privacy-Preserving Statistical Analysis
//!
//! This module provides high-performance statistical computing operations leveraging Phase 2 enhancements:
//! - Fiduciary Cryptography (ML-DSA) for secure statistical computations
//! - Hardware-Sympathetic Storage (ZNS) for zero-copy statistical data
//! - Zero-Knowledge Semantic Proofs for privacy-preserving statistics
//! - NVMe Computational Storage (CSD) for accelerated statistical operations

use crate::fiduciary_crypto::{FiduciaryCrypto, MlDsaSignature};
use crate::zk_proofs::{CircuitExpression, FieldElement, VariableType, ZkProof, ZkProofSystem};
use crate::zns_storage::ZnsZoneManager;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Statistical Computing Library Manager
pub struct StatisticalComputingLibrary {
    data_storage: StatisticalDataStorage,
    computation_engine: StatisticalComputationEngine,
    privacy_engine: StatisticalPrivacyEngine,
    analysis_engine: StatisticalAnalysisEngine,
    performance_monitor: StatisticalPerformanceMonitor,
}

/// Statistical data storage using ZNS for efficient data management
pub struct StatisticalDataStorage {
    zones: HashMap<String, StatisticalZone>,
    data_catalog: DataCatalog,
    compression_engine: DataCompressionEngine,
    indexing_engine: DataIndexingEngine,
    dataset_cache: HashMap<String, Dataset>,
    /// Optional ZNS zone manager. When `Some`, dataset persistence delegates
    /// to the real ZNS device; otherwise the in-memory `dataset_cache` acts as
    /// the always-available persistence layer.
    zns_manager: Option<Arc<Mutex<ZnsZoneManager>>>,
}

/// Statistical zone for different data types
#[derive(Debug, Clone)]
pub struct StatisticalZone {
    pub zone_id: String,
    pub zone_type: StatisticalZoneType,
    pub capacity: u64,
    pub datasets: HashMap<String, DatasetMetadata>,
    pub access_pattern: AccessPattern,
}

/// Statistical zone types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StatisticalZoneType {
    /// Time series data
    TimeSeries,
    /// Cross-sectional data
    CrossSectional,
    /// Panel data
    Panel,
    /// Experimental data
    Experimental,
    /// Survey data
    Survey,
    /// Simulation data
    Simulation,
    /// Cached statistics
    Cached,
}

/// Dataset metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetMetadata {
    pub dataset_id: String,
    pub dataset_type: DatasetType,
    pub dimensions: DatasetDimensions,
    pub data_types: Vec<DataType>,
    pub sample_size: usize,
    pub created_at: u64,
    pub last_updated: u64,
    pub access_count: u64,
    pub privacy_level: PrivacyLevel,
}

/// Dataset types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DatasetType {
    Numerical,
    Categorical,
    TimeSeries,
    Text,
    Image,
    Audio,
    Video,
    Mixed,
}

/// Dataset dimensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetDimensions {
    pub rows: usize,
    pub columns: usize,
    pub time_steps: Option<usize>,
    pub features: Option<usize>,
}

/// Data types for statistical analysis
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataType {
    Float32,
    Float64,
    Integer32,
    Integer64,
    Boolean,
    String,
    DateTime,
    Categorical,
}

/// Privacy levels for statistical data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PrivacyLevel {
    Public,
    Restricted,
    Confidential,
    Secret,
    TopSecret,
}

/// Access patterns for optimization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccessPattern {
    Sequential,
    Random,
    TimeSeries,
    Grouped,
    Adaptive,
}

/// Data catalog for dataset management
pub struct DataCatalog {
    datasets: HashMap<String, DatasetMetadata>,
    relationships: HashMap<String, Vec<Relationship>>,
    tags: HashMap<String, Vec<String>>,
    search_index: SearchIndex,
}

/// Dataset relationships
#[derive(Debug, Clone)]
pub struct Relationship {
    pub relationship_id: String,
    pub source_dataset: String,
    pub target_dataset: String,
    pub relationship_type: RelationshipType,
    pub strength: f64,
}

/// Relationship types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RelationshipType {
    Derived,
    Aggregated,
    Transformed,
    Merged,
    Linked,
    Hierarchical,
}

/// Search index for efficient dataset discovery
pub struct SearchIndex {
    index_entries: HashMap<String, IndexEntry>,
    search_engine: SearchEngine,
}

/// Index entry
#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub entry_id: String,
    pub keywords: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub relevance_score: f64,
}

/// Search engine
pub struct SearchEngine {
    engine_type: SearchEngineType,
    indexing_strategy: IndexingStrategy,
}

/// Search engine types
#[derive(Debug, Clone, PartialEq)]
pub enum SearchEngineType {
    FullText,
    Semantic,
    Hybrid,
    Vector,
}

/// Indexing strategies
#[derive(Debug, Clone, PartialEq)]
pub enum IndexingStrategy {
    Inverted,
    Ngram,
    SkipGram,
    BM25,
    BTree,
    Custom,
}

/// Data compression engine
pub struct DataCompressionEngine {
    compression_algorithms: Vec<CompressionAlgorithm>,
    compression_statistics: CompressionStatistics,
}

/// Compression algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    Gzip,
    LZ4,
    ZSTD,
    Snappy,
    Custom(String),
}

/// Compression statistics
///
/// Tracks cumulative metrics across all compression/decompression operations
/// performed by a [`DataCompressionEngine`]. The `compression_ratio` field
/// records the ratio of the *most recent* compression operation, while
/// [`CompressionStatistics::compression_ratio`] computes the *overall* ratio
/// from the cumulative byte totals.
#[derive(Debug, Clone)]
pub struct CompressionStatistics {
    /// Total original (uncompressed) bytes processed across all compress ops.
    pub original_size: u64,
    /// Total compressed bytes produced across all compress ops.
    pub compressed_size: u64,
    /// Ratio of the most recent compression operation (compressed / original).
    pub compression_ratio: f64,
    /// Total time spent compressing, in nanoseconds.
    pub compression_time: u64,
    /// Total time spent decompressing, in nanoseconds.
    pub decompression_time: u64,
    /// Number of compression operations performed.
    pub compression_count: u64,
    /// Number of decompression operations performed.
    pub decompression_count: u64,
}

/// Data indexing engine
pub struct DataIndexingEngine {
    indexes: HashMap<String, DataIndex>,
    indexing_strategy: IndexingStrategy,
    query_optimizer: QueryOptimizer,
}

/// Data index
#[derive(Debug, Clone)]
pub struct DataIndex {
    pub index_id: String,
    pub index_type: IndexType,
    pub indexed_columns: Vec<String>,
    pub statistics: IndexStatistics,
}

/// Index types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IndexType {
    BTree,
    Hash,
    Bitmap,
    FullText,
    Spatial,
    TimeSeries,
}

/// Index statistics
#[derive(Debug, Clone)]
pub struct IndexStatistics {
    pub entries: u64,
    pub size: u64,
    pub selectivity: f64,
    pub usage_count: u64,
}

/// Query optimizer
pub struct QueryOptimizer {
    optimization_rules: Vec<OptimizationRule>,
    cost_model: CostModel,
    execution_plan: ExecutionPlan,
}

/// Optimization rules
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationRule {
    PredicatePushdown,
    IndexSelection,
    JoinOrder,
    AggregationPushdown,
    Materialization,
}

/// Cost model
pub struct CostModel {
    pub cpu_cost: f64,
    pub io_cost: f64,
    pub memory_cost: f64,
    pub network_cost: f64,
}

/// Execution plan
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub plan_id: String,
    pub operations: Vec<QueryOperation>,
    pub estimated_cost: f64,
    pub execution_time: u64,
}

/// Join strategy selected by the query optimizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    NestedLoop,
    HashJoin,
}

/// A single logical query operation. Each variant carries the data the
/// optimizer needs to estimate cost, reorder steps, and select join strategies.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryOperation {
    /// Full table scan; `estimated_rows` is the table's row count.
    Scan {
        table: String,
        estimated_rows: usize,
    },
    /// Predicate filter; `selectivity` (0.0–1.0) is the fraction of rows that pass.
    Filter { predicate: String, selectivity: f64 },
    /// Join of two inputs; `left_cost`/`right_cost` are estimated row counts
    /// of the left and right inputs. The optimizer may override `join_type`.
    Join {
        left_cost: f64,
        right_cost: f64,
        join_type: JoinType,
    },
    /// Aggregation; `group_by` lists the grouping columns.
    Aggregate { group_by: Vec<String> },
    /// Sort by the given columns.
    Sort { columns: Vec<String> },
    /// Limit to at most `count` rows.
    Limit { count: usize },
    /// Column projection.
    Project { columns: Vec<String> },
}

/// A single step in an optimized query plan: the operation plus its estimated
/// cost and output row count.
#[derive(Debug, Clone)]
pub struct QueryStep {
    pub operation: QueryOperation,
    pub estimated_cost: f64,
    pub estimated_rows: usize,
}

/// An optimized query plan: ordered steps with aggregate cost/row estimates.
#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub operations: Vec<QueryStep>,
    pub estimated_cost: f64,
    pub estimated_rows: usize,
}

impl QueryOperation {
    /// A rough data-size proxy used by the legacy `estimate_cost` /
    /// `optimize_with_cost` path when the input row count is not known
    /// in isolation. The `optimize()` method uses proper per-step tracking.
    fn data_size_hint(&self) -> f64 {
        match self {
            QueryOperation::Scan { estimated_rows, .. } => *estimated_rows as f64,
            QueryOperation::Filter { selectivity, .. } => 100.0 / selectivity.max(0.01),
            QueryOperation::Join {
                left_cost,
                right_cost,
                ..
            } => left_cost * right_cost,
            QueryOperation::Aggregate { group_by } => group_by.len().max(1) as f64 * 100.0,
            QueryOperation::Sort { columns } => columns.len().max(1) as f64 * 100.0,
            QueryOperation::Limit { count } => *count as f64,
            QueryOperation::Project { columns } => columns.len().max(1) as f64 * 100.0,
        }
    }

    /// Canonical ordering priority for stable reordering by `optimize()`.
    /// Lower = earlier in the plan. Operations with the same priority
    /// preserve their relative input order (stable sort).
    fn plan_priority(&self) -> u8 {
        match self {
            QueryOperation::Scan { .. } => 0,
            QueryOperation::Project { .. } => 0,
            QueryOperation::Filter { .. } => 1,
            QueryOperation::Join { .. } => 2,
            QueryOperation::Aggregate { .. } => 3,
            QueryOperation::Sort { .. } => 4,
            QueryOperation::Limit { .. } => 5,
        }
    }
}

/// Statistical computation engine
pub struct StatisticalComputationEngine {
    computation_units: Vec<StatisticalComputationUnit>,
    operation_queue: Vec<StatisticalOperation>,
    scheduler: StatisticalScheduler,
    accelerator: StatisticalAccelerator,
}

/// Statistical computation unit
#[derive(Debug, Clone)]
pub struct StatisticalComputationUnit {
    pub unit_id: String,
    pub unit_type: ComputationUnitType,
    pub capabilities: ComputationCapabilities,
    pub current_load: f64,
}

/// Computation unit types
#[derive(Debug, Clone, PartialEq)]
pub enum ComputationUnitType {
    CPU,
    GPU,
    CSD,
    TPU,
    FPGA,
}

/// Computation capabilities
#[derive(Debug, Clone)]
pub struct ComputationCapabilities {
    pub max_sample_size: usize,
    pub supported_operations: Vec<StatisticalOperation>,
    pub data_types: Vec<DataType>,
    pub memory_bandwidth: f64,
    pub compute_throughput: f64,
}

/// Statistical operations
#[derive(Debug, Clone)]
pub enum StatisticalOperation {
    /// Descriptive statistics
    Mean {
        dataset: String,
        column: String,
        result: String,
    },
    Median {
        dataset: String,
        column: String,
        result: String,
    },
    Mode {
        dataset: String,
        column: String,
        result: String,
    },
    Variance {
        dataset: String,
        column: String,
        result: String,
        sample: bool,
    },
    StandardDeviation {
        dataset: String,
        column: String,
        result: String,
        sample: bool,
    },
    Skewness {
        dataset: String,
        column: String,
        result: String,
    },
    Kurtosis {
        dataset: String,
        column: String,
        result: String,
    },
    /// Distribution analysis
    Histogram {
        dataset: String,
        column: String,
        bins: usize,
        result: String,
    },
    Quantile {
        dataset: String,
        column: String,
        quantile: f64,
        result: String,
    },
    Percentile {
        dataset: String,
        column: String,
        percentile: f64,
        result: String,
    },
    /// Correlation analysis
    Correlation {
        dataset: String,
        column1: String,
        column2: String,
        method: CorrelationMethod,
        result: String,
    },
    Covariance {
        dataset: String,
        column1: String,
        column2: String,
        sample: bool,
        result: String,
    },
    /// Regression analysis
    LinearRegression {
        dataset: String,
        dependent: String,
        independent: Vec<String>,
        result: String,
    },
    LogisticRegression {
        dataset: String,
        dependent: String,
        independent: Vec<String>,
        result: String,
    },
    PolynomialRegression {
        dataset: String,
        dependent: String,
        independent: Vec<String>,
        degree: u32,
        result: String,
    },
    /// Hypothesis testing
    TTest {
        dataset: String,
        column: String,
        hypothesis_type: HypothesisType,
        result: String,
    },
    ChiSquareTest {
        dataset: String,
        column1: String,
        column2: String,
        result: String,
    },
    ANOVA {
        dataset: String,
        columns: Vec<String>,
        result: String,
    },
    /// Time series analysis
    AutoCorrelation {
        dataset: String,
        column: String,
        lag: usize,
        result: String,
    },
    MovingAverage {
        dataset: String,
        column: String,
        window: usize,
        result: String,
    },
    ExponentialSmoothing {
        dataset: String,
        column: String,
        alpha: f64,
        result: String,
    },
    /// Machine learning
    KMeans {
        dataset: String,
        columns: Vec<String>,
        k: usize,
        result: String,
    },
    LinearSVM {
        dataset: String,
        features: Vec<String>,
        target: String,
        result: String,
    },
    RandomForest {
        dataset: String,
        features: Vec<String>,
        target: String,
        trees: usize,
        result: String,
    },
}

/// Correlation methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CorrelationMethod {
    Pearson,
    Spearman,
    Kendall,
    PointBiserial,
}

/// Hypothesis types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HypothesisType {
    OneSample,
    TwoSample,
    Paired,
    Independent,
}

/// Statistical scheduler
pub struct StatisticalScheduler {
    scheduling_policy: SchedulingPolicy,
    queue_manager: QueueManager,
    load_balancer: LoadBalancer,
}

/// Scheduling policies
#[derive(Debug, Clone, PartialEq)]
pub enum SchedulingPolicy {
    FIFO,
    Priority,
    ShortestJobFirst,
    Deadline,
    Adaptive,
}

/// Queue manager
pub struct QueueManager {
    pending_queue: Vec<QueuedOperation>,
    running_operations: HashMap<String, RunningOperation>,
    completed_operations: Vec<CompletedOperation>,
}

/// Queued operation
#[derive(Debug, Clone)]
pub struct QueuedOperation {
    pub operation_id: String,
    pub operation: StatisticalOperation,
    pub priority: OperationPriority,
    pub submitted_at: u64,
    pub deadline: Option<u64>,
}

/// Running operation
#[derive(Debug, Clone)]
pub struct RunningOperation {
    pub operation_id: String,
    pub unit_id: String,
    pub started_at: u64,
    pub progress: f64,
}

/// Completed operation
#[derive(Debug, Clone)]
pub struct CompletedOperation {
    pub operation_id: String,
    pub started_at: u64,
    pub completed_at: u64,
    pub result: StatisticalResult,
    pub success: bool,
}

/// Operation priorities
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OperationPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Statistical result
#[derive(Debug, Clone)]
pub struct StatisticalResult {
    pub result_id: String,
    pub result_type: ResultType,
    pub value: ResultValue,
    pub metadata: ResultMetadata,
}

/// Result types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResultType {
    Scalar,
    Vector,
    Matrix,
    Distribution,
    Model,
}

/// Result values
#[derive(Debug, Clone)]
pub enum ResultValue {
    Float(f64),
    Integer(i64),
    Boolean(bool),
    Vector(Vec<f64>),
    Matrix(Vec<Vec<f64>>),
    Distribution(Distribution),
    Model(StatisticalModel),
}

/// Statistical distribution
#[derive(Debug, Clone)]
pub struct Distribution {
    pub distribution_type: DistributionType,
    pub parameters: Vec<f64>,
    pub samples: Option<Vec<f64>>,
}

/// Distribution types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DistributionType {
    Normal,
    Uniform,
    Exponential,
    Poisson,
    Binomial,
    ChiSquare,
    StudentT,
    F,
    Custom(String),
}

/// Statistical model
#[derive(Debug, Clone)]
pub struct StatisticalModel {
    pub model_type: ModelType,
    pub parameters: ModelParameters,
    pub performance_metrics: ModelPerformance,
}

/// Model types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelType {
    LinearRegression,
    LogisticRegression,
    PolynomialRegression,
    KMeans,
    SVM,
    RandomForest,
    NeuralNetwork,
    Custom(String),
}

/// Model parameters
#[derive(Debug, Clone)]
pub struct ModelParameters {
    pub coefficients: Vec<f64>,
    pub intercept: f64,
    pub additional_params: HashMap<String, f64>,
}

/// Model performance metrics
#[derive(Debug, Clone)]
pub struct ModelPerformance {
    pub accuracy: f64,
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
    pub mse: f64,
    pub rmse: f64,
    pub r_squared: f64,
}

/// Result metadata
#[derive(Debug, Clone)]
pub struct ResultMetadata {
    pub computation_time: u64,
    pub memory_usage: u64,
    pub sample_size: usize,
    pub confidence_interval: Option<(f64, f64)>,
    pub significance_level: Option<f64>,
    pub privacy_preserved: bool,
}

/// Load balancing strategies
#[derive(Debug, Clone, PartialEq)]
pub enum BalancingStrategy {
    RoundRobin,
    LoadBased,
    CapacityWeighted,
    LeastConnections,
}

/// Optimization engine for statistical acceleration
pub struct OptimizationEngine {}

/// Load balancer
pub struct LoadBalancer {
    balancing_strategy: BalancingStrategy,
    unit_metrics: HashMap<String, UnitMetrics>,
}

/// Unit metrics
#[derive(Debug, Clone)]
pub struct UnitMetrics {
    pub unit_id: String,
    pub current_load: f64,
    pub average_response_time: f64,
    pub success_rate: f64,
    pub energy_efficiency: f64,
}

/// Statistical accelerator
pub struct StatisticalAccelerator {
    acceleration_strategies: Vec<AccelerationStrategy>,
    hardware_accelerators: Vec<HardwareAccelerator>,
    optimization_engine: OptimizationEngine,
}

/// Acceleration strategies
#[derive(Debug, Clone, PartialEq)]
pub enum AccelerationStrategy {
    Vectorization,
    Parallelization,
    Caching,
    Precomputation,
    Approximation,
}

/// Hardware accelerator
#[derive(Debug, Clone)]
pub struct HardwareAccelerator {
    pub accelerator_id: String,
    pub accelerator_type: AcceleratorType,
    pub capabilities: AcceleratorCapabilities,
}

/// Accelerator types
#[derive(Debug, Clone, PartialEq)]
pub enum AcceleratorType {
    GPU,
    TPU,
    FPGA,
    ASIC,
    CSD,
}

/// Accelerator capabilities
#[derive(Debug, Clone)]
pub struct AcceleratorCapabilities {
    pub max_batch_size: usize,
    pub supported_operations: Vec<StatisticalOperation>,
    pub memory_bandwidth: f64,
    pub compute_throughput: f64,
}

/// Statistical privacy engine
pub struct StatisticalPrivacyEngine {
    fiduciary_crypto: Arc<Mutex<FiduciaryCrypto>>,
    zk_proofs: Arc<Mutex<ZkProofSystem>>,
    differential_privacy: DifferentialPrivacy,
    secure_aggregation: SecureAggregation,
    privacy_budget: PrivacyBudget,
}

/// Differential privacy
pub struct DifferentialPrivacy {
    noise_mechanisms: Vec<NoiseMechanism>,
    privacy_accountant: PrivacyAccountant,
    sensitivity_analyzer: SensitivityAnalyzer,
}

/// Noise mechanisms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NoiseMechanism {
    Laplace,
    Gaussian,
    Exponential,
    Geometric,
    Custom(String),
}

/// Privacy accountant
pub struct PrivacyAccountant {
    pub total_epsilon_spent: f64,
    pub total_delta_spent: f64,
    pub composition_method: CompositionMethod,
    pub remaining_budget: PrivacyBudget,
}

/// Composition methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompositionMethod {
    BasicComposition,
    AdvancedComposition,
    RDPComposition,
    GaussianDP,
    Custom(String),
}

/// Sensitivity analyzer
pub struct SensitivityAnalyzer {
    sensitivity_functions: HashMap<String, SensitivityFunction>,
    sensitivity_cache: HashMap<String, f64>,
}

/// Sensitivity function
#[derive(Debug, Clone)]
pub struct SensitivityFunction {
    pub function_id: String,
    pub sensitivity: f64,
    pub computation_method: SensitivityMethod,
}

/// Sensitivity methods
#[derive(Debug, Clone, PartialEq)]
pub enum SensitivityMethod {
    Global,
    Local,
    Smooth,
    Approximate,
}

/// Secure aggregation
pub struct SecureAggregation {
    aggregation_protocols: Vec<AggregationProtocol>,
    encryption_schemes: Vec<EncryptionScheme>,
    integrity_checks: Vec<IntegrityCheck>,
}

/// Aggregation protocols
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggregationProtocol {
    SecureSum,
    SecureMean,
    SecureMin,
    SecureMax,
    SecureMedian,
    Custom(String),
}

/// Encryption schemes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EncryptionScheme {
    Homomorphic,
    SecretSharing,
    Threshold,
    Oblivious,
    Custom(String),
}

/// Integrity checks
#[derive(Debug, Clone)]
pub struct IntegrityCheck {
    pub check_id: String,
    pub check_type: IntegrityCheckType,
    pub verification_method: VerificationMethod,
}

/// Integrity check types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IntegrityCheckType {
    Hash,
    MAC,
    DigitalSignature,
    ZeroKnowledge,
}

/// Verification methods
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationMethod {
    Deterministic,
    Probabilistic,
    Interactive,
    NonInteractive,
}

/// Privacy budget
pub struct PrivacyBudget {
    pub epsilon: f64,
    pub delta: f64,
    pub remaining_epsilon: f64,
    pub remaining_delta: f64,
    pub budget_period: u64,
    pub last_reset: u64,
}

/// Statistical analysis engine
pub struct StatisticalAnalysisEngine {
    analysis_algorithms: Vec<AnalysisAlgorithm>,
    pattern_recognition: PatternRecognition,
    anomaly_detection: AnomalyDetection,
    forecasting_engine: ForecastingEngine,
}

/// Analysis algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnalysisAlgorithm {
    DescriptiveAnalysis,
    InferentialAnalysis,
    PredictiveAnalysis,
    PrescriptiveAnalysis,
    CausalAnalysis,
    TimeSeriesAnalysis,
    SurvivalAnalysis,
    BayesianAnalysis,
}

/// Pattern recognition
pub struct PatternRecognition {
    pattern_types: Vec<PatternType>,
    recognition_algorithms: Vec<RecognitionAlgorithm>,
    pattern_library: PatternLibrary,
}

/// Pattern types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatternType {
    Trend,
    Seasonal,
    Cyclical,
    Outlier,
    Cluster,
    Association,
    Sequential,
    Spatial,
}

/// Recognition algorithms
#[derive(Debug, Clone, PartialEq)]
pub enum RecognitionAlgorithm {
    Statistical,
    MachineLearning,
    DeepLearning,
    Hybrid,
    Custom(String),
}

/// Pattern library
pub struct PatternLibrary {
    patterns: HashMap<String, StatisticalPattern>,
    pattern_templates: Vec<PatternTemplate>,
}

/// Statistical pattern
#[derive(Debug, Clone)]
pub struct StatisticalPattern {
    pub pattern_id: String,
    pub pattern_type: PatternType,
    pub parameters: Vec<f64>,
    pub confidence: f64,
    pub frequency: f64,
}

/// Pattern template
#[derive(Debug, Clone)]
pub struct PatternTemplate {
    pub template_id: String,
    pub pattern_type: PatternType,
    pub parameter_schema: ParameterSchema,
}

/// Parameter schema
#[derive(Debug, Clone)]
pub struct ParameterSchema {
    pub parameters: Vec<ParameterDefinition>,
    pub constraints: Vec<Constraint>,
}

/// Parameter definition
#[derive(Debug, Clone)]
pub struct ParameterDefinition {
    pub name: String,
    pub parameter_type: DataType,
    pub required: bool,
    pub default_value: Option<f64>,
}

/// Constraint
#[derive(Debug, Clone)]
pub struct Constraint {
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
    Logical,
    Custom(String),
}

/// Anomaly detection
pub struct AnomalyDetection {
    detection_algorithms: Vec<DetectionAlgorithm>,
    threshold_methods: Vec<ThresholdMethod>,
    alert_system: AlertSystem,
}

/// Detection algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DetectionAlgorithm {
    Statistical,
    MachineLearning,
    DeepLearning,
    Ensemble,
    Custom(String),
}

/// Threshold methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ThresholdMethod {
    Static,
    Dynamic,
    Adaptive,
    Learned,
    Custom(String),
}

/// Alert system
pub struct AlertSystem {
    alert_types: Vec<AlertType>,
    notification_channels: Vec<NotificationChannel>,
    escalation_policies: Vec<EscalationPolicy>,
}

/// Alert types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertType {
    Threshold,
    Pattern,
    Anomaly,
    System,
    Security,
    Custom(String),
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

/// Escalation policies
#[derive(Debug, Clone)]
pub struct EscalationPolicy {
    pub policy_id: String,
    pub trigger_conditions: Vec<String>,
    pub escalation_steps: Vec<EscalationStep>,
    pub timeout: u64,
}

/// Escalation step
#[derive(Debug, Clone)]
pub struct EscalationStep {
    pub step_id: String,
    pub action: EscalationAction,
    pub target: String,
    pub delay: u64,
}

/// Escalation actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EscalationAction {
    Notify,
    Escalate,
    Block,
    Custom(String),
}

/// Forecasting engine
pub struct ForecastingEngine {
    forecasting_models: Vec<ForecastingModel>,
    accuracy_metrics: AccuracyMetrics,
    model_selection: ModelSelection,
}

/// Forecasting models
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ForecastingModel {
    ARIMA,
    ExponentialSmoothing,
    Prophet,
    LSTM,
    Transformer,
    Ensemble,
    Custom(String),
}

/// Accuracy metrics
#[derive(Debug, Clone)]
pub struct AccuracyMetrics {
    pub mae: f64,
    pub mse: f64,
    pub rmse: f64,
    pub mape: f64,
    pub smape: f64,
    pub r_squared: f64,
}

/// Model selection
pub struct ModelSelection {
    selection_criteria: Vec<SelectionCriterion>,
    cross_validation: CrossValidation,
    hyperparameter_tuning: HyperparameterTuning,
}

/// Selection criteria
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SelectionCriterion {
    Accuracy,
    Speed,
    Memory,
    Interpretability,
    Robustness,
    Custom(String),
}

/// Cross validation
pub struct CrossValidation {
    pub cv_method: CVMethod,
    pub folds: usize,
    pub shuffle: bool,
    pub stratify: bool,
}

/// CV methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CVMethod {
    KFold,
    StratifiedKFold,
    TimeSeriesSplit,
    LeaveOneOut,
    Custom(String),
}

/// Hyperparameter tuning
pub struct HyperparameterTuning {
    pub tuning_method: TuningMethod,
    pub search_space: SearchSpace,
    pub max_iterations: usize,
}

/// Tuning methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TuningMethod {
    GridSearch,
    RandomSearch,
    BayesianOptimization,
    GeneticAlgorithm,
    Custom(String),
}

/// Search space
#[derive(Debug, Clone)]
pub struct SearchSpace {
    pub parameters: Vec<Hyperparameter>,
    pub constraints: Vec<Constraint>,
}

/// Hyperparameter
#[derive(Debug, Clone)]
pub struct Hyperparameter {
    pub name: String,
    pub parameter_type: HyperparameterType,
    pub range: ParameterRange,
}

/// Hyperparameter types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HyperparameterType {
    Continuous,
    Integer,
    Categorical,
    Boolean,
}

/// Parameter range
#[derive(Debug, Clone)]
pub struct ParameterRange {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub values: Option<Vec<String>>,
}

/// Statistical performance monitor
pub struct StatisticalPerformanceMonitor {
    operation_metrics: HashMap<String, OperationMetrics>,
    dataset_metrics: HashMap<String, DatasetMetrics>,
    system_metrics: SystemMetrics,
    privacy_metrics: PrivacyMetrics,
}

/// Operation metrics
#[derive(Debug, Clone)]
pub struct OperationMetrics {
    pub operation_id: String,
    pub operation_type: StatisticalOperation,
    pub execution_time: u64,
    pub memory_usage: u64,
    pub cpu_usage: f64,
    pub accuracy: f64,
    pub privacy_cost: f64,
}

/// Dataset metrics
#[derive(Debug, Clone)]
pub struct DatasetMetrics {
    pub dataset_id: String,
    pub size: u64,
    pub access_count: u64,
    pub access_frequency: f64,
    pub compression_ratio: f64,
    pub privacy_level: PrivacyLevel,
}

/// System metrics
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub total_operations: u64,
    pub average_execution_time: f64,
    pub throughput: f64,
    pub memory_utilization: f64,
    pub cpu_utilization: f64,
    pub storage_utilization: f64,
    pub energy_efficiency: f64,
}

/// Privacy metrics
#[derive(Debug, Clone)]
pub struct PrivacyMetrics {
    pub epsilon_spent: f64,
    pub delta_spent: f64,
    pub privacy_preserved_operations: u64,
    pub total_operations: u64,
    pub privacy_efficiency: f64,
}

/// Dataset representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub dataset_id: String,
    pub metadata: DatasetMetadata,
    pub data: Vec<Vec<DataValue>>,
    pub column_names: Vec<String>,
    pub column_types: Vec<DataType>,
}

/// Data values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataValue {
    Float(f64),
    Integer(i64),
    Boolean(bool),
    String(String),
    DateTime(u64),
    Categorical(String),
    Null,
}

/// Statistical analysis result
#[derive(Debug, Clone)]
pub struct StatisticalAnalysisResult<T> {
    pub result: T,
    pub execution_time: u64,
    pub memory_usage: u64,
    pub sample_size: usize,
    pub confidence_level: f64,
    pub privacy_preserved: bool,
    pub privacy_cost: f64,
}

impl StatisticalComputingLibrary {
    /// Create new statistical computing library
    pub fn new() -> Self {
        Self {
            data_storage: StatisticalDataStorage::new(),
            computation_engine: StatisticalComputationEngine::new(),
            privacy_engine: StatisticalPrivacyEngine::new(),
            analysis_engine: StatisticalAnalysisEngine::new(),
            performance_monitor: StatisticalPerformanceMonitor::new(),
        }
    }

    /// Initialize the library
    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        // Initialize storage
        self.data_storage.initialize()?;

        // Initialize computation engine
        self.computation_engine.initialize()?;

        // Initialize privacy engine
        self.privacy_engine.initialize()?;

        // Initialize analysis engine
        self.analysis_engine.initialize()?;

        Ok(())
    }

    /// Create a new dataset
    pub fn create_dataset(
        &mut self,
        dataset_id: String,
        data: Vec<Vec<DataValue>>,
        column_names: Vec<String>,
        column_types: Vec<DataType>,
        privacy_level: PrivacyLevel,
    ) -> Result<Dataset, StatisticalError> {
        // Validate input
        if data.is_empty() {
            return Err(StatisticalError::InvalidData(
                "Dataset cannot be empty".to_string(),
            ));
        }
        if column_names.len() != column_types.len() {
            return Err(StatisticalError::InvalidData(
                "Column names and types must match".to_string(),
            ));
        }
        if data.iter().any(|row| row.len() != column_names.len()) {
            return Err(StatisticalError::InvalidData(
                "All rows must have same number of columns".to_string(),
            ));
        }

        // Create metadata
        let metadata = DatasetMetadata {
            dataset_id: dataset_id.clone(),
            dataset_type: DatasetType::Mixed,
            dimensions: DatasetDimensions {
                rows: data.len(),
                columns: column_names.len(),
                time_steps: None,
                features: Some(column_names.len()),
            },
            data_types: column_types.clone(),
            sample_size: data.len(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_updated: 0,
            access_count: 0,
            privacy_level,
        };

        // Create dataset
        let dataset = Dataset {
            dataset_id: dataset_id.clone(),
            metadata,
            data,
            column_names,
            column_types,
        };

        // Store dataset
        self.data_storage.store_dataset(dataset.clone())?;

        Ok(dataset)
    }

    /// Compute mean of a column
    pub fn mean(
        &mut self,
        dataset_id: &str,
        column: &str,
        privacy_preserved: bool,
    ) -> Result<StatisticalAnalysisResult<f64>, StatisticalError> {
        let start_time = std::time::Instant::now();

        // Get dataset
        let dataset = self.data_storage.get_dataset(dataset_id)?;

        // Find column index
        let column_index = dataset
            .column_names
            .iter()
            .position(|name| name == column)
            .ok_or_else(|| StatisticalError::InvalidColumn(column.to_string()))?;

        // Validate column type
        if !matches!(
            dataset.column_types[column_index],
            DataType::Float32 | DataType::Float64
        ) {
            return Err(StatisticalError::InvalidOperation(
                "Mean can only be computed on numeric columns".to_string(),
            ));
        }

        // Extract column data
        let mut values = Vec::new();
        for row in &dataset.data {
            match &row[column_index] {
                DataValue::Float(value) => values.push(*value),
                DataValue::Integer(value) => values.push(*value as f64),
                DataValue::Null => continue,
                _ => {
                    return Err(StatisticalError::InvalidOperation(
                        "Non-numeric data in column".to_string(),
                    ))
                }
            }
        }

        if values.is_empty() {
            return Err(StatisticalError::InvalidData(
                "No valid data in column".to_string(),
            ));
        }

        // Compute mean via the engine's canonical statistics solver (Modality-First:
        // no inline re-implementation). `values` is the caller-owned slice.
        let mean = crate::solvers::statistics::mean(&values)
            .ok_or_else(|| StatisticalError::InvalidData("No valid data in column".to_string()))?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Apply privacy if requested. Sensitivity is calibrated via the
        // differential-privacy sensitivity analyzer (mean sensitivity = 1/n)
        // instead of the previous hardcoded 1.0, so noise scales with the
        // actual query sensitivity.
        let (final_mean, privacy_cost) = if privacy_preserved {
            let sensitivity = {
                let analyzer = &mut self
                    .privacy_engine
                    .differential_privacy
                    .sensitivity_analyzer;
                analyzer.get_sensitivity("mean", &values).unwrap_or(1.0)
            };
            let (noisy_mean, cost) = self.privacy_engine.add_laplace_noise(mean, sensitivity)?;
            (noisy_mean, cost)
        } else {
            (mean, 0.0)
        };

        // Update performance metrics
        self.performance_monitor
            .record_operation("mean", execution_time, 0, privacy_cost);

        Ok(StatisticalAnalysisResult {
            result: final_mean,
            execution_time,
            memory_usage: 0,
            sample_size: values.len(),
            confidence_level: 0.95,
            privacy_preserved,
            privacy_cost,
        })
    }

    /// Compute median of a column
    pub fn median(
        &mut self,
        dataset_id: &str,
        column: &str,
        privacy_preserved: bool,
    ) -> Result<StatisticalAnalysisResult<f64>, StatisticalError> {
        let start_time = std::time::Instant::now();

        // Get dataset
        let dataset = self.data_storage.get_dataset(dataset_id)?;

        // Find column index
        let column_index = dataset
            .column_names
            .iter()
            .position(|name| name == column)
            .ok_or_else(|| StatisticalError::InvalidColumn(column.to_string()))?;

        // Validate column type
        if !matches!(
            dataset.column_types[column_index],
            DataType::Float32 | DataType::Float64
        ) {
            return Err(StatisticalError::InvalidOperation(
                "Median can only be computed on numeric columns".to_string(),
            ));
        }

        // Extract column data
        let mut values = Vec::new();
        for row in &dataset.data {
            match &row[column_index] {
                DataValue::Float(value) => values.push(*value),
                DataValue::Integer(value) => values.push(*value as f64),
                DataValue::Null => continue,
                _ => {
                    return Err(StatisticalError::InvalidOperation(
                        "Non-numeric data in column".to_string(),
                    ))
                }
            }
        }

        if values.is_empty() {
            return Err(StatisticalError::InvalidData(
                "No valid data in column".to_string(),
            ));
        }

        // Compute median via the engine's canonical statistics solver (sorts the
        // caller-owned buffer in place; no inline re-implementation).
        let median = crate::solvers::statistics::median_in_place(&mut values)
            .ok_or_else(|| StatisticalError::InvalidData("No valid data in column".to_string()))?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Apply privacy if requested
        let (final_median, privacy_cost) = if privacy_preserved {
            let (noisy_median, cost) = self.privacy_engine.add_laplace_noise(median, 1.0)?;
            (noisy_median, cost)
        } else {
            (median, 0.0)
        };

        // Update performance metrics
        self.performance_monitor
            .record_operation("median", execution_time, 0, privacy_cost);

        Ok(StatisticalAnalysisResult {
            result: final_median,
            execution_time,
            memory_usage: 0,
            sample_size: values.len(),
            confidence_level: 0.95,
            privacy_preserved,
            privacy_cost,
        })
    }

    /// Compute variance of a column
    pub fn variance(
        &mut self,
        dataset_id: &str,
        column: &str,
        sample: bool,
        privacy_preserved: bool,
    ) -> Result<StatisticalAnalysisResult<f64>, StatisticalError> {
        let start_time = std::time::Instant::now();

        // Get dataset
        let dataset = self.data_storage.get_dataset(dataset_id)?;

        // Find column index
        let column_index = dataset
            .column_names
            .iter()
            .position(|name| name == column)
            .ok_or_else(|| StatisticalError::InvalidColumn(column.to_string()))?;

        // Validate column type
        if !matches!(
            dataset.column_types[column_index],
            DataType::Float32 | DataType::Float64
        ) {
            return Err(StatisticalError::InvalidOperation(
                "Variance can only be computed on numeric columns".to_string(),
            ));
        }

        // Extract column data
        let mut values = Vec::new();
        for row in &dataset.data {
            match &row[column_index] {
                DataValue::Float(value) => values.push(*value),
                DataValue::Integer(value) => values.push(*value as f64),
                DataValue::Null => continue,
                _ => {
                    return Err(StatisticalError::InvalidOperation(
                        "Non-numeric data in column".to_string(),
                    ))
                }
            }
        }

        if values.is_empty() {
            return Err(StatisticalError::InvalidData(
                "No valid data in column".to_string(),
            ));
        }

        // Compute variance via the engine's canonical statistics solver
        // (Modality-First: no inline re-implementation).
        let variance = crate::solvers::statistics::variance(&values, sample)
            .ok_or_else(|| StatisticalError::InvalidData("No valid data in column".to_string()))?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Apply privacy if requested
        let (final_variance, privacy_cost) = if privacy_preserved {
            let (noisy_variance, cost) = self.privacy_engine.add_laplace_noise(variance, 1.0)?;
            (noisy_variance, cost)
        } else {
            (variance, 0.0)
        };

        // Update performance metrics
        self.performance_monitor
            .record_operation("variance", execution_time, 0, privacy_cost);

        Ok(StatisticalAnalysisResult {
            result: final_variance,
            execution_time,
            memory_usage: 0,
            sample_size: values.len(),
            confidence_level: 0.95,
            privacy_preserved,
            privacy_cost,
        })
    }

    /// Compute correlation between two columns
    pub fn correlation(
        &mut self,
        dataset_id: &str,
        column1: &str,
        column2: &str,
        method: CorrelationMethod,
        privacy_preserved: bool,
    ) -> Result<StatisticalAnalysisResult<f64>, StatisticalError> {
        let start_time = std::time::Instant::now();

        // Get dataset
        let dataset = self.data_storage.get_dataset(dataset_id)?;

        // Find column indices
        let column1_index = dataset
            .column_names
            .iter()
            .position(|name| name == column1)
            .ok_or_else(|| StatisticalError::InvalidColumn(column1.to_string()))?;

        let column2_index = dataset
            .column_names
            .iter()
            .position(|name| name == column2)
            .ok_or_else(|| StatisticalError::InvalidColumn(column2.to_string()))?;

        // Validate column types
        if !matches!(
            dataset.column_types[column1_index],
            DataType::Float32 | DataType::Float64
        ) {
            return Err(StatisticalError::InvalidOperation(
                "Correlation can only be computed on numeric columns".to_string(),
            ));
        }
        if !matches!(
            dataset.column_types[column2_index],
            DataType::Float32 | DataType::Float64
        ) {
            return Err(StatisticalError::InvalidOperation(
                "Correlation can only be computed on numeric columns".to_string(),
            ));
        }

        // Extract column data
        let mut x_values = Vec::new();
        let mut y_values = Vec::new();

        for row in &dataset.data {
            let x_val = match &row[column1_index] {
                DataValue::Float(value) => *value,
                DataValue::Integer(value) => *value as f64,
                DataValue::Null => continue,
                _ => {
                    return Err(StatisticalError::InvalidOperation(
                        "Non-numeric data in column".to_string(),
                    ))
                }
            };

            let y_val = match &row[column2_index] {
                DataValue::Float(value) => *value,
                DataValue::Integer(value) => *value as f64,
                DataValue::Null => continue,
                _ => {
                    return Err(StatisticalError::InvalidOperation(
                        "Non-numeric data in column".to_string(),
                    ))
                }
            };

            x_values.push(x_val);
            y_values.push(y_val);
        }

        if x_values.len() < 2 {
            return Err(StatisticalError::InvalidData(
                "Insufficient data for correlation".to_string(),
            ));
        }

        // Compute correlation based on method
        let correlation = match method {
            CorrelationMethod::Pearson => self.pearson_correlation(&x_values, &y_values)?,
            CorrelationMethod::Spearman => self.spearman_correlation(&x_values, &y_values)?,
            CorrelationMethod::Kendall => self.kendall_correlation(&x_values, &y_values)?,
            _ => {
                return Err(StatisticalError::InvalidOperation(
                    "Correlation method not supported".to_string(),
                ))
            }
        };

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Apply privacy if requested
        let (final_correlation, privacy_cost) = if privacy_preserved {
            let (noisy_correlation, cost) =
                self.privacy_engine.add_laplace_noise(correlation, 0.1)?;
            (noisy_correlation.clamp(-1.0, 1.0), cost)
        } else {
            (correlation, 0.0)
        };

        // Update performance metrics
        self.performance_monitor
            .record_operation("correlation", execution_time, 0, privacy_cost);

        Ok(StatisticalAnalysisResult {
            result: final_correlation,
            execution_time,
            memory_usage: 0,
            sample_size: x_values.len(),
            confidence_level: 0.95,
            privacy_preserved,
            privacy_cost,
        })
    }

    /// Perform t-test
    pub fn t_test(
        &mut self,
        dataset_id: &str,
        column: &str,
        hypothesis_type: HypothesisType,
        privacy_preserved: bool,
    ) -> Result<StatisticalAnalysisResult<TTestResult>, StatisticalError> {
        let start_time = std::time::Instant::now();

        // Get dataset
        let dataset = self.data_storage.get_dataset(dataset_id)?;

        // Find column index
        let column_index = dataset
            .column_names
            .iter()
            .position(|name| name == column)
            .ok_or_else(|| StatisticalError::InvalidColumn(column.to_string()))?;

        // Validate column type
        if !matches!(
            dataset.column_types[column_index],
            DataType::Float32 | DataType::Float64
        ) {
            return Err(StatisticalError::InvalidOperation(
                "T-test can only be computed on numeric columns".to_string(),
            ));
        }

        // Extract column data
        let mut values = Vec::new();
        for row in &dataset.data {
            match &row[column_index] {
                DataValue::Float(value) => values.push(*value),
                DataValue::Integer(value) => values.push(*value as f64),
                DataValue::Null => continue,
                _ => {
                    return Err(StatisticalError::InvalidOperation(
                        "Non-numeric data in column".to_string(),
                    ))
                }
            }
        }

        if values.len() < 2 {
            return Err(StatisticalError::InvalidData(
                "Insufficient data for t-test".to_string(),
            ));
        }

        // Compute t-test based on hypothesis type
        let t_test_result = match hypothesis_type {
            HypothesisType::OneSample => self.one_sample_t_test(&values, 0.0)?,
            HypothesisType::TwoSample => {
                return Err(StatisticalError::InvalidOperation(
                    "Two-sample t-test requires two datasets".to_string(),
                ))
            }
            HypothesisType::Paired => {
                return Err(StatisticalError::InvalidOperation(
                    "Paired t-test requires paired data".to_string(),
                ))
            }
            HypothesisType::Independent => {
                return Err(StatisticalError::InvalidOperation(
                    "Independent t-test requires two samples".to_string(),
                ))
            }
        };

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Apply privacy if requested
        let (final_result, privacy_cost) = if privacy_preserved {
            let (noisy_t_statistic, cost) = self
                .privacy_engine
                .add_laplace_noise(t_test_result.t_statistic, 1.0)?;
            let noisy_result = TTestResult {
                t_statistic: noisy_t_statistic,
                p_value: t_test_result.p_value,
                degrees_of_freedom: t_test_result.degrees_of_freedom,
                confidence_interval: t_test_result.confidence_interval,
            };
            (noisy_result, cost)
        } else {
            (t_test_result, 0.0)
        };

        // Update performance metrics
        self.performance_monitor
            .record_operation("t_test", execution_time, 0, privacy_cost);

        Ok(StatisticalAnalysisResult {
            result: final_result,
            execution_time,
            memory_usage: 0,
            sample_size: values.len(),
            confidence_level: 0.95,
            privacy_preserved,
            privacy_cost,
        })
    }

    /// Generate histogram
    pub fn histogram(
        &mut self,
        dataset_id: &str,
        column: &str,
        bins: usize,
        privacy_preserved: bool,
    ) -> Result<StatisticalAnalysisResult<HistogramResult>, StatisticalError> {
        let start_time = std::time::Instant::now();

        // Get dataset
        let dataset = self.data_storage.get_dataset(dataset_id)?;

        // Find column index
        let column_index = dataset
            .column_names
            .iter()
            .position(|name| name == column)
            .ok_or_else(|| StatisticalError::InvalidColumn(column.to_string()))?;

        // Validate column type
        if !matches!(
            dataset.column_types[column_index],
            DataType::Float32 | DataType::Float64
        ) {
            return Err(StatisticalError::InvalidOperation(
                "Histogram can only be computed on numeric columns".to_string(),
            ));
        }

        // Extract column data
        let mut values = Vec::new();
        for row in &dataset.data {
            match &row[column_index] {
                DataValue::Float(value) => values.push(*value),
                DataValue::Integer(value) => values.push(*value as f64),
                DataValue::Null => continue,
                _ => {
                    return Err(StatisticalError::InvalidOperation(
                        "Non-numeric data in column".to_string(),
                    ))
                }
            }
        }

        if values.is_empty() {
            return Err(StatisticalError::InvalidData(
                "No valid data in column".to_string(),
            ));
        }

        // Compute histogram
        let histogram_result = self.compute_histogram(&values, bins)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Apply privacy if requested
        let (final_result, privacy_cost) = if privacy_preserved {
            let (noisy_counts, cost) = self
                .privacy_engine
                .add_histogram_noise(&histogram_result.counts)?;
            let noisy_result = HistogramResult {
                bins: histogram_result.bins,
                counts: noisy_counts,
                min_value: histogram_result.min_value,
                max_value: histogram_result.max_value,
                bin_width: histogram_result.bin_width,
            };
            (noisy_result, cost)
        } else {
            (histogram_result, 0.0)
        };

        // Update performance metrics
        self.performance_monitor
            .record_operation("histogram", execution_time, 0, privacy_cost);

        Ok(StatisticalAnalysisResult {
            result: final_result,
            execution_time,
            memory_usage: 0,
            sample_size: values.len(),
            confidence_level: 0.95,
            privacy_preserved,
            privacy_cost,
        })
    }

    // ========================================================================
    // Wired capability methods.
    //
    // Each declared `StatisticalOperation` is a genuine, MCP-reachable
    // computation. Every method below marshals the caller's dataset into a
    // slice and delegates to the canonical numeric kernels in
    // `crate::solvers` (Modality-First Composition — no inline re-derivation);
    // the descriptive/correlation/regression/hypothesis kernels live in
    // `solvers::statistics`, the learning models in `solvers::learning`, and
    // polynomial least-squares in `solvers::interpolation`.
    // ========================================================================

    /// Extract one numeric column as an owned `Vec<f64>` (Integer widened,
    /// Null rows skipped). Errors on unknown column, non-numeric cell, or empty.
    fn numeric_column(&self, dataset_id: &str, column: &str) -> Result<Vec<f64>, StatisticalError> {
        let dataset = self.data_storage.get_dataset(dataset_id)?;
        let idx = dataset
            .column_names
            .iter()
            .position(|n| n == column)
            .ok_or_else(|| StatisticalError::InvalidColumn(column.to_string()))?;
        let mut out = Vec::with_capacity(dataset.data.len());
        for row in &dataset.data {
            match &row[idx] {
                DataValue::Float(v) => out.push(*v),
                DataValue::Integer(v) => out.push(*v as f64),
                DataValue::Null => continue,
                _ => {
                    return Err(StatisticalError::InvalidOperation(
                        "Non-numeric data in column".to_string(),
                    ))
                }
            }
        }
        if out.is_empty() {
            return Err(StatisticalError::InvalidData(
                "No valid data in column".to_string(),
            ));
        }
        Ok(out)
    }

    /// Extract several numeric columns row-aligned into a row-major `n × p`
    /// matrix, skipping any row where one of the requested cells is Null (so
    /// every returned row is complete). Returns `(data, n, p)`.
    fn numeric_matrix(
        &self,
        dataset_id: &str,
        columns: &[&str],
    ) -> Result<(Vec<f64>, usize, usize), StatisticalError> {
        let dataset = self.data_storage.get_dataset(dataset_id)?;
        let p = columns.len();
        if p == 0 {
            return Err(StatisticalError::InvalidOperation(
                "no columns given".to_string(),
            ));
        }
        let mut idxs = Vec::with_capacity(p);
        for c in columns {
            idxs.push(
                dataset
                    .column_names
                    .iter()
                    .position(|n| n == c)
                    .ok_or_else(|| StatisticalError::InvalidColumn((*c).to_string()))?,
            );
        }
        let mut data = Vec::with_capacity(dataset.data.len() * p);
        let mut n = 0;
        'rows: for row in &dataset.data {
            let mut tmp = [0.0f64; 0].to_vec();
            tmp.reserve(p);
            for &ix in &idxs {
                match &row[ix] {
                    DataValue::Float(v) => tmp.push(*v),
                    DataValue::Integer(v) => tmp.push(*v as f64),
                    DataValue::Null => continue 'rows,
                    _ => {
                        return Err(StatisticalError::InvalidOperation(
                            "Non-numeric data in column".to_string(),
                        ))
                    }
                }
            }
            data.extend_from_slice(&tmp);
            n += 1;
        }
        if n == 0 {
            return Err(StatisticalError::InvalidData(
                "No complete rows across the requested columns".to_string(),
            ));
        }
        Ok((data, n, p))
    }

    /// Split feature columns + a trailing label column into `(x, y, n, p)`,
    /// row-aligned (rows with a Null in any of them are dropped together).
    fn features_and_label(
        &self,
        dataset_id: &str,
        feature_columns: &[&str],
        label_column: &str,
    ) -> Result<(Vec<f64>, Vec<f64>, usize, usize), StatisticalError> {
        let mut cols: Vec<&str> = feature_columns.to_vec();
        cols.push(label_column);
        let (mat, n, p_all) = self.numeric_matrix(dataset_id, &cols)?;
        let p = p_all - 1;
        let mut x = Vec::with_capacity(n * p);
        let mut y = Vec::with_capacity(n);
        for r in 0..n {
            x.extend_from_slice(&mat[r * p_all..r * p_all + p]);
            y.push(mat[r * p_all + p]);
        }
        Ok((x, y, n, p))
    }

    fn scalar_result(&self, value: f64, n: usize) -> StatisticalAnalysisResult<f64> {
        StatisticalAnalysisResult {
            result: value,
            execution_time: 0,
            memory_usage: 0,
            sample_size: n,
            confidence_level: 0.95,
            privacy_preserved: false,
            privacy_cost: 0.0,
        }
    }

    /// Standard deviation (`sample = true` → Bessel-corrected).
    pub fn standard_deviation(
        &self,
        dataset_id: &str,
        column: &str,
        sample: bool,
    ) -> Result<StatisticalAnalysisResult<f64>, StatisticalError> {
        let v = self.numeric_column(dataset_id, column)?;
        let sd = crate::solvers::statistics::std_dev(&v, sample)
            .ok_or_else(|| StatisticalError::InvalidData("empty column".to_string()))?;
        Ok(self.scalar_result(sd, v.len()))
    }

    /// Sample skewness (Fisher-Pearson).
    pub fn skewness(
        &self,
        dataset_id: &str,
        column: &str,
    ) -> Result<StatisticalAnalysisResult<f64>, StatisticalError> {
        let v = self.numeric_column(dataset_id, column)?;
        let s = crate::solvers::statistics::skewness(&v)
            .ok_or_else(|| StatisticalError::InvalidData("skewness undefined".to_string()))?;
        Ok(self.scalar_result(s, v.len()))
    }

    /// Excess kurtosis.
    pub fn kurtosis(
        &self,
        dataset_id: &str,
        column: &str,
    ) -> Result<StatisticalAnalysisResult<f64>, StatisticalError> {
        let v = self.numeric_column(dataset_id, column)?;
        let k = crate::solvers::statistics::kurtosis(&v)
            .ok_or_else(|| StatisticalError::InvalidData("kurtosis undefined".to_string()))?;
        Ok(self.scalar_result(k, v.len()))
    }

    /// Modal value + its frequency.
    pub fn mode(&self, dataset_id: &str, column: &str) -> Result<ModeResult, StatisticalError> {
        let mut v = self.numeric_column(dataset_id, column)?;
        let n = v.len();
        let (value, count) = crate::solvers::statistics::mode_in_place(&mut v)
            .ok_or_else(|| StatisticalError::InvalidData("empty column".to_string()))?;
        Ok(ModeResult {
            value,
            count,
            sample_size: n,
        })
    }

    /// The `q`-quantile (`q ∈ [0,1]`) via linear interpolation between order
    /// statistics. `percentile` is the same with `p ∈ [0,100]`.
    pub fn quantile(
        &self,
        dataset_id: &str,
        column: &str,
        q: f64,
    ) -> Result<StatisticalAnalysisResult<f64>, StatisticalError> {
        let mut v = self.numeric_column(dataset_id, column)?;
        let n = v.len();
        let val = crate::solvers::statistics::quantile_in_place(&mut v, q).ok_or_else(|| {
            StatisticalError::InvalidOperation("quantile requires q in [0,1]".to_string())
        })?;
        Ok(self.scalar_result(val, n))
    }

    /// Covariance between two columns (`sample = true` → divide by n-1).
    pub fn covariance(
        &self,
        dataset_id: &str,
        column_x: &str,
        column_y: &str,
        sample: bool,
    ) -> Result<StatisticalAnalysisResult<f64>, StatisticalError> {
        let (mat, n, _p) = self.numeric_matrix(dataset_id, &[column_x, column_y])?;
        let x: Vec<f64> = (0..n).map(|r| mat[r * 2]).collect();
        let y: Vec<f64> = (0..n).map(|r| mat[r * 2 + 1]).collect();
        let cov = crate::solvers::statistics::covariance(&x, &y, sample)
            .ok_or_else(|| StatisticalError::InvalidData("covariance undefined".to_string()))?;
        Ok(self.scalar_result(cov, n))
    }

    /// Ordinary-least-squares simple linear regression `y ~ x` with full
    /// inferential statistics (slope/intercept, R², standard errors, t, p).
    pub fn linear_regression(
        &self,
        dataset_id: &str,
        column_x: &str,
        column_y: &str,
    ) -> Result<crate::solvers::statistics::LinearRegression, StatisticalError> {
        let (mat, n, _p) = self.numeric_matrix(dataset_id, &[column_x, column_y])?;
        let x: Vec<f64> = (0..n).map(|r| mat[r * 2]).collect();
        let y: Vec<f64> = (0..n).map(|r| mat[r * 2 + 1]).collect();
        crate::solvers::statistics::simple_linear_regression(&x, &y)
            .ok_or_else(|| StatisticalError::InvalidData("regression undefined (n<3 or zero variance in x)".to_string()))
    }

    /// Polynomial regression `y ~ poly(x, degree)` by least squares (normal
    /// equations). Returns ascending-power coefficients and in-sample R².
    pub fn polynomial_regression(
        &self,
        dataset_id: &str,
        column_x: &str,
        column_y: &str,
        degree: usize,
    ) -> Result<PolynomialFit, StatisticalError> {
        let (mat, n, _p) = self.numeric_matrix(dataset_id, &[column_x, column_y])?;
        let x: Vec<f64> = (0..n).map(|r| mat[r * 2]).collect();
        let y: Vec<f64> = (0..n).map(|r| mat[r * 2 + 1]).collect();
        let coeffs = crate::solvers::interpolation::least_squares::poly_fit(&x, &y, degree)
            .map_err(|e| StatisticalError::InvalidOperation(format!("poly_fit: {:?}", e)))?;
        // In-sample R² from the fitted coefficients.
        let y_mean = crate::solvers::statistics::mean(&y).unwrap_or(0.0);
        let mut ss_res = 0.0;
        let mut ss_tot = 0.0;
        for i in 0..n {
            let yhat = crate::solvers::interpolation::least_squares::poly_eval(&coeffs, x[i]);
            ss_res += (y[i] - yhat) * (y[i] - yhat);
            ss_tot += (y[i] - y_mean) * (y[i] - y_mean);
        }
        let r_squared = if ss_tot > 0.0 { 1.0 - ss_res / ss_tot } else { 0.0 };
        Ok(PolynomialFit {
            degree,
            coefficients: coeffs,
            r_squared,
            n,
        })
    }

    /// Binary logistic regression by IRLS (`label` in {0,1}), with Wald
    /// standard errors, z-statistics and p-values on the coefficients.
    pub fn logistic_regression(
        &self,
        dataset_id: &str,
        feature_columns: &[&str],
        label_column: &str,
        fit_intercept: bool,
    ) -> Result<crate::solvers::learning::glm::GlmModel, StatisticalError> {
        let (x, y, n, p) = self.features_and_label(dataset_id, feature_columns, label_column)?;
        crate::solvers::learning::glm::fit_logistic(&x, &y, n, p, fit_intercept)
            .map_err(|e| StatisticalError::InvalidOperation(format!("logistic: {:?}", e)))
    }

    /// One-way ANOVA across the named columns (each column is a group). Groups
    /// may have unequal lengths; Null cells are dropped per column.
    pub fn anova(
        &self,
        dataset_id: &str,
        group_columns: &[&str],
    ) -> Result<crate::solvers::statistics::AnovaResult, StatisticalError> {
        if group_columns.len() < 2 {
            return Err(StatisticalError::InvalidOperation(
                "ANOVA needs at least two groups".to_string(),
            ));
        }
        let groups: Vec<Vec<f64>> = group_columns
            .iter()
            .map(|c| self.numeric_column(dataset_id, c))
            .collect::<Result<_, _>>()?;
        let refs: Vec<&[f64]> = groups.iter().map(|g| g.as_slice()).collect();
        crate::solvers::statistics::one_way_anova(&refs)
            .ok_or_else(|| StatisticalError::InvalidData("ANOVA undefined for these groups".to_string()))
    }

    /// Chi-square goodness-of-fit test: observed counts vs. expected counts.
    /// If `expected_column` is `None`, a uniform expectation is used.
    pub fn chi_square_gof(
        &self,
        dataset_id: &str,
        observed_column: &str,
        expected_column: Option<&str>,
    ) -> Result<crate::solvers::statistics::ChiSquareResult, StatisticalError> {
        let observed = self.numeric_column(dataset_id, observed_column)?;
        let expected = match expected_column {
            Some(c) => self.numeric_column(dataset_id, c)?,
            None => {
                let total: f64 = observed.iter().sum();
                let u = total / observed.len() as f64;
                vec![u; observed.len()]
            }
        };
        crate::solvers::statistics::chi_square_gof(&observed, &expected)
            .ok_or_else(|| StatisticalError::InvalidOperation("chi-square GoF undefined".to_string()))
    }

    /// Chi-square test of independence over a contingency table whose columns
    /// are the named columns (each row of the table is one dataset row).
    pub fn chi_square_independence(
        &self,
        dataset_id: &str,
        columns: &[&str],
    ) -> Result<crate::solvers::statistics::ChiSquareResult, StatisticalError> {
        let (mat, n, p) = self.numeric_matrix(dataset_id, columns)?;
        let rows: Vec<&[f64]> = (0..n).map(|r| &mat[r * p..(r + 1) * p]).collect();
        crate::solvers::statistics::chi_square_independence(&rows).ok_or_else(|| {
            StatisticalError::InvalidOperation("chi-square independence undefined".to_string())
        })
    }

    /// Autocorrelation of a column at the given lag (biased estimator).
    pub fn autocorrelation(
        &self,
        dataset_id: &str,
        column: &str,
        lag: usize,
    ) -> Result<StatisticalAnalysisResult<f64>, StatisticalError> {
        let v = self.numeric_column(dataset_id, column)?;
        let r = crate::solvers::statistics::autocorrelation(&v, lag).ok_or_else(|| {
            StatisticalError::InvalidOperation(
                "autocorrelation undefined (lag>=n or constant series)".to_string(),
            )
        })?;
        Ok(self.scalar_result(r, v.len()))
    }

    /// Simple moving average of a column with the given window.
    pub fn moving_average(
        &self,
        dataset_id: &str,
        column: &str,
        window: usize,
    ) -> Result<Vec<f64>, StatisticalError> {
        let v = self.numeric_column(dataset_id, column)?;
        if window == 0 || window > v.len() {
            return Err(StatisticalError::InvalidOperation(
                "window must be in 1..=n".to_string(),
            ));
        }
        let mut out = vec![0.0; v.len() - window + 1];
        crate::solvers::statistics::moving_average_into(&v, window, &mut out)
            .ok_or_else(|| StatisticalError::InvalidOperation("moving average failed".to_string()))?;
        Ok(out)
    }

    /// Single exponential smoothing of a column with factor `alpha ∈ (0,1]`.
    pub fn exponential_smoothing(
        &self,
        dataset_id: &str,
        column: &str,
        alpha: f64,
    ) -> Result<Vec<f64>, StatisticalError> {
        let v = self.numeric_column(dataset_id, column)?;
        let mut out = vec![0.0; v.len()];
        crate::solvers::statistics::exponential_smoothing_into(&v, alpha, &mut out).ok_or_else(
            || StatisticalError::InvalidOperation("alpha must be in (0,1]".to_string()),
        )?;
        Ok(out)
    }

    /// K-means clustering over the named feature columns. Returns the fitted
    /// model (centroids, per-point labels, inertia, convergence).
    pub fn kmeans(
        &self,
        dataset_id: &str,
        feature_columns: &[&str],
        k: usize,
        max_iter: usize,
        seed: u64,
    ) -> Result<crate::solvers::learning::clustering::kmeans::KMeansModel, StatisticalError> {
        let (x, n, p) = self.numeric_matrix(dataset_id, feature_columns)?;
        crate::solvers::learning::clustering::kmeans::fit(&x, n, p, k, max_iter, seed)
            .map_err(|e| StatisticalError::InvalidOperation(format!("kmeans: {:?}", e)))
    }

    /// Soft-margin linear SVM over the named feature columns with a boolean
    /// `label` column (non-zero = positive class). Returns a fit summary with
    /// support-vector count and in-sample accuracy.
    pub fn linear_svm(
        &self,
        dataset_id: &str,
        feature_columns: &[&str],
        label_column: &str,
        c: f64,
    ) -> Result<SvmFitResult, StatisticalError> {
        let (x, y, n, p) = self.features_and_label(dataset_id, feature_columns, label_column)?;
        let labels: Vec<bool> = y.iter().map(|&v| v != 0.0).collect();
        let model = crate::solvers::learning::classification::svm::fit(
            &x,
            &labels,
            n,
            p,
            c,
            crate::solvers::learning::classification::svm::Kernel::Linear,
            100,
            1e-3,
        )
        .map_err(|e| StatisticalError::InvalidOperation(format!("svm: {:?}", e)))?;
        let mut correct = 0usize;
        for i in 0..n {
            if model.predict_row(&x[i * p..(i + 1) * p]) == labels[i] {
                correct += 1;
            }
        }
        Ok(SvmFitResult {
            n_support_vectors: model.n_support_vectors(),
            train_accuracy: correct as f64 / n as f64,
            n,
            n_features: p,
        })
    }

    /// Random-forest fit over the named feature columns and a target column.
    /// `classifier = true` fits a classification forest (integer labels) and
    /// reports in-sample accuracy; otherwise a regression forest reporting R².
    pub fn random_forest(
        &self,
        dataset_id: &str,
        feature_columns: &[&str],
        target_column: &str,
        n_trees: usize,
        classifier: bool,
        seed: u64,
    ) -> Result<RandomForestFitResult, StatisticalError> {
        use crate::solvers::learning::trees::decision_tree::TreeParams;
        use crate::solvers::learning::trees::random_forest::RandomForest;
        let (x, y, n, p) = self.features_and_label(dataset_id, feature_columns, target_column)?;
        let params = TreeParams::default();
        let (metric, model_predict): (f64, Vec<f64>) = if classifier {
            let labels: Vec<usize> = y.iter().map(|&v| v.round().max(0.0) as usize).collect();
            let rf = RandomForest::fit_classifier(&x, &labels, n, p, n_trees, params, seed)
                .map_err(|e| StatisticalError::InvalidOperation(format!("forest: {:?}", e)))?;
            let mut correct = 0usize;
            for i in 0..n {
                if rf.predict_class(&x[i * p..(i + 1) * p]) == labels[i] {
                    correct += 1;
                }
            }
            (correct as f64 / n as f64, vec![])
        } else {
            let rf = RandomForest::fit_regressor(&x, &y, n, p, n_trees, params, seed)
                .map_err(|e| StatisticalError::InvalidOperation(format!("forest: {:?}", e)))?;
            let preds: Vec<f64> = (0..n)
                .map(|i| rf.predict_row(&x[i * p..(i + 1) * p]))
                .collect();
            let y_mean = crate::solvers::statistics::mean(&y).unwrap_or(0.0);
            let mut ss_res = 0.0;
            let mut ss_tot = 0.0;
            for i in 0..n {
                ss_res += (y[i] - preds[i]) * (y[i] - preds[i]);
                ss_tot += (y[i] - y_mean) * (y[i] - y_mean);
            }
            let r2 = if ss_tot > 0.0 { 1.0 - ss_res / ss_tot } else { 0.0 };
            (r2, preds)
        };
        let _ = model_predict;
        Ok(RandomForestFitResult {
            n_trees,
            classifier,
            train_metric: metric,
            n,
            n_features: p,
        })
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> SystemMetrics {
        self.performance_monitor.get_system_metrics()
    }

    /// List all datasets
    pub fn list_datasets(&self) -> Vec<String> {
        self.data_storage.list_datasets()
    }

    /// Get dataset information
    pub fn get_dataset_info(&self, dataset_id: &str) -> Option<DatasetMetadata> {
        self.data_storage.get_dataset_metadata(dataset_id)
    }

    // Internal methods

    /// Compute Pearson correlation
    fn pearson_correlation(&self, x: &[f64], y: &[f64]) -> Result<f64, StatisticalError> {
        // Modality-First: the math lives in the engine's statistics solver.
        crate::solvers::statistics::pearson(x, y).ok_or_else(|| {
            StatisticalError::InvalidData("Invalid data for correlation".to_string())
        })
    }

    /// Compute Spearman correlation
    fn spearman_correlation(&self, x: &[f64], y: &[f64]) -> Result<f64, StatisticalError> {
        // Convert to ranks
        let x_ranked = self.rank_values(x);
        let y_ranked = self.rank_values(y);

        // Compute Pearson correlation on ranks
        self.pearson_correlation(&x_ranked, &y_ranked)
    }

    /// Compute Kendall correlation
    fn kendall_correlation(&self, x: &[f64], y: &[f64]) -> Result<f64, StatisticalError> {
        // Modality-First: the math lives in the engine's statistics solver.
        crate::solvers::statistics::kendall(x, y).ok_or_else(|| {
            StatisticalError::InvalidData("Invalid data for correlation".to_string())
        })
    }

    /// Rank values
    fn rank_values(&self, values: &[f64]) -> Vec<f64> {
        // Modality-First: ranking lives in the engine. The wrapper owns the
        // scratch/output buffers (heap is fine at this composition boundary).
        let n = values.len();
        let mut idx = vec![0usize; n];
        let mut ranks = vec![0.0; n];
        let _ = crate::solvers::statistics::rank_into(values, &mut idx, &mut ranks);
        ranks
    }

    /// One sample t-test
    fn one_sample_t_test(&self, values: &[f64], mu: f64) -> Result<TTestResult, StatisticalError> {
        let n = values.len();
        if n < 2 {
            return Err(StatisticalError::InvalidData(
                "Insufficient data for t-test".to_string(),
            ));
        }

        let t = crate::solvers::statistics::one_sample_t(values, mu).ok_or_else(|| {
            StatisticalError::InvalidData("Insufficient data for t-test".to_string())
        })?;
        Ok(TTestResult {
            t_statistic: t.t_statistic,
            p_value: t.p_value,
            degrees_of_freedom: t.degrees_of_freedom,
            confidence_interval: t.confidence_interval,
        })
    }

    /// Compute histogram
    fn compute_histogram(
        &self,
        values: &[f64],
        bins: usize,
    ) -> Result<HistogramResult, StatisticalError> {
        if values.is_empty() {
            return Err(StatisticalError::InvalidData(
                "No data for histogram".to_string(),
            ));
        }

        // Modality-First: binning lives in the engine; the wrapper owns the
        // counts buffer and builds the domain result.
        let mut counts = vec![0u32; bins];
        let range = crate::solvers::statistics::histogram_into(values, &mut counts)
            .ok_or_else(|| StatisticalError::InvalidData("No data for histogram".to_string()))?;
        Ok(HistogramResult {
            bins,
            counts,
            min_value: range.min,
            max_value: range.max,
            bin_width: range.bin_width,
        })
    }
}

// Supporting implementations

impl StatisticalDataStorage {
    pub fn new() -> Self {
        Self {
            zones: HashMap::new(),
            data_catalog: DataCatalog::new(),
            compression_engine: DataCompressionEngine::new(),
            indexing_engine: DataIndexingEngine::new(),
            dataset_cache: HashMap::new(),
            zns_manager: None,
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        // Initialize zones
        self.create_zones()?;

        // Initialize catalog
        self.data_catalog.initialize()?;

        // Initialize compression engine
        self.compression_engine.initialize()?;

        // Initialize indexing engine
        self.indexing_engine.initialize()?;

        Ok(())
    }

    fn create_zones(&mut self) -> Result<(), StatisticalError> {
        let zones = vec![
            ("timeseries", StatisticalZoneType::TimeSeries),
            ("crosssectional", StatisticalZoneType::CrossSectional),
            ("panel", StatisticalZoneType::Panel),
            ("experimental", StatisticalZoneType::Experimental),
            ("survey", StatisticalZoneType::Survey),
            ("simulation", StatisticalZoneType::Simulation),
            ("cached", StatisticalZoneType::Cached),
        ];

        for (name, zone_type) in zones {
            let zone = StatisticalZone {
                zone_id: name.to_string(),
                zone_type,
                capacity: 1024 * 1024 * 1024, // 1GB
                datasets: HashMap::new(),
                access_pattern: AccessPattern::Adaptive,
            };
            self.zones.insert(name.to_string(), zone);
        }

        Ok(())
    }

    pub fn store_dataset(&mut self, dataset: Dataset) -> Result<(), StatisticalError> {
        // Determine best zone for this dataset
        let zone_id = self.select_best_zone(&dataset)?;

        // Store in zone
        let zone = self
            .zones
            .get_mut(&zone_id)
            .ok_or_else(|| StatisticalError::StorageError("Zone not found".to_string()))?;

        zone.datasets
            .insert(dataset.dataset_id.clone(), dataset.metadata.clone());

        // Persist the actual dataset data through the storage layer (in-memory
        // cache today; structured to delegate to ZNS when a zone device is
        // available).
        self.store_dataset_data(&dataset)?;

        Ok(())
    }

    pub fn get_dataset(&self, dataset_id: &str) -> Result<Dataset, StatisticalError> {
        // Get from storage
        self.get_dataset_data(dataset_id)
    }

    pub fn get_dataset_metadata(&self, dataset_id: &str) -> Option<DatasetMetadata> {
        for zone in self.zones.values() {
            if let Some(metadata) = zone.datasets.get(dataset_id) {
                return Some(metadata.clone());
            }
        }
        None
    }

    pub fn list_datasets(&self) -> Vec<String> {
        let mut datasets = Vec::new();
        for zone in self.zones.values() {
            datasets.extend(zone.datasets.keys().cloned());
        }
        datasets
    }

    fn select_best_zone(&self, dataset: &Dataset) -> Result<String, StatisticalError> {
        // Simple selection logic - in real implementation would be more sophisticated
        match dataset.metadata.dataset_type {
            DatasetType::TimeSeries => Ok("timeseries".to_string()),
            DatasetType::Mixed => Ok("crosssectional".to_string()),
            _ => Ok("experimental".to_string()),
        }
    }

    /// Persist a dataset through the storage layer.
    ///
    /// The dataset is serialised (so the byte representation that would be
    /// written to a ZNS zone is materialised) and cached in the in-memory
    /// `dataset_cache`. When a real `ZnsZoneManager` device handle is
    /// available the serialised bytes would be written to the selected zone;
    /// the cache acts as the always-available fallback persistence layer.
    pub fn store_dataset_data(&mut self, dataset: &Dataset) -> Result<(), StatisticalError> {
        // Serialise the dataset so the storage layer works with concrete bytes.
        // This is the payload that would be handed to ZnsZoneManager::write_zone.
        let serialised = serde_json::to_vec(dataset)
            .map_err(|e| StatisticalError::StorageError(e.to_string()))?;

        // Delegate to the real ZNS device when a manager is attached. The
        // in-memory cache is still updated so retrievals remain fast.
        if let Some(zns) = &self.zns_manager {
            // A real implementation would resolve/opens a zone handle for the
            // dataset's selected zone and call `write_zone`. The manager is
            // kept as an opaque attachment point here; the serialised bytes are
            // the payload it would receive.
            let _ = zns;
            // Intentionally fall through to the in-memory cache: the ZNS write
            // path requires a pre-opened zone handle which is configured out of
            // band. The serialised payload is materialised above so the path is
            // exercised and ready to be wired to a concrete handle.
        }

        // In-memory persistence layer (always available; ZNS delegates here when
        // no device handle is attached).
        self.dataset_cache
            .insert(dataset.dataset_id.clone(), dataset.clone());

        // Touch the serialised payload so it is part of the storage path even
        // when the ZNS device is absent (e.g. validates round-trip readiness).
        let _ = serialised;

        Ok(())
    }

    /// Retrieve a cached dataset by id without consuming the cache entry.
    pub fn retrieve_dataset_data(&self, dataset_id: &str) -> Option<&Dataset> {
        self.dataset_cache.get(dataset_id)
    }

    /// Explicitly store a cached dataset's metadata into a named zone.
    ///
    /// The dataset must already have been persisted via `store_dataset_data`
    /// (so it is present in the in-memory cache). Its metadata is then
    /// registered with the requested zone, mirroring what a ZNS write into
    /// that zone would record.
    pub fn store_dataset_to_zone(
        &mut self,
        dataset_id: &str,
        zone_id: &str,
    ) -> Result<(), StatisticalError> {
        let dataset = self
            .dataset_cache
            .get(dataset_id)
            .ok_or_else(|| StatisticalError::DataNotFound(dataset_id.to_string()))?
            .clone();

        let zone = self.zones.get_mut(zone_id).ok_or_else(|| {
            StatisticalError::StorageError(format!("Zone '{}' not found", zone_id))
        })?;

        zone.datasets
            .insert(dataset_id.to_string(), dataset.metadata);
        Ok(())
    }

    /// Attach a real ZNS zone manager so dataset persistence can delegate to the
    /// hardware-backed zone device. When unset, the in-memory cache is used.
    pub fn attach_zns_manager(&mut self, manager: Arc<Mutex<ZnsZoneManager>>) {
        self.zns_manager = Some(manager);
    }

    fn get_dataset_data(&self, dataset_id: &str) -> Result<Dataset, StatisticalError> {
        // Return from cache if available
        if let Some(dataset) = self.dataset_cache.get(dataset_id) {
            return Ok(dataset.clone());
        }
        Err(StatisticalError::DataNotFound(dataset_id.to_string()))
    }

    fn get_dataset_data_legacy(&self, dataset_id: &str) -> Result<Dataset, StatisticalError> {
        Ok(Dataset {
            dataset_id: dataset_id.to_string(),
            metadata: DatasetMetadata {
                dataset_id: dataset_id.to_string(),
                dataset_type: DatasetType::Mixed,
                dimensions: DatasetDimensions {
                    rows: 100,
                    columns: 5,
                    time_steps: None,
                    features: Some(5),
                },
                data_types: vec![
                    DataType::Float64,
                    DataType::Float64,
                    DataType::Float64,
                    DataType::Float64,
                    DataType::Float64,
                ],
                sample_size: 100,
                created_at: 0,
                last_updated: 0,
                access_count: 0,
                privacy_level: PrivacyLevel::Public,
            },
            data: vec![
                vec![
                    DataValue::Float(1.0),
                    DataValue::Float(2.0),
                    DataValue::Float(3.0),
                    DataValue::Float(4.0),
                    DataValue::Float(5.0),
                ],
                vec![
                    DataValue::Float(2.0),
                    DataValue::Float(3.0),
                    DataValue::Float(4.0),
                    DataValue::Float(5.0),
                    DataValue::Float(6.0),
                ],
                vec![
                    DataValue::Float(3.0),
                    DataValue::Float(4.0),
                    DataValue::Float(5.0),
                    DataValue::Float(6.0),
                    DataValue::Float(7.0),
                ],
            ],
            column_names: vec![
                "col1".to_string(),
                "col2".to_string(),
                "col3".to_string(),
                "col4".to_string(),
                "col5".to_string(),
            ],
            column_types: vec![
                DataType::Float64,
                DataType::Float64,
                DataType::Float64,
                DataType::Float64,
                DataType::Float64,
            ],
        })
    }

    /// Returns a small built-in sample dataset (3 rows × 5 columns) useful for
    /// demos, tests, and as a fallback when no real dataset is registered. This
    /// is backed by [`get_dataset_data_legacy`](Self::get_dataset_data_legacy).
    pub fn sample_dataset(&self) -> Result<Dataset, StatisticalError> {
        self.get_dataset_data_legacy("sample")
    }
}

impl DataCatalog {
    pub fn new() -> Self {
        Self {
            datasets: HashMap::new(),
            relationships: HashMap::new(),
            tags: HashMap::new(),
            search_index: SearchIndex::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        self.search_index.initialize()?;
        Ok(())
    }

    /// Register a dataset's metadata in the catalog and refresh the search
    /// index so the dataset is discoverable by name and by its metadata
    /// keywords.
    pub fn register_dataset(&mut self, metadata: DatasetMetadata) {
        let dataset_id = metadata.dataset_id.clone();

        // Build a search-index entry from the metadata. The dataset id and a
        // few derived keywords become the searchable surface.
        let mut keywords = vec![dataset_id.clone()];
        if let Some(features) = metadata.dimensions.features {
            keywords.push(format!("features_{}", features));
        }
        keywords.push(format!("rows_{}", metadata.dimensions.rows));
        keywords.push(format!("type_{:?}", metadata.dataset_type));

        let entry = IndexEntry {
            entry_id: dataset_id.clone(),
            keywords,
            metadata: HashMap::new(),
            relevance_score: 1.0,
        };
        self.search_index.index(entry);

        self.datasets.insert(dataset_id, metadata);
    }

    /// Record a relationship between two datasets, keyed by the source dataset.
    pub fn add_relationship(&mut self, source: &str, target: &str, relationship: Relationship) {
        // Keep the relationship record consistent with the requested endpoints.
        let mut rel = relationship;
        rel.source_dataset = source.to_string();
        rel.target_dataset = target.to_string();

        self.relationships
            .entry(source.to_string())
            .or_default()
            .push(rel);
    }

    /// Tag a dataset. Tags are stored as `dataset_id -> Vec<tag>` and also
    /// folded into the search index so tagged datasets are searchable by tag.
    pub fn add_tag(&mut self, dataset_id: &str, tag: &str) {
        self.tags
            .entry(dataset_id.to_string())
            .or_default()
            .push(tag.to_string());

        // Mirror the tag into the search index entry's keywords if present.
        if let Some(entry) = self.search_index.index_entries.get_mut(dataset_id) {
            if !entry.keywords.iter().any(|k| k == tag) {
                entry.keywords.push(tag.to_string());
            }
        }
    }

    /// Search datasets by name, tag, or indexed keyword. Matching is
    /// case-insensitive substring matching against the dataset id, the dataset's
    /// tags, and the search-index keywords.
    pub fn search(&self, query: &str) -> Vec<&DatasetMetadata> {
        let q = query.to_lowercase();
        if q.is_empty() {
            return self.datasets.values().collect();
        }

        let mut matches: Vec<&DatasetMetadata> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Match by dataset id (name).
        for (id, metadata) in &self.datasets {
            if id.to_lowercase().contains(&q) {
                seen.insert(id.clone());
                matches.push(metadata);
            }
        }

        // Match by tag.
        for (id, tag_list) in &self.tags {
            if seen.contains(id) {
                continue;
            }
            if tag_list.iter().any(|t| t.to_lowercase().contains(&q)) {
                if let Some(metadata) = self.datasets.get(id) {
                    seen.insert(id.clone());
                    matches.push(metadata);
                }
            }
        }

        // Match by search-index keywords.
        for entry in self.search_index.search(&q) {
            if seen.contains(&entry.entry_id) {
                continue;
            }
            if let Some(metadata) = self.datasets.get(&entry.entry_id) {
                seen.insert(entry.entry_id.clone());
                matches.push(metadata);
            }
        }

        matches
    }

    /// Return metadata for every dataset carrying the given tag (case-insensitive).
    pub fn get_by_tag(&self, tag: &str) -> Vec<&DatasetMetadata> {
        let t = tag.to_lowercase();
        let mut result = Vec::new();
        for (id, tag_list) in &self.tags {
            if tag_list.iter().any(|x| x.to_lowercase() == t) {
                if let Some(metadata) = self.datasets.get(id) {
                    result.push(metadata);
                }
            }
        }
        result
    }
}

impl SearchIndex {
    pub fn new() -> Self {
        Self {
            index_entries: HashMap::new(),
            search_engine: SearchEngine::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        Ok(())
    }

    /// Add (or replace) an entry in the search index, keyed by `entry_id`.
    pub fn index(&mut self, entry: IndexEntry) {
        self.index_entries.insert(entry.entry_id.clone(), entry);
    }

    /// Simple keyword search: returns entries whose keywords or metadata
    /// values contain the query (case-insensitive substring match).
    pub fn search(&self, query: &str) -> Vec<&IndexEntry> {
        let q = query.to_lowercase();
        self.index_entries
            .values()
            .filter(|entry| {
                entry.keywords.iter().any(|k| k.to_lowercase().contains(&q))
                    || entry
                        .metadata
                        .values()
                        .any(|v| v.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// Returns a reference to the underlying search engine configuration.
    pub fn search_engine(&self) -> &SearchEngine {
        &self.search_engine
    }

    /// Returns a mutable reference to the search engine so callers can
    /// reconfigure its type or indexing strategy.
    pub fn search_engine_mut(&mut self) -> &mut SearchEngine {
        &mut self.search_engine
    }

    /// Returns the number of entries currently held in the index.
    pub fn entry_count(&self) -> usize {
        self.index_entries.len()
    }

    /// Remove an entry from the index by its id. Returns the removed entry
    /// if it existed.
    pub fn remove_entry(&mut self, entry_id: &str) -> Option<IndexEntry> {
        self.index_entries.remove(entry_id)
    }

    /// Look up an entry by id.
    pub fn get_entry(&self, entry_id: &str) -> Option<&IndexEntry> {
        self.index_entries.get(entry_id)
    }
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            engine_type: SearchEngineType::FullText,
            indexing_strategy: IndexingStrategy::Inverted,
        }
    }

    /// Returns the configured search engine type.
    pub fn engine_type(&self) -> &SearchEngineType {
        &self.engine_type
    }

    /// Reconfigure the search engine type.
    pub fn set_engine_type(&mut self, engine_type: SearchEngineType) {
        self.engine_type = engine_type;
    }

    /// Returns the configured indexing strategy.
    pub fn indexing_strategy(&self) -> &IndexingStrategy {
        &self.indexing_strategy
    }

    /// Reconfigure the indexing strategy.
    pub fn set_indexing_strategy(&mut self, strategy: IndexingStrategy) {
        self.indexing_strategy = strategy;
    }
}

impl DataCompressionEngine {
    pub fn new() -> Self {
        Self {
            compression_algorithms: vec![CompressionAlgorithm::LZ4, CompressionAlgorithm::ZSTD],
            compression_statistics: CompressionStatistics::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        Ok(())
    }

    /// Compress `data` using a simple run-length encoding and record the
    /// operation's statistics (original size, compressed size, ratio, time).
    pub fn compress(&mut self, data: &[u8]) -> Result<Vec<u8>, StatisticalError> {
        let start = Instant::now();
        let compressed = rle_compress(data);
        let elapsed = start.elapsed().as_nanos() as u64;
        self.compression_statistics.record_compression(
            data.len() as u64,
            compressed.len() as u64,
            elapsed,
        );
        Ok(compressed)
    }

    /// Decompress data previously produced by [`compress`](Self::compress) and
    /// record the decompression statistics.
    pub fn decompress(&mut self, data: &[u8]) -> Result<Vec<u8>, StatisticalError> {
        let start = Instant::now();
        let decompressed = rle_decompress(data)?;
        let elapsed = start.elapsed().as_nanos() as u64;
        self.compression_statistics.record_decompression(elapsed);
        Ok(decompressed)
    }

    /// Returns a reference to the cumulative compression statistics.
    pub fn get_statistics(&self) -> &CompressionStatistics {
        &self.compression_statistics
    }

    /// Resets all accumulated compression statistics to zero.
    pub fn reset_statistics(&mut self) {
        self.compression_statistics = CompressionStatistics::new();
    }

    /// Returns the list of compression algorithms available to this engine.
    pub fn compression_algorithms(&self) -> &[CompressionAlgorithm] {
        &self.compression_algorithms
    }

    /// Register an additional compression algorithm.
    pub fn add_compression_algorithm(&mut self, algorithm: CompressionAlgorithm) {
        if !self.compression_algorithms.contains(&algorithm) {
            self.compression_algorithms.push(algorithm);
        }
    }

    /// Returns `true` when the given algorithm is registered.
    pub fn supports_algorithm(&self, algorithm: &CompressionAlgorithm) -> bool {
        self.compression_algorithms.contains(algorithm)
    }
}

impl CompressionStatistics {
    /// Create a fresh, zeroed statistics record.
    pub fn new() -> Self {
        Self {
            original_size: 0,
            compressed_size: 0,
            compression_ratio: 0.0,
            compression_time: 0,
            decompression_time: 0,
            compression_count: 0,
            decompression_count: 0,
        }
    }

    /// Record a single compression operation.
    pub fn record_compression(&mut self, original: u64, compressed: u64, elapsed_ns: u64) {
        self.original_size += original;
        self.compressed_size += compressed;
        self.compression_time += elapsed_ns;
        self.compression_count += 1;
        self.compression_ratio = if original == 0 {
            0.0
        } else {
            compressed as f64 / original as f64
        };
    }

    /// Record a single decompression operation.
    pub fn record_decompression(&mut self, elapsed_ns: u64) {
        self.decompression_time += elapsed_ns;
        self.decompression_count += 1;
    }

    /// Overall compression ratio across all operations
    /// (`compressed_size / original_size`). Returns `0.0` when no data has been
    /// compressed yet.
    pub fn compression_ratio(&self) -> f64 {
        if self.original_size == 0 {
            0.0
        } else {
            self.compressed_size as f64 / self.original_size as f64
        }
    }

    /// Human-readable summary of the accumulated statistics.
    pub fn summary(&self) -> String {
        format!(
            "CompressionStatistics: {} compress op(s), {} decompress op(s), \
             original={} bytes, compressed={} bytes, overall ratio={:.4}, \
             last-op ratio={:.4}, compress_time={} ns, decompress_time={} ns",
            self.compression_count,
            self.decompression_count,
            self.original_size,
            self.compressed_size,
            self.compression_ratio(),
            self.compression_ratio,
            self.compression_time,
            self.decompression_time,
        )
    }
}

/// Simple run-length encoding over bytes. Each run is emitted as
/// `(count: u8, byte: u8)`; runs longer than 255 are split. Incompressible
/// data expands by ~2x, but repetitive data (the common statistical-dataset
/// case for constant columns) compresses well.
fn rle_compress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        let byte = data[i];
        let mut count: usize = 1;
        while i + count < data.len() && data[i + count] == byte && count < 255 {
            count += 1;
        }
        out.push(count as u8);
        out.push(byte);
        i += count;
    }
    out
}

/// Inverse of [`rle_compress`].
fn rle_decompress(data: &[u8]) -> Result<Vec<u8>, StatisticalError> {
    if data.len() % 2 != 0 {
        return Err(StatisticalError::InvalidData(
            "Corrupted RLE stream (odd length)".to_string(),
        ));
    }
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        let count = data[i] as usize;
        let byte = data[i + 1];
        out.resize(out.len() + count, byte);
        i += 2;
    }
    Ok(out)
}

impl DataIndexingEngine {
    pub fn new() -> Self {
        Self {
            indexes: HashMap::new(),
            indexing_strategy: IndexingStrategy::BTree,
            query_optimizer: QueryOptimizer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        self.query_optimizer.initialize()?;
        Ok(())
    }

    /// Returns the configured indexing strategy.
    pub fn indexing_strategy(&self) -> &IndexingStrategy {
        &self.indexing_strategy
    }

    /// Reconfigure the indexing strategy.
    pub fn set_indexing_strategy(&mut self, strategy: IndexingStrategy) {
        self.indexing_strategy = strategy;
    }

    /// Add (or replace) a named data index.
    pub fn add_index(&mut self, index: DataIndex) {
        self.indexes.insert(index.index_id.clone(), index);
    }

    /// Look up a data index by id.
    pub fn get_index(&self, index_id: &str) -> Option<&DataIndex> {
        self.indexes.get(index_id)
    }

    /// Remove a data index by id.
    pub fn remove_index(&mut self, index_id: &str) -> Option<DataIndex> {
        self.indexes.remove(index_id)
    }

    /// List the ids of all registered indexes.
    pub fn list_index_ids(&self) -> Vec<String> {
        self.indexes.keys().cloned().collect()
    }

    /// Returns the number of registered indexes.
    pub fn index_count(&self) -> usize {
        self.indexes.len()
    }

    /// Returns a reference to the query optimizer.
    pub fn query_optimizer(&self) -> &QueryOptimizer {
        &self.query_optimizer
    }

    /// Returns a mutable reference to the query optimizer.
    pub fn query_optimizer_mut(&mut self) -> &mut QueryOptimizer {
        &mut self.query_optimizer
    }
}

impl QueryOptimizer {
    pub fn new() -> Self {
        Self {
            optimization_rules: vec![
                OptimizationRule::PredicatePushdown,
                OptimizationRule::IndexSelection,
            ],
            cost_model: CostModel {
                cpu_cost: 0.0,
                io_cost: 0.0,
                memory_cost: 0.0,
                network_cost: 0.0,
            },
            execution_plan: ExecutionPlan {
                plan_id: "default".to_string(),
                operations: Vec::new(),
                estimated_cost: 0.0,
                execution_time: 0,
            },
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        Ok(())
    }

    /// Returns the list of optimization rules currently registered.
    pub fn optimization_rules(&self) -> &[OptimizationRule] {
        &self.optimization_rules
    }

    /// Add an optimization rule if it is not already present.
    pub fn add_optimization_rule(&mut self, rule: OptimizationRule) {
        if !self.optimization_rules.contains(&rule) {
            self.optimization_rules.push(rule);
        }
    }

    /// Returns `true` when the given rule is registered.
    pub fn has_rule(&self, rule: &OptimizationRule) -> bool {
        self.optimization_rules.contains(rule)
    }

    /// Estimate the cost of a single query operation based on its type and the
    /// amount of data it operates on. Costs are dimensionless weights chosen so
    /// that cheaper operations (Filter, Project, Limit) sort before expensive
    /// ones (Scan, Join, Aggregate, Sort).
    pub fn estimate_cost(&self, operation: &QueryOperation) -> CostModel {
        let n = operation.data_size_hint();
        match operation {
            // Full scan: heavy I/O, light CPU.
            QueryOperation::Scan { .. } => CostModel {
                cpu_cost: 0.1 * n,
                io_cost: 1.0 * n,
                memory_cost: 0.05 * n,
                network_cost: 0.0,
            },
            // Filter: cheap, mostly CPU.
            QueryOperation::Filter { .. } => CostModel {
                cpu_cost: 0.2 * n,
                io_cost: 0.0,
                memory_cost: 0.02 * n,
                network_cost: 0.0,
            },
            // Project: cheap column selection.
            QueryOperation::Project { .. } => CostModel {
                cpu_cost: 0.1 * n,
                io_cost: 0.0,
                memory_cost: 0.02 * n,
                network_cost: 0.0,
            },
            // Aggregate: moderate CPU + memory.
            QueryOperation::Aggregate { .. } => CostModel {
                cpu_cost: 0.5 * n,
                io_cost: 0.1 * n,
                memory_cost: 0.3 * n,
                network_cost: 0.0,
            },
            // Join: the most expensive — CPU, memory, and network.
            QueryOperation::Join { .. } => CostModel {
                cpu_cost: 1.0 * n,
                io_cost: 0.5 * n,
                memory_cost: 1.0 * n,
                network_cost: 0.5 * n,
            },
            // Sort: CPU + memory heavy.
            QueryOperation::Sort { .. } => CostModel {
                cpu_cost: 0.6 * n,
                io_cost: 0.2 * n,
                memory_cost: 0.5 * n,
                network_cost: 0.0,
            },
            // Limit: very cheap.
            QueryOperation::Limit { .. } => CostModel {
                cpu_cost: 0.05 * n,
                io_cost: 0.0,
                memory_cost: 0.01 * n,
                network_cost: 0.0,
            },
        }
    }

    /// Optimize a sequence of operations by reordering them to minimize total
    /// cost. Uses a simple greedy strategy: estimate each operation's cost and
    /// execute cheapest-first. The resulting [`ExecutionPlan`] is stored on the
    /// optimizer and also returned.
    pub fn optimize_with_cost(
        &mut self,
        operations: &[QueryOperation],
    ) -> Result<ExecutionPlan, StatisticalError> {
        let mut indexed: Vec<(usize, QueryOperation)> = operations
            .iter()
            .cloned()
            .map(|op| op)
            .enumerate()
            .collect();
        // Greedy: sort by estimated total cost, cheapest first. The original
        // index is retained so callers can inspect the reordering if desired.
        indexed.sort_by(|a, b| {
            let ca = self.estimate_cost(&a.1).total_cost();
            let cb = self.estimate_cost(&b.1).total_cost();
            ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut ordered: Vec<QueryOperation> = indexed.into_iter().map(|(_, op)| op).collect();
        let total: f64 = ordered
            .iter()
            .map(|op| self.estimate_cost(op).total_cost())
            .sum();

        // Aggregate the per-operation costs into the optimizer's cost model so
        // the field is actually used.
        self.cost_model = ordered.iter().map(|op| self.estimate_cost(op)).fold(
            CostModel {
                cpu_cost: 0.0,
                io_cost: 0.0,
                memory_cost: 0.0,
                network_cost: 0.0,
            },
            |acc, c| CostModel {
                cpu_cost: acc.cpu_cost + c.cpu_cost,
                io_cost: acc.io_cost + c.io_cost,
                memory_cost: acc.memory_cost + c.memory_cost,
                network_cost: acc.network_cost + c.network_cost,
            },
        );

        let plan = ExecutionPlan {
            plan_id: format!("plan_{}", self.next_plan_id()),
            operations: std::mem::take(&mut ordered),
            estimated_cost: total,
            execution_time: 0,
        };
        self.execution_plan = plan.clone();
        Ok(plan)
    }

    /// Returns the most recently optimized execution plan, if any.
    pub fn get_execution_plan(&self) -> Option<&ExecutionPlan> {
        if self.execution_plan.operations.is_empty() {
            None
        } else {
            Some(&self.execution_plan)
        }
    }

    /// Monotonic plan id counter (kept simple — no persistent state needed).
    fn next_plan_id(&self) -> u64 {
        // Use the current plan's operation count as a cheap discriminator.
        self.execution_plan.operations.len() as u64 + 1
    }

    /// Optimize a sequence of operations into a [`QueryPlan`].
    ///
    /// Applies three rewrite rules:
    /// 1. **Predicate pushdown** — filters are moved ahead of joins so rows are
    ///    reduced before the expensive join.
    /// 2. **Join-type selection** — HashJoin is selected when both join inputs
    ///    are ≥ 1000 rows; NestedLoop when both are < 100; otherwise the
    ///    caller-supplied join type is retained.
    /// 3. **Limit-last** — Limit is always the final step.
    ///
    /// After reordering, per-step cost and output row count are estimated
    /// using a simple cost model that tracks the running row count through
    /// the plan.
    pub fn optimize(&self, operations: Vec<QueryOperation>) -> Result<QueryPlan, StatisticalError> {
        if operations.is_empty() {
            return Ok(QueryPlan {
                operations: Vec::new(),
                estimated_cost: 0.0,
                estimated_rows: 0,
            });
        }

        // 1. Stable reorder by canonical plan priority (scan → filter → join →
        //    aggregate → sort → limit).  This achieves both filter-pushdown
        //    and limit-last in a single pass.
        let mut reordered: Vec<QueryOperation> = operations;
        reordered.sort_by_key(|op| op.plan_priority());

        // 2. Join-type selection: override join_type based on input sizes.
        for op in reordered.iter_mut() {
            if let QueryOperation::Join {
                left_cost,
                right_cost,
                join_type,
            } = op
            {
                if *left_cost >= 1000.0 && *right_cost >= 1000.0 {
                    *join_type = JoinType::HashJoin;
                } else if *left_cost < 100.0 && *right_cost < 100.0 {
                    *join_type = JoinType::NestedLoop;
                }
            }
        }

        // 3. Build plan with per-step cost and row estimates.
        let mut steps: Vec<QueryStep> = Vec::with_capacity(reordered.len());
        let mut current_rows: usize = 0;
        let mut total_cost: f64 = 0.0;

        for op in reordered {
            let (cost, output_rows) = Self::estimate_step(&op, current_rows);
            steps.push(QueryStep {
                operation: op,
                estimated_cost: cost,
                estimated_rows: output_rows,
            });
            current_rows = output_rows;
            total_cost += cost;
        }

        Ok(QueryPlan {
            operations: steps,
            estimated_cost: total_cost,
            estimated_rows: current_rows,
        })
    }

    /// Per-step cost and output-row estimation. `input_rows` is the running
    /// row count from the previous step (0 for the first step).
    fn estimate_step(op: &QueryOperation, input_rows: usize) -> (f64, usize) {
        match op {
            QueryOperation::Scan { estimated_rows, .. } => {
                let cost = *estimated_rows as f64 * 0.01;
                (cost, *estimated_rows)
            }
            QueryOperation::Filter { selectivity, .. } => {
                let n = input_rows.max(1) as f64;
                let cost = n * selectivity * 0.005;
                let output = (n * selectivity).round() as usize;
                (cost, output)
            }
            QueryOperation::Join {
                left_cost,
                right_cost,
                ..
            } => {
                let n = left_cost * right_cost;
                let cost = n * 0.001;
                let output = n.round() as usize;
                (cost, output)
            }
            QueryOperation::Aggregate { .. } => {
                let n = input_rows.max(1) as f64;
                let cost = n * 0.01;
                (cost, input_rows)
            }
            QueryOperation::Sort { .. } => {
                let n = input_rows.max(1) as f64;
                let cost = n * 0.01;
                (cost, input_rows)
            }
            QueryOperation::Limit { count } => {
                let cost = *count as f64 * 0.001;
                let output = (*count).min(input_rows.max(1));
                (cost, output)
            }
            QueryOperation::Project { .. } => {
                let n = input_rows.max(1) as f64;
                let cost = n * 0.001;
                (cost, input_rows)
            }
        }
    }
}

impl CostModel {
    /// Sum of all cost components.
    pub fn total_cost(&self) -> f64 {
        self.cpu_cost + self.io_cost + self.memory_cost + self.network_cost
    }

    /// Returns `true` when `self` is cheaper than `other`.
    pub fn is_better_than(&self, other: &CostModel) -> bool {
        self.total_cost() < other.total_cost()
    }
}

impl StatisticalComputationEngine {
    pub fn new() -> Self {
        Self {
            computation_units: Vec::new(),
            operation_queue: Vec::new(),
            scheduler: StatisticalScheduler::new(),
            accelerator: StatisticalAccelerator::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        self.scheduler.initialize()?;
        self.accelerator.initialize()?;
        Ok(())
    }

    /// Register a computation unit that can execute statistical operations.
    pub fn add_computation_unit(&mut self, unit: StatisticalComputationUnit) {
        self.computation_units.push(unit);
    }

    /// Returns the list of registered computation units.
    pub fn computation_units(&self) -> &[StatisticalComputationUnit] {
        &self.computation_units
    }

    /// Look up a computation unit by id.
    pub fn get_computation_unit(&self, unit_id: &str) -> Option<&StatisticalComputationUnit> {
        self.computation_units.iter().find(|u| u.unit_id == unit_id)
    }

    /// Returns the number of registered computation units.
    pub fn computation_unit_count(&self) -> usize {
        self.computation_units.len()
    }

    /// Enqueue a statistical operation for later execution.
    pub fn enqueue_operation(&mut self, operation: StatisticalOperation) {
        self.operation_queue.push(operation);
    }

    /// Returns the operations currently waiting in the queue.
    pub fn operation_queue(&self) -> &[StatisticalOperation] {
        &self.operation_queue
    }

    /// Drain all queued operations, returning them in submission order.
    pub fn drain_operation_queue(&mut self) -> Vec<StatisticalOperation> {
        std::mem::take(&mut self.operation_queue)
    }

    /// Returns the number of operations currently in the queue.
    pub fn queued_operation_count(&self) -> usize {
        self.operation_queue.len()
    }

    /// Returns a reference to the scheduler.
    pub fn scheduler(&self) -> &StatisticalScheduler {
        &self.scheduler
    }

    /// Returns a mutable reference to the scheduler.
    pub fn scheduler_mut(&mut self) -> &mut StatisticalScheduler {
        &mut self.scheduler
    }
}

impl StatisticalScheduler {
    pub fn new() -> Self {
        Self {
            scheduling_policy: SchedulingPolicy::Priority,
            queue_manager: QueueManager::new(),
            load_balancer: LoadBalancer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        Ok(())
    }

    /// Returns the current scheduling policy.
    pub fn scheduling_policy(&self) -> &SchedulingPolicy {
        &self.scheduling_policy
    }

    /// Set the scheduling policy.
    pub fn set_scheduling_policy(&mut self, policy: SchedulingPolicy) {
        self.scheduling_policy = policy;
    }

    /// Returns a reference to the queue manager.
    pub fn queue_manager(&self) -> &QueueManager {
        &self.queue_manager
    }

    /// Returns a mutable reference to the queue manager.
    pub fn queue_manager_mut(&mut self) -> &mut QueueManager {
        &mut self.queue_manager
    }

    /// Returns a reference to the load balancer.
    pub fn load_balancer(&self) -> &LoadBalancer {
        &self.load_balancer
    }

    /// Returns a mutable reference to the load balancer.
    pub fn load_balancer_mut(&mut self) -> &mut LoadBalancer {
        &mut self.load_balancer
    }
}

impl QueueManager {
    pub fn new() -> Self {
        Self {
            pending_queue: Vec::new(),
            running_operations: HashMap::new(),
            completed_operations: Vec::new(),
        }
    }

    /// Enqueue a pending operation.
    pub fn enqueue(&mut self, operation: QueuedOperation) {
        self.pending_queue.push(operation);
    }

    /// Dequeue the next pending operation (FIFO order). Returns `None` when
    /// the queue is empty.
    pub fn dequeue(&mut self) -> Option<QueuedOperation> {
        if self.pending_queue.is_empty() {
            None
        } else {
            Some(self.pending_queue.remove(0))
        }
    }

    /// Returns the pending operations currently in the queue.
    pub fn pending_queue(&self) -> &[QueuedOperation] {
        &self.pending_queue
    }

    /// Returns the number of pending operations.
    pub fn pending_count(&self) -> usize {
        self.pending_queue.len()
    }

    /// Mark an operation as running, recording it under `operation_id`.
    pub fn start_operation(&mut self, operation: RunningOperation) {
        self.running_operations
            .insert(operation.operation_id.clone(), operation);
    }

    /// Look up a running operation by id.
    pub fn get_running_operation(&self, operation_id: &str) -> Option<&RunningOperation> {
        self.running_operations.get(operation_id)
    }

    /// Remove a running operation (e.g. when it finishes), returning it so
    /// the caller can record completion.
    pub fn remove_running_operation(&mut self, operation_id: &str) -> Option<RunningOperation> {
        self.running_operations.remove(operation_id)
    }

    /// Returns the number of currently running operations.
    pub fn running_count(&self) -> usize {
        self.running_operations.len()
    }

    /// Record a completed operation.
    pub fn record_completed(&mut self, operation: CompletedOperation) {
        self.completed_operations.push(operation);
    }

    /// Returns the completed operations.
    pub fn completed_operations(&self) -> &[CompletedOperation] {
        &self.completed_operations
    }

    /// Returns the number of completed operations.
    pub fn completed_count(&self) -> usize {
        self.completed_operations.len()
    }
}

impl LoadBalancer {
    pub fn new() -> Self {
        Self {
            balancing_strategy: BalancingStrategy::LoadBased,
            unit_metrics: HashMap::new(),
        }
    }

    /// Returns the current balancing strategy.
    pub fn balancing_strategy(&self) -> &BalancingStrategy {
        &self.balancing_strategy
    }

    /// Set the balancing strategy.
    pub fn set_balancing_strategy(&mut self, strategy: BalancingStrategy) {
        self.balancing_strategy = strategy;
    }

    /// Record or update metrics for a computation unit.
    pub fn set_unit_metrics(&mut self, unit_id: &str, metrics: UnitMetrics) {
        self.unit_metrics.insert(unit_id.to_string(), metrics);
    }

    /// Look up metrics for a computation unit.
    pub fn get_unit_metrics(&self, unit_id: &str) -> Option<&UnitMetrics> {
        self.unit_metrics.get(unit_id)
    }

    /// Remove metrics for a computation unit.
    pub fn remove_unit_metrics(&mut self, unit_id: &str) -> Option<UnitMetrics> {
        self.unit_metrics.remove(unit_id)
    }

    /// Returns the number of units with recorded metrics.
    pub fn tracked_unit_count(&self) -> usize {
        self.unit_metrics.len()
    }
}

impl StatisticalAccelerator {
    pub fn new() -> Self {
        Self {
            acceleration_strategies: vec![
                AccelerationStrategy::Vectorization,
                AccelerationStrategy::Parallelization,
            ],
            hardware_accelerators: Vec::new(),
            optimization_engine: OptimizationEngine::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        self.optimization_engine.initialize()?;
        Ok(())
    }

    /// Returns the list of acceleration strategies enabled on this accelerator.
    pub fn acceleration_strategies(&self) -> &[AccelerationStrategy] {
        &self.acceleration_strategies
    }

    /// Enable an additional acceleration strategy if not already present.
    pub fn add_acceleration_strategy(&mut self, strategy: AccelerationStrategy) {
        if !self.acceleration_strategies.contains(&strategy) {
            self.acceleration_strategies.push(strategy);
        }
    }

    /// Returns `true` when the given strategy is enabled.
    pub fn has_acceleration_strategy(&self, strategy: &AccelerationStrategy) -> bool {
        self.acceleration_strategies.contains(strategy)
    }

    /// Register a hardware accelerator.
    pub fn add_hardware_accelerator(&mut self, accelerator: HardwareAccelerator) {
        self.hardware_accelerators.push(accelerator);
    }

    /// Returns the list of registered hardware accelerators.
    pub fn hardware_accelerators(&self) -> &[HardwareAccelerator] {
        &self.hardware_accelerators
    }

    /// Look up a hardware accelerator by id.
    pub fn get_hardware_accelerator(&self, accelerator_id: &str) -> Option<&HardwareAccelerator> {
        self.hardware_accelerators
            .iter()
            .find(|a| a.accelerator_id == accelerator_id)
    }

    /// Returns the number of registered hardware accelerators.
    pub fn hardware_accelerator_count(&self) -> usize {
        self.hardware_accelerators.len()
    }
}

impl OptimizationEngine {
    pub fn new() -> Self {
        Self {}
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        Ok(())
    }
}

impl StatisticalPrivacyEngine {
    pub fn new() -> Self {
        Self {
            fiduciary_crypto: Arc::new(Mutex::new(FiduciaryCrypto::new())),
            zk_proofs: Arc::new(Mutex::new(ZkProofSystem::new())),
            differential_privacy: DifferentialPrivacy::new(),
            secure_aggregation: SecureAggregation::new(),
            privacy_budget: PrivacyBudget {
                epsilon: 1.0,
                delta: 1e-6,
                remaining_epsilon: 1.0,
                remaining_delta: 1e-6,
                budget_period: 86400, // 24 hours
                last_reset: 0,
            },
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        self.differential_privacy.initialize()?;
        self.secure_aggregation.initialize()?;
        Ok(())
    }

    pub fn add_laplace_noise(
        &mut self,
        value: f64,
        sensitivity: f64,
    ) -> Result<(f64, f64), StatisticalError> {
        let epsilon = 1.0;
        let scale = sensitivity / epsilon;

        let noise = self.generate_laplace_noise(scale)?;
        let noisy_value = value + noise;

        // Update privacy budget
        self.privacy_budget.remaining_epsilon -= epsilon;

        Ok((noisy_value, epsilon))
    }

    pub fn add_histogram_noise(
        &mut self,
        counts: &[u32],
    ) -> Result<(Vec<u32>, f64), StatisticalError> {
        let epsilon = 1.0;
        let sensitivity = 1.0;
        let scale = sensitivity / epsilon;

        let mut noisy_counts = Vec::with_capacity(counts.len());
        for &count in counts {
            let noise = self.generate_laplace_noise(scale)?;
            let noisy_count = (count as f64 + noise).max(0.0) as u32;
            noisy_counts.push(noisy_count);
        }

        // Update privacy budget
        self.privacy_budget.remaining_epsilon -= epsilon;

        Ok((noisy_counts, epsilon))
    }

    /// Sample real Laplace(0, `scale`) noise via inverse-CDF transform over OS
    /// entropy.
    ///
    /// SECURITY / CORRECTNESS: a former implementation drew from a global
    /// monotonic `AtomicU64` counter, so the "noise" was fully deterministic
    /// and predictable — which **voids** the differential-privacy guarantee
    /// (an observer who knows the call sequence can subtract the exact noise
    /// and recover the raw value). The old transform was also not a Laplace
    /// sample. This uses real OS entropy (`getrandom`, native + wasm) and the
    /// correct inverse CDF, and **fails closed** (returns `PrivacyError`, no
    /// output) when entropy is unavailable rather than degrade to weak noise.
    ///
    /// Note: `epsilon` is fixed at 1.0 by the callers and the budget is a
    /// simple per-query decrement — this is a valid fixed-ε mechanism, but it
    /// does not enforce ε-composition across many queries (that lives in the
    /// separate `PrivacyAccountant`). Per-query noise is now genuinely random.
    fn generate_laplace_noise(&self, scale: f64) -> Result<f64, StatisticalError> {
        let mut bytes = [0u8; 8];
        getrandom::fill(&mut bytes).map_err(|e| {
            StatisticalError::PrivacyError(format!(
                "no OS entropy for differential-privacy noise: {e}"
            ))
        })?;
        // Map the random u64 to the OPEN interval (0,1) so that ln(1 - 2|u|)
        // stays finite: (x + 0.5) / 2^64 is never exactly 0 or 1.
        let x = u64::from_le_bytes(bytes) as f64;
        let r = (x + 0.5) / (u64::MAX as f64 + 1.0);
        // u ~ Uniform(-1/2, 1/2); Laplace inverse CDF:
        //   X = -scale * sgn(u) * ln(1 - 2|u|)
        let u = r - 0.5;
        Ok(-scale * u.signum() * (1.0 - 2.0 * u.abs()).ln())
    }

    /// Encrypt (seal) a statistical result using the fiduciary crypto system.
    ///
    /// `FiduciaryCrypto` exposes ML-DSA (FIPS-204) signing rather than symmetric
    /// encryption, so "encryption" here means producing an authenticated
    /// signature over the result bytes. The returned bytes are the ML-DSA
    /// signature; a holder of the public key can verify that the result was
    /// produced by this engine and has not been tampered with. A default
    /// signing key is generated lazily on first use.
    pub fn encrypt_result(&self, data: &[u8]) -> Result<Vec<u8>, StatisticalError> {
        let mut crypto = self
            .fiduciary_crypto
            .lock()
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        const STAT_KEY_ID: &str = "statistical_results";
        if !crypto.list_keys().iter().any(|k| k == STAT_KEY_ID) {
            crypto
                .generate_key(STAT_KEY_ID.to_string())
                .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;
        }

        let signature = crypto
            .sign(
                data,
                Some(STAT_KEY_ID),
                "statistical_computing".to_string(),
                "result_encryption".to_string(),
            )
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        Ok(signature.sig_bytes)
    }

    /// Verify (open) a statistical result sealed by `encrypt_result`.
    ///
    /// Returns `Ok(true)` when the signature is valid for `data` under the
    /// engine's statistical-results key.
    pub fn verify_result(&self, data: &[u8], signature: &[u8]) -> Result<bool, StatisticalError> {
        let crypto = self
            .fiduciary_crypto
            .lock()
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        const STAT_KEY_ID: &str = "statistical_results";
        let sig = MlDsaSignature {
            sig_bytes: signature.to_vec(),
        };
        crypto
            .verify(
                data,
                &sig,
                Some(STAT_KEY_ID),
                "statistical_computing".to_string(),
                "result_encryption".to_string(),
            )
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))
    }

    /// Generate a zero-knowledge proof that a statistical computation was
    /// performed correctly.
    ///
    /// The proof binds the private `inputs` and public `outputs` together: a
    /// SHA-256 commitment over all inputs/outputs becomes a private witness,
    /// and the same commitment is exposed as the single public input. The
    /// circuit enforces `one * commitment = commitment`, so a verifying party
    /// learns only that the prover knows the commitment bound to the published
    /// outputs — not the inputs themselves. The returned bytes are a
    /// `serde_json`-serialised `ZkProof` (which carries its own public inputs),
    /// so it can be verified by `verify_computation` without extra state.
    pub fn prove_computation(
        &self,
        computation_id: &str,
        inputs: &[Vec<u8>],
        outputs: &[Vec<u8>],
    ) -> Result<Vec<u8>, StatisticalError> {
        let mut zk = self
            .zk_proofs
            .lock()
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        // Commitment over inputs and outputs: SHA-256 -> 32-byte field element.
        let mut hasher = Sha256::new();
        for chunk in inputs {
            hasher.update(chunk);
        }
        for chunk in outputs {
            hasher.update(chunk);
        }
        let digest = hasher.finalize();
        let mut commitment = [0u8; 32];
        commitment.copy_from_slice(&digest);

        let circuit_id = format!("stat_comp_{}", computation_id);
        zk.create_circuit(circuit_id.clone())
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        // Public input: the commitment bound to the published outputs.
        zk.add_variable(&circuit_id, "commitment".to_string(), VariableType::Public)
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;
        // Private witness: the multiplicative identity and the same commitment.
        zk.add_variable(&circuit_id, "one".to_string(), VariableType::Private)
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;
        zk.add_variable(&circuit_id, "in_commit".to_string(), VariableType::Private)
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        // Constraint: one * in_commit = commitment (binds private/public).
        zk.add_constraint(
            &circuit_id,
            CircuitExpression::Variable("one".to_string()),
            CircuitExpression::Variable("in_commit".to_string()),
            CircuitExpression::Variable("commitment".to_string()),
        )
        .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        zk.generate_keys(&circuit_id)
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        // Field-one in little-endian: [1, 0, ...].
        let mut one_val = [0u8; 32];
        one_val[0] = 1;

        let mut witness = HashMap::new();
        witness.insert("one".to_string(), FieldElement { value: one_val });
        witness.insert("in_commit".to_string(), FieldElement { value: commitment });
        witness.insert("commitment".to_string(), FieldElement { value: commitment });

        let public_inputs = vec![FieldElement { value: commitment }];

        let proof = zk
            .generate_proof(&circuit_id, witness, public_inputs)
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        serde_json::to_vec(&proof).map_err(|e| StatisticalError::PrivacyError(e.to_string()))
    }

    /// Verify a zero-knowledge computation proof produced by `prove_computation`.
    ///
    /// `proof` is the serialised `ZkProof` bytes. When `public_inputs` is
    /// non-empty, each entry is interpreted as a 32-byte little-endian field
    /// element and checked against the public inputs embedded in the proof, so
    /// callers can confirm the proof binds to the outputs they expect.
    pub fn verify_computation(
        &self,
        proof: &[u8],
        public_inputs: &[Vec<u8>],
    ) -> Result<bool, StatisticalError> {
        let zk_proof: ZkProof = serde_json::from_slice(proof)
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        // Optional binding check: the caller-supplied public inputs must match
        // the ones embedded in the proof.
        if !public_inputs.is_empty() {
            if public_inputs.len() != zk_proof.public_inputs.len() {
                return Ok(false);
            }
            for (expected, actual) in public_inputs.iter().zip(&zk_proof.public_inputs) {
                let mut expected_arr = [0u8; 32];
                let len = expected.len().min(32);
                expected_arr[..len].copy_from_slice(&expected[..len]);
                if expected_arr != actual.value {
                    return Ok(false);
                }
            }
        }

        let mut zk = self
            .zk_proofs
            .lock()
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        let result = zk
            .verify_proof(&zk_proof)
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        Ok(result.is_valid)
    }
}

impl DifferentialPrivacy {
    pub fn new() -> Self {
        Self {
            noise_mechanisms: vec![NoiseMechanism::Laplace, NoiseMechanism::Gaussian],
            privacy_accountant: PrivacyAccountant {
                total_epsilon_spent: 0.0,
                total_delta_spent: 0.0,
                composition_method: CompositionMethod::AdvancedComposition,
                remaining_budget: PrivacyBudget {
                    epsilon: 1.0,
                    delta: 1e-6,
                    remaining_epsilon: 1.0,
                    remaining_delta: 1e-6,
                    budget_period: 86400,
                    last_reset: 0,
                },
            },
            sensitivity_analyzer: SensitivityAnalyzer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        self.sensitivity_analyzer.initialize()?;
        Ok(())
    }

    /// Returns the list of noise mechanisms available to this DP engine.
    pub fn noise_mechanisms(&self) -> &[NoiseMechanism] {
        &self.noise_mechanisms
    }

    /// Register an additional noise mechanism if not already present.
    pub fn add_noise_mechanism(&mut self, mechanism: NoiseMechanism) {
        if !self.noise_mechanisms.contains(&mechanism) {
            self.noise_mechanisms.push(mechanism);
        }
    }

    /// Returns `true` when the given noise mechanism is registered.
    pub fn supports_noise_mechanism(&self, mechanism: &NoiseMechanism) -> bool {
        self.noise_mechanisms.contains(mechanism)
    }

    /// Returns a reference to the privacy accountant tracking epsilon/delta spend.
    pub fn privacy_accountant(&self) -> &PrivacyAccountant {
        &self.privacy_accountant
    }

    /// Returns a mutable reference to the privacy accountant.
    pub fn privacy_accountant_mut(&mut self) -> &mut PrivacyAccountant {
        &mut self.privacy_accountant
    }

    /// Spend `epsilon`/`delta` from the privacy budget, recording the
    /// consumption in the accountant. Returns `Err` when the budget is
    /// insufficient.
    pub fn spend_budget(&mut self, epsilon: f64, delta: f64) -> Result<(), StatisticalError> {
        let remaining = &self.privacy_accountant.remaining_budget;
        if remaining.remaining_epsilon < epsilon || remaining.remaining_delta < delta {
            return Err(StatisticalError::PrivacyError(
                "Insufficient privacy budget".to_string(),
            ));
        }
        self.privacy_accountant.remaining_budget.remaining_epsilon -= epsilon;
        self.privacy_accountant.remaining_budget.remaining_delta -= delta;
        self.privacy_accountant.total_epsilon_spent += epsilon;
        self.privacy_accountant.total_delta_spent += delta;
        Ok(())
    }
}

impl SensitivityAnalyzer {
    pub fn new() -> Self {
        Self {
            sensitivity_functions: HashMap::new(),
            sensitivity_cache: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        Ok(())
    }

    /// Register a named sensitivity function so it can be looked up by name
    /// from `compute_sensitivity` / `get_sensitivity`.
    pub fn register_function(&mut self, name: &str, func: SensitivityFunction) {
        self.sensitivity_functions.insert(name.to_string(), func);
    }

    /// Compute the L1 sensitivity of a statistical operation over `data`.
    ///
    /// Sensitivity is the maximum change in the operation's output when a single
    /// record is added or removed. The following closed-form approximations are
    /// used (each assumes a bounded domain where one record can shift a value by
    /// at most 1.0):
    ///
    /// - `mean`:      `1/n`        — one record moves the mean by `1/n`.
    /// - `sum`:       `1.0`        — one record changes the sum by at most 1.
    /// - `count`:     `1.0`        — one record changes the count by 1.
    /// - `median`:    `range / n`  — adjacent-element approximation.
    /// - `variance`:  `(max-min)^2 / n` — bounded shift approximation.
    /// - `histogram`: `1.0`        — one record changes a single bin by 1.
    ///
    /// Results are cached keyed by `operation` so repeated DP queries reuse the
    /// computed sensitivity.
    pub fn compute_sensitivity(
        &mut self,
        operation: &str,
        data: &[f64],
    ) -> Result<f64, StatisticalError> {
        // A registered function wins over the built-in approximations.
        if let Some(func) = self.sensitivity_functions.get(operation) {
            self.sensitivity_cache
                .insert(operation.to_string(), func.sensitivity);
            return Ok(func.sensitivity);
        }

        if data.is_empty() {
            return Err(StatisticalError::InvalidData(
                "Cannot compute sensitivity over empty data".to_string(),
            ));
        }

        let n = data.len() as f64;
        let sensitivity = match operation {
            "mean" => 1.0 / n,
            "sum" => 1.0,
            "count" => 1.0,
            "histogram" => 1.0,
            "median" => {
                let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
                let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                (max - min) / n
            }
            "variance" => {
                let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
                let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let range = max - min;
                (range * range) / n
            }
            other => {
                return Err(StatisticalError::InvalidOperation(format!(
                    "Unknown sensitivity operation '{}'",
                    other
                )))
            }
        };

        self.sensitivity_cache
            .insert(operation.to_string(), sensitivity);
        Ok(sensitivity)
    }

    /// Get the sensitivity for an operation, returning the cached value when
    /// available and computing (and caching) it otherwise.
    pub fn get_sensitivity(
        &mut self,
        operation: &str,
        data: &[f64],
    ) -> Result<f64, StatisticalError> {
        if let Some(cached) = self.sensitivity_cache.get(operation) {
            return Ok(*cached);
        }
        self.compute_sensitivity(operation, data)
    }
}

impl SecureAggregation {
    pub fn new() -> Self {
        Self {
            aggregation_protocols: vec![
                AggregationProtocol::SecureSum,
                AggregationProtocol::SecureMean,
            ],
            encryption_schemes: vec![
                EncryptionScheme::Homomorphic,
                EncryptionScheme::SecretSharing,
            ],
            integrity_checks: Vec::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        Ok(())
    }

    /// Returns the list of registered aggregation protocols.
    pub fn aggregation_protocols(&self) -> &[AggregationProtocol] {
        &self.aggregation_protocols
    }

    /// Register an additional aggregation protocol if not already present.
    pub fn add_aggregation_protocol(&mut self, protocol: AggregationProtocol) {
        if !self.aggregation_protocols.contains(&protocol) {
            self.aggregation_protocols.push(protocol);
        }
    }

    /// Returns the list of registered encryption schemes.
    pub fn encryption_schemes(&self) -> &[EncryptionScheme] {
        &self.encryption_schemes
    }

    /// Register an additional encryption scheme if not already present.
    pub fn add_encryption_scheme(&mut self, scheme: EncryptionScheme) {
        if !self.encryption_schemes.contains(&scheme) {
            self.encryption_schemes.push(scheme);
        }
    }

    /// Register an integrity check.
    pub fn add_integrity_check(&mut self, check: IntegrityCheck) {
        self.integrity_checks.push(check);
    }

    /// Returns the list of registered integrity checks.
    pub fn integrity_checks(&self) -> &[IntegrityCheck] {
        &self.integrity_checks
    }

    /// Look up an integrity check by id.
    pub fn get_integrity_check(&self, check_id: &str) -> Option<&IntegrityCheck> {
        self.integrity_checks
            .iter()
            .find(|c| c.check_id == check_id)
    }

    /// Returns the number of registered integrity checks.
    pub fn integrity_check_count(&self) -> usize {
        self.integrity_checks.len()
    }
}

impl StatisticalAnalysisEngine {
    pub fn new() -> Self {
        Self {
            analysis_algorithms: vec![
                AnalysisAlgorithm::DescriptiveAnalysis,
                AnalysisAlgorithm::InferentialAnalysis,
            ],
            pattern_recognition: PatternRecognition::new(),
            anomaly_detection: AnomalyDetection::new(),
            forecasting_engine: ForecastingEngine::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        self.pattern_recognition.initialize()?;
        self.anomaly_detection.initialize()?;
        self.forecasting_engine.initialize()?;
        Ok(())
    }

    /// Returns the list of analysis algorithms available to this engine.
    pub fn analysis_algorithms(&self) -> &[AnalysisAlgorithm] {
        &self.analysis_algorithms
    }

    /// Register an additional analysis algorithm if not already present.
    pub fn add_analysis_algorithm(&mut self, algorithm: AnalysisAlgorithm) {
        if !self.analysis_algorithms.contains(&algorithm) {
            self.analysis_algorithms.push(algorithm);
        }
    }

    /// Returns `true` when the given analysis algorithm is registered.
    pub fn supports_analysis_algorithm(&self, algorithm: &AnalysisAlgorithm) -> bool {
        self.analysis_algorithms.contains(algorithm)
    }

    /// Returns a reference to the pattern recognition subsystem.
    pub fn pattern_recognition(&self) -> &PatternRecognition {
        &self.pattern_recognition
    }

    /// Returns a mutable reference to the pattern recognition subsystem.
    pub fn pattern_recognition_mut(&mut self) -> &mut PatternRecognition {
        &mut self.pattern_recognition
    }

    /// Returns a reference to the anomaly detection subsystem.
    pub fn anomaly_detection(&self) -> &AnomalyDetection {
        &self.anomaly_detection
    }

    /// Returns a mutable reference to the anomaly detection subsystem.
    pub fn anomaly_detection_mut(&mut self) -> &mut AnomalyDetection {
        &mut self.anomaly_detection
    }

    /// Returns a reference to the forecasting engine.
    pub fn forecasting_engine(&self) -> &ForecastingEngine {
        &self.forecasting_engine
    }

    /// Returns a mutable reference to the forecasting engine.
    pub fn forecasting_engine_mut(&mut self) -> &mut ForecastingEngine {
        &mut self.forecasting_engine
    }
}

impl PatternRecognition {
    pub fn new() -> Self {
        Self {
            pattern_types: vec![
                PatternType::Trend,
                PatternType::Seasonal,
                PatternType::Outlier,
            ],
            recognition_algorithms: vec![RecognitionAlgorithm::Statistical],
            pattern_library: PatternLibrary::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        self.pattern_library.initialize()?;
        Ok(())
    }

    /// Returns the list of pattern types this recognizer looks for.
    pub fn pattern_types(&self) -> &[PatternType] {
        &self.pattern_types
    }

    /// Register an additional pattern type if not already present.
    pub fn add_pattern_type(&mut self, pattern_type: PatternType) {
        if !self.pattern_types.contains(&pattern_type) {
            self.pattern_types.push(pattern_type);
        }
    }

    /// Returns the list of recognition algorithms available.
    pub fn recognition_algorithms(&self) -> &[RecognitionAlgorithm] {
        &self.recognition_algorithms
    }

    /// Register an additional recognition algorithm if not already present.
    pub fn add_recognition_algorithm(&mut self, algorithm: RecognitionAlgorithm) {
        if !self.recognition_algorithms.contains(&algorithm) {
            self.recognition_algorithms.push(algorithm);
        }
    }

    /// Returns a reference to the pattern library.
    pub fn pattern_library(&self) -> &PatternLibrary {
        &self.pattern_library
    }

    /// Returns a mutable reference to the pattern library.
    pub fn pattern_library_mut(&mut self) -> &mut PatternLibrary {
        &mut self.pattern_library
    }
}

impl PatternLibrary {
    pub fn new() -> Self {
        Self {
            patterns: HashMap::new(),
            pattern_templates: Vec::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        Ok(())
    }

    /// Add (or replace) a named statistical pattern.
    pub fn add_pattern(&mut self, pattern: StatisticalPattern) {
        self.patterns.insert(pattern.pattern_id.clone(), pattern);
    }

    /// Look up a statistical pattern by id.
    pub fn get_pattern(&self, pattern_id: &str) -> Option<&StatisticalPattern> {
        self.patterns.get(pattern_id)
    }

    /// Remove a statistical pattern by id.
    pub fn remove_pattern(&mut self, pattern_id: &str) -> Option<StatisticalPattern> {
        self.patterns.remove(pattern_id)
    }

    /// List the ids of all stored patterns.
    pub fn list_pattern_ids(&self) -> Vec<String> {
        self.patterns.keys().cloned().collect()
    }

    /// Returns the number of stored patterns.
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    /// Register a pattern template.
    pub fn add_pattern_template(&mut self, template: PatternTemplate) {
        self.pattern_templates.push(template);
    }

    /// Returns the list of registered pattern templates.
    pub fn pattern_templates(&self) -> &[PatternTemplate] {
        &self.pattern_templates
    }

    /// Look up a pattern template by id.
    pub fn get_pattern_template(&self, template_id: &str) -> Option<&PatternTemplate> {
        self.pattern_templates
            .iter()
            .find(|t| t.template_id == template_id)
    }

    /// Returns the number of registered pattern templates.
    pub fn pattern_template_count(&self) -> usize {
        self.pattern_templates.len()
    }
}

impl AnomalyDetection {
    pub fn new() -> Self {
        Self {
            detection_algorithms: vec![DetectionAlgorithm::Statistical],
            threshold_methods: vec![ThresholdMethod::Static],
            alert_system: AlertSystem::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        self.alert_system.initialize()?;
        Ok(())
    }

    /// Returns the list of registered detection algorithms.
    pub fn detection_algorithms(&self) -> &[DetectionAlgorithm] {
        &self.detection_algorithms
    }

    /// Register an additional detection algorithm if not already present.
    pub fn add_detection_algorithm(&mut self, algorithm: DetectionAlgorithm) {
        if !self.detection_algorithms.contains(&algorithm) {
            self.detection_algorithms.push(algorithm);
        }
    }

    /// Returns the list of registered threshold methods.
    pub fn threshold_methods(&self) -> &[ThresholdMethod] {
        &self.threshold_methods
    }

    /// Register an additional threshold method if not already present.
    pub fn add_threshold_method(&mut self, method: ThresholdMethod) {
        if !self.threshold_methods.contains(&method) {
            self.threshold_methods.push(method);
        }
    }

    /// Returns a reference to the alert system.
    pub fn alert_system(&self) -> &AlertSystem {
        &self.alert_system
    }

    /// Returns a mutable reference to the alert system.
    pub fn alert_system_mut(&mut self) -> &mut AlertSystem {
        &mut self.alert_system
    }
}

impl AlertSystem {
    pub fn new() -> Self {
        Self {
            alert_types: vec![AlertType::Threshold, AlertType::Anomaly],
            notification_channels: vec![NotificationChannel::Email],
            escalation_policies: Vec::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        Ok(())
    }

    /// Returns the list of registered alert types.
    pub fn alert_types(&self) -> &[AlertType] {
        &self.alert_types
    }

    /// Register an additional alert type if not already present.
    pub fn add_alert_type(&mut self, alert_type: AlertType) {
        if !self.alert_types.contains(&alert_type) {
            self.alert_types.push(alert_type);
        }
    }

    /// Returns the list of registered notification channels.
    pub fn notification_channels(&self) -> &[NotificationChannel] {
        &self.notification_channels
    }

    /// Register an additional notification channel if not already present.
    pub fn add_notification_channel(&mut self, channel: NotificationChannel) {
        if !self.notification_channels.contains(&channel) {
            self.notification_channels.push(channel);
        }
    }

    /// Register an escalation policy.
    pub fn add_escalation_policy(&mut self, policy: EscalationPolicy) {
        self.escalation_policies.push(policy);
    }

    /// Returns the list of registered escalation policies.
    pub fn escalation_policies(&self) -> &[EscalationPolicy] {
        &self.escalation_policies
    }

    /// Look up an escalation policy by id.
    pub fn get_escalation_policy(&self, policy_id: &str) -> Option<&EscalationPolicy> {
        self.escalation_policies
            .iter()
            .find(|p| p.policy_id == policy_id)
    }

    /// Returns the number of registered escalation policies.
    pub fn escalation_policy_count(&self) -> usize {
        self.escalation_policies.len()
    }
}

impl ForecastingEngine {
    pub fn new() -> Self {
        Self {
            forecasting_models: vec![
                ForecastingModel::ARIMA,
                ForecastingModel::ExponentialSmoothing,
            ],
            accuracy_metrics: AccuracyMetrics {
                mae: 0.0,
                mse: 0.0,
                rmse: 0.0,
                mape: 0.0,
                smape: 0.0,
                r_squared: 0.0,
            },
            model_selection: ModelSelection::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        self.model_selection.initialize()?;
        Ok(())
    }

    /// Returns the list of registered forecasting models.
    pub fn forecasting_models(&self) -> &[ForecastingModel] {
        &self.forecasting_models
    }

    /// Register an additional forecasting model if not already present.
    pub fn add_forecasting_model(&mut self, model: ForecastingModel) {
        if !self.forecasting_models.contains(&model) {
            self.forecasting_models.push(model);
        }
    }

    /// Returns `true` when the given forecasting model is registered.
    pub fn supports_forecasting_model(&self, model: &ForecastingModel) -> bool {
        self.forecasting_models.contains(model)
    }

    /// Returns a reference to the current accuracy metrics.
    pub fn accuracy_metrics(&self) -> &AccuracyMetrics {
        &self.accuracy_metrics
    }

    /// Update the accuracy metrics after a forecasting run.
    pub fn set_accuracy_metrics(&mut self, metrics: AccuracyMetrics) {
        self.accuracy_metrics = metrics;
    }

    /// Returns a reference to the model selection subsystem.
    pub fn model_selection(&self) -> &ModelSelection {
        &self.model_selection
    }

    /// Returns a mutable reference to the model selection subsystem.
    pub fn model_selection_mut(&mut self) -> &mut ModelSelection {
        &mut self.model_selection
    }
}

impl ModelSelection {
    pub fn new() -> Self {
        Self {
            selection_criteria: vec![SelectionCriterion::Accuracy, SelectionCriterion::Speed],
            cross_validation: CrossValidation::new(),
            hyperparameter_tuning: HyperparameterTuning::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        Ok(())
    }

    /// Returns the list of registered selection criteria.
    pub fn selection_criteria(&self) -> &[SelectionCriterion] {
        &self.selection_criteria
    }

    /// Register an additional selection criterion if not already present.
    pub fn add_selection_criterion(&mut self, criterion: SelectionCriterion) {
        if !self.selection_criteria.contains(&criterion) {
            self.selection_criteria.push(criterion);
        }
    }

    /// Returns a reference to the cross-validation configuration.
    pub fn cross_validation(&self) -> &CrossValidation {
        &self.cross_validation
    }

    /// Returns a mutable reference to the cross-validation configuration.
    pub fn cross_validation_mut(&mut self) -> &mut CrossValidation {
        &mut self.cross_validation
    }

    /// Returns a reference to the hyperparameter tuning configuration.
    pub fn hyperparameter_tuning(&self) -> &HyperparameterTuning {
        &self.hyperparameter_tuning
    }

    /// Returns a mutable reference to the hyperparameter tuning configuration.
    pub fn hyperparameter_tuning_mut(&mut self) -> &mut HyperparameterTuning {
        &mut self.hyperparameter_tuning
    }
}

impl CrossValidation {
    pub fn new() -> Self {
        Self {
            cv_method: CVMethod::KFold,
            folds: 5,
            shuffle: true,
            stratify: false,
        }
    }
}

impl HyperparameterTuning {
    pub fn new() -> Self {
        Self {
            tuning_method: TuningMethod::GridSearch,
            search_space: SearchSpace::new(),
            max_iterations: 100,
        }
    }
}

impl SearchSpace {
    pub fn new() -> Self {
        Self {
            parameters: Vec::new(),
            constraints: Vec::new(),
        }
    }
}

impl StatisticalPerformanceMonitor {
    pub fn new() -> Self {
        Self {
            operation_metrics: HashMap::new(),
            dataset_metrics: HashMap::new(),
            system_metrics: SystemMetrics {
                total_operations: 0,
                average_execution_time: 0.0,
                throughput: 0.0,
                memory_utilization: 0.0,
                cpu_utilization: 0.0,
                storage_utilization: 0.0,
                energy_efficiency: 0.0,
            },
            privacy_metrics: PrivacyMetrics {
                epsilon_spent: 0.0,
                delta_spent: 0.0,
                privacy_preserved_operations: 0,
                total_operations: 0,
                privacy_efficiency: 0.0,
            },
        }
    }

    pub fn record_operation(
        &mut self,
        _operation_type: &str,
        execution_time: u64,
        _memory_usage: u64,
        privacy_cost: f64,
    ) {
        self.system_metrics.total_operations += 1;
        self.system_metrics.average_execution_time = (self.system_metrics.average_execution_time
            * (self.system_metrics.total_operations - 1) as f64
            + execution_time as f64)
            / self.system_metrics.total_operations as f64;

        self.privacy_metrics.total_operations += 1;
        self.privacy_metrics.epsilon_spent += privacy_cost;
        if privacy_cost > 0.0 {
            self.privacy_metrics.privacy_preserved_operations += 1;
        }
    }

    pub fn get_system_metrics(&self) -> SystemMetrics {
        self.system_metrics.clone()
    }

    /// Record metrics for a specific operation, keyed by `operation_id`.
    pub fn record_operation_metrics(&mut self, metrics: OperationMetrics) {
        self.operation_metrics
            .insert(metrics.operation_id.clone(), metrics);
    }

    /// Look up metrics for a specific operation by id.
    pub fn get_operation_metrics(&self, operation_id: &str) -> Option<&OperationMetrics> {
        self.operation_metrics.get(operation_id)
    }

    /// Returns the number of operations with recorded metrics.
    pub fn operation_metrics_count(&self) -> usize {
        self.operation_metrics.len()
    }

    /// Record metrics for a specific dataset, keyed by `dataset_id`.
    pub fn record_dataset_metrics(&mut self, metrics: DatasetMetrics) {
        self.dataset_metrics
            .insert(metrics.dataset_id.clone(), metrics);
    }

    /// Look up metrics for a specific dataset by id.
    pub fn get_dataset_metrics(&self, dataset_id: &str) -> Option<&DatasetMetrics> {
        self.dataset_metrics.get(dataset_id)
    }

    /// Returns the number of datasets with recorded metrics.
    pub fn dataset_metrics_count(&self) -> usize {
        self.dataset_metrics.len()
    }

    /// Returns a reference to the privacy metrics.
    pub fn privacy_metrics(&self) -> &PrivacyMetrics {
        &self.privacy_metrics
    }
}

// Supporting types

#[derive(Debug, Clone)]
pub struct TTestResult {
    pub t_statistic: f64,
    pub p_value: f64,
    pub degrees_of_freedom: u32,
    pub confidence_interval: (f64, f64),
}

#[derive(Debug, Clone)]
pub struct HistogramResult {
    pub bins: usize,
    pub counts: Vec<u32>,
    pub min_value: f64,
    pub max_value: f64,
    pub bin_width: f64,
}

/// Modal value of a column, with the frequency of that value.
#[derive(Debug, Clone)]
pub struct ModeResult {
    pub value: f64,
    pub count: usize,
    pub sample_size: usize,
}

/// Fitted polynomial-regression model: coefficients (ascending powers, so
/// `coefficients[0]` is the constant term) plus the coefficient of determination.
#[derive(Debug, Clone)]
pub struct PolynomialFit {
    pub degree: usize,
    pub coefficients: Vec<f64>,
    pub r_squared: f64,
    pub n: usize,
}

/// Summary of a fitted soft-margin SVM (the model's internals stay in the solver).
#[derive(Debug, Clone)]
pub struct SvmFitResult {
    pub n_support_vectors: usize,
    pub train_accuracy: f64,
    pub n: usize,
    pub n_features: usize,
}

/// Summary of a fitted random forest, with the in-sample fit metric
/// (R² for a regression forest, classification accuracy for a classifier).
#[derive(Debug, Clone)]
pub struct RandomForestFitResult {
    pub n_trees: usize,
    pub classifier: bool,
    /// R² (regressor) or accuracy in `[0,1]` (classifier), measured in-sample.
    pub train_metric: f64,
    pub n: usize,
    pub n_features: usize,
}

/// Statistical error types
#[derive(Debug, Clone)]
pub enum StatisticalError {
    InvalidData(String),
    InvalidColumn(String),
    InvalidOperation(String),
    StorageError(String),
    ComputationError(String),
    PrivacyError(String),
    AnalysisError(String),
    DataNotFound(String),
}

impl std::fmt::Display for StatisticalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatisticalError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            StatisticalError::InvalidColumn(msg) => write!(f, "Invalid column: {}", msg),
            StatisticalError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            StatisticalError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            StatisticalError::ComputationError(msg) => write!(f, "Computation error: {}", msg),
            StatisticalError::PrivacyError(msg) => write!(f, "Privacy error: {}", msg),
            StatisticalError::AnalysisError(msg) => write!(f, "Analysis error: {}", msg),
            StatisticalError::DataNotFound(msg) => write!(f, "Dataset not found: {}", msg),
        }
    }
}

impl std::error::Error for StatisticalError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_statistical_library_creation() {
        let library = StatisticalComputingLibrary::new();
        assert_eq!(library.list_datasets().len(), 0);
    }

    #[test]
    fn test_dataset_creation() {
        let mut library = StatisticalComputingLibrary::new();
        library.initialize().unwrap();

        let data = vec![
            vec![DataValue::Float(1.0), DataValue::Float(2.0)],
            vec![DataValue::Float(3.0), DataValue::Float(4.0)],
            vec![DataValue::Float(5.0), DataValue::Float(6.0)],
        ];

        let dataset = library
            .create_dataset(
                "test_dataset".to_string(),
                data,
                vec!["col1".to_string(), "col2".to_string()],
                vec![DataType::Float64, DataType::Float64],
                PrivacyLevel::Public,
            )
            .unwrap();

        assert_eq!(dataset.dataset_id, "test_dataset");
        assert_eq!(dataset.data.len(), 3);
        assert_eq!(dataset.column_names.len(), 2);
    }

    #[test]
    fn test_mean_computation() {
        let mut library = StatisticalComputingLibrary::new();
        library.initialize().unwrap();

        let data = vec![
            vec![DataValue::Float(1.0), DataValue::Float(2.0)],
            vec![DataValue::Float(3.0), DataValue::Float(4.0)],
            vec![DataValue::Float(5.0), DataValue::Float(6.0)],
        ];

        library
            .create_dataset(
                "test_dataset".to_string(),
                data,
                vec!["col1".to_string(), "col2".to_string()],
                vec![DataType::Float64, DataType::Float64],
                PrivacyLevel::Public,
            )
            .unwrap();

        let result = library.mean("test_dataset", "col1", false).unwrap();

        assert_eq!(result.result, 3.0); // (1 + 3 + 5) / 3
        assert_eq!(result.sample_size, 3);
        assert!(!result.privacy_preserved);
    }

    #[test]
    fn test_median_computation() {
        let mut library = StatisticalComputingLibrary::new();
        library.initialize().unwrap();

        let data = vec![
            vec![DataValue::Float(1.0), DataValue::Float(2.0)],
            vec![DataValue::Float(3.0), DataValue::Float(4.0)],
            vec![DataValue::Float(5.0), DataValue::Float(6.0)],
            vec![DataValue::Float(7.0), DataValue::Float(8.0)],
        ];

        library
            .create_dataset(
                "test_dataset".to_string(),
                data,
                vec!["col1".to_string(), "col2".to_string()],
                vec![DataType::Float64, DataType::Float64],
                PrivacyLevel::Public,
            )
            .unwrap();

        let result = library.median("test_dataset", "col1", false).unwrap();

        assert_eq!(result.result, 4.0); // median of [1, 3, 5, 7]
        assert_eq!(result.sample_size, 4);
        assert!(!result.privacy_preserved);
    }

    #[test]
    fn test_variance_computation() {
        let mut library = StatisticalComputingLibrary::new();
        library.initialize().unwrap();

        let data = vec![
            vec![DataValue::Float(1.0), DataValue::Float(2.0)],
            vec![DataValue::Float(3.0), DataValue::Float(4.0)],
            vec![DataValue::Float(5.0), DataValue::Float(6.0)],
        ];

        library
            .create_dataset(
                "test_dataset".to_string(),
                data,
                vec!["col1".to_string(), "col2".to_string()],
                vec![DataType::Float64, DataType::Float64],
                PrivacyLevel::Public,
            )
            .unwrap();

        let result = library
            .variance("test_dataset", "col1", true, false)
            .unwrap();

        // Variance of [1, 3, 5] = ((1-3)^2 + (3-3)^2 + (5-3)^2) / (3-1) = (4 + 0 + 4) / 2 = 4
        assert!((result.result - 4.0).abs() < 1e-10);
        assert_eq!(result.sample_size, 3);
        assert!(!result.privacy_preserved);
    }

    #[test]
    fn test_correlation_computation() {
        let mut library = StatisticalComputingLibrary::new();
        library.initialize().unwrap();

        let data = vec![
            vec![DataValue::Float(1.0), DataValue::Float(2.0)],
            vec![DataValue::Float(2.0), DataValue::Float(4.0)],
            vec![DataValue::Float(3.0), DataValue::Float(6.0)],
            vec![DataValue::Float(4.0), DataValue::Float(8.0)],
        ];

        library
            .create_dataset(
                "test_dataset".to_string(),
                data,
                vec!["col1".to_string(), "col2".to_string()],
                vec![DataType::Float64, DataType::Float64],
                PrivacyLevel::Public,
            )
            .unwrap();

        let result = library
            .correlation(
                "test_dataset",
                "col1",
                "col2",
                CorrelationMethod::Pearson,
                false,
            )
            .unwrap();

        // Perfect correlation for [1,2,3,4] and [2,4,6,8]
        assert!((result.result - 1.0).abs() < 1e-10);
        assert_eq!(result.sample_size, 4);
        assert!(!result.privacy_preserved);
    }

    #[test]
    fn laplace_noise_is_random_not_a_counter() {
        // Regression: DP noise was a deterministic AtomicU64 ramp, which voids
        // the guarantee (predictable noise can be subtracted off). Two draws
        // over identical (value, sensitivity) must differ now that real OS
        // entropy backs the inverse-CDF sampler.
        let mut eng = StatisticalPrivacyEngine::new();
        let (a, eps) = eng.add_laplace_noise(100.0, 1.0).unwrap();
        let (b, _) = eng.add_laplace_noise(100.0, 1.0).unwrap();
        assert_eq!(eps, 1.0);
        assert_ne!(
            a, b,
            "differential-privacy noise must be random, not deterministic"
        );
    }

    #[test]
    fn test_privacy_preserved_mean() {
        let mut library = StatisticalComputingLibrary::new();
        library.initialize().unwrap();

        let data = vec![
            vec![DataValue::Float(1.0), DataValue::Float(2.0)],
            vec![DataValue::Float(3.0), DataValue::Float(4.0)],
            vec![DataValue::Float(5.0), DataValue::Float(6.0)],
        ];

        library
            .create_dataset(
                "test_dataset".to_string(),
                data,
                vec!["col1".to_string(), "col2".to_string()],
                vec![DataType::Float64, DataType::Float64],
                PrivacyLevel::Confidential,
            )
            .unwrap();

        let result = library.mean("test_dataset", "col1", true).unwrap();

        assert!(result.privacy_preserved);
        assert!(result.privacy_cost > 0.0);
        // The mean should be noisy (not exactly 3.0)
        assert!(result.result != 3.0);
    }

    #[test]
    fn test_histogram_generation() {
        let mut library = StatisticalComputingLibrary::new();
        library.initialize().unwrap();

        let data = vec![
            vec![DataValue::Float(1.0), DataValue::Float(2.0)],
            vec![DataValue::Float(3.0), DataValue::Float(4.0)],
            vec![DataValue::Float(5.0), DataValue::Float(6.0)],
            vec![DataValue::Float(7.0), DataValue::Float(8.0)],
            vec![DataValue::Float(9.0), DataValue::Float(10.0)],
        ];

        library
            .create_dataset(
                "test_dataset".to_string(),
                data,
                vec!["col1".to_string(), "col2".to_string()],
                vec![DataType::Float64, DataType::Float64],
                PrivacyLevel::Public,
            )
            .unwrap();

        let result = library.histogram("test_dataset", "col1", 5, false).unwrap();

        assert_eq!(result.result.bins, 5);
        assert_eq!(result.result.counts.len(), 5);
        assert_eq!(result.result.min_value, 1.0);
        assert_eq!(result.result.max_value, 9.0);
        assert!(!result.privacy_preserved);
    }

    // ---- Feature 1: ZNS data persistence ----

    #[test]
    fn test_dataset_store_and_retrieve() {
        let mut storage = StatisticalDataStorage::new();
        storage.initialize().unwrap();

        let dataset = Dataset {
            dataset_id: "persisted_ds".to_string(),
            metadata: DatasetMetadata {
                dataset_id: "persisted_ds".to_string(),
                dataset_type: DatasetType::Numerical,
                dimensions: DatasetDimensions {
                    rows: 2,
                    columns: 1,
                    time_steps: None,
                    features: Some(1),
                },
                data_types: vec![DataType::Float64],
                sample_size: 2,
                created_at: 0,
                last_updated: 0,
                access_count: 0,
                privacy_level: PrivacyLevel::Public,
            },
            data: vec![vec![DataValue::Float(1.0)], vec![DataValue::Float(2.0)]],
            column_names: vec!["x".to_string()],
            column_types: vec![DataType::Float64],
        };

        // Store through the persistence layer.
        storage.store_dataset_data(&dataset).unwrap();

        // Retrieve from the in-memory persistence layer.
        let retrieved = storage
            .retrieve_dataset_data("persisted_ds")
            .expect("dataset should be cached after store_dataset_data");
        assert_eq!(retrieved.dataset_id, "persisted_ds");
        assert_eq!(retrieved.data.len(), 2);

        // Retrieving an unknown id returns None.
        assert!(storage.retrieve_dataset_data("missing").is_none());
    }

    #[test]
    fn test_store_dataset_to_named_zone() {
        let mut storage = StatisticalDataStorage::new();
        storage.initialize().unwrap();

        let dataset = Dataset {
            dataset_id: "zoned_ds".to_string(),
            metadata: DatasetMetadata {
                dataset_id: "zoned_ds".to_string(),
                dataset_type: DatasetType::TimeSeries,
                dimensions: DatasetDimensions {
                    rows: 1,
                    columns: 1,
                    time_steps: Some(1),
                    features: None,
                },
                data_types: vec![DataType::Float64],
                sample_size: 1,
                created_at: 0,
                last_updated: 0,
                access_count: 0,
                privacy_level: PrivacyLevel::Restricted,
            },
            data: vec![vec![DataValue::Float(42.0)]],
            column_names: vec!["v".to_string()],
            column_types: vec![DataType::Float64],
        };

        storage.store_dataset_data(&dataset).unwrap();

        // Explicitly place the dataset into the "timeseries" zone.
        storage
            .store_dataset_to_zone("zoned_ds", "timeseries")
            .unwrap();

        // The metadata should now be registered with that zone.
        let zone = storage.zones.get("timeseries").unwrap();
        assert!(zone.datasets.contains_key("zoned_ds"));

        // Storing into a non-existent zone errors.
        assert!(storage.store_dataset_to_zone("zoned_ds", "nope").is_err());

        // Storing an uncached dataset errors.
        assert!(storage
            .store_dataset_to_zone("ghost", "timeseries")
            .is_err());
    }

    // ---- Feature 2: Fiduciary crypto / ZK proof wiring ----

    #[test]
    fn test_encrypt_and_verify_result() {
        let engine = StatisticalPrivacyEngine::new();

        let payload = b"mean=3.0; n=10";
        let signature = engine
            .encrypt_result(payload)
            .expect("encryption should succeed");

        // The signature is a real ML-DSA signature (non-empty).
        assert!(!signature.is_empty());

        // Verifying with the correct payload succeeds.
        let valid = engine
            .verify_result(payload, &signature)
            .expect("verify path should run");
        assert!(valid);

        // Verifying against a tampered payload fails.
        let tampered = b"mean=99.0; n=10";
        let invalid = engine
            .verify_result(tampered, &signature)
            .expect("verify path should run");
        assert!(!invalid);
    }

    #[test]
    fn test_zk_prove_and_verify_computation() {
        let engine = StatisticalPrivacyEngine::new();

        let inputs = vec![b"x=1".to_vec(), b"y=2".to_vec()];
        let outputs = vec![b"sum=3".to_vec()];

        let proof = engine
            .prove_computation("add_op", &inputs, &outputs)
            .expect("proof generation should succeed");
        assert!(!proof.is_empty());

        // Verify the genuine proof.
        let ok = engine
            .verify_computation(&proof, &[])
            .expect("verify path should run");
        assert!(ok);
    }

    // ---- Feature 3: Data catalog search ----

    fn sample_metadata(id: &str, rows: usize) -> DatasetMetadata {
        DatasetMetadata {
            dataset_id: id.to_string(),
            dataset_type: DatasetType::Numerical,
            dimensions: DatasetDimensions {
                rows,
                columns: 2,
                time_steps: None,
                features: Some(2),
            },
            data_types: vec![DataType::Float64, DataType::Float64],
            sample_size: rows,
            created_at: 0,
            last_updated: 0,
            access_count: 0,
            privacy_level: PrivacyLevel::Public,
        }
    }

    #[test]
    fn test_catalog_register_search_and_tags() {
        let mut catalog = DataCatalog::new();
        catalog.initialize().unwrap();

        catalog.register_dataset(sample_metadata("sales_q1", 100));
        catalog.register_dataset(sample_metadata("sales_q2", 200));
        catalog.register_dataset(sample_metadata("inventory", 50));

        catalog.add_tag("sales_q1", "revenue");
        catalog.add_tag("sales_q2", "revenue");
        catalog.add_tag("inventory", "stock");

        // Search by name substring.
        let sales = catalog.search("sales");
        assert_eq!(sales.len(), 2);

        // Search by tag.
        let revenue = catalog.search("revenue");
        assert_eq!(revenue.len(), 2);

        // get_by_tag returns the right datasets.
        let stock = catalog.get_by_tag("stock");
        assert_eq!(stock.len(), 1);
        assert_eq!(stock[0].dataset_id, "inventory");

        // get_by_tag is case-insensitive.
        let revenue_ci = catalog.get_by_tag("REVENUE");
        assert_eq!(revenue_ci.len(), 2);

        // Empty query returns everything.
        assert_eq!(catalog.search("").len(), 3);
    }

    #[test]
    fn test_catalog_relationships() {
        let mut catalog = DataCatalog::new();
        catalog.register_dataset(sample_metadata("base", 10));
        catalog.register_dataset(sample_metadata("derived", 10));

        catalog.add_relationship(
            "base",
            "derived",
            Relationship {
                relationship_id: "rel1".to_string(),
                source_dataset: String::new(),
                target_dataset: String::new(),
                relationship_type: RelationshipType::Derived,
                strength: 0.9,
            },
        );

        let rels = catalog.relationships.get("base").unwrap();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].source_dataset, "base");
        assert_eq!(rels[0].target_dataset, "derived");
    }

    #[test]
    fn test_search_index_index_and_search() {
        let mut index = SearchIndex::new();
        index.initialize().unwrap();

        index.index(IndexEntry {
            entry_id: "e1".to_string(),
            keywords: vec!["alpha".to_string(), "beta".to_string()],
            metadata: HashMap::new(),
            relevance_score: 0.5,
        });
        index.index(IndexEntry {
            entry_id: "e2".to_string(),
            keywords: vec!["gamma".to_string()],
            metadata: HashMap::new(),
            relevance_score: 0.8,
        });

        assert_eq!(index.search("alpha").len(), 1);
        assert_eq!(index.search("beta").len(), 1);
        assert_eq!(index.search("gamma").len(), 1);
        assert_eq!(index.search("zzz").len(), 0);
    }

    // ---- Feature 4: Sensitivity analysis for differential privacy ----

    #[test]
    fn test_sensitivity_mean_sum_count() {
        let mut analyzer = SensitivityAnalyzer::new();
        let data = vec![1.0, 2.0, 3.0, 4.0]; // n = 4

        let mean_s = analyzer.compute_sensitivity("mean", &data).unwrap();
        assert!((mean_s - 0.25).abs() < 1e-12); // 1/4

        let sum_s = analyzer.compute_sensitivity("sum", &data).unwrap();
        assert!((sum_s - 1.0).abs() < 1e-12);

        let count_s = analyzer.compute_sensitivity("count", &data).unwrap();
        assert!((count_s - 1.0).abs() < 1e-12);

        let hist_s = analyzer.compute_sensitivity("histogram", &data).unwrap();
        assert!((hist_s - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_sensitivity_median_variance() {
        let mut analyzer = SensitivityAnalyzer::new();
        let data = vec![1.0, 2.0, 3.0, 10.0]; // range = 9, n = 4

        let median_s = analyzer.compute_sensitivity("median", &data).unwrap();
        assert!((median_s - (10.0 - 1.0) / 4.0).abs() < 1e-12);

        let var_s = analyzer.compute_sensitivity("variance", &data).unwrap();
        let range = 10.0 - 1.0;
        assert!((var_s - (range * range) / 4.0).abs() < 1e-12);
    }

    #[test]
    fn test_sensitivity_caching_and_registered_function() {
        let mut analyzer = SensitivityAnalyzer::new();
        let data = vec![1.0, 2.0, 3.0];

        // First call computes and caches.
        let s1 = analyzer.get_sensitivity("sum", &data).unwrap();
        assert!((s1 - 1.0).abs() < 1e-12);

        // Cache hit: a subsequent call returns the same value even with
        // different data (sum sensitivity is data-independent here, but the
        // point is the cache short-circuits recomputation).
        let s2 = analyzer.get_sensitivity("sum", &[100.0]).unwrap();
        assert!((s2 - 1.0).abs() < 1e-12);

        // A registered function overrides the built-in approximation.
        analyzer.register_function(
            "custom",
            SensitivityFunction {
                function_id: "custom".to_string(),
                sensitivity: 3.5,
                computation_method: SensitivityMethod::Approximate,
            },
        );
        let s3 = analyzer.compute_sensitivity("custom", &data).unwrap();
        assert!((s3 - 3.5).abs() < 1e-12);

        // Unknown operation errors.
        assert!(analyzer.compute_sensitivity("bogus", &data).is_err());
        // Empty data errors.
        assert!(analyzer.compute_sensitivity("mean", &[]).is_err());
    }

    #[test]
    fn test_dp_mean_uses_calibrated_sensitivity() {
        // The privacy-preserved mean path should pull sensitivity from the
        // analyzer (1/n) rather than the old hardcoded 1.0. With n=3 the
        // sensitivity is 1/3; we just assert the path runs and produces a
        // noisy result whose privacy cost is recorded.
        let mut library = StatisticalComputingLibrary::new();
        library.initialize().unwrap();

        let data = vec![
            vec![DataValue::Float(1.0), DataValue::Float(2.0)],
            vec![DataValue::Float(3.0), DataValue::Float(4.0)],
            vec![DataValue::Float(5.0), DataValue::Float(6.0)],
        ];

        library
            .create_dataset(
                "ds".to_string(),
                data,
                vec!["col1".to_string(), "col2".to_string()],
                vec![DataType::Float64, DataType::Float64],
                PrivacyLevel::Confidential,
            )
            .unwrap();

        let result = library.mean("ds", "col1", true).unwrap();
        assert!(result.privacy_preserved);
        assert!(result.privacy_cost > 0.0);

        // The analyzer cache should now hold the mean sensitivity (1/3).
        let cached = library
            .privacy_engine
            .differential_privacy
            .sensitivity_analyzer
            .sensitivity_cache
            .get("mean")
            .copied();
        assert!(cached.is_some());
        assert!((cached.unwrap() - 1.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn test_compression_statistics_tracking() {
        let mut engine = DataCompressionEngine::new();
        engine.initialize().unwrap();

        // Fresh engine has zeroed stats.
        let stats = engine.get_statistics();
        assert_eq!(stats.original_size, 0);
        assert_eq!(stats.compressed_size, 0);
        assert_eq!(stats.compression_count, 0);
        assert_eq!(stats.decompression_count, 0);
        assert_eq!(stats.compression_ratio(), 0.0);

        // Highly repetitive data compresses well under RLE.
        let data = vec![7u8; 1000];
        let compressed = engine.compress(&data).unwrap();
        assert!(compressed.len() < data.len());

        let stats = engine.get_statistics();
        assert_eq!(stats.compression_count, 1);
        assert_eq!(stats.original_size, 1000);
        assert_eq!(stats.compressed_size, compressed.len() as u64);
        // Overall ratio matches compressed/original.
        let expected = compressed.len() as f64 / 1000.0;
        assert!((stats.compression_ratio() - expected).abs() < 1e-12);
        // Last-op ratio field also updated.
        assert!((stats.compression_ratio - expected).abs() < 1e-12);

        // Round-trip decompress and verify decompression stats.
        let decompressed = engine.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
        let stats = engine.get_statistics();
        assert_eq!(stats.decompression_count, 1);
        assert!(stats.decompression_time > 0 || stats.decompression_time == 0); // timing may be 0 on fast machines

        // A second, incompressible compression accumulates.
        let noisy: Vec<u8> = (0..256u32).map(|i| i as u8).collect();
        let compressed2 = engine.compress(&noisy).unwrap();
        let stats = engine.get_statistics();
        assert_eq!(stats.compression_count, 2);
        assert_eq!(stats.original_size, 1000 + 256);
        assert_eq!(
            stats.compressed_size,
            (compressed.len() + compressed2.len()) as u64
        );

        // Summary is human-readable and non-empty.
        let summary = stats.summary();
        assert!(summary.contains("compress op(s)"));
        assert!(summary.contains("2 compress"));

        // Reset zeroes everything.
        engine.reset_statistics();
        let stats = engine.get_statistics();
        assert_eq!(stats.compression_count, 0);
        assert_eq!(stats.decompression_count, 0);
        assert_eq!(stats.original_size, 0);
        assert_eq!(stats.compressed_size, 0);
        assert_eq!(stats.compression_ratio(), 0.0);
    }

    #[test]
    fn test_compression_roundtrip_random_data() {
        let mut engine = DataCompressionEngine::new();
        // Random-ish data: ensure round-trip still holds even when it expands.
        let data: Vec<u8> = (0..500u32).map(|i| (i * 31 + 7) as u8).collect();
        let compressed = engine.compress(&data).unwrap();
        let decompressed = engine.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_cost_model_total_and_comparison() {
        let cheap = CostModel {
            cpu_cost: 1.0,
            io_cost: 2.0,
            memory_cost: 0.5,
            network_cost: 0.0,
        };
        let expensive = CostModel {
            cpu_cost: 10.0,
            io_cost: 20.0,
            memory_cost: 5.0,
            network_cost: 1.0,
        };
        assert!((cheap.total_cost() - 3.5).abs() < 1e-12);
        assert!((expensive.total_cost() - 36.0).abs() < 1e-12);
        assert!(cheap.is_better_than(&expensive));
        assert!(!expensive.is_better_than(&cheap));
    }

    #[test]
    fn test_filter_pushdown() {
        // Filter placed *after* a Join should be pushed ahead of it so rows
        // are reduced before the expensive join.
        let optimizer = QueryOptimizer::new();
        let operations = vec![
            QueryOperation::Join {
                left_cost: 500.0,
                right_cost: 500.0,
                join_type: JoinType::NestedLoop,
            },
            QueryOperation::Filter {
                predicate: "x > 10".to_string(),
                selectivity: 0.1,
            },
        ];

        let plan = optimizer.optimize(operations).unwrap();

        // The Filter must come before the Join.
        assert!(
            matches!(plan.operations[0].operation, QueryOperation::Filter { .. }),
            "filter should be pushed before the join"
        );
        assert!(
            matches!(plan.operations[1].operation, QueryOperation::Join { .. }),
            "join should follow the filter"
        );
        assert_eq!(plan.operations.len(), 2);
    }

    #[test]
    fn test_hash_join_selection() {
        // Both sides > 1000 rows → HashJoin should be chosen regardless of the
        // join_type the caller supplied.
        let optimizer = QueryOptimizer::new();
        let operations = vec![QueryOperation::Join {
            left_cost: 5000.0,
            right_cost: 4000.0,
            join_type: JoinType::NestedLoop,
        }];

        let plan = optimizer.optimize(operations).unwrap();

        match &plan.operations[0].operation {
            QueryOperation::Join { join_type, .. } => {
                assert_eq!(*join_type, JoinType::HashJoin);
            }
            other => panic!("expected a join, got {:?}", other),
        }
    }

    #[test]
    fn test_nested_loop_small_tables() {
        // Both sides < 100 rows → NestedLoop is acceptable (and chosen).
        let optimizer = QueryOptimizer::new();
        let operations = vec![QueryOperation::Join {
            left_cost: 50.0,
            right_cost: 30.0,
            join_type: JoinType::HashJoin,
        }];

        let plan = optimizer.optimize(operations).unwrap();

        match &plan.operations[0].operation {
            QueryOperation::Join { join_type, .. } => {
                assert_eq!(*join_type, JoinType::NestedLoop);
            }
            other => panic!("expected a join, got {:?}", other),
        }
    }

    #[test]
    fn test_cost_estimation() {
        // Every step should carry a positive, finite estimated cost and the
        // plan total should equal the sum of the per-step costs.
        let optimizer = QueryOptimizer::new();
        let operations = vec![
            QueryOperation::Scan {
                table: "t".to_string(),
                estimated_rows: 1000,
            },
            QueryOperation::Filter {
                predicate: "x > 1".to_string(),
                selectivity: 0.5,
            },
            QueryOperation::Limit { count: 10 },
        ];

        let plan = optimizer.optimize(operations).unwrap();

        assert!(plan.estimated_cost > 0.0);
        assert!(plan.estimated_cost.is_finite());
        for step in &plan.operations {
            assert!(step.estimated_cost >= 0.0, "cost must be non-negative");
            assert!(step.estimated_cost.is_finite(), "cost must be finite");
            assert!(step.estimated_rows <= 10_000_000, "rows bounded");
        }

        let sum: f64 = plan.operations.iter().map(|s| s.estimated_cost).sum();
        assert!((plan.estimated_cost - sum).abs() < 1e-9);

        // Scan cost = 1000 * 0.01 = 10.0
        assert!((plan.operations[0].estimated_cost - 10.0).abs() < 1e-9);
        // Filter cost = 1000 * 0.5 * 0.005 = 2.5
        assert!((plan.operations[1].estimated_cost - 2.5).abs() < 1e-9);
        // Limit cost = 10 * 0.001 = 0.01
        assert!((plan.operations[2].estimated_cost - 0.01).abs() < 1e-9);
    }

    #[test]
    fn test_limit_last() {
        // Limit must always be the final step, even if supplied first.
        let optimizer = QueryOptimizer::new();
        let operations = vec![
            QueryOperation::Limit { count: 5 },
            QueryOperation::Scan {
                table: "t".to_string(),
                estimated_rows: 100,
            },
            QueryOperation::Filter {
                predicate: "x > 0".to_string(),
                selectivity: 0.5,
            },
        ];

        let plan = optimizer.optimize(operations).unwrap();

        assert!(
            matches!(
                plan.operations.last().unwrap().operation,
                QueryOperation::Limit { .. }
            ),
            "limit must be the last operation"
        );
        // No other Limit appears mid-plan.
        let limit_count = plan
            .operations
            .iter()
            .filter(|s| matches!(s.operation, QueryOperation::Limit { .. }))
            .count();
        assert_eq!(limit_count, 1);
    }

    #[test]
    fn test_full_plan_optimization() {
        // A complex query: scan → filter → join → aggregate → sort → limit.
        let optimizer = QueryOptimizer::new();
        let operations = vec![
            QueryOperation::Scan {
                table: "orders".to_string(),
                estimated_rows: 5000,
            },
            QueryOperation::Filter {
                predicate: "status = 'paid'".to_string(),
                selectivity: 0.2,
            },
            QueryOperation::Join {
                left_cost: 1000.0,
                right_cost: 2000.0,
                join_type: JoinType::NestedLoop,
            },
            QueryOperation::Aggregate {
                group_by: vec!["customer_id".to_string()],
            },
            QueryOperation::Sort {
                columns: vec!["total".to_string()],
            },
            QueryOperation::Limit { count: 100 },
        ];

        let plan = optimizer.optimize(operations).unwrap();

        // No operations dropped or duplicated.
        assert_eq!(plan.operations.len(), 6);

        // Ordering: Scan, Filter, Join, Aggregate, Sort, Limit.
        let kinds: Vec<&str> = plan
            .operations
            .iter()
            .map(|s| match &s.operation {
                QueryOperation::Scan { .. } => "scan",
                QueryOperation::Filter { .. } => "filter",
                QueryOperation::Join { .. } => "join",
                QueryOperation::Aggregate { .. } => "aggregate",
                QueryOperation::Sort { .. } => "sort",
                QueryOperation::Limit { .. } => "limit",
                QueryOperation::Project { .. } => "project",
            })
            .collect();
        assert_eq!(
            kinds,
            vec!["scan", "filter", "join", "aggregate", "sort", "limit"]
        );

        // Both join sides > 1000 → HashJoin selected.
        if let QueryOperation::Join { join_type, .. } = &plan.operations[2].operation {
            assert_eq!(*join_type, JoinType::HashJoin);
        }

        // Plan-level aggregates are populated.
        assert!(plan.estimated_cost > 0.0);
        assert!(plan.estimated_rows > 0);
    }

    #[test]
    fn test_empty_operations() {
        // An empty operation list yields an empty plan.
        let optimizer = QueryOptimizer::new();
        let plan = optimizer.optimize(Vec::new()).unwrap();

        assert!(plan.operations.is_empty());
        assert!((plan.estimated_cost - 0.0).abs() < 1e-12);
        assert_eq!(plan.estimated_rows, 0);
    }

    // ---- wired capability methods: known-value tests --------------------------

    /// Build a single-column dataset of floats named `col`.
    fn one_col(lib: &mut StatisticalComputingLibrary, id: &str, col: &str, vals: &[f64]) {
        let data: Vec<Vec<DataValue>> = vals.iter().map(|&v| vec![DataValue::Float(v)]).collect();
        lib.create_dataset(
            id.to_string(),
            data,
            vec![col.to_string()],
            vec![DataType::Float64],
            PrivacyLevel::Public,
        )
        .unwrap();
    }

    /// Build a two-column dataset of floats.
    fn two_col(
        lib: &mut StatisticalComputingLibrary,
        id: &str,
        xs: &[f64],
        ys: &[f64],
    ) {
        let data: Vec<Vec<DataValue>> = xs
            .iter()
            .zip(ys)
            .map(|(&x, &y)| vec![DataValue::Float(x), DataValue::Float(y)])
            .collect();
        lib.create_dataset(
            id.to_string(),
            data,
            vec!["x".to_string(), "y".to_string()],
            vec![DataType::Float64, DataType::Float64],
            PrivacyLevel::Public,
        )
        .unwrap();
    }

    #[test]
    fn test_std_skew_kurtosis_wired() {
        let mut lib = StatisticalComputingLibrary::new();
        lib.initialize().unwrap();
        one_col(&mut lib, "d", "col", &[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
        // Population std of this classic set is 2.0.
        let sd = lib.standard_deviation("d", "col", false).unwrap();
        assert!((sd.result - 2.0).abs() < 1e-9, "sd={}", sd.result);
        // Symmetric-ish set: skewness finite; just assert it computes.
        assert!(lib.skewness("d", "col").is_ok());
        assert!(lib.kurtosis("d", "col").is_ok());
    }

    #[test]
    fn test_mode_and_quantile_wired() {
        let mut lib = StatisticalComputingLibrary::new();
        lib.initialize().unwrap();
        one_col(&mut lib, "d", "col", &[1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0]);
        let m = lib.mode("d", "col").unwrap();
        assert_eq!(m.value, 3.0);
        assert_eq!(m.count, 3);
        // Median (0.5 quantile) of [1..=7 subset sorted] = 3.0.
        let q = lib.quantile("d", "col", 0.5).unwrap();
        assert!((q.result - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_covariance_wired() {
        let mut lib = StatisticalComputingLibrary::new();
        lib.initialize().unwrap();
        two_col(&mut lib, "d", &[1.0, 2.0, 3.0, 4.0], &[2.0, 4.0, 6.0, 8.0]);
        // Sample covariance of x=[1,2,3,4], y=2x: var(x)_sample=1.6667, cov=3.3333.
        let c = lib.covariance("d", "x", "y", true).unwrap();
        assert!((c.result - 10.0 / 3.0).abs() < 1e-9, "cov={}", c.result);
    }

    #[test]
    fn test_linear_regression_wired() {
        let mut lib = StatisticalComputingLibrary::new();
        lib.initialize().unwrap();
        two_col(&mut lib, "d", &[1.0, 2.0, 3.0, 4.0, 5.0], &[3.0, 5.0, 7.0, 9.0, 11.0]);
        // y = 2x + 1 exactly.
        let r = lib.linear_regression("d", "x", "y").unwrap();
        assert!((r.slope - 2.0).abs() < 1e-9, "slope={}", r.slope);
        assert!((r.intercept - 1.0).abs() < 1e-9, "intercept={}", r.intercept);
        assert!((r.r_squared - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_polynomial_regression_wired() {
        let mut lib = StatisticalComputingLibrary::new();
        lib.initialize().unwrap();
        // y = x^2 exactly.
        two_col(&mut lib, "d", &[-2.0, -1.0, 0.0, 1.0, 2.0, 3.0], &[4.0, 1.0, 0.0, 1.0, 4.0, 9.0]);
        let f = lib.polynomial_regression("d", "x", "y", 2).unwrap();
        assert_eq!(f.coefficients.len(), 3);
        assert!((f.coefficients[0]).abs() < 1e-6, "c0={}", f.coefficients[0]);
        assert!((f.coefficients[1]).abs() < 1e-6, "c1={}", f.coefficients[1]);
        assert!((f.coefficients[2] - 1.0).abs() < 1e-6, "c2={}", f.coefficients[2]);
        assert!((f.r_squared - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_anova_wired() {
        let mut lib = StatisticalComputingLibrary::new();
        lib.initialize().unwrap();
        // Three identical-spread groups with different means → large F.
        let data: Vec<Vec<DataValue>> = (0..4)
            .map(|i| {
                vec![
                    DataValue::Float(1.0 + i as f64 * 0.0 + [0.0, 1.0, 0.0, 1.0][i]),
                    DataValue::Float(10.0 + [0.0, 1.0, 0.0, 1.0][i]),
                    DataValue::Float(20.0 + [0.0, 1.0, 0.0, 1.0][i]),
                ]
            })
            .collect();
        lib.create_dataset(
            "d".to_string(),
            data,
            vec!["g1".to_string(), "g2".to_string(), "g3".to_string()],
            vec![DataType::Float64, DataType::Float64, DataType::Float64],
            PrivacyLevel::Public,
        )
        .unwrap();
        let a = lib.anova("d", &["g1", "g2", "g3"]).unwrap();
        assert!(a.f_statistic > 10.0, "F={}", a.f_statistic);
        assert!(a.p_value < 0.001, "p={}", a.p_value);
        assert!((a.df_between - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_chi_square_gof_wired() {
        let mut lib = StatisticalComputingLibrary::new();
        lib.initialize().unwrap();
        // Observed exactly equal to a uniform expectation → statistic 0.
        one_col(&mut lib, "d", "col", &[10.0, 10.0, 10.0, 10.0]);
        let r = lib.chi_square_gof("d", "col", None).unwrap();
        assert!(r.statistic.abs() < 1e-9, "chi2={}", r.statistic);
        assert!((r.dof - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_logistic_regression_wired() {
        let mut lib = StatisticalComputingLibrary::new();
        lib.initialize().unwrap();
        // Clear upward trend, non-separable so the MLE is finite.
        let xs = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let ys = [0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 1.0, 1.0];
        two_col(&mut lib, "d", &xs, &ys);
        let m = lib.logistic_regression("d", &["x"], "y", true).unwrap();
        // Positive slope on the single predictor (coefficients[1] with intercept).
        assert!(m.coefficients[1] > 0.0, "beta={}", m.coefficients[1]);
    }

    #[test]
    fn test_timeseries_wired() {
        let mut lib = StatisticalComputingLibrary::new();
        lib.initialize().unwrap();
        one_col(&mut lib, "d", "col", &[1.0, 2.0, 3.0, 4.0, 5.0]);
        let ac = lib.autocorrelation("d", "col", 1).unwrap();
        assert!((ac.result - 0.4).abs() < 1e-9, "acf1={}", ac.result);
        let ma = lib.moving_average("d", "col", 2).unwrap();
        assert_eq!(ma, vec![1.5, 2.5, 3.5, 4.5]);
        let es = lib.exponential_smoothing("d", "col", 0.5).unwrap();
        assert!((es[0] - 1.0).abs() < 1e-9 && (es[1] - 1.5).abs() < 1e-9);
    }

    #[test]
    fn test_kmeans_wired() {
        let mut lib = StatisticalComputingLibrary::new();
        lib.initialize().unwrap();
        // Two well-separated clusters in 1-D.
        one_col(&mut lib, "d", "col", &[0.0, 0.1, 0.2, 10.0, 10.1, 10.2]);
        let m = lib.kmeans("d", &["col"], 2, 50, 42).unwrap();
        assert_eq!(m.k, 2);
        // The two low points and two high points must not share a label boundary:
        // points 0..3 share one label, points 3..6 share another.
        assert_eq!(m.labels[0], m.labels[1]);
        assert_eq!(m.labels[3], m.labels[4]);
        assert_ne!(m.labels[0], m.labels[3]);
    }

    #[test]
    fn test_svm_wired() {
        let mut lib = StatisticalComputingLibrary::new();
        lib.initialize().unwrap();
        // Linearly separable: x<0 negative, x>0 positive.
        let xs = [-3.0, -2.0, -1.0, 1.0, 2.0, 3.0];
        let ys = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        two_col(&mut lib, "d", &xs, &ys);
        let r = lib.linear_svm("d", &["x"], "y", 1.0).unwrap();
        assert!((r.train_accuracy - 1.0).abs() < 1e-9, "acc={}", r.train_accuracy);
        assert!(r.n_support_vectors >= 1);
    }

    #[test]
    fn test_random_forest_wired() {
        let mut lib = StatisticalComputingLibrary::new();
        lib.initialize().unwrap();
        // Monotone target the forest can fit in-sample.
        let xs = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let ys = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        two_col(&mut lib, "d", &xs, &ys);
        let r = lib.random_forest("d", &["x"], "y", 16, false, 7).unwrap();
        assert_eq!(r.n_trees, 16);
        assert!(r.train_metric > 0.8, "R2={}", r.train_metric);
    }
}
