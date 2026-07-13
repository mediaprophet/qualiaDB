//! Kalman filter (PRML ch 13.3) — exact inference for a linear-Gaussian state-space
//! model. Recursively estimates the hidden state `x` and its covariance `P` from
//! noisy linear observations. The matrix products reuse `linear_algebra::gemm` and
//! the innovation-covariance inverse reuses `linear_algebra::cholesky` (no new
//! solver). Kernel-class `DenseLinear`.
//!
//! Model: `xₜ = F xₜ₋₁ + w` (`w ~ N(0, Q)`), `zₜ = H xₜ + v` (`v ~ N(0, R)`).
//! Predict: `x ← Fx`, `P ← FPFᵀ + Q`.
//! Update with `z`: `S = HPHᵀ + R`, `K = PHᵀS⁻¹`, `x ← x + K(z − Hx)`,
//! `P ← (I − KH)P`.

use crate::solvers::learning::LearningError;
use crate::solvers::linear_algebra::cholesky::{cholesky_factor, cholesky_solve};
use crate::solvers::linear_algebra::gemm::{gemm, matvec, Transpose};

/// A linear-Gaussian Kalman filter with current state estimate.
#[derive(Debug, Clone)]
pub struct KalmanFilter {
    f: Vec<f64>, // n_x × n_x transition
    h: Vec<f64>, // n_z × n_x observation
    q: Vec<f64>, // n_x × n_x process noise
    r: Vec<f64>, // n_z × n_z measurement noise
    x: Vec<f64>, // n_x state estimate
    p: Vec<f64>, // n_x × n_x state covariance
    nx: usize,
    nz: usize,
}

fn no(
    m: usize,
    n: usize,
    k: usize,
    a: &[f64],
    b: &[f64],
    out: &mut [f64],
) -> Result<(), LearningError> {
    gemm(Transpose::No, Transpose::No, m, n, k, 1.0, a, b, 0.0, out).map_err(Into::into)
}
fn no_t(
    m: usize,
    n: usize,
    k: usize,
    a: &[f64],
    b: &[f64],
    out: &mut [f64],
) -> Result<(), LearningError> {
    // out(m×n) = a(m×k) · b(n×k)ᵀ
    gemm(Transpose::No, Transpose::Yes, m, n, k, 1.0, a, b, 0.0, out).map_err(Into::into)
}

impl KalmanFilter {
    /// Construct with model matrices and an initial state estimate `(x0, p0)`.
    pub fn new(
        f: Vec<f64>,
        h: Vec<f64>,
        q: Vec<f64>,
        r: Vec<f64>,
        x0: Vec<f64>,
        p0: Vec<f64>,
        nx: usize,
        nz: usize,
    ) -> Result<Self, LearningError> {
        if nx == 0
            || nz == 0
            || f.len() != nx * nx
            || h.len() != nz * nx
            || q.len() != nx * nx
            || r.len() != nz * nz
            || x0.len() != nx
            || p0.len() != nx * nx
        {
            return Err(LearningError::InvalidDimension);
        }
        Ok(Self {
            f,
            h,
            q,
            r,
            x: x0,
            p: p0,
            nx,
            nz,
        })
    }

    pub fn state(&self) -> &[f64] {
        &self.x
    }
    pub fn covariance(&self) -> &[f64] {
        &self.p
    }

    /// Time update (predict): advance the state and inflate the covariance.
    pub fn predict(&mut self) -> Result<(), LearningError> {
        let nx = self.nx;
        // x ← F x.
        let mut xn = vec![0.0; nx];
        matvec(Transpose::No, nx, nx, &self.f, &self.x, &mut xn)?;
        self.x = xn;
        // P ← F P Fᵀ + Q.
        let mut fp = vec![0.0; nx * nx];
        no(nx, nx, nx, &self.f, &self.p, &mut fp)?;
        let mut pn = vec![0.0; nx * nx];
        no_t(nx, nx, nx, &fp, &self.f, &mut pn)?;
        for i in 0..nx * nx {
            pn[i] += self.q[i];
        }
        self.p = pn;
        Ok(())
    }

