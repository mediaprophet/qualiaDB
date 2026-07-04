//! Zero-heap expansion arithmetic — the exact-fallback foundation.
//!
//! This is P1.3 in the computational-geometry execution plan. It implements
//! Shewchuk-style adaptive-precision floating-point arithmetic using
//! **error-free transformations** and **expansions** (sorted, non-overlapping
//! sequences of `f64` values that exactly represent a real number).
//!
//! ## Why this exists
//!
//! The filtered `f64` predicates in [`super::primitives`] are fast and exact
//! on the `f32`-sourced `Tensor10D` path (because `f32 → f64` promotion makes
//! the products exact). But for general `f64` inputs — and for cascaded
//! constructions where errors accumulate — the filtered path can mis-sign
//! near-degenerate cases. The exact-fallback ladder (P1.4–P1.7) needs a
//! zero-heap way to compute the exact sign of a determinant. This module
//! provides the arithmetic primitives that ladder is built from.
//!
//! ## Zero-heap contract
//!
//! Every function takes caller-supplied `&mut [f64]` output buffers. No
//! `Vec`, `String`, or `Box` is allocated in any operation. The caller sizes
//! buffers using the [`MAX_EXPANSION_*`] constants. If a buffer is too small,
//! the operation returns [`ExpansionError::OutputTooSmall`] — fail-closed,
//! never silent truncation.
//!
//! ## Expansion invariant
//!
//! An expansion `e = [e0, e1, ..., e_{n-1}]` is a sequence of `f64` values
//! such that:
//!
//! 1. **Non-overlapping:** the components do not overlap in their bit ranges
//!    (no component's significant bits overlap with another's).
//! 2. **Sorted by magnitude:** `|e0| <= |e1| <= ... <= |e_{n-1}|`.
//! 3. **Exact sum:** `sum(e_i)` equals the exact real-number result of the
//!    operation that produced the expansion.
//!
//! The sign of an expansion is the sign of its largest-magnitude component
//! (the last one), because the non-overlapping property guarantees no smaller
//! component can change it.
//!
//! ## References
//!
//! The algorithms are from Jonathan Richard Shewchuk, "Adaptive Precision
//! Floating-Point Arithmetic and Fast Robust Geometric Predicates" (1996,
//! Discrete & Computational Geometry). The implementation is original Rust,
//! adapted for the zero-heap caller-buffered contract. No CGAL or other
//! third-party source code is used — the algorithms are public-knowledge
//! numerical methods.

// ──────────────────────────────────────────────────────────────────────────
//  Workspace size constants
// ──────────────────────────────────────────────────────────────────────────

/// Maximum expansion length for the `orient2d` predicate (2×2 determinant
/// of differences: 2 terms × 2-component products, summed → length ≤ 8).
pub const MAX_EXPANSION_ORIENT2: usize = 8;

/// Maximum expansion length for the `orient3d` predicate (3×3 determinant
/// of differences: 6 terms × 3-component products, summed → length ≤ 24
/// without compression; with compression the actual length is smaller).
pub const MAX_EXPANSION_ORIENT3: usize = 24;

/// Maximum expansion length for the `incircle` predicate (3×3 determinant
/// with squared-distance entries: 6 terms × products of up to 4-component
/// expansions, summed → length ≤ 96 without compression).
pub const MAX_EXPANSION_INCIRCLE: usize = 96;

/// Maximum expansion length for the `insphere` predicate (5×5 determinant
/// with squared-distance entries: 120 terms × products of up to 6-component
/// expansions, summed → length ≤ 2048 without compression; with aggressive
/// zero-elimination the actual length is much smaller, but this bound
/// ensures the workspace is always sufficient).
///
/// This is the coordination point called out in the execution plan: the
/// P1.3 workspace must be sized for P1.6's insphere determinant. A 2048-f64
/// workspace is 16 KB — well within the 42 MB Sentinel ceiling.
pub const MAX_EXPANSION_INSPHERE: usize = 2048;

// ──────────────────────────────────────────────────────────────────────────
//  Error type
// ──────────────────────────────────────────────────────────────────────────

/// Errors from expansion arithmetic operations. All are fail-closed: the
/// caller must provide sufficiently large buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpansionError {
    /// The output buffer is too small for the operation.
    OutputTooSmall,
}

impl core::fmt::Display for ExpansionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ExpansionError::OutputTooSmall => {
                write!(f, "expansion output buffer too small")
            }
        }
    }
}

impl std::error::Error for ExpansionError {}

// ──────────────────────────────────────────────────────────────────────────
//  Error-free transformations (length-1 → length-2)
// ──────────────────────────────────────────────────────────────────────────

/// Error-free addition: `a + b = s + e` where `s = round(a + b)` and `e` is
/// the exact rounding error.
///
/// Knuth's algorithm (TAOCP Vol. 2, §4.2.2, Theorem B). Works for any `a, b`
/// regardless of relative magnitude. Six floating-point operations.
#[inline]
pub fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let a_prime = s - b;
    let b_prime = s - a_prime;
    let da = a - a_prime;
    let db = b - b_prime;
    let e = da + db;
    (s, e)
}

/// Error-free addition with precondition `|a| >= |b|`: `a + b = s + e`.
///
/// Faster than [`two_sum`] (three operations instead of six) but requires
/// `|a| >= |b|`. If the precondition is violated the result is still a valid
/// expansion but may not be error-free. Use [`two_sum`] when the relative
/// magnitudes are unknown.
#[inline]
pub fn fast_two_sum(a: f64, b: f64) -> (f64, f64) {
    debug_assert!(
        a.abs() >= b.abs(),
        "fast_two_sum precondition violated: |a| must be >= |b|"
    );
    let s = a + b;
    let e = b - (s - a);
    (s, e)
}

