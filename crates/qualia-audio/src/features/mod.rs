//! Streaming / block feature extraction (CPU reference path).

pub mod stft_stream;
pub mod energy;
pub mod cqt;

pub use energy::{frame_energy, frame_zcr};
pub use stft_stream::{log_mel_from_mono, magnitude_stft_chunk, StreamingStft};
pub use cqt::forward_cqt_mono;
