//! Wave-26 Host binds: math-geometry affine / quaternion / quadratic leftovers.
//!
//! Pure CPU paths from `specialized_libs::computational_geometry`.
//! No forge / `caps()` / CUDA / GPU.

use super::super::args;
use crate::specialized_libs::computational_geometry::{
    barycentric_tetra, frame_to_world, quaternion_slerp, quaternion_to_matrix, schur_complement_2x2,
    separating_plane_aabb, solve_diagonal_quadratic, world_to_frame, AffineFrame3, Point3,
    Quaternion,
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

fn parse_point3_or(
    args_v: &Value,
    key: &str,
    default: Point3,
    span: Span,
    what: &str,
) -> Result<Point3, Diagnostic> {
    match args::rec_f64_list(args_v, key) {
        None => Ok(default),
        Some(coords) if coords.len() < 3 => {
            Err(args::bad(span, format!("{what}: {key} needs ≥3 coords")))
        }
        Some(coords) => Ok(Point3::new(coords[0], coords[1], coords[2])),
    }
}

fn parse_quat_keys(
    args_v: &Value,
    w_key: &str,
    x_key: &str,
    y_key: &str,
    z_key: &str,
    span: Span,
    what: &str,
) -> Result<Quaternion, Diagnostic> {
    Ok(Quaternion {
        w: args::rec_f64(args_v, w_key)
            .ok_or_else(|| args::bad(span, format!("{what} needs {w_key}")))?,
        x: args::rec_f64(args_v, x_key).unwrap_or(0.0),
        y: args::rec_f64(args_v, y_key).unwrap_or(0.0),
        z: args::rec_f64(args_v, z_key).unwrap_or(0.0),
    })
}

fn parse_frame(args_v: &Value, span: Span, what: &str) -> Result<AffineFrame3, Diagnostic> {
    Ok(AffineFrame3 {
        origin: parse_point3_or(args_v, "origin", Point3::new(0.0, 0.0, 0.0), span, what)?,
        e0: parse_point3_or(args_v, "e0", Point3::new(1.0, 0.0, 0.0), span, what)?,
        e1: parse_point3_or(args_v, "e1", Point3::new(0.0, 1.0, 0.0), span, what)?,
        e2: parse_point3_or(args_v, "e2", Point3::new(0.0, 0.0, 1.0), span, what)?,
    })
}

fn point3_record(p: Point3) -> Value {
    args::record([
        ("x", Value::F64(p.x)),
        ("y", Value::F64(p.y)),
        ("z", Value::F64(p.z)),
    ])
}

fn quat_record(q: Quaternion) -> Value {
    args::record([
        ("w", Value::F64(q.w)),
        ("x", Value::F64(q.x)),
        ("y", Value::F64(q.y)),
        ("z", Value::F64(q.z)),
    ])
}

/// `ComputationalGeometry.frame_to_world` — local coords → world point.
/// Args: `{ coords: [f64;3], origin?: [f64;3], e0?, e1?, e2? }`. Out: `{ x, y, z }`.
pub fn frame_to_world_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let coords = args::rec_f64_list(args_v, "coords")
        .ok_or_else(|| args::bad(span, "frame_to_world needs coords: [f64; 3]"))?;
    if coords.len() < 3 {
        return Err(args::bad(span, "frame_to_world: coords needs ≥3 values"));
    }
    let frame = parse_frame(args_v, span, "frame_to_world")?;
    Ok(point3_record(frame_to_world(frame, [coords[0], coords[1], coords[2]])))
}

/// `ComputationalGeometry.world_to_frame` — world point → local coords.
/// Args: `{ point: [f64;3], origin?, e0?, e1?, e2? }`. Out: `{ u, v, w }`.
pub fn world_to_frame_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = parse_point3_arg(args_v, "point", span, "world_to_frame")?;
    let frame = parse_frame(args_v, span, "world_to_frame")?;
    let uvw = world_to_frame(frame, p).map_err(|e| args::bad(span, format!("world_to_frame: {e:?}")))?;
    Ok(args::record([
        ("u", Value::F64(uvw[0])),
        ("v", Value::F64(uvw[1])),
        ("w", Value::F64(uvw[2])),
    ]))
}

/// `ComputationalGeometry.barycentric_tetra` — barycentric coords in tet ABCD.
/// Args: `{ p, a, b, c, d }` each `[f64;3]`. Out: `{ w0, w1, w2, w3 }`.
pub fn barycentric_tetra_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = parse_point3_arg(args_v, "p", span, "barycentric_tetra")?;
    let a = parse_point3_arg(args_v, "a", span, "barycentric_tetra")?;
    let b = parse_point3_arg(args_v, "b", span, "barycentric_tetra")?;
    let c = parse_point3_arg(args_v, "c", span, "barycentric_tetra")?;
    let d = parse_point3_arg(args_v, "d", span, "barycentric_tetra")?;
    let w = barycentric_tetra(p, a, b, c, d)
        .map_err(|e| args::bad(span, format!("barycentric_tetra: {e:?}")))?;
    Ok(args::record([
        ("w0", Value::F64(w[0])),
        ("w1", Value::F64(w[1])),
        ("w2", Value::F64(w[2])),
        ("w3", Value::F64(w[3])),
    ]))
}

