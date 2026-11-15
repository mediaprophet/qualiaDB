// Copyright 2026 QualiaDB Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Quantitative Geomorphometric Realism Verification & Mass-Conservation Auditing.
//!
//! Provides empirical geomorphological validation metrics:
//! - Hypsometric curve (21 elevation quantiles) & Hypsometric Integral (HI)
//! - Horton-Strahler stream order propagation & mean bifurcation ratio ($R_b$)
//! - Flint's channel concavity index ($\theta$) & Hack's law scaling exponent ($h$)
//! - Slope distribution skewness & mass-conservation relative error auditing
//!
//! Allocation Class: Tier-1 `HotZeroHeap` (zero `Vec`, `String`, or `Box` in non-test code).

/// Quantitative geomorphometric report verifying natural realism and mass conservation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeomorphQaReport {
    pub hypsometric_curve_21: [f32; 21],
    pub hypsometric_integral: f32,
    pub drainage_density_m_per_m2: f32,
    pub strahler_max_order: u8,
    pub strahler_bifurcation_ratio: f32,
    pub hacks_law_exponent_h: f32,
    pub channel_concavity_theta: f32,
    pub slope_skewness: f32,
    pub mass_conservation_rel_err: f32,
    pub passes_natural_realism_gate: bool,
}

#[inline]
fn cell_dist(i: usize, j: usize, width: usize, cell_size: f32) -> f32 {
    let (dx, dy) = ((j % width) as isize - (i % width) as isize, (j / width) as isize - (i / width) as isize);
    ((dx * dx + dy * dy) as f32).sqrt() * cell_size
}

#[inline]
fn cell_slope(i: usize, j: usize, dem: &[f32], width: usize, cell_size: f32) -> f64 {
    if j == i || j >= dem.len() { return 0.0; }
    let d = cell_dist(i, j, width, cell_size) as f64;
    if d > 1e-6 { ((dem[i] - dem[j]).max(0.0) as f64) / d } else { 0.0 }
}

#[inline]
fn lin_regress(n: f64, sx: f64, sy: f64, sxx: f64, sxy: f64) -> Option<f64> {
    let denom = n * sxx - sx * sx;
    if n >= 3.0 && denom.abs() > 1e-9 { Some((n * sxy - sx * sy) / denom) } else { None }
}

/// Computes relative area $a/A$ at 21 elevation quantiles $h/H \in \{0.0, 0.05, \dots, 1.0\}$
/// using a 256-bin stack histogram and integrates the Hypsometric Integral $HI \in [0.0, 1.0]$.
pub fn compute_hypsometric_curve(dem: &[f32], out_curve_21: &mut [f32; 21]) -> f32 {
    if dem.is_empty() { out_curve_21.fill(0.0); return 0.0; }
    let (mut z_min, mut z_max) = (f32::INFINITY, f32::NEG_INFINITY);
    for &z in dem {
        if z < z_min { z_min = z; }
        if z > z_max { z_max = z; }
    }
    let relief = z_max - z_min;
    if relief <= 1e-6 {
        out_curve_21.fill(0.0);
        out_curve_21[0] = 1.0;
        return 0.5;
    }

    let inv_relief = 1.0 / relief;
    let mut hist = [0u32; 256];
    for &z in dem {
        let bin = (((z - z_min) * inv_relief).clamp(0.0, 1.0) * 255.0) as usize;
        hist[bin.min(255)] += 1;
    }

    let mut suffix = [0u32; 257];
    for b in (0..256).rev() { suffix[b] = suffix[b + 1] + hist[b]; }

    let total = dem.len() as f32;
    out_curve_21[0] = 1.0;
    out_curve_21[20] = 0.0;

    for k in 1..20 {
        let s = (k as f32 / 20.0) * 255.0;
        let b = (s as usize).min(254);
        let frac = s - (b as f32);
        let count_ge = (1.0 - frac) * (hist[b] as f32) + (suffix[b + 1] as f32);
        out_curve_21[k] = (count_ge / total).clamp(0.0, 1.0);
    }

    let mut hi = 0.0f32;
    for k in 0..20 { hi += 0.5 * (out_curve_21[k] + out_curve_21[k + 1]) * 0.05; }
    hi.clamp(0.0, 1.0)
}

