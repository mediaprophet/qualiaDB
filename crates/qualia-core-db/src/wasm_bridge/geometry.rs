//! P9.1 — Browser/WASM geometry + creation API surface.
//!
//! Exposes the computational-geometry kernel ops to JavaScript via
//! `#[wasm_bindgen]`. Each function delegates to the same caller-buffered
//! native kernel that `execute_geometry_tool_json` uses, guaranteeing
//! identical results between the WASM surface and the MCP/native surface.
//!
//! ## Determinism
//!
//! Every function produces bit-identical output to the native kernel for
//! identical input — the WASM boundary adds serialization overhead only;
//! the kernel path is the same.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use crate::specialized_libs::computational_geometry::{
    convex_hull_indices_2, delaunay_triangulation_2, nearest_site_brute_force, orientation_2,
    voronoi_diagram_2, EdgeSlot, HalfEdge, Orientation, Point2, VoronoiEdge, VoronoiVertex,
};

// ───────────────────────────────────────────────────────────────────────────
//  orientation_2
// ───────────────────────────────────────────────────────────────────────────

/// Robust 2-D orientation predicate.
///
/// Returns `"clockwise"`, `"collinear"`, or `"counter_clockwise"` —
/// identical to the native `orientation_2` sign.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn geometry_orientation_2(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64) -> String {
    let orient = orientation_2(
        Point2::new(ax, ay),
        Point2::new(bx, by),
        Point2::new(cx, cy),
    );
    match orient {
        Orientation::Clockwise => "clockwise",
        Orientation::Collinear => "collinear",
        Orientation::CounterClockwise => "counter_clockwise",
    }
    .to_string()
}

/// Numeric orientation sign (-1, 0, 1) for machine consumption.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn geometry_orientation_2_sign(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64) -> i8 {
    let orient = orientation_2(
        Point2::new(ax, ay),
        Point2::new(bx, by),
        Point2::new(cx, cy),
    );
    orient as i8
}

// ───────────────────────────────────────────────────────────────────────────
//  convex_hull_2
// ───────────────────────────────────────────────────────────────────────────

/// Input for `convex_hull_2`.
#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct ConvexHullInput {
    /// Flat `[x0, y0, x1, y1, ...]` array.
    pub points: Vec<f64>,
}

/// Output for `convex_hull_2`.
#[cfg(target_arch = "wasm32")]
#[derive(Serialize)]
pub struct ConvexHullOutput {
    pub indices: Vec<u32>,
    pub vertex_count: usize,
    /// Flat `[x0, y0, ...]` hull vertex coordinates.
    pub hull_points: Vec<f64>,
}

/// Compute the 2-D convex hull of a point set.
///
/// `points` is a flat `[x0, y0, x1, y1, ...]` array.
/// Returns `{ indices, vertex_count, hull_points }` where `hull_points`
/// is a flat `[x0, y0, ...]` array of hull vertices in order.
///
/// Over the 5-point fixture `[[0,0],[1,0],[0.5,0.5],[1,1],[0,1]]` this
/// returns `indices = [0,1,3,4]`, `vertex_count = 4` — identical to the
/// native `execute_geometry_tool_json` test.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn geometry_convex_hull_2(val: JsValue) -> Result<JsValue, JsValue> {
    let input: ConvexHullInput =
        serde_wasm_bindgen::from_value(val).map_err(|e| JsValue::from_str(&e.to_string()))?;

    if input.points.len() % 2 != 0 {
        return Err(JsValue::from_str("points must be a flat [x,y,...] array"));
    }

    let pts: Vec<Point2> = input
        .points
        .chunks(2)
        .map(|pair| Point2::new(pair[0], pair[1]))
        .collect();

    let n = pts.len();
    let mut scratch = vec![0u32; n.saturating_mul(3)];
    let mut indices = vec![0u32; n];

    let count = convex_hull_indices_2(&pts, &mut scratch, &mut indices)
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;

    indices.truncate(count);

    let mut hull_points = Vec::with_capacity(count * 2);
    for &idx in &indices {
        let p = pts[idx as usize];
        hull_points.push(p.x);
        hull_points.push(p.y);
    }

    Ok(serde_wasm_bindgen::to_value(&ConvexHullOutput {
        indices,
        vertex_count: count,
        hull_points,
    })?)
}

