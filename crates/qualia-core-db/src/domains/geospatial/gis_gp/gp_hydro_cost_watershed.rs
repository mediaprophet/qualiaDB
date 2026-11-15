// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) RustGIS contributors (adapted under MIT License)
//! Hydrological Indices (TWI, SPI, STI), RUSLE Soil Erosion Loss,
//! Watershed & Sub-basin Delineation, Isochrone Time-of-Concentration,
//! and Anisotropic Cost-Distance & Least-Cost Path Tracing.
//!
//! Allocation Class:
//! - Tier-1 (`HotZeroHeap`): `gp_twi_spi_sti_into`, `gp_rusle_soil_loss_into`.
//! - Tier-2 (`ColdBounded`): `gp_d8_flow_dir_and_watershed_basins`,
//!   `gp_time_of_concentration_isochrones`, `gp_anisotropic_cost_distance_and_backlink`,
//!   `gp_trace_least_cost_path` (enforces caller-supplied `byte_budget`).

const D8_DX: [isize; 8] = [1, 1, 0, -1, -1, -1, 0, 1];
const D8_DY: [isize; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
const SQRT_2: f32 = std::f32::consts::SQRT_2;
const M_EXP: f32 = 0.4;
const N_EXP: f32 = 1.3;
const M_PLUS_1: f32 = 1.4;
const INV_22_13: f32 = 1.0 / 22.13;
const INV_0_0896: f32 = 1.0 / 0.0896;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpHydroCostError {
    DimensionMismatch { expected: usize, found: usize },
    InvalidDimensions,
    InvalidParameter(&'static str),
    ByteBudgetExceeded { requested: usize, budget: usize },
    UnreachableTarget,
}

impl core::fmt::Display for GpHydroCostError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DimensionMismatch { expected, found } => write!(f, "Dim mismatch: exp {expected}, found {found}"),
            Self::InvalidDimensions => write!(f, "Invalid grid dimensions (zero or overflow)"),
            Self::InvalidParameter(msg) => write!(f, "Invalid parameter: {msg}"),
            Self::ByteBudgetExceeded { requested, budget } => write!(f, "Budget exceeded: req {requested} B, budget {budget} B"),
            Self::UnreachableTarget => write!(f, "Target is unreachable from given sources"),
        }
    }
}
impl std::error::Error for GpHydroCostError {}

#[derive(Clone, Copy, PartialEq)]
struct OrderedF32(f32);
impl Eq for OrderedF32 {}
impl PartialOrd for OrderedF32 {
    #[inline] fn partial_cmp(&self, o: &Self) -> Option<core::cmp::Ordering> { Some(self.cmp(o)) }
}
impl Ord for OrderedF32 {
    #[inline] fn cmp(&self, o: &Self) -> core::cmp::Ordering {
        self.0.partial_cmp(&o.0).unwrap_or(core::cmp::Ordering::Equal)
    }
}

/// Tier-1 `HotZeroHeap` Topographic Wetness Index (TWI), Stream Power Index (SPI),
/// and Sediment Transport Index (STI).
pub fn gp_twi_spi_sti_into(
    flow_accum_cells: &[f32],
    slope_rad: &[f32],
    cell_size_m: f32,
    out_twi: &mut [f32],
    out_spi: &mut [f32],
    out_sti: &mut [f32],
) -> Result<(), GpHydroCostError> {
    let n = flow_accum_cells.len();
    if slope_rad.len() != n || out_twi.len() != n || out_spi.len() != n || out_sti.len() != n {
        return Err(GpHydroCostError::DimensionMismatch {
            expected: n,
            found: slope_rad.len().min(out_twi.len()).min(out_spi.len()).min(out_sti.len()),
        });
    }
    if cell_size_m <= 0.0 || !cell_size_m.is_finite() {
        return Err(GpHydroCostError::InvalidParameter("cell_size_m must be positive and finite"));
    }
    for i in 0..n {
        let a = (flow_accum_cells[i].max(0.0) + 1.0) * cell_size_m;
        let beta = slope_rad[i].max(0.0);
        let tan_b = beta.tan().max(1e-4);
        let sin_b = beta.sin().max(1e-4);
        out_twi[i] = (a / tan_b).ln();
        out_spi[i] = a * tan_b;
        out_sti[i] = M_PLUS_1 * (a * INV_22_13).powf(M_EXP) * (sin_b * INV_0_0896).powf(N_EXP);
    }
    Ok(())
}

