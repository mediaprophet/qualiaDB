//! Additional ML invoke seams — classification metrics, resampling,
//! multiple testing, dimensionality, experiment design, KG embedding.

use super::super::args;
use crate::solvers::learning;
use poet_vibe::{Diagnostic, Span, Value};

/// `MachineLearning.log_loss` — logistic loss (cross-entropy).
/// Args: { probs: [f64], labels: [bool] }
pub fn log_loss(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let probs = args::rec_f64_list(args, "probs")
        .ok_or_else(|| args::bad(span, "MachineLearning.log_loss needs probs"))?;
    let labels = args::rec_bool_list(args, "labels")
        .ok_or_else(|| args::bad(span, "MachineLearning.log_loss needs labels"))?;
    learning::metrics::classification::log_loss(&probs, &labels)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "log_loss: invalid input"))
}

/// `MachineLearning.confusion_binary` — binary confusion matrix.
/// Args: { y_true: [bool], y_pred: [bool] }
pub fn confusion_binary(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let y_true = args::rec_bool_list(args, "y_true")
        .ok_or_else(|| args::bad(span, "MachineLearning.confusion_binary needs y_true"))?;
    let y_pred = args::rec_bool_list(args, "y_pred")
        .ok_or_else(|| args::bad(span, "MachineLearning.confusion_binary needs y_pred"))?;
    match learning::metrics::classification::confusion_binary(&y_true, &y_pred) {
        Some(c) => Ok(args::record([
            ("tp", Value::U64(c.tp)),
            ("fp", Value::U64(c.fp)),
            ("tn", Value::U64(c.tn)),
            ("fn", Value::U64(c.fn_)),
        ])),
        None => Err(args::bad(
            span,
            "confusion_binary: mismatched or empty arrays",
        )),
    }
}

/// `MachineLearning.k_fold` — k-fold cross-validation splits.
/// Args: { n: u64, k: u64, shuffle: bool, seed: u64 }
pub fn k_fold(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "MachineLearning.k_fold needs n"))? as usize;
    let k = args::rec_u64(args, "k")
        .ok_or_else(|| args::bad(span, "MachineLearning.k_fold needs k"))? as usize;
    let shuffle = args::rec_bool(args, "shuffle").unwrap_or(true);
    let seed = args::rec_u64(args, "seed").unwrap_or(42);
    let folds = learning::resampling::folds::k_fold(n, k, shuffle, seed);
    let fold_records: Vec<Value> = folds
        .iter()
        .map(|f| {
            args::record([
                (
                    "train",
                    Value::List(f.train.iter().map(|i| Value::U64(*i as u64)).collect()),
                ),
                (
                    "test",
                    Value::List(f.test.iter().map(|i| Value::U64(*i as u64)).collect()),
                ),
            ])
        })
        .collect();
    Ok(args::record([
        ("folds", Value::List(fold_records)),
        ("k", Value::U64(k as u64)),
    ]))
}

/// `MachineLearning.bootstrap_indices` — bootstrap sample indices.
/// Args: { n: u64, seed: u64 }
pub fn bootstrap_indices(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "MachineLearning.bootstrap_indices needs n"))?
        as usize;
    let seed = args::rec_u64(args, "seed").unwrap_or(42);
    let indices = learning::resampling::bootstrap::bootstrap_indices(n, seed);
    Ok(args::record([
        (
            "indices",
            Value::List(indices.iter().map(|i| Value::U64(*i as u64)).collect()),
        ),
        ("n", Value::U64(indices.len() as u64)),
    ]))
}

/// `MachineLearning.bonferroni` — Bonferroni correction.
/// Args: { p: [f64] }
pub fn bonferroni(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = args::rec_f64_list(args, "p")
        .ok_or_else(|| args::bad(span, "MachineLearning.bonferroni needs p"))?;
    let adjusted = learning::multiple_testing::bonferroni(&p);
    Ok(Value::List(
        adjusted.iter().map(|v| Value::F64(*v)).collect(),
    ))
}

/// `MachineLearning.holm` — Holm-Bonferroni correction.
/// Args: { p: [f64] }
pub fn holm(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = args::rec_f64_list(args, "p")
        .ok_or_else(|| args::bad(span, "MachineLearning.holm needs p"))?;
    let adjusted = learning::multiple_testing::holm(&p);
    Ok(Value::List(
        adjusted.iter().map(|v| Value::F64(*v)).collect(),
    ))
}

/// `MachineLearning.benjamini_hochberg` — BH FDR correction.
/// Args: { p: [f64] }
pub fn benjamini_hochberg(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = args::rec_f64_list(args, "p")
        .ok_or_else(|| args::bad(span, "MachineLearning.benjamini_hochberg needs p"))?;
    let adjusted = learning::multiple_testing::benjamini_hochberg(&p);
    Ok(Value::List(
        adjusted.iter().map(|v| Value::F64(*v)).collect(),
    ))
}

