//! Zero-Copy Structural Reader for EMF-64 Containers.
//!
//! Provides zero-copy parsing, CRC-32C and BLAKE3-Bao verification, and
//! fast 3D Hilbert key chunk lookup.

use super::chunk_directory::{
    emf_brick_voxel_coords, emf_brick_voxel_linear_index, EmfChunkDirectoryEntry,
    CHUNK_DIRECTORY_ENTRY_BYTES, EMF_BRICK_SIDE,
};
use super::header::{EmfCellRecord64, EmfContainerHeader, EmfError, MAX_DENSE_CELLS};
use crate::q42::m64_matter::brick3d::{decode_hilbert3d_u64, encode_hilbert3d_u64};

/// Explicit EMF admission budgets (file bytes and directory rows).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmfReadLimits {
    pub max_file_bytes: usize,
    pub max_chunks: usize,
}

impl EmfReadLimits {
    /// Conservative documented defaults used by [`EmfContainerReader::parse`].
    ///
    /// 16 MiB matches the produce-path resource budget; 4096 chunks is a
    /// directory-row cap independent of brick payload size. Untrusted callers
    /// should pass explicit limits via [`EmfContainerReader::parse_with_limits`].
    pub const fn conservative() -> Self {
        Self {
            max_file_bytes: 16 * 1024 * 1024,
            max_chunks: 4096,
        }
    }
}

#[inline]
fn ranges_overlap(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> bool {
    a_start < b_end && b_start < a_end
}

/// Immutable zero-copy view of an EMF-64 container file.
#[derive(Debug)]
pub struct EmfContainerReader<'a> {
    bytes: &'a [u8],
    header: EmfContainerHeader,
    directory_offset: usize,
    chunk_count: usize,
}

