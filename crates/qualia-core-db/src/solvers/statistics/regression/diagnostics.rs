//! Classical residual / specification tests for single-equation OLS (Welc Ch.4).

use crate::solvers::statistics::descriptive::{kurtosis, mean, skewness, variance};
use crate::solvers::statistics::distributions::{chi_squared, fisher_f, students_t};
use crate::solvers::statistics::regression::multiple_ols;

use serde::Serialize;

/// Jarque–Bera normality of residuals.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct JarqueBeraResult {
    pub statistic: f64,
    pub p_value: f64,
    pub skewness: f64,
    pub excess_kurtosis: f64,
}

/// JB = n/6 · (S² + K²/4); asymptotic χ²(2). `None` if n < 4 or zero variance.
pub fn jarque_bera(residuals: &[f64]) -> Option<JarqueBeraResult> {
    let n = residuals.len();
    if n < 4 {
        return None;
    }
    let s = skewness(residuals)?;
    let k = kurtosis(residuals)?; // Fisher excess
    let jb = (n as f64) / 6.0 * (s * s + k * k / 4.0);
    Some(JarqueBeraResult {
        statistic: jb,
        p_value: chi_squared::upper_p(jb, 2.0),
        skewness: s,
        excess_kurtosis: k,
    })
}

/// Breusch–Pagan heteroscedasticity: regress e² on the same design (no intercept
/// double-count — uses predictors only when design includes intercept column).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct BreuschPaganResult {
    pub lm_statistic: f64,
    pub p_value: f64,
    pub df: f64,
}

/// `design` is row-major `n × k` (same as MultipleOls.design). Uses all columns
/// of `design` as regressors for e² (including intercept). LM = n·R² ~ χ²(k−1).
pub fn breusch_pagan(residuals: &[f64], design: &[f64], n: usize, k: usize) -> Option<BreuschPaganResult> {
    if residuals.len() != n || design.len() != n * k || k < 2 || n <= k {
        return None;
    }
    let e2: Vec<f64> = residuals.iter().map(|e| e * e).collect();
    // Predictors = design columns 1..k-1 (drop intercept) for classical BP;
    // if k==1 only intercept, nothing to test.
    let p = k - 1;
    let mut x = vec![0.0; n * p];
    for i in 0..n {
        x[i * p..(i + 1) * p].copy_from_slice(&design[i * k + 1..i * k + k]);
    }
    let m = multiple_ols(&x, &e2, n, p, true)?;
    let lm = n as f64 * m.r_squared;
    let df = p as f64;
    Some(BreuschPaganResult {
        lm_statistic: lm,
        p_value: chi_squared::upper_p(lm, df),
        df,
    })
}

/// ARCH-LM: regress e_t² on lag-1..q of e².
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ArchLmResult {
    pub lm_statistic: f64,
    pub p_value: f64,
    pub df: f64,
    pub q: usize,
}

pub fn arch_lm(residuals: &[f64], q: usize) -> Option<ArchLmResult> {
    let n = residuals.len();
    if q == 0 || n <= q + 2 {
        return None;
    }
    let e2: Vec<f64> = residuals.iter().map(|e| e * e).collect();
    let m = n - q;
    let mut x = vec![0.0; m * q];
    let mut y = vec![0.0; m];
    for t in 0..m {
        y[t] = e2[t + q];
        for j in 0..q {
            x[t * q + j] = e2[t + q - 1 - j];
        }
    }
    let fit = multiple_ols(&x, &y, m, q, true)?;
    let lm = m as f64 * fit.r_squared;
    Some(ArchLmResult {
        lm_statistic: lm,
        p_value: chi_squared::upper_p(lm, q as f64),
        df: q as f64,
        q,
    })
}

/// Durbin–Watson statistic on ordered residuals.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct DurbinWatsonResult {
    pub statistic: f64,
    /// Rough two-sided p via normal approx of DW around 2 (informational).
    pub approx_p_value: f64,
}

pub fn durbin_watson(residuals: &[f64]) -> Option<DurbinWatsonResult> {
    let n = residuals.len();
    if n < 3 {
        return None;
    }
    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..n {
        den += residuals[i] * residuals[i];
        if i > 0 {
            let d = residuals[i] - residuals[i - 1];
            num += d * d;
        }
    }
    if den <= 0.0 {
        return None;
    }
    let dw = num / den;
    // Under H0, E[DW]≈2, Var≈4/n → Z = (DW−2)/√(4/n).
    let se = (4.0 / n as f64).sqrt();
    let z = (dw - 2.0) / se;
    let approx_p = crate::solvers::statistics::distributions::normal::two_sided_p(z);
    Some(DurbinWatsonResult {
        statistic: dw,
        approx_p_value: approx_p,
    })
}

