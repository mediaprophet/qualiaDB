//! Stereo — mux / demux / width. Re-exports only (AU-PROD).

pub mod demux;
pub mod mux;
pub mod width;

pub use demux::demux;
pub use mux::mux;
pub use width::width;
