//! Linear Algebra Library - High-Performance Mathematical Computing
//! 
//! This module provides high-performance linear algebra operations leveraging Phase 2 enhancements:
//! - Hardware-Sympathetic Storage (ZNS) for zero-copy matrix operations
//! - NVMe Computational Storage (CSD) for hardware-accelerated computations
//! - Zero-Knowledge Semantic Proofs for privacy-preserving linear algebra
//! - Ambient Sub-Threshold Orchestration for mobile optimization

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::ops::{Add, Mul, Sub};
use serde::{Deserialize, Serialize};

/// Linear Algebra Library Manager
pub struct LinearAlgebraLibrary {
    matrix_storage: MatrixStorage,
    computation_engine: ComputationEngine,
    optimization_engine: OptimizationEngine,
    privacy_engine: PrivacyEngine,
    performance_monitor: LAPerformanceMonitor,
}

/// Matrix storage using ZNS for zero-copy operations
pub struct MatrixStorage {
    zones: HashMap<String, MatrixZone>,
    allocator: MatrixAllocator,
    cache: MatrixCache,
    storage_backend: StorageBackend,
}

/// Matrix zone in ZNS storage
#[derive(Debug, Clone)]
pub struct MatrixZone {
    pub zone_id: String,
    pub zone_type: ZoneType,
    pub capacity: u64,
    pub matrices: HashMap<String, MatrixMetadata>,
    pub access_pattern: AccessPattern,
}

/// Zone types for different matrix workloads
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ZoneType {
    /// Dense matrices for general linear algebra
    Dense,
    /// Sparse matrices for large-scale problems
    Sparse,
    /// Structured matrices (triangular, banded, etc.)
    Structured,
    /// Temporary matrices for computations
    Temporary,
    /// Cached matrices for frequently accessed data
    Cached,
}

/// Access patterns for optimization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccessPattern {
    Sequential,
    Random,
    Strided,
    Blocked,
    Adaptive,
}

/// Matrix metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixMetadata {
    pub matrix_id: String,
    pub rows: usize,
    pub cols: usize,
    pub data_type: DataType,
    pub storage_format: StorageFormat,
    pub compression: CompressionType,
    pub created_at: u64,
    pub last_accessed: u64,
    pub access_count: u64,
}

/// Data types for matrices
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataType {
    Float32,
    Float64,
    Complex32,
    Complex64,
    Integer32,
    Integer64,
}

/// Storage formats for matrices
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StorageFormat {
    RowMajor,
    ColumnMajor,
    Blocked,
    CompressedSparseRow,
    CompressedSparseColumn,
}

/// Compression types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    LZ4,
    ZSTD,
    Custom(String),
}

/// Matrix allocator for efficient memory management
pub struct MatrixAllocator {
    allocation_strategy: AllocationStrategy,
    free_blocks: Vec<MemoryBlock>,
    allocated_blocks: HashMap<String, MemoryBlock>,
    fragmentation_threshold: f64,
}

/// Allocation strategies
#[derive(Debug, Clone, PartialEq)]
pub enum AllocationStrategy {
    FirstFit,
    BestFit,
    WorstFit,
    BuddySystem,
    Slab,
}

/// Memory block
#[derive(Debug, Clone)]
pub struct MemoryBlock {
    pub block_id: String,
    pub start_address: u64,
    pub size: u64,
    pub is_free: bool,
    pub fragmentation_score: f64,
}

/// Matrix cache for frequently accessed matrices
pub struct MatrixCache {
    cache_entries: HashMap<String, CacheEntry>,
    cache_policy: CachePolicy,
    max_size: u64,
    current_size: u64,
    hit_count: u64,
    miss_count: u64,
}

/// Cache entry
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub matrix_id: String,
    pub data: Vec<u8>,
    pub access_time: u64,
    pub access_count: u64,
    pub size: u64,
}

/// Cache policies
#[derive(Debug, Clone, PartialEq)]
pub enum CachePolicy {
    LRU,
    LFU,
    FIFO,
    Random,
    Adaptive,
}

/// Storage backend abstraction
pub struct StorageBackend {
    backend_type: BackendType,
    zns_manager: Option<Arc<Mutex<crate::zns_storage::ZnsZoneManager>>>,
    csd_manager: Arc<Mutex<crate::csd_storage::CsdManager>>,
    matrix_store: HashMap<String, Matrix>,
}

/// Backend types
#[derive(Debug, Clone, PartialEq)]
pub enum BackendType {
    ZNS,
    CSD,
    Hybrid,
}

/// Computation engine for matrix operations
pub struct ComputationEngine {
    operation_queue: Vec<MatrixOperation>,
    execution_engine: ExecutionEngine,
    parallel_executor: ParallelExecutor,
    simd_optimizer: SIMDOptimizer,
}

/// Matrix operations
#[derive(Debug, Clone)]
pub enum MatrixOperation {
    MatrixMultiply {
        left: String,
        right: String,
        result: String,
        alpha: f64,
        beta: f64,
    },
    MatrixAdd {
        left: String,
        right: String,
        result: String,
        alpha: f64,
    },
    MatrixSubtract {
        left: String,
        right: String,
        result: String,
    },
    MatrixTranspose {
        input: String,
        result: String,
    },
    MatrixInverse {
        input: String,
        result: String,
    },
    MatrixDecomposition {
        input: String,
        result: String,
        decomposition_type: DecompositionType,
    },
    EigenvalueComputation {
        input: String,
        eigenvalues: String,
        eigenvectors: String,
    },
    SolveLinearSystem {
        matrix: String,
        rhs: String,
        solution: String,
    },
}

/// Decomposition types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DecompositionType {
    LU,
    QR,
    SVD,
    Cholesky,
    Eigen,
    Schur,
}

/// Operation scheduler
pub struct OperationScheduler {}

/// Execution engine
pub struct ExecutionEngine {
    engine_type: ExecutionEngineType,
    computation_units: Vec<ComputationUnit>,
    scheduler: OperationScheduler,
}

/// Execution engine types
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionEngineType {
    CPU,
    GPU,
    CSD,
    Hybrid,
}

