//! ML fitter invoke seams — regression, clustering, GLM, survival, and
//! classification model fitting with domain-object construction from
//! VibeScript records.

use super::super::args;
use crate::solvers::learning as ml;
use poet_vibe::{Diagnostic, Span, Value};

/// Parse a row-major `n × p` matrix from a VibeScript list-of-lists.
pub(super) fn parse_matrix(v: &Value, key: &str) -> Option<(Vec<f64>, usize, usize)> {
    let list_val = args::rec(v, key)?;
    let rows = args::list(&list_val)?;
    let n = rows.len();
    if n == 0 {
        return None;
    }
    let p = args::f64s(&rows[0])?.len();
    if p == 0 {
        return None;
    }
    let mut data = Vec::with_capacity(n * p);
    for row in rows {
        let cells = args::f64s(row)?;
        if cells.len() != p {
            return None;
        }
        data.extend(cells);
    }
    Some((data, n, p))
}

// ── Regression fitters ──────────────────────────────────────────────

/// `MachineLearning.ridge_fit` — ridge regression with L2 penalty.
/// Args: { x: [[f64]], y: [f64], lambda: f64 }
pub fn ridge_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) =
        parse_matrix(args, "x").ok_or_else(|| args::bad(span, "ridge_fit needs x: [[f64]]"))?;
    let y = args::rec_f64_list(args, "y").ok_or_else(|| args::bad(span, "ridge_fit needs y"))?;
    let lambda = args::rec_f64(args, "lambda").unwrap_or(1.0);
    match ml::regression::ridge::fit(&x, &y, n, p, lambda) {
        Ok(model) => Ok(args::record([
            ("coefficients", args::f64_list_value(model.coefficients)),
            ("intercept", Value::F64(model.intercept)),
            ("lambda", Value::F64(model.lambda)),
        ])),
        Err(e) => Err(args::bad(span, format!("ridge_fit: {e:?}"))),
    }
}

/// `MachineLearning.lasso_fit` — lasso regression with L1 penalty.
/// Args: { x: [[f64]], y: [f64], lambda: f64, max_iter: u64, tol: f64 }
pub fn lasso_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) =
        parse_matrix(args, "x").ok_or_else(|| args::bad(span, "lasso_fit needs x: [[f64]]"))?;
    let y = args::rec_f64_list(args, "y").ok_or_else(|| args::bad(span, "lasso_fit needs y"))?;
    let lambda = args::rec_f64(args, "lambda").unwrap_or(1.0);
    let max_iter = args::rec_u64(args, "max_iter").unwrap_or(100) as usize;
    let tol = args::rec_f64(args, "tol").unwrap_or(1e-6);
    match ml::regression::lasso::fit(&x, &y, n, p, lambda, max_iter, tol) {
        Ok(model) => Ok(args::record([
            ("coefficients", args::f64_list_value(model.coefficients)),
            ("intercept", Value::F64(model.intercept)),
            ("lambda", Value::F64(model.lambda)),
            ("n_iter", Value::U64(model.n_iter as u64)),
            ("converged", Value::Bool(model.converged)),
        ])),
        Err(e) => Err(args::bad(span, format!("lasso_fit: {e:?}"))),
    }
}

