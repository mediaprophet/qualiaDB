// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) RustGIS contributors (adapted under MIT License)
//! Viewshed, Line-of-Sight, Earth-Curvature & Atmospheric-Refraction Visibility,
//! Horn Slope/Aspect/Curvature, Topographic Metrics (TPI/TRI/VRM/Roughness),
//! Directional Wind Exposition, and 10-Class Geomorphons Landform Classification.
//!
//! Allocation Class:
//! - Tier-1 (`HotZeroHeap`): `gp_line_of_sight_profile`, `gp_horn_slope_aspect_curvature_into`,
//!   `gp_tpi_tri_vrm_roughness_into`, `gp_wind_exposition_index_into`, `gp_classify_geomorphons_into`.
//! - Tier-2 (`ColdBounded`): `gp_viewshed_radial_sweep`, `gp_cumulative_viewshed`
//!   (enforces caller-supplied `byte_budget`).

pub const EARTH_RADIUS_M: f32 = 6_371_000.0;
const GEOMORPHON_DIRS: [(isize, isize); 8] = [
    (1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1), (0, 1), (1, 1),
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObserverSpec {
    pub col: usize, pub row: usize,
    pub observer_offset_m: f32, pub target_offset_m: f32,
    pub max_radius_m: f32, pub earth_curvature_refraction_k: f32,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GeomorphonClass {
    Flat = 1, Peak = 2, Ridge = 3, Shoulder = 4, Spur = 5,
    Slope = 6, Hollow = 7, Footslope = 8, Valley = 9, Pit = 10,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpMorphError {
    InvalidDimensions, DimensionMismatch, IndexOutOfBounds, InvalidParameters,
    ByteBudgetExceeded { requested: usize, budget: usize },
}

impl core::fmt::Display for GpMorphError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidDimensions => write!(f, "Invalid raster dimensions or cell size"),
            Self::DimensionMismatch => write!(f, "DEM slice length does not match width * height"),
            Self::IndexOutOfBounds => write!(f, "Observer or target coordinate is out of bounds"),
            Self::InvalidParameters => write!(f, "Invalid search steps or parameters"),
            Self::ByteBudgetExceeded { requested, budget } => {
                write!(f, "Budget exceeded: req {requested} B, budget {budget} B")
            }
        }
    }
}
impl std::error::Error for GpMorphError {}

#[inline]
fn validate_dem(dem: &[f32], width: usize, height: usize, cell_size_m: f32) -> Result<(), GpMorphError> {
    if width == 0 || height == 0 || cell_size_m <= 0.0 || !cell_size_m.is_finite() {
        return Err(GpMorphError::InvalidDimensions);
    }
    let expected = width.checked_mul(height).ok_or(GpMorphError::InvalidDimensions)?;
    if dem.len() != expected {
        return Err(GpMorphError::DimensionMismatch);
    }
    Ok(())
}

pub fn classify_geomorphon_pair(h: usize, l: usize) -> GeomorphonClass {
    match (h, l) {
        (0, 0) => GeomorphonClass::Flat, (0, 8) => GeomorphonClass::Peak, (8, 0) => GeomorphonClass::Pit,
        (0, l) if l >= 5 => GeomorphonClass::Ridge, (0, _) => GeomorphonClass::Shoulder,
        (h, 0) if h >= 5 => GeomorphonClass::Valley, (_, 0) => GeomorphonClass::Footslope,
        (1, l) if l >= 5 => GeomorphonClass::Ridge, (1, l) if l >= 3 => GeomorphonClass::Shoulder,
        (1, 2) => GeomorphonClass::Spur, (1, 1) => GeomorphonClass::Slope,
        (h, 1) if h >= 5 => GeomorphonClass::Valley, (h, 1) if h >= 3 => GeomorphonClass::Footslope,
        (2, 1) => GeomorphonClass::Hollow, (2, l) if l >= 3 => GeomorphonClass::Spur, (2, 2) => GeomorphonClass::Slope,
        (h, 2) if h >= 3 => GeomorphonClass::Hollow, (3, 3) => GeomorphonClass::Slope, (3, 4) => GeomorphonClass::Spur,
        (4, 3) => GeomorphonClass::Hollow, (4, 4) => GeomorphonClass::Slope,
        (h, l) if h > l => GeomorphonClass::Hollow, (h, l) if l > h => GeomorphonClass::Spur,
        _ => GeomorphonClass::Slope,
    }
}

