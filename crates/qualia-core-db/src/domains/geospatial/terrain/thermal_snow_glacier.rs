// Copyright 2026 QualiaDB Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Geomorphic thermal weathering, glacier shallow-ice approximation (SIA),
//! snow wind/avalanche runout, and aeolian sand transport.
//!
//! All operations strictly adhere to Tier-1 `HotZeroHeap` caller-buffered
//! execution contracts with deterministic mass conservation.

const NEIGHBORS_8: [(i32, i32, f32); 8] = [
    (-1, 0, 1.0), (1, 0, 1.0), (0, -1, 1.0), (0, 1, 1.0),
    (-1, -1, 1.41421356), (1, -1, 1.41421356), (-1, 1, 1.41421356), (1, 1, 1.41421356),
];

#[inline(always)]
fn validate_grid(w: usize, h: usize, cell_size: f32, bufs: &[usize]) -> Result<usize, &'static str> {
    let n = w.checked_mul(h).ok_or("Grid dimensions overflow")?;
    if w == 0 || h == 0 || cell_size <= 0.0 { return Err("Invalid dimensions or cell size"); }
    for &len in bufs { if len != n { return Err("Buffer length mismatch"); } }
    Ok(n)
}

#[inline(always)]
fn diffuse_steep_mass(
    x: usize, y: usize, w: usize, h: usize, cell_size: f32, tan_crit: f32,
    mobile: f32, base: &[f32], layer: &[f32], scratch: &mut [f32],
) -> f32 {
    let idx = y * w + x;
    let z = base[idx] + layer[idx];
    let (mut ex, mut nbrs, mut sum_ex, mut count) = ([0.0f32; 8], [0usize; 8], 0.0f32, 0);
    for &(dx, dy, dist) in &NEIGHBORS_8 {
        let (nx, ny) = (x as i32 + dx, y as i32 + dy);
        if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
            let n_idx = (ny as usize) * w + (nx as usize);
            let diff = z - (base[n_idx] + layer[n_idx]);
            let thresh = cell_size * dist * tan_crit;
            if diff > thresh {
                let e = (diff - thresh) / dist;
                ex[count] = e; nbrs[count] = n_idx; sum_ex += e; count += 1;
            }
        }
    }
    if sum_ex > 0.0 {
        let mv = mobile.min(0.5 * sum_ex);
        scratch[idx] -= mv;
        for i in 0..count { scratch[nbrs[i]] += mv * (ex[i] / sum_ex); }
        mv
    } else { 0.0 }
}

/// 8-Neighbor anisotropic thermal talus creep & soil-to-bedrock freeze-thaw weathering.
///
/// Converts exposed steep bedrock into scree and redistributes mobile scree
/// downslope across all 8 neighbors whenever local slope exceeds `talus_angle_deg`.
/// Total mass (`bedrock + scree`) is strictly conserved.
pub fn step_thermal_talus_creep(
    bedrock: &mut [f32], scree: &mut [f32], delta_scratch: &mut [f32],
    width: usize, height: usize, cell_size: f32,
    talus_angle_deg: f32, creep_rate: f32, freeze_thaw_rate: f32,
) -> Result<f32, &'static str> {
    let n = validate_grid(width, height, cell_size, &[bedrock.len(), scree.len(), delta_scratch.len()])?;
    let tan_talus = talus_angle_deg.to_radians().tan();

    // Phase 1: Bedrock freeze-thaw weathering into mobile scree
    if freeze_thaw_rate > 0.0 {
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let b = bedrock[idx];
                if b <= 0.0 { continue; }
                let mut max_b_slope: f32 = 0.0;
                for &(dx, dy, dist) in &NEIGHBORS_8 {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                        let diff = b - bedrock[(ny as usize) * width + (nx as usize)];
                        if diff > 0.0 { max_b_slope = max_b_slope.max(diff / (cell_size * dist)); }
                    }
                }
                if max_b_slope > tan_talus {
                    let weathered = (b * freeze_thaw_rate * (max_b_slope / tan_talus).min(2.0)).min(b);
                    bedrock[idx] -= weathered;
                    scree[idx] += weathered;
                }
            }
        }
    }

    // Phase 2: Anisotropic 8-neighbor talus creep
    delta_scratch.fill(0.0);
    let (mut total_moved, k_creep) = (0.0f32, creep_rate.clamp(0.0, 1.0));
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let mobile = scree[idx] * k_creep;
            if mobile > 1e-6 {
                total_moved += diffuse_steep_mass(
                    x, y, width, height, cell_size, tan_talus, mobile,
                    bedrock, scree, delta_scratch,
                );
            }
        }
    }
    for i in 0..n { scree[i] = (scree[i] + delta_scratch[i]).max(0.0); }
    Ok(total_moved)
}

