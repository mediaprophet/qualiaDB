// Copyright 2026 QualiaDB Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Hydrological flow routing, depression remediation, FastScape stream power
//! erosion, and coastal wave-cut cliff dynamics.
//!
//! Caller-buffered, zero-heap (`HotZeroHeap`) geomorphological operators:
//! - Priority-flood depression filling & least-cost breach carving.
//! - D8 steepest-descent and Tarboton (1997) D-Infinity (`Dinf`) flow routing.
//! - Implicit FastScape $O(N)$ Stream Power Law bedrock incision (Braun & Willett 2013).
//! - Coastal wave-cut notch abrasion, marine terraces, and cliff collapse.

use core::f32::consts::{FRAC_PI_4, PI, SQRT_2};

// ---------------------------------------------------------------------------
// 1. Depression Filling & Breach Carving
// ---------------------------------------------------------------------------

#[inline]
fn check_and_raise_pit_cell(
    dem: &mut [f32], x: usize, y: usize, width: usize, height: usize, eps: f32,
) -> bool {
    let (idx, h) = (y * width + x, dem[y * width + x]);
    let (mut min_higher, mut can_drain) = (f32::INFINITY, false);

    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 { continue; }
            let (nx, ny) = (x as isize + dx, y as isize + dy);
            let nh = dem[ny as usize * width + nx as usize];
            if nh < h {
                can_drain = true;
            } else if nh == h {
                if nx == 0 || nx as usize == width - 1 || ny == 0 || ny as usize == height - 1 {
                    can_drain = true;
                } else {
                    for (ddx, ddy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        if dem[(ny + ddy) as usize * width + (nx + ddx) as usize] < h {
                            can_drain = true;
                            break;
                        }
                    }
                }
            } else if nh < min_higher {
                min_higher = nh;
            }
        }
    }

    if !can_drain && min_higher.is_finite() && min_higher + eps > h {
        dem[idx] = min_higher + eps;
        true
    } else {
        false
    }
}

/// Eliminates spurious pits by raising depression cells until all interior cells
/// have a monotonic or strictly draining path to the DEM boundary.
pub fn priority_flood_fill_depressions(
    dem: &mut [f32], width: usize, height: usize, epsilon: f32,
) -> Result<usize, &'static str> {
    if width < 3 || height < 3 { return Ok(0); }
    if dem.len() < width * height { return Err("DEM buffer smaller than width * height"); }
    let (eps, mut total_raised) = (if epsilon > 0.0 { epsilon } else { 1e-4 }, 0);

    for _ in 0..(width + height) * 2 {
        let mut changed = false;
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                if check_and_raise_pit_cell(dem, x, y, width, height, eps) {
                    total_raised += 1;
                    changed = true;
                }
            }
        }
        for y in (1..height - 1).rev() {
            for x in (1..width - 1).rev() {
                if check_and_raise_pit_cell(dem, x, y, width, height, eps) {
                    total_raised += 1;
                    changed = true;
                }
            }
        }
        if !changed { break; }
    }
    Ok(total_raised)
}

