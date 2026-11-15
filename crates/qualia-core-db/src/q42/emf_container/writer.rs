//! Container Writer and Builder for EMF-64 Containers.
//!
//! Provides zero-copy / caller-buffered serialisation with automatic
//! CRC-32C calculations and full 256-bit BLAKE3 chunk trailers.

use super::chunk_directory::{EmfChunkDirectoryEntry, CHUNK_DIRECTORY_ENTRY_BYTES};
use crate::q42::chunk_integrity::{blake3_trailer, BLAKE3_TRAILER_BYTES};
use super::header::{
    EmfCellRecord64, EmfContainerHeader, EmfError, EMF_CONTAINER_MAGIC, EMF_CONTAINER_VERSION,
    EMF_FLAG_3D_BRICK_8X8X8, EMF_FLAG_BAO_VERIFIED, EMF_CHUNK_FLAG_BLAKE3_FULL,
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

    /// Directory table size in bytes for `chunk_count` rows.
    pub(crate) fn checked_directory_bytes(chunk_count: usize) -> Result<usize, EmfError> {
        chunk_count
            .checked_mul(CHUNK_DIRECTORY_ENTRY_BYTES)
            .ok_or(EmfError::ArithmeticOverflow)
    }

    /// Payload size in bytes for `cell_count` 64-byte cells.
    pub(crate) fn checked_payload_bytes(cell_count: usize) -> Result<usize, EmfError> {
        cell_count
            .checked_mul(64)
            .and_then(|n| n.checked_add(BLAKE3_TRAILER_BYTES))
            .ok_or(EmfError::ArithmeticOverflow)
    }

    /// Calculate the exact total byte size needed to serialize this container.
    pub fn total_required_bytes(&self) -> Result<usize, EmfError> {
        let directory_bytes = Self::checked_directory_bytes(self.chunks.len())?;
        let mut payload_bytes = 0usize;
        for c in &self.chunks {
            let chunk_payload = Self::checked_payload_bytes(c.cells.len())?;
            payload_bytes = payload_bytes
                .checked_add(chunk_payload)
                .ok_or(EmfError::ArithmeticOverflow)?;
        }
        64usize
            .checked_add(directory_bytes)
            .and_then(|n| n.checked_add(payload_bytes))
            .ok_or(EmfError::ArithmeticOverflow)
    }

    /// Serialize into caller output; cold sorting uses bounded-per-chunk scratch.
    pub fn write_to_slice(&self, out: &mut [u8]) -> Result<usize, EmfError> {
        let required = self.total_required_bytes()?;
        if out.len() < required {
            return Err(EmfError::BufferTooSmall {
                required,
                provided: out.len(),
            });
        }

        // Sort chunks by Hilbert index so directory is sorted for binary search
        let mut sorted_indices: Vec<usize> = (0..self.chunks.len()).collect();
        sorted_indices.sort_by_key(|&idx| self.chunks[idx].spatial_hilbert_index);

        let chunk_count =
            u32::try_from(self.chunks.len()).map_err(|_| EmfError::ArithmeticOverflow)?;
        let directory_offset = 64u64;
        let directory_bytes = u64::try_from(Self::checked_directory_bytes(self.chunks.len())?)
            .map_err(|_| EmfError::ArithmeticOverflow)?;
        let mut current_payload_offset = directory_offset
            .checked_add(directory_bytes)
            .ok_or(EmfError::ArithmeticOverflow)?;

        // Write directory and payloads
        for (i, &chunk_idx) in sorted_indices.iter().enumerate() {
            let chunk = &self.chunks[chunk_idx];
            let cell_count =
                u32::try_from(chunk.cells.len()).map_err(|_| EmfError::ArithmeticOverflow)?;
            let byte_length = cell_count
                .checked_mul(64)
                .and_then(|n| n.checked_add(BLAKE3_TRAILER_BYTES as u32))
                .ok_or(EmfError::ArithmeticOverflow)?;

            // Serialize cell records to payload region
            let p_start = usize::try_from(current_payload_offset)
                .map_err(|_| EmfError::ArithmeticOverflow)?;
            let payload_len = usize::try_from(byte_length).map_err(|_| EmfError::ArithmeticOverflow)? - BLAKE3_TRAILER_BYTES;
            let p_end = p_start
                .checked_add(payload_len)
                .ok_or(EmfError::ArithmeticOverflow)?;
            let payload_slice = bytemuck::cast_slice(&chunk.cells);
            out.get_mut(p_start..p_end)
                .ok_or(EmfError::PayloadOutOfBounds)?
                .copy_from_slice(payload_slice);

            let trailer = blake3_trailer(&out[p_start..p_end]);
            out[p_end..p_end + BLAKE3_TRAILER_BYTES].copy_from_slice(&trailer);

            // Construct directory entry
            let mut entry = EmfChunkDirectoryEntry {
                spatial_hilbert_index: chunk.spatial_hilbert_index,
                byte_offset: current_payload_offset,
                byte_length,
                cell_count,
                spectral_band_mask: chunk.spectral_band_mask,
                flags: (chunk.flags & !2) | EMF_CHUNK_FLAG_BLAKE3_FULL,
                time_slice_index: chunk.time_slice_index,
                bao_hash_prefix: [0; 16],
                reserved: [0u8; 12],
                entry_crc32c: 0,
            };
            entry.update_crc();

            // Write directory row
            let d_start = usize::try_from(directory_offset)
                .map_err(|_| EmfError::ArithmeticOverflow)?
                .checked_add(
                    i.checked_mul(CHUNK_DIRECTORY_ENTRY_BYTES)
                        .ok_or(EmfError::ArithmeticOverflow)?,
                )
                .ok_or(EmfError::ArithmeticOverflow)?;
            let d_end = d_start
                .checked_add(CHUNK_DIRECTORY_ENTRY_BYTES)
                .ok_or(EmfError::ArithmeticOverflow)?;
            out.get_mut(d_start..d_end)
                .ok_or(EmfError::DirectoryBounds)?
                .copy_from_slice(bytemuck::bytes_of(&entry));

            current_payload_offset = current_payload_offset
                .checked_add(u64::from(byte_length))
                .ok_or(EmfError::ArithmeticOverflow)?;
        }

        // Build and write container header
        let mut header = EmfContainerHeader {
            magic: EMF_CONTAINER_MAGIC,
            version: EMF_CONTAINER_VERSION,
            flags: self.flags & !EMF_FLAG_BAO_VERIFIED,
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

    /// Opt-in v3. Cells and their repaired v2 trailers remain byte-identical.
    /// All extension/file limits are checked before allocating the output buffer.
    pub fn to_bytes_v3(&self,field:&super::extension::EmfFieldInterpretation,tracks:&[super::extension::EmfAudioTrack<'_>],limits:super::extension::EmfExtensionLimits)->Result<Vec<u8>,EmfError>{
        if !cfg!(target_endian="little")||self.flags&!7!=0||self.chunks.iter().any(|c|c.flags & !EMF_CHUNK_FLAG_BLAKE3_FULL !=0){return Err(EmfError::ExtensionMetadata);}
        let probe=EmfContainerHeader{reference_epoch_s:self.reference_epoch_s,voxel_size_meters:self.voxel_size_meters,
            grid_dims_bricks:self.grid_dims_bricks,origin_meters:self.origin_meters,..EmfContainerHeader::default()};
        field.validate_header(&probe)?;
        let prepared=super::extension::prepare_extension(field,tracks,limits)?;
        let field_end=self.total_required_bytes()?;let total=prepared.required_file_bytes(field_end)?;
        let mut bytes=vec![0u8;total];self.write_to_slice(&mut bytes[..field_end])?;
        prepared.write_into(tracks,&mut bytes,field_end)?;
        let mut header:EmfContainerHeader=bytemuck::pod_read_unaligned(&bytes[..64]);
        header.version=super::header::EMF_EXTENDED_VERSION;header.update_crc();
        bytes[..64].copy_from_slice(bytemuck::bytes_of(&header));
        super::reader::EmfContainerReader::parse_with_extension_limits(&bytes,super::reader::EmfReadLimits{max_file_bytes:limits.max_file_bytes,max_chunks:self.chunks.len()},limits)?;
        Ok(bytes)
    }

    /// Serialize the container into a heap-allocated `Vec<u8>` (Tier-2 Cold Construction).
    pub fn to_bytes(&self) -> Result<Vec<u8>, EmfError> {
        let required = self.total_required_bytes()?;
        let mut out = vec![0u8; required];
        self.write_to_slice(&mut out)?;
        Ok(out)
    }
}