/// Shallow translational landslide and debris-flow runout.
///
/// Evaluates Infinite-Slope Mohr-Coulomb Factor of Safety:
/// FS = (c' + (gamma * z * cos^2(beta) - u) * tan(phi')) / (gamma * z * sin(beta) * cos(beta)).
/// Where FS < 1.0, mobilizes failed regolith downslope, returning count of failed cells.
pub fn step_landslide_debris_runout(
    bedrock: &[f32], regolith: &mut [f32], pore_pressure: &[f32], flux_scratch: &mut [f32],
    width: usize, height: usize, cell_size: f32,
    cohesion_kpa: f32, friction_angle_deg: f32, runout_mobility: f32,
) -> Result<usize, &'static str> {
    let n = validate_grid(
        width, height, cell_size,
        &[bedrock.len(), regolith.len(), pore_pressure.len(), flux_scratch.len()],
    )?;
    flux_scratch.fill(0.0);
    let (tan_phi, gamma) = (friction_angle_deg.to_radians().tan(), 18.0f32);
    let (mobility, mut failed_count) = (runout_mobility.clamp(0.05, 0.95), 0usize);

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let z = regolith[idx];
            if z <= 0.01 { continue; }
            let total_z = bedrock[idx] + z;
            let (mut max_slope, mut steepest_nbr) = (0.0f32, idx);

            for &(dx, dy, dist) in &NEIGHBORS_8 {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    let n_idx = (ny as usize) * width + (nx as usize);
                    let diff = total_z - (bedrock[n_idx] + regolith[n_idx]);
                    if diff > 0.0 {
                        let s = diff / (cell_size * dist);
                        if s > max_slope { max_slope = s; steepest_nbr = n_idx; }
                    }
                }
            }
            if max_slope <= 1e-4 || steepest_nbr == idx { continue; }

            let beta = max_slope.atan();
            let (sin_b, cos_b) = (beta.sin(), beta.cos());
            let sigma_eff = (gamma * z * cos_b * cos_b - pore_pressure[idx]).max(0.0);
            let resisting = cohesion_kpa + sigma_eff * tan_phi;
            let driving = (gamma * z * sin_b * cos_b).max(1e-4);

            if resisting / driving < 1.0 {
                failed_count += 1;
                let mobilized = z * mobility;
                flux_scratch[idx] -= mobilized;
                flux_scratch[steepest_nbr] += mobilized;
            }
        }
    }
    for i in 0..n { regolith[i] = (regolith[i] + flux_scratch[i]).max(0.0); }
    Ok(failed_count)
}

/// Shallow Ice Approximation (SIA) alpine glacier solver.
///
/// Accumulates ice above ELA, ablates below, diffuses ice via Glen's Flow Law
/// (n=3), and carves bedrock via basal quarrying and abrasion.
/// Returns total ice volume.
pub fn step_glacier_sia_carving(
    bedrock: &mut [f32], ice_thickness: &mut [f32], flux_scratch: &mut [f32],
    width: usize, height: usize, cell_size: f32, dt_years: f32,
    ela_altitude_m: f32, mass_balance_grad: f32, glen_a: f32, quarrying_coeff: f32,
) -> Result<f32, &'static str> {
    let n = validate_grid(width, height, cell_size, &[bedrock.len(), ice_thickness.len(), flux_scratch.len()])?;
    if dt_years <= 0.0 { return Err("Invalid dt_years"); }

    for i in 0..n {
        let balance = (bedrock[i] + ice_thickness[i] - ela_altitude_m) * mass_balance_grad;
        ice_thickness[i] = (ice_thickness[i] + balance * dt_years).max(0.0);
    }

    flux_scratch.fill(0.0);
    let c_glen = glen_a.max(1e-9) * 0.4;
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let h = ice_thickness[idx];
            if h <= 1e-4 { continue; }
            let s_i = bedrock[idx] + h;
            let mut max_slope = 0.0f32;

            for &(dx, dy, dist) in &NEIGHBORS_8 {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    let n_idx = (ny as usize) * width + (nx as usize);
                    let diff = s_i - (bedrock[n_idx] + ice_thickness[n_idx]);
                    if diff > 0.0 {
                        let slope = diff / (cell_size * dist);
                        max_slope = max_slope.max(slope);
                        let h_eff = 0.5 * (h + ice_thickness[n_idx]);
                        let h_pow5 = (h_eff * h_eff * h_eff * h_eff * h_eff).min(1e7);
                        let q = (c_glen * h_pow5 * slope.powi(3) * dt_years).min(h * 0.12);
                        flux_scratch[idx] -= q;
                        flux_scratch[n_idx] += q;
                    }
                }
            }
            if quarrying_coeff > 0.0 && max_slope > 0.0 {
                bedrock[idx] -= (quarrying_coeff * h * max_slope * max_slope * dt_years).min(h * 0.25);
            }
        }
    }

    let (mut total_vol, cell_area) = (0.0f32, cell_size * cell_size);
    for i in 0..n {
        ice_thickness[i] = (ice_thickness[i] + flux_scratch[i]).max(0.0);
        total_vol += ice_thickness[i] * cell_area;
    }
    Ok(total_vol)
}

