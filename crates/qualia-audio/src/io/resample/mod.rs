//! Quality resampling (windowed-sinc / polyphase + anti-alias). Re-exports only (AU-IO).
//!
//! Unlike [`crate::resample::resample_linear_mono`] (linear, aliases on
//! downsample), these paths are band-limited to the destination Nyquist:
//!
//! - [`antialias_cutoff`] designs the lowpass cutoff.
//! - [`resample_sinc`] — direct windowed-sinc (any ratio), zero-heap hot path.
//! - [`resample_polyphase`] — exact rational-ratio polyphase, zero-heap.

pub mod anti_alias;
pub mod polyphase;
pub mod windowed_sinc;

pub use anti_alias::antialias_cutoff;
pub use polyphase::resample_polyphase;
pub use windowed_sinc::resample_sinc;
