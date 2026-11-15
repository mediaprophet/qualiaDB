//! EMF Container Chunk Directory Entry and 8x8x8 Brick Indexing.
//!
//! Every chunk (8x8x8 volumetric brick) is indexed by an `EmfChunkDirectoryEntry`,
//! which is strictly 64 bytes aligned to 64 bytes (L1 cache-line DOD).

use crate::container_10d::crc32c::crc32c;
use bytemuck::{Pod, Zeroable};
use super::header::EmfError;

/// Exact byte length of an EMF chunk directory row.
pub const CHUNK_DIRECTORY_ENTRY_BYTES: usize = 64;

/// Side dimension of an EMF 3D volumetric brick (8 cells along each axis).
pub const EMF_BRICK_SIDE: usize = 8;

/// Total number of 64B cells in one 8x8x8 brick (512 cells).
pub const EMF_BRICK_CELLS: usize = 512;

/// Total byte size of one 8x8x8 brick payload (32 KB = 32,768 bytes).
pub const EMF_BRICK_BYTES: usize = 512 * 64;

/// One row of the EMF chunk directory table (64 B, cache-line DOD).
#[repr(C, align(64))]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub struct EmfChunkDirectoryEntry {
    /// 0x00..0x08: 3D Hilbert space-filling curve coordinate for 8x8x8 bricks
    pub spatial_hilbert_index: u64,
    /// 0x08..0x10: Absolute byte offset of chunk payload in .emf file
    pub byte_offset: u64,
    /// 0x10..0x14: Byte length of chunk payload (e.g. 32,768)
    pub byte_length: u32,
    /// 0x14..0x18: Number of 64B cells in chunk (e.g. 512)
    pub cell_count: u32,
    /// 0x18..0x1A: Bitmask of spectral bands present in this chunk
    pub spectral_band_mask: u16,
    /// 0x1A..0x1C: Chunk flags (bit 1 = defective legacy prefix; bit 2 = full BLAKE3 trailer)
    pub flags: u16,
    /// 0x1C..0x20: Temporal slice index (4D x,y,z,t support)
    pub time_slice_index: u32,
    /// 0x20..0x30: Legacy truncated digest field (zero in v2; full digest is in the trailer)
    pub bao_hash_prefix: [u8; 16],
    /// 0x30..0x3C: Reserved for expansion (12 bytes)
    pub reserved: [u8; 12],
    /// 0x3C..0x40: CRC-32C checksum of bytes 0..60 (Total = 64B)
    pub entry_crc32c: u32,
}

impl Default for EmfChunkDirectoryEntry {
    fn default() -> Self {
        Self {
            spatial_hilbert_index: 0,
            byte_offset: 0,
            byte_length: 0,
            cell_count: 0,
            spectral_band_mask: 0,
            flags: 0,
            time_slice_index: 0,
            bao_hash_prefix: [0u8; 16],
            reserved: [0u8; 12],
            entry_crc32c: 0,
        }
    }
}

impl EmfChunkDirectoryEntry {
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
    pub fn validate(&self) -> Result<(), EmfError> {
        let computed = self.compute_crc32c();
        if self.entry_crc32c == computed {
            Ok(())
        } else {
            Err(EmfError::ChecksumMismatch {
                stored: self.entry_crc32c,
                computed,
            })
        }
    }
}

/// Computes the linear 1D index `0..512` of a cell inside an 8x8x8 brick.
///
/// Uses 3D Morton (Z-order) interleaving across the 3-bit local coordinates
/// (`lx, ly, lz` in `0..8`), maximizing L1 cache locality.
#[inline]
pub fn emf_brick_voxel_linear_index(lx: u8, ly: u8, lz: u8) -> usize {
    let x = (lx as usize) & 0x07;
    let y = (ly as usize) & 0x07;
    let z = (lz as usize) & 0x07;
    let mut morton = 0usize;
    for i in 0..3 {
        morton |= ((x >> i) & 1) << (3 * i);
        morton |= ((y >> i) & 1) << (3 * i + 1);
        morton |= ((z >> i) & 1) << (3 * i + 2);
    }
    morton
}

/// Computes local 3D coordinates `(lx, ly, lz)` from an intra-brick linear index `0..512`.
#[inline]
pub fn emf_brick_voxel_coords(idx: usize) -> (u8, u8, u8) {
    let mut x = 0u8;
    let mut y = 0u8;
    let mut z = 0u8;
    for i in 0..3 {
        x |= (((idx >> (3 * i)) & 1) as u8) << i;
        y |= (((idx >> (3 * i + 1)) & 1) as u8) << i;
        z |= (((idx >> (3 * i + 2)) & 1) as u8) << i;
    }
    (x, y, z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emf_chunk_directory_entry_size_and_alignment() {
        assert_eq!(
            std::mem::size_of::<EmfChunkDirectoryEntry>(),
            64,
            "EmfChunkDirectoryEntry must be exactly 64 bytes"
        );
        assert_eq!(
            std::mem::align_of::<EmfChunkDirectoryEntry>(),
            64,
            "EmfChunkDirectoryEntry must have 64-byte alignment"
        );
    }

    #[test]
    fn test_emf_chunk_directory_crc_roundtrip() {
        let mut entry = EmfChunkDirectoryEntry {
            spatial_hilbert_index: 0x0123_4567_89AB_CDEF,
            byte_offset: 4096,
            byte_length: EMF_BRICK_BYTES as u32,
            cell_count: EMF_BRICK_CELLS as u32,
            spectral_band_mask: 0b0000_0001,
            flags: 1,
            time_slice_index: 42,
            bao_hash_prefix: [7u8; 16],
            reserved: [0u8; 12],
            entry_crc32c: 0,
        };

        entry.update_crc();
        assert_ne!(entry.entry_crc32c, 0);
        assert!(entry.validate().is_ok());

        entry.cell_count ^= 1;
        assert!(matches!(entry.validate(), Err(EmfError::ChecksumMismatch { .. })));
    }

    #[test]
    fn test_emf_brick_voxel_morton_roundtrip() {
        for lz in 0..8u8 {
            for ly in 0..8u8 {
                for lx in 0..8u8 {
                    let idx = emf_brick_voxel_linear_index(lx, ly, lz);
                    assert!(idx < 512);
                    let (rx, ry, rz) = emf_brick_voxel_coords(idx);
                    assert_eq!((rx, ry, rz), (lx, ly, lz));
                }
            }
        }
    }
}
