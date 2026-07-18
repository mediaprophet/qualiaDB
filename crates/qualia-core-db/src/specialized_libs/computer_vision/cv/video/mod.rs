//! Video / sequence helpers (frame ring; full demux later).

pub mod frame_sequence;
pub use frame_sequence::{
    synthetic_pulse_sequence, FrameSequence, MAX_SEQ_FRAMES,
};

pub mod demux_mp4;
pub use demux_mp4::{demux_mp4_info, demux_mp4_packets, VideoPacket, VideoTrackInfo};
