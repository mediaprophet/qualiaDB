//! M64 Matter-64 Container Byte Layout and Header.
//!
//! A sibling of the parameter/probabilistic `.p64` weight container.
//! Carrying independent section magic `b"m64\0"`, `.m64` represents the physical
//! and material universe: geodetic point-tensors, biological morphogenesis,
//! multi-layer subsurface strata stacks, and stationary-action phase junctures.
//!
//! Every record and header is strictly 64 bytes (L1 cache-line DOD).

use crate::container_10d::crc32c::crc32c;
use bytemuck::{Pod, Zeroable};

pub use super::chunk_directory::{M64ChunkDirectoryEntry, M64Error, CHUNK_DIRECTORY_ENTRY_BYTES};

/// Magic identifier for .m64 matter container files: ASCII `m64\0`.
pub const M64_MAGIC: [u8; 4] = *b"m64\0";

/// Backward-compatible magic identifier for early .e64 prototypes: ASCII `e64\0`.
pub const E64_MAGIC: [u8; 4] = *b"e64\0";

/// Current container format version.
pub const M64_VERSION: u32 = 2;

/// Standard record byte length across all M64 modes.
pub const M64_RECORD_BYTES: usize = 64;

/// Mode 0x00: Raw LiDAR / Geodetic surface anchor points & spectral reflectance.
pub const M64_MODE_POINT: u8 = 0x00;
/// Mode 0x01: Holographic biological entity & allometric morphogenesis.
pub const M64_MODE_HOLO: u8 = 0x01;
/// Mode 0x02: Multi-layer subsurface strata column & fluid depth stack.
pub const M64_MODE_STRATA: u8 = 0x02;
/// Mode 0x03: Quantum q-branch boundary conditions & CSG tool excavation.
pub const M64_MODE_PHASE_JUNCTURE: u8 = 0x03;

/// Return `true` if slice begins with `m64\0` or `e64\0`.
#[inline]
pub fn has_m64_magic(data: &[u8]) -> bool {
    data.starts_with(&M64_MAGIC) || data.starts_with(&E64_MAGIC)
}

/// Fixed 64-Byte M64 Container Header (cache-line DOD).
#[repr(C, align(64))]
#[derive(Debug, Copy, Clone, PartialEq, Pod, Zeroable)]
pub struct M64Header {
    /// 0x00..0x04: ASCII Magic `m64\0` (or `e64\0`)
    pub magic: [u8; 4],
    /// 0x04..0x08: Format version (1/2, or explicit developmental georaster 3)
    pub version: u32,
    /// 0x08..0x10: Header size (64 for v1/v2; 192 for opt-in georaster v3)
    pub header_size: u64,
    /// 0x10..0x18: Total file size in bytes
    pub total_file_size: u64,
    /// 0x18..0x20: Geodetic Reference Origin: [latitude, longitude] in degrees
    pub spatial_origin: [f32; 2],
    /// 0x20..0x28: Elevation datum: [base_altitude_m, vertical_scale_factor]
    pub elevation_datum: [f32; 2],
    /// 0x28..0x30: Byte offset to chunk directory table
    pub chunk_table_offset: u64,
    /// 0x30..0x38: Number of spatial chunk partitions
    pub chunk_count: u64,
    /// 0x38..0x3C: CRC-32C checksum of the first 56 bytes of this header
    pub header_crc32c: u32,
    /// 0x3C..0x40: Container flags / alignment padding
    pub container_flags: u32,
}

pub type E64Header = M64Header;

impl Default for M64Header {
    fn default() -> Self {
        Self {
            magic: M64_MAGIC,
            version: M64_VERSION,
            header_size: 64,
            total_file_size: 64,
            spatial_origin: [0.0, 0.0],
            elevation_datum: [0.0, 1.0],
            chunk_table_offset: 64,
            chunk_count: 0,
            header_crc32c: 0,
            container_flags: 0,
        }
    }
}

impl M64Header {
    /// Compute the CRC-32C over the first 56 bytes of the header.
    #[inline]
    pub fn compute_crc32c(&self) -> u32 {
        let bytes: &[u8; 64] = bytemuck::cast_ref(self);
        crc32c(&bytes[..56])
    }

    /// Update `header_crc32c` with the computed CRC-32C.
    #[inline]
    pub fn update_crc(&mut self) {
        self.header_crc32c = self.compute_crc32c();
    }

    /// Validate magic, version, header size, and CRC-32C.
    #[inline]
    pub fn validate(&self) -> Result<(), M64Error> {
        if !has_m64_magic(&self.magic) {
            return Err(M64Error::InvalidMagic);
        }
        if !matches!(self.version, 1 | 2 | 3) {
            return Err(M64Error::InvalidVersion(self.version as u16));
        }
        let required_size = if self.version == 3 { 192 } else { 64 };
        if self.header_size != required_size {
            return Err(M64Error::InvalidHeaderSize);
        }

        let computed = self.compute_crc32c();
        if self.header_crc32c == computed {
            Ok(())
        } else {
            Err(M64Error::ChecksumMismatch {
                stored: self.header_crc32c,
                computed,
            })
        }
    }
}
