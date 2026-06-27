//! Specialized Libraries Module
//! 
//! This module contains high-performance specialized mathematical and scientific libraries
//! that leverage Phase 2 architectural enhancements for unprecedented performance and capabilities.

pub mod qpu_bridge;
pub mod linear_algebra;
pub mod symbolic_algebra;
pub mod constructibility;
pub mod multivar_calculus;
pub mod polynomial_algebra;
pub mod symbolic_integration;
pub mod symbolic_series;
pub mod symbolic_limits;
pub mod symbolic_solve;
pub mod symbolic_assumptions;
pub mod symbolic_trig;
pub mod symbolic_ode;
pub mod statistical_computing;
pub mod cryptographic_library;
pub mod physics_simulation;
pub mod machine_learning;
pub mod financial_modeling;
pub mod chemistry_modeling;
pub mod medical_computing;
pub mod engineering_analysis;
pub mod quantum_biology;

// Shared zero-heap utilities
pub mod shared;
pub use shared::{FixedArray, FixedStack, RingBuffer, FixedQueue};
