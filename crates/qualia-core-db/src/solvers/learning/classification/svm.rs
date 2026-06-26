//! Support Vector Machine (ISL ch 9, PRML ch 7) — soft-margin binary classifier
//! trained by simplified Sequential Minimal Optimization (SMO) on the dual, with a
//! linear or RBF (Gaussian) kernel. Kernel SVM separates classes a linear boundary
//! cannot. Labels are boolean (true = +1, false = −1). Kernel-class `DenseLinear`
//! (the kernel matrix) + `Divergent` (the SMO working-set loop) → CPU here.

use crate::solvers::learning::LearningError;

/// The kernel `K(a, b)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kernel {
    /// `⟨a, b⟩`.
    Linear,
    /// `exp(−γ·‖a − b‖²)`.
    Rbf { gamma: f64 },
}

impl Kernel {
    fn eval(self, a: &[f64], b: &[f64]) -> f64 {
        match self {
            Kernel::Linear => a.iter().zip(b).map(|(x, y)| x * y).sum(),
            Kernel::Rbf { gamma } => {
                let d2: f64 = a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum();
                (-gamma * d2).exp()
            }
        }
    }
}

/// A fitted SVM: the support vectors (the training points with non-zero `α`) plus
/// the bias and kernel.
#[derive(Debug, Clone)]
pub struct Svm {
    sv_x: Vec<f64>,     // n_sv × p
    sv_alpha_y: Vec<f64>, // αᵢ·yᵢ per support vector
    b: f64,
    kernel: Kernel,
    p: usize,
}

struct Lcg(u64);
impl Lcg {
    fn below(&mut self, bound: usize) -> usize {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % bound.max(1)
    }
}

/// Fit a soft-margin SVM by simplified SMO. `c` is the regularization (box) bound,
/// `max_passes` the number of consecutive no-change sweeps to declare convergence.
/// Fails closed on shape mismatch / a single-class target.
pub fn fit(
    x: &[f64],
    y: &[bool],
    n: usize,
    p: usize,
    c: f64,
    kernel: Kernel,
    max_passes: usize,
    tol: f64,
) -> Result<Svm, LearningError> {
    if n == 0 || p == 0 || x.len() != n * p || y.len() != n {
        return Err(LearningError::InvalidDimension);
    }
    if !(c > 0.0) {
        return Err(LearningError::InsufficientData);
    }
    let yi: Vec<f64> = y.iter().map(|&b| if b { 1.0 } else { -1.0 }).collect();
    let n_pos = y.iter().filter(|&&b| b).count();
    if n_pos == 0 || n_pos == n {
        return Err(LearningError::InsufficientData); // need both classes
    }

    // Precompute the kernel matrix.
    let mut k = vec![0.0; n * n];
    for i in 0..n {
        for j in i..n {
            let v = kernel.eval(&x[i * p..(i + 1) * p], &x[j * p..(j + 1) * p]);
            k[i * n + j] = v;
            k[j * n + i] = v;
        }
    }

    let mut alpha = vec![0.0; n];
    let mut b = 0.0;
    let mut rng = Lcg(0x9E3779B97F4A7C15);

    // f(xᵢ) = Σ_m α_m y_m K(m,i) + b.
    let f = |alpha: &[f64], b: f64, i: usize, k: &[f64]| -> f64 {
        let mut s = b;
        for m in 0..n {
            if alpha[m] != 0.0 {
                s += alpha[m] * yi[m] * k[m * n + i];
            }
        }
        s
    };

    let mut passes = 0;
    let max_iter = max_passes.max(1);
    let hard_cap = 10_000; // total outer sweeps guard
    let mut sweeps = 0;
    while passes < max_iter && sweeps < hard_cap {
        sweeps += 1;
        let mut num_changed = 0;
        for i in 0..n {
            let ei = f(&alpha, b, i, &k) - yi[i];
            if (yi[i] * ei < -tol && alpha[i] < c) || (yi[i] * ei > tol && alpha[i] > 0.0) {
                // Pick j ≠ i.
                let mut j = rng.below(n);
                if j == i {
                    j = (j + 1) % n;
                }
                let ej = f(&alpha, b, j, &k) - yi[j];
                let (ai_old, aj_old) = (alpha[i], alpha[j]);
                // Bounds on α_j.
                let (lo, hi) = if yi[i] != yi[j] {
                    ((aj_old - ai_old).max(0.0), c + (aj_old - ai_old).min(0.0))
                } else {
                    ((ai_old + aj_old - c).max(0.0), (ai_old + aj_old).min(c))
                };
                if (hi - lo).abs() < 1e-12 {
                    continue;
                }
                let eta = 2.0 * k[i * n + j] - k[i * n + i] - k[j * n + j];
                if eta >= 0.0 {
                    continue;
                }
                let mut aj = aj_old - yi[j] * (ei - ej) / eta;
                aj = aj.clamp(lo, hi);
                if (aj - aj_old).abs() < 1e-9 {
                    continue;
                }
                let ai = ai_old + yi[i] * yi[j] * (aj_old - aj);
                // Bias update.
                let b1 = b - ei - yi[i] * (ai - ai_old) * k[i * n + i] - yi[j] * (aj - aj_old) * k[i * n + j];
                let b2 = b - ej - yi[i] * (ai - ai_old) * k[i * n + j] - yi[j] * (aj - aj_old) * k[j * n + j];
                alpha[i] = ai;
                alpha[j] = aj;
                b = if ai > 0.0 && ai < c {
                    b1
                } else if aj > 0.0 && aj < c {
                    b2
                } else {
                    0.5 * (b1 + b2)
                };
                num_changed += 1;
            }
        }
        if num_changed == 0 {
            passes += 1;
        } else {
            passes = 0;
        }
    }

    // Keep only the support vectors (α > 0).
    let mut sv_x = Vec::new();
    let mut sv_alpha_y = Vec::new();
    for i in 0..n {
        if alpha[i] > 1e-8 {
            sv_x.extend_from_slice(&x[i * p..(i + 1) * p]);
            sv_alpha_y.push(alpha[i] * yi[i]);
        }
    }
    Ok(Svm { sv_x, sv_alpha_y, b, kernel, p })
}

