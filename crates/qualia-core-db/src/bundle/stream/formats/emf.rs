use super::{
    range::{extent, nonoverlap, overlap},
    AuthenticatedRange, FormatError as E, FormatLimits, PayloadInterval,
};
use crate::q42::{
    chunk_integrity::{verify_blake3_trailer, BLAKE3_TRAILER_BYTES},
    emf_container::{
        chunk_directory::{EmfChunkDirectoryEntry, CHUNK_DIRECTORY_ENTRY_BYTES},
        header::{
            EmfCellRecord64, EmfContainerHeader, EMF_CHUNK_FLAG_BLAKE3_FULL, EMF_CONTAINER_VERSION,
        },
    },
};
/// Authenticated complete v2 directory with global payload extent consistency.
/// V3 interpretation/PCM closure is exposed separately by EmfV3DirectoryView.
pub struct EmfDirectoryView<'a> {
    header: EmfContainerHeader,
    directory: AuthenticatedRange<'a>,
    limits: FormatLimits,
}
pub struct EmfChunkView<'a> {
    entry: EmfChunkDirectoryEntry,
    records: &'a [u8],
}
impl<'a> EmfChunkView<'a> {
    pub const fn entry(&self) -> &EmfChunkDirectoryEntry {
        &self.entry
    }
    pub fn records(&self) -> &'a [u8] {
        self.records
    }
    pub fn cell(&self, index: usize) -> Option<EmfCellRecord64> {
        let start = index.checked_mul(64)?;
        let end = start.checked_add(64)?;
        self.records
            .get(start..end)
            .map(bytemuck::pod_read_unaligned)
    }
}
impl<'a> EmfDirectoryView<'a> {
    pub fn admit(
        header: AuthenticatedRange<'a>,
        directory: AuthenticatedRange<'a>,
        limits: FormatLimits,
        scratch: &mut [PayloadInterval],
    ) -> Result<Self, E> {
        Self::admit_version(header, directory, limits, scratch, EMF_CONTAINER_VERSION)
    }
    pub(super) fn admit_version(
        header: AuthenticatedRange<'a>,
        directory: AuthenticatedRange<'a>,
        limits: FormatLimits,
        scratch: &mut [PayloadInterval],
        expected_version: u16,
    ) -> Result<Self, E> {
        if !cfg!(target_endian = "little") {
            return Err(E::Unsupported);
        }
        if !header.same_entry(&directory) {
            return Err(E::Identity);
        }
        if header.offset() != 0 || header.bytes().len() != 64 {
            return Err(E::Bounds);
        }
        if header.selected().logical_length() > limits.max_entry_bytes {
            return Err(E::Limit);
        }
        let h: EmfContainerHeader = bytemuck::pod_read_unaligned(header.bytes());
        h.validate().map_err(|_| E::Integrity)?;
        if h.version != expected_version || h.flags & !7 != 0 {
            return Err(E::Unsupported);
        }
        let count = usize::try_from(h.chunk_count).map_err(|_| E::Limit)?;
        if count > limits.max_rows || count > 4096 {
            return Err(E::Limit);
        }
        let bytes = count
            .checked_mul(CHUNK_DIRECTORY_ENTRY_BYTES)
            .ok_or(E::Bounds)?;
        if bytes.checked_add(64).ok_or(E::Bounds)? > limits.max_metadata_bytes {
            return Err(E::Limit);
        }
        if scratch.len() < count {
            return Err(E::Capacity);
        }
        let dstart = h.directory_offset;
        let dend = extent(dstart, bytes as u64, header.selected().logical_length())?;
        if dstart < 64
            || dstart % 64 != 0
            || directory.offset() != dstart
            || directory.bytes().len() != bytes
        {
            return Err(E::Geometry);
        }
        for (i, raw) in directory.bytes().chunks_exact(64).enumerate() {
            let row: EmfChunkDirectoryEntry = bytemuck::pod_read_unaligned(raw);
            row.validate().map_err(|_| E::Integrity)?;
            let records = u64::from(row.cell_count).checked_mul(64).ok_or(E::Bounds)?;
            let length = records
                .checked_add(BLAKE3_TRAILER_BYTES as u64)
                .ok_or(E::Bounds)?;
            if length != u64::from(row.byte_length)
                || row.flags & EMF_CHUNK_FLAG_BLAKE3_FULL == 0
                || row.flags & 2 != 0
                || row.bao_hash_prefix != [0; 16]
                || row.reserved != [0; 12]
            {
                return Err(E::Integrity);
            }
            let end = extent(row.byte_offset, length, header.selected().logical_length())?;
            if row.byte_offset < 64
                || row.byte_offset % 64 != 0
                || overlap(row.byte_offset, end, dstart, dend)
            {
                return Err(E::Geometry);
            }
            scratch[i] = PayloadInterval {
                start: row.byte_offset,
                end,
            };
        }
        nonoverlap(&mut scratch[..count])?;
        Ok(Self {
            header: h,
            directory,
            limits,
        })
    }
    pub const fn header(&self) -> &EmfContainerHeader {
        &self.header
    }
    pub fn selected_chunk(
        &self,
        index: usize,
        payload: AuthenticatedRange<'a>,
    ) -> Result<EmfChunkView<'a>, E> {
        if !self.directory.same_entry(&payload) {
            return Err(E::Identity);
        }
        let start = index.checked_mul(64).ok_or(E::Bounds)?;
        let raw = self
            .directory
            .bytes()
            .get(start..start.checked_add(64).ok_or(E::Bounds)?)
            .ok_or(E::Bounds)?;
        let row: EmfChunkDirectoryEntry = bytemuck::pod_read_unaligned(raw);
        let length = usize::try_from(row.byte_length).map_err(|_| E::Limit)?;
        if length > self.limits.max_chunk_bytes {
            return Err(E::Limit);
        }
        if payload.offset() != row.byte_offset || payload.bytes().len() != length {
            return Err(E::Bounds);
        }
        let split = length.checked_sub(BLAKE3_TRAILER_BYTES).ok_or(E::Bounds)?;
        let (records, trailer) = payload.bytes().split_at(split);
        if !verify_blake3_trailer(records, trailer) {
            return Err(E::Integrity);
        }
        Ok(EmfChunkView {
            entry: row,
            records,
        })
    }
}