/// Traces Bresenham line-of-sight ray with Earth curvature & atmospheric refraction.
///
/// Allocation Class: Tier-1 (`HotZeroHeap`).
pub fn gp_line_of_sight_profile(
    dem: &[f32], width: usize, height: usize, cell_size_m: f32,
    obs: &ObserverSpec, target_col: usize, target_row: usize,
) -> Result<(bool, f32), GpMorphError> {
    validate_dem(dem, width, height, cell_size_m)?;
    if obs.col >= width || obs.row >= height || target_col >= width || target_row >= height {
        return Err(GpMorphError::IndexOutOfBounds);
    }
    let d_tgt = ((target_col as f32 - obs.col as f32).hypot(target_row as f32 - obs.row as f32)) * cell_size_m;
    if obs.col == target_col && obs.row == target_row { return Ok((true, 0.0)); }
    if d_tgt > obs.max_radius_m { return Ok((false, 0.0)); }

    let drop_factor = (1.0 - obs.earth_curvature_refraction_k) / (2.0 * EARTH_RADIUS_M);
    let z_eye = dem[obs.row * width + obs.col] + obs.observer_offset_m;
    let z_tgt = dem[target_row * width + target_col] + obs.target_offset_m - drop_factor * d_tgt * d_tgt;
    let theta_tgt = ((z_tgt - z_eye) / d_tgt).atan();

    let (mut c0, mut r0, c1, r1) = (obs.col as isize, obs.row as isize, target_col as isize, target_row as isize);
    let (dc, dr) = ((c1 - c0).abs(), -(r1 - r0).abs());
    let (sc, sr) = (if c0 < c1 { 1 } else { -1 }, if r0 < r1 { 1 } else { -1 });
    let (mut err, mut min_clearance, mut is_visible) = (dc + dr, f32::INFINITY, true);

    loop {
        if c0 == c1 && r0 == r1 { break; }
        if !(c0 == obs.col as isize && r0 == obs.row as isize) {
            let d_cell = ((c0 - obs.col as isize) as f32).hypot((r0 - obs.row as isize) as f32) * cell_size_m;
            let z_ground = dem[r0 as usize * width + c0 as usize] - drop_factor * d_cell * d_cell;
            let clearance = theta_tgt - ((z_ground - z_eye) / d_cell).atan();
            if clearance < min_clearance { min_clearance = clearance; }
            if clearance < 0.0 { is_visible = false; }
        }
        let e2 = 2 * err;
        if e2 >= dr { err += dr; c0 += sc; }
        if e2 <= dc { err += dc; r0 += sr; }
    }
    Ok((is_visible, if min_clearance == f32::INFINITY { 0.0 } else { min_clearance }))
}

/// Computes binary visibility grid using radial horizon elevation angle sweep.
///
/// Allocation Class: Tier-2 (`ColdBounded`).
pub fn gp_viewshed_radial_sweep(
    dem: &[f32], width: usize, height: usize, cell_size_m: f32,
    obs: &ObserverSpec, byte_budget: usize,
) -> Result<Vec<u8>, GpMorphError> {
    validate_dem(dem, width, height, cell_size_m)?;
    if obs.col >= width || obs.row >= height { return Err(GpMorphError::IndexOutOfBounds); }
    let needed = width * height;
    if needed > byte_budget {
        return Err(GpMorphError::ByteBudgetExceeded { requested: needed, budget: byte_budget });
    }
    let mut vis = vec![0u8; needed];
    vis[obs.row * width + obs.col] = 1;

    let drop_factor = (1.0 - obs.earth_curvature_refraction_k) / (2.0 * EARTH_RADIUS_M);
    let z_eye = dem[obs.row * width + obs.col] + obs.observer_offset_m;
    let r_cells = (obs.max_radius_m / cell_size_m).ceil() as isize;
    let num_rays = ((2.0 * std::f32::consts::PI * r_cells as f32 * 2.0).ceil() as usize).max(360);
    let step_cell = 0.5f32;
    let max_steps = ((r_cells as f32 / step_cell).ceil() as usize) + 2;

    for i in 0..num_rays {
        let phi = 2.0 * std::f32::consts::PI * (i as f32) / (num_rays as f32);
        let (sin_p, cos_p, mut max_theta) = (phi.sin(), phi.cos(), -f32::INFINITY);
        for s in 1..=max_steps {
            let cf = obs.col as f32 + (s as f32 * step_cell) * cos_p;
            let rf = obs.row as f32 + (s as f32 * step_cell) * sin_p;
            let (ci, ri) = (cf.round() as isize, rf.round() as isize);
            if ci < 0 || ci >= width as isize || ri < 0 || ri >= height as isize { continue; }
            let (c, r) = (ci as usize, ri as usize);
            let d_m = ((ci - obs.col as isize) as f32).hypot((ri - obs.row as isize) as f32) * cell_size_m;
            if d_m <= 0.0 || d_m > obs.max_radius_m { continue; }
            let drop = -drop_factor * d_m * d_m;
            let theta_tgt = ((dem[r * width + c] + obs.target_offset_m + drop - z_eye) / d_m).atan();
            if theta_tgt >= max_theta { vis[r * width + c] = 1; }
            let theta_obst = ((dem[r * width + c] + drop - z_eye) / d_m).atan();
            if theta_obst > max_theta { max_theta = theta_obst; }
        }
    }
    Ok(vis)
}

