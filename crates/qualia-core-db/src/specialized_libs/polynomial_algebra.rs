//! Dense univariate polynomial algebra over `f64` coefficients.
//!
//! A [`Polynomial`] stores its dense coefficient vector little-endian: index `i`
//! holds the coefficient of `x^i`. The vector is kept *trimmed* so the leading
//! (highest-index) coefficient is non-zero, except for the zero polynomial which
//! is represented by an empty vector (degree reported as `None`).
//!
//! Provided: add, sub, mul, long division (quotient + remainder), Euclidean
//! `gcd`, `derivative`, Horner `eval`, and the `resultant` of two polynomials
//! via the Euclidean (subresultant-style) remainder sequence.
//!
//! Fallible operations (dividing by the zero polynomial) **fail closed** with
//! `Option` and never fabricate a result.

/// Tolerance below which a coefficient is treated as exactly zero when trimming
/// leading terms. Floating-point polynomial GCD/division is inherently
/// approximate; this keeps the leading-coefficient bookkeeping robust.
const EPS: f64 = 1e-9;

/// Dense univariate polynomial with `f64` coefficients, little-endian.
#[derive(Clone, Debug, PartialEq)]
pub struct Polynomial {
    /// `coeffs[i]` is the coefficient of `x^i`. Trimmed: last element non-zero
    /// (unless the vector is empty, i.e. the zero polynomial).
    coeffs: Vec<f64>,
}

impl Polynomial {
    /// Build from coefficients (index `i` = coefficient of `x^i`), trimming any
    /// near-zero high-order terms.
    pub fn new(coeffs: Vec<f64>) -> Self {
        let mut p = Polynomial { coeffs };
        p.trim();
        p
    }

    /// The zero polynomial.
    pub fn zero() -> Self {
        Polynomial { coeffs: Vec::new() }
    }

    /// The constant polynomial `c`.
    pub fn constant(c: f64) -> Self {
        Polynomial::new(vec![c])
    }

    /// Borrow the (trimmed) coefficient slice, little-endian.
    pub fn coeffs(&self) -> &[f64] {
        &self.coeffs
    }

    /// True if this is the zero polynomial.
    pub fn is_zero(&self) -> bool {
        self.coeffs.is_empty()
    }

    /// Degree of the polynomial, or `None` for the zero polynomial.
    pub fn degree(&self) -> Option<usize> {
        if self.coeffs.is_empty() {
            None
        } else {
            Some(self.coeffs.len() - 1)
        }
    }

    /// Leading coefficient (highest-order). `0.0` for the zero polynomial.
    pub fn leading(&self) -> f64 {
        *self.coeffs.last().unwrap_or(&0.0)
    }

    /// Drop near-zero high-order coefficients so the leading term is non-zero.
    fn trim(&mut self) {
        while let Some(&c) = self.coeffs.last() {
            if c.abs() <= EPS {
                self.coeffs.pop();
            } else {
                break;
            }
        }
    }

    /// Evaluate at `x` using Horner's method.
    pub fn eval(&self, x: f64) -> f64 {
        let mut acc = 0.0;
        for &c in self.coeffs.iter().rev() {
            acc = acc * x + c;
        }
        acc
    }

    /// Sum `self + other`.
    pub fn add(&self, other: &Polynomial) -> Polynomial {
        let n = self.coeffs.len().max(other.coeffs.len());
        let mut out = vec![0.0; n];
        for (i, c) in self.coeffs.iter().enumerate() {
            out[i] += c;
        }
        for (i, c) in other.coeffs.iter().enumerate() {
            out[i] += c;
        }
        Polynomial::new(out)
    }

    /// Difference `self - other`.
    pub fn sub(&self, other: &Polynomial) -> Polynomial {
        let n = self.coeffs.len().max(other.coeffs.len());
        let mut out = vec![0.0; n];
        for (i, c) in self.coeffs.iter().enumerate() {
            out[i] += c;
        }
        for (i, c) in other.coeffs.iter().enumerate() {
            out[i] -= c;
        }
        Polynomial::new(out)
    }

    /// Scale every coefficient by `s`.
    pub fn scale(&self, s: f64) -> Polynomial {
        Polynomial::new(self.coeffs.iter().map(|c| c * s).collect())
    }

    /// Product `self * other` (schoolbook convolution).
    pub fn mul(&self, other: &Polynomial) -> Polynomial {
        if self.is_zero() || other.is_zero() {
            return Polynomial::zero();
        }
        let mut out = vec![0.0; self.coeffs.len() + other.coeffs.len() - 1];
        for (i, a) in self.coeffs.iter().enumerate() {
            for (j, b) in other.coeffs.iter().enumerate() {
                out[i + j] += a * b;
            }
        }
        Polynomial::new(out)
    }

