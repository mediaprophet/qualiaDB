//! Wave-25 Host binds: math-geometry projective / quaternion / hyperplane.
//!
//! Pure CPU paths from `specialized_libs::computational_geometry`.
//! No forge / `caps()` / CUDA / GPU.

use super::super::args;
use crate::specialized_libs::computational_geometry::{
    cross_ratio_1d, householder_reflect, hyperplane_eval, point_from_projective,
    projective_from_point, quaternion_normalize, so3_exp, so3_log, HomogeneousPoint3, Hyperplane3,
    Point3, Quaternion,
};
use vibe::{Diagnostic, Span, Value};

fn parse_point3_arg(args_v: &Value, key: &str, span: Span, what: &str) -> Result<Point3, Diagnostic> {
    let coords = args::rec_f64_list(args_v, key)
        .ok_or_else(|| args::bad(span, format!("{what} needs {key}: [f64; 3]")))?;
    if coords.len() < 3 {
        return Err(args::bad(span, format!("{what}: {key} needs ≥3 coords")));
    }
    Ok(Point3::new(coords[0], coords[1], coords[2]))
}

fn quat_record(q: Quaternion) -> Value {
    args::record([
        ("w", Value::F64(q.w)),
        ("x", Value::F64(q.x)),
        ("y", Value::F64(q.y)),
        ("z", Value::F64(q.z)),
    ])
}

fn point3_record(p: Point3) -> Value {
    args::record([
        ("x", Value::F64(p.x)),
        ("y", Value::F64(p.y)),
        ("z", Value::F64(p.z)),
    ])
}

fn parse_quat(args_v: &Value, span: Span, what: &str) -> Result<Quaternion, Diagnostic> {
    Ok(Quaternion {
        w: args::rec_f64(args_v, "w").ok_or_else(|| args::bad(span, format!("{what} needs w")))?,
        x: args::rec_f64(args_v, "x").unwrap_or(0.0),
        y: args::rec_f64(args_v, "y").unwrap_or(0.0),
        z: args::rec_f64(args_v, "z").unwrap_or(0.0),
    })
}

/// `ComputationalGeometry.cross_ratio_1d` — `(A,C; B,D)` on the line.
/// Args: `{ a, b, c, d }`. Out: `{ value }`.
pub fn cross_ratio_1d_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64(args_v, "a").ok_or_else(|| args::bad(span, "cross_ratio_1d needs a"))?;
    let b = args::rec_f64(args_v, "b").ok_or_else(|| args::bad(span, "cross_ratio_1d needs b"))?;
    let c = args::rec_f64(args_v, "c").ok_or_else(|| args::bad(span, "cross_ratio_1d needs c"))?;
    let d = args::rec_f64(args_v, "d").ok_or_else(|| args::bad(span, "cross_ratio_1d needs d"))?;
    let value = cross_ratio_1d(a, b, c, d)
        .map_err(|e| args::bad(span, format!("cross_ratio_1d: {e:?}")))?;
    Ok(args::record([("value", Value::F64(value))]))
}

/// `ComputationalGeometry.hyperplane_eval` — n·p + offset.
/// Args: `{ normal: [f64;3], offset?: f64, point: [f64;3] }`. Out: `{ value }`.
pub fn hyperplane_eval_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let normal = parse_point3_arg(args_v, "normal", span, "hyperplane_eval")?;
    let point = parse_point3_arg(args_v, "point", span, "hyperplane_eval")?;
    let offset = args::rec_f64(args_v, "offset").unwrap_or(0.0);
    let plane = Hyperplane3 { normal, offset };
    Ok(args::record([("value", Value::F64(hyperplane_eval(plane, point)))]))
}

/// `ComputationalGeometry.householder_reflect` — reflect v in the plane of `normal`.
/// Args: `{ v: [f64;3], normal: [f64;3] }`. Out: `{ x, y, z }`.
pub fn householder_reflect_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let v = parse_point3_arg(args_v, "v", span, "householder_reflect")?;
    let normal = parse_point3_arg(args_v, "normal", span, "householder_reflect")?;
    Ok(point3_record(householder_reflect(v, normal)))
}

/// `ComputationalGeometry.quaternion_normalize` — unit quaternion (identity if zero).
/// Args: `{ w, x?, y?, z? }`. Out: `{ w, x, y, z }`.
pub fn quaternion_normalize_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let q = parse_quat(args_v, span, "quaternion_normalize")?;
    Ok(quat_record(quaternion_normalize(q)))
}

/// `ComputationalGeometry.so3_exp` — axis-angle vector → quaternion.
/// Args: `{ axis_angle: [f64;3] }`. Out: `{ w, x, y, z }`.
pub fn so3_exp_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let axis = parse_point3_arg(args_v, "axis_angle", span, "so3_exp")?;
    Ok(quat_record(so3_exp(axis)))
}

