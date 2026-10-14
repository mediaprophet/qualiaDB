//! Continuous environmental raster and GeoTIFF conversion pipeline (§GAP-02).
//!
//! Provides continuous `f32` field handling for environmental layers (elevation,
//! slope, canopy height, canopy cover, solar radiation, soil moisture, rainfall),
//! derived raster calculations, bilinear spatial sampling, and serialisation into
//! the `.10d` Field section (`FIEL`).

use crate::container_10d::field_section::{
    encode_field_section_with_rasters, ContinuousRasterField, FieldSectionError,
};
use crate::q_hash;

/// Standard environmental raster layer identifiers and units.
pub const ENV_ELEVATION_URI: &str = "qualia:env:elevation";
pub const ENV_SLOPE_URI: &str = "qualia:env:slope";
pub const ENV_CANOPY_HEIGHT_URI: &str = "qualia:env:canopy_height";
pub const ENV_CANOPY_COVER_URI: &str = "qualia:env:canopy_cover";
pub const ENV_SOLAR_RADIATION_URI: &str = "qualia:env:solar_radiation";
pub const ENV_SOIL_MOISTURE_URI: &str = "qualia:env:soil_moisture";
pub const ENV_RAINFALL_URI: &str = "qualia:env:rainfall";

pub const UNIT_METER: &str = "qudt:Meter";
pub const UNIT_DEGREE: &str = "qudt:Degree";
pub const UNIT_PERCENT: &str = "qudt:Percent";
pub const UNIT_WATT_PER_SQM: &str = "qudt:WattPerSquareMeter";
pub const UNIT_MILLIMETER: &str = "qudt:MilliMeter";

/// Continuous environmental raster layer representation.
#[derive(Debug, Clone)]
pub struct EnvironmentalRasterLayer<'a> {
    pub name_uri: &'static str,
    pub unit_uri: &'static str,
    pub width: usize,
    pub height: usize,
    pub cell_size_m: f64,
    pub bbox: (f64, f64, f64, f64), // west, south, east, north
    pub samples: &'a [f32],
}

impl<'a> EnvironmentalRasterLayer<'a> {
    pub fn new(
        name_uri: &'static str,
        unit_uri: &'static str,
        width: usize,
        height: usize,
        cell_size_m: f64,
        bbox: (f64, f64, f64, f64),
        samples: &'a [f32],
    ) -> Self {
        assert_eq!(
            samples.len(),
            width * height,
            "Raster samples length must equal width * height"
        );
        Self {
            name_uri,
            unit_uri,
            width,
            height,
            cell_size_m,
            bbox,
            samples,
        }
    }

    /// Sample the continuous field at normalised UV coordinates `[0.0, 1.0]` using bilinear interpolation.
    #[inline]
    pub fn sample_uv(&self, u: f32, v: f32) -> f32 {
        sample_bilinear(self.samples, self.width, self.height, u, v)
    }

    /// Sample the continuous field at geodetic coordinates (lon, lat) within the layer's bbox.
    pub fn sample_geo(&self, lon: f64, lat: f64) -> Option<f32> {
        let (w, s, e, n) = self.bbox;
        if lon < w || lon > e || lat < s || lat > n {
            return None;
        }
        let u = ((lon - w) / (e - w)).clamp(0.0, 1.0) as f32;
        let v = ((n - lat) / (n - s)).clamp(0.0, 1.0) as f32;
        Some(self.sample_uv(u, v))
    }
}

/// Bilinear interpolation of a continuous 2D scalar grid.
/// Zero heap, hot path.
pub fn sample_bilinear(grid: &[f32], width: usize, height: usize, u: f32, v: f32) -> f32 {
    if width == 0 || height == 0 || grid.is_empty() {
        return 0.0;
    }
    let u_clamped = u.clamp(0.0, 1.0);
    let v_clamped = v.clamp(0.0, 1.0);

    let fx = u_clamped * (width.saturating_sub(1) as f32);
    let fy = v_clamped * (height.saturating_sub(1) as f32);

    let x0 = fx.floor() as usize;
    let y0 = fy.floor() as usize;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);

    let wx = fx - x0 as f32;
    let wy = fy - y0 as f32;

    let v00 = grid[y0 * width + x0];
    let v10 = grid[y0 * width + x1];
    let v01 = grid[y1 * width + x0];
    let v11 = grid[y1 * width + x1];

    let top = v00 * (1.0 - wx) + v10 * wx;
    let bot = v01 * (1.0 - wx) + v11 * wx;
    top * (1.0 - wy) + bot * wy
}

