//! Statistics engine exports — descriptive summary, correlation, hypothesis
//! tests, simple linear regression, and distribution pdf/cdf/quantile.
//!
//! Wraps the engine's wasm-clean solver math (`crate::solvers::statistics::*`).
//! Same code the native MCP statistics tools and the solver unit tests exercise:
//! zero-allocation kernels over `f64` slices, no timing/IO/thread/RNG, so it runs
//! identically in the browser and natively.
#![cfg(target_arch = "wasm32")]

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use super::jserr;

/// Full descriptive summary of a sample. Input `{ data:[..], sample?:bool }`
/// (`sample` defaults to `true` → Bessel-corrected variance/std) →
/// `{ n, sum, mean, variance, std_dev, min, max, median, q1, q3, skewness, kurtosis }`.
/// `variance`/`std_dev` are `null` when n < 2 in sample mode (no residual dof);
/// `skewness`/`kurtosis` are excess-kurtosis (Fisher) conventions.
#[wasm_bindgen]
pub fn stats_describe_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        data: Vec<f64>,
        #[serde(default = "default_true")]
        sample: bool,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if p.data.is_empty() {
        return Err(JsValue::from_str("data must be non-empty"));
    }
    use crate::solvers::statistics::descriptive as d;

    let mean = d::mean(&p.data).ok_or_else(|| JsValue::from_str("data must be non-empty"))?;
    let variance = d::variance(&p.data, p.sample);
    let std_dev = d::std_dev(&p.data, p.sample);
    let min = d::min(&p.data).ok_or_else(|| JsValue::from_str("data must be non-empty"))?;
    let max = d::max(&p.data).ok_or_else(|| JsValue::from_str("data must be non-empty"))?;
    let sum = d::sum(&p.data);
    let skewness = d::skewness(&p.data);
    let kurtosis = d::kurtosis(&p.data);

    // median / quantiles need a sorted owned buffer (the `*_in_place` kernels sort
    // the caller's slice — clone so we never mutate caller intent twice).
    let mut buf = p.data.clone();
    let median = d::median_in_place(&mut buf)
        .ok_or_else(|| JsValue::from_str("data must be non-empty"))?;
    // `buf` is now sorted ascending; reuse the sorted-slice quantile.
    let q1 = d::quantile_sorted(&buf, 0.25)
        .ok_or_else(|| JsValue::from_str("data must be non-empty"))?;
    let q3 = d::quantile_sorted(&buf, 0.75)
        .ok_or_else(|| JsValue::from_str("data must be non-empty"))?;

    #[derive(Serialize)]
    struct Out {
        n: usize,
        sum: f64,
        mean: f64,
        variance: Option<f64>,
        std_dev: Option<f64>,
        min: f64,
        max: f64,
        median: f64,
        q1: f64,
        q3: f64,
        skewness: Option<f64>,
        kurtosis: Option<f64>,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        n: p.data.len(),
        sum,
        mean,
        variance,
        std_dev,
        min,
        max,
        median,
        q1,
        q3,
        skewness,
        kurtosis,
    })?)
}

/// Linear-interpolated quantile (numpy "linear" / R type-7). Input
/// `{ data:[..], q:0.0..1.0 }` → `{ quantile }`. `q` is clamped to `[0,1]`.
#[wasm_bindgen]
pub fn stats_quantile_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        data: Vec<f64>,
        q: f64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if p.data.is_empty() {
        return Err(JsValue::from_str("data must be non-empty"));
    }
    let mut buf = p.data.clone();
    let quantile = crate::solvers::statistics::descriptive::quantile_in_place(&mut buf, p.q)
        .ok_or_else(|| JsValue::from_str("data must be non-empty"))?;
    #[derive(Serialize)]
    struct Out {
        quantile: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { quantile })?)
}

