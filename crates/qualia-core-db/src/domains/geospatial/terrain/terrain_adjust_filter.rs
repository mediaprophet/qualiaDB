// Copyright 2026 QualiaDB Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! High-precision terrain heightfield adjustments and spatial filters.
//!
//! Caller-buffered, zero-heap operators for geomorphic modeling, geological
//! benching/stratification, morphological filtering, and differential curvature.

// ---------------------------------------------------------------------------
// 1. Per-Cell & Global Adjustments
// ---------------------------------------------------------------------------

/// Smoothstepped geological benching/terracing.
#[inline]
pub fn apply_terrace(height: f32, steps: u32, sharpness: f32) -> f32 {
    if steps == 0 { return height; }
    let s = steps as f32;
    let h = height * s;
    let base = h.floor();
    let f = h - base;
    let k = sharpness.clamp(0.0, 1.0);
    let w = 0.5 * (1.0 - k);
    let stepped = if w < 1e-6 {
        if f < 0.5 { 0.0 } else { 1.0 }
    } else if f < 0.5 - w {
        0.0
    } else if f > 0.5 + w {
        1.0
    } else {
        let u = (f - (0.5 - w)) / (2.0 * w);
        u * u * (3.0 - 2.0 * u)
    };
    (base + f * (1.0 - k) + stepped * k) / s
}

/// Differential sedimentary bed ledges with hard/soft rock layers.
#[inline]
pub fn apply_stratify(height: f32, frequency: f32, hard_layer_ratio: f32, strength: f32) -> f32 {
    if frequency <= 0.0 || strength <= 0.0 { return height; }
    let (s, r) = (strength.clamp(0.0, 1.0), hard_layer_ratio.clamp(0.05, 0.95));
    let layer = height * frequency;
    let (base, p) = (layer.floor(), layer - layer.floor());
    let p_warped = if p < r {
        let u = p / r;
        u * u * (3.0 - 2.0 * u) * r
    } else {
        let v = (p - r) / (1.0 - r);
        r + v * v * (3.0 - 2.0 * v) * (1.0 - r)
    };
    (base + p * (1.0 - s) + p_warped * s) / frequency
}

/// Tectonic anticline and syncline fold deformation around a neutral plane.
#[inline]
pub fn apply_fold(height: f32, neutral_plane: f32, compression: f32, asymmetry: f32) -> f32 {
    if compression == 0.0 { return height; }
    let z = height - neutral_plane;
    let norm_z = z / (z * z + 1.0).sqrt();
    neutral_plane + z * (1.0 + compression * (1.0 + asymmetry * norm_z))
}

/// Bilinear-interpolated coordinate shear centered on the grid midpoint.
pub fn apply_shear_grid(
    src: &[f32], width: usize, height: usize, shear_x: f32, shear_y: f32, out: &mut [f32],
) -> Result<(), &'static str> {
    if src.len() != width * height || out.len() != width * height { return Err("Invalid buffer length"); }
    if width == 0 || height == 0 { return Err("Zero dimension"); }
    let (cx, cy) = ((width as f32 - 1.0) * 0.5, (height as f32 - 1.0) * 0.5);
    for y in 0..height {
        let dy = y as f32 - cy;
        for x in 0..width {
            let dx = x as f32 - cx;
            let sx = (x as f32 - shear_x * dy).clamp(0.0, width as f32 - 1.0);
            let sy = (y as f32 - shear_y * dx).clamp(0.0, height as f32 - 1.0);
            let (x0, y0) = (sx.floor() as usize, sy.floor() as usize);
            let (x1, y1) = ((x0 + 1).min(width - 1), (y0 + 1).min(height - 1));
            let (fx, fy) = (sx - x0 as f32, sy - y0 as f32);
            let top = src[y0 * width + x0] * (1.0 - fx) + src[y0 * width + x1] * fx;
            let bot = src[y1 * width + x0] * (1.0 - fx) + src[y1 * width + x1] * fx;
            out[y * width + x] = top * (1.0 - fy) + bot * fy;
        }
    }
    Ok(())
}