/// `ComputationalGeometry.so3_log` — quaternion → axis-angle vector.
/// Args: `{ w, x?, y?, z? }`. Out: `{ x, y, z }`.
pub fn so3_log_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let q = parse_quat(args_v, span, "so3_log")?;
    Ok(point3_record(so3_log(q)))
}

/// `ComputationalGeometry.projective_from_point` — Euclidean → homogeneous (w=1).
/// Args: `{ point: [f64;3] }`. Out: `{ x, y, z, w }`.
pub fn projective_from_point_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = parse_point3_arg(args_v, "point", span, "projective_from_point")?;
    let h = projective_from_point(p);
    Ok(args::record([
        ("x", Value::F64(h.x)),
        ("y", Value::F64(h.y)),
        ("z", Value::F64(h.z)),
        ("w", Value::F64(h.w)),
    ]))
}

/// `ComputationalGeometry.point_from_projective` — homogeneous → Euclidean.
/// Args: `{ x, y, z, w }`. Out: `{ x, y, z }`.
pub fn point_from_projective_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args_v, "x").ok_or_else(|| args::bad(span, "point_from_projective needs x"))?;
    let y = args::rec_f64(args_v, "y").ok_or_else(|| args::bad(span, "point_from_projective needs y"))?;
    let z = args::rec_f64(args_v, "z").ok_or_else(|| args::bad(span, "point_from_projective needs z"))?;
    let w = args::rec_f64(args_v, "w").ok_or_else(|| args::bad(span, "point_from_projective needs w"))?;
    let p = point_from_projective(HomogeneousPoint3 { x, y, z, w })
        .map_err(|e| args::bad(span, format!("point_from_projective: {e:?}")))?;
    Ok(point3_record(p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn wave25_cross_ratio_1d_harmonic() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), Value::F64(0.0));
        m.insert("b".into(), Value::F64(1.0));
        m.insert("c".into(), Value::F64(2.0));
        m.insert("d".into(), Value::F64(3.0));
        let out = cross_ratio_1d_host(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "value").unwrap() - 4.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn wave25_hyperplane_eval_xy_plane() {
        let mut m = BTreeMap::new();
        m.insert("normal".into(), args::f64_list_value([0.0, 0.0, 1.0]));
        m.insert("offset".into(), Value::F64(0.0));
        m.insert("point".into(), args::f64_list_value([0.0, 0.0, 1.0]));
        let out = hyperplane_eval_host(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "value").unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn wave25_householder_reflect_z() {
        let mut m = BTreeMap::new();
        m.insert("v".into(), args::f64_list_value([0.0, 0.0, 1.0]));
        m.insert("normal".into(), args::f64_list_value([0.0, 0.0, 1.0]));
        let out = householder_reflect_host(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "z").unwrap() + 1.0).abs() < 1e-12);
    }

    #[test]
    fn wave25_quaternion_normalize_scaled_identity() {
        let mut m = BTreeMap::new();
        m.insert("w".into(), Value::F64(2.0));
        let out = quaternion_normalize_host(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "w").unwrap() - 1.0).abs() < 1e-12);
        assert!(args::rec_f64(&out, "x").unwrap().abs() < 1e-12);
    }

    #[test]
    fn wave25_so3_exp_zero() {
        let mut m = BTreeMap::new();
        m.insert("axis_angle".into(), args::f64_list_value([0.0, 0.0, 0.0]));
        let out = so3_exp_host(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "w").unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn wave25_so3_log_identity() {
        let mut m = BTreeMap::new();
        m.insert("w".into(), Value::F64(1.0));
        let out = so3_log_host(&Value::Record(m), span()).unwrap();
        assert!(args::rec_f64(&out, "x").unwrap().abs() < 1e-12);
        assert!(args::rec_f64(&out, "y").unwrap().abs() < 1e-12);
        assert!(args::rec_f64(&out, "z").unwrap().abs() < 1e-12);
    }

    #[test]
    fn wave25_projective_from_point_w_one() {
        let mut m = BTreeMap::new();
        m.insert("point".into(), args::f64_list_value([1.0, 2.0, 3.0]));
        let out = projective_from_point_host(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "w").unwrap() - 1.0).abs() < 1e-12);
        assert!((args::rec_f64(&out, "x").unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn wave25_point_from_projective_divide() {
        let mut m = BTreeMap::new();
        m.insert("x".into(), Value::F64(2.0));
        m.insert("y".into(), Value::F64(4.0));
        m.insert("z".into(), Value::F64(6.0));
        m.insert("w".into(), Value::F64(2.0));
        let out = point_from_projective_host(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "x").unwrap() - 1.0).abs() < 1e-12);
        assert!((args::rec_f64(&out, "y").unwrap() - 2.0).abs() < 1e-12);
        assert!((args::rec_f64(&out, "z").unwrap() - 3.0).abs() < 1e-12);
    }
}