/// Pearson, Spearman, and Kendall correlation of two equal-length series, plus the
/// two-sided p-value for the Pearson coefficient. Input `{ x:[..], y:[..] }` →
/// `{ pearson, spearman, kendall, pearson_p_value }`. Each coefficient is `null`
/// when undefined (lengths differ, or n < 2); `pearson_p_value` is `null` for n < 3.
#[wasm_bindgen]
pub fn stats_correlation_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        x: Vec<f64>,
        y: Vec<f64>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if p.x.is_empty() || p.y.is_empty() {
        return Err(JsValue::from_str("x and y must be non-empty"));
    }
    if p.x.len() != p.y.len() {
        return Err(JsValue::from_str("x and y must have equal length"));
    }
    use crate::solvers::statistics::correlation as c;
    let pearson = c::pearson(&p.x, &p.y);
    let spearman = c::spearman(&p.x, &p.y);
    let kendall = c::kendall(&p.x, &p.y);
    let pearson_p_value = pearson.and_then(|r| c::correlation_p_value(r, p.x.len()));
    #[derive(Serialize)]
    struct Out {
        pearson: Option<f64>,
        spearman: Option<f64>,
        kendall: Option<f64>,
        pearson_p_value: Option<f64>,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        pearson,
        spearman,
        kendall,
        pearson_p_value,
    })?)
}

/// One-sample t-test of the sample mean against `mu`. Input `{ data:[..], mu:f64 }`
/// → `{ t_statistic, p_value, degrees_of_freedom, ci_lower, ci_upper }`
/// (95% CI around the sample mean, t critical value). Errors if n < 2.
#[wasm_bindgen]
pub fn stats_one_sample_t_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        data: Vec<f64>,
        mu: f64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let r = crate::solvers::statistics::hypothesis::one_sample_t(&p.data, p.mu)
        .ok_or_else(|| JsValue::from_str("one-sample t-test requires n >= 2"))?;
    #[derive(Serialize)]
    struct Out {
        t_statistic: f64,
        p_value: f64,
        degrees_of_freedom: u32,
        ci_lower: f64,
        ci_upper: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        t_statistic: r.t_statistic,
        p_value: r.p_value,
        degrees_of_freedom: r.degrees_of_freedom,
        ci_lower: r.confidence_interval.0,
        ci_upper: r.confidence_interval.1,
    })?)
}

/// Two-sample t-test of `mean(a) − mean(b) = 0`. Input
/// `{ a:[..], b:[..], equal_var?:bool }` (`equal_var` defaults to `false` → the
/// Welch test; `true` → pooled Student) → `{ t_statistic, p_value,
/// degrees_of_freedom, mean_difference, ci_lower, ci_upper }`. Errors if either
/// sample has n < 2.
#[wasm_bindgen]
pub fn stats_two_sample_t_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        a: Vec<f64>,
        b: Vec<f64>,
        #[serde(default)]
        equal_var: bool,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let r = crate::solvers::statistics::hypothesis::two_sample_t(&p.a, &p.b, p.equal_var)
        .ok_or_else(|| JsValue::from_str("two-sample t-test requires n >= 2 in each sample"))?;
    #[derive(Serialize)]
    struct Out {
        t_statistic: f64,
        p_value: f64,
        degrees_of_freedom: f64,
        mean_difference: f64,
        ci_lower: f64,
        ci_upper: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        t_statistic: r.t_statistic,
        p_value: r.p_value,
        degrees_of_freedom: r.degrees_of_freedom,
        mean_difference: r.mean_difference,
        ci_lower: r.confidence_interval.0,
        ci_upper: r.confidence_interval.1,
    })?)
}

/// Paired t-test (one-sample t-test of the paired differences against 0). Input
/// `{ a:[..], b:[..] }` (equal length) → `{ t_statistic, p_value,
/// degrees_of_freedom, ci_lower, ci_upper }`. Errors if lengths differ or n < 2.
#[wasm_bindgen]
pub fn stats_paired_t_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        a: Vec<f64>,
        b: Vec<f64>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if p.a.len() != p.b.len() {
        return Err(JsValue::from_str("paired t-test requires equal-length samples"));
    }
    let r = crate::solvers::statistics::hypothesis::paired_t(&p.a, &p.b)
        .ok_or_else(|| JsValue::from_str("paired t-test requires equal-length samples with n >= 2"))?;
    #[derive(Serialize)]
    struct Out {
        t_statistic: f64,
        p_value: f64,
        degrees_of_freedom: u32,
        ci_lower: f64,
        ci_upper: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        t_statistic: r.t_statistic,
        p_value: r.p_value,
        degrees_of_freedom: r.degrees_of_freedom,
        ci_lower: r.confidence_interval.0,
        ci_upper: r.confidence_interval.1,
    })?)
}

