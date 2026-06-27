//! Arbitrary-precision signed integer (`BigInt`).
//!
//! This is part of the §3.1 "exact computation" foundation. It is a real,
//! self-contained big-integer implementation — sign + little-endian `u32`
//! magnitude limbs with schoolbook multiply and long division. It is **not** a
//! wrapper around `i128`.
//!
//! Heap-side is fine here: this is exact arbitrary-precision arithmetic, not a
//! zero-heap hot path. Operations that can fail (division by zero) return an
//! `Option`/`Result` and **fail closed** — they never fabricate a value.
//!
//! Internal representation invariants:
//! - `mag` holds base-2^32 limbs, little-endian (least significant first).
//! - `mag` is always *normalised*: no trailing zero limbs. Zero is the empty
//!   vector with `sign == 0`.
//! - `sign` is `-1`, `0`, or `+1`. `sign == 0` **iff** `mag` is empty.

use core::cmp::Ordering;
use core::fmt;

const BASE_BITS: u32 = 32;
const BASE: u64 = 1u64 << BASE_BITS; // 2^32

/// Arbitrary-precision signed integer.
#[derive(Clone, PartialEq, Eq)]
pub struct BigInt {
    /// `-1`, `0`, or `+1`. Zero iff `mag` is empty.
    sign: i8,
    /// Base-2^32 magnitude limbs, little-endian, no trailing zeros.
    mag: Vec<u32>,
}

impl BigInt {
    /// The integer zero.
    pub fn zero() -> Self {
        BigInt { sign: 0, mag: Vec::new() }
    }

    /// The integer one.
    pub fn one() -> Self {
        BigInt { sign: 1, mag: vec![1] }
    }

    /// Construct from an `i64`.
    pub fn from_i64(mut v: i64) -> Self {
        if v == 0 {
            return BigInt::zero();
        }
        let sign: i8 = if v < 0 { -1 } else { 1 };
        // Use unsigned magnitude to handle i64::MIN safely.
        let mut uv: u64 = if v < 0 {
            // negate in u64 space (handles MIN without overflow)
            (v as i128).unsigned_abs() as u64
        } else {
            v as u64
        };
        v = 0; // silence unused-assignment lints in some toolchains
        let _ = v;
        let mut mag = Vec::new();
        while uv > 0 {
            mag.push((uv & 0xFFFF_FFFF) as u32);
            uv >>= BASE_BITS;
        }
        let mut out = BigInt { sign, mag };
        out.normalize();
        out
    }

    /// Construct from a `u64`.
    pub fn from_u64(mut v: u64) -> Self {
        if v == 0 {
            return BigInt::zero();
        }
        let mut mag = Vec::new();
        while v > 0 {
            mag.push((v & 0xFFFF_FFFF) as u32);
            v >>= BASE_BITS;
        }
        BigInt { sign: 1, mag }
    }

