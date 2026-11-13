//! Bounded structural reader for little-endian M64/E64 containers.
//!
//! Header and directory PODs are copied to aligned stack values; payload bytes
//! remain borrowed zero-copy slices. Supports full BLAKE3 digests; truncated legacy hashes require migration opt-in.

use crate::q42::chunk_integrity::{verify_blake3_trailer, BLAKE3_TRAILER_BYTES};
use super::M64_CODEC_BLAKE3_FULL;

use super::{
    M64ChunkDirectoryEntry, M64Error, M64GeoRasterError, M64GeoRasterMetadata, M64Header,
    M64LocalMetricHeader, M64PhaseJunctureRecord, M64RasterSample, M64ShellRecord,
    M64StrataColumnRecord, LRM3_BLOCK_BYTES, M64_FLAG_3D_BRICK_16X16X16,
    M64_FLAG_LOCAL_METRIC_LRM3, M64_MAGIC, M64_MODE_HOLO, M64_MODE_PHASE_JUNCTURE, M64_MODE_POINT,
    M64_MODE_SHELL, M64_MODE_STRATA,
};

#[cfg(test)]
use super::{E64_MAGIC, M64HoloRecord, M64PointRecord};

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
    pub full_hash_chunks: usize,
    pub legacy_prefix_chunks: usize,
    pub records_without_checksum: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum M64ReadError {
    GeoRaster(M64GeoRasterError),
    UnsupportedEndian,
    FileByteLimit,
    TruncatedHeader,
    Header(M64Error),
    Lrm3Missing,
    Lrm3(M64Error),
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
    FullHashMismatch { chunk_index: usize },
    LegacyIntegrityRequiresOptIn { chunk_index: usize },
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
    lrm3: Option<M64LocalMetricHeader>,
}

impl<'a> M64Reader<'a> {
    /// Admit the entire file with bounded structural checks.
    pub fn parse(
        bytes: &'a [u8],
        limits: M64ReadLimits,
        scratch: &mut [M64PayloadInterval],
    ) -> Result<Self, M64ReadError> {
        Self::parse_policy(bytes, limits, scratch, false)
    }

    /// Migration-only admission of defective legacy truncated hashes.
    /// `legacy_prefix_chunks` reports these separately from full verification.
    pub fn parse_legacy_with_limits(
        bytes: &'a [u8], limits: M64ReadLimits, scratch: &mut [M64PayloadInterval],
    ) -> Result<Self, M64ReadError> {
        Self::parse_policy(bytes, limits, scratch, true)
    }

    fn parse_policy(
        bytes: &'a [u8], limits: M64ReadLimits, scratch: &mut [M64PayloadInterval],
        allow_legacy: bool,
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

        let lrm3 = if header.container_flags & (M64_FLAG_LOCAL_METRIC_LRM3 as u32) != 0 {
            let header_size = usize::try_from(header.header_size)
                .map_err(|_| M64ReadError::ArithmeticOverflow)?;
            let lrm3_end = header_size
                .checked_add(LRM3_BLOCK_BYTES)
                .ok_or(M64ReadError::ArithmeticOverflow)?;
            if lrm3_end > directory_start || lrm3_end > bytes.len() {
                return Err(M64ReadError::Lrm3Missing);
            }
            let block: M64LocalMetricHeader =
                bytemuck::pod_read_unaligned(&bytes[header_size..lrm3_end]);
            block.validate().map_err(M64ReadError::Lrm3)?;
            Some(block)
        } else {
            None
        };

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
                M64_MODE_POINT
                    | M64_MODE_HOLO
                    | M64_MODE_STRATA
                    | M64_MODE_PHASE_JUNCTURE
                    | M64_MODE_SHELL
            ) {
                return Err(M64ReadError::UnsupportedMode {
                    chunk,
                    mode: entry.chunk_record_mode,
                });
            }
            // Codec 2 is legacy migration only; codec 3 verifies all 256 digest bits.
            if !matches!(entry.compression_flags, 0 | 2 | M64_CODEC_BLAKE3_FULL) {
                return Err(M64ReadError::UnsupportedCodec {
                    chunk,
                    codec: entry.compression_flags,
                });
            }