/// One-way ANOVA F-test for equality of `k` group means. Input
/// `{ groups:[[..],[..],..] }` (≥ 2 groups, each non-empty, total > k) →
/// `{ f_statistic, p_value, df_between, df_within, ss_between, ss_within,
/// ms_between, ms_within }`. Errors on degenerate input.
#[wasm_bindgen]
pub fn stats_anova_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        groups: Vec<Vec<f64>>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if p.groups.len() < 2 {
        return Err(JsValue::from_str("ANOVA requires at least 2 groups"));
    }
    let refs: Vec<&[f64]> = p.groups.iter().map(|g| g.as_slice()).collect();
    let r = crate::solvers::statistics::hypothesis::one_way_anova(&refs).ok_or_else(|| {
        JsValue::from_str(
            "ANOVA requires >= 2 non-empty groups and total observations > number of groups",
        )
    })?;
    #[derive(Serialize)]
    struct Out {
        f_statistic: f64,
        p_value: f64,
        df_between: f64,
        df_within: f64,
        ss_between: f64,
        ss_within: f64,
        ms_between: f64,
        ms_within: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        f_statistic: r.f_statistic,
        p_value: r.p_value,
        df_between: r.df_between,
        df_within: r.df_within,
        ss_between: r.ss_between,
        ss_within: r.ss_within,
        ms_between: r.ms_between,
        ms_within: r.ms_within,
    })?)
}

/// Pearson χ² goodness-of-fit test, `Σ(Oᵢ−Eᵢ)²/Eᵢ`, dof = k−1. Input
/// `{ observed:[..], expected:[..] }` (equal length ≥ 2, all expected > 0) →
/// `{ statistic, p_value, dof }`. Errors on length mismatch, len < 2, or a
/// non-positive expected count.
#[wasm_bindgen]
pub fn stats_chi_square_gof_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        observed: Vec<f64>,
        expected: Vec<f64>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if p.observed.len() != p.expected.len() {
        return Err(JsValue::from_str("observed and expected must have equal length"));
    }
    let r = crate::solvers::statistics::hypothesis::chi_square_gof(&p.observed, &p.expected)
        .ok_or_else(|| {
            JsValue::from_str(
                "chi-square GOF requires equal-length vectors of length >= 2 with all expected > 0",
            )
        })?;
    #[derive(Serialize)]
    struct Out {
        statistic: f64,
        p_value: f64,
        dof: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        statistic: r.statistic,
        p_value: r.p_value,
        dof: r.dof,
    })?)
}

/// χ² test of independence on an R×C contingency table of counts. Input
/// `{ table:[[..],[..],..] }` (≥ 2 rows, ≥ 2 cols, rectangular, grand total > 0) →
/// `{ statistic, p_value, dof }` with `dof = (R−1)(C−1)`. Errors on a ragged or
/// undersized table.
#[wasm_bindgen]
pub fn stats_chi_square_independence_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        table: Vec<Vec<f64>>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if p.table.len() < 2 {
        return Err(JsValue::from_str("contingency table needs at least 2 rows"));
    }
    let refs: Vec<&[f64]> = p.table.iter().map(|r| r.as_slice()).collect();
    let r = crate::solvers::statistics::hypothesis::chi_square_independence(&refs).ok_or_else(
        || {
            JsValue::from_str(
                "chi-square independence requires a rectangular table that is at least 2x2 with grand total > 0",
            )
        },
    )?;
    #[derive(Serialize)]
    struct Out {
        statistic: f64,
        p_value: f64,
        dof: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        statistic: r.statistic,
        p_value: r.p_value,
        dof: r.dof,
    })?)
}