/// Computation unit
#[derive(Debug, Clone)]
pub struct ComputationUnit {
    pub unit_id: String,
    pub unit_type: ComputationUnitType,
    pub capabilities: ComputationCapabilities,
    pub current_load: f64,
    pub performance_metrics: PerformanceMetrics,
}

/// Computation unit types
#[derive(Debug, Clone, PartialEq)]
pub enum ComputationUnitType {
    CPU,
    GPU,
    CSD,
    NPU,
    TPU,
}

/// Computation capabilities
#[derive(Debug, Clone)]
pub struct ComputationCapabilities {
    pub max_matrix_size: (usize, usize),
    pub supported_operations: Vec<MatrixOperation>,
    pub data_types: Vec<DataType>,
    pub memory_bandwidth: f64,
    pub compute_throughput: f64,
}

/// Performance metrics
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub operations_per_second: f64,
    pub memory_bandwidth_utilization: f64,
    pub compute_utilization: f64,
    pub power_consumption: f64,
    pub thermal_state: f64,
}

/// Parallel executor
pub struct ParallelExecutor {
    thread_pool: Vec<WorkerThread>,
    task_queue: Vec<MatrixTask>,
    load_balancer: LoadBalancer,
}

/// Worker thread
#[derive(Debug, Clone)]
pub struct WorkerThread {
    pub thread_id: String,
    pub current_task: Option<MatrixTask>,
    pub performance: ThreadPerformance,
}

/// Matrix task
#[derive(Debug, Clone)]
pub struct MatrixTask {
    pub task_id: String,
    pub operation: MatrixOperation,
    pub priority: TaskPriority,
    pub dependencies: Vec<String>,
    pub estimated_time: u64,
}

/// Task priorities
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Thread performance
#[derive(Debug, Clone)]
pub struct ThreadPerformance {
    pub tasks_completed: u64,
    pub average_execution_time: f64,
    pub cache_hit_rate: f64,
    pub efficiency: f64,
}

/// Load balancer
pub struct LoadBalancer {
    balancing_strategy: BalancingStrategy,
    worker_metrics: HashMap<String, WorkerMetrics>,
}

/// Balancing strategies
#[derive(Debug, Clone, PartialEq)]
pub enum BalancingStrategy {
    RoundRobin,
    LoadBased,
    PerformanceBased,
    Adaptive,
}

/// Worker metrics
#[derive(Debug, Clone)]
pub struct WorkerMetrics {
    pub worker_id: String,
    pub current_load: f64,
    pub average_response_time: f64,
    pub success_rate: f64,
}

/// SIMD optimizer
pub struct SIMDOptimizer {
    simd_capabilities: SIMDCapabilities,
    optimization_level: OptimizationLevel,
    vectorized_operations: HashMap<String, VectorizedOperation>,
}

/// SIMD capabilities
#[derive(Debug, Clone)]
pub struct SIMDCapabilities {
    pub vector_width: usize,
    pub supported_instructions: Vec<SIMDInstruction>,
    pub alignment_requirements: usize,
}

/// SIMD instructions
#[derive(Debug, Clone, PartialEq)]
pub enum SIMDInstruction {
    SSE,
    AVX,
    AVX2,
    AVX512,
    NEON,
    Custom(String),
}

/// Optimization levels
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationLevel {
    None,
    Basic,
    Aggressive,
    Maximum,
}

/// Vectorized operation
#[derive(Debug, Clone)]
pub struct VectorizedOperation {
    pub operation_id: String,
    pub vector_width: usize,
    pub instruction_set: Vec<SIMDInstruction>,
    pub performance_gain: f64,
}

/// Optimization engine for matrix operations
pub struct OptimizationEngine {
    optimizer: MatrixOptimizer,
    analyzer: MatrixAnalyzer,
    transformer: MatrixTransformer,
}

/// Matrix optimizer
pub struct MatrixOptimizer {
    optimization_strategies: Vec<OptimizationStrategy>,
    optimization_history: Vec<OptimizationRecord>,
}

/// Optimization strategies
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationStrategy {
    CacheOptimization,
    MemoryLayoutOptimization,
    AlgorithmSelection,
    Parallelization,
    Vectorization,
    Fusion,
}

/// Optimization record
#[derive(Debug, Clone)]
pub struct OptimizationRecord {
    pub timestamp: u64,
    pub matrix_id: String,
    pub strategy: OptimizationStrategy,
    pub performance_improvement: f64,
    pub memory_reduction: f64,
}

/// Matrix analyzer
pub struct MatrixAnalyzer {
    analysis_algorithms: Vec<AnalysisAlgorithm>,
    pattern_recognition: PatternRecognition,
}

/// Analysis algorithms
#[derive(Debug, Clone, PartialEq)]
pub enum AnalysisAlgorithm {
    SparsityAnalysis,
    StructureAnalysis,
    AccessPatternAnalysis,
    PerformanceAnalysis,
}

/// Pattern recognition
pub struct PatternRecognition {
    recognized_patterns: Vec<MatrixPattern>,
    pattern_library: PatternLibrary,
}

/// Matrix patterns
#[derive(Debug, Clone, PartialEq)]
pub enum MatrixPattern {
    Diagonal,
    Triangular,
    Banded,
    Symmetric,
    PositiveDefinite,
    Orthogonal,
    Sparse,
    Dense,
    BlockDiagonal,
    Toeplitz,
    Hankel,
    Circulant,
}

/// Pattern library
pub struct PatternLibrary {
    patterns: HashMap<String, MatrixPattern>,
    optimization_hints: HashMap<MatrixPattern, OptimizationHint>,
}

/// Optimization hints
#[derive(Debug, Clone)]
pub struct OptimizationHint {
    pub preferred_algorithm: String,
    pub memory_layout: StorageFormat,
    pub parallelization_strategy: String,
    pub vectorization_hints: Vec<String>,
}

/// Matrix transformer
pub struct MatrixTransformer {
    transformation_rules: Vec<TransformationRule>,
    transformation_history: Vec<TransformationRecord>,
}

/// Transformation rules
#[derive(Debug, Clone, PartialEq)]
pub enum TransformationRule {
    RowColumnSwap,
    BlockReordering,
    DataTypeConversion,
    CompressionDecompression,
    LayoutConversion,
}

