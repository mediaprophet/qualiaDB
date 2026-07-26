//! Econometrics: OLS, WLS, 2SLS, logistic MLE, GMM, and calibration records.
//!
//! Allocation class: **HotZeroHeap**. All scratch uses fixed-capacity stack
//! arrays. No `Vec`/`String`/`Box` in any kernel.
//!
//! Assumptions:
//! - OLS assumes exogeneity (E[ε|X] = 0), iid errors, no perfect
//!   multicollinearity. Standard errors are not yet computed (future work).
//! - WLS assumes known weights proportional to inverse error variance.
//! - 2SLS assumes instrument relevance (n_instr >= n_reg) and exogeneity.
//!   Underidentified models (n_instr < n_reg) are refused.
//! - Logistic MLE assumes iid Bernoulli outcomes with logit link; uses
//!   Newton-Raphson (IRLS).

use super::error::EconConvergence;

/// Maximum regressors (including constant) in a bounded regression.
pub const MAX_REGRESSORS: usize = 16;
/// Maximum observations in a bounded regression.
pub const MAX_OBSERVATIONS: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EconometricsError {
    InvalidInput,
    InsufficientData,
    SingularSystem,
    BufferTooSmall,
    NonFinite,
    NonConverged,
    Underidentified,
}

/// A `repr(C)` calibration record linking a fitted model to its data and
/// diagnostics.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CalibrationRecord {
    /// Static model name (e.g. "ols_v1", "logistic_irls").
    pub model_name: &'static str,
    /// FNV-1a or caller-supplied hash of the calibration dataset.
    pub data_hash: u64,
    /// Number of fitted parameters.
    pub n_params: u32,
    /// Final loss (RSS for OLS, negative log-likelihood for MLE).
    pub loss: f64,
    /// Iterations executed (0 for closed-form OLS).
    pub iterations: u32,
    /// Caller-supplied epoch seconds for provenance.
    pub epoch_seconds: u64,
}

impl CalibrationRecord {
    pub const fn new(
        model_name: &'static str,
        data_hash: u64,
        n_params: u32,
        loss: f64,
        iterations: u32,
        epoch_seconds: u64,
    ) -> Self {
        Self {
            model_name,
            data_hash,
            n_params,
            loss,
            iterations,
            epoch_seconds,
        }
    }
}

/// Solve a linear system `A * x = b` in place using Gaussian elimination with
/// partial pivoting. `a` is `n x n` row-major (destroyed), `b` is length `n`
/// (overwritten with solution). Returns `SingularSystem` if a zero pivot is
/// encountered.
fn gaussian_solve(a: &mut [f64], b: &mut [f64], n: usize) -> Result<(), EconometricsError> {
    for col in 0..n {
        // Find pivot.
        let mut max_row = col;
        let mut max_val = a[col * n + col].abs();
        for row in (col + 1)..n {
            let val = a[row * n + col].abs();
            if val > max_val {
                max_val = val;
                max_row = row;
            }
        }
        if max_val < 1e-14 {
            return Err(EconometricsError::SingularSystem);
        }
        if max_row != col {
            for j in 0..n {
                let tmp = a[col * n + j];
                a[col * n + j] = a[max_row * n + j];
                a[max_row * n + j] = tmp;
            }
            let tmp = b[col];
            b[col] = b[max_row];
            b[max_row] = tmp;
        }
        // Eliminate below.
        let pivot = a[col * n + col];
        for row in (col + 1)..n {
            let factor = a[row * n + col] / pivot;
            if !factor.is_finite() {
                return Err(EconometricsError::NonFinite);
            }
            a[row * n + col] = 0.0;
            for j in (col + 1)..n {
                a[row * n + j] -= factor * a[col * n + j];
            }
            b[row] -= factor * b[col];
        }
    }
    // Back-substitution.
    for row in (0..n).rev() {
        let mut acc = b[row];
        for j in (row + 1)..n {
            acc -= a[row * n + j] * b[j];
        }
        b[row] = acc / a[row * n + row];
        if !b[row].is_finite() {
            return Err(EconometricsError::NonFinite);
        }
    }
    Ok(())
}

