//! Ambient Sub-Threshold Orchestration Implementation
//!
//! This module provides ambient sub-threshold orchestration for mobile scientific computing
//! using NNAPI/CoreML integration. Designed for edge optimization and power-efficient processing.

mod types;
mod manager;
mod power;
mod scheduler;
mod workload;
mod monitor;

pub use types::*;
pub use manager::*;
pub use power::*;
pub use scheduler::*;
pub use workload::*;
pub use monitor::*;

#[cfg(test)]
mod tests;