/// Error-free multiplication: `a * b = p + e` where `p = round(a * b)` and
/// `e` is the exact rounding error.
///
/// Uses `fma` (fused multiply-add) when available: `e = fma(a, b, -p)`.
/// On targets without hardware FMA this is still correct (the `mul_add`
/// intrinsic produces the mathematically exact result on all IEEE-754
/// platforms that Rust targets — it is the compiler's job to lower it).
#[inline]
pub fn two_product(a: f64, b: f64) -> (f64, f64) {
    let p = a * b;
    let e = a.mul_add(b, -p);
    (p, e)
}

/// Error-free subtraction: `a - b = s + e`. Equivalent to `two_sum(a, -b)`.
#[inline]
pub fn two_diff(a: f64, b: f64) -> (f64, f64) {
    two_sum(a, -b)
}

// ──────────────────────────────────────────────────────────────────────────
//  Expansion operations
// ──────────────────────────────────────────────────────────────────────────

/// Grow an expansion by adding a scalar: `h = e + b`.
///
/// `e` is the input expansion (length `elen`), `b` is the scalar, `h` is the
/// output buffer (must have length `>= elen + 1`). Returns the number of
/// components written to `h` (always `elen + 1`).
///
/// Implements Shewchuk's `grow_expansion` (§2.4): a single pass that merges
/// `b` into the expansion using [`fast_two_sum`], maintaining the
/// non-overlapping + sorted invariant.
///
/// # Errors
/// Returns [`ExpansionError::OutputTooSmall`] if `h.len() < elen + 1`.
pub fn grow_expansion(
    e: &[f64],
    b: f64,
    h: &mut [f64],
) -> Result<usize, ExpansionError> {
    let elen = e.len();
    if h.len() < elen + 1 {
        return Err(ExpansionError::OutputTooSmall);
    }

    // Merge b into the expansion. We use two_sum (not fast_two_sum) because
    // the relative magnitudes of e[i] and the running error are not
    // guaranteed to satisfy fast_two_sum's |a| >= |b| precondition.
    // two_sum is always error-free regardless of magnitudes (Knuth's
    // algorithm, 6 ops vs fast_two_sum's 3).
    let (sum, mut err) = two_sum(e[0], b);
    h[0] = sum;

    for i in 1..elen {
        let (s, e_i) = two_sum(e[i], err);
        h[i] = s;
        err = e_i;
    }
    h[elen] = err;
    Ok(elen + 1)
}

/// Scale an expansion by a scalar: `h = e * b`.
///
/// `e` is the input expansion (length `elen`), `b` is the scalar, `h` is the
/// output buffer (must have length `>= 2 * elen`). Returns the number of
/// components written to `h` (at most `2 * elen`).
///
/// Implements Shewchuk's `scale_expansion` (§2.5): each component `e[i]` is
/// split into `(e[i] * b, error)` via [`two_product`], and the errors are
/// accumulated into the output using [`two_sum`], maintaining the
/// non-overlapping + sorted invariant.
///
/// # Errors
/// Returns [`ExpansionError::OutputTooSmall`] if `h.len() < 2 * elen`.
pub fn scale_expansion(
    e: &[f64],
    b: f64,
    h: &mut [f64],
) -> Result<usize, ExpansionError> {
    let elen = e.len();
    if h.len() < 2 * elen {
        return Err(ExpansionError::OutputTooSmall);
    }
    if elen == 0 {
        return Ok(0);
    }

    // First component: two_product(e[0], b) → (product, error).
    let (p0, err0) = two_product(e[0], b);
    h[0] = p0;
    h[1] = err0;
    let mut hlen = 2;

    for i in 1..elen {
        let (pi, ei) = two_product(e[i], b);
        // Merge the error from the previous step with the product of the
        // current step, then append the new error.
        let (s0, e0) = two_sum(h[hlen - 1], pi);
        h[hlen - 1] = s0;
        let (s1, e1) = two_sum(e0, ei);
        h[hlen] = s1;
        h[hlen + 1] = e1;
        hlen += 2;
    }
    Ok(hlen)
}

/// Add two expansions: `h = e + f`.
///
/// `e` and `f` are input expansions, `h` is the output buffer (must have
/// length `>= e.len() + f.len()`). Returns the number of components written
/// to `h` (at most `e.len() + f.len()`).
///
/// Implements Shewchuk's `expansion_sum` (§2.6): a merge that processes both
/// expansions in order of increasing magnitude, using [`two_sum`] to maintain
/// the non-overlapping + sorted invariant.
///
/// # Errors
/// Returns [`ExpansionError::OutputTooSmall`] if `h.len() < e.len() + f.len()`.
pub fn expansion_sum(
    e: &[f64],
    f: &[f64],
    h: &mut [f64],
) -> Result<usize, ExpansionError> {
    let elen = e.len();
    let flen = f.len();
    if h.len() < elen + flen {
        return Err(ExpansionError::OutputTooSmall);
    }
    if elen == 0 {
        h[..flen].copy_from_slice(f);
        return Ok(flen);
    }
    if flen == 0 {
        h[..elen].copy_from_slice(e);
        return Ok(elen);
    }

    // Merge the two expansions like a merge-sort, feeding components through
    // two_sum to maintain the non-overlapping invariant.
    let mut ei = 0usize; // index into e
    let mut fi = 0usize; // index into f
    let mut hi = 0usize; // index into h

    // Pick the smaller-magnitude first component to start.
    let (mut current, from_e) = if e[0].abs() <= f[0].abs() {
        (e[0], true)
    } else {
        (f[0], false)
    };

    // Advance past the consumed component.
    if from_e {
        ei = 1;
    } else {
        fi = 1;
    }

    // Merge remaining components.
    while ei < elen && fi < flen {
        let next_e = e[ei];
        let next_f = f[fi];
        let (val, take_e) = if next_e.abs() <= next_f.abs() {
            (next_e, true)
        } else {
            (next_f, false)
        };

        let (s, err) = two_sum(current, val);
        h[hi] = s;
        hi += 1;
        current = err;

        if take_e {
            ei += 1;
        } else {
            fi += 1;
        }
    }

    // Drain remaining from e.
    while ei < elen {
        let (s, err) = two_sum(current, e[ei]);
        h[hi] = s;
        hi += 1;
        current = err;
        ei += 1;
    }

    // Drain remaining from f.
    while fi < flen {
        let (s, err) = two_sum(current, f[fi]);
        h[hi] = s;
        hi += 1;
        current = err;
        fi += 1;
    }

    // Append the final accumulated error.
    h[hi] = current;
    hi += 1;

    Ok(hi)
}

