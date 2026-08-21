//! Additional statistics invoke seams — remaining distribution CDFs/quantiles,
//! special functions, multivariate normal, and extra descriptive/anomaly
//! functions.

use super::super::args;
use crate::solvers::statistics as stats;
use poet_vibe::{Diagnostic, Span, Value};

// ── Descriptive extras ──────────────────────────────────────────────

/// `Statistics.argmax` — index of the maximum value.
/// Args: { values: [f64] }
pub fn argmax(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs =
        args::rec_f64_list(args, "values").ok_or_else(|| args::bad(span, "argmax needs values"))?;
    match stats::descriptive::argmax(&xs) {
        Some(idx) => Ok(args::record([
            ("index", Value::U64(idx as u64)),
            ("value", Value::F64(xs[idx])),
        ])),
        None => Err(args::bad(span, "argmax: empty input")),
    }
}

/// `Statistics.standard_pdf` — standard normal PDF.
/// Args: { z: f64 }
pub fn standard_pdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let z = args::rec_f64(args, "z").ok_or_else(|| args::bad(span, "standard_pdf needs z"))?;
    Ok(Value::F64(stats::distributions::normal::standard_pdf(z)))
}

/// `Statistics.standard_quantile` — standard normal quantile (p → z).
/// Args: { p: f64 }
pub fn standard_quantile(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = args::rec_f64(args, "p").ok_or_else(|| args::bad(span, "standard_quantile needs p"))?;
    Ok(Value::F64(stats::distributions::normal::standard_quantile(
        p,
    )))
}

// ── Distribution CDFs and quantiles ─────────────────────────────────

/// `Statistics.lognormal_cdf` — lognormal CDF.
/// Args: { x: f64, mu: f64, sigma: f64 }
pub fn lognormal_cdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "lognormal_cdf needs x"))?;
    let mu = args::rec_f64(args, "mu").unwrap_or(0.0);
    let sigma = args::rec_f64(args, "sigma").unwrap_or(1.0);
    Ok(Value::F64(stats::distributions::lognormal_cdf(
        x, mu, sigma,
    )))
}

/// `Statistics.uniform_cdf` — uniform CDF.
/// Args: { x: f64, a: f64, b: f64 }
pub fn uniform_cdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "uniform_cdf needs x"))?;
    let a = args::rec_f64(args, "a").unwrap_or(0.0);
    let b = args::rec_f64(args, "b").unwrap_or(1.0);
    Ok(Value::F64(stats::distributions::uniform_cdf(x, a, b)))
}

/// `Statistics.laplace_cdf` — Laplace CDF.
/// Args: { x: f64, mu: f64, b: f64 }
pub fn laplace_cdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "laplace_cdf needs x"))?;
    let mu = args::rec_f64(args, "mu").unwrap_or(0.0);
    let b = args::rec_f64(args, "b").unwrap_or(1.0);
    Ok(Value::F64(stats::distributions::laplace_cdf(x, mu, b)))
}

/// `Statistics.students_t_quantile` — Student's t quantile.
/// Args: { p: f64, nu: f64 }
pub fn students_t_quantile(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p =
        args::rec_f64(args, "p").ok_or_else(|| args::bad(span, "students_t_quantile needs p"))?;
    let nu =
        args::rec_f64(args, "nu").ok_or_else(|| args::bad(span, "students_t_quantile needs nu"))?;
    Ok(Value::F64(stats::distributions::students_t::quantile(
        p, nu,
    )))
}

/// `Statistics.students_t_upper_p` — Student's t upper tail probability.
/// Args: { t: f64, nu: f64 }
pub fn students_t_upper_p(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let t =
        args::rec_f64(args, "t").ok_or_else(|| args::bad(span, "students_t_upper_p needs t"))?;
    let nu =
        args::rec_f64(args, "nu").ok_or_else(|| args::bad(span, "students_t_upper_p needs nu"))?;
    Ok(Value::F64(stats::distributions::students_t::upper_p(t, nu)))
}