/// Wind snow transport and steep slope avalanche runout.
///
/// Scours snow from windward crests, deposits in leeward bowls, and avalanches
/// snow when slope exceeds `critical_avalanche_deg`.
/// Returns total snow volume.
pub fn step_snow_wind_and_avalanche(
    bedrock: &[f32], snow_depth: &mut [f32], scratch: &mut [f32],
    width: usize, height: usize, cell_size: f32,
    wind_dir_rad: f32, wind_speed: f32, critical_avalanche_deg: f32,
) -> Result<f32, &'static str> {
    let n = validate_grid(width, height, cell_size, &[bedrock.len(), snow_depth.len(), scratch.len()])?;
    scratch.fill(0.0);
    let (step_x, step_y) = (wind_dir_rad.cos().round() as i32, wind_dir_rad.sin().round() as i32);
    let scour_factor = (wind_speed * 0.015).clamp(0.0, 0.4);

    // Phase 1: Wind scour from crests into leeward bowls
    if scour_factor > 0.0 && (step_x != 0 || step_y != 0) {
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let snow = snow_depth[idx];
                if snow <= 1e-4 { continue; }
                let (nx, ny) = (x as i32 + step_x, y as i32 + step_y);
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    let n_idx = (ny as usize) * width + (nx as usize);
                    if (bedrock[idx] + snow) > (bedrock[n_idx] + snow_depth[n_idx]) {
                        let amount = (snow * scour_factor).min(snow);
                        scratch[idx] -= amount;
                        scratch[n_idx] += amount;
                    }
                }
            }
        }
        for i in 0..n { snow_depth[i] = (snow_depth[i] + scratch[i]).max(0.0); }
    }

    // Phase 2: Avalanche runout
    scratch.fill(0.0);
    let tan_crit = critical_avalanche_deg.to_radians().tan();
    for y in 0..height {
        for x in 0..width {
            let snow = snow_depth[y * width + x];
            if snow > 1e-4 {
                diffuse_steep_mass(x, y, width, height, cell_size, tan_crit, snow * 0.6, bedrock, snow_depth, scratch);
            }
        }
    }

    let (mut total_snow, cell_area) = (0.0f32, cell_size * cell_size);
    for i in 0..n {
        snow_depth[i] = (snow_depth[i] + scratch[i]).max(0.0);
        total_snow += snow_depth[i] * cell_area;
    }
    Ok(total_snow)
}

