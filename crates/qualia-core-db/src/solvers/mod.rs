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
//!
//! (Note: this module is **not** actually `#![no_std]` — the `qpu` submodule pulls in
//! std + tokio. The individual solver kernels are written to be no-std-compatible, but the
//! attribute only has effect at the crate root, so it is not applied here.)

// QPU integration — uses std + tokio; gated to non-WASM targets.
#[cfg(not(target_arch = "wasm32"))]
pub mod qpu;

pub mod activation;
pub mod attention;
pub mod calculus;
pub mod feed_forward;
pub mod fuzzy_query;
pub mod geometric_algebra;
pub mod graph_match;
pub mod graph_opt;
pub mod grounding;
pub mod learning;
pub mod number_theory;
pub mod linear_algebra;
pub mod ontology_align;
pub mod optimization;
pub mod polynomial;
pub mod rope;
pub mod units;
pub mod quantum_optimizers;
pub mod statistics;
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
    /// Solver-specific packed data:
    /// - solver_data[0]: cost_value (f64 bits)
    /// - solver_data[1]: satisfiable (u64 boolean)
    /// - solver_data[2]: quantum_calls (u32 cast to u64)
    pub solver_data: [u64; 4],
}

impl SolverState {
    pub fn cost_value(&self) -> f64 { f64::from_bits(self.solver_data[0]) }
    pub fn set_cost_value(&mut self, val: f64) { self.solver_data[0] = val.to_bits(); }
    pub fn satisfiable(&self) -> Option<bool> {
        match self.solver_data[1] {
            0 => None,
            1 => Some(false),
            _ => Some(true),
        }
    }
    pub fn set_satisfiable(&mut self, val: Option<bool>) {
        self.solver_data[1] = match val {
            None => 0,
            Some(false) => 1,
            Some(true) => 2,
        };
    }
    pub fn quantum_calls(&self) -> u32 { self.solver_data[2] as u32 }
    pub fn set_quantum_calls(&mut self, val: u32) { self.solver_data[2] = val as u64; }
    pub fn add_quantum_calls(&mut self, val: u32) { self.solver_data[2] += val as u64; }
}

impl Default for SolverState {
    fn default() -> Self {
        Self {
            iteration: 0,
            error: f64::MAX,
            converged: false,
            solver_data: [f64::MAX.to_bits(), 0, 0, 0],
        }
    }
}