    /// Parse a decimal string (optional leading `+`/`-`). Fails closed on any
    /// non-digit character or empty input.
    pub fn from_str(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }
        let (sign, digits) = match s.as_bytes()[0] {
            b'+' => (1i8, &s[1..]),
            b'-' => (-1i8, &s[1..]),
            _ => (1i8, s),
        };
        if digits.is_empty() {
            return None;
        }
        let mut acc = BigInt::zero();
        let ten = BigInt::from_u64(10);
        for ch in digits.bytes() {
            if !ch.is_ascii_digit() {
                return None;
            }
            let d = BigInt::from_u64((ch - b'0') as u64);
            acc = acc.mul(&ten).add(&d);
        }
        if acc.is_zero() {
            // "-0" / "0000" → canonical zero
            return Some(BigInt::zero());
        }
        acc.sign = sign;
        Some(acc)
    }

    /// Render as a decimal string with a leading `-` for negatives.
    pub fn to_string(&self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }
        // Repeatedly divmod by 10^9 (a chunk that fits in u32 output) and emit
        // groups of 9 decimal digits.
        let chunk = 1_000_000_000u64; // 10^9 < 2^32
        let mut limbs = self.mag.clone();
        let mut groups: Vec<u32> = Vec::new();
        while !limbs.is_empty() {
            let mut rem: u64 = 0;
            // divide magnitude (little-endian) by chunk, MSB-first
            for i in (0..limbs.len()).rev() {
                let cur = (rem << BASE_BITS) | limbs[i] as u64;
                limbs[i] = (cur / chunk) as u32;
                rem = cur % chunk;
            }
            // strip trailing zero limbs
            while let Some(&0) = limbs.last() {
                limbs.pop();
            }
            groups.push(rem as u32);
        }
        let mut out = String::new();
        if self.sign < 0 {
            out.push('-');
        }
        // most-significant group printed without leading zeros
        let last = groups.len() - 1;
        out.push_str(&groups[last].to_string());
        for i in (0..last).rev() {
            out.push_str(&format!("{:09}", groups[i]));
        }
        out
    }

    /// True if this is zero.
    pub fn is_zero(&self) -> bool {
        self.sign == 0
    }

    /// True if this is negative.
    pub fn is_negative(&self) -> bool {
        self.sign < 0
    }

    /// Sign: `-1`, `0`, or `+1`.
    pub fn signum(&self) -> i8 {
        self.sign
    }

    /// Absolute value.
    pub fn abs(&self) -> Self {
        BigInt { sign: if self.sign == 0 { 0 } else { 1 }, mag: self.mag.clone() }
    }

    /// Arithmetic negation.
    pub fn neg(&self) -> Self {
        BigInt { sign: -self.sign, mag: self.mag.clone() }
    }

    // ── normalisation ──────────────────────────────────────────────────────

    fn normalize(&mut self) {
        while let Some(&0) = self.mag.last() {
            self.mag.pop();
        }
        if self.mag.is_empty() {
            self.sign = 0;
        } else if self.sign == 0 {
            self.sign = 1;
        }
    }

    // ── magnitude helpers (ignore sign) ────────────────────────────────────

    /// Compare two magnitudes (little-endian limb vectors).
    fn cmp_mag(a: &[u32], b: &[u32]) -> Ordering {
        if a.len() != b.len() {
            return a.len().cmp(&b.len());
        }
        for i in (0..a.len()).rev() {
            if a[i] != b[i] {
                return a[i].cmp(&b[i]);
            }
        }
        Ordering::Equal
    }

    /// Add two magnitudes.
    fn add_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
        let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
        let mut out = Vec::with_capacity(long.len() + 1);
        let mut carry: u64 = 0;
        for i in 0..long.len() {
            let mut sum = long[i] as u64 + carry;
            if i < short.len() {
                sum += short[i] as u64;
            }
            out.push((sum & 0xFFFF_FFFF) as u32);
            carry = sum >> BASE_BITS;
        }
        if carry > 0 {
            out.push(carry as u32);
        }
        out
    }

    /// Subtract `b` from `a` where `a >= b` (magnitudes). Result is normalised.
    fn sub_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
        debug_assert!(Self::cmp_mag(a, b) != Ordering::Less);
        let mut out = Vec::with_capacity(a.len());
        let mut borrow: i64 = 0;
        for i in 0..a.len() {
            let bi = if i < b.len() { b[i] as i64 } else { 0 };
            let mut diff = a[i] as i64 - bi - borrow;
            if diff < 0 {
                diff += BASE as i64;
                borrow = 1;
            } else {
                borrow = 0;
            }
            out.push(diff as u32);
        }
        while let Some(&0) = out.last() {
            out.pop();
        }
        out
    }

    /// Multiply two magnitudes (schoolbook).
    fn mul_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
        if a.is_empty() || b.is_empty() {
            return Vec::new();
        }
        let mut out = vec![0u32; a.len() + b.len()];
        for i in 0..a.len() {
            let mut carry: u64 = 0;
            let ai = a[i] as u64;
            for j in 0..b.len() {
                let idx = i + j;
                let cur = out[idx] as u64 + ai * b[j] as u64 + carry;
                out[idx] = (cur & 0xFFFF_FFFF) as u32;
                carry = cur >> BASE_BITS;
            }
            // propagate remaining carry
            let mut idx = i + b.len();
            while carry > 0 {
                let cur = out[idx] as u64 + carry;
                out[idx] = (cur & 0xFFFF_FFFF) as u32;
                carry = cur >> BASE_BITS;
                idx += 1;
            }
        }
        while let Some(&0) = out.last() {
            out.pop();
        }
        out
    }

    // ── arithmetic ─────────────────────────────────────────────────────────

    /// Sum `self + other`.
    pub fn add(&self, other: &BigInt) -> BigInt {
        if self.is_zero() {
            return other.clone();
        }
        if other.is_zero() {
            return self.clone();
        }
        if self.sign == other.sign {
            let mag = Self::add_mag(&self.mag, &other.mag);
            let mut r = BigInt { sign: self.sign, mag };
            r.normalize();
            r
        } else {
            // different signs → subtract smaller magnitude from larger
            match Self::cmp_mag(&self.mag, &other.mag) {
                Ordering::Equal => BigInt::zero(),
                Ordering::Greater => {
                    let mag = Self::sub_mag(&self.mag, &other.mag);
                    let mut r = BigInt { sign: self.sign, mag };
                    r.normalize();
                    r
                }
                Ordering::Less => {
                    let mag = Self::sub_mag(&other.mag, &self.mag);
                    let mut r = BigInt { sign: other.sign, mag };
                    r.normalize();
                    r
                }
            }
        }
    }

    /// Difference `self - other`.
    pub fn sub(&self, other: &BigInt) -> BigInt {
        self.add(&other.neg())
    }

    /// Product `self * other`.
    pub fn mul(&self, other: &BigInt) -> BigInt {
        if self.is_zero() || other.is_zero() {
            return BigInt::zero();
        }
        let mag = Self::mul_mag(&self.mag, &other.mag);
        let mut r = BigInt { sign: self.sign * other.sign, mag };
        r.normalize();
        r
    }

    /// Truncated division and remainder: returns `(quotient, remainder)` such
    /// that `self == quotient * divisor + remainder`, with the remainder taking
    /// the sign of `self` (truncation toward zero, matching Rust's `/` and `%`
    /// on primitive integers). Fails closed (`None`) on division by zero.
    pub fn divmod(&self, divisor: &BigInt) -> Option<(BigInt, BigInt)> {
        if divisor.is_zero() {
            return None; // fail closed — never fabricate
        }
        if self.is_zero() {
            return Some((BigInt::zero(), BigInt::zero()));
        }
        // |self| < |divisor| → quotient 0, remainder self
        if Self::cmp_mag(&self.mag, &divisor.mag) == Ordering::Less {
            return Some((BigInt::zero(), self.clone()));
        }
        let (q_mag, r_mag) = Self::divmod_mag(&self.mag, &divisor.mag);
        let mut q = BigInt { sign: self.sign * divisor.sign, mag: q_mag };
        let mut r = BigInt { sign: self.sign, mag: r_mag };
        q.normalize();
        r.normalize();
        Some((q, r))
    }

    /// Quotient only (truncated toward zero). Fails closed on zero divisor.
    pub fn div(&self, divisor: &BigInt) -> Option<BigInt> {
        self.divmod(divisor).map(|(q, _)| q)
    }

    /// Remainder only (sign of `self`). Fails closed on zero divisor.
    pub fn rem(&self, divisor: &BigInt) -> Option<BigInt> {
        self.divmod(divisor).map(|(_, r)| r)
    }

    /// Knuth-style long division of magnitudes. Returns `(quotient, remainder)`
    /// magnitudes. Requires `a >= b` in magnitude and `b` non-empty.
    fn divmod_mag(a: &[u32], b: &[u32]) -> (Vec<u32>, Vec<u32>) {
        // Single-limb divisor: fast path.
        if b.len() == 1 {
            let d = b[0] as u64;
            let mut q = vec![0u32; a.len()];
            let mut rem: u64 = 0;
            for i in (0..a.len()).rev() {
                let cur = (rem << BASE_BITS) | a[i] as u64;
                q[i] = (cur / d) as u32;
                rem = cur % d;
            }
            while let Some(&0) = q.last() {
                q.pop();
            }
            let r = if rem == 0 { Vec::new() } else { vec![rem as u32] };
            return (q, r);
        }

        // Normalize so the divisor's top limb has its high bit set (Knuth D1).
        let shift = b[b.len() - 1].leading_zeros();
        let bn = Self::shl_bits(b, shift);
        let mut an = Self::shl_bits(a, shift);
        // Ensure `an` has one extra high limb to simplify indexing.
        if an.len() == a.len() {
            an.push(0);
        }
        let n = bn.len();
        let m = an.len() - n; // number of quotient limbs (an has n+m limbs)
        let mut q = vec![0u32; m];

        let b_high = bn[n - 1] as u64;
        let b_second = bn[n - 2] as u64;

        for j in (0..m).rev() {
            // Estimate q_hat from the top two limbs of the current remainder.
            let top = ((an[j + n] as u64) << BASE_BITS) | an[j + n - 1] as u64;
            let mut q_hat = top / b_high;
            let mut r_hat = top % b_high;
            // Refine q_hat (Knuth D3).
            while q_hat >= BASE
                || q_hat * b_second > (r_hat << BASE_BITS) | an[j + n - 2] as u64
            {
                q_hat -= 1;
                r_hat += b_high;
                if r_hat >= BASE {
                    break;
                }
            }

            // Multiply and subtract q_hat * bn from an[j..=j+n].
            let mut borrow: i64 = 0;
            let mut carry: u64 = 0;
            for i in 0..n {
                let p = q_hat * bn[i] as u64 + carry;
                carry = p >> BASE_BITS;
                let sub = an[j + i] as i64 - (p & 0xFFFF_FFFF) as i64 - borrow;
                if sub < 0 {
                    an[j + i] = (sub + BASE as i64) as u32;
                    borrow = 1;
                } else {
                    an[j + i] = sub as u32;
                    borrow = 0;
                }
            }
            let sub = an[j + n] as i64 - carry as i64 - borrow;
            if sub < 0 {
                // q_hat was one too big: add back (Knuth D6).
                an[j + n] = (sub + BASE as i64) as u32;
                q_hat -= 1;
                let mut carry2: u64 = 0;
                for i in 0..n {
                    let s = an[j + i] as u64 + bn[i] as u64 + carry2;
                    an[j + i] = (s & 0xFFFF_FFFF) as u32;
                    carry2 = s >> BASE_BITS;
                }
                an[j + n] = (an[j + n] as u64 + carry2) as u32;
            } else {
                an[j + n] = sub as u32;
            }
            q[j] = q_hat as u32;
        }

        while let Some(&0) = q.last() {
            q.pop();
        }
        // Remainder = (top n limbs of an) >> shift.
        let mut rem = an[..n].to_vec();
        while let Some(&0) = rem.last() {
            rem.pop();
        }
        let rem = Self::shr_bits(&rem, shift);
        (q, rem)
    }

    /// Shift a magnitude left by `bits` (0..32).
    fn shl_bits(a: &[u32], bits: u32) -> Vec<u32> {
        if bits == 0 || a.is_empty() {
            return a.to_vec();
        }
        let mut out = Vec::with_capacity(a.len() + 1);
        let mut carry: u32 = 0;
        for &limb in a {
            let v = ((limb as u64) << bits) | carry as u64;
            out.push((v & 0xFFFF_FFFF) as u32);
            carry = (v >> BASE_BITS) as u32;
        }
        if carry > 0 {
            out.push(carry);
        }
        out
    }

    /// Shift a magnitude right by `bits` (0..32).
    fn shr_bits(a: &[u32], bits: u32) -> Vec<u32> {
        if bits == 0 || a.is_empty() {
            return a.to_vec();
        }
        let mut out = vec![0u32; a.len()];
        let mut carry: u32 = 0;
        for i in (0..a.len()).rev() {
            let v = a[i];
            out[i] = (v >> bits) | carry;
            carry = v << (BASE_BITS - bits);
        }
        while let Some(&0) = out.last() {
            out.pop();
        }
        out
    }

    /// Raise to a non-negative integer power (exponentiation by squaring).
    pub fn pow(&self, exp: u32) -> BigInt {
        let mut result = BigInt::one();
        let mut base = self.clone();
        let mut e = exp;
        while e > 0 {
            if e & 1 == 1 {
                result = result.mul(&base);
            }
            e >>= 1;
            if e > 0 {
                base = base.mul(&base);
            }
        }
        result
    }

    /// Greatest common divisor (always non-negative). `gcd(0,0) == 0`.
    pub fn gcd(&self, other: &BigInt) -> BigInt {
        let mut a = self.abs();
        let mut b = other.abs();
        while !b.is_zero() {
            // a, b non-negative ⇒ rem is non-negative
            let r = a.rem(&b).expect("b non-zero in gcd loop");
            a = b;
            b = r;
        }
        a
    }
}

