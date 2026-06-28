//! **Equation solving** (Gap analysis §3.2) — polynomial roots (any degree), real-root
//! extraction, linear systems, and roots of a CAS polynomial expression.
//!
//! Reuses the engine's real root finder ([`crate::solvers::polynomial::polynomial_roots`],
//! Durand–Kerner) and the polynomial least-squares fit
//! ([`crate::solvers::interpolation::poly_fit`]) — no re-implementation. (The earlier
//! sub-agent could not reach these because it was built on the wrong branch; here they
//! exist.)

use crate::solvers::interpolation::poly_fit;
use crate::solvers::polynomial::{polynomial_roots, Complex};
use crate::specialized_libs::symbolic_algebra::Expr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolveError {
    /// Degenerate / non-finite polynomial, or a singular linear system.
    NoSolution,
}

/// All complex roots of a real polynomial given in **descending** coefficients
/// (`coeffs[0]·xⁿ + … + coeffs[n]`).
pub fn roots(coeffs: &[f64]) -> Result<Vec<Complex>, SolveError> {
    polynomial_roots(coeffs).map_err(|_| SolveError::NoSolution)
}

/// The **real** roots (those with `|im| < tol`), ascending.
pub fn real_roots(coeffs: &[f64], tol: f64) -> Result<Vec<f64>, SolveError> {
    let mut rs: Vec<f64> = roots(coeffs)?
        .into_iter()
        .filter(|z| z.im.abs() < tol)
        .map(|z| z.re)
        .collect();
    rs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    Ok(rs)
}

/// Solve `A x = b` (row-major `n×n`) by Gaussian elimination with partial pivoting.
/// `None` if singular or shapes are inconsistent.
pub fn solve_linear_system(a: &[f64], b: &[f64], n: usize) -> Option<Vec<f64>> {
    if a.len() != n * n || b.len() != n || n == 0 {
        return None;
    }
    let mut m = a.to_vec();
    let mut rhs = b.to_vec();
    for col in 0..n {
        let mut piv = col;
        let mut best = m[col * n + col].abs();
        for r in (col + 1)..n {
            let v = m[r * n + col].abs();
            if v > best {
                best = v;
                piv = r;
            }
        }
        if best < 1e-14 {
            return None;
        }
        if piv != col {
            for c in 0..n {
                m.swap(col * n + c, piv * n + c);
            }
            rhs.swap(col, piv);
        }
        for r in (col + 1)..n {
            let f = m[r * n + col] / m[col * n + col];
            for c in col..n {
                m[r * n + c] -= f * m[col * n + c];
            }
            rhs[r] -= f * rhs[col];
        }
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut s = rhs[i];
        for j in (i + 1)..n {
            s -= m[i * n + j] * x[j];
        }
        x[i] = s / m[i * n + i];
    }
    Some(x)
}

/// Real roots of a polynomial **expression** of one variable, by sampling it at
/// `degree+1` points, recovering its power-basis coefficients via [`poly_fit`], and
/// root-finding. (Exact for a genuine polynomial of the given degree.)
pub fn solve_polynomial_expr(
    expr: &Expr,
    var: &str,
    degree: usize,
    tol: f64,
) -> Result<Vec<f64>, SolveError> {
    let xs: Vec<f64> = (0..=degree + 1)
        .map(|i| i as f64 - (degree as f64) / 2.0)
        .collect();
    let ys: Vec<f64> = xs
        .iter()
        .map(|&x| {
            let mut env = std::collections::HashMap::new();
            env.insert(var.to_string(), x);
            expr.eval(&env).ok_or(SolveError::NoSolution)
        })
        .collect::<Result<_, _>>()?;
    // poly_fit → ascending coeffs; polynomial_roots wants descending.
    let mut asc = poly_fit(&xs, &ys, degree).map_err(|_| SolveError::NoSolution)?;
    asc.reverse();
    real_roots(&asc, tol)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::symbolic_algebra::{add, c, mul, pow, sub, var};

    fn close(a: &[f64], b: &[f64]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-6)
    }

    #[test]
    fn quadratic_and_cubic_real_roots() {
        // x² − 5x + 6 → {2, 3}
        assert!(close(
            &real_roots(&[1.0, -5.0, 6.0], 1e-6).unwrap(),
            &[2.0, 3.0]
        ));
        // x³ − 6x² + 11x − 6 → {1, 2, 3}
        assert!(close(
            &real_roots(&[1.0, -6.0, 11.0, -6.0], 1e-6).unwrap(),
            &[1.0, 2.0, 3.0]
        ));
    }

    #[test]
    fn complex_roots_filtered_out() {
        // x² + 1 → no real roots.
        assert!(real_roots(&[1.0, 0.0, 1.0], 1e-6).unwrap().is_empty());
    }

    #[test]
    fn linear_system_solves() {
        // [[2,1],[1,3]] x = [3,5] → x = [4/5, 7/5]
        let x = solve_linear_system(&[2.0, 1.0, 1.0, 3.0], &[3.0, 5.0], 2).unwrap();
        assert!((x[0] - 0.8).abs() < 1e-9 && (x[1] - 1.4).abs() < 1e-9);
        // Singular → None.
        assert!(solve_linear_system(&[1.0, 2.0, 2.0, 4.0], &[1.0, 2.0], 2).is_none());
    }

    #[test]
    fn roots_from_a_cas_expression() {
        // f = x² − 5x + 6 as an Expr → {2, 3}
        let f = add(sub(pow(var("x"), 2), mul(c(5.0), var("x"))), c(6.0));
        let r = solve_polynomial_expr(&f, "x", 2, 1e-6).unwrap();
        assert!(close(&r, &[2.0, 3.0]));
    }
}
