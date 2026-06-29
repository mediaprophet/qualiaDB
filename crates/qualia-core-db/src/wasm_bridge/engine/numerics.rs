//! Numerical-mathematics exports — special functions, number theory, interpolation
//! and derivative-free optimization.
//!
//! Wraps the engine's wasm-clean solver math: `crate::solvers::special_functions`,
//! `crate::solvers::number_theory`, `crate::solvers::interpolation`, and the fixed-size
//! `crate::solvers::optimization::NelderMeadSimplex`. This is the exact same code the
//! native MCP tools and the solver unit tests exercise — pure deterministic `f64`/`u64`
//! math, no `Instant`, no threads, no RNG.
//!
//! Conventions: every export takes one JS object and returns one JS object via
//! `serde_wasm_bindgen`; domain errors come back as `Err(JsValue::from_str(..))`. The
//! number-theory functions that can overflow `u128` (`factorial`, `binomial`, Stirling,
//! Catalan) fail closed with a clear message rather than wrapping. The overflow-prone
//! results are serialized as decimal **strings** because JSON/`f64` cannot hold a `u128`
//! exactly.
#![cfg(target_arch = "wasm32")]

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use super::jserr;

// ===========================================================================
// Special functions  (crate::solvers::special_functions)
// ===========================================================================

/// Bessel function of the first kind `J_n(x)`, integer order (any sign), defined for all
/// real `x`. Input `{ n, x }` -> `{ value }`.
#[wasm_bindgen]
pub fn num_bessel_j_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        n: i32,
        x: f64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let value = crate::solvers::special_functions::bessel_j(p.n, p.x);
    #[derive(Serialize)]
    struct Out {
        value: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { value })?)
}

/// Modified Bessel function of the first kind `I_n(x)`, integer order. Defined for all
/// real `x`. Input `{ n, x }` -> `{ value }`.
#[wasm_bindgen]
pub fn num_bessel_i_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        n: i32,
        x: f64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let value = crate::solvers::special_functions::bessel_i(p.n, p.x);
    #[derive(Serialize)]
    struct Out {
        value: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { value })?)
}

/// Bessel function of the second kind `Y_n(x)`, integer order `n >= 0`. Requires `x > 0`
/// (singular at the origin) and `J_0(x) != 0`. Input `{ n, x }` -> `{ value }`; errors
/// for `x <= 0` or an ill-posed Wronskian solve.
#[wasm_bindgen]
pub fn num_bessel_y_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        n: u32,
        x: f64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let value = crate::solvers::special_functions::bessel_y(p.n, p.x)
        .ok_or_else(|| JsValue::from_str("bessel_y: requires x > 0 (and J_0(x) != 0)"))?;
    #[derive(Serialize)]
    struct Out {
        value: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { value })?)
}

/// Modified Bessel function of the second kind `K_n(x)`, integer order `n >= 0`. Requires
/// `x > 0`. Input `{ n, x }` -> `{ value }`; errors for `x <= 0`.
#[wasm_bindgen]
pub fn num_bessel_k_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        n: u32,
        x: f64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let value = crate::solvers::special_functions::bessel_k(p.n, p.x)
        .ok_or_else(|| JsValue::from_str("bessel_k: requires x > 0"))?;
    #[derive(Serialize)]
    struct Out {
        value: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { value })?)
}

/// Airy functions `Ai(x)` and `Bi(x)` (both, from one Maclaurin-series evaluation).
/// Input `{ x }` -> `{ ai, bi }`.
#[wasm_bindgen]
pub fn num_airy_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        x: f64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let ai = crate::solvers::special_functions::airy_ai(p.x);
    let bi = crate::solvers::special_functions::airy_bi(p.x);
    #[derive(Serialize)]
    struct Out {
        ai: f64,
        bi: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { ai, bi })?)
}

/// Riemann zeta function `zeta(s)` for real `s > 1` (Euler-Maclaurin). Input `{ s }` ->
/// `{ value }`; errors for `s <= 1` (needs analytic continuation, out of this domain).
#[wasm_bindgen]
pub fn num_zeta_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        s: f64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let value = crate::solvers::special_functions::zeta(p.s)
        .ok_or_else(|| JsValue::from_str("zeta: requires s > 1"))?;
    #[derive(Serialize)]
    struct Out {
        value: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { value })?)
}

