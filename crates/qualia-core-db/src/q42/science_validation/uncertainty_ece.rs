//! Uncertainty calibration, Gaussian coverage factor, covariance rotation, Mahalanobis NEES,
//! and Expected Calibration Error (ECE) verifiers (C02-C03, C02-C04, C02-C05, Modes 6 & 7).
//!
//! All reference calculations are evaluated in independent `f64` arithmetic:
//! - Target quantity, unit, and uncertainty kind admission (`StandardAbsolute`, `ExpandedAbsolute`, `Relative`).
//! - One-shot affine calibration $\text{physical} = \text{scale}\cdot\text{raw} + \text{offset}$ with
//!   idempotence guard against double-scaling already calibrated streams.
//! - Half-open calibration validity interval $[t_{\text{from}}, t_{\text{until}})$ with exact boundary rejection.
//! - Exact positive-term Kummer-confluent series $\operatorname{erf}(k/\sqrt{2})$ verifying Gaussian
//!   coverage factor $k \leftrightarrow p$ consistency ($k=1 \leftrightarrow 68.268949\%$,
//!   $k=1.959964 \leftrightarrow 95\%$, $k=2 \leftrightarrow 95.449974\%$, $k=3 \leftrightarrow 99.730020\%$).
//! - $3\times 3$ covariance symmetry, positive-definiteness, frame rotation $\boldsymbol{\Sigma}' = \mathbf{R}\boldsymbol{\Sigma}\mathbf{R}^T$,
//!   Mahalanobis distance / NEES, ECE, Brier score, and Gaussian NLL.

use crate::q42::science_validation::stokes_mueller::{C02_ROUNDING_FACTOR, F64_EPS};

/// Uncertainty representation kind required by C02-C05 and Modes 6/7.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UncertaintyKind {
    /// Standard uncertainty $1\sigma$ in the target quantity's physical units.
    StandardAbsolute,
    /// Expanded uncertainty $U = k \cdot u_c$ with explicit coverage factor $k > 0$
    /// and Gaussian coverage probability $p \in (0, 1)$.
    ExpandedAbsolute {
        coverage_factor_k: f64,
        confidence_probability: f64,
    },
    /// Dimensionless relative standard uncertainty $u_r = u(x)/|x|$.
    Relative,
}

/// Calibration stage of a sample stream (C02-C05 & QAC Schema 210).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationStage {
    /// Uncalibrated raw ADC/sensor counts (`raw-counts`).
    RawCounts,
    /// Already calibrated physical base units (`physical-base-units`, `units_per_sample == 1.0`).
    CalibratedPhysical,
    /// Digital values unrelated to physical pressure/field (`digital-unrelated-to-pressure`).
    DigitalUnrelated,
}

/// Uncertainty descriptor bound to a specific quantity and unit (C02-C05).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UncertaintyDescriptor<'a> {
    pub target_quantity: &'a str,
    pub target_unit: &'a str,
    pub kind: UncertaintyKind,
    pub value: f64,
    pub record_has_mixed_units: bool,
    pub broadcast_across_all_fields: bool,
}