impl PartialOrd for BigInt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BigInt {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.sign.cmp(&other.sign) {
            Ordering::Equal => {}
            non_eq => return non_eq,
        }
        // same sign
        match self.sign {
            0 => Ordering::Equal,
            1 => Self::cmp_mag(&self.mag, &other.mag),
            _ => Self::cmp_mag(&other.mag, &self.mag), // both negative: reverse
        }
    }
}

impl fmt::Debug for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BigInt({})", self.to_string())
    }
}

impl fmt::Display for BigInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_and_to_i64_roundtrip() {
        for v in [0i64, 1, -1, 42, -42, 1_000_000, -1_000_000, i64::MAX, i64::MIN] {
            let b = BigInt::from_i64(v);
            assert_eq!(b.to_string(), v.to_string(), "value {v}");
        }
    }

    #[test]
    fn from_str_roundtrip_and_failclosed() {
        assert_eq!(BigInt::from_str("12345678901234567890").unwrap().to_string(),
                   "12345678901234567890");
        assert_eq!(BigInt::from_str("-99999999999999999999").unwrap().to_string(),
                   "-99999999999999999999");
        assert_eq!(BigInt::from_str("+7").unwrap().to_string(), "7");
        assert_eq!(BigInt::from_str("-0").unwrap().to_string(), "0");
        assert!(BigInt::from_str("").is_none());
        assert!(BigInt::from_str("12a3").is_none());
        assert!(BigInt::from_str("--3").is_none());
    }

    #[test]
    fn add_sub_signs() {
        let a = BigInt::from_i64(100);
        let b = BigInt::from_i64(-30);
        assert_eq!(a.add(&b).to_string(), "70");
        assert_eq!(b.add(&a).to_string(), "70");
        assert_eq!(a.sub(&b).to_string(), "130");
        assert_eq!(b.sub(&a).to_string(), "-130");
        assert_eq!(a.add(&a.neg()).to_string(), "0");
    }

    #[test]
    fn mul_and_pow_2_to_100() {
        // 2^100 known value
        let two = BigInt::from_i64(2);
        let p = two.pow(100);
        assert_eq!(p.to_string(), "1267650600228229401496703205376");
    }

    #[test]
    fn factorial_100_known_value() {
        let mut acc = BigInt::one();
        for k in 1..=100u64 {
            acc = acc.mul(&BigInt::from_u64(k));
        }
        // 100! — a well-known 158-digit constant.
        let expected = "93326215443944152681699238856266700490715968264381621468592963895217\
59999322991560894146397615651828625369792082722375825118521091686400\
0000000000000000000000";
        assert_eq!(acc.to_string(), expected);
    }

    #[test]
    fn divmod_known_values() {
        let a = BigInt::from_i64(100);
        let b = BigInt::from_i64(7);
        let (q, r) = a.divmod(&b).unwrap();
        assert_eq!(q.to_string(), "14");
        assert_eq!(r.to_string(), "2");

        // Reconstruct: q*b + r == a
        assert_eq!(q.mul(&b).add(&r), a);

        // Negative dividend → truncation toward zero, remainder sign of dividend
        let (q2, r2) = BigInt::from_i64(-100).divmod(&BigInt::from_i64(7)).unwrap();
        assert_eq!(q2.to_string(), "-14");
        assert_eq!(r2.to_string(), "-2");
    }

    #[test]
    fn divmod_multilimb() {
        // (2^100) / (2^50) == 2^50 exactly
        let num = BigInt::from_i64(2).pow(100);
        let den = BigInt::from_i64(2).pow(50);
        let (q, r) = num.divmod(&den).unwrap();
        assert!(r.is_zero());
        assert_eq!(q, BigInt::from_i64(2).pow(50));

        // big % big with remainder, verify reconstruction
        let x = BigInt::from_str("123456789012345678901234567890").unwrap();
        let y = BigInt::from_str("98765432109876543").unwrap();
        let (q, r) = x.divmod(&y).unwrap();
        assert_eq!(q.mul(&y).add(&r), x);
        assert!(r.abs() < y.abs());
    }

    #[test]
    fn divide_by_zero_fails_closed() {
        assert!(BigInt::from_i64(5).divmod(&BigInt::zero()).is_none());
        assert!(BigInt::from_i64(5).div(&BigInt::zero()).is_none());
        assert!(BigInt::from_i64(5).rem(&BigInt::zero()).is_none());
    }

    #[test]
    fn ordering() {
        assert!(BigInt::from_i64(-5) < BigInt::from_i64(-3));
        assert!(BigInt::from_i64(-3) < BigInt::from_i64(0));
        assert!(BigInt::from_i64(0) < BigInt::from_i64(3));
        assert!(BigInt::from_i64(3) < BigInt::from_i64(5));
        assert!(BigInt::from_i64(2).pow(100) > BigInt::from_i64(2).pow(99));
        assert_eq!(BigInt::from_i64(7).cmp(&BigInt::from_i64(7)), Ordering::Equal);
    }

    #[test]
    fn gcd_known() {
        assert_eq!(BigInt::from_i64(48).gcd(&BigInt::from_i64(36)).to_string(), "12");
        assert_eq!(BigInt::from_i64(-48).gcd(&BigInt::from_i64(36)).to_string(), "12");
        assert_eq!(BigInt::from_i64(17).gcd(&BigInt::from_i64(5)).to_string(), "1");
        assert_eq!(BigInt::from_i64(0).gcd(&BigInt::from_i64(9)).to_string(), "9");
    }

    #[test]
    fn abs_neg() {
        assert_eq!(BigInt::from_i64(-42).abs().to_string(), "42");
        assert_eq!(BigInt::from_i64(42).neg().to_string(), "-42");
        assert!(BigInt::zero().neg().is_zero());
    }
}
