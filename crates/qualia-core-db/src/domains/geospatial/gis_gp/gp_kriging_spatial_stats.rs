// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) RustGIS contributors (adapted under MIT License)
//! Geostatistical Kriging, Variogram Analysis, Spatial Autocorrelation,
//! Ripley's K/L Point Patterns, and Geographically Weighted Regression (GWR).
//!
//! Allocation Class:
//! - Tier-1 (`HotZeroHeap`): `gp_empirical_variogram`, `gp_fit_variogram_wls`,
//!   `gp_idw_interpolate_into`, `gp_global_morans_i_and_gearys_c`,
//!   `gp_local_lisa_and_getis_ord_gi_star`, `gp_ripleys_k_and_l_into`.
//! - Tier-2 (`ColdBounded`): `gp_ordinary_kriging_grid`,
//!   `gp_natural_neighbor_sibson`, `gp_gwr_local_regression`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpStatsError {
    EmptyDataset, InsufficientSamples, InvalidParameter, SingularMatrix, BufferTooSmall,
    ByteBudgetExceeded { requested: usize, budget: usize },
}

impl core::fmt::Display for GpStatsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyDataset => write!(f, "Empty dataset provided"),
            Self::InsufficientSamples => write!(f, "Insufficient samples for operation"),
            Self::InvalidParameter => write!(f, "Invalid parameter specified"),
            Self::SingularMatrix => write!(f, "Singular matrix in Kriging solve"),
            Self::BufferTooSmall => write!(f, "Supplied buffer too small"),
            Self::ByteBudgetExceeded { requested, budget } => write!(f, "Budget exceeded: req {requested} B, budget {budget} B"),
        }
    }
}
impl std::error::Error for GpStatsError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VariogramModel {
    Spherical { nugget: f64, sill: f64, range: f64 },
    Exponential { nugget: f64, sill: f64, range: f64 },
    Gaussian { nugget: f64, sill: f64, range: f64 },
}

