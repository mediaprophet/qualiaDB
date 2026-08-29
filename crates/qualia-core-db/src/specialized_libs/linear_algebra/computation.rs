use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::core_types::*;
use super::optimization::*;
use super::privacy::*;

/// Computation engine for matrix operations
pub struct ComputationEngine {
    pub operation_queue: Vec<MatrixOperation>,
    pub execution_engine: ExecutionEngine,
    pub parallel_executor: ParallelExecutor,
    pub simd_optimizer: SIMDOptimizer,
    pub privacy: PrivacyEngine,
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
    pub engine_type: ExecutionEngineType,
    pub computation_units: Vec<ComputationUnit>,
    pub scheduler: OperationScheduler,
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
    pub thread_pool: Vec<WorkerThread>,
    pub task_queue: Vec<MatrixTask>,
    pub load_balancer: LoadBalancer,
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

/// A unit of parallelisable work submitted to [`ParallelExecutor::execute_parallel`].
///
/// This is the logical-task abstraction used by the parallel executor; it is
/// distinct from [`MatrixTask`] (which carries a concrete [`MatrixOperation`]).
/// A `ParallelTask` describes *what* to run, not *where* — the load balancer
/// decides the worker assignment.
#[derive(Debug, Clone)]
pub struct ParallelTask {
    /// Stable, caller-supplied task identifier.
    pub task_id: usize,
    /// The kind of operation this task represents.
    pub operation: ParallelOperation,
    /// Estimated work units (e.g. microseconds). Used for accounting only;
    /// no real scheduling decision currently depends on it.
    pub estimated_work: u64,
    /// Relative priority of the task.
    pub priority: TaskPriority,
}

/// The operation a [`ParallelTask`] performs. Mirrors the subset of
/// [`MatrixOperation`] variants that are meaningful to dispatch in parallel,
/// plus a `Custom` escape hatch.
#[derive(Debug, Clone, PartialEq)]
pub enum ParallelOperation {
    MatrixMultiply,
    MatrixAdd,
    MatrixSubtract,
    MatrixTranspose,
    MatrixInverse,
    Decomposition(DecompositionType),
    EigenvalueComputation,
    SolveLinearSystem,
    Custom(String),
}

/// Status of a completed (or failed) parallel task.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    /// The task ran to completion.
    Completed,
    /// The task failed; the string carries a diagnostic.
    Failed(String),
    /// The task is still pending (not yet dispatched).
    Pending,
}

/// The result of executing a [`ParallelTask`] via [`ParallelExecutor`].
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// Identifier of the task this result corresponds to.
    pub task_id: usize,
    /// Index of the logical worker the task was assigned to.
    pub worker_index: usize,
    /// Final status of the task.
    pub status: TaskStatus,
    /// Recorded execution time (currently the task's estimated work).
    pub execution_time: u64,
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
///
/// Distributes tasks across a fixed pool of logical workers according to a
/// [`LoadBalancingStrategy`]. This is purely logical scheduling — no real
/// threads are spawned (actual parallelism is a Phase 2 concern).
#[derive(Debug, Clone)]
pub struct LoadBalancer {
    pub balancing_strategy: BalancingStrategy,
    pub worker_metrics: HashMap<String, WorkerMetrics>,
    /// Strategy used by [`LoadBalancer::assign_task`] to schedule tasks.
    pub scheduling_strategy: LoadBalancingStrategy,
    /// Number of logical workers to distribute tasks across.
    pub num_workers: usize,
    /// Total number of tasks in the current batch (used by `Static` partitioning).
    pub total_tasks: usize,
    /// Round-robin cursor: index of the next worker to assign (used by `RoundRobin`).
    pub next_worker: usize,
    /// Pending task count per worker (used by `WorkStealing`).
    pub pending_tasks: Vec<usize>,
}

/// Balancing strategies
#[derive(Debug, Clone, PartialEq)]
pub enum BalancingStrategy {
    RoundRobin,
    LoadBased,
    PerformanceBased,
    Adaptive,
}