/// Accumulates visibility counts across multiple observers.
///
/// Allocation Class: Tier-2 (`ColdBounded`).
pub fn gp_cumulative_viewshed(
    dem: &[f32], width: usize, height: usize, cell_size_m: f32,
    observers: &[ObserverSpec], byte_budget: usize,
) -> Result<Vec<u16>, GpMorphError> {
    validate_dem(dem, width, height, cell_size_m)?;
    let needed = width * height * 2 + width * height;
    if needed > byte_budget {
        return Err(GpMorphError::ByteBudgetExceeded { requested: needed, budget: byte_budget });
    }
    let mut counts = vec![0u16; width * height];
    let sub_budget = byte_budget - (width * height * 2);

    for obs in observers {
        let vis = gp_viewshed_radial_sweep(dem, width, height, cell_size_m, obs, sub_budget)?;
        for (cnt, &v) in counts.iter_mut().zip(vis.iter()) {
            if v != 0 { *cnt = cnt.saturating_add(1); }
        }
    }
    Ok(counts)
}

/// Computes Horn 3x3 finite-difference slope, aspect, planform curvature, and profile curvature.
///
/// Allocation Class: Tier-1 (`HotZeroHeap`).
pub fn gp_horn_slope_aspect_curvature_into(
    dem: &[f32], width: usize, height: usize, cell_size_m: f32,
    out_slope_rad: &mut [f32], out_aspect_rad: &mut [f32],
    out_plan_curv: &mut [f32], out_profile_curv: &mut [f32],
) -> Result<(), GpMorphError> {
    validate_dem(dem, width, height, cell_size_m)?;
    let n = width * height;
    if out_slope_rad.len() != n || out_aspect_rad.len() != n || out_plan_curv.len() != n || out_profile_curv.len() != n {
        return Err(GpMorphError::DimensionMismatch);
    }
    let l2 = cell_size_m * cell_size_m;

    for r in 0..height {
        let (r_prev, r_next) = (r.saturating_sub(1), (r + 1).min(height - 1));
        for c in 0..width {
            let (c_prev, c_next) = (c.saturating_sub(1), (c + 1).min(width - 1));
            let (a, b, c_val) = (dem[r_prev * width + c_prev], dem[r_prev * width + c], dem[r_prev * width + c_next]);
            let (d, e, f) = (dem[r * width + c_prev], dem[r * width + c], dem[r * width + c_next]);
            let (g, h, i) = (dem[r_next * width + c_prev], dem[r_next * width + c], dem[r_next * width + c_next]);

            let p = ((c_val + 2.0 * f + i) - (a + 2.0 * d + g)) / (8.0 * cell_size_m);
            let q = ((a + 2.0 * b + c_val) - (g + 2.0 * h + i)) / (8.0 * cell_size_m);
            let p2_q2 = p * p + q * q;
            let (grad, r_c, t_c, s_c) = (p2_q2.sqrt(), (f - 2.0 * e + d) / l2, (b - 2.0 * e + h) / l2, (c_val - a - i + g) / (4.0 * l2));
            let mut asp = if grad > 1e-7 { (-p).atan2(-q) } else { 0.0 };
            if asp < 0.0 { asp += 2.0 * std::f32::consts::PI; }

            let (plan, prof) = if p2_q2 > 1e-9 {
                (-(q * q * r_c - 2.0 * p * q * s_c + p * p * t_c) / p2_q2.powf(1.5),
                 -(p * p * r_c + 2.0 * p * q * s_c + q * q * t_c) / (p2_q2 * (1.0 + p2_q2).powf(1.5)))
            } else { (0.0, 0.0) };

            let idx = r * width + c;
            out_slope_rad[idx] = grad.atan(); out_aspect_rad[idx] = asp;
            out_plan_curv[idx] = plan; out_profile_curv[idx] = prof;
        }
    }
    Ok(())
}

