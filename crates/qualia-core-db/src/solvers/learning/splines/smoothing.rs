//! Penalized smoothing spline (ISL ch 7.5) — least squares with a roughness
//! penalty that shrinks the wiggly (knot) part of the fit.
//!
//! Minimise `‖y − Bβ‖² + λ·βᵀPβ`, where `B` is the truncated-power spline basis
//! (shared with [`super::RegressionSpline`]) and `P` penalizes only the
//! truncated-power (knot) coefficients — the part that controls smoothness. `λ = 0`
//! reproduces the (interpolating) regression spline; large `λ` shrinks the knot
//! terms toward a global polynomial (a smooth fit). The penalized normal equations
//! `(BᵀB + λP)β = Bᵀy` are solved with `linear_algebra::cholesky` (no new solver).
//! Kernel-class `DenseLinear`.

use crate::solvers::learning::splines::{basis_len, basis_row};
use crate::solvers::learning::LearningError;
use crate::solvers::linear_algebra::cholesky::{cholesky_factor, cholesky_solve};
use crate::solvers::linear_algebra::gemm::{gemm, matvec, Transpose};

/// A fitted smoothing spline.
#[derive(Debug, Clone)]
pub struct SmoothingSpline {
    pub degree: usize,
    pub knots: Vec<f64>,
    pub coefficients: Vec<f64>,
    pub lambda: f64,
}

impl SmoothingSpline {
    /// Fit with smoothing parameter `lambda ≥ 0`. Fails closed on shape mismatch /
    /// `degree == 0`.
    pub fn fit(
        x: &[f64],
        y: &[f64],
        n: usize,
        degree: usize,
        knots: &[f64],
        lambda: f64,
    ) -> Result<Self, LearningError> {
        if n == 0 || x.len() != n || y.len() != n || degree == 0 || lambda < 0.0 {
            return Err(LearningError::InvalidDimension);
        }
        let m = basis_len(degree, knots.len());
        // Design matrix B (n × m).
        let mut b = vec![0.0; n * m];
        for i in 0..n {
            basis_row(x[i], degree, knots, &mut b[i * m..(i + 1) * m]);
        }
        // A = BᵀB + λP, where P = diag(0,…,0,1,…,1) penalizes only the knot terms.
        let mut a = vec![0.0; m * m];
        gemm(
            Transpose::Yes,
            Transpose::No,
            m,
            m,
            n,
            1.0,
            &b,
            &b,
            0.0,
            &mut a,
        )?;
        for j in (degree + 1)..m {
            a[j * m + j] += lambda;
        }
        // rhs = Bᵀy.
        let mut rhs = vec![0.0; m];
        matvec(Transpose::Yes, m, n, &b, y, &mut rhs)?;
        // Solve.
        let mut l = vec![0.0; m * m];
        cholesky_factor(m, &a, &mut l).map_err(|_| LearningError::Singular)?;
        let mut coefficients = vec![0.0; m];
        cholesky_solve(m, &l, &rhs, &mut coefficients)?;
        Ok(Self {
            degree,
            knots: knots.to_vec(),
            coefficients,
            lambda,
        })
    }

    pub fn predict_one(&self, x: f64) -> f64 {
        let m = basis_len(self.degree, self.knots.len());
        let mut row = vec![0.0; m];
        basis_row(x, self.degree, &self.knots, &mut row);
        row.iter().zip(&self.coefficients).map(|(b, c)| b * c).sum()
    }

    pub fn predict(&self, x: &[f64]) -> Vec<f64> {
        x.iter().map(|&xi| self.predict_one(xi)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::learning::splines::RegressionSpline;

    /// Roughness proxy: sum of squared second differences of the fitted values.
    fn roughness(vals: &[f64]) -> f64 {
        let mut r = 0.0;
        for i in 1..vals.len() - 1 {
            let d2 = vals[i + 1] - 2.0 * vals[i] + vals[i - 1];
            r += d2 * d2;
        }
        r
    }

    #[test]
    fn lambda_zero_matches_regression_spline() {
        let x: Vec<f64> = (0..20).map(|i| i as f64 * 0.3).collect();
        let y: Vec<f64> = x.iter().map(|&xi| xi.sin()).collect();
        let knots = vec![1.5, 3.0, 4.5];
        let ss = SmoothingSpline::fit(&x, &y, 20, 3, &knots, 0.0).unwrap();
        let rs = RegressionSpline::fit(&x, &y, 20, 3, &knots).unwrap();
        for i in 0..20 {
            assert!((ss.predict_one(x[i]) - rs.predict_one(x[i])).abs() < 1e-6);
        }
    }

    #[test]
    fn larger_lambda_is_smoother() {
        // Noisy data; a larger penalty yields a smoother (less wiggly) fit.
        let n = 40;
        let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.2).collect();
        let y: Vec<f64> = x
            .iter()
            .enumerate()
            .map(|(i, &xi)| xi.sin() + ((i % 2) as f64 - 0.5) * 0.6)
            .collect();
        let knots: Vec<f64> = (1..8).map(|k| k as f64).collect();
        let light = SmoothingSpline::fit(&x, &y, n, 3, &knots, 0.01).unwrap();
        let heavy = SmoothingSpline::fit(&x, &y, n, 3, &knots, 100.0).unwrap();
        let r_light = roughness(&light.predict(&x));
        let r_heavy = roughness(&heavy.predict(&x));
        assert!(
            r_heavy < r_light,
            "heavier penalty must be smoother: {r_heavy} !< {r_light}"
        );
    }

    #[test]
    fn guards() {
        assert_eq!(
            SmoothingSpline::fit(&[1.0, 2.0], &[1.0], 2, 3, &[], 1.0).unwrap_err(),
            LearningError::InvalidDimension
        );
    }
}
