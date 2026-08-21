//! Additional statistics invoke seams — histogram, robust extras, information
//! extras, anomaly extras, nonparametric tests, bootstrap, time-series
//! diagnostics, correlation p-value, chi-square independence.

use super::super::args;
use crate::solvers::statistics;
use poet_vibe::{Diagnostic, Span, Value};

/// `Statistics.mode` — modal value of a list.
/// Args: { values: [f64] }
pub fn mode(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut xs = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Statistics.mode needs values"))?;
    match statistics::descriptive::mode_in_place(&mut xs) {
        Some((value, count)) => Ok(args::record([
            ("value", Value::F64(value)),
            ("count", Value::U64(count as u64)),
        ])),
        None => Err(args::bad(span, "mode of empty list")),
    }
}

/// `Statistics.winsorized_mean` — winsorized mean (clamp extremes).
/// Args: { values: [f64], proportion: f64 }
pub fn winsorized_mean(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Statistics.winsorized_mean needs values"))?;
    let proportion = args::rec_f64(args, "proportion")
        .ok_or_else(|| args::bad(span, "Statistics.winsorized_mean needs proportion"))?;
    statistics::robust::winsorized_mean(&xs, proportion)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "winsorized_mean undefined"))
}

/// `Statistics.cross_entropy` — cross-entropy H(p, q).
/// Args: { p: [f64], q: [f64] }
pub fn cross_entropy(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = args::rec_f64_list(args, "p")
        .ok_or_else(|| args::bad(span, "Statistics.cross_entropy needs p"))?;
    let q = args::rec_f64_list(args, "q")
        .ok_or_else(|| args::bad(span, "Statistics.cross_entropy needs q"))?;
    statistics::information::cross_entropy(&p, &q)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "cross_entropy: invalid distributions"))
}

/// `Statistics.mutual_information` — mutual information between two discrete
/// variables.
/// Args: { x: [u64], y: [u64] }
pub fn mutual_information(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x_u64 = args::rec_u64_list(args, "x")
        .ok_or_else(|| args::bad(span, "Statistics.mutual_information needs x"))?;
    let y_u64 = args::rec_u64_list(args, "y")
        .ok_or_else(|| args::bad(span, "Statistics.mutual_information needs y"))?;
    let x: Vec<usize> = x_u64.iter().map(|v| *v as usize).collect();
    let y: Vec<usize> = y_u64.iter().map(|v| *v as usize).collect();
    statistics::information::mutual_information_discrete(&x, &y)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "mutual_information: invalid input"))
}

/// `Statistics.histogram` — histogram with equal-width bins.
/// Args: { values: [f64], bins: u64 }
pub fn histogram(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Statistics.histogram needs values"))?;
    let bins = args::rec_u64(args, "bins")
        .ok_or_else(|| args::bad(span, "Statistics.histogram needs bins"))? as usize;
    if bins == 0 || bins > 256 {
        return Err(args::bad(span, "histogram: bins must be 1..=256"));
    }
    let mut counts = vec![0u32; bins];
    match statistics::histogram::histogram_into(&xs, &mut counts) {
        Some(range) => Ok(args::record([
            (
                "counts",
                Value::List(counts.iter().map(|c| Value::U64(*c as u64)).collect()),
            ),
            ("min", Value::F64(range.min)),
            ("max", Value::F64(range.max)),
            ("bin_width", Value::F64(range.bin_width)),
            ("bins", Value::U64(bins as u64)),
        ])),
        None => Err(args::bad(span, "histogram: insufficient data")),
    }
}

/// `Statistics.correlation_p_value` — p-value for a correlation coefficient.
/// Args: { r: f64, n: u64 }
pub fn correlation_p_value(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let r = args::rec_f64(args, "r")
        .ok_or_else(|| args::bad(span, "Statistics.correlation_p_value needs r"))?;
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "Statistics.correlation_p_value needs n"))?
        as usize;
    statistics::correlation::correlation_p_value(r, n)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "correlation_p_value: invalid input"))
}

