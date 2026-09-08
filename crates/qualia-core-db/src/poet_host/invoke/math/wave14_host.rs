//! Wave-14 Host binds: remaining pure Calculus / GraphReasoning helpers.
//!
//! No forge/`caps()` paths — scalar CPU only.

use super::super::args;
use crate::solvers::calculus::exterior::permutation_parity as exterior_permutation_parity;
use crate::solvers::calculus::grid::{pack_f32_pair, unpack_f32_pair};
use crate::solvers::calculus::mechanics::invariant_drift as mechanics_invariant_drift;
use crate::solvers::calculus::ode_advanced::{bdf1_step, bdf2_step, hermite_dense_output};
use crate::solvers::graph_opt::top_k as graph_top_k;
use vibe::{Diagnostic, Span, Value};

/// `Calculus.hermite_dense_output` — cubic Hermite state at θ∈[0,1].
/// Args: `{ y0, f0, y1, f1, h, theta }`. Out: `{ value }`.
pub fn hermite_dense_output_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let y0 = args::rec_f64(args_v, "y0").ok_or_else(|| args::bad(span, "needs y0"))?;
    let f0 = args::rec_f64(args_v, "f0").ok_or_else(|| args::bad(span, "needs f0"))?;
    let y1 = args::rec_f64(args_v, "y1").ok_or_else(|| args::bad(span, "needs y1"))?;
    let f1 = args::rec_f64(args_v, "f1").ok_or_else(|| args::bad(span, "needs f1"))?;
    let h = args::rec_f64(args_v, "h").ok_or_else(|| args::bad(span, "needs h"))?;
    let theta = args::rec_f64(args_v, "theta").ok_or_else(|| args::bad(span, "needs theta"))?;
    Ok(args::record([(
        "value",
        Value::F64(hermite_dense_output(y0, f0, y1, f1, h, theta)),
    )]))
}

/// Power-law RHS `f(t,y) = rate * y.powf(power)` (defaults rate=-1, power=1 → −y).
fn power_rhs(args_v: &Value) -> impl Fn(f64, f64) -> f64 {
    let rate = args::rec_f64(args_v, "rate").unwrap_or(-1.0);
    let power = args::rec_f64(args_v, "power").unwrap_or(1.0);
    move |_t, y| rate * y.powf(power)
}

/// `Calculus.bdf1_step` — implicit Euler step. Args: `{ t0, y0, h, rate?, power? }`.
pub fn bdf1_step_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let t0 = args::rec_f64(args_v, "t0").ok_or_else(|| args::bad(span, "needs t0"))?;
    let y0 = args::rec_f64(args_v, "y0").ok_or_else(|| args::bad(span, "needs y0"))?;
    let h = args::rec_f64(args_v, "h").ok_or_else(|| args::bad(span, "needs h"))?;
    let f = power_rhs(args_v);
    Ok(args::record([("value", Value::F64(bdf1_step(t0, y0, h, f)))]))
}

/// `Calculus.bdf2_step` — BDF2. Args: `{ t1, y1, y0, h, rate?, power? }`.
pub fn bdf2_step_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let t1 = args::rec_f64(args_v, "t1").ok_or_else(|| args::bad(span, "needs t1"))?;
    let y1 = args::rec_f64(args_v, "y1").ok_or_else(|| args::bad(span, "needs y1"))?;
    let y0 = args::rec_f64(args_v, "y0").ok_or_else(|| args::bad(span, "needs y0"))?;
    let h = args::rec_f64(args_v, "h").ok_or_else(|| args::bad(span, "needs h"))?;
    let f = power_rhs(args_v);
    Ok(args::record([(
        "value",
        Value::F64(bdf2_step(t1, y1, y0, h, f)),
    )]))
}

/// `Calculus.invariant_drift` — absolute/relative drift of a conserved quantity.
/// Args: `{ initial, final }`. Out: `{ absolute_drift, relative_drift }`.
pub fn invariant_drift_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let initial =
        args::rec_f64(args_v, "initial").ok_or_else(|| args::bad(span, "needs initial"))?;
    let final_value =
        args::rec_f64(args_v, "final").ok_or_else(|| args::bad(span, "needs final"))?;
    let drift = mechanics_invariant_drift(initial, final_value)
        .map_err(|e| args::bad(span, format!("invariant_drift: {e:?}")))?;
    Ok(args::record([
        ("absolute_drift", Value::F64(drift.absolute_drift)),
        ("relative_drift", Value::F64(drift.relative_drift)),
    ]))
}

