//! Wave-24 Host binds: width coreset, duality, convexity, boolean areas.
//!
//! Pure CPU paths from `specialized_libs::computational_geometry`.
//! No forge / `caps()` / CUDA / GPU.

use super::super::args;
use crate::specialized_libs::computational_geometry::{
    boolean_difference_area, boolean_intersection_area, boolean_union_area, dual_point_to_line,
    dual_round_trip, is_convex_polygon, point_in_or_on_polygon, width_coreset, Point2,
};
use vibe::{Diagnostic, Span, Value};

const MAX_POINTS: usize = 256;

fn parse_point2_arg(args_v: &Value, key: &str, span: Span, what: &str) -> Result<Point2, Diagnostic> {
    let coords = args::rec_f64_list(args_v, key)
        .ok_or_else(|| args::bad(span, format!("{what} needs {key}: [f64; 2]")))?;
    if coords.len() < 2 {
        return Err(args::bad(span, format!("{what}: {key} needs ≥2 coords")));
    }
    Ok(Point2::new(coords[0], coords[1]))
}

fn parse_point2_list(args_v: &Value, key: &str, span: Span, what: &str) -> Result<Vec<Point2>, Diagnostic> {
    let v = args::rec(args_v, key)
        .ok_or_else(|| args::bad(span, format!("{what} needs {key}: [[f64;2]; N]")))?;
    let list = args::list(v).ok_or_else(|| args::bad(span, format!("{what}: {key} must be list")))?;
    if list.is_empty() || list.len() > MAX_POINTS {
        return Err(args::bad(
            span,
            format!("{what}: {key} must have 1..={MAX_POINTS} points"),
        ));
    }
    let mut pts = Vec::with_capacity(list.len());
    for item in list {
        let coords = args::f64s(item)
            .ok_or_else(|| args::bad(span, format!("{what}: each point must be [f64;2]")))?;
        if coords.len() < 2 {
            return Err(args::bad(span, format!("{what}: each point needs ≥2 coords")));
        }
        pts.push(Point2::new(coords[0], coords[1]));
    }
    Ok(pts)
}

/// `ComputationalGeometry.width_coreset` — Dudley directional-width coreset.
/// Args: `{ points: [[f64;2]], epsilon?: f64 }`. Out: `{ count: u64, indices: [u64] }`.
pub fn width_coreset_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let points = parse_point2_list(args_v, "points", span, "width_coreset")?;
    let epsilon = args::rec_f64(args_v, "epsilon").unwrap_or(0.25);
    if epsilon <= 0.0 {
        return Err(args::bad(span, "width_coreset: epsilon must be > 0"));
    }
    let core = width_coreset(&points, epsilon)
        .ok_or_else(|| args::bad(span, "width_coreset: empty or failed"))?;
    Ok(args::record([
        ("count", Value::U64(core.indices.len() as u64)),
        (
            "indices",
            Value::List(
                core.indices
                    .into_iter()
                    .map(|i| Value::U64(i as u64))
                    .collect(),
            ),
        ),
    ]))
}

/// `ComputationalGeometry.dual_point_to_line` — point (a,b) → line y = a·x − b.
/// Args: `{ point: [f64;2] }`. Out: `{ slope, intercept, is_vertical }`.
pub fn dual_point_to_line_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = parse_point2_arg(args_v, "point", span, "dual_point_to_line")?;
    let line = dual_point_to_line(p);
    Ok(args::record([
        ("slope", Value::F64(line.slope)),
        ("intercept", Value::F64(line.intercept)),
        ("is_vertical", Value::Bool(line.is_vertical)),
    ]))
}

/// `ComputationalGeometry.dual_round_trip` — dual(dual(p)) ≈ p.
/// Args: `{ point: [f64;2] }`. Out: `{ x, y }`.
pub fn dual_round_trip_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = parse_point2_arg(args_v, "point", span, "dual_round_trip")?;
    let q = dual_round_trip(p);
    Ok(args::record([("x", Value::F64(q.x)), ("y", Value::F64(q.y))]))
}

/// `ComputationalGeometry.is_convex_polygon` — CCW/CW convexity of a simple polygon.
/// Args: `{ vertices: [[f64;2]] }`. Out: `{ convex: bool }`.
pub fn is_convex_polygon_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let vertices = parse_point2_list(args_v, "vertices", span, "is_convex_polygon")?;
    Ok(args::record([(
        "convex",
        Value::Bool(is_convex_polygon(&vertices)),
    )]))
}

