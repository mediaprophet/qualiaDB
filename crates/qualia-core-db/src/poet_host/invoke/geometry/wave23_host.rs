//! Wave-23 Host binds: insphere, ham-sandwich, enclosing disk, polygon
//! predicates, Minkowski sum, nearest segment-site.
//!
//! Pure CPU paths from `specialized_libs::computational_geometry`.
//! No forge / `caps()` / CUDA / GPU.

use super::super::args;
use crate::specialized_libs::computational_geometry::{
    ham_sandwich_cut, insphere, minkowski_sum_convex, point_in_polygon, polygon_area,
    polygon_signed_area, smallest_enclosing_disk, Point2, Point3, Sign,
};
use crate::specialized_libs::computational_geometry::voronoi_variants::{
    nearest_segment_site, SegmentSite,
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

fn parse_point3_arg(args_v: &Value, key: &str, span: Span, what: &str) -> Result<Point3, Diagnostic> {
    let coords = args::rec_f64_list(args_v, key)
        .ok_or_else(|| args::bad(span, format!("{what} needs {key}: [f64; 3]")))?;
    if coords.len() < 3 {
        return Err(args::bad(span, format!("{what}: {key} needs ≥3 coords")));
    }
    Ok(Point3::new(coords[0], coords[1], coords[2]))
}

fn parse_point2_list(args_v: &Value, key: &str, span: Span, what: &str) -> Result<Vec<Point2>, Diagnostic> {
    let v = args::rec(args_v, key)
        .ok_or_else(|| args::bad(span, format!("{what} needs {key}: [[f64;2]; N]")))?;
    parse_point2_list_value(v, span, what, key)
}

fn parse_point2_list_value(v: &Value, span: Span, what: &str, key: &str) -> Result<Vec<Point2>, Diagnostic> {
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

fn sign_i64(sign: Sign) -> i64 {
    match sign {
        Sign::Negative => -1,
        Sign::Zero => 0,
        Sign::Positive => 1,
    }
}

/// `ComputationalGeometry.insphere` — exact in-sphere predicate of e vs tetra abcd.
/// Args: `{ a,b,c,d,e: [f64; 3] }`. Out: `{ sign: i64 }`.
///
/// For a positively oriented tet, −1 = inside, 0 = on, +1 = outside
/// (library convention in `insphere.rs`).
pub fn insphere_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = parse_point3_arg(args_v, "a", span, "insphere")?;
    let b = parse_point3_arg(args_v, "b", span, "insphere")?;
    let c = parse_point3_arg(args_v, "c", span, "insphere")?;
    let d = parse_point3_arg(args_v, "d", span, "insphere")?;
    let e = parse_point3_arg(args_v, "e", span, "insphere")?;
    Ok(args::record([(
        "sign",
        Value::I64(sign_i64(insphere(a, b, c, d, e))),
    )]))
}

/// `ComputationalGeometry.ham_sandwich_cut` — line that bisects two planar sets.
/// Args: `{ set_a: [[f64;2]], set_b: [[f64;2]] }`. Out: `{ px, py, dx, dy }`.
pub fn ham_sandwich_cut_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let set_a = parse_point2_list(args_v, "set_a", span, "ham_sandwich_cut")?;
    let set_b = parse_point2_list(args_v, "set_b", span, "ham_sandwich_cut")?;
    let cut = ham_sandwich_cut(&set_a, &set_b)
        .ok_or_else(|| args::bad(span, "ham_sandwich_cut: no bisecting line"))?;
    Ok(args::record([
        ("px", Value::F64(cut.point.x)),
        ("py", Value::F64(cut.point.y)),
        ("dx", Value::F64(cut.dir.x)),
        ("dy", Value::F64(cut.dir.y)),
    ]))
}

