use super::super::{M64ChunkDirectoryEntry, M64Header, M64_MAGIC, M64_MODE_STRATA};
use super::*;

/// Runs only after generic metadata/extent/record CRC admission. Exact profile
/// lengths bound these additional policy checks to the admitted record count.
pub(crate) fn validate_container(
    bytes: &[u8],
    header: &M64Header,
    meta: &M64GeoRasterMetadata,
) -> Result<(), M64GeoRasterError> {
    use M64GeoRasterError as E;
    let shape = meta.shape()?;
    if header.magic != M64_MAGIC
        || header.header_size != 192
        || header.chunk_table_offset != 192
        || header.chunk_count != shape.chunks as u64
        || bytes.len() != shape.file_bytes
        || header.spatial_origin[0].to_bits() != (meta.origin_lat_deg as f32).to_bits()
        || header.spatial_origin[1].to_bits() != (meta.origin_lon_deg as f32).to_bits()
        || header.elevation_datum[0].to_bits()
            != (meta.origin_elevation_mm as f32 / 1000.0).to_bits()
        || header.elevation_datum[1].to_bits() != 1.0f32.to_bits()
    {
        return Err(E::HeaderSummary);
    }
    let payload_start = 192 + shape.chunks * 64;
    for chunk in 0..shape.chunks {
        let directory_offset = 192 + chunk * 64;
        let entry: M64ChunkDirectoryEntry =
            bytemuck::pod_read_unaligned(&bytes[directory_offset..directory_offset + 64]);
        if entry.srid_geodetic_code != meta.srid
            || entry.chunk_record_mode != M64_MODE_STRATA
            || entry.compression_flags != 0
            || entry.record_count != 4096
            || entry.byte_length != 4096 * 64
            || entry.bao_root_hash_prefix != [0; 12]
            || entry.byte_offset != (payload_start + chunk * 4096 * 64) as u64
            || entry.spatial_hilbert_index
                != georaster_chunk_hilbert(
                    (chunk % shape.chunks_x) as u32,
                    (chunk / shape.chunks_x) as u32,
                )
        {
            return Err(E::Directory);
        }
        let start = entry.byte_offset as usize;
        for record in 0..4096 {
            let offset = start + record * 64;
            // Raster profile contains only elevations, not invented soil/physics.
            if bytes[offset + 4..offset + 60].iter().any(|&v| v != 0) {
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
