//! ML fitter invoke seams (part 2) — decision trees, HMM, variational
//! inference, MCMC, and Gaussian process regression.

use super::super::args;
use super::fitters::parse_matrix;
use crate::solvers::learning as ml;
use vibe::{Diagnostic, Span, Value};

// ── Decision trees ──────────────────────────────────────────────────

/// `MachineLearning.decision_tree_fit_regressor` — fit a regression tree.
/// Args: { x: [[f64]], y: [f64], max_depth: u64, min_samples_split: u64, min_samples_leaf: u64, seed: u64 }
pub fn decision_tree_fit_regressor(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) = parse_matrix(args, "x")
        .ok_or_else(|| args::bad(span, "decision_tree_fit_regressor needs x: [[f64]]"))?;
    let y = args::rec_f64_list(args, "y")
        .ok_or_else(|| args::bad(span, "decision_tree_fit_regressor needs y"))?;
    let params = ml::trees::decision_tree::TreeParams {
        max_depth: args::rec_u64(args, "max_depth").unwrap_or(8) as usize,
        min_samples_split: args::rec_u64(args, "min_samples_split").unwrap_or(2) as usize,
        min_samples_leaf: args::rec_u64(args, "min_samples_leaf").unwrap_or(1) as usize,
        max_features: None,
        seed: args::rec_u64(args, "seed").unwrap_or(0),
    };
    match ml::trees::decision_tree::DecisionTree::fit_regressor(&x, &y, n, p, params) {
        Ok(tree) => {
            let preds = tree.predict(&x, n);
            Ok(args::record([
                ("fitted", Value::Bool(true)),
                ("n", Value::U64(n as u64)),
                ("p", Value::U64(p as u64)),
                ("predictions", args::f64_list_value(preds)),
            ]))
        }
        Err(e) => Err(args::bad(
            span,
            format!("decision_tree_fit_regressor: {e:?}"),
        )),
    }
}

/// `MachineLearning.decision_tree_fit_classifier` — fit a classification tree.
/// Args: { x: [[f64]], y: [u64], max_depth: u64, min_samples_split: u64, min_samples_leaf: u64, seed: u64 }
pub fn decision_tree_fit_classifier(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) = parse_matrix(args, "x")
        .ok_or_else(|| args::bad(span, "decision_tree_fit_classifier needs x: [[f64]]"))?;
    let y_u64 = args::rec_u64_list(args, "y")
        .ok_or_else(|| args::bad(span, "decision_tree_fit_classifier needs y"))?;
    let y: Vec<usize> = y_u64.iter().map(|&v| v as usize).collect();
    let params = ml::trees::decision_tree::TreeParams {
        max_depth: args::rec_u64(args, "max_depth").unwrap_or(8) as usize,
        min_samples_split: args::rec_u64(args, "min_samples_split").unwrap_or(2) as usize,
        min_samples_leaf: args::rec_u64(args, "min_samples_leaf").unwrap_or(1) as usize,
        max_features: None,
        seed: args::rec_u64(args, "seed").unwrap_or(0),
    };
    match ml::trees::decision_tree::DecisionTree::fit_classifier(&x, &y, n, p, params) {
        Ok(tree) => {
            let preds: Vec<usize> = (0..n)
                .map(|i| tree.predict_class(&x[i * p..(i + 1) * p]))
                .collect();
            Ok(args::record([
                ("fitted", Value::Bool(true)),
                ("n", Value::U64(n as u64)),
                ("p", Value::U64(p as u64)),
                (
                    "predictions",
                    Value::List(preds.iter().map(|&p| Value::U64(p as u64)).collect()),
                ),
            ]))
        }
        Err(e) => Err(args::bad(
            span,
            format!("decision_tree_fit_classifier: {e:?}"),
        )),
    }
}

// ── HMM ─────────────────────────────────────────────────────────────

/// `MachineLearning.hmm_baum_welch` — learn HMM parameters by Baum-Welch (EM).
/// Args: { obs: [u64], k: u64, m: u64, max_iter: u64, tol: f64, seed: u64 }
pub fn hmm_baum_welch(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let obs_u64 = args::rec_u64_list(args, "obs")
        .ok_or_else(|| args::bad(span, "hmm_baum_welch needs obs"))?;
    let obs: Vec<usize> = obs_u64.iter().map(|&o| o as usize).collect();
    let k =
        args::rec_u64(args, "k").ok_or_else(|| args::bad(span, "hmm_baum_welch needs k"))? as usize;
    let m =
        args::rec_u64(args, "m").ok_or_else(|| args::bad(span, "hmm_baum_welch needs m"))? as usize;
    let max_iter = args::rec_u64(args, "max_iter").unwrap_or(100) as usize;
    let tol = args::rec_f64(args, "tol").unwrap_or(1e-6);
    let seed = args::rec_u64(args, "seed").unwrap_or(0);
    match ml::sequential::hmm::baum_welch(&obs, k, m, max_iter, tol, seed) {
        Ok((model, log_lik)) => Ok(args::record([
            ("pi", args::f64_list_value(model.pi)),
            ("a", args::f64_list_value(model.a)),
            ("b", args::f64_list_value(model.b)),
            ("k", Value::U64(model.k as u64)),
            ("m", Value::U64(model.m as u64)),
            ("log_likelihood", Value::F64(log_lik)),
        ])),
        Err(e) => Err(args::bad(span, format!("hmm_baum_welch: {e:?}"))),
    }
}

// ── Variational inference ───────────────────────────────────────────