impl<'a> EmfContainerReader<'a> {
    /// Parse under conservative 16 MiB / 4096-chunk budgets.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, EmfError> {
        Self::parse_with_limits(bytes, EmfReadLimits::conservative())
    }

    /// Parse and validate an EMF container under explicit file/chunk budgets.
    pub fn parse_with_limits(bytes: &'a [u8], limits: EmfReadLimits) -> Result<Self, EmfError> {
        if !cfg!(target_endian = "little") {
            return Err(EmfError::InvalidMagic);
        }
        if bytes.len() < 64 {
            return Err(EmfError::BufferTooSmall {
                required: 64,
                provided: bytes.len(),
            });
        }
        if bytes.len() > limits.max_file_bytes {
            return Err(EmfError::FileByteLimit);
        }

        let header: EmfContainerHeader = bytemuck::pod_read_unaligned(&bytes[..64]);
        header.validate()?;

        let directory_offset = usize::try_from(header.directory_offset)
            .map_err(|_| EmfError::ArithmeticOverflow)?;
        let chunk_count = usize::try_from(header.chunk_count)
            .map_err(|_| EmfError::ArithmeticOverflow)?;

        if chunk_count > limits.max_chunks {
            return Err(EmfError::ChunkLimitExceeded);
        }
        if directory_offset < 64 || directory_offset % 64 != 0 {
            return Err(if directory_offset % 64 != 0 {
                EmfError::DirectoryAlignment
            } else {
                EmfError::DirectoryBounds
            });
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
            if start < 64 || ranges_overlap(start, end, directory_offset, directory_end) {
                return Err(EmfError::PayloadOverlap);
            }
            let expected_bytes = usize::try_from(entry.cell_count)
                .map_err(|_| EmfError::ArithmeticOverflow)?
                .checked_mul(64)
                .ok_or(EmfError::ArithmeticOverflow)?;
            if length != expected_bytes {
                return Err(EmfError::PayloadOutOfBounds);
            }

            for prev in 0..chunk {
                let prev_off = directory_offset + prev * CHUNK_DIRECTORY_ENTRY_BYTES;
                let prev_entry: EmfChunkDirectoryEntry = bytemuck::pod_read_unaligned(
                    &bytes[prev_off..prev_off + CHUNK_DIRECTORY_ENTRY_BYTES],
                );
                let prev_start = usize::try_from(prev_entry.byte_offset)
                    .map_err(|_| EmfError::ArithmeticOverflow)?;
                let prev_len = usize::try_from(prev_entry.byte_length)
                    .map_err(|_| EmfError::ArithmeticOverflow)?;
                let prev_end = prev_start
                    .checked_add(prev_len)
                    .ok_or(EmfError::ArithmeticOverflow)?;
                if ranges_overlap(start, end, prev_start, prev_end) {
                    return Err(EmfError::PayloadOverlap);
                }
            }

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

    /// Unpack Hilbert 8³ bricks into a dense `[t][z][y][x]` grid of scientific cells.
    ///
    /// This is the source of truth for GPU RGBA and AcousticPlane σ. `.10d` MID2/FIEL
    /// names the field; it does not duplicate these cells.
    pub fn dense_volume(&self) -> Result<DenseEmfVolume, EmfError> {
        let dims = self.header.grid_dims_bricks;
        if dims[0] == 0 || dims[1] == 0 || dims[2] == 0 {
            return Err(EmfError::ChunkLimitExceeded);
        }
        let side = EMF_BRICK_SIDE as u32;
        let nx = dims[0]
            .checked_mul(side)
            .ok_or(EmfError::ArithmeticOverflow)?;
        let ny = dims[1]
            .checked_mul(side)
            .ok_or(EmfError::ArithmeticOverflow)?;
        let nz = dims[2]
            .checked_mul(side)
            .ok_or(EmfError::ArithmeticOverflow)?;

        let mut max_t = 0u32;
        for i in 0..self.chunk_count {
            let entry = self.chunk_entry(i).ok_or(EmfError::DirectoryBounds)?;
            max_t = max_t.max(entry.time_slice_index);
        }
        let nt = max_t
            .checked_add(1)
            .ok_or(EmfError::ArithmeticOverflow)?;

        let cell_count = (nx as usize)
            .checked_mul(ny as usize)
            .and_then(|n| n.checked_mul(nz as usize))
            .and_then(|n| n.checked_mul(nt as usize))
            .ok_or(EmfError::ArithmeticOverflow)?;
        if cell_count > MAX_DENSE_CELLS {
            return Err(EmfError::DenseVolumeCap {
                cells: cell_count,
                cap: MAX_DENSE_CELLS,
            });
        }
        let mut cells = vec![EmfCellRecord64::default(); cell_count];
        let mut sigma_sum = 0.0f64;
        let mut sigma_n = 0u64;
        let order = infer_container_hilbert_order(self, dims)?;

        for i in 0..self.chunk_count {
            let entry = self.chunk_entry(i).ok_or(EmfError::DirectoryBounds)?;
            let (bx, by, bz) = decode_hilbert3d_u64(entry.spatial_hilbert_index, order);
            if bx >= dims[0] || by >= dims[1] || bz >= dims[2] {
                return Err(EmfError::HilbertOrder);
            }
            let t = entry.time_slice_index;
            let n_cells = usize::try_from(entry.cell_count).map_err(|_| EmfError::ArithmeticOverflow)?;
            for vi in 0..n_cells {
                let cell = self.cell(i, vi).ok_or(EmfError::PayloadOutOfBounds)?;
                let (lx, ly, lz) = emf_brick_voxel_coords(vi);
                let x = bx
                    .checked_mul(side)
                    .and_then(|v| v.checked_add(lx as u32))
                    .ok_or(EmfError::ArithmeticOverflow)?;
                let y = by
                    .checked_mul(side)
                    .and_then(|v| v.checked_add(ly as u32))
                    .ok_or(EmfError::ArithmeticOverflow)?;
                let z = bz
                    .checked_mul(side)
                    .and_then(|v| v.checked_add(lz as u32))
                    .ok_or(EmfError::ArithmeticOverflow)?;
                if x >= nx || y >= ny || z >= nz {
                    return Err(EmfError::PayloadOutOfBounds);
                }
                let idx = dense_cell_index(x, y, z, t, nx, ny, nz)?;
                cells[idx] = cell;
                if cell.e_magnitude() > 0.0
                    || cell.wavelength_nm > 0.0
                    || cell.spectral_sigma != 0.0
                {
                    sigma_sum += cell.effective_sigma() as f64;
                    sigma_n += 1;
                }
            }
        }

        let mean_sigma = if sigma_n > 0 {
            (sigma_sum / sigma_n as f64) as f32
        } else {
            0.0
        };

        Ok(DenseEmfVolume {
            origin_meters: self.header.origin_meters,
            voxel_size_meters: self.header.voxel_size_meters,
            nx,
            ny,
            nz,
            nt,
            cells,
            mean_sigma,
        })
    }
}

/// Dense `[t][z][y][x]` unpack of a `Q42EMF64` container.
#[derive(Clone, Debug)]
pub struct DenseEmfVolume {
    pub origin_meters: [f32; 3],
    pub voxel_size_meters: f32,
    pub nx: u32,
    pub ny: u32,
    pub nz: u32,
    pub nt: u32,
    pub cells: Vec<EmfCellRecord64>,
    pub mean_sigma: f32,
}

impl DenseEmfVolume {
    /// Axis-aligned bounds in metres for GPU slice uniforms.
    pub fn bounds_meters(&self) -> [f32; 6] {
        let ox = self.origin_meters[0];
        let oy = self.origin_meters[1];
        let oz = self.origin_meters[2];
        let vx = self.voxel_size_meters * self.nx as f32;
        let vy = self.voxel_size_meters * self.ny as f32;
        let vz = self.voxel_size_meters * self.nz as f32;
        [ox, ox + vx, oy, oy + vy, oz, oz + vz]
    }
}

fn dense_cell_index(
    x: u32,
    y: u32,
    z: u32,
    t: u32,
    nx: u32,
    ny: u32,
    nz: u32,
) -> Result<usize, EmfError> {
    let plane = (nx as usize)
        .checked_mul(ny as usize)
        .ok_or(EmfError::ArithmeticOverflow)?;
    let volume = plane
        .checked_mul(nz as usize)
        .ok_or(EmfError::ArithmeticOverflow)?;
    let t_off = (t as usize)
        .checked_mul(volume)
        .ok_or(EmfError::ArithmeticOverflow)?;
    let z_off = (z as usize)
        .checked_mul(plane)
        .ok_or(EmfError::ArithmeticOverflow)?;
    let y_off = (y as usize)
        .checked_mul(nx as usize)
        .ok_or(EmfError::ArithmeticOverflow)?;
    t_off
        .checked_add(z_off)
        .and_then(|n| n.checked_add(y_off))
        .and_then(|n| n.checked_add(x as usize))
        .ok_or(EmfError::ArithmeticOverflow)
}

fn infer_container_hilbert_order(
    reader: &EmfContainerReader<'_>,
    dims: [u32; 3],
) -> Result<u32, EmfError> {
    if reader.chunk_count == 0 {
        return Ok(2);
    }
    // Writers: compile_emf uses 2, anatomy seal uses 3. Prefer those before
    // a lower order that can accidentally round-trip a subset of keys.
    for order in [2, 3, 1, 4, 5, 6, 7, 8] {
        let mut seen = std::collections::BTreeSet::new();
        let mut ok = true;
        for i in 0..reader.chunk_count {
            let entry = reader.chunk_entry(i).ok_or(EmfError::DirectoryBounds)?;
            let (bx, by, bz) = decode_hilbert3d_u64(entry.spatial_hilbert_index, order);
            if bx >= dims[0]
                || by >= dims[1]
                || bz >= dims[2]
                || encode_hilbert3d_u64(bx, by, bz, order) != entry.spatial_hilbert_index
                || !seen.insert((bx, by, bz, entry.time_slice_index))
            {
                ok = false;
                break;
            }
        }
        if ok {
            return Ok(order);
        }
    }
    Err(EmfError::HilbertOrder)
}
