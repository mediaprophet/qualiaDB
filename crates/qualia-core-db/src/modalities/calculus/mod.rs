//! # Calculus Modality
//!
//! Zero-heap numerical integration and differential equation solving for QualiaDB.
//!
//! ## Architecture
//!
//! This module operates under strict `#![no_std]` constraints:
//! - No heap allocations (no `Vec`, `String`, `Box`)
//! - Stack-bound processing only
//! - Memory-mapped I/O via Host-Core split
//! - SIMD-accelerated chunked processing
//!
//! ## Usage
//!
//! ### Host-Side (std)
//! ```no_run
//! use qualia_core_db::modalities::calculus::host::MmapGridManager;
//!
//! let manager = MmapGridManager::new("grid_data.bin")?;
//! let slice = manager.get_slice();
//! ```
//!
//! ### Core-Side (no_std)
//! ```no_run
//! use qualia_core_db::modalities::calculus::{ContinuousGrid, integrate_simpsons_chunked};
//!
//! let grid = ContinuousGrid::new(slice, 5000)?;
//! let result = integrate_simpsons_chunked(&grid, 0.001);
//! ```
//!
//! ## Submodules
//!
//! - `host`: Host-side I/O management (ZeroCopyStreamer, io_uring, IOCP)
//! - `gpu`: GPU integration (DirectStorage, GPUDirect, WebGPU)

// Numerical core (ContinuousGrid + integration + DMA/SIMD helpers) relocated to
// `solvers::calculus::grid`; re-exported so `modalities::calculus::{ContinuousGrid,
// integrate_*, resolve_aligned_byte_offset, SimdWidth, CalculusError, ...}` keep resolving.
pub use crate::solvers::calculus::grid::*;

// Hardware I/O + dispatch relocated to `crate::platform`; re-exported here so existing
// `modalities::calculus::{host,gpu,hetero_dispatch}` paths keep resolving (facade).
#[cfg(not(target_arch = "wasm32"))]
pub use crate::platform::host;

#[cfg(not(target_arch = "wasm32"))]
pub use crate::platform::gpu;

#[cfg(not(target_arch = "wasm32"))]
pub use crate::platform::hetero_dispatch;

// Numerical solvers relocated to `crate::solvers::calculus`; re-exported here so the
// existing `modalities::calculus::{ode_solver,ode_advanced,tensor_*}` paths and their
// item re-exports keep resolving (facade).
pub use crate::solvers::calculus::ode_solver;
pub use ode_solver::{
    create_ode_step_quin, extract_ode_state, pack_ode_state, BvpSystem, ChaoitonProfile,
    CoupledBoltzmann, ExponentialDecay, HarmonicOscillator, LinearDecayBvp, OdeSystem,
    QuantizationMapper, Rk4Solver, ShootingMethod, StandardModelMasses, StepSizeAnalyzer,
};

pub use crate::solvers::calculus::ode_advanced;
pub use crate::solvers::calculus::tensor_provenance;
pub use ode_advanced::{
    bdf1_step, bdf2_step, hermite_dense_output, integrate_bdf, integrate_symplectic,
    integrate_with_sensitivity, ruth3_step, verlet_step, yoshida4_step, SensitivityResult,
    SymplecticMethod, SymplecticResult,
};
pub use tensor_provenance::{TensorProvenance, TensorState};

pub use crate::solvers::calculus::tensor_integrity;
pub use tensor_integrity::{
    commit_state, integrity_root, lineage_commitment, transformation_commitment, verify_lineage,
    LineageCommitment,
};

#[cfg(not(target_arch = "wasm32"))]
pub use host::{DmaBuffer, IoError, DEFAULT_BUFFER_SIZE, PAGE_SIZE};

#[cfg(not(target_arch = "wasm32"))]
pub use gpu::{GpuError, GpuIntegrator, PlatformGpuIntegrator, WebGpuIntegrator};

#[cfg(not(target_arch = "wasm32"))]
pub use hetero_dispatch::{
    plan_fusion, select_precision, ComputeBackend, HeterogeneousDispatcher, HostCapabilities,
    PowerThermalBudget, Precision, TensorOp, TensorOpKind, ZeroCopyStrategy,
};

// ─── Opcodes ─────────────────────────────────────────────────────────────────────
//
// - `OP_SIMPSONS_INTEGRATION` (0x50): Simpson's rule integration
// - `OP_TRAPEZOIDAL_INTEGRATION` (0x51): Trapezoidal rule
// - `OP_RK4_STEP` (0x52): Runge-Kutta 4th order ODE step
// - `OP_ADAPTIVE_STEP` (0x53): Adaptive step size control
// - `OP_GPU_INTEGRATION` (0x54): GPU-accelerated integration

pub const OP_SIMPSONS_INTEGRATION: u8 = 0x50;
pub const OP_TRAPEZOIDAL_INTEGRATION: u8 = 0x51;
pub const OP_RK4_STEP: u8 = 0x52;
pub const OP_ADAPTIVE_STEP: u8 = 0x53;
pub const OP_GPU_INTEGRATION: u8 = 0x54;
