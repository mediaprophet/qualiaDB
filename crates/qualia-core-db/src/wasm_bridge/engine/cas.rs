//! Computer-algebra (symbolic) exports — parse / differentiate / simplify / expand /
//! evaluate / factor / quadratic roots.
//!
//! Wraps `crate::specialized_libs::symbolic_algebra`, the ONE CAS module that is wasm
//! available (it is NOT `#[cfg(not(wasm32))]`-gated — see `specialized_libs/mod.rs:34`).
//! These are pure free functions over an `Expr` tree (parser, `differentiate`,
//! `simplify`, `expand`, `Expr::eval`, `factor_quadratic`, `solve_quadratic_symbolic`)
//! with no `Instant`/IO/threads/RNG — the same code the native CAS path and the
//! `symbolic_algebra` unit tests exercise. We do NOT touch any `*Library` struct.
//!
//! Deliberately EXCLUDED (all `#[cfg(not(target_arch = "wasm32"))]` native-only, so they
//! will not compile to wasm): `symbolic_integration`, `symbolic_limits`,
//! `symbolic_series`, `symbolic_solve`, `symbolic_ode`, `symbolic_trig`,
//! `symbolic_assumptions`, `multivar_calculus`, and `polynomial_algebra`. There is no
//! wasm integration / limit / general-equation-solve here; only the wasm-clean surface
//! of `symbolic_algebra` is exported.

#![cfg(target_arch = "wasm32")]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use super::jserr;
use crate::specialized_libs::symbolic_algebra as sa;

/// Symbolic derivative. Input `{ expr, var }` (e.g. `{ "expr":"x^3 - 2*x^2 + 5",
/// "var":"x" }`) → `{ derivative }`. The result is simplified, then rendered with the
/// `Expr` `Display` (fully parenthesised). Errors on a parse failure.
#[wasm_bindgen]
pub fn cas_differentiate_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        expr: String,
        var: String,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if p.expr.trim().is_empty() {
        return Err(JsValue::from_str("expr must be non-empty"));
    }
    if p.var.trim().is_empty() {
        return Err(JsValue::from_str("var must be non-empty"));
    }
    let e = sa::parse(&p.expr).map_err(|e| JsValue::from_str(&format!("parse error: {e}")))?;
    let d = sa::simplify(&sa::differentiate(&e, &p.var));
    #[derive(Serialize)]
    struct Out {
        derivative: String,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        derivative: format!("{d}"),
    })?)
}

/// Algebraic simplification (constant folding + identity elimination, to a bounded
/// fixpoint). Input `{ expr }` → `{ simplified }`. Errors on a parse failure.
#[wasm_bindgen]
pub fn cas_simplify_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        expr: String,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if p.expr.trim().is_empty() {
        return Err(JsValue::from_str("expr must be non-empty"));
    }
    let e = sa::parse(&p.expr).map_err(|e| JsValue::from_str(&format!("parse error: {e}")))?;
    let s = sa::simplify(&e);
    #[derive(Serialize)]
    struct Out {
        simplified: String,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        simplified: format!("{s}"),
    })?)
}

/// Distribute products over sums and expand small (≤ 8) positive integer powers, so the
/// result has no product/power over an additive child. Value-preserving. Input
/// `{ expr }` → `{ expanded }`. Errors on a parse failure.
#[wasm_bindgen]
pub fn cas_expand_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        expr: String,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if p.expr.trim().is_empty() {
        return Err(JsValue::from_str("expr must be non-empty"));
    }
    let e = sa::parse(&p.expr).map_err(|e| JsValue::from_str(&format!("parse error: {e}")))?;
    let x = sa::expand(&e);
    #[derive(Serialize)]
    struct Out {
        expanded: String,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        expanded: format!("{x}"),
    })?)
}

/// Numerically evaluate an expression given variable bindings. Input
/// `{ expr, bindings }` where `bindings` is an object of `name -> number`
/// (e.g. `{ "expr":"x^2 + 3*x + 2", "bindings":{ "x":4 } }`) → `{ value }`.
/// Errors if a referenced variable is unbound, or the result is non-finite
/// (division by zero, √negative, ln of a non-positive value).
#[wasm_bindgen]
pub fn cas_evaluate_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        expr: String,
        #[serde(default)]
        bindings: HashMap<String, f64>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if p.expr.trim().is_empty() {
        return Err(JsValue::from_str("expr must be non-empty"));
    }
    for (k, v) in p.bindings.iter() {
        if !v.is_finite() {
            return Err(JsValue::from_str(&format!(
                "binding '{k}' must be a finite number"
            )));
        }
    }
    let e = sa::parse(&p.expr).map_err(|e| JsValue::from_str(&format!("parse error: {e}")))?;
    let value = e.eval(&p.bindings).ok_or_else(|| {
        JsValue::from_str(
            "evaluate: an unbound variable or a non-finite result (e.g. division by zero, \
             sqrt of a negative, ln of a non-positive)",
        )
    })?;
    #[derive(Serialize)]
    struct Out {
        value: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { value })?)
}

/// Factor a real quadratic `a·x² + b·x + c` into `a·(x − r₁)·(x — r₂)` (roots snapped to
/// integers/halves when numerically close). Input `{ a, b, c, var }` (`var` defaults to
/// `"x"`) → `{ factored }`. Errors when `a = 0` or the discriminant is negative (no real
/// factorisation).
#[wasm_bindgen]
pub fn cas_factor_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        a: f64,
        b: f64,
        c: f64,
        #[serde(default = "default_var")]
        var: String,
    }
    fn default_var() -> String {
        "x".to_string()
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if !(p.a.is_finite() && p.b.is_finite() && p.c.is_finite()) {
        return Err(JsValue::from_str("a, b, c must be finite numbers"));
    }
    if p.var.trim().is_empty() {
        return Err(JsValue::from_str("var must be non-empty"));
    }
    let f = sa::factor_quadratic(p.a, p.b, p.c, &p.var).ok_or_else(|| {
        JsValue::from_str("factor: requires a != 0 and a non-negative discriminant (real roots)")
    })?;
    #[derive(Serialize)]
    struct Out {
        factored: String,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        factored: format!("{f}"),
    })?)
}

/// Symbolic roots of `a·x² + b·x + c = 0` as `(-b ± √(b²−4ac)) / (2a)` (simplified
/// `Expr` strings), plus their numeric values when the discriminant is non-negative.
/// Input `{ a, b, c }` → `{ roots:[{ expr, value }] }`. For `a = 0, b ≠ 0` returns the
/// single linear root `-c/b`; for `a = 0, b = 0` returns an empty list. A complex /
/// non-finite root value is reported as `null`.
#[wasm_bindgen]
pub fn cas_solve_quadratic_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        a: f64,
        b: f64,
        c: f64,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if !(p.a.is_finite() && p.b.is_finite() && p.c.is_finite()) {
        return Err(JsValue::from_str("a, b, c must be finite numbers"));
    }
    let roots = sa::solve_quadratic_symbolic(p.a, p.b, p.c);
    #[derive(Serialize)]
    struct Root {
        expr: String,
        value: Option<f64>,
    }
    let empty: HashMap<String, f64> = HashMap::new();
    let out: Vec<Root> = roots
        .iter()
        .map(|r| Root {
            expr: format!("{r}"),
            value: r.eval(&empty),
        })
        .collect();
    #[derive(Serialize)]
    struct Out {
        roots: Vec<Root>,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { roots: out })?)
}
