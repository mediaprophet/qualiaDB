//! Reusable authenticated-range checks for the M64 georaster profile.
//! These routines never require or synthesize bytes for unrelated chunks.
use super::validation::{
    directory_row_profile, non_raster_fields_present, shape_and_header_summary,
};
use super::{
    M64GeoRasterError as E, M64GeoRasterMetadata, M64GeoRasterShape, M64RasterSample,
    M64_GEORASTER_HEADER_BYTES, M64_GEORASTER_VERSION,
};
use crate::q42::m64_matter::{M64ChunkDirectoryEntry, M64Header, M64StrataColumnRecord};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleError {
    Geometry,
    Padding,
    RecordCrc,
    NonRasterFields,
}

/// Validate the complete georaster header/directory closure without reading any records.
/// Callers must authenticate the exact whole directory before passing it here.
pub fn validate_header_directory(
    logical_length: u64,
    header: &M64Header,
    meta: &M64GeoRasterMetadata,
    directory: &[u8],
) -> Result<M64GeoRasterShape, E> {
    if header.version != M64_GEORASTER_VERSION
        || header.container_flags != 0
        || header.total_file_size != logical_length
    {
        return Err(E::HeaderSummary);
    }
    let shape = shape_and_header_summary(header, meta, logical_length)?;
    let row_bytes = shape.chunks.checked_mul(64).ok_or(E::Overflow)?;
    if directory.len() != row_bytes {
        return Err(E::HeaderSummary);
    }
    let payload_start = M64_GEORASTER_HEADER_BYTES
        .checked_add(row_bytes)
        .ok_or(E::Overflow)?;
    for chunk in 0..shape.chunks {
        let offset = chunk.checked_mul(64).ok_or(E::Overflow)?;
        let end = offset.checked_add(64).ok_or(E::Overflow)?;
        let row: M64ChunkDirectoryEntry =
            bytemuck::pod_read_unaligned(directory.get(offset..end).ok_or(E::Directory)?);
        row.validate().map_err(|_| E::Directory)?;
        directory_row_profile(meta, &shape, chunk, payload_start, &row)?;
    }
    Ok(shape)
}

/// Canonical row-major map from an in-bounds raster coordinate to (chunk, record).
pub fn sample_location(
    shape: &M64GeoRasterShape,
    x: usize,
    y: usize,
) -> Result<(usize, usize), SampleError> {
    if x >= shape.width || y >= shape.height || shape.chunks_x == 0 {
        return Err(SampleError::Padding);
    }
    let chunk = (y / 64)
        .checked_mul(shape.chunks_x)
        .and_then(|n| n.checked_add(x / 64))
        .ok_or(SampleError::Geometry)?;
    let record = (y % 64)
        .checked_mul(64)
        .and_then(|n| n.checked_add(x % 64))
        .ok_or(SampleError::Geometry)?;
    if chunk >= shape.chunks || record >= 4096 {
        return Err(SampleError::Geometry);
    }
    Ok((chunk, record))
}

/// Validate exactly one 64-byte georaster record. Padding is a coordinate
/// question handled by `sample_location`; this checks CRC and non-raster bytes.
pub fn validate_sample_record(
    meta: &M64GeoRasterMetadata,
    chunk_index: usize,
    raw: &[u8],
) -> Result<M64RasterSample, SampleError> {
    let shape = meta.shape().map_err(|_| SampleError::Geometry)?;
    if raw.len() != 64 || chunk_index >= shape.chunks {
        return Err(SampleError::Geometry);
    }
    let typed: M64StrataColumnRecord = bytemuck::pod_read_unaligned(raw);
    typed.validate().map_err(|_| SampleError::RecordCrc)?;
    if non_raster_fields_present(raw) {
        return Err(SampleError::NonRasterFields);
    }
    let elevation = i32::from_le_bytes(raw[..4].try_into().map_err(|_| SampleError::Geometry)?);
    Ok(if meta.nodata_sentinel_mm == Some(elevation) {
        M64RasterSample::Nodata(elevation)
    } else {
        M64RasterSample::ElevationMm(elevation)
    })
}