/// Simple (one-predictor) OLS linear regression of `y` on `x`. Input
/// `{ x:[..], y:[..] }` (equal length, n ≥ 3, x not constant) →
/// `{ slope, intercept, r_squared, residual_std_error, slope_std_error, slope_t,
/// slope_p_value, intercept_std_error, intercept_p_value, n }`. Errors on length
/// mismatch, n < 3, or zero-variance `x`.
#[wasm_bindgen]
pub fn stats_linear_regression_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        x: Vec<f64>,
        y: Vec<f64>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if p.x.len() != p.y.len() {
        return Err(JsValue::from_str("x and y must have equal length"));
    }
    let r = crate::solvers::statistics::regression::simple_linear_regression(&p.x, &p.y)
        .ok_or_else(|| {
            JsValue::from_str(
                "linear regression requires equal-length x,y with n >= 3 and non-constant x",
            )
        })?;
    #[derive(Serialize)]
    struct Out {
        slope: f64,
        intercept: f64,
        r_squared: f64,
        residual_std_error: f64,
        slope_std_error: f64,
        slope_t: f64,
        slope_p_value: f64,
        intercept_std_error: f64,
        intercept_p_value: f64,
        n: usize,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        slope: r.slope,
        intercept: r.intercept,
        r_squared: r.r_squared,
        residual_std_error: r.residual_std_error,
        slope_std_error: r.slope_std_error,
        slope_t: r.slope_t,
        slope_p_value: r.slope_p_value,
        intercept_std_error: r.intercept_std_error,
        intercept_p_value: r.intercept_p_value,
        n: r.n,
    })?)
}

/// Normal (Gaussian) distribution pdf/cdf/quantile at one point. Input
/// `{ x:f64, mu?:f64, sigma?:f64, p?:f64 }` (`mu` defaults 0, `sigma` defaults 1,
/// must be > 0) → `{ pdf, cdf, quantile }`. `pdf`/`cdf` are evaluated at `x`;
/// `quantile` is `Φ⁻¹(p)` when `p` is supplied (0<p<1), else `null`.
#[wasm_bindgen]
pub fn stats_normal_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        x: f64,
        #[serde(default)]
        mu: f64,
        #[serde(default = "default_one")]
        sigma: f64,
        #[serde(default)]
        p: Option<f64>,
    }
    let inp: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if !(inp.sigma > 0.0) {
        return Err(JsValue::from_str("sigma must be > 0"));
    }
    use crate::solvers::statistics::distributions::normal;
    let pdf = normal::pdf(inp.x, inp.mu, inp.sigma);
    let cdf = normal::cdf(inp.x, inp.mu, inp.sigma);
    let quantile = match inp.p {
        Some(p) => {
            if !(p > 0.0 && p < 1.0) {
                return Err(JsValue::from_str("p must be in the open interval (0,1)"));
            }
            Some(normal::quantile(p, inp.mu, inp.sigma))
        }
        None => None,
    };
    #[derive(Serialize)]
    struct Out {
        pdf: f64,
        cdf: f64,
        quantile: Option<f64>,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { pdf, cdf, quantile })?)
}

/// Student's t-distribution pdf/cdf at `t` with `nu` degrees of freedom, plus the
/// two-sided p-value. Input `{ t:f64, nu:f64, p?:f64 }` (`nu` > 0) →
/// `{ pdf, cdf, two_sided_p, quantile }`. `quantile` is the inverse-cdf at `p`
/// when supplied (0<p<1), else `null`.
#[wasm_bindgen]
pub fn stats_students_t_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        t: f64,
        nu: f64,
        #[serde(default)]
        p: Option<f64>,
    }
    let inp: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if !(inp.nu > 0.0) {
        return Err(JsValue::from_str("nu (degrees of freedom) must be > 0"));
    }
    use crate::solvers::statistics::distributions::students_t;
    let pdf = students_t::pdf(inp.t, inp.nu);
    let cdf = students_t::cdf(inp.t, inp.nu);
    let two_sided_p = students_t::two_sided_p(inp.t, inp.nu);
    let quantile = match inp.p {
        Some(p) => {
            if !(p > 0.0 && p < 1.0) {
                return Err(JsValue::from_str("p must be in the open interval (0,1)"));
            }
            Some(students_t::quantile(p, inp.nu))
        }
        None => None,
    };
    #[derive(Serialize)]
    struct Out {
        pdf: f64,
        cdf: f64,
        two_sided_p: f64,
        quantile: Option<f64>,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        pdf,
        cdf,
        two_sided_p,
        quantile,
    })?)
}