/// Ordinary Least Squares via normal equations `X'X b = X'y`.
///
/// `x` is `n_obs x n_reg` row-major. `y` is length `n_obs`. Writes
/// coefficients into `coef_out[..n_reg]` and residuals into
/// `resid_out[..n_obs]`. Returns R-squared.
pub fn ols_into(
    x: &[f64],
    y: &[f64],
    n_obs: usize,
    n_reg: usize,
    coef_out: &mut [f64],
    resid_out: &mut [f64],
) -> Result<f64, EconometricsError> {
    if n_obs == 0
        || n_reg == 0
        || n_obs < n_reg
        || n_obs > MAX_OBSERVATIONS
        || n_reg > MAX_REGRESSORS
    {
        return Err(EconometricsError::InsufficientData);
    }
    if x.len() < n_obs * n_reg
        || y.len() < n_obs
        || coef_out.len() < n_reg
        || resid_out.len() < n_obs
    {
        return Err(EconometricsError::BufferTooSmall);
    }
    for v in x.iter().take(n_obs * n_reg) {
        if !v.is_finite() {
            return Err(EconometricsError::NonFinite);
        }
    }
    for v in y.iter().take(n_obs) {
        if !v.is_finite() {
            return Err(EconometricsError::NonFinite);
        }
    }

    let mut xtx = [0.0f64; MAX_REGRESSORS * MAX_REGRESSORS];
    let mut xty = [0.0f64; MAX_REGRESSORS];
    // X'X
    for i in 0..n_reg {
        for j in 0..n_reg {
            let mut acc = 0.0;
            for k in 0..n_obs {
                acc += x[k * n_reg + i] * x[k * n_reg + j];
            }
            xtx[i * n_reg + j] = acc;
        }
    }
    // X'y
    for i in 0..n_reg {
        let mut acc = 0.0;
        for k in 0..n_obs {
            acc += x[k * n_reg + i] * y[k];
        }
        xty[i] = acc;
    }

    gaussian_solve(&mut xtx, &mut xty, n_reg)?;

    for i in 0..n_reg {
        coef_out[i] = xty[i];
    }

    // Residuals and R-squared.
    let y_mean = {
        let mut sum = 0.0;
        for k in 0..n_obs {
            sum += y[k];
        }
        sum / n_obs as f64
    };
    let mut tss = 0.0;
    let mut rss = 0.0;
    for k in 0..n_obs {
        let mut fitted = 0.0;
        for i in 0..n_reg {
            fitted += coef_out[i] * x[k * n_reg + i];
        }
        let resid = y[k] - fitted;
        resid_out[k] = resid;
        rss += resid * resid;
        let dev = y[k] - y_mean;
        tss += dev * dev;
    }
    let r_sq = if tss > 0.0 { 1.0 - rss / tss } else { 0.0 };
    Ok(r_sq)
}

