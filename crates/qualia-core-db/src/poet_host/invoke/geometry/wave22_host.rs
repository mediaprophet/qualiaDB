//! Wave-22 Host binds: exact predicates, Tukey depth, width, and site queries.
//!
//! Pure CPU paths from `specialized_libs::computational_geometry`
//! (`incircle`, `ham_sandwich`, `voronoi_variants`, `calipers_enclosing_disk`).
//! No forge / `caps()` / CUDA / GPU.

use super::super::args;
use crate::specialized_libs::computational_geometry::{
    diameter_and_width, directional_width, incircle, tukey_depth, width, Point2, Sign,
};
use crate::specialized_libs::computational_geometry::voronoi_variants::{
    farthest_site_brute, is_hull_site, k_nearest_sites,
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

fn sign_i64(sign: Sign) -> i64 {
    match sign {
        Sign::Negative => -1,
        Sign::Zero => 0,
        Sign::Positive => 1,
    }
}

/// `ComputationalGeometry.incircle` — exact in-circle predicate sign of d vs triangle abc.
/// Args: `{ a,b,c,d: [f64; 2] }`. Out: `{ sign: i64 }` (−1 outside / 0 on / +1 inside, CCW abc).
pub fn incircle_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = parse_point2_arg(args_v, "a", span, "incircle")?;
    let b = parse_point2_arg(args_v, "b", span, "incircle")?;
    let c = parse_point2_arg(args_v, "c", span, "incircle")?;
    let d = parse_point2_arg(args_v, "d", span, "incircle")?;
    Ok(args::record([("sign", Value::I64(sign_i64(incircle(a, b, c, d))))]))
}

/// `ComputationalGeometry.tukey_depth` — Tukey depth of query vs point set.
/// Args: `{ query: [f64; 2], points: [[f64; 2]] }`. Out: `{ depth: u64 }`.
pub fn tukey_depth_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let query = parse_point2_arg(args_v, "query", span, "tukey_depth")?;
    let points = parse_point2_list(args_v, "points", span, "tukey_depth")?;
    Ok(args::record([(
        "depth",
        Value::U64(tukey_depth(query, &points) as u64),
    )]))
}

/// `ComputationalGeometry.directional_width` — projected width along `dir`.
/// Args: `{ points: [[f64; 2]], dir: [f64; 2] }`. Out: `{ width: f64 }`.
pub fn directional_width_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let points = parse_point2_list(args_v, "points", span, "directional_width")?;
    let dir = parse_point2_arg(args_v, "dir", span, "directional_width")?;
    Ok(args::record([(
        "width",
        Value::F64(directional_width(&points, dir)),
    )]))
}

/// `ComputationalGeometry.width` — sampled minimum directional width.
/// Args: `{ points: [[f64; 2]] }`. Out: `{ width: f64 }`.
pub fn width_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let points = parse_point2_list(args_v, "points", span, "width")?;
    Ok(args::record([("width", Value::F64(width(&points)))]))
}

/// `ComputationalGeometry.farthest_site_brute` — index of farthest site from `q`.
/// Args: `{ sites: [[f64; 2]], q: [f64; 2] }`. Out: `{ index: u64 }`.
pub fn farthest_site_brute_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let sites = parse_point2_list(args_v, "sites", span, "farthest_site_brute")?;
    let q = parse_point2_arg(args_v, "q", span, "farthest_site_brute")?;
    Ok(args::record([(
        "index",
        Value::U64(farthest_site_brute(&sites, q) as u64),
    )]))
}

/// `ComputationalGeometry.k_nearest_sites` — k nearest site indices (nearest first).
/// Args: `{ sites: [[f64; 2]], q: [f64; 2], k: u64 }`. Out: `{ indices: [u64] }`.
pub fn k_nearest_sites_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let sites = parse_point2_list(args_v, "sites", span, "k_nearest_sites")?;
    let q = parse_point2_arg(args_v, "q", span, "k_nearest_sites")?;
    let k = args::rec_u64(args_v, "k").unwrap_or(1) as usize;
    if k == 0 || k > sites.len() {
        return Err(args::bad(
            span,
            format!("k_nearest_sites: k={k} invalid for n={}", sites.len()),
        ));
    }
    let ids = k_nearest_sites(&sites, q, k);
    Ok(args::record([(
        "indices",
        Value::List(ids.into_iter().map(|i| Value::U64(i as u64)).collect()),
    )]))
}

/// `ComputationalGeometry.is_hull_site` — whether `index` is on the convex hull.
/// Args: `{ sites: [[f64; 2]], index: u64 }`. Out: `{ on_hull: bool }`.
pub fn is_hull_site_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let sites = parse_point2_list(args_v, "sites", span, "is_hull_site")?;
    let index = args::rec_u64(args_v, "index")
        .ok_or_else(|| args::bad(span, "is_hull_site needs index: u64"))?;
    if index as usize >= sites.len() {
        return Err(args::bad(span, "is_hull_site: index out of range"));
    }
    Ok(args::record([(
        "on_hull",
        Value::Bool(is_hull_site(&sites, index as u32)),
    )]))
}

