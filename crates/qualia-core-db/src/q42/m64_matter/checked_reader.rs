//! Bounded structural reader for little-endian M64/E64 containers.
//!
//! Header and directory PODs are copied to aligned stack values; payload bytes
//! remain borrowed zero-copy slices. Supports BLAKE3-Bao digest prefix verification.

use super::{
    M64ChunkDirectoryEntry, M64Error, M64GeoRasterError, M64GeoRasterMetadata, M64Header,
    M64PhaseJunctureRecord, M64RasterSample, M64StrataColumnRecord, M64_FLAG_3D_BRICK_16X16X16,
    M64_FLAG_LOCAL_METRIC_LRM3, M64_MAGIC, M64_MODE_HOLO, M64_MODE_PHASE_JUNCTURE,
    M64_MODE_POINT, M64_MODE_STRATA,
};

/// Explicit admission budgets; there is no implicit unlimited/default policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct M64ReadLimits {
    pub max_file_bytes: usize,
    pub max_chunks: usize,
    pub max_records: u64,
}

/// Caller-owned overlap scratch. One element per directory entry is required.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct M64PayloadInterval {
    pub start: usize,
    pub end: usize,
}

/// Verification metrics collected during structural validation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct M64Verification {
    pub chunks: usize,
    pub total_records: u64,
    pub record_crc_checked: u64,
    pub records_without_checksum: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum M64ReadError {
    GeoRaster(M64GeoRasterError),
    UnsupportedEndian,
    FileByteLimit,
    TruncatedHeader,
    Header(M64Error),
    UnsupportedVersion(u32),
    UnsupportedContainerFlags(u32),
    FileSizeMismatch,
    ChunkLimit,
    RecordLimit,
    ScratchTooSmall { required: usize, provided: usize },
    ArithmeticOverflow,
    DirectoryBounds,
    DirectoryAlignment,
    Directory { chunk: usize, error: M64Error },
    UnsupportedMode { chunk: usize, mode: u8 },
    UnsupportedCodec { chunk: usize, codec: u8 },
    PayloadBounds { chunk: usize },
    PayloadAlignment { chunk: usize },
    RecordLength { chunk: usize },
    MetadataOverlap { chunk: usize },
    PayloadOverlap,
    Record { chunk: usize, record: usize, error: M64Error },
    BaoHashMismatch { chunk_index: usize },
}

impl std::fmt::Display for M64ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for M64ReadError {}

fn sift_down(values: &mut [M64PayloadInterval], mut root: usize, end: usize) {
    while root < end / 2 {
        let mut child = root * 2 + 1;
        if child + 1 < end && values[child].start < values[child + 1].start {
            child += 1;
        }
        if values[root].start >= values[child].start {
            break;
        }
        values.swap(root, child);
        root = child;
    }
}

fn has_overlap(values: &mut [M64PayloadInterval]) -> bool {
    for root in (0..values.len() / 2).rev() {
        sift_down(values, root, values.len());
    }
    for end in (1..values.len()).rev() {
        values.swap(0, end);
        sift_down(values, 0, end);
    }
    values.windows(2).any(|pair| pair[0].end > pair[1].start)
}

/// A fully structurally checked immutable file view.
#[derive(Debug)]
pub struct M64Reader<'a> {
    bytes: &'a [u8],
    header: M64Header,
    directory_start: usize,
    verification: M64Verification,
    georaster: Option<M64GeoRasterMetadata>,
}

