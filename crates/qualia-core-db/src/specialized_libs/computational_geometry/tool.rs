//! Cold JSON boundary shared by MCP and desktop/qapp hosts.

use serde_json::{json, Value};

use super::{
    build_triangle_half_edges, convex_hull_indices_2, delaunay_triangulation_2,
    nearest_site_brute_force, orientation_2, required_edge_slots, voronoi_diagram_2,
    EdgeSlot, HalfEdge, Point2, INVALID_INDEX,
};
use crate::container_10d::topology_section::{
    decode_topology_section, encode_topology_section, encoded_len as topology_encoded_len,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeometryToolError {
    InvalidJson,
    InvalidOperation,
    InvalidParameters,
    Geometry(String),
}

impl core::fmt::Display for GeometryToolError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidJson => write!(f, "invalid JSON"),
            Self::InvalidOperation => write!(f, "unknown geometry operation"),
            Self::InvalidParameters => write!(f, "invalid geometry parameters"),
            Self::Geometry(message) => f.write_str(message),
        }
    }
}

fn point(value: &Value) -> Result<Point2, GeometryToolError> {
    let pair = value
        .as_array()
        .filter(|pair| pair.len() >= 2)
        .ok_or(GeometryToolError::InvalidParameters)?;
    let x = pair[0]
        .as_f64()
        .ok_or(GeometryToolError::InvalidParameters)?;
    let y = pair[1]
        .as_f64()
        .ok_or(GeometryToolError::InvalidParameters)?;
    if !x.is_finite() || !y.is_finite() {
        return Err(GeometryToolError::InvalidParameters);
    }
    Ok(Point2::new(x, y))
}

fn points(value: &Value, key: &str) -> Result<Vec<Point2>, GeometryToolError> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or(GeometryToolError::InvalidParameters)?
        .iter()
        .map(point)
        .collect()
}

fn triangles(value: &Value) -> Result<Vec<[u32; 3]>, GeometryToolError> {
    value
        .get("triangles")
        .and_then(Value::as_array)
        .ok_or(GeometryToolError::InvalidParameters)?
        .iter()
        .map(|triangle| {
            let values = triangle
                .as_array()
                .filter(|values| values.len() == 3)
                .ok_or(GeometryToolError::InvalidParameters)?;
            Ok([
                u32::try_from(
                    values[0]
                        .as_u64()
                        .ok_or(GeometryToolError::InvalidParameters)?,
                )
                .map_err(|_| GeometryToolError::InvalidParameters)?,
                u32::try_from(
                    values[1]
                        .as_u64()
                        .ok_or(GeometryToolError::InvalidParameters)?,
                )
                .map_err(|_| GeometryToolError::InvalidParameters)?,
                u32::try_from(
                    values[2]
                        .as_u64()
                        .ok_or(GeometryToolError::InvalidParameters)?,
                )
                .map_err(|_| GeometryToolError::InvalidParameters)?,
            ])
        })
        .collect()
}

/// Parse a mesh from a JSON value with `positions` and `triangles` arrays.
fn parse_mesh_json(value: &Value) -> Result<crate::render::assets::Mesh, GeometryToolError> {
    let pos_arr = value.get("positions").and_then(Value::as_array)
        .ok_or(GeometryToolError::InvalidParameters)?;
    let tri_arr = value.get("triangles").and_then(Value::as_array)
        .ok_or(GeometryToolError::InvalidParameters)?;

    let mut positions = Vec::with_capacity(pos_arr.len());
    for p in pos_arr {
        let pa = p.as_array().ok_or(GeometryToolError::InvalidParameters)?;
        if pa.len() != 3 {
            return Err(GeometryToolError::InvalidParameters);
        }
        positions.push([
            pa[0].as_f64().ok_or(GeometryToolError::InvalidParameters)? as f32,
            pa[1].as_f64().ok_or(GeometryToolError::InvalidParameters)? as f32,
            pa[2].as_f64().ok_or(GeometryToolError::InvalidParameters)? as f32,
        ]);
    }

    let mut triangles = Vec::with_capacity(tri_arr.len());
    for t in tri_arr {
        let ta = t.as_array().ok_or(GeometryToolError::InvalidParameters)?;
        if ta.len() != 3 {
            return Err(GeometryToolError::InvalidParameters);
        }
        triangles.push([
            ta[0].as_u64().ok_or(GeometryToolError::InvalidParameters)? as u32,
            ta[1].as_u64().ok_or(GeometryToolError::InvalidParameters)? as u32,
            ta[2].as_u64().ok_or(GeometryToolError::InvalidParameters)? as u32,
        ]);
    }

    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in &positions {
        for a in 0..3 {
            min[a] = min[a].min(p[a]);
            max[a] = max[a].max(p[a]);
        }
    }

    Ok(crate::render::assets::Mesh { positions, triangles, min, max })
}

