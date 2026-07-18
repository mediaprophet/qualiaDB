//! Streaming / block feature extraction (CPU reference path).

pub mod stft_stream;
pub mod energy;
pub mod cqt;
pub mod filters;
pub mod envelope;
pub mod fft;
pub mod window;
pub mod framing;
pub mod mel;
pub mod peaks;
pub mod spectral;
pub mod pitch;
pub mod loudness;
pub mod rhythm;
pub mod tonal;
pub mod structure;
pub mod salience;
pub mod contours;
pub mod pitch_midi;
pub mod multipitch;
pub mod vad;
pub mod cqt_stream;

pub use energy::{frame_energy, frame_zcr};
pub use stft_stream::{log_mel_from_mono, magnitude_stft_chunk, StreamingStft};
pub use cqt::forward_cqt_mono;