/// `Statistics.chi_squared_quantile` — chi-squared quantile.
/// Args: { p: f64, k: f64 }
pub fn chi_squared_quantile(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p =
        args::rec_f64(args, "p").ok_or_else(|| args::bad(span, "chi_squared_quantile needs p"))?;
    let k =
        args::rec_f64(args, "k").ok_or_else(|| args::bad(span, "chi_squared_quantile needs k"))?;
    Ok(Value::F64(stats::distributions::chi_squared::quantile(
        p, k,
    )))
}

/// `Statistics.fisher_f_quantile` — Fisher F quantile.
/// Args: { p: f64, d1: f64, d2: f64 }
pub fn fisher_f_quantile(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = args::rec_f64(args, "p").ok_or_else(|| args::bad(span, "fisher_f_quantile needs p"))?;
    let d1 =
        args::rec_f64(args, "d1").ok_or_else(|| args::bad(span, "fisher_f_quantile needs d1"))?;
    let d2 =
        args::rec_f64(args, "d2").ok_or_else(|| args::bad(span, "fisher_f_quantile needs d2"))?;
    Ok(Value::F64(stats::distributions::fisher_f::quantile(
        p, d1, d2,
    )))
}

// ── Special functions ───────────────────────────────────────────────

/// `Statistics.gammp` — regularized lower incomplete gamma P(a, x).
/// Args: { a: f64, x: f64 }
pub fn gammp(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64(args, "a").ok_or_else(|| args::bad(span, "gammp needs a"))?;
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "gammp needs x"))?;
    Ok(Value::F64(stats::distributions::special::gammp(a, x)))
}

/// `Statistics.gammq` — regularized upper incomplete gamma Q(a, x).
/// Args: { a: f64, x: f64 }
pub fn gammq(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64(args, "a").ok_or_else(|| args::bad(span, "gammq needs a"))?;
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "gammq needs x"))?;
    Ok(Value::F64(stats::distributions::special::gammq(a, x)))
}

/// `Statistics.betai` — regularized incomplete beta I_x(a, b).
/// Args: { a: f64, b: f64, x: f64 }
pub fn betai(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64(args, "a").ok_or_else(|| args::bad(span, "betai needs a"))?;
    let b = args::rec_f64(args, "b").ok_or_else(|| args::bad(span, "betai needs b"))?;
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "betai needs x"))?;
    Ok(Value::F64(stats::distributions::special::betai(a, b, x)))
}

// ── Information theory extras ───────────────────────────────────────

/// `Statistics.entropy_from_counts` — Shannon entropy from integer counts.
/// Args: { counts: [u64] }
pub fn entropy_from_counts(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let counts_u64 = args::rec_u64_list(args, "counts")
        .ok_or_else(|| args::bad(span, "entropy_from_counts needs counts"))?;
    let counts: Vec<usize> = counts_u64.iter().map(|&c| c as usize).collect();
    match stats::information::entropy_from_counts(&counts) {
        Some(h) => Ok(Value::F64(h)),
        None => Err(args::bad(span, "entropy_from_counts: invalid counts")),
    }
}

// ── Anomaly extras ──────────────────────────────────────────────────

/// `Statistics.tukey_fences` — Tukey fence bounds (k=1.5 default).
/// Args: { values: [f64], k: f64 }
pub fn tukey_fences(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "tukey_fences needs values"))?;
    let k = args::rec_f64(args, "k").unwrap_or(1.5);
    match stats::anomaly::tukey_fences(&xs, k) {
        Some((lo, hi)) => Ok(args::record([
            ("lower", Value::F64(lo)),
            ("upper", Value::F64(hi)),
        ])),
        None => Err(args::bad(span, "tukey_fences: insufficient data")),
    }
}

/// `Statistics.mahalanobis_sq` — squared Mahalanobis distance.
/// Args: { x: [f64], mean: [f64], inv_cov: [f64] }
pub fn mahalanobis_sq(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x =
        args::rec_f64_list(args, "x").ok_or_else(|| args::bad(span, "mahalanobis_sq needs x"))?;
    let mean = args::rec_f64_list(args, "mean")
        .ok_or_else(|| args::bad(span, "mahalanobis_sq needs mean"))?;
    let inv_cov = args::rec_f64_list(args, "inv_cov")
        .ok_or_else(|| args::bad(span, "mahalanobis_sq needs inv_cov"))?;
    match stats::anomaly::mahalanobis_sq(&x, &mean, &inv_cov) {
        Some(d2) => Ok(Value::F64(d2)),
        None => Err(args::bad(span, "mahalanobis_sq: dimension mismatch")),
    }
}

