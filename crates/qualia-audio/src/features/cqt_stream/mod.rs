//! Streaming multi-frame Constant-Q transform + inverse CQT. Re-exports only (AU-CQT-STREAM).
//!
//! - [`cqt_spectrogram`] — hop a window across a mono signal, computing a
//!   row-major `[n_frames × n_bins]` CQT magnitude spectrogram into a caller
//!   buffer (reuses [`crate::features::cqt::forward_cqt_mono`] per frame).
//! - [`inverse_cqt`] — approximate magnitude-only overlap-add reconstruction
//!   of a time-domain signal from a CQT magnitude spectrogram.

pub mod inverse;
pub mod spectrogram;

pub use inverse::inverse_cqt;
pub use spectrogram::cqt_spectrogram;