/// `ComputationalGeometry.smallest_enclosing_disk` — Welzl disk.
/// Args: `{ points: [[f64;2]], seed?: u64 }`. Out: `{ cx, cy, radius, support: [u64] }`.
pub fn smallest_enclosing_disk_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let points = parse_point2_list(args_v, "points", span, "smallest_enclosing_disk")?;
    let seed = args::rec_u64(args_v, "seed").unwrap_or(1);
    let disk = smallest_enclosing_disk(&points, seed)
        .map_err(|e| args::bad(span, format!("smallest_enclosing_disk: {e:?}")))?;
    Ok(args::record([
        ("cx", Value::F64(disk.disk.center.x)),
        ("cy", Value::F64(disk.disk.center.y)),
        ("radius", Value::F64(disk.disk.radius)),
        (
            "support",
            Value::List(
                disk.support
                    .into_iter()
                    .map(|i| Value::U64(i as u64))
                    .collect(),
            ),
        ),
    ]))
}

/// `ComputationalGeometry.polygon_signed_area` — twice-area / 2 signed area.
/// Args: `{ vertices: [[f64;2]] }`. Out: `{ area: f64 }`.
pub fn polygon_signed_area_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let vertices = parse_point2_list(args_v, "vertices", span, "polygon_signed_area")?;
    Ok(args::record([(
        "area",
        Value::F64(polygon_signed_area(&vertices)),
    )]))
}

/// `ComputationalGeometry.polygon_area` — absolute polygon area.
/// Args: `{ vertices: [[f64;2]] }`. Out: `{ area: f64 }`.
pub fn polygon_area_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let vertices = parse_point2_list(args_v, "vertices", span, "polygon_area")?;
    Ok(args::record([("area", Value::F64(polygon_area(&vertices)))]))
}

/// `ComputationalGeometry.point_in_polygon` — winding / even-odd interior test.
/// Args: `{ point: [f64;2], polygon: [[f64;2]] }`. Out: `{ inside: bool }`.
pub fn point_in_polygon_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let point = parse_point2_arg(args_v, "point", span, "point_in_polygon")?;
    let polygon = parse_point2_list(args_v, "polygon", span, "point_in_polygon")?;
    Ok(args::record([(
        "inside",
        Value::Bool(point_in_polygon(point, &polygon)),
    )]))
}

/// `ComputationalGeometry.minkowski_sum_convex` — convex Minkowski sum vertices.
/// Args: `{ a: [[f64;2]], b: [[f64;2]] }`. Out: `{ vertices: [[f64;2]], count: u64 }`.
pub fn minkowski_sum_convex_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = parse_point2_list(args_v, "a", span, "minkowski_sum_convex")?;
    let b = parse_point2_list(args_v, "b", span, "minkowski_sum_convex")?;
    let sum = minkowski_sum_convex(&a, &b);
    let verts: Vec<Value> = sum
        .iter()
        .map(|p| args::f64_list_value([p.x, p.y]))
        .collect();
    Ok(args::record([
        ("vertices", Value::List(verts)),
        ("count", Value::U64(sum.len() as u64)),
    ]))
}