/// `ComputationalGeometry.quaternion_slerp` — spherical lerp of unit quaternions.
/// Args: `{ a_w, a_x?, a_y?, a_z?, b_w, b_x?, b_y?, b_z?, t }`. Out: `{ w, x, y, z }`.
pub fn quaternion_slerp_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = parse_quat_keys(args_v, "a_w", "a_x", "a_y", "a_z", span, "quaternion_slerp")?;
    let b = parse_quat_keys(args_v, "b_w", "b_x", "b_y", "b_z", span, "quaternion_slerp")?;
    let t = args::rec_f64(args_v, "t").ok_or_else(|| args::bad(span, "quaternion_slerp needs t"))?;
    Ok(quat_record(quaternion_slerp(a, b, t)))
}

/// `ComputationalGeometry.quaternion_to_matrix` — unit quaternion → SO(3) matrix.
/// Args: `{ w, x?, y?, z? }`. Out: `{ rows: [[f64;3]; 3] }`.
pub fn quaternion_to_matrix_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let q = parse_quat_keys(args_v, "w", "x", "y", "z", span, "quaternion_to_matrix")?;
    let m = quaternion_to_matrix(q);
    Ok(args::record([(
        "rows",
        Value::List(
            m.into_iter()
                .map(|row| args::f64_list_value(row))
                .collect(),
        ),
    )]))
}

/// `ComputationalGeometry.solve_diagonal_quadratic` — min ½xᵀDx + ℓ·x, D diagonal.
/// Args: `{ diag: [f64;3], linear: [f64;3] }`. Out: `{ x, objective, residual }`.
pub fn solve_diagonal_quadratic_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let diag = args::rec_f64_list(args_v, "diag")
        .ok_or_else(|| args::bad(span, "solve_diagonal_quadratic needs diag: [f64; 3]"))?;
    let linear = args::rec_f64_list(args_v, "linear")
        .ok_or_else(|| args::bad(span, "solve_diagonal_quadratic needs linear: [f64; 3]"))?;
    if diag.len() < 3 || linear.len() < 3 {
        return Err(args::bad(span, "solve_diagonal_quadratic: diag and linear need ≥3 values"));
    }
    let sol = solve_diagonal_quadratic(
        [diag[0], diag[1], diag[2]],
        [linear[0], linear[1], linear[2]],
    )
    .map_err(|e| args::bad(span, format!("solve_diagonal_quadratic: {e:?}")))?;
    Ok(args::record([
        ("x", args::f64_list_value(sol.x)),
        ("objective", Value::F64(sol.objective)),
        ("residual", Value::F64(sol.stationarity_residual)),
    ]))
}

/// `ComputationalGeometry.schur_complement_2x2` — bᵀ A⁻¹ b for SPD 2×2 A.
/// Args: `{ a00, a01, a11, b: [f64;2] }`. Out: `{ value }`.
pub fn schur_complement_2x2_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a00 = args::rec_f64(args_v, "a00")
        .ok_or_else(|| args::bad(span, "schur_complement_2x2 needs a00"))?;
    let a01 = args::rec_f64(args_v, "a01")
        .ok_or_else(|| args::bad(span, "schur_complement_2x2 needs a01"))?;
    let a11 = args::rec_f64(args_v, "a11")
        .ok_or_else(|| args::bad(span, "schur_complement_2x2 needs a11"))?;
    let b = args::rec_f64_list(args_v, "b")
        .ok_or_else(|| args::bad(span, "schur_complement_2x2 needs b: [f64; 2]"))?;
    if b.len() < 2 {
        return Err(args::bad(span, "schur_complement_2x2: b needs ≥2 values"));
    }
    let value = schur_complement_2x2(a00, a01, a11, [b[0], b[1]])
        .map_err(|e| args::bad(span, format!("schur_complement_2x2: {e:?}")))?;
    Ok(args::record([("value", Value::F64(value))]))
}