/// `MachineLearning.pls_fit` — PLS1 regression by NIPALS.
/// Args: { x: [[f64]], y: [f64], n_components: u64 }
pub fn pls_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) =
        parse_matrix(args, "x").ok_or_else(|| args::bad(span, "pls_fit needs x: [[f64]]"))?;
    let y = args::rec_f64_list(args, "y").ok_or_else(|| args::bad(span, "pls_fit needs y"))?;
    let n_components = args::rec_u64(args, "n_components").unwrap_or(p as u64) as usize;
    match ml::regression::pls::fit(&x, &y, n, p, n_components) {
        Ok(model) => Ok(args::record([
            ("coefficients", args::f64_list_value(model.coefficients)),
            ("intercept", Value::F64(model.intercept)),
            ("n_components", Value::U64(model.n_components as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("pls_fit: {e:?}"))),
    }
}

// ── Clustering fitters ──────────────────────────────────────────────

/// `MachineLearning.kmeans_fit` — k-means clustering with k-means++ seeding.
/// Args: { x: [[f64]], k: u64, max_iter: u64, seed: u64 }
pub fn kmeans_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) =
        parse_matrix(args, "x").ok_or_else(|| args::bad(span, "kmeans_fit needs x: [[f64]]"))?;
    let k = args::rec_u64(args, "k").ok_or_else(|| args::bad(span, "kmeans_fit needs k"))? as usize;
    let max_iter = args::rec_u64(args, "max_iter").unwrap_or(100) as usize;
    let seed = args::rec_u64(args, "seed").unwrap_or(0);
    match ml::clustering::kmeans::fit(&x, n, p, k, max_iter, seed) {
        Ok(model) => Ok(args::record([
            ("centroids", args::f64_list_value(model.centroids)),
            (
                "labels",
                Value::List(model.labels.iter().map(|&l| Value::U64(l as u64)).collect()),
            ),
            ("inertia", Value::F64(model.inertia)),
            ("k", Value::U64(model.k as u64)),
            ("n_iter", Value::U64(model.n_iter as u64)),
            ("converged", Value::Bool(model.converged)),
        ])),
        Err(e) => Err(args::bad(span, format!("kmeans_fit: {e:?}"))),
    }
}

/// `MachineLearning.gmm_fit` — diagonal-covariance Gaussian mixture by EM.
/// Args: { x: [[f64]], k: u64, max_iter: u64, tol: f64, seed: u64 }
pub fn gmm_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) =
        parse_matrix(args, "x").ok_or_else(|| args::bad(span, "gmm_fit needs x: [[f64]]"))?;
    let k = args::rec_u64(args, "k").ok_or_else(|| args::bad(span, "gmm_fit needs k"))? as usize;
    let max_iter = args::rec_u64(args, "max_iter").unwrap_or(100) as usize;
    let tol = args::rec_f64(args, "tol").unwrap_or(1e-6);
    let seed = args::rec_u64(args, "seed").unwrap_or(0);
    match ml::clustering::gmm::fit(&x, n, p, k, max_iter, tol, seed) {
        Ok(model) => Ok(args::record([
            ("weights", args::f64_list_value(model.weights)),
            ("means", args::f64_list_value(model.means)),
            ("variances", args::f64_list_value(model.variances)),
            (
                "labels",
                Value::List(model.labels.iter().map(|&l| Value::U64(l as u64)).collect()),
            ),
            ("log_likelihood", Value::F64(model.log_likelihood)),
            ("n_iter", Value::U64(model.n_iter as u64)),
            ("converged", Value::Bool(model.converged)),
        ])),
        Err(e) => Err(args::bad(span, format!("gmm_fit: {e:?}"))),
    }
}

// ── GLM fitters ─────────────────────────────────────────────────────

/// `MachineLearning.logistic_fit` — logistic regression (Bernoulli GLM).
/// Args: { x: [[f64]], y: [f64], intercept: bool }
pub fn logistic_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) =
        parse_matrix(args, "x").ok_or_else(|| args::bad(span, "logistic_fit needs x: [[f64]]"))?;
    let y = args::rec_f64_list(args, "y").ok_or_else(|| args::bad(span, "logistic_fit needs y"))?;
    let intercept = args::rec_bool(args, "intercept").unwrap_or(true);
    match ml::glm::fit_logistic(&x, &y, n, p, intercept) {
        Ok(model) => Ok(args::record([
            ("coefficients", args::f64_list_value(model.coefficients)),
            ("std_errors", args::f64_list_value(model.std_errors)),
            ("z_values", args::f64_list_value(model.z_values)),
            ("p_values", args::f64_list_value(model.p_values)),
            ("deviance", Value::F64(model.deviance)),
            ("n_iter", Value::U64(model.n_iter as u64)),
            ("converged", Value::Bool(model.converged)),
        ])),
        Err(e) => Err(args::bad(span, format!("logistic_fit: {e:?}"))),
    }
}