/// Weighted Least Squares: `(X'WX) b = X'Wy`.
///
/// `weights` is length `n_obs`. Writes coefficients and residuals.
pub fn wls_into(
    x: &[f64],
    y: &[f64],
    weights: &[f64],
    n_obs: usize,
    n_reg: usize,
    coef_out: &mut [f64],
    resid_out: &mut [f64],
) -> Result<f64, EconometricsError> {
    if n_obs == 0
        || n_reg == 0
        || n_obs < n_reg
        || n_obs > MAX_OBSERVATIONS
        || n_reg > MAX_REGRESSORS
    {
        return Err(EconometricsError::InsufficientData);
    }
    if x.len() < n_obs * n_reg
        || y.len() < n_obs
        || weights.len() < n_obs
        || coef_out.len() < n_reg
        || resid_out.len() < n_obs
    {
        return Err(EconometricsError::BufferTooSmall);
    }
    for v in weights.iter().take(n_obs) {
        if !v.is_finite() || *v < 0.0 {
            return Err(EconometricsError::NonFinite);
        }
    }

    let mut xtwx = [0.0f64; MAX_REGRESSORS * MAX_REGRESSORS];
    let mut xtwy = [0.0f64; MAX_REGRESSORS];
    for i in 0..n_reg {
        for j in 0..n_reg {
            let mut acc = 0.0;
            for k in 0..n_obs {
                acc += x[k * n_reg + i] * weights[k] * x[k * n_reg + j];
            }
            xtwx[i * n_reg + j] = acc;
        }
    }
    for i in 0..n_reg {
        let mut acc = 0.0;
        for k in 0..n_obs {
            acc += x[k * n_reg + i] * weights[k] * y[k];
        }
        xtwy[i] = acc;
    }

    gaussian_solve(&mut xtwx, &mut xtwy, n_reg)?;
    for i in 0..n_reg {
        coef_out[i] = xtwy[i];
    }

    let mut rss = 0.0;
    let mut wss = 0.0;
    let y_wmean = {
        let mut sw = 0.0;
        let mut swy = 0.0;
        for k in 0..n_obs {
            sw += weights[k];
            swy += weights[k] * y[k];
        }
        if sw > 0.0 {
            swy / sw
        } else {
            0.0
        }
    };
    for k in 0..n_obs {
        let mut fitted = 0.0;
        for i in 0..n_reg {
            fitted += coef_out[i] * x[k * n_reg + i];
        }
        let resid = y[k] - fitted;
        resid_out[k] = resid;
        rss += weights[k] * resid * resid;
        let dev = y[k] - y_wmean;
        wss += weights[k] * dev * dev;
    }
    let r_sq = if wss > 0.0 { 1.0 - rss / wss } else { 0.0 };
    Ok(r_sq)
}

/// Two-Stage Least Squares (2SLS).
///
/// First stage: regress endogenous `X` on instruments `Z`, get `X-hat`.
/// Second stage: OLS of `y` on `X-hat`. Refuses if `n_instr < n_reg`
/// (underidentified).
pub fn iv_2sls_into(
    x_endogenous: &[f64],
    z_instruments: &[f64],
    y: &[f64],
    n_obs: usize,
    n_reg: usize,
    n_instr: usize,
    coef_out: &mut [f64],
) -> Result<f64, EconometricsError> {
    if n_instr < n_reg {
        return Err(EconometricsError::Underidentified);
    }
    if n_obs == 0 || n_obs < n_reg || n_obs > MAX_OBSERVATIONS || n_reg > MAX_REGRESSORS {
        return Err(EconometricsError::InsufficientData);
    }
    if x_endogenous.len() < n_obs * n_reg
        || z_instruments.len() < n_obs * n_instr
        || y.len() < n_obs
        || coef_out.len() < n_reg
    {
        return Err(EconometricsError::BufferTooSmall);
    }

    // First stage: for each regressor, regress on instruments → x_hat.
    let mut x_hat = [0.0f64; MAX_OBSERVATIONS * MAX_REGRESSORS];
    let mut ztz = [0.0f64; MAX_REGRESSORS * MAX_REGRESSORS];
    let mut ztx_col = [0.0f64; MAX_REGRESSORS];
    let mut first_stage_coef = [0.0f64; MAX_REGRESSORS];

    for col in 0..n_reg {
        // Z'Z
        for i in 0..n_instr {
            for j in 0..n_instr {
                let mut acc = 0.0;
                for k in 0..n_obs {
                    acc += z_instruments[k * n_instr + i] * z_instruments[k * n_instr + j];
                }
                ztz[i * n_instr + j] = acc;
            }
        }
        // Z'x_col
        for i in 0..n_instr {
            let mut acc = 0.0;
            for k in 0..n_obs {
                acc += z_instruments[k * n_instr + i] * x_endogenous[k * n_reg + col];
            }
            ztx_col[i] = acc;
        }
        // Solve Z'Z * gamma = Z'x_col
        let mut ztz_copy = ztz;
        let mut ztx_copy = ztx_col;
        gaussian_solve(&mut ztz_copy[..n_instr * n_instr], &mut ztx_copy, n_instr)?;
        for i in 0..n_instr {
            first_stage_coef[i] = ztx_copy[i];
        }
        // x_hat[k, col] = Z[k] · gamma
        for k in 0..n_obs {
            let mut acc = 0.0;
            for i in 0..n_instr {
                acc += z_instruments[k * n_instr + i] * first_stage_coef[i];
            }
            x_hat[k * n_reg + col] = acc;
        }
    }

    // Second stage: OLS of y on x_hat.
    let mut resid = [0.0f64; MAX_OBSERVATIONS];
    ols_into(&x_hat, y, n_obs, n_reg, coef_out, &mut resid)
}

