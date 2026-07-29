//! F1 — list / inspect sealed vision `.10d` assets under storage.
//!
//! Scans `{storage}/vision_geometry/**/recon.10d` (and sibling `.10d`) for
//! Library / desktop browse. Does not invent digests; reports file facts only.

use qualia_core_db::container_10d::{
    header::Container10dHeader, integrity::verify_whole_file_crc32c,
    mesh_section::decode_mesh_section, node_section::parse_node_header, parse_section_table,
    section::SectionType,
};
use qualia_core_db::render::compile_10d::{compiled_digest, decode_10d_mesh};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct Vision10dEntry {
    pub path: String,
    pub filename: String,
    pub media_digest_hex: Option<String>,
    pub size_bytes: u64,
    pub section_count: u32,
    pub has_mesh: bool,
    pub has_tensor_nodes: bool,
    pub has_provenance: bool,
    pub crc_valid: bool,
    pub compiled_digest_hex: Option<String>,
    pub mesh_vertices: Option<u32>,
    pub mesh_triangles: Option<u32>,
    pub node_count: Option<u32>,
}

/// List vision recon containers under `{storage_root}/vision_geometry`.
pub fn list_vision_10d_containers(storage_root: &Path) -> Result<Vec<Vision10dEntry>, String> {
    let root = storage_root.join("vision_geometry");
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    scan_vision_dir(&root, storage_root, &mut out)?;
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn scan_vision_dir(dir: &Path, base: &Path, out: &mut Vec<Vision10dEntry>) -> Result<(), String> {
    let rd = std::fs::read_dir(dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
    for ent in rd.flatten() {
        let path = ent.path();
        if path.is_dir() {
            scan_vision_dir(&path, base, out)?;
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("10d") {
            continue;
        }
        out.push(inspect_vision_10d_path(&path, base)?);
    }
    Ok(())
}

/// Inspect one `.10d` path relative to storage (or absolute under storage).
pub fn inspect_vision_10d(
    storage_root: &Path,
    relative_or_abs: &str,
) -> Result<Vision10dEntry, String> {
    let p = PathBuf::from(relative_or_abs);
    let full = if p.is_absolute() {
        p
    } else {
        storage_root.join(relative_or_abs)
    };
    inspect_vision_10d_path(&full, storage_root)
}

fn inspect_vision_10d_path(path: &Path, base: &Path) -> Result<Vision10dEntry, String> {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("?")
        .to_string();
    let size_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let relative = path
        .strip_prefix(base)
        .ok()
        .and_then(|p| p.to_str())
        .unwrap_or(&filename)
        .to_string();

    // Parent folder name is often the media digest hex from Gs continuum.
    let media_digest_hex = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .filter(|s| s.len() >= 8 && s.chars().all(|c| c.is_ascii_hexdigit()))
        .map(|s| s.to_string());

    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut bytes_mut = bytes.clone();
    let crc_valid = verify_whole_file_crc32c(&mut bytes_mut).is_ok();

    let (section_count, has_mesh, has_tensor_nodes, has_provenance, node_count, mesh_v, mesh_t) =
        match Container10dHeader::parse(&bytes) {
            Ok(header) => {
                let descs = parse_section_table(&bytes, &header).ok();
                let mut hm = false;
                let mut ht = false;
                let mut hp = false;
                let mut nc = None;
                let mut mv = None;
                let mut mt = None;
                if let Some(ref descs) = descs {
                    for d in descs.iter() {
                        match d.typ() {
                            Some(SectionType::QuantizedMesh) => {
                                hm = true;
                                let start = d.byte_offset as usize;
                                let end = start.saturating_add(d.byte_length as usize);
                                if let Some(payload) = bytes.get(start..end) {
                                    if let Ok(m) = decode_mesh_section(payload) {
                                        mv = Some(m.vertex_count() as u32);
                                        mt = Some(m.triangle_count() as u32);
                                    }
                                }
                            }
                            Some(SectionType::Tensor10DNodes) => {
                                ht = true;
                                let start = d.byte_offset as usize;
                                let end = start.saturating_add(d.byte_length as usize);
                                if let Some(payload) = bytes.get(start..end) {
                                    if let Ok((nh, _)) = parse_node_header(payload) {
                                        nc = Some(nh.node_count);
                                    }
                                }
                            }
                            Some(SectionType::ProvenanceSidecar) => hp = true,
                            _ => {}
                        }
                    }
                }
                (header.section_count, hm, ht, hp, nc, mv, mt)
            }
            Err(_) => (0, false, false, false, None, None, None),
        };

    // Prefer decode_10d_mesh counts when header parse path missed them.
    let (mesh_vertices, mesh_triangles) = if mesh_v.is_some() {
        (mesh_v, mesh_t)
    } else if let Ok(m) = decode_10d_mesh(&bytes) {
        (
            Some(m.vertex_count() as u32),
            Some(m.triangle_count() as u32),
        )
    } else {
        (None, None)
    };

    let compiled_digest_hex = if crc_valid || has_mesh {
        Some(format!("{:08x}", compiled_digest(&bytes)))
    } else {
        None
    };

    Ok(Vision10dEntry {
        path: relative,
        filename,
        media_digest_hex,
        size_bytes,
        section_count,
        has_mesh,
        has_tensor_nodes,
        has_provenance,
        crc_valid,
        compiled_digest_hex,
        mesh_vertices,
        mesh_triangles,
        node_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_core_db::render::assets::Mesh;
    use qualia_core_db::render::compile_10d::compile_mesh_to_10d_with_nodes;
    use qualia_core_db::tensor::Tensor10D;
    use std::fs;

    #[test]
    fn lists_recon_under_vision_geometry() {
        let dir = std::env::temp_dir().join(format!("vision_10d_browse_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let digest = "aabbccddeeff0011";
        let recon = dir.join("vision_geometry").join(digest);
        fs::create_dir_all(&recon).unwrap();
        let mesh = Mesh {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            triangles: vec![[0, 1, 2]],
            min: [0.0, 0.0, 0.0],
            max: [1.0, 1.0, 0.0],
        };
        let nodes = [Tensor10D::ground_truth(
            0.0, 0.0, 0.5, 0.5, 0.0, 0.0, 1.0, 0.0, 0.3,
        )];
        let bytes = compile_mesh_to_10d_with_nodes(&mesh, &nodes).unwrap();
        fs::write(recon.join("recon.10d"), &bytes).unwrap();

        let list = list_vision_10d_containers(&dir).unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].has_mesh);
        assert!(list[0].has_tensor_nodes);
        assert_eq!(list[0].node_count, Some(1));
        assert_eq!(list[0].media_digest_hex.as_deref(), Some(digest));
        assert!(list[0].crc_valid);
        let _ = fs::remove_dir_all(&dir);
    }
}