/// Computes Topographic Position Index (TPI), Riley Ruggedness (TRI),
/// Sappington Vector Ruggedness (VRM), and window Roughness.
///
/// Allocation Class: Tier-1 (`HotZeroHeap`).
pub fn gp_tpi_tri_vrm_roughness_into(
    dem: &[f32], width: usize, height: usize, cell_size_m: f32,
    out_tpi: &mut [f32], out_tri: &mut [f32], out_vrm: &mut [f32], out_roughness: &mut [f32],
) -> Result<(), GpMorphError> {
    validate_dem(dem, width, height, cell_size_m)?;
    let n = width * height;
    if out_tpi.len() != n || out_tri.len() != n || out_vrm.len() != n || out_roughness.len() != n {
        return Err(GpMorphError::DimensionMismatch);
    }

    for r in 0..height {
        for c in 0..width {
            let z_c = dem[r * width + c];
            let (mut sum_neighbors, mut count_neighbors, mut sum_sq_diff) = (0.0f32, 0usize, 0.0f32);
            let (mut min_z, mut max_z) = (z_c, z_c);
            let (mut sum_nx, mut sum_ny, mut sum_nz, mut normal_count) = (0.0f32, 0.0f32, 0.0f32, 0usize);

            for dr in -1..=1 {
                for dc in -1..=1 {
                    let nr = (r as isize + dr).clamp(0, height as isize - 1) as usize;
                    let nc = (c as isize + dc).clamp(0, width as isize - 1) as usize;
                    let z = dem[nr * width + nc];
                    min_z = min_z.min(z); max_z = max_z.max(z);
                    if dr != 0 || dc != 0 { sum_neighbors += z; sum_sq_diff += (z - z_c) * (z - z_c); count_neighbors += 1; }
                    let (nc_prev, nc_next) = (nc.saturating_sub(1), (nc + 1).min(width - 1));
                    let (nr_prev, nr_next) = (nr.saturating_sub(1), (nr + 1).min(height - 1));
                    let p = (dem[nr * width + nc_next] - dem[nr * width + nc_prev]) / (2.0 * cell_size_m);
                    let q = (dem[nr_prev * width + nc] - dem[nr_next * width + nc]) / (2.0 * cell_size_m);
                    let len = (p * p + q * q + 1.0).sqrt();
                    sum_nx += -p / len; sum_ny += -q / len; sum_nz += 1.0 / len; normal_count += 1;
                }
            }

            let idx = r * width + c;
            out_tpi[idx] = if count_neighbors > 0 { z_c - (sum_neighbors / count_neighbors as f32) } else { 0.0 };
            out_tri[idx] = sum_sq_diff.sqrt();
            out_vrm[idx] = if normal_count > 0 {
                (1.0 - ((sum_nx * sum_nx + sum_ny * sum_ny + sum_nz * sum_nz).sqrt() / normal_count as f32)).clamp(0.0, 1.0)
            } else { 0.0 };
            out_roughness[idx] = max_z - min_z;
        }
    }
    Ok(())
}

/// Directional windward exposure / leeward shelter index from horizon angle along wind azimuth.
///
/// Allocation Class: Tier-1 (`HotZeroHeap`).
pub fn gp_wind_exposition_index_into(
    dem: &[f32], width: usize, height: usize, cell_size_m: f32,
    wind_azimuth_rad: f32, search_steps: usize, out_wind_index: &mut [f32],
) -> Result<(), GpMorphError> {
    validate_dem(dem, width, height, cell_size_m)?;
    if search_steps == 0 { return Err(GpMorphError::InvalidParameters); }
    if out_wind_index.len() != width * height { return Err(GpMorphError::DimensionMismatch); }

    let (delta_c, delta_r) = (-wind_azimuth_rad.sin(), -wind_azimuth_rad.cos());

    for r in 0..height {
        for c in 0..width {
            let z_c = dem[r * width + c];
            let (mut max_angle, mut step_count) = (-f32::INFINITY, 0usize);
            for s in 1..=search_steps {
                let cf = c as f32 + s as f32 * delta_c;
                let rf = r as f32 + s as f32 * delta_r;
                let (ci, ri) = (cf.round() as isize, rf.round() as isize);
                if ci < 0 || ci >= width as isize || ri < 0 || ri >= height as isize { break; }
                let d = ((ci - c as isize) as f32).hypot((ri - r as isize) as f32) * cell_size_m;
                if d <= 0.0 { continue; }
                let theta = ((dem[ri as usize * width + ci as usize] - z_c) / d).atan();
                if theta > max_angle { max_angle = theta; }
                step_count += 1;
            }
            out_wind_index[r * width + c] = if step_count > 0 { -max_angle } else { 0.0 };
        }
    }
    Ok(())
}

