//! Polynomial & complex algebra — the engine's home for complex arithmetic, real
//! quadratic solving, and dependency-free polynomial root finding.
//!
//! This is *not* linear algebra (it was previously co-located with it in a specialized
//! lib); it is the computer-algebra primitive that matrix-spectral routines
//! (`solvers::linear_algebra::spectral`) build on. Allocating where outputs are
//! inherently dynamic (root vectors); all scratch is local.

use crate::solvers::SolversError;

/// A complex number `re + im·i`. Minimal arithmetic for polynomial root finding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }
    pub const fn real(re: f64) -> Self {
        Self { re, im: 0.0 }
    }

    #[inline]
    pub fn add(self, o: Complex) -> Complex {
        Complex::new(self.re + o.re, self.im + o.im)
    }
    #[inline]
    pub fn sub(self, o: Complex) -> Complex {
        Complex::new(self.re - o.re, self.im - o.im)
    }
    #[inline]
    pub fn mul(self, o: Complex) -> Complex {
        Complex::new(
            self.re * o.re - self.im * o.im,
            self.re * o.im + self.im * o.re,
        )
    }
    #[inline]
    pub fn div(self, o: Complex) -> Complex {
        let d = o.re * o.re + o.im * o.im;
        Complex::new(
            (self.re * o.re + self.im * o.im) / d,
            (self.im * o.re - self.re * o.im) / d,
        )
    }
    /// Modulus |z|.
    #[inline]
    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
    /// True if within `tol` of the real axis.
    #[inline]
    pub fn is_real(self, tol: f64) -> bool {
        self.im.abs() <= tol
    }
}

/// The roots of a real quadratic `a·x² + b·x + c = 0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuadraticRoots {
    /// Two distinct real roots, ascending.
    TwoReal(f64, f64),
    /// One repeated real root (discriminant ≈ 0).
    DoubleReal(f64),
    /// A complex conjugate pair `re ± im·i` (im > 0).
    ComplexPair { re: f64, im: f64 },
    /// Degenerate leading coefficient (a ≈ 0): the single linear root of `b·x + c = 0`.
    Linear(f64),
}

/// Solve `a·x² + b·x + c = 0` over the reals, numerically stably.
///
/// Uses the cancellation-avoiding form `q = -(b + sign(b)·√Δ)/2`, roots `q/a` and `c/q`,
/// for `Δ > 0`; classifies `Δ ≈ 0` as a double root and `Δ < 0` as a complex pair. Falls
/// back to the linear root when `a ≈ 0`. Returns [`SolversError::ComputationError`] for a
/// non-finite or fully-degenerate input.
pub fn solve_quadratic(a: f64, b: f64, c: f64) -> Result<QuadraticRoots, SolversError> {
    if !(a.is_finite() && b.is_finite() && c.is_finite()) {
        return Err(SolversError::ComputationError);
    }
    let scale = a.abs().max(b.abs()).max(c.abs()).max(1.0);

    // Degenerate leading coefficient → linear (or no/everywhere solution).
    if a.abs() <= f64::EPSILON * scale {
        if b.abs() <= f64::EPSILON * scale {
            return Err(SolversError::ComputationError);
        }
        return Ok(QuadraticRoots::Linear(-c / b));
    }

    let disc = b * b - 4.0 * a * c;
    let disc_scale = (b * b).max((4.0 * a * c).abs()).max(1.0);
    if disc.abs() <= 1e-12 * disc_scale {
        return Ok(QuadraticRoots::DoubleReal(-b / (2.0 * a)));
    }

    if disc > 0.0 {
        let sqrt_d = disc.sqrt();
        let sign_b = if b >= 0.0 { 1.0 } else { -1.0 };
        let q = -0.5 * (b + sign_b * sqrt_d);
        let r1 = q / a;
        let r2 = c / q;
        let (lo, hi) = if r1 <= r2 { (r1, r2) } else { (r2, r1) };
        Ok(QuadraticRoots::TwoReal(lo, hi))
    } else {
        let re = -b / (2.0 * a);
        let im = (-disc).sqrt() / (2.0 * a.abs());
        Ok(QuadraticRoots::ComplexPair { re, im })
    }
}

/// Evaluate a polynomial at a complex point via Horner's method.
/// `coeffs` are in DESCENDING order: `coeffs[0]·x^n + … + coeffs[n]`.
fn poly_eval_complex(coeffs: &[f64], x: Complex) -> Complex {
    let mut acc = Complex::real(0.0);
    for &c in coeffs {
        acc = acc.mul(x).add(Complex::real(c));
    }
    acc
}