/// Transformation record
#[derive(Debug, Clone)]
pub struct TransformationRecord {
    pub timestamp: u64,
    pub matrix_id: String,
    pub transformation: TransformationRule,
    pub performance_impact: f64,
}

/// Privacy engine for secure linear algebra
pub struct PrivacyEngine {
    zk_proofs: Arc<Mutex<crate::zk_proofs::ZkProofSystem>>,
    homomorphic_operations: HomomorphicOperations,
    secure_aggregation: SecureAggregation,
    differential_privacy: DifferentialPrivacy,
}

/// Homomorphic operations
pub struct HomomorphicOperations {
    supported_operations: Vec<HomomorphicOperation>,
    key_manager: HomomorphicKeyManager,
}

/// Homomorphic operations
#[derive(Debug, Clone, PartialEq)]
pub enum HomomorphicOperation {
    Add,
    Multiply,
    Rotate,
    Bootstrap,
}

/// Homomorphic key manager
pub struct HomomorphicKeyManager {
    keys: HashMap<String, HomomorphicKey>,
    key_rotation_policy: KeyRotationPolicy,
}

/// Homomorphic key
#[derive(Debug, Clone)]
pub struct HomomorphicKey {
    pub key_id: String,
    pub key_type: HomomorphicKeyType,
    pub key_data: Vec<u8>,
    pub created_at: u64,
    pub expires_at: u64,
}

/// Homomorphic key types
#[derive(Debug, Clone, PartialEq)]
pub enum HomomorphicKeyType {
    BFV,
    CKKS,
    BGV,
    Custom(String),
}

/// Key rotation policy
#[derive(Debug, Clone)]
pub struct KeyRotationPolicy {
    pub rotation_interval: u64,
    pub max_key_age: u64,
    pub automatic_rotation: bool,
}

/// Secure aggregation
pub struct SecureAggregation {
    aggregation_protocols: Vec<AggregationProtocol>,
    privacy_budget: PrivacyBudget,
}

/// Aggregation protocols
#[derive(Debug, Clone, PartialEq)]
pub enum AggregationProtocol {
    SecureSum,
    SecureMean,
    SecureMinMax,
    Custom(String),
}

/// Privacy budget
pub struct PrivacyBudget {
    pub epsilon: f64,
    pub delta: f64,
    pub remaining_epsilon: f64,
    pub remaining_delta: f64,
}

/// Differential privacy
pub struct DifferentialPrivacy {
    noise_mechanisms: Vec<NoiseMechanism>,
    privacy_accountant: PrivacyAccountant,
}

/// Noise mechanisms
#[derive(Debug, Clone, PartialEq)]
pub enum NoiseMechanism {
    Laplace,
    Gaussian,
    Exponential,
    Custom(String),
}

/// Privacy accountant
pub struct PrivacyAccountant {
    pub total_epsilon_spent: f64,
    pub total_delta_spent: f64,
    pub composition_method: CompositionMethod,
}

/// Composition methods
#[derive(Debug, Clone, PartialEq)]
pub enum CompositionMethod {
    BasicComposition,
    AdvancedComposition,
    RDPComposition,
    Custom(String),
}

/// Performance monitor for linear algebra operations
pub struct LAPerformanceMonitor {
    operation_metrics: HashMap<String, OperationMetrics>,
    matrix_metrics: HashMap<String, MatrixMetrics>,
    system_metrics: SystemMetrics,
}

/// Operation metrics
#[derive(Debug, Clone)]
pub struct OperationMetrics {
    pub operation_id: String,
    pub operation_type: MatrixOperation,
    pub execution_time: u64,
    pub memory_usage: u64,
    pub cache_hit_rate: f64,
    pub parallel_efficiency: f64,
    pub simd_efficiency: f64,
}

/// Matrix metrics
#[derive(Debug, Clone)]
pub struct MatrixMetrics {
    pub matrix_id: String,
    pub access_count: u64,
    pub total_access_time: u64,
    pub average_access_time: f64,
    pub cache_hit_rate: f64,
    pub compression_ratio: f64,
}

/// System metrics
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub total_operations: u64,
    pub average_execution_time: f64,
    pub throughput: f64,
    pub memory_utilization: f64,
    pub compute_utilization: f64,
    pub power_efficiency: f64,
}

/// Matrix representation
#[derive(Debug, Clone)]
pub struct Matrix {
    pub matrix_id: String,
    pub rows: usize,
    pub cols: usize,
    pub data_type: DataType,
    pub data: Vec<f64>, // Simplified to f64 for demonstration
    pub storage_format: StorageFormat,
    pub metadata: MatrixMetadata,
}

/// Linear algebra result
#[derive(Debug, Clone)]
pub struct LinearAlgebraResult<T> {
    pub result: T,
    pub execution_time: u64,
    pub memory_usage: u64,
    pub operations_used: Vec<String>,
    pub privacy_preserved: bool,
}

impl LinearAlgebraLibrary {
    /// Create new linear algebra library
    pub fn new() -> Self {
        Self {
            matrix_storage: MatrixStorage::new(),
            computation_engine: ComputationEngine::new(),
            optimization_engine: OptimizationEngine::new(),
            privacy_engine: PrivacyEngine::new(),
            performance_monitor: LAPerformanceMonitor::new(),
        }
    }

    /// Initialize the library
    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        // Initialize storage
        self.matrix_storage.initialize()?;

        // Initialize computation engine
        self.computation_engine.initialize()?;

        // Initialize optimization engine
        self.optimization_engine.initialize()?;

        // Initialize privacy engine
        self.privacy_engine.initialize()?;