impl<'a> M64Reader<'a> {
    /// Admit the entire file with bounded structural checks.
    pub fn parse(
        bytes: &'a [u8],
        limits: M64ReadLimits,
        scratch: &mut [M64PayloadInterval],
    ) -> Result<Self, M64ReadError> {
        if !cfg!(target_endian = "little") {
            return Err(M64ReadError::UnsupportedEndian);
        }
        if bytes.len() > limits.max_file_bytes {
            return Err(M64ReadError::FileByteLimit);
        }
        let raw_header = bytes.get(..64).ok_or(M64ReadError::TruncatedHeader)?;
        let header: M64Header = bytemuck::pod_read_unaligned(raw_header);
        if !matches!(header.version, 1 | 2 | 3) {
            return Err(M64ReadError::UnsupportedVersion(header.version));
        }
        header.validate().map_err(M64ReadError::Header)?;

        let admitted_flags =
            (M64_FLAG_3D_BRICK_16X16X16 as u32) | (M64_FLAG_LOCAL_METRIC_LRM3 as u32);
        if header.container_flags & !admitted_flags != 0 {
            return Err(M64ReadError::UnsupportedContainerFlags(header.container_flags));
        }
        if header.total_file_size != bytes.len() as u64 {
            return Err(M64ReadError::FileSizeMismatch);
        }

        let georaster = if header.version == 3 {
            let meta = M64GeoRasterMetadata::decode(
                bytes.get(64..192).ok_or(M64ReadError::GeoRaster(M64GeoRasterError::Truncated))?,
            ).map_err(M64ReadError::GeoRaster)?;
            let shape = meta.shape().map_err(M64ReadError::GeoRaster)?;
            if header.magic != M64_MAGIC
                || header.chunk_table_offset != 192
                || header.chunk_count != shape.chunks as u64
                || bytes.len() != shape.file_bytes
            {
                return Err(M64ReadError::GeoRaster(M64GeoRasterError::HeaderSummary));
            }
            Some(meta)
        } else {
            None
        };

        let count = usize::try_from(header.chunk_count).map_err(|_| M64ReadError::ArithmeticOverflow)?;
        if count > limits.max_chunks {
            return Err(M64ReadError::ChunkLimit);
        }
        if scratch.len() < count {
            return Err(M64ReadError::ScratchTooSmall {
                required: count,
                provided: scratch.len(),
            });
        }
        let directory_start =
            usize::try_from(header.chunk_table_offset).map_err(|_| M64ReadError::ArithmeticOverflow)?;
        if directory_start % 64 != 0 {
            return Err(M64ReadError::DirectoryAlignment);
        }
        let directory_end = directory_start
            .checked_add(count.checked_mul(64).ok_or(M64ReadError::ArithmeticOverflow)?)
            .ok_or(M64ReadError::ArithmeticOverflow)?;
        if directory_start < header.header_size as usize || directory_end > bytes.len() {
            return Err(M64ReadError::DirectoryBounds);
        }

        let mut verification = M64Verification {
            chunks: count,
            ..M64Verification::default()
        };

        for (chunk, interval) in scratch[..count].iter_mut().enumerate() {
            let offset = directory_start + chunk * 64;
            let entry: M64ChunkDirectoryEntry =
                bytemuck::pod_read_unaligned(&bytes[offset..offset + 64]);
            entry
                .validate()
                .map_err(|error| M64ReadError::Directory { chunk, error })?;

            if !matches!(
                entry.chunk_record_mode,
                M64_MODE_POINT | M64_MODE_HOLO | M64_MODE_STRATA | M64_MODE_PHASE_JUNCTURE
            ) {
                return Err(M64ReadError::UnsupportedMode {
                    chunk,
                    mode: entry.chunk_record_mode,
                });
            }
            // Codec 0: Uncompressed, Codec 2: Bao verified outboard hash tree
            if entry.compression_flags != 0 && entry.compression_flags != 2 {
                return Err(M64ReadError::UnsupportedCodec {
                    chunk,
                    codec: entry.compression_flags,
                });
            }

            let expected_length = u64::from(entry.record_count)
                .checked_mul(64)
                .ok_or(M64ReadError::ArithmeticOverflow)?;
            if entry.record_count == 0 || expected_length != u64::from(entry.byte_length) {
                return Err(M64ReadError::RecordLength { chunk });
            }
            verification.total_records = verification
                .total_records
                .checked_add(u64::from(entry.record_count))
                .ok_or(M64ReadError::ArithmeticOverflow)?;
            if verification.total_records > limits.max_records {
                return Err(M64ReadError::RecordLimit);
            }

            let start = usize::try_from(entry.byte_offset).map_err(|_| M64ReadError::ArithmeticOverflow)?;
            let end = start
                .checked_add(entry.byte_length as usize)
                .ok_or(M64ReadError::ArithmeticOverflow)?;
            if start % 64 != 0 {
                return Err(M64ReadError::PayloadAlignment { chunk });
            }
            if end > bytes.len() {
                return Err(M64ReadError::PayloadBounds { chunk });
            }
            if start < header.header_size as usize || (start < directory_end && end > directory_start) {
                return Err(M64ReadError::MetadataOverlap { chunk });
            }

            // BLAKE3-Bao root hash prefix verification
            if entry.compression_flags == 2 || entry.bao_root_hash_prefix != [0u8; 12] {
                let chunk_payload = &bytes[start..end];
                let digest = blake3::hash(chunk_payload);
                if digest.as_bytes()[..12] != entry.bao_root_hash_prefix {
                    return Err(M64ReadError::BaoHashMismatch { chunk_index: chunk });
                }
            }

            *interval = M64PayloadInterval { start, end };
        }

        if has_overlap(&mut scratch[..count]) {
            return Err(M64ReadError::PayloadOverlap);
        }

        for chunk in 0..count {
            let offset = directory_start + chunk * 64;
            let entry: M64ChunkDirectoryEntry =
                bytemuck::pod_read_unaligned(&bytes[offset..offset + 64]);
            if matches!(entry.chunk_record_mode, M64_MODE_POINT | M64_MODE_HOLO) {
                verification.records_without_checksum += u64::from(entry.record_count);
                continue;
            }
            let start = entry.byte_offset as usize;
            let end = start + entry.byte_length as usize;
            for (record, raw) in bytes[start..end].chunks_exact(64).enumerate() {
                let result = if entry.chunk_record_mode == M64_MODE_STRATA {
                    bytemuck::pod_read_unaligned::<M64StrataColumnRecord>(raw).validate()
                } else {
                    bytemuck::pod_read_unaligned::<M64PhaseJunctureRecord>(raw).validate()
                };
                result.map_err(|error| M64ReadError::Record { chunk, record, error })?;
                verification.record_crc_checked += 1;
            }
        }

        if let Some(meta) = &georaster {
            super::georaster::validate_container(bytes, &header, meta)
                .map_err(M64ReadError::GeoRaster)?;
        }

        Ok(M64Reader {
            bytes,
            header,
            directory_start,
            verification,
            georaster,
        })
    }

