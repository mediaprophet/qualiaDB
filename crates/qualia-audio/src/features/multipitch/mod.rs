//! Multi-pitch estimation (Klapuri iterative spectral subtraction / multi-Melodia). Re-exports only (AU-MULTIPITCH).
//!
//! - [`multipitch_klapuri`]: iterative spectral-subtraction polyphonic F0 set for
//!   one frame's magnitude spectrum (declared max-polyphony; abstains on low
//!   salience).
//! - [`track_multi_pitch`]: thread per-frame multi-pitch estimates into multiple
//!   concurrent pitch-contour tracks across time.

pub mod klapuri;
pub mod multi_melodia;

pub use klapuri::multipitch_klapuri;
pub use multi_melodia::track_multi_pitch;
