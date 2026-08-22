//! Time-series, robust, information, and anomaly invoke seams.

use super::super::args;
use crate::solvers::statistics;
use vibe::{Diagnostic, Span, Value};

/// `Statistics.autocorrelation` — autocorrelation at a given lag.
/// Args: { values: [f64], lag: u64 }
pub fn autocorrelation(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Statistics.autocorrelation needs values"))?;
    let lag = args::rec_u64(args, "lag")
        .ok_or_else(|| args::bad(span, "Statistics.autocorrelation needs lag"))?
        as usize;
    statistics::timeseries::autocorrelation(&xs, lag)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "autocorrelation undefined"))
}

/// `Statistics.moving_average` — simple moving average.
/// Args: { values: [f64], window: u64 }
pub fn moving_average(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Statistics.moving_average needs values"))?;
    let window = args::rec_u64(args, "window")
        .ok_or_else(|| args::bad(span, "Statistics.moving_average needs window"))?
        as usize;
    let mut out = vec![0.0f64; xs.len()];
    match statistics::timeseries::moving_average_into(&xs, window, &mut out) {
        Some(_) => Ok(args::record([
            (
                "values",
                Value::List(out.iter().map(|v| Value::F64(*v)).collect()),
            ),
            ("n", Value::U64(xs.len() as u64)),
        ])),
        None => Err(args::bad(span, "moving_average: invalid window")),
    }
}

/// `Statistics.exponential_smoothing` — single exponential smoothing.
/// Args: { values: [f64], alpha: f64 }
pub fn exponential_smoothing(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Statistics.exponential_smoothing needs values"))?;
    let alpha = args::rec_f64(args, "alpha")
        .ok_or_else(|| args::bad(span, "Statistics.exponential_smoothing needs alpha"))?;
    let mut out = vec![0.0f64; xs.len()];
    match statistics::timeseries::exponential_smoothing_into(&xs, alpha, &mut out) {
        Some(_) => Ok(args::record([
            (
                "values",
                Value::List(out.iter().map(|v| Value::F64(*v)).collect()),
            ),
            ("n", Value::U64(xs.len() as u64)),
        ])),
        None => Err(args::bad(
            span,
            "exponential_smoothing: alpha must be in (0,1]",
        )),
    }
}

/// `Statistics.trimmed_mean` — trimmed mean (discard proportion from each tail).
/// Args: { values: [f64], proportion: f64 }
pub fn trimmed_mean(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Statistics.trimmed_mean needs values"))?;
    let proportion = args::rec_f64(args, "proportion")
        .ok_or_else(|| args::bad(span, "Statistics.trimmed_mean needs proportion"))?;
    statistics::robust::trimmed_mean(&xs, proportion)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "trimmed_mean undefined"))
}

/// `Statistics.iqr` — interquartile range.
/// Args: { values: [f64] }
pub fn iqr(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Statistics.iqr needs values"))?;
    statistics::robust::iqr(&xs)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "iqr undefined"))
}

/// `Statistics.median_abs_deviation` — MAD (scaled to estimate std dev if scaled=true).
/// Args: { values: [f64], scaled?: bool }
pub fn median_abs_deviation(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Statistics.median_abs_deviation needs values"))?;
    let scaled = args::rec_bool(args, "scaled").unwrap_or(false);
    statistics::robust::median_abs_deviation(&xs, scaled)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "median_abs_deviation undefined"))
}

/// `Statistics.entropy` — Shannon entropy of a probability distribution.
/// Args: { p: [f64] }
pub fn entropy(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = args::rec_f64_list(args, "p")
        .ok_or_else(|| args::bad(span, "Statistics.entropy needs p"))?;
    statistics::information::entropy(&p)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "entropy: invalid distribution"))
}

/// `Statistics.kl_divergence` — KL divergence D(p || q).
/// Args: { p: [f64], q: [f64] }
pub fn kl_divergence(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = args::rec_f64_list(args, "p")
        .ok_or_else(|| args::bad(span, "Statistics.kl_divergence needs p"))?;
    let q = args::rec_f64_list(args, "q")
        .ok_or_else(|| args::bad(span, "Statistics.kl_divergence needs q"))?;
    statistics::information::kl_divergence(&p, &q)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "kl_divergence: invalid distributions"))
}

/// `Statistics.z_score_outliers` — indices of outliers by z-score threshold.
/// Args: { values: [f64], threshold: f64 }
pub fn z_score_outliers(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Statistics.z_score_outliers needs values"))?;
    let threshold = args::rec_f64(args, "threshold")
        .ok_or_else(|| args::bad(span, "Statistics.z_score_outliers needs threshold"))?;
    match statistics::anomaly::z_score_outliers(&xs, threshold) {
        Some(indices) => Ok(args::record([
            (
                "outlier_indices",
                Value::List(indices.iter().map(|i| Value::U64(*i as u64)).collect()),
            ),
            ("count", Value::U64(indices.len() as u64)),
        ])),
        None => Err(args::bad(span, "z_score_outliers: insufficient data")),
    }
}