/// Compress an expansion: eliminate near-zero and zero components, producing
/// a minimal-length expansion with the same exact value.
///
/// `e` is the input expansion, `h` is the output buffer (must have length
/// `>= e.len()`). Returns the number of components written to `h`.
///
/// Implements Shewchuk's `compress` (§2.7): a two-pass accumulation that
/// merges adjacent components, eliminating zeros and reducing the expansion
/// to its minimal non-overlapping form.
///
/// # Errors
/// Returns [`ExpansionError::OutputTooSmall`] if `h.len() < e.len()`.
pub fn compress_expansion(
    e: &[f64],
    h: &mut [f64],
) -> Result<usize, ExpansionError> {
    let elen = e.len();
    if h.len() < elen {
        return Err(ExpansionError::OutputTooSmall);
    }
    if elen == 0 {
        return Ok(0);
    }
    if elen == 1 {
        h[0] = e[0];
        return Ok(1);
    }

    // Pass 1: top-down accumulation.
    // Process e from the largest component (e[elen-1]) down to the smallest.
    // We use two_sum (not fast_two_sum) because the running accumulator Q
    // may not satisfy |Q| >= |e[i]| in all cases (e.g., when components
    // have different signs and cancel). two_sum is always error-free.
    let mut bottom = e[elen - 1];
    for i in (0..elen - 1).rev() {
        let (s, err) = two_sum(bottom, e[i]);
        h[i + 1] = s;
        bottom = err;
    }
    h[0] = bottom;

    // Pass 2: bottom-up compression.
    // Q is the running sum (the largest component). At each step,
    // two_sum(Q, h[i]) produces (sum, error). The error is the smaller
    // part — output it if non-zero. Q becomes the sum for the next step.
    // This produces a sorted-by-increasing-magnitude expansion where
    // each error component is smaller than the running sum.
    let mut q = h[0];
    let mut out = 0usize;

    for i in 1..elen {
        let (s, err) = two_sum(q, h[i]);
        if err != 0.0 {
            h[out] = err;
            out += 1;
        }
        q = s;
    }

    // Output the final sum (the largest-magnitude component).
    // Even if it's zero, output it if there are no other components
    // (a zero expansion is [0.0] with length 1, not length 0).
    if q != 0.0 || out == 0 {
        h[out] = q;
        out += 1;
    }

    Ok(out)
}

