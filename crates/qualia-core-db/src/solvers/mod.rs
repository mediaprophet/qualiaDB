//! Zero-Allocation Solver Library
//!
//! This module provides mathematical solvers designed for the #![no_std]
//! zero-allocation environment of Qualia-DB. All solvers operate on
//! fixed-size stack arrays and maintain strict memory constraints.
//!
//! Enabled:
//! - `qpu` — QPU problem formulation + in-process job queue (non-WASM only)
//!
//! Disabled (build errors to fix — broken ExecutionError/SolverState refs):
//! - calculus, linear_algebra, optimization, quantum_optimizers, symbolic_logic

#![no_std]

// QPU integration — uses std + tokio; gated to non-WASM targets.
#[cfg(not(target_arch = "wasm32"))]
pub mod qpu;

pub mod calculus;
pub mod linear_algebra;
pub mod optimization;
pub mod quantum_optimizers;
pub mod symbolic_logic;

pub use calculus::{RungeKutta4Static, ShootingMethodBVP, SimpsonsIntegratorChunked, ODEState, BVPState, IntegralChunk};
pub use linear_algebra::{FixedLanczosEigensolver, StaticLuDecomposition, ConstTensorContractor, Matrix4x4, Vector4, Tensor3x3x3};
pub use optimization::{NelderMeadSimplex, BoundedNewtonRaphson, LevenbergMarquardtStack, OptimizationState, RootFindingState, CurveFitState};
pub use quantum_optimizers::{QAOAAngleOptimizer, SpsaOptimizer, QuantumOptimizerState, QAOAAngles, SpsaGradient};
pub use symbolic_logic::{ForwardChainingDefeasible, BoundedSatSolver, DefeasibleState, SatState};

/// Unified error type for solver operations.
#[derive(Debug, Clone, PartialEq)]
pub enum SolversError {
    CapacityExceeded,
    SingularMatrix,
    InvalidParameters,
    ConvergenceFailed,
    InvalidDimension,
    ComputationError,
    QuantumError(u32),
    OutOfMemory,
    Unsatisfiable,
    BacktrackFailed,
}

/// Result type for solver operations
pub type SolverResult<T> = Result<T, SolversError>;

/// Common solver configuration
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SolverConfig {
    pub max_iterations: u32,
    pub tolerance: f64,
    pub step_size: f64,
    pub verbose: bool,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            max_iterations: 1000,
            tolerance: 1e-6,
            step_size: 0.01,
            verbose: false,
        }
    }
}

/// Common solver state — includes all fields referenced by disabled sub-modules
/// so they can be re-enabled without structural changes.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SolverState {
    pub iteration: u32,
    pub error: f64,
    pub converged: bool,
    /// Classical cost / energy value (optimization, quantum_optimizers)
    pub cost_value: f64,
    /// Satisfiability flag (symbolic_logic) — None = unknown, Some(true/false) = result
    pub satisfiable: Option<bool>,
    /// QPU call counter (quantum_optimizers)
    pub quantum_calls: u32,
    /// Solver-specific packed data
    pub solver_data: [u64; 4],
}

impl Default for SolverState {
    fn default() -> Self {
        Self {
            iteration: 0,
            error: f64::MAX,
            converged: false,
            cost_value: f64::MAX,
            satisfiable: None,
            quantum_calls: 0,
            solver_data: [0; 4],
        }
    }
}
