use super::*;
use bytemuck::Zeroable;
use qualia_core_db::q42::m64_matter::{
    georaster_chunk_hilbert, M64ChunkDirectoryEntry, M64Header, M64StrataColumnRecord, M64_MAGIC,
    M64_MODE_STRATA,
};

/// Opt-in developmental v3: exact GRM3 metadata plus raw elevation-only strata.
/// Elevations remain absolute integer mm in the declared vertical datum; no
/// frame-base subtraction or addition occurs. Explicit limits, caller output,
/// no allocation or recursion. Every error and
/// cancellation check occurs before any output write. The admitted write batch
/// is non-interruptible and returns success even if cancellation arrives later.
pub fn compile_raster_to_m64_v3(
    raster: &ElevationRaster<'_>,
    frame: &GeodeticFrame,
    lut_bank_epoch: u16,
    policy: RasterM64Policy,
    limits: RasterM64Limits,
    cancelled: &AtomicBool,
    out: &mut [u8],
) -> Result<usize, RasterM64Error> {
    use RasterM64Error as E;
    if cancelled.load(Ordering::Relaxed) {
        return Err(E::Cancelled);
    }
    if !cfg!(target_endian = "little") {
        return Err(E::UnsupportedEndian);
    }
    let meta = policy.metadata(raster, frame);
    let shape = meta.shape().map_err(E::Metadata)?;
    if shape.width.checked_mul(shape.height) != Some(raster.elevations_mm.len()) {
        return Err(E::DimensionMismatch);
    }
    if shape.chunks > limits.max_chunks {
        return Err(E::ChunkLimit);
    }
    if shape.records > limits.max_records {
        return Err(E::RecordLimit);
    }
    if shape.file_bytes > limits.max_output_bytes {
        return Err(E::OutputByteLimit);
    }
    if out.len() < shape.file_bytes {
        return Err(E::BufferTooSmall);
    }
    let mut block = [0u8; 128];
    meta.encode(&mut block).map_err(E::Metadata)?;
    let mut header = M64Header {
        magic: M64_MAGIC,
        version: 3,
        header_size: 192,
        total_file_size: shape.file_bytes as u64,
        spatial_origin: [frame.origin_lat_deg as f32, frame.origin_lon_deg as f32],
        elevation_datum: [frame.origin_elevation_mm as f32 / 1000.0, 1.0],
        chunk_table_offset: 192,
        chunk_count: shape.chunks as u64,
        header_crc32c: 0,
        container_flags: 0,
    };
    header.update_crc();
    if cancelled.load(Ordering::Relaxed) {
        return Err(E::Cancelled);
    }
    out[..64].copy_from_slice(bytemuck::bytes_of(&header));
    out[64..192].copy_from_slice(&block);
    let payload_start = 192 + shape.chunks * 64;
    for chunk in 0..shape.chunks {
        let cx = chunk % shape.chunks_x;
        let cy = chunk / shape.chunks_x;
        let key = georaster_chunk_hilbert(cx as u32, cy as u32);
        let start = payload_start + chunk * 4096 * 64;
        let mut entry = M64ChunkDirectoryEntry {
            chunk_uuid: [0; 16],
            spatial_hilbert_index: key,
            byte_offset: start as u64,
            byte_length: 4096 * 64,
            record_count: 4096,
            srid_geodetic_code: frame.srid,
            lut_bank_epoch_id: lut_bank_epoch,
            chunk_record_mode: M64_MODE_STRATA,
            compression_flags: 0,
            bao_root_hash_prefix: [0; 12],
            entry_crc32c: 0,
        };
        entry.chunk_uuid[..8].copy_from_slice(&key.to_le_bytes());
        entry.chunk_uuid[8..].copy_from_slice(&key.to_le_bytes());
        entry.update_crc();
        out[192 + chunk * 64..192 + (chunk + 1) * 64].copy_from_slice(bytemuck::bytes_of(&entry));
        for record in 0..4096 {
            let x = (cx * 64 + record % 64).min(shape.width - 1);
            let y = (cy * 64 + record / 64).min(shape.height - 1);
            // Unpopulated strata fields are not fabricated soil/material values.
            let mut value = M64StrataColumnRecord::zeroed();
            value.surface_elevation_mm = raster.elevations_mm[y * shape.width + x];
            value.update_crc();
            let offset = start + record * 64;
            out[offset..offset + 64].copy_from_slice(bytemuck::bytes_of(&value));
        }
    }
    Ok(shape.file_bytes)
}