    /// Polynomial long division: returns `(quotient, remainder)` with
    /// `self == quotient * divisor + remainder` and `deg(remainder) <
    /// deg(divisor)`. Fails closed (`None`) when dividing by the zero polynomial.
    pub fn div_rem(&self, divisor: &Polynomial) -> Option<(Polynomial, Polynomial)> {
        if divisor.is_zero() {
            return None; // fail closed — division by zero polynomial undefined
        }
        // deg(self) < deg(divisor) → quotient 0, remainder self
        let div_deg = divisor.degree().unwrap();
        if self.is_zero() || self.degree().unwrap() < div_deg {
            return Some((Polynomial::zero(), self.clone()));
        }
        let mut rem = self.coeffs.clone();
        let div_lead = divisor.leading();
        let quot_len = self.coeffs.len() - divisor.coeffs.len() + 1;
        let mut quot = vec![0.0; quot_len];

        // Work from the highest-order coefficient of the remainder downward.
        for i in (0..quot_len).rev() {
            let rem_idx = i + div_deg; // current leading term of `rem`
            let factor = rem[rem_idx] / div_lead;
            quot[i] = factor;
            if factor != 0.0 {
                for (j, dc) in divisor.coeffs.iter().enumerate() {
                    rem[i + j] -= factor * dc;
                }
            }
        }
        Some((Polynomial::new(quot), Polynomial::new(rem)))
    }

    /// First derivative.
    pub fn derivative(&self) -> Polynomial {
        if self.coeffs.len() <= 1 {
            return Polynomial::zero();
        }
        let mut out = Vec::with_capacity(self.coeffs.len() - 1);
        for (i, c) in self.coeffs.iter().enumerate().skip(1) {
            out.push(c * i as f64);
        }
        Polynomial::new(out)
    }

    /// Make the polynomial monic (leading coefficient 1). Zero polynomial maps
    /// to itself.
    pub fn monic(&self) -> Polynomial {
        if self.is_zero() {
            return Polynomial::zero();
        }
        let lead = self.leading();
        self.scale(1.0 / lead)
    }

    /// Greatest common divisor via the Euclidean algorithm, returned *monic*
    /// (so it is unique up to the normalisation `gcd` of the zero polynomial
    /// with `p` is `monic(p)`).
    pub fn gcd(&self, other: &Polynomial) -> Polynomial {
        let mut a = self.clone();
        let mut b = other.clone();
        while !b.is_zero() {
            // a mod b — divisor is non-zero so div_rem is Some
            let (_, r) = a.div_rem(&b).expect("b non-zero in gcd loop");
            a = b;
            b = r;
        }
        a.monic()
    }

