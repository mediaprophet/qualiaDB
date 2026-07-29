//! Pitch salience function + peaks (melody foundation). Re-exports only (AU-PITCH-2).
//!
//! - [`pitch_salience`]: harmonic salience curve over a cent/bin grid.
//! - [`salient_pitch_peaks`]: per-frame salient pitch candidates (curve peaks).

pub mod salience_function;
pub mod salience_peaks;

pub use salience_function::pitch_salience;
pub use salience_peaks::salient_pitch_peaks;
