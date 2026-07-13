//! WASM exports for the engine's **integral & discrete transforms** — Fourier
//! (DFT / inverse DFT), the Z-transform of a finite sequence plus its standard
//! closed forms, and the Laplace transform (numeric quadrature + symbolic table).
//!
//! Wraps the engine's wasm-clean solver math (`crate::solvers::transforms::*`).
//! Same code the native MCP tools and the solver unit tests exercise. On wasm
//! there is no GPU, so the forward DFT runs the exact f64 CPU path
//! (`dft` is the f64-exact reference; the f32 forge fast path is native-only and
//! is intentionally NOT reached here). Complex values are `(re, im)` `Cplx`
//! tuples throughout; every wrapper fails closed with a clear message.
#![cfg(target_arch = "wasm32")]

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use super::jserr;

use crate::solvers::transforms::fourier::{dft, idft, Cplx};
use crate::solvers::transforms::laplace::{laplace_numeric, laplace_table, LaplaceError};
use crate::solvers::transforms::ztransform::{geometric_z, unit_step_z, z_transform_finite};

// ── shared input/output shapes ──────────────────────────────────────────────

/// Build the complex sample vector the solver wants from the two accepted JS
/// encodings:
/// * `data` is real-only — each entry becomes `(x, 0)`; OR
/// * `re` and `im` are given as equal-length parallel arrays — `(re[k], im[k])`.
/// Exactly one encoding must be supplied and it must be non-empty.
fn build_cplx(
    data: &Option<Vec<f64>>,
    re: &Option<Vec<f64>>,
    im: &Option<Vec<f64>>,
) -> Result<Vec<Cplx>, JsValue> {
    match (data, re, im) {
        (Some(d), None, None) => {
            if d.is_empty() {
                return Err(JsValue::from_str("data must be non-empty"));
            }
            Ok(d.iter().map(|&x| (x, 0.0)).collect())
        }
        (None, Some(r), Some(i)) => {
            if r.is_empty() {
                return Err(JsValue::from_str("re/im must be non-empty"));
            }
            if r.len() != i.len() {
                return Err(JsValue::from_str("re and im must have the same length"));
            }
            Ok(r.iter().zip(i.iter()).map(|(&a, &b)| (a, b)).collect())
        }
        (Some(_), _, _) => Err(JsValue::from_str(
            "provide EITHER `data` (real) OR both `re` and `im` (complex), not both",
        )),
        _ => Err(JsValue::from_str(
            "provide `data` (real array) or both `re` and `im` (complex arrays)",
        )),
    }
}

/// Spectrum / signal output: parallel real, imaginary, and magnitude arrays.
#[derive(Serialize)]
struct Spectrum {
    re: Vec<f64>,
    im: Vec<f64>,
    magnitude: Vec<f64>,
    /// Number of bins/samples (== input length; the DFT is square).
    n: usize,
}

fn to_spectrum(out: Vec<Cplx>) -> Spectrum {
    let re: Vec<f64> = out.iter().map(|c| c.0).collect();
    let im: Vec<f64> = out.iter().map(|c| c.1).collect();
    let magnitude: Vec<f64> = out.iter().map(|c| (c.0 * c.0 + c.1 * c.1).sqrt()).collect();
    let n = out.len();
    Spectrum {
        re,
        im,
        magnitude,
        n,
    }
}

// ── Fourier: forward DFT ─────────────────────────────────────────────────────

/// Forward discrete Fourier transform `X[k] = Σ_n x[n] e^{-2πi kn/N}`
/// (un-normalized, forward sign convention). f64-exact CPU reference path.
///
/// Input `{ data:[..] }` (real signal) OR `{ re:[..], im:[..] }` (complex signal).
/// Output `{ re:[..], im:[..], magnitude:[..], n }`.
#[wasm_bindgen]
pub fn xform_dft(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        data: Option<Vec<f64>>,
        re: Option<Vec<f64>>,
        im: Option<Vec<f64>>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let x = build_cplx(&p.data, &p.re, &p.im)?;
    let out = dft(&x);
    Ok(serde_wasm_bindgen::to_value(&to_spectrum(out))?)
}

