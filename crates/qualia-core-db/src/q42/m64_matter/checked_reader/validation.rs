use super::super::{
    M64ChunkDirectoryEntry, M64Header, M64PhaseJunctureRecord, M64StrataColumnRecord,
    M64_MODE_HOLO, M64_MODE_PHASE_JUNCTURE, M64_MODE_POINT, M64_MODE_STRATA,
};
use super::{
    intervals, M64PayloadInterval, M64ReadError as Error, M64ReadLimits, M64Reader, M64Verification,
};

pub(super) fn parse<'a>(
    bytes: &'a [u8],
    limits: M64ReadLimits,
    scratch: &mut [M64PayloadInterval],
) -> Result<M64Reader<'a>, Error> {
    if !cfg!(target_endian = "little") {
        return Err(Error::UnsupportedEndian);
    }
    if bytes.len() > limits.max_file_bytes {
        return Err(Error::FileByteLimit);
    }
    let raw_header = bytes.get(..64).ok_or(Error::TruncatedHeader)?;
    let header: M64Header = bytemuck::pod_read_unaligned(raw_header);
    if !matches!(header.version, 1 | 2 | 3) {
        return Err(Error::UnsupportedVersion(header.version));
    }
    header.validate().map_err(Error::Header)?;
    if header.container_flags != 0 {
        return Err(Error::UnsupportedContainerFlags(header.container_flags));
    }
    if header.total_file_size != bytes.len() as u64 {
        return Err(Error::FileSizeMismatch);
    }
    let georaster = if header.version == 3 {
        let meta = super::super::M64GeoRasterMetadata::decode(
            bytes
                .get(64..192)
                .ok_or(Error::GeoRaster(super::super::M64GeoRasterError::Truncated))?,
        )
        .map_err(Error::GeoRaster)?;
        let shape = meta.shape().map_err(Error::GeoRaster)?;
        if header.magic != super::super::M64_MAGIC
            || header.chunk_table_offset != 192
            || header.chunk_count != shape.chunks as u64
            || bytes.len() != shape.file_bytes
        {
            return Err(Error::GeoRaster(
                super::super::M64GeoRasterError::HeaderSummary,
            ));
        }
        Some(meta)
    } else {
        None
    };
    let count = usize::try_from(header.chunk_count).map_err(|_| Error::ArithmeticOverflow)?;
    if count > limits.max_chunks {
        return Err(Error::ChunkLimit);
    }
    if scratch.len() < count {
        return Err(Error::ScratchTooSmall {
            required: count,
            provided: scratch.len(),
        });
    }
    let directory_start =
        usize::try_from(header.chunk_table_offset).map_err(|_| Error::ArithmeticOverflow)?;
    if directory_start % 64 != 0 {
        return Err(Error::DirectoryAlignment);
    }
    let directory_end = directory_start
        .checked_add(count.checked_mul(64).ok_or(Error::ArithmeticOverflow)?)
        .ok_or(Error::ArithmeticOverflow)?;
    if directory_start < header.header_size as usize || directory_end > bytes.len() {
        return Err(Error::DirectoryBounds);
    }
    let mut verification = M64Verification {
        chunks: count,
        ..M64Verification::default()
    };
    // Metadata preflight: no record scan until all limits and ranges pass.
    for (chunk, interval) in scratch[..count].iter_mut().enumerate() {
        let offset = directory_start + chunk * 64;
        let entry: M64ChunkDirectoryEntry =
            bytemuck::pod_read_unaligned(&bytes[offset..offset + 64]);
        entry
            .validate()
            .map_err(|error| Error::Directory { chunk, error })?;
        if !matches!(
            entry.chunk_record_mode,
            M64_MODE_POINT | M64_MODE_HOLO | M64_MODE_STRATA | M64_MODE_PHASE_JUNCTURE
        ) {
            return Err(Error::UnsupportedMode {
                chunk,
                mode: entry.chunk_record_mode,
            });
        }
        if entry.compression_flags != 0 {
            return Err(Error::UnsupportedCodec {
                chunk,
                codec: entry.compression_flags,
            });
        }
        let expected_length = u64::from(entry.record_count)
            .checked_mul(64)
            .ok_or(Error::ArithmeticOverflow)?;
        if entry.record_count == 0 || expected_length != u64::from(entry.byte_length) {
            return Err(Error::RecordLength { chunk });
        }
        verification.total_records = verification
            .total_records
            .checked_add(u64::from(entry.record_count))
            .ok_or(Error::ArithmeticOverflow)?;
        if verification.total_records > limits.max_records {
            return Err(Error::RecordLimit);
        }
        let start = usize::try_from(entry.byte_offset).map_err(|_| Error::ArithmeticOverflow)?;
        let end = start
            .checked_add(entry.byte_length as usize)
            .ok_or(Error::ArithmeticOverflow)?;
        if start % 64 != 0 {
            return Err(Error::PayloadAlignment { chunk });
        }
        if end > bytes.len() {
            return Err(Error::PayloadBounds { chunk });
        }
        if start < header.header_size as usize || (start < directory_end && end > directory_start) {
            return Err(Error::MetadataOverlap { chunk });
        }
        *interval = M64PayloadInterval { start, end };
    }
    if intervals::has_overlap(&mut scratch[..count]) {
        return Err(Error::PayloadOverlap);
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
            result.map_err(|error| Error::Record {
                chunk,
                record,
                error,
            })?;
            verification.record_crc_checked += 1;
        }
    }
    if let Some(meta) = &georaster {
        super::super::georaster::validate_container(bytes, &header, meta)
            .map_err(Error::GeoRaster)?;
    }
    Ok(M64Reader {
        bytes,
        header,
        directory_start,
        verification,
        georaster,
    })
}
