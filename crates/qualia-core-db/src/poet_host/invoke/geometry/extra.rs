//! Additional computational geometry invoke seams — triangulation, mesh
//! measurement, spatial ordering, intersections, CAD curves, orientation.

use super::super::args;
use crate::specialized_libs::computational_geometry as cg;
use poet_vibe::{Diagnostic, Span, Value};

/// `ComputationalGeometry.triangulate_polygon` — ear-clipping triangulation.
/// Args: { vertices: [[f64; 2]] }
pub fn triangulate_polygon(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let pts = parse_point2_list(args, "vertices").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.triangulate_polygon needs vertices",
        )
    })?;
    let tris = cg::triangulate_polygon(&pts);
    let tri_records: Vec<Value> = tris
        .iter()
        .map(|t| {
            args::record([
                ("ax", Value::F64(t.a.x)),
                ("ay", Value::F64(t.a.y)),
                ("bx", Value::F64(t.b.x)),
                ("by", Value::F64(t.b.y)),
                ("cx", Value::F64(t.c.x)),
                ("cy", Value::F64(t.c.y)),
            ])
        })
        .collect();
    Ok(args::record([
        ("triangles", Value::List(tri_records)),
        ("count", Value::U64(tris.len() as u64)),
    ]))
}

/// `ComputationalGeometry.surface_area` — surface area of a triangle mesh.
/// Args: { vertices: [[f64; 3]], triangles: [[u64; 3]] }
pub fn surface_area(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let verts = parse_point3_list(args, "vertices")
        .ok_or_else(|| args::bad(span, "ComputationalGeometry.surface_area needs vertices"))?;
    let tris = parse_triangle_indices(args, "triangles")
        .ok_or_else(|| args::bad(span, "ComputationalGeometry.surface_area needs triangles"))?;
    match cg::surface_area(&verts, &tris) {
        Ok(area) => Ok(Value::F64(area)),
        Err(e) => Err(args::bad(span, format!("surface_area: {e:?}"))),
    }
}

/// `ComputationalGeometry.signed_volume` — signed volume of a triangle mesh.
/// Args: { vertices: [[f64; 3]], triangles: [[u64; 3]] }
pub fn signed_volume(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let verts = parse_point3_list(args, "vertices")
        .ok_or_else(|| args::bad(span, "ComputationalGeometry.signed_volume needs vertices"))?;
    let tris = parse_triangle_indices(args, "triangles")
        .ok_or_else(|| args::bad(span, "ComputationalGeometry.signed_volume needs triangles"))?;
    match cg::signed_volume(&verts, &tris) {
        Ok(vol) => Ok(Value::F64(vol)),
        Err(e) => Err(args::bad(span, format!("signed_volume: {e:?}"))),
    }
}

/// `ComputationalGeometry.morton_encode_2d` — Morton (Z-order) code for 2D.
/// Args: { x: u64, y: u64 }
pub fn morton_encode_2d(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_u64(args, "x").unwrap_or(0) as u16;
    let y = args::rec_u64(args, "y").unwrap_or(0) as u16;
    Ok(Value::U64(cg::morton_encode_2d(x, y) as u64))
}

/// `ComputationalGeometry.morton_decode_2d` — decode a 2D Morton code.
/// Args: { code: u64 }
pub fn morton_decode_2d(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let code = args::rec_u64(args, "code").unwrap_or(0) as u32;
    let (x, y) = cg::morton_decode_2d(code);
    Ok(args::record([
        ("x", Value::U64(x as u64)),
        ("y", Value::U64(y as u64)),
    ]))
}

/// `ComputationalGeometry.morton_encode_3d` — Morton code for 3D.
/// Args: { x: u64, y: u64, z: u64 }
pub fn morton_encode_3d(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_u64(args, "x").unwrap_or(0) as u16;
    let y = args::rec_u64(args, "y").unwrap_or(0) as u16;
    let z = args::rec_u64(args, "z").unwrap_or(0) as u16;
    Ok(Value::U64(cg::morton_encode_3d(x, y, z)))
}

/// `ComputationalGeometry.hilbert_encode_2d` — Hilbert curve code for 2D.
/// Args: { x: u64, y: u64 }
pub fn hilbert_encode_2d(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_u64(args, "x").unwrap_or(0) as u16;
    let y = args::rec_u64(args, "y").unwrap_or(0) as u16;
    Ok(Value::U64(cg::hilbert_encode_2d(x, y) as u64))
}

/// `ComputationalGeometry.orientation_2` — orientation of three 2D points.
/// Args: { a: [f64; 2], b: [f64; 2], c: [f64; 2] }
pub fn orientation_2(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = parse_point2(args, "a").ok_or_else(|| args::bad(span, "orientation_2 needs a"))?;
    let b = parse_point2(args, "b").ok_or_else(|| args::bad(span, "orientation_2 needs b"))?;
    let c = parse_point2(args, "c").ok_or_else(|| args::bad(span, "orientation_2 needs c"))?;
    let orient = cg::orientation_2(a, b, c);
    Ok(args::record([(
        "orientation",
        Value::String(format!("{orient:?}")),
    )]))
}

/// `ComputationalGeometry.circumcenter` — circumcenter of a triangle.
/// Args: { a: [f64; 2], b: [f64; 2], c: [f64; 2] }
pub fn circumcenter(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = parse_point2(args, "a").ok_or_else(|| args::bad(span, "circumcenter needs a"))?;
    let b = parse_point2(args, "b").ok_or_else(|| args::bad(span, "circumcenter needs b"))?;
    let c = parse_point2(args, "c").ok_or_else(|| args::bad(span, "circumcenter needs c"))?;
    let center = cg::circumcenter(a, b, c);
    Ok(args::record([
        ("x", Value::F64(center.x)),
        ("y", Value::F64(center.y)),
    ]))
}