/// Piecewise cubic Hermite transfer curve on `[0, 1]` with 8 uniformly spaced knots.
#[inline]
pub fn apply_curve_lut(height: f32, control_y: &[f32; 8]) -> f32 {
    let u = height.clamp(0.0, 1.0) * 7.0;
    let i = (u.floor() as usize).min(6);
    let t = u - i as f32;
    let (y0, y1) = (control_y[i], control_y[i + 1]);
    let m0 = if i == 0 { control_y[1] - y0 } else { 0.5 * (y1 - control_y[i - 1]) };
    let m1 = if i + 1 == 7 { control_y[7] - control_y[6] } else { 0.5 * (control_y[i + 2] - y0) };
    let (t2, t3) = (t * t, t * t * t);
    (2.0 * t3 - 3.0 * t2 + 1.0) * y0 + (t3 - 2.0 * t2 + t) * m0 + (-2.0 * t3 + 3.0 * t2) * y1 + (t3 - t2) * m1
}

/// In-place clip and remap to `[out_min, out_max]`.
pub fn apply_clamp_normalize(grid: &mut [f32], clip_min: f32, clip_max: f32, out_min: f32, out_max: f32) {
    if clip_max <= clip_min {
        grid.fill(out_min);
        return;
    }
    let (in_span, out_span) = (clip_max - clip_min, out_max - out_min);
    for v in grid.iter_mut() {
        *v = out_min + (v.clamp(clip_min, clip_max) - clip_min) / in_span * out_span;
    }
}

/// Stretch active dynamic range `[min, max]` to `[target_min, target_max]`.
pub fn apply_autolevel(grid: &mut [f32], target_min: f32, target_max: f32) {
    if grid.is_empty() { return; }
    let (mut min, mut max) = (f32::INFINITY, f32::NEG_INFINITY);
    for &v in grid.iter() {
        if !v.is_nan() { min = min.min(v); max = max.max(v); }
    }
    if min.is_infinite() || max.is_infinite() { return; }
    let span = max - min;
    if span <= 1e-7 {
        grid.fill(target_min);
    } else {
        let target_span = target_max - target_min;
        for v in grid.iter_mut() { *v = target_min + (*v - min) / span * target_span; }
    }
}

/// 256-bin stack-allocated histogram CDF equalization on `[0, 1]`.
pub fn apply_histogram_equalize(src: &[f32], out: &mut [f32]) -> Result<(), &'static str> {
    if src.len() != out.len() { return Err("Mismatched slice lengths"); }
    if src.is_empty() { return Ok(()); }
    let mut hist = [0u32; 256];
    for &v in src.iter() {
        hist[((v.clamp(0.0, 1.0) * 255.0).round() as usize).min(255)] += 1;
    }
    let (mut cdf, mut accum, mut cdf_min, mut found_min) = ([0u32; 256], 0u32, 0u32, false);
    for i in 0..256 {
        accum += hist[i];
        cdf[i] = accum;
        if !found_min && accum > 0 { cdf_min = accum; found_min = true; }
    }
    let total = src.len() as u32;
    if total <= cdf_min {
        out.copy_from_slice(src);
        return Ok(());
    }
    let denom = (total - cdf_min) as f32;
    let mut lut = [0.0f32; 256];
    for i in 0..256 {
        lut[i] = (cdf[i].saturating_sub(cdf_min) as f32 / denom).clamp(0.0, 1.0);
    }
    for (i, &v) in src.iter().enumerate() {
        out[i] = lut[((v.clamp(0.0, 1.0) * 255.0).round() as usize).min(255)];
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 2. Spatial & Morphological Filters
// ---------------------------------------------------------------------------

/// 3x3 morphological dilation (local maximum).
pub fn filter_dilate_3x3(src: &[f32], width: usize, height: usize, out: &mut [f32]) -> Result<(), &'static str> {
    if src.len() != width * height || out.len() != width * height { return Err("Invalid buffer length"); }
    if width == 0 || height == 0 { return Err("Zero dimension"); }
    for y in 0..height {
        let yi = y as isize;
        for x in 0..width {
            let (xi, mut max_v) = (x as isize, f32::NEG_INFINITY);
            for dy in -1..=1 {
                let cy = (yi + dy).clamp(0, height as isize - 1) as usize;
                for dx in -1..=1 {
                    let cx = (xi + dx).clamp(0, width as isize - 1) as usize;
                    max_v = max_v.max(src[cy * width + cx]);
                }
            }
            out[y * width + x] = max_v;
        }
    }
    Ok(())
}