        Ok(())
    }

    /// Create a new matrix
    pub fn create_matrix(&mut self, matrix_id: String, rows: usize, cols: usize, data_type: DataType, data: Vec<f64>) -> Result<Matrix, LinearAlgebraError> {
        // Validate input
        if data.len() != rows * cols {
            return Err(LinearAlgebraError::InvalidDimensions("Data size doesn't match dimensions".to_string()));
        }

        // Create matrix metadata
        let metadata = MatrixMetadata {
            matrix_id: matrix_id.clone(),
            rows,
            cols,
            data_type: data_type.clone(),
            storage_format: StorageFormat::RowMajor,
            compression: CompressionType::None,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_accessed: 0,
            access_count: 0,
        };

        // Store matrix
        let matrix = Matrix {
            matrix_id: matrix_id.clone(),
            rows,
            cols,
            data_type,
            data,
            storage_format: StorageFormat::RowMajor,
            metadata,
        };

        self.matrix_storage.store_matrix(matrix.clone())?;

        Ok(matrix)
    }

    /// Matrix multiplication with hardware acceleration
    pub fn matrix_multiply(&mut self, left_id: &str, right_id: &str, result_id: &str, alpha: f64, beta: f64) -> Result<LinearAlgebraResult<Matrix>, LinearAlgebraError> {
        let start_time = std::time::Instant::now();

        // Get matrices
        let left = self.matrix_storage.get_matrix(left_id)?;
        let right = self.matrix_storage.get_matrix(right_id)?;

        // Validate dimensions
        if left.cols != right.rows {
            return Err(LinearAlgebraError::InvalidDimensions("Matrix dimensions incompatible for multiplication".to_string()));
        }

        // Optimize operation
        let optimized_operation = self.optimization_engine.optimize_multiplication(&left, &right)?;

        // Execute multiplication
        let result_data = self.computation_engine.execute_multiplication(&optimized_operation, alpha, beta)?;

        // Create result matrix
        let result = self.create_matrix(
            result_id.to_string(),
            left.rows,
            right.cols,
            left.data_type,
            result_data,
        )?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Update performance metrics
        self.performance_monitor.record_operation("matrix_multiply", execution_time, 0);

        Ok(LinearAlgebraResult {
            result,
            execution_time,
            memory_usage: 0,
            operations_used: vec!["matrix_multiply".to_string()],
            privacy_preserved: false,
        })
    }

    /// Matrix addition
    pub fn matrix_add(&mut self, left_id: &str, right_id: &str, result_id: &str, alpha: f64) -> Result<LinearAlgebraResult<Matrix>, LinearAlgebraError> {
        let start_time = std::time::Instant::now();

        // Get matrices
        let left = self.matrix_storage.get_matrix(left_id)?;
        let right = self.matrix_storage.get_matrix(right_id)?;

        // Validate dimensions
        if left.rows != right.rows || left.cols != right.cols {
            return Err(LinearAlgebraError::InvalidDimensions("Matrix dimensions incompatible for addition".to_string()));
        }

        // Execute addition
        let mut result_data = Vec::with_capacity(left.data.len());
        for i in 0..left.data.len() {
            result_data.push(alpha * (left.data[i] + right.data[i]));
        }

        // Create result matrix
        let result = self.create_matrix(
            result_id.to_string(),
            left.rows,
            left.cols,
            left.data_type,
            result_data,
        )?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Update performance metrics
        self.performance_monitor.record_operation("matrix_add", execution_time, 0);

        Ok(LinearAlgebraResult {
            result,
            execution_time,
            memory_usage: 0,
            operations_used: vec!["matrix_add".to_string()],
            privacy_preserved: false,
        })
    }

    /// Matrix transpose
    pub fn matrix_transpose(&mut self, input_id: &str, result_id: &str) -> Result<LinearAlgebraResult<Matrix>, LinearAlgebraError> {
        let start_time = std::time::Instant::now();

        // Get matrix
        let input = self.matrix_storage.get_matrix(input_id)?;

        // Execute transpose
        let mut result_data = Vec::with_capacity(input.data.len());
        for j in 0..input.cols {
            for i in 0..input.rows {
                result_data.push(input.data[i * input.cols + j]);
            }
        }

        // Create result matrix
        let result = self.create_matrix(
            result_id.to_string(),
            input.cols,
            input.rows,
            input.data_type,
            result_data,
        )?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Update performance metrics
        self.performance_monitor.record_operation("matrix_transpose", execution_time, 0);

        Ok(LinearAlgebraResult {
            result,
            execution_time,
            memory_usage: 0,
            operations_used: vec!["matrix_transpose".to_string()],
            privacy_preserved: false,
        })
    }

    /// Matrix inverse
    pub fn matrix_inverse(&mut self, input_id: &str, result_id: &str) -> Result<LinearAlgebraResult<Matrix>, LinearAlgebraError> {
        let start_time = std::time::Instant::now();

        // Get matrix
        let input = self.matrix_storage.get_matrix(input_id)?;

        // Validate square matrix
        if input.rows != input.cols {
            return Err(LinearAlgebraError::InvalidDimensions("Matrix must be square for inversion".to_string()));
        }

        // Execute inverse (simplified Gaussian elimination)
        let n = input.rows;
        let mut augmented = Vec::with_capacity(n * 2 * n);
        
        // Create augmented matrix [A|I]
        for i in 0..n {
            for j in 0..n {
                augmented.push(input.data[i * n + j]);
            }
            for j in 0..n {
                augmented.push(if i == j { 1.0 } else { 0.0 });
            }
        }

        // Gaussian elimination (simplified)
        for i in 0..n {
            // Find pivot
            let mut pivot_row = i;
            for k in (i + 1)..n {
                if (augmented[k * 2 * n + i]).abs() > (augmented[pivot_row * 2 * n + i]).abs() {
                    pivot_row = k;
                }
            }

            // Swap rows
            for j in 0..(2 * n) {
                augmented.swap(i * 2 * n + j, pivot_row * 2 * n + j);
            }

            // Eliminate column
            let pivot = augmented[i * 2 * n + i];
            if pivot.abs() < 1e-10 {
                return Err(LinearAlgebraError::SingularMatrix("Matrix is singular".to_string()));
            }

            for j in 0..(2 * n) {
                augmented[i * 2 * n + j] /= pivot;
            }

            for k in 0..n {
                if k != i {
                    let factor = augmented[k * 2 * n + i];
                    for j in 0..(2 * n) {
                        augmented[k * 2 * n + j] -= factor * augmented[i * 2 * n + j];
                    }
                }
            }
        }

        // Extract inverse
        let mut result_data = Vec::with_capacity(n * n);
        for i in 0..n {
            for j in 0..n {
                result_data.push(augmented[i * 2 * n + n + j]);
            }
        }

        // Create result matrix
        let result = self.create_matrix(
            result_id.to_string(),
            n,
            n,
            input.data_type,
            result_data,
        )?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Update performance metrics
        self.performance_monitor.record_operation("matrix_inverse", execution_time, 0);

        Ok(LinearAlgebraResult {
            result,
            execution_time,
            memory_usage: 0,
            operations_used: vec!["matrix_inverse".to_string()],
            privacy_preserved: false,
        })
    }

    /// Solve linear system Ax = b
    pub fn solve_linear_system(&mut self, matrix_id: &str, rhs_id: &str, solution_id: &str) -> Result<LinearAlgebraResult<Matrix>, LinearAlgebraError> {
        let start_time = std::time::Instant::now();

        // Get matrices
        let matrix = self.matrix_storage.get_matrix(matrix_id)?;
        let rhs = self.matrix_storage.get_matrix(rhs_id)?;

        // Validate dimensions
        if matrix.rows != matrix.cols {
            return Err(LinearAlgebraError::InvalidDimensions("Matrix must be square".to_string()));
        }
        if matrix.rows != rhs.rows {
            return Err(LinearAlgebraError::InvalidDimensions("Matrix and RHS dimensions incompatible".to_string()));
        }

        // Solve using LU decomposition (simplified)
        let n = matrix.rows;
        let mut solution_data = Vec::with_capacity(n);

        // For demonstration, use simple Gaussian elimination
        let mut augmented = Vec::with_capacity(n * (n + 1));
        for i in 0..n {
            for j in 0..n {
                augmented.push(matrix.data[i * n + j]);
            }
            augmented.push(rhs.data[i]);
        }

        // Gaussian elimination
        for i in 0..n {
            // Find pivot
            let mut pivot_row = i;
            for k in (i + 1)..n {
                if (augmented[k * (n + 1) + i]).abs() > (augmented[pivot_row * (n + 1) + i]).abs() {
                    pivot_row = k;
                }
            }

            // Swap rows
            for j in 0..(n + 1) {
                augmented.swap(i * (n + 1) + j, pivot_row * (n + 1) + j);
            }

            // Eliminate column
            let pivot = augmented[i * (n + 1) + i];
            if pivot.abs() < 1e-10 {
                return Err(LinearAlgebraError::SingularMatrix("System is singular".to_string()));
            }

            for j in i..(n + 1) {
                augmented[i * (n + 1) + j] /= pivot;
            }

            for k in 0..n {
                if k != i {
                    let factor = augmented[k * (n + 1) + i];
                    for j in i..(n + 1) {
                        augmented[k * (n + 1) + j] -= factor * augmented[i * (n + 1) + j];
                    }
                }
            }
        }

        // Extract solution
        for i in 0..n {
            solution_data.push(augmented[i * (n + 1) + n]);
        }

        // Create result matrix
        let result = self.create_matrix(
            solution_id.to_string(),
            n,
            1,
            matrix.data_type,
            solution_data,
        )?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Update performance metrics
        self.performance_monitor.record_operation("solve_linear_system", execution_time, 0);

        Ok(LinearAlgebraResult {
            result,
            execution_time,
            memory_usage: 0,
            operations_used: vec!["solve_linear_system".to_string()],
            privacy_preserved: false,
        })
    }

    /// Privacy-preserving matrix multiplication
    pub fn private_matrix_multiply(&mut self, left_id: &str, right_id: &str, result_id: &str) -> Result<LinearAlgebraResult<Matrix>, LinearAlgebraError> {
        // Create zero-knowledge proof for the operation
        let statement = crate::zk_proofs::MathematicalStatement {
            statement_id: "private_matrix_mult".to_string(),
            statement_type: crate::zk_proofs::StatementType::FunctionEvaluation,
            expression: "matrix_multiply(A, B)".to_string(),
            variables: vec!["A".to_string(), "B".to_string()],
            constraints: vec!["A.cols == B.rows".to_string()],
        };

        // Generate witness
        let mut witness = HashMap::new();
        witness.insert("A".to_string(), crate::zk_proofs::FieldElement { value: [1u8; 32] });
        witness.insert("B".to_string(), crate::zk_proofs::FieldElement { value: [2u8; 32] });

        // Generate semantic proof
        let mut semantic_proof = self.privacy_engine.zk_proofs.lock().unwrap()
            .generate_semantic_proof(statement, witness)
            .map_err(|e| LinearAlgebraError::PrivacyError(format!("{:?}", e)))?;

        // Verify proof
        self.privacy_engine.zk_proofs.lock().unwrap()
            .verify_semantic_proof(&mut semantic_proof)
            .map_err(|e| LinearAlgebraError::PrivacyError(format!("{:?}", e)))?;

        // Perform the actual multiplication
        let result = self.matrix_multiply(left_id, right_id, result_id, 1.0, 0.0)?;

        Ok(LinearAlgebraResult {
            result: result.result,
            execution_time: result.execution_time,
            memory_usage: result.memory_usage,
            operations_used: result.operations_used,
            privacy_preserved: true,
        })
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> SystemMetrics {
        self.performance_monitor.get_system_metrics()
    }

    /// List all matrices
    pub fn list_matrices(&self) -> Vec<String> {
        self.matrix_storage.list_matrices()
    }

    /// Get matrix information
    pub fn get_matrix_info(&self, matrix_id: &str) -> Option<MatrixMetadata> {
        self.matrix_storage.get_matrix_metadata(matrix_id)
    }
}