/// `MachineLearning.poisson_fit` — Poisson regression (count GLM).
/// Args: { x: [[f64]], y: [f64], intercept: bool }
pub fn poisson_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) =
        parse_matrix(args, "x").ok_or_else(|| args::bad(span, "poisson_fit needs x: [[f64]]"))?;
    let y = args::rec_f64_list(args, "y").ok_or_else(|| args::bad(span, "poisson_fit needs y"))?;
    let intercept = args::rec_bool(args, "intercept").unwrap_or(true);
    match ml::glm::fit_poisson(&x, &y, n, p, intercept) {
        Ok(model) => Ok(args::record([
            ("coefficients", args::f64_list_value(model.coefficients)),
            ("std_errors", args::f64_list_value(model.std_errors)),
            ("z_values", args::f64_list_value(model.z_values)),
            ("p_values", args::f64_list_value(model.p_values)),
            ("deviance", Value::F64(model.deviance)),
            ("n_iter", Value::U64(model.n_iter as u64)),
            ("converged", Value::Bool(model.converged)),
        ])),
        Err(e) => Err(args::bad(span, format!("poisson_fit: {e:?}"))),
    }
}

// ── Survival ────────────────────────────────────────────────────────

/// `MachineLearning.cox_fit` — Cox proportional-hazards regression.
/// Args: { x: [[f64]], times: [f64], event: [bool] }
pub fn cox_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) =
        parse_matrix(args, "x").ok_or_else(|| args::bad(span, "cox_fit needs x: [[f64]]"))?;
    let times =
        args::rec_f64_list(args, "times").ok_or_else(|| args::bad(span, "cox_fit needs times"))?;
    let event =
        args::rec_bool_list(args, "event").ok_or_else(|| args::bad(span, "cox_fit needs event"))?;
    match ml::survival::cox::fit(&x, &times, &event, n, p) {
        Ok(model) => Ok(args::record([
            ("coefficients", args::f64_list_value(model.coefficients)),
            ("std_errors", args::f64_list_value(model.std_errors)),
            ("z_values", args::f64_list_value(model.z_values)),
            ("p_values", args::f64_list_value(model.p_values)),
            (
                "log_partial_likelihood",
                Value::F64(model.log_partial_likelihood),
            ),
            ("n_iter", Value::U64(model.n_iter as u64)),
            ("converged", Value::Bool(model.converged)),
        ])),
        Err(e) => Err(args::bad(span, format!("cox_fit: {e:?}"))),
    }
}

// ── Classification ──────────────────────────────────────────────────

/// `MachineLearning.svm_fit` — soft-margin SVM by simplified SMO.
/// Args: { x: [[f64]], y: [bool], c: f64, kernel: "linear"|"rbf", gamma: f64, max_passes: u64, tol: f64 }
pub fn svm_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) =
        parse_matrix(args, "x").ok_or_else(|| args::bad(span, "svm_fit needs x: [[f64]]"))?;
    let y = args::rec_bool_list(args, "y").ok_or_else(|| args::bad(span, "svm_fit needs y"))?;
    let c = args::rec_f64(args, "c").unwrap_or(1.0);
    let kernel_str = args::rec_str(args, "kernel").unwrap_or("linear");
    let gamma = args::rec_f64(args, "gamma").unwrap_or(0.5);
    let kernel = match kernel_str {
        "rbf" | "Rbf" | "RBF" => ml::classification::svm::Kernel::Rbf { gamma },
        _ => ml::classification::svm::Kernel::Linear,
    };
    let max_passes = args::rec_u64(args, "max_passes").unwrap_or(5) as usize;
    let tol = args::rec_f64(args, "tol").unwrap_or(1e-3);
    match ml::classification::svm::fit(&x, &y, n, p, c, kernel, max_passes, tol) {
        Ok(_model) => Ok(args::record([
            ("fitted", Value::Bool(true)),
            ("n", Value::U64(n as u64)),
            ("p", Value::U64(p as u64)),
            ("kernel", Value::String(kernel_str.to_string())),
        ])),
        Err(e) => Err(args::bad(span, format!("svm_fit: {e:?}"))),
    }
}