/// Carves outlet channels from local sinks to lower neighbors within `max_breach_len`.
pub fn breach_depressions_least_cost(
    dem: &mut [f32], width: usize, height: usize, max_breach_len: usize,
) -> Result<usize, &'static str> {
    if width < 3 || height < 3 { return Ok(0); }
    if dem.len() < width * height { return Err("DEM buffer smaller than width * height"); }
    let (r, mut carved_cells) = (max_breach_len.max(1), 0);

    for sy in 1..height - 1 {
        for sx in 1..width - 1 {
            let sh = dem[sy * width + sx];
            let mut is_sink = true;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if (dx != 0 || dy != 0)
                        && dem[(sy as isize + dy) as usize * width + (sx as isize + dx) as usize] < sh
                    {
                        is_sink = false;
                        break;
                    }
                }
                if !is_sink { break; }
            }
            if !is_sink { continue; }

            let (mut best_target, mut min_cost) = (None, f32::INFINITY);
            let (min_x, max_x) = (sx.saturating_sub(r), (sx + r).min(width - 1));
            let (min_y, max_y) = (sy.saturating_sub(r), (sy + r).min(height - 1));

            for ty in min_y..=max_y {
                for tx in min_x..=max_x {
                    let th = dem[ty * width + tx];
                    if th >= sh { continue; }
                    let steps = (sx as isize - tx as isize).abs().max((sy as isize - ty as isize).abs()) as usize;
                    if steps == 0 { continue; }
                    let mut cost = steps as f32 * 0.05;
                    for s in 1..=steps {
                        let t = s as f32 / steps as f32;
                        let ix = ((sx as f32) + t * (tx as f32 - sx as f32)).round() as usize;
                        let iy = ((sy as f32) + t * (ty as f32 - sy as f32)).round() as usize;
                        let h_cur = dem[iy * width + ix];
                        let h_line = sh + t * (th - sh);
                        if h_cur > h_line { cost += h_cur - h_line; }
                    }
                    if cost < min_cost { min_cost = cost; best_target = Some((tx, ty, steps)); }
                }
            }

            if let Some((tx, ty, steps)) = best_target {
                let th = dem[ty * width + tx];
                for s in 0..steps {
                    let t = s as f32 / steps as f32;
                    let ix = ((sx as f32) + t * (tx as f32 - sx as f32)).round() as usize;
                    let iy = ((sy as f32) + t * (ty as f32 - sy as f32)).round() as usize;
                    let h_line = sh + t * (th - sh);
                    let c_idx = iy * width + ix;
                    if dem[c_idx] > h_line { dem[c_idx] = h_line; carved_cells += 1; }
                }
            }
        }
    }
    Ok(carved_cells)
}

// ---------------------------------------------------------------------------
// 2. D8 & Tarboton (1997) D-Infinity Flow Routing
// ---------------------------------------------------------------------------

/// Computes D8 steepest-descent flow direction receivers and accumulated drainage area.
pub fn compute_d8_flow(
    dem: &[f32], width: usize, height: usize, cell_size: f32,
    order_scratch: &mut [usize], out_receiver: &mut [usize], out_accum_area: &mut [f32],
) -> Result<(), &'static str> {
    let n = width * height;
    if dem.len() < n || order_scratch.len() < n || out_receiver.len() < n || out_accum_area.len() < n {
        return Err("Buffer capacity smaller than width * height");
    }
    if cell_size <= 0.0 { return Err("cell_size must be positive"); }

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            if x == 0 || x == width - 1 || y == 0 || y == height - 1 {
                out_receiver[idx] = idx;
                continue;
            }
            let (h, mut max_slope, mut best_rec) = (dem[idx], 0.0f32, idx);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 { continue; }
                    let n_idx = (y as isize + dy) as usize * width + (x as isize + dx) as usize;
                    let dist = if dx != 0 && dy != 0 { cell_size * SQRT_2 } else { cell_size };
                    let slope = (h - dem[n_idx]) / dist;
                    if slope > max_slope {
                        max_slope = slope;
                        best_rec = n_idx;
                    }
                }
            }
            out_receiver[idx] = best_rec;
        }
    }

    let cell_area = cell_size * cell_size;
    for i in 0..n {
        out_accum_area[i] = cell_area;
        order_scratch[i] = i;
    }
    order_scratch[..n].sort_unstable_by(|&a, &b| dem[b].total_cmp(&dem[a]));

    for &idx in &order_scratch[..n] {
        let rec = out_receiver[idx];
        if rec != idx { out_accum_area[rec] += out_accum_area[idx]; }
    }
    Ok(())
}

