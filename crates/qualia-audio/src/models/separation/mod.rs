//! Source separation (classical masking now; learned = NeedsWeights). Re-exports only (AU-GEN).
//!
//! [`apply_soft_mask`] / [`binary_mask_from_ratio`] are real classical spectral masking and work
//! now over caller-owned buffers. [`separate_learned`] is a fail-closed stub for demucs-class
//! learned separation that always returns [`crate::types::AudioError::BackendUnavailable`] — it
//! never fabricates stems.

pub mod learned;
pub mod mask;

pub use learned::separate_learned;
pub use mask::{apply_soft_mask, binary_mask_from_ratio};
