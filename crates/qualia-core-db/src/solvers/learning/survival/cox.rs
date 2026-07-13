//! Cox proportional-hazards regression (ISL ch 11.5) — semiparametric hazard model
//! `h(t|x) = h₀(t)·exp(βᵀx)`, fit by maximizing the Breslow partial likelihood with
//! Newton–Raphson. The p×p information-matrix solve reuses `linear_algebra::cholesky`
//! (no new solver); Wald standard errors / p-values use `statistics::distributions`.

use crate::solvers::learning::LearningError;
use crate::solvers::linear_algebra::cholesky::{cholesky_factor, cholesky_solve};
use crate::solvers::statistics::distributions::normal;

/// A fitted Cox model.
#[derive(Debug, Clone)]
pub struct CoxModel {
    pub coefficients: Vec<f64>,
    pub std_errors: Vec<f64>,
    /// Wald z-statistics (coefficient / std error). A positive coefficient means the
    /// covariate increases the hazard (shortens survival).
    pub z_values: Vec<f64>,
    pub p_values: Vec<f64>,
    pub log_partial_likelihood: f64,
    pub n_iter: usize,
    pub converged: bool,
}

const MAX_ITER: usize = 100;
const TOL: f64 = 1e-8;

/// Fit Cox PH of right-censored `(times, event)` on a row-major `n × p` covariate
/// matrix. `event[i] = true` is an observed event, `false` is right-censoring.
/// Fails closed: `InvalidDimension`, `InsufficientData`, `Singular`, `NotConverged`.
pub fn fit(
    x: &[f64],
    times: &[f64],
    event: &[bool],
    n: usize,
    p: usize,
) -> Result<CoxModel, LearningError> {
    if n == 0 || p == 0 || x.len() != n * p || times.len() != n || event.len() != n {
        return Err(LearningError::InvalidDimension);
    }
    if event.iter().filter(|&&e| e).count() == 0 {
        return Err(LearningError::InsufficientData); // no events ⇒ nothing to fit
    }

    let mut beta = vec![0.0; p];
    let mut grad = vec![0.0; p];
    let mut info = vec![0.0; p * p];
    let mut converged = false;
    let mut iters = 0;
    let mut log_pl = 0.0;

    for it in 1..=MAX_ITER {
        iters = it;
        grad.iter_mut().for_each(|v| *v = 0.0);
        info.iter_mut().for_each(|v| *v = 0.0);
        log_pl = 0.0;

        // Linear predictors and weights.
        let mut w = vec![0.0; n];
        for j in 0..n {
            let eta: f64 = (0..p).map(|c| beta[c] * x[j * p + c]).sum();
            w[j] = eta.exp();
        }

        for i in 0..n {
            if !event[i] {
                continue;
            }
            // Risk set R_i = { j : times[j] >= times[i] }, Breslow.
            let ti = times[i];
            let mut sum_w = 0.0;
            let mut sum_wx = vec![0.0; p];
            let mut sum_wxx = vec![0.0; p * p];
            for j in 0..n {
                if times[j] >= ti {
                    let wj = w[j];
                    sum_w += wj;
                    for a in 0..p {
                        let xa = x[j * p + a];
                        sum_wx[a] += wj * xa;
                        for b in 0..p {
                            sum_wxx[a * p + b] += wj * xa * x[j * p + b];
                        }
                    }
                }
            }
            if sum_w <= 0.0 {
                continue;
            }
            let eta_i: f64 = (0..p).map(|c| beta[c] * x[i * p + c]).sum();
            log_pl += eta_i - sum_w.ln();
            // Gradient + observed information.
            for a in 0..p {
                let mean_a = sum_wx[a] / sum_w;
                grad[a] += x[i * p + a] - mean_a;
                for b in 0..p {
                    let mean_b = sum_wx[b] / sum_w;
                    info[a * p + b] += sum_wxx[a * p + b] / sum_w - mean_a * mean_b;
                }
            }
        }

        // Newton step: solve info · delta = grad (info is the observed information,
        // positive-definite near the maximum).
        let mut l = vec![0.0; p * p];
        cholesky_factor(p, &info, &mut l).map_err(|_| LearningError::Singular)?;
        let mut delta = vec![0.0; p];
        cholesky_solve(p, &l, &grad, &mut delta)?;
        let mut max_step = 0.0_f64;
        for c in 0..p {
            beta[c] += delta[c];
            max_step = max_step.max(delta[c].abs());
        }
        if max_step < TOL {
            converged = true;
            break;
        }
    }

    // Standard errors from info⁻¹ at the solution.
    let mut l = vec![0.0; p * p];
    cholesky_factor(p, &info, &mut l).map_err(|_| LearningError::Singular)?;

    let mut std_errors = vec![0.0; p];
    let mut z_values = vec![0.0; p];
    let mut p_values = vec![0.0; p];
    let mut ej = vec![0.0; p];
    let mut cj = vec![0.0; p];
    for a in 0..p {
        ej.iter_mut().for_each(|v| *v = 0.0);
        ej[a] = 1.0;
        cholesky_solve(p, &l, &ej, &mut cj)?;
        let se = if cj[a] > 0.0 { cj[a].sqrt() } else { 0.0 };
        std_errors[a] = se;
        if se > 0.0 {
            let z = beta[a] / se;
            z_values[a] = z;
            p_values[a] = normal::two_sided_p(z);
        }
    }

    Ok(CoxModel {
        coefficients: beta,
        std_errors,
        z_values,
        p_values,
        log_partial_likelihood: log_pl,
        n_iter: iters,
        converged,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn higher_covariate_increases_hazard() {
        // Higher x ⇒ generally shorter survival (with inversions so the MLE is
        // finite) ⇒ positive coefficient.
        let x = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let times = [8.0, 7.0, 5.0, 6.0, 3.0, 4.0, 2.0, 1.0];
        let event = [true, true, true, true, true, true, true, true];
        let m = fit(&x, &times, &event, 8, 1).unwrap();
        assert!(m.converged);
        assert!(m.coefficients[0] > 0.0, "coef {}", m.coefficients[0]);
        assert!(m.std_errors[0] > 0.0 && m.std_errors[0].is_finite());
        assert!(m.log_partial_likelihood.is_finite());
    }

    #[test]
    fn protective_covariate_is_negative() {
        // Higher x ⇒ generally LONGER survival (with inversions) ⇒ negative coef.
        let x = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let times = [1.0, 2.0, 4.0, 3.0, 6.0, 5.0, 7.0, 8.0];
        let event = [true, true, true, true, true, true, true, true];
        let m = fit(&x, &times, &event, 8, 1).unwrap();
        assert!(m.coefficients[0] < 0.0, "coef {}", m.coefficients[0]);
    }

    #[test]
    fn guards() {
        assert_eq!(
            fit(&[1.0, 2.0], &[1.0, 2.0], &[false, false], 2, 1).unwrap_err(),
            LearningError::InsufficientData
        );
        assert_eq!(
            fit(&[1.0], &[1.0], &[true], 1, 2).unwrap_err(),
            LearningError::InvalidDimension
        );
    }
}