// ── Multivariate normal ─────────────────────────────────────────────

/// `Statistics.mvn_log_pdf` — multivariate normal log-density.
/// Args: { x: [f64], mean: [f64], cov: [f64], p: u64 }
pub fn mvn_log_pdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64_list(args, "x").ok_or_else(|| args::bad(span, "mvn_log_pdf needs x"))?;
    let mean = args::rec_f64_list(args, "mean")
        .ok_or_else(|| args::bad(span, "mvn_log_pdf needs mean"))?;
    let cov =
        args::rec_f64_list(args, "cov").ok_or_else(|| args::bad(span, "mvn_log_pdf needs cov"))?;
    let p =
        args::rec_u64(args, "p").ok_or_else(|| args::bad(span, "mvn_log_pdf needs p"))? as usize;
    match stats::distributions::multivariate_normal::log_pdf(&x, &mean, &cov, p) {
        Some(lp) => Ok(Value::F64(lp)),
        None => Err(args::bad(
            span,
            "mvn_log_pdf: shape mismatch or non-PD covariance",
        )),
    }
}

/// `Statistics.mvn_pdf` — multivariate normal density.
/// Args: { x: [f64], mean: [f64], cov: [f64], p: u64 }
pub fn mvn_pdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64_list(args, "x").ok_or_else(|| args::bad(span, "mvn_pdf needs x"))?;
    let mean =
        args::rec_f64_list(args, "mean").ok_or_else(|| args::bad(span, "mvn_pdf needs mean"))?;
    let cov =
        args::rec_f64_list(args, "cov").ok_or_else(|| args::bad(span, "mvn_pdf needs cov"))?;
    let p = args::rec_u64(args, "p").ok_or_else(|| args::bad(span, "mvn_pdf needs p"))? as usize;
    match stats::distributions::multivariate_normal::pdf(&x, &mean, &cov, p) {
        Some(p_val) => Ok(Value::F64(p_val)),
        None => Err(args::bad(
            span,
            "mvn_pdf: shape mismatch or non-PD covariance",
        )),
    }
}

/// `Statistics.mvn_sample` — draw one sample from a multivariate normal.
/// Args: { mean: [f64], cov: [f64], p: u64, seed: u64 }
pub fn mvn_sample(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mean =
        args::rec_f64_list(args, "mean").ok_or_else(|| args::bad(span, "mvn_sample needs mean"))?;
    let cov =
        args::rec_f64_list(args, "cov").ok_or_else(|| args::bad(span, "mvn_sample needs cov"))?;
    let p = args::rec_u64(args, "p").ok_or_else(|| args::bad(span, "mvn_sample needs p"))? as usize;
    let seed = args::rec_u64(args, "seed").unwrap_or(0);
    match stats::distributions::multivariate_normal::sample(&mean, &cov, p, seed) {
        Some(s) => Ok(args::f64_list_value(s)),
        None => Err(args::bad(
            span,
            "mvn_sample: shape mismatch or non-PD covariance",
        )),
    }
}

/// `Statistics.mvn_mle` — maximum-likelihood mean and covariance.
/// Args: { data: [f64], n: u64, p: u64 }
pub fn mvn_mle(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let data =
        args::rec_f64_list(args, "data").ok_or_else(|| args::bad(span, "mvn_mle needs data"))?;
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "mvn_mle needs n"))? as usize;
    let p = args::rec_u64(args, "p").ok_or_else(|| args::bad(span, "mvn_mle needs p"))? as usize;
    match stats::distributions::multivariate_normal::mle(&data, n, p) {
        Some((mean, cov)) => Ok(args::record([
            ("mean", args::f64_list_value(mean)),
            ("cov", args::f64_list_value(cov)),
        ])),
        None => Err(args::bad(
            span,
            "mvn_mle: insufficient data or shape mismatch",
        )),
    }
}
