// Copyright (c) EarthCraft contributors
// Adapted under MIT License for QualiaDB.
//! Geomorphic mask selectors, layer combine modes, domain warp, and post-processing modifiers.
//! Allocation Class: Tier-1 `HotZeroHeap` (caller-buffered `&mut [f32]`).

/// Smoothstep band weight for continuous range selection `[min_val, max_val]` with falloff.
#[inline]
pub fn selector_band_weight(val: f32, min_val: f32, max_val: f32, falloff: f32) -> f32 {
    if val >= min_val && val <= max_val { return 1.0; }
    if falloff <= 0.0 { return 0.0; }
    if val < min_val {
        if val <= min_val - falloff { 0.0 } else { let t = (val - (min_val - falloff)) / falloff; t * t * (3.0 - 2.0 * t) }
    } else if val >= max_val + falloff {
        0.0
    } else {
        let t = (max_val + falloff - val) / falloff; t * t * (3.0 - 2.0 * t)
    }
}

/// 1. Selects terrain height range with smoothstep falloff.
pub fn select_height(dem: &[f32], min_h: f32, max_h: f32, falloff: f32, out_mask: &mut [f32]) -> Result<(), &'static str> {
    if dem.len() != out_mask.len() { return Err("dem and out_mask slice length mismatch"); }
    for (d, m) in dem.iter().zip(out_mask.iter_mut()) { *m = selector_band_weight(*d, min_h, max_h, falloff); }
    Ok(())
}

#[inline]
fn horn_gradient(dem: &[f32], w: usize, x: usize, y: usize, xm: usize, xp: usize, ym: usize, yp: usize, scale: f32) -> (f32, f32) {
    let dz_dx = ((dem[ym * w + xp] + 2.0 * dem[y * w + xp] + dem[yp * w + xp])
        - (dem[ym * w + xm] + 2.0 * dem[y * w + xm] + dem[yp * w + xm])) / scale;
    let dz_dy = ((dem[yp * w + xm] + 2.0 * dem[yp * w + x] + dem[yp * w + xp])
        - (dem[ym * w + xm] + 2.0 * dem[ym * w + x] + dem[ym * w + xp])) / scale;
    (dz_dx, dz_dy)
}

/// 2 & 3. Selects slope and optional aspect via Horn 3x3 gradient.
pub fn select_slope_aspect(
    dem: &[f32], width: usize, height: usize, cell_size: f32, min_slope_deg: f32, max_slope_deg: f32,
    target_aspect_deg: Option<f32>, aspect_tol_deg: f32, out_mask: &mut [f32],
) -> Result<(), &'static str> {
    if width == 0 || height == 0 || cell_size <= 0.0 { return Err("invalid grid dimensions or cell size"); }
    if dem.len() != width * height || out_mask.len() != width * height { return Err("slice length mismatch"); }
    let scale = 8.0 * cell_size;
    for y in 0..height {
        let (ym, yp) = (y.saturating_sub(1), (y + 1).min(height - 1));
        for x in 0..width {
            let (xm, xp) = (x.saturating_sub(1), (x + 1).min(width - 1));
            let (dz_dx, dz_dy) = horn_gradient(dem, width, x, y, xm, xp, ym, yp, scale);
            let rise_run = (dz_dx * dz_dx + dz_dy * dz_dy).sqrt();
            let slope_deg = rise_run.atan().to_degrees();
            let slope_w = selector_band_weight(slope_deg, min_slope_deg, max_slope_deg, 2.0);

            let weight = if let Some(target_deg) = target_aspect_deg {
                if rise_run < 1e-6 { 0.0 } else {
                    let mut aspect = (-dz_dx).atan2(dz_dy).to_degrees();
                    if aspect < 0.0 { aspect += 360.0; }
                    let mut diff = (aspect - target_deg).abs() % 360.0;
                    if diff > 180.0 { diff = 360.0 - diff; }
                    let tol = aspect_tol_deg.max(1e-4);
                    if diff <= tol { let t = 1.0 - (diff / tol); slope_w * t * t * (3.0 - 2.0 * t) } else { 0.0 }
                }
            } else { slope_w };
            out_mask[y * width + x] = weight;
        }
    }
    Ok(())
}