/// Structured rejection reasons for calibration, uncertainty, and statistical consistency checks.
#[derive(Debug, Clone, PartialEq)]
pub enum UncertaintyValidationError {
    /// Uncertainty value or parameter is NaN, infinite, or negative.
    InvalidUncertaintyValue { value: f64 },
    /// Uncertainty lane is marked valid but lacks an explicit target quantity or unit string.
    MissingTargetQuantityOrUnit,
    /// A single unlabeled scalar uncertainty was broadcast across a mixed-unit record (C02-C05 / EMF-A05).
    MixedUnitScalarBroadcastForbidden,
    /// Coverage factor $k$ and claimed probability $p$ do not satisfy $p = \operatorname{erf}(k/\sqrt{2})$.
    GaussianCoverageMismatch {
        k: f64,
        claimed_probability: f64,
        expected_probability: f64,
        abs_diff: f64,
        tolerance: f64,
    },
    /// Affine calibration scale is non-positive or offset/scale is non-finite.
    InvalidAffineCalibration { scale: f64, offset: f64 },
    /// Attempted to reapply raw affine calibration to a stream already in `CalibratedPhysical` stage,
    /// or `CalibratedPhysical` stream has `units_per_sample != 1.0` or `offset != 0.0`.
    DoubleCalibrationForbidden {
        stage: CalibrationStage,
        units_per_sample: f64,
        offset: f64,
    },
    /// Invalid or expired half-open calibration interval `[from_tick, until_tick)`.
    CalibrationWindowExpiredOrInvalid {
        sample_tick: i128,
        from_tick: i128,
        until_tick: i128,
    },
    /// Covariance matrix is non-finite, asymmetric, or not positive definite.
    InvalidCovarianceMatrix {
        reason: &'static str,
        value: f64,
    },
    /// Covariance rotation $\mathbf{R}\boldsymbol{\Sigma}\mathbf{R}^T$ violated trace or determinant invariance.
    CovarianceRotationInvarianceViolation {
        trace_before: f64,
        trace_after: f64,
        det_before: f64,
        det_after: f64,
    },
    /// Invalid probability or empty sample set in ECE / Brier / NLL / NEES evaluation.
    InvalidStatisticalSample { reason: &'static str },
    /// Empirical calibration error or NEES exceeded the declared acceptance threshold.
    StatisticalCalibrationExceeded {
        metric_name: &'static str,
        observed: f64,
        threshold: f64,
    },
}

/// Evaluate $\operatorname{erf}(x)$ in independent `f64` arithmetic with zero-cancellation
/// positive-term Kummer series for $|x| \le 5.5$ and Laplace continued fraction for $|x| > 5.5$.
pub fn erf_f64(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x == 0.0 {
        return x;
    }
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let ax = x.abs();
    if ax > 6.5 {
        return sign;
    }
    if ax <= 5.5 {
        // erf(x) = (2 / sqrt(pi)) * exp(-x^2) * sum_{n=0}^\infty (2^n x^{2n+1}) / (2n+1)!!
        // All terms for ax > 0 are strictly positive, eliminating alternating cancellation.
        let two_x2 = 2.0 * ax * ax;
        let mut term = ax;
        let mut sum = term;
        for n in 1..140 {
            term *= two_x2 / ((2 * n + 1) as f64);
            sum += term;
            if term <= sum * 1.0e-17 {
                break;
            }
        }
        let two_over_sqrt_pi = 2.0 / core::f64::consts::PI.sqrt();
        let val = two_over_sqrt_pi * (-ax * ax).exp() * sum;
        sign * val.min(1.0)
    } else {
        // Laplace continued fraction for erfc(x) = (exp(-x^2) / (sqrt(pi) * x)) * CF
        let mut f = 1.0e-30;
        let mut c = f;
        let mut d = 0.0;
        for m in 1..60 {
            let a = 0.5 * (m as f64);
            let b = if m % 2 == 1 { ax } else { ax };
            let _ = b;
            d = ax + a * d;
            if d.abs() < 1.0e-30 {
                d = 1.0e-30;
            }
            c = ax + a / c;
            if c.abs() < 1.0e-30 {
                c = 1.0e-30;
            }
            d = 1.0 / d;
            let delta = c * d;
            f *= delta;
            if (delta - 1.0).abs() < 1.0e-16 {
                break;
            }
        }
        let erfc_approx = ((-ax * ax).exp() / core::f64::consts::PI.sqrt()) * d;
        let _ = f;
        sign * (1.0 - erfc_approx).clamp(0.0, 1.0)
    }
}

/// Compute two-sided Gaussian coverage probability $p(k) = \operatorname{erf}(k / \sqrt{2})$ for $k > 0$.
#[inline]
pub fn gaussian_coverage_probability(k: f64) -> f64 {
    erf_f64(k / core::f64::consts::SQRT_2)
}

/// Verify consistency between a Gaussian coverage factor $k > 0$ and claimed confidence probability $p \in (0, 1)$.
pub fn verify_gaussian_coverage_factor(
    k: f64,
    claimed_probability: f64,
    tolerance: f64,
) -> Result<f64, UncertaintyValidationError> {
    if !k.is_finite()
        || k <= 0.0
        || !claimed_probability.is_finite()
        || claimed_probability <= 0.0
        || claimed_probability >= 1.0
        || !tolerance.is_finite()
        || tolerance <= 0.0
    {
        return Err(UncertaintyValidationError::InvalidUncertaintyValue { value: k });
    }
    let expected = gaussian_coverage_probability(k);
    let abs_diff = (claimed_probability - expected).abs();
    if abs_diff > tolerance {
        return Err(UncertaintyValidationError::GaussianCoverageMismatch {
            k,
            claimed_probability,
            expected_probability: expected,
            abs_diff,
            tolerance,
        });
    }
    Ok(expected)
}

/// Validate a C02-C05 / Mode 6 / Mode 7 uncertainty descriptor.
pub fn verify_uncertainty_descriptor(
    desc: &UncertaintyDescriptor<'_>,
    probability_tolerance: f64,
) -> Result<(), UncertaintyValidationError> {
    if !desc.value.is_finite() || desc.value < 0.0 {
        return Err(UncertaintyValidationError::InvalidUncertaintyValue { value: desc.value });
    }
    if desc.target_quantity.trim().is_empty() || desc.target_unit.trim().is_empty() {
        return Err(UncertaintyValidationError::MissingTargetQuantityOrUnit);
    }
    if desc.record_has_mixed_units && desc.broadcast_across_all_fields {
        return Err(UncertaintyValidationError::MixedUnitScalarBroadcastForbidden);
    }
    if let UncertaintyKind::ExpandedAbsolute {
        coverage_factor_k,
        confidence_probability,
    } = desc.kind
    {
        verify_gaussian_coverage_factor(
            coverage_factor_k,
            confidence_probability,
            probability_tolerance,
        )?;
    }
    Ok(())
}

/// Apply one-shot affine calibration $\text{physical} = \text{scale} \cdot \text{raw} + \text{offset}$
/// to a raw-count sample, or verify that an already calibrated physical stream has
/// `units_per_sample == 1.0` and `offset == 0.0` without re-scaling (C02-C05).
pub fn apply_or_verify_affine_calibration(
    sample: f64,
    stage: CalibrationStage,
    units_per_stored_sample: f64,
    stored_offset: f64,
    calibration_scale: f64,
    calibration_offset: f64,
) -> Result<(f64, CalibrationStage), UncertaintyValidationError> {
    if !sample.is_finite()
        || !units_per_stored_sample.is_finite()
        || !stored_offset.is_finite()
        || !calibration_scale.is_finite()
        || !calibration_offset.is_finite()
        || units_per_stored_sample <= 0.0
        || calibration_scale <= 0.0
    {
        return Err(UncertaintyValidationError::InvalidAffineCalibration {
            scale: calibration_scale,
            offset: calibration_offset,
        });
    }
    match stage {
        CalibrationStage::RawCounts => {
            if (units_per_stored_sample - 1.0).abs() > F64_EPS || stored_offset != 0.0 {
                return Err(UncertaintyValidationError::DoubleCalibrationForbidden {
                    stage,
                    units_per_sample: units_per_stored_sample,
                    offset: stored_offset,
                });
            }
            let physical = calibration_scale * sample + calibration_offset;
            Ok((physical, CalibrationStage::CalibratedPhysical))
        }
        CalibrationStage::CalibratedPhysical => {
            // Already calibrated physical samples must have units_per_sample == 1 and offset == 0;
            // calibration is retained as lineage only and never re-applied on read.
            if (units_per_stored_sample - 1.0).abs() > F64_EPS || stored_offset != 0.0 {
                return Err(UncertaintyValidationError::DoubleCalibrationForbidden {
                    stage,
                    units_per_sample: units_per_stored_sample,
                    offset: stored_offset,
                });
            }
            Ok((sample, CalibrationStage::CalibratedPhysical))
        }
        CalibrationStage::DigitalUnrelated => {
            Err(UncertaintyValidationError::DoubleCalibrationForbidden {
                stage,
                units_per_sample: units_per_stored_sample,
                offset: stored_offset,
            })
        }
    }
}

/// Verify half-open calibration validity window $[t_{\text{from}}, t_{\text{until}})$ (C02-C03 & C02-C05).
/// Exact boundary rule: $t_{\text{from}} < t_{\text{until}}$ and $t_{\text{from}} \le t < t_{\text{until}}$.
pub fn verify_half_open_calibration_window(
    sample_tick: i128,
    from_tick: i128,
    until_tick: i128,
) -> Result<(), UncertaintyValidationError> {
    if from_tick >= until_tick || sample_tick < from_tick || sample_tick >= until_tick {
        return Err(
            UncertaintyValidationError::CalibrationWindowExpiredOrInvalid {
                sample_tick,
                from_tick,
                until_tick,
            },
        );
    }
    Ok(())
}

/// Verify that a $3\times 3$ covariance matrix $\boldsymbol{\Sigma}$ is finite, symmetric, and
/// strictly positive definite (Sylvester's criterion on all leading principal minors).
pub fn verify_covariance_3x3_spd(
    cov: &[[f64; 3]; 3],
) -> Result<(f64, f64), UncertaintyValidationError> {
    let mut frob_sq = 0.0f64;
    for r in 0..3 {
        for c in 0..3 {
            let v = cov[r][c];
            if !v.is_finite() {
                return Err(UncertaintyValidationError::InvalidCovarianceMatrix {
                    reason: "non_finite_entry",
                    value: v,
                });
            }
            frob_sq += v * v;
        }
    }
    let frob = frob_sq.sqrt();
    let sym_tol = C02_ROUNDING_FACTOR * frob.max(1.0);
    for r in 0..3 {
        for c in (r + 1)..3 {
            let diff = (cov[r][c] - cov[c][r]).abs();
            if diff > sym_tol {
                return Err(UncertaintyValidationError::InvalidCovarianceMatrix {
                    reason: "asymmetric_covariance",
                    value: diff,
                });
            }
        }
    }
    let m1 = cov[0][0];
    let m2 = cov[0][0] * cov[1][1] - cov[0][1] * cov[1][0];
    let det = determinant_3x3(cov);
    if m1 <= 0.0 || m2 <= 0.0 || det <= 0.0 {
        return Err(UncertaintyValidationError::InvalidCovarianceMatrix {
            reason: "not_positive_definite",
            value: det.min(m1).min(m2),
        });
    }
    let trace = cov[0][0] + cov[1][1] + cov[2][2];
    Ok((trace, det))
}

/// Rotate a $3\times 3$ covariance or Cauchy stress tensor by a proper rotation matrix $\mathbf{R}$:
/// $\boldsymbol{\Sigma}' = \mathbf{R}\boldsymbol{\Sigma}\mathbf{R}^T$ (C02-C04 & C02-C05), verifying trace
/// and determinant invariance.
pub fn rotate_covariance_3x3(
    cov: &[[f64; 3]; 3],
    rot: &[[f64; 3]; 3],
) -> Result<[[f64; 3]; 3], UncertaintyValidationError> {
    let (trace_before, det_before) = verify_covariance_3x3_spd(cov)?;
    let mut temp = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            temp[i][j] = rot[i][0] * cov[0][j] + rot[i][1] * cov[1][j] + rot[i][2] * cov[2][j];
        }
    }
    let mut out = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = temp[i][0] * rot[j][0] + temp[i][1] * rot[j][1] + temp[i][2] * rot[j][2];
        }
    }
    let (trace_after, det_after) = verify_covariance_3x3_spd(&out)?;
    let trace_tol = C02_ROUNDING_FACTOR * trace_before.abs().max(1.0);
    let det_tol = C02_ROUNDING_FACTOR * det_before.abs().max(1.0e-18);
    if (trace_after - trace_before).abs() > trace_tol || (det_after - det_before).abs() > det_tol {
        return Err(
            UncertaintyValidationError::CovarianceRotationInvarianceViolation {
                trace_before,
                trace_after,
                det_before,
                det_after,
            },
        );
    }
    Ok(out)
}