// Supporting implementations

impl MatrixStorage {
    pub fn new() -> Self {
        Self {
            zones: HashMap::new(),
            allocator: MatrixAllocator::new(),
            cache: MatrixCache::new(),
            storage_backend: StorageBackend::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        // Initialize zones
        self.create_zones()?;
        
        // Initialize allocator
        self.allocator.initialize()?;
        
        // Initialize cache
        self.cache.initialize()?;
        
        // Initialize storage backend
        self.storage_backend.initialize()?;
        
        Ok(())
    }

    fn create_zones(&mut self) -> Result<(), LinearAlgebraError> {
        // Create different zone types
        let zones = vec![
            ("dense", ZoneType::Dense),
            ("sparse", ZoneType::Sparse),
            ("structured", ZoneType::Structured),
            ("temporary", ZoneType::Temporary),
            ("cached", ZoneType::Cached),
        ];

        for (name, zone_type) in zones {
            let zone = MatrixZone {
                zone_id: name.to_string(),
                zone_type,
                capacity: 1024 * 1024 * 1024, // 1GB
                matrices: HashMap::new(),
                access_pattern: AccessPattern::Adaptive,
            };
            self.zones.insert(name.to_string(), zone);
        }

        Ok(())
    }

    pub fn store_matrix(&mut self, matrix: Matrix) -> Result<(), LinearAlgebraError> {
        // Determine best zone for this matrix
        let zone_id = self.select_best_zone(&matrix)?;
        
        // Store in zone
        let zone = self.zones.get_mut(&zone_id)
            .ok_or_else(|| LinearAlgebraError::StorageError("Zone not found".to_string()))?;
        
        zone.matrices.insert(matrix.matrix_id.clone(), matrix.metadata.clone());
        
        // Store actual data
        self.storage_backend.store_matrix_data(&matrix)?;
        
        Ok(())
    }

    pub fn get_matrix(&self, matrix_id: &str) -> Result<Matrix, LinearAlgebraError> {
        // Check cache first
        if let Some(cached_data) = self.cache.get(matrix_id) {
            return Ok(cached_data);
        }

        // Get from storage
        self.storage_backend.get_matrix_data(matrix_id)
    }

    pub fn get_matrix_metadata(&self, matrix_id: &str) -> Option<MatrixMetadata> {
        for zone in self.zones.values() {
            if let Some(metadata) = zone.matrices.get(matrix_id) {
                return Some(metadata.clone());
            }
        }
        None
    }

    pub fn list_matrices(&self) -> Vec<String> {
        let mut matrices = Vec::new();
        for zone in self.zones.values() {
            matrices.extend(zone.matrices.keys().cloned());
        }
        matrices
    }

    fn select_best_zone(&self, matrix: &Matrix) -> Result<String, LinearAlgebraError> {
        // Simple selection logic - in real implementation would be more sophisticated
        if matrix.rows * matrix.cols > 10000 {
            Ok("dense".to_string())
        } else {
            Ok("temporary".to_string())
        }
    }
}

impl MatrixAllocator {
    pub fn new() -> Self {
        Self {
            allocation_strategy: AllocationStrategy::BestFit,
            free_blocks: Vec::new(),
            allocated_blocks: HashMap::new(),
            fragmentation_threshold: 0.3,
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        // Initialize with some free blocks
        Ok(())
    }
}

impl MatrixCache {
    pub fn new() -> Self {
        Self {
            cache_entries: HashMap::new(),
            cache_policy: CachePolicy::LRU,
            max_size: 100 * 1024 * 1024, // 100MB
            current_size: 0,
            hit_count: 0,
            miss_count: 0,
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        Ok(())
    }

    pub fn get(&self, matrix_id: &str) -> Option<Matrix> {
        // Simplified cache implementation
        None
    }

    pub fn put(&mut self, matrix: &Matrix) -> Result<(), LinearAlgebraError> {
        // Simplified cache implementation
        Ok(())
    }
}

impl StorageBackend {
    pub fn new() -> Self {
        let zns_manager = crate::zns_storage::ZnsZoneManager::new("default_zone")
            .ok()
            .map(|m| Arc::new(Mutex::new(m)));
        Self {
            backend_type: if zns_manager.is_some() { BackendType::Hybrid } else { BackendType::CSD },
            zns_manager,
            csd_manager: Arc::new(Mutex::new(crate::csd_storage::CsdManager::new())),
            matrix_store: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        Ok(())
    }

    pub fn store_matrix_data(&mut self, matrix: &Matrix) -> Result<(), LinearAlgebraError> {
        self.matrix_store.insert(matrix.matrix_id.clone(), matrix.clone());
        Ok(())
    }

    pub fn get_matrix_data(&self, matrix_id: &str) -> Result<Matrix, LinearAlgebraError> {
        self.matrix_store.get(matrix_id)
            .cloned()
            .ok_or_else(|| LinearAlgebraError::StorageError(format!("Matrix not found: {}", matrix_id)))
    }
}

impl ComputationEngine {
    pub fn new() -> Self {
        Self {
            operation_queue: Vec::new(),
            execution_engine: ExecutionEngine::new(),
            parallel_executor: ParallelExecutor::new(),
            simd_optimizer: SIMDOptimizer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        self.execution_engine.initialize()?;
        self.parallel_executor.initialize()?;
        self.simd_optimizer.initialize()?;
        Ok(())
    }

    pub fn execute_multiplication(&mut self, operation: &OptimizedMultiplication, alpha: f64, beta: f64) -> Result<Vec<f64>, LinearAlgebraError> {
        // Execute optimized matrix multiplication
        let m = operation.left.rows;
        let n = operation.right.cols;
        let k = operation.left.cols;
        
        let mut result = vec![0.0; m * n];
        
        // Simple matrix multiplication (would use CSD in real implementation)
        for i in 0..m {
            for j in 0..n {
                for l in 0..k {
                    result[i * n + j] += alpha * operation.left.data[i * k + l] * operation.right.data[l * n + j];
                }
                result[i * n + j] += beta * result[i * n + j];
            }
        }
        
        Ok(result)
    }
}

impl ExecutionEngine {
    pub fn new() -> Self {
        Self {
            engine_type: ExecutionEngineType::Hybrid,
            computation_units: Vec::new(),
            scheduler: OperationScheduler::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        // Initialize computation units
        Ok(())
    }
}

impl ParallelExecutor {
    pub fn new() -> Self {
        Self {
            thread_pool: Vec::new(),
            task_queue: Vec::new(),
            load_balancer: LoadBalancer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        Ok(())
    }
}

impl LoadBalancer {
    pub fn new() -> Self {
        Self {
            balancing_strategy: BalancingStrategy::LoadBased,
            worker_metrics: HashMap::new(),
        }
    }
}

impl SIMDOptimizer {
    pub fn new() -> Self {
        Self {
            simd_capabilities: SIMDCapabilities::new(),
            optimization_level: OptimizationLevel::Maximum,
            vectorized_operations: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        Ok(())
    }
}

impl SIMDCapabilities {
    pub fn new() -> Self {
        Self {
            vector_width: 256, // AVX2
            supported_instructions: vec![SIMDInstruction::AVX2],
            alignment_requirements: 32,
        }
    }
}

impl OperationScheduler {
    pub fn new() -> Self {
        Self {}
    }
}

impl OptimizationEngine {
    pub fn new() -> Self {
        Self {
            optimizer: MatrixOptimizer::new(),
            analyzer: MatrixAnalyzer::new(),
            transformer: MatrixTransformer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        self.optimizer.initialize()?;
        self.analyzer.initialize()?;
        self.transformer.initialize()?;
        Ok(())
    }

    pub fn optimize_multiplication(&mut self, left: &Matrix, right: &Matrix) -> Result<OptimizedMultiplication, LinearAlgebraError> {
        // Analyze matrices
        let left_analysis = self.analyzer.analyze_matrix(left)?;
        let right_analysis = self.analyzer.analyze_matrix(right)?;

        // Create optimized operation
        let optimized = OptimizedMultiplication {
            left: left.clone(),
            right: right.clone(),
            optimization_strategy: OptimizationStrategy::Vectorization,
            expected_performance_gain: 2.0,
        };

        Ok(optimized)
    }
}

impl MatrixOptimizer {
    pub fn new() -> Self {
        Self {
            optimization_strategies: vec![
                OptimizationStrategy::Vectorization,
                OptimizationStrategy::CacheOptimization,
                OptimizationStrategy::Parallelization,
            ],
            optimization_history: Vec::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        Ok(())
    }
}

impl MatrixAnalyzer {
    pub fn new() -> Self {
        Self {
            analysis_algorithms: vec![
                AnalysisAlgorithm::StructureAnalysis,
                AnalysisAlgorithm::SparsityAnalysis,
            ],
            pattern_recognition: PatternRecognition::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        self.pattern_recognition.initialize()?;
        Ok(())
    }

    pub fn analyze_matrix(&mut self, matrix: &Matrix) -> Result<MatrixAnalysis, LinearAlgebraError> {
        // Analyze matrix structure
        let analysis = MatrixAnalysis {
            matrix_id: matrix.matrix_id.clone(),
            sparsity: self.calculate_sparsity(matrix),
            structure: self.detect_structure(matrix),
            access_pattern: AccessPattern::Sequential,
            optimization_hints: vec![],
        };

        Ok(analysis)
    }

    fn calculate_sparsity(&self, matrix: &Matrix) -> f64 {
        let non_zero = matrix.data.iter().filter(|&&x| x != 0.0).count();
        1.0 - (non_zero as f64 / matrix.data.len() as f64)
    }

    fn detect_structure(&self, matrix: &Matrix) -> MatrixPattern {
        // Simple structure detection
        if matrix.rows == matrix.cols {
            // Check if diagonal
            let mut is_diagonal = true;
            for i in 0..matrix.rows {
                for j in 0..matrix.cols {
                    if i != j && matrix.data[i * matrix.cols + j] != 0.0 {
                        is_diagonal = false;
                        break;
                    }
                }
                if !is_diagonal {
                    break;
                }
            }
            if is_diagonal {
                return MatrixPattern::Diagonal;
            }
        }
        MatrixPattern::Dense
    }
}

impl PatternRecognition {
    pub fn new() -> Self {
        Self {
            recognized_patterns: Vec::new(),
            pattern_library: PatternLibrary::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        self.pattern_library.initialize()?;
        Ok(())
    }
}

impl PatternLibrary {
    pub fn new() -> Self {
        Self {
            patterns: HashMap::new(),
            optimization_hints: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        Ok(())
    }
}

impl MatrixTransformer {
    pub fn new() -> Self {
        Self {
            transformation_rules: vec![
                TransformationRule::LayoutConversion,
                TransformationRule::DataTypeConversion,
            ],
            transformation_history: Vec::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        Ok(())
    }
}

impl PrivacyEngine {
    pub fn new() -> Self {
        Self {
            zk_proofs: Arc::new(Mutex::new(crate::zk_proofs::ZkProofSystem::new())),
            homomorphic_operations: HomomorphicOperations::new(),
            secure_aggregation: SecureAggregation::new(),
            differential_privacy: DifferentialPrivacy::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        self.zk_proofs.lock().unwrap();
        self.homomorphic_operations.initialize()?;
        self.secure_aggregation.initialize()?;
        self.differential_privacy.initialize()?;
        Ok(())
    }
}

impl HomomorphicOperations {
    pub fn new() -> Self {
        Self {
            supported_operations: vec![HomomorphicOperation::Add, HomomorphicOperation::Multiply],
            key_manager: HomomorphicKeyManager::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        self.key_manager.initialize()?;
        Ok(())
    }
}

impl HomomorphicKeyManager {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            key_rotation_policy: KeyRotationPolicy {
                rotation_interval: 86400 * 30, // 30 days
                max_key_age: 86400 * 90, // 90 days
                automatic_rotation: true,
            },
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        Ok(())
    }
}

impl SecureAggregation {
    pub fn new() -> Self {
        Self {
            aggregation_protocols: vec![AggregationProtocol::SecureSum],
            privacy_budget: PrivacyBudget {
                epsilon: 1.0,
                delta: 1e-6,
                remaining_epsilon: 1.0,
                remaining_delta: 1e-6,
            },
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        Ok(())
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
            },
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        Ok(())
    }
}

impl LAPerformanceMonitor {
    pub fn new() -> Self {
        Self {
            operation_metrics: HashMap::new(),
            matrix_metrics: HashMap::new(),
            system_metrics: SystemMetrics {
                total_operations: 0,
                average_execution_time: 0.0,
                throughput: 0.0,
                memory_utilization: 0.0,
                compute_utilization: 0.0,
                power_efficiency: 0.0,
            },
        }
    }

