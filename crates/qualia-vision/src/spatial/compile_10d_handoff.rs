//! Host handoff notes for sealing vision geometry into `.10d`.
//!
//! # Where sealing happens
//!
//! `qualia-vision` **does not** link `qualia-core-db` and must not call
//! `compile_10d` itself (keeps the vision WASM / edge surface free of the
//! full render stack). On the **host** (desktop, client-core, studio):
//!
//! 1. Build / validate a [`MeshIR`](super::geometry_ir::MeshIR).
//! 2. Convert with [`mesh_ir_to_export`](super::mesh_ir_to_export::mesh_ir_to_export)
//!    or the validated variant.
//! 3. Pack fields with [`pack_geometry_export_for_10d`].
//! 4. Construct `qualia_core_db::render::assets::Mesh` from those fields.
//! 5. Seal with `qualia_core_db::render::compile_10d::compile_mesh_to_10d`.
//!
//! Programme D1/D2 may later project detections into Tensor10D nodes; see
//! [`detections_to_node_hints`].

use super::mesh_ir_to_export::{detection_center_to_node_hint, NodeHint, RenderMeshExport};
use super::sigma_map::detection_to_sigma;
use crate::types::Detection;

/// Geometry fields layout-compatible with `qualia_core_db::render::assets::Mesh`.
///
/// Host code maps this 1:1 into `Mesh { positions, triangles, min, max }`.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryFor10d {
    pub positions: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// Pack a [`RenderMeshExport`] into fields ready for host `Mesh` construction.
///
/// Does not allocate beyond cloning the export vectors (cold path).
#[inline]
pub fn pack_geometry_export_for_10d(export: &RenderMeshExport) -> GeometryFor10d {
    GeometryFor10d {
        positions: export.positions.clone(),
        triangles: export.triangles.clone(),
        min: export.min,
        max: export.max,
    }
}

/// Map detections to Tensor10D-style node hints (D1/D2).
///
/// - `x,y` = box centre in normalised 0..1 space (`u16` / 65535)
/// - `z` = 0 (image-plane stub)
/// - `t` = `frame_index` as f32
/// - `sigma` = class×score map ([`detection_to_sigma`]) for spectral paint
///
/// Writes at most `out.len()` hints; returns count written.
pub fn detections_to_node_hints(dets: &[Detection], out: &mut [NodeHint]) -> usize {
    let n = dets.len().min(out.len());
    for i in 0..n {
        let d = &dets[i];
        let cx = (d.x_min_u16 as f32 + d.x_max_u16 as f32) * 0.5 / 65535.0;
        let cy = (d.y_min_u16 as f32 + d.y_max_u16 as f32) * 0.5 / 65535.0;
        out[i] = detection_center_to_node_hint(cx, cy, d.frame_index as f32, detection_to_sigma(d));
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial::geometry_ir::MeshIR;
    use crate::spatial::mesh_ir_to_export::mesh_ir_to_export;
    use crate::types::Detection;

    #[test]
    fn pack_preserves_mesh_fields() {
        let mut m = MeshIR::empty();
        m.positions = vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 3.0, 1.0]];
        m.indices = vec![0, 1, 2];
        m.recompute_bounds_and_hash();
        let e = mesh_ir_to_export(&m).unwrap();
        let g = pack_geometry_export_for_10d(&e);
        assert_eq!(g.positions, e.positions);
        assert_eq!(g.triangles, e.triangles);
        assert_eq!(g.min, e.min);
        assert_eq!(g.max, e.max);
        assert_eq!(g.min, [0.0, 0.0, 0.0]);
        assert_eq!(g.max, [2.0, 3.0, 1.0]);
    }

    #[test]
    fn detections_map_score_to_sigma() {
        let mut d = Detection::empty();
        d.x_min_u16 = 0;
        d.x_max_u16 = 65535;
        d.y_min_u16 = 0;
        d.y_max_u16 = 65535;
        d.score_u16 = 32768; // ~0.5
        d.class_hash = 4;
        d.frame_index = 7;
        let mut out = [NodeHint {
            x: 0.0,
            y: 0.0,
            z: 1.0,
            t: 0.0,
            sigma: 0.0,
        }; 2];
        let n = detections_to_node_hints(&[d], &mut out);
        assert_eq!(n, 1);
        assert!((out[0].x - 0.5).abs() < 1e-4);
        assert!((out[0].y - 0.5).abs() < 1e-4);
        assert_eq!(out[0].z, 0.0);
        assert_eq!(out[0].t, 7.0);
        let expected = crate::spatial::sigma_map::detection_to_sigma(&d);
        assert!((out[0].sigma - expected).abs() < 1e-5);
    }
}
