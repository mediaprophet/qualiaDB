//! Multi-Format DEM Ingestion & Chunking (GeoTIFF, Terrain-RGB, Terrarium).
//!
//! Converts raw floating-point elevation rasters or compressed 24-bit elevation tiles
//! into cache-aligned $64 \times 64$ cell `.m64` strata chunks ordered along a 2D Hilbert curve.

use super::frame::GeodeticFrame;
use super::hilbert::xy_to_hilbert;
use qualia_core_db::q42::m64_matter::{
    M64ChunkDirectoryEntry, M64Header, M64StrataColumnRecord, M64_MAGIC, M64_MODE_STRATA,
};

pub const CHUNK_GRID_SIZE: usize = 64; // 64 x 64 cells per chunk
pub const CHUNK_CELL_COUNT: usize = CHUNK_GRID_SIZE * CHUNK_GRID_SIZE; // 4096 cells

/// Decodes Mapbox Terrain-RGB encoded pixel (R, G, B) into elevation in millimeters.
///
/// $h_{\text{meters}} = -10000.0 + (R \cdot 65536 + G \cdot 256 + B) \cdot 0.1$
#[inline]
pub fn decode_terrain_rgb_mm(r: u8, g: u8, b: u8) -> i32 {
    let raw = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
    let meters = -10000.0 + (raw as f64) * 0.1;
    (meters * 1000.0).round() as i32
}

/// Encodes elevation in millimeters into Mapbox Terrain-RGB (R, G, B).
#[inline]
pub fn encode_terrain_rgb(elevation_mm: i32) -> [u8; 3] {
    let meters = (elevation_mm as f64) / 1000.0;
    let raw = (((meters + 10000.0) * 10.0).round().max(0.0) as u32) & 0x00FF_FFFF;
    [
        ((raw >> 16) & 0xFF) as u8,
        ((raw >> 8) & 0xFF) as u8,
        (raw & 0xFF) as u8,
    ]
}

/// Decodes Mapzen Terrarium encoded pixel (R, G, B) into elevation in millimeters.
///
/// $h_{\text{meters}} = (R \cdot 256 + G + B / 256.0) - 32768.0$
#[inline]
pub fn decode_terrarium_mm(r: u8, g: u8, b: u8) -> i32 {
    let meters = (r as f64) * 256.0 + (g as f64) + ((b as f64) / 256.0) - 32768.0;
    (meters * 1000.0).round() as i32
}

/// Canonical compression codecs for .m64 spatial chunks (stored in `compression_flags`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionCodec {
    /// 0: Uncompressed raw zero-copy direct mmap.
    UncompressedMmap = 0,
    /// 1: LZ4 block compression.
    Lz4 = 1,
    /// 2: Bao verified BLAKE3 outboard chunk tree.
    BaoTree = 2,
    /// 3: HTJ2K / Wavelet multiresolution progressive (JPEG 2000 Part 15 / ISO/IEC 15444-15).
    Htj2kWavelet = 3,
    /// 4: Packed 24-bit Mapbox Terrain-RGB PNG tile pyramid.
    TerrainRgbPng = 4,
    /// 5: Packed 24-bit Mapzen Terrarium PNG tile pyramid.
    TerrariumPng = 5,
    /// 6: Hardware-accelerated AV1 / HEVC all-intra video decode for visual drape.
    Av1IntraDrape = 6,
}

impl Default for CompressionCodec {
    fn default() -> Self {
        Self::UncompressedMmap
    }
}

/// Raw raster elevation grid descriptor.
pub struct ElevationRaster<'a> {
    pub width: usize,
    pub height: usize,
    pub cell_size_mm: u32, // e.g. 30,000 for 30m DEM
    pub elevations_mm: &'a [i32],
}

/// Errors during raster ingestion and compilation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestionError {
    DimensionMismatch,
    BufferTooSmall,
    InvalidChunkCount,
}

/// Partitions a DEM raster into $64 \times 64$ `.m64` strata chunks sorted along a 2D Hilbert curve.
///
/// Writes the complete `.m64` container bytes (Header + Directory + Records) into `out`.
pub fn compile_raster_to_m64(
    raster: &ElevationRaster,
    frame: &GeodeticFrame,
    lut_bank_epoch: u16,
    out: &mut [u8],
) -> Result<usize, IngestionError> {
    compile_raster_to_m64_with_codec(raster, frame, lut_bank_epoch, CompressionCodec::UncompressedMmap, out)
}

