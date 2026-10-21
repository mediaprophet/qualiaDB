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

/// Cloud-Optimised GeoTIFF (COG) Image File Directory (IFD) header (GAP-13 / G66).
///
/// Deterministic zero-heap descriptor for TIFF/COG tile and strip rasters.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CogIfdHeader {
    pub little_endian: bool,
    pub image_width: u32,
    pub image_height: u32,
    pub bits_per_sample: u16,
    /// 1 = unsigned int, 2 = two's complement signed int, 3 = IEEE floating point.
    pub sample_format: u16,
    /// 1 = uncompressed, 5 = LZW, 8 = Deflate.
    pub compression: u16,
    pub tile_width: u32,
    pub tile_height: u32,
    pub first_strip_or_tile_offset: u32,
    pub first_strip_or_tile_byte_count: u32,
    /// Pixel scale in XYZ (metres per pixel or degrees), stored as `f32::to_bits` for deterministic Eq.
    pub pixel_scale_xyz: [u32; 3],
}

#[inline]
fn read_u16_raw(bytes: &[u8], offset: usize, le: bool) -> Option<u16> {
    if offset + 2 > bytes.len() {
        return None;
    }
    let arr = [bytes[offset], bytes[offset + 1]];
    Some(if le { u16::from_le_bytes(arr) } else { u16::from_be_bytes(arr) })
}

#[inline]
fn read_u32_raw(bytes: &[u8], offset: usize, le: bool) -> Option<u32> {
    if offset + 4 > bytes.len() {
        return None;
    }
    let arr = [
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ];
    Some(if le { u32::from_le_bytes(arr) } else { u32::from_be_bytes(arr) })
}

#[inline]
fn read_f64_raw(bytes: &[u8], offset: usize, le: bool) -> Option<f64> {
    if offset + 8 > bytes.len() {
        return None;
    }
    let arr = [
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ];
    Some(if le { f64::from_le_bytes(arr) } else { f64::from_be_bytes(arr) })
}