            let expected_length = u64::from(entry.record_count)
                .checked_mul(64)
                .ok_or(M64ReadError::ArithmeticOverflow)?;
            let full_hash = entry.compression_flags == M64_CODEC_BLAKE3_FULL;
            let stored_length = expected_length.checked_add(if full_hash { BLAKE3_TRAILER_BYTES as u64 } else { 0 })
                .ok_or(M64ReadError::ArithmeticOverflow)?;
            if entry.record_count == 0 || stored_length != u64::from(entry.byte_length) {
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
            // LRM3 lives in [header_size, header_size+LRM3_BLOCK_BYTES). CRC covers those
            // bytes, but a payload alias of the authenticated gap is still overlap.
            let header_end = header.header_size as usize;
            let metadata_end = if lrm3.is_some() {
                header_end + LRM3_BLOCK_BYTES
            } else {
                header_end
            };
            if start < metadata_end || (start < directory_end && end > directory_start) {
                return Err(M64ReadError::MetadataOverlap { chunk });
            }

            if full_hash {
                let record_end = end - BLAKE3_TRAILER_BYTES;
                if entry.bao_root_hash_prefix != [0; 12]
                    || !verify_blake3_trailer(&bytes[start..record_end], &bytes[record_end..end]) {
                    return Err(M64ReadError::FullHashMismatch { chunk_index: chunk });
                }
                verification.full_hash_chunks += 1;
            } else if entry.compression_flags == 2 || entry.bao_root_hash_prefix != [0u8; 12] {
                if !allow_legacy {
                    return Err(M64ReadError::LegacyIntegrityRequiresOptIn { chunk_index: chunk });
                }
                verification.legacy_prefix_chunks += 1;
                // Defective historical comparison retained ONLY for explicit migration.
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
            let end = start + entry.record_count as usize * 64;
            for (record, raw) in bytes[start..end].chunks_exact(64).enumerate() {
                let result = match entry.chunk_record_mode {
                    M64_MODE_STRATA => {
                        bytemuck::pod_read_unaligned::<M64StrataColumnRecord>(raw).validate()
                    }
                    M64_MODE_SHELL => {
                        bytemuck::pod_read_unaligned::<M64ShellRecord>(raw).validate()
                    }
                    M64_MODE_PHASE_JUNCTURE => {
                        bytemuck::pod_read_unaligned::<M64PhaseJunctureRecord>(raw).validate()
                    }
                    M64_MODE_POINT | M64_MODE_HOLO => continue,
                    mode => {
                        return Err(M64ReadError::UnsupportedMode { chunk, mode });
                    }
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
            lrm3,
        })
    }

    pub fn header(&self) -> &M64Header {
        &self.header
    }

    pub fn georaster(&self) -> Option<&M64GeoRasterMetadata> {
        self.georaster.as_ref()
    }

    pub fn lrm3(&self) -> Option<&M64LocalMetricHeader> {
        self.lrm3.as_ref()
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
        let end = start + entry.record_count as usize * 64;
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
    use super::super::has_m64_magic;
    use super::*;

    #[test]
    fn legacy_prefix_requires_opt_in_and_detects_tampering() {
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

        assert!(matches!(M64Reader::parse(&buffer, limits, &mut scratch),
            Err(M64ReadError::LegacyIntegrityRequiresOptIn { .. })));
        let reader = M64Reader::parse_legacy_with_limits(&buffer, limits, &mut scratch).unwrap();
        assert_eq!(reader.verification().legacy_prefix_chunks, 1);
        assert_eq!(reader.verification().full_hash_chunks, 0);

        // Tamper with chunk payload byte
        buffer[130] ^= 0xFF;
        let tampered_result = M64Reader::parse_legacy_with_limits(&buffer, limits, &mut scratch);
        assert_eq!(
            tampered_result.err(),
            Some(M64ReadError::BaoHashMismatch { chunk_index: 0 }),
            "Expected BaoHashMismatch on tampered payload"
        );
    }

    fn one_record_container(magic: [u8; 4], mode: u8, payload: &[u8; 64]) -> Vec<u8> {
        let mut header = M64Header::default();
        header.magic = magic;
        header.total_file_size = 192;
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
            chunk_record_mode: mode,
            compression_flags: 0,
            bao_root_hash_prefix: [0u8; 12],
            entry_crc32c: 0,
        };
        entry.update_crc();
        let mut buffer = vec![0u8; 192];
        buffer[..64].copy_from_slice(bytemuck::bytes_of(&header));
        buffer[64..128].copy_from_slice(bytemuck::bytes_of(&entry));
        buffer[128..192].copy_from_slice(payload);
        buffer
    }

    #[test]
    fn admits_e64_magic_and_shell_mode_with_crc() {
        let mut rec = M64ShellRecord::default();
        rec.face_id = 3;
        rec.update_crc();
        let buffer = one_record_container(E64_MAGIC, M64_MODE_SHELL, bytemuck::cast_ref(&rec));
        let limits = M64ReadLimits {
            max_file_bytes: 1024,
            max_chunks: 4,
            max_records: 4,
        };
        let mut scratch = [M64PayloadInterval::default(); 4];
        let reader = M64Reader::parse(&buffer, limits, &mut scratch).expect("e64 shell");
        assert_eq!(reader.header().magic, E64_MAGIC);
        assert_eq!(reader.verification().record_crc_checked, 1);
        assert_eq!(
            reader.chunk(0).unwrap().directory_entry().chunk_record_mode,
            M64_MODE_SHELL
        );
    }

    #[test]
    fn admits_holo_without_per_record_crc() {
        let rec = M64HoloRecord::default();
        let buffer = one_record_container(M64_MAGIC, M64_MODE_HOLO, bytemuck::cast_ref(&rec));
        let limits = M64ReadLimits {
            max_file_bytes: 1024,
            max_chunks: 4,
            max_records: 4,
        };
        let mut scratch = [M64PayloadInterval::default(); 4];
        let reader = M64Reader::parse(&buffer, limits, &mut scratch).expect("holo");
        assert_eq!(reader.verification().records_without_checksum, 1);
        assert_eq!(reader.verification().record_crc_checked, 0);
        assert_eq!(
            reader.chunk(0).unwrap().directory_entry().chunk_record_mode,
            M64_MODE_HOLO
        );
    }

    fn parse_one(buffer: &[u8]) -> Result<M64Reader<'_>, M64ReadError> {
        let limits = M64ReadLimits {
            max_file_bytes: 1024,
            max_chunks: 4,
            max_records: 4,
        };
        let mut scratch = [M64PayloadInterval::default(); 4];
        M64Reader::parse(buffer, limits, &mut scratch)
    }

