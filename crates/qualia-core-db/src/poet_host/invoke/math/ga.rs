//! Geometric-algebra 3-vector utils (dot / cross / normalize / angle).

use super::super::args;
use crate::solvers::geometric_algebra::utils::{
    angle_between_vectors, cross_product, dot_product, normalize_vector,
};
use vibe::{Diagnostic, Span, Value};

pub fn dot(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = vec3(args_v, "a", span)?;
    let b = vec3(args_v, "b", span)?;
    Ok(Value::F64(dot_product(&a, &b) as f64))
}

/// `GeometricAlgebra.cross_product` — Args: `{ a, b }` each `[x,y,z]`. Out: `[x,y,z]`.
pub fn cross_product_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = vec3(args_v, "a", span)?;
    let b = vec3(args_v, "b", span)?;
    let c = cross_product(&a, &b);
    Ok(args::f64_list_value(vec![
        c[0] as f64,
        c[1] as f64,
        c[2] as f64,
    ]))
}

/// `GeometricAlgebra.normalize_vector` — Args: `{ v: [x,y,z] }`. Out: `[x,y,z]`.
pub fn normalize_vector_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let v = vec3(args_v, "v", span)?;
    let n = normalize_vector(&v);
    Ok(args::f64_list_value(vec![
        n[0] as f64,
        n[1] as f64,
        n[2] as f64,
    ]))
}

/// `GeometricAlgebra.angle_between_vectors` — Args: `{ a, b }`. Out: `{ radians }`.
pub fn angle_between_vectors_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = vec3(args_v, "a", span)?;
    let b = vec3(args_v, "b", span)?;
    Ok(args::record([(
        "radians",
        Value::F64(angle_between_vectors(&a, &b) as f64),
    )]))
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

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn unit_dot() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), args::f64_list_value(vec![1.0, 0.0, 0.0]));
        m.insert("b".into(), args::f64_list_value(vec![1.0, 0.0, 0.0]));
        assert_eq!(dot(&Value::Record(m), span()).unwrap(), Value::F64(1.0));
    }

    #[test]
    fn wave16_cross_product_basis() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), args::f64_list_value(vec![1.0, 0.0, 0.0]));
        m.insert("b".into(), args::f64_list_value(vec![0.0, 1.0, 0.0]));
        let out = cross_product_host(&Value::Record(m), span()).unwrap();
        let c = args::f64s(&out).unwrap();
        assert!((c[0] - 0.0).abs() < 1e-6);
        assert!((c[1] - 0.0).abs() < 1e-6);
        assert!((c[2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn wave16_normalize_vector_345() {
        let mut m = BTreeMap::new();
        m.insert("v".into(), args::f64_list_value(vec![3.0, 4.0, 0.0]));
        let out = normalize_vector_host(&Value::Record(m), span()).unwrap();
        let n = args::f64s(&out).unwrap();
        let mag = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        assert!((mag - 1.0).abs() < 1e-5);
    }

    #[test]
    fn wave16_angle_between_orthogonal() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), args::f64_list_value(vec![1.0, 0.0, 0.0]));
        m.insert("b".into(), args::f64_list_value(vec![0.0, 1.0, 0.0]));
        let out = angle_between_vectors_host(&Value::Record(m), span()).unwrap();
        let r = args::rec_f64(&out, "radians").unwrap();
        assert!((r - std::f64::consts::FRAC_PI_2).abs() < 1e-5);
    }
}