/// `ComputationalGeometry.nearest_segment_site` — nearest segment-site index.
/// Args: `{ segments: [[[f64;2],[f64;2]]], q: [f64;2] }`. Out: `{ index: u64 }`.
pub fn nearest_segment_site_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let q = parse_point2_arg(args_v, "q", span, "nearest_segment_site")?;
    let v = args::rec(args_v, "segments").ok_or_else(|| {
        args::bad(span, "nearest_segment_site needs segments: [[[f64;2],[f64;2]]]")
    })?;
    let list = args::list(v)
        .ok_or_else(|| args::bad(span, "nearest_segment_site: segments must be list"))?;
    if list.is_empty() || list.len() > MAX_POINTS {
        return Err(args::bad(
            span,
            format!("nearest_segment_site: 1..={MAX_POINTS} segments"),
        ));
    }
    let mut segs = Vec::with_capacity(list.len());
    for (i, item) in list.iter().enumerate() {
        let pair = args::list(item).ok_or_else(|| {
            args::bad(span, "nearest_segment_site: each segment is [[x,y],[x,y]]")
        })?;
        if pair.len() < 2 {
            return Err(args::bad(span, "nearest_segment_site: each segment needs 2 points"));
        }
        let a = args::f64s(&pair[0])
            .ok_or_else(|| args::bad(span, "nearest_segment_site: segment a needs [f64;2]"))?;
        let b = args::f64s(&pair[1])
            .ok_or_else(|| args::bad(span, "nearest_segment_site: each segment b needs [f64;2]"))?;
        if a.len() < 2 || b.len() < 2 {
            return Err(args::bad(span, "nearest_segment_site: endpoints need ≥2 coords"));
        }
        segs.push(SegmentSite {
            a: Point2::new(a[0], a[1]),
            b: Point2::new(b[0], b[1]),
            index: i as u32,
        });
    }
    Ok(args::record([(
        "index",
        Value::U64(nearest_segment_site(&segs, q) as u64),
    )]))
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

    #[test]
    fn wave23_insphere_origin_inside_tetra() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), args::f64_list_value([0.0, 0.0, 0.0]));
        m.insert("b".into(), args::f64_list_value([1.0, 0.0, 0.0]));
        m.insert("c".into(), args::f64_list_value([0.0, 1.0, 0.0]));
        m.insert("d".into(), args::f64_list_value([0.0, 0.0, 1.0]));
        m.insert("e".into(), args::f64_list_value([0.1, 0.1, 0.1]));
        let out = insphere_host(&Value::Record(m), span()).unwrap();
        // Positively oriented tet: inside → Negative (−1). See insphere.rs.
        assert_eq!(args::rec_i64(&out, "sign").unwrap(), -1);
    }

    #[test]
    fn wave23_ham_sandwich_two_pairs() {
        let mut m = BTreeMap::new();
        m.insert("set_a".into(), pts(&[[0.0, 0.0], [2.0, 0.0]]));
        m.insert("set_b".into(), pts(&[[0.0, 2.0], [2.0, 2.0]]));
        let out = ham_sandwich_cut_host(&Value::Record(m), span()).unwrap();
        assert!(args::rec_f64(&out, "px").unwrap().is_finite());
        assert!(args::rec_f64(&out, "dy").is_some() || args::rec_f64(&out, "dx").is_some());
    }

    #[test]
    fn wave23_smallest_enclosing_disk_unit_square() {
        let mut m = BTreeMap::new();
        m.insert(
            "points".into(),
            pts(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
        );
        m.insert("seed".into(), Value::U64(1));
        let out = smallest_enclosing_disk_host(&Value::Record(m), span()).unwrap();
        let r = args::rec_f64(&out, "radius").unwrap();
        assert!((r - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9);
    }

    #[test]
    fn wave23_polygon_signed_area_unit_square() {
        let mut m = BTreeMap::new();
        m.insert(
            "vertices".into(),
            pts(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
        );
        let out = polygon_signed_area_host(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "area").unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn wave23_polygon_area_unit_square() {
        let mut m = BTreeMap::new();
        m.insert(
            "vertices".into(),
            pts(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
        );
        let out = polygon_area_host(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "area").unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn wave23_point_in_polygon_inside() {
        let mut m = BTreeMap::new();
        m.insert("point".into(), args::f64_list_value([0.5, 0.5]));
        m.insert(
            "polygon".into(),
            pts(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
        );
        let out = point_in_polygon_host(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_bool(&out, "inside"), Some(true));
    }

    #[test]
    fn wave23_minkowski_sum_convex_unit_squares() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), pts(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]));
        m.insert("b".into(), pts(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]));
        let out = minkowski_sum_convex_host(&Value::Record(m), span()).unwrap();
        assert!(args::rec_u64(&out, "count").unwrap() >= 4);
    }

    #[test]
    fn wave23_nearest_segment_site_two_segs() {
        let mut m = BTreeMap::new();
        m.insert(
            "segments".into(),
            Value::List(vec![
                Value::List(vec![
                    args::f64_list_value([0.0, 0.0]),
                    args::f64_list_value([1.0, 0.0]),
                ]),
                Value::List(vec![
                    args::f64_list_value([10.0, 10.0]),
                    args::f64_list_value([11.0, 10.0]),
                ]),
            ]),
        );
        m.insert("q".into(), args::f64_list_value([0.5, 0.1]));
        let out = nearest_segment_site_host(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_u64(&out, "index").unwrap(), 0);
    }
}