/// Inverse discrete Fourier transform `x[n] = (1/N) Σ_k X[k] e^{+2πi kn/N}`.
/// Round-trips `xform_dft` to ~1e-9.
///
/// Input the spectrum as `{ re:[..], im:[..] }` (complex bins) OR `{ data:[..] }`
/// (real bins → imaginary parts taken as 0).
/// Output `{ re:[..], im:[..], magnitude:[..], n }` — the recovered samples.
#[wasm_bindgen]
pub fn xform_idft(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        data: Option<Vec<f64>>,
        re: Option<Vec<f64>>,
        im: Option<Vec<f64>>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let spectrum = build_cplx(&p.data, &p.re, &p.im)?;
    let out = idft(&spectrum);
    Ok(serde_wasm_bindgen::to_value(&to_spectrum(out))?)
}

// ── Z-transform ──────────────────────────────────────────────────────────────

/// Z-transform of a finite causal sequence evaluated at a complex point `z`:
/// `X(z) = Σ_{n=0}^{N-1} x[n] z^{-n}`. Fails closed at `z = 0`.
///
/// Input `{ x:[..], z_re, z_im }`. Output `{ re, im, magnitude }`.
#[wasm_bindgen]
pub fn xform_z_transform(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        x: Vec<f64>,
        z_re: f64,
        z_im: f64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if p.x.is_empty() {
        return Err(JsValue::from_str("x must be a non-empty sequence"));
    }
    let v = z_transform_finite(&p.x, (p.z_re, p.z_im))
        .ok_or_else(|| JsValue::from_str("Z-transform undefined at z = 0"))?;
    Ok(serde_wasm_bindgen::to_value(&CplxOut::from(v))?)
}

/// Closed form of the unit-step `u[n]` Z-transform `X(z) = z/(z-1)`
/// (valid for `|z| > 1`). Fails closed at `z = 0` or `z = 1`.
///
/// Input `{ z_re, z_im }`. Output `{ re, im, magnitude }`.
#[wasm_bindgen]
pub fn xform_z_unit_step(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        z_re: f64,
        z_im: f64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let v = unit_step_z((p.z_re, p.z_im))
        .ok_or_else(|| JsValue::from_str("unit-step Z undefined at z = 0 or z = 1"))?;
    Ok(serde_wasm_bindgen::to_value(&CplxOut::from(v))?)
}

/// Closed form of the geometric `aⁿ u[n]` Z-transform `X(z) = 1/(1 - a z^{-1})`
/// (valid for `|z| > |a|`). Fails closed where the denominator vanishes / at `z = 0`.
///
/// Input `{ a, z_re, z_im }`. Output `{ re, im, magnitude }`.
#[wasm_bindgen]
pub fn xform_z_geometric(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        a: f64,
        z_re: f64,
        z_im: f64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let v = geometric_z(p.a, (p.z_re, p.z_im))
        .ok_or_else(|| JsValue::from_str("geometric Z undefined (z = 0 or 1 - a/z = 0)"))?;
    Ok(serde_wasm_bindgen::to_value(&CplxOut::from(v))?)
}

/// A single complex scalar result with its magnitude.
#[derive(Serialize)]
struct CplxOut {
    re: f64,
    im: f64,
    magnitude: f64,
}
impl From<Cplx> for CplxOut {
    fn from(c: Cplx) -> Self {
        CplxOut {
            re: c.0,
            im: c.1,
            magnitude: (c.0 * c.0 + c.1 * c.1).sqrt(),
        }
    }
}

// ── Laplace: numeric quadrature ──────────────────────────────────────────────

