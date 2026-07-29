//! Streaming / block feature extraction (CPU reference path).

pub mod contours;
pub mod cqt;
pub mod cqt_stream;
pub mod energy;
pub mod envelope;
pub mod fft;
pub mod filters;
pub mod framing;
pub mod loudness;
pub mod mel;
pub mod multipitch;
pub mod peaks;
pub mod pitch;
pub mod pitch_midi;
pub mod rhythm;
pub mod salience;
pub mod spectral;
pub mod stft_stream;
pub mod structure;
pub mod tonal;
pub mod vad;
pub mod window;

pub use cqt::forward_cqt_mono;
pub use energy::{frame_energy, frame_zcr};
pub use stft_stream::{log_mel_from_mono, magnitude_stft_chunk, StreamingStft};
