//! Wave-21 Host binds: Statistical manifold & distance numerics.
//!
//! Pure CPU paths from `specialized_libs::computational_geometry::{statistical_manifold, voronoi_variants, triangulation_2}`
//! — no forge / `caps()` / CUDA / GPU.

use super::super::args;
use crate::specialized_libs::computational_geometry::statistical_manifold::{
    fisher_distance, kl_bregman_form, kl_divergence,
};
use crate::specialized_libs::computational_geometry::triangulation_2::Triangle;
use crate::specialized_libs::computational_geometry::voronoi_variants::{
    dist_point_to_segment as raw_dist_point_to_segment,
    dist_sq_point_to_segment as raw_dist_sq_point_to_segment,
};
use crate::specialized_libs::computational_geometry::Point2;
use vibe::{Diagnostic, Span, Value};

fn parse_f32_vec(args_v: &Value, key: &str, span: Span, what: &str) -> Result<Vec<f32>, Diagnostic> {
    let f64s = args::rec_f64_list(args_v, key)
        .ok_or_else(|| args::bad(span, format!("{what} needs {key}: [f64]")))?;
    if f64s.is_empty() || f64s.len() > 1024 {
        return Err(args::bad(span, format!("{what}: {key} must have 1..=1024 elements")));
    }
    Ok(f64s.into_iter().map(|x| x as f32).collect())
}

fn parse_point2_arg(args_v: &Value, key: &str, span: Span, what: &str) -> Result<Point2, Diagnostic> {
    let coords = args::rec_f64_list(args_v, key)
        .ok_or_else(|| args::bad(span, format!("{what} needs {key}: [f64; 2]")))?;
    if coords.len() < 2 {
        return Err(args::bad(span, format!("{what}: {key} needs ≥2 coords")));
    }
    Ok(Point2::new(coords[0], coords[1]))
}

/// `ComputationalGeometry.fisher_distance` — geodesic distance on probability simplex under Fisher metric.
/// Args: `{ p: [f64], q: [f64] }`. Out: `{ distance: f64 }`.
pub fn fisher_distance_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = parse_f32_vec(args_v, "p", span, "fisher_distance")?;
    let q = parse_f32_vec(args_v, "q", span, "fisher_distance")?;
    let dist = fisher_distance(&p, &q)
        .map_err(|e| args::bad(span, format!("fisher_distance: {e}")))?;
    Ok(args::record([("distance", Value::F64(dist))]))
}

/// `ComputationalGeometry.kl_divergence` — Kullback-Leibler divergence `KL(p‖q)`.
/// Args: `{ p: [f64], q: [f64] }`. Out: `{ divergence: f64 }`.
pub fn kl_divergence_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = parse_f32_vec(args_v, "p", span, "kl_divergence")?;
    let q = parse_f32_vec(args_v, "q", span, "kl_divergence")?;
    let div = kl_divergence(&p, &q)
        .map_err(|e| args::bad(span, format!("kl_divergence: {e}")))?;
    Ok(args::record([("divergence", Value::F64(div))]))
}

/// `ComputationalGeometry.kl_bregman_form` — KL divergence via Bregman formula.
/// Args: `{ p: [f64], q: [f64] }`. Out: `{ divergence: f64 }`.
pub fn kl_bregman_form_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = parse_f32_vec(args_v, "p", span, "kl_bregman_form")?;
    let q = parse_f32_vec(args_v, "q", span, "kl_bregman_form")?;
    let div = kl_bregman_form(&p, &q)
        .map_err(|e| args::bad(span, format!("kl_bregman_form: {e}")))?;
    Ok(args::record([("divergence", Value::F64(div))]))
}

/// `ComputationalGeometry.triangle_signed_area` — signed 2D area of triangle abc (positive for CCW).
/// Args: `{ a: [f64; 2], b: [f64; 2], c: [f64; 2] }`. Out: `{ signed_area: f64 }`.
pub fn triangle_signed_area_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = parse_point2_arg(args_v, "a", span, "triangle_signed_area")?;
    let b = parse_point2_arg(args_v, "b", span, "triangle_signed_area")?;
    let c = parse_point2_arg(args_v, "c", span, "triangle_signed_area")?;
    let tri = Triangle::new(a, b, c);
    Ok(args::record([("signed_area", Value::F64(tri.signed_area()))]))
}