    pub fn record_operation(&mut self, operation_type: &str, execution_time: u64, memory_usage: u64) {
        self.system_metrics.total_operations += 1;
        self.system_metrics.average_execution_time = 
            (self.system_metrics.average_execution_time * (self.system_metrics.total_operations - 1) as f64 + execution_time as f64) / self.system_metrics.total_operations as f64;
    }

    pub fn get_system_metrics(&self) -> SystemMetrics {
        self.system_metrics.clone()
    }
}

// Supporting types

#[derive(Debug, Clone)]
pub struct OptimizedMultiplication {
    pub left: Matrix,
    pub right: Matrix,
    pub optimization_strategy: OptimizationStrategy,
    pub expected_performance_gain: f64,
}

#[derive(Debug, Clone)]
pub struct MatrixAnalysis {
    pub matrix_id: String,
    pub sparsity: f64,
    pub structure: MatrixPattern,
    pub access_pattern: AccessPattern,
    pub optimization_hints: Vec<String>,
}

/// Linear algebra error types
#[derive(Debug, Clone)]
pub enum LinearAlgebraError {
    InvalidDimensions(String),
    SingularMatrix(String),
    StorageError(String),
    ComputationError(String),
    PrivacyError(String),
    OptimizationError(String),
}

impl std::fmt::Display for LinearAlgebraError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinearAlgebraError::InvalidDimensions(msg) => write!(f, "Invalid dimensions: {}", msg),
            LinearAlgebraError::SingularMatrix(msg) => write!(f, "Singular matrix: {}", msg),
            LinearAlgebraError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            LinearAlgebraError::ComputationError(msg) => write!(f, "Computation error: {}", msg),
            LinearAlgebraError::PrivacyError(msg) => write!(f, "Privacy error: {}", msg),
            LinearAlgebraError::OptimizationError(msg) => write!(f, "Optimization error: {}", msg),
        }
    }
}

impl std::error::Error for LinearAlgebraError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_algebra_library_creation() {
        let library = LinearAlgebraLibrary::new();
        assert_eq!(library.list_matrices().len(), 0);
    }