/// `MachineLearning.variational_gaussian_fit` — mean-field VI for a univariate Gaussian.
/// Args: { data: [f64], mu0: f64, lambda0: f64, a0: f64, b0: f64, max_iter: u64, tol: f64 }
pub fn variational_gaussian_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let data = args::rec_f64_list(args, "data")
        .ok_or_else(|| args::bad(span, "variational_gaussian_fit needs data"))?;
    let mu0 = args::rec_f64(args, "mu0").unwrap_or(0.0);
    let lambda0 = args::rec_f64(args, "lambda0").unwrap_or(1.0);
    let a0 = args::rec_f64(args, "a0").unwrap_or(1.0);
    let b0 = args::rec_f64(args, "b0").unwrap_or(1.0);
    let max_iter = args::rec_u64(args, "max_iter").unwrap_or(100) as usize;
    let tol = args::rec_f64(args, "tol").unwrap_or(1e-6);
    match ml::variational::gaussian::fit(&data, mu0, lambda0, a0, b0, max_iter, tol) {
        Ok(vg) => Ok(args::record([
            ("mu_n", Value::F64(vg.mu_n)),
            ("lambda_n", Value::F64(vg.lambda_n)),
            ("a_n", Value::F64(vg.a_n)),
            ("b_n", Value::F64(vg.b_n)),
            ("n_iter", Value::U64(vg.n_iter as u64)),
            ("converged", Value::Bool(vg.converged)),
            ("posterior_mean", Value::F64(vg.mean())),
            ("posterior_precision", Value::F64(vg.precision_mean())),
            ("posterior_variance", Value::F64(vg.variance_mean())),
        ])),
        Err(e) => Err(args::bad(span, format!("variational_gaussian_fit: {e:?}"))),
    }
}

// ── MCMC ────────────────────────────────────────────────────────────

/// `MachineLearning.mcmc_metropolis` — random-walk Metropolis-Hastings.
/// The log-density is selected by a string: "standard_normal" or
/// "mvn" (with mean and cov arguments).
/// Args: { target: "standard_normal"|"mvn", initial: [f64], proposal_std: f64, n_samples: u64, burn_in: u64, seed: u64, mean: [f64]?, cov: [f64]?, p: u64? }
pub fn mcmc_metropolis(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let target = args::rec_str(args, "target").unwrap_or("standard_normal");
    let initial = args::rec_f64_list(args, "initial")
        .ok_or_else(|| args::bad(span, "mcmc_metropolis needs initial"))?;
    let proposal_std = args::rec_f64(args, "proposal_std").unwrap_or(1.0);
    let n_samples = args::rec_u64(args, "n_samples").unwrap_or(100) as usize;
    let burn_in = args::rec_u64(args, "burn_in").unwrap_or(10) as usize;
    let seed = args::rec_u64(args, "seed").unwrap_or(0);

    let result = match target {
        "mvn" | "Mvn" | "MVN" => {
            let mean = args::rec_f64_list(args, "mean")
                .ok_or_else(|| args::bad(span, "mcmc_metropolis mvn needs mean"))?;
            let cov = args::rec_f64_list(args, "cov")
                .ok_or_else(|| args::bad(span, "mcmc_metropolis mvn needs cov"))?;
            let p = args::rec_u64(args, "p")
                .ok_or_else(|| args::bad(span, "mcmc_metropolis mvn needs p"))?
                as usize;
            let log_density = move |x: &[f64]| {
                crate::solvers::statistics::distributions::multivariate_normal::log_pdf(
                    x, &mean, &cov, p,
                )
                .unwrap_or(f64::NEG_INFINITY)
            };
            ml::sampling::mcmc::metropolis_hastings(
                log_density,
                &initial,
                proposal_std,
                n_samples,
                burn_in,
                seed,
            )
        }
        _ => {
            // Standard normal: log p(x) = -0.5 * sum(x_i^2) - const
            let log_density = |x: &[f64]| -0.5 * x.iter().map(|v| v * v).sum::<f64>();
            ml::sampling::mcmc::metropolis_hastings(
                log_density,
                &initial,
                proposal_std,
                n_samples,
                burn_in,
                seed,
            )
        }
    };

    Ok(args::record([
        ("samples", args::f64_list_value(result.samples)),
        ("n_samples", Value::U64(result.n_samples as u64)),
        ("dim", Value::U64(result.dim as u64)),
        ("acceptance_rate", Value::F64(result.acceptance_rate)),
    ]))
}

// ── Gaussian process ────────────────────────────────────────────────

/// `MachineLearning.gp_fit` — Gaussian process regression with squared-exponential kernel.
/// Args: { x: [[f64]], y: [f64], length_scale: f64, signal_var: f64, noise_var: f64 }
pub fn gp_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (x, n, p) =
        parse_matrix(args, "x").ok_or_else(|| args::bad(span, "gp_fit needs x: [[f64]]"))?;
    let y = args::rec_f64_list(args, "y").ok_or_else(|| args::bad(span, "gp_fit needs y"))?;
    let length_scale = args::rec_f64(args, "length_scale").unwrap_or(1.0);
    let signal_var = args::rec_f64(args, "signal_var").unwrap_or(1.0);
    let noise_var = args::rec_f64(args, "noise_var").unwrap_or(1e-6);
    match ml::gaussian_process::GpRegressor::fit(&x, &y, n, p, length_scale, signal_var, noise_var)
    {
        Ok(_gp) => Ok(args::record([
            ("fitted", Value::Bool(true)),
            ("n", Value::U64(n as u64)),
            ("p", Value::U64(p as u64)),
            ("length_scale", Value::F64(length_scale)),
            ("signal_var", Value::F64(signal_var)),
            ("noise_var", Value::F64(noise_var)),
        ])),
        Err(e) => Err(args::bad(span, format!("gp_fit: {e:?}"))),
    }
}