/// Propagates Horton-Strahler stream orders $\omega \in 1..=8$ downstream along `d8_receiver`
/// for cells with `accum_area >= channel_threshold_m2`, counts stream segments per order,
/// and computes mean bifurcation ratio $R_b = N_\omega / N_{\omega+1}$.
pub fn compute_strahler_order_and_bifurcation(
    d8_receiver: &[usize],
    order_desc: &[usize],
    accum_area: &[f32],
    channel_threshold_m2: f32,
    out_strahler: &mut [u8],
) -> Result<(u8, f32), &'static str> {
    let n = d8_receiver.len();
    if n == 0 { return Err("Empty slice in compute_strahler_order_and_bifurcation"); }
    if order_desc.len() != n || accum_area.len() != n || out_strahler.len() != n {
        return Err("Mismatched slice lengths in compute_strahler_order_and_bifurcation");
    }

    out_strahler.fill(0);
    let mut segment_counts = [0u32; 9];

    for &u in order_desc {
        if u >= n { return Err("Index out of bounds in order_desc"); }
        if accum_area[u] < channel_threshold_m2 { continue; }

        let cur = out_strahler[u];
        let (max_in, count_in) = (cur & 0x0F, (cur >> 4) & 0x0F);
        let my_order = if max_in == 0 {
            1u8
        } else if count_in >= 2 {
            (max_in + 1).min(8)
        } else {
            max_in
        };

        if max_in == 0 || count_in >= 2 { segment_counts[my_order as usize] += 1; }

        let v = d8_receiver[u];
        if v != u && v < n {
            let (rec_max, rec_count) = (out_strahler[v] & 0x0F, (out_strahler[v] >> 4) & 0x0F);
            if my_order > rec_max {
                out_strahler[v] = (1 << 4) | (my_order & 0x0F);
            } else if my_order == rec_max {
                out_strahler[v] = (((rec_count + 1).min(15)) << 4) | (rec_max & 0x0F);
            }
        }
        out_strahler[u] = my_order;
    }

    let mut max_order = 0u8;
    for o in 1..=8 {
        if segment_counts[o] > 0 { max_order = o as u8; }
    }

    let bifurcation_ratio = if max_order < 2 {
        0.0f32
    } else {
        let (mut sum_rb, mut count_rb) = (0.0f32, 0.0f32);
        for o in 1..max_order as usize {
            let (n_cur, n_next) = (segment_counts[o] as f32, segment_counts[o + 1] as f32);
            if n_next > 0.0 {
                sum_rb += n_cur / n_next;
                count_rb += 1.0;
            }
        }
        if count_rb > 0.0 { sum_rb / count_rb } else { 0.0 }
    };

    Ok((max_order, bifurcation_ratio))
}

/// Fits log-log slope-area relationship $\log S = -\theta \log A + \log k_s$
/// to extract concavity $\theta$, and Hack's law mainstem scaling $L \propto A^h$.
pub fn compute_hacks_law_and_concavity(
    dem: &[f32],
    d8_receiver: &[usize],
    accum_area: &[f32],
    width: usize,
    height: usize,
    cell_size: f32,
    channel_threshold_m2: f32,
) -> (f32, f32) {
    let n = dem.len();
    if n == 0 || width == 0 || height == 0 || cell_size <= 0.0 { return (0.55, 0.45); }

    let mut max_accum = -1.0f32;
    let mut outlet = 0usize;
    for (i, &a) in accum_area.iter().enumerate() {
        if a > max_accum { max_accum = a; outlet = i; }
    }

    let hacks_h = if max_accum >= channel_threshold_m2 {
        let (mut curr, max_steps) = (outlet, width * height);
        let mut steps = 0usize;
        while steps < max_steps {
            steps += 1;
            let (cx, cy) = (curr % width, curr / width);
            let mut best_upstream = None;
            let mut best_accum = -1.0f32;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 { continue; }
                    let (nx, ny) = (cx as isize + dx, cy as isize + dy);
                    if nx >= 0 && nx < width as isize && ny >= 0 && ny < height as isize {
                        let u = (ny as usize) * width + (nx as usize);
                        if u < n && u != curr && d8_receiver[u] == curr && accum_area[u] >= channel_threshold_m2 {
                            if accum_area[u] > best_accum {
                                best_accum = accum_area[u];
                                best_upstream = Some(u);
                            }
                        }
                    }
                }
            }
            match best_upstream {
                Some(u) => curr = u,
                None => break,
            }
        }

        let mut current_l = cell_size as f64;
        let (mut n_m, mut sla, mut sll, mut slala, mut slall) = (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
        let mut walk_steps = 0usize;
        while walk_steps < max_steps {
            walk_steps += 1;
            let a = accum_area[curr] as f64;
            if a >= channel_threshold_m2 as f64 && current_l > 0.0 {
                let (x, y) = (a.ln(), current_l.ln());
                n_m += 1.0; sla += x; sll += y; slala += x * x; slall += x * y;
            }
            let nxt = d8_receiver[curr];
            if nxt == curr || nxt >= n { break; }
            current_l += cell_dist(curr, nxt, width, cell_size) as f64;
            curr = nxt;
        }
        lin_regress(n_m, sla, sll, slala, slall).map_or(0.55, |s| (s as f32).clamp(0.3, 0.9))
    } else {
        0.55
    };

    let (mut n_c, mut ssa, mut sss, mut ssasa, mut ssass) = (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for i in 0..n {
        let a = accum_area[i] as f64;
        if a < channel_threshold_m2 as f64 { continue; }
        let slope = cell_slope(i, d8_receiver[i], dem, width, cell_size);
        if slope >= 1e-6 {
            let (x, y) = (a.ln(), slope.ln());
            n_c += 1.0; ssa += x; sss += y; ssasa += x * x; ssass += x * y;
        }
    }
    let concavity_theta = lin_regress(n_c, ssa, sss, ssasa, ssass).map_or(0.45, |s| (-s as f32).clamp(0.1, 1.2));

    (hacks_h, concavity_theta)
}

