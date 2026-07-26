//! Ambient Sub-Threshold Orchestration Implementation
//!
//! This module provides ambient sub-threshold orchestration for mobile scientific computing
//! using NNAPI/CoreML integration. Designed for edge optimization and power-efficient processing.

mod manager;
mod monitor;
mod power;
mod scheduler;
mod types;
mod workload;

pub use manager::*;
pub use monitor::*;
pub use power::*;
pub use scheduler::*;
pub use types::*;
pub use workload::*;

#[cfg(test)]
mod tests;