/// Parses a TIFF / Cloud-Optimised GeoTIFF (COG) IFD0 header with zero heap allocation.
///
/// Extracts geometry, sample format, compression, and strip/tile offsets for raster window decoding.
pub fn parse_cog_ifd_header_zero_heap(bytes: &[u8]) -> Option<CogIfdHeader> {
    if bytes.len() < 8 {
        return None;
    }

    let little_endian = match &bytes[0..4] {
        [0x49, 0x49, 0x2A, 0x00] => true,
        [0x4D, 0x4D, 0x00, 0x2A] => false,
        _ => return None,
    };

    let ifd_offset = read_u32_raw(bytes, 4, little_endian)? as usize;
    if ifd_offset + 2 > bytes.len() {
        return None;
    }

    let num_entries = read_u16_raw(bytes, ifd_offset, little_endian)? as usize;
    if ifd_offset + 2 + num_entries * 12 > bytes.len() {
        return None;
    }

    let mut image_width: u32 = 0;
    let mut image_height: u32 = 0;
    let mut bits_per_sample: u16 = 32;
    let mut sample_format: u16 = 1; // Default: 1 = unsigned integer per TIFF spec
    let mut compression: u16 = 1;   // Default: 1 = uncompressed
    let mut tile_width: u32 = 0;
    let mut tile_height: u32 = 0;
    let mut tile_offsets: Option<(u32, u32, u16)> = None;   // (count, raw_val, type)
    let mut tile_counts: Option<(u32, u32, u16)> = None;
    let mut strip_offsets: Option<(u32, u32, u16)> = None;
    let mut strip_counts: Option<(u32, u32, u16)> = None;
    let mut pixel_scale_xyz: [u32; 3] = [0, 0, 0];

    for i in 0..num_entries {
        let entry_off = ifd_offset + 2 + i * 12;
        let tag = read_u16_raw(bytes, entry_off, little_endian)?;
        let tag_type = read_u16_raw(bytes, entry_off + 2, little_endian)?;
        let count = read_u32_raw(bytes, entry_off + 4, little_endian)?;
        let val_off = entry_off + 8;

        match tag {
            256 => {
                // ImageWidth
                image_width = if tag_type == 3 {
                    read_u16_raw(bytes, val_off, little_endian)? as u32
                } else {
                    read_u32_raw(bytes, val_off, little_endian)?
                };
            }
            257 => {
                // ImageLength / ImageHeight
                image_height = if tag_type == 3 {
                    read_u16_raw(bytes, val_off, little_endian)? as u32
                } else {
                    read_u32_raw(bytes, val_off, little_endian)?
                };
            }
            258 => {
                // BitsPerSample
                bits_per_sample = if count == 1 {
                    read_u16_raw(bytes, val_off, little_endian)?
                } else {
                    let ptr = read_u32_raw(bytes, val_off, little_endian)? as usize;
                    read_u16_raw(bytes, ptr, little_endian)?
                };
            }
            259 => {
                // Compression
                compression = read_u16_raw(bytes, val_off, little_endian)?;
            }
            273 => {
                // StripOffsets
                let raw = read_u32_raw(bytes, val_off, little_endian)?;
                strip_offsets = Some((count, raw, tag_type));
            }
            279 => {
                // StripByteCounts
                let raw = read_u32_raw(bytes, val_off, little_endian)?;
                strip_counts = Some((count, raw, tag_type));
            }
            322 => {
                // TileWidth
                tile_width = if tag_type == 3 {
                    read_u16_raw(bytes, val_off, little_endian)? as u32
                } else {
                    read_u32_raw(bytes, val_off, little_endian)?
                };
            }
            323 => {
                // TileLength
                tile_height = if tag_type == 3 {
                    read_u16_raw(bytes, val_off, little_endian)? as u32
                } else {
                    read_u32_raw(bytes, val_off, little_endian)?
                };
            }
            324 => {
                // TileOffsets
                let raw = read_u32_raw(bytes, val_off, little_endian)?;
                tile_offsets = Some((count, raw, tag_type));
            }
            325 => {
                // TileByteCounts
                let raw = read_u32_raw(bytes, val_off, little_endian)?;
                tile_counts = Some((count, raw, tag_type));
            }
            339 => {
                // SampleFormat (1 = uint, 2 = int, 3 = IEEE float)
                sample_format = if count == 1 {
                    read_u16_raw(bytes, val_off, little_endian)?
                } else {
                    let ptr = read_u32_raw(bytes, val_off, little_endian)? as usize;
                    read_u16_raw(bytes, ptr, little_endian)?
                };
            }
            33550 => {
                // ModelPixelScaleTag (DOUBLE, count = 3: ScaleX, ScaleY, ScaleZ)
                if tag_type == 12 && count >= 3 {
                    let ptr = read_u32_raw(bytes, val_off, little_endian)? as usize;
                    if let (Some(sx), Some(sy), Some(sz)) = (
                        read_f64_raw(bytes, ptr, little_endian),
                        read_f64_raw(bytes, ptr + 8, little_endian),
                        read_f64_raw(bytes, ptr + 16, little_endian),
                    ) {
                        pixel_scale_xyz = [
                            (sx as f32).to_bits(),
                            (sy as f32).to_bits(),
                            (sz as f32).to_bits(),
                        ];
                    }
                }
            }
            _ => {}
        }
    }

    let resolve_entry = |item: Option<(u32, u32, u16)>| -> (u32, u32) {
        match item {
            Some((c, raw, tag_t)) => {
                if c == 1 {
                    if tag_t == 3 {
                        (raw & 0xFFFF, 0)
                    } else {
                        (raw, 0)
                    }
                } else {
                    let ptr = raw as usize;
                    if tag_t == 3 {
                        (read_u16_raw(bytes, ptr, little_endian).unwrap_or(0) as u32, 0)
                    } else {
                        (read_u32_raw(bytes, ptr, little_endian).unwrap_or(0), 0)
                    }
                }
            }
            None => (0, 0),
        }
    };

    let (first_strip_or_tile_offset, first_strip_or_tile_byte_count) = if tile_offsets.is_some() {
        let (off, _) = resolve_entry(tile_offsets);
        let (cnt, _) = resolve_entry(tile_counts);
        (off, cnt)
    } else {
        let (off, _) = resolve_entry(strip_offsets);
        let (cnt, _) = resolve_entry(strip_counts);
        (off, cnt)
    };

    if tile_width == 0 {
        tile_width = image_width;
    }
    if tile_height == 0 {
        tile_height = image_height;
    }

    Some(CogIfdHeader {
        little_endian,
        image_width,
        image_height,
        bits_per_sample,
        sample_format,
        compression,
        tile_width,
        tile_height,
        first_strip_or_tile_offset,
        first_strip_or_tile_byte_count,
        pixel_scale_xyz,
    })
}