/// Aeolian sand saltation and slipface avalanche (Werner 1995).
///
/// Models windward saltation hops, leeward shadow-zone deposition,
/// and angle-of-repose slipface avalanches.
pub fn step_aeolian_sand_saltation(
    bedrock: &[f32], sand_depth: &mut [f32], scratch: &mut [f32],
    width: usize, height: usize, cell_size: f32,
    wind_dir_rad: f32, saltation_hop_cells: f32, slipface_deg: f32,
) -> Result<(), &'static str> {
    let n = validate_grid(width, height, cell_size, &[bedrock.len(), sand_depth.len(), scratch.len()])?;
    scratch.fill(0.0);
    let hop = saltation_hop_cells.max(1.0);
    let (hop_x, hop_y) = ((wind_dir_rad.cos() * hop).round() as i32, (wind_dir_rad.sin() * hop).round() as i32);
    let tan_shadow: f32 = 0.2679;

    // Phase 1: Windward saltation & shadow-zone deposition
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let sand = sand_depth[idx];
            if sand <= 1e-4 { continue; }
            let z = bedrock[idx] + sand;
            let (up_x, up_y) = (x as i32 - hop_x, y as i32 - hop_y);
            let in_shadow = up_x >= 0 && up_x < width as i32 && up_y >= 0 && up_y < height as i32
                && ((bedrock[(up_y as usize) * width + (up_x as usize)] + sand_depth[(up_y as usize) * width + (up_x as usize)] - z) / (cell_size * hop)) > tan_shadow;

            if !in_shadow {
                let (down_x, down_y) = (x as i32 + hop_x, y as i32 + hop_y);
                if down_x >= 0 && down_x < width as i32 && down_y >= 0 && down_y < height as i32 {
                    let moved = sand * 0.15;
                    scratch[idx] -= moved;
                    scratch[(down_y as usize) * width + (down_x as usize)] += moved;
                }
            }
        }
    }
    for i in 0..n { sand_depth[i] = (sand_depth[i] + scratch[i]).max(0.0); }

    // Phase 2: Slipface angle-of-repose avalanche
    scratch.fill(0.0);
    let tan_slip = slipface_deg.to_radians().tan();
    for y in 0..height {
        for x in 0..width {
            let sand = sand_depth[y * width + x];
            if sand > 1e-4 {
                diffuse_steep_mass(x, y, width, height, cell_size, tan_slip, sand * 0.5, bedrock, sand_depth, scratch);
            }
        }
    }
    for i in 0..n { sand_depth[i] = (sand_depth[i] + scratch[i]).max(0.0); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thermal_talus_creep_mass_conservation() {
        let (w, h) = (5, 5);
        let (mut b, mut s, mut sc) = ([0.0f32; 25], [0.0f32; 25], [0.0f32; 25]);
        s[12] = 20.0;
        let init_mass = b.iter().sum::<f32>() + s.iter().sum::<f32>();
        let moved = step_thermal_talus_creep(&mut b, &mut s, &mut sc, w, h, 1.0, 33.0, 0.5, 0.05).unwrap();
        assert!(moved > 0.0 && (init_mass - (b.iter().sum::<f32>() + s.iter().sum::<f32>())).abs() < 1e-4);
        assert!(s[12] < 20.0);
    }

    #[test]
    fn test_landslide_debris_runout_high_pore_pressure() {
        let (w, h) = (4, 4);
        let b = [10.0, 8.0, 6.0, 4.0, 10.0, 8.0, 6.0, 4.0, 10.0, 8.0, 6.0, 4.0, 10.0, 8.0, 6.0, 4.0];
        let (mut r, mut pp, mut sc) = ([2.0f32; 16], [0.0f32; 16], [0.0f32; 16]);
        pp[0] = 50.0; pp[4] = 50.0;
        let failed = step_landslide_debris_runout(&b, &mut r, &pp, &mut sc, w, h, 1.0, 5.0, 30.0, 0.5).unwrap();
        assert!(failed >= 2 && r[0] < 2.0);
    }

    #[test]
    fn test_glacier_sia_carving_valley() {
        let (w, h) = (5, 5);
        let (mut b, mut ice, mut sc) = ([2000.0f32; 25], [0.0f32; 25], [0.0f32; 25]);
        ice[12] = 50.0;
        let vol = step_glacier_sia_carving(&mut b, &mut ice, &mut sc, w, h, 10.0, 1.0, 1950.0, 0.01, 1e-3, 0.1).unwrap();
        assert!(vol > 0.0 && b[12] < 2000.0);
    }

    #[test]
    fn test_snow_wind_and_avalanche_transport() {
        let (w, h) = (5, 5);
        let (b, mut sn, mut sc) = ([0.0f32; 25], [0.0f32; 25], [0.0f32; 25]);
        sn[12] = 15.0;
        let vol = step_snow_wind_and_avalanche(&b, &mut sn, &mut sc, w, h, 1.0, 0.0, 10.0, 38.0).unwrap();
        assert!(vol > 0.0 && sn[12] < 15.0);
    }

    #[test]
    fn test_aeolian_sand_saltation_slipface() {
        let (w, h) = (5, 5);
        let (b, mut sd, mut sc) = ([0.0f32; 25], [0.0f32; 25], [0.0f32; 25]);
        sd[12] = 10.0;
        let init_sand = sd.iter().sum::<f32>();
        step_aeolian_sand_saltation(&b, &mut sd, &mut sc, w, h, 1.0, 0.0, 1.0, 34.0).unwrap();
        assert!((init_sand - sd.iter().sum::<f32>()).abs() < 1e-4);
    }
}