    #[test]
    fn admits_point_without_per_record_crc() {
        let rec = M64PointRecord::default();
        let buffer = one_record_container(M64_MAGIC, M64_MODE_POINT, bytemuck::cast_ref(&rec));
        let reader = parse_one(&buffer).expect("point");
        assert_eq!(reader.verification().records_without_checksum, 1);
        assert_eq!(reader.verification().record_crc_checked, 0);
        assert_eq!(
            reader.chunk(0).unwrap().directory_entry().chunk_record_mode,
            M64_MODE_POINT
        );
    }

    #[test]
    fn shell_m64_and_e64_alias_writer_round_trip() {
        let mut rec = M64ShellRecord::default();
        rec.face_id = 11;
        rec.vertex_indices = [1, 2, 3];
        rec.edge_indices = [4, 5, 6];
        rec.crease = 0.25;
        rec.update_crc();
        let payload: &[u8; 64] = bytemuck::cast_ref(&rec);
        for magic in [M64_MAGIC, E64_MAGIC] {
            let buffer = one_record_container(magic, M64_MODE_SHELL, payload);
            assert!(has_m64_magic(&buffer));
            let reader = parse_one(&buffer).expect("shell round-trip");
            assert_eq!(reader.header().magic, magic);
            assert!(has_m64_magic(&reader.header().magic));
            assert_eq!(reader.verification().record_crc_checked, 1);
            assert_eq!(reader.verification().records_without_checksum, 0);
            let chunk = reader.chunk(0).unwrap();
            assert_eq!(chunk.directory_entry().chunk_record_mode, M64_MODE_SHELL);
            assert_eq!(chunk.record_bytes(0).unwrap(), payload.as_slice());
            let decoded: M64ShellRecord =
                bytemuck::pod_read_unaligned(chunk.record_bytes(0).unwrap());
            assert_eq!(decoded, rec);
            assert!(decoded.validate().is_ok());
        }
    }

    #[test]
    fn rejects_shell_record_crc_mismatch() {
        let mut rec = M64ShellRecord::default();
        rec.update_crc();
        let mut buffer = one_record_container(M64_MAGIC, M64_MODE_SHELL, bytemuck::cast_ref(&rec));
        buffer[128] ^= 1;
        assert!(matches!(
            parse_one(&buffer),
            Err(M64ReadError::Record {
                chunk: 0,
                record: 0,
                error: M64Error::ChecksumMismatch { .. },
            })
        ));
    }

    #[test]
    fn strata_terrain_still_uses_record_crc() {
        let mut rec = M64StrataColumnRecord::default();
        rec.update_crc();
        let buffer = one_record_container(M64_MAGIC, M64_MODE_STRATA, bytemuck::cast_ref(&rec));
        let reader = parse_one(&buffer).expect("strata");
        assert_eq!(reader.verification().record_crc_checked, 1);
        assert_eq!(reader.verification().records_without_checksum, 0);
        assert_eq!(
            reader.chunk(0).unwrap().directory_entry().chunk_record_mode,
            M64_MODE_STRATA
        );
    }