/// Classical orthogonal polynomial `P_n(x)` by three-term recurrence. `kind` is one of
/// `"legendre" | "chebyshev_t" | "chebyshev_u" | "hermite" | "laguerre"`.
/// Input `{ kind, n, x }` -> `{ value }`; errors on an unknown kind.
#[wasm_bindgen]
pub fn num_orthopoly_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        kind: String,
        n: u32,
        x: f64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    use crate::solvers::special_functions::{chebyshev_t, chebyshev_u, hermite, laguerre, legendre};
    let value = match p.kind.as_str() {
        "legendre" => legendre(p.n, p.x),
        "chebyshev_t" => chebyshev_t(p.n, p.x),
        "chebyshev_u" => chebyshev_u(p.n, p.x),
        "hermite" => hermite(p.n, p.x),
        "laguerre" => laguerre(p.n, p.x),
        other => {
            return Err(JsValue::from_str(&format!(
                "orthopoly: unknown kind '{other}' (legendre|chebyshev_t|chebyshev_u|hermite|laguerre)"
            )));
        }
    };
    #[derive(Serialize)]
    struct Out {
        value: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { value })?)
}

// ===========================================================================
// Number theory  (crate::solvers::number_theory)
// ===========================================================================

/// Deterministic Miller-Rabin primality test (exact for all `u64`).
/// Input `{ n }` -> `{ prime }`.
#[wasm_bindgen]
pub fn num_is_prime_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        n: u64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let prime = crate::solvers::number_theory::is_prime(p.n);
    #[derive(Serialize)]
    struct Out {
        prime: bool,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { prime })?)
}

/// Smallest prime strictly greater than `n`. Input `{ n }` -> `{ next_prime }`.
#[wasm_bindgen]
pub fn num_next_prime_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        n: u64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let next_prime = crate::solvers::number_theory::next_prime(p.n);
    #[derive(Serialize)]
    struct Out {
        next_prime: u64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { next_prime })?)
}

/// Prime factorization (trial division then Pollard's rho), correct across all `u64`.
/// Input `{ n }` -> `{ factors:[{ prime, exponent }] }`. Empty for `n < 2`.
#[wasm_bindgen]
pub fn num_prime_factorize_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        n: u64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    #[derive(Serialize)]
    struct Factor {
        prime: u64,
        exponent: u32,
    }
    let factors: Vec<Factor> = crate::solvers::number_theory::prime_factors(p.n)
        .into_iter()
        .map(|(prime, exponent)| Factor { prime, exponent })
        .collect();
    #[derive(Serialize)]
    struct Out {
        factors: Vec<Factor>,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { factors })?)
}

/// All positive divisors of `n`, ascending. Input `{ n }` -> `{ divisors:[..] }`.
#[wasm_bindgen]
pub fn num_divisors_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        n: u64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let divisors = crate::solvers::number_theory::divisors(p.n);
    #[derive(Serialize)]
    struct Out {
        divisors: Vec<u64>,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { divisors })?)
}

/// Greatest common divisor and least common multiple of `a` and `b`.
/// Input `{ a, b }` -> `{ gcd, lcm }`.
#[wasm_bindgen]
pub fn num_gcd_lcm_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        a: u64,
        b: u64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let gcd = crate::solvers::number_theory::gcd(p.a, p.b);
    let lcm = crate::solvers::number_theory::lcm(p.a, p.b);
    #[derive(Serialize)]
    struct Out {
        gcd: u64,
        lcm: u64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { gcd, lcm })?)
}

/// `(base^exp) mod modulus` by repeated squaring (overflow-safe via `u128`).
/// Input `{ base, exp, modulus }` -> `{ value }`.
#[wasm_bindgen]
pub fn num_mod_pow_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        base: u64,
        exp: u64,
        modulus: u64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let value = crate::solvers::number_theory::mod_pow(p.base, p.exp, p.modulus);
    #[derive(Serialize)]
    struct Out {
        value: u64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { value })?)
}

