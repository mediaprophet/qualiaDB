//! M64 Chunk Directory Entry Layout and Validation.
//!
//! Every chunk in an `.m64` file is indexed by an `M64ChunkDirectoryEntry`,
//! which is strictly 64 bytes aligned to 64 bytes.

use crate::container_10d::crc32c::crc32c;

pub const CHUNK_DIRECTORY_ENTRY_BYTES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum M64Error {
    InvalidMagic,
    InvalidVersion(u16),
    InvalidHeaderSize,
    ChecksumMismatch { stored: u32, computed: u32 },
    InvalidRecordMode(u8),
    BufferTooSmall { required: usize, provided: usize },
}

impl std::fmt::Display for M64Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "Invalid M64 container magic"),
            Self::InvalidVersion(v) => write!(f, "Unsupported M64 version: {v}"),
            Self::InvalidHeaderSize => write!(f, "Invalid M64 header size"),
            Self::ChecksumMismatch { stored, computed } => {
                write!(f, "M64 CRC-32C mismatch: stored {stored:#010x}, computed {computed:#010x}")
            }
            Self::InvalidRecordMode(m) => write!(f, "Invalid M64 record mode: {m:#04x}"),
            Self::BufferTooSmall { required, provided } => {
                write!(f, "M64 buffer too small: required {required} bytes, provided {provided} bytes")
            }
        }
    }
}

impl std::error::Error for M64Error {}

/// One row of the M64 chunk directory table (64 B, cache-line DOD).
#[repr(C, align(64))]
#[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct M64ChunkDirectoryEntry {
    /// 16B: Canonical UUID or content-addressed BLAKE3 hash prefix of chunk.
    pub chunk_uuid: [u8; 16],
    /// 8B: 2D/3D Hilbert space-filling curve coordinate for spatial clustering.
    pub spatial_hilbert_index: u64,
    /// 8B: Absolute byte offset of chunk payload in .m64 file.
    pub byte_offset: u64,
    /// 4B: Exact byte length of chunk payload (multiple of 64).
    pub byte_length: u32,
    /// 4B: Number of 64B records in chunk.
    pub record_count: u32,
    /// 4B: EPSG/SRID geodetic code (e.g. 4326 for WGS84, 7844 for GDA2020).
    pub srid_geodetic_code: u32,
    /// 2B: Identifier of the Bank 1 LUT recipe set for this chunk.
    pub lut_bank_epoch_id: u16,
    /// 1B: Chunk record mode (0x00=Point, 0x01=Holo, 0x02=Strata, 0x03=PhaseJuncture).
    pub chunk_record_mode: u8,
    /// 1B: Compression flags (0=Uncompressed mmap, 1=LZ4 block, 2=Bao verified).
    pub compression_flags: u8,
    /// 12B: Prefix of BLAKE3-Bao verified outboard hash tree.
    pub bao_root_hash_prefix: [u8; 12],
    /// 4B: CRC-32C checksum of bytes 0..59 (Total = 64B).
    pub entry_crc32c: u32,
}

impl M64ChunkDirectoryEntry {
    /// Compute the CRC-32C over the first 60 bytes of the directory entry.
    #[inline]
    pub fn compute_crc32c(&self) -> u32 {
        let bytes: &[u8; CHUNK_DIRECTORY_ENTRY_BYTES] = bytemuck::cast_ref(self);
        crc32c(&bytes[..60])
    }

    /// Update `entry_crc32c` with the computed CRC-32C.
    #[inline]
    pub fn update_crc(&mut self) {
        self.entry_crc32c = self.compute_crc32c();
    }

    /// Validate the CRC-32C of this directory entry.
    #[inline]
    pub fn validate(&self) -> Result<(), M64Error> {
        let computed = self.compute_crc32c();
        if self.entry_crc32c == computed {
            Ok(())
        } else {
            Err(M64Error::ChecksumMismatch {
                stored: self.entry_crc32c,
                computed,
            })
        }
    }
}