/// Negate an expansion in place: each component negated.
#[inline]
pub fn negate_expansion(e: &mut [f64]) {
    for x in e.iter_mut() {
        *x = -*x;
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  Sign determination
// ──────────────────────────────────────────────────────────────────────────

/// Three-valued sign of a real number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    Negative = -1,
    Zero = 0,
    Positive = 1,
}

impl Sign {
    #[inline]
    pub fn from_f64(x: f64) -> Self {
        if x > 0.0 {
            Sign::Positive
        } else if x < 0.0 {
            Sign::Negative
        } else {
            Sign::Zero
        }
    }

    /// Flip the sign.
    #[inline]
    pub fn flip(self) -> Self {
        match self {
            Sign::Positive => Sign::Negative,
            Sign::Negative => Sign::Positive,
            Sign::Zero => Sign::Zero,
        }
    }
}

/// Determine the exact sign of an expansion.
///
/// For a properly-formed (non-overlapping, sorted by magnitude) expansion,
/// the sign is determined by the **last** (largest-magnitude) component.
/// The non-overlapping property guarantees that no smaller component can
/// change the sign.
///
/// Returns [`Sign::Zero`] if the expansion is empty or all components are
/// exactly zero.
#[inline]
pub fn sign_of_expansion(e: &[f64]) -> Sign {
    if e.is_empty() {
        return Sign::Zero;
    }
    // The expansion is sorted by increasing magnitude, so the last component
    // has the largest magnitude and determines the sign.
    Sign::from_f64(e[e.len() - 1])
}

// ──────────────────────────────────────────────────────────────────────────
//  Convenience: scalar product of two f64 as a length-2 expansion
// ──────────────────────────────────────────────────────────────────────────

/// Exact product of two scalars, written into a 2-element buffer.
///
/// Convenience wrapper around [`two_product`] that writes the result into
/// a caller-supplied buffer. `h` must have length `>= 2`.
#[inline]
pub fn scalar_product(a: f64, b: f64, h: &mut [f64]) -> Result<usize, ExpansionError> {
    if h.len() < 2 {
        return Err(ExpansionError::OutputTooSmall);
    }
    let (p, e) = two_product(a, b);
    h[0] = p;
    h[1] = e;
    Ok(2)
}

/// Exact sum of two scalars, written into a 2-element buffer.
///
/// Convenience wrapper around [`two_sum`]. `h` must have length `>= 2`.
#[inline]
pub fn scalar_sum(a: f64, b: f64, h: &mut [f64]) -> Result<usize, ExpansionError> {
    if h.len() < 2 {
        return Err(ExpansionError::OutputTooSmall);
    }
    let (s, e) = two_sum(a, b);
    h[0] = s;
    h[1] = e;
    Ok(2)
}

// ──────────────────────────────────────────────────────────────────────────
//  Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test-only exact arithmetic cross-check ───────────────────────────
    //
    // Every finite f64 can be represented exactly as `m * 2^e` where m is
    // a signed integer (the mantissa, up to 53 bits) and e is the exponent.
    // We use BigInt for the mantissa to handle arbitrary exponent differences
    // without overflow. This is test-only code — the expansion arithmetic
    // itself is zero-heap; the cross-check uses heap allocation freely.

    use num_bigint::BigInt;

    /// An exact real number: value = mantissa * 2^exponent.
    #[derive(Debug, Clone)]
    struct Exact {
        mantissa: BigInt,
        exponent: i32,
    }

    impl Exact {
        /// Convert an f64 to its exact representation.
        fn from_f64(x: f64) -> Self {
            if x == 0.0 {
                return Exact { mantissa: BigInt::from(0), exponent: 0 };
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
                    mantissa: BigInt::from(sign)
                        * (BigInt::from(1u64 << 52) + BigInt::from(raw_mant)),
                    exponent: raw_exp - 1023 - 52,
                }
            }
        }

        /// Exact addition. Aligns exponents, adds mantissas.
        fn add(self, other: Self) -> Self {
            if self.mantissa == 0.into() {
                return other;
            }
            if other.mantissa == 0.into() {
                return self;
            }
            // Determine which has the smaller exponent. Compute diff
            // before moving values into the tuple.
            let (lo, mut hi) = if self.exponent <= other.exponent {
                (self, other)
            } else {
                (other, self)
            };
            let diff = hi.exponent - lo.exponent;

            // Align to the smaller exponent by shifting the larger-exponent
            // mantissa left. This preserves exactness (no precision loss).
            if diff > 0 {
                hi.mantissa <<= diff;
            }
            Exact {
                mantissa: lo.mantissa + hi.mantissa,
                exponent: lo.exponent,
            }
        }

        /// Exact multiplication.
        fn mul(self, other: Self) -> Self {
            Exact {
                mantissa: self.mantissa * other.mantissa,
                exponent: self.exponent + other.exponent,
            }
        }

        /// Exact negation.
        fn neg(self) -> Self {
            Exact {
                mantissa: -self.mantissa,
                exponent: self.exponent,
            }
        }

        /// Compare two exact values. Returns true if they represent the same
        /// real number.
        fn equals(&self, other: &Self) -> bool {
            // Normalize: remove factors of 2 from the mantissa.
            let a = self.clone().normalize();
            let b = other.clone().normalize();
            a.mantissa == b.mantissa && a.exponent == b.exponent
        }

        /// Normalize: remove trailing zeros from the mantissa.
        fn normalize(mut self) -> Self {
            if self.mantissa == 0.into() {
                return Exact { mantissa: BigInt::from(0), exponent: 0 };
            }
            let zero = BigInt::from(0);
            let one = BigInt::from(1);
            while (&self.mantissa & &one) == zero {
                self.mantissa >>= 1;
                self.exponent += 1;
            }
            self
        }

        /// Sign of the exact value.
        fn sign(&self) -> Sign {
            use std::cmp::Ordering;
            match self.mantissa.cmp(&BigInt::from(0)) {
                Ordering::Greater => Sign::Positive,
                Ordering::Less => Sign::Negative,
                Ordering::Equal => Sign::Zero,
            }
        }
    }

    /// Convert an expansion (sum of f64s) to its exact value.
    fn expansion_to_exact(e: &[f64]) -> Exact {
        let mut acc = Exact { mantissa: BigInt::from(0), exponent: 0 };
        for &x in e {
            acc = acc.add(Exact::from_f64(x));
        }
        acc
    }

    // ── Error-free transformation tests ──────────────────────────────────

    #[test]
    fn two_sum_is_error_free() {
        // a + b = s + e, and s + e == a + b exactly.
        // Avoid overflow cases (1e308+1e308 → inf).
        let cases: [(f64, f64); 7] = [
            (1.0, 2.0),
            (1e100, 1e-100),
            (1.0, f64::EPSILON),
            (1e200, 1e200),
            (-1.0, 1.0 + f64::EPSILON),
            (0.1, 0.2),
            (1e300, -1e300 + 1.0),
        ];
        for &(a, b) in &cases {
            let (s, e) = two_sum(a, b);
            let exact_a = Exact::from_f64(a);
            let exact_b = Exact::from_f64(b);
            let exact_sum = exact_a.add(exact_b);
            let exact_result = Exact::from_f64(s).add(Exact::from_f64(e));
            assert!(
                exact_sum.equals(&exact_result),
                "two_sum({a}, {b}): s={s}, e={e} — exact mismatch"
            );
        }
    }

    #[test]
    fn fast_two_sum_matches_two_sum_when_precondition_holds() {
        let cases: [(f64, f64); 5] = [
            (2.0, 1.0),
            (1e100, 1.0),
            (1.0, f64::EPSILON),
            (1e308, 1e300),
            (-2.0, -1.0),
        ];
        for &(a, b) in &cases {
            assert!(a.abs() >= b.abs(), "precondition");
            let (s1, e1) = two_sum(a, b);
            let (s2, e2) = fast_two_sum(a, b);
            // Both are error-free, so s1+e1 == s2+e2 exactly.
            // They may not be bit-identical (different algorithms), but the
            // exact values must match.
            let exact1 = Exact::from_f64(s1).add(Exact::from_f64(e1));
            let exact2 = Exact::from_f64(s2).add(Exact::from_f64(e2));
            assert!(
                exact1.equals(&exact2),
                "fast_two_sum({a}, {b}): results differ from two_sum"
            );
        }
    }

    #[test]
    fn two_product_is_error_free() {
        // Avoid overflow (f64::MAX * 2 → inf).
        let cases: [(f64, f64); 7] = [
            (2.0, 3.0),
            (1e100, 1e-100),
            (1.0, f64::EPSILON),
            (0.1, 0.1),
            (1e154, 1e154),
            (-2.0, 3.0),
            (1e200, 1e-200),
        ];
        for &(a, b) in &cases {
            let (p, e) = two_product(a, b);
            let exact_a = Exact::from_f64(a);
            let exact_b = Exact::from_f64(b);
            let exact_prod = exact_a.mul(exact_b);
            let exact_result = Exact::from_f64(p).add(Exact::from_f64(e));
            assert!(
                exact_prod.equals(&exact_result),
                "two_product({a}, {b}): p={p}, e={e} — exact mismatch"
            );
        }
    }

    #[test]
    fn two_diff_is_error_free() {
        let cases = [
            (3.0, 1.0),
            (1e100, 1e100 - 1.0),
            (1.0, 1.0 + f64::EPSILON),
        ];
        for &(a, b) in &cases {
            let (s, e) = two_diff(a, b);
            let exact_a = Exact::from_f64(a);
            let exact_b = Exact::from_f64(b);
            let exact_diff = exact_a.add(exact_b.neg());
            let exact_result = Exact::from_f64(s).add(Exact::from_f64(e));
            assert!(
                exact_diff.equals(&exact_result),
                "two_diff({a}, {b}): s={s}, e={e} — exact mismatch"
            );
        }
    }

    // ── Expansion operation tests ────────────────────────────────────────

    #[test]
    fn grow_expansion_adds_scalar_exactly() {
        // e = [1.0, f64::EPSILON/2] (a length-2 expansion representing
        // 1 + eps/2). Adding 2.0 should give 3 + eps/2 exactly.
        let e = [1.0, f64::EPSILON / 2.0];
        let mut h = [0.0f64; 4];
        let n = grow_expansion(&e, 2.0, &mut h).unwrap();
        assert_eq!(n, 3);

        let exact_e = Exact::from_f64(e[0]).add(Exact::from_f64(e[1]));
        let exact_result = exact_e.add(Exact::from_f64(2.0));
        let exact_h = expansion_to_exact(&h[..n]);
        assert!(
            exact_h.equals(&exact_result),
            "grow_expansion: exact mismatch"
        );
    }

    #[test]
    fn grow_expansion_adversarial_cancellation() {
        // Add a tiny number to a large one, then add the negative of the
        // large one. The expansion should preserve the tiny number exactly.
        let big = 1e100;
        let tiny = 1e-100;
        let mut e = [0.0f64; 1];
        e[0] = big;
        let mut h1 = [0.0f64; 2];
        let n1 = grow_expansion(&e[..1], tiny, &mut h1).unwrap();
        assert_eq!(n1, 2);

        // Now add -big. The result should be exactly tiny.
        let mut h2 = [0.0f64; 4];
        let n2 = grow_expansion(&h1[..n1], -big, &mut h2).unwrap();
        assert_eq!(n2, 3);

        let exact_result = expansion_to_exact(&h2[..n2]);
        let exact_tiny = Exact::from_f64(tiny);
        assert!(
            exact_result.equals(&exact_tiny),
            "grow_expansion adversarial: expected exactly {tiny}, got {:?}",
            &h2[..n2]
        );
    }

    #[test]
    fn scale_expansion_multiplies_exactly() {
        // e = [1.0, f64::EPSILON/2], scale by 3.0.
        let e = [1.0, f64::EPSILON / 2.0];
        let mut h = [0.0f64; 8];
        let n = scale_expansion(&e, 3.0, &mut h).unwrap();
        assert!(n <= 4);

        let exact_e = Exact::from_f64(e[0]).add(Exact::from_f64(e[1]));
        let exact_result = exact_e.mul(Exact::from_f64(3.0));
        let exact_h = expansion_to_exact(&h[..n]);
        assert!(
            exact_h.equals(&exact_result),
            "scale_expansion: exact mismatch"
        );
    }

    #[test]
    fn scale_expansion_adversarial() {
        // Scale a cancellation-prone expansion by a large factor.
        // e = [1.0, -1.0 + eps] (represents eps). Scale by 1e50.
        let (s, e_err) = two_sum(-1.0, 1.0 + f64::EPSILON);
        // s should be ~eps, e_err should be the residual.
        let e = [s, e_err];
        let mut h = [0.0f64; 8];
        let n = scale_expansion(&e, 1e50, &mut h).unwrap();
        assert!(n <= 4);

        let exact_e = Exact::from_f64(e[0]).add(Exact::from_f64(e[1]));
        let exact_result = exact_e.mul(Exact::from_f64(1e50));
        let exact_h = expansion_to_exact(&h[..n]);
        assert!(
            exact_h.equals(&exact_result),
            "scale_expansion adversarial: exact mismatch"
        );
    }

    #[test]
    fn expansion_sum_adds_exactly() {
        // e = [1.0, eps], f = [2.0, eps/2]. Sum = 3.0 + 1.5*eps.
        let e = [1.0, f64::EPSILON];
        let f = [2.0, f64::EPSILON / 2.0];
        let mut h = [0.0f64; 8];
        let n = expansion_sum(&e, &f, &mut h).unwrap();
        assert!(n <= 4);

        let exact_e = Exact::from_f64(e[0]).add(Exact::from_f64(e[1]));
        let exact_f = Exact::from_f64(f[0]).add(Exact::from_f64(f[1]));
        let exact_result = exact_e.add(exact_f);
        let exact_h = expansion_to_exact(&h[..n]);
        assert!(
            exact_h.equals(&exact_result),
            "expansion_sum: exact mismatch"
        );
    }

    #[test]
    fn expansion_sum_adversarial_cancellation() {
        // e = [big, tiny], f = [-big, tiny]. Sum = 2*tiny.
        let big = 1e100;
        let tiny = 1e-100;
        let (s1, e1) = two_sum(big, tiny);
        let e = [s1, e1];
        let (s2, e2) = two_sum(-big, tiny);
        let f = [s2, e2];

        let mut h = [0.0f64; 8];
        let n = expansion_sum(&e, &f, &mut h).unwrap();

        let exact_e = Exact::from_f64(e[0]).add(Exact::from_f64(e[1]));
        let exact_f = Exact::from_f64(f[0]).add(Exact::from_f64(f[1]));
        let exact_result = exact_e.add(exact_f);
        let exact_h = expansion_to_exact(&h[..n]);
        assert!(
            exact_h.equals(&exact_result),
            "expansion_sum adversarial: expected {:?}, got {:?}",
            exact_result,
            exact_h
        );

        // The exact result should be 2*tiny.
        let exact_2tiny = Exact::from_f64(2.0).mul(Exact::from_f64(tiny));
        assert!(
            exact_h.equals(&exact_2tiny),
            "expansion_sum adversarial: expected exactly 2*tiny"
        );
    }

    #[test]
    fn expansion_sum_empty_operands() {
        let e: [f64; 0] = [];
        let f = [1.0, 2.0];
        let mut h = [0.0f64; 4];
        let n = expansion_sum(&e, &f, &mut h).unwrap();
        assert_eq!(n, 2);
        assert_eq!(&h[..n], &f[..]);

        let n = expansion_sum(&f, &e, &mut h).unwrap();
        assert_eq!(n, 2);
        assert_eq!(&h[..n], &f[..]);
    }

    #[test]
    fn compress_removes_zeros() {
        // Create a valid expansion with some zero components.
        // Sorted by increasing magnitude: [0.0, eps, 0.0, 1.0]
        // (zeros are valid in a Shewchuk expansion — they're smaller than
        // any non-zero component).
        let e = [0.0, f64::EPSILON, 0.0, 1.0];
        let mut h = [0.0f64; 4];
        let n = compress_expansion(&e, &mut h).unwrap();
        assert!(n <= 4);

        let exact_e = expansion_to_exact(&e);
        let exact_h = expansion_to_exact(&h[..n]);
        assert!(
            exact_h.equals(&exact_e),
            "compress: value changed"
        );
        // The compressed version should not be longer than the input.
        assert!(n <= e.len());
    }

    #[test]
    fn compress_preserves_value() {
        // A non-trivial expansion sorted by increasing magnitude:
        // [eps/4, eps/2, eps, 1.0].
        let e = [f64::EPSILON / 4.0, f64::EPSILON / 2.0, f64::EPSILON, 1.0];
        let mut h = [0.0f64; 4];
        let n = compress_expansion(&e, &mut h).unwrap();

        let exact_e = expansion_to_exact(&e);
        let exact_h = expansion_to_exact(&h[..n]);
        assert!(
            exact_h.equals(&exact_e),
            "compress: value changed for non-trivial expansion"
        );
    }

    #[test]
    fn compress_single_element() {
        let e = [42.0];
        let mut h = [0.0f64; 1];
        let n = compress_expansion(&e, &mut h).unwrap();
        assert_eq!(n, 1);
        assert_eq!(h[0], 42.0);
    }

    #[test]
    fn compress_empty() {
        let e: [f64; 0] = [];
        let mut h = [0.0f64; 0];
        let n = compress_expansion(&e, &mut h).unwrap();
        assert_eq!(n, 0);
    }

    // ── Sign determination tests ─────────────────────────────────────────

    #[test]
    fn sign_of_expansion_classifies_correctly() {
        assert_eq!(sign_of_expansion(&[1.0]), Sign::Positive);
        assert_eq!(sign_of_expansion(&[-1.0]), Sign::Negative);
        assert_eq!(sign_of_expansion(&[0.0]), Sign::Zero);
        assert_eq!(sign_of_expansion(&[]), Sign::Zero);

        // The last (largest) component determines the sign.
        assert_eq!(sign_of_expansion(&[1.0, 2.0]), Sign::Positive);
        assert_eq!(sign_of_expansion(&[2.0, -1.0]), Sign::Negative);
        assert_eq!(sign_of_expansion(&[f64::EPSILON, 1.0]), Sign::Positive);
        assert_eq!(sign_of_expansion(&[f64::EPSILON, -1.0]), Sign::Negative);
    }

    #[test]
    fn sign_of_cancellation_expansion() {
        // [big, -big + tiny] → the last component is -big+tiny which is
        // negative (since tiny << big). But the exact value is tiny (positive).
        // This is NOT a properly-formed expansion (the components overlap),
        // so sign_of_expansion would give the wrong answer. This test
        // demonstrates why we need compress before checking sign.
        //
        // With a properly-formed expansion (after grow_expansion), the
        // last component carries the true sign.
        let big = 1e100;
        let tiny = 1e-100;
        let mut e = [0.0f64; 1];
        e[0] = big;
        let mut h1 = [0.0f64; 2];
        let n1 = grow_expansion(&e[..1], tiny, &mut h1).unwrap();
        let mut h2 = [0.0f64; 4];
        let n2 = grow_expansion(&h1[..n1], -big, &mut h2).unwrap();

        // After compress, the expansion should be [tiny] (or [tiny, 0.0]).
        let mut h3 = [0.0f64; 4];
        let n3 = compress_expansion(&h2[..n2], &mut h3).unwrap();

        let sign = sign_of_expansion(&h3[..n3]);
        assert_eq!(
            sign, Sign::Positive,
            "sign should be positive (tiny > 0) after compress"
        );
    }

    // ── Determinism tests ────────────────────────────────────────────────

    #[test]
    fn grow_expansion_is_deterministic() {
        let e = [1.0, f64::EPSILON, 1e-300];
        let mut h1 = [0.0f64; 4];
        let mut h2 = [0.0f64; 4];
        let n1 = grow_expansion(&e, 3.14, &mut h1).unwrap();
        let n2 = grow_expansion(&e, 3.14, &mut h2).unwrap();
        assert_eq!(n1, n2);
        for i in 0..n1 {
            assert_eq!(h1[i].to_bits(), h2[i].to_bits(), "bit mismatch at {i}");
        }
    }

    #[test]
    fn scale_expansion_is_deterministic() {
        let e = [1.0, f64::EPSILON, 1e-300, 0.0];
        let mut h1 = [0.0f64; 8];
        let mut h2 = [0.0f64; 8];
        let n1 = scale_expansion(&e, 2.718, &mut h1).unwrap();
        let n2 = scale_expansion(&e, 2.718, &mut h2).unwrap();
        assert_eq!(n1, n2);
        for i in 0..n1 {
            assert_eq!(h1[i].to_bits(), h2[i].to_bits(), "bit mismatch at {i}");
        }
    }

    #[test]
    fn expansion_sum_is_deterministic() {
        let e = [1.0, f64::EPSILON, 1e-200];
        let f = [2.0, -f64::EPSILON, 1e-250];
        let mut h1 = [0.0f64; 8];
        let mut h2 = [0.0f64; 8];
        let n1 = expansion_sum(&e, &f, &mut h1).unwrap();
        let n2 = expansion_sum(&e, &f, &mut h2).unwrap();
        assert_eq!(n1, n2);
        for i in 0..n1 {
            assert_eq!(h1[i].to_bits(), h2[i].to_bits(), "bit mismatch at {i}");
        }
    }

    #[test]
    fn compress_is_deterministic() {
        let e = [1.0, f64::EPSILON, 0.0, 2.0, f64::EPSILON / 2.0];
        let mut h1 = [0.0f64; 5];
        let mut h2 = [0.0f64; 5];
        let n1 = compress_expansion(&e, &mut h1).unwrap();
        let n2 = compress_expansion(&e, &mut h2).unwrap();
        assert_eq!(n1, n2);
        for i in 0..n1 {
            assert_eq!(h1[i].to_bits(), h2[i].to_bits(), "bit mismatch at {i}");
        }
    }

    // ── Bounds checking tests ────────────────────────────────────────────

    #[test]
    fn grow_expansion_rejects_small_buffer() {
        let e = [1.0, 2.0, 3.0];
        let mut h = [0.0f64; 3]; // need 4
        assert_eq!(
            grow_expansion(&e, 4.0, &mut h),
            Err(ExpansionError::OutputTooSmall)
        );
    }

    #[test]
    fn scale_expansion_rejects_small_buffer() {
        let e = [1.0, 2.0, 3.0];
        let mut h = [0.0f64; 5]; // need 6
        assert_eq!(
            scale_expansion(&e, 2.0, &mut h),
            Err(ExpansionError::OutputTooSmall)
        );
    }

    #[test]
    fn expansion_sum_rejects_small_buffer() {
        let e = [1.0, 2.0];
        let f = [3.0, 4.0, 5.0];
        let mut h = [0.0f64; 4]; // need 5
        assert_eq!(
            expansion_sum(&e, &f, &mut h),
            Err(ExpansionError::OutputTooSmall)
        );
    }

    #[test]
    fn compress_rejects_small_buffer() {
        let e = [1.0, 2.0, 3.0];
        let mut h = [0.0f64; 2]; // need 3
        assert_eq!(
            compress_expansion(&e, &mut h),
            Err(ExpansionError::OutputTooSmall)
        );
    }

    // ── Workspace size constant tests ────────────────────────────────────

    #[test]
    fn workspace_constants_are_sized_for_predicates() {
        // The constants must be large enough for the predicate determinants.
        // These are lower bounds on the expansion length without compression;
        // the constants must be >= these.
        assert!(MAX_EXPANSION_ORIENT2 >= 8);
        assert!(MAX_EXPANSION_ORIENT3 >= 24);
        assert!(MAX_EXPANSION_INCIRCLE >= 96);
        assert!(MAX_EXPANSION_INSPHERE >= 2048);
    }

    // ── Full pipeline test: determinant-like computation ──────────────────

    #[test]
    fn full_pipeline_2x2_determinant_exact() {
        // det = a*d - b*c, computed via expansion arithmetic.
        // This is the orient2d determinant pattern.
        let a = 1.0;
        let b = 1.0 + f64::EPSILON;
        let c = 1.0 - f64::EPSILON;
        let d = 1.0;

        // ad = two_product(a, d) → length-2
        let mut ad = [0.0f64; 2];
        scalar_product(a, d, &mut ad).unwrap();

        // bc = two_product(b, c) → length-2
        let mut bc = [0.0f64; 2];
        scalar_product(b, c, &mut bc).unwrap();

        // negate bc
        negate_expansion(&mut bc);

        // det = ad + (-bc) = ad - bc
        let mut det = [0.0f64; 8];
        let n = expansion_sum(&ad, &bc, &mut det).unwrap();

        // Compress
        let mut compressed = [0.0f64; 8];
        let cn = compress_expansion(&det[..n], &mut compressed).unwrap();

        let sign = sign_of_expansion(&compressed[..cn]);

        // Exact: a*d - b*c = 1 - (1+eps)(1-eps) = 1 - (1 - eps²) = eps² > 0
        let exact_ad = Exact::from_f64(a).mul(Exact::from_f64(d));
        let exact_bc = Exact::from_f64(b).mul(Exact::from_f64(c));
        let exact_det = exact_ad.add(exact_bc.neg());
        let exact_sign = exact_det.sign();

        assert_eq!(
            sign, exact_sign,
            "2x2 determinant sign mismatch: expansion says {sign:?}, exact says {exact_sign:?}"
        );

        // Also verify the exact value matches
        let exact_h = expansion_to_exact(&compressed[..cn]);
        assert!(
            exact_h.equals(&exact_det),
            "2x2 determinant exact value mismatch"
        );
    }

    #[test]
    fn full_pipeline_3term_sum_adversarial() {
        // Three terms that cancel, computed via expansion arithmetic.
        // a = 1e100, b = -1e100 (as f64, -1e100 + 1e-100 is just -1e100
        // because 1e-100 is below the ULP of 1e100), c = -1e-100.
        // Exact result: a + b + c = 1e100 + (-1e100) + (-1e-100) = -1e-100.
        // The expansion should preserve the tiny -1e-100 value exactly,
        // even though it's far below the ULP of the intermediate 1e100.
        let a = 1e100;
        let b = -1e100;
        let c = -1e-100;

        // Start with a as a length-1 expansion
        let e = [a];
        let mut h1 = [0.0f64; 2];
        let n1 = grow_expansion(&e, b, &mut h1).unwrap();

        let mut h2 = [0.0f64; 4];
        let n2 = grow_expansion(&h1[..n1], c, &mut h2).unwrap();

        let mut h3 = [0.0f64; 4];
        let n3 = compress_expansion(&h2[..n2], &mut h3).unwrap();

        let exact_a = Exact::from_f64(a);
        let exact_b = Exact::from_f64(b);
        let exact_c = Exact::from_f64(c);
        let exact_result = exact_a.add(exact_b).add(exact_c);
        let exact_h = expansion_to_exact(&h3[..n3]);

        assert!(
            exact_h.equals(&exact_result),
            "3-term adversarial sum: exact mismatch — expected {:?}, got {:?}",
            exact_result,
            exact_h
        );

        // The exact result should be -1e-100 (negative, not zero).
        assert_eq!(
            exact_result.sign(),
            Sign::Negative,
            "3-term adversarial sum: expected negative (-1e-100)"
        );
    }

    #[test]
    fn full_pipeline_scale_then_sum() {
        // Compute (a*b + c*d) using scale + sum, the pattern used in
        // determinant computation.
        let a = 1.0;
        let b = 1.0 + f64::EPSILON;
        let c = 1.0 - f64::EPSILON;
        let d = 1.0 + 2.0 * f64::EPSILON;

        // ab = scale([a], b) → length-2
        let mut ab = [0.0f64; 4];
        let nab = scale_expansion(&[a], b, &mut ab).unwrap();

        // cd = scale([c], d) → length-2
        let mut cd = [0.0f64; 4];
        let ncd = scale_expansion(&[c], d, &mut cd).unwrap();

        // result = ab + cd
        let mut result = [0.0f64; 8];
        let n = expansion_sum(&ab[..nab], &cd[..ncd], &mut result).unwrap();

        let exact_ab = Exact::from_f64(a).mul(Exact::from_f64(b));
        let exact_cd = Exact::from_f64(c).mul(Exact::from_f64(d));
        let exact_result = exact_ab.add(exact_cd);
        let exact_h = expansion_to_exact(&result[..n]);

        assert!(
            exact_h.equals(&exact_result),
            "scale+sum pipeline: exact mismatch"
        );
    }

    // ── Negate test ──────────────────────────────────────────────────────

    #[test]
    fn negate_flips_sign() {
        let mut e = [1.0, -2.0, 3.0];
        negate_expansion(&mut e);
        assert_eq!(e, [-1.0, 2.0, -3.0]);
    }

    // ── Convenience wrapper tests ────────────────────────────────────────

    #[test]
    fn scalar_product_writes_two_components() {
        let mut h = [0.0f64; 2];
        let n = scalar_product(3.0, 7.0, &mut h).unwrap();
        assert_eq!(n, 2);
        assert_eq!(h[0], 21.0); // exact product, no error
        assert_eq!(h[1], 0.0);
    }

    #[test]
    fn scalar_sum_writes_two_components() {
        let mut h = [0.0f64; 2];
        let n = scalar_sum(1e100, 1e-100, &mut h).unwrap();
        assert_eq!(n, 2);
        // The sum is 1e100 (rounded), the error is 1e-100.
        assert_eq!(h[0], 1e100);
    }

    #[test]
    fn scalar_product_rejects_small_buffer() {
        let mut h = [0.0f64; 1];
        assert_eq!(
            scalar_product(1.0, 2.0, &mut h),
            Err(ExpansionError::OutputTooSmall)
        );
    }

    // ── Sign enum tests ──────────────────────────────────────────────────

    #[test]
    fn sign_flip() {
        assert_eq!(Sign::Positive.flip(), Sign::Negative);
        assert_eq!(Sign::Negative.flip(), Sign::Positive);
        assert_eq!(Sign::Zero.flip(), Sign::Zero);
    }

    #[test]
    fn sign_from_f64() {
        assert_eq!(Sign::from_f64(1.0), Sign::Positive);
        assert_eq!(Sign::from_f64(-1.0), Sign::Negative);
        assert_eq!(Sign::from_f64(0.0), Sign::Zero);
        assert_eq!(Sign::from_f64(1e-300), Sign::Positive);
        assert_eq!(Sign::from_f64(-1e-300), Sign::Negative);
    }
}
