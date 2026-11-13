//! Zero-Copy Structural Reader for EMF-64 Containers.
//!
//! Provides zero-copy parsing, CRC-32C and full BLAKE3 verification, and
//! fast 3D Hilbert key chunk lookup.

use super::chunk_directory::{
    emf_brick_voxel_coords, emf_brick_voxel_linear_index, EmfChunkDirectoryEntry,
    CHUNK_DIRECTORY_ENTRY_BYTES, EMF_BRICK_SIDE,
};
use super::header::{EmfCellRecord64, EmfContainerHeader, EmfError, MAX_DENSE_CELLS};
use super::header::{EMF_EXTENDED_VERSION, EMF_CONTAINER_VERSION, EMF_CHUNK_FLAG_BLAKE3_FULL, EMF_FLAG_BAO_VERIFIED};
use crate::q42::chunk_integrity::{verify_blake3_trailer, BLAKE3_TRAILER_BYTES};
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
    directory_sorted: bool,
    lookup_work_bound: usize,
    extension: Option<super::extension::EmfExtensionView<'a>>,
}

impl<'a> EmfContainerReader<'a> {
    /// Parse under conservative 16 MiB / 4096-chunk budgets.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, EmfError> {
        Self::parse_with_limits(bytes, EmfReadLimits::conservative())
    }

    /// Parse and validate an EMF container under explicit file/chunk budgets.
    pub fn parse_with_limits(bytes: &'a [u8], limits: EmfReadLimits) -> Result<Self, EmfError> {
        Self::parse_policy(bytes, limits, false, super::extension::EmfExtensionLimits {max_file_bytes:limits.max_file_bytes,..super::extension::EmfExtensionLimits::conservative()})
    }

    /// Migration-only admission of v1 files with defective truncated integrity.
    /// A successful call does not certify a full digest for a legacy file.
    pub fn parse_legacy_with_limits(bytes: &'a [u8], limits: EmfReadLimits) -> Result<Self, EmfError> {
        Self::parse_policy(bytes, limits, true, super::extension::EmfExtensionLimits {max_file_bytes:limits.max_file_bytes,..super::extension::EmfExtensionLimits::conservative()})
    }

    /// Current-format reader with independently bounded metadata/PCM admission.
    pub fn parse_with_extension_limits(bytes:&'a[u8],limits:EmfReadLimits,extension_limits:super::extension::EmfExtensionLimits)->Result<Self,EmfError>{
        Self::parse_policy(bytes,limits,false,extension_limits)
    }

    fn parse_policy(bytes: &'a [u8], limits: EmfReadLimits, allow_legacy: bool, extension_limits:super::extension::EmfExtensionLimits) -> Result<Self, EmfError> {
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
        if header.version == super::header::EMF_LEGACY_VERSION && !allow_legacy {
            return Err(EmfError::LegacyIntegrityRequiresOptIn);
        }
        let full_hash = matches!(header.version,EMF_CONTAINER_VERSION|EMF_EXTENDED_VERSION);
        let extension = if header.version==EMF_EXTENDED_VERSION {
            if header.flags & !7 != 0 {return Err(EmfError::ExtensionMetadata);}
            Some(super::extension::parse_extension(bytes,extension_limits)?)
        } else {None};
        let field_end=extension.as_ref().map(|e|e.field_end).unwrap_or(bytes.len());
        if full_hash && header.flags & EMF_FLAG_BAO_VERIFIED != 0 {
            return Err(EmfError::InvalidIntegrityEncoding { chunk_index: 0 });
        }

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

        if directory_end > field_end {
            return Err(EmfError::DirectoryBounds);
        }

        let mut directory_sorted=true;let mut previous_key=None;let mut equal_run=0usize;let mut max_equal_run=0usize;
        for chunk in 0..chunk_count {
            let offset = directory_offset + chunk * CHUNK_DIRECTORY_ENTRY_BYTES;
            let entry: EmfChunkDirectoryEntry =
                bytemuck::pod_read_unaligned(&bytes[offset..offset + CHUNK_DIRECTORY_ENTRY_BYTES]);
            entry.validate()?;
            if previous_key.is_some_and(|key|key>entry.spatial_hilbert_index){directory_sorted=false;}
            equal_run=if previous_key==Some(entry.spatial_hilbert_index){equal_run+1}else{1};
            max_equal_run=max_equal_run.max(equal_run);previous_key=Some(entry.spatial_hilbert_index);

            let start = usize::try_from(entry.byte_offset).map_err(|_| EmfError::ArithmeticOverflow)?;
            let length = usize::try_from(entry.byte_length).map_err(|_| EmfError::ArithmeticOverflow)?;
            let end = start.checked_add(length).ok_or(EmfError::ArithmeticOverflow)?;

            if start % 64 != 0 {
                return Err(EmfError::PayloadUnaligned);
            }
            if end > field_end {
                return Err(EmfError::PayloadOutOfBounds);
            }
            if start < 64 || ranges_overlap(start, end, directory_offset, directory_end) {
                return Err(EmfError::PayloadOverlap);
            }
            let expected_bytes = usize::try_from(entry.cell_count)
                .map_err(|_| EmfError::ArithmeticOverflow)?
                .checked_mul(64)
                .ok_or(EmfError::ArithmeticOverflow)?;
            let stored_bytes = expected_bytes.checked_add(if full_hash { BLAKE3_TRAILER_BYTES } else { 0 })
                .ok_or(EmfError::ArithmeticOverflow)?;
            if length != stored_bytes {
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

            if full_hash {
                if entry.flags & EMF_CHUNK_FLAG_BLAKE3_FULL == 0 || entry.flags & 2 != 0
                    || entry.bao_hash_prefix != [0; 16] || entry.reserved != [0; 12] {
                    return Err(EmfError::InvalidIntegrityEncoding { chunk_index: chunk });
                }
                let records_end = start + expected_bytes;
                if !verify_blake3_trailer(&bytes[start..records_end], &bytes[records_end..end]) {
                    return Err(EmfError::FullHashMismatch { chunk_index: chunk });
                }
            } else if (entry.flags & 2 != 0) || (entry.bao_hash_prefix != [0u8; 16]) {
                // Legacy migration only: reproduce the defective comparison, never call it Bao.
                let payload = &bytes[start..end];
                let digest = blake3::hash(payload);
                if digest.as_bytes()[..16] != entry.bao_hash_prefix {
                    return Err(EmfError::BaoHashMismatch { chunk_index: chunk });
                }
            }
        }

        let lookup_work_bound=if directory_sorted{(usize::BITS-chunk_count.leading_zeros()) as usize+max_equal_run+1}else{chunk_count}.max(1);
        Ok(Self {
            bytes,
            header,
            directory_offset,
            chunk_count,
            directory_sorted,
            lookup_work_bound,
            extension,
        }).and_then(|reader| {
            if let Some(extension)=&reader.extension {
                super::extension::validate_field_directory(&reader.header,reader.directory_offset,reader.chunk_count,reader.bytes,extension)?;
            }
            Ok(reader)
        })
    }

    /// Access the parsed container header.
    #[inline]
    pub fn header(&self) -> &EmfContainerHeader {
        &self.header
    }

    /// Typed v3 interpretation and verified original PCM; absent for repaired v2.
    pub fn extension(&self)->Option<&super::extension::EmfExtensionView<'a>>{self.extension.as_ref()}

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
        let end = start + entry.cell_count as usize * 64;
        self.bytes.get(start..end)
    }

    /// Read a specific 64B cell from a chunk.
    pub fn cell(&self, chunk_index: usize, cell_index: usize) -> Option<EmfCellRecord64> {
        let payload = self.chunk_payload(chunk_index)?;
        let start = cell_index.checked_mul(64)?;
        let raw = payload.get(start..start.checked_add(64)?)?;
        Some(bytemuck::pod_read_unaligned(raw))
    }

    /// Legacy spatial-only lookup deterministically selects time index zero.
    /// Use find_chunk_at_time for another stored snapshot.
    pub fn find_chunk_by_hilbert(&self,hilbert_key:u64)->Option<usize>{self.find_chunk_at_time(hilbert_key,0)}

    /// Exact spatial/time lookup. V3 canonical directories use a lower-bound
    /// binary search then scan only equal spatial keys; unsorted v2 is bounded linear.
    pub fn find_chunk_at_time(&self,hilbert_key:u64,time_slice:u32)->Option<usize>{
        if !self.directory_sorted {
            return (0..self.chunk_count).find(|&index|self.chunk_entry(index).is_some_and(|entry|entry.spatial_hilbert_index==hilbert_key&&entry.time_slice_index==time_slice));
        }
        let mut low=0usize;let mut high=self.chunk_count;
        while low<high {let middle=low+(high-low)/2;if self.chunk_entry(middle)?.spatial_hilbert_index<hilbert_key{low=middle+1;}else{high=middle;}}
        while low<self.chunk_count {
            let entry=self.chunk_entry(low)?;if entry.spatial_hilbert_index!=hilbert_key{return None;}
            if entry.time_slice_index==time_slice{return Some(low);}low+=1;
        }None
    }

    /// Conservative directory visits per exact lookup, for caller work admission.
    pub fn lookup_work_bound(&self)->usize{self.lookup_work_bound}

    /// Direct lookup of a voxel cell using brick coordinates `(bx, by, bz)` and local coords `(lx, ly, lz)`.
    pub fn lookup_brick_cell(&self,bx:u32,by:u32,bz:u32,lx:u8,ly:u8,lz:u8,order:u32)->Option<EmfCellRecord64>{
        self.lookup_brick_cell_at_time(bx,by,bz,lx,ly,lz,order,0)
    }

    pub fn lookup_brick_cell_at_time(&self,bx:u32,by:u32,bz:u32,lx:u8,ly:u8,lz:u8,order:u32,time_slice:u32)->Option<EmfCellRecord64>{
        if lx>=8||ly>=8||lz>=8||order==0||order>21{return None;}
        if bx>=(1u32<<order)||by>=(1u32<<order)||bz>=(1u32<<order){return None;}
        let hilbert_key=encode_hilbert3d_u64(bx,by,bz,order);
        let chunk=self.find_chunk_at_time(hilbert_key,time_slice)?;
        self.cell(chunk,emf_brick_voxel_linear_index(lx,ly,lz))
    }

    /// Unpack Hilbert 8³ bricks into a dense `[t][z][y][x]` grid of scientific cells.
    ///
    /// This decodes physical fields only. V3 independent PCM and interpretation
    /// remain available through extension(); they must not be inferred from sigma.
    /// `.10d` MID2/FIEL names the field; it does not duplicate these cells.
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
        let order = match &self.extension {
            Some(extension)=>u32::from(extension.metadata().field.hilbert_order),
            None=>infer_container_hilbert_order(self,dims)?,
        };

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
