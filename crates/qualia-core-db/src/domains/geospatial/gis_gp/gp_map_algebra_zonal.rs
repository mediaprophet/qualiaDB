// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) RustGIS contributors (adapted under MIT License)
//! Map Algebra (Tomlin Local/Focal/Zonal/Global), Zonal Statistics, Resampling,
//! Marching-Squares Contours, Polygon Scanline Rasterization, and Saaty AHP.
//!
//! Allocation Class:
//! - Tier-1 (`HotZeroHeap`): `gp_eval_local_map_algebra_into`, `gp_focal_filter_into`,
//!   `gp_reclassify_ranges_into`, `gp_resample_grid_into`, `gp_rasterize_polygon_scanline_into`.
//! - Tier-2 (`ColdBounded`): `gp_zonal_statistics`, `gp_marching_squares_contours`,
//!   `gp_ahp_weights_and_consistency`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpAlgebraError {
    DimensionMismatch, InvalidDimensions, StackOverflow, StackUnderflow,
    BandIndexOutOfBounds(usize), InvalidProgram, InvalidMatrix,
    ByteBudgetExceeded { requested: usize, budget: usize },
}

impl core::fmt::Display for GpAlgebraError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DimensionMismatch => write!(f, "Dimension mismatch in raster or vector inputs"),
            Self::InvalidDimensions => write!(f, "Invalid dimensions (width, height, or N must be > 0)"),
            Self::StackOverflow => write!(f, "Map algebra stack overflow (exceeded 16 items)"),
            Self::StackUnderflow => write!(f, "Map algebra stack underflow"),
            Self::BandIndexOutOfBounds(idx) => write!(f, "Band index {idx} out of bounds"),
            Self::InvalidProgram => write!(f, "Invalid map algebra program: stack must hold 1 result"),
            Self::InvalidMatrix => write!(f, "Invalid matrix: dimensions must be N x N with N > 0"),
            Self::ByteBudgetExceeded { requested, budget } => write!(f, "Byte budget exceeded: requested {requested} B, budget {budget} B"),
        }
    }
}
impl std::error::Error for GpAlgebraError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MapAlgebraOp {
    PushBand(usize), PushConst(f32), Add, Sub, Mul, SafeDiv,
    NormalizedDiff, Min, Max, Sqrt, Abs, Clamp(f32, f32), Gt, Lt, SelectIfElse,
}

