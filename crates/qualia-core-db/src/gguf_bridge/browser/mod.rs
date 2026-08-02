//! Browser-only inference backends and execution receipts.

#[cfg(target_arch = "wasm32")]
pub(crate) mod webgpu;
