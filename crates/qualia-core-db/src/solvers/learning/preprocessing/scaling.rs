//! Feature scaling — z-score standardization of a row-major feature matrix.
//! Reuses `statistics::descriptive` for the column mean / std-dev.

use crate::solvers::statistics::descriptive::{mean, std_dev};

/// Standardizes each column to zero mean / unit variance. `fit` learns the column
/// means and std-devs; `transform` applies `(x − μ)/σ`. A zero-variance column is
/// left centered (σ treated as 1) rather than producing NaNs.
#[derive(Debug, Clone)]
pub struct StandardScaler {
    means: Vec<f64>,
    stds: Vec<f64>,
}

impl StandardScaler {
    /// Learn per-column statistics from a row-major `n_rows × n_cols` matrix.
    /// `None` on a shape mismatch or empty input.
    pub fn fit(x: &[f64], n_rows: usize, n_cols: usize) -> Option<Self> {
        if n_rows == 0 || n_cols == 0 || x.len() != n_rows * n_cols {
            return None;
        }
        let mut means = vec![0.0; n_cols];
        let mut stds = vec![0.0; n_cols];
        let mut col = vec![0.0; n_rows];
        for j in 0..n_cols {
            for i in 0..n_rows {
                col[i] = x[i * n_cols + j];
            }
            means[j] = mean(&col)?;
            let s = std_dev(&col, true).unwrap_or(0.0);
            stds[j] = if s > 0.0 { s } else { 1.0 };
        }
        Some(Self { means, stds })
    }

    pub fn means(&self) -> &[f64] {
        &self.means
    }
    pub fn stds(&self) -> &[f64] {
        &self.stds
    }

    /// Apply standardization in place to a row-major `n_rows × n_cols` matrix using
    /// the learned statistics. `None` on a shape mismatch.
    pub fn transform_inplace(&self, x: &mut [f64], n_rows: usize, n_cols: usize) -> Option<()> {
        if n_cols != self.means.len() || x.len() != n_rows * n_cols {
            return None;
        }
        for i in 0..n_rows {
            for j in 0..n_cols {
                x[i * n_cols + j] = (x[i * n_cols + j] - self.means[j]) / self.stds[j];
            }
        }
        Some(())
    }

    /// Fit then transform a copy of `x`, returning the standardized matrix.
    pub fn fit_transform(x: &[f64], n_rows: usize, n_cols: usize) -> Option<(Self, Vec<f64>)> {
        let scaler = Self::fit(x, n_rows, n_cols)?;
        let mut out = x.to_vec();
        scaler.transform_inplace(&mut out, n_rows, n_cols)?;
        Some((scaler, out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::statistics::descriptive::{mean, std_dev};

    #[test]
    fn standardizes_to_zero_mean_unit_var() {
        // 4×2 matrix.
        let x = [1.0, 10.0, 2.0, 20.0, 3.0, 30.0, 4.0, 40.0];
        let (_, z) = StandardScaler::fit_transform(&x, 4, 2).unwrap();
        for j in 0..2 {
            let col: Vec<f64> = (0..4).map(|i| z[i * 2 + j]).collect();
            assert!(mean(&col).unwrap().abs() < 1e-12, "col {j} mean");
            assert!((std_dev(&col, true).unwrap() - 1.0).abs() < 1e-9, "col {j} std");
        }
    }

    #[test]
    fn constant_column_is_centered_not_nan() {
        let x = [5.0, 1.0, 5.0, 2.0, 5.0, 3.0]; // col0 constant
        let (_, z) = StandardScaler::fit_transform(&x, 3, 2).unwrap();
        for i in 0..3 {
            assert_eq!(z[i * 2], 0.0); // constant column → all zeros, no NaN
        }
    }

    #[test]
    fn guards_shape() {
        assert!(StandardScaler::fit(&[1.0, 2.0], 2, 2).is_none());
    }
}
