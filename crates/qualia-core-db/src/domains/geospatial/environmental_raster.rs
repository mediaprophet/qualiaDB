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
}
