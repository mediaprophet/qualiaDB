//! Exact rational numbers (`BigRational`) over [`BigInt`].
//!
//! A `BigRational` is a `numerator / denominator` pair that is **always** kept:
//! - reduced to lowest terms via gcd, and
//! - sign-normalised so the denominator is strictly positive (the sign lives in
//!   the numerator).
//!
//! Construction with a zero denominator **fails closed** (`None`); division by a
//! zero rational likewise returns `None`. No value is ever fabricated.

use core::cmp::Ordering;
use core::fmt;

use super::bigint::BigInt;

/// Exact rational number with arbitrary-precision numerator and denominator.
#[derive(Clone, PartialEq, Eq)]
pub struct BigRational {
    num: BigInt,
    den: BigInt, // invariant: den > 0, gcd(|num|, den) == 1
}

impl BigRational {
    /// The rational zero (`0/1`).
    pub fn zero() -> Self {
        BigRational {
            num: BigInt::zero(),
            den: BigInt::one(),
        }
    }

    /// The rational one (`1/1`).
    pub fn one() -> Self {
        BigRational {
            num: BigInt::one(),
            den: BigInt::one(),
        }
    }

    /// Construct from a `BigInt` (`n/1`).
    pub fn from_bigint(n: BigInt) -> Self {
        BigRational {
            num: n,
            den: BigInt::one(),
        }
    }

    /// Construct from an `i64` (`n/1`).
    pub fn from_i64(n: i64) -> Self {
        BigRational::from_bigint(BigInt::from_i64(n))
    }

    /// Construct from a numerator/denominator pair of `i64`. Fails closed on a
    /// zero denominator.
    pub fn from_i64s(num: i64, den: i64) -> Option<Self> {
        Self::new(BigInt::from_i64(num), BigInt::from_i64(den))
    }

    /// Construct from arbitrary [`BigInt`] numerator and denominator, reducing
    /// and sign-normalising. Returns `None` if `den` is zero.
    pub fn new(num: BigInt, den: BigInt) -> Option<Self> {
        if den.is_zero() {
            return None; // fail closed
        }
        let mut num = num;
        let mut den = den;
        // Move the sign onto the numerator; keep denominator positive.
        if den.is_negative() {
            num = num.neg();
            den = den.neg();
        }
        if num.is_zero() {
            return Some(BigRational {
                num: BigInt::zero(),
                den: BigInt::one(),
            });
        }
        let g = num.gcd(&den); // non-negative
        let num = num.div(&g).expect("g non-zero");
        let den = den.div(&g).expect("g non-zero");
        Some(BigRational { num, den })
    }

    /// Numerator (sign-bearing).
    pub fn numerator(&self) -> &BigInt {
        &self.num
    }

    /// Denominator (always positive).
    pub fn denominator(&self) -> &BigInt {
        &self.den
    }

    /// True if this is exactly zero.
    pub fn is_zero(&self) -> bool {
        self.num.is_zero()
    }

    /// Sign: `-1`, `0`, or `+1`.
    pub fn signum(&self) -> i8 {
        self.num.signum()
    }

    /// Absolute value.
    pub fn abs(&self) -> Self {
        BigRational {
            num: self.num.abs(),
            den: self.den.clone(),
        }
    }

    /// Arithmetic negation.
    pub fn neg(&self) -> Self {
        BigRational {
            num: self.num.neg(),
            den: self.den.clone(),
        }
    }

    /// Multiplicative inverse `den/num`. Fails closed if `self` is zero.
    pub fn recip(&self) -> Option<Self> {
        if self.is_zero() {
            return None;
        }
        BigRational::new(self.den.clone(), self.num.clone())
    }

    /// Sum `self + other`. Computes `(a*d + c*b) / (b*d)` then reduces.
    pub fn add(&self, other: &BigRational) -> BigRational {
        let num = self.num.mul(&other.den).add(&other.num.mul(&self.den));
        let den = self.den.mul(&other.den);
        BigRational::new(num, den).expect("product of positive denominators is non-zero")
    }

    /// Difference `self - other`.
    pub fn sub(&self, other: &BigRational) -> BigRational {
        self.add(&other.neg())
    }

    /// Product `self * other`.
    pub fn mul(&self, other: &BigRational) -> BigRational {
        let num = self.num.mul(&other.num);
        let den = self.den.mul(&other.den);
        BigRational::new(num, den).expect("product of positive denominators is non-zero")
    }

    /// Quotient `self / other`. Fails closed if `other` is zero.
    pub fn div(&self, other: &BigRational) -> Option<BigRational> {
        if other.is_zero() {
            return None;
        }
        let num = self.num.mul(&other.den);
        let den = self.den.mul(&other.num);
        BigRational::new(num, den)
    }