/// Tier-1 `HotZeroHeap` Revised Universal Soil Loss Equation (RUSLE) soil loss:
/// A = R * K * LS * C * P with Mitasova 3D upslope LS factor.
pub fn gp_rusle_soil_loss_into(
    r_factor: &[f32],
    k_factor: &[f32],
    flow_accum_cells: &[f32],
    slope_rad: &[f32],
    c_factor: &[f32],
    p_factor: &[f32],
    cell_size_m: f32,
    out_tons_ha_yr: &mut [f32],
) -> Result<(), GpHydroCostError> {
    let n = out_tons_ha_yr.len();
    if r_factor.len() != n || k_factor.len() != n || flow_accum_cells.len() != n
        || slope_rad.len() != n || c_factor.len() != n || p_factor.len() != n
    {
        return Err(GpHydroCostError::DimensionMismatch { expected: n, found: r_factor.len() });
    }
    if cell_size_m <= 0.0 || !cell_size_m.is_finite() {
        return Err(GpHydroCostError::InvalidParameter("cell_size_m must be positive and finite"));
    }
    for i in 0..n {
        let a = (flow_accum_cells[i].max(0.0) + 1.0) * cell_size_m;
        let beta = slope_rad[i].max(0.0);
        let sin_b = beta.sin().max(1e-4);
        let ls = M_PLUS_1 * (a * INV_22_13).powf(M_EXP) * (sin_b * INV_0_0896).powf(N_EXP);
        out_tons_ha_yr[i] = r_factor[i].max(0.0) * k_factor[i].max(0.0) * ls * c_factor[i].max(0.0) * p_factor[i].max(0.0);
    }
    Ok(())
}

