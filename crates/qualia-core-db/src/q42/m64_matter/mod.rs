//! M64 **Matter-64 Container** specification & binary codec.
//!
//! A sibling of `.p64` (probabilistic neural weights), `.m64` represents
//! the physical and material universe: geodetic point-tensors, biological morphogenesis,
//! multi-layer subsurface strata stacks, and stationary-action phase junctures.
//!
//! Output is strictly cache-aligned (64-byte records) and little-endian.

pub mod banked_lut;
pub mod brick3d;
pub mod checked_reader;
pub mod chunk_directory;
pub mod georaster;
pub mod holo_record;
pub mod layout;
pub mod lrm3;
pub mod lut_entry;
pub mod phase_juncture;
pub mod point_record;
pub mod strata_record;
#[cfg(test)]
mod tests;

pub use banked_lut::*;
pub use brick3d::*;
pub use checked_reader::{
    M64Chunk, M64PayloadInterval, M64ReadError, M64ReadLimits, M64Reader, M64Verification,
};
pub use chunk_directory::*;
pub use georaster::*;
pub use holo_record::*;
pub use layout::*;
pub use lrm3::*;
pub use lut_entry::*;
pub use phase_juncture::*;
pub use point_record::*;
pub use strata_record::*;
