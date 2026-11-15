//! QAC Spatial DSP, Higher-Order Ambisonics (HOA), Wigner-D Spherical Harmonic Rotation,
//! Binaural HRIR/RIR Rendering, ITU-R BS.1770-5 Gated Loudness, and Modal/Perceptual Bridges
//! (`audio::qac::spatial_dsp`).
//!
//! Implements numerical providers for QAC v1 Modes `0x0102` (HOA), `0x0103` (HRIR / ray RIR),
//! `0x0104` (Modal resonance), and `0x0105` (Perceptual filterbank features) under the C02
//! scientific and numerical acceptance contract (`quantities-models.md`) and C03 wire freeze
//! (`wire-freeze.md`), reusing `etalon_signal` allocation-free DSP primitives.

pub mod binaural_hrtf;
pub mod bs1770_loudness;
pub mod hoa_acn_sn3d;
pub mod modal_perceptual_bridge;
pub mod wigner_rotation;

pub use binaural_hrtf::*;
pub use bs1770_loudness::*;
pub use hoa_acn_sn3d::*;
pub use modal_perceptual_bridge::*;
pub use wigner_rotation::*;

#[cfg(test)]
mod tests;