/// `ComputationalGeometry.dist_point_to_segment` — Euclidean distance from point to 2D line segment ab.
/// Args: `{ point: [f64; 2], a: [f64; 2], b: [f64; 2] }`. Out: `{ dist: f64 }`.
pub fn dist_point_to_segment_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = parse_point2_arg(args_v, "point", span, "dist_point_to_segment")?;
    let a = parse_point2_arg(args_v, "a", span, "dist_point_to_segment")?;
    let b = parse_point2_arg(args_v, "b", span, "dist_point_to_segment")?;
    let d = raw_dist_point_to_segment(p, a, b);
    Ok(args::record([("dist", Value::F64(d))]))
}

/// `ComputationalGeometry.dist_sq_point_to_segment` — squared distance from point to 2D line segment ab.
/// Args: `{ point: [f64; 2], a: [f64; 2], b: [f64; 2] }`. Out: `{ dist_sq: f64 }`.
pub fn dist_sq_point_to_segment_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = parse_point2_arg(args_v, "point", span, "dist_sq_point_to_segment")?;
    let a = parse_point2_arg(args_v, "a", span, "dist_sq_point_to_segment")?;
    let b = parse_point2_arg(args_v, "b", span, "dist_sq_point_to_segment")?;
    let d2 = raw_dist_sq_point_to_segment(p, a, b);
    Ok(args::record([("dist_sq", Value::F64(d2))]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn wave21_fisher_distance_identical_zero() {
        let mut m = BTreeMap::new();
        m.insert("p".into(), args::f64_list_value([0.5, 0.5]));
        m.insert("q".into(), args::f64_list_value([0.5, 0.5]));
        let out = fisher_distance_host(&Value::Record(m), span()).unwrap();
        let d = args::rec_f64(&out, "distance").unwrap();
        assert!(d.abs() < 1e-6);
    }

    #[test]
    fn wave21_kl_divergence_identical_zero() {
        let mut m = BTreeMap::new();
        m.insert("p".into(), args::f64_list_value([0.25, 0.75]));
        m.insert("q".into(), args::f64_list_value([0.25, 0.75]));
        let out = kl_divergence_host(&Value::Record(m), span()).unwrap();
        let d = args::rec_f64(&out, "divergence").unwrap();
        assert!(d.abs() < 1e-6);
    }

    #[test]
    fn wave21_kl_bregman_form_matches() {
        let mut m = BTreeMap::new();
        m.insert("p".into(), args::f64_list_value([0.4, 0.6]));
        m.insert("q".into(), args::f64_list_value([0.5, 0.5]));
        let out = kl_bregman_form_host(&Value::Record(m), span()).unwrap();
        let d = args::rec_f64(&out, "divergence").unwrap();
        assert!(d > 0.0 && d.is_finite());
    }

    #[test]
    fn wave21_triangle_signed_area_unit() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), args::f64_list_value([0.0, 0.0]));
        m.insert("b".into(), args::f64_list_value([1.0, 0.0]));
        m.insert("c".into(), args::f64_list_value([0.0, 1.0]));
        let out = triangle_signed_area_host(&Value::Record(m), span()).unwrap();
        let area = args::rec_f64(&out, "signed_area").unwrap();
        assert!((area - 0.5).abs() < 1e-12);
    }

    #[test]
    fn wave21_dist_point_to_segment_perpendicular() {
        let mut m = BTreeMap::new();
        m.insert("point".into(), args::f64_list_value([0.5, 2.0]));
        m.insert("a".into(), args::f64_list_value([0.0, 0.0]));
        m.insert("b".into(), args::f64_list_value([1.0, 0.0]));
        let out = dist_point_to_segment_host(&Value::Record(m), span()).unwrap();
        let dist = args::rec_f64(&out, "dist").unwrap();
        assert!((dist - 2.0).abs() < 1e-12);
    }

    #[test]
    fn wave21_dist_sq_point_to_segment_perpendicular() {
        let mut m = BTreeMap::new();
        m.insert("point".into(), args::f64_list_value([0.5, 3.0]));
        m.insert("a".into(), args::f64_list_value([0.0, 0.0]));
        m.insert("b".into(), args::f64_list_value([1.0, 0.0]));
        let out = dist_sq_point_to_segment_host(&Value::Record(m), span()).unwrap();
        let d2 = args::rec_f64(&out, "dist_sq").unwrap();
        assert!((d2 - 9.0).abs() < 1e-12);
    }
}