/// Derives slope in degrees from an elevation heightfield using Horn's 3x3 finite-difference algorithm.
/// Zero heap caller-buffered.
pub fn derive_slope_horn(
    elevation: &[f32],
    width: usize,
    height: usize,
    cell_size_m: f64,
    out: &mut [f32],
) {
    assert_eq!(elevation.len(), width * height);
    assert_eq!(out.len(), width * height);
    if width < 2 || height < 2 || cell_size_m <= 0.0 {
        out.fill(0.0);
        return;
    }

    let scale = (8.0 * cell_size_m) as f32;

    for y in 0..height {
        let ym = y.saturating_sub(1);
        let yp = (y + 1).min(height - 1);

        for x in 0..width {
            let xm = x.saturating_sub(1);
            let xp = (x + 1).min(width - 1);

            let z_tl = elevation[ym * width + xm];
            let z_tc = elevation[ym * width + x];
            let z_tr = elevation[ym * width + xp];

            let z_ml = elevation[y * width + xm];
            let z_mr = elevation[y * width + xp];

            let z_bl = elevation[yp * width + xm];
            let z_bc = elevation[yp * width + x];
            let z_br = elevation[yp * width + xp];

            // Horizontal gradient dz/dx
            let dz_dx = ((z_tr + 2.0 * z_mr + z_br) - (z_tl + 2.0 * z_ml + z_bl)) / scale;
            // Vertical gradient dz/dy
            let dz_dy = ((z_bl + 2.0 * z_bc + z_br) - (z_tl + 2.0 * z_tc + z_tr)) / scale;

            let rise_run = (dz_dx * dz_dx + dz_dy * dz_dy).sqrt();
            let slope_deg = rise_run.atan().to_degrees().clamp(0.0, 90.0);

            out[y * width + x] = slope_deg;
        }
    }
}

/// Derives a local canopy cover percentage grid `[0.0, 100.0]` from canopy height values.
/// Any pixel with height >= `min_height_m` is considered canopy.
pub fn derive_canopy_cover_grid(
    canopy_height: &[f32],
    min_height_m: f32,
    window_radius: usize,
    width: usize,
    height: usize,
    out: &mut [f32],
) {
    assert_eq!(canopy_height.len(), width * height);
    assert_eq!(out.len(), width * height);

    for y in 0..height {
        let y_min = y.saturating_sub(window_radius);
        let y_max = (y + window_radius).min(height - 1);

        for x in 0..width {
            let x_min = x.saturating_sub(window_radius);
            let x_max = (x + window_radius).min(width - 1);

            let mut count = 0usize;
            let mut canopy_count = 0usize;

            for wy in y_min..=y_max {
                for wx in x_min..=x_max {
                    count += 1;
                    if canopy_height[wy * width + wx] >= min_height_m {
                        canopy_count += 1;
                    }
                }
            }

            let pct = if count > 0 {
                (canopy_count as f32 / count as f32) * 100.0
            } else {
                0.0
            };
            out[y * width + x] = pct;
        }
    }
}

/// Error kinds for GeoTIFF parsing and decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeoTiffError {
    InvalidMagic,
    UnsupportedEndian,
    MalformedIfd,
    UnsupportedCompression,
    UnsupportedFormat,
    BufferTooSmall,
    TruncatedData,
}

/// Metadata and spatial bounds extracted from a GeoTIFF header.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoTiffRasterInfo {
    pub width: usize,
    pub height: usize,
    pub cell_size_m: f64,
    pub bbox: (f64, f64, f64, f64), // west, south, east, north
    pub sample_format: u16,         // 1 = UInt, 2 = Int, 3 = IEEE Float
    pub bits_per_sample: u16,
    pub sample_count: usize,
}

