//! Pitch estimation (real YIN / YinFFT + confidence, pitch filter, vibrato).
//!
//! Real fundamental-frequency estimation: difference function →
//! cumulative-mean-normalised difference → absolute-threshold lag selection →
//! parabolic interpolation → f0 + aperiodicity. This replaces the coarse
//! integer-lag argmin previously in `music.rs`. Re-exports only (AU-PITCH-1).

mod estimate;
pub mod yin;
pub mod yin_fft;
pub mod confidence;
pub mod pitch_filter;
pub mod vibrato;

pub use confidence::pitch_confidence;
pub use estimate::PitchEstimate;
pub use pitch_filter::pitch_filter;
pub use vibrato::{detect_vibrato, vibrato_scratch_len, Vibrato};
pub use yin::yin_pitch;
pub use yin_fft::{yin_fft_pitch, yin_fft_scratch_len};
