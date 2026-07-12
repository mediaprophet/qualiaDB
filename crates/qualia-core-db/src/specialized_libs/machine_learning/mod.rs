//! Machine Learning Library - Edge AI and Neural Network Computing
//!
//! This module provides high-performance machine learning operations leveraging Phase 2 enhancements:
//! - NVMe Computational Storage (CSD) for hardware-accelerated neural computations
//! - Ambient Sub-Threshold Orchestration for mobile edge AI optimization
//! - Hardware-Sympathetic Storage (ZNS) for zero-copy model storage
//! - Zero-Copy LoRA Multiplexing for efficient model serving
//!
//! This module was split out of the former monolithic `machine_learning.rs` into a
//! subdirectory library (pure code motion; no behaviour, logic, or signature changes).
//! The public surface is re-exported unchanged so every external path
//! (`crate::specialized_libs::machine_learning::<Item>`) resolves exactly as before.

/// Maximum number of token embeddings materialised into `Model.weights` when loading a
/// real GGUF file. The full vocabulary embedding table can be multiple gigabytes, so only
/// a bounded preview is kept in the in-memory `Vec<f64>` (this is not a hot-path module).
pub const GGUF_EMBEDDING_PREVIEW_TOKENS: usize = 256;

mod errors;
mod inference;
mod library;
mod loader;
mod model_manager;
mod monitoring;
mod optimization;
mod training;
mod types;
mod version_control;

pub use errors::*;
pub use types::*;

#[cfg(test)]
mod tests;