impl VariogramModel {
    #[inline]
    pub fn semivariance(&self, h: f64) -> f64 {
        if h <= 0.0 { return 0.0; }
        match *self {
            Self::Spherical { nugget, sill, range } => {
                let a = range.max(1e-9);
                if h <= a { let hr = h / a; nugget + sill * (1.5 * hr - 0.5 * hr * hr * hr) } else { nugget + sill }
            }
            Self::Exponential { nugget, sill, range } => nugget + sill * (1.0 - (-3.0 * h / range.max(1e-9)).exp()),
            Self::Gaussian { nugget, sill, range } => nugget + sill * (1.0 - (-3.0 * (h / range.max(1e-9)).powi(2)).exp()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmpiricalVariogramBin { pub lag_center: f64, pub semivariance: f64, pub pair_count: usize }

#[inline]
fn model_curve(kind: u8, h: f64, a: f64) -> f64 {
    match kind {
        0 => if h <= a { let hr = h / a; 1.5 * hr - 0.5 * hr * hr * hr } else { 1.0 },
        1 => 1.0 - (-3.0 * h / a).exp(),
        _ => 1.0 - (-3.0 * (h / a).powi(2)).exp(),
    }
}

pub fn gp_empirical_variogram(
    samples: &[[f64; 3]], num_lags: usize, max_distance: f64, out_bins: &mut [EmpiricalVariogramBin],
) -> Result<usize, GpStatsError> {
    if samples.len() < 2 { return Err(GpStatsError::InsufficientSamples); }
    if num_lags == 0 || max_distance <= 0.0 { return Err(GpStatsError::InvalidParameter); }
    if out_bins.len() < num_lags { return Err(GpStatsError::BufferTooSmall); }
    let lag_width = max_distance / num_lags as f64;
    for (k, bin) in out_bins[..num_lags].iter_mut().enumerate() {
        *bin = EmpiricalVariogramBin { lag_center: (k as f64 + 0.5) * lag_width, semivariance: 0.0, pair_count: 0 };
    }
    for i in 0..samples.len() {
        for j in (i + 1)..samples.len() {
            let d = (samples[i][0] - samples[j][0]).hypot(samples[i][1] - samples[j][1]);
            let bin_idx = (d / lag_width) as usize;
            if d < max_distance && bin_idx < num_lags {
                out_bins[bin_idx].semivariance += (samples[i][2] - samples[j][2]).powi(2);
                out_bins[bin_idx].pair_count += 1;
            }
        }
    }
    for bin in out_bins[..num_lags].iter_mut() {
        if bin.pair_count > 0 { bin.semivariance /= 2.0 * bin.pair_count as f64; }
    }
    Ok(num_lags)
}

pub fn gp_fit_variogram_wls(bins: &[EmpiricalVariogramBin], model_kind: u8) -> Result<VariogramModel, GpStatsError> {
    if model_kind > 2 { return Err(GpStatsError::InvalidParameter); }
    let (mut valid, mut min_h, mut max_h) = (0usize, f64::INFINITY, 0.0f64);
    for b in bins.iter().filter(|b| b.pair_count > 0 && b.lag_center > 0.0) {
        valid += 1; min_h = min_h.min(b.lag_center); max_h = max_h.max(b.lag_center);
    }
    if valid < 2 { return Err(GpStatsError::InsufficientSamples); }
    let (mut best_rss, mut best_c0, mut best_c, mut best_a) = (f64::INFINITY, 0.0, 1.0, max_h);
    for step in 1..=40 {
        let a = min_h + (max_h * 1.5 - min_h) * (step as f64 / 40.0);
        let (mut sw, mut swx, mut swy, mut swxx, mut swxy) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for b in bins.iter().filter(|b| b.pair_count > 0 && b.lag_center > 0.0) {
            let x = model_curve(model_kind, b.lag_center, a);
            let w = b.pair_count as f64 / (b.semivariance * b.semivariance).max(1e-4);
            sw += w; swx += w * x; swy += w * b.semivariance; swxx += w * x * x; swxy += w * x * b.semivariance;
        }
        let det = sw * swxx - swx * swx;
        let (c0, c) = if det.abs() > 1e-12 {
            (((swxx * swy - swx * swxy) / det).max(0.0), ((sw * swxy - swx * swy) / det).max(1e-6))
        } else {
            (0.0, (swy / swx.max(1e-6)).max(1e-6))
        };
        let mut rss = 0.0f64;
        for b in bins.iter().filter(|b| b.pair_count > 0 && b.lag_center > 0.0) {
            let diff = b.semivariance - (c0 + c * model_curve(model_kind, b.lag_center, a));
            let w = b.pair_count as f64 / (b.semivariance * b.semivariance).max(1e-4);
            rss += w * diff * diff;
        }
        if rss < best_rss { best_rss = rss; best_c0 = c0; best_c = c; best_a = a; }
    }
    match model_kind {
        0 => Ok(VariogramModel::Spherical { nugget: best_c0, sill: best_c, range: best_a }),
        1 => Ok(VariogramModel::Exponential { nugget: best_c0, sill: best_c, range: best_a }),
        _ => Ok(VariogramModel::Gaussian { nugget: best_c0, sill: best_c, range: best_a }),
    }
}

pub fn gp_ordinary_kriging_grid(
    samples: &[[f64; 3]], model: &VariogramModel, bbox: [f64; 4],
    width: usize, height: usize, byte_budget: usize,
) -> Result<(Vec<f64>, Vec<f64>), GpStatsError> {
    let n = samples.len();
    if n == 0 { return Err(GpStatsError::EmptyDataset); }
    if width == 0 || height == 0 { return Err(GpStatsError::InvalidParameter); }
    let (m, grid_len) = (n + 1, width * height);
    let req = grid_len * 16 + (m * 2 * m) * 8 + m * 8;
    if req > byte_budget { return Err(GpStatsError::ByteBudgetExceeded { requested: req, budget: byte_budget }); }

    let mut aug = vec![0.0f64; m * 2 * m];
    for i in 0..m {
        for j in 0..m {
            aug[i * 2 * m + j] = if i < n && j < n {
                model.semivariance((samples[i][0] - samples[j][0]).hypot(samples[i][1] - samples[j][1]))
            } else if i < n || j < n { 1.0 } else { 0.0 };
        }
        aug[i * 2 * m + m + i] = 1.0;
    }

    for i in 0..m {
        let (mut max_r, mut max_v) = (i, aug[i * 2 * m + i].abs());
        for r in (i + 1)..m { let v = aug[r * 2 * m + i].abs(); if v > max_v { max_v = v; max_r = r; } }
        if max_v < 1e-13 { return Err(GpStatsError::SingularMatrix); }
        if max_r != i { for c in 0..(2 * m) { aug.swap(i * 2 * m + c, max_r * 2 * m + c); } }
        let piv = aug[i * 2 * m + i];
        for c in 0..(2 * m) { aug[i * 2 * m + c] /= piv; }
        for r in 0..m {
            if r != i && aug[r * 2 * m + i].abs() > 1e-15 {
                let fac = aug[r * 2 * m + i];
                for c in 0..(2 * m) { aug[r * 2 * m + c] -= fac * aug[i * 2 * m + c]; }
            }
        }
    }

    let (mut grid_val, mut grid_var, mut gamma_b) = (vec![0.0f64; grid_len], vec![0.0f64; grid_len], vec![0.0f64; m]);
    gamma_b[n] = 1.0;
    let (dx, dy) = (if width > 1 { (bbox[2] - bbox[0]) / (width - 1) as f64 } else { 0.0 },
                    if height > 1 { (bbox[3] - bbox[1]) / (height - 1) as f64 } else { 0.0 });

    for row in 0..height {
        let y0 = bbox[1] + row as f64 * dy;
        for col in 0..width {
            let x0 = bbox[0] + col as f64 * dx;
            for (i, s) in samples.iter().enumerate() { gamma_b[i] = model.semivariance((x0 - s[0]).hypot(y0 - s[1])); }
            let (mut z_est, mut var_est) = (0.0f64, 0.0f64);
            for i in 0..m {
                let mut w_i = 0.0f64;
                for j in 0..m { w_i += aug[i * 2 * m + m + j] * gamma_b[j]; }
                if i < n { z_est += w_i * samples[i][2]; }
                var_est += w_i * gamma_b[i];
            }
            let idx = row * width + col;
            grid_val[idx] = z_est; grid_var[idx] = var_est.max(0.0);
        }
    }
    Ok((grid_val, grid_var))
}

pub fn gp_idw_interpolate_into(
    samples: &[[f64; 3]], power: f64, bbox: [f64; 4],
    width: usize, height: usize, out_grid: &mut [f64],
) -> Result<(), GpStatsError> {
    if samples.is_empty() { return Err(GpStatsError::EmptyDataset); }
    if power <= 0.0 || width == 0 || height == 0 { return Err(GpStatsError::InvalidParameter); }
    if out_grid.len() < width * height { return Err(GpStatsError::BufferTooSmall); }
    let (dx, dy) = (if width > 1 { (bbox[2] - bbox[0]) / (width - 1) as f64 } else { 0.0 },
                    if height > 1 { (bbox[3] - bbox[1]) / (height - 1) as f64 } else { 0.0 });
    for row in 0..height {
        let y = bbox[1] + row as f64 * dy;
        for col in 0..width {
            let (x, mut exact, mut sw, mut swz) = (bbox[0] + col as f64 * dx, None, 0.0f64, 0.0f64);
            for s in samples {
                let d2 = (x - s[0]).powi(2) + (y - s[1]).powi(2);
                if d2 < 1e-24 { exact = Some(s[2]); break; }
                let w = 1.0 / d2.sqrt().powf(power);
                sw += w; swz += w * s[2];
            }
            out_grid[row * width + col] = exact.unwrap_or_else(|| if sw > 0.0 { swz / sw } else { 0.0 });
        }
    }
    Ok(())
}

pub fn gp_natural_neighbor_sibson(
    samples: &[[f64; 3]], query_pt: [f64; 2], byte_budget: usize,
) -> Result<f64, GpStatsError> {
    if samples.is_empty() { return Err(GpStatsError::EmptyDataset); }
    for s in samples {
        if (query_pt[0] - s[0]).hypot(query_pt[1] - s[1]) < 1e-12 { return Ok(s[2]); }
    }
    let (n, g) = (samples.len(), 32usize);
    let req = n * 8 + (g * g) * 8;
    if req > byte_budget { return Err(GpStatsError::ByteBudgetExceeded { requested: req, budget: byte_budget }); }

    let (mut min_d, mut max_d) = (f64::INFINITY, 0.0f64);
    for s in samples {
        let d = (query_pt[0] - s[0]).hypot(query_pt[1] - s[1]);
        min_d = min_d.min(d); max_d = max_d.max(d);
    }
    let r = (min_d * 2.5).clamp(1e-4, max_d.max(1e-3));
    let (step, mut stolen, mut total_stolen) = (2.0 * r / (g as f64 - 1.0), vec![0.0f64; n], 0.0f64);
    for row in 0..g {
        let py = (query_pt[1] - r) + row as f64 * step;
        for col in 0..g {
            let px = (query_pt[0] - r) + col as f64 * step;
            let d_qp_sq = (px - query_pt[0]).powi(2) + (py - query_pt[1]).powi(2);
            let (mut bi, mut bd) = (0, f64::INFINITY);
            for (i, s) in samples.iter().enumerate() {
                let d = (px - s[0]).powi(2) + (py - s[1]).powi(2);
                if d < bd { bd = d; bi = i; }
            }
            if d_qp_sq < bd { stolen[bi] += 1.0; total_stolen += 1.0; }
        }
    }
    if total_stolen > 0.0 {
        Ok(samples.iter().enumerate().map(|(i, s)| (stolen[i] / total_stolen) * s[2]).sum())
    } else {
        samples.iter().min_by(|a, b| (query_pt[0] - a[0]).hypot(query_pt[1] - a[1])
            .partial_cmp(&(query_pt[0] - b[0]).hypot(query_pt[1] - b[1])).unwrap())
            .map(|s| s[2]).ok_or(GpStatsError::EmptyDataset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoranGlobalResult {
    pub moran_i: f64, pub expected_i: f64, pub z_score: f64, pub geary_c: f64,
}

pub fn gp_global_morans_i_and_gearys_c(
    samples: &[[f64; 3]], bandwidth: f64,
) -> Result<MoranGlobalResult, GpStatsError> {
    let n = samples.len();
    if n < 3 { return Err(GpStatsError::InsufficientSamples); }
    if bandwidth <= 0.0 { return Err(GpStatsError::InvalidParameter); }

    let (nf, mean_z) = (n as f64, samples.iter().map(|s| s[2]).sum::<f64>() / n as f64);
    let ss = samples.iter().map(|s| (s[2] - mean_z).powi(2)).sum::<f64>();
    if ss < 1e-14 { return Err(GpStatsError::InvalidParameter); }

    let (mut s0, mut s2, mut num_i, mut num_c) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for i in 0..n {
        let mut row_sum = 0.0f64;
        for j in 0..n {
            if i != j && (samples[i][0] - samples[j][0]).hypot(samples[i][1] - samples[j][1]) <= bandwidth {
                row_sum += 1.0;
                num_i += (samples[i][2] - mean_z) * (samples[j][2] - mean_z);
                num_c += (samples[i][2] - samples[j][2]).powi(2);
            }
        }
        s0 += row_sum; s2 += 4.0 * row_sum * row_sum;
    }
    if s0 < 1e-12 { return Err(GpStatsError::InvalidParameter); }
    let s1 = 2.0 * s0;
    let moran_i = (nf / s0) * (num_i / ss);
    let expected_i = -1.0 / (nf - 1.0);
    let var_i = (nf * nf * s1 - nf * s2 + 3.0 * s0 * s0) / (s0 * s0 * (nf * nf - 1.0)) - expected_i * expected_i;
    let z_score = (moran_i - expected_i) / var_i.max(1e-12).sqrt();
    let geary_c = ((nf - 1.0) / (2.0 * s0)) * (num_c / ss);
    Ok(MoranGlobalResult { moran_i, expected_i, z_score, geary_c })
}

pub fn gp_local_lisa_and_getis_ord_gi_star(
    samples: &[[f64; 3]], bandwidth: f64, out_lisa_i: &mut [f64], out_gi_star_z: &mut [f64],
) -> Result<(), GpStatsError> {
    let n = samples.len();
    if n < 3 { return Err(GpStatsError::InsufficientSamples); }
    if bandwidth <= 0.0 { return Err(GpStatsError::InvalidParameter); }
    if out_lisa_i.len() < n || out_gi_star_z.len() < n { return Err(GpStatsError::BufferTooSmall); }

    let (nf, mean_z) = (n as f64, samples.iter().map(|s| s[2]).sum::<f64>() / n as f64);
    let m2 = samples.iter().map(|s| (s[2] - mean_z).powi(2)).sum::<f64>() / nf;
    let s_std = (samples.iter().map(|s| s[2] * s[2]).sum::<f64>() / nf - mean_z * mean_z).max(0.0).sqrt();

    for i in 0..n {
        let (mut lag, mut sum_gi, mut w_gi) = (0.0f64, samples[i][2], 1.0f64);
        for j in 0..n {
            if i != j && (samples[i][0] - samples[j][0]).hypot(samples[i][1] - samples[j][1]) <= bandwidth {
                lag += samples[j][2] - mean_z;
                sum_gi += samples[j][2];
                w_gi += 1.0;
            }
        }
        let zi = samples[i][2] - mean_z;
        out_lisa_i[i] = if m2 > 1e-14 { (zi / m2) * lag } else { 0.0 };
        let denom = s_std * ((nf * w_gi - w_gi * w_gi) / (nf - 1.0)).max(0.0).sqrt();
        out_gi_star_z[i] = if denom > 1e-12 { (sum_gi - mean_z * w_gi) / denom } else { 0.0 };
    }
    Ok(())
}

pub fn gp_ripleys_k_and_l_into(
    pts: &[[f64; 2]], study_area: f64, radii: &[f64], out_l_minus_r: &mut [f64],
) -> Result<(), GpStatsError> {
    let n = pts.len();
    if n < 2 { return Err(GpStatsError::InsufficientSamples); }
    if study_area <= 0.0 { return Err(GpStatsError::InvalidParameter); }
    if out_l_minus_r.len() < radii.len() { return Err(GpStatsError::BufferTooSmall); }
    let nf = n as f64;
    for (k, &r) in radii.iter().enumerate() {
        if r < 0.0 { return Err(GpStatsError::InvalidParameter); }
        let (r2, mut count) = (r * r, 0.0f64);
        for i in 0..n {
            for j in 0..n {
                if i != j && (pts[i][0] - pts[j][0]).powi(2) + (pts[i][1] - pts[j][1]).powi(2) <= r2 { count += 1.0; }
            }
        }
        out_l_minus_r[k] = ((study_area / (nf * (nf - 1.0)) * count) / std::f64::consts::PI).sqrt() - r;
    }
    Ok(())
}

pub fn gp_gwr_local_regression(
    coords: &[[f64; 2]], x_var: &[f64], y_var: &[f64], bandwidth: f64, byte_budget: usize,
) -> Result<Vec<[f64; 3]>, GpStatsError> {
    let n = coords.len();
    if n == 0 { return Err(GpStatsError::EmptyDataset); }
    if x_var.len() != n || y_var.len() != n || bandwidth <= 0.0 { return Err(GpStatsError::InvalidParameter); }
    let req = n * 24;
    if req > byte_budget { return Err(GpStatsError::ByteBudgetExceeded { requested: req, budget: byte_budget }); }

    let mut res = Vec::with_capacity(n);
    let inv_b2 = 1.0 / (bandwidth * bandwidth);

    for i in 0..n {
        let (mut sw, mut swx, mut swy) = (0.0f64, 0.0f64, 0.0f64);
        for j in 0..n {
            let w = (-0.5 * ((coords[i][0] - coords[j][0]).powi(2) + (coords[i][1] - coords[j][1]).powi(2)) * inv_b2).exp();
            sw += w; swx += w * x_var[j]; swy += w * y_var[j];
        }
        let (mean_x, mean_y) = (swx / sw, swy / sw);
        let (mut sxx, mut sxy, mut syy) = (0.0f64, 0.0f64, 0.0f64);
        for j in 0..n {
            let w = (-0.5 * ((coords[i][0] - coords[j][0]).powi(2) + (coords[i][1] - coords[j][1]).powi(2)) * inv_b2).exp();
            let (dx, dy) = (x_var[j] - mean_x, y_var[j] - mean_y);
            sxx += w * dx * dx; sxy += w * dx * dy; syy += w * dy * dy;
        }
        let slope = if sxx > 1e-12 { sxy / sxx } else { 0.0 };
        let intercept = mean_y - slope * mean_x;
        let r2 = if sxx > 1e-12 && syy > 1e-12 { (sxy * sxy / (sxx * syy)).clamp(0.0, 1.0) } else { 0.0 };
        res.push([intercept, slope, r2]);
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variogram_kriging_idw_sibson() {
        let samples = [
            [0.0, 0.0, 10.0], [10.0, 0.0, 20.0], [0.0, 10.0, 15.0], [10.0, 10.0, 25.0], [5.0, 5.0, 17.5], [2.0, 0.0, 12.0],
        ];
        let mut bins = [EmpiricalVariogramBin { lag_center: 0.0, semivariance: 0.0, pair_count: 0 }; 4];
        let n_lags = gp_empirical_variogram(&samples, 4, 15.0, &mut bins).expect("variogram calc");
        assert_eq!(n_lags, 4);
        assert!(bins[0].pair_count > 0);

        let model = gp_fit_variogram_wls(&bins, 0).expect("wls fit");
        assert!(model.semivariance(5.0) > 0.0);
        assert!(model.semivariance(15.0) >= model.semivariance(5.0));

        let bbox = [0.0, 0.0, 10.0, 10.0];
        let (vals, vars) = gp_ordinary_kriging_grid(&samples, &model, bbox, 3, 3, 100_000).expect("kriging");
        assert_eq!(vals.len(), 9);
        assert_eq!(vars.len(), 9);

        // Verification of sample locations: (0,0)->idx 0, (10,10)->idx 8, (5,5)->idx 4
        assert!((vals[0] - 10.0).abs() < 1e-3, "Sample 0 match: {}", vals[0]);
        assert!(vars[0] < 1e-4, "Sample 0 zero var: {}", vars[0]);
        assert!((vals[8] - 25.0).abs() < 1e-3, "Sample 3 match: {}", vals[8]);
        assert!(vars[8] < 1e-4, "Sample 3 zero var: {}", vars[8]);
        assert!((vals[4] - 17.5).abs() < 1e-3, "Sample 4 match: {}", vals[4]);
        assert!(vars[4] < 1e-4, "Sample 4 zero var: {}", vars[4]);

        // IDW & Sibson tests
        let samples_2pt = [[0.0, 0.0, 10.0], [2.0, 0.0, 20.0]];
        let mut idw_out = [0.0; 3];
        gp_idw_interpolate_into(&samples_2pt, 2.0, [0.0, 0.0, 2.0, 0.0], 3, 1, &mut idw_out).expect("idw");
        assert!((idw_out[0] - 10.0).abs() < 1e-6 && (idw_out[1] - 15.0).abs() < 1e-6 && (idw_out[2] - 20.0).abs() < 1e-6);

        let sib_mid = gp_natural_neighbor_sibson(&samples_2pt, [1.0, 0.0], 50_000).expect("sibson");
        assert!((sib_mid - 15.0).abs() < 0.1, "Sibson midpoint ~15.0: {}", sib_mid);
        let sib_exact = gp_natural_neighbor_sibson(&samples_2pt, [0.0, 0.0], 50_000).expect("sibson exact");
        assert!((sib_exact - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_spatial_autocorrelation_moran_geary_lisa_getis() {
        let clustered = [
            [0.0, 0.0, 100.0], [0.0, 1.0, 105.0], [1.0, 0.0, 98.0], [1.0, 1.0, 102.0],
            [10.0, 10.0, 10.0], [10.0, 11.0, 12.0], [11.0, 10.0, 8.0], [11.0, 11.0, 11.0],
        ];
        let moran_res = gp_global_morans_i_and_gearys_c(&clustered, 2.5).expect("moran clustered");
        assert!(moran_res.moran_i > 0.4, "Clustered Moran's I > 0.4: {}", moran_res.moran_i);
        assert!(moran_res.geary_c < 0.6, "Clustered Geary's C < 0.6: {}", moran_res.geary_c);

        let checkerboard = [[0.0, 0.0, 100.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 100.0]];
        let disp_res = gp_global_morans_i_and_gearys_c(&checkerboard, 1.1).expect("moran dispersed");
        assert!(disp_res.moran_i < disp_res.expected_i, "Dispersed Moran < exp: {}", disp_res.moran_i);
        assert!(disp_res.geary_c > 1.0, "Dispersed Geary > 1: {}", disp_res.geary_c);

        let mut lisa = [0.0; 8];
        let mut gi = [0.0; 8];
        gp_local_lisa_and_getis_ord_gi_star(&clustered, 2.5, &mut lisa, &mut gi).expect("lisa & gi");
        assert!(gi[0] > 0.0, "Hot spot G_i* positive: {}", gi[0]);
        assert!(gi[4] < 0.0, "Cold spot G_i* negative: {}", gi[4]);
        assert!(lisa[0] > 0.0, "Cluster LISA positive: {}", lisa[0]);
    }

    #[test]
    fn test_ripley_gwr_and_budget_fail_closed() {
        let pts = [[0.0, 0.0], [0.1, 0.0], [0.0, 0.1], [0.1, 0.1]];
        let radii = [0.05, 0.2, 1.0];
        let mut l_r = [0.0; 3];
        gp_ripleys_k_and_l_into(&pts, 100.0, &radii, &mut l_r).expect("ripley");
        assert!(l_r[1] > 0.0, "Clustered Ripley L(r) - r > 0: {}", l_r[1]);

        let coords = [[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [3.0, 0.0]];
        let x = [1.0, 2.0, 3.0, 4.0];
        let y = [3.0, 5.0, 7.0, 9.0];
        let gwr = gp_gwr_local_regression(&coords, &x, &y, 10.0, 10_000).expect("gwr");
        assert_eq!(gwr.len(), 4);
        assert!((gwr[0][1] - 2.0).abs() < 1e-3, "Slope ~ 2.0: {}", gwr[0][1]);
        assert!((gwr[0][0] - 1.0).abs() < 1e-3, "Intercept ~ 1.0: {}", gwr[0][0]);
        assert!((gwr[0][2] - 1.0).abs() < 1e-3, "R2 ~ 1.0: {}", gwr[0][2]);

        // Budget fail-closed checks
        let samples = [[0.0, 0.0, 1.0], [1.0, 1.0, 2.0]];
        let model = VariogramModel::Spherical { nugget: 0.0, sill: 1.0, range: 2.0 };
        let krig_err = gp_ordinary_kriging_grid(&samples, &model, [0.0, 0.0, 1.0, 1.0], 5, 5, 10).unwrap_err();
        assert!(matches!(krig_err, GpStatsError::ByteBudgetExceeded { .. }));

        let gwr_err = gp_gwr_local_regression(&coords, &x, &y, 10.0, 10).unwrap_err();
        assert!(matches!(gwr_err, GpStatsError::ByteBudgetExceeded { .. }));

        let sib_err = gp_natural_neighbor_sibson(&samples, [0.5, 0.5], 10).unwrap_err();
        assert!(matches!(sib_err, GpStatsError::ByteBudgetExceeded { .. }));
    }
}
