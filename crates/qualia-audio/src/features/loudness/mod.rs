//! Loudness / dynamics: RMS, EBU R128 / BS.1770 loudness, LRA, ReplayGain,
//! crest factor, statistical moments. Re-exports only (AU-LOUD).
//!
//! All hot paths are zero-heap and reuse the shared biquad engine
//! (`crate::features::filters::biquad`) for K-weighting.

pub mod crest_factor;
pub mod dynamic_complexity;
pub mod k_weighting;
pub mod lra;
pub mod moments;
pub mod r128;
pub mod replay_gain;
pub mod rms;

pub use crest_factor::{crest_factor, crest_factor_db};
pub use dynamic_complexity::dynamic_complexity;
pub use k_weighting::k_weighting_coeffs;
pub use lra::loudness_range;
pub use moments::{moments, Moments};
pub use r128::{integrated_lufs, momentary_lufs, short_term_lufs};
pub use replay_gain::{replay_gain_db, replay_gain_scale, REPLAY_GAIN_2_TARGET_LUFS};
pub use rms::{rms, rms_dbfs};