    /// Measurement update (correct) with observation `z`.
    pub fn update(&mut self, z: &[f64]) -> Result<(), LearningError> {
        let (nx, nz) = (self.nx, self.nz);
        if z.len() != nz {
            return Err(LearningError::InvalidDimension);
        }
        // Innovation y = z − H x.
        let mut hx = vec![0.0; nz];
        matvec(Transpose::No, nz, nx, &self.h, &self.x, &mut hx)?;
        let y: Vec<f64> = z.iter().zip(&hx).map(|(zi, hxi)| zi - hxi).collect();
        // HP (nz×nx) and S = HP Hᵀ + R (nz×nz).
        let mut hp = vec![0.0; nz * nx];
        no(nz, nx, nx, &self.h, &self.p, &mut hp)?;
        let mut s = vec![0.0; nz * nz];
        no_t(nz, nz, nx, &hp, &self.h, &mut s)?;
        for i in 0..nz * nz {
            s[i] += self.r[i];
        }
        // S⁻¹ via Cholesky.
        let mut l = vec![0.0; nz * nz];
        cholesky_factor(nz, &s, &mut l).map_err(|_| LearningError::Singular)?;
        let mut s_inv = vec![0.0; nz * nz];
        let mut ej = vec![0.0; nz];
        let mut cj = vec![0.0; nz];
        for j in 0..nz {
            ej.iter_mut().for_each(|v| *v = 0.0);
            ej[j] = 1.0;
            cholesky_solve(nz, &l, &ej, &mut cj)?;
            for i in 0..nz {
                s_inv[i * nz + j] = cj[i];
            }
        }
        // P Hᵀ (nx×nz), then K = P Hᵀ S⁻¹ (nx×nz).
        let mut pht = vec![0.0; nx * nz];
        no_t(nx, nz, nx, &self.p, &self.h, &mut pht)?;
        let mut kgain = vec![0.0; nx * nz];
        no(nx, nz, nz, &pht, &s_inv, &mut kgain)?;
        // x ← x + K y.
        let mut ky = vec![0.0; nx];
        matvec(Transpose::No, nx, nz, &kgain, &y, &mut ky)?;
        for i in 0..nx {
            self.x[i] += ky[i];
        }
        // P ← P − K (H P) = P − K·HP.
        let mut khp = vec![0.0; nx * nx];
        no(nx, nx, nz, &kgain, &hp, &mut khp)?;
        for i in 0..nx * nx {
            self.p[i] -= khp[i];
        }
        Ok(())
    }

    /// Filter a sequence of observations (row-major `t × nz`), returning the state
    /// estimate after each (predict→update) step, row-major `t × nx`.
    pub fn filter(&mut self, observations: &[f64], t: usize) -> Result<Vec<f64>, LearningError> {
        if observations.len() != t * self.nz {
            return Err(LearningError::InvalidDimension);
        }
        let mut out = vec![0.0; t * self.nx];
        for step in 0..t {
            self.predict()?;
            self.update(&observations[step * self.nz..(step + 1) * self.nz])?;
            out[step * self.nx..(step + 1) * self.nx].copy_from_slice(&self.x);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_a_constant_with_noisy_measurements() {
        // 1-D random-walk model tracking a constant true value 5, noisy obs.
        let mut kf = KalmanFilter::new(
            vec![1.0],  // F
            vec![1.0],  // H
            vec![1e-4], // Q (nearly constant state)
            vec![1.0],  // R (noisy measurement)
            vec![0.0],  // x0
            vec![10.0], // P0 (weak prior → the data dominates)
            1,
            1,
        )
        .unwrap();
        // Measurements jitter around 5.
        let obs = [5.3, 4.7, 5.1, 4.9, 5.2, 4.8, 5.0, 5.1, 4.9, 5.0];
        let est = kf.filter(&obs, 10).unwrap();
        // The final estimate is close to 5 and the covariance has shrunk.
        assert!((est[9] - 5.0).abs() < 0.3, "estimate {}", est[9]);
        assert!(kf.covariance()[0] < 1.0, "covariance should shrink");
    }

    #[test]
    fn smooths_better_than_raw_measurements() {
        // The filtered estimate has lower variance than the raw noisy obs.
        let mut kf = KalmanFilter::new(
            vec![1.0],
            vec![1.0],
            vec![1e-3],
            vec![1.0],
            vec![10.0],
            vec![1.0],
            1,
            1,
        )
        .unwrap();
        let obs = [10.5, 9.4, 10.6, 9.5, 10.4, 9.6, 10.5, 9.5];
        let est = kf.filter(&obs, 8).unwrap();
        let var = |v: &[f64]| {
            let m = v.iter().sum::<f64>() / v.len() as f64;
            v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / v.len() as f64
        };
        let obs_var = var(&obs);
        let est_var = var(&est[2..]); // skip the warm-up
        assert!(
            est_var < obs_var,
            "filter should smooth: {est_var} !< {obs_var}"
        );
    }

    #[test]
    fn guards() {
        assert_eq!(
            KalmanFilter::new(
                vec![1.0],
                vec![1.0],
                vec![1.0],
                vec![1.0],
                vec![0.0],
                vec![1.0, 0.0],
                1,
                1
            )
            .unwrap_err(),
            LearningError::InvalidDimension
        );
    }
}
