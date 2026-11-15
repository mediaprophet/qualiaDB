//! Exact / arbitrary-precision arithmetic exports (`exact_*`).
//!
//! Wraps the engine's wasm-clean exact math (`crate::solvers::exact::{BigInt,
//! BigRational}`). Same code the native MCP tools and the `solvers::exact` unit
//! tests exercise — a real big-integer (sign + little-endian `u32` limbs,
//! schoolbook multiply, Knuth long division) and an always-reduced rational over
//! it. All I/O is via decimal / `"p/q"` STRINGS because JS `f64` cannot carry
//! arbitrary precision. Every fallible op (zero divisor / denominator, malformed
//! input) fails closed with a clear `Err` — nothing is fabricated.

#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use super::jserr;
#[cfg(target_arch = "wasm32")]
use crate::solvers::exact::{BigInt, BigRational};

// ── Zero-heap 256-bit stack-limb exact cubic verification (Q42-08) ───────────

#[inline]
const fn u256_add(a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    let mut out = [0u64; 4];
    let mut carry = 0u128;
    let mut i = 0;
    while i < 4 {
        let sum = (a[i] as u128) + (b[i] as u128) + carry;
        out[i] = sum as u64;
        carry = sum >> 64;
        i += 1;
    }
    out
}

#[inline]
const fn u256_neg(a: [u64; 4]) -> [u64; 4] {
    u256_add([!a[0], !a[1], !a[2], !a[3]], [1, 0, 0, 0])
}

#[inline]
const fn i256_from_i64(v: i64) -> [u64; 4] {
    let ext = if v < 0 { u64::MAX } else { 0u64 };
    [v as u64, ext, ext, ext]
}

#[inline]
const fn i256_cube_i64(v: i64) -> [u64; 4] {
    let neg = v < 0;
    let mag = v.unsigned_abs();
    let sq = (mag as u128) * (mag as u128);
    let sq_lo = sq as u64;
    let sq_hi = (sq >> 64) as u64;
    let p0 = (sq_lo as u128) * (mag as u128);
    let limb0 = p0 as u64;
    let p1 = (sq_hi as u128) * (mag as u128) + (p0 >> 64);
    let limb1 = p1 as u64;
    let limb2 = (p1 >> 64) as u64;
    let cube = [limb0, limb1, limb2, 0u64];
    if neg {
        u256_neg(cube)
    } else {
        cube
    }
}

/// Returns `true` iff `k ≡ 4 or 5 (mod 9)` (`k ≡ ±4 (mod 9)`), proving global algebraic
/// impossibility of `x^3 + y^3 + z^3 = k` over `ℤ^3` (`Q42-08` / §6.2.3).
#[inline]
pub const fn has_sum_of_three_cubes_mod9_obstruction(k: i64) -> bool {
    matches!(k.rem_euclid(9), 4 | 5)
}

/// Zero-heap exact signed 256-bit verification of `x^3 + y^3 + z^3 == k` over `[u64; 4]` stack limbs.
#[inline]
pub fn verify_sum_of_three_cubes_exact(
    x: i64,
    y: i64,
    z: i64,
    k: i64,
) -> Result<bool, &'static str> {
    if has_sum_of_three_cubes_mod9_obstruction(k) {
        return Ok(false);
    }
    let sum = u256_add(
        u256_add(i256_cube_i64(x), i256_cube_i64(y)),
        i256_cube_i64(z),
    );
    Ok(sum == i256_from_i64(k))
}

#[cfg(target_arch = "wasm32")]
mod wasm_exports {
    use super::*;

