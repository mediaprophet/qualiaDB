//! ML metrics and utilities invoke seams — `solvers::learning`.

use super::super::args;
use crate::solvers::learning;
use poet_vibe::{Diagnostic, Span, Value};

/// `MachineLearning.mse` — mean squared error.
/// Args: { y_true: [f64], y_pred: [f64] }
pub fn mse(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let y_true = args::rec_f64_list(args, "y_true")
        .ok_or_else(|| args::bad(span, "MachineLearning.mse needs y_true"))?;
    let y_pred = args::rec_f64_list(args, "y_pred")
        .ok_or_else(|| args::bad(span, "MachineLearning.mse needs y_pred"))?;
    learning::metrics::regression::mse(&y_true, &y_pred)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "mse: mismatched or empty arrays"))
}

/// `MachineLearning.rmse` — root mean squared error.
/// Args: { y_true: [f64], y_pred: [f64] }
pub fn rmse(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let y_true = args::rec_f64_list(args, "y_true")
        .ok_or_else(|| args::bad(span, "MachineLearning.rmse needs y_true"))?;
    let y_pred = args::rec_f64_list(args, "y_pred")
        .ok_or_else(|| args::bad(span, "MachineLearning.rmse needs y_pred"))?;
    learning::metrics::regression::rmse(&y_true, &y_pred)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "rmse: mismatched or empty arrays"))
}

/// `MachineLearning.mae` — mean absolute error.
/// Args: { y_true: [f64], y_pred: [f64] }
pub fn mae(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let y_true = args::rec_f64_list(args, "y_true")
        .ok_or_else(|| args::bad(span, "MachineLearning.mae needs y_true"))?;
    let y_pred = args::rec_f64_list(args, "y_pred")
        .ok_or_else(|| args::bad(span, "MachineLearning.mae needs y_pred"))?;
    learning::metrics::regression::mae(&y_true, &y_pred)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "mae: mismatched or empty arrays"))
}

/// `MachineLearning.r2_score` — coefficient of determination R².
/// Args: { y_true: [f64], y_pred: [f64] }
pub fn r2_score(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let y_true = args::rec_f64_list(args, "y_true")
        .ok_or_else(|| args::bad(span, "MachineLearning.r2_score needs y_true"))?;
    let y_pred = args::rec_f64_list(args, "y_pred")
        .ok_or_else(|| args::bad(span, "MachineLearning.r2_score needs y_pred"))?;
    learning::metrics::regression::r2_score(&y_true, &y_pred)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "r2_score: mismatched or empty arrays"))
}

/// `MachineLearning.accuracy` — classification accuracy.
/// Args: { y_true: [u64], y_pred: [u64] }
pub fn accuracy(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let y_true = args::rec_u64_list(args, "y_true")
        .ok_or_else(|| args::bad(span, "MachineLearning.accuracy needs y_true"))?;
    let y_pred = args::rec_u64_list(args, "y_pred")
        .ok_or_else(|| args::bad(span, "MachineLearning.accuracy needs y_pred"))?;
    let t: Vec<usize> = y_true.iter().map(|v| *v as usize).collect();
    let p: Vec<usize> = y_pred.iter().map(|v| *v as usize).collect();
    learning::metrics::classification::accuracy(&t, &p)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "accuracy: mismatched or empty arrays"))
}

/// `MachineLearning.roc_auc` — ROC AUC score.
/// Args: { scores: [f64], labels: [bool] }
pub fn roc_auc(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let scores = args::rec_f64_list(args, "scores")
        .ok_or_else(|| args::bad(span, "MachineLearning.roc_auc needs scores"))?;
    let labels = args::rec_bool_list(args, "labels")
        .ok_or_else(|| args::bad(span, "MachineLearning.roc_auc needs labels"))?;
    learning::metrics::classification::roc_auc(&scores, &labels)
        .map(Value::F64)
        .ok_or_else(|| args::bad(span, "roc_auc: invalid input"))
}

/// `MachineLearning.kmeans` — k-means clustering.
/// Args: { data: [f64] (row-major), n: u64, p: u64, k: u64, max_iter: u64, seed: u64 }
pub fn kmeans(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let data = args::rec_f64_list(args, "data")
        .ok_or_else(|| args::bad(span, "MachineLearning.kmeans needs data"))?;
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "MachineLearning.kmeans needs n"))? as usize;
    let p = args::rec_u64(args, "p")
        .ok_or_else(|| args::bad(span, "MachineLearning.kmeans needs p"))? as usize;
    let k = args::rec_u64(args, "k")
        .ok_or_else(|| args::bad(span, "MachineLearning.kmeans needs k"))? as usize;
    let max_iter = args::rec_u64(args, "max_iter").unwrap_or(100) as usize;
    let seed = args::rec_u64(args, "seed").unwrap_or(42);
    if data.len() < n * p {
        return Err(args::bad(
            span,
            "kmeans: data must have at least n*p values",
        ));
    }
    match learning::clustering::kmeans::fit(&data, n, p, k, max_iter, seed) {
        Ok(model) => Ok(args::record([
            (
                "centroids",
                Value::List(model.centroids.iter().map(|v| Value::F64(*v)).collect()),
            ),
            (
                "labels",
                Value::List(model.labels.iter().map(|v| Value::U64(*v as u64)).collect()),
            ),
            ("inertia", Value::F64(model.inertia)),
            ("k", Value::U64(model.k as u64)),
            ("n_iter", Value::U64(model.n_iter as u64)),
            ("converged", Value::Bool(model.converged)),
        ])),
        Err(e) => Err(args::bad(span, format!("kmeans: {e:?}"))),
    }
}

/// `MachineLearning.train_test_split` — split data into train/test sets.
/// Args: { n: u64, test_ratio: f64, seed: u64 }
pub fn train_test_split(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "MachineLearning.train_test_split needs n"))?
        as usize;
    let test_ratio = args::rec_f64(args, "test_ratio")
        .ok_or_else(|| args::bad(span, "MachineLearning.train_test_split needs test_ratio"))?;
    let seed = args::rec_u64(args, "seed").unwrap_or(42);
    match learning::resampling::folds::train_test_split(n, test_ratio, seed) {
        Some((train, test)) => Ok(args::record([
            (
                "train_indices",
                Value::List(train.iter().map(|i| Value::U64(*i as u64)).collect()),
            ),
            (
                "test_indices",
                Value::List(test.iter().map(|i| Value::U64(*i as u64)).collect()),
            ),
            ("train_size", Value::U64(train.len() as u64)),
            ("test_size", Value::U64(test.len() as u64)),
        ])),
        None => Err(args::bad(span, "train_test_split: invalid parameters")),
    }
}