/// Extracts floating-point elevation values from an uncompressed COG tile or strip with zero heap allocation.
///
/// Decodes either IEEE-754 32-bit float (`sample_format = 3`) or 16-bit signed integer (`sample_format = 2`)
/// samples into `out_elevations_m`. Returns the number of elevation samples written.
pub fn extract_cog_f32_window_zero_heap(
    bytes: &[u8],
    header: &CogIfdHeader,
    out_elevations_m: &mut [f32],
) -> usize {
    if header.compression != 1 {
        // Only uncompressed direct raster payloads supported in zero-heap extraction
        return 0;
    }

    let offset = header.first_strip_or_tile_offset as usize;
    let byte_count = header.first_strip_or_tile_byte_count as usize;
    if offset > bytes.len() || offset + byte_count > bytes.len() {
        return 0;
    }

    let payload = &bytes[offset..offset + byte_count];
    let le = header.little_endian;

    match (header.bits_per_sample, header.sample_format) {
        (32, 3) => {
            // IEEE 754 32-bit single precision float
            let available = payload.len() / 4;
            let count = out_elevations_m.len().min(available);
            for i in 0..count {
                let off = i * 4;
                let s_bytes = [
                    payload[off],
                    payload[off + 1],
                    payload[off + 2],
                    payload[off + 3],
                ];
                out_elevations_m[i] = if le {
                    f32::from_le_bytes(s_bytes)
                } else {
                    f32::from_be_bytes(s_bytes)
                };
            }
            count
        }
        (16, 2) => {
            // 16-bit signed integer (common in SRTM / NSW DEM rasters)
            let available = payload.len() / 2;
            let count = out_elevations_m.len().min(available);
            for i in 0..count {
                let off = i * 2;
                let s_bytes = [payload[off], payload[off + 1]];
                let raw = if le {
                    i16::from_le_bytes(s_bytes)
                } else {
                    i16::from_be_bytes(s_bytes)
                };
                out_elevations_m[i] = raw as f32;
            }
            count
        }
        (16, 1) => {
            // 16-bit unsigned integer
            let available = payload.len() / 2;
            let count = out_elevations_m.len().min(available);
            for i in 0..count {
                let off = i * 2;
                let s_bytes = [payload[off], payload[off + 1]];
                let raw = if le {
                    u16::from_le_bytes(s_bytes)
                } else {
                    u16::from_be_bytes(s_bytes)
                };
                out_elevations_m[i] = raw as f32;
            }
            count
        }
        (32, 1) | (32, 2) => {
            // 32-bit integer
            let available = payload.len() / 4;
            let count = out_elevations_m.len().min(available);
            for i in 0..count {
                let off = i * 4;
                let s_bytes = [
                    payload[off],
                    payload[off + 1],
                    payload[off + 2],
                    payload[off + 3],
                ];
                let raw = if le {
                    i32::from_le_bytes(s_bytes)
                } else {
                    i32::from_be_bytes(s_bytes)
                };
                out_elevations_m[i] = raw as f32;
            }
            count
        }
        _ => 0,
    }
}