/// Load-balancing strategy for distributing parallel tasks across workers.
///
/// This is a logical scheduling policy — no real threads are spawned.
#[derive(Debug, Clone, PartialEq)]
pub enum LoadBalancingStrategy {
    /// Cycle through workers in fixed order (task i → worker i mod N).
    RoundRobin,
    /// Assign each task to the worker with the fewest pending tasks (a simple
    /// work-stealing approximation).
    WorkStealing,
    /// Pre-compute an equal contiguous partition of tasks across workers.
    Static,
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
    pub simd_capabilities: SIMDCapabilities,
    pub optimization_level: OptimizationLevel,
    pub vectorized_operations: HashMap<String, VectorizedOperation>,
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
    SSE2,
    SSE4_1,
    SSE4_2,
    AVX,
    AVX2,
    AVX512,
    FMA,
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

impl ComputationEngine {
    pub fn new() -> Self {
        Self {
            operation_queue: Vec::new(),
            execution_engine: ExecutionEngine::new(),
            parallel_executor: ParallelExecutor::new(4),
            simd_optimizer: SIMDOptimizer::new(),
            privacy: PrivacyEngine::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        self.execution_engine.initialize()?;
        self.parallel_executor.initialize()?;
        self.simd_optimizer.initialize()?;
        self.privacy.initialize()?;
        Ok(())
    }

    pub fn execute_multiplication(
        &mut self,
        operation: &OptimizedMultiplication,
        alpha: f64,
        beta: f64,
    ) -> Result<Vec<f64>, LinearAlgebraError> {
        // Composition boundary: marshal the domain matrices into a caller-owned
        // buffer and call the engine's canonical dynamic GEMM. No inline math here.
        let m = operation.left.rows;
        let n = operation.right.cols;
        let k = operation.left.cols;

        // C := alpha·A·B + beta·C, row-major. `result` is freshly zeroed, so it is
        // the accumulator C; beta·0 = 0 (this entry point always produces a fresh
        // product — there is no prior C to accumulate into). Routing here also fixes
        // the old inline loop, which applied beta to the just-computed product
        // (yielding alpha·AB·(1+beta) instead of the BLAS alpha·AB + beta·C).
        let mut result = vec![0.0; m * n];
        crate::solvers::linear_algebra::gemm::gemm(
            crate::solvers::linear_algebra::gemm::Transpose::No,
            crate::solvers::linear_algebra::gemm::Transpose::No,
            m,
            n,
            k,
            alpha,
            &operation.left.data,
            &operation.right.data,
            beta,
            &mut result,
        )
        .map_err(|_| {
            LinearAlgebraError::InvalidDimensions(
                "matrix dimensions incompatible for multiplication".to_string(),
            )
        })?;

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
    /// Create a parallel executor with `num_workers` logical workers.
    ///
    /// The default load-balancing strategy is [`LoadBalancingStrategy::RoundRobin`].
    /// No OS threads are spawned — workers are logical slots the load balancer
    /// assigns tasks to. `num_workers == 0` is clamped to 1 so that task
    /// assignment always has a valid target.
    pub fn new(num_workers: usize) -> Self {
        let workers = num_workers.max(1);
        let thread_pool = (0..workers)
            .map(|i| WorkerThread {
                thread_id: format!("worker-{i}"),
                current_task: None,
                performance: ThreadPerformance {
                    tasks_completed: 0,
                    average_execution_time: 0.0,
                    cache_hit_rate: 0.0,
                    efficiency: 1.0,
                },
            })
            .collect();
        Self {
            thread_pool,
            task_queue: Vec::new(),
            load_balancer: LoadBalancer::new(LoadBalancingStrategy::RoundRobin)
                .with_workers(workers),
        }
    }

    /// Returns the number of logical workers in the pool.
    pub fn num_workers(&self) -> usize {
        self.thread_pool.len()
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        Ok(())
    }

    /// Distribute `tasks` across the worker pool using the load balancer and
    /// collect the results.
    ///
    /// Tasks are dispatched in input order and results are returned in the same
    /// order (one [`TaskResult`] per input task). Execution is logical: each
    /// task is marked [`TaskStatus::Completed`] with its estimated work recorded
    /// as the execution time. No real threads are spawned.
    ///
    /// An empty task list yields an empty result list (no error).
    pub fn execute_parallel(
        &self,
        tasks: &[ParallelTask],
    ) -> Result<Vec<TaskResult>, LinearAlgebraError> {
        if tasks.is_empty() {
            return Ok(Vec::new());
        }

        // Clone the balancer so a shared `&self` executor does not mutate its
        // own scheduling state across calls. Each batch starts from a clean slate.
        let mut balancer = self.load_balancer.clone();
        balancer.prepare(tasks.len());

        let mut results = Vec::with_capacity(tasks.len());
        for task in tasks {
            let worker_index = balancer.assign_task(task.task_id);
            results.push(TaskResult {
                task_id: task.task_id,
                worker_index,
                status: TaskStatus::Completed,
                execution_time: task.estimated_work,
            });
        }

        Ok(results)
    }
}

impl LoadBalancer {
    /// Create a load balancer that uses `strategy` to assign tasks.
    ///
    /// The balancer starts with zero workers; call [`LoadBalancer::with_workers`]
    /// (or set [`LoadBalancer::num_workers`] directly) before invoking
    /// [`LoadBalancer::assign_task`].
    pub fn new(strategy: LoadBalancingStrategy) -> Self {
        Self {
            balancing_strategy: BalancingStrategy::LoadBased,
            worker_metrics: HashMap::new(),
            scheduling_strategy: strategy,
            num_workers: 0,
            total_tasks: 0,
            next_worker: 0,
            pending_tasks: Vec::new(),
        }
    }

    /// Builder-style setter for the worker count. Resizes the per-worker pending
    /// task vector to match and resets scheduling cursors.
    pub fn with_workers(mut self, num_workers: usize) -> Self {
        self.set_workers(num_workers);
        self
    }

    /// Set the worker count, resizing the per-worker pending task vector and
    /// resetting scheduling cursors.
    pub fn set_workers(&mut self, num_workers: usize) {
        self.num_workers = num_workers;
        self.pending_tasks = vec![0; num_workers];
        self.next_worker = 0;
    }

    /// Prepare the balancer for a fresh batch of `total_tasks` tasks.
    ///
    /// Resets the round-robin cursor and per-worker pending counts, and records
    /// the batch size used by [`LoadBalancingStrategy::Static`] partitioning.
    /// [`ParallelExecutor::execute_parallel`] calls this before assigning tasks;
    /// callers using [`LoadBalancer::assign_task`] directly should call it first
    /// (notably for `Static`).
    pub fn prepare(&mut self, total_tasks: usize) {
        self.total_tasks = total_tasks;
        self.next_worker = 0;
        if self.pending_tasks.len() != self.num_workers {
            self.pending_tasks = vec![0; self.num_workers];
        } else {
            for count in self.pending_tasks.iter_mut() {
                *count = 0;
            }
        }
    }

    /// Return the worker index to assign the next task to.
    ///
    /// - [`LoadBalancingStrategy::RoundRobin`]: cycles through workers
    ///   `0,1,…,N-1,0,1,…` regardless of `task_id`.
    /// - [`LoadBalancingStrategy::WorkStealing`]: assigns to the worker with the
    ///   fewest pending tasks (ties broken by lowest index) and increments that
    ///   worker's pending count.
    /// - [`LoadBalancingStrategy::Static`]: contiguous equal partition —
    ///   `worker = task_id * num_workers / total_tasks` (falls back to
    ///   `task_id % num_workers` when `total_tasks` is unset).
    ///
    /// Returns `0` when the balancer has no workers configured.
    pub fn assign_task(&mut self, task_id: usize) -> usize {
        if self.num_workers == 0 {
            return 0;
        }
        match self.scheduling_strategy {
            LoadBalancingStrategy::RoundRobin => {
                let worker = self.next_worker % self.num_workers;
                self.next_worker = (self.next_worker + 1) % self.num_workers;
                worker
            }
            LoadBalancingStrategy::WorkStealing => {
                let worker = (0..self.num_workers)
                    .min_by_key(|&w| (self.pending_tasks[w], w))
                    .expect("num_workers > 0");
                self.pending_tasks[worker] += 1;
                worker
            }
            LoadBalancingStrategy::Static => {
                if self.total_tasks == 0 {
                    task_id % self.num_workers
                } else {
                    (task_id * self.num_workers) / self.total_tasks
                }
            }
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

    /// Probe the running CPU and populate the SIMD capability set.
    ///
    /// On `x86_64` this queries SSE2, SSE4.1, SSE4.2, AVX, AVX2, AVX-512 and
    /// FMA via `std::arch::is_x86_feature_detected!` (CPUID at runtime). On
    /// `aarch64` it reports NEON. On any other architecture the capability set
    /// is left empty (scalar fallback).
    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        self.simd_capabilities = SIMDCapabilities::detect();
        Ok(())
    }

    /// Returns a reference to the runtime-detected SIMD capabilities.
    pub fn capabilities(&self) -> &SIMDCapabilities {
        &self.simd_capabilities
    }

    /// Checks whether a specific SIMD instruction set is available on the
    /// current CPU. Returns `true` when the given instruction was detected
    /// during `initialize()` (or `SIMDCapabilities::detect()`).
    pub fn has_feature(&self, instruction: &SIMDInstruction) -> bool {
        self.simd_capabilities
            .supported_instructions
            .iter()
            .any(|i| i == instruction)
    }
}

impl SIMDCapabilities {
    pub fn new() -> Self {
        Self::detect()
    }

    /// Probe the running CPU for available SIMD instruction sets at runtime.
    ///
    /// The widest detected vector width and its natural alignment are recorded.
    /// On `x86_64` detection uses `std::arch::is_x86_feature_detected!` (CPUID);
    /// on `aarch64` it uses `std::arch::is_aarch64_feature_detected!`. On any
    /// other architecture no SIMD instructions are reported and a scalar
    /// (width 1, alignment 1) configuration is returned.
    pub fn detect() -> Self {
        #[allow(unused_mut)]
        let mut instructions: Vec<SIMDInstruction> = Vec::new();
        let mut vector_width = 0usize;
        let mut alignment = 1usize;

        #[cfg(target_arch = "x86_64")]
        {
            // SSE2 is mandatory in the x86_64 baseline, but probe anyway for
            // uniformity with the rest of the feature set.
            if is_x86_feature_detected!("sse2") {
                instructions.push(SIMDInstruction::SSE2);
                if vector_width < 128 {
                    vector_width = 128;
                    alignment = 16;
                }
            }
            if is_x86_feature_detected!("sse4.1") {
                instructions.push(SIMDInstruction::SSE4_1);
            }
            if is_x86_feature_detected!("sse4.2") {
                instructions.push(SIMDInstruction::SSE4_2);
            }
            if is_x86_feature_detected!("avx") {
                instructions.push(SIMDInstruction::AVX);
                if vector_width < 256 {
                    vector_width = 256;
                    alignment = 32;
                }
            }
            if is_x86_feature_detected!("avx2") {
                instructions.push(SIMDInstruction::AVX2);
            }
            // AVX-512 Foundation; wider 512-bit registers require 64-byte alignment.
            if is_x86_feature_detected!("avx512f") {
                instructions.push(SIMDInstruction::AVX512);
                if vector_width < 512 {
                    vector_width = 512;
                    alignment = 64;
                }
            }
            if is_x86_feature_detected!("fma") {
                instructions.push(SIMDInstruction::FMA);
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            // NEON is mandatory in the ARMv8 baseline; confirm via the runtime
            // probe for consistency with the x86_64 path.
            if std::arch::is_aarch64_feature_detected!("neon") {
                instructions.push(SIMDInstruction::NEON);
                if vector_width < 128 {
                    vector_width = 128;
                    alignment = 16;
                }
            }
        }

        // Scalar fallback for architectures without a recognised SIMD baseline.
        if vector_width == 0 {
            vector_width = 1;
            alignment = 1;
        }

        Self {
            vector_width,
            supported_instructions: instructions,
            alignment_requirements: alignment,
        }
    }
}

impl OperationScheduler {
    pub fn new() -> Self {
        Self {}
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_detection() {
        let mut optimizer = SIMDOptimizer::new();
        optimizer.initialize().expect("initialize should succeed");

        #[cfg(target_arch = "x86_64")]
        {
            // SSE2 is mandatory in the x86_64 baseline, so it must always be
            // detected at runtime.
            assert!(
                optimizer.has_feature(&SIMDInstruction::SSE2),
                "SSE2 should be detected on x86_64"
            );
        }

        #[cfg(target_arch = "aarch64")]
        {
            assert!(
                optimizer.has_feature(&SIMDInstruction::NEON),
                "NEON should be detected on aarch64"
            );
        }

        // On architectures without a recognised SIMD baseline we simply verify
        // that detection completed without panicking; an empty capability set
        // is a valid "no SIMD" result.
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            let _ = optimizer.capabilities();
        }
    }

    #[test]
    fn test_simd_capabilities_report() {
        let mut optimizer = SIMDOptimizer::new();
        optimizer.initialize().expect("initialize should succeed");

        let caps = optimizer.capabilities();
        assert!(caps.vector_width >= 1, "vector width should be at least 1");
        assert!(
            caps.alignment_requirements >= 1,
            "alignment requirements should be at least 1"
        );

        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        {
            assert!(
                !caps.supported_instructions.is_empty(),
                "at least one SIMD instruction should be detected on x86_64/aarch64"
            );
        }
    }

    #[test]
    fn test_simd_feature_check() {
        let mut optimizer = SIMDOptimizer::new();
        optimizer.initialize().expect("initialize should succeed");

        // A known feature detected on this CPU must report as available.
        #[cfg(target_arch = "x86_64")]
        {
            assert!(
                optimizer.has_feature(&SIMDInstruction::SSE2),
                "has_feature should report SSE2 as available on x86_64"
            );
        }

        #[cfg(target_arch = "aarch64")]
        {
            assert!(
                optimizer.has_feature(&SIMDInstruction::NEON),
                "has_feature should report NEON as available on aarch64"
            );
        }

        // An instruction that was never probed must report as unavailable.
        assert!(
            !optimizer.has_feature(&SIMDInstruction::Custom("nonexistent".to_string())),
            "has_feature should report a non-probed custom instruction as unavailable"
        );
    }

    #[test]
    fn test_round_robin_assignment() {
        let mut lb = LoadBalancer::new(LoadBalancingStrategy::RoundRobin).with_workers(2);
        let assignments: Vec<usize> = (0..4).map(|i| lb.assign_task(i)).collect();
        assert_eq!(
            assignments,
            vec![0, 1, 0, 1],
            "round-robin over 2 workers should cycle 0,1,0,1"
        );
    }

    #[test]
    fn test_static_partition() {
        let mut lb = LoadBalancer::new(LoadBalancingStrategy::Static).with_workers(3);
        lb.prepare(6);
        let assignments: Vec<usize> = (0..6).map(|i| lb.assign_task(i)).collect();

        // Each of the 3 workers should receive exactly 2 tasks.
        let mut counts = vec![0usize; 3];
        for &w in &assignments {
            counts[w] += 1;
        }
        assert_eq!(counts, vec![2, 2, 2], "static partition should be balanced");

        // Contiguous blocks: tasks 0,1 → worker 0; 2,3 → worker 1; 4,5 → worker 2.
        assert_eq!(
            assignments,
            vec![0, 0, 1, 1, 2, 2],
            "static partition should assign contiguous blocks"
        );
    }

    #[test]
    fn test_work_stealing() {
        let mut lb = LoadBalancer::new(LoadBalancingStrategy::WorkStealing).with_workers(3);

        // First task: all workers have 0 pending; tie broken by lowest index → 0.
        assert_eq!(lb.assign_task(0), 0);
        // Worker 0 now has 1 pending; workers 1 and 2 have 0 → assign to 1.
        assert_eq!(lb.assign_task(1), 1);
        // Workers 0,1 have 1; worker 2 has 0 → assign to 2.
        assert_eq!(lb.assign_task(2), 2);
        // All have 1 pending; tie broken by lowest index → 0.
        assert_eq!(lb.assign_task(3), 0);

        // Verify the invariant directly: the next task must go to a worker with
        // the minimum pending count.
        let min_pending = *lb.pending_tasks.iter().min().unwrap();
        let next = lb.assign_task(4);
        assert_eq!(
            lb.pending_tasks[next],
            min_pending + 1,
            "work-stealing should assign to the worker with the fewest pending tasks"
        );
    }

    #[test]
    fn test_parallel_execution_collects_results() {
        let executor = ParallelExecutor::new(2);
        let tasks: Vec<ParallelTask> = (0..3)
            .map(|i| ParallelTask {
                task_id: i,
                operation: ParallelOperation::MatrixMultiply,
                estimated_work: 10 + i as u64,
                priority: TaskPriority::Normal,
            })
            .collect();

        let results = executor
            .execute_parallel(&tasks)
            .expect("execution should succeed");

        assert_eq!(
            results.len(),
            3,
            "execute_parallel should return one result per task"
        );
        // Results are returned in input order with matching task ids.
        assert_eq!(
            results.iter().map(|r| r.task_id).collect::<Vec<_>>(),
            vec![0, 1, 2],
            "results should preserve input order"
        );
        // Every task should be marked completed.
        assert!(
            results.iter().all(|r| r.status == TaskStatus::Completed),
            "all tasks should complete"
        );
        // Every result should be assigned to a valid worker index.
        assert!(
            results
                .iter()
                .all(|r| r.worker_index < executor.num_workers()),
            "worker indices must be within the pool"
        );
    }

    #[test]
    fn test_empty_tasks() {
        let executor = ParallelExecutor::new(4);
        let results = executor
            .execute_parallel(&[])
            .expect("empty execution should succeed");
        assert!(
            results.is_empty(),
            "empty task list should yield no results"
        );
    }
}