/// Box–Pearce / residual AC F-style: first-lag autocorrelation squared × (n−1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct BoxPearceResult {
    pub lag1_acf: f64,
    pub statistic: f64,
    pub p_value: f64,
}

pub fn box_pearce_ac(residuals: &[f64]) -> Option<BoxPearceResult> {
    let n = residuals.len();
    if n < 4 {
        return None;
    }
    let ac = crate::solvers::statistics::timeseries::autocorrelation(residuals, 1)?;
    let stat = (n as f64 - 1.0) * ac * ac;
    Some(BoxPearceResult {
        lag1_acf: ac,
        statistic: stat,
        p_value: chi_squared::upper_p(stat, 1.0),
    })
}

/// Per-predictor VIF from the design columns excluding intercept.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VifResult {
    pub vif: Vec<f64>,
}

/// `x` is row-major `n × p` predictors (no intercept column). VIF_j = 1/(1−R²_j).
pub fn vif_columns(x: &[f64], n: usize, p: usize) -> Option<VifResult> {
    if p < 2 || x.len() != n * p || n <= p {
        return None;
    }
    let mut vif = vec![0.0; p];
    for j in 0..p {
        let mut y = vec![0.0; n];
        let mut xo = vec![0.0; n * (p - 1)];
        for i in 0..n {
            y[i] = x[i * p + j];
            let mut c = 0;
            for k in 0..p {
                if k == j {
                    continue;
                }
                xo[i * (p - 1) + c] = x[i * p + k];
                c += 1;
            }
        }
        let m = multiple_ols(&xo, &y, n, p - 1, true)?;
        let r2 = m.r_squared.clamp(0.0, 1.0 - 1e-15);
        vif[j] = 1.0 / (1.0 - r2);
    }
    Some(VifResult { vif })
}

/// Ramsey RESET: augment with powers of fitted ŷ.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct RamseyResetResult {
    pub f_statistic: f64,
    pub p_value: f64,
    pub df_num: f64,
    pub df_den: f64,
}

/// Powers 2..=power_max of fitted values added to original predictors.
pub fn ramsey_reset(
    x: &[f64],
    y: &[f64],
    n: usize,
    p: usize,
    power_max: usize,
) -> Option<RamseyResetResult> {
    if power_max < 2 || n < p + power_max + 2 {
        return None;
    }
    let restricted = multiple_ols(x, y, n, p, true)?;
    let extra = power_max - 1; // ŷ² .. ŷ^power_max
    let p2 = p + extra;
    let mut x2 = vec![0.0; n * p2];
    for i in 0..n {
        x2[i * p2..i * p2 + p].copy_from_slice(&x[i * p..(i + 1) * p]);
        let yh = restricted.fitted[i];
        for (t, pow) in (2..=power_max).enumerate() {
            x2[i * p2 + p + t] = yh.powi(pow as i32);
        }
    }
    let unrestricted = multiple_ols(&x2, y, n, p2, true)?;
    let sse_r = restricted
        .residuals
        .iter()
        .map(|e| e * e)
        .sum::<f64>();
    let sse_u = unrestricted
        .residuals
        .iter()
        .map(|e| e * e)
        .sum::<f64>();
    let df_num = extra as f64;
    let df_den = unrestricted.df_residual as f64;
    if df_den <= 0.0 || sse_u <= 0.0 {
        return None;
    }
    let f = ((sse_r - sse_u) / df_num) / (sse_u / df_den);
    Some(RamseyResetResult {
        f_statistic: f,
        p_value: fisher_f::upper_p(f, df_num, df_den),
        df_num,
        df_den,
    })
}

/// Chow structural-break test at `break_index` (second subsample starts here).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ChowResult {
    pub f_statistic: f64,
    pub p_value: f64,
    pub df_num: f64,
    pub df_den: f64,
}

pub fn chow_test(
    x: &[f64],
    y: &[f64],
    n: usize,
    p: usize,
    break_index: usize,
) -> Option<ChowResult> {
    let k = p + 1; // with intercept
    if break_index < k + 1 || n - break_index < k + 1 || x.len() != n * p {
        return None;
    }
    let pooled = multiple_ols(x, y, n, p, true)?;
    let sse_p: f64 = pooled.residuals.iter().map(|e| e * e).sum();

    let n1 = break_index;
    let n2 = n - break_index;
    let x1 = &x[..n1 * p];
    let y1 = &y[..n1];
    let x2 = &x[n1 * p..];
    let y2 = &y[n1..];
    let m1 = multiple_ols(x1, y1, n1, p, true)?;
    let m2 = multiple_ols(x2, y2, n2, p, true)?;
    let sse_1: f64 = m1.residuals.iter().map(|e| e * e).sum();
    let sse_2: f64 = m2.residuals.iter().map(|e| e * e).sum();
    let sse_u = sse_1 + sse_2;
    let df_num = k as f64;
    let df_den = (n - 2 * k) as f64;
    if df_den <= 0.0 || sse_u <= 0.0 {
        return None;
    }
    let f = ((sse_p - sse_u) / df_num) / (sse_u / df_den);
    Some(ChowResult {
        f_statistic: f,
        p_value: fisher_f::upper_p(f, df_num, df_den),
        df_num,
        df_den,
    })
}

