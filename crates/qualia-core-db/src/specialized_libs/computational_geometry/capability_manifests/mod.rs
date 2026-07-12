//! P9.4 — qapp/MCP capability manifests: per-op resource limits and
//! backend descriptors (scalar / SIMD / wgpu / CUDA / exact-fallback).
//!
//! ## Design
//!
//! Every registered computational-geometry op exposes a manifest with:
//! - **Backends**: non-empty list of execution backends.
//! - **Determinism class**: `bit-exact` (identical bits across runs) or
//!   `tolerance` (within a stated tolerance).
//! - **Resource limits**: max input size, max output size, max memory.
//! - **GPU fallback**: any op advertising `wgpu` or `cuda` MUST also
//!   advertise a deterministic `cpu` or `wasm` fallback — never GPU-only
//!   for robust topology.
//!
//! ## Reserve-mode budget query
//!
//! A Reserve-mode budget query returns only the backends runnable on the
//! current device (e.g. if no GPU adapter is available, wgpu/CUDA are
//! filtered out).

mod json;
mod query;
mod registry;
mod types;
mod validation;

pub use json::*;
pub use query::*;
pub use registry::*;
pub use types::*;
pub use validation::*;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod zero_heap_tests;
