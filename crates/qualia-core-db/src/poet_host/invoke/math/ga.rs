//! Geometric-algebra 3-vector dot product.

use super::super::args;
use crate::solvers::geometric_algebra::utils::dot_product;
use poet_vibe::{Diagnostic, Span, Value};

pub fn dot(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = vec3(args_v, "a", span)?;
    let b = vec3(args_v, "b", span)?;
    Ok(Value::F64(dot_product(&a, &b) as f64))
}

fn vec3(args_v: &Value, key: &str, span: Span) -> Result<[f32; 3], Diagnostic> {
    let xs = args::rec(args_v, key)
        .and_then(args::f64s)
        .ok_or_else(|| args::bad(span, format!("{key} needs [x,y,z]")))?;
    if xs.len() != 3 {
        return Err(args::bad(span, format!("{key} needs three numbers")));
    }
    Ok([xs[0] as f32, xs[1] as f32, xs[2] as f32])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn unit_dot() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), args::f64_list_value(vec![1.0, 0.0, 0.0]));
        m.insert("b".into(), args::f64_list_value(vec![1.0, 0.0, 0.0]));
        assert_eq!(
            dot(&Value::Record(m), Span { start: 0, end: 0 }).unwrap(),
            Value::F64(1.0)
        );
    }
}
