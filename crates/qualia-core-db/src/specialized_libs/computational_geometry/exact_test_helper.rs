//! Shared test-only exact-arithmetic cross-check for the predicate ladders.
//!
//! Every finite `f64` is represented exactly as `mantissa * 2^exponent` with a
//! `BigInt` mantissa (arbitrary precision). This lets the P1.4–P1.6 tests
//! validate the filtered/compensated/exact ladder against a ground-truth sign
//! computed with no rounding, over adversarial cancellation cases.
//!
//! This is **test-only** code (`#[cfg(test)]`). The predicate implementations
//! themselves are zero-heap; this helper uses heap allocation freely.
//!
//! Reused from `expansion.rs`'s own test module — factored here so each
//! predicate file's tests share one cross-check implementation.

use num_bigint::BigInt;

/// An exact real number: `value = mantissa * 2^exponent`.
#[derive(Debug, Clone)]
pub struct Exact {
    pub mantissa: BigInt,
    pub exponent: i32,
}

impl Exact {
    /// Convert an `f64` to its exact representation.
    pub fn from_f64(x: f64) -> Self {
        if x == 0.0 {
            return Exact {
                mantissa: BigInt::from(0),
                exponent: 0,
            };
        }
        let bits = x.to_bits();
        let sign: i8 = if bits >> 63 != 0 { -1 } else { 1 };
        let raw_exp = ((bits >> 52) & 0x7FF) as i32;
        let raw_mant = bits & 0x000F_FFFF_FFFF_FFFF;

        if raw_exp == 0 {
            // Subnormal: value = sign * raw_mant * 2^(-1074)
            Exact {
                mantissa: BigInt::from(sign) * BigInt::from(raw_mant),
                exponent: -1074,
            }
        } else {
            // Normalized: value = sign * (2^52 + raw_mant) * 2^(raw_exp - 1023 - 52)
            Exact {
                mantissa: BigInt::from(sign) * (BigInt::from(1u64 << 52) + BigInt::from(raw_mant)),
                exponent: raw_exp - 1023 - 52,
            }
        }
    }

    /// Exact addition. Aligns exponents by shifting the higher-exponent
    /// mantissa left (no precision loss), then adds.
    pub fn add(self, other: Self) -> Self {
        if self.mantissa == 0.into() {
            return other;
        }
        if other.mantissa == 0.into() {
            return self;
        }
        let (lo, mut hi) = if self.exponent <= other.exponent {
            (self, other)
        } else {
            (other, self)
        };
        let diff = hi.exponent - lo.exponent;
        if diff > 0 {
            hi.mantissa <<= diff;
        }
        Exact {
            mantissa: lo.mantissa + hi.mantissa,
            exponent: lo.exponent,
        }
    }

    /// Exact subtraction.
    pub fn sub(self, other: Self) -> Self {
        self.add(other.neg())
    }

    /// Exact multiplication.
    pub fn mul(self, other: Self) -> Self {
        Exact {
            mantissa: self.mantissa * other.mantissa,
            exponent: self.exponent + other.exponent,
        }
    }

    /// Exact negation.
    pub fn neg(self) -> Self {
        Exact {
            mantissa: -self.mantissa,
            exponent: self.exponent,
        }
    }

    /// Sign of the exact value.
    pub fn sign(&self) -> super::expansion::Sign {
        use std::cmp::Ordering;
        match self.mantissa.cmp(&BigInt::from(0)) {
            Ordering::Greater => super::expansion::Sign::Positive,
            Ordering::Less => super::expansion::Sign::Negative,
            Ordering::Equal => super::expansion::Sign::Zero,
        }
    }

    /// Compare two exact values for equality (after normalization).
    #[allow(dead_code)]
    pub fn equals(&self, other: &Self) -> bool {
        let a = self.clone().normalize();
        let b = other.clone().normalize();
        a.mantissa == b.mantissa && a.exponent == b.exponent
    }

    /// Normalize: remove trailing zero bits from the mantissa.
    #[allow(dead_code)]
    fn normalize(mut self) -> Self {
        if self.mantissa == 0.into() {
            return Exact {
                mantissa: BigInt::from(0),
                exponent: 0,
            };
        }
        let zero = BigInt::from(0);
        let one = BigInt::from(1);
        while (&self.mantissa & &one) == zero {
            self.mantissa >>= 1;
            self.exponent += 1;
        }
        self
    }
}

/// Convert an expansion (sum of `f64`s) to its exact value.
#[allow(dead_code)]
pub fn expansion_to_exact(e: &[f64]) -> Exact {
    let mut acc = Exact {
        mantissa: BigInt::from(0),
        exponent: 0,
    };
    for &x in e {
        acc = acc.add(Exact::from_f64(x));
    }
    acc
}