/// Jasiewicz & Stepinski (2013) 8-azimuth ternary pattern classification into 10 landform classes.
///
/// Allocation Class: Tier-1 (`HotZeroHeap`).
pub fn gp_classify_geomorphons_into(
    dem: &[f32], width: usize, height: usize, cell_size_m: f32,
    lookup_cells: usize, flatness_rad: f32, out_class: &mut [u8],
) -> Result<(), GpMorphError> {
    validate_dem(dem, width, height, cell_size_m)?;
    if lookup_cells == 0 || flatness_rad < 0.0 { return Err(GpMorphError::InvalidParameters); }
    if out_class.len() != width * height { return Err(GpMorphError::DimensionMismatch); }

    for r in 0..height {
        for c in 0..width {
            let z_c = dem[r * width + c];
            let (mut num_plus, mut num_minus) = (0usize, 0usize);

            for &(dc, dr) in &GEOMORPHON_DIRS {
                let (mut max_theta, mut min_theta, mut valid) = (-f32::INFINITY, f32::INFINITY, false);
                for s in 1..=lookup_cells {
                    let (ci, ri) = (c as isize + s as isize * dc, r as isize + s as isize * dr);
                    if ci < 0 || ci >= width as isize || ri < 0 || ri >= height as isize { break; }
                    let d = ((s as f32 * dc as f32).hypot(s as f32 * dr as f32)) * cell_size_m;
                    let theta = ((dem[ri as usize * width + ci as usize] - z_c) / d).atan();
                    if theta > max_theta { max_theta = theta; }
                    if theta < min_theta { min_theta = theta; }
                    valid = true;
                }
                if valid {
                    if max_theta > flatness_rad && min_theta.abs() <= max_theta {
                        num_plus += 1;
                    } else if min_theta < -flatness_rad {
                        num_minus += 1;
                    }
                }
            }
            out_class[r * width + c] = classify_geomorphon_pair(num_plus, num_minus) as u8;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_los_viewshed_ridge_occlusion_and_cumulative() {
        let (width, height) = (5, 5);
        let mut dem = vec![10.0f32; 25];
        for c in 0..5 { dem[2 * width + c] = 30.0; } // Ridge at row 2
        dem[4 * width + 2] = 0.0;   // Pit behind ridge
        dem[4 * width + 4] = 200.0; // High peak behind ridge

        let obs = ObserverSpec {
            col: 2, row: 0, observer_offset_m: 2.0, target_offset_m: 0.0,
            max_radius_m: 100.0, earth_curvature_refraction_k: 0.13,
        };

        let (vis_pit, clearance_pit) = gp_line_of_sight_profile(&dem, width, height, 10.0, &obs, 2, 4).unwrap();
        assert!(!vis_pit && clearance_pit < 0.0);

        let (vis_peak, clearance_peak) = gp_line_of_sight_profile(&dem, width, height, 10.0, &obs, 4, 4).unwrap();
        assert!(vis_peak && clearance_peak > 0.0);

        let vis_grid = gp_viewshed_radial_sweep(&dem, width, height, 10.0, &obs, 1024).unwrap();
        assert_eq!((vis_grid[0 * width + 2], vis_grid[4 * width + 2], vis_grid[4 * width + 4]), (1, 0, 1));

        let obs2 = ObserverSpec {
            col: 2, row: 4, observer_offset_m: 2.0, target_offset_m: 0.0,
            max_radius_m: 100.0, earth_curvature_refraction_k: 0.13,
        };
        let cum = gp_cumulative_viewshed(&dem, width, height, 10.0, &[obs, obs2], 2048).unwrap();
        assert_eq!(cum.len(), 25);
        assert!(cum[4 * width + 4] >= 1);
    }

    #[test]
    fn test_morphometry_horn_tpi_tri_vrm_wind() {
        let (width, height) = (5, 5);
        let dem_flat = vec![50.0f32; 25];
        let (mut slope, mut aspect, mut plan, mut prof) = (vec![0.0f32; 25], vec![0.0f32; 25], vec![0.0f32; 25], vec![0.0f32; 25]);
        gp_horn_slope_aspect_curvature_into(&dem_flat, width, height, 10.0, &mut slope, &mut aspect, &mut plan, &mut prof).unwrap();
        assert_eq!((slope[12], plan[12], prof[12]), (0.0, 0.0, 0.0));

        let (mut tpi, mut tri, mut vrm, mut rough) = (vec![0.0f32; 25], vec![0.0f32; 25], vec![0.0f32; 25], vec![0.0f32; 25]);
        gp_tpi_tri_vrm_roughness_into(&dem_flat, width, height, 10.0, &mut tpi, &mut tri, &mut vrm, &mut rough).unwrap();
        assert_eq!((tpi[12], tri[12], vrm[12], rough[12]), (0.0, 0.0, 0.0, 0.0));

        let mut dem_tilted = vec![0.0f32; 25];
        for r in 0..5 {
            for c in 0..5 { dem_tilted[r * 5 + c] = 100.0 - c as f32 * 10.0; }
        }
        gp_horn_slope_aspect_curvature_into(&dem_tilted, width, height, 10.0, &mut slope, &mut aspect, &mut plan, &mut prof).unwrap();
        assert!(slope[12] > 0.0);
        assert!((aspect[12] - std::f32::consts::FRAC_PI_2).abs() < 0.05);

        let mut wind = vec![0.0f32; 25];
        gp_wind_exposition_index_into(&dem_tilted, width, height, 10.0, 3.0 * std::f32::consts::FRAC_PI_2, 3, &mut wind).unwrap();
        assert_eq!(wind.len(), 25);
    }

    #[test]
    fn test_geomorphons_and_budget_fail_closed() {
        let (width, height) = (5, 5);
        let dem_flat = vec![100.0f32; 25];
        let mut classes = vec![0u8; 25];
        gp_classify_geomorphons_into(&dem_flat, width, height, 10.0, 2, 0.01, &mut classes).unwrap();
        assert_eq!(classes[12], GeomorphonClass::Flat as u8);

        let (mut dem_peak, mut dem_pit) = (vec![10.0f32; 25], vec![50.0f32; 25]);
        dem_peak[12] = 50.0; dem_pit[12] = 10.0;
        gp_classify_geomorphons_into(&dem_peak, width, height, 10.0, 2, 0.01, &mut classes).unwrap();
        assert_eq!(classes[12], GeomorphonClass::Peak as u8);
        gp_classify_geomorphons_into(&dem_pit, width, height, 10.0, 2, 0.01, &mut classes).unwrap();
        assert_eq!(classes[12], GeomorphonClass::Pit as u8);

        let (mut dem_ridge, mut dem_valley) = (vec![10.0f32; 25], vec![50.0f32; 25]);
        for r in 0..5 { dem_ridge[r * 5 + 2] = 50.0; dem_valley[r * 5 + 2] = 10.0; }
        gp_classify_geomorphons_into(&dem_ridge, width, height, 10.0, 2, 0.01, &mut classes).unwrap();
        assert_eq!(classes[12], GeomorphonClass::Ridge as u8);
        gp_classify_geomorphons_into(&dem_valley, width, height, 10.0, 2, 0.01, &mut classes).unwrap();
        assert_eq!(classes[12], GeomorphonClass::Valley as u8);

        let obs = ObserverSpec {
            col: 2, row: 2, observer_offset_m: 1.5, target_offset_m: 0.0,
            max_radius_m: 50.0, earth_curvature_refraction_k: 0.13,
        };
        let err = gp_viewshed_radial_sweep(&dem_flat, width, height, 10.0, &obs, 10).unwrap_err();
        assert!(matches!(err, GpMorphError::ByteBudgetExceeded { .. }));
    }

    #[test]
    fn test_all_10_geomorphon_classes() {
        let cases = [
            ((0, 0), GeomorphonClass::Flat), ((0, 8), GeomorphonClass::Peak),
            ((0, 6), GeomorphonClass::Ridge), ((0, 3), GeomorphonClass::Shoulder),
            ((1, 2), GeomorphonClass::Spur), ((2, 2), GeomorphonClass::Slope),
            ((2, 1), GeomorphonClass::Hollow), ((3, 0), GeomorphonClass::Footslope),
            ((6, 0), GeomorphonClass::Valley), ((8, 0), GeomorphonClass::Pit),
        ];
        for ((h, l), expected) in cases { assert_eq!(classify_geomorphon_pair(h, l), expected); }
    }
}