/// Numerical Laplace transform `L{f}(s) = ∫₀^∞ e^{-st} f(t) dt` by Simpson
/// quadrature, for a built-in time-function family (so a deterministic kernel
/// crosses the JS boundary instead of an arbitrary closure):
/// * `"one"`   → f(t)=1            (closed form 1/s)
/// * `"t"`     → f(t)=t            (1/s²)
/// * `"exp"`   → f(t)=e^{a·t}      (1/(s-a) for s>a)
/// * `"poly"`  → f(t)=tⁿ           (n!/s^{n+1}); supply `n`
/// * `"sin"`   → f(t)=sin(a·t)     (a/(s²+a²))
/// * `"cos"`   → f(t)=cos(a·t)     (s/(s²+a²))
/// `a` defaults to 1, `n` defaults to 1. Requires `s>0`, `t_max>0`, even `steps≥2`.
///
/// Input `{ fn, s, t_max, steps, a?, n? }`. Output `{ value, s, t_max, steps }`.
#[wasm_bindgen]
pub fn xform_laplace_numeric(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        #[serde(rename = "fn")]
        kernel: String,
        s: f64,
        t_max: f64,
        steps: usize,
        a: Option<f64>,
        n: Option<i32>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let a = p.a.unwrap_or(1.0);
    let n = p.n.unwrap_or(1);
    if n < 0 {
        return Err(JsValue::from_str("n must be >= 0 for the `poly` kernel"));
    }

    // Build the chosen deterministic time-function, then quadrature it.
    let value = match p.kernel.as_str() {
        "one" => laplace_numeric(|_t| 1.0, p.s, p.t_max, p.steps),
        "t" => laplace_numeric(|t| t, p.s, p.t_max, p.steps),
        "exp" => laplace_numeric(|t| (a * t).exp(), p.s, p.t_max, p.steps),
        "poly" => laplace_numeric(|t| t.powi(n), p.s, p.t_max, p.steps),
        "sin" => laplace_numeric(|t| (a * t).sin(), p.s, p.t_max, p.steps),
        "cos" => laplace_numeric(|t| (a * t).cos(), p.s, p.t_max, p.steps),
        other => {
            return Err(JsValue::from_str(&format!(
                "unknown fn kernel `{other}` (use one|t|exp|poly|sin|cos)"
            )))
        }
    }
    .map_err(laplace_err)?;

    #[derive(Serialize)]
    struct Out {
        value: f64,
        s: f64,
        t_max: f64,
        steps: usize,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        value,
        s: p.s,
        t_max: p.t_max,
        steps: p.steps,
    })?)
}

// ── Laplace: symbolic table ──────────────────────────────────────────────────

/// Symbolic Laplace transform of a polynomial in `t` from the table the CAS can
/// represent: a sum of `coeff · t^power` terms (constants are `power = 0`).
/// Returns the resulting `Expr` in `s` as a pretty string and, when `s` is
/// supplied, its numeric value `L{f}(s)`. Fails closed (`NotTransformable`) on
/// anything outside constants / integer powers / their linear combinations.
///
/// Input `{ terms:[{coeff, power}, ..], s? }`. Output `{ expr, value? }`.
#[wasm_bindgen]
pub fn xform_laplace_table(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::specialized_libs::symbolic_algebra::{add, c, mul, pow, var, Expr};
    use std::collections::HashMap;

    #[derive(Deserialize)]
    struct Term {
        coeff: f64,
        power: i32,
    }
    #[derive(Deserialize)]
    struct In {
        terms: Vec<Term>,
        s: Option<f64>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if p.terms.is_empty() {
        return Err(JsValue::from_str("terms must be non-empty"));
    }

    // Assemble the time-domain polynomial Σ coeff · t^power as an Expr.
    let term_expr = |coeff: f64, power: i32| -> Result<Expr, JsValue> {
        if power < 0 {
            return Err(JsValue::from_str("power must be >= 0"));
        }
        let base: Expr = match power {
            0 => c(1.0),
            1 => var("t"),
            k => pow(var("t"), k),
        };
        Ok(mul(c(coeff), base))
    };

    let mut iter = p.terms.iter();
    let first = iter.next().unwrap();
    let mut expr = term_expr(first.coeff, first.power)?;
    for t in iter {
        expr = add(expr, term_expr(t.coeff, t.power)?);
    }

    let transformed = laplace_table(&expr).map_err(laplace_err)?;

    let value = match p.s {
        Some(s) => {
            let mut env = HashMap::new();
            env.insert("s".to_string(), s);
            Some(transformed.eval(&env).ok_or_else(|| {
                JsValue::from_str("could not evaluate transform at the given s (e.g. s = 0)")
            })?)
        }
        None => None,
    };

    #[derive(Serialize)]
    struct Out {
        expr: String,
        value: Option<f64>,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        expr: transformed.to_string(),
        value,
    })?)
}

/// Render the solver's `LaplaceError` as a clear JS error string.
fn laplace_err(e: LaplaceError) -> JsValue {
    let msg = match e {
        LaplaceError::NotTransformable => {
            "expression is outside the symbolic Laplace table (constants / integer powers t^n / linear combinations only)"
        }
        LaplaceError::OutOfDomain => {
            "domain error: require s > 0, t_max > 0, and an even steps >= 2"
        }
    };
    JsValue::from_str(msg)
}
