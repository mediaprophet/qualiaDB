//! MeshIR → Wavefront OBJ (cold path, for handoff / renderer / dogfood).

use super::geometry_ir::MeshIR;
use crate::specialized_libs::computer_vision::types::VisionError;

/// Write a minimal OBJ into `out` (UTF-8 text). Returns bytes written.
pub fn mesh_ir_to_obj(mesh: &MeshIR, out: &mut [u8]) -> Result<usize, VisionError> {
    let mut s = String::with_capacity(mesh.positions.len() * 48 + mesh.indices.len() * 16);
    s.push_str("# qualia-vision MeshIR export\n");
    for p in &mesh.positions {
        s.push_str(&format!("v {} {} {}\n", p[0], p[1], p[2]));
    }
    for n in &mesh.normals {
        s.push_str(&format!("vn {} {} {}\n", n[0], n[1], n[2]));
    }
    // OBJ is 1-based
    let mut t = 0usize;
    while t + 2 < mesh.indices.len() {
        let a = mesh.indices[t] + 1;
        let b = mesh.indices[t + 1] + 1;
        let c = mesh.indices[t + 2] + 1;
        if mesh.normals.len() == mesh.positions.len() {
            s.push_str(&format!("f {a}//{a} {b}//{b} {c}//{c}\n"));
        } else {
            s.push_str(&format!("f {a} {b} {c}\n"));
        }
        t += 3;
    }
    let bytes = s.as_bytes();
    if out.len() < bytes.len() {
        return Err(VisionError::OutputBufferTooSmall);
    }
    out[..bytes.len()].copy_from_slice(bytes);
    Ok(bytes.len())
}

/// Triangle list as `[u32; 3]` slices (for core Mesh handoff).
pub fn mesh_ir_triangles(mesh: &MeshIR) -> Vec<[u32; 3]> {
    let mut tris = Vec::with_capacity(mesh.triangle_count());
    let mut t = 0usize;
    while t + 2 < mesh.indices.len() {
        tris.push([
            mesh.indices[t],
            mesh.indices[t + 1],
            mesh.indices[t + 2],
        ]);
        t += 3;
    }
    tris
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computer_vision::spatial::geometry_ir::MeshIR;

    #[test]
    fn obj_contains_faces() {
        let mut m = MeshIR::empty();
        m.positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        m.normals = vec![[0.0, 0.0, 1.0]; 3];
        m.indices = vec![0, 1, 2];
        let mut buf = vec![0u8; 4096];
        let n = mesh_ir_to_obj(&m, &mut buf).unwrap();
        let s = std::str::from_utf8(&buf[..n]).unwrap();
        assert!(s.contains("v "));
        assert!(s.contains("f "));
    }
}