/// Residual symmetry: one-sample t of (residuals − median) vs 0 is weak;
/// here: skewness t-approx and mean-vs-median absolute gap.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SymmetryResult {
    pub skewness: f64,
    pub mean_minus_median: f64,
    pub t_statistic: f64,
    pub p_value: f64,
}

pub fn residual_symmetry(residuals: &[f64]) -> Option<SymmetryResult> {
    let n = residuals.len();
    if n < 5 {
        return None;
    }
    let mu = mean(residuals)?;
    let mut sorted = residuals.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let med = crate::solvers::statistics::descriptive::median_sorted(&sorted)?;
    let s = skewness(residuals)?;
    let var = variance(residuals, true)?;
    if var <= 0.0 {
        return None;
    }
    // Test H0: mean = median via SE of mean.
    let se = (var / n as f64).sqrt();
    let t = (mu - med) / se;
    let p = students_t::two_sided_p(t, (n - 1) as f64);
    Some(SymmetryResult {
        skewness: s,
        mean_minus_median: mu - med,
        t_statistic: t,
        p_value: p,
    })
}

/// Wald–Wolfowitz runs test on residual signs (above/below median).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct RunsResult {
    pub n_runs: usize,
    pub n_pos: usize,
    pub n_neg: usize,
    pub z_statistic: f64,
    pub p_value: f64,
}

pub fn residual_runs(residuals: &[f64]) -> Option<RunsResult> {
    let n = residuals.len();
    if n < 8 {
        return None;
    }
    let mut sorted = residuals.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let med = crate::solvers::statistics::descriptive::median_sorted(&sorted)?;
    let mut signs: Vec<i8> = Vec::with_capacity(n);
    for &r in residuals {
        if (r - med).abs() < 1e-15 {
            continue; // skip exact median ties
        }
        signs.push(if r > med { 1 } else { -1 });
    }
    if signs.len() < 8 {
        return None;
    }
    let mut n_pos = 0usize;
    let mut n_neg = 0usize;
    let mut n_runs = 1usize;
    for (i, &s) in signs.iter().enumerate() {
        if s > 0 {
            n_pos += 1;
        } else {
            n_neg += 1;
        }
        if i > 0 && s != signs[i - 1] {
            n_runs += 1;
        }
    }
    let n1 = n_pos as f64;
    let n2 = n_neg as f64;
    let nn = n1 + n2;
    if n1 == 0.0 || n2 == 0.0 {
        return None;
    }
    let expected = 1.0 + 2.0 * n1 * n2 / nn;
    let var = (2.0 * n1 * n2 * (2.0 * n1 * n2 - nn)) / (nn * nn * (nn - 1.0));
    if var <= 0.0 {
        return None;
    }
    let z = (n_runs as f64 - expected) / var.sqrt();
    Some(RunsResult {
        n_runs,
        n_pos,
        n_neg,
        z_statistic: z,
        p_value: crate::solvers::statistics::distributions::normal::two_sided_p(z),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jb_near_zero_for_near_normal() {
        // Roughly symmetric small sample.
        let r = [-1.2, -0.5, 0.1, 0.4, 0.8, -0.2, 0.3, -0.7, 0.6, -0.1];
        let jb = jarque_bera(&r).unwrap();
        assert!(jb.p_value > 0.05, "p={}", jb.p_value);
    }

    #[test]
    fn dw_near_two_for_white_noise() {
        let r = [0.1, 0.05, -0.02, 0.08, -0.04, 0.01, -0.06, 0.03, -0.01, 0.02];
        let dw = durbin_watson(&r).unwrap();
        assert!(
            (dw.statistic - 2.0).abs() < 1.2,
            "dw={}",
            dw.statistic
        );
    }

    #[test]
    fn vif_flags_collinear_pair() {
        // x2 ≈ x1 → high VIF.
        let x = [
            1.0, 1.01, 2.0, 2.02, 3.0, 2.99, 4.0, 4.01, 5.0, 5.02, 6.0, 5.98,
        ];
        let v = vif_columns(&x, 6, 2).unwrap();
        assert!(v.vif[0] > 10.0 && v.vif[1] > 10.0, "vif={:?}", v.vif);
    }
}
