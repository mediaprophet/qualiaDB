//! Discrete dependent variable: logit and 2-class LDA wrappers for Civics receipts.

use serde::Serialize;

use crate::solvers::learning::classification::discriminant::LdaModel;
use crate::solvers::learning::glm::{fit_logistic, GlmModel};

#[derive(Debug, Clone, Serialize)]
pub struct LogitFitSummary {
    pub coefficients: Vec<f64>,
    pub std_errors: Vec<f64>,
    pub z_values: Vec<f64>,
    pub p_values: Vec<f64>,
    pub deviance: f64,
    pub converged: bool,
    pub n: usize,
    pub fit_intercept: bool,
}

pub fn fit_logit(
    x: &[f64],
    y: &[f64],
    n: usize,
    p: usize,
    fit_intercept: bool,
) -> Option<LogitFitSummary> {
    let m: GlmModel = fit_logistic(x, y, n, p, fit_intercept).ok()?;
    Some(LogitFitSummary {
        coefficients: m.coefficients,
        std_errors: m.std_errors,
        z_values: m.z_values,
        p_values: m.p_values,
        deviance: m.deviance,
        converged: m.converged,
        n: m.n,
        fit_intercept: m.fit_intercept,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct LdaFitSummary {
    pub classes: Vec<usize>,
    pub predictions: Vec<usize>,
    pub n: usize,
    pub p: usize,
}

/// Two-or-more class LDA; `y` are integer class labels.
pub fn fit_lda_2class(x: &[f64], y: &[usize], n: usize, p: usize) -> Option<LdaFitSummary> {
    let m = LdaModel::fit(x, y, n, p).ok()?;
    let predictions = m.predict(x, n);
    Some(LdaFitSummary {
        classes: m.classes.clone(),
        predictions,
        n,
        p,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logit_separates_simple() {
        // Mild overlap so IRLS does not hit perfect separation.
        let x = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0];
        let y = [0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let m = fit_logit(&x, &y, 12, 1, true).expect("logit should converge");
        assert!(m.converged);
        assert!(m.coefficients[1] > 0.0);
    }

    #[test]
    fn lda_two_blobs() {
        let x = [
            0.0, 0.0, 0.1, 0.0, -0.1, 0.1, 0.0, -0.1, 5.0, 5.0, 5.1, 4.9, 4.9, 5.1, 5.0, 5.2,
        ];
        let y = [0usize, 0, 0, 0, 1, 1, 1, 1];
        let m = fit_lda_2class(&x, &y, 8, 2).unwrap();
        assert_eq!(m.classes.len(), 2);
        let ok = m
            .predictions
            .iter()
            .zip(y.iter())
            .filter(|(a, b)| a == b)
            .count();
        assert!(ok >= 6, "pred={:?}", m.predictions);
    }
}