/// `ComputationalGeometry.separating_plane_aabb` — AABB vs AABB SAT plane.
/// Args: `{ a_min, a_max, b_min, b_max }` each `[f64;3]`.
/// Out: `{ separated, nx?, ny?, nz?, offset? }`.
pub fn separating_plane_aabb_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a_min = parse_point3_arg(args_v, "a_min", span, "separating_plane_aabb")?;
    let a_max = parse_point3_arg(args_v, "a_max", span, "separating_plane_aabb")?;
    let b_min = parse_point3_arg(args_v, "b_min", span, "separating_plane_aabb")?;
    let b_max = parse_point3_arg(args_v, "b_max", span, "separating_plane_aabb")?;
    match separating_plane_aabb(a_min, a_max, b_min, b_max) {
        Some(plane) => Ok(args::record([
            ("separated", Value::Bool(true)),
            ("nx", Value::F64(plane.normal.x)),
            ("ny", Value::F64(plane.normal.y)),
            ("nz", Value::F64(plane.normal.z)),
            ("offset", Value::F64(plane.offset)),
        ])),
        None => Ok(args::record([("separated", Value::Bool(false))])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn wave26_frame_to_world_identity() {
        let mut m = BTreeMap::new();
        m.insert("coords".into(), args::f64_list_value([1.0, 2.0, 3.0]));
        let out = frame_to_world_host(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "x").unwrap() - 1.0).abs() < 1e-12);
        assert!((args::rec_f64(&out, "y").unwrap() - 2.0).abs() < 1e-12);
        assert!((args::rec_f64(&out, "z").unwrap() - 3.0).abs() < 1e-12);
    }

    #[test]
    fn wave26_world_to_frame_identity() {
        let mut m = BTreeMap::new();
        m.insert("point".into(), args::f64_list_value([4.0, 5.0, 6.0]));
        let out = world_to_frame_host(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "u").unwrap() - 4.0).abs() < 1e-12);
        assert!((args::rec_f64(&out, "v").unwrap() - 5.0).abs() < 1e-12);
        assert!((args::rec_f64(&out, "w").unwrap() - 6.0).abs() < 1e-12);
    }

    #[test]
    fn wave26_barycentric_tetra_vertex_a() {
        let mut m = BTreeMap::new();
        m.insert("p".into(), args::f64_list_value([0.0, 0.0, 0.0]));
        m.insert("a".into(), args::f64_list_value([0.0, 0.0, 0.0]));
        m.insert("b".into(), args::f64_list_value([1.0, 0.0, 0.0]));
        m.insert("c".into(), args::f64_list_value([0.0, 1.0, 0.0]));
        m.insert("d".into(), args::f64_list_value([0.0, 0.0, 1.0]));
        let out = barycentric_tetra_host(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "w0").unwrap() - 1.0).abs() < 1e-12);
        assert!(args::rec_f64(&out, "w1").unwrap().abs() < 1e-12);
        assert!(args::rec_f64(&out, "w2").unwrap().abs() < 1e-12);
        assert!(args::rec_f64(&out, "w3").unwrap().abs() < 1e-12);
    }

    #[test]
    fn wave26_quaternion_slerp_endpoints() {
        let mut m = BTreeMap::new();
        m.insert("a_w".into(), Value::F64(1.0));
        m.insert("b_w".into(), Value::F64(0.0));
        m.insert("b_z".into(), Value::F64(1.0));
        m.insert("t".into(), Value::F64(0.0));
        let out = quaternion_slerp_host(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "w").unwrap() - 1.0).abs() < 1e-9);
        assert!(args::rec_f64(&out, "z").unwrap().abs() < 1e-9);
    }

    #[test]
    fn wave26_quaternion_to_matrix_identity() {
        let mut m = BTreeMap::new();
        m.insert("w".into(), Value::F64(1.0));
        let out = quaternion_to_matrix_host(&Value::Record(m), span()).unwrap();
        let rows = match &out {
            Value::Record(r) => r.get("rows").cloned(),
            _ => None,
        };
        let Value::List(rows) = rows.expect("rows") else {
            panic!("rows not a list");
        };
        let Value::List(r0) = &rows[0] else {
            panic!("row0");
        };
        let Value::F64(m00) = r0[0] else {
            panic!("m00");
        };
        assert!((m00 - 1.0).abs() < 1e-12);
    }

    #[test]
    fn wave26_solve_diagonal_quadratic_unit() {
        let mut m = BTreeMap::new();
        m.insert("diag".into(), args::f64_list_value([2.0, 2.0, 2.0]));
        m.insert("linear".into(), args::f64_list_value([-2.0, -4.0, -6.0]));
        let out = solve_diagonal_quadratic_host(&Value::Record(m), span()).unwrap();
        let x = args::rec_f64_list(&out, "x").unwrap();
        assert!((x[0] - 1.0).abs() < 1e-12);
        assert!((x[1] - 2.0).abs() < 1e-12);
        assert!((x[2] - 3.0).abs() < 1e-12);
        assert!(args::rec_f64(&out, "residual").unwrap().abs() < 1e-12);
    }

    #[test]
    fn wave26_schur_complement_2x2_identity() {
        let mut m = BTreeMap::new();
        m.insert("a00".into(), Value::F64(1.0));
        m.insert("a01".into(), Value::F64(0.0));
        m.insert("a11".into(), Value::F64(1.0));
        m.insert("b".into(), args::f64_list_value([3.0, 4.0]));
        let out = schur_complement_2x2_host(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "value").unwrap() - 25.0).abs() < 1e-12);
    }

    #[test]
    fn wave26_separating_plane_aabb_separated() {
        let mut m = BTreeMap::new();
        m.insert("a_min".into(), args::f64_list_value([0.0, 0.0, 0.0]));
        m.insert("a_max".into(), args::f64_list_value([1.0, 1.0, 1.0]));
        m.insert("b_min".into(), args::f64_list_value([2.0, 0.0, 0.0]));
        m.insert("b_max".into(), args::f64_list_value([3.0, 1.0, 1.0]));
        let out = separating_plane_aabb_host(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_bool(&out, "separated"), Some(true));
        assert!((args::rec_f64(&out, "nx").unwrap() - 1.0).abs() < 1e-12);
    }
}
