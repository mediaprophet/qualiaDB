//! Held-out evaluation and conversion receipt generation (W5: EOS-052).
//!
//! Evaluates candidate hybrid operators on separate held-out validation samples:
//! - Computes Frobenius norm error against ground-truth weights.
//! - Computes activation-weighted output deviation on untouched validation tokens.
//! - Enforces that no production promotion occurs without verified receipts.

use std::time::Instant;
use serde::{Deserialize, Serialize};
use super::activation_stats::ConverterError;
use super::hybrid_fit::{fit_hybrid_decomposition, HybridDecomposition, HybridFitConfig};

/// Comprehensive receipt capturing conversion cost and held-out validation metrics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConversionReceipt {
    pub rows: usize,
    pub cols: usize,
    pub rank: usize,
    pub sparse_entries_count: usize,
    pub frobenius_err_base: f32,
    pub frobenius_err_hybrid: f32,
    pub relative_gain_frobenius: f32,
    pub held_out_mse_base: f32,
    pub held_out_mse_hybrid: f32,
    pub relative_gain_held_out: f32,
    pub conversion_duration_micros: u64,
    pub peak_scratch_bytes: usize,
    pub passed_quality_gate: bool,
}

/// Evaluates a hybrid decomposition against base quantized weights on held-out activation vectors.
pub fn evaluate_conversion(
    rows: usize,
    cols: usize,
    w_source: &[f32],
    base_q: &[f32],
    calibration_acts: &[Vec<f32>],
    held_out_acts: &[Vec<f32>],
    config: &HybridFitConfig,
    min_required_gain: f32,
) -> Result<(HybridDecomposition, ConversionReceipt), ConverterError> {
    if w_source.len() != rows * cols || base_q.len() != rows * cols {
        return Err(ConverterError::DimensionMismatch {
            expected: rows * cols,
            got: w_source.len(),
        });
    }

    let start_time = Instant::now();

    // 1. Gather diagonal weights from calibration samples if provided
    let mut act_weights = vec![1.0f32; cols];
    if !calibration_acts.is_empty() {
        let mut sum_sq = vec![0.0f64; cols];
        for act in calibration_acts {
            if act.len() != cols {
                return Err(ConverterError::DimensionMismatch {
                    expected: cols,
                    got: act.len(),
                });
            }
            for j in 0..cols {
                sum_sq[j] += (act[j] as f64).powi(2);
            }
        }
        let inv_n = 1.0 / calibration_acts.len() as f64;
        for j in 0..cols {
            act_weights[j] = ((sum_sq[j] * inv_n) + 1e-4).sqrt() as f32;
        }
    }

    // 2. Fit hybrid decomposition
    let hybrid = fit_hybrid_decomposition(
        rows,
        cols,
        w_source,
        base_q,
        Some(&act_weights),
        config,
    )?;

    let duration_micros = start_time.elapsed().as_micros() as u64;

    // 3. Compute Frobenius errors
    let mut f_err_base_sq = 0.0f64;
    let mut f_err_hybrid_sq = 0.0f64;
    let w_hat = hybrid.reconstruct_dense();

    for idx in 0..rows * cols {
        let target = w_source[idx] as f64;
        let base = base_q[idx] as f64;
        let hat = w_hat[idx] as f64;

        f_err_base_sq += (target - base).powi(2);
        f_err_hybrid_sq += (target - hat).powi(2);
    }

    let frobenius_err_base = (f_err_base_sq / (rows * cols) as f64).sqrt() as f32;
    let frobenius_err_hybrid = (f_err_hybrid_sq / (rows * cols) as f64).sqrt() as f32;

    let relative_gain_frobenius = if frobenius_err_base > 1e-12 {
        (frobenius_err_base - frobenius_err_hybrid) / frobenius_err_base
    } else {
        0.0
    };

    // 4. Compute Held-Out Activation MSE
    let mut held_out_base_sq = 0.0f64;
    let mut held_out_hybrid_sq = 0.0f64;
    let mut total_held_out_elems = 0usize;

    for act in held_out_acts {
        if act.len() != cols {
            return Err(ConverterError::DimensionMismatch {
                expected: cols,
                got: act.len(),
            });
        }

        // True y = W * x
        let mut y_true = vec![0.0f32; rows];
        for i in 0..rows {
            let mut dot = 0.0f32;
            let offset = i * cols;
            for j in 0..cols {
                dot += w_source[offset + j] * act[j];
            }
            y_true[i] = dot;
        }

        // Base y = Base_Q * x
        let mut y_base = vec![0.0f32; rows];
        for i in 0..rows {
            let mut dot = 0.0f32;
            let offset = i * cols;
            for j in 0..cols {
                dot += base_q[offset + j] * act[j];
            }
            y_base[i] = dot;
        }

        // Hybrid y = (Q + AB + S) * x
        let mut y_hybrid = vec![0.0f32; rows];
        hybrid.apply_gemv(act, &mut y_hybrid)?;

        for i in 0..rows {
            held_out_base_sq += ((y_true[i] - y_base[i]) as f64).powi(2);
            held_out_hybrid_sq += ((y_true[i] - y_hybrid[i]) as f64).powi(2);
        }
        total_held_out_elems += rows;
    }

    let (held_out_mse_base, held_out_mse_hybrid, relative_gain_held_out) =
        if total_held_out_elems > 0 {
            let mse_base = (held_out_base_sq / total_held_out_elems as f64) as f32;
            let mse_hybrid = (held_out_hybrid_sq / total_held_out_elems as f64) as f32;
            let gain = if mse_base > 1e-12 {
                (mse_base - mse_hybrid) / mse_base
            } else {
                0.0
            };
            (mse_base, mse_hybrid, gain)
        } else {
            (0.0, 0.0, 0.0)
        };

    let passed_quality_gate = relative_gain_frobenius >= min_required_gain
        && (total_held_out_elems == 0 || relative_gain_held_out >= min_required_gain);

    let peak_scratch_bytes = (rows * cols * 4) * 3 + (rows * config.rank * 4) + (config.rank * cols * 4);

    let receipt = ConversionReceipt {
        rows,
        cols,
        rank: config.rank,
        sparse_entries_count: hybrid.sparse_entries.len(),
        frobenius_err_base,
        frobenius_err_hybrid,
        relative_gain_frobenius,
        held_out_mse_base,
        held_out_mse_hybrid,
        relative_gain_held_out,
        conversion_duration_micros: duration_micros,
        peak_scratch_bytes,
        passed_quality_gate,
    };

    Ok((hybrid, receipt))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversion_evaluation_held_out_gain() {
        let rows = 12;
        let cols = 16;
        let mut w_source = vec![0.0f32; rows * cols];
        let mut base_q = vec![0.0f32; rows * cols];

        for i in 0..rows {
            for j in 0..cols {
                let v = ((i * 11 + j * 17) as f32 * 0.1).sin();
                w_source[i * cols + j] = v;
                base_q[i * cols + j] = (v * 3.0).round() / 3.0; // coarse quant
            }
        }

        // Distinct calibration and held-out activations
        let cal_acts = vec![vec![1.0f32; cols], vec![0.5f32; cols]];
        let held_out_acts = vec![
            vec![0.8f32; cols],
            (0..cols).map(|c| (c as f32 * 0.2).cos()).collect(),
        ];

        let config = HybridFitConfig {
            rank: 3,
            max_sparse_ratio: 0.05,
            outlier_threshold_std: 1.2,
            num_power_iters: 15,
            max_budget_bytes: 42 * 1024 * 1024,
        };

        let (_hybrid, receipt) = evaluate_conversion(
            rows,
            cols,
            &w_source,
            &base_q,
            &cal_acts,
            &held_out_acts,
            &config,
            0.05, // require at least 5% gain
        )
        .unwrap();

        assert!(receipt.frobenius_err_hybrid < receipt.frobenius_err_base);
        assert!(receipt.relative_gain_frobenius > 0.05);
        assert!(receipt.held_out_mse_hybrid < receipt.held_out_mse_base);
        assert!(receipt.passed_quality_gate);
        assert!(receipt.peak_scratch_bytes < 42 * 1024 * 1024);
    }
}
