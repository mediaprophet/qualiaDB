//! Canonical current .qac APIs. Structural and physical admission are distinct.
mod storage;
pub use storage::{QacStorageError, QacStorageOptions, QacStorageView};
/// Canonical acoustic container extension.
pub const QAC_EXTENSION: &str = "qac";

#[cfg(not(target_arch = "wasm32"))]
pub mod persistence;

/// Typed current signal profiles.
pub mod signal;
pub mod metadata;
pub mod numerical;
pub mod profiles;
pub mod spatial_dsp;

#[cfg(not(target_arch = "wasm32"))]
pub mod production;
#[cfg(not(target_arch = "wasm32"))]
pub mod production_verify;

#[cfg(test)]
mod storage_cutover_tests;