/// 3x3 morphological erosion (local minimum).
pub fn filter_erode_morph_3x3(src: &[f32], width: usize, height: usize, out: &mut [f32]) -> Result<(), &'static str> {
    if src.len() != width * height || out.len() != width * height { return Err("Invalid buffer length"); }
    if width == 0 || height == 0 { return Err("Zero dimension"); }
    for y in 0..height {
        let yi = y as isize;
        for x in 0..width {
            let (xi, mut min_v) = (x as isize, f32::INFINITY);
            for dy in -1..=1 {
                let cy = (yi + dy).clamp(0, height as isize - 1) as usize;
                for dx in -1..=1 {
                    let cx = (xi + dx).clamp(0, width as isize - 1) as usize;
                    min_v = min_v.min(src[cy * width + cx]);
                }
            }
            out[y * width + x] = min_v;
        }
    }
    Ok(())
}

/// Morphological aperture filter: ridge-sharpening and valley-widening.
pub fn filter_aperture_3x3(
    src: &[f32], width: usize, height: usize, strength: f32, out: &mut [f32],
) -> Result<(), &'static str> {
    if src.len() != width * height || out.len() != width * height { return Err("Invalid buffer length"); }
    if width == 0 || height == 0 { return Err("Zero dimension"); }
    let s = strength.clamp(0.0, 1.0);
    for y in 0..height {
        let yi = y as isize;
        for x in 0..width {
            let (xi, mut min_v, mut max_v) = (x as isize, f32::INFINITY, f32::NEG_INFINITY);
            for dy in -1..=1 {
                let cy = (yi + dy).clamp(0, height as isize - 1) as usize;
                for dx in -1..=1 {
                    let v = src[cy * width + (xi + dx).clamp(0, width as isize - 1) as usize];
                    min_v = min_v.min(v); max_v = max_v.max(v);
                }
            }
            let cur = src[y * width + x];
            let delta = max_v - min_v;
            out[y * width + x] = if delta < 1e-6 {
                cur
            } else {
                let u = (cur - 0.5 * (min_v + max_v)) / (0.5 * delta);
                if u >= 0.0 { cur + s * u * (max_v - cur) } else { cur + s * u * (cur - min_v) }
            };
        }
    }
    Ok(())
}

/// 3x3 median filter using a 9-element sorting network on stack for despiking.
pub fn filter_median_3x3(src: &[f32], width: usize, height: usize, out: &mut [f32]) -> Result<(), &'static str> {
    if src.len() != width * height || out.len() != width * height { return Err("Invalid buffer length"); }
    if width == 0 || height == 0 { return Err("Zero dimension"); }
    for y in 0..height {
        let yi = y as isize;
        for x in 0..width {
            let (xi, mut arr, mut k) = (x as isize, [0.0f32; 9], 0);
            for dy in -1..=1 {
                let cy = (yi + dy).clamp(0, height as isize - 1) as usize;
                for dx in -1..=1 {
                    arr[k] = src[cy * width + (xi + dx).clamp(0, width as isize - 1) as usize];
                    k += 1;
                }
            }
            for step in 0..9 {
                let mut i = step % 2;
                while i + 1 < 9 {
                    if arr[i] > arr[i + 1] { arr.swap(i, i + 1); }
                    i += 2;
                }
            }
            out[y * width + x] = arr[4];
        }
    }
    Ok(())
}