/// Tier-2 `ColdBounded` D8 flow direction (0..8, 8 = sink/outlet) via priority-flood depression filling
/// and upslope catchment basin delineation for pour points.
pub fn gp_d8_flow_dir_and_watershed_basins(
    dem: &[f32],
    width: usize,
    height: usize,
    pour_points: &[(usize, usize, u32)],
    byte_budget: usize,
) -> Result<(Vec<u8>, Vec<u32>), GpHydroCostError> {
    if width == 0 || height == 0 { return Err(GpHydroCostError::InvalidDimensions); }
    let n = width.checked_mul(height).ok_or(GpHydroCostError::InvalidDimensions)?;
    if dem.len() != n { return Err(GpHydroCostError::DimensionMismatch { expected: n, found: dem.len() }); }
    let requested = n.saturating_mul(core::mem::size_of::<u8>() + core::mem::size_of::<u32>() + core::mem::size_of::<f32>() + core::mem::size_of::<bool>() + 24);
    if requested > byte_budget { return Err(GpHydroCostError::ByteBudgetExceeded { requested, budget: byte_budget }); }

    let mut filled = dem.to_vec();
    let mut visited = vec![false; n];
    let mut heap = std::collections::BinaryHeap::new();

    for y in 0..height {
        for x in 0..width {
            if x == 0 || x == width - 1 || y == 0 || y == height - 1 {
                let idx = y * width + x;
                visited[idx] = true;
                heap.push(std::cmp::Reverse((OrderedF32(filled[idx]), x, y)));
            }
        }
    }

    while let Some(std::cmp::Reverse((OrderedF32(elev), cx, cy))) = heap.pop() {
        for dir in 0..8 {
            let (nx, ny) = (cx as isize + D8_DX[dir], cy as isize + D8_DY[dir]);
            if nx >= 0 && nx < width as isize && ny >= 0 && ny < height as isize {
                let n_idx = ny as usize * width + nx as usize;
                if !visited[n_idx] {
                    visited[n_idx] = true;
                    if filled[n_idx] < elev { filled[n_idx] = elev; }
                    heap.push(std::cmp::Reverse((OrderedF32(filled[n_idx]), nx as usize, ny as usize)));
                }
            }
        }
    }

    let mut d8_dir = vec![8u8; n];
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let (z, mut max_slope, mut best_dir) = (filled[idx], 0.0f32, 8u8);
            for dir in 0..8 {
                let (nx, ny) = (x as isize + D8_DX[dir], y as isize + D8_DY[dir]);
                if nx >= 0 && nx < width as isize && ny >= 0 && ny < height as isize {
                    let n_idx = ny as usize * width + nx as usize;
                    let dist = if D8_DX[dir] == 0 || D8_DY[dir] == 0 { 1.0 } else { SQRT_2 };
                    let drop = (z - filled[n_idx]) / dist;
                    if drop > max_slope {
                        max_slope = drop;
                        best_dir = dir as u8;
                    }
                }
            }
            d8_dir[idx] = best_dir;
        }
    }

    let mut basins = vec![0u32; n];
    for &(px, py, basin_id) in pour_points {
        if px >= width || py >= height || basin_id == 0 { continue; }
        basins[py * width + px] = basin_id;
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((px, py));

        while let Some((cx, cy)) = queue.pop_front() {
            for dir in 0..8 {
                let (nx, ny) = (cx as isize + D8_DX[dir], cy as isize + D8_DY[dir]);
                if nx >= 0 && nx < width as isize && ny >= 0 && ny < height as isize {
                    let n_idx = ny as usize * width + nx as usize;
                    if basins[n_idx] == 0 {
                        let ndir = d8_dir[n_idx];
                        if ndir < 8 && (nx + D8_DX[ndir as usize] == cx as isize) && (ny + D8_DY[ndir as usize] == cy as isize) {
                            basins[n_idx] = basin_id;
                            queue.push_back((nx as usize, ny as usize));
                        }
                    }
                }
            }
        }
    }

    Ok((d8_dir, basins))
}

/// Tier-2 `ColdBounded` Travel time isochrones (s) to outlet along D8 paths (Manning/kinematic wave).
pub fn gp_time_of_concentration_isochrones(
    d8_dir: &[u8],
    slope_rad: &[f32],
    mannings_n: &[f32],
    width: usize,
    height: usize,
    cell_size_m: f32,
    byte_budget: usize,
) -> Result<Vec<f32>, GpHydroCostError> {
    if width == 0 || height == 0 { return Err(GpHydroCostError::InvalidDimensions); }
    let n = width.checked_mul(height).ok_or(GpHydroCostError::InvalidDimensions)?;
    if d8_dir.len() != n || slope_rad.len() != n || mannings_n.len() != n {
        return Err(GpHydroCostError::DimensionMismatch { expected: n, found: d8_dir.len() });
    }
    if cell_size_m <= 0.0 || !cell_size_m.is_finite() {
        return Err(GpHydroCostError::InvalidParameter("cell_size_m must be positive and finite"));
    }
    let requested = n.saturating_mul(core::mem::size_of::<f32>() * 3 + core::mem::size_of::<usize>());
    if requested > byte_budget { return Err(GpHydroCostError::ByteBudgetExceeded { requested, budget: byte_budget }); }

    const R_2_3: f32 = 0.13572088; // (0.05 m sheet flow depth)^(2/3)
    let mut cell_dt = vec![0.0f32; n];
    for i in 0..n {
        let slope = slope_rad[i].max(0.0).sin().max(1e-4);
        let n_val = mannings_n[i].max(0.01);
        let v = ((1.0 / n_val) * R_2_3 * slope.sqrt()).clamp(0.01, 10.0);
        let dir = d8_dir[i];
        let dist = if dir < 8 && (D8_DX[dir as usize] == 0 || D8_DY[dir as usize] == 0) { cell_size_m } else { cell_size_m * SQRT_2 };
        cell_dt[i] = dist / v;
    }

    let mut travel_time = vec![-1.0f32; n];
    let mut path = Vec::with_capacity(32);
    for i in 0..n {
        if travel_time[i] >= 0.0 { continue; }
        path.clear();
        let mut curr = i;
        while curr < n && travel_time[curr] < 0.0 {
            let dir = d8_dir[curr];
            if dir >= 8 {
                travel_time[curr] = 0.0;
                break;
            }
            path.push(curr);
            let (cx, cy) = (curr % width, curr / width);
            let (nx, ny) = (cx as isize + D8_DX[dir as usize], cy as isize + D8_DY[dir as usize]);
            if nx < 0 || nx >= width as isize || ny < 0 || ny >= height as isize {
                travel_time[curr] = 0.0;
                break;
            }
            let next_idx = ny as usize * width + nx as usize;
            if path.len() > n { travel_time[curr] = 0.0; break; }
            curr = next_idx;
        }
        let mut acc = if curr < n && travel_time[curr] >= 0.0 { travel_time[curr] } else { 0.0 };
        for &cell in path.iter().rev() {
            acc += cell_dt[cell];
            travel_time[cell] = acc;
        }
    }
    Ok(travel_time)
}