/// Modular multiplicative inverse: the `x` with `a*x ≡ 1 (mod m)`. Errors (fail closed)
/// when `gcd(a, m) != 1`. Input `{ a, m }` -> `{ inverse }`.
#[wasm_bindgen]
pub fn num_mod_inverse_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        a: u64,
        m: u64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let inverse = crate::solvers::number_theory::mod_inverse(p.a, p.m)
        .ok_or_else(|| JsValue::from_str("mod_inverse: no inverse (gcd(a, m) != 1)"))?;
    #[derive(Serialize)]
    struct Out {
        inverse: u64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { inverse })?)
}

/// Euler's totient `phi(n)`, the Mobius `mu(n)`, divisor count `d(n)` and divisor sum
/// `sigma(n)` — the classic multiplicative arithmetic functions, all from the prime
/// factorization. Input `{ n }` -> `{ totient, mobius, divisor_count, divisor_sum }`.
#[wasm_bindgen]
pub fn num_arithmetic_functions_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        n: u64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let totient = crate::solvers::number_theory::euler_totient(p.n);
    let mobius = crate::solvers::number_theory::mobius(p.n);
    let divisor_count = crate::solvers::number_theory::divisor_count(p.n);
    let divisor_sum = crate::solvers::number_theory::divisor_sum(p.n);
    #[derive(Serialize)]
    struct Out {
        totient: u64,
        mobius: i8,
        divisor_count: u64,
        divisor_sum: u64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        totient,
        mobius,
        divisor_count,
        divisor_sum,
    })?)
}

/// Binomial coefficient `C(n, k)` (exact integer at every step). Result is returned as a
/// decimal **string** since it may exceed `f64`/`u53` precision. Errors (fail closed) on
/// `u128` overflow. Input `{ n, k }` -> `{ value }` (value is a string).
#[wasm_bindgen]
pub fn num_binomial_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        n: u64,
        k: u64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let value = crate::solvers::number_theory::binomial(p.n, p.k)
        .ok_or_else(|| JsValue::from_str("binomial: result overflows u128"))?;
    #[derive(Serialize)]
    struct Out {
        value: String,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        value: value.to_string(),
    })?)
}

/// Factorial `n!` as an exact integer (decimal **string**; `f64` cannot hold it).
/// Errors (fail closed) for `n >= 35` (`35!` overflows `u128`).
/// Input `{ n }` -> `{ value }` (value is a string).
#[wasm_bindgen]
pub fn num_factorial_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        n: u64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let value = crate::solvers::number_theory::factorial(p.n)
        .ok_or_else(|| JsValue::from_str("factorial: result overflows u128 (n must be < 35)"))?;
    #[derive(Serialize)]
    struct Out {
        value: String,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        value: value.to_string(),
    })?)
}

/// Number of integer partitions `p(n)` (ways to write `n` as an unordered sum of positive
/// integers). Input `{ n }` -> `{ value }`.
#[wasm_bindgen]
pub fn num_partitions_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        n: u64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let value = crate::solvers::number_theory::partitions(p.n);
    #[derive(Serialize)]
    struct Out {
        value: u64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { value })?)
}

/// The `n`-th Catalan number, plus the Stirling numbers `S(n,k)` (second kind) and
/// `c(n,k)` (unsigned first kind). All exact integers as decimal **strings**; errors
/// (fail closed) on `u128` overflow. Input `{ n, k }` ->
/// `{ catalan, stirling_second, stirling_first }`.
#[wasm_bindgen]
pub fn num_combinatorics_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        n: u64,
        k: u64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let catalan = crate::solvers::number_theory::catalan(p.n)
        .ok_or_else(|| JsValue::from_str("catalan: result overflows u128"))?;
    let stirling_second = crate::solvers::number_theory::stirling_second(p.n, p.k)
        .ok_or_else(|| JsValue::from_str("stirling_second: result overflows u128"))?;
    let stirling_first = crate::solvers::number_theory::stirling_first(p.n, p.k)
        .ok_or_else(|| JsValue::from_str("stirling_first: result overflows u128"))?;
    #[derive(Serialize)]
    struct Out {
        catalan: String,
        stirling_second: String,
        stirling_first: String,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        catalan: catalan.to_string(),
        stirling_second: stirling_second.to_string(),
        stirling_first: stirling_first.to_string(),
    })?)
}

// ===========================================================================
// Interpolation  (crate::solvers::interpolation)
// ===========================================================================

