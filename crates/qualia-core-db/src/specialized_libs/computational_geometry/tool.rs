//! Cold JSON boundary shared by MCP and desktop/qapp hosts.

use serde_json::{json, Value};

use super::{
    build_triangle_half_edges, convex_hull_indices_2, orientation_2, required_edge_slots,
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
            let mut scratch = vec![0u32; input.len().saturating_mul(2)];
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
        "package_inventory" => {
            let ported = super::generated::CGAL_PACKAGES
                .iter()
                .filter(|package| package.status != super::generated::PortStatus::Planned)
                .count();
            Ok(json!({
                "op": op,
                "upstream_ref": super::generated::CGAL_UPSTREAM_REF,
                "upstream_commit": super::generated::CGAL_UPSTREAM_COMMIT,
                "package_count": super::generated::CGAL_PACKAGES.len(),
                "started_count": ported,
                "packages": super::generated::CGAL_PACKAGES.iter().map(|package| json!({
                    "name": package.upstream_name,
                    "module": package.rust_module,
                    "license": package.upstream_license,
                    "status": format!("{:?}", package.status).to_lowercase(),
                    "doc_files": package.doc_files,
                    "test_files": package.test_files,
                })).collect::<Vec<_>>(),
            })
            .to_string())
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
    fn package_inventory_covers_upstream_surface() {
        let result = execute_geometry_tool_json(r#"{"op":"package_inventory"}"#).unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert!(value["package_count"].as_u64().unwrap() >= 100);
        assert_eq!(value["upstream_ref"], "v6.2");
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
}
