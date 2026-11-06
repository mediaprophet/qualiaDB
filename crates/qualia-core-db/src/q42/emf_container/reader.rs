//! Zero-Copy Structural Reader for EMF-64 Containers.
//!
//! Provides zero-copy parsing, CRC-32C and BLAKE3-Bao verification, and
//! fast 3D Hilbert key chunk lookup.

use super::chunk_directory::{
    emf_brick_voxel_linear_index, EmfChunkDirectoryEntry, CHUNK_DIRECTORY_ENTRY_BYTES,
};
use super::header::{EmfCellRecord64, EmfContainerHeader, EmfError};
use crate::q42::m64_matter::brick3d::encode_hilbert3d_u64;

/// Immutable zero-copy view of an EMF-64 container file.
#[derive(Debug)]
pub struct EmfContainerReader<'a> {
    bytes: &'a [u8],
    header: EmfContainerHeader,
    directory_offset: usize,
    chunk_count: usize,
}

impl<'a> EmfContainerReader<'a> {
    /// Parse and validate an EMF container from a byte slice.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, EmfError> {
        if !cfg!(target_endian = "little") {
            return Err(EmfError::InvalidMagic);
        }
        if bytes.len() < 64 {
            return Err(EmfError::BufferTooSmall {
                required: 64,
                provided: bytes.len(),
            });
        }

        let header: EmfContainerHeader = bytemuck::pod_read_unaligned(&bytes[..64]);
        header.validate()?;

        let directory_offset = usize::try_from(header.directory_offset)
            .map_err(|_| EmfError::ArithmeticOverflow)?;
        let chunk_count = usize::try_from(header.chunk_count)
            .map_err(|_| EmfError::ArithmeticOverflow)?;

        if directory_offset % 64 != 0 {
            return Err(EmfError::DirectoryAlignment);
        }
        let directory_bytes = chunk_count
            .checked_mul(CHUNK_DIRECTORY_ENTRY_BYTES)
            .ok_or(EmfError::ArithmeticOverflow)?;
        let directory_end = directory_offset
            .checked_add(directory_bytes)
            .ok_or(EmfError::ArithmeticOverflow)?;

        if directory_end > bytes.len() {
            return Err(EmfError::DirectoryBounds);
        }

        // Validate each chunk directory entry and Bao hash
        for chunk in 0..chunk_count {
            let offset = directory_offset + chunk * CHUNK_DIRECTORY_ENTRY_BYTES;
            let entry: EmfChunkDirectoryEntry =
                bytemuck::pod_read_unaligned(&bytes[offset..offset + CHUNK_DIRECTORY_ENTRY_BYTES]);
            entry.validate()?;

            let start = usize::try_from(entry.byte_offset).map_err(|_| EmfError::ArithmeticOverflow)?;
            let length = usize::try_from(entry.byte_length).map_err(|_| EmfError::ArithmeticOverflow)?;
            let end = start.checked_add(length).ok_or(EmfError::ArithmeticOverflow)?;

            if start % 64 != 0 {
                return Err(EmfError::PayloadUnaligned);
            }
            if end > bytes.len() {
                return Err(EmfError::PayloadOutOfBounds);
            }
            let expected_bytes = usize::try_from(entry.cell_count)
                .map_err(|_| EmfError::ArithmeticOverflow)?
                .checked_mul(64)
                .ok_or(EmfError::ArithmeticOverflow)?;
            if length != expected_bytes {
                return Err(EmfError::PayloadOutOfBounds);
            }

            // BLAKE3-Bao digest prefix verification
            if (entry.flags & 2 != 0) || (entry.bao_hash_prefix != [0u8; 16]) {
                let payload = &bytes[start..end];
                let digest = blake3::hash(payload);
                if digest.as_bytes()[..16] != entry.bao_hash_prefix {
                    return Err(EmfError::BaoHashMismatch { chunk_index: chunk });
                }
            }
        }

        Ok(Self {
            bytes,
            header,
            directory_offset,
            chunk_count,
        })
    }

    /// Access the parsed container header.
    #[inline]
    pub fn header(&self) -> &EmfContainerHeader {
        &self.header
    }

    /// Total number of chunks / bricks in this container.
    #[inline]
    pub fn chunk_count(&self) -> usize {
        self.chunk_count
    }

    /// Read a chunk directory entry by index.
    pub fn chunk_entry(&self, index: usize) -> Option<EmfChunkDirectoryEntry> {
        if index >= self.chunk_count {
            return None;
        }
        let offset = self.directory_offset + index * CHUNK_DIRECTORY_ENTRY_BYTES;
        Some(bytemuck::pod_read_unaligned(
            &self.bytes[offset..offset + CHUNK_DIRECTORY_ENTRY_BYTES],
        ))
    }

    /// Borrow the raw byte payload of a chunk.
    pub fn chunk_payload(&self, index: usize) -> Option<&'a [u8]> {
        let entry = self.chunk_entry(index)?;
        let start = entry.byte_offset as usize;
        let end = start + entry.byte_length as usize;
        self.bytes.get(start..end)
    }

    /// Read a specific 64B cell from a chunk.
    pub fn cell(&self, chunk_index: usize, cell_index: usize) -> Option<EmfCellRecord64> {
        let payload = self.chunk_payload(chunk_index)?;
        let start = cell_index.checked_mul(64)?;
        let raw = payload.get(start..start.checked_add(64)?)?;
        Some(bytemuck::pod_read_unaligned(raw))
    }

    /// Binary search the directory for a chunk matching the given 3D Hilbert spatial index.
    pub fn find_chunk_by_hilbert(&self, hilbert_key: u64) -> Option<usize> {
        let mut low = 0;
        let mut high = self.chunk_count;

        while low < high {
            let mid = low + (high - low) / 2;
            let entry = self.chunk_entry(mid)?;
            if entry.spatial_hilbert_index == hilbert_key {
                return Some(mid);
            } else if entry.spatial_hilbert_index < hilbert_key {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        None
    }

    /// Direct lookup of a voxel cell using brick coordinates `(bx, by, bz)` and local coords `(lx, ly, lz)`.
    pub fn lookup_brick_cell(
        &self,
        bx: u32,
        by: u32,
        bz: u32,
        lx: u8,
        ly: u8,
        lz: u8,
        order: u32,
    ) -> Option<EmfCellRecord64> {
        let hilbert_key = encode_hilbert3d_u64(bx, by, bz, order);
        let chunk_index = self.find_chunk_by_hilbert(hilbert_key)?;
        let voxel_index = emf_brick_voxel_linear_index(lx, ly, lz);
        self.cell(chunk_index, voxel_index)
    }
}
