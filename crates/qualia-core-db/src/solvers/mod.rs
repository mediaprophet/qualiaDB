//! Zero-Allocation Solver Library
//! 
//! This module provides a comprehensive suite of mathematical solvers designed
//! specifically for the #![no_std] zero-allocation environment of Qualia-DB.
//! All solvers operate on fixed-size stack arrays and maintain strict memory
//! constraints while providing advanced computational capabilities.

#![no_std]

pub mod calculus;
pub mod linear_algebra;
pub mod optimization;
pub mod quantum_optimizers;
pub mod symbolic_logic;

pub use calculus::{
    RungeKutta4Static, ShootingMethodBVP, SimpsonsIntegratorChunked,
    ODEState, BVPState, IntegralChunk
};

pub use linear_algebra::{
    FixedLanczosEigensolver, StaticLuDecomposition, ConstTensorContractor,
    Matrix4x4, Vector4, Tensor3x3x3
};

pub use optimization::{
    NelderMeadSimplex, BoundedNewtonRaphson, LevenbergMarquardtStack,
    OptimizationState, RootFindingState, CurveFitState
};

pub use quantum_optimizers::{
    QAOAAngleOptimizer, SpsaOptimizer, QuantumOptimizerState,
    QAOAAngles, SpsaGradient
};

pub use symbolic_logic::{
    ForwardChainingDefeasible, BoundedSatSolver,
    DefeasibleState, SatState
};

use crate::execution_error::ExecutionError;

/// Result type for solver operations
pub type SolverResult<T> = Result<T, ExecutionError>;

/// Common solver configuration
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SolverConfig {
    /// Maximum iterations
    pub max_iterations: u32,
    /// Convergence tolerance
    pub tolerance: f64,
    /// Step size (for applicable solvers)
    pub step_size: f64,
    /// Enable verbose logging
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

/// Common solver state
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SolverState {
    /// Current iteration
    pub iteration: u32,
    /// Current error/residual
    pub error: f64,
    /// Converged flag
    pub converged: bool,
    /// Solver-specific data
    pub solver_data: [u64; 4],
}

impl Default for SolverState {
    fn default() -> Self {
        Self {
            iteration: 0,
            error: f64::MAX,
            converged: false,
            solver_data: [0; 4],
        }
    }
}