/// 4 & 5. Laplacian convexity and local-mean protrusion mask.
pub fn select_convexity_protrusion(
    dem: &[f32], width: usize, height: usize, cell_size: f32, out_convexity: &mut [f32], out_protrusion: &mut [f32],
) -> Result<(), &'static str> {
    if width == 0 || height == 0 || cell_size <= 0.0 { return Err("invalid grid dimensions or cell size"); }
    let n = width * height;
    if dem.len() != n || out_convexity.len() != n || out_protrusion.len() != n { return Err("slice length mismatch"); }
    let denom = (cell_size * cell_size).max(1e-6);
    for y in 0..height {
        let (ym, yp) = (y.saturating_sub(1), (y + 1).min(height - 1));
        for x in 0..width {
            let (xm, xp) = (x.saturating_sub(1), (x + 1).min(width - 1));
            let z_c = dem[y * width + x];
            let sum_cross = dem[ym * width + x] + dem[yp * width + x] + dem[y * width + xm] + dem[y * width + xp];
            let lap = (sum_cross - 4.0 * z_c) / denom;
            let sum_8 = dem[ym * width + xm] + dem[ym * width + x] + dem[ym * width + xp]
                + dem[y * width + xm] + dem[y * width + xp]
                + dem[yp * width + xm] + dem[yp * width + x] + dem[yp * width + xp];
            let idx = y * width + x;
            out_convexity[idx] = -lap;
            out_protrusion[idx] = (z_c - sum_8 / 8.0).max(0.0);
        }
    }
    Ok(())
}

/// 6. Local convergent drainage index from 8-neighbor higher-elevation inflow.
pub fn select_flow_accumulation_proxy(
    dem: &[f32], width: usize, height: usize, out_flow_mask: &mut [f32],
) -> Result<(), &'static str> {
    if width == 0 || height == 0 { return Err("invalid grid dimensions"); }
    let n = width * height;
    if dem.len() != n || out_flow_mask.len() != n { return Err("slice length mismatch"); }
    for y in 0..height {
        let (ym, yp) = (y.saturating_sub(1), (y + 1).min(height - 1));
        for x in 0..width {
            let (xm, xp) = (x.saturating_sub(1), (x + 1).min(width - 1));
            let z_c = dem[y * width + x];
            let neighbors = [
                (xm, ym, 1.4142135_f32), (x, ym, 1.0), (xp, ym, 1.4142135),
                (xm, y, 1.0),                          (xp, y, 1.0),
                (xm, yp, 1.4142135), (x, yp, 1.0), (xp, yp, 1.4142135),
            ];
            let (mut higher_inflow, mut lower_outflow) = (0.0_f32, 0.0_f32);
            for &(nx, ny, dist) in &neighbors {
                let dz = dem[ny * width + nx] - z_c;
                if dz > 0.0 { higher_inflow += dz / dist; } else if dz < 0.0 { lower_outflow += (-dz) / dist; }
            }
            let denom = higher_inflow + lower_outflow;
            out_flow_mask[y * width + x] = if denom < 1e-6 { 0.0 } else { (higher_inflow / denom).clamp(0.0, 1.0) };
        }
    }
    Ok(())
}

/// 7. 8-direction horizon-angle sky-view occlusion in `[0, 1]`.
pub fn select_ambient_occlusion(
    dem: &[f32], width: usize, height: usize, cell_size: f32, radius_cells: usize, out_ao: &mut [f32],
) -> Result<(), &'static str> {
    if width == 0 || height == 0 || cell_size <= 0.0 { return Err("invalid grid dimensions or cell size"); }
    let n = width * height;
    if dem.len() != n || out_ao.len() != n { return Err("slice length mismatch"); }
    let r = radius_cells.max(1);
    let dirs: [(isize, isize, f32); 8] = [
        (1, 0, 1.0), (1, 1, 1.4142135), (0, 1, 1.0), (-1, 1, 1.4142135),
        (-1, 0, 1.0), (-1, -1, 1.4142135), (0, -1, 1.0), (1, -1, 1.4142135),
    ];
    let half_pi = std::f32::consts::FRAC_PI_2;
    for y in 0..height {
        for x in 0..width {
            let z0 = dem[y * width + x];
            let mut sum_occ = 0.0_f32;
            for &(dx, dy, step_dist) in &dirs {
                let mut max_angle = 0.0_f32;
                for step in 1..=r {
                    let px = x as isize + dx * step as isize;
                    let py = y as isize + dy * step as isize;
                    if px < 0 || px >= width as isize || py < 0 || py >= height as isize { break; }
                    let z = dem[py as usize * width + px as usize];
                    let dist = step_dist * (step as f32) * cell_size;
                    let angle = ((z - z0) / dist).atan();
                    if angle > max_angle { max_angle = angle; }
                }
                sum_occ += (max_angle / half_pi).clamp(0.0, 1.0);
            }
            out_ao[y * width + x] = (sum_occ / 8.0).clamp(0.0, 1.0);
        }
    }
    Ok(())
}