// ───────────────────────────────────────────────────────────────────────────
//  delaunay_2
// ───────────────────────────────────────────────────────────────────────────

/// Input for `delaunay_2`.
#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct DelaunayInput {
    pub points: Vec<f64>,
}

/// Output for `delaunay_2`.
#[cfg(target_arch = "wasm32")]
#[derive(Serialize)]
pub struct DelaunayOutput {
    pub triangle_count: usize,
    /// Flat `[v0, v1, v2, v0, v1, v2, ...]` vertex index triples.
    pub triangles: Vec<u32>,
}

/// Compute the Delaunay triangulation of a 2-D point set.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn geometry_delaunay_2(val: JsValue) -> Result<JsValue, JsValue> {
    let input: DelaunayInput =
        serde_wasm_bindgen::from_value(val).map_err(|e| JsValue::from_str(&e.to_string()))?;

    if input.points.len() % 2 != 0 {
        return Err(JsValue::from_str("points must be a flat [x,y,...] array"));
    }

    let pts: Vec<Point2> = input
        .points
        .chunks(2)
        .map(|pair| Point2::new(pair[0], pair[1]))
        .collect();

    let n = pts.len();
    if n < 3 {
        return Err(JsValue::from_str("need >= 3 points"));
    }

    let mut scratch = vec![0u32; n];
    let mut tris = vec![[0u32; 3]; 2 * n + 1];

    let count = delaunay_triangulation_2(&pts, &mut scratch, &mut tris)
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;

    tris.truncate(count);

    let mut flat = Vec::with_capacity(count * 3);
    for t in &tris {
        flat.push(t[0]);
        flat.push(t[1]);
        flat.push(t[2]);
    }

    Ok(serde_wasm_bindgen::to_value(&DelaunayOutput {
        triangle_count: count,
        triangles: flat,
    })?)
}

// ───────────────────────────────────────────────────────────────────────────
//  voronoi_2
// ───────────────────────────────────────────────────────────────────────────

/// Input for `voronoi_2`.
#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct VoronoiInput {
    pub points: Vec<f64>,
}

/// Output for `voronoi_2`.
#[cfg(target_arch = "wasm32")]
#[derive(Serialize)]
pub struct VoronoiOutput {
    pub vertex_count: usize,
    pub edge_count: usize,
    /// Flat `[cx, cy, cx, cy, ...]` circumcenters.
    pub vertices: Vec<f64>,
    /// Flat `[site_a, site_b, triangle, neighbor_or_-1, ...]`.
    pub edges: Vec<i64>,
}

/// Compute the Voronoi diagram of a 2-D point set.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn geometry_voronoi_2(val: JsValue) -> Result<JsValue, JsValue> {
    let input: VoronoiInput =
        serde_wasm_bindgen::from_value(val).map_err(|e| JsValue::from_str(&e.to_string()))?;

    if input.points.len() % 2 != 0 {
        return Err(JsValue::from_str("points must be a flat [x,y,...] array"));
    }

    let pts: Vec<Point2> = input
        .points
        .chunks(2)
        .map(|pair| Point2::new(pair[0], pair[1]))
        .collect();

    let n = pts.len();
    if n < 3 {
        return Err(JsValue::from_str("need >= 3 sites"));
    }

    let mut tri_scratch = vec![0u32; n];
    let mut tri_out = vec![[0u32; 3]; 2 * n + 1];
    let mut verts = vec![
        VoronoiVertex {
            triangle_index: 0,
            center: Point2::new(0.0, 0.0)
        };
        2 * n + 1
    ];
    let mut edges = vec![
        VoronoiEdge {
            site_a: 0,
            site_b: 0,
            triangle: 0,
            neighbor_triangle: None
        };
        3 * n
    ];

    let (vc, ec) = voronoi_diagram_2(&pts, &mut tri_scratch, &mut tri_out, &mut verts, &mut edges)
        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;

    let mut flat_verts = Vec::with_capacity(vc * 2);
    for v in &verts[..vc] {
        flat_verts.push(v.center.x);
        flat_verts.push(v.center.y);
    }

    let mut flat_edges = Vec::with_capacity(ec * 4);
    for e in &edges[..ec] {
        flat_edges.push(e.site_a as i64);
        flat_edges.push(e.site_b as i64);
        flat_edges.push(e.triangle as i64);
        flat_edges.push(e.neighbor_triangle.map(|t| t as i64).unwrap_or(-1));
    }

    Ok(serde_wasm_bindgen::to_value(&VoronoiOutput {
        vertex_count: vc,
        edge_count: ec,
        vertices: flat_verts,
        edges: flat_edges,
    })?)
}

