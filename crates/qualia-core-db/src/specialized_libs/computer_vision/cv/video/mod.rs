//! Video / sequence helpers (frame ring; full demux later).

pub mod frame_sequence;
pub use frame_sequence::{
    synthetic_pulse_sequence, FrameSequence, MAX_SEQ_FRAMES,
};
