//! Complete QAC v1 (`qac\0\0\0\0\0`) domain profile codecs, metadata registry (`Schemas 200–210`),
//! zero-heap structural views, and legacy Q4AU v2/v3 lossless migration helpers covering:
//!
//! - **Q03**: Continuum acoustics (`mode 0x0000` pressure/velocity, `mode 0x0001` symmetric Cauchy
//!   stress tensor + velocity, explicit `VacuumMask` vs `UnknownMedium`, Yee staggering guards).
//! - **Q05**: Spatial audio & impulse responses (`mode 0x0102` temporal HOA order 0..=3,
//!   `mode 0x0103` HRIR/RIR + lossless `Q4AU v3` SOFA migration, `mode 0x0003` ray contributions).
//! - **Q06**: Modal & perceptual representations (`mode 0x0104` modal resonance bank disambiguated
//!   from opaque `0x0104` preservation, `mode 0x0105` auditory filterbank / BS.1770 LUFS / dB-SPL +
//!   lossless `Q4AU v2` migration).
//! - **Q07**: Tactile & ultrasound (`mode 0x0106` passive haptic contact/force/mechanoreceptor storage
//!   with separate `evaluate_actuator_command_at` authority/expiry gate, `mode 0x0107` ultrasound
//!   beam summaries linked to raw `0x0105` RF/IQ acquisition + array geometry & context-qualified `TI`).

pub mod builder;
pub mod dsp_bridge;
pub mod elastic;
pub mod envelope;
pub mod geometry_time;
pub mod haptic;
pub mod hoa;
pub mod hrir;
pub mod modal;
pub mod numerical_bridge;
pub mod perceptual;
pub mod pressure_velocity;
pub mod ray_rir;
pub mod registry;
pub mod types;
pub mod ultrasound;
pub mod ultrasound_codec;

#[cfg(test)]
mod tests_q03_q05;
#[cfg(test)]
mod tests_q06_q07;

pub use dsp_bridge::*;
pub use elastic::*;
pub use envelope::*;
pub use haptic::*;
pub use hoa::*;
pub use hrir::*;
pub use modal::*;
pub use numerical_bridge::*;
pub use perceptual::*;
pub use pressure_velocity::*;
pub use ray_rir::*;
pub use registry::QacProfileRegistry;
pub use types::*;
pub use ultrasound::*;
