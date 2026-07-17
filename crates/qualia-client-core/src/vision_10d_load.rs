//! F2 + D3 + F3 + F4 — load sealed vision `.10d` into mesh + spectral/acoustic paint.
//!
//! Portal / desktop consume this instead of re-parsing mesh-only. CRC is verified
//! fail-closed. σ → colour via `render::spectral`; σ → Hz via `render::acoustic`.
//! F3: temporal scrub on node `t`. F4: optional citable provenance barrier.

use crate::vision_10d_rights::{
    evaluate_vision_10d_barrier, Vision10dAccess, Vision10dBarrier,
};
use qualia_core_db::container_10d::{
    header::Container10dHeader,
    integrity::verify_whole_file_crc32c,
    node_section::parse_node_header,
    parse_section_table,
    section::SectionType,
};
use qualia_core_db::render::acoustic::sigma_to_center_frequency_hz;
use qualia_core_db::render::assets::Mesh;
use qualia_core_db::render::compile_10d::{compiled_digest, decode_10d_mesh, decode_10d_nodes};
use qualia_core_db::render::spectral::sigma_to_display_rgb;
use qualia_core_db::tensor::Tensor10D;
use serde::Serialize;
use std::path::Path;

/// Spectral + acoustic view of a vision node (D3).
#[derive(Debug, Clone, Copy, Serialize)]
pub struct VisionNodePaint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub t: f32,
    pub sigma: f32,
    pub rgb: [u8; 3],
    pub frequency_hz: f32,
    /// True when q==0 (ground truth). Vision detections use q>0.
    pub ground_truth: bool,
}

/// Loaded vision `.10d` package for browser / portal paint.
#[derive(Debug, Clone, Serialize)]
pub struct Vision10dLoaded {
    pub path: Option<String>,
    pub size_bytes: u64,
    pub compiled_digest_hex: String,
    pub crc_valid: bool,
    pub mesh_vertices: u32,
    pub mesh_triangles: u32,
    pub node_count: u32,
    pub has_topology: bool,
    pub has_spatial_index: bool,
    pub has_provenance: bool,
    pub paint: Vec<VisionNodePaint>,
    /// Mean σ across nodes (for single-colour mesh tint).
    pub mean_sigma: f32,
    pub mean_rgb: [u8; 3],
    pub mean_frequency_hz: f32,
}

/// Decode container bytes into a paint package (browse access: unattested OK).
pub fn load_vision_10d_bytes(bytes: &[u8]) -> Result<(Mesh, Vision10dLoaded), String> {
    load_vision_10d_bytes_with_access(bytes, Vision10dAccess::BrowseAllowUnattested)
}

/// Load with explicit F4 access policy.
pub fn load_vision_10d_bytes_with_access(
    bytes: &[u8],
    access: Vision10dAccess,
) -> Result<(Mesh, Vision10dLoaded), String> {
    match evaluate_vision_10d_barrier(bytes, access) {
        Vision10dBarrier::Permit => {}
        Vision10dBarrier::Deny { reason } => {
            return Err(format!("vision .10d barrier: {reason}"));
        }
    }

    let mut bytes_mut = bytes.to_vec();
    let crc_valid = verify_whole_file_crc32c(&mut bytes_mut).is_ok();
    if !crc_valid {
        return Err("vision .10d: whole-file CRC failed".into());
    }

    let header = Container10dHeader::parse(&bytes_mut).map_err(|e| format!("header: {e}"))?;
    let descs =
        parse_section_table(&bytes_mut, &header).map_err(|e| format!("section table: {e}"))?;

    let mut has_topology = false;
    let mut has_spatial_index = false;
    let mut has_provenance = false;
    for d in descs.iter() {
        match d.typ() {
            Some(SectionType::Topology) => has_topology = true,
            Some(SectionType::SpatialIndex) => has_spatial_index = true,
            Some(SectionType::ProvenanceSidecar) => has_provenance = true,
            _ => {}
        }
    }

    let mesh = decode_10d_mesh(&bytes_mut).map_err(|e| format!("mesh: {e}"))?;

    // Count nodes from header when present.
    let mut node_cap = 0usize;
    for d in descs.iter() {
        if d.typ() == Some(SectionType::Tensor10DNodes) {
            let start = d.byte_offset as usize;
            let end = start.saturating_add(d.byte_length as usize);
            if let Some(payload) = bytes_mut.get(start..end) {
                if let Ok((nh, _)) = parse_node_header(payload) {
                    node_cap = nh.node_count as usize;
                }
            }
        }
    }

    let mut nodes = vec![Tensor10D::default(); node_cap.max(1)];
    let n = if node_cap == 0 {
        0
    } else {
        decode_10d_nodes(&bytes_mut, &mut nodes).unwrap_or(0)
    };
    nodes.truncate(n);

    let paint: Vec<VisionNodePaint> = nodes.iter().map(node_to_paint).collect();
    let mean_sigma = if paint.is_empty() {
        0.35
    } else {
        paint.iter().map(|p| p.sigma).sum::<f32>() / paint.len() as f32
    };
    let (mr, mg, mb) = sigma_to_display_rgb(mean_sigma);
    let mean_hz = sigma_to_center_frequency_hz(mean_sigma);

    let loaded = Vision10dLoaded {
        path: None,
        size_bytes: bytes.len() as u64,
        compiled_digest_hex: format!("{:08x}", compiled_digest(&bytes_mut)),
        crc_valid: true,
        mesh_vertices: mesh.vertex_count() as u32,
        mesh_triangles: mesh.triangle_count() as u32,
        node_count: n as u32,
        has_topology,
        has_spatial_index,
        has_provenance,
        paint,
        mean_sigma,
        mean_rgb: [mr, mg, mb],
        mean_frequency_hz: mean_hz,
    };
    Ok((mesh, loaded))
}

