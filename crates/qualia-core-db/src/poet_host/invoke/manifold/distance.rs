//! 10-D tensor distance — the geometric kernel behind 3D/4D manifolds.

use super::super::args;
use crate::tensor::Tensor10D;
use vibe::{Diagnostic, Span, Value};

pub fn distance(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = tensor(args_v, "a", span)?;
    let b = tensor(args_v, "b", span)?;
    Ok(Value::F64(a.full_distance(&b) as f64))
}

pub fn from_list(xs: &[f64]) -> Option<Tensor10D> {
    if xs.len() != 10 {
        return None;
    }
    Some(Tensor10D::new(
        xs[0] as f32,
        xs[1] as f32,
        xs[2] as f32,
        xs[3] as f32,
        xs[4] as f32,
        xs[5] as f32,
        xs[6] as f32,
        xs[7] as f32,
        xs[8] as f32,
        xs[9] as f32,
    ))
}

fn tensor(args_v: &Value, key: &str, span: Span) -> Result<Tensor10D, Diagnostic> {
    let xs = args::rec(args_v, key)
        .and_then(args::f64s)
        .ok_or_else(|| args::bad(span, format!("{key} needs [q,v,w,x,y,z,t,α,μ,σ]")))?;
    from_list(&xs).ok_or_else(|| args::bad(span, format!("{key} needs 10 numbers")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn origin_to_unit_x() {
        let mut m = BTreeMap::new();
        m.insert(
            "a".into(),
            args::f64_list_value([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
        );
        m.insert(
            "b".into(),
            args::f64_list_value([0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
        );
        match distance(&Value::Record(m), Span { start: 0, end: 0 }).unwrap() {
            Value::F64(d) => assert!((d - 1.0).abs() < 1e-5),
            other => panic!("{other:?}"),
        }
    }
}