/// Evaluate the Lagrange interpolating polynomial through `(xs, ys)` at `x`. Errors on
/// empty/mismatched data or duplicate nodes. Input `{ xs:[..], ys:[..], x }` -> `{ value }`.
#[wasm_bindgen]
pub fn num_lagrange_eval_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        xs: Vec<f64>,
        ys: Vec<f64>,
        x: f64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let value = crate::solvers::interpolation::lagrange_eval(&p.xs, &p.ys, p.x)
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
    #[derive(Serialize)]
    struct Out {
        value: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { value })?)
}

/// Newton divided-difference interpolation: build the coefficients from `(xs, ys)` and
/// evaluate the interpolant at `x`. Input `{ xs:[..], ys:[..], x }` ->
/// `{ value, coefficients:[..] }`.
#[wasm_bindgen]
pub fn num_newton_eval_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        xs: Vec<f64>,
        ys: Vec<f64>,
        x: f64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let coefficients = crate::solvers::interpolation::newton_coefficients(&p.xs, &p.ys)
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
    let value = crate::solvers::interpolation::newton_eval(&p.xs, &coefficients, p.x);
    #[derive(Serialize)]
    struct Out {
        value: f64,
        coefficients: Vec<f64>,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        value,
        coefficients,
    })?)
}

/// Natural cubic spline through `(xs, ys)` (xs strictly increasing), evaluated at each
/// query in `queries`. Errors on insufficient data, unsorted/duplicate nodes, or a
/// singular tridiagonal system. Input `{ xs:[..], ys:[..], queries:[..] }` ->
/// `{ values:[..] }`.
#[wasm_bindgen]
pub fn num_cubic_spline_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        xs: Vec<f64>,
        ys: Vec<f64>,
        queries: Vec<f64>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let spline = crate::solvers::interpolation::CubicSpline::natural(&p.xs, &p.ys)
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
    let values: Vec<f64> = p.queries.iter().map(|&q| spline.eval(q)).collect();
    #[derive(Serialize)]
    struct Out {
        values: Vec<f64>,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { values })?)
}

/// Piecewise-linear interpolation of `(xs, ys)` (xs strictly increasing) at `x` (clamped
/// to the endpoints outside the range). Input `{ xs:[..], ys:[..], x }` -> `{ value }`.
#[wasm_bindgen]
pub fn num_linear_interp_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        xs: Vec<f64>,
        ys: Vec<f64>,
        x: f64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let value = crate::solvers::interpolation::linear_interp(&p.xs, &p.ys, p.x)
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
    #[derive(Serialize)]
    struct Out {
        value: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { value })?)
}

/// Least-squares polynomial fit of degree `degree` to `(xs, ys)` (via the normal
/// equations). Returns coefficients in **ascending** order `[c0, c1, ..., c_degree]` (so
/// the polynomial is `sum c_k x^k`). Optionally evaluates the fit at each `queries` value.
/// Errors on too few points, `degree + 1 > n`, or a singular system.
/// Input `{ xs:[..], ys:[..], degree, queries?:[..] }` ->
/// `{ coefficients:[..], values:[..] }`.
#[wasm_bindgen]
pub fn num_poly_fit_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        xs: Vec<f64>,
        ys: Vec<f64>,
        degree: usize,
        #[serde(default)]
        queries: Vec<f64>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let coefficients = crate::solvers::interpolation::poly_fit(&p.xs, &p.ys, p.degree)
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
    let values: Vec<f64> = p
        .queries
        .iter()
        .map(|&q| crate::solvers::interpolation::poly_eval(&coefficients, q))
        .collect();
    #[derive(Serialize)]
    struct Out {
        coefficients: Vec<f64>,
        values: Vec<f64>,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        coefficients,
        values,
    })?)
}

// ===========================================================================
// Optimization  (crate::solvers::optimization::NelderMeadSimplex)
//
// The simplex optimizer minimizes an `ObjectiveFunction`. JS cannot hand us a Rust
// closure, so we expose a small set of BUILT-IN benchmark objectives selected by name.
// All are over a fixed 4-D vector `[f64; 4]` (the simplex dimension); callers supply the
// starting point (pad/truncate to 4 components).
// ===========================================================================

/// One of the built-in benchmark objectives, dispatched by name. Each minimizes over the
/// 4-D point `p = [p0, p1, p2, p3]`.
struct BuiltinObjective {
    kind: ObjectiveKind,
}