/// Load from storage-relative or absolute path (browse access).
pub fn load_vision_10d_path(
    storage_root: &Path,
    relative_or_abs: &str,
) -> Result<(Mesh, Vision10dLoaded), String> {
    load_vision_10d_path_with_access(
        storage_root,
        relative_or_abs,
        Vision10dAccess::BrowseAllowUnattested,
    )
}

/// Load path with F4 access policy.
pub fn load_vision_10d_path_with_access(
    storage_root: &Path,
    relative_or_abs: &str,
    access: Vision10dAccess,
) -> Result<(Mesh, Vision10dLoaded), String> {
    let p = Path::new(relative_or_abs);
    let full = if p.is_absolute() {
        p.to_path_buf()
    } else {
        storage_root.join(relative_or_abs)
    };
    let bytes = std::fs::read(&full).map_err(|e| format!("read {}: {e}", full.display()))?;
    let (mesh, mut loaded) = load_vision_10d_bytes_with_access(&bytes, access)?;
    loaded.path = Some(
        full.strip_prefix(storage_root)
            .ok()
            .and_then(|x| x.to_str())
            .unwrap_or(relative_or_abs)
            .to_string(),
    );
    Ok((mesh, loaded))
}

/// F3 — temporal scrub: keep paint nodes with `t` in `[t_slice ± t_window/2]`.
///
/// Returns indices into the original `paint` slice (caller filters).
pub fn temporal_scrub_paint(
    paint: &[VisionNodePaint],
    t_slice: f32,
    t_window: f32,
    out_indices: &mut [u32],
) -> usize {
    let half = t_window * 0.5;
    let lo = t_slice - half;
    let hi = t_slice + half;
    let mut n = 0usize;
    for (i, p) in paint.iter().enumerate() {
        if p.t >= lo && p.t <= hi {
            if n < out_indices.len() {
                out_indices[n] = i as u32;
                n += 1;
            }
        }
    }
    n
}

/// F3 — filter paint into a new Vec (cold path).
pub fn temporal_scrub_paint_vec(
    paint: &[VisionNodePaint],
    t_slice: f32,
    t_window: f32,
) -> Vec<VisionNodePaint> {
    let half = t_window * 0.5;
    let lo = t_slice - half;
    let hi = t_slice + half;
    paint
        .iter()
        .copied()
        .filter(|p| p.t >= lo && p.t <= hi)
        .collect()
}

/// D3: map one Tensor10D to spectral RGB + acoustic Hz.
pub fn node_to_paint(t: &Tensor10D) -> VisionNodePaint {
    let (r, g, b) = sigma_to_display_rgb(t.sigma);
    VisionNodePaint {
        x: t.x,
        y: t.y,
        z: t.z,
        t: t.t,
        sigma: t.sigma,
        rgb: [r, g, b],
        frequency_hz: sigma_to_center_frequency_hz(t.sigma),
        ground_truth: t.is_ground_truth(),
    }
}