    /// Resultant of `self` and `other` via the Euclidean remainder sequence.
    ///
    /// The resultant is zero **iff** the two polynomials share a common root
    /// (over the complex numbers / have a non-constant gcd). It is computed by
    /// running the Euclidean algorithm and accumulating the standard
    /// degree/leading-coefficient factors that relate `res(a, b)` to
    /// `res(b, a mod b)`:
    ///
    /// `res(a, b) = (-1)^(deg a · deg b) · lc(b)^(deg a − deg r) · res(b, r)`
    ///
    /// with base cases `res(a, const c) = c^(deg a)` and a zero result whenever
    /// a remainder vanishes with positive remaining degree (a common factor).
    pub fn resultant(&self, other: &Polynomial) -> f64 {
        // Degenerate cases.
        if self.is_zero() || other.is_zero() {
            return 0.0;
        }
        let mut a = self.clone();
        let mut b = other.clone();
        let mut result = 1.0_f64;

        loop {
            let deg_a = a.degree().unwrap();
            let deg_b = b.degree().unwrap();

            // Base case: b is a constant.
            if deg_b == 0 {
                // res(a, c) = c^(deg a)
                result *= b.leading().powi(deg_a as i32);
                return result;
            }

            // a mod b
            let (_, r) = a.div_rem(&b).expect("b non-constant ⇒ non-zero");

            // Sign factor from swapping the Euclidean step.
            if (deg_a % 2 == 1) && (deg_b % 2 == 1) {
                result = -result;
            }

            if r.is_zero() {
                // Common factor of positive degree ⇒ resultant is zero.
                return 0.0;
            }

            let deg_r = r.degree().unwrap();
            // lc(b)^(deg a − deg r)
            result *= b.leading().powi((deg_a as i32) - (deg_r as i32));

            a = b;
            b = r;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: `x - r` as a polynomial.
    fn linear(r: f64) -> Polynomial {
        Polynomial::new(vec![-r, 1.0])
    }

    fn approx(a: &Polynomial, b: &Polynomial) -> bool {
        if a.coeffs().len() != b.coeffs().len() {
            return false;
        }
        a.coeffs()
            .iter()
            .zip(b.coeffs())
            .all(|(x, y)| (x - y).abs() < 1e-6)
    }

    #[test]
    fn eval_horner() {
        // p(x) = 2 + 3x + x^2, p(2) = 2 + 6 + 4 = 12
        let p = Polynomial::new(vec![2.0, 3.0, 1.0]);
        assert!((p.eval(2.0) - 12.0).abs() < 1e-12);
    }

    #[test]
    fn add_sub() {
        let a = Polynomial::new(vec![1.0, 2.0, 3.0]);
        let b = Polynomial::new(vec![0.0, 1.0, -3.0]);
        assert_eq!(a.add(&b), Polynomial::new(vec![1.0, 3.0, 0.0]));
        // leading cancels → trimmed to degree 1
        assert_eq!(a.add(&b).degree(), Some(1));
        assert_eq!(a.sub(&a), Polynomial::zero());
    }

    #[test]
    fn mul_factors() {
        // (x-1)(x+1) = x^2 - 1
        let prod = linear(1.0).mul(&Polynomial::new(vec![1.0, 1.0]));
        assert!(approx(&prod, &Polynomial::new(vec![-1.0, 0.0, 1.0])));
    }

    #[test]
    fn division_exact_x2_minus_1() {
        // (x^2 - 1) / (x - 1) = (x + 1), remainder 0
        let num = Polynomial::new(vec![-1.0, 0.0, 1.0]);
        let den = linear(1.0); // x - 1
        let (q, r) = num.div_rem(&den).unwrap();
        assert!(approx(&q, &Polynomial::new(vec![1.0, 1.0])), "q = {:?}", q);
        assert!(r.is_zero(), "remainder = {:?}", r);
    }

    #[test]
    fn division_with_remainder_reconstructs() {
        // (x^3 + 2x + 1) / (x^2 + 1)
        let num = Polynomial::new(vec![1.0, 2.0, 0.0, 1.0]);
        let den = Polynomial::new(vec![1.0, 0.0, 1.0]);
        let (q, r) = num.div_rem(&den).unwrap();
        // q*den + r == num
        let recon = q.mul(&den).add(&r);
        assert!(approx(&recon, &num), "recon = {:?}", recon);
        assert!(r.degree().unwrap_or(0) < den.degree().unwrap());
    }

    #[test]
    fn divide_by_zero_poly_fails_closed() {
        let num = Polynomial::new(vec![1.0, 1.0]);
        assert!(num.div_rem(&Polynomial::zero()).is_none());
    }

    #[test]
    fn gcd_x2_minus_1_and_x_minus_1() {
        // gcd(x^2 - 1, x - 1) = x - 1 (up to scale → monic x - 1)
        let a = Polynomial::new(vec![-1.0, 0.0, 1.0]);
        let b = linear(1.0);
        let g = a.gcd(&b);
        // monic(x - 1) = x - 1
        assert!(approx(&g, &Polynomial::new(vec![-1.0, 1.0])), "gcd = {:?}", g);
    }

    #[test]
    fn gcd_shared_quadratic_factor() {
        // a = (x-1)(x-2), b = (x-2)(x-3) → gcd = (x-2) monic
        let a = linear(1.0).mul(&linear(2.0));
        let b = linear(2.0).mul(&linear(3.0));
        let g = a.gcd(&b);
        assert!(approx(&g, &linear(2.0)), "gcd = {:?}", g);
    }

    #[test]
    fn derivative_basic() {
        // d/dx (x^3 + 2x^2 + 5x + 7) = 3x^2 + 4x + 5
        let p = Polynomial::new(vec![7.0, 5.0, 2.0, 1.0]);
        assert_eq!(p.derivative(), Polynomial::new(vec![5.0, 4.0, 3.0]));
        assert!(Polynomial::constant(4.0).derivative().is_zero());
    }

    #[test]
    fn resultant_zero_iff_common_root() {
        // Share root x=2 ⇒ resultant 0.
        let a = linear(1.0).mul(&linear(2.0)); // (x-1)(x-2)
        let b = linear(2.0).mul(&linear(3.0)); // (x-2)(x-3)
        assert!(a.resultant(&b).abs() < 1e-6, "expected ~0, got {}", a.resultant(&b));

        // No common root ⇒ resultant non-zero.
        let c = linear(1.0).mul(&linear(2.0)); // (x-1)(x-2)
        let d = linear(3.0).mul(&linear(4.0)); // (x-3)(x-4)
        assert!(c.resultant(&d).abs() > 1e-6, "expected non-zero, got {}", c.resultant(&d));
    }

    #[test]
    fn resultant_known_value() {
        // res(x-1, x-2): the product of differences of roots = (1 - 2) = -1.
        // res(a,b) = lc(a)^deg(b) * prod over roots α of a of b(α)
        //          = 1 * b(1) = (1 - 2) = -1
        let a = linear(1.0); // x - 1
        let b = linear(2.0); // x - 2
        let r = a.resultant(&b);
        assert!((r - (-1.0)).abs() < 1e-9, "resultant = {}", r);
    }
}
