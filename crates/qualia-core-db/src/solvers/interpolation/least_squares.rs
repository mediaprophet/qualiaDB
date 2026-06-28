//! Polynomial least-squares fitting via the normal equations `(VᵀV) c = Vᵀy`, solved by
//! Gaussian elimination with partial pivoting (the small `(d+1)×(d+1)` system).

use super::InterpolationError;

/// Fit a degree-`degree` polynomial to `(xs, ys)` in the least-squares sense. Returns
/// coefficients in **ascending** order `[c₀, c₁, …, c_degree]` (so the polynomial is
/// `Σ cₖ xᵏ`). Fails closed if there are too few points or the system is singular.
pub fn poly_fit(xs: &[f64], ys: &[f64], degree: usize) -> Result<Vec<f64>, InterpolationError> {
    if xs.is_empty() || xs.len() != ys.len() {
        return Err(InterpolationError::InsufficientData);
    }
    if degree + 1 > xs.len() {
        return Err(InterpolationError::InvalidDegree);
    }
    let m = degree + 1;
    // Normal equations: A[j][k] = Σ x^(j+k), b[j] = Σ y·x^j.
    let mut a = vec![0.0; m * m];
    let mut b = vec![0.0; m];
    // Precompute power sums up to 2·degree.
    let mut powsum = vec![0.0; 2 * degree + 1];
    for &x in xs {
        let mut p = 1.0;
        for s in powsum.iter_mut() {
            *s += p;
            p *= x;
        }
    }
    for j in 0..m {
        for k in 0..m {
            a[j * m + k] = powsum[j + k];
        }
        let mut s = 0.0;
        for (&x, &y) in xs.iter().zip(ys) {
            s += y * x.powi(j as i32);
        }
        b[j] = s;
    }
    gauss_solve(m, &mut a, &mut b).ok_or(InterpolationError::Singular)
}

/// Evaluate a polynomial given ascending coefficients at `x` (Horner).
pub fn poly_eval(coeffs: &[f64], x: f64) -> f64 {
    coeffs.iter().rev().fold(0.0, |acc, &c| acc * x + c)
}

/// Solve `A x = b` (row-major `n×n`) by Gaussian elimination with partial pivoting.
/// Consumes `a`/`b`. `None` if singular.
fn gauss_solve(n: usize, a: &mut [f64], b: &mut [f64]) -> Option<Vec<f64>> {
    for col in 0..n {
        // Partial pivot.
        let mut piv = col;
        let mut best = a[col * n + col].abs();
        for r in (col + 1)..n {
            let v = a[r * n + col].abs();
            if v > best {
                best = v;
                piv = r;
            }
        }
        if best < 1e-14 {
            return None; // singular
        }
        if piv != col {
            for c in 0..n {
                a.swap(col * n + c, piv * n + c);
            }
            b.swap(col, piv);
        }
        // Eliminate below.
        for r in (col + 1)..n {
            let f = a[r * n + col] / a[col * n + col];
            for c in col..n {
                a[r * n + c] -= f * a[col * n + c];
            }
            b[r] -= f * b[col];
        }
    }
    // Back-substitution.
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut s = b[i];
        for j in (i + 1)..n {
            s -= a[i * n + j] * x[j];
        }
        x[i] = s / a[i * n + i];
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_a_line_from_collinear_points() {
        // y = 3x + 1
        let xs = [0.0, 1.0, 2.0, 3.0, 4.0];
        let ys = xs.map(|x| 3.0 * x + 1.0);
        let c = poly_fit(&xs, &ys, 1).unwrap();
        assert!((c[0] - 1.0).abs() < 1e-9);
        assert!((c[1] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn recovers_a_parabola_exactly() {
        // y = 2x² − x + 5, fit degree 2 → exact.
        let f = |x: f64| 2.0 * x * x - x + 5.0;
        let xs = [-2.0, -1.0, 0.0, 1.0, 2.0, 3.0];
        let ys = xs.map(f);
        let c = poly_fit(&xs, &ys, 2).unwrap();
        for &q in &[0.5, 1.3, -1.7] {
            assert!((poly_eval(&c, q) - f(q)).abs() < 1e-7);
        }
    }

    #[test]
    fn least_squares_minimises_on_noisy_data() {
        // Points near y = x; degree-1 fit slope ≈ 1, intercept ≈ 0.
        let xs = [0.0, 1.0, 2.0, 3.0, 4.0];
        let ys = [0.1, 0.9, 2.1, 2.9, 4.05];
        let c = poly_fit(&xs, &ys, 1).unwrap();
        assert!((c[1] - 1.0).abs() < 0.1);
        assert!(c[0].abs() < 0.2);
    }

    #[test]
    fn fails_closed() {
        assert_eq!(
            poly_fit(&[1.0, 2.0], &[1.0, 2.0], 5).unwrap_err(),
            InterpolationError::InvalidDegree
        );
        assert_eq!(
            poly_fit(&[], &[], 0).unwrap_err(),
            InterpolationError::InsufficientData
        );
    }
}
