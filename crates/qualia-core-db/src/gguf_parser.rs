//! GGUF Parser Module
//! Re-exports parser-related types from gguf_sharder for modular organization.

// The actual GGUF header parsing and validation logic is in gguf_sharder::GgufTensorIndex
// This module provides a clean interface for parser-related functionality
pub use crate::gguf_sharder::{GgufTensorIndex, GgufHyperparams, GgufTensorInfo};