/// `Statistics.chi_square_independence` — chi-square test of independence on a
/// contingency table.
/// Args: { table: [[f64]] }
pub fn chi_square_independence(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let table_val = args::rec(args, "table")
        .ok_or_else(|| args::bad(span, "Statistics.chi_square_independence needs table"))?;
    let rows = match table_val {
        Value::List(l) => l,
        _ => {
            return Err(args::bad(
                span,
                "chi_square_independence: table must be a list of lists",
            ))
        }
    };
    let mut table: Vec<Vec<f64>> = Vec::new();
    for row in rows {
        let vals = args::f64s(&row)
            .ok_or_else(|| args::bad(span, "chi_square_independence: each row must be numbers"))?;
        table.push(vals);
    }
    let refs: Vec<&[f64]> = table.iter().map(|r| r.as_slice()).collect();
    match statistics::hypothesis::chi_square::chi_square_independence(&refs) {
        Some(r) => Ok(args::record([
            ("chi_square", Value::F64(r.statistic)),
            ("p_value", Value::F64(r.p_value)),
            ("degrees_of_freedom", Value::F64(r.dof)),
        ])),
        None => Err(args::bad(span, "chi_square_independence: invalid table")),
    }
}

// ── Anomaly detection extras ────────────────────────────────────────

/// `Statistics.modified_z_score_outliers` — modified z-score outlier indices.
/// Args: { values: [f64], threshold: f64 }
pub fn modified_z_score_outliers(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Statistics.modified_z_score_outliers needs values"))?;
    let threshold = args::rec_f64(args, "threshold")
        .ok_or_else(|| args::bad(span, "Statistics.modified_z_score_outliers needs threshold"))?;
    match statistics::anomaly::modified_z_score_outliers(&xs, threshold) {
        Some(indices) => Ok(args::record([
            (
                "outlier_indices",
                Value::List(indices.iter().map(|i| Value::U64(*i as u64)).collect()),
            ),
            ("count", Value::U64(indices.len() as u64)),
        ])),
        None => Err(args::bad(
            span,
            "modified_z_score_outliers: insufficient data",
        )),
    }
}

/// `Statistics.iqr_outliers` — IQR (Tukey) outlier indices.
/// Args: { values: [f64], k: f64 }
pub fn iqr_outliers(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Statistics.iqr_outliers needs values"))?;
    let k = args::rec_f64(args, "k")
        .ok_or_else(|| args::bad(span, "Statistics.iqr_outliers needs k"))?;
    match statistics::anomaly::iqr_outliers(&xs, k) {
        Some(indices) => Ok(args::record([
            (
                "outlier_indices",
                Value::List(indices.iter().map(|i| Value::U64(*i as u64)).collect()),
            ),
            ("count", Value::U64(indices.len() as u64)),
        ])),
        None => Err(args::bad(span, "iqr_outliers: insufficient data")),
    }
}

/// `Statistics.grubbs_test` — Grubbs' test for a single outlier.
/// Args: { values: [f64], alpha: f64 }
pub fn grubbs_test(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Statistics.grubbs_test needs values"))?;
    let alpha = args::rec_f64(args, "alpha")
        .ok_or_else(|| args::bad(span, "Statistics.grubbs_test needs alpha"))?;
    match statistics::anomaly::grubbs_test(&xs, alpha) {
        Some(r) => Ok(args::record([
            ("g_statistic", Value::F64(r.statistic)),
            ("critical_value", Value::F64(r.critical)),
            ("is_outlier", Value::Bool(r.is_outlier)),
            ("index", Value::U64(r.index as u64)),
        ])),
        None => Err(args::bad(span, "grubbs_test: insufficient data")),
    }
}

// ── Nonparametric tests ─────────────────────────────────────────────

/// `Statistics.mann_whitney_u` — Mann-Whitney U test.
/// Args: { x: [f64], y: [f64] }
pub fn mann_whitney_u(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64_list(args, "x")
        .ok_or_else(|| args::bad(span, "Statistics.mann_whitney_u needs x"))?;
    let y = args::rec_f64_list(args, "y")
        .ok_or_else(|| args::bad(span, "Statistics.mann_whitney_u needs y"))?;
    match statistics::hypothesis::nonparametric::mann_whitney_u(&x, &y) {
        Some(r) => Ok(args::record([
            ("u_statistic", Value::F64(r.u)),
            ("p_value", Value::F64(r.p_value)),
            ("n1", Value::U64(r.n1 as u64)),
            ("n2", Value::U64(r.n2 as u64)),
        ])),
        None => Err(args::bad(span, "mann_whitney_u: insufficient data")),
    }
}

