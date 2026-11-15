// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) EarthCraft and Nerthus contributors (adapted under MIT License)
//! Fixed-point 8.24 bit-exact hydraulic pipe solver, 6-species stratigraphic erosion,
//! Lagrangian droplet erosion, and meandering channel carving.
//!
//! Tier-1 `HotZeroHeap`: zero heap allocations in all simulation loops. All grid
//! state is passed as caller-supplied slices. Bit-exact integer arithmetic guarantees
//! identical results across x86_64, ARM64, and WASM.

/// Scale factor for 8.24 fixed-point integer representations (2^24 = 16,777,216).
pub const FIXED_8_24_SCALE: i64 = 1 << 24;

/// Converts an `f32` value to 8.24 fixed-point format (`i64`).
#[inline]
pub fn f32_to_fixed824(val: f32) -> i64 {
    (val * (FIXED_8_24_SCALE as f32)).round() as i64
}

/// Converts an 8.24 fixed-point format (`i64`) to `f32`.
#[inline]
pub fn fixed824_to_f32(val: i64) -> f32 {
    val as f32 / (FIXED_8_24_SCALE as f32)
}

/// Executes a single bit-exact 8.24 fixed-point virtual-pipe hydraulic step.
/// Conserves bedrock + sediment mass within integer rounding; returns net mass delta.
pub fn step_hydraulic_pipe_fixed824(
    height_q24: &mut [i64],
    water_q24: &mut [i64],
    sediment_q24: &mut [i64],
    flux_nesw_q24: &mut [[i64; 4]],
    width: usize,
    height: usize,
    rain_q24: i64,
    evap_shift: u32,
    capacity_shift: u32,
) -> Result<i64, &'static str> {
    if width == 0 || height == 0 { return Err("Grid dimensions must be non-zero"); }
    let total = width.checked_mul(height).ok_or("Grid dimensions overflow")?;
    if height_q24.len() != total || water_q24.len() != total || sediment_q24.len() != total || flux_nesw_q24.len() != total {
        return Err("Slice length does not match width * height");
    }

    let mut mass_before: i64 = 0;
    for i in 0..total { mass_before = mass_before.wrapping_add(height_q24[i]).wrapping_add(sediment_q24[i]); }
    if rain_q24 > 0 { for w in water_q24.iter_mut() { *w = w.saturating_add(rain_q24); } }

    // 1. Outflow flux update across the 4 virtual pipes (0:N, 1:E, 2:S, 3:W)
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let total_h = height_q24[idx].saturating_add(water_q24[idx]);
            let n_idx = [y.checked_sub(1).map(|py| py * width + x), (x + 1 < width).then_some(y * width + x + 1), (y + 1 < height).then_some((y + 1) * width + x), x.checked_sub(1).map(|px| y * width + px)];

            for d in 0..4 {
                flux_nesw_q24[idx][d] = match n_idx[d] {
                    Some(ni) => (flux_nesw_q24[idx][d] + ((total_h - height_q24[ni].saturating_add(water_q24[ni])) >> 1)).max(0),
                    None => 0,
                };
            }

            let sum_flux = flux_nesw_q24[idx][0] + flux_nesw_q24[idx][1] + flux_nesw_q24[idx][2] + flux_nesw_q24[idx][3];
            let cur_w = water_q24[idx];
            if cur_w <= 0 {
                flux_nesw_q24[idx] = [0; 4];
            } else if sum_flux > cur_w {
                let mut sum_scaled = 0i64;
                for d in 0..3 {
                    let scaled = ((flux_nesw_q24[idx][d] as i128 * cur_w as i128) / sum_flux as i128) as i64;
                    flux_nesw_q24[idx][d] = scaled;
                    sum_scaled += scaled;
                }
                flux_nesw_q24[idx][3] = (cur_w - sum_scaled).max(0);
            }
        }
    }

    // 2. Water update and sediment exchange
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let outflow = flux_nesw_q24[idx][0] + flux_nesw_q24[idx][1] + flux_nesw_q24[idx][2] + flux_nesw_q24[idx][3];
            let inflow = (if y > 0 { flux_nesw_q24[(y - 1) * width + x][2] } else { 0 }) + (if x + 1 < width { flux_nesw_q24[y * width + x + 1][3] } else { 0 })
                + (if y + 1 < height { flux_nesw_q24[(y + 1) * width + x][0] } else { 0 }) + (if x > 0 { flux_nesw_q24[y * width + x - 1][1] } else { 0 });

            let new_w = (water_q24[idx] + (inflow - outflow)).max(0);
            let evap = if evap_shift < 64 { new_w >> evap_shift } else { 0 };
            water_q24[idx] = new_w.saturating_sub(evap);

            let flow_mag = (inflow + outflow) >> 1;
            let capacity = if capacity_shift < 64 { flow_mag >> capacity_shift } else { 0 };
            let s = sediment_q24[idx];
            if s < capacity {
                let erode = ((capacity - s) >> 2).min(height_q24[idx]);
                height_q24[idx] -= erode;
                sediment_q24[idx] += erode;
            } else if s > capacity {
                let deposit = (s - capacity) >> 2;
                height_q24[idx] += deposit;
                sediment_q24[idx] -= deposit;
            }
        }
    }

    let mut mass_after: i64 = 0;
    for i in 0..total { mass_after = mass_after.wrapping_add(height_q24[i]).wrapping_add(sediment_q24[i]); }
    Ok(mass_after.wrapping_sub(mass_before))
}

