//! Reusable authenticated-range checks for the M64 georaster profile.
//! These routines never require or synthesize bytes for unrelated chunks.
use super::{
    georaster_chunk_hilbert, M64GeoRasterError as E, M64GeoRasterMetadata, M64GeoRasterShape,
    M64RasterSample,
};
use crate::q42::m64_matter::{M64ChunkDirectoryEntry, M64Header, M64StrataColumnRecord, M64_MAGIC, M64_MODE_STRATA};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SampleError {
    Geometry,
    Padding,
    RecordCrc,
    NonRasterFields,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SampleLocation {
    pub chunk: usize,
    pub record: usize,
    pub x: usize,
    pub y: usize,
}

/// Validate the complete georaster header/directory closure without reading any records.
/// This is the metadata half of `validate_container`; callers must authenticate the exact
/// whole directory before passing it here.
pub(crate) fn validate_header_directory(
    header: &M64Header,
    meta: &M64GeoRasterMetadata,
    logical_length: u64,
    directory: &[u8],
) -> Result<M64GeoRasterShape, E> {
    use E as Err;
    let shape = meta.shape()?;
    let file_bytes = u64::try_from(shape.file_bytes).map_err(|_| Err::Overflow)?;
    let row_bytes = shape.chunks.checked_mul(64).ok_or(Err::Overflow)?;
    if header.magic != M64_MAGIC
        || header.version != super::M64_GEORASTER_VERSION
        || header.header_size != super::M64_GEORASTER_HEADER_BYTES as u64
        || header.chunk_table_offset != super::M64_GEORASTER_HEADER_BYTES as u64
        || header.container_flags != 0
        || header.total_file_size != logical_length
        || logical_length != file_bytes
        || header.chunk_count != shape.chunks as u64
        || header.spatial_origin[0].to_bits() != (meta.origin_lat_deg as f32).to_bits()
        || header.spatial_origin[1].to_bits() != (meta.origin_lon_deg as f32).to_bits()
        || header.elevation_datum[0].to_bits()
            != (meta.origin_elevation_mm as f32 / 1000.0).to_bits()
        || header.elevation_datum[1].to_bits() != 1.0f32.to_bits()
        || directory.len() != row_bytes
    {
        return Err(Err::HeaderSummary);
    }
    let payload_start = super::M64_GEORASTER_HEADER_BYTES
        .checked_add(row_bytes)
        .ok_or(Err::Overflow)?;
    for chunk in 0..shape.chunks {
        let offset = chunk.checked_mul(64).ok_or(Err::Overflow)?;
        let row: M64ChunkDirectoryEntry = bytemuck::pod_read_unaligned(
            directory.get(offset..offset + 64).ok_or(Err::Directory)?,
        );
        row.validate().map_err(|_| Err::Directory)?;
        let x = chunk % shape.chunks_x;
        let y = chunk / shape.chunks_x;
        let x = u32::try_from(x).map_err(|_| Err::HilbertExtent)?;
        let y = u32::try_from(y).map_err(|_| Err::HilbertExtent)?;
        let expected_offset = payload_start
            .checked_add(chunk.checked_mul(4096 * 64).ok_or(Err::Overflow)?)
            .ok_or(Err::Overflow)?;
        if row.srid_geodetic_code != meta.srid
            || row.chunk_record_mode != M64_MODE_STRATA
            || row.compression_flags != 0
            || row.record_count != 4096
            || row.byte_length != 4096 * 64
            || row.bao_root_hash_prefix != [0; 12]
            || row.byte_offset != expected_offset as u64
            || row.spatial_hilbert_index != georaster_chunk_hilbert(x, y)
        {
            return Err(Err::Directory);
        }
    }
    Ok(shape)
}

/// Canonical row-major map from an in-bounds raster coordinate to chunk and record.
pub(crate) fn sample_location(
    shape: &M64GeoRasterShape,
    x: usize,
    y: usize,
) -> Result<SampleLocation, SampleError> {
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
    Ok(SampleLocation {
        chunk,
        record,
        x,
        y,
    })
}

/// Validate exactly one in-bounds 64-byte georaster record and decode its elevation.
/// Padding records are deliberately refused because their RepeatNearest source can be remote.
pub(crate) fn validate_sample_record(
    shape: &M64GeoRasterShape,
    meta: &M64GeoRasterMetadata,
    location: SampleLocation,
    raw: &[u8],
) -> Result<M64RasterSample, SampleError> {
    if raw.len() != 64
        || location.chunk >= shape.chunks
        || location.record >= 4096
        || location.x >= shape.width
        || location.y >= shape.height
    {
        return Err(SampleError::Geometry);
    }
    if location.chunk != (location.y / 64) * shape.chunks_x + location.x / 64
        || location.record != (location.y % 64) * 64 + location.x % 64
    {
        return Err(SampleError::Geometry);
    }
    let typed: M64StrataColumnRecord = bytemuck::pod_read_unaligned(raw);
    typed.validate().map_err(|_| SampleError::RecordCrc)?;
    if raw[4..60].iter().any(|&byte| byte != 0) {
        return Err(SampleError::NonRasterFields);
    }
    let elevation = i32::from_le_bytes(raw[..4].try_into().map_err(|_| SampleError::Geometry)?);
    Ok(if meta.nodata_sentinel_mm == Some(elevation) {
        M64RasterSample::Nodata(elevation)
    } else {
        M64RasterSample::ElevationMm(elevation)
    })
}
