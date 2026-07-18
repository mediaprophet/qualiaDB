//! Rhythm — spectral-flux onset, autocorrelation tempogram, comb-filter beat
//! tracking, and BPM histogram (AU-RHY). Real implementations replacing the
//! energy-novelty / median-IOI placeholders in `music.rs`.
//!
//! Each leaf module owns exactly one public function over caller-supplied
//! buffers (zero-heap hot path). This module re-exports only.
//!
//! Reuses (does not reimplement): `features::spectral::spectral_flux`,
//! `features::peaks::{autocorrelation, detect_peaks}`.

pub mod beat_tracker;
pub mod bpm_histogram;
pub mod spectral_flux_onset;
pub mod tempogram;

pub use beat_tracker::track_beats;
pub use bpm_histogram::bpm_histogram;
pub use spectral_flux_onset::onset_detection;
pub use tempogram::{bpm_for_bin, tempogram};