/// Tier-2 `ColdBounded` Anisotropic cost-distance & backlink grid (Dijkstra wavefront
/// with Tobler hiking slope factor exp(3.5 * |s + 0.05|)). Backlink: 0..7 dir, 8 source, 255 unreached.
pub fn gp_anisotropic_cost_distance_and_backlink(
    friction_cost: &[f32],
    dem: &[f32],
    width: usize,
    height: usize,
    cell_size_m: f32,
    sources: &[(usize, usize)],
    tobler_hiking_weight: f32,
    byte_budget: usize,
) -> Result<(Vec<f32>, Vec<u8>), GpHydroCostError> {
    if width == 0 || height == 0 { return Err(GpHydroCostError::InvalidDimensions); }
    let n = width.checked_mul(height).ok_or(GpHydroCostError::InvalidDimensions)?;
    if friction_cost.len() != n || dem.len() != n {
        return Err(GpHydroCostError::DimensionMismatch { expected: n, found: friction_cost.len() });
    }
    if cell_size_m <= 0.0 || !cell_size_m.is_finite() {
        return Err(GpHydroCostError::InvalidParameter("cell_size_m must be positive and finite"));
    }
    if sources.is_empty() { return Err(GpHydroCostError::InvalidParameter("sources list cannot be empty")); }
    let requested = n.saturating_mul(core::mem::size_of::<f32>() + core::mem::size_of::<u8>() + 24);
    if requested > byte_budget { return Err(GpHydroCostError::ByteBudgetExceeded { requested, budget: byte_budget }); }

    let mut cost = vec![f32::INFINITY; n];
    let mut backlink = vec![255u8; n];
    let mut heap = std::collections::BinaryHeap::new();

    for &(sx, sy) in sources {
        if sx < width && sy < height {
            let s_idx = sy * width + sx;
            cost[s_idx] = 0.0; backlink[s_idx] = 8;
            heap.push(std::cmp::Reverse((OrderedF32(0.0), s_idx)));
        }
    }

    let weight = tobler_hiking_weight.max(0.0);
    while let Some(std::cmp::Reverse((OrderedF32(c), u_idx))) = heap.pop() {
        if c > cost[u_idx] { continue; }
        let (ux, uy) = (u_idx % width, u_idx / width);

        for dir in 0..8 {
            let (nx, ny) = (ux as isize + D8_DX[dir], uy as isize + D8_DY[dir]);
            if nx >= 0 && nx < width as isize && ny >= 0 && ny < height as isize {
                let v_idx = ny as usize * width + nx as usize;
                let dist = cell_size_m * (if D8_DX[dir] == 0 || D8_DY[dir] == 0 { 1.0 } else { SQRT_2 });
                let s = (dem[v_idx] - dem[u_idx]) / dist;
                let tobler_factor = (3.5 * (s + 0.05).abs()).exp();
                let f_base = 0.5 * (friction_cost[u_idx].max(0.0) + friction_cost[v_idx].max(0.0));
                let f_eff = (f_base * (1.0 + weight * (tobler_factor - 1.0))).max(0.0001);
                let new_cost = c + dist * f_eff;

                if new_cost < cost[v_idx] {
                    cost[v_idx] = new_cost;
                    backlink[v_idx] = ((dir + 4) % 8) as u8;
                    heap.push(std::cmp::Reverse((OrderedF32(new_cost), v_idx)));
                }
            }
        }
    }

    Ok((cost, backlink))
}

