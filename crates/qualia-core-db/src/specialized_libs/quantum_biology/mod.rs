//! Quantum Biology Library - Quantum-Enhanced Biological Analysis
//!
//! This module provides quantum-enhanced biological analysis capabilities while maintaining
//! strict zero-allocation invariants and 512MB RAM constraints. It acts as a semantic router
//! for quantum computations, leveraging the Bifurcated Compute Fabric.
//!
//! ## Architecture
//! - Orchestrator (Rust/Sentinel): Semantic router for biological entities
//! - Continuous Solver (WebGPU/SIMD): GPU-accelerated quantum approximations
//! - QPU Bridge (Remote API): IBM Quantum API integration via NativeQuantumDft

pub mod entities;
pub mod quantum_state;
pub mod gpu_pipeline;
pub mod qpu_bridge;
pub mod results;
pub mod context;
pub mod orchestrator;

// Re-export main types for convenience
pub use entities::{BiologicalEntity, BiologicalEntityType, QuantumComputationType};
pub use quantum_state::QuantumState;
pub use gpu_pipeline::{QuantumGPUPipeline, GPUComputationState, GPUShaderParams};
pub use qpu_bridge::{QPUBridge, QPUBridgeState, QPUJobParams};
pub use results::{QuantumBiologyResult, QuantumResultType};
pub use context::QuantumBiologyContext;
pub use orchestrator::QuantumBiologyOrchestrator;

// Re-export error type
pub use orchestrator::QuantumBiologyError;