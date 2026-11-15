use super::{
    range::{extent, nonoverlap, overlap},
    AuthenticatedRange, FormatError as E, FormatLimits, PayloadInterval,
};
use crate::q42::{
    chunk_integrity::{verify_blake3_trailer, BLAKE3_TRAILER_BYTES},
    m64_matter::{
        M64ChunkDirectoryEntry, M64Header, M64LocalMetricHeader, M64PhaseJunctureRecord,
        M64ShellRecord, M64StrataColumnRecord, LRM3_BLOCK_BYTES, M64_CODEC_BLAKE3_FULL,
        M64_FLAG_3D_BRICK_16X16X16, M64_FLAG_LOCAL_METRIC_LRM3, M64_MODE_HOLO,
        M64_MODE_PHASE_JUNCTURE, M64_MODE_POINT, M64_MODE_SHELL, M64_MODE_STRATA,
    },
};
/// M64/E64 full-digest v1/v2 selective directory view. V3 raster metadata has a
/// distinct closure and refuses here until that complete closure is supplied.
pub struct M64DirectoryView<'a> {
    header: M64Header,
    directory: AuthenticatedRange<'a>,
    lrm3: Option<M64LocalMetricHeader>,
    limits: FormatLimits,
}
pub struct M64ChunkView<'a> {
    entry: M64ChunkDirectoryEntry,
    records: &'a [u8],
}
impl<'a> M64ChunkView<'a> {
    pub const fn entry(&self) -> &M64ChunkDirectoryEntry {
        &self.entry
    }
    pub fn records(&self) -> &'a [u8] {
        self.records
    }
    pub fn record(&self, index: usize) -> Option<&'a [u8]> {
        let start = index.checked_mul(64)?;
        self.records.get(start..start.checked_add(64)?)
    }
}
impl<'a> M64DirectoryView<'a> {
    pub fn admit(
        header: AuthenticatedRange<'a>,
        directory: AuthenticatedRange<'a>,
        local_metric: Option<AuthenticatedRange<'a>>,
        limits: FormatLimits,
        max_records: u64,
        scratch: &mut [PayloadInterval],
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
        let h: M64Header = bytemuck::pod_read_unaligned(header.bytes());
        h.validate().map_err(|_| E::Integrity)?;
        if !matches!(h.version, 1 | 2)
            || h.container_flags
                & !((M64_FLAG_LOCAL_METRIC_LRM3 | M64_FLAG_3D_BRICK_16X16X16) as u32)
                != 0
        {
            return Err(E::Unsupported);
        }
        if h.total_file_size != header.selected().logical_length() {
            return Err(E::Geometry);
        }
        let count = usize::try_from(h.chunk_count).map_err(|_| E::Limit)?;
        if count > limits.max_rows || count > 4096 {
            return Err(E::Limit);
        }
        if scratch.len() < count {
            return Err(E::Capacity);
        }
        let bytes = count.checked_mul(64).ok_or(E::Bounds)?;
        let mut metadata_bytes = 64usize.checked_add(bytes).ok_or(E::Bounds)?;
        let mut metadata_end = u64::from(h.header_size);
        let lrm3 = if h.container_flags & M64_FLAG_LOCAL_METRIC_LRM3 as u32 != 0 {
            let metric = local_metric.ok_or(E::Bounds)?;
            if !header.same_entry(&metric) {
                return Err(E::Identity);
            }
            if metric.offset() != metadata_end || metric.bytes().len() != LRM3_BLOCK_BYTES {
                return Err(E::Bounds);
            }
            let block: M64LocalMetricHeader = bytemuck::pod_read_unaligned(metric.bytes());
            block.validate().map_err(|_| E::Integrity)?;
            metadata_bytes = metadata_bytes
                .checked_add(LRM3_BLOCK_BYTES)
                .ok_or(E::Bounds)?;
            metadata_end = extent(metadata_end, LRM3_BLOCK_BYTES as u64, h.total_file_size)?;
            Some(block)
        } else {
            if local_metric.is_some() {
                return Err(E::Unsupported);
            }
            None
        };
        if metadata_bytes > limits.max_metadata_bytes {
            return Err(E::Limit);
        }
        let dstart = h.chunk_table_offset;
        let dend = extent(dstart, bytes as u64, h.total_file_size)?;
        if dstart < metadata_end
            || dstart % 64 != 0
            || directory.offset() != dstart
            || directory.bytes().len() != bytes
        {
            return Err(E::Geometry);
        }
        let mut records = 0u64;
        for (i, raw) in directory.bytes().chunks_exact(64).enumerate() {
            let row: M64ChunkDirectoryEntry = bytemuck::pod_read_unaligned(raw);
            row.validate().map_err(|_| E::Integrity)?;
            if !matches!(
                row.chunk_record_mode,
                M64_MODE_POINT
                    | M64_MODE_HOLO
                    | M64_MODE_STRATA
                    | M64_MODE_SHELL
                    | M64_MODE_PHASE_JUNCTURE
            ) {
                return Err(E::Unsupported);
            }
            if row.compression_flags != M64_CODEC_BLAKE3_FULL || row.bao_root_hash_prefix != [0; 12]
            {
                return Err(E::Integrity);
            }
            let length = u64::from(row.record_count)
                .checked_mul(64)
                .and_then(|n| n.checked_add(BLAKE3_TRAILER_BYTES as u64))
                .ok_or(E::Bounds)?;
            if row.record_count == 0 || length != u64::from(row.byte_length) {
                return Err(E::Geometry);
            }
            records = records
                .checked_add(u64::from(row.record_count))
                .ok_or(E::Bounds)?;
            if records > max_records {
                return Err(E::Limit);
            }
            let end = extent(row.byte_offset, length, h.total_file_size)?;
            if row.byte_offset < metadata_end
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
            lrm3,
            limits,
        })
    }
    pub const fn header(&self) -> &M64Header {
        &self.header
    }
    pub const fn local_metric(&self) -> Option<&M64LocalMetricHeader> {
        self.lrm3.as_ref()
    }
    pub fn selected_chunk(
        &self,
        index: usize,
        payload: AuthenticatedRange<'a>,
    ) -> Result<M64ChunkView<'a>, E> {
        if !self.directory.same_entry(&payload) {
            return Err(E::Identity);
        }
        let start = index.checked_mul(64).ok_or(E::Bounds)?;
        let raw = self
            .directory
            .bytes()
            .get(start..start.checked_add(64).ok_or(E::Bounds)?)
            .ok_or(E::Bounds)?;
        let row: M64ChunkDirectoryEntry = bytemuck::pod_read_unaligned(raw);
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
        for raw in records.chunks_exact(64) {
            let valid = match row.chunk_record_mode {
                M64_MODE_STRATA => {
                    bytemuck::pod_read_unaligned::<M64StrataColumnRecord>(raw).validate()
                }
                M64_MODE_SHELL => bytemuck::pod_read_unaligned::<M64ShellRecord>(raw).validate(),
                M64_MODE_PHASE_JUNCTURE => {
                    bytemuck::pod_read_unaligned::<M64PhaseJunctureRecord>(raw).validate()
                }
                M64_MODE_POINT | M64_MODE_HOLO => Ok(()),
                _ => return Err(E::Unsupported),
            };
            valid.map_err(|_| E::Integrity)?;
        }
        Ok(M64ChunkView {
            entry: row,
            records,
        })
    }
}