/// `ComputationalGeometry.line_segment_intersection_2` — intersection of two
/// 2D line segments.
/// Args: { a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2] }
pub fn line_segment_intersection_2(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = parse_point2(args, "a")
        .ok_or_else(|| args::bad(span, "line_segment_intersection_2 needs a"))?;
    let b = parse_point2(args, "b")
        .ok_or_else(|| args::bad(span, "line_segment_intersection_2 needs b"))?;
    let c = parse_point2(args, "c")
        .ok_or_else(|| args::bad(span, "line_segment_intersection_2 needs c"))?;
    let d = parse_point2(args, "d")
        .ok_or_else(|| args::bad(span, "line_segment_intersection_2 needs d"))?;
    match cg::line_segment_intersection_2(a, b, c, d) {
        Some(p) => Ok(args::record([
            ("x", Value::F64(p.x)),
            ("y", Value::F64(p.y)),
            ("intersects", Value::Bool(true)),
        ])),
        None => Ok(args::record([("intersects", Value::Bool(false))])),
    }
}

/// `ComputationalGeometry.bezier_eval` — evaluate a Bézier curve at parameter t.
/// Args: { control: [[f64; 3]], t: f64 }
pub fn bezier_eval(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let control = parse_point3_list(args, "control")
        .ok_or_else(|| args::bad(span, "ComputationalGeometry.bezier_eval needs control"))?;
    let t = args::rec_f64(args, "t")
        .ok_or_else(|| args::bad(span, "ComputationalGeometry.bezier_eval needs t"))?;
    match cg::bezier_eval(&control, t) {
        Ok(p) => Ok(args::record([
            ("x", Value::F64(p.x)),
            ("y", Value::F64(p.y)),
            ("z", Value::F64(p.z)),
        ])),
        Err(e) => Err(args::bad(span, format!("bezier_eval: {e:?}"))),
    }
}

/// `ComputationalGeometry.nearest_site_brute_force` — nearest Voronoi site.
/// Args: { sites: [[f64; 2]], query: [f64; 2] }
pub fn nearest_site_brute_force(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let sites = parse_point2_list(args, "sites").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.nearest_site_brute_force needs sites",
        )
    })?;
    let query = parse_point2(args, "query").ok_or_else(|| {
        args::bad(
            span,
            "ComputationalGeometry.nearest_site_brute_force needs query",
        )
    })?;
    match cg::nearest_site_brute_force(&sites, query) {
        Some(idx) => Ok(args::record([
            ("index", Value::U64(idx as u64)),
            ("found", Value::Bool(true)),
        ])),
        None => Ok(args::record([("found", Value::Bool(false))])),
    }
}

/// `ComputationalGeometry.orient_3d` — orientation of a 3D tetrahedron.
/// Args: { a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3] }
pub fn orient_3d(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = parse_point3(args, "a").ok_or_else(|| args::bad(span, "orient_3d needs a"))?;
    let b = parse_point3(args, "b").ok_or_else(|| args::bad(span, "orient_3d needs b"))?;
    let c = parse_point3(args, "c").ok_or_else(|| args::bad(span, "orient_3d needs c"))?;
    let d = parse_point3(args, "d").ok_or_else(|| args::bad(span, "orient_3d needs d"))?;
    let sign = cg::orient_3d(a, b, c, d);
    Ok(args::record([(
        "orientation",
        Value::String(format!("{sign:?}")),
    )]))
}

// ── Helpers ─────────────────────────────────────────────────────────

fn parse_point2(args: &Value, key: &str) -> Option<cg::Point2> {
    let v = args::rec(args, key)?;
    let list = args::f64s(&v)?;
    if list.len() >= 2 {
        Some(cg::Point2::new(list[0], list[1]))
    } else {
        None
    }
}

fn parse_point3(args: &Value, key: &str) -> Option<cg::Point3> {
    let v = args::rec(args, key)?;
    let list = args::f64s(&v)?;
    if list.len() >= 3 {
        Some(cg::Point3::new(list[0], list[1], list[2]))
    } else {
        None
    }
}

fn parse_point2_list(args: &Value, key: &str) -> Option<Vec<cg::Point2>> {
    let v = args::rec(args, key)?;
    let list = match v {
        Value::List(l) => l,
        _ => return None,
    };
    let mut pts = Vec::new();
    for item in list {
        let coords = args::f64s(item)?;
        if coords.len() >= 2 {
            pts.push(cg::Point2::new(coords[0], coords[1]));
        }
    }
    Some(pts)
}

fn parse_point3_list(args: &Value, key: &str) -> Option<Vec<cg::Point3>> {
    let v = args::rec(args, key)?;
    let list = match v {
        Value::List(l) => l,
        _ => return None,
    };
    let mut pts = Vec::new();
    for item in list {
        let coords = args::f64s(item)?;
        if coords.len() >= 3 {
            pts.push(cg::Point3::new(coords[0], coords[1], coords[2]));
        }
    }
    Some(pts)
}

fn parse_triangle_indices(args: &Value, key: &str) -> Option<Vec<[u32; 3]>> {
    let v = args::rec(args, key)?;
    let list = match v {
        Value::List(l) => l,
        _ => return None,
    };
    let mut tris = Vec::new();
    for item in list {
        let idxs = args::f64s(item)?;
        if idxs.len() >= 3 {
            tris.push([idxs[0] as u32, idxs[1] as u32, idxs[2] as u32]);
        }
    }
    Some(tris)
}