/// 6-species stratigraphic column with water and suspended sediment.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct StrataCell6 {
    pub bedrock: f32, pub saprolite: f32, pub regolith: f32, pub sand: f32,
    pub silt: f32, pub snow_ice: f32, pub water: f32, pub suspended_sediment: f32,
}

impl StrataCell6 {
    /// Total bed surface elevation below water.
    #[inline] pub fn surface_elevation(&self) -> f32 { self.bedrock + self.saprolite + self.regolith + self.sand + self.silt + self.snow_ice }
    /// Total solid mass in the geological strata.
    #[inline] pub fn total_solid_mass(&self) -> f32 { self.bedrock + self.saprolite + self.regolith + self.sand + self.silt + self.snow_ice }
}

/// Parameters for coupled 6-species stratigraphic erosion.
#[derive(Debug, Clone, Copy)]
pub struct CoupledErosionParams {
    pub dt: f32, pub cell_size: f32, pub rainfall_rate: f32, pub evaporation_rate: f32,
    pub kc_transport: f32, pub ks_silt: f32, pub ks_sand: f32, pub ks_regolith: f32,
    pub ks_saprolite: f32, pub ks_bedrock: f32, pub kd_deposition: f32,
}

impl Default for CoupledErosionParams {
    fn default() -> Self {
        Self {
            dt: 0.1, cell_size: 1.0, rainfall_rate: 0.01, evaporation_rate: 0.005,
            kc_transport: 1.0, ks_silt: 0.8, ks_sand: 0.5, ks_regolith: 0.2,
            ks_saprolite: 0.08, ks_bedrock: 0.02, kd_deposition: 0.1,
        }
    }
}

