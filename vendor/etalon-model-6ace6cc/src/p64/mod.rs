//! Borrowed P64 v4 containers with explicit integrity policy.
//!
//! Parsing validates the legacy structural and CRC contract. Schedule contents,
//! alternate blobs, dtype/shape products and manifold meaning are not validated.
//! Raw bytes remain authoritative, including unknown fields and padding.
//! Environment policy, GGUF synthesis and semantic manifold checks belong to callers.

pub mod build;
pub mod integrity;
pub mod layout;
mod reader;
mod views;

pub use integrity::IntegrityMode;
pub use layout::*;
pub use reader::{P64Error, P64OverflowContext, P64View};
pub use views::{P64ManifoldRecord, P64TensorView};

#[cfg(test)]
mod tests;

#[cfg(feature = "inference-model")]
pub mod convert;
