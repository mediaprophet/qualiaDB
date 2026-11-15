use super::super::{M64ChunkDirectoryEntry, M64Header, M64_MAGIC, M64_MODE_STRATA};
use super::*;

pub(super) fn shape_and_header_summary(
    header: &M64Header,
    meta: &M64GeoRasterMetadata,
    logical_length: u64,
) -> Result<M64GeoRasterShape, M64GeoRasterError> {
    use M64GeoRasterError as E;
    let shape = meta.shape()?;
    let file_bytes = u64::try_from(shape.file_bytes).map_err(|_| E::Overflow)?;
    if header.magic != M64_MAGIC
        || header.header_size != M64_GEORASTER_HEADER_BYTES as u64
        || header.chunk_table_offset != M64_GEORASTER_HEADER_BYTES as u64
        || header.chunk_count != shape.chunks as u64
        || logical_length != file_bytes
        || header.spatial_origin[0].to_bits() != (meta.origin_lat_deg as f32).to_bits()
        || header.spatial_origin[1].to_bits() != (meta.origin_lon_deg as f32).to_bits()
        || header.elevation_datum[0].to_bits()
            != (meta.origin_elevation_mm as f32 / 1000.0).to_bits()
        || header.elevation_datum[1].to_bits() != 1.0f32.to_bits()
    {
        return Err(E::HeaderSummary);
    }
    Ok(shape)
}

pub(super) fn directory_row_profile(
    meta: &M64GeoRasterMetadata,
    shape: &M64GeoRasterShape,
    chunk: usize,
    payload_start: usize,
    row: &M64ChunkDirectoryEntry,
) -> Result<(), M64GeoRasterError> {
    if row.srid_geodetic_code != meta.srid
        || row.chunk_record_mode != M64_MODE_STRATA
        || row.compression_flags != 0
        || row.record_count != 4096
        || row.byte_length != 4096 * 64
        || row.bao_root_hash_prefix != [0; 12]
        || row.byte_offset != (payload_start + chunk * 4096 * 64) as u64
        || row.spatial_hilbert_index
            != georaster_chunk_hilbert(
                (chunk % shape.chunks_x) as u32,
                (chunk / shape.chunks_x) as u32,
            )
    {
        return Err(M64GeoRasterError::Directory);
    }
    Ok(())
}

pub(super) fn non_raster_fields_present(record: &[u8]) -> bool {
    record
        .get(4..60)
        .map_or(true, |fields| fields.iter().any(|&v| v != 0))
}

/// Runs only after generic metadata/extent/record CRC admission. Exact profile
/// lengths bound these additional policy checks to the admitted record count.
pub(crate) fn validate_container(
    bytes: &[u8],
    header: &M64Header,
    meta: &M64GeoRasterMetadata,
) -> Result<(), M64GeoRasterError> {
    use M64GeoRasterError as E;
    let logical_length = u64::try_from(bytes.len()).map_err(|_| E::Overflow)?;
    let shape = shape_and_header_summary(header, meta, logical_length)?;
    let payload_start = M64_GEORASTER_HEADER_BYTES + shape.chunks * 64;
    for chunk in 0..shape.chunks {
        let directory_offset = M64_GEORASTER_HEADER_BYTES + chunk * 64;
        let entry: M64ChunkDirectoryEntry =
            bytemuck::pod_read_unaligned(&bytes[directory_offset..directory_offset + 64]);
        directory_row_profile(meta, &shape, chunk, payload_start, &entry)?;
        let start = entry.byte_offset as usize;
        for record in 0..4096 {
            let offset = start + record * 64;
            // Raster profile contains only elevations, not invented soil/physics.
            if non_raster_fields_present(&bytes[offset..offset + 64]) {
                return Err(E::NonRasterFields);
            }
            let x = (chunk % shape.chunks_x) * 64 + record % 64;
            let y = (chunk / shape.chunks_x) * 64 + record / 64;
            if x >= shape.width || y >= shape.height {
                let sx = x.min(shape.width - 1);
                let sy = y.min(shape.height - 1);
                let source_chunk = (sy / 64) * shape.chunks_x + sx / 64;
                let source_record = (sy % 64) * 64 + sx % 64;
                let source = payload_start + (source_chunk * 4096 + source_record) * 64;
                if bytes[offset..offset + 4] != bytes[source..source + 4] {
                    return Err(E::Padding);
                }
            }
        }
    }
    Ok(())
}