/// Logistic regression via Newton-Raphson (IRLS).
///
/// `x` is `n_obs x n_reg` row-major. `y_binary` in {0, 1}. Writes
/// coefficients into `coef_out[..n_reg]`. Returns convergence report.
pub fn logistic_mle_into(
    x: &[f64],
    y_binary: &[f64],
    n_obs: usize,
    n_reg: usize,
    max_iter: u32,
    tolerance: f64,
    coef_out: &mut [f64],
) -> Result<EconConvergence, EconometricsError> {
    use super::error::EconStatus;
    if n_obs == 0
        || n_reg == 0
        || n_obs < n_reg
        || n_obs > MAX_OBSERVATIONS
        || n_reg > MAX_REGRESSORS
    {
        return Err(EconometricsError::InsufficientData);
    }
    if x.len() < n_obs * n_reg || y_binary.len() < n_obs || coef_out.len() < n_reg {
        return Err(EconometricsError::BufferTooSmall);
    }
    for v in y_binary.iter().take(n_obs) {
        if !v.is_finite() || (*v != 0.0 && *v != 1.0) {
            return Err(EconometricsError::InvalidInput);
        }
    }

    // Initialize coef = 0.
    for i in 0..n_reg {
        coef_out[i] = 0.0;
    }
    let mut hessian = [0.0f64; MAX_REGRESSORS * MAX_REGRESSORS];
    let mut gradient = [0.0f64; MAX_REGRESSORS];
    let mut score = [0.0f64; MAX_OBSERVATIONS];

    for iter in 0..max_iter {
        // Compute p_k = sigmoid(X_k · coef) and score_k = y_k - p_k.
        for k in 0..n_obs {
            let mut eta = 0.0;
            for i in 0..n_reg {
                eta += x[k * n_reg + i] * coef_out[i];
            }
            let p = 1.0 / (1.0 + (-eta).exp());
            score[k] = y_binary[k] - p;
        }
        // Gradient = X' (y - p)
        for i in 0..n_reg {
            let mut acc = 0.0;
            for k in 0..n_obs {
                acc += x[k * n_reg + i] * score[k];
            }
            gradient[i] = acc;
        }
        // Hessian = -X' W X where W = diag(p(1-p))
        for i in 0..n_reg {
            for j in 0..n_reg {
                let mut acc = 0.0;
                for k in 0..n_obs {
                    let mut eta = 0.0;
                    for r in 0..n_reg {
                        eta += x[k * n_reg + r] * coef_out[r];
                    }
                    let p = 1.0 / (1.0 + (-eta).exp());
                    let w = p * (1.0 - p);
                    acc += x[k * n_reg + i] * w * x[k * n_reg + j];
                }
                hessian[i * n_reg + j] = -acc;
            }
        }
        // Solve Hessian * delta = gradient (Newton step).
        let mut h_copy = hessian;
        let mut g_copy = gradient;
        match gaussian_solve(&mut h_copy[..n_reg * n_reg], &mut g_copy, n_reg) {
            Ok(()) => {
                let mut delta_norm = 0.0;
                for i in 0..n_reg {
                    // Newton-Raphson maximizing the log-likelihood: β_new = β − H⁻¹∇L.
                    // `g_copy` is H⁻¹∇L (H = ∇²L = −X'WX, negative definite), so the
                    // ascent step subtracts it. (Adding it, as before, descended away
                    // from the MLE — coefficients moved the wrong direction.)
                    coef_out[i] -= g_copy[i];
                    delta_norm += g_copy[i] * g_copy[i];
                }
                let delta_norm = delta_norm.sqrt();
                if !delta_norm.is_finite() {
                    return Ok(EconConvergence::stalled(
                        EconStatus::NonFinite,
                        iter + 1,
                        delta_norm,
                    ));
                }
                if delta_norm < tolerance {
                    return Ok(EconConvergence::converged(iter + 1, delta_norm));
                }
            }
            Err(EconometricsError::SingularSystem) => {
                return Ok(EconConvergence::stalled(
                    EconStatus::Singular,
                    iter + 1,
                    0.0,
                ));
            }
            Err(e) => return Err(e),
        }
    }
    Ok(EconConvergence::stalled(
        EconStatus::MaxIterations,
        max_iter,
        0.0,
    ))
}