/// Zero-heap GeoTIFF raster decoder.
///
/// Decodes uncompressed (raw/strip) raster bands (Float32, Int16, or UInt16)
/// into a caller-supplied `&mut [f32]` buffer, extracting spatial resolution,
/// georeferenced bounding box (`ModelTiepointTag` + `ModelPixelScaleTag`),
/// and dimensions.
pub fn decode_geotiff_raster_into(
    tiff_bytes: &[u8],
    out: &mut [f32],
) -> Result<GeoTiffRasterInfo, GeoTiffError> {
    if tiff_bytes.len() < 8 {
        return Err(GeoTiffError::TruncatedData);
    }

    let is_le = match &tiff_bytes[0..2] {
        b"II" => true,
        b"MM" => false,
        _ => return Err(GeoTiffError::InvalidMagic),
    };

    if !is_le {
        // Little-endian is the canonical Qualia standard
        return Err(GeoTiffError::UnsupportedEndian);
    }

    let magic = u16::from_le_bytes(tiff_bytes[2..4].try_into().unwrap());
    if magic != 42 {
        return Err(GeoTiffError::InvalidMagic);
    }

    let ifd_offset = u32::from_le_bytes(tiff_bytes[4..8].try_into().unwrap()) as usize;
    if ifd_offset + 2 > tiff_bytes.len() {
        return Err(GeoTiffError::TruncatedData);
    }

    let num_entries = u16::from_le_bytes(tiff_bytes[ifd_offset..ifd_offset + 2].try_into().unwrap()) as usize;
    let entries_start = ifd_offset + 2;
    let entries_end = entries_start + num_entries * 12;
    if entries_end > tiff_bytes.len() {
        return Err(GeoTiffError::MalformedIfd);
    }

    let mut width = 0usize;
    let mut height = 0usize;
    let mut bits_per_sample = 32u16;
    let mut compression = 1u16; // 1 = None
    let mut sample_format = 3u16; // default Float32
    let mut strip_offsets_val = 0u32;
    let mut strip_offsets_count = 0u32;
    let mut _strip_byte_counts = 0u32;

    let mut tiepoint_origin = (0.0f64, 0.0f64);
    let mut pixel_scale = (1.0f64, 1.0f64);
    let mut has_tiepoint = false;
    let mut _has_scale = false;

    for i in 0..num_entries {
        let entry_off = entries_start + i * 12;
        let tag = u16::from_le_bytes(tiff_bytes[entry_off..entry_off + 2].try_into().unwrap());
        let field_type = u16::from_le_bytes(tiff_bytes[entry_off + 2..entry_off + 4].try_into().unwrap());
        let count = u32::from_le_bytes(tiff_bytes[entry_off + 4..entry_off + 8].try_into().unwrap());
        let val_or_off = u32::from_le_bytes(tiff_bytes[entry_off + 8..entry_off + 12].try_into().unwrap());

        let read_u32 = || -> u32 {
            if field_type == 3 {
                // SHORT
                val_or_off & 0xFFFF
            } else {
                val_or_off
            }
        };

        match tag {
            256 => width = read_u32() as usize,
            257 => height = read_u32() as usize,
            258 => bits_per_sample = read_u32() as u16,
            259 => compression = read_u32() as u16,
            273 => {
                strip_offsets_val = val_or_off;
                strip_offsets_count = count;
            }
            279 => _strip_byte_counts = read_u32(),
            339 => sample_format = read_u32() as u16,
            // ModelPixelScaleTag (33550)
            33550 if field_type == 12 && count >= 2 => {
                let off = val_or_off as usize;
                if off + 16 <= tiff_bytes.len() {
                    let sx = f64::from_le_bytes(tiff_bytes[off..off + 8].try_into().unwrap());
                    let sy = f64::from_le_bytes(tiff_bytes[off + 8..off + 16].try_into().unwrap());
                    pixel_scale = (sx, sy);
                    _has_scale = true;
                }
            }
            // ModelTiepointTag (33922) - typically 6 doubles: (I, J, K, X, Y, Z)
            33922 if field_type == 12 && count >= 6 => {
                let off = val_or_off as usize;
                if off + 48 <= tiff_bytes.len() {
                    let x = f64::from_le_bytes(tiff_bytes[off + 24..off + 32].try_into().unwrap());
                    let y = f64::from_le_bytes(tiff_bytes[off + 32..off + 40].try_into().unwrap());
                    tiepoint_origin = (x, y);
                    has_tiepoint = true;
                }
            }
            _ => {}
        }
    }

    if width == 0 || height == 0 {
        return Err(GeoTiffError::MalformedIfd);
    }
    if compression != 1 {
        return Err(GeoTiffError::UnsupportedCompression);
    }

    let total_samples = width * height;
    if out.len() < total_samples {
        return Err(GeoTiffError::BufferTooSmall);
    }

    let data_offset = if strip_offsets_count == 1 {
        strip_offsets_val as usize
    } else {
        // If multiple strips or offset points to array, read first offset
        let off = strip_offsets_val as usize;
        if off + 4 <= tiff_bytes.len() {
            u32::from_le_bytes(tiff_bytes[off..off + 4].try_into().unwrap()) as usize
        } else {
            return Err(GeoTiffError::TruncatedData);
        }
    };

    // Decode sample values
    match (sample_format, bits_per_sample) {
        (3, 32) => {
            // IEEE Float32
            let bytes_needed = total_samples * 4;
            if data_offset + bytes_needed > tiff_bytes.len() {
                return Err(GeoTiffError::TruncatedData);
            }
            for i in 0..total_samples {
                let b_off = data_offset + i * 4;
                out[i] = f32::from_le_bytes(tiff_bytes[b_off..b_off + 4].try_into().unwrap());
            }
        }
        (1, 16) | (2, 16) => {
            // 16-bit integer
            let bytes_needed = total_samples * 2;
            if data_offset + bytes_needed > tiff_bytes.len() {
                return Err(GeoTiffError::TruncatedData);
            }
            for i in 0..total_samples {
                let b_off = data_offset + i * 2;
                let val = if sample_format == 1 {
                    u16::from_le_bytes(tiff_bytes[b_off..b_off + 2].try_into().unwrap()) as f32
                } else {
                    i16::from_le_bytes(tiff_bytes[b_off..b_off + 2].try_into().unwrap()) as f32
                };
                out[i] = val;
            }
        }
        _ => return Err(GeoTiffError::UnsupportedFormat),
    }

    // Compute bounding box
    let (west, north) = if has_tiepoint {
        tiepoint_origin
    } else {
        (0.0, (height as f64) * pixel_scale.1)
    };
    let east = west + (width as f64) * pixel_scale.0;
    let south = north - (height as f64) * pixel_scale.1;
    let bbox = (west, south, east, north);

    let cell_size_m = pixel_scale.0;

    Ok(GeoTiffRasterInfo {
        width,
        height,
        cell_size_m,
        bbox,
        sample_format,
        bits_per_sample,
        sample_count: total_samples,
    })
}