/// χ² (chi-squared) distribution pdf/cdf at `x` with `k` degrees of freedom, plus
/// the upper-tail p-value. Input `{ x:f64, k:f64, p?:f64 }` (`k` > 0, `x` ≥ 0) →
/// `{ pdf, cdf, upper_p, quantile }`. `quantile` is the inverse-cdf at `p` when
/// supplied (0<p<1), else `null`.
#[wasm_bindgen]
pub fn stats_chi_squared_dist_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        x: f64,
        k: f64,
        #[serde(default)]
        p: Option<f64>,
    }
    let inp: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if !(inp.k > 0.0) {
        return Err(JsValue::from_str("k (degrees of freedom) must be > 0"));
    }
    use crate::solvers::statistics::distributions::chi_squared;
    let pdf = chi_squared::pdf(inp.x, inp.k);
    let cdf = chi_squared::cdf(inp.x, inp.k);
    let upper_p = chi_squared::upper_p(inp.x, inp.k);
    let quantile = match inp.p {
        Some(p) => {
            if !(p > 0.0 && p < 1.0) {
                return Err(JsValue::from_str("p must be in the open interval (0,1)"));
            }
            Some(chi_squared::quantile(p, inp.k))
        }
        None => None,
    };
    #[derive(Serialize)]
    struct Out {
        pdf: f64,
        cdf: f64,
        upper_p: f64,
        quantile: Option<f64>,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        pdf,
        cdf,
        upper_p,
        quantile,
    })?)
}

fn default_true() -> bool {
    true
}

fn default_one() -> f64 {
    1.0
}
/// McNemar's test for two paired binary classifiers. Input `{ b, c }` — the
/// discordant counts (b = first right / second wrong, c = first wrong / second
/// right) — → `{ statistic, p_value, dof }`. Continuity-corrected χ², dof 1.
#[wasm_bindgen]
pub fn stats_mcnemar_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        b: u64,
        c: u64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let r = crate::solvers::statistics::hypothesis::mcnemar(p.b, p.c)
        .ok_or_else(|| JsValue::from_str("mcnemar requires b + c > 0"))?;
    #[derive(Serialize)]
    struct Out {
        statistic: f64,
        p_value: f64,
        dof: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        statistic: r.statistic,
        p_value: r.p_value,
        dof: r.dof,
    })?)
}

/// Friedman test for k treatments across n blocks (e.g. classifiers × datasets).
/// Input `{ blocks:[[m1,…,mk], …] }` (each block length k, higher = better) →
/// `{ chi_square, chi_p_value, df, iman_davenport_f, f_p_value }`.
#[wasm_bindgen]
pub fn stats_friedman_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        blocks: Vec<Vec<f64>>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if p.blocks.len() < 2 {
        return Err(JsValue::from_str("friedman needs >= 2 blocks"));
    }
    let rows: Vec<&[f64]> = p.blocks.iter().map(|b| b.as_slice()).collect();
    let r = crate::solvers::statistics::hypothesis::friedman(&rows).ok_or_else(|| {
        JsValue::from_str("friedman requires >= 2 blocks of equal length k >= 2")
    })?;
    #[derive(Serialize)]
    struct Out {
        chi_square: f64,
        chi_p_value: f64,
        df: f64,
        iman_davenport_f: f64,
        f_p_value: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        chi_square: r.chi_square,
        chi_p_value: r.chi_p_value,
        df: r.df,
        iman_davenport_f: r.iman_davenport_f,
        f_p_value: r.f_p_value,
    })?)
}

/// Fisher–Snedecor F-distribution: pdf and cdf at x with (d1, d2) degrees of
/// freedom, plus the inverse-cdf quantile when an optional `p` is supplied.
/// Input `{ x, d1, d2, p? }` → `{ pdf, cdf, quantile? }`.
#[wasm_bindgen]
pub fn stats_fisher_f_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        x: f64,
        d1: f64,
        d2: f64,
        #[serde(default)]
        p: Option<f64>,
    }
    let inp: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if inp.d1 <= 0.0 || inp.d2 <= 0.0 {
        return Err(JsValue::from_str("d1 and d2 must be positive"));
    }
    use crate::solvers::statistics::distributions::fisher_f as f;
    let quantile = inp.p.map(|p| f::quantile(p, inp.d1, inp.d2));
    #[derive(Serialize)]
    struct Out {
        pdf: f64,
        cdf: f64,
        quantile: Option<f64>,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        pdf: f::pdf(inp.x, inp.d1, inp.d2),
        cdf: f::cdf(inp.x, inp.d1, inp.d2),
        quantile,
    })?)
}