/// Separable 5x5 Gaussian filter `[1, 4, 6, 4, 1] / 16` horizontal + vertical pass.
pub fn filter_gaussian_5x5(
    src: &[f32], width: usize, height: usize, scratch: &mut [f32], out: &mut [f32],
) -> Result<(), &'static str> {
    let len = width * height;
    if src.len() != len || scratch.len() != len || out.len() != len { return Err("Invalid buffer length"); }
    if width == 0 || height == 0 { return Err("Zero dimension"); }
    for y in 0..height {
        let row = y * width;
        for x in 0..width {
            let xi = x as isize;
            let mut sum = 0.0f32;
            for (w, dx) in [(1.0, -2), (4.0, -1), (6.0, 0), (4.0, 1), (1.0, 2)] {
                sum += w * src[row + (xi + dx).clamp(0, width as isize - 1) as usize];
            }
            scratch[row + x] = sum / 16.0;
        }
    }
    for y in 0..height {
        let yi = y as isize;
        for x in 0..width {
            let mut sum = 0.0f32;
            for (w, dy) in [(1.0, -2), (4.0, -1), (6.0, 0), (4.0, 1), (1.0, 2)] {
                let cy = (yi + dy).clamp(0, height as isize - 1) as usize;
                sum += w * scratch[cy * width + x];
            }
            out[y * width + x] = sum / 16.0;
        }
    }
    Ok(())
}

/// 2-pass Rosenfeld-Pfaltz `3-4` chamfer distance transform in-place on `out_dist`.
pub fn filter_chamfer_distance_transform(
    mask: &[u8], width: usize, height: usize, cell_size: f32, out_dist: &mut [f32],
) -> Result<(), &'static str> {
    let len = width * height;
    if mask.len() != len || out_dist.len() != len { return Err("Invalid buffer length"); }
    if width == 0 || height == 0 { return Err("Zero dimension"); }
    if cell_size <= 0.0 { return Err("Invalid cell_size"); }
    let (d_ortho, d_diag) = (cell_size, cell_size * (4.0 / 3.0));
    for i in 0..len { out_dist[i] = if mask[i] != 0 { 0.0 } else { 1e9 }; }
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let mut d = out_dist[idx];
            if y > 0 {
                d = d.min(out_dist[(y - 1) * width + x] + d_ortho);
                if x > 0 { d = d.min(out_dist[(y - 1) * width + (x - 1)] + d_diag); }
                if x + 1 < width { d = d.min(out_dist[(y - 1) * width + (x + 1)] + d_diag); }
            }
            if x > 0 { d = d.min(out_dist[y * width + (x - 1)] + d_ortho); }
            out_dist[idx] = d;
        }
    }
    for y in (0..height).rev() {
        for x in (0..width).rev() {
            let idx = y * width + x;
            let mut d = out_dist[idx];
            if y + 1 < height {
                d = d.min(out_dist[(y + 1) * width + x] + d_ortho);
                if x > 0 { d = d.min(out_dist[(y + 1) * width + (x - 1)] + d_diag); }
                if x + 1 < width { d = d.min(out_dist[(y + 1) * width + (x + 1)] + d_diag); }
            }
            if x + 1 < width { d = d.min(out_dist[y * width + (x + 1)] + d_ortho); }
            out_dist[idx] = d;
        }
    }
    Ok(())
}