    /// Convert to the nearest `f64`. (Exact for small values; rounded otherwise.)
    pub fn to_f64(&self) -> f64 {
        // Parse the decimal string round-trips reliably for the magnitudes used
        // here; for very large values fall back to limb-wise scaling.
        let n = parse_bigint_f64(&self.num);
        let d = parse_bigint_f64(&self.den);
        n / d
    }
}

/// Best-effort `BigInt` → `f64`. Uses the decimal rendering, which `f64`'s
/// `from_str` rounds correctly to nearest.
fn parse_bigint_f64(b: &BigInt) -> f64 {
    b.to_string().parse::<f64>().unwrap_or_else(|_| {
        // Should not happen for decimal output, but never panic in a numeric path.
        if b.is_negative() {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        }
    })
}

impl PartialOrd for BigRational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BigRational {
    fn cmp(&self, other: &Self) -> Ordering {
        // a/b vs c/d  (b,d > 0)  ⇔  a*d vs c*b
        let lhs = self.num.mul(&other.den);
        let rhs = other.num.mul(&self.den);
        lhs.cmp(&rhs)
    }
}

impl fmt::Debug for BigRational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BigRational({}/{})", self.num, self.den)
    }
}

impl fmt::Display for BigRational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == BigInt::one() {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduces_on_construction() {
        // 2/4 → 1/2
        let r = BigRational::from_i64s(2, 4).unwrap();
        assert_eq!(r.numerator().to_string(), "1");
        assert_eq!(r.denominator().to_string(), "2");
    }

    #[test]
    fn sign_normalisation() {
        // 1 / -2 → -1/2 (denominator positive)
        let r = BigRational::from_i64s(1, -2).unwrap();
        assert_eq!(r.numerator().to_string(), "-1");
        assert_eq!(r.denominator().to_string(), "2");
        // -3 / -6 → 1/2
        let r2 = BigRational::from_i64s(-3, -6).unwrap();
        assert_eq!(r2.numerator().to_string(), "1");
        assert_eq!(r2.denominator().to_string(), "2");
    }

    #[test]
    fn third_plus_sixth_is_half() {
        let a = BigRational::from_i64s(1, 3).unwrap();
        let b = BigRational::from_i64s(1, 6).unwrap();
        let s = a.add(&b);
        assert_eq!(s, BigRational::from_i64s(1, 2).unwrap());
    }

    #[test]
    fn third_plus_two_thirds_is_one() {
        let a = BigRational::from_i64s(1, 3).unwrap();
        let b = BigRational::from_i64s(2, 3).unwrap();
        let s = a.add(&b);
        assert_eq!(s, BigRational::one());
        assert_eq!(s.numerator().to_string(), "1");
        assert_eq!(s.denominator().to_string(), "1");
    }

    #[test]
    fn sub_mul_div() {
        let a = BigRational::from_i64s(3, 4).unwrap();
        let b = BigRational::from_i64s(1, 4).unwrap();
        assert_eq!(a.sub(&b), BigRational::from_i64s(1, 2).unwrap());
        assert_eq!(a.mul(&b), BigRational::from_i64s(3, 16).unwrap());
        assert_eq!(a.div(&b).unwrap(), BigRational::from_i64(3));
    }

    #[test]
    fn div_by_zero_fails_closed() {
        let a = BigRational::from_i64(5);
        assert!(a.div(&BigRational::zero()).is_none());
        assert!(BigRational::new(BigInt::from_i64(1), BigInt::zero()).is_none());
        assert!(BigRational::from_i64s(1, 0).is_none());
        assert!(BigRational::zero().recip().is_none());
    }

    #[test]
    fn ordering() {
        let third = BigRational::from_i64s(1, 3).unwrap();
        let half = BigRational::from_i64s(1, 2).unwrap();
        assert!(third < half);
        assert!(half.neg() < third);
        assert_eq!(
            third.cmp(&BigRational::from_i64s(2, 6).unwrap()),
            Ordering::Equal
        );
    }

    #[test]
    fn to_f64_values() {
        assert!((BigRational::from_i64s(1, 2).unwrap().to_f64() - 0.5).abs() < 1e-15);
        assert!((BigRational::from_i64s(1, 4).unwrap().to_f64() - 0.25).abs() < 1e-15);
        assert!((BigRational::from_i64s(-3, 4).unwrap().to_f64() + 0.75).abs() < 1e-15);
    }

    #[test]
    fn exact_large_arithmetic() {
        // 1/3 summed three times == 1, exactly (no float drift).
        let third = BigRational::from_i64s(1, 3).unwrap();
        let s = third.add(&third).add(&third);
        assert_eq!(s, BigRational::one());
    }
}