/// Evaluates a coupled 6-species stratigraphic hydraulic erosion step.
pub fn step_coupled_6species_erosion(
    cells: &mut [StrataCell6],
    flux_scratch: &mut [[f32; 4]],
    width: usize,
    height: usize,
    params: &CoupledErosionParams,
) -> Result<f32, &'static str> {
    if width == 0 || height == 0 { return Err("Grid dimensions must be non-zero"); }
    let total = width.checked_mul(height).ok_or("Grid dimensions overflow")?;
    if cells.len() != total || flux_scratch.len() != total { return Err("Slice length mismatch"); }
    if params.cell_size <= 0.0 || params.dt <= 0.0 { return Err("Invalid parameters"); }

    let rain_vol = params.rainfall_rate * params.dt;
    if rain_vol > 0.0 { for c in cells.iter_mut() { c.water += rain_vol; } }

    let g = 9.81f32;
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let total_h = cells[idx].surface_elevation() + cells[idx].water;
            let n_idx = [y.checked_sub(1).map(|py| py * width + x), (x + 1 < width).then_some(y * width + x + 1), (y + 1 < height).then_some((y + 1) * width + x), x.checked_sub(1).map(|px| y * width + px)];

            for d in 0..4 {
                flux_scratch[idx][d] = match n_idx[d] {
                    Some(ni) => (flux_scratch[idx][d] + params.dt * (g * (total_h - (cells[ni].surface_elevation() + cells[ni].water)) / params.cell_size)).max(0.0),
                    None => 0.0,
                };
            }

            let sum_flux = flux_scratch[idx][0] + flux_scratch[idx][1] + flux_scratch[idx][2] + flux_scratch[idx][3];
            let max_flux = cells[idx].water / params.dt;
            if max_flux <= 0.0 {
                flux_scratch[idx] = [0.0; 4];
            } else if sum_flux > max_flux && sum_flux > 1e-7 {
                let scale = max_flux / sum_flux;
                for d in 0..4 { flux_scratch[idx][d] *= scale; }
            }
        }
    }

    let mut total_mobilized = 0.0f32;
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let outflow = flux_scratch[idx][0] + flux_scratch[idx][1] + flux_scratch[idx][2] + flux_scratch[idx][3];
            let inflow = (if y > 0 { flux_scratch[(y - 1) * width + x][2] } else { 0.0 }) + (if x + 1 < width { flux_scratch[y * width + x + 1][3] } else { 0.0 })
                + (if y + 1 < height { flux_scratch[(y + 1) * width + x][0] } else { 0.0 }) + (if x > 0 { flux_scratch[y * width + x - 1][1] } else { 0.0 });

            let new_w = (cells[idx].water + (inflow - outflow) * params.dt).max(0.0);
            cells[idx].water = (new_w - params.evaporation_rate * params.dt).max(0.0);

            let cur_elev = cells[idx].surface_elevation();
            let mut max_dh = 0.0f32;
            if y > 0 { max_dh = max_dh.max(cur_elev - cells[(y - 1) * width + x].surface_elevation()); } if x + 1 < width { max_dh = max_dh.max(cur_elev - cells[y * width + x + 1].surface_elevation()); }
            if y + 1 < height { max_dh = max_dh.max(cur_elev - cells[(y + 1) * width + x].surface_elevation()); } if x > 0 { max_dh = max_dh.max(cur_elev - cells[y * width + x - 1].surface_elevation()); }
            let slope = (max_dh / params.cell_size).max(0.01);

            let capacity = params.kc_transport * 0.5 * (inflow + outflow) * slope;
            let suspended = cells[idx].suspended_sediment;

            if suspended < capacity {
                let mut deficit = (capacity - suspended) * params.dt;
                let layers: [(&mut f32, f32); 5] = [
                    (&mut cells[idx].silt, params.ks_silt),
                    (&mut cells[idx].sand, params.ks_sand),
                    (&mut cells[idx].regolith, params.ks_regolith),
                    (&mut cells[idx].saprolite, params.ks_saprolite),
                    (&mut cells[idx].bedrock, params.ks_bedrock),
                ];
                for (layer, rate) in layers {
                    if deficit <= 0.0 { break; }
                    if *layer > 0.0 {
                        let take = (deficit * rate).min(*layer);
                        *layer -= take;
                        cells[idx].suspended_sediment += take;
                        deficit -= take;
                        total_mobilized += take;
                    }
                }
            } else if suspended > capacity {
                let dep = ((suspended - capacity) * params.kd_deposition * params.dt).min(cells[idx].suspended_sediment);
                let dep_sand = dep * 0.7;
                cells[idx].sand += dep_sand;
                cells[idx].silt += dep - dep_sand;
                cells[idx].suspended_sediment -= dep;
            }
        }
    }

    Ok(total_mobilized)
}