/// Compute squared Mahalanobis distance (Normalized Estimation Error Squared, NEES)
/// $d_M^2 = (\mathbf{x} - \boldsymbol{\mu})^T \boldsymbol{\Sigma}^{-1} (\mathbf{x} - \boldsymbol{\mu})$ in $\mathbb{R}^3$.
pub fn mahalanobis_nees_3d(
    observation: [f64; 3],
    mean: [f64; 3],
    cov: &[[f64; 3]; 3],
) -> Result<f64, UncertaintyValidationError> {
    let (_, det) = verify_covariance_3x3_spd(cov)?;
    let dx = [
        observation[0] - mean[0],
        observation[1] - mean[1],
        observation[2] - mean[2],
    ];
    for &d in &dx {
        if !d.is_finite() {
            return Err(UncertaintyValidationError::InvalidUncertaintyValue { value: d });
        }
    }
    // Cofactor inverse of 3x3 symmetric SPD matrix:
    let c00 = cov[1][1] * cov[2][2] - cov[1][2] * cov[2][1];
    let c01 = -(cov[1][0] * cov[2][2] - cov[1][2] * cov[2][0]);
    let c02 = cov[1][0] * cov[2][1] - cov[1][1] * cov[2][0];
    let c11 = cov[0][0] * cov[2][2] - cov[0][2] * cov[2][0];
    let c12 = -(cov[0][0] * cov[2][1] - cov[0][1] * cov[2][0]);
    let c22 = cov[0][0] * cov[1][1] - cov[0][1] * cov[1][0];

    let inv_dx = [
        (c00 * dx[0] + c01 * dx[1] + c02 * dx[2]) / det,
        (c01 * dx[0] + c11 * dx[1] + c12 * dx[2]) / det,
        (c02 * dx[0] + c12 * dx[1] + c22 * dx[2]) / det,
    ];
    let nees = dx[0] * inv_dx[0] + dx[1] * inv_dx[1] + dx[2] * inv_dx[2];
    Ok(nees.max(0.0))
}

