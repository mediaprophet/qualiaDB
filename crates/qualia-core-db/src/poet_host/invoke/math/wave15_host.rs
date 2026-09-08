//! Wave-15 Host binds: remaining pure Calculus symplectic / BDF / sensitivity.
//!
//! No forge/`caps()` / CUDA paths — scalar CPU only.

use super::super::args;
use crate::solvers::calculus::ode_advanced::{
    integrate_bdf, integrate_with_sensitivity, ruth3_step, verlet_step, yoshida4_step,
};
use vibe::{Diagnostic, Span, Value};

/// Harmonic-oscillator defaults: `force(q)=−k·q`, `kinetic_velocity(p)=p/m`.
fn harmonic_force_kv(args_v: &Value) -> (impl Fn(f64) -> f64, impl Fn(f64) -> f64) {
    let k = args::rec_f64(args_v, "k").unwrap_or(1.0);
    let mass = args::rec_f64(args_v, "mass").unwrap_or(1.0);
    let force = move |q: f64| -k * q;
    let kv = move |p: f64| p / mass;
    (force, kv)
}

/// Power-law RHS `f(t,y)=rate·y^power` (defaults rate=-1, power=1 → −y).
fn power_rhs(args_v: &Value) -> impl Fn(f64, f64) -> f64 {
    let rate = args::rec_f64(args_v, "rate").unwrap_or(-1.0);
    let power = args::rec_f64(args_v, "power").unwrap_or(1.0);
    move |_t, y| rate * y.powf(power)
}

/// `Calculus.verlet_step` — one Störmer–Verlet step. Args: `{ q, p, h, k?, mass? }`.
pub fn verlet_step_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let q = args::rec_f64(args_v, "q").ok_or_else(|| args::bad(span, "needs q"))?;
    let p = args::rec_f64(args_v, "p").ok_or_else(|| args::bad(span, "needs p"))?;
    let h = args::rec_f64(args_v, "h").ok_or_else(|| args::bad(span, "needs h"))?;
    let (force, kv) = harmonic_force_kv(args_v);
    let (q_new, p_new) = verlet_step(q, p, h, force, kv);
    Ok(args::record([("q", Value::F64(q_new)), ("p", Value::F64(p_new))]))
}

/// `Calculus.ruth3_step` — one Ruth 3rd-order symplectic step.
pub fn ruth3_step_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let q = args::rec_f64(args_v, "q").ok_or_else(|| args::bad(span, "needs q"))?;
    let p = args::rec_f64(args_v, "p").ok_or_else(|| args::bad(span, "needs p"))?;
    let h = args::rec_f64(args_v, "h").ok_or_else(|| args::bad(span, "needs h"))?;
    let (force, kv) = harmonic_force_kv(args_v);
    let (q_new, p_new) = ruth3_step(q, p, h, force, kv);
    Ok(args::record([("q", Value::F64(q_new)), ("p", Value::F64(p_new))]))
}

/// `Calculus.yoshida4_step` — one Yoshida 4th-order symplectic step.
pub fn yoshida4_step_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let q = args::rec_f64(args_v, "q").ok_or_else(|| args::bad(span, "needs q"))?;
    let p = args::rec_f64(args_v, "p").ok_or_else(|| args::bad(span, "needs p"))?;
    let h = args::rec_f64(args_v, "h").ok_or_else(|| args::bad(span, "needs h"))?;
    let (force, kv) = harmonic_force_kv(args_v);
    let (q_new, p_new) = yoshida4_step(q, p, h, force, kv);
    Ok(args::record([("q", Value::F64(q_new)), ("p", Value::F64(p_new))]))
}

/// `Calculus.integrate_bdf` — BDF2 integration. Args: `{ t0, y0, h, steps, rate?, power? }`.
pub fn integrate_bdf_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let t0 = args::rec_f64(args_v, "t0").ok_or_else(|| args::bad(span, "needs t0"))?;
    let y0 = args::rec_f64(args_v, "y0").ok_or_else(|| args::bad(span, "needs y0"))?;
    let h = args::rec_f64(args_v, "h").ok_or_else(|| args::bad(span, "needs h"))?;
    let steps = args::rec_u64(args_v, "steps").ok_or_else(|| args::bad(span, "needs steps"))?;
    let f = power_rhs(args_v);
    Ok(args::record([(
        "value",
        Value::F64(integrate_bdf(t0, y0, h, steps, f)),
    )]))
}

