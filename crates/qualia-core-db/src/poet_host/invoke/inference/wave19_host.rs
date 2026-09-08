//! Wave-19 Host binds: Inference activation / norm numerics.
//!
//! Pure CPU paths from `solvers::activation` (MLP / transformer primitives used by
//! specialized_libs ML inference) — no forge / `caps()` / CUDA.

use super::super::args;
use crate::solvers::activation;
use vibe::{Diagnostic, Span, Value};

const MAX_LEN: usize = 65_536;

fn take_x(args_v: &Value, span: Span, what: &str) -> Result<Vec<f64>, Diagnostic> {
    let x = args::rec_f64_list(args_v, "x")
        .ok_or_else(|| args::bad(span, format!("{what} needs x: [f64]")))?;
    if x.len() > MAX_LEN {
        return Err(args::bad(
            span,
            format!("{what}: x length {} exceeds {MAX_LEN}", x.len()),
        ));
    }
    Ok(x)
}

fn out_list(x: Vec<f64>) -> Value {
    args::record([("out", args::f64_list_value(x))])
}

/// `Inference.relu` — `{ x: [f64] }` → `{ out: [f64] }`.
pub fn relu_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut x = take_x(args_v, span, "Inference.relu")?;
    activation::relu(&mut x);
    Ok(out_list(x))
}

/// `Inference.sigmoid` — `{ x: [f64] }` → `{ out: [f64] }`.
pub fn sigmoid_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut x = take_x(args_v, span, "Inference.sigmoid")?;
    activation::sigmoid(&mut x);
    Ok(out_list(x))
}

/// `Inference.gelu` — `{ x: [f64] }` → `{ out: [f64] }`.
pub fn gelu_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut x = take_x(args_v, span, "Inference.gelu")?;
    activation::gelu(&mut x);
    Ok(out_list(x))
}

/// `Inference.softmax` — `{ x: [f64] }` → `{ out: [f64] }` (sums to 1).
pub fn softmax_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut x = take_x(args_v, span, "Inference.softmax")?;
    activation::softmax(&mut x);
    Ok(out_list(x))
}

/// `Inference.rms_norm` — `{ x, weight: [f64], eps? }` → `{ out: [f64] }`.
pub fn rms_norm_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut x = take_x(args_v, span, "Inference.rms_norm")?;
    let weight = args::rec_f64_list(args_v, "weight")
        .ok_or_else(|| args::bad(span, "Inference.rms_norm needs weight: [f64]"))?;
    if weight.len() > MAX_LEN {
        return Err(args::bad(span, "Inference.rms_norm: weight too long"));
    }
    let eps = args::rec_f64(args_v, "eps").unwrap_or(1e-6);
    activation::rms_norm(&mut x, &weight, eps);
    Ok(out_list(x))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn wave19_relu_clamps_negative() {
        let mut m = BTreeMap::new();
        m.insert("x".into(), args::f64_list_value([-1.0, 0.0, 2.0]));
        let out = relu_host(&Value::Record(m), span()).unwrap();
        let v = args::rec_f64_list(&out, "out").unwrap();
        assert_eq!(v, vec![0.0, 0.0, 2.0]);
    }

    #[test]
    fn wave19_sigmoid_half_at_zero() {
        let mut m = BTreeMap::new();
        m.insert("x".into(), args::f64_list_value([0.0]));
        let out = sigmoid_host(&Value::Record(m), span()).unwrap();
        let v = args::rec_f64_list(&out, "out").unwrap();
        assert!((v[0] - 0.5).abs() < 1e-12);
    }

    #[test]
    fn wave19_gelu_zero() {
        let mut m = BTreeMap::new();
        m.insert("x".into(), args::f64_list_value([0.0]));
        let out = gelu_host(&Value::Record(m), span()).unwrap();
        let v = args::rec_f64_list(&out, "out").unwrap();
        assert!(v[0].abs() < 1e-12);
    }

    #[test]
    fn wave19_softmax_sums_to_one() {
        let mut m = BTreeMap::new();
        m.insert("x".into(), args::f64_list_value([1.0, 2.0, 3.0]));
        let out = softmax_host(&Value::Record(m), span()).unwrap();
        let v = args::rec_f64_list(&out, "out").unwrap();
        let sum: f64 = v.iter().sum();
        assert!((sum - 1.0).abs() < 1e-12);
        assert!(v[2] > v[1] && v[1] > v[0]);
    }

    #[test]
    fn wave19_rms_norm_unit_weight() {
        let mut m = BTreeMap::new();
        m.insert("x".into(), args::f64_list_value([3.0, -4.0]));
        m.insert("weight".into(), args::f64_list_value([1.0, 1.0]));
        m.insert("eps".into(), Value::F64(0.0));
        let out = rms_norm_host(&Value::Record(m), span()).unwrap();
        let v = args::rec_f64_list(&out, "out").unwrap();
        // mean(x²)=12.5, rms=√12.5=√(25/2)=5/√2 → [3,−4]/rms = [0.6√2, −0.8√2]
        let rms = (12.5_f64).sqrt();
        assert!((v[0] - 3.0 / rms).abs() < 1e-12);
        assert!((v[1] + 4.0 / rms).abs() < 1e-12);
    }
}
