//! C2 — MeshIR quality cleanup (vision-side, no core-db CG link).
//!
//! Removes degenerate triangles and optional near-duplicate vertices. Full QEM
//! remesh remains on the host via computational_geometry (optional recipe).

use super::geometry_ir::MeshIR;
use crate::cv::error::CvError;

/// Report from quality cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshQualityReport {
    pub vertices_in: u32,
    pub vertices_out: u32,
    pub triangles_in: u32,
    pub triangles_out: u32,
    pub degenerates_removed: u32,
    pub vertices_welded: u32,
}

/// Options for [`cleanup_mesh_ir`].
#[derive(Debug, Clone, Copy)]
pub struct MeshCleanupOptions {
    /// Weld vertices closer than this Euclidean distance (0 = no weld).
    pub weld_epsilon: f32,
    /// Drop faces with area below this (world units²). 0 keeps only exact degenerates.
    pub min_area: f32,
}

impl Default for MeshCleanupOptions {
    fn default() -> Self {
        Self {
            weld_epsilon: 0.0,
            min_area: 0.0,
        }
    }
}

/// Clean a MeshIR in place: weld (optional) then drop degenerate triangles.
///
/// Fail-closed if the mesh becomes empty.
pub fn cleanup_mesh_ir(mesh: &mut MeshIR, opts: MeshCleanupOptions) -> Result<MeshQualityReport, CvError> {
    let vertices_in = mesh.vertex_count() as u32;
    let triangles_in = mesh.triangle_count() as u32;
    if mesh.positions.is_empty() || mesh.indices.len() < 3 {
        return Err(CvError::EmptyInput);
    }

    let mut vertices_welded = 0u32;
    if opts.weld_epsilon > 0.0 {
        vertices_welded = weld_vertices(mesh, opts.weld_epsilon);
    }

    let mut new_idx = Vec::with_capacity(mesh.indices.len());
    let mut degenerates_removed = 0u32;
    let min_area2 = opts.min_area * opts.min_area;

    let n_tri = mesh.indices.len() / 3;
    for t in 0..n_tri {
        let i0 = mesh.indices[t * 3] as usize;
        let i1 = mesh.indices[t * 3 + 1] as usize;
        let i2 = mesh.indices[t * 3 + 2] as usize;
        if i0 >= mesh.positions.len()
            || i1 >= mesh.positions.len()
            || i2 >= mesh.positions.len()
            || i0 == i1
            || i1 == i2
            || i0 == i2
        {
            degenerates_removed += 1;
            continue;
        }
        let a = mesh.positions[i0];
        let b = mesh.positions[i1];
        let c = mesh.positions[i2];
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let cx = ab[1] * ac[2] - ab[2] * ac[1];
        let cy = ab[2] * ac[0] - ab[0] * ac[2];
        let cz = ab[0] * ac[1] - ab[1] * ac[0];
        let area2 = cx * cx + cy * cy + cz * cz;
        // area = 0.5 * |cross|; compare area² against min_area² → |cross|² >= 4 * min_area²
        let thr = if opts.min_area > 0.0 {
            4.0 * min_area2
        } else {
            0.0
        };
        if area2 <= thr {
            degenerates_removed += 1;
            continue;
        }
        new_idx.push(i0 as u32);
        new_idx.push(i1 as u32);
        new_idx.push(i2 as u32);
    }

    if new_idx.len() < 3 {
        return Err(CvError::InvalidParameter);
    }
    mesh.indices = new_idx;
    // Drop unreferenced vertices.
    compact_unreferenced(mesh);
    mesh.recompute_bounds_and_hash();

    Ok(MeshQualityReport {
        vertices_in,
        vertices_out: mesh.vertex_count() as u32,
        triangles_in,
        triangles_out: mesh.triangle_count() as u32,
        degenerates_removed,
        vertices_welded,
    })
}

