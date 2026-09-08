//! Wave-19 Host binds: 3-D point-set numerics (ThreeD lane).
//!
//! Pure CPU paths from `specialized_libs::computational_geometry::point_set_3d`
//! — no forge / `caps()` / CUDA / GPU.

use super::super::args;
use crate::specialized_libs::computational_geometry::{
    average_spacing_3d, local_density_3d, KnnEntry, Point3, MAX_K,
};
use vibe::{Diagnostic, Span, Value};

const MAX_POINTS: usize = 4096;

fn parse_points(args_v: &Value, span: Span, what: &str) -> Result<Vec<Point3>, Diagnostic> {
    let v = args::rec(args_v, "points")
        .ok_or_else(|| args::bad(span, format!("{what} needs points: [[f64;3]; N]")))?;
    let list = args::list(v).ok_or_else(|| args::bad(span, format!("{what}: points must be list")))?;
    if list.len() > MAX_POINTS {
        return Err(args::bad(
            span,
            format!("{what}: at most {MAX_POINTS} points"),
        ));
    }
    let mut pts = Vec::with_capacity(list.len());
    for item in list {
        let coords = args::f64s(item)
            .ok_or_else(|| args::bad(span, format!("{what}: each point must be [f64;3]")))?;
        if coords.len() < 3 {
            return Err(args::bad(span, format!("{what}: each point needs ≥3 coords")));
        }
        pts.push(Point3::new(coords[0], coords[1], coords[2]));
    }
    Ok(pts)
}

/// `ComputationalGeometry.average_spacing_3d` — mean kNN spacing over a point cloud.
/// Args: `{ points: [[f64;3]], k: u64 }`. Out: `{ spacing: f64 }`.
pub fn average_spacing_3d_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let points = parse_points(args_v, span, "average_spacing_3d")?;
    let k = args::rec_u64(args_v, "k").unwrap_or(1) as usize;
    if k == 0 || k >= points.len() || k > MAX_K {
        return Err(args::bad(
            span,
            format!(
                "average_spacing_3d: k={k} invalid for n={} (1..=min(n-1,{MAX_K}))",
                points.len()
            ),
        ));
    }
    let n = points.len();
    let mut knn_buffer = vec![KnnEntry::default(); n * k];
    let mut scratch = vec![KnnEntry::default(); MAX_K + 1];
    let spacing = average_spacing_3d(&points, k, &mut knn_buffer, &mut scratch)
        .map_err(|e| args::bad(span, format!("average_spacing_3d: {e}")))?;
    Ok(args::record([("spacing", Value::F64(spacing))]))
}

/// `ComputationalGeometry.local_density_3d` — per-point kNN volume density.
/// Args: `{ points: [[f64;3]], k: u64 }`. Out: `{ density: [f64] }`.
pub fn local_density_3d_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let points = parse_points(args_v, span, "local_density_3d")?;
    let k = args::rec_u64(args_v, "k").unwrap_or(1) as usize;
    if k == 0 || k >= points.len() || k > MAX_K {
        return Err(args::bad(
            span,
            format!(
                "local_density_3d: k={k} invalid for n={} (1..=min(n-1,{MAX_K}))",
                points.len()
            ),
        ));
    }
    let n = points.len();
    let mut density = vec![0.0f64; n];
    let mut knn_buffer = vec![KnnEntry::default(); n * k];
    let mut scratch = vec![KnnEntry::default(); MAX_K + 1];
    local_density_3d(
        &points,
        k,
        &mut density,
        &mut knn_buffer,
        &mut scratch,
    )
    .map_err(|e| args::bad(span, format!("local_density_3d: {e}")))?;
    Ok(args::record([("density", args::f64_list_value(density))]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    fn unit_cube_corners() -> Value {
        // 8 corners of the unit cube — nearest-neighbour spacing is 1.0 for k=1.
        Value::List(vec![
            args::f64_list_value([0.0, 0.0, 0.0]),
            args::f64_list_value([1.0, 0.0, 0.0]),
            args::f64_list_value([0.0, 1.0, 0.0]),
            args::f64_list_value([0.0, 0.0, 1.0]),
            args::f64_list_value([1.0, 1.0, 0.0]),
            args::f64_list_value([1.0, 0.0, 1.0]),
            args::f64_list_value([0.0, 1.0, 1.0]),
            args::f64_list_value([1.0, 1.0, 1.0]),
        ])
    }

    #[test]
    fn wave19_average_spacing_unit_cube_k1() {
        let mut m = BTreeMap::new();
        m.insert("points".into(), unit_cube_corners());
        m.insert("k".into(), Value::U64(1));
        let out = average_spacing_3d_host(&Value::Record(m), span()).unwrap();
        let s = args::rec_f64(&out, "spacing").unwrap();
        assert!((s - 1.0).abs() < 1e-12);
    }

    #[test]
    fn wave19_local_density_positive() {
        let mut m = BTreeMap::new();
        m.insert("points".into(), unit_cube_corners());
        m.insert("k".into(), Value::U64(1));
        let out = local_density_3d_host(&Value::Record(m), span()).unwrap();
        let d = args::rec_f64_list(&out, "density").unwrap();
        assert_eq!(d.len(), 8);
        assert!(d.iter().all(|&x| x > 0.0 && x.is_finite()));
    }
}
