//! Voice activity detection: a multi-feature, epistemic **proposal** stack
//! (energy above an adaptive noise floor + spectral flatness + ZCR), with
//! hysteresis/hangover segmentation. Silence is never coerced into speech.
//!
//! Re-exports only (AU-VAD). Zero-heap hot path (caller-buffered / stack scratch).

pub mod frame_vad;
pub mod noise_estimate;
pub mod segmenter;

pub use frame_vad::{frame_is_voiced, frame_voicing_score, MAX_VAD_FFT};
pub use noise_estimate::{noise_floor_min_stat, NOISE_BIAS};
pub use segmenter::segment_voiced;