fn weld_vertices(mesh: &mut MeshIR, eps: f32) -> u32 {
    let n = mesh.positions.len();
    if n == 0 {
        return 0;
    }
    let mut remap = vec![0u32; n];
    let mut keep: Vec<[f32; 3]> = Vec::with_capacity(n);
    let eps2 = eps * eps;
    let mut welded = 0u32;
    for i in 0..n {
        let p = mesh.positions[i];
        let mut found = None;
        for (k, q) in keep.iter().enumerate() {
            let dx = p[0] - q[0];
            let dy = p[1] - q[1];
            let dz = p[2] - q[2];
            if dx * dx + dy * dy + dz * dz <= eps2 {
                found = Some(k as u32);
                break;
            }
        }
        if let Some(k) = found {
            remap[i] = k;
            welded += 1;
        } else {
            remap[i] = keep.len() as u32;
            keep.push(p);
        }
    }
    if welded == 0 {
        return 0;
    }
    for idx in &mut mesh.indices {
        *idx = remap[*idx as usize];
    }
    // Remap optional attributes by first occurrence of each keep slot.
    if mesh.normals.len() == n {
        let mut nn = vec![[0.0f32; 3]; keep.len()];
        let mut filled = vec![false; keep.len()];
        for i in 0..n {
            let k = remap[i] as usize;
            if !filled[k] {
                nn[k] = mesh.normals[i];
                filled[k] = true;
            }
        }
        mesh.normals = nn;
    } else {
        mesh.normals.clear();
    }
    if mesh.uvs.len() == n {
        let mut uu = vec![[0.0f32; 2]; keep.len()];
        let mut filled = vec![false; keep.len()];
        for i in 0..n {
            let k = remap[i] as usize;
            if !filled[k] {
                uu[k] = mesh.uvs[i];
                filled[k] = true;
            }
        }
        mesh.uvs = uu;
    } else {
        mesh.uvs.clear();
    }
    mesh.positions = keep;
    welded
}

fn compact_unreferenced(mesh: &mut MeshIR) {
    let n = mesh.positions.len();
    let mut used = vec![false; n];
    for &i in &mesh.indices {
        if (i as usize) < n {
            used[i as usize] = true;
        }
    }
    let mut remap = vec![u32::MAX; n];
    let mut new_pos = Vec::new();
    let mut new_n = Vec::new();
    let mut new_uv = Vec::new();
    let has_n = mesh.normals.len() == n;
    let has_uv = mesh.uvs.len() == n;
    for i in 0..n {
        if !used[i] {
            continue;
        }
        remap[i] = new_pos.len() as u32;
        new_pos.push(mesh.positions[i]);
        if has_n {
            new_n.push(mesh.normals[i]);
        }
        if has_uv {
            new_uv.push(mesh.uvs[i]);
        }
    }
    for idx in &mut mesh.indices {
        *idx = remap[*idx as usize];
    }
    mesh.positions = new_pos;
    mesh.normals = new_n;
    mesh.uvs = new_uv;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_degenerate_face() {
        let mut m = MeshIR::empty();
        m.positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
        ];
        // good tri + collapsed tri (duplicate verts)
        m.indices = vec![0, 1, 2, 0, 0, 1];
        m.recompute_bounds_and_hash();
        let r = cleanup_mesh_ir(&mut m, MeshCleanupOptions::default()).unwrap();
        assert_eq!(r.triangles_out, 1);
        assert_eq!(r.degenerates_removed, 1);
        assert_eq!(m.triangle_count(), 1);
    }

    #[test]
    fn weld_close_verts() {
        let mut m = MeshIR::empty();
        m.positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0001, 0.0, 0.0], // near v0
        ];
        m.indices = vec![0, 1, 2, 3, 1, 2];
        m.recompute_bounds_and_hash();
        let r = cleanup_mesh_ir(
            &mut m,
            MeshCleanupOptions {
                weld_epsilon: 0.001,
                min_area: 0.0,
            },
        )
        .unwrap();
        assert!(r.vertices_welded >= 1);
        assert!(m.vertex_count() <= 3);
        // Both faces remain geometrically valid after weld (same three verts twice).
        assert!(m.triangle_count() >= 1 && m.triangle_count() <= 2);
    }
}