/// `ComputationalGeometry.diameter_and_width` — hull diameter + width via rotating calipers.
/// Args: `{ points: [[f64; 2]] }` (≥2). Out: `{ diameter: f64, width: f64 }`.
pub fn diameter_and_width_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let points = parse_point2_list(args_v, "points", span, "diameter_and_width")?;
    if points.len() < 2 {
        return Err(args::bad(span, "diameter_and_width needs ≥2 points"));
    }
    let result = diameter_and_width(&points)
        .map_err(|e| args::bad(span, format!("diameter_and_width: {e:?}")))?;
    Ok(args::record([
        ("diameter", Value::F64(result.diameter)),
        ("width", Value::F64(result.width)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    fn pts(pairs: &[[f64; 2]]) -> Value {
        Value::List(
            pairs
                .iter()
                .map(|p| args::f64_list_value(*p))
                .collect(),
        )
    }

    #[test]
    fn wave22_incircle_origin_inside_unit() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), args::f64_list_value([0.0, 0.0]));
        m.insert("b".into(), args::f64_list_value([1.0, 0.0]));
        m.insert("c".into(), args::f64_list_value([0.0, 1.0]));
        m.insert("d".into(), args::f64_list_value([0.2, 0.2]));
        let out = incircle_host(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_i64(&out, "sign").unwrap(), 1);
    }

    #[test]
    fn wave22_tukey_depth_centroid_positive() {
        let mut m = BTreeMap::new();
        m.insert("query".into(), args::f64_list_value([0.5, 0.5]));
        m.insert(
            "points".into(),
            pts(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
        );
        let out = tukey_depth_host(&Value::Record(m), span()).unwrap();
        assert!(args::rec_u64(&out, "depth").unwrap() >= 1);
    }

    #[test]
    fn wave22_directional_width_unit_square_x() {
        let mut m = BTreeMap::new();
        m.insert(
            "points".into(),
            pts(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
        );
        m.insert("dir".into(), args::f64_list_value([1.0, 0.0]));
        let out = directional_width_host(&Value::Record(m), span()).unwrap();
        let w = args::rec_f64(&out, "width").unwrap();
        assert!((w - 1.0).abs() < 1e-9);
    }

    #[test]
    fn wave22_width_unit_square() {
        let mut m = BTreeMap::new();
        m.insert(
            "points".into(),
            pts(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
        );
        let out = width_host(&Value::Record(m), span()).unwrap();
        let w = args::rec_f64(&out, "width").unwrap();
        assert!((w - 1.0).abs() < 1e-6);
    }

    #[test]
    fn wave22_farthest_site_brute_far_corner() {
        let mut m = BTreeMap::new();
        m.insert(
            "sites".into(),
            pts(&[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]),
        );
        m.insert("q".into(), args::f64_list_value([-1.0, -1.0]));
        let out = farthest_site_brute_host(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_u64(&out, "index").unwrap(), 1);
    }

    #[test]
    fn wave22_k_nearest_sites_k1_is_nearest() {
        let mut m = BTreeMap::new();
        m.insert(
            "sites".into(),
            pts(&[[0.0, 0.0], [10.0, 0.0], [0.0, 10.0]]),
        );
        m.insert("q".into(), args::f64_list_value([0.1, 0.0]));
        m.insert("k".into(), Value::U64(1));
        let out = k_nearest_sites_host(&Value::Record(m), span()).unwrap();
        let ids = args::rec_u64_list(&out, "indices").unwrap();
        assert_eq!(ids, vec![0]);
    }

    #[test]
    fn wave22_is_hull_site_corner_true_interior_false() {
        let square = pts(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.5, 0.5]]);
        let mut m = BTreeMap::new();
        m.insert("sites".into(), square.clone());
        m.insert("index".into(), Value::U64(0));
        let out = is_hull_site_host(&Value::Record(m), span()).unwrap();
        assert!(args::rec_bool(&out, "on_hull").unwrap());

        let mut m = BTreeMap::new();
        m.insert("sites".into(), square);
        m.insert("index".into(), Value::U64(4));
        let out = is_hull_site_host(&Value::Record(m), span()).unwrap();
        assert!(!args::rec_bool(&out, "on_hull").unwrap());
    }

    #[test]
    fn wave22_diameter_and_width_unit_segment() {
        let mut m = BTreeMap::new();
        m.insert("points".into(), pts(&[[0.0, 0.0], [2.0, 0.0]]));
        let out = diameter_and_width_host(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "diameter").unwrap() - 2.0).abs() < 1e-12);
        assert!(args::rec_f64(&out, "width").unwrap().abs() < 1e-12);
    }
}