/// Compile a collection of environmental raster layers into `.10d` Field section bytes.
pub fn compile_environmental_rasters_to_10d(
    layers: &[EnvironmentalRasterLayer],
    out: &mut [u8],
) -> Result<usize, FieldSectionError> {
    let mut fields = Vec::with_capacity(layers.len());
    for l in layers {
        fields.push(ContinuousRasterField {
            name_hash: q_hash(l.name_uri),
            unit_hash: q_hash(l.unit_uri),
            width: l.width as u32,
            height: l.height as u32,
            samples: l.samples,
        });
    }

    encode_field_section_with_rasters(&fields, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bilinear_interpolation() {
        // 2x2 grid:
        // 0.0  10.0
        // 20.0 30.0
        let grid = [0.0f32, 10.0, 20.0, 30.0];
        assert_eq!(sample_bilinear(&grid, 2, 2, 0.0, 0.0), 0.0);
        assert_eq!(sample_bilinear(&grid, 2, 2, 1.0, 0.0), 10.0);
        assert_eq!(sample_bilinear(&grid, 2, 2, 0.0, 1.0), 20.0);
        assert_eq!(sample_bilinear(&grid, 2, 2, 1.0, 1.0), 30.0);
        assert_eq!(sample_bilinear(&grid, 2, 2, 0.5, 0.5), 15.0);
    }

    #[test]
    fn test_derive_slope_flat_and_steep() {
        let flat = [100.0f32; 9];
        let mut slope = [0.0f32; 9];
        derive_slope_horn(&flat, 3, 3, 10.0, &mut slope);
        for &s in &slope {
            assert!(s.abs() < 1e-4);
        }

        // 3x3 inclined ramp: elevation increases 10m per 10m cell horizontally
        let ramp = [
            0.0f32, 10.0, 20.0,
            0.0,    10.0, 20.0,
            0.0,    10.0, 20.0,
        ];
        let mut slope_ramp = [0.0f32; 9];
        derive_slope_horn(&ramp, 3, 3, 10.0, &mut slope_ramp);
        // Centre cell dz/dx = 1.0 -> 45 degrees slope
        assert!((slope_ramp[4] - 45.0).abs() < 1.0);
    }

    #[test]
    fn test_compile_environmental_rasters_to_10d() {
        let elev = [100.0f32, 105.0, 110.0, 115.0];
        let canopy = [2.0f32, 15.0, 18.0, 0.5];

        let layers = [
            EnvironmentalRasterLayer::new(
                ENV_ELEVATION_URI,
                UNIT_METER,
                2,
                2,
                5.0,
                (144.0, -37.0, 144.1, -36.9),
                &elev,
            ),
            EnvironmentalRasterLayer::new(
                ENV_CANOPY_HEIGHT_URI,
                UNIT_METER,
                2,
                2,
                5.0,
                (144.0, -37.0, 144.1, -36.9),
                &canopy,
            ),
        ];

        let mut out = vec![0u8; 1024];
        let written = compile_environmental_rasters_to_10d(&layers, &mut out).unwrap();
        assert!(written > 0);
    }

    #[test]
    fn test_decode_geotiff_raster_into() {
        // Construct a synthetic 2x2 Float32 GeoTIFF in memory
        let mut tiff = Vec::new();
        // 1. Header (8 bytes)
        tiff.extend_from_slice(b"II"); // Little endian
        tiff.extend_from_slice(&42u16.to_le_bytes()); // Magic
        tiff.extend_from_slice(&8u32.to_le_bytes()); // IFD offset at 8

        // 2. IFD at offset 8: 7 entries
        let entries_count = 7u16;
        tiff.extend_from_slice(&entries_count.to_le_bytes());

        let strip_data_offset = 120u32;

        let mut push_entry = |tag: u16, ftype: u16, count: u32, val_or_off: u32| {
            tiff.extend_from_slice(&tag.to_le_bytes());
            tiff.extend_from_slice(&ftype.to_le_bytes());
            tiff.extend_from_slice(&count.to_le_bytes());
            tiff.extend_from_slice(&val_or_off.to_le_bytes());
        };

        push_entry(256, 4, 1, 2); // Width = 2
        push_entry(257, 4, 1, 2); // Height = 2
        push_entry(258, 3, 1, 32); // BitsPerSample = 32
        push_entry(259, 3, 1, 1); // Compression = 1 (None)
        push_entry(273, 4, 1, strip_data_offset); // StripOffsets
        push_entry(279, 4, 1, 16); // StripByteCounts = 16 (4 * 4 bytes)
        push_entry(339, 3, 1, 3); // SampleFormat = 3 (Float32)

        tiff.extend_from_slice(&0u32.to_le_bytes()); // Next IFD = 0

        // Pad up to strip_data_offset
        while tiff.len() < strip_data_offset as usize {
            tiff.push(0);
        }

        // Float samples: 10.5, 20.5, 30.5, 40.5
        tiff.extend_from_slice(&10.5f32.to_le_bytes());
        tiff.extend_from_slice(&20.5f32.to_le_bytes());
        tiff.extend_from_slice(&30.5f32.to_le_bytes());
        tiff.extend_from_slice(&40.5f32.to_le_bytes());

        let mut out = [0.0f32; 4];
        let info = decode_geotiff_raster_into(&tiff, &mut out).expect("should decode GeoTIFF");
        assert_eq!(info.width, 2);
        assert_eq!(info.height, 2);
        assert_eq!(info.sample_count, 4);
        assert_eq!(out, [10.5, 20.5, 30.5, 40.5]);
    }
}