/// 8 & 9. Physical angle-of-repose rock exposure and bioclimatic treeline mask.
pub fn select_rock_exposure_and_treeline(
    dem: &[f32], width: usize, height: usize, cell_size: f32, latitude_deg: f32, critical_slope_deg: f32,
    basal_treeline_m: f32, out_rock: &mut [f32], out_treeline: &mut [f32],
) -> Result<(), &'static str> {
    if width == 0 || height == 0 || cell_size <= 0.0 { return Err("invalid grid dimensions or cell size"); }
    let n = width * height;
    if dem.len() != n || out_rock.len() != n || out_treeline.len() != n { return Err("slice length mismatch"); }
    let scale = 8.0 * cell_size;
    let lat_factor = latitude_deg.abs().min(90.0) / 90.0;
    for y in 0..height {
        let (ym, yp) = (y.saturating_sub(1), (y + 1).min(height - 1));
        for x in 0..width {
            let (xm, xp) = (x.saturating_sub(1), (x + 1).min(width - 1));
            let z = dem[y * width + x];
            let (dz_dx, dz_dy) = horn_gradient(dem, width, x, y, xm, xp, ym, yp, scale);
            let rise_run = (dz_dx * dz_dx + dz_dy * dz_dy).sqrt();
            let slope_deg = rise_run.atan().to_degrees();
            let rock = selector_band_weight(slope_deg, critical_slope_deg, 90.0, 5.0);

            let aspect_shift = if rise_run > 1e-6 {
                let aspect = (-dz_dx).atan2(dz_dy);
                let hemi = if latitude_deg >= 0.0 { -1.0 } else { 1.0 };
                hemi * aspect.cos() * 150.0
            } else { 0.0 };
            let local_treeline = (basal_treeline_m * (1.0 - 0.5 * lat_factor) + aspect_shift).max(0.0);
            let treeline = selector_band_weight(z, -10000.0, local_treeline, 100.0);

            let idx = y * width + x;
            out_rock[idx] = rock;
            out_treeline[idx] = treeline;
        }
    }
    Ok(())
}

/// 12 layer combine/blend modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombineMode {
    Add, Subtract, Multiply, DivideSafe, Max, Min,
    Blend, Screen, Overlay, HardLight, SoftLight, Difference,
}

/// Combines layer `a` and `b` with optional mask, blend mode, and weight.
pub fn combine_layers(
    a: &[f32], b: &[f32], mask: Option<&[f32]>, mode: CombineMode, weight: f32, out: &mut [f32],
) -> Result<(), &'static str> {
    let n = a.len();
    if b.len() != n || out.len() != n { return Err("slice length mismatch"); }
    if let Some(m) = mask { if m.len() != n { return Err("mask length mismatch"); } }
    for i in 0..n {
        let (va, vb) = (a[i], b[i]);
        let c = match mode {
            CombineMode::Add => va + vb,
            CombineMode::Subtract => va - vb,
            CombineMode::Multiply => va * vb,
            CombineMode::DivideSafe => if vb.abs() < 1e-6 { 0.0 } else { va / vb },
            CombineMode::Max => va.max(vb),
            CombineMode::Min => va.min(vb),
            CombineMode::Blend => vb,
            CombineMode::Screen => va + vb - va * vb,
            CombineMode::Overlay => if va < 0.5 { 2.0 * va * vb } else { 1.0 - 2.0 * (1.0 - va) * (1.0 - vb) },
            CombineMode::HardLight => if vb < 0.5 { 2.0 * va * vb } else { 1.0 - 2.0 * (1.0 - va) * (1.0 - vb) },
            CombineMode::SoftLight => (1.0 - 2.0 * vb) * va * va + 2.0 * vb * va,
            CombineMode::Difference => (va - vb).abs(),
        };
        let m = mask.map_or(1.0, |msk| msk[i].clamp(0.0, 1.0));
        let alpha = (weight * m).clamp(0.0, 1.0);
        out[i] = va * (1.0 - alpha) + c * alpha;
    }
    Ok(())
}