    #[test]
    fn rejects_unknown_record_mode() {
        let buffer = one_record_container(M64_MAGIC, 0x05, &[0u8; 64]);
        assert_eq!(
            parse_one(&buffer).err(),
            Some(M64ReadError::UnsupportedMode {
                chunk: 0,
                mode: 0x05
            })
        );
    }

    fn lrm3_container(lrm3: &M64LocalMetricHeader) -> Vec<u8> {
        let rec = M64HoloRecord::default();
        let mut header = M64Header::default();
        header.total_file_size = 320;
        header.chunk_count = 1;
        header.chunk_table_offset = 192;
        header.container_flags = M64_FLAG_LOCAL_METRIC_LRM3 as u32;
        header.update_crc();
        let mut entry = M64ChunkDirectoryEntry {
            chunk_uuid: [0; 16],
            spatial_hilbert_index: 0,
            byte_offset: 256,
            byte_length: 64,
            record_count: 1,
            srid_geodetic_code: 0,
            lut_bank_epoch_id: 0,
            chunk_record_mode: M64_MODE_HOLO,
            compression_flags: 0,
            bao_root_hash_prefix: [0u8; 12],
            entry_crc32c: 0,
        };
        entry.update_crc();
        let mut buffer = vec![0u8; 320];
        buffer[..64].copy_from_slice(bytemuck::bytes_of(&header));
        buffer[64..192].copy_from_slice(bytemuck::bytes_of(lrm3));
        buffer[192..256].copy_from_slice(bytemuck::bytes_of(&entry));
        buffer[256..320].copy_from_slice(bytemuck::bytes_of(&rec));
        buffer
    }

    #[test]
    fn admits_lrm3_gap_when_flag_set_and_crc_valid() {
        let mut lrm3 = M64LocalMetricHeader::default();
        lrm3.anchor_did_hash = 0xAABB_CCDD_EEFF_0011;
        lrm3.anisotropic_scale_um = [2.0, 3.0, 4.0];
        lrm3.update_crc();
        let buffer = lrm3_container(&lrm3);
        let reader = parse_one(&buffer).expect("lrm3 container");
        let parsed = reader.lrm3().expect("authenticated LRM3");
        assert_eq!(parsed.anchor_did_hash, 0xAABB_CCDD_EEFF_0011);
        assert_eq!(parsed.anisotropic_scale_um, [2.0, 3.0, 4.0]);
        assert!(parsed.validate().is_ok());
    }

    #[test]
    fn rejects_lrm3_flag_when_block_missing() {
        let rec = M64HoloRecord::default();
        let buffer = one_record_container(M64_MAGIC, M64_MODE_HOLO, bytemuck::cast_ref(&rec));
        let mut header: M64Header = bytemuck::pod_read_unaligned(&buffer[..64]);
        header.container_flags = M64_FLAG_LOCAL_METRIC_LRM3 as u32;
        header.update_crc();
        let mut tampered = buffer;
        tampered[..64].copy_from_slice(bytemuck::bytes_of(&header));
        assert_eq!(parse_one(&tampered).err(), Some(M64ReadError::Lrm3Missing));
    }

    #[test]
    fn rejects_lrm3_gap_crc_mismatch() {
        let mut lrm3 = M64LocalMetricHeader::default();
        lrm3.update_crc();
        let mut buffer = lrm3_container(&lrm3);
        buffer[70] ^= 1;
        assert!(matches!(
            parse_one(&buffer),
            Err(M64ReadError::Lrm3(M64Error::ChecksumMismatch { .. }))
        ));
    }

    #[test]
    fn rejects_payload_overlapping_authenticated_lrm3_gap() {
        let mut lrm3 = M64LocalMetricHeader::default();
        lrm3.update_crc();
        let mut buffer = lrm3_container(&lrm3);
        // Directory at 192..256; LRM3 occupies [64, 192). Alias the second LRM3
        // cache line (128..192) so the range does not also hit the directory.
        let mut entry: M64ChunkDirectoryEntry =
            bytemuck::pod_read_unaligned(&buffer[192..256]);
        entry.byte_offset = 128;
        entry.update_crc();
        buffer[192..256].copy_from_slice(bytemuck::bytes_of(&entry));
        assert_eq!(
            parse_one(&buffer).err(),
            Some(M64ReadError::MetadataOverlap { chunk: 0 })
        );
    }
}