/// `Calculus.permutation_parity` — +1 even / −1 odd. Args: `{ permutation: [u64] }`.
pub fn permutation_parity_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let raw = args::rec_u64_list(args_v, "permutation")
        .ok_or_else(|| args::bad(span, "needs permutation: u64 list"))?;
    let perm: Vec<usize> = raw.into_iter().map(|v| v as usize).collect();
    let parity = exterior_permutation_parity(&perm)
        .map_err(|e| args::bad(span, format!("permutation_parity: {e:?}")))?;
    Ok(args::record([("parity", Value::I64(i64::from(parity)))]))
}

/// `Calculus.pack_f32_pair` — pack two f32 into one u64. Args: `{ step, comp }`.
pub fn pack_f32_pair_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let step = args::rec_f64(args_v, "step").ok_or_else(|| args::bad(span, "needs step"))? as f32;
    let comp = args::rec_f64(args_v, "comp").ok_or_else(|| args::bad(span, "needs comp"))? as f32;
    Ok(args::record([(
        "packed",
        Value::U64(pack_f32_pair(step, comp)),
    )]))
}

/// `Calculus.unpack_f32_pair` — unpack u64 → `{ step, comp }`.
pub fn unpack_f32_pair_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let packed =
        args::rec_u64(args_v, "packed").ok_or_else(|| args::bad(span, "needs packed"))?;
    let (step, comp) = unpack_f32_pair(packed);
    Ok(args::record([
        ("step", Value::F64(f64::from(step))),
        ("comp", Value::F64(f64::from(comp))),
    ]))
}

/// `GraphReasoning.top_k` — indices of top-k activations. Args: `{ activation, k }`.
pub fn top_k_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let activation = args::rec(args_v, "activation")
        .and_then(args::f64s)
        .ok_or_else(|| args::bad(span, "needs activation: f64 list"))?;
    let k = args::rec_u64(args_v, "k").unwrap_or(3) as usize;
    let idx = graph_top_k(&activation, k);
    Ok(args::record([(
        "indices",
        Value::List(idx.into_iter().map(|i| Value::U64(i as u64)).collect()),
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn wave14_hermite_endpoints() {
        let mut m = BTreeMap::new();
        m.insert("y0".into(), Value::F64(1.0));
        m.insert("f0".into(), Value::F64(0.0));
        m.insert("y1".into(), Value::F64(2.0));
        m.insert("f1".into(), Value::F64(0.0));
        m.insert("h".into(), Value::F64(1.0));
        m.insert("theta".into(), Value::F64(0.0));
        let out = hermite_dense_output_host(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_f64(&out, "value"), Some(1.0));
    }

    #[test]
    fn wave14_bdf1_decay() {
        let mut m = BTreeMap::new();
        m.insert("t0".into(), Value::F64(0.0));
        m.insert("y0".into(), Value::F64(1.0));
        m.insert("h".into(), Value::F64(0.1));
        let out = bdf1_step_host(&Value::Record(m), span()).unwrap();
        let y = args::rec_f64(&out, "value").unwrap();
        assert!(y < 1.0 && y > 0.8);
    }

    #[test]
    fn wave14_invariant_drift_zero() {
        let mut m = BTreeMap::new();
        m.insert("initial".into(), Value::F64(2.0));
        m.insert("final".into(), Value::F64(2.0));
        let out = invariant_drift_host(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_f64(&out, "absolute_drift"), Some(0.0));
    }

    #[test]
    fn wave14_permutation_parity_swap() {
        let mut m = BTreeMap::new();
        m.insert(
            "permutation".into(),
            Value::List(vec![Value::U64(1), Value::U64(0)]),
        );
        let out = permutation_parity_host(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_i64(&out, "parity"), Some(-1));
    }

    #[test]
    fn wave14_pack_unpack_roundtrip() {
        let mut m = BTreeMap::new();
        m.insert("step".into(), Value::F64(1.5));
        m.insert("comp".into(), Value::F64(-0.25));
        let packed = pack_f32_pair_host(&Value::Record(m), span()).unwrap();
        let p = args::rec_u64(&packed, "packed").unwrap();
        let mut u = BTreeMap::new();
        u.insert("packed".into(), Value::U64(p));
        let out = unpack_f32_pair_host(&Value::Record(u), span()).unwrap();
        assert!((args::rec_f64(&out, "step").unwrap() - 1.5).abs() < 1e-6);
        assert!((args::rec_f64(&out, "comp").unwrap() + 0.25).abs() < 1e-6);
    }

    #[test]
    fn wave14_top_k_order() {
        let mut m = BTreeMap::new();
        m.insert(
            "activation".into(),
            args::f64_list_value(vec![0.1, 0.9, 0.4]),
        );
        m.insert("k".into(), Value::U64(2));
        let out = top_k_host(&Value::Record(m), span()).unwrap();
        let Value::List(xs) = args::rec(&out, "indices").unwrap() else {
            panic!("indices");
        };
        assert_eq!(xs[0], Value::U64(1));
        assert_eq!(xs[1], Value::U64(2));
    }
}
