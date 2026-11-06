//! Container Writer and Builder for EMF-64 Containers.
//!
//! Provides zero-copy / caller-buffered serialisation with automatic
//! CRC-32C calculations and BLAKE3-Bao digest prefix generation.

use super::chunk_directory::{EmfChunkDirectoryEntry, CHUNK_DIRECTORY_ENTRY_BYTES};
use super::header::{
    EmfCellRecord64, EmfContainerHeader, EmfError, EMF_CONTAINER_MAGIC, EMF_CONTAINER_VERSION,
    EMF_FLAG_3D_BRICK_8X8X8,
};

/// An uncompressed chunk staged for writing into an EMF-64 container.
#[derive(Clone, Debug)]
pub struct StagedEmfChunk {
    pub spatial_hilbert_index: u64,
    pub cells: Vec<EmfCellRecord64>,
    pub time_slice_index: u32,
    pub spectral_band_mask: u16,
    pub flags: u16,
}

/// Builder for constructing valid EMF-64 container files.
#[derive(Clone, Debug)]
pub struct EmfContainerWriter {
    pub origin_meters: [f32; 3],
    pub voxel_size_meters: f32,
    pub grid_dims_bricks: [u32; 3],
    pub reference_epoch_s: f64,
    pub flags: u16,
    chunks: Vec<StagedEmfChunk>,
}

impl EmfContainerWriter {
    /// Create a new EMF container builder.
    pub fn new(
        origin_meters: [f32; 3],
        voxel_size_meters: f32,
        grid_dims_bricks: [u32; 3],
        reference_epoch_s: f64,
    ) -> Self {
        Self {
            origin_meters,
            voxel_size_meters,
            grid_dims_bricks,
            reference_epoch_s,
            flags: EMF_FLAG_3D_BRICK_8X8X8,
            chunks: Vec::new(),
        }
    }

    /// Add an 8x8x8 volumetric brick chunk.
    pub fn add_brick_chunk(
        &mut self,
        spatial_hilbert_index: u64,
        cells: &[EmfCellRecord64],
        time_slice_index: u32,
        spectral_band_mask: u16,
        flags: u16,
    ) -> Result<(), EmfError> {
        self.chunks.push(StagedEmfChunk {
            spatial_hilbert_index,
            cells: cells.to_vec(),
            time_slice_index,
            spectral_band_mask,
            flags,
        });
        Ok(())
    }

    /// Calculate the exact total byte size needed to serialize this container.
    pub fn total_required_bytes(&self) -> usize {
        let chunk_count = self.chunks.len();
        let header_bytes = 64;
        let directory_bytes = chunk_count * CHUNK_DIRECTORY_ENTRY_BYTES;
        let mut payload_bytes = 0;
        for c in &self.chunks {
            payload_bytes += c.cells.len() * 64;
        }
        header_bytes + directory_bytes + payload_bytes
    }

    /// Serialize the container into a caller-supplied fixed buffer (Tier-1 Zero-Heap).
    pub fn write_to_slice(&self, out: &mut [u8]) -> Result<usize, EmfError> {
        let required = self.total_required_bytes();
        if out.len() < required {
            return Err(EmfError::BufferTooSmall {
                required,
                provided: out.len(),
            });
        }

        // Sort chunks by Hilbert index so directory is sorted for binary search
        let mut sorted_indices: Vec<usize> = (0..self.chunks.len()).collect();
        sorted_indices.sort_by_key(|&idx| self.chunks[idx].spatial_hilbert_index);

        let chunk_count = self.chunks.len() as u32;
        let directory_offset = 64u64;
        let directory_bytes = (self.chunks.len() * CHUNK_DIRECTORY_ENTRY_BYTES) as u64;
        let mut current_payload_offset = directory_offset + directory_bytes;

        // Write directory and payloads
        for (i, &chunk_idx) in sorted_indices.iter().enumerate() {
            let chunk = &self.chunks[chunk_idx];
            let cell_count = chunk.cells.len() as u32;
            let byte_length = cell_count * 64;

            // Serialize cell records to payload region
            let p_start = current_payload_offset as usize;
            let p_end = p_start + (byte_length as usize);
            let payload_slice = bytemuck::cast_slice(&chunk.cells);
            out[p_start..p_end].copy_from_slice(payload_slice);

            // Compute BLAKE3 digest of payload
            let digest = blake3::hash(&out[p_start..p_end]);
            let mut bao_hash_prefix = [0u8; 16];
            bao_hash_prefix.copy_from_slice(&digest.as_bytes()[..16]);

            // Construct directory entry
            let mut entry = EmfChunkDirectoryEntry {
                spatial_hilbert_index: chunk.spatial_hilbert_index,
                byte_offset: current_payload_offset,
                byte_length,
                cell_count,
                spectral_band_mask: chunk.spectral_band_mask,
                flags: chunk.flags | 2, // Bit 1 = Bao verified
                time_slice_index: chunk.time_slice_index,
                bao_hash_prefix,
                reserved: [0u8; 12],
                entry_crc32c: 0,
            };
            entry.update_crc();

            // Write directory row
            let d_start = (directory_offset as usize) + i * CHUNK_DIRECTORY_ENTRY_BYTES;
            let d_end = d_start + CHUNK_DIRECTORY_ENTRY_BYTES;
            out[d_start..d_end].copy_from_slice(bytemuck::bytes_of(&entry));

            current_payload_offset += byte_length as u64;
        }

        // Build and write container header
        let mut header = EmfContainerHeader {
            magic: EMF_CONTAINER_MAGIC,
            version: EMF_CONTAINER_VERSION,
            flags: self.flags,
            chunk_count,
            directory_offset,
            reference_epoch_s: self.reference_epoch_s,
            grid_dims_bricks: self.grid_dims_bricks,
            voxel_size_meters: self.voxel_size_meters,
            origin_meters: self.origin_meters,
            header_crc32c: 0,
        };
        header.update_crc();
        out[..64].copy_from_slice(bytemuck::bytes_of(&header));

        Ok(required)
    }

    /// Serialize the container into a heap-allocated `Vec<u8>` (Tier-2 Cold Construction).
    pub fn to_bytes(&self) -> Vec<u8> {
        let required = self.total_required_bytes();
        let mut out = vec![0u8; required];
        self.write_to_slice(&mut out).expect("buffer allocated to exact size");
        out
    }
}