// ───────────────────────────────────────────────────────────────────────────
//  nearest_site
// ───────────────────────────────────────────────────────────────────────────

/// Find the nearest site to a query point (brute-force).
///
/// Returns the index of the nearest site, or -1 if the point set is empty.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn geometry_nearest_site(points: &[f64], qx: f64, qy: f64) -> i32 {
    if points.len() % 2 != 0 || points.is_empty() {
        return -1;
    }

    let pts: Vec<Point2> = points
        .chunks(2)
        .map(|pair| Point2::new(pair[0], pair[1]))
        .collect();

    nearest_site_brute_force(&pts, Point2::new(qx, qy))
        .map(|idx| idx as i32)
        .unwrap_or(-1)
}

// ───────────────────────────────────────────────────────────────────────────
//  JSON passthrough (full tool surface)
// ───────────────────────────────────────────────────────────────────────────

/// Execute any geometry tool via the JSON boundary — same function as
/// `execute_geometry_tool_json` on native. This is the full op surface
/// (`orientation_2`, `convex_hull_2`, `triangle_topology`, `mesh_topology`,
/// `delaunay_2`, `voronoi_2`, `nearest_site`).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn geometry_execute_json(args: &str) -> Result<String, JsValue> {
    crate::specialized_libs::computational_geometry::execute_geometry_tool_json(args)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

// ───────────────────────────────────────────────────────────────────────────
//  Native tests (run on host, not WASM)
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::specialized_libs::computational_geometry::{
        convex_hull_indices_2, execute_geometry_tool_json, orientation_2, Orientation, Point2,
    };

    /// The 5-point fixture used by the acceptance gate: WASM hull must
    /// return `[0,1,3,4]` / vertex_count 4 — identical to native.
    #[test]
    fn native_hull_5_point_fixture() {
        let result = execute_geometry_tool_json(
            r#"{"op":"convex_hull_2","points":[[0,0],[1,0],[0.5,0.5],[1,1],[0,1]]}"#,
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["vertex_count"], 4);
        assert_eq!(value["indices"], serde_json::json!([0, 1, 3, 4]));
    }

    /// Orientation sign must match on collinear / CW / CCW triples.
    #[test]
    fn native_orientation_signs() {
        let collinear = orientation_2(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(2.0, 0.0),
        );
        assert_eq!(collinear, Orientation::Collinear);

        let cw = orientation_2(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, -1.0),
        );
        assert_eq!(cw, Orientation::Clockwise);

        let ccw = orientation_2(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        );
        assert_eq!(ccw, Orientation::CounterClockwise);
    }

    /// Verify the raw kernel matches the JSON boundary for the 5-point fixture.
    #[test]
    fn kernel_hull_matches_json_boundary() {
        let pts = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.5, 0.5),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];
        let mut scratch = vec![0u32; 15];
        let mut indices = vec![0u32; 5];
        let count = convex_hull_indices_2(&pts, &mut scratch, &mut indices).unwrap();
        indices.truncate(count);
        assert_eq!(count, 4);
        assert_eq!(indices, vec![0, 1, 3, 4]);
    }
}