/// Summary of probabilistic calibration metrics (ECE, MCE, Brier score).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalibrationMetricsReport {
    pub expected_calibration_error: f64,
    pub maximum_calibration_error: f64,
    pub brier_score: f64,
    pub sample_count: usize,
}

/// Compute Expected Calibration Error (ECE), Maximum Calibration Error (MCE), and Brier score
/// over equal-width probability bins in $[0, 1]$.
pub fn evaluate_ece_and_brier(
    predicted_probabilities: &[f64],
    binary_outcomes: &[bool],
    num_bins: usize,
    max_allowed_ece: f64,
) -> Result<CalibrationMetricsReport, UncertaintyValidationError> {
    let n = predicted_probabilities.len();
    if n == 0 || n != binary_outcomes.len() || num_bins == 0 || num_bins > 64 {
        return Err(UncertaintyValidationError::InvalidStatisticalSample {
            reason: "invalid_sample_or_bin_count",
        });
    }
    let mut bin_counts = [0usize; 64];
    let mut bin_prob_sum = [0.0f64; 64];
    let mut bin_acc_sum = [0.0f64; 64];
    let mut brier_sum = 0.0f64;

    for (&p, &y) in predicted_probabilities.iter().zip(binary_outcomes.iter()) {
        if !p.is_finite() || !(0.0..=1.0).contains(&p) {
            return Err(UncertaintyValidationError::InvalidStatisticalSample {
                reason: "probability_out_of_unit_interval",
            });
        }
        let outcome = if y { 1.0 } else { 0.0 };
        let diff = p - outcome;
        brier_sum += diff * diff;

        let mut bin_idx = (p * (num_bins as f64)).floor() as usize;
        if bin_idx >= num_bins {
            bin_idx = num_bins - 1;
        }
        bin_counts[bin_idx] += 1;
        bin_prob_sum[bin_idx] += p;
        bin_acc_sum[bin_idx] += outcome;
    }

    let n_f64 = n as f64;
    let mut ece = 0.0f64;
    let mut mce = 0.0f64;
    for b in 0..num_bins {
        let count = bin_counts[b];
        if count > 0 {
            let avg_conf = bin_prob_sum[b] / (count as f64);
            let avg_acc = bin_acc_sum[b] / (count as f64);
            let gap = (avg_acc - avg_conf).abs();
            ece += ((count as f64) / n_f64) * gap;
            if gap > mce {
                mce = gap;
            }
        }
    }
    let brier_score = brier_sum / n_f64;
    if ece > max_allowed_ece {
        return Err(UncertaintyValidationError::StatisticalCalibrationExceeded {
            metric_name: "ECE",
            observed: ece,
            threshold: max_allowed_ece,
        });
    }

    Ok(CalibrationMetricsReport {
        expected_calibration_error: ece,
        maximum_calibration_error: mce,
        brier_score,
        sample_count: n,
    })
}