impl Svm {
    /// The signed decision value `Σ αᵢyᵢ K(svᵢ, q) + b`.
    pub fn decision_row(&self, q: &[f64]) -> f64 {
        let mut s = self.b;
        for (k, &ay) in self.sv_alpha_y.iter().enumerate() {
            s += ay * self.kernel.eval(&self.sv_x[k * self.p..(k + 1) * self.p], q);
        }
        s
    }

    /// Predicted class (`true` = +1) = `decision ≥ 0`.
    pub fn predict_row(&self, q: &[f64]) -> bool {
        self.decision_row(q) >= 0.0
    }

    pub fn n_support_vectors(&self) -> usize {
        self.sv_alpha_y.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_svm_separates_linearly_separable() {
        // Two clearly separated 2-D classes.
        let x = [
            0.0, 0.0, 1.0, 0.5, 0.5, 1.0, -0.5, 0.2, 6.0, 6.0, 5.5, 6.5, 6.5, 5.5, 5.0, 6.0,
        ];
        let y = [false, false, false, false, true, true, true, true];
        let svm = fit(&x, &y, 8, 2, 1.0, Kernel::Linear, 20, 1e-3).unwrap();
        assert!(!svm.predict_row(&[0.2, 0.2]));
        assert!(svm.predict_row(&[6.0, 6.0]));
        assert!(svm.n_support_vectors() >= 1);
        // Perfect training accuracy on a separable set.
        for i in 0..8 {
            assert_eq!(svm.predict_row(&x[i * 2..i * 2 + 2]), y[i]);
        }
    }

    #[test]
    fn rbf_svm_handles_nonlinear_boundary() {
        // Concentric-ish: inner points class 0, outer ring class 1 — not linearly
        // separable, but an RBF kernel handles it.
        let mut x = Vec::new();
        let mut y = Vec::new();
        // inner cluster (class false) near origin
        for &(a, b) in &[(0.0, 0.0), (0.3, 0.0), (0.0, 0.3), (-0.3, 0.0), (0.0, -0.3)] {
            x.push(a);
            x.push(b);
            y.push(false);
        }
        // outer ring (class true)
        for &(a, b) in &[(3.0, 0.0), (-3.0, 0.0), (0.0, 3.0), (0.0, -3.0), (2.1, 2.1), (-2.1, -2.1)] {
            x.push(a);
            x.push(b);
            y.push(true);
        }
        let n = 11;
        let svm = fit(&x, &y, n, 2, 1.0, Kernel::Rbf { gamma: 0.5 }, 50, 1e-3).unwrap();
        assert!(!svm.predict_row(&[0.1, 0.1]), "inner point should be class 0");
        assert!(svm.predict_row(&[3.0, 0.0]), "outer point should be class 1");
        assert!(svm.predict_row(&[0.0, -3.0]));
    }

    #[test]
    fn guards() {
        assert_eq!(fit(&[1.0, 2.0], &[true, true][..1], 1, 2, 1.0, Kernel::Linear, 5, 1e-3).unwrap_err(), LearningError::InsufficientData);
        let x = [0.0, 0.0, 1.0, 1.0];
        assert_eq!(fit(&x, &[true, true], 2, 2, 1.0, Kernel::Linear, 5, 1e-3).unwrap_err(), LearningError::InsufficientData);
    }
}