/// Computes Tarboton (1997) D-Infinity flow direction angles and accumulates flow area.
pub fn compute_dinf_flow_accumulation(
    dem: &[f32], width: usize, height: usize, cell_size: f32,
    order_scratch: &mut [usize], out_flow_angle_rad: &mut [f32], out_accum_area: &mut [f32],
) -> Result<(), &'static str> {
    let n = width * height;
    if dem.len() < n || order_scratch.len() < n || out_flow_angle_rad.len() < n || out_accum_area.len() < n {
        return Err("Buffer capacity smaller than width * height");
    }
    if cell_size <= 0.0 { return Err("cell_size must be positive"); }

    const FACETS: [((isize, isize), (isize, isize), f32, f32); 8] = [
        ((1, 0), (1, -1), 0.0, 1.0), ((0, -1), (1, -1), PI * 0.5, -1.0),
        ((0, -1), (-1, -1), PI * 0.5, 1.0), ((-1, 0), (-1, -1), PI, -1.0),
        ((-1, 0), (-1, 1), PI, 1.0), ((0, 1), (-1, 1), PI * 1.5, -1.0),
        ((0, 1), (1, 1), PI * 1.5, 1.0), ((1, 0), (1, 1), 2.0 * PI, -1.0),
    ];

    let cell_area = cell_size * cell_size;
    for i in 0..n {
        out_accum_area[i] = cell_area;
        order_scratch[i] = i;
        out_flow_angle_rad[i] = 0.0;
    }
    order_scratch[..n].sort_unstable_by(|&a, &b| dem[b].total_cmp(&dem[a]));

    for &idx in &order_scratch[..n] {
        let (x, y) = (idx % width, idx / width);
        if x == 0 || x == width - 1 || y == 0 || y == height - 1 { continue; }
        let (e0, mut max_slope, mut best_angle) = (dem[idx], 0.0f32, 0.0f32);
        let (mut best_n1, mut best_n2, mut best_p1, mut best_p2) = (idx, idx, 1.0f32, 0.0f32);

        for &((c_dx, c_dy), (d_dx, d_dy), base_angle, sign) in &FACETS {
            let n1 = (y as isize + c_dy) as usize * width + (x as isize + c_dx) as usize;
            let n2 = (y as isize + d_dy) as usize * width + (x as isize + d_dx) as usize;
            let (e1, e2) = (dem[n1], dem[n2]);
            let (s1, s2) = ((e0 - e1) / cell_size, (e1 - e2) / cell_size);
            let mut r = s2.atan2(s1);
            let s = if s1 <= 0.0 && s2 <= 0.0 {
                0.0
            } else if r < 0.0 {
                r = 0.0;
                s1
            } else if r > FRAC_PI_4 {
                r = FRAC_PI_4;
                (e0 - e2) / (cell_size * SQRT_2)
            } else {
                (s1 * s1 + s2 * s2).sqrt()
            };

            if s > max_slope {
                max_slope = s;
                best_angle = (base_angle + sign * r).rem_euclid(2.0 * PI);
                best_n1 = n1;
                best_n2 = n2;
                best_p2 = r / FRAC_PI_4;
                best_p1 = 1.0 - best_p2;
            }
        }

        if max_slope > 0.0 {
            out_flow_angle_rad[idx] = best_angle;
            let flow = out_accum_area[idx];
            out_accum_area[best_n1] += best_p1 * flow;
            out_accum_area[best_n2] += best_p2 * flow;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 3. Implicit $O(N)$ FastScape Stream Power Law Solver
// ---------------------------------------------------------------------------

/// Solves the Stream Power Law $\partial h/\partial t = U - K A^m S^n$ unconditionally
/// stably in $O(N)$ using FastScape implicit backwards-Euler routing.
pub fn step_fastscape_stream_power(
    dem: &mut [f32], width: usize, height: usize, cell_size: f32, dt: f32,
    uplift_rate: f32, k_erodibility: f32, m_exp: f32, n_exp: f32,
    order_scratch: &mut [usize], receiver_scratch: &mut [usize], accum_scratch: &mut [f32],
) -> Result<f32, &'static str> {
    let n = width * height;
    if dem.len() < n || order_scratch.len() < n || receiver_scratch.len() < n || accum_scratch.len() < n {
        return Err("Buffer capacity smaller than width * height");
    }
    if dt <= 0.0 || cell_size <= 0.0 { return Err("dt and cell_size must be positive"); }

    compute_d8_flow(dem, width, height, cell_size, order_scratch, receiver_scratch, accum_scratch)?;

    let mut max_change = 0.0f32;
    for k in (0..n).rev() {
        let i = order_scratch[k];
        let (x, y) = (i % width, i / width);
        if x == 0 || x == width - 1 || y == 0 || y == height - 1 { continue; }
        let rec = receiver_scratch[i];
        if rec == i {
            let new_h = dem[i] + uplift_rate * dt;
            let chg = (new_h - dem[i]).abs();
            dem[i] = new_h;
            if chg > max_change { max_change = chg; }
            continue;
        }

        let (rx, ry) = (rec % width, rec / width);
        let (dx, dy) = ((x as isize - rx as isize).abs(), (y as isize - ry as isize).abs());
        let dist = if dx != 0 && dy != 0 { cell_size * SQRT_2 } else { cell_size };
        let c = (k_erodibility * dt * accum_scratch[i].powf(m_exp)) / dist.powf(n_exp);
        let (h_rec, h0) = (dem[rec], dem[i] + uplift_rate * dt);

        let h_new = if (n_exp - 1.0).abs() < 1e-4 {
            (h0 + c * h_rec) / (1.0 + c)
        } else {
            let mut h = h0.max(h_rec + 1e-4);
            for _ in 0..8 {
                let diff = (h - h_rec).max(0.0);
                let f = h - h0 + c * diff.powf(n_exp);
                let df = 1.0 + c * n_exp * diff.powf(n_exp - 1.0);
                let step = f / df;
                h -= step;
                if step.abs() < 1e-4 { break; }
            }
            h.max(h_rec)
        };

        let chg = (h_new - dem[i]).abs();
        dem[i] = h_new;
        if chg > max_change { max_change = chg; }
    }
    Ok(max_change)
}

// ---------------------------------------------------------------------------
// 4. Coastal Wave Cliff Notch & Marine Terrace Formation
// ---------------------------------------------------------------------------

/// Models wave-cut notch abrasion in `[sea_level - tidal_range, sea_level + tidal_range]`,
/// marine abrasion platform formation, and subaerial sea-cliff collapse.
pub fn apply_coastal_wave_cliff_terrace(
    dem: &mut [f32], width: usize, height: usize, sea_level: f32,
    tidal_range: f32, wave_energy: f32, cliff_repose_slope: f32, cell_size: f32,
) -> Result<usize, &'static str> {
    if width < 3 || height < 3 || dem.len() < width * height {
        return Err("Invalid DEM dimensions or buffer length");
    }
    if cell_size <= 0.0 { return Err("cell_size must be positive"); }
    let (z_min, z_max, mut modified_count) = (sea_level - tidal_range, sea_level + tidal_range, 0);

    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let (idx, h) = (y * width + x, dem[y * width + x]);
            if h >= z_min && h <= z_max {
                let mut has_submerged = false;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if (dx != 0 || dy != 0)
                            && dem[(y as isize + dy) as usize * width + (x as isize + dx) as usize] < sea_level
                        {
                            has_submerged = true; break;
                        }
                    }
                    if has_submerged { break; }
                }
                if has_submerged {
                    let wear = wave_energy.min(h - z_min);
                    if wear > 0.0 { dem[idx] -= wear; modified_count += 1; }
                }
            }
        }
    }

    let repose = cliff_repose_slope.max(0.1);
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let base_h = dem[y * width + x];
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 { continue; }
                    let n_idx = (y as isize + dy) as usize * width + (x as isize + dx) as usize;
                    let dist = if dx != 0 && dy != 0 { cell_size * SQRT_2 } else { cell_size };
                    let max_h = base_h + repose * dist;
                    if dem[n_idx] > max_h { dem[n_idx] = max_h; modified_count += 1; }
                }
            }
        }
    }
    Ok(modified_count)
}