/// Traces a single Lagrangian water droplet downhill, carrying and depositing sediment.
pub fn trace_erosion_droplet(
    dem: &mut [f32],
    width: usize,
    height: usize,
    start_x: f32,
    start_y: f32,
    inertia: f32,
    capacity_mult: f32,
    deposition_rate: f32,
    erosion_rate: f32,
    evaporation: f32,
    max_steps: u32,
) -> Result<f32, &'static str> {
    if width < 2 || height < 2 { return Err("Grid dimensions must be at least 2x2"); }
    if dem.len() != width * height { return Err("DEM length mismatch"); }

    let (mut px, mut py) = (start_x, start_y);
    let (mut dir_x, mut dir_y) = (0.0f32, 0.0f32);
    let (mut speed, mut water, mut sediment) = (1.0f32, 1.0f32, 0.0f32);
    let mut total_eroded = 0.0f32;

    for _ in 0..max_steps {
        let (ix, iy) = (px.floor() as usize, py.floor() as usize);
        if ix >= width - 1 || iy >= height - 1 || px < 0.0 || py < 0.0 { break; }

        let (u, v) = (px - ix as f32, py - iy as f32);
        let idx00 = iy * width + ix;
        let (h00, h10, h01, h11) = (dem[idx00], dem[idx00 + 1], dem[idx00 + width], dem[idx00 + width + 1]);
        let cur_h = (1.0 - u) * (1.0 - v) * h00 + u * (1.0 - v) * h10 + (1.0 - u) * v * h01 + u * v * h11;
        let gx = (h10 - h00) * (1.0 - v) + (h11 - h01) * v;
        let gy = (h01 - h00) * (1.0 - u) + (h11 - h10) * u;

        dir_x = dir_x * inertia - gx * (1.0 - inertia);
        dir_y = dir_y * inertia - gy * (1.0 - inertia);
        let len_sq = dir_x * dir_x + dir_y * dir_y;
        if len_sq < 1e-8 { break; }
        let inv_len = 1.0 / len_sq.sqrt();
        dir_x *= inv_len; dir_y *= inv_len;

        let (next_px, next_py) = (px + dir_x, py + dir_y);
        let (nix, niy) = (next_px.floor() as usize, next_py.floor() as usize);
        if nix >= width - 1 || niy >= height - 1 || next_px < 0.0 || next_py < 0.0 { break; }

        let (nu, nv) = (next_px - nix as f32, next_py - niy as f32);
        let nidx00 = niy * width + nix;
        let next_h = (1.0 - nu) * (1.0 - nv) * dem[nidx00] + nu * (1.0 - nv) * dem[nidx00 + 1]
            + (1.0 - nu) * nv * dem[nidx00 + width] + nu * nv * dem[nidx00 + width + 1];
        let dh = next_h - cur_h;

        let w = [(1.0 - u) * (1.0 - v), u * (1.0 - v), (1.0 - u) * v, u * v];
        let c = [idx00, idx00 + 1, idx00 + width, idx00 + width + 1];

        if dh > 0.0 {
            let dep = dh.min(sediment);
            for i in 0..4 { dem[c[i]] += dep * w[i]; }
            sediment -= dep;
            if sediment < 1e-5 { break; }
        } else {
            let capacity = (-dh).max(0.01) * speed * water * capacity_mult;
            if sediment > capacity {
                let dep = (sediment - capacity) * deposition_rate;
                for i in 0..4 { dem[c[i]] += dep * w[i]; }
                sediment -= dep;
            } else {
                let erode = ((capacity - sediment) * erosion_rate).min(-dh);
                for i in 0..4 { dem[c[i]] -= erode * w[i]; }
                sediment += erode;
                total_eroded += erode;
            }
        }

        speed = (speed * speed + (-dh) * 9.81).max(0.0).sqrt();
        water *= (1.0 - evaporation).max(0.0);
        if water < 1e-4 { break; }
        px = next_px; py = next_py;
    }
    Ok(total_eroded)
}