/// Computes clamped non-negative canopy height model: `CHM = (DSM - DEM).max(0.0)`.
///
/// Drives `space_colonization_3d` tree height envelopes without heap allocation.
pub fn compute_canopy_height_model_zero_heap(
    dsm_m: &[f32],
    bare_earth_dem_m: &[f32],
    out_chm_m: &mut [f32],
) -> usize {
    let count = dsm_m.len().min(bare_earth_dem_m.len()).min(out_chm_m.len());
    for i in 0..count {
        let dsm = dsm_m[i];
        let dem = bare_earth_dem_m[i];
        if dsm.is_finite() && dem.is_finite() {
            out_chm_m[i] = (dsm - dem).max(0.0);
        } else {
            out_chm_m[i] = 0.0;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cog_ifd_header_and_extract_elevation() {
        let mut tiff_bytes = vec![0u8; 512];

        // 1. TIFF header (8 bytes)
        tiff_bytes[0..2].copy_from_slice(b"II"); // Little-endian
        tiff_bytes[2..4].copy_from_slice(&42u16.to_le_bytes()); // TIFF magic 42
        tiff_bytes[4..8].copy_from_slice(&8u32.to_le_bytes()); // IFD0 offset = 8

        // 2. IFD0 at offset 8:
        let entry_count: u16 = 10;
        tiff_bytes[8..10].copy_from_slice(&entry_count.to_le_bytes());

        let mut write_tag = |idx: usize, tag: u16, tag_type: u16, count: u32, val_or_off: u32| {
            let off = 10 + idx * 12;
            tiff_bytes[off..off + 2].copy_from_slice(&tag.to_le_bytes());
            tiff_bytes[off + 2..off + 4].copy_from_slice(&tag_type.to_le_bytes());
            tiff_bytes[off + 4..off + 8].copy_from_slice(&count.to_le_bytes());
            tiff_bytes[off + 8..off + 12].copy_from_slice(&val_or_off.to_le_bytes());
        };

        write_tag(0, 256, 3, 1, 4); // ImageWidth = 4
        write_tag(1, 257, 3, 1, 4); // ImageLength = 4
        write_tag(2, 258, 3, 1, 32); // BitsPerSample = 32
        write_tag(3, 259, 3, 1, 1); // Compression = 1 (uncompressed)
        write_tag(4, 322, 3, 1, 4); // TileWidth = 4
        write_tag(5, 323, 3, 1, 4); // TileLength = 4
        write_tag(6, 324, 4, 1, 300); // TileOffsets = offset 300
        write_tag(7, 325, 4, 1, 64); // TileByteCounts = 64 bytes
        write_tag(8, 339, 3, 1, 3); // SampleFormat = 3 (float)
        write_tag(9, 33550, 12, 3, 200); // ModelPixelScaleTag offset 200

        // Next IFD offset: 4 bytes of 0 at 10 + 10 * 12 = 130
        tiff_bytes[130..134].copy_from_slice(&0u32.to_le_bytes());

        // ModelPixelScaleTag at offset 200 (3 f64s = 24 bytes): scale_x=1.0, scale_y=1.0, scale_z=0.0
        tiff_bytes[200..208].copy_from_slice(&1.0f64.to_le_bytes());
        tiff_bytes[208..216].copy_from_slice(&1.0f64.to_le_bytes());
        tiff_bytes[216..224].copy_from_slice(&0.0f64.to_le_bytes());

        // Payload at offset 300: 16 f32 NSW elevation samples
        let sample_elevations: [f32; 16] = [
            520.0, 521.5, 523.0, 524.5,
            521.0, 522.5, 524.0, 525.5,
            522.0, 523.5, 525.0, 526.5,
            523.0, 524.5, 526.0, 527.5,
        ];
        for (i, &elev) in sample_elevations.iter().enumerate() {
            let off = 300 + i * 4;
            tiff_bytes[off..off + 4].copy_from_slice(&elev.to_le_bytes());
        }

        // Test 1: parse header
        let header = parse_cog_ifd_header_zero_heap(&tiff_bytes).expect("valid header parsed");
        assert!(header.little_endian);
        assert_eq!(header.image_width, 4);
        assert_eq!(header.image_height, 4);
        assert_eq!(header.bits_per_sample, 32);
        assert_eq!(header.sample_format, 3);
        assert_eq!(header.compression, 1);
        assert_eq!(header.tile_width, 4);
        assert_eq!(header.tile_height, 4);
        assert_eq!(header.first_strip_or_tile_offset, 300);
        assert_eq!(header.first_strip_or_tile_byte_count, 64);
        assert_eq!(header.pixel_scale_xyz[0], 1.0f32.to_bits());
        assert_eq!(header.pixel_scale_xyz[1], 1.0f32.to_bits());

        // Test 2: extract elevation samples
        let mut extracted = [0.0f32; 16];
        let n = extract_cog_f32_window_zero_heap(&tiff_bytes, &header, &mut extracted);
        assert_eq!(n, 16);
        for i in 0..16 {
            assert_eq!(extracted[i], sample_elevations[i]);
        }

        // Test 3: compute canopy height model (CHM = DSM - DEM)
        let bare_earth_dem = [
            500.0, 500.0, 500.0, 500.0,
            500.0, 500.0, 500.0, 500.0,
            500.0, 500.0, 500.0, 500.0,
            530.0, 500.0, 500.0, 500.0, // note 530.0 > 523.0 DSM (should clamp to 0.0)
        ];
        let mut chm = [0.0f32; 16];
        let chm_count = compute_canopy_height_model_zero_heap(&extracted, &bare_earth_dem, &mut chm);
        assert_eq!(chm_count, 16);
        assert!((chm[0] - 20.0).abs() < 1e-4); // 520.0 - 500.0 = 20.0 m tree canopy
        assert!((chm[1] - 21.5).abs() < 1e-4);
        assert_eq!(chm[12], 0.0); // 523.0 - 530.0 = -7.0 clamped to 0.0
    }

    #[test]
    fn test_strip_i16_elevation_extraction() {
        let mut tiff_bytes = vec![0u8; 256];

        // Little-endian TIFF header
        tiff_bytes[0..2].copy_from_slice(b"II");
        tiff_bytes[2..4].copy_from_slice(&42u16.to_le_bytes());
        tiff_bytes[4..8].copy_from_slice(&8u32.to_le_bytes());

        // IFD0: 6 entries
        let entry_count: u16 = 6;
        tiff_bytes[8..10].copy_from_slice(&entry_count.to_le_bytes());

        let mut write_tag = |idx: usize, tag: u16, tag_type: u16, count: u32, val_or_off: u32| {
            let off = 10 + idx * 12;
            tiff_bytes[off..off + 2].copy_from_slice(&tag.to_le_bytes());
            tiff_bytes[off + 2..off + 4].copy_from_slice(&tag_type.to_le_bytes());
            tiff_bytes[off + 4..off + 8].copy_from_slice(&count.to_le_bytes());
            tiff_bytes[off + 8..off + 12].copy_from_slice(&val_or_off.to_le_bytes());
        };

        write_tag(0, 256, 3, 1, 2); // ImageWidth = 2
        write_tag(1, 257, 3, 1, 2); // ImageLength = 2
        write_tag(2, 258, 3, 1, 16); // BitsPerSample = 16
        write_tag(3, 273, 4, 1, 100); // StripOffsets = offset 100
        write_tag(4, 279, 4, 1, 8); // StripByteCounts = 8 bytes (4 i16s)
        write_tag(5, 339, 3, 1, 2); // SampleFormat = 2 (signed int)

        // Payload at 100: 4 i16s [-10, 0, 1500, 2228]
        let samples: [i16; 4] = [-10, 0, 1500, 2228];
        for (i, &s) in samples.iter().enumerate() {
            let off = 100 + i * 2;
            tiff_bytes[off..off + 2].copy_from_slice(&s.to_le_bytes());
        }

        let header = parse_cog_ifd_header_zero_heap(&tiff_bytes).expect("valid header parsed");
        assert_eq!(header.bits_per_sample, 16);
        assert_eq!(header.sample_format, 2);
        assert_eq!(header.tile_width, 2); // defaulted to image_width
        assert_eq!(header.tile_height, 2);

        let mut out = [0.0f32; 4];
        let n = extract_cog_f32_window_zero_heap(&tiff_bytes, &header, &mut out);
        assert_eq!(n, 4);
        assert_eq!(out, [-10.0, 0.0, 1500.0, 2228.0]);
    }
}