// ---------------------------------------------------------------------------
// 5. Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depression_fill_and_breach() {
        #[rustfmt::skip]
        let mut dem = [
            10.0, 10.0, 10.0, 10.0, 10.0, 10.0,  6.0,  6.0,  6.0, 10.0,
            10.0,  6.0,  2.0,  6.0, 10.0, 10.0,  6.0,  6.0,  6.0, 10.0,
            10.0, 10.0, 10.0, 10.0, 10.0,
        ];
        let raised = priority_flood_fill_depressions(&mut dem, 5, 5, 0.01).unwrap();
        assert!(raised > 0 && dem[12] >= 6.0);

        #[rustfmt::skip]
        let mut dem_breach = [
            5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 1.0, 8.0, 0.0, 5.0,
            5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0,
            5.0, 5.0, 5.0, 5.0, 5.0,
        ];
        let carved = breach_depressions_least_cost(&mut dem_breach, 5, 5, 3).unwrap();
        assert!(carved > 0 && dem_breach[7] < 8.0);
    }

    #[test]
    fn test_d8_and_dinf_flow_v_valley() {
        #[rustfmt::skip]
        let dem = [
            10.0, 10.0, 10.0, 10.0, 10.0,  5.0,  5.0,  5.0,  5.0,  5.0,
             2.0,  1.5,  1.0,  0.5,  0.0,  5.0,  5.0,  5.0,  5.0,  5.0,
            10.0, 10.0, 10.0, 10.0, 10.0,
        ];
        let (mut order, mut receiver, mut accum_d8) = ([0usize; 25], [0usize; 25], [0.0f32; 25]);
        let (mut angles, mut accum_dinf) = ([0.0f32; 25], [0.0f32; 25]);

        compute_d8_flow(&dem, 5, 5, 10.0, &mut order, &mut receiver, &mut accum_d8).unwrap();
        assert!(accum_d8[14] > accum_d8[0]);

        compute_dinf_flow_accumulation(&dem, 5, 5, 10.0, &mut order, &mut angles, &mut accum_dinf).unwrap();
        assert!(accum_dinf[14] > accum_dinf[0]);
    }

    #[test]
    fn test_fastscape_stability_and_coastal_erosion() {
        #[rustfmt::skip]
        let mut dem = [
            10.0, 10.0, 10.0, 10.0, 10.0,  8.0,  8.0,  8.0,  8.0,  8.0,
             6.0,  6.0,  6.0,  6.0,  6.0,  4.0,  4.0,  4.0,  4.0,  4.0,
             0.0,  0.0,  0.0,  0.0,  0.0,
        ];
        let (mut order, mut receiver, mut accum) = ([0usize; 25], [0usize; 25], [0.0f32; 25]);
        let max_chg = step_fastscape_stream_power(
            &mut dem, 5, 5, 10.0, 1000.0, 0.001, 1e-4, 0.5, 1.0,
            &mut order, &mut receiver, &mut accum,
        ).unwrap();
        assert!(max_chg >= 0.0);
        for &h in &dem { assert!(!h.is_nan() && !h.is_infinite()); }

        #[rustfmt::skip]
        let mut coastal_dem = [
            0.0,  0.0,  0.0,  0.0,  0.0,  0.0,  5.5, 20.0, 20.0, 20.0,
            0.0,  5.5, 20.0, 20.0, 20.0,  0.0,  5.5, 20.0, 20.0, 20.0,
            0.0,  0.0,  0.0,  0.0,  0.0,
        ];
        let modified = apply_coastal_wave_cliff_terrace(
            &mut coastal_dem, 5, 5, 5.0, 1.0, 2.0, 1.0, 10.0,
        ).unwrap();
        assert!(modified > 0 && coastal_dem[6] < 5.5 && coastal_dem[7] < 20.0);
    }
}