/// Carves an asymmetric meandering river cross-section (outer cutbank / inner point bar) along a polyline.
pub fn carve_meander_channel(
    dem: &mut [f32],
    width: usize,
    height: usize,
    centerline_xy: &[[f32; 2]],
    bankfull_width_cells: f32,
    thalweg_depth: f32,
    point_bar_asymmetry: f32,
) -> Result<(), &'static str> {
    if width == 0 || height == 0 { return Err("Grid dimensions must be non-zero"); }
    if dem.len() != width * height { return Err("DEM slice length mismatch"); }
    if centerline_xy.len() < 2 { return Err("Centerline must have at least 2 points"); }
    if bankfull_width_cells <= 0.0 || thalweg_depth <= 0.0 { return Err("Invalid channel geometry"); }

    for y in 0..height {
        let cy = y as f32;
        for x in 0..width {
            let cx = x as f32;
            let mut min_dist_sq = f32::MAX;
            let mut best_signed_lat = 0.0f32;
            let mut best_kappa = 0.0f32;

            for i in 0..(centerline_xy.len() - 1) {
                let (p0, p1) = (centerline_xy[i], centerline_xy[i + 1]);
                let (vx, vy) = (p1[0] - p0[0], p1[1] - p0[1]);
                let len_sq = vx * vx + vy * vy;
                if len_sq < 1e-8 { continue; }
                let inv_len = 1.0 / len_sq.sqrt();
                let (tx, ty) = (vx * inv_len, vy * inv_len);
                let (nx, ny) = (-ty, tx);

                let (wx, wy) = (cx - p0[0], cy - p0[1]);
                let t_clamped = ((wx * tx + wy * ty) * inv_len).clamp(0.0, 1.0);
                let (dx, dy) = (cx - (p0[0] + vx * t_clamped), cy - (p0[1] + vy * t_clamped));
                let d_sq = dx * dx + dy * dy;

                if d_sq < min_dist_sq {
                    min_dist_sq = d_sq;
                    best_signed_lat = dx * nx + dy * ny;
                    let k0 = if i > 0 { let (ux, uy) = (p0[0] - centerline_xy[i - 1][0], p0[1] - centerline_xy[i - 1][1]); (ux * vy - uy * vx) / ((ux * ux + uy * uy).sqrt() * len_sq.sqrt() + 1e-6) } else { 0.0 };
                    let k1 = if i + 2 < centerline_xy.len() { let (nx2, ny2) = (centerline_xy[i + 2][0] - p1[0], centerline_xy[i + 2][1] - p1[1]); (vx * ny2 - vy * nx2) / (len_sq.sqrt() * (nx2 * nx2 + ny2 * ny2).sqrt() + 1e-6) } else { 0.0 };
                    best_kappa = ((1.0 - t_clamped) * k0 + t_clamped * k1).clamp(-1.0, 1.0);
                }
            }

            let min_dist = min_dist_sq.sqrt();
            if min_dist <= bankfull_width_cells {
                let xi = (best_signed_lat / bankfull_width_cells).clamp(-1.0, 1.0);
                let thalweg_shift = (point_bar_asymmetry * best_kappa).clamp(-0.7, 0.7);
                let xi_rel = if xi < thalweg_shift { (xi - thalweg_shift) / (1.0 + thalweg_shift) } else { (xi - thalweg_shift) / (1.0 - thalweg_shift) };
                let profile = (1.0 - xi_rel * xi_rel).max(0.0);
                dem[y * width + x] -= thalweg_depth * profile * profile;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_hydraulic_pipe_fixed824_conservation_and_determinism() {
        let (mut height1, mut water1, mut sediment1, mut flux1) = ([0i64; 16], [0i64; 16], [0i64; 16], [[0i64; 4]; 16]);
        for i in 0..16 { height1[i] = f32_to_fixed824((16 - i) as f32 * 5.0); }
        water1[0] = f32_to_fixed824(10.0);

        let (mut height2, mut water2, mut sediment2, mut flux2) = (height1, water1, sediment1, flux1);

        for _ in 0..5 {
            let delta1 = step_hydraulic_pipe_fixed824(
                &mut height1, &mut water1, &mut sediment1, &mut flux1,
                4, 4, f32_to_fixed824(0.05), 3, 2,
            ).unwrap();
            assert_eq!(delta1, 0, "Mass delta must be zero (mass conserved)");

            let delta2 = step_hydraulic_pipe_fixed824(
                &mut height2, &mut water2, &mut sediment2, &mut flux2,
                4, 4, f32_to_fixed824(0.05), 3, 2,
            ).unwrap();
            assert_eq!(delta2, 0);
        }

        assert_eq!(height1, height2, "Bit-exact determinism across identical runs");
        assert_eq!(water1, water2);
        assert_eq!(sediment1, sediment2);
        assert_eq!(flux1, flux2);
    }

    #[test]
    fn test_step_coupled_6species_differential_erodibility() {
        let (mut cells, mut flux) = ([StrataCell6::default(); 4], [[0.0f32; 4]; 4]);
        cells[0].silt = 20.0; cells[0].sand = 10.0; cells[0].regolith = 5.0; cells[0].saprolite = 5.0; cells[0].bedrock = 50.0; cells[0].water = 5.0;
        cells[1].bedrock = 20.0; cells[2].bedrock = 10.0; cells[3].bedrock = 0.0;

        let params = CoupledErosionParams {
            dt: 0.1, cell_size: 1.0, rainfall_rate: 0.1, evaporation_rate: 0.01,
            kc_transport: 5.0, ks_silt: 0.9, ks_sand: 0.5, ks_regolith: 0.2,
            ks_saprolite: 0.08, ks_bedrock: 0.01, kd_deposition: 0.1,
        };

        let mass_before: f32 = cells.iter().map(|c| c.total_solid_mass() + c.suspended_sediment).sum();
        let initial_silt = cells[0].silt;
        let initial_bedrock = cells[0].bedrock;

        let mobilized = step_coupled_6species_erosion(&mut cells, &mut flux, 2, 2, &params).unwrap();
        assert!(mobilized > 0.0, "Erosion should mobilize mass");

        let mass_after: f32 = cells.iter().map(|c| c.total_solid_mass() + c.suspended_sediment).sum();
        assert!((mass_after - mass_before).abs() < 1e-4, "Solid mass + sediment strictly conserved");

        let silt_lost = initial_silt - cells[0].silt;
        let bedrock_lost = initial_bedrock - cells[0].bedrock;
        assert!(silt_lost > bedrock_lost, "Silt must erode faster than bedrock");
    }

    #[test]
    fn test_trace_erosion_droplet_16x16() {
        let mut dem = [0.0f32; 256];
        for y in 0..16 {
            for x in 0..16 { dem[y * 16 + x] = 100.0 - (x + y) as f32 * 2.5; }
        }

        let initial_elev = dem[2 * 16 + 2];
        let eroded = trace_erosion_droplet(
            &mut dem, 16, 16, 2.0, 2.0, 0.2, 2.0, 0.1, 0.3, 0.01, 40,
        ).unwrap();

        assert!(eroded > 0.0, "Droplet must erode terrain on downhill slope");
        assert!(dem[2 * 16 + 2] <= initial_elev, "Elevation at start path should be eroded");
    }

    #[test]
    fn test_carve_meander_channel_16x16() {
        let mut dem = [60.0f32; 256];
        let centerline = [[2.0, 8.0], [8.0, 4.0], [14.0, 8.0]];

        carve_meander_channel(&mut dem, 16, 16, &centerline, 3.0, 12.0, 0.7).unwrap();

        assert_eq!(dem[0], 60.0, "Corner far from centerline must be untouched");
        assert!(dem[4 * 16 + 8] < 60.0, "Channel center must be carved");

        let depth_point_bar = 60.0 - dem[6 * 16 + 8];
        let depth_cutbank = 60.0 - dem[2 * 16 + 8];
        assert!(depth_cutbank != depth_point_bar, "Meander channel cross-section must be asymmetric");
    }
}
