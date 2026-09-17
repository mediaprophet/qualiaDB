//! ML fitter invoke seams (part 4) — factor graph belief propagation,
//! standard scaling, and BART (Bayesian additive regression trees).

use super::super::args;
use super::fitters::parse_matrix;
use crate::solvers::learning as ml;
use vibe::{Diagnostic, Span, Value};

// ── Factor graph ────────────────────────────────────────────────────

/// `MachineLearning.factor_graph_marginals` — sum-product belief propagation.
/// Args: { cardinalities: [u64], factors: [{ vars: [u64], table: [f64] }], max_iter: u64, tol: f64 }
pub fn factor_graph_marginals(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let cards_u64 = args::rec_u64_list(args, "cardinalities")
        .ok_or_else(|| args::bad(span, "factor_graph_marginals needs cardinalities"))?;
    let cardinalities: Vec<usize> = cards_u64.iter().map(|&c| c as usize).collect();
    let factors_val = args::rec(args, "factors")
        .ok_or_else(|| args::bad(span, "factor_graph_marginals needs factors"))?;
    let factor_items = args::list(factors_val)
        .ok_or_else(|| args::bad(span, "factor_graph_marginals: factors must be a list"))?;
    let mut factors = Vec::new();
    for fval in factor_items {
        let vars_u64 = args::rec_u64_list(fval, "vars")
            .ok_or_else(|| args::bad(span, "factor_graph_marginals: factor needs vars"))?;
        let table = args::rec_f64_list(fval, "table")
            .ok_or_else(|| args::bad(span, "factor_graph_marginals: factor needs table"))?;
        factors.push(ml::graphical_models::factor_graph::Factor {
            vars: vars_u64.iter().map(|&v| v as usize).collect(),
            table,
        });
    }
    let max_iter = args::rec_u64(args, "max_iter").unwrap_or(100) as usize;
    let tol = args::rec_f64(args, "tol").unwrap_or(1e-6);
    match ml::graphical_models::factor_graph::FactorGraph::new(cardinalities, factors) {
        Ok(fg) => {
            let marginals = fg.marginals(max_iter, tol);
            let marginal_records: Vec<Value> = marginals
                .iter()
                .map(|m| args::f64_list_value(m.clone()))
                .collect();
            Ok(args::record([
                ("marginals", Value::List(marginal_records)),
                ("n_variables", Value::U64(marginals.len() as u64)),
            ]))
        }
        Err(e) => Err(args::bad(span, format!("factor_graph_marginals: {e:?}"))),
    }
}

// ── Standard scaler ─────────────────────────────────────────────────

/// `MachineLearning.standard_scaler_fit_transform` — z-score standardize a matrix.
/// Args: { x: [[f64]] }
pub fn standard_scaler_fit_transform(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) = parse_matrix(args, "x")
        .ok_or_else(|| args::bad(span, "standard_scaler_fit_transform needs x: [[f64]]"))?;
    match ml::preprocessing::scaling::StandardScaler::fit_transform(&x, n, p) {
        Some((scaler, scaled)) => Ok(args::record([
            ("means", args::f64_list_value(scaler.means().to_vec())),
            ("stds", args::f64_list_value(scaler.stds().to_vec())),
            ("scaled", args::f64_list_value(scaled)),
            ("n", Value::U64(n as u64)),
            ("p", Value::U64(p as u64)),
        ])),
        None => Err(args::bad(
            span,
            "standard_scaler_fit_transform: invalid input",
        )),
    }
}

// ── BART ────────────────────────────────────────────────────────────

/// `MachineLearning.bart_fit` — Bayesian additive regression trees.
/// Args: { x: [[f64]], y: [f64], m: u64, n_iter: u64, burn_in: u64, k: f64, seed: u64 }
pub fn bart_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) =
        parse_matrix(args, "x").ok_or_else(|| args::bad(span, "bart_fit needs x: [[f64]]"))?;
    let y = args::rec_f64_list(args, "y").ok_or_else(|| args::bad(span, "bart_fit needs y"))?;
    let m = args::rec_u64(args, "m").unwrap_or(20) as usize;
    let n_iter = args::rec_u64(args, "n_iter").unwrap_or(100) as usize;
    let burn_in = args::rec_u64(args, "burn_in").unwrap_or(20) as usize;
    let k = args::rec_f64(args, "k").unwrap_or(2.0);
    let seed = args::rec_u64(args, "seed").unwrap_or(0);
    match ml::trees::bart::Bart::fit(&x, &y, n, p, m, n_iter, burn_in, k, seed) {
        Ok(model) => {
            let preds = model.predict(&x, n);
            Ok(args::record([
                ("n", Value::U64(n as u64)),
                ("p", Value::U64(p as u64)),
                ("m", Value::U64(m as u64)),
                ("n_iter", Value::U64(n_iter as u64)),
                ("burn_in", Value::U64(burn_in as u64)),
                ("predictions", args::f64_list_value(preds)),
            ]))
        }
        Err(e) => Err(args::bad(span, format!("bart_fit: {e:?}"))),
    }
}