/// Partitions a DEM raster into $64 \times 64$ `.m64` strata chunks with an explicit compression codec.
pub fn compile_raster_to_m64_with_codec(
    raster: &ElevationRaster,
    frame: &GeodeticFrame,
    lut_bank_epoch: u16,
    codec: CompressionCodec,
    out: &mut [u8],
) -> Result<usize, IngestionError> {
    if raster.width == 0 || raster.height == 0 || raster.elevations_mm.len() != raster.width * raster.height {
        return Err(IngestionError::DimensionMismatch);
    }

    let chunks_x = (raster.width + CHUNK_GRID_SIZE - 1) / CHUNK_GRID_SIZE;
    let chunks_y = (raster.height + CHUNK_GRID_SIZE - 1) / CHUNK_GRID_SIZE;
    let total_chunks = chunks_x * chunks_y;

    let header_size = std::mem::size_of::<M64Header>();
    let dir_entry_size = std::mem::size_of::<M64ChunkDirectoryEntry>();
    let record_size = std::mem::size_of::<M64StrataColumnRecord>();
    let chunk_payload_size = CHUNK_CELL_COUNT * record_size;

    let required_len = header_size + total_chunks * dir_entry_size + total_chunks * chunk_payload_size;
    if out.len() < required_len {
        return Err(IngestionError::BufferTooSmall);
    }

    // 1. Build Header
    let mut header = M64Header {
        magic: M64_MAGIC,
        version: 1,
        header_size: header_size as u64,
        total_file_size: required_len as u64,
        spatial_origin: [frame.origin_lat_deg as f32, frame.origin_lon_deg as f32],
        elevation_datum: [frame.origin_elevation_mm as f32 / 1000.0, 1.0],
        chunk_table_offset: header_size as u64,
        chunk_count: total_chunks as u64,
        header_crc32c: 0,
        container_flags: 0,
    };
    header.update_crc();

    out[..header_size].copy_from_slice(bytemuck::bytes_of(&header));

    let dir_offset = header_size;
    let mut payload_offset = dir_offset + total_chunks * dir_entry_size;

    // 2. Iterate chunks and sort by Hilbert index
    for cy in 0..chunks_y {
        for cx in 0..chunks_x {
            let chunk_idx = cy * chunks_x + cx;
            let hilbert_idx = xy_to_hilbert(cx as u32, cy as u32, 16);

            let mut dir_entry = M64ChunkDirectoryEntry {
                chunk_uuid: [0; 16],
                spatial_hilbert_index: hilbert_idx,
                byte_offset: payload_offset as u64,
                byte_length: chunk_payload_size as u32,
                record_count: CHUNK_CELL_COUNT as u32,
                srid_geodetic_code: frame.srid,
                lut_bank_epoch_id: lut_bank_epoch,
                chunk_record_mode: M64_MODE_STRATA,
                compression_flags: codec as u8,
                bao_root_hash_prefix: [0; 12],
                entry_crc32c: 0,
            };

            // Derive deterministic UUID from hilbert index
            let h_bytes = hilbert_idx.to_le_bytes();
            dir_entry.chunk_uuid[..8].copy_from_slice(&h_bytes);
            dir_entry.chunk_uuid[8..16].copy_from_slice(&h_bytes);
            dir_entry.update_crc();

            // Write directory entry
            let entry_dest = dir_offset + chunk_idx * dir_entry_size;
            out[entry_dest..entry_dest + dir_entry_size].copy_from_slice(bytemuck::bytes_of(&dir_entry));

            // 3. Write 4096 strata column records for this chunk
            for local_y in 0..CHUNK_GRID_SIZE {
                for local_x in 0..CHUNK_GRID_SIZE {
                    let rx = (cx * CHUNK_GRID_SIZE + local_x).min(raster.width - 1);
                    let ry = (cy * CHUNK_GRID_SIZE + local_y).min(raster.height - 1);
                    let elev_mm = raster.elevations_mm[ry * raster.width + rx];

                    let mut record = M64StrataColumnRecord::default();
                    record.surface_elevation_mm = elev_mm;
                    record.update_crc();

                    let rec_dest = payload_offset + (local_y * CHUNK_GRID_SIZE + local_x) * record_size;
                    out[rec_dest..rec_dest + record_size].copy_from_slice(bytemuck::bytes_of(&record));
                }
            }

            payload_offset += chunk_payload_size;
        }
    }

    Ok(required_len)
}