/// Tier-2 `ColdBounded` Traces optimal least-cost path (col, row) from target to source.
pub fn gp_trace_least_cost_path(
    backlink: &[u8],
    width: usize,
    height: usize,
    target_col: usize,
    target_row: usize,
    byte_budget: usize,
) -> Result<Vec<(usize, usize)>, GpHydroCostError> {
    if width == 0 || height == 0 { return Err(GpHydroCostError::InvalidDimensions); }
    let n = width.checked_mul(height).ok_or(GpHydroCostError::InvalidDimensions)?;
    if backlink.len() != n { return Err(GpHydroCostError::DimensionMismatch { expected: n, found: backlink.len() }); }
    if target_col >= width || target_row >= height { return Err(GpHydroCostError::InvalidParameter("target out of bounds")); }

    let initial_cap = (width + height).max(16);
    let requested = initial_cap.saturating_mul(core::mem::size_of::<(usize, usize)>() + core::mem::size_of::<bool>());
    if requested > byte_budget { return Err(GpHydroCostError::ByteBudgetExceeded { requested, budget: byte_budget }); }

    let mut path = Vec::with_capacity(initial_cap);
    let mut visited_path = vec![false; n];
    let (mut curr_col, mut curr_row) = (target_col, target_row);

    path.push((curr_col, curr_row));
    loop {
        let idx = curr_row * width + curr_col;
        let b = backlink[idx];
        if b == 8 { break; }
        if b == 255 || b > 8 { return Err(GpHydroCostError::UnreachableTarget); }
        visited_path[idx] = true;
        let (nx, ny) = (curr_col as isize + D8_DX[b as usize], curr_row as isize + D8_DY[b as usize]);
        if nx < 0 || nx >= width as isize || ny < 0 || ny >= height as isize { return Err(GpHydroCostError::UnreachableTarget); }
        let next_idx = ny as usize * width + nx as usize;
        if visited_path[next_idx] { return Err(GpHydroCostError::UnreachableTarget); }
        curr_col = nx as usize;
        curr_row = ny as usize;

        if path.len().saturating_mul(core::mem::size_of::<(usize, usize)>()) > byte_budget {
            return Err(GpHydroCostError::ByteBudgetExceeded {
                requested: path.len() * core::mem::size_of::<(usize, usize)>(),
                budget: byte_budget,
            });
        }
        path.push((curr_col, curr_row));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_twi_spi_sti_and_rusle() {
        let (accum, slope) = ([99.0f32, 499.0], [0.1f32, 0.2]);
        let (mut twi, mut spi, mut sti) = ([0.0f32; 2], [0.0f32; 2], [0.0f32; 2]);
        gp_twi_spi_sti_into(&accum, &slope, 10.0, &mut twi, &mut spi, &mut sti).expect("indices");
        assert!(twi[0] > 0.0 && twi[1] > 0.0);
        assert!(spi[1] > spi[0] && sti[1] > sti[0]);

        let (r, k, c, p) = ([100.0f32; 2], [0.3f32; 2], [0.1f32; 2], [0.8f32; 2]);
        let mut soil_loss = [0.0f32; 2];
        gp_rusle_soil_loss_into(&r, &k, &accum, &slope, &c, &p, 10.0, &mut soil_loss).expect("rusle");
        let expected_ls_0 = 1.4 * (1000.0f32 / 22.13).powf(0.4) * (0.1f32.sin() / 0.0896).powf(1.3);
        let expected_loss_0 = 100.0 * 0.3 * expected_ls_0 * 0.1 * 0.8;
        assert!((soil_loss[0] - expected_loss_0).abs() < 1e-3);
    }

    #[test]
    fn test_watershed_basin_delineation_v_valley() {
        let (w, h) = (5, 5);
        let mut dem = vec![0.0f32; w * h];
        for y in 0..h {
            for x in 0..w { dem[y * w + x] = (y as f32 - 2.0).abs() * 10.0 + (x as f32) * 2.0; }
        }
        let (d8, basins) = gp_d8_flow_dir_and_watershed_basins(&dem, w, h, &[(0, 2, 42)], 50_000).expect("basins");
        assert_eq!(d8[2 * w + 0], 8);
        assert_eq!(basins[2 * w + 0], 42);
        assert_eq!(basins[0 * w + 4], 42);
        assert_eq!(basins[4 * w + 4], 42);

        let (slope, manning) = (vec![0.08f32; w * h], vec![0.035f32; w * h]);
        let tc = gp_time_of_concentration_isochrones(&d8, &slope, &manning, w, h, 10.0, 50_000).expect("tc");
        assert_eq!(tc[2 * w + 0], 0.0);
        assert!(tc[2 * w + 4] > 0.0 && tc[0 * w + 4] > tc[2 * w + 2]);
    }

    #[test]
    fn test_anisotropic_cost_distance_and_least_cost_path() {
        let (w, h) = (7, 7);
        let dem = vec![0.0f32; w * h];
        let mut friction = vec![1.0f32; w * h];
        for y in 1..=5 { friction[y * w + 3] = 1000.0; }
        let (cost, backlink) = gp_anisotropic_cost_distance_and_backlink(
            &friction, &dem, w, h, 10.0, &[(0, 3)], 1.0, 50_000,
        ).expect("cost-dist");

        assert_eq!(cost[3 * w + 0], 0.0);
        assert_eq!(backlink[3 * w + 0], 8);
        assert!(cost[3 * w + 6] < 10000.0);

        let path = gp_trace_least_cost_path(&backlink, w, h, 6, 3, 50_000).expect("path");
        assert_eq!(path.first(), Some(&(6, 3)));
        assert_eq!(path.last(), Some(&(0, 3)));
        for &(px, py) in &path {
            if px == 3 { assert!(py == 0 || py == 6); }
        }
    }

    #[test]
    fn test_byte_budget_fail_closed() {
        let dem = vec![10.0f32; 16];
        let res = gp_d8_flow_dir_and_watershed_basins(&dem, 4, 4, &[(0, 0, 1)], 10);
        assert!(matches!(res, Err(GpHydroCostError::ByteBudgetExceeded { .. })));

        let (d8, slope, manning) = (vec![8u8; 16], vec![0.1f32; 16], vec![0.03f32; 16]);
        let res_tc = gp_time_of_concentration_isochrones(&d8, &slope, &manning, 4, 4, 10.0, 10);
        assert!(matches!(res_tc, Err(GpHydroCostError::ByteBudgetExceeded { .. })));

        let friction = vec![1.0f32; 16];
        let res_cost = gp_anisotropic_cost_distance_and_backlink(&friction, &dem, 4, 4, 10.0, &[(0, 0)], 1.0, 10);
        assert!(matches!(res_cost, Err(GpHydroCostError::ByteBudgetExceeded { .. })));

        let res_path = gp_trace_least_cost_path(&[8u8; 16], 4, 4, 2, 2, 10);
        assert!(matches!(res_path, Err(GpHydroCostError::ByteBudgetExceeded { .. })));
    }
}