/// `MachineLearning.pca` — principal component analysis.
/// Args: { data: [f64] (row-major n×p), n: u64, p: u64 }
pub fn pca(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let data = args::rec_f64_list(args, "data")
        .ok_or_else(|| args::bad(span, "MachineLearning.pca needs data"))?;
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "MachineLearning.pca needs n"))? as usize;
    let p = args::rec_u64(args, "p")
        .ok_or_else(|| args::bad(span, "MachineLearning.pca needs p"))? as usize;
    if data.len() < n * p {
        return Err(args::bad(span, "pca: data must have at least n*p values"));
    }
    match learning::dimensionality::pca::fit(&data, n, p) {
        Ok(model) => Ok(args::record([
            (
                "mean",
                Value::List(model.mean.iter().map(|v| Value::F64(*v)).collect()),
            ),
            (
                "components",
                Value::List(model.components.iter().map(|v| Value::F64(*v)).collect()),
            ),
            (
                "explained_variance",
                Value::List(
                    model
                        .explained_variance
                        .iter()
                        .map(|v| Value::F64(*v))
                        .collect(),
                ),
            ),
            (
                "explained_variance_ratio",
                Value::List(
                    model
                        .explained_variance_ratio
                        .iter()
                        .map(|v| Value::F64(*v))
                        .collect(),
                ),
            ),
            ("n_components", Value::U64(model.n_components as u64)),
            ("p", Value::U64(model.p as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("pca: {e:?}"))),
    }
}

/// `MachineLearning.ab_test` — two-proportion z-test for A/B testing.
/// Args: { conv_a: u64, n_a: u64, conv_b: u64, n_b: u64, alpha: f64 }
pub fn ab_test(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let conv_a = args::rec_u64(args, "conv_a").unwrap_or(0);
    let n_a = args::rec_u64(args, "n_a").unwrap_or(0);
    let conv_b = args::rec_u64(args, "conv_b").unwrap_or(0);
    let n_b = args::rec_u64(args, "n_b").unwrap_or(0);
    let alpha = args::rec_f64(args, "alpha").unwrap_or(0.05);
    match learning::experiment::ab_test::ab_test(conv_a, n_a, conv_b, n_b, alpha) {
        Some(r) => Ok(args::record([
            ("rate_a", Value::F64(r.rate_a)),
            ("rate_b", Value::F64(r.rate_b)),
            ("difference", Value::F64(r.difference)),
            ("z_statistic", Value::F64(r.z_statistic)),
            ("p_value", Value::F64(r.p_value)),
            ("ci_lower", Value::F64(r.ci.0)),
            ("ci_upper", Value::F64(r.ci.1)),
            ("confidence", Value::F64(r.confidence)),
        ])),
        None => Err(args::bad(span, "ab_test: invalid counts")),
    }
}

/// `MachineLearning.power_two_sample` — statistical power of a two-sample test.
/// Args: { n: u64, d: f64, alpha: f64 }
pub fn power_two_sample(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "MachineLearning.power_two_sample needs n"))?
        as usize;
    let d = args::rec_f64(args, "d")
        .ok_or_else(|| args::bad(span, "MachineLearning.power_two_sample needs d"))?;
    let alpha = args::rec_f64(args, "alpha").unwrap_or(0.05);
    learning::experiment::power::power_two_sample(n, d, alpha)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "power_two_sample: invalid parameters"))
}

/// `MachineLearning.required_sample_size` — required n for a two-sample test.
/// Args: { d: f64, alpha: f64, power: f64 }
pub fn required_sample_size(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let d = args::rec_f64(args, "d")
        .ok_or_else(|| args::bad(span, "MachineLearning.required_sample_size needs d"))?;
    let alpha = args::rec_f64(args, "alpha").unwrap_or(0.05);
    let power = args::rec_f64(args, "power").unwrap_or(0.8);
    learning::experiment::power::required_sample_size_two_sample(d, alpha, power)
        .map(|n| Value::U64(n as u64))
        .ok_or_else(|| args::bad(span, "required_sample_size: invalid parameters"))
}

/// `MachineLearning.transe_score` — TransE knowledge graph score.
/// Args: { h: [f64], r: [f64], t: [f64], p: u64 }
pub fn transe_score(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let h = args::rec_f64_list(args, "h")
        .ok_or_else(|| args::bad(span, "MachineLearning.transe_score needs h"))?;
    let r = args::rec_f64_list(args, "r")
        .ok_or_else(|| args::bad(span, "MachineLearning.transe_score needs r"))?;
    let t = args::rec_f64_list(args, "t")
        .ok_or_else(|| args::bad(span, "MachineLearning.transe_score needs t"))?;
    let p = args::rec_u64(args, "p").unwrap_or(2) as u8;
    Ok(Value::F64(learning::kg_embedding::score::transe_score(
        &h, &r, &t, p,
    )))
}

/// `MachineLearning.distmult_score` — DistMult knowledge graph score.
/// Args: { h: [f64], r: [f64], t: [f64] }
pub fn distmult_score(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let h = args::rec_f64_list(args, "h")
        .ok_or_else(|| args::bad(span, "MachineLearning.distmult_score needs h"))?;
    let r = args::rec_f64_list(args, "r")
        .ok_or_else(|| args::bad(span, "MachineLearning.distmult_score needs r"))?;
    let t = args::rec_f64_list(args, "t")
        .ok_or_else(|| args::bad(span, "MachineLearning.distmult_score needs t"))?;
    Ok(Value::F64(learning::kg_embedding::score::distmult_score(
        &h, &r, &t,
    )))
}