/// Evaluate GMM moment conditions: `g(theta) = (1/n) sum m_i(theta)`.
///
/// `moments` is a caller-supplied closure-free precomputed table:
/// `moments[i * n_moments + j]` = m_j(observation_i, params). Actually this
/// function computes the sample average of the supplied moment values.
/// `out` receives `n_moments` averaged moment conditions.
pub fn gmm_moment_eval(
    moment_values: &[f64],
    n_obs: usize,
    n_moments: usize,
    out: &mut [f64],
) -> Result<usize, EconometricsError> {
    if n_obs == 0
        || n_moments == 0
        || moment_values.len() < n_obs * n_moments
        || out.len() < n_moments
    {
        return Err(EconometricsError::InvalidInput);
    }
    for v in moment_values.iter().take(n_obs * n_moments) {
        if !v.is_finite() {
            return Err(EconometricsError::NonFinite);
        }
    }
    for j in 0..n_moments {
        let mut acc = 0.0;
        for i in 0..n_obs {
            acc += moment_values[i * n_moments + j];
        }
        out[j] = acc / n_obs as f64;
    }
    Ok(n_moments)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn ols_recovers_exact_linear() {
        // y = 2 + 3x, x = [1, 2, 3, 4, 5]
        // Design matrix with constant: [[1, 1], [1, 2], [1, 3], [1, 4], [1, 5]]
        let x = [1.0, 1.0, 1.0, 2.0, 1.0, 3.0, 1.0, 4.0, 1.0, 5.0];
        let y = [5.0, 8.0, 11.0, 14.0, 17.0];
        let mut coef = [0.0f64; 2];
        let mut resid = [0.0f64; 5];
        let r_sq = ols_into(&x, &y, 5, 2, &mut coef, &mut resid).unwrap();
        assert!(approx(coef[0], 2.0));
        assert!(approx(coef[1], 3.0));
        assert!(approx(r_sq, 1.0));
        for r in resid.iter() {
            assert!(approx(*r, 0.0));
        }
    }

    #[test]
    fn ols_insufficient_data() {
        let x = [1.0, 1.0];
        let y = [1.0];
        let mut coef = [0.0f64; 2];
        let mut resid = [0.0f64];
        let err = ols_into(&x, &y, 1, 2, &mut coef, &mut resid).unwrap_err();
        assert_eq!(err, EconometricsError::InsufficientData);
    }

    #[test]
    fn ols_singular_system_multicollinearity() {
        // Two identical columns → singular X'X
        let x = [1.0, 2.0, 1.0, 2.0, 1.0, 2.0, 1.0, 2.0];
        let y = [1.0, 2.0, 3.0, 4.0];
        let mut coef = [0.0f64; 2];
        let mut resid = [0.0f64; 4];
        let err = ols_into(&x, &y, 4, 2, &mut coef, &mut resid).unwrap_err();
        assert_eq!(err, EconometricsError::SingularSystem);
    }

    #[test]
    fn wls_recovers_weighted_fit() {
        // Simple: y = x, weights all 1 → same as OLS
        let x = [1.0, 1.0, 1.0, 2.0, 1.0, 3.0];
        let y = [1.0, 2.0, 3.0];
        let w = [1.0, 1.0, 1.0];
        let mut coef = [0.0f64; 2];
        let mut resid = [0.0f64; 3];
        let r_sq = wls_into(&x, &y, &w, 3, 2, &mut coef, &mut resid).unwrap();
        assert!(approx(coef[0], 0.0));
        assert!(approx(coef[1], 1.0));
        assert!(approx(r_sq, 1.0));
    }

    #[test]
    fn iv_2sls_underidentified() {
        let x = [1.0, 1.0, 1.0, 1.0];
        let z = [1.0, 1.0]; // 1 instrument for 2 regressors
        let y = [1.0, 2.0];
        let mut coef = [0.0f64; 2];
        let err = iv_2sls_into(&x, &z, &y, 2, 2, 1, &mut coef).unwrap_err();
        assert_eq!(err, EconometricsError::Underidentified);
    }

    #[test]
    fn logistic_mle_separable() {
        // Perfectly separable: y=1 when x>0, y=0 when x<0
        // Design: [[1, -5], [1, -3], [1, -1], [1, 1], [1, 3], [1, 5]]
        // y = [0, 0, 0, 1, 1, 1]
        let x = [
            1.0, -5.0, 1.0, -3.0, 1.0, -1.0, 1.0, 1.0, 1.0, 3.0, 1.0, 5.0,
        ];
        let y = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let mut coef = [0.0f64; 2];
        let conv = logistic_mle_into(&x, &y, 6, 2, 100, 1e-8, &mut coef).unwrap();
        // For separable data, coef[1] should be large positive.
        assert!(coef[1] > 1.0, "coef[1] = {}", coef[1]);
        let _ = conv;
    }

    #[test]
    fn logistic_mle_rejects_non_binary() {
        // 4 observations × 2 regressors → the design needs 8 values. (The prior
        // test passed only 4, so it tripped the BufferTooSmall guard before ever
        // reaching the binary-y check it meant to exercise.) With a correctly
        // sized design, the non-binary y=0.5 is what triggers InvalidInput.
        let x = [1.0, 0.0, 1.0, 1.0, 1.0, 2.0, 1.0, 3.0];
        let y = [0.0, 0.5, 1.0, 1.0];
        let mut coef = [0.0f64; 2];
        let err = logistic_mle_into(&x, &y, 4, 2, 100, 1e-8, &mut coef).unwrap_err();
        assert_eq!(err, EconometricsError::InvalidInput);
    }

    #[test]
    fn gmm_moment_eval_averages() {
        // 3 observations, 2 moments: m = [[1, 2], [3, 4], [5, 6]]
        let m = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let mut out = [0.0f64; 2];
        gmm_moment_eval(&m, 3, 2, &mut out).unwrap();
        assert!(approx(out[0], 3.0)); // (1+3+5)/3
        assert!(approx(out[1], 4.0)); // (2+4+6)/3
    }

    #[test]
    fn calibration_record_construction() {
        let rec = CalibrationRecord::new("ols_v1", 0xDEAD_BEEF, 3, 0.05, 0, 12345);
        assert_eq!(rec.model_name, "ols_v1");
        assert_eq!(rec.data_hash, 0xDEAD_BEEF);
        assert_eq!(rec.n_params, 3);
        assert!(approx(rec.loss, 0.05));
    }

    #[test]
    fn buffer_too_small_errors() {
        let x = [1.0, 1.0, 1.0, 2.0];
        let y = [1.0, 2.0];
        let mut coef = [0.0f64; 1]; // too small
        let mut resid = [0.0f64; 2];
        let err = ols_into(&x, &y, 2, 2, &mut coef, &mut resid).unwrap_err();
        assert_eq!(err, EconometricsError::BufferTooSmall);
    }
}