/// Find all complex roots of a real polynomial (DESCENDING coefficients,
/// `coeffs[0]·x^n + … + coeffs[n]`) via the Durand–Kerner iteration.
///
/// Dependency-free and finds all `n` roots simultaneously; suitable for moderate degree.
/// Leading/trailing zeros are trimmed. Returns `n` roots (real roots have `im ≈ 0`).
/// Returns [`SolversError::ComputationError`] for a zero or non-finite polynomial.
pub fn polynomial_roots(coeffs: &[f64]) -> Result<Vec<Complex>, SolversError> {
    // Trim leading zeros (they do not change the polynomial's degree meaningfully).
    let start = coeffs
        .iter()
        .position(|c| c.abs() > 0.0)
        .ok_or(SolversError::ComputationError)?;
    let coeffs = &coeffs[start..];
    if coeffs.len() == 1 {
        return Ok(Vec::new()); // a nonzero constant: no roots
    }
    if coeffs.iter().any(|c| !c.is_finite()) {
        return Err(SolversError::ComputationError);
    }

    // Normalise to monic.
    let lead = coeffs[0];
    let monic: Vec<f64> = coeffs.iter().map(|c| c / lead).collect();
    let degree = monic.len() - 1;

    // Distinct complex initial guesses on a spiral (the classic 0.4 + 0.9i seed).
    let seed = Complex::new(0.4, 0.9);
    let mut roots: Vec<Complex> = (0..degree)
        .map(|k| {
            let mut z = Complex::real(1.0);
            for _ in 0..k {
                z = z.mul(seed);
            }
            z
        })
        .collect();

    const MAX_ITERS: usize = 500;
    const TOL: f64 = 1e-14;
    for _ in 0..MAX_ITERS {
        let mut max_delta = 0.0_f64;
        for i in 0..degree {
            let zi = roots[i];
            // denominator = Π_{j≠i} (zi - zj)
            let mut denom = Complex::real(1.0);
            for j in 0..degree {
                if j != i {
                    denom = denom.mul(zi.sub(roots[j]));
                }
            }
            if denom.abs() == 0.0 {
                continue; // coincident guesses; perturb on the next sweep
            }
            let delta = poly_eval_complex(&monic, zi).div(denom);
            roots[i] = zi.sub(delta);
            max_delta = max_delta.max(delta.abs());
        }
        if max_delta < TOL {
            break;
        }
    }

    // Snap near-real roots to the real axis for clean output.
    for r in roots.iter_mut() {
        if r.im.abs() < 1e-9 * (1.0 + r.re.abs()) {
            r.im = 0.0;
        }
    }
    Ok(roots)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quadratic_two_real() {
        // x² − 3x + 2 = 0 → 1, 2
        assert_eq!(
            solve_quadratic(1.0, -3.0, 2.0).unwrap(),
            QuadraticRoots::TwoReal(1.0, 2.0)
        );
    }

    #[test]
    fn quadratic_double_and_complex_and_linear() {
        // x² − 2x + 1 → double 1
        assert_eq!(
            solve_quadratic(1.0, -2.0, 1.0).unwrap(),
            QuadraticRoots::DoubleReal(1.0)
        );
        // x² + 1 → ±i
        match solve_quadratic(1.0, 0.0, 1.0).unwrap() {
            QuadraticRoots::ComplexPair { re, im } => {
                assert!(re.abs() < 1e-12 && (im - 1.0).abs() < 1e-12);
            }
            other => panic!("expected complex pair, got {other:?}"),
        }
        // 0·x² + 2x + 4 → linear root −2
        assert_eq!(
            solve_quadratic(0.0, 2.0, 4.0).unwrap(),
            QuadraticRoots::Linear(-2.0)
        );
    }

    #[test]
    fn quadratic_rejects_degenerate() {
        assert!(matches!(
            solve_quadratic(0.0, 0.0, 1.0),
            Err(SolversError::ComputationError)
        ));
        assert!(matches!(
            solve_quadratic(f64::NAN, 1.0, 1.0),
            Err(SolversError::ComputationError)
        ));
    }

    #[test]
    fn roots_of_known_polynomial() {
        // (x−1)(x−2)(x−3) = x³ − 6x² + 11x − 6
        let roots = polynomial_roots(&[1.0, -6.0, 11.0, -6.0]).unwrap();
        assert_eq!(roots.len(), 3);
        let mut reals: Vec<f64> = roots.iter().map(|r| r.re).collect();
        reals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for (got, want) in reals.iter().zip([1.0, 2.0, 3.0]) {
            assert!((got - want).abs() < 1e-6, "{got} != {want}");
        }
        assert!(roots.iter().all(|r| r.is_real(1e-6)));
    }

    #[test]
    fn roots_complex_pair() {
        // x² + 1 → ±i
        let roots = polynomial_roots(&[1.0, 0.0, 1.0]).unwrap();
        assert_eq!(roots.len(), 2);
        assert!(roots.iter().any(|r| (r.im.abs() - 1.0).abs() < 1e-6));
    }

    #[test]
    fn roots_reject_zero_polynomial() {
        assert!(matches!(
            polynomial_roots(&[0.0, 0.0]),
            Err(SolversError::ComputationError)
        ));
    }
}