/// Zevenbergen-Thorne 3x3 second-derivative mean, planform, and profile curvature.
pub fn filter_curvature_map(
    src: &[f32],
    width: usize,
    height: usize,
    cell_size: f32,
    out_mean_curv: &mut [f32],
    out_plan_curv: &mut [f32],
    out_profile_curv: &mut [f32],
) -> Result<(), &'static str> {
    let len = width * height;
    if src.len() != len || out_mean_curv.len() != len || out_plan_curv.len() != len || out_profile_curv.len() != len {
        return Err("Buffer length mismatch");
    }
    if width == 0 || height == 0 { return Err("Zero dimension"); }
    if cell_size <= 0.0 { return Err("Invalid cell_size"); }
    let (l, l2) = (cell_size, cell_size * cell_size);
    let get = |xi: isize, yi: isize| -> f32 {
        let cx = xi.clamp(0, width as isize - 1) as usize;
        let cy = yi.clamp(0, height as isize - 1) as usize;
        src[cy * width + cx]
    };
    for y in 0..height {
        let yi = y as isize;
        for x in 0..width {
            let xi = x as isize;
            let (z1, z2, z3) = (get(xi - 1, yi - 1), get(xi, yi - 1), get(xi + 1, yi - 1));
            let (z4, z5, z6) = (get(xi - 1, yi),     get(xi, yi),     get(xi + 1, yi));
            let (z7, z8, z9) = (get(xi - 1, yi + 1), get(xi, yi + 1), get(xi + 1, yi + 1));
            let d = (z4 + z6 - 2.0 * z5) / (2.0 * l2);
            let e = (z2 + z8 - 2.0 * z5) / (2.0 * l2);
            let f = (-z1 + z3 + z7 - z9) / (4.0 * l2);
            let g = (z6 - z4) / (2.0 * l);
            let h = (z2 - z8) / (2.0 * l);
            let slope_sq = g * g + h * h;
            let idx = y * width + x;
            out_mean_curv[idx] = -(d + e);
            if slope_sq > 1e-10 {
                out_profile_curv[idx] = -2.0 * (d * g * g + e * h * h + f * g * h) / slope_sq;
                out_plan_curv[idx] = 2.0 * (d * h * h + e * g * g - f * g * h) / slope_sq;
            } else {
                out_profile_curv[idx] = 0.0;
                out_plan_curv[idx] = 0.0;
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 3. Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adjustments_basic() {
        assert!((apply_terrace(0.5, 4, 0.0) - 0.5).abs() < 1e-4);
        assert!((apply_terrace(0.48, 4, 1.0) - 0.5).abs() < 1e-4);
        let s = apply_stratify(10.0, 1.0, 0.3, 0.5);
        assert!(s >= 9.0 && s <= 11.0);
        assert!((apply_fold(100.0, 100.0, 0.5, 0.2) - 100.0).abs() < 1e-4);
        assert!(apply_fold(110.0, 100.0, 0.5, 0.0) > 110.0);
        let lut = [0.0, 0.1, 0.2, 0.4, 0.6, 0.8, 0.9, 1.0];
        assert!((apply_curve_lut(0.0, &lut) - 0.0).abs() < 1e-4);
        assert!((apply_curve_lut(1.0, &lut) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_autolevel_and_histogram_equalize() {
        let mut grid = [10.0, 20.0, 30.0, 50.0];
        apply_autolevel(&mut grid, 0.0, 100.0);
        assert!((grid[0] - 0.0).abs() < 1e-4);
        assert!((grid[3] - 100.0).abs() < 1e-4);

        let src = [0.1, 0.2, 0.2, 0.9];
        let mut out = [0.0; 4];
        apply_histogram_equalize(&src, &mut out).unwrap();
        assert!(out[0] <= out[1] && out[1] <= out[3]);
    }

    #[test]
    fn test_spatial_morphology_and_median() {
        let src = [1.0, 1.0, 1.0, 1.0, 10.0, 1.0, 1.0, 1.0, 1.0];
        let mut out_dilate = [0.0; 9];
        filter_dilate_3x3(&src, 3, 3, &mut out_dilate).unwrap();
        assert_eq!(out_dilate[0], 10.0);

        let mut out_erode = [0.0; 9];
        filter_erode_morph_3x3(&src, 3, 3, &mut out_erode).unwrap();
        assert_eq!(out_erode[4], 1.0);

        let mut out_median = [0.0; 9];
        filter_median_3x3(&src, 3, 3, &mut out_median).unwrap();
        assert_eq!(out_median[4], 1.0); // spike removed!
    }

    #[test]
    fn test_gaussian_chamfer_and_curvature() {
        let src = [10.0; 25];
        let mut scratch = [0.0; 25];
        let mut out = [0.0; 25];
        filter_gaussian_5x5(&src, 5, 5, &mut scratch, &mut out).unwrap();
        assert!((out[12] - 10.0).abs() < 1e-4);

        let mask = [1, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut dist = [0.0; 9];
        filter_chamfer_distance_transform(&mask, 3, 3, 10.0, &mut dist).unwrap();
        assert_eq!(dist[0], 0.0);
        assert!((dist[1] - 10.0).abs() < 1e-4);
        assert!((dist[4] - (40.0 / 3.0)).abs() < 1e-4);

        let flat = [5.0; 9];
        let mut mean_c = [0.0; 9];
        let mut plan_c = [0.0; 9];
        let mut prof_c = [0.0; 9];
        filter_curvature_map(&flat, 3, 3, 1.0, &mut mean_c, &mut plan_c, &mut prof_c).unwrap();
        assert_eq!(mean_c[4], 0.0);
        assert_eq!(plan_c[4], 0.0);
        assert_eq!(prof_c[4], 0.0);
    }
}