/// `Calculus.integrate_with_sensitivity` — state + ∂y/∂y₀ via RK4.
/// Args: `{ t0, y0, h, steps, rate?, power? }`. Out: `{ y, sensitivity }`.
pub fn integrate_with_sensitivity_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let t0 = args::rec_f64(args_v, "t0").ok_or_else(|| args::bad(span, "needs t0"))?;
    let y0 = args::rec_f64(args_v, "y0").ok_or_else(|| args::bad(span, "needs y0"))?;
    let h = args::rec_f64(args_v, "h").ok_or_else(|| args::bad(span, "needs h"))?;
    let steps = args::rec_u64(args_v, "steps").ok_or_else(|| args::bad(span, "needs steps"))?;
    let f = power_rhs(args_v);
    let r = integrate_with_sensitivity(t0, y0, h, steps, f);
    Ok(args::record([
        ("y", Value::F64(r.y)),
        ("sensitivity", Value::F64(r.sensitivity)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    fn qp_h(q: f64, p: f64, h: f64) -> Value {
        let mut m = BTreeMap::new();
        m.insert("q".into(), Value::F64(q));
        m.insert("p".into(), Value::F64(p));
        m.insert("h".into(), Value::F64(h));
        Value::Record(m)
    }

    #[test]
    fn wave15_verlet_conserves_energy_approx() {
        let out = verlet_step_host(&qp_h(1.0, 0.0, 0.01), span()).unwrap();
        let q = args::rec_f64(&out, "q").unwrap();
        let p = args::rec_f64(&out, "p").unwrap();
        let e0 = 0.5; // ½k q² + ½p²/m with k=m=1, q=1,p=0
        let e1 = 0.5 * q * q + 0.5 * p * p;
        assert!((e1 - e0).abs() < 1e-4);
    }

    #[test]
    fn wave15_ruth3_moves() {
        let out = ruth3_step_host(&qp_h(1.0, 0.0, 0.1), span()).unwrap();
        assert!(args::rec_f64(&out, "q").unwrap().abs() < 1.0);
        assert!(args::rec_f64(&out, "p").unwrap().abs() > 0.0);
    }

    #[test]
    fn wave15_yoshida4_moves() {
        let out = yoshida4_step_host(&qp_h(1.0, 0.0, 0.1), span()).unwrap();
        assert!(args::rec_f64(&out, "q").unwrap().abs() < 1.0);
    }

    #[test]
    fn wave15_integrate_bdf_decay() {
        let mut m = BTreeMap::new();
        m.insert("t0".into(), Value::F64(0.0));
        m.insert("y0".into(), Value::F64(1.0));
        m.insert("h".into(), Value::F64(0.1));
        m.insert("steps".into(), Value::U64(10));
        let out = integrate_bdf_host(&Value::Record(m), span()).unwrap();
        let y = args::rec_f64(&out, "value").unwrap();
        assert!(y > 0.0 && y < 1.0);
        assert!((y - (-1.0f64).exp()).abs() < 0.05); // e^{-1} ≈ 0.367
    }

    #[test]
    fn wave15_sensitivity_decay() {
        let mut m = BTreeMap::new();
        m.insert("t0".into(), Value::F64(0.0));
        m.insert("y0".into(), Value::F64(1.0));
        m.insert("h".into(), Value::F64(0.05));
        m.insert("steps".into(), Value::U64(20));
        let out = integrate_with_sensitivity_host(&Value::Record(m), span()).unwrap();
        let y = args::rec_f64(&out, "y").unwrap();
        let s = args::rec_f64(&out, "sensitivity").unwrap();
        // y'=−y → y(t)=e^{-t}, ∂y/∂y0 = e^{-t}
        assert!((y - (-1.0f64).exp()).abs() < 0.02);
        assert!((s - (-1.0f64).exp()).abs() < 0.02);
    }
}
