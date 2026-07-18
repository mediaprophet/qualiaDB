//! Forge-native audio kernels (GPU compute with a CPU oracle + parity certify).
//!
//! Each kernel here follows the forge-native template (see
//! [`crate::wgsl_forge::physics::kinematics`]): the WGSL is embedded via `include_str!`
//! (single source of truth), an exact scalar CPU oracle grades it, and a public entry
//! point picks the GPU path when a wgpu adapter is present and otherwise falls back to
//! the CPU floor — so the feature works with or without a GPU.

pub mod mel;

pub use mel::{mel_apply, mel_apply_cpu, mel_apply_forge, MEL_APPLY_ENTRY, MEL_APPLY_WGSL};