/// `ComputationalGeometry.point_in_or_on_polygon` — interior or boundary.
/// Args: `{ point: [f64;2], polygon: [[f64;2]] }`. Out: `{ inside: bool }`.
pub fn point_in_or_on_polygon_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let point = parse_point2_arg(args_v, "point", span, "point_in_or_on_polygon")?;
    let polygon = parse_point2_list(args_v, "polygon", span, "point_in_or_on_polygon")?;
    Ok(args::record([(
        "inside",
        Value::Bool(point_in_or_on_polygon(point, &polygon)),
    )]))
}

/// `ComputationalGeometry.boolean_union_area` — |A ∪ B|.
/// Args: `{ a: [[f64;2]], b: [[f64;2]] }`. Out: `{ area: f64 }`.
pub fn boolean_union_area_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = parse_point2_list(args_v, "a", span, "boolean_union_area")?;
    let b = parse_point2_list(args_v, "b", span, "boolean_union_area")?;
    let area = boolean_union_area(&a, &b)
        .map_err(|e| args::bad(span, format!("boolean_union_area: {e:?}")))?;
    Ok(args::record([("area", Value::F64(area))]))
}

/// `ComputationalGeometry.boolean_intersection_area` — |A ∩ B|.
/// Args: `{ a: [[f64;2]], b: [[f64;2]] }`. Out: `{ area: f64 }`.
pub fn boolean_intersection_area_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = parse_point2_list(args_v, "a", span, "boolean_intersection_area")?;
    let b = parse_point2_list(args_v, "b", span, "boolean_intersection_area")?;
    let area = boolean_intersection_area(&a, &b)
        .map_err(|e| args::bad(span, format!("boolean_intersection_area: {e:?}")))?;
    Ok(args::record([("area", Value::F64(area))]))
}

/// `ComputationalGeometry.boolean_difference_area` — |A \\ B|.
/// Args: `{ a: [[f64;2]], b: [[f64;2]] }`. Out: `{ area: f64 }`.
pub fn boolean_difference_area_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = parse_point2_list(args_v, "a", span, "boolean_difference_area")?;
    let b = parse_point2_list(args_v, "b", span, "boolean_difference_area")?;
    let area = boolean_difference_area(&a, &b)
        .map_err(|e| args::bad(span, format!("boolean_difference_area: {e:?}")))?;
    Ok(args::record([("area", Value::F64(area))]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    fn pts(pairs: &[[f64; 2]]) -> Value {
        Value::List(pairs.iter().map(|p| args::f64_list_value(*p)).collect())
    }

    fn square() -> Value {
        pts(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]])
    }

    #[test]
    fn wave24_width_coreset_square() {
        let mut m = BTreeMap::new();
        m.insert("points".into(), square());
        m.insert("epsilon".into(), Value::F64(0.5));
        let out = width_coreset_host(&Value::Record(m), span()).unwrap();
        assert!(args::rec_u64(&out, "count").unwrap() >= 1);
    }

    #[test]
    fn wave24_dual_point_to_line_origin() {
        let mut m = BTreeMap::new();
        m.insert("point".into(), args::f64_list_value([2.0, 3.0]));
        let out = dual_point_to_line_host(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "slope").unwrap() - 2.0).abs() < 1e-12);
        assert!((args::rec_f64(&out, "intercept").unwrap() + 3.0).abs() < 1e-12);
        assert_eq!(args::rec_bool(&out, "is_vertical"), Some(false));
    }

    #[test]
    fn wave24_dual_round_trip_point() {
        let mut m = BTreeMap::new();
        m.insert("point".into(), args::f64_list_value([1.5, -2.0]));
        let out = dual_round_trip_host(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "x").unwrap() - 1.5).abs() < 1e-9);
        assert!((args::rec_f64(&out, "y").unwrap() + 2.0).abs() < 1e-9);
    }

    #[test]
    fn wave24_is_convex_polygon_square() {
        let mut m = BTreeMap::new();
        m.insert("vertices".into(), square());
        let out = is_convex_polygon_host(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_bool(&out, "convex"), Some(true));
    }

    #[test]
    fn wave24_point_in_or_on_polygon_corner() {
        let mut m = BTreeMap::new();
        m.insert("point".into(), args::f64_list_value([0.0, 0.0]));
        m.insert("polygon".into(), square());
        let out = point_in_or_on_polygon_host(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_bool(&out, "inside"), Some(true));
    }

    #[test]
    fn wave24_boolean_union_area_same_square() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), square());
        m.insert("b".into(), square());
        let out = boolean_union_area_host(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "area").unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn wave24_boolean_intersection_area_same_square() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), square());
        m.insert("b".into(), square());
        let out = boolean_intersection_area_host(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "area").unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn wave24_boolean_difference_area_same_square() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), square());
        m.insert("b".into(), square());
        let out = boolean_difference_area_host(&Value::Record(m), span()).unwrap();
        assert!(args::rec_f64(&out, "area").unwrap().abs() < 1e-9);
    }
}
