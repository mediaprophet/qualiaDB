//! MeshIR → render-ready export (for `compile_10d` on the host).

use super::export_obj::mesh_ir_triangles;
use super::geometry_ir::MeshIR;
use super::validate::{validate_mesh_ir, MeshValidationStatus};
use crate::cv::error::CvError;

/// Geometry export matching `qualia_core_db::render::assets::Mesh` field layout
/// without linking core-db into the vision crate.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderMeshExport {
    pub positions: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub content_hash: u64,
}

/// Convert MeshIR to export (no validation).
pub fn mesh_ir_to_export(ir: &MeshIR) -> Result<RenderMeshExport, CvError> {
    if ir.positions.is_empty() || ir.indices.len() < 3 {
        return Err(CvError::EmptyInput);
    }
    let triangles = mesh_ir_triangles(ir);
    if triangles.is_empty() {
        return Err(CvError::InvalidParameter);
    }
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in &ir.positions {
        for k in 0..3 {
            min[k] = min[k].min(p[k]);
            max[k] = max[k].max(p[k]);
        }
    }
    Ok(RenderMeshExport {
        positions: ir.positions.clone(),
        triangles,
        min,
        max,
        content_hash: ir.content_hash,
    })
}

/// Validate then export. Fails closed on invalid mesh.
pub fn mesh_ir_to_export_validated(ir: &MeshIR) -> Result<RenderMeshExport, CvError> {
    let rep = validate_mesh_ir(ir);
    if rep.status != MeshValidationStatus::Valid {
        return Err(CvError::InvalidParameter);
    }
    mesh_ir_to_export(ir)
}

/// Hint for future Tensor10D node packing (x,y,z,t,σ) — programme D1/D2.
#[derive(Debug, Clone, Copy)]
pub struct NodeHint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub t: f32,
    /// Spectral / EMF index 0..1 (caller maps class→σ).
    pub sigma: f32,
}

/// Map a detection-like box centre (normalised 0..1) into a node hint.
pub fn detection_center_to_node_hint(
    cx: f32,
    cy: f32,
    frame_t: f32,
    sigma: f32,
) -> NodeHint {
    NodeHint {
        x: cx,
        y: cy,
        z: 0.0,
        t: frame_t,
        sigma: sigma.clamp(0.0, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial::export_obj::mesh_ir_triangles;
    use crate::spatial::geometry_ir::MeshIR;

    fn tri_mesh() -> MeshIR {
        let mut m = MeshIR::empty();
        m.positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        m.indices = vec![0, 1, 2];
        m.recompute_bounds_and_hash();
        m
    }

    #[test]
    fn export_one_triangle() {
        let m = tri_mesh();
        let e = mesh_ir_to_export(&m).unwrap();
        assert_eq!(e.positions.len(), 3);
        assert_eq!(e.triangles.len(), 1);
        assert_eq!(e.triangles[0], [0, 1, 2]);
    }

    #[test]
    fn export_round_trip_fields() {
        let mut m = MeshIR::empty();
        m.positions = vec![
            [-1.0, 0.5, 2.0],
            [3.0, -2.0, 0.0],
            [0.0, 1.0, -0.5],
            [1.0, 1.0, 1.0],
        ];
        m.indices = vec![0, 1, 2, 0, 2, 3];
        m.recompute_bounds_and_hash();

        let e = mesh_ir_to_export(&m).unwrap();
        assert_eq!(e.positions, m.positions);
        assert_eq!(e.triangles, mesh_ir_triangles(&m));
        assert_eq!(e.triangles.len(), 2);
        assert_eq!(e.min, m.bounds.min);
        assert_eq!(e.max, m.bounds.max);
        assert_eq!(e.content_hash, m.content_hash);
        assert_eq!(e.min, [-1.0, -2.0, -0.5]);
        assert_eq!(e.max, [3.0, 1.0, 2.0]);
    }

    #[test]
    fn validated_export_accepts_valid_mesh() {
        let m = tri_mesh();
        let e = mesh_ir_to_export_validated(&m).unwrap();
        assert_eq!(e.triangles[0], [0, 1, 2]);
        assert_eq!(e.min, [0.0, 0.0, 0.0]);
        assert_eq!(e.max, [1.0, 1.0, 0.0]);
    }

    #[test]
    fn validated_export_rejects_empty() {
        assert!(mesh_ir_to_export_validated(&MeshIR::empty()).is_err());
    }

    #[test]
    fn empty_fails() {
        assert!(mesh_ir_to_export(&MeshIR::empty()).is_err());
    }

    #[test]
    fn node_hint_clamps_sigma() {
        let h = detection_center_to_node_hint(0.25, 0.75, 3.0, 1.5);
        assert_eq!(h.z, 0.0);
        assert_eq!(h.sigma, 1.0);
        assert_eq!(h.t, 3.0);
    }
}
