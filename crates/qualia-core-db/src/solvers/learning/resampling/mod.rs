//! Resampling methods (ISL ch 5) — cross-validation and the bootstrap. The
//! generic harness every later chapter reuses to estimate test error / variability
//! without a separate validation set.
//!
//! [`folds`] generates the index splits; [`bootstrap`] resamples a statistic;
//! [`cross_val_score`] runs a caller-supplied fit→predict over the folds and scores
//! each with a caller-supplied metric (so it is estimator-agnostic — works with the
//! `regression`, `glm`, … estimators or any closure).

pub mod bootstrap;
pub mod folds;
pub mod permutation;

pub use bootstrap::{
    bootstrap_ci, bootstrap_estimate, bootstrap_indices, BootstrapCi, BootstrapResult, CiMethod,
};
pub use folds::{k_fold, loocv, train_test_split, Fold};
pub use permutation::{two_sample_test, PermutationResult};

/// Gather the rows named by `idx` from a row-major `_ × p` matrix into a fresh
/// contiguous `idx.len() × p` matrix.
fn gather_rows(x: &[f64], p: usize, idx: &[usize]) -> Vec<f64> {
    let mut out = vec![0.0; idx.len() * p];
    for (r, &i) in idx.iter().enumerate() {
        out[r * p..(r + 1) * p].copy_from_slice(&x[i * p..(i + 1) * p]);
    }
    out
}

/// Cross-validated score of an estimator across `folds`.
///
/// `fit_predict(train_x, train_y, n_train, test_x, n_test) -> predictions` trains on
/// the fold's training rows and predicts its test rows; `metric(y_true, preds) ->
/// score` scores that fold (e.g. `metrics::mse` or a negated error). Returns one
/// score per fold, or `None` on a shape mismatch / empty folds.
pub fn cross_val_score<F, M>(
    x: &[f64],
    y: &[f64],
    n: usize,
    p: usize,
    folds: &[Fold],
    mut fit_predict: F,
    metric: M,
) -> Option<Vec<f64>>
where
    F: FnMut(&[f64], &[f64], usize, &[f64], usize) -> Vec<f64>,
    M: Fn(&[f64], &[f64]) -> Option<f64>,
{
    if x.len() != n * p || y.len() != n || folds.is_empty() {
        return None;
    }
    let mut scores = Vec::with_capacity(folds.len());
    for fold in folds {
        let n_tr = fold.train.len();
        let n_te = fold.test.len();
        if n_tr == 0 || n_te == 0 {
            return None;
        }
        let train_x = gather_rows(x, p, &fold.train);
        let train_y: Vec<f64> = fold.train.iter().map(|&i| y[i]).collect();
        let test_x = gather_rows(x, p, &fold.test);
        let test_y: Vec<f64> = fold.test.iter().map(|&i| y[i]).collect();
        let preds = fit_predict(&train_x, &train_y, n_tr, &test_x, n_te);
        if preds.len() != n_te {
            return None;
        }
        scores.push(metric(&test_y, &preds)?);
    }
    Some(scores)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::learning::metrics::regression::mse;
    use crate::solvers::learning::regression::linear;

    #[test]
    fn cross_validates_a_linear_model() {
        // y = 2 + 3x + small noise; CV MSE should be small for OLS.
        let n = 20;
        let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let y: Vec<f64> = x
            .iter()
            .map(|&xi| 2.0 + 3.0 * xi + ((xi as i64 % 3) as f64 - 1.0) * 0.1)
            .collect();
        let folds = k_fold(n, 5, true, 1);
        let scores = cross_val_score(
            &x,
            &y,
            n,
            1,
            &folds,
            |tx, ty, ntr, tex, nte| {
                let m = linear::fit(tx, ty, ntr, 1, true).unwrap();
                m.predict(tex, nte, 1)
            },
            |yt, yp| mse(yt, yp),
        )
        .unwrap();
        assert_eq!(scores.len(), 5);
        // Mean CV MSE is small (the model fits the near-linear data).
        let mean_mse: f64 = scores.iter().sum::<f64>() / scores.len() as f64;
        assert!(mean_mse < 0.1, "CV MSE too large: {mean_mse}");
    }

    #[test]
    fn loocv_runs_n_folds() {
        let n = 8;
        let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&xi| 1.0 + 0.5 * xi).collect();
        let folds = loocv(n);
        let scores = cross_val_score(
            &x,
            &y,
            n,
            1,
            &folds,
            |tx, ty, ntr, tex, nte| {
                linear::fit(tx, ty, ntr, 1, true)
                    .unwrap()
                    .predict(tex, nte, 1)
            },
            |yt, yp| mse(yt, yp),
        )
        .unwrap();
        assert_eq!(scores.len(), n);
        // Exact line → ~zero error on every held-out point.
        assert!(scores.iter().all(|&s| s < 1e-9));
    }
}
