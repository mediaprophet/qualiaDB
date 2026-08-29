//! Specialized Libraries Module
//!
//! This module contains high-performance specialized mathematical and scientific libraries
//! that leverage Phase 2 architectural enhancements for unprecedented performance and capabilities.

#[cfg(not(target_arch = "wasm32"))]
pub mod category_theory;
#[cfg(not(target_arch = "wasm32"))]
pub mod chemistry_modeling;
/// Computational economics and finance coordination layer: capability matrices,
/// shared categorical transforms, and native economics kernels. Kept available
/// to WASM because the first layer is metadata + zero-dependency utilities.
pub mod computational_economics;
/// Native computational geometry: robust predicates, topology/graph structures,
/// Q42/10D adapters, and the computational-geometry algorithm families. Unlike the older
/// specialized libraries this module is available to browser/WASM builds.
pub mod computational_geometry;
pub mod computer_vision;
#[cfg(not(target_arch = "wasm32"))]
pub mod constructibility;
#[cfg(not(target_arch = "wasm32"))]
pub mod cryptographic_library;
#[cfg(not(target_arch = "wasm32"))]
pub mod engineering_analysis;
#[cfg(not(target_arch = "wasm32"))]
pub mod financial_modeling;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "portal",
    feature = "wasm-logic",
    feature = "wasm-scientific"
))]
pub mod linear_algebra;
#[cfg(not(target_arch = "wasm32"))]
pub mod machine_learning;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "portal",
    feature = "wasm-logic",
    feature = "wasm-scientific"
))]
pub mod medical_computing;
#[cfg(not(target_arch = "wasm32"))]
pub mod multivar_calculus;
#[cfg(not(target_arch = "wasm32"))]
pub mod physics_simulation;
#[cfg(not(target_arch = "wasm32"))]
pub mod polynomial_algebra;
#[cfg(not(target_arch = "wasm32"))]
pub mod qpu_bridge;
#[cfg(not(target_arch = "wasm32"))]
pub mod quantum_biology;
#[cfg(not(target_arch = "wasm32"))]
pub mod statistical_computing;
pub mod symbolic_algebra;
#[cfg(not(target_arch = "wasm32"))]
pub mod symbolic_assumptions;
#[cfg(not(target_arch = "wasm32"))]
pub mod symbolic_integration;
#[cfg(not(target_arch = "wasm32"))]
pub mod symbolic_limits;
#[cfg(not(target_arch = "wasm32"))]
pub mod symbolic_ode;
#[cfg(not(target_arch = "wasm32"))]
pub mod symbolic_series;
#[cfg(not(target_arch = "wasm32"))]
pub mod symbolic_solve;
#[cfg(not(target_arch = "wasm32"))]
pub mod symbolic_trig;

// Shared zero-heap utilities
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "portal",
    feature = "wasm-logic",
    feature = "wasm-scientific"
))]
pub mod shared;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "portal",
    feature = "wasm-logic",
    feature = "wasm-scientific"
))]
pub use shared::{FixedArray, FixedQueue, FixedStack, RingBuffer};