pub fn gp_eval_local_map_algebra_into(
    bands: &[&[f32]], program: &[MapAlgebraOp], out: &mut [f32],
) -> Result<(), GpAlgebraError> {
    let n = out.len();
    if n == 0 { return Ok(()); }
    for b in bands { if b.len() != n { return Err(GpAlgebraError::DimensionMismatch); } }
    for op in program {
        if let MapAlgebraOp::PushBand(idx) = op {
            if *idx >= bands.len() { return Err(GpAlgebraError::BandIndexOutOfBounds(*idx)); }
        }
    }
    let mut stack = [0.0f32; 16];
    for i in 0..n {
        let mut sp = 0usize;
        for op in program {
            match *op {
                MapAlgebraOp::PushBand(b) => { if sp >= 16 { return Err(GpAlgebraError::StackOverflow); } stack[sp] = bands[b][i]; sp += 1; }
                MapAlgebraOp::PushConst(c) => { if sp >= 16 { return Err(GpAlgebraError::StackOverflow); } stack[sp] = c; sp += 1; }
                MapAlgebraOp::Add => { if sp < 2 { return Err(GpAlgebraError::StackUnderflow); } sp -= 1; stack[sp - 1] += stack[sp]; }
                MapAlgebraOp::Sub => { if sp < 2 { return Err(GpAlgebraError::StackUnderflow); } sp -= 1; stack[sp - 1] -= stack[sp]; }
                MapAlgebraOp::Mul => { if sp < 2 { return Err(GpAlgebraError::StackUnderflow); } sp -= 1; stack[sp - 1] *= stack[sp]; }
                MapAlgebraOp::SafeDiv => { if sp < 2 { return Err(GpAlgebraError::StackUnderflow); } sp -= 1; let d = stack[sp]; stack[sp - 1] = if d.abs() < 1e-12 { 0.0 } else { stack[sp - 1] / d }; }
                MapAlgebraOp::NormalizedDiff => { if sp < 2 { return Err(GpAlgebraError::StackUnderflow); } sp -= 1; let (a, b) = (stack[sp - 1], stack[sp]); let den = a + b; stack[sp - 1] = if den.abs() < 1e-12 { 0.0 } else { (a - b) / den }; }
                MapAlgebraOp::Min => { if sp < 2 { return Err(GpAlgebraError::StackUnderflow); } sp -= 1; stack[sp - 1] = stack[sp - 1].min(stack[sp]); }
                MapAlgebraOp::Max => { if sp < 2 { return Err(GpAlgebraError::StackUnderflow); } sp -= 1; stack[sp - 1] = stack[sp - 1].max(stack[sp]); }
                MapAlgebraOp::Sqrt => { if sp < 1 { return Err(GpAlgebraError::StackUnderflow); } stack[sp - 1] = stack[sp - 1].max(0.0).sqrt(); }
                MapAlgebraOp::Abs => { if sp < 1 { return Err(GpAlgebraError::StackUnderflow); } stack[sp - 1] = stack[sp - 1].abs(); }
                MapAlgebraOp::Clamp(lo, hi) => { if sp < 1 { return Err(GpAlgebraError::StackUnderflow); } stack[sp - 1] = stack[sp - 1].clamp(lo, hi); }
                MapAlgebraOp::Gt => { if sp < 2 { return Err(GpAlgebraError::StackUnderflow); } sp -= 1; stack[sp - 1] = if stack[sp - 1] > stack[sp] { 1.0 } else { 0.0 }; }
                MapAlgebraOp::Lt => { if sp < 2 { return Err(GpAlgebraError::StackUnderflow); } sp -= 1; stack[sp - 1] = if stack[sp - 1] < stack[sp] { 1.0 } else { 0.0 }; }
                MapAlgebraOp::SelectIfElse => { if sp < 3 { return Err(GpAlgebraError::StackUnderflow); } let (c, t, f) = (stack[sp - 3], stack[sp - 2], stack[sp - 1]); sp -= 2; stack[sp - 1] = if c.abs() > 1e-6 { t } else { f }; }
            }
        }
        if sp != 1 { return Err(GpAlgebraError::InvalidProgram); }
        out[i] = stack[0];
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocalStatKind { Mean, Min, Max, Range, StdDev, Majority, SobelMagnitude, Gaussian3x3 }

pub fn gp_focal_filter_into(
    grid: &[f32], width: usize, height: usize, radius: usize, kind: FocalStatKind, out: &mut [f32],
) -> Result<(), GpAlgebraError> {
    if width == 0 || height == 0 || grid.len() != width * height || out.len() != width * height {
        return Err(GpAlgebraError::DimensionMismatch);
    }
    let r = radius.max(1);
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            match kind {
                FocalStatKind::SobelMagnitude => {
                    let (mut gx, mut gy) = (0.0f32, 0.0f32);
                    for dy in -1i32..=1 {
                        let py = (y as i32 + dy).clamp(0, height as i32 - 1) as usize;
                        let (wy, wyc) = (if dy == 0 { 0.0 } else { dy as f32 }, if dy == 0 { 2.0 } else { 1.0 });
                        for dx in -1i32..=1 {
                            let px = (x as i32 + dx).clamp(0, width as i32 - 1) as usize;
                            let (wx, wxc) = (if dx == 0 { 0.0 } else { dx as f32 }, if dx == 0 { 2.0 } else { 1.0 });
                            gx += grid[py * width + px] * wx * wyc; gy += grid[py * width + px] * wy * wxc;
                        }
                    }
                    out[idx] = (gx * gx + gy * gy).sqrt();
                }
                FocalStatKind::Gaussian3x3 => {
                    let (mut sum, gw) = (0.0f32, [1.0f32, 2.0, 1.0]);
                    for dy in -1i32..=1 {
                        let py = (y as i32 + dy).clamp(0, height as i32 - 1) as usize;
                        for dx in -1i32..=1 {
                            let px = (x as i32 + dx).clamp(0, width as i32 - 1) as usize;
                            sum += grid[py * width + px] * gw[(dy + 1) as usize] * gw[(dx + 1) as usize];
                        }
                    }
                    out[idx] = sum / 16.0;
                }
                _ => {
                    let (y0, y1, x0, x1) = (y.saturating_sub(r), (y + r).min(height - 1), x.saturating_sub(r), (x + r).min(width - 1));
                    let (mut min_v, mut max_v, mut sum, mut cnt) = (f32::INFINITY, f32::NEG_INFINITY, 0.0f64, 0usize);
                    for ny in y0..=y1 { for nx in x0..=x1 { let v = grid[ny * width + nx]; min_v = min_v.min(v); max_v = max_v.max(v); sum += v as f64; cnt += 1; } }
                    out[idx] = match kind {
                        FocalStatKind::Min => min_v, FocalStatKind::Max => max_v, FocalStatKind::Range => max_v - min_v,
                        FocalStatKind::Mean => (sum / cnt as f64) as f32,
                        FocalStatKind::StdDev => {
                            let (mean, mut sq) = (sum / cnt as f64, 0.0f64);
                            for ny in y0..=y1 { for nx in x0..=x1 { let d = grid[ny * width + nx] as f64 - mean; sq += d * d; } }
                            (sq / cnt as f64).sqrt() as f32
                        }
                        FocalStatKind::Majority => {
                            let (mut best_v, mut max_c) = (grid[idx], 0usize);
                            for ny in y0..=y1 { for nx in x0..=x1 {
                                let (cand, mut c) = (grid[ny * width + nx], 0usize);
                                for cy in y0..=y1 { for cx in x0..=x1 { if (grid[cy * width + cx] - cand).abs() < 1e-5 { c += 1; } } }
                                if c > max_c { max_c = c; best_v = cand; }
                            }}
                            best_v
                        }
                        _ => unreachable!(),
                    };
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZonalRecord {
    pub zone_id: u32, pub count: usize, pub min: f64, pub max: f64,
    pub sum: f64, pub mean: f64, pub std_dev: f64, pub median: f64,
}

pub fn gp_zonal_statistics(
    zones: &[u32], values: &[f32], byte_budget: usize,
) -> Result<Vec<ZonalRecord>, GpAlgebraError> {
    if zones.len() != values.len() { return Err(GpAlgebraError::DimensionMismatch); }
    if zones.is_empty() { return Ok(Vec::new()); }
    let req = zones.len().saturating_mul(core::mem::size_of::<(u32, f32)>()).saturating_add(64);
    if req > byte_budget { return Err(GpAlgebraError::ByteBudgetExceeded { requested: req, budget: byte_budget }); }
    let mut pairs: Vec<(u32, f32)> = Vec::with_capacity(zones.len());
    for i in 0..zones.len() { pairs.push((zones[i], values[i])); }
    pairs.sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)));

    let (mut records, mut i) = (Vec::new(), 0);
    while i < pairs.len() {
        let (zone, start) = (pairs[i].0, i);
        while i < pairs.len() && pairs[i].0 == zone { i += 1; }
        let slice = &pairs[start..i];
        let (cnt, min, max) = (slice.len(), slice[0].1 as f64, slice[slice.len() - 1].1 as f64);
        let sum: f64 = slice.iter().map(|p| p.1 as f64).sum();
        let mean = sum / cnt as f64;
        let std_dev = if cnt <= 1 { 0.0 } else { (slice.iter().map(|p| (p.1 as f64 - mean).powi(2)).sum::<f64>() / cnt as f64).sqrt() };
        let median = if cnt % 2 == 1 { slice[cnt / 2].1 as f64 } else { (slice[cnt / 2 - 1].1 as f64 + slice[cnt / 2].1 as f64) * 0.5 };
        let total_alloc = req.saturating_add((records.len() + 1) * core::mem::size_of::<ZonalRecord>());
        if total_alloc > byte_budget { return Err(GpAlgebraError::ByteBudgetExceeded { requested: total_alloc, budget: byte_budget }); }
        records.push(ZonalRecord { zone_id: zone, count: cnt, min, max, sum, mean, std_dev, median });
    }
    Ok(records)
}

pub fn gp_reclassify_ranges_into(
    grid: &[f32], intervals: &[(f32, f32, f32)], default_val: f32, out: &mut [f32],
) -> Result<(), GpAlgebraError> {
    if grid.len() != out.len() { return Err(GpAlgebraError::DimensionMismatch); }
    for (i, &val) in grid.iter().enumerate() {
        let mut mapped = default_val;
        for &(lo, hi, target) in intervals {
            if val >= lo && val <= hi { mapped = target; break; }
        }
        out[i] = mapped;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResampleKernel { Nearest, Bilinear, BicubicCatmullRom, Lanczos3 }

fn catmull_rom_weight(x: f64) -> f64 {
    let t = x.abs();
    if t <= 1.0 { 1.5 * t * t * t - 2.5 * t * t + 1.0 } else if t < 2.0 { -0.5 * t * t * t + 2.5 * t * t - 4.0 * t + 2.0 } else { 0.0 }
}

fn lanczos3_weight(x: f64) -> f64 {
    let ax = x.abs();
    if ax < 1e-9 { 1.0 } else if ax >= 3.0 { 0.0 } else {
        let p = std::f64::consts::PI * ax; (p.sin() / p) * ((p / 3.0).sin() / (p / 3.0))
    }
}

pub fn gp_resample_grid_into(
    src: &[f32], src_w: usize, src_h: usize, dst_w: usize, dst_h: usize, kernel: ResampleKernel, out: &mut [f32],
) -> Result<(), GpAlgebraError> {
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 { return Err(GpAlgebraError::InvalidDimensions); }
    if src.len() != src_w * src_h || out.len() != dst_w * dst_h { return Err(GpAlgebraError::DimensionMismatch); }
    let (sx_scale, sy_scale) = (src_w as f64 / dst_w as f64, src_h as f64 / dst_h as f64);
    for dy in 0..dst_h {
        let sy = (dy as f64 + 0.5) * sy_scale - 0.5;
        for dx in 0..dst_w {
            let sx = (dx as f64 + 0.5) * sx_scale - 0.5;
            let out_idx = dy * dst_w + dx;
            match kernel {
                ResampleKernel::Nearest => {
                    let ix = (sx.round() as isize).clamp(0, src_w as isize - 1) as usize;
                    let iy = (sy.round() as isize).clamp(0, src_h as isize - 1) as usize;
                    out[out_idx] = src[iy * src_w + ix];
                }
                ResampleKernel::Bilinear => {
                    let (x0, y0) = ((sx.floor() as isize).clamp(0, src_w as isize - 1) as usize, (sy.floor() as isize).clamp(0, src_h as isize - 1) as usize);
                    let (x1, y1) = ((x0 + 1).min(src_w - 1), (y0 + 1).min(src_h - 1));
                    let (fx, fy) = ((sx - x0 as f64).clamp(0.0, 1.0) as f32, (sy - y0 as f64).clamp(0.0, 1.0) as f32);
                    out[out_idx] = (1.0 - fx) * (1.0 - fy) * src[y0 * src_w + x0] + fx * (1.0 - fy) * src[y0 * src_w + x1]
                        + (1.0 - fx) * fy * src[y1 * src_w + x0] + fx * fy * src[y1 * src_w + x1];
                }
                ResampleKernel::BicubicCatmullRom | ResampleKernel::Lanczos3 => {
                    let rad = if kernel == ResampleKernel::BicubicCatmullRom { 2isize } else { 3isize };
                    let w_fn = if kernel == ResampleKernel::BicubicCatmullRom { catmull_rom_weight } else { lanczos3_weight };
                    let (mut sum, mut w_sum, xb, yb) = (0.0f64, 0.0f64, sx.floor() as isize, sy.floor() as isize);
                    for iy in (yb - rad + 1)..=(yb + rad) {
                        let (cy, wy) = (iy.clamp(0, src_h as isize - 1) as usize, w_fn(sy - iy as f64));
                        for ix in (xb - rad + 1)..=(xb + rad) {
                            let (cx, w) = (ix.clamp(0, src_w as isize - 1) as usize, wy * w_fn(sx - ix as f64));
                            sum += src[cy * src_w + cx] as f64 * w; w_sum += w;
                        }
                    }
                    out[out_idx] = if w_sum.abs() < 1e-12 { 0.0 } else { (sum / w_sum) as f32 };
                }
            }
        }
    }
    Ok(())
}

pub fn gp_marching_squares_contours(
    grid: &[f32], width: usize, height: usize, bbox: [f64; 4], isovalues: &[f32], byte_budget: usize,
) -> Result<Vec<(f32, Vec<[[f64; 2]; 2]>)>, GpAlgebraError> {
    if width < 2 || height < 2 || grid.len() != width * height { return Err(GpAlgebraError::DimensionMismatch); }
    let (dx, dy) = ((bbox[2] - bbox[0]) / (width - 1) as f64, (bbox[3] - bbox[1]) / (height - 1) as f64);
    let (seg_size, entry_size) = (core::mem::size_of::<[[f64; 2]; 2]>(), core::mem::size_of::<(f32, Vec<[[f64; 2]; 2]>) >());
    let mut total_bytes = isovalues.len().saturating_mul(entry_size);
    if total_bytes > byte_budget { return Err(GpAlgebraError::ByteBudgetExceeded { requested: total_bytes, budget: byte_budget }); }

    let mut result = Vec::with_capacity(isovalues.len());
    for &iso in isovalues {
        let mut segments = Vec::new();
        for r in 0..(height - 1) {
            let (y0, y1) = (bbox[1] + r as f64 * dy, bbox[1] + (r + 1) as f64 * dy);
            for c in 0..(width - 1) {
                let (x0, x1) = (bbox[0] + c as f64 * dx, bbox[0] + (c + 1) as f64 * dx);
                let (v0, v1) = (grid[r * width + c], grid[r * width + c + 1]);
                let (v2, v3) = (grid[(r + 1) * width + c + 1], grid[(r + 1) * width + c]);
                let case_idx = (if v0 >= iso { 1 } else { 0 }) | (if v1 >= iso { 2 } else { 0 })
                    | (if v2 >= iso { 4 } else { 0 }) | (if v3 >= iso { 8 } else { 0 });
                if case_idx == 0 || case_idx == 15 { continue; }

                let interp = |va: f32, vb: f32| -> f64 {
                    let d = vb as f64 - va as f64;
                    if d.abs() < 1e-12 { 0.5 } else { ((iso as f64 - va as f64) / d).clamp(0.0, 1.0) }
                };
                let (pt0, pt1) = ([x0 + interp(v0, v1) * dx, y0], [x1, y0 + interp(v1, v2) * dy]);
                let (pt2, pt3) = ([x0 + interp(v3, v2) * dx, y1], [x0, y0 + interp(v0, v3) * dy]);
                let mut push_seg = |pa: [f64; 2], pb: [f64; 2]| -> Result<(), GpAlgebraError> {
                    total_bytes = total_bytes.saturating_add(seg_size);
                    if total_bytes > byte_budget { return Err(GpAlgebraError::ByteBudgetExceeded { requested: total_bytes, budget: byte_budget }); }
                    segments.push([pa, pb]); Ok(())
                };
                let vc = (v0 + v1 + v2 + v3) * 0.25;
                match case_idx {
                    1 | 14 => push_seg(pt3, pt0)?, 2 | 13 => push_seg(pt0, pt1)?,
                    3 | 12 => push_seg(pt3, pt1)?, 4 | 11 => push_seg(pt1, pt2)?,
                    6 | 9  => push_seg(pt0, pt2)?, 7 | 8  => push_seg(pt2, pt3)?,
                    5 => { if vc >= iso { push_seg(pt3, pt0)?; push_seg(pt1, pt2)?; } else { push_seg(pt3, pt2)?; push_seg(pt0, pt1)?; } }
                    10 => { if vc >= iso { push_seg(pt0, pt1)?; push_seg(pt2, pt3)?; } else { push_seg(pt3, pt0)?; push_seg(pt1, pt2)?; } }
                    _ => {}
                }
            }
        }
        result.push((iso, segments));
    }
    Ok(result)
}

pub fn gp_rasterize_polygon_scanline_into(
    ring: &[[f64; 2]], bbox: [f64; 4], width: usize, height: usize, burn_value: f32, out_grid: &mut [f32],
) -> Result<usize, GpAlgebraError> {
    if width == 0 || height == 0 || out_grid.len() != width * height { return Err(GpAlgebraError::DimensionMismatch); }
    if ring.len() < 3 { return Ok(0); }
    let (dx, dy) = ((bbox[2] - bbox[0]) / width as f64, (bbox[3] - bbox[1]) / height as f64);
    let (mut p_min_x, mut p_max_x, mut p_min_y, mut p_max_y) = (ring[0][0], ring[0][0], ring[0][1], ring[0][1]);
    for pt in ring.iter().skip(1) {
        p_min_x = p_min_x.min(pt[0]); p_max_x = p_max_x.max(pt[0]); p_min_y = p_min_y.min(pt[1]); p_max_y = p_max_y.max(pt[1]);
    }
    let (n, mut burned) = (ring.len(), 0usize);
    for r in 0..height {
        let py = bbox[1] + (r as f64 + 0.5) * dy;
        if py < p_min_y || py > p_max_y { continue; }
        for c in 0..width {
            let px = bbox[0] + (c as f64 + 0.5) * dx;
            if px < p_min_x || px > p_max_x { continue; }
            let (mut inside, mut j) = (false, n - 1);
            for i in 0..n {
                let ((xi, yi), (xj, yj)) = ((ring[i][0], ring[i][1]), (ring[j][0], ring[j][1]));
                if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) { inside = !inside; }
                j = i;
            }
            if inside { out_grid[r * width + c] = burn_value; burned += 1; }
        }
    }
    Ok(burned)
}

#[derive(Debug, Clone, PartialEq)]
pub struct AhpResult {
    pub weights: Vec<f64>, pub lambda_max: f64, pub consistency_index: f64,
    pub consistency_ratio: f64, pub is_consistent: bool,
}

pub fn gp_ahp_weights_and_consistency(
    pairwise_matrix: &[f64], n: usize, byte_budget: usize,
) -> Result<AhpResult, GpAlgebraError> {
    if n == 0 || pairwise_matrix.len() != n * n { return Err(GpAlgebraError::InvalidMatrix); }
    let req = n.saturating_mul(core::mem::size_of::<f64>() * 2).saturating_add(core::mem::size_of::<AhpResult>());
    if req > byte_budget { return Err(GpAlgebraError::ByteBudgetExceeded { requested: req, budget: byte_budget }); }
    if n == 1 {
        return Ok(AhpResult { weights: vec![1.0], lambda_max: 1.0, consistency_index: 0.0, consistency_ratio: 0.0, is_consistent: true });
    }
    let (mut w, mut next_w) = (vec![1.0 / n as f64; n], vec![0.0f64; n]);
    for _ in 0..100 {
        let mut sum = 0.0f64;
        for i in 0..n {
            let dot: f64 = (0..n).map(|j| pairwise_matrix[i * n + j] * w[j]).sum();
            next_w[i] = dot; sum += dot;
        }
        if sum.abs() > 1e-15 { for v in next_w.iter_mut() { *v /= sum; } }
        let diff = (0..n).map(|i| (next_w[i] - w[i]).abs()).fold(0.0f64, f64::max);
        w.copy_from_slice(&next_w);
        if diff < 1e-10 { break; }
    }
    let mut lambda_sum = 0.0f64;
    for i in 0..n {
        let aw_i: f64 = (0..n).map(|j| pairwise_matrix[i * n + j] * w[j]).sum();
        lambda_sum += if w[i].abs() > 1e-15 { aw_i / w[i] } else { n as f64 };
    }
    let lambda_max = lambda_sum / n as f64;
    let ci = if n <= 2 { 0.0 } else { ((lambda_max - n as f64) / (n - 1) as f64).max(0.0) };
    const SAATY_RI: [f64; 16] = [0.0, 0.0, 0.0, 0.58, 0.90, 1.12, 1.24, 1.32, 1.41, 1.45, 1.49, 1.51, 1.54, 1.56, 1.57, 1.59];
    let ri = if n < SAATY_RI.len() { SAATY_RI[n] } else { 1.60 };
    let cr = if ri == 0.0 { 0.0 } else { ci / ri };
    Ok(AhpResult { weights: w, lambda_max, consistency_index: ci, consistency_ratio: cr, is_consistent: cr < 0.10 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_algebra_ndvi_and_select() {
        let (nir, red) = ([0.8f32, 0.2, 0.5], [0.2f32, 0.2, 0.5]);
        let mut out = [0.0f32; 3];
        let prog = [MapAlgebraOp::PushBand(0), MapAlgebraOp::PushBand(1), MapAlgebraOp::NormalizedDiff];
        gp_eval_local_map_algebra_into(&[&nir, &red], &prog, &mut out).unwrap();
        assert!((out[0] - 0.6).abs() < 1e-5 && (out[1] - 0.0).abs() < 1e-5);

        let sel = [
            MapAlgebraOp::PushBand(0), MapAlgebraOp::PushConst(0.5), MapAlgebraOp::Gt,
            MapAlgebraOp::PushConst(100.0), MapAlgebraOp::PushConst(10.0), MapAlgebraOp::SelectIfElse,
        ];
        gp_eval_local_map_algebra_into(&[&nir], &sel, &mut out).unwrap();
        assert_eq!((out[0], out[1]), (100.0, 10.0));
    }

    #[test]
    fn test_focal_statistics() {
        let grid = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let mut out = [0.0f32; 9];
        gp_focal_filter_into(&grid, 3, 3, 1, FocalStatKind::Mean, &mut out).unwrap();
        assert_eq!(out[4], 5.0);
        gp_focal_filter_into(&grid, 3, 3, 1, FocalStatKind::Min, &mut out).unwrap();
        assert_eq!(out[4], 1.0);
        gp_focal_filter_into(&grid, 3, 3, 1, FocalStatKind::Max, &mut out).unwrap();
        assert_eq!(out[4], 9.0);
        gp_focal_filter_into(&grid, 3, 3, 1, FocalStatKind::Gaussian3x3, &mut out).unwrap();
        assert!((out[4] - 5.0).abs() < 1e-5);
    }

    #[test]
    fn test_reclassify_and_resampling() {
        let grid = [5.0f32, 15.0, 25.0];
        let mut out = [0.0f32; 3];
        gp_reclassify_ranges_into(&grid, &[(0.0, 10.0, 1.0), (10.0, 20.0, 2.0)], 99.0, &mut out).unwrap();
        assert_eq!(out, [1.0, 2.0, 99.0]);

        let (src, mut dst) = ([0.0f32, 10.0, 10.0, 20.0], [0.0f32; 4]);
        for k in [ResampleKernel::Nearest, ResampleKernel::Bilinear, ResampleKernel::BicubicCatmullRom, ResampleKernel::Lanczos3] {
            gp_resample_grid_into(&src, 2, 2, 2, 2, k, &mut dst).unwrap();
            assert!((dst[0] - 0.0).abs() < 1.0 && (dst[3] - 20.0).abs() < 1.0);
        }
    }

    #[test]
    fn test_zonal_statistics() {
        let (zones, vals) = ([1u32, 1, 1, 2, 2, 2], [10.0f32, 20.0, 30.0, 5.0, 15.0, 25.0]);
        let recs = gp_zonal_statistics(&zones, &vals, 1024).unwrap();
        assert_eq!(recs.len(), 2);
        assert_eq!((recs[0].zone_id, recs[0].count, recs[0].min, recs[0].max, recs[0].mean, recs[0].median), (1, 3, 10.0, 30.0, 20.0, 20.0));
        assert!((recs[0].std_dev - 8.1649).abs() < 1e-3);
    }

    #[test]
    fn test_marching_squares_and_scanline() {
        let contours = gp_marching_squares_contours(&[0.0f32, 10.0, 10.0, 0.0], 2, 2, [0.0, 0.0, 10.0, 10.0], &[5.0], 4096).unwrap();
        assert_eq!(contours.len(), 1);
        assert!(!contours[0].1.is_empty());

        let ring = [[2.0, 2.0], [8.0, 2.0], [8.0, 8.0], [2.0, 8.0]];
        let mut r_grid = [0.0f32; 100];
        let burned = gp_rasterize_polygon_scanline_into(&ring, [0.0, 0.0, 10.0, 10.0], 10, 10, 1.0, &mut r_grid).unwrap();
        assert!(burned > 0);
    }

    #[test]
    fn test_saaty_ahp_and_budget() {
        let m = [1.0, 2.0, 4.0, 0.5, 1.0, 2.0, 0.25, 0.5, 1.0];
        let res = gp_ahp_weights_and_consistency(&m, 3, 4096).unwrap();
        assert!(res.is_consistent && res.consistency_ratio < 0.10 && res.weights[0] > res.weights[1]);

        assert!(matches!(gp_zonal_statistics(&[1, 2], &[1.0, 2.0], 10), Err(GpAlgebraError::ByteBudgetExceeded { .. })));
        assert!(matches!(gp_marching_squares_contours(&[0.0, 1.0, 1.0, 0.0], 2, 2, [0.0, 0.0, 1.0, 1.0], &[0.5], 10), Err(GpAlgebraError::ByteBudgetExceeded { .. })));
        assert!(matches!(gp_ahp_weights_and_consistency(&m, 3, 10), Err(GpAlgebraError::ByteBudgetExceeded { .. })));
    }
}