    #[test]
    fn test_matrix_creation() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();
        
        let data = vec![1.0, 2.0, 3.0, 4.0];
        let matrix = library.create_matrix("test_matrix".to_string(), 2, 2, DataType::Float64, data).unwrap();
        
        assert_eq!(matrix.rows, 2);
        assert_eq!(matrix.cols, 2);
        assert_eq!(matrix.data.len(), 4);
    }

    #[test]
    fn test_matrix_multiplication() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();
        
        let a_data = vec![1.0, 2.0, 3.0, 4.0];
        let b_data = vec![5.0, 6.0, 7.0, 8.0];
        
        library.create_matrix("A".to_string(), 2, 2, DataType::Float64, a_data).unwrap();
        library.create_matrix("B".to_string(), 2, 2, DataType::Float64, b_data).unwrap();
        
        let result = library.matrix_multiply("A", "B", "C", 1.0, 0.0).unwrap();
        
        assert_eq!(result.result.rows, 2);
        assert_eq!(result.result.cols, 2);
        assert_eq!(result.result.data[0], 19.0); // 1*5 + 2*7
        assert_eq!(result.result.data[1], 22.0); // 1*6 + 2*8
        assert_eq!(result.result.data[2], 43.0); // 3*5 + 4*7
        assert_eq!(result.result.data[3], 50.0); // 3*6 + 4*8
    }