enum ObjectiveKind {
    /// `sum p_i^2` — global minimum 0 at the origin.
    Sphere,
    /// 4-D Rosenbrock: `sum_{i<3} [100 (p_{i+1} - p_i^2)^2 + (1 - p_i)^2]` — min 0 at all 1s.
    Rosenbrock,
    /// Booth (2-D, uses p0,p1): `(p0 + 2 p1 - 7)^2 + (2 p0 + p1 - 5)^2` — min 0 at (1, 3).
    Booth,
    /// Matyas (2-D, uses p0,p1): `0.26(p0^2 + p1^2) - 0.48 p0 p1` — min 0 at the origin.
    Matyas,
    /// Sum of absolute values `sum |p_i|` (the L1 "Bohachevsky-free" cone) — min 0 at origin.
    SumAbs,
}

impl crate::solvers::optimization::ObjectiveFunction for BuiltinObjective {
    fn evaluate(&self, p: &[f64; 4]) -> f64 {
        match self.kind {
            ObjectiveKind::Sphere => p.iter().map(|v| v * v).sum(),
            ObjectiveKind::Rosenbrock => {
                let mut s = 0.0;
                for i in 0..3 {
                    s += 100.0 * (p[i + 1] - p[i] * p[i]).powi(2) + (1.0 - p[i]).powi(2);
                }
                s
            }
            ObjectiveKind::Booth => {
                (p[0] + 2.0 * p[1] - 7.0).powi(2) + (2.0 * p[0] + p[1] - 5.0).powi(2)
            }
            ObjectiveKind::Matyas => 0.26 * (p[0] * p[0] + p[1] * p[1]) - 0.48 * p[0] * p[1],
            ObjectiveKind::SumAbs => p.iter().map(|v| v.abs()).sum(),
        }
    }
}

fn objective_by_name(name: &str) -> Option<ObjectiveKind> {
    match name {
        "sphere" => Some(ObjectiveKind::Sphere),
        "rosenbrock" => Some(ObjectiveKind::Rosenbrock),
        "booth" => Some(ObjectiveKind::Booth),
        "matyas" => Some(ObjectiveKind::Matyas),
        "sum_abs" => Some(ObjectiveKind::SumAbs),
        _ => None,
    }
}

/// Minimize a built-in benchmark objective with the Nelder-Mead simplex method
/// (derivative-free, deterministic, zero-allocation `[f64; 4]` simplex).
///
/// `objective` is one of `"sphere" | "rosenbrock" | "booth" | "matyas" | "sum_abs"`.
/// `start` is the initial 4-D point (missing components default to 0, extras ignored).
/// `max_iterations` (optional, default 1000) and `tolerance` (optional, default 1e-6)
/// configure the solver. Input
/// `{ objective, start:[..], max_iterations?, tolerance? }` ->
/// `{ best_point:[4], best_value, iterations, converged }`. Errors on an unknown objective.
#[wasm_bindgen]
pub fn num_minimize_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        objective: String,
        #[serde(default)]
        start: Vec<f64>,
        max_iterations: Option<u32>,
        tolerance: Option<f64>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let kind = objective_by_name(&p.objective).ok_or_else(|| {
        JsValue::from_str(&format!(
            "minimize: unknown objective '{}' (sphere|rosenbrock|booth|matyas|sum_abs)",
            p.objective
        ))
    })?;
    // Pad/truncate the starting point to the fixed 4-D simplex.
    let mut start = [0.0_f64; 4];
    for (i, slot) in start.iter_mut().enumerate() {
        if let Some(&v) = p.start.get(i) {
            *slot = v;
        }
    }
    let mut config = crate::solvers::SolverConfig::default();
    if let Some(m) = p.max_iterations {
        config.max_iterations = m;
    }
    if let Some(t) = p.tolerance {
        config.tolerance = t;
    }
    let mut simplex = crate::solvers::optimization::NelderMeadSimplex::new(start, config);
    let obj = BuiltinObjective { kind };
    let state = simplex
        .optimize(&obj)
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
    let (best_point, best_value) = simplex.get_best_solution();
    #[derive(Serialize)]
    struct Out {
        best_point: [f64; 4],
        best_value: f64,
        iterations: u32,
        converged: bool,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        best_point,
        best_value,
        iterations: state.iteration,
        converged: state.converged,
    })?)
}
