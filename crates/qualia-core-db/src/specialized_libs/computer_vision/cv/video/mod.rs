//! Video / sequence helpers (frame ring; full demux later).

pub mod frame_sequence;
pub use frame_sequence::{synthetic_pulse_sequence, FrameSequence, MAX_SEQ_FRAMES};

#[cfg(not(target_arch = "wasm32"))]
pub mod demux_mp4;
#[cfg(not(target_arch = "wasm32"))]
pub use demux_mp4::{demux_mp4_info, demux_mp4_packets, VideoPacket, VideoTrackInfo};