    pub fn header(&self) -> &M64Header {
        &self.header
    }

    pub fn georaster(&self) -> Option<&M64GeoRasterMetadata> {
        self.georaster.as_ref()
    }

    pub fn raster_sample(&self, x: usize, y: usize) -> Option<M64RasterSample> {
        let meta = self.georaster.as_ref()?;
        let shape = meta.shape().ok()?;
        if x >= shape.width || y >= shape.height {
            return None;
        }
        let chunk = self.chunk((y / 64) * shape.chunks_x + x / 64)?;
        let raw = chunk.record_bytes((y % 64) * 64 + x % 64)?;
        let value = i32::from_le_bytes(raw[..4].try_into().ok()?);
        Some(if meta.nodata_sentinel_mm == Some(value) {
            M64RasterSample::Nodata(value)
        } else {
            M64RasterSample::ElevationMm(value)
        })
    }

    pub fn verification(&self) -> M64Verification {
        self.verification
    }

    pub fn chunk(&self, index: usize) -> Option<M64Chunk<'a>> {
        if index >= self.verification.chunks {
            return None;
        }
        let offset = self.directory_start + index * 64;
        let entry: M64ChunkDirectoryEntry =
            bytemuck::pod_read_unaligned(&self.bytes[offset..offset + 64]);
        let start = entry.byte_offset as usize;
        let end = start + entry.byte_length as usize;
        Some(M64Chunk {
            entry,
            payload: &self.bytes[start..end],
        })
    }
}

#[derive(Debug)]
pub struct M64Chunk<'a> {
    entry: M64ChunkDirectoryEntry,
    payload: &'a [u8],
}

impl<'a> M64Chunk<'a> {
    pub fn directory_entry(&self) -> &M64ChunkDirectoryEntry {
        &self.entry
    }

    pub fn payload(&self) -> &'a [u8] {
        self.payload
    }

    pub fn record_bytes(&self, index: usize) -> Option<&'a [u8]> {
        let start = index.checked_mul(64)?;
        self.payload.get(start..start.checked_add(64)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checked_reader_bao_verification_and_tampering() {
        let payload = [0x42u8; 64];
        let digest = blake3::hash(&payload);
        let mut bao_prefix = [0u8; 12];
        bao_prefix.copy_from_slice(&digest.as_bytes()[..12]);

        let mut header = M64Header::default();
        header.total_file_size = 192; // 64B header + 64B dir + 64B payload
        header.chunk_count = 1;
        header.chunk_table_offset = 64;
        header.update_crc();

        let mut entry = M64ChunkDirectoryEntry {
            chunk_uuid: [0; 16],
            spatial_hilbert_index: 0,
            byte_offset: 128,
            byte_length: 64,
            record_count: 1,
            srid_geodetic_code: 0,
            lut_bank_epoch_id: 0,
            chunk_record_mode: M64_MODE_POINT,
            compression_flags: 2, // Bao verified
            bao_root_hash_prefix: bao_prefix,
            entry_crc32c: 0,
        };
        entry.update_crc();

        let mut buffer = vec![0u8; 192];
        buffer[..64].copy_from_slice(bytemuck::bytes_of(&header));
        buffer[64..128].copy_from_slice(bytemuck::bytes_of(&entry));
        buffer[128..192].copy_from_slice(&payload);

        let limits = M64ReadLimits {
            max_file_bytes: 1024,
            max_chunks: 16,
            max_records: 16,
        };
        let mut scratch = [M64PayloadInterval::default(); 16];

        // Valid payload with Bao verification succeeds
        let reader = M64Reader::parse(&buffer, limits, &mut scratch);
        assert!(reader.is_ok(), "Expected valid Bao verification to pass");

        // Tamper with chunk payload byte
        buffer[130] ^= 0xFF;
        let tampered_result = M64Reader::parse(&buffer, limits, &mut scratch);
        assert_eq!(
            tampered_result.err(),
            Some(M64ReadError::BaoHashMismatch { chunk_index: 0 }),
            "Expected BaoHashMismatch on tampered payload"
        );
    }
}