/// Warps a 2D scalar grid along vector displacement fields `(warp_dx, warp_dy)` bilinearly.
pub fn domain_warp_grid(
    src: &[f32], warp_dx: &[f32], warp_dy: &[f32], width: usize, height: usize, strength_cells: f32, out: &mut [f32],
) -> Result<(), &'static str> {
    if width == 0 || height == 0 { return Err("invalid grid dimensions"); }
    let n = width * height;
    if src.len() != n || warp_dx.len() != n || warp_dy.len() != n || out.len() != n { return Err("slice length mismatch"); }
    let (max_x, max_y) = ((width - 1) as f32, (height - 1) as f32);
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let sx = (x as f32 + warp_dx[idx] * strength_cells).clamp(0.0, max_x);
            let sy = (y as f32 + warp_dy[idx] * strength_cells).clamp(0.0, max_y);
            let (x0, y0) = (sx.floor() as usize, sy.floor() as usize);
            let (x1, y1) = ((x0 + 1).min(width - 1), (y0 + 1).min(height - 1));
            let (fx, fy) = (sx - x0 as f32, sy - y0 as f32);

            let top = src[y0 * width + x0] * (1.0 - fx) + src[y0 * width + x1] * fx;
            let bot = src[y1 * width + x0] * (1.0 - fx) + src[y1 * width + x1] * fx;
            out[idx] = top * (1.0 - fy) + bot * fy;
        }
    }
    Ok(())
}

/// EarthCraft 19 post-process modifiers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PostProcessOp {
    Invert, Abs, Clamp { min: f32, max: f32 }, Gain(f32), Bias(f32), Gamma(f32),
    ScaleOffset { scale: f32, offset: f32 }, Quantize(u32), Smoothstep { edge0: f32, edge1: f32 },
    Fold { threshold: f32 }, SinWave { freq: f32 }, Power(f32), ExpNorm, Threshold(f32),
    MirrorCenter(f32), SoftClip(f32), Contrast { factor: f32, midpoint: f32 },
    Remap01 { from_min: f32, from_max: f32 }, Saturate,
}

#[inline]
fn schlick_bias(t: f32, b: f32) -> f32 {
    let b = b.clamp(1e-4, 1.0 - 1e-4);
    t / ((1.0 / b - 2.0) * (1.0 - t) + 1.0)
}

#[inline]
fn apply_op(v: f32, op: PostProcessOp) -> f32 {
    match op {
        PostProcessOp::Invert => 1.0 - v,
        PostProcessOp::Abs => v.abs(),
        PostProcessOp::Clamp { min, max } => v.clamp(min, max),
        PostProcessOp::Gain(g) => {
            let vc = v.clamp(0.0, 1.0);
            if vc < 0.5 { schlick_bias(2.0 * vc, 1.0 - g) * 0.5 } else { 1.0 - schlick_bias(2.0 * (1.0 - vc), 1.0 - g) * 0.5 }
        }
        PostProcessOp::Bias(b) => schlick_bias(v.clamp(0.0, 1.0), b),
        PostProcessOp::Gamma(gamma) => if gamma > 0.0 { v.max(0.0).powf(1.0 / gamma) } else { v },
        PostProcessOp::ScaleOffset { scale, offset } => v * scale + offset,
        PostProcessOp::Quantize(steps) => if steps <= 1 { v.round() } else { (v * steps as f32).round() / steps as f32 },
        PostProcessOp::Smoothstep { edge0, edge1 } => {
            let diff = edge1 - edge0;
            if diff.abs() < 1e-6 { if v < edge0 { 0.0 } else { 1.0 } } else {
                let t = ((v - edge0) / diff).clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t)
            }
        }
        PostProcessOp::Fold { threshold } => if threshold.abs() < 1e-6 { 0.0 } else {
            let cycle = (v / threshold).abs() % 2.0;
            if cycle > 1.0 { (2.0 - cycle) * threshold } else { cycle * threshold }
        },
        PostProcessOp::SinWave { freq } => (v * freq * std::f32::consts::TAU).sin() * 0.5 + 0.5,
        PostProcessOp::Power(p) => if v >= 0.0 { v.powf(p) } else { -((-v).powf(p)) },
        PostProcessOp::ExpNorm => 1.0 - (-v.max(0.0)).exp(),
        PostProcessOp::Threshold(th) => if v >= th { 1.0 } else { 0.0 },
        PostProcessOp::MirrorCenter(center) => center - (v - center).abs(),
        PostProcessOp::SoftClip(knee) => (v / knee.max(1e-4)).tanh() * knee.max(1e-4),
        PostProcessOp::Contrast { factor, midpoint } => (v - midpoint) * factor + midpoint,
        PostProcessOp::Remap01 { from_min, from_max } => {
            let d = from_max - from_min;
            if d.abs() < 1e-6 { 0.0 } else { ((v - from_min) / d).clamp(0.0, 1.0) }
        }
        PostProcessOp::Saturate => v.clamp(0.0, 1.0),
    }
}