    // ── shared (de)serialization shapes ──────────────────────────────────────────

/// Two arbitrary-precision integers as decimal strings.
#[derive(Deserialize)]
struct TwoBigInts {
    a: String,
    b: String,
}

/// A single decimal-string result.
#[derive(Serialize)]
struct StringOut {
    result: String,
}

/// Parse a decimal string into a `BigInt`, failing closed with a clear message.
fn parse_int(s: &str, name: &str) -> Result<BigInt, JsValue> {
    BigInt::from_str(s).ok_or_else(|| {
        JsValue::from_str(&format!("`{name}` is not a valid decimal integer: {s:?}"))
    })
}

/// Parse a `"p/q"` (or bare `"p"`) string into a reduced `BigRational`.
/// Fails closed on malformed input or a zero denominator.
fn parse_rational(s: &str, name: &str) -> Result<BigRational, JsValue> {
    let trimmed = s.trim();
    let (num_s, den_s) = match trimmed.split_once('/') {
        Some((n, d)) => (n.trim(), d.trim()),
        None => (trimmed, "1"),
    };
    let num = parse_int(num_s, name)?;
    let den = parse_int(den_s, name)?;
    BigRational::new(num, den)
        .ok_or_else(|| JsValue::from_str(&format!("`{name}` has a zero denominator: {s:?}")))
}

/// Render a reduced rational as `"p/q"` (denominator always shown, always
/// positive — the demo wants an unambiguous canonical pair).
fn rational_pq(r: &BigRational) -> String {
    format!(
        "{}/{}",
        r.numerator().to_string(),
        r.denominator().to_string()
    )
}

// ── BigInt exports ───────────────────────────────────────────────────────────

/// Factorial `n!` as an exact decimal string. Input `{ n: u32 }` ->
/// `{ result }`. Computed from the wasm-clean `BigInt` primitives (the same
/// `mul` loop the solver's `factorial_100_known_value` test uses), so e.g.
/// `n = 100` returns the full 158-digit value with no overflow.
#[wasm_bindgen]
pub fn exact_bigint_factorial(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        n: u32,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    // Guard against absurd inputs that would never finish / would OOM the browser.
    if p.n > 50_000 {
        return Err(JsValue::from_str(
            "n is too large for the browser demo (max 50000)",
        ));
    }
    let mut acc = BigInt::one();
    for k in 1..=(p.n as u64) {
        acc = acc.mul(&BigInt::from_u64(k));
    }
    Ok(serde_wasm_bindgen::to_value(&StringOut {
        result: acc.to_string(),
    })?)
}

/// Exact integer power `base ^ exp`. Input `{ base: String, exp: u32 }` ->
/// `{ result }`. `base` is an arbitrary-precision decimal string; e.g.
/// `base = "2", exp = 100` returns `1267650600228229401496703205376`.
#[wasm_bindgen]
pub fn exact_bigint_pow(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        base: String,
        exp: u32,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if p.exp > 1_000_000 {
        return Err(JsValue::from_str(
            "exp is too large for the browser demo (max 1000000)",
        ));
    }
    let base = parse_int(&p.base, "base")?;
    Ok(serde_wasm_bindgen::to_value(&StringOut {
        result: base.pow(p.exp).to_string(),
    })?)
}

/// Exact sum `a + b`. Input `{ a: String, b: String }` -> `{ result }`.
#[wasm_bindgen]
pub fn exact_bigint_add(val: JsValue) -> Result<JsValue, JsValue> {
    let p: TwoBigInts = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let a = parse_int(&p.a, "a")?;
    let b = parse_int(&p.b, "b")?;
    Ok(serde_wasm_bindgen::to_value(&StringOut {
        result: a.add(&b).to_string(),
    })?)
}

/// Exact product `a * b`. Input `{ a: String, b: String }` -> `{ result }`.
#[wasm_bindgen]
pub fn exact_bigint_mul(val: JsValue) -> Result<JsValue, JsValue> {
    let p: TwoBigInts = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let a = parse_int(&p.a, "a")?;
    let b = parse_int(&p.b, "b")?;
    Ok(serde_wasm_bindgen::to_value(&StringOut {
        result: a.mul(&b).to_string(),
    })?)
}

/// Truncated division with remainder: `a = quotient*b + remainder`, remainder
/// taking the sign of `a` (toward-zero truncation, matching Rust `/` and `%`).
/// Input `{ a: String, b: String }` -> `{ quotient, remainder }`. Fails closed
/// (`Err`) when `b` is zero.
#[wasm_bindgen]
pub fn exact_bigint_divmod(val: JsValue) -> Result<JsValue, JsValue> {
    let p: TwoBigInts = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let a = parse_int(&p.a, "a")?;
    let b = parse_int(&p.b, "b")?;
    let (q, r) = a
        .divmod(&b)
        .ok_or_else(|| JsValue::from_str("division by zero"))?;
    #[derive(Serialize)]
    struct Out {
        quotient: String,
        remainder: String,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        quotient: q.to_string(),
        remainder: r.to_string(),
    })?)
}

/// Greatest common divisor `gcd(a, b)` (always non-negative; `gcd(0,0) = 0`).
/// Input `{ a: String, b: String }` -> `{ result }`.
#[wasm_bindgen]
pub fn exact_bigint_gcd(val: JsValue) -> Result<JsValue, JsValue> {
    let p: TwoBigInts = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let a = parse_int(&p.a, "a")?;
    let b = parse_int(&p.b, "b")?;
    Ok(serde_wasm_bindgen::to_value(&StringOut {
        result: a.gcd(&b).to_string(),
    })?)
}

// ── BigRational exports ──────────────────────────────────────────────────────

/// Exact rational sum `a + b`, returned reduced and sign-normalised as `"p/q"`
/// (q > 0). Inputs are `"p/q"` strings (a bare `"p"` is read as `p/1`). Input
/// `{ a: String, b: String }` -> `{ result }`. E.g. `"1/3" + "1/6" = "1/2"`.
#[wasm_bindgen]
pub fn exact_rational_add(val: JsValue) -> Result<JsValue, JsValue> {
    let p: TwoBigInts = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let a = parse_rational(&p.a, "a")?;
    let b = parse_rational(&p.b, "b")?;
    Ok(serde_wasm_bindgen::to_value(&StringOut {
        result: rational_pq(&a.add(&b)),
    })?)
}

/// Exact rational product `a * b`, returned reduced and sign-normalised as
/// `"p/q"` (q > 0). Inputs are `"p/q"` strings (a bare `"p"` is read as `p/1`).
/// Input `{ a: String, b: String }` -> `{ result }`. E.g. `"3/4" * "1/4" =
/// "3/16"`.
#[wasm_bindgen]
pub fn exact_rational_mul(val: JsValue) -> Result<JsValue, JsValue> {
    let p: TwoBigInts = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let a = parse_rational(&p.a, "a")?;
    let b = parse_rational(&p.b, "b")?;
    Ok(serde_wasm_bindgen::to_value(&StringOut {
        result: rational_pq(&a.mul(&b)),
    })?)
}
}

#[cfg(target_arch = "wasm32")]
pub use wasm_exports::*;