/// Per-vertex colours for a mesh, painted by nearest vision node σ (or mean).
pub fn mesh_vertex_colors_from_nodes(
    mesh: &Mesh,
    nodes: &[Tensor10D],
) -> Vec<[f32; 4]> {
    let paints: Vec<_> = nodes.iter().map(node_to_paint).collect();
    if paints.is_empty() {
        let (r, g, b) = sigma_to_display_rgb(0.35);
        return vec![
            [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0];
            mesh.vertex_count()
        ];
    }
    mesh.positions
        .iter()
        .map(|p| {
            let mut best = 0usize;
            let mut best_d = f32::INFINITY;
            for (i, n) in paints.iter().enumerate() {
                // Nodes may be in normalised image space; blend with mesh-space
                // when magnitudes look like unit square, else use mean.
                let dx = p[0] - n.x;
                let dy = p[1] - n.y;
                let dz = p[2] - n.z;
                let d = dx * dx + dy * dy + dz * dz;
                if d < best_d {
                    best_d = d;
                    best = i;
                }
            }
            let c = paints[best].rgb;
            [
                c[0] as f32 / 255.0,
                c[1] as f32 / 255.0,
                c[2] as f32 / 255.0,
                1.0,
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_core_db::container_10d::provenance_section::ProvenanceSidecar;
    use qualia_core_db::render::compile_10d::{
        compile_mesh_to_10d_vision, compile_mesh_to_10d_vision_with_provenance,
    };

    #[test]
    fn load_vision_seal_with_paint() {
        let mesh = Mesh {
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
            min: [0.0, 0.0, 0.0],
            max: [1.0, 1.0, 1.0],
        };
        let nodes = [
            Tensor10D::parallel_context(1.0, 0.0, 0.0, 0.2, 0.2, 0.0, 0.0, 1.0, 0.0, 0.2),
            Tensor10D::parallel_context(1.0, 0.0, 0.0, 0.8, 0.8, 0.0, 1.0, 1.0, 0.0, 0.8),
        ];
        let bytes = compile_mesh_to_10d_vision(&mesh, &nodes).unwrap();
        let (m, loaded) = load_vision_10d_bytes(&bytes).unwrap();
        assert_eq!(m.triangle_count(), 2);
        assert_eq!(loaded.node_count, 2);
        assert_eq!(loaded.paint.len(), 2);
        assert!(loaded.crc_valid);
        assert!(loaded.mean_frequency_hz > 0.0);
        let colors = mesh_vertex_colors_from_nodes(&m, &nodes);
        assert_eq!(colors.len(), 4);
        // Host product path includes C3 extras (portal wasm may skip them).
        assert!(loaded.has_topology, "expected Topology on host vision seal");
        assert!(
            loaded.has_spatial_index,
            "expected SpatialIndex on host vision seal"
        );
    }

    #[test]
    fn temporal_scrub_keeps_window() {
        let paint = [
            VisionNodePaint {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                t: 0.0,
                sigma: 0.1,
                rgb: [0, 0, 0],
                frequency_hz: 100.0,
                ground_truth: false,
            },
            VisionNodePaint {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                t: 5.0,
                sigma: 0.2,
                rgb: [0, 0, 0],
                frequency_hz: 100.0,
                ground_truth: false,
            },
            VisionNodePaint {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                t: 10.0,
                sigma: 0.3,
                rgb: [0, 0, 0],
                frequency_hz: 100.0,
                ground_truth: false,
            },
        ];
        let kept = temporal_scrub_paint_vec(&paint, 5.0, 2.0);
        assert_eq!(kept.len(), 1);
        assert!((kept[0].t - 5.0).abs() < 1e-5);
        let mut idx = [0u32; 8];
        let n = temporal_scrub_paint(&paint, 5.0, 12.0, &mut idx);
        assert_eq!(n, 3);
    }

    #[test]
    fn citable_load_requires_provenance() {
        let mesh = Mesh {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            triangles: vec![[0, 1, 2]],
            min: [0.0, 0.0, 0.0],
            max: [1.0, 1.0, 0.0],
        };
        let nodes = [Tensor10D::default()];
        let bare = compile_mesh_to_10d_vision(&mesh, &nodes).unwrap();
        assert!(load_vision_10d_bytes_with_access(
            &bare,
            Vision10dAccess::CitableRequireProvenance
        )
        .is_err());
        let prov = ProvenanceSidecar::new(b"src", "image/rgb8", "CC0");
        let sealed =
            compile_mesh_to_10d_vision_with_provenance(&mesh, &nodes, &prov).unwrap();
        let (_m, loaded) = load_vision_10d_bytes_with_access(
            &sealed,
            Vision10dAccess::CitableRequireProvenance,
        )
        .unwrap();
        assert!(loaded.has_provenance);
    }
}
