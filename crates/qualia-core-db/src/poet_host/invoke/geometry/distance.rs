//! Computational geometry invoke seams — distance and intersection functions.
//!
//! Exposes `specialized_libs::computational_geometry::distance` functions
//! through VibeScript invoke IDs.

use super::super::args;
use crate::specialized_libs::computational_geometry as cg;
use vibe::{Diagnostic, Span, Value};

/// `ComputationalGeometry.distance_2d` — Euclidean distance between two 2D points.
/// Args: { ax, ay, bx, by }
pub fn distance_2d(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let ax = args::rec_f64(args, "ax")
        .ok_or_else(|| args::bad(span, "ComputationalGeometry.distance_2d needs ax"))?;
    let ay = args::rec_f64(args, "ay")
        .ok_or_else(|| args::bad(span, "ComputationalGeometry.distance_2d needs ay"))?;
    let bx = args::rec_f64(args, "bx")
        .ok_or_else(|| args::bad(span, "ComputationalGeometry.distance_2d needs bx"))?;
    let by = args::rec_f64(args, "by")
        .ok_or_else(|| args::bad(span, "ComputationalGeometry.distance_2d needs by"))?;
    let d = cg::distance_2d(cg::Point2::new(ax, ay), cg::Point2::new(bx, by));
    Ok(args::record([("distance", Value::F64(d))]))
}

/// `ComputationalGeometry.distance_3d` — Euclidean distance between two 3D points.
/// Args: { ax, ay, az, bx, by, bz }
pub fn distance_3d(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let ax = args::rec_f64(args, "ax")
        .ok_or_else(|| args::bad(span, "ComputationalGeometry.distance_3d needs ax"))?;
    let ay = args::rec_f64(args, "ay")
        .ok_or_else(|| args::bad(span, "ComputationalGeometry.distance_3d needs ay"))?;
    let az = args::rec_f64(args, "az")
        .ok_or_else(|| args::bad(span, "ComputationalGeometry.distance_3d needs az"))?;
    let bx = args::rec_f64(args, "bx")
        .ok_or_else(|| args::bad(span, "ComputationalGeometry.distance_3d needs bx"))?;
    let by = args::rec_f64(args, "by")
        .ok_or_else(|| args::bad(span, "ComputationalGeometry.distance_3d needs by"))?;
    let bz = args::rec_f64(args, "bz")
        .ok_or_else(|| args::bad(span, "ComputationalGeometry.distance_3d needs bz"))?;
    let d = cg::distance_3d(cg::Point3::new(ax, ay, az), cg::Point3::new(bx, by, bz));
    Ok(args::record([("distance", Value::F64(d))]))
}

/// `ComputationalGeometry.point_segment_distance_2d` — distance from point to segment in 2D.
/// Args: { px, py, ax, ay, bx, by }
pub fn point_segment_distance_2d(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let px = args::rec_f64(args, "px").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_segment_distance_2d needs px",
        )
    })?;
    let py = args::rec_f64(args, "py").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_segment_distance_2d needs py",
        )
    })?;
    let ax = args::rec_f64(args, "ax").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_segment_distance_2d needs ax",
        )
    })?;
    let ay = args::rec_f64(args, "ay").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_segment_distance_2d needs ay",
        )
    })?;
    let bx = args::rec_f64(args, "bx").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_segment_distance_2d needs bx",
        )
    })?;
    let by = args::rec_f64(args, "by").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_segment_distance_2d needs by",
        )
    })?;
    let d = cg::point_segment_distance_2d(
        cg::Point2::new(px, py),
        cg::Point2::new(ax, ay),
        cg::Point2::new(bx, by),
    );
    Ok(args::record([("distance", Value::F64(d))]))
}

/// `ComputationalGeometry.point_segment_distance_3d` — distance from point to segment in 3D.
/// Args: { px, py, pz, ax, ay, az, bx, by, bz }
pub fn point_segment_distance_3d(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let px = args::rec_f64(args, "px").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_segment_distance_3d needs px",
        )
    })?;
    let py = args::rec_f64(args, "py").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_segment_distance_3d needs py",
        )
    })?;
    let pz = args::rec_f64(args, "pz").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_segment_distance_3d needs pz",
        )
    })?;
    let ax = args::rec_f64(args, "ax").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_segment_distance_3d needs ax",
        )
    })?;
    let ay = args::rec_f64(args, "ay").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_segment_distance_3d needs ay",
        )
    })?;
    let az = args::rec_f64(args, "az").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_segment_distance_3d needs az",
        )
    })?;
    let bx = args::rec_f64(args, "bx").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_segment_distance_3d needs bx",
        )
    })?;
    let by = args::rec_f64(args, "by").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_segment_distance_3d needs by",
        )
    })?;
    let bz = args::rec_f64(args, "bz").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_segment_distance_3d needs bz",
        )
    })?;
    let d = cg::point_segment_distance_sq_3d(
        cg::Point3::new(px, py, pz),
        cg::Point3::new(ax, ay, az),
        cg::Point3::new(bx, by, bz),
    )
    .sqrt();
    Ok(args::record([("distance", Value::F64(d))]))
}

/// `ComputationalGeometry.point_triangle_distance_3d` — squared distance from point to triangle in 3D.
/// Args: { px, py, pz, ax, ay, az, bx, by, bz, cx, cy, cz }
pub fn point_triangle_distance_3d(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let px = args::rec_f64(args, "px").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_triangle_distance_3d needs px",
        )
    })?;
    let py = args::rec_f64(args, "py").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_triangle_distance_3d needs py",
        )
    })?;
    let pz = args::rec_f64(args, "pz").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_triangle_distance_3d needs pz",
        )
    })?;
    let ax = args::rec_f64(args, "ax").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_triangle_distance_3d needs ax",
        )
    })?;
    let ay = args::rec_f64(args, "ay").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_triangle_distance_3d needs ay",
        )
    })?;
    let az = args::rec_f64(args, "az").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_triangle_distance_3d needs az",
        )
    })?;
    let bx = args::rec_f64(args, "bx").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_triangle_distance_3d needs bx",
        )
    })?;
    let by = args::rec_f64(args, "by").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_triangle_distance_3d needs by",
        )
    })?;
    let bz = args::rec_f64(args, "bz").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_triangle_distance_3d needs bz",
        )
    })?;
    let cx = args::rec_f64(args, "cx").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_triangle_distance_3d needs cx",
        )
    })?;
    let cy = args::rec_f64(args, "cy").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_triangle_distance_3d needs cy",
        )
    })?;
    let cz = args::rec_f64(args, "cz").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.point_triangle_distance_3d needs cz",
        )
    })?;
    let d = cg::point_triangle_distance_sq_3d(
        cg::Point3::new(px, py, pz),
        cg::Point3::new(ax, ay, az),
        cg::Point3::new(bx, by, bz),
        cg::Point3::new(cx, cy, cz),
    );
    Ok(args::record([("distance_sq", Value::F64(d))]))
}
