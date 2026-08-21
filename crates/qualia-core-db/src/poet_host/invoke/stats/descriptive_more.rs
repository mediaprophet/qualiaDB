//! Descriptive statistics invoke seams — `solvers::statistics::descriptive`.

use super::super::args;
use crate::solvers::statistics::descriptive;
use poet_vibe::{Diagnostic, Span, Value};

/// `Statistics.median` — median of a number list.
pub fn median(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut xs = args::f64s(args)
        .or_else(|| args::rec(args, "values").and_then(args::f64s))
        .ok_or_else(|| args::bad(span, "Statistics.median needs a number list"))?;
    descriptive::median_in_place(&mut xs)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "median of empty list"))
}

/// `Statistics.variance` — variance of a number list.
/// Args: { values: [f64], sample?: bool }
pub fn variance(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::f64s(args)
        .or_else(|| args::rec(args, "values").and_then(args::f64s))
        .ok_or_else(|| args::bad(span, "Statistics.variance needs values"))?;
    let sample = args::rec_bool(args, "sample").unwrap_or(true);
    descriptive::variance(&xs, sample)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "variance of empty list"))
}

/// `Statistics.std_dev` — standard deviation of a number list.
/// Args: { values: [f64], sample?: bool }
pub fn std_dev(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::f64s(args)
        .or_else(|| args::rec(args, "values").and_then(args::f64s))
        .ok_or_else(|| args::bad(span, "Statistics.std_dev needs values"))?;
    let sample = args::rec_bool(args, "sample").unwrap_or(true);
    descriptive::std_dev(&xs, sample)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "std_dev of empty list"))
}

/// `Statistics.skewness` — sample skewness (Fisher-Pearson).
pub fn skewness(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::f64s(args)
        .or_else(|| args::rec(args, "values").and_then(args::f64s))
        .ok_or_else(|| args::bad(span, "Statistics.skewness needs values"))?;
    descriptive::skewness(&xs)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "skewness undefined"))
}

/// `Statistics.kurtosis` — excess kurtosis.
pub fn kurtosis(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::f64s(args)
        .or_else(|| args::rec(args, "values").and_then(args::f64s))
        .ok_or_else(|| args::bad(span, "Statistics.kurtosis needs values"))?;
    descriptive::kurtosis(&xs)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "kurtosis undefined"))
}

/// `Statistics.quantile` — quantile of a number list.
/// Args: { values: [f64], q: f64 }
pub fn quantile(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut xs = args::f64s(args)
        .or_else(|| args::rec(args, "values").and_then(args::f64s))
        .ok_or_else(|| args::bad(span, "Statistics.quantile needs values"))?;
    let q =
        args::rec_f64(args, "q").ok_or_else(|| args::bad(span, "Statistics.quantile needs q"))?;
    descriptive::quantile_in_place(&mut xs, q)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "quantile of empty list"))
}

/// `Statistics.covariance` — covariance between two lists.
/// Args: { x: [f64], y: [f64], sample?: bool }
pub fn covariance(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64_list(args, "x")
        .ok_or_else(|| args::bad(span, "Statistics.covariance needs x"))?;
    let y = args::rec_f64_list(args, "y")
        .ok_or_else(|| args::bad(span, "Statistics.covariance needs y"))?;
    let sample = args::rec_bool(args, "sample").unwrap_or(true);
    descriptive::covariance(&x, &y, sample)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "covariance undefined"))
}

/// `Statistics.min` — minimum of a list.
pub fn min(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::f64s(args)
        .or_else(|| args::rec(args, "values").and_then(args::f64s))
        .ok_or_else(|| args::bad(span, "Statistics.min needs values"))?;
    descriptive::min(&xs)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "min of empty list"))
}

/// `Statistics.max` — maximum of a list.
pub fn max(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::f64s(args)
        .or_else(|| args::rec(args, "values").and_then(args::f64s))
        .ok_or_else(|| args::bad(span, "Statistics.max needs values"))?;
    descriptive::max(&xs)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "max of empty list"))
}

/// `Statistics.sum` — sum of a list.
pub fn sum(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::f64s(args)
        .or_else(|| args::rec(args, "values").and_then(args::f64s))
        .ok_or_else(|| args::bad(span, "Statistics.sum needs values"))?;
    Ok(Value::F64(descriptive::sum(&xs)))
}