/// Execute a computational-geometry operation over a serde JSON boundary.
///
/// Allocations belong only to this explicit qapp/MCP boundary; every algorithm
/// below delegates to the caller-buffered native kernel.
pub fn execute_geometry_tool_json(args: &str) -> Result<String, GeometryToolError> {
    let value: Value = serde_json::from_str(args).map_err(|_| GeometryToolError::InvalidJson)?;
    let op = value
        .get("op")
        .and_then(Value::as_str)
        .ok_or(GeometryToolError::InvalidParameters)?;
    match op {
        "orientation_2" => {
            let input = points(&value, "points")?;
            if input.len() != 3 {
                return Err(GeometryToolError::InvalidParameters);
            }
            let orientation = orientation_2(input[0], input[1], input[2]);
            Ok(json!({
                "op": op,
                "orientation": match orientation {
                    super::Orientation::Clockwise => "clockwise",
                    super::Orientation::Collinear => "collinear",
                    super::Orientation::CounterClockwise => "counter_clockwise",
                },
                "value": orientation as i8,
            })
            .to_string())
        }
        "convex_hull_2" => {
            let input = points(&value, "points")?;
            let mut scratch = vec![0u32; input.len().saturating_mul(3)];
            let mut indices = vec![0u32; input.len()];
            let count = convex_hull_indices_2(&input, &mut scratch, &mut indices)
                .map_err(|error| GeometryToolError::Geometry(format!("{error:?}")))?;
            indices.truncate(count);
            let hull: Vec<[f64; 2]> = indices
                .iter()
                .map(|&index| {
                    let point = input[index as usize];
                    [point.x, point.y]
                })
                .collect();
            Ok(json!({
                "op": op,
                "indices": indices,
                "points": hull,
                "vertex_count": count,
            })
            .to_string())
        }
        "triangle_topology" => {
            let vertex_count = value
                .get("vertex_count")
                .and_then(Value::as_u64)
                .and_then(|count| u32::try_from(count).ok())
                .ok_or(GeometryToolError::InvalidParameters)?;
            let triangles = triangles(&value)?;
            let mut edges = vec![HalfEdge::default(); triangles.len().saturating_mul(3)];
            let mut slots = vec![EdgeSlot::default(); required_edge_slots(triangles.len())];
            let summary =
                build_triangle_half_edges(vertex_count, &triangles, &mut edges, &mut slots)
                    .map_err(|error| GeometryToolError::Geometry(format!("{error:?}")))?;
            Ok(json!({
                "op": op,
                "summary": summary,
                "half_edges": edges,
            })
            .to_string())
        }
        "mesh_topology" => {
            let vertex_count = value
                .get("vertex_count")
                .and_then(Value::as_u64)
                .and_then(|count| u32::try_from(count).ok())
                .ok_or(GeometryToolError::InvalidParameters)?;
            let tris = triangles(&value)?;
            let face_count = tris.len() as u32;
            let mut edges = vec![HalfEdge::default(); tris.len().saturating_mul(3)];
            let mut slots = vec![EdgeSlot::default(); required_edge_slots(tris.len())];
            build_triangle_half_edges(vertex_count, &tris, &mut edges, &mut slots)
                .map_err(|error| GeometryToolError::Geometry(format!("{error:?}")))?;

            let need = topology_encoded_len(vertex_count, face_count, edges.len() as u32);
            let mut buf = vec![0u8; need];
            let n = encode_topology_section(vertex_count, face_count, &edges, &mut buf)
                .map_err(|error| GeometryToolError::Geometry(format!("{error:?}")))?;
            buf.truncate(n);

            let header = decode_topology_section(&buf)
                .map_err(|error| GeometryToolError::Geometry(format!("{error:?}")))?
                .header;

            let genus = if header.genus == INVALID_INDEX {
                Value::Null
            } else {
                json!(header.genus)
            };

            Ok(json!({
                "op": op,
                "section_bytes": n,
                "header": {
                    "vertex_count": header.vertex_count,
                    "face_count": header.face_count,
                    "half_edge_count": header.half_edge_count,
                    "boundary_loop_count": header.boundary_loop_count,
                    "component_count": header.component_count,
                    "euler_characteristic": header.euler_characteristic,
                    "genus": genus,
                },
            })
            .to_string())
        }
        "delaunay_2" => {
            let input = points(&value, "points")?;
            let n = input.len();
            if n < 3 {
                return Err(GeometryToolError::InvalidParameters);
            }
            let mut scratch = vec![0u32; n];
            let mut tris = vec![[0u32; 3]; 2 * n + 1];
            let count = delaunay_triangulation_2(&input, &mut scratch, &mut tris)
                .map_err(|error| GeometryToolError::Geometry(format!("{error:?}")))?;
            tris.truncate(count);
            Ok(json!({
                "op": op,
                "triangle_count": count,
                "triangles": tris,
            })
            .to_string())
        }
        "voronoi_2" => {
            let input = points(&value, "points")?;
            let n = input.len();
            if n < 3 {
                return Err(GeometryToolError::InvalidParameters);
            }
            let mut tri_scratch = vec![0u32; n];
            let mut tri_out = vec![[0u32; 3]; 2 * n + 1];
            let mut verts = vec![super::VoronoiVertex { triangle_index: 0, center: Point2::new(0.0, 0.0) }; 2 * n + 1];
            let mut edges = vec![super::VoronoiEdge { site_a: 0, site_b: 0, triangle: 0, neighbor_triangle: None }; 3 * n];
            let (vc, ec) = voronoi_diagram_2(&input, &mut tri_scratch, &mut tri_out, &mut verts, &mut edges)
                .map_err(|error| GeometryToolError::Geometry(format!("{error:?}")))?;
            let vert_json: Vec<Value> = verts[..vc].iter().map(|v| json!({
                "triangle": v.triangle_index,
                "center": [v.center.x, v.center.y],
            })).collect();
            let edge_json: Vec<Value> = edges[..ec].iter().map(|e| json!({
                "sites": [e.site_a, e.site_b],
                "triangle": e.triangle,
                "neighbor": e.neighbor_triangle,
            })).collect();
            Ok(json!({
                "op": op,
                "vertex_count": vc,
                "edge_count": ec,
                "vertices": vert_json,
                "edges": edge_json,
            })
            .to_string())
        }
        "nearest_site" => {
            let input = points(&value, "points")?;
            let query_arr = value.get("query")
                .and_then(Value::as_array)
                .ok_or(GeometryToolError::InvalidParameters)?;
            if query_arr.len() < 2 {
                return Err(GeometryToolError::InvalidParameters);
            }
            let qx = query_arr[0].as_f64().ok_or(GeometryToolError::InvalidParameters)?;
            let qy = query_arr[1].as_f64().ok_or(GeometryToolError::InvalidParameters)?;
            let idx = nearest_site_brute_force(&input, Point2::new(qx, qy))
                .ok_or(GeometryToolError::InvalidParameters)?;
            Ok(json!({
                "op": op,
                "nearest_index": idx,
                "nearest_point": [input[idx as usize].x, input[idx as usize].y],
            })
            .to_string())
        }
        "create_box" => {
            let width = value.get("width").and_then(Value::as_f64)
                .ok_or(GeometryToolError::InvalidParameters)?;
            let height = value.get("height").and_then(Value::as_f64)
                .ok_or(GeometryToolError::InvalidParameters)?;
            let depth = value.get("depth").and_then(Value::as_f64)
                .ok_or(GeometryToolError::InvalidParameters)?;
            let mesh = super::authoring::box_mesh(width as f32, height as f32, depth as f32)
                .map_err(|e| GeometryToolError::Geometry(format!("{e}")))?;
            Ok(json!({
                "op": op,
                "vertex_count": mesh.positions.len(),
                "triangle_count": mesh.triangles.len(),
                "positions": mesh.positions,
                "triangles": mesh.triangles,
            })
            .to_string())
        }
        "create_sphere" => {
            let radius = value.get("radius").and_then(Value::as_f64)
                .ok_or(GeometryToolError::InvalidParameters)?;
            let lat = value.get("lat_segments").and_then(Value::as_u64).unwrap_or(8) as u32;
            let lon = value.get("lon_segments").and_then(Value::as_u64).unwrap_or(16) as u32;
            let mesh = super::authoring::uv_sphere(radius as f32, lat, lon)
                .map_err(|e| GeometryToolError::Geometry(format!("{e}")))?;
            Ok(json!({
                "op": op,
                "vertex_count": mesh.positions.len(),
                "triangle_count": mesh.triangles.len(),
                "positions": mesh.positions,
                "triangles": mesh.triangles,
            })
            .to_string())
        }
        "create_cylinder" => {
            let radius = value.get("radius").and_then(Value::as_f64)
                .ok_or(GeometryToolError::InvalidParameters)?;
            let height = value.get("height").and_then(Value::as_f64)
                .ok_or(GeometryToolError::InvalidParameters)?;
            let segments = value.get("segments").and_then(Value::as_u64).unwrap_or(16) as u32;
            let mesh = super::authoring::cylinder(radius as f32, height as f32, segments)
                .map_err(|e| GeometryToolError::Geometry(format!("{e}")))?;
            Ok(json!({
                "op": op,
                "vertex_count": mesh.positions.len(),
                "triangle_count": mesh.triangles.len(),
                "positions": mesh.positions,
                "triangles": mesh.triangles,
            })
            .to_string())
        }
        "create_plane" => {
            let size = value.get("size").and_then(Value::as_f64)
                .ok_or(GeometryToolError::InvalidParameters)?;
            let mesh = super::authoring::plane(size as f32)
                .map_err(|e| GeometryToolError::Geometry(format!("{e}")))?;
            Ok(json!({
                "op": op,
                "vertex_count": mesh.positions.len(),
                "triangle_count": mesh.triangles.len(),
                "positions": mesh.positions,
                "triangles": mesh.triangles,
            })
            .to_string())
        }
        "create_torus" => {
            let major = value.get("major_radius").and_then(Value::as_f64)
                .ok_or(GeometryToolError::InvalidParameters)?;
            let minor = value.get("minor_radius").and_then(Value::as_f64)
                .ok_or(GeometryToolError::InvalidParameters)?;
            let maj_seg = value.get("major_segments").and_then(Value::as_u64).unwrap_or(16) as u32;
            let min_seg = value.get("minor_segments").and_then(Value::as_u64).unwrap_or(8) as u32;
            let mesh = super::authoring::torus(major as f32, minor as f32, maj_seg, min_seg)
                .map_err(|e| GeometryToolError::Geometry(format!("{e}")))?;
            Ok(json!({
                "op": op,
                "vertex_count": mesh.positions.len(),
                "triangle_count": mesh.triangles.len(),
                "positions": mesh.positions,
                "triangles": mesh.triangles,
            })
            .to_string())
        }
        "create_grid" => {
            let size = value.get("size").and_then(Value::as_f64)
                .ok_or(GeometryToolError::InvalidParameters)?;
            let subs = value.get("subdivisions").and_then(Value::as_u64).unwrap_or(4) as u32;
            let mesh = super::authoring::grid(size as f32, subs)
                .map_err(|e| GeometryToolError::Geometry(format!("{e}")))?;
            Ok(json!({
                "op": op,
                "vertex_count": mesh.positions.len(),
                "triangle_count": mesh.triangles.len(),
                "positions": mesh.positions,
                "triangles": mesh.triangles,
            })
            .to_string())
        }
        "boolean_union" | "boolean_intersect" | "boolean_difference" => {
            let bool_op = match op {
                "boolean_union" => super::authoring::BooleanOp::Union,
                "boolean_intersect" => super::authoring::BooleanOp::Intersection,
                _ => super::authoring::BooleanOp::Difference,
            };
            let mesh_a = parse_mesh_json(value.get("mesh_a")
                .ok_or(GeometryToolError::InvalidParameters)?)?;
            let mesh_b = parse_mesh_json(value.get("mesh_b")
                .ok_or(GeometryToolError::InvalidParameters)?)?;
            let result = super::authoring::boolean_op(&mesh_a, &mesh_b, bool_op)
                .map_err(|e| GeometryToolError::Geometry(format!("{e}")))?;
            Ok(json!({
                "op": op,
                "vertex_count": result.positions.len(),
                "triangle_count": result.triangles.len(),
                "positions": result.positions,
                "triangles": result.triangles,
            })
            .to_string())
        }
        "drag_vertex" => {
            let mesh = parse_mesh_json(value.get("mesh")
                .ok_or(GeometryToolError::InvalidParameters)?)?;
            let vi = value.get("vertex_index").and_then(Value::as_u64)
                .ok_or(GeometryToolError::InvalidParameters)? as usize;
            let np = value.get("new_position").and_then(Value::as_array)
                .ok_or(GeometryToolError::InvalidParameters)?;
            if np.len() != 3 {
                return Err(GeometryToolError::InvalidParameters);
            }
            let new_pos = [
                np[0].as_f64().ok_or(GeometryToolError::InvalidParameters)? as f32,
                np[1].as_f64().ok_or(GeometryToolError::InvalidParameters)? as f32,
                np[2].as_f64().ok_or(GeometryToolError::InvalidParameters)? as f32,
            ];
            let prior_t = value.get("prior_t").and_then(Value::as_f64).unwrap_or(0.0) as f32;
            let consent = value.get("consent_granted").and_then(Value::as_bool).unwrap_or(true);
            let drag_consent = super::authoring::DragConsent {
                consent_granted: consent,
                sealed_prior: true,
            };
            match super::authoring::drag_vertex(&mesh, vi, new_pos, prior_t, drag_consent) {
                Ok(result) => Ok(json!({
                    "op": "drag_vertex",
                    "new_t": result.new_t,
                    "prior_t": result.prior_t,
                    "vertex_count": result.mesh.positions.len(),
                    "triangle_count": result.mesh.triangles.len(),
                    "positions": result.mesh.positions,
                    "triangles": result.mesh.triangles,
                })
                .to_string()),
                Err(e) => Ok(json!({
                    "op": "drag_vertex",
                    "error": format!("{e}"),
                    "refused": matches!(e, super::authoring::DragError::GovernanceRefused),
                })
                .to_string()),
            }
        }
        _ => Err(GeometryToolError::InvalidOperation),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_hull_routes_to_native_kernel() {
        let result = execute_geometry_tool_json(
            r#"{"op":"convex_hull_2","points":[[0,0],[1,0],[0.5,0.5],[1,1],[0,1]]}"#,
        )
        .unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["vertex_count"], 4);
        assert_eq!(value["indices"], json!([0, 1, 3, 4]));
    }


    #[test]
    fn json_delaunay_returns_triangles() {
        let result = execute_geometry_tool_json(
            r#"{"op":"delaunay_2","points":[[0,0],[1,0],[1,1],[0,1]]}"#,
        )
        .unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["op"], "delaunay_2");
        assert_eq!(value["triangle_count"], 2);
        let tris = value["triangles"].as_array().unwrap();
        assert_eq!(tris.len(), 2);
    }

    #[test]
    fn json_voronoi_returns_vertices_and_edges() {
        let result = execute_geometry_tool_json(
            r#"{"op":"voronoi_2","points":[[0,0],[2,0],[2,2],[0,2]]}"#,
        )
        .unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["op"], "voronoi_2");
        assert!(value["vertex_count"].as_u64().unwrap() > 0);
        assert!(value["edge_count"].as_u64().unwrap() > 0);
    }

    #[test]
    fn json_nearest_site_returns_index() {
        let result = execute_geometry_tool_json(
            r#"{"op":"nearest_site","points":[[0,0],[3,0],[2,3]],"query":[0.1,0.1]}"#,
        )
        .unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["op"], "nearest_site");
        assert_eq!(value["nearest_index"], 0);
    }

    #[test]
    fn mesh_topology_returns_connectivity_invariants() {
        let result = execute_geometry_tool_json(
            r#"{"op":"mesh_topology","vertex_count":4,"triangles":[[0,1,2],[0,2,3],[0,3,1],[1,3,2]]}"#,
        )
        .unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["op"], "mesh_topology");
        assert_eq!(value["header"]["vertex_count"], 4);
        assert_eq!(value["header"]["face_count"], 4);
        assert_eq!(value["header"]["half_edge_count"], 12);
        assert_eq!(value["header"]["boundary_loop_count"], 0);
        assert_eq!(value["header"]["component_count"], 1);
        assert_eq!(value["header"]["euler_characteristic"], 2);
        assert_eq!(value["header"]["genus"], 0);
        assert!(value["section_bytes"].as_u64().unwrap() > 0);
    }

    #[test]
    fn json_create_box_returns_mesh() {
        let result = execute_geometry_tool_json(
            r#"{"op":"create_box","width":2.0,"height":3.0,"depth":4.0}"#,
        )
        .unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["op"], "create_box");
        assert_eq!(value["vertex_count"], 8);
        assert_eq!(value["triangle_count"], 12);
    }

    #[test]
    fn json_create_sphere_returns_mesh() {
        let result = execute_geometry_tool_json(
            r#"{"op":"create_sphere","radius":1.0,"lat_segments":8,"lon_segments":16}"#,
        )
        .unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["op"], "create_sphere");
        assert!(value["vertex_count"].as_u64().unwrap() > 0);
        assert!(value["triangle_count"].as_u64().unwrap() > 0);
    }

    #[test]
    fn json_create_cylinder_returns_mesh() {
        let result = execute_geometry_tool_json(
            r#"{"op":"create_cylinder","radius":1.0,"height":2.0,"segments":8}"#,
        )
        .unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["op"], "create_cylinder");
        assert_eq!(value["vertex_count"], 18);
    }

    #[test]
    fn json_create_plane_returns_mesh() {
        let result = execute_geometry_tool_json(
            r#"{"op":"create_plane","size":2.0}"#,
        )
        .unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["op"], "create_plane");
        assert_eq!(value["vertex_count"], 4);
        assert_eq!(value["triangle_count"], 2);
    }

    #[test]
    fn json_create_torus_returns_mesh() {
        let result = execute_geometry_tool_json(
            r#"{"op":"create_torus","major_radius":1.0,"minor_radius":0.3,"major_segments":16,"minor_segments":8}"#,
        ).unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["op"], "create_torus");
        assert_eq!(value["vertex_count"], 128);
        assert_eq!(value["triangle_count"], 256);
    }

    #[test]
    fn json_create_grid_returns_mesh() {
        let result = execute_geometry_tool_json(
            r#"{"op":"create_grid","size":2.0,"subdivisions":4}"#,
        ).unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["op"], "create_grid");
        assert_eq!(value["vertex_count"], 25);
        assert_eq!(value["triangle_count"], 32);
    }

    #[test]
    fn json_boolean_union_of_disjoint_cubes() {
        let box_json = r#"{"positions":[[-0.5,-0.5,-0.5],[0.5,-0.5,-0.5],[0.5,0.5,-0.5],[-0.5,0.5,-0.5],[-0.5,-0.5,0.5],[0.5,-0.5,0.5],[0.5,0.5,0.5],[-0.5,0.5,0.5]],"triangles":[[0,1,2],[0,2,3],[4,6,5],[4,7,6],[0,4,5],[0,5,1],[2,6,7],[2,7,3],[0,3,7],[0,7,4],[1,5,6],[1,6,2]]}"#;
        let result = execute_geometry_tool_json(
            &format!(r#"{{"op":"boolean_union","mesh_a":{box_json},"mesh_b":{box_json}}}"#),
        ).unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["op"], "boolean_union");
        assert_eq!(value["triangle_count"], 12, "union of identical cubes = one cube");
    }

    #[test]
    fn json_drag_vertex_produces_new_t_slice() {
        let result = execute_geometry_tool_json(
            r#"{"op":"drag_vertex","mesh":{"positions":[[-0.5,-0.5,-0.5],[0.5,-0.5,-0.5],[0.5,0.5,-0.5],[-0.5,0.5,-0.5],[-0.5,-0.5,0.5],[0.5,-0.5,0.5],[0.5,0.5,0.5],[-0.5,0.5,0.5]],"triangles":[[0,1,2],[0,2,3],[4,6,5],[4,7,6],[0,4,5],[0,5,1],[2,6,7],[2,7,3],[0,3,7],[0,7,4],[1,5,6],[1,6,2]]},"vertex_index":0,"new_position":[1.0,2.0,3.0],"prior_t":10.0,"consent_granted":true}"#,
        ).unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["op"], "drag_vertex");
        assert_eq!(value["new_t"], 11.0);
        assert_eq!(value["prior_t"], 10.0);
    }

    #[test]
    fn json_drag_vertex_governance_refused() {
        let result = execute_geometry_tool_json(
            r#"{"op":"drag_vertex","mesh":{"positions":[[-0.5,-0.5,-0.5],[0.5,-0.5,-0.5],[0.5,0.5,-0.5],[-0.5,0.5,-0.5],[-0.5,-0.5,0.5],[0.5,-0.5,0.5],[0.5,0.5,0.5],[-0.5,0.5,0.5]],"triangles":[[0,1,2],[0,2,3],[4,6,5],[4,7,6],[0,4,5],[0,5,1],[2,6,7],[2,7,3],[0,3,7],[0,7,4],[1,5,6],[1,6,2]]},"vertex_index":0,"new_position":[1.0,2.0,3.0],"prior_t":10.0,"consent_granted":false}"#,
        ).unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["op"], "drag_vertex");
        assert_eq!(value["refused"], true);
    }
}