/// Applies a sequence of post-processing modifiers in-place.
pub fn apply_post_process_chain(grid: &mut [f32], ops: &[PostProcessOp]) {
    for val in grid.iter_mut() {
        for &op in ops { *val = apply_op(*val, op); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_selectors() {
        let dem = [0.0, 0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 0.0];
        let mut mask = [0.0; 9];
        select_height(&dem, 5.0, 15.0, 2.0, &mut mask).unwrap();
        assert_eq!(mask[4], 1.0);
        assert_eq!(mask[0], 0.0);

        let mut slope_mask = [0.0; 9];
        select_slope_aspect(&dem, 3, 3, 1.0, 10.0, 80.0, None, 45.0, &mut slope_mask).unwrap();
        assert!(slope_mask[1] > 0.0);

        let mut conv = [0.0; 9];
        let mut prot = [0.0; 9];
        select_convexity_protrusion(&dem, 3, 3, 1.0, &mut conv, &mut prot).unwrap();
        assert!(conv[4] > 0.0);
        assert!(prot[4] > 0.0);

        let mut flow = [0.0; 9];
        select_flow_accumulation_proxy(&dem, 3, 3, &mut flow).unwrap();
        assert_eq!(flow[4], 0.0);
        assert!(flow[0] > 0.0);

        let mut ao = [0.0; 9];
        select_ambient_occlusion(&dem, 3, 3, 1.0, 2, &mut ao).unwrap();
        assert_eq!(ao[4], 0.0);

        let mut rock = [0.0; 9];
        let mut tree = [0.0; 9];
        select_rock_exposure_and_treeline(&dem, 3, 3, 1.0, 45.0, 30.0, 2000.0, &mut rock, &mut tree).unwrap();
        assert!(rock[1] > 0.0);
        assert_eq!(tree[0], 1.0);
    }

    #[test]
    fn test_all_combine_modes_and_warp() {
        let a = [0.6, 0.4];
        let b = [0.3, 0.2];
        let mut out = [0.0; 2];
        let modes = [
            CombineMode::Add, CombineMode::Subtract, CombineMode::Multiply,
            CombineMode::DivideSafe, CombineMode::Max, CombineMode::Min,
            CombineMode::Blend, CombineMode::Screen, CombineMode::Overlay,
            CombineMode::HardLight, CombineMode::SoftLight, CombineMode::Difference,
        ];
        for &m in &modes {
            combine_layers(&a, &b, None, m, 1.0, &mut out).unwrap();
            assert!(out[0].is_finite());
        }

        let src = [1.0, 2.0, 3.0, 4.0];
        let dx = [0.0; 4];
        let dy = [0.0; 4];
        let mut warp_out = [0.0; 4];
        domain_warp_grid(&src, &dx, &dy, 2, 2, 1.0, &mut warp_out).unwrap();
        assert_eq!(warp_out, src);
    }

    #[test]
    fn test_all_19_post_process_ops() {
        let ops = [
            PostProcessOp::Invert, PostProcessOp::Abs, PostProcessOp::Clamp { min: 0.0, max: 1.0 },
            PostProcessOp::Gain(0.7), PostProcessOp::Bias(0.3), PostProcessOp::Gamma(2.2),
            PostProcessOp::ScaleOffset { scale: 2.0, offset: 0.5 }, PostProcessOp::Quantize(4),
            PostProcessOp::Smoothstep { edge0: 0.1, edge1: 0.9 }, PostProcessOp::Fold { threshold: 0.8 },
            PostProcessOp::SinWave { freq: 1.0 }, PostProcessOp::Power(1.5), PostProcessOp::ExpNorm,
            PostProcessOp::Threshold(0.5), PostProcessOp::MirrorCenter(0.5), PostProcessOp::SoftClip(0.8),
            PostProcessOp::Contrast { factor: 1.2, midpoint: 0.5 },
            PostProcessOp::Remap01 { from_min: 0.2, from_max: 0.8 }, PostProcessOp::Saturate,
        ];
        assert_eq!(ops.len(), 19);
        let mut grid = [0.5];
        apply_post_process_chain(&mut grid, &ops);
        assert!(grid[0].is_finite());
    }
}