/// Compute univariate Gaussian Negative Log-Likelihood (NLL):
/// $\text{NLL} = \frac{1}{N} \sum_{i=1}^N \left[\frac{1}{2}\ln(2\pi \sigma_i^2) + \frac{(y_i - \mu_i)^2}{2\sigma_i^2}\right]$.
pub fn evaluate_gaussian_nll(
    observations: &[f64],
    means: &[f64],
    std_devs: &[f64],
) -> Result<f64, UncertaintyValidationError> {
    let n = observations.len();
    if n == 0 || n != means.len() || n != std_devs.len() {
        return Err(UncertaintyValidationError::InvalidStatisticalSample {
            reason: "mismatched_nll_slice_lengths",
        });
    }
    let two_pi = 2.0 * core::f64::consts::PI;
    let mut sum_nll = 0.0f64;
    for i in 0..n {
        let y = observations[i];
        let mu = means[i];
        let sigma = std_devs[i];
        if !y.is_finite() || !mu.is_finite() || !sigma.is_finite() || sigma <= 0.0 {
            return Err(UncertaintyValidationError::InvalidStatisticalSample {
                reason: "invalid_gaussian_nll_parameters",
            });
        }
        let var = sigma * sigma;
        let z = (y - mu) / sigma;
        sum_nll += 0.5 * ((two_pi * var).ln() + z * z);
    }
    Ok(sum_nll / (n as f64))
}

#[inline]
fn determinant_3x3(m: &[[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}
