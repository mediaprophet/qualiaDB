//! Polynomial interpolation through `n` points: the Lagrange form (direct evaluation)
//! and the Newton divided-difference form (build coefficients once, evaluate cheaply).

use super::InterpolationError;

fn validate(xs: &[f64], ys: &[f64]) -> Result<(), InterpolationError> {
    if xs.is_empty() || xs.len() != ys.len() {
        return Err(InterpolationError::InsufficientData);
    }
    for i in 0..xs.len() {
        for j in (i + 1)..xs.len() {
            if xs[i] == xs[j] {
                return Err(InterpolationError::DuplicateNodes);
            }
        }
    }
    Ok(())
}

/// Evaluate the Lagrange interpolating polynomial through `(xs, ys)` at `x`.
pub fn lagrange_eval(xs: &[f64], ys: &[f64], x: f64) -> Result<f64, InterpolationError> {
    validate(xs, ys)?;
    let n = xs.len();
    let mut sum = 0.0;
    for i in 0..n {
        let mut li = 1.0;
        for j in 0..n {
            if i != j {
                li *= (x - xs[j]) / (xs[i] - xs[j]);
            }
        }
        sum += ys[i] * li;
    }
    Ok(sum)
}

/// Newton divided-difference coefficients for `(xs, ys)` (the leading diagonal of the
/// divided-difference table). Use with [`newton_eval`].
pub fn newton_coefficients(xs: &[f64], ys: &[f64]) -> Result<Vec<f64>, InterpolationError> {
    validate(xs, ys)?;
    let n = xs.len();
    let mut coef = ys.to_vec();
    for j in 1..n {
        for i in (j..n).rev() {
            coef[i] = (coef[i] - coef[i - 1]) / (xs[i] - xs[i - j]);
        }
    }
    Ok(coef)
}

/// Evaluate the Newton form (Horner over the nested products) at `x`.
pub fn newton_eval(xs: &[f64], coef: &[f64], x: f64) -> f64 {
    let n = coef.len();
    let mut acc = coef[n - 1];
    for i in (0..n - 1).rev() {
        acc = acc * (x - xs[i]) + coef[i];
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f64 = 1e-9;

    #[test]
    fn interpolant_passes_through_nodes() {
        let xs = [0.0, 1.0, 2.0, 3.0];
        let ys = [1.0, 3.0, 2.0, 5.0];
        for i in 0..xs.len() {
            assert!((lagrange_eval(&xs, &ys, xs[i]).unwrap() - ys[i]).abs() < EPS);
        }
    }

    #[test]
    fn reproduces_a_quadratic_exactly() {
        // f(x) = 2x² − 3x + 1 sampled at 3 points → interpolant equals f everywhere.
        let f = |x: f64| 2.0 * x * x - 3.0 * x + 1.0;
        let xs = [-1.0, 0.0, 2.0];
        let ys = xs.map(f);
        for &q in &[0.5, 1.7, -0.3, 5.0] {
            assert!((lagrange_eval(&xs, &ys, q).unwrap() - f(q)).abs() < 1e-8);
        }
    }

    #[test]
    fn newton_matches_lagrange() {
        let xs = [0.0, 1.0, 2.0, 4.0];
        let ys = [1.0, 2.0, 0.0, 8.0];
        let coef = newton_coefficients(&xs, &ys).unwrap();
        for &q in &[0.3, 1.5, 3.0] {
            assert!(
                (newton_eval(&xs, &coef, q) - lagrange_eval(&xs, &ys, q).unwrap()).abs() < 1e-9
            );
        }
    }

    #[test]
    fn fails_closed() {
        assert_eq!(
            lagrange_eval(&[], &[], 0.0).unwrap_err(),
            InterpolationError::InsufficientData
        );
        assert_eq!(
            lagrange_eval(&[1.0, 1.0], &[2.0, 3.0], 0.0).unwrap_err(),
            InterpolationError::DuplicateNodes
        );
    }
}