/// `Statistics.ks_1sample` — one-sample Kolmogorov-Smirnov test.
/// Args: { values: [f64] }
pub fn ks_1sample(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Statistics.ks_1sample needs values"))?;
    match statistics::hypothesis::nonparametric::ks_1sample(&xs) {
        Some(r) => Ok(args::record([
            ("d_statistic", Value::F64(r.d)),
            ("p_value", Value::F64(r.p_value)),
        ])),
        None => Err(args::bad(span, "ks_1sample: insufficient data")),
    }
}

/// `Statistics.friedman` — Friedman test for related samples.
/// Args: { groups: [[f64]] }
pub fn friedman(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let groups_val = args::rec(args, "groups")
        .ok_or_else(|| args::bad(span, "Statistics.friedman needs groups"))?;
    let groups_list = match groups_val {
        Value::List(l) => l,
        _ => return Err(args::bad(span, "friedman: groups must be a list of lists")),
    };
    let mut groups: Vec<Vec<f64>> = Vec::new();
    for g in groups_list {
        let vals = args::f64s(&g)
            .ok_or_else(|| args::bad(span, "friedman: each group must be a number list"))?;
        groups.push(vals);
    }
    let refs: Vec<&[f64]> = groups.iter().map(|g| g.as_slice()).collect();
    match statistics::hypothesis::nonparametric::friedman(&refs) {
        Some(r) => Ok(args::record([
            ("chi_square", Value::F64(r.chi_square)),
            ("chi_p_value", Value::F64(r.chi_p_value)),
            ("df", Value::F64(r.df)),
            ("iman_davenport_f", Value::F64(r.iman_davenport_f)),
            ("f_p_value", Value::F64(r.f_p_value)),
        ])),
        None => Err(args::bad(span, "friedman: insufficient data")),
    }
}

/// `Statistics.mcnemar` — McNemar's test for paired nominal data.
/// Args: { b: u64, c: u64 }
pub fn mcnemar(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let b =
        args::rec_u64(args, "b").ok_or_else(|| args::bad(span, "Statistics.mcnemar needs b"))?;
    let c =
        args::rec_u64(args, "c").ok_or_else(|| args::bad(span, "Statistics.mcnemar needs c"))?;
    match statistics::hypothesis::nonparametric::mcnemar(b, c) {
        Some(r) => Ok(args::record([
            ("chi_square", Value::F64(r.statistic)),
            ("p_value", Value::F64(r.p_value)),
            ("df", Value::F64(r.dof)),
        ])),
        None => Err(args::bad(span, "mcnemar: b + c must be > 0")),
    }
}

// ── Bootstrap ───────────────────────────────────────────────────────

/// `Statistics.bootstrap_means` — bootstrap distribution of the mean.
/// Args: { values: [f64], iterations: u64, seed: u64 }
pub fn bootstrap_means(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Statistics.bootstrap_means needs values"))?;
    let iterations = args::rec_u64(args, "iterations").unwrap_or(1000) as usize;
    let seed = args::rec_u64(args, "seed").unwrap_or(42);
    let mut out = vec![0.0f64; iterations];
    match statistics::bootstrap_means(&xs, iterations, seed, &mut out) {
        Ok(n) => Ok(args::record([
            (
                "means",
                Value::List(out[..n].iter().map(|v| Value::F64(*v)).collect()),
            ),
            ("n", Value::U64(n as u64)),
        ])),
        Err(_) => Err(args::bad(span, "bootstrap_means: invalid parameters")),
    }
}

// ── Time-series diagnostics ─────────────────────────────────────────

/// `Statistics.ljung_box` — Ljung-Box test for autocorrelation.
/// Args: { acf: [f64], n: u64, h: u64 }
pub fn ljung_box(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let acf = args::rec_f64_list(args, "acf")
        .ok_or_else(|| args::bad(span, "Statistics.ljung_box needs acf"))?;
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "Statistics.ljung_box needs n"))? as usize;
    let h = args::rec_u64(args, "h")
        .ok_or_else(|| args::bad(span, "Statistics.ljung_box needs h"))? as usize;
    Ok(Value::F64(statistics::ljung_box(&acf, n, h)))
}

/// `Statistics.adf_proxy` — augmented Dickey-Fuller proxy (stationarity).
/// Args: { series: [f64] }
pub fn adf_proxy(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let series = args::rec_f64_list(args, "series")
        .ok_or_else(|| args::bad(span, "Statistics.adf_proxy needs series"))?;
    Ok(Value::F64(statistics::adf_proxy(&series)))
}