/// Assembles the full `GeomorphQaReport` and evaluates realism & conservation gates.
pub fn audit_geomorph_realism(
    dem: &[f32],
    d8_receiver: &[usize],
    order_desc: &[usize],
    accum_area: &[f32],
    strahler_scratch: &mut [u8],
    width: usize,
    height: usize,
    cell_size: f32,
    channel_threshold_m2: f32,
    mass_before: f32,
    mass_after: f32,
) -> Result<GeomorphQaReport, &'static str> {
    let n = dem.len();
    if n == 0 || width == 0 || height == 0 || cell_size <= 0.0 {
        return Err("Invalid grid dimensions or cell size");
    }
    if width.checked_mul(height) != Some(n) {
        return Err("DEM length does not match width * height");
    }
    if d8_receiver.len() != n || order_desc.len() != n || accum_area.len() != n || strahler_scratch.len() != n {
        return Err("Mismatched slice lengths in audit_geomorph_realism");
    }

    let mut hypsometric_curve_21 = [0.0f32; 21];
    let hypsometric_integral = compute_hypsometric_curve(dem, &mut hypsometric_curve_21);

    let (strahler_max_order, strahler_bifurcation_ratio) = compute_strahler_order_and_bifurcation(
        d8_receiver,
        order_desc,
        accum_area,
        channel_threshold_m2,
        strahler_scratch,
    )?;

    let total_area_m2 = (n as f32) * cell_size * cell_size;
    let mut total_channel_len = 0.0f32;
    for i in 0..n {
        if strahler_scratch[i] > 0 {
            let j = d8_receiver[i];
            total_channel_len += if j != i && j < n {
                cell_dist(i, j, width, cell_size)
            } else {
                cell_size
            };
        }
    }
    let drainage_density_m_per_m2 = if total_area_m2 > 1e-6 {
        total_channel_len / total_area_m2
    } else {
        0.0
    };

    let (hacks_law_exponent_h, channel_concavity_theta) = compute_hacks_law_and_concavity(
        dem,
        d8_receiver,
        accum_area,
        width,
        height,
        cell_size,
        channel_threshold_m2,
    );

    let mut sum_slope = 0.0f64;
    for i in 0..n { sum_slope += cell_slope(i, d8_receiver[i], dem, width, cell_size); }
    let mean_slope = sum_slope / (n as f64);

    let (mut m2, mut m3) = (0.0f64, 0.0f64);
    for i in 0..n {
        let diff = cell_slope(i, d8_receiver[i], dem, width, cell_size) - mean_slope;
        m2 += diff * diff;
        m3 += diff * diff * diff;
    }
    let std = (m2 / (n as f64)).sqrt();
    let slope_skewness = if std > 1e-6 { ((m3 / (n as f64)) / (std * std * std)) as f32 } else { 0.0 };

    let mass_conservation_rel_err = (mass_after - mass_before).abs() / mass_before.abs().max(1e-7);
    let passes_natural_realism_gate = (0.15..=0.85).contains(&hypsometric_integral)
        && mass_conservation_rel_err < 1e-4;

    Ok(GeomorphQaReport {
        hypsometric_curve_21,
        hypsometric_integral,
        drainage_density_m_per_m2,
        strahler_max_order,
        strahler_bifurcation_ratio,
        hacks_law_exponent_h,
        channel_concavity_theta,
        slope_skewness,
        mass_conservation_rel_err,
        passes_natural_realism_gate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hypsometric_curve_stages() {
        let mut curve = [0.0f32; 21];

        // 1. Linear slope DEM
        let mut linear_dem = [0.0f32; 100];
        for i in 0..100 { linear_dem[i] = i as f32; }
        let hi_linear = compute_hypsometric_curve(&linear_dem, &mut curve);
        assert!((hi_linear - 0.50).abs() < 0.05, "Linear HI ~0.50, got {}", hi_linear);
        assert_eq!(curve[0], 1.0);
        assert_eq!(curve[20], 0.0);

        // 2. Convex dome DEM (youthful stage, HI > 0.58)
        let mut dome_dem = [0.0f32; 100];
        for i in 0..100 {
            let t = i as f32 / 99.0;
            dome_dem[i] = 100.0 * (1.0 - (t - 0.5) * (t - 0.5) * 4.0).max(0.0).sqrt();
        }
        let hi_dome = compute_hypsometric_curve(&dome_dem, &mut curve);
        assert!(hi_dome > 0.58, "Convex dome HI should be elevated (> 0.58), got {}", hi_dome);

        // 3. Concave basin DEM (peneplain stage, HI < 0.35)
        let mut basin_dem = [0.0f32; 100];
        for i in 0..100 {
            let t = i as f32 / 99.0;
            basin_dem[i] = 100.0 * (t * t * t * t);
        }
        let hi_basin = compute_hypsometric_curve(&basin_dem, &mut curve);
        assert!(hi_basin < 0.35, "Concave basin HI should be low (< 0.35), got {}", hi_basin);
    }

    #[test]
    fn test_compute_strahler_order_and_bifurcation_y_junction() {
        let mut d8 = [0usize; 25];
        for i in 0..25 { d8[i] = i; }
        let mut accum = [10.0f32; 25];
        let thresh = 50.0f32;

        // Dendritic Y-junction:
        // Left arm: A(1) -> B(6) -> J(12); Right arm: C(3) -> D(8) -> J(12); Main: J(12) -> K(17) -> Out(22)
        d8[1] = 6; d8[6] = 12; d8[3] = 8; d8[8] = 12; d8[12] = 17; d8[17] = 22; d8[22] = 22;
        accum[1] = 50.0; accum[6] = 100.0; accum[3] = 50.0; accum[8] = 100.0;
        accum[12] = 250.0; accum[17] = 300.0; accum[22] = 350.0;

        let mut order_desc = [0usize; 25];
        order_desc[0..7].copy_from_slice(&[1, 3, 6, 8, 12, 17, 22]);
        let mut idx = 7;
        for i in 0..25 {
            if !order_desc[0..7].contains(&i) {
                order_desc[idx] = i;
                idx += 1;
            }
        }

        let mut strahler = [0u8; 25];
        let (max_order, rb) = compute_strahler_order_and_bifurcation(
            &d8, &order_desc, &accum, thresh, &mut strahler,
        ).expect("Strahler computation failed");

        assert_eq!(max_order, 2, "Strahler order should increment 1 + 1 -> 2");
        assert!((rb - 2.0).abs() < 1e-4, "Bifurcation ratio N1/N2 = 2/1 = 2.0, got {}", rb);
        for &k in &[1, 6, 3, 8] { assert_eq!(strahler[k], 1); }
        for &k in &[12, 17, 22] { assert_eq!(strahler[k], 2); }
    }

    #[test]
    fn test_audit_geomorph_realism_fluvial_valley() {
        let (width, height) = (16, 16);
        let n = width * height;
        let (cell_size, channel_thresh) = (10.0f32, 200.0f32);

        let mut dem = [0.0f32; 256];
        let mut d8 = [0usize; 256];
        let mut accum = [0.0f32; 256];
        let mut scratch = [0u8; 256];
        let mut order_desc = [0usize; 256];

        let mid_x = 8;
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let dist_to_mid = (x as isize - mid_x as isize).abs() as f32;
                dem[idx] = 200.0 - (y as f32) * 8.0 + dist_to_mid * 5.0;

                d8[idx] = if y == height - 1 && x == mid_x {
                    idx
                } else if x < mid_x {
                    (y + 1).min(height - 1) * width + (x + 1)
                } else if x > mid_x {
                    (y + 1).min(height - 1) * width + (x - 1)
                } else {
                    (y + 1).min(height - 1) * width + mid_x
                };
                accum[idx] = ((y + 1) as f32) * 50.0 / (dist_to_mid + 1.0);
            }
        }

        for i in 0..n { order_desc[i] = i; }
        order_desc.sort_by(|&a, &b| dem[b].partial_cmp(&dem[a]).unwrap());

        let (mass_before, mass_after) = (50000.0f32, 50000.005f32);
        let report = audit_geomorph_realism(
            &dem, &d8, &order_desc, &accum, &mut scratch,
            width, height, cell_size, channel_thresh,
            mass_before, mass_after,
        ).expect("Audit should succeed");

        assert!(report.passes_natural_realism_gate);
        assert!((0.15..=0.85).contains(&report.hypsometric_integral));
        assert!(report.strahler_max_order >= 1);
        assert!(report.mass_conservation_rel_err < 1e-4);

        let report_bad_mass = audit_geomorph_realism(
            &dem, &d8, &order_desc, &accum, &mut scratch,
            width, height, cell_size, channel_thresh,
            mass_before, 50100.0,
        ).expect("Audit should succeed");
        assert!(!report_bad_mass.passes_natural_realism_gate);
    }
}