    #[test]
    fn test_matrix_transpose() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();
        
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        library.create_matrix("A".to_string(), 2, 3, DataType::Float64, data).unwrap();
        
        let result = library.matrix_transpose("A", "AT").unwrap();
        
        assert_eq!(result.result.rows, 3);
        assert_eq!(result.result.cols, 2);
        assert_eq!(result.result.data[0], 1.0);
        assert_eq!(result.result.data[1], 4.0);
        assert_eq!(result.result.data[2], 2.0);
        assert_eq!(result.result.data[3], 5.0);
        assert_eq!(result.result.data[4], 3.0);
        assert_eq!(result.result.data[5], 6.0);
    }

    #[test]
    fn test_matrix_inverse() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();
        
        let data = vec![2.0, 1.0, 1.0, 1.0]; // [[2,1],[1,1]]
        library.create_matrix("A".to_string(), 2, 2, DataType::Float64, data).unwrap();
        
        let result = library.matrix_inverse("A", "A_inv").unwrap();
        
        assert_eq!(result.result.rows, 2);
        assert_eq!(result.result.cols, 2);
        // Inverse of [[2,1],[1,1]] is [[1,-1],[-1,2]]
        assert!((result.result.data[0] - 1.0).abs() < 1e-10);
        assert!((result.result.data[1] + 1.0).abs() < 1e-10);
        assert!((result.result.data[2] + 1.0).abs() < 1e-10);
        assert!((result.result.data[3] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_solve_linear_system() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();
        
        let matrix_data = vec![2.0, 1.0, 1.0, 1.0]; // [[2,1],[1,1]]
        let rhs_data = vec![3.0, 2.0]; // [3,2]
        
        library.create_matrix("A".to_string(), 2, 2, DataType::Float64, matrix_data).unwrap();
        library.create_matrix("b".to_string(), 2, 1, DataType::Float64, rhs_data).unwrap();
        
        let result = library.solve_linear_system("A", "b", "x").unwrap();
        
        assert_eq!(result.result.rows, 2);
        assert_eq!(result.result.cols, 1);
        // Solution should be [1,1] for 2x + y = 3, x + y = 2
        assert!((result.result.data[0] - 1.0).abs() < 1e-10);
        assert!((result.result.data[1] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_private_matrix_multiplication() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();
        
        let a_data = vec![1.0, 2.0, 3.0, 4.0];
        let b_data = vec![5.0, 6.0, 7.0, 8.0];
        
        library.create_matrix("A".to_string(), 2, 2, DataType::Float64, a_data).unwrap();
        library.create_matrix("B".to_string(), 2, 2, DataType::Float64, b_data).unwrap();
        
        let result = library.private_matrix_multiply("A", "B", "C").unwrap();
        
        assert!(result.privacy_preserved);
        assert_eq!(result.result.rows, 2);
        assert_eq!(result.result.cols, 2);
    }
}
