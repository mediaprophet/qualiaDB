//! MeshIR → glTF 2.0 binary (GLB) export (cold path; pure Rust, no dependency).
//!
//! Emits a single-buffer GLB: a 12-byte header, a JSON chunk describing one
//! scene/node/mesh/primitive with a VEC3 f32 POSITION accessor (with required
//! `min`/`max`) and a SCALAR u32 index accessor (componentType 5125), and a BIN
//! chunk holding the little-endian vertex positions followed by the indices.
//!
//! This is a cold, caller-buffered path (`String`/`Vec` are fine here); the hot
//! edge path never touches it. Fails closed to `CvError` — never panics — on an
//! empty mesh, out-of-range indices, or a too-small output buffer.

use super::geometry_ir::MeshIR;
use crate::specialized_libs::computer_vision::cv::error::CvError;

// GLB container constants (little-endian on the wire).
const GLB_MAGIC: u32 = 0x4654_6C67; // "glTF"
const GLB_VERSION: u32 = 2;
const CHUNK_TYPE_JSON: u32 = 0x4E4F_534A; // "JSON"
const CHUNK_TYPE_BIN: u32 = 0x004E_4942; // "BIN\0"

// glTF accessor / bufferView enums.
const COMPONENT_TYPE_FLOAT: u32 = 5126; // f32
const COMPONENT_TYPE_UINT: u32 = 5125; // u32
const TARGET_ARRAY_BUFFER: u32 = 34962; // vertex attributes
const TARGET_ELEMENT_ARRAY_BUFFER: u32 = 34963; // indices
const PRIMITIVE_MODE_TRIANGLES: u32 = 4;

/// Write a valid binary glTF 2.0 (GLB) describing `mesh` into `out`.
///
/// Returns the number of bytes written, which equals the total length stored in
/// the GLB header. Positions are written as VEC3 f32 and indices as SCALAR u32
/// (componentType 5125), one triangle primitive (mode 4). The position accessor
/// carries the glTF-required `min`/`max` bounds.
pub fn mesh_ir_to_glb(mesh: &MeshIR, out: &mut [u8]) -> Result<usize, CvError> {
    let n_verts = mesh.positions.len();
    let n_indices = mesh.indices.len();
    if n_verts == 0 || n_indices == 0 {
        return Err(CvError::EmptyInput);
    }
    // Triangle list only.
    if n_indices % 3 != 0 {
        return Err(CvError::InvalidParameter);
    }
    // Every index must reference a real vertex.
    for &i in &mesh.indices {
        if i as usize >= n_verts {
            return Err(CvError::InvalidParameter);
        }
    }

    // --- Per-component min/max over positions (glTF requires them). ---
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in &mesh.positions {
        for c in 0..3 {
            if p[c] < min[c] {
                min[c] = p[c];
            }
            if p[c] > max[c] {
                max[c] = p[c];
            }
        }
    }

    // --- BIN chunk layout (positions, then indices; both 4-byte aligned). ---
    let pos_len = n_verts * 3 * 4; // VEC3 f32, multiple of 4
    let idx_len = n_indices * 4; // SCALAR u32, multiple of 4
    let idx_offset = pos_len; // pos_len is a multiple of 12 → 4-aligned
    let bin_len = pos_len + idx_len; // already 4-aligned

    // --- JSON chunk (built as a plain String; cold path). ---
    let json = build_gltf_json(
        n_verts, n_indices, pos_len, idx_len, idx_offset, bin_len, &min, &max,
    );
    let json_bytes = json.as_bytes();
    // Pad JSON to a 4-byte boundary with spaces (0x20 per glTF spec).
    let json_pad = (4 - (json_bytes.len() % 4)) % 4;
    let json_chunk_len = json_bytes.len() + json_pad;

    // --- Total length and bounds check before any write. ---
    let total = 12 + (8 + json_chunk_len) + (8 + bin_len);
    if out.len() < total {
        return Err(CvError::BufferTooSmall);
    }

    let mut o = 0usize;
    let put_u32 = |buf: &mut [u8], off: &mut usize, v: u32| {
        buf[*off..*off + 4].copy_from_slice(&v.to_le_bytes());
        *off += 4;
    };

    // GLB header: magic, version, total length.
    put_u32(out, &mut o, GLB_MAGIC);
    put_u32(out, &mut o, GLB_VERSION);
    put_u32(out, &mut o, total as u32);

    // JSON chunk header + data + space padding.
    put_u32(out, &mut o, json_chunk_len as u32);
    put_u32(out, &mut o, CHUNK_TYPE_JSON);
    out[o..o + json_bytes.len()].copy_from_slice(json_bytes);
    o += json_bytes.len();
    for _ in 0..json_pad {
        out[o] = b' ';
        o += 1;
    }

    // BIN chunk header + data (positions then indices, little-endian).
    put_u32(out, &mut o, bin_len as u32);
    put_u32(out, &mut o, CHUNK_TYPE_BIN);
    for p in &mesh.positions {
        for c in 0..3 {
            out[o..o + 4].copy_from_slice(&p[c].to_le_bytes());
            o += 4;
        }
    }
    for &i in &mesh.indices {
        out[o..o + 4].copy_from_slice(&i.to_le_bytes());
        o += 4;
    }

    debug_assert_eq!(o, total);
    Ok(o)
}

/// Build the glTF JSON object as a compact string.
#[allow(clippy::too_many_arguments)]
fn build_gltf_json(
    n_verts: usize,
    n_indices: usize,
    pos_len: usize,
    idx_len: usize,
    idx_offset: usize,
    bin_len: usize,
    min: &[f32; 3],
    max: &[f32; 3],
) -> String {
    let mut s = String::with_capacity(768);
    s.push_str("{\"asset\":{\"version\":\"2.0\",\"generator\":\"qualia-vision MeshIR GLB\"},");
    s.push_str("\"scene\":0,\"scenes\":[{\"nodes\":[0]}],");
    s.push_str("\"nodes\":[{\"mesh\":0}],");
    s.push_str("\"meshes\":[{\"primitives\":[{\"attributes\":{\"POSITION\":0},\"indices\":1,\"mode\":");
    s.push_str(&PRIMITIVE_MODE_TRIANGLES.to_string());
    s.push_str("}]}],");

    // buffers
    s.push_str("\"buffers\":[{\"byteLength\":");
    s.push_str(&bin_len.to_string());
    s.push_str("}],");

    // bufferViews: 0 = positions, 1 = indices
    s.push_str("\"bufferViews\":[");
    s.push_str("{\"buffer\":0,\"byteOffset\":0,\"byteLength\":");
    s.push_str(&pos_len.to_string());
    s.push_str(",\"target\":");
    s.push_str(&TARGET_ARRAY_BUFFER.to_string());
    s.push_str("},");
    s.push_str("{\"buffer\":0,\"byteOffset\":");
    s.push_str(&idx_offset.to_string());
    s.push_str(",\"byteLength\":");
    s.push_str(&idx_len.to_string());
    s.push_str(",\"target\":");
    s.push_str(&TARGET_ELEMENT_ARRAY_BUFFER.to_string());
    s.push_str("}],");

    // accessors: 0 = POSITION VEC3 f32 (with min/max), 1 = indices SCALAR u32
    s.push_str("\"accessors\":[");
    s.push_str("{\"bufferView\":0,\"byteOffset\":0,\"componentType\":");
    s.push_str(&COMPONENT_TYPE_FLOAT.to_string());
    s.push_str(",\"count\":");
    s.push_str(&n_verts.to_string());
    s.push_str(",\"type\":\"VEC3\",\"min\":");
    push_f32_array(&mut s, min);
    s.push_str(",\"max\":");
    push_f32_array(&mut s, max);
    s.push_str("},");
    s.push_str("{\"bufferView\":1,\"byteOffset\":0,\"componentType\":");
    s.push_str(&COMPONENT_TYPE_UINT.to_string());
    s.push_str(",\"count\":");
    s.push_str(&n_indices.to_string());
    s.push_str(",\"type\":\"SCALAR\"}");
    s.push_str("]}");
    s
}

/// Append a JSON array of three f32 with round-trippable formatting.
fn push_f32_array(s: &mut String, v: &[f32; 3]) {
    s.push('[');
    for (i, c) in v.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        // f64 widening gives a stable, JSON-parseable decimal for the f32 value.
        s.push_str(&format_json_f32(*c));
    }
    s.push(']');
}

/// Format an f32 as a JSON number (finite guaranteed by callers writing bounds).
fn format_json_f32(v: f32) -> String {
    if v.is_finite() {
        // Widen to f64 so the shortest decimal round-trips back to this f32.
        format!("{}", v as f64)
    } else {
        // Should be unreachable for a non-empty finite mesh; keep JSON valid.
        "0".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computer_vision::spatial::geometry_ir::MeshIR;

    fn triangle() -> MeshIR {
        let mut m = MeshIR::empty();
        m.positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 2.0, 0.0]];
        m.indices = vec![0, 1, 2];
        m
    }

    #[test]
    fn glb_magic_version_and_length() {
        let m = triangle();
        let mut buf = vec![0u8; 4096];
        let n = mesh_ir_to_glb(&m, &mut buf).expect("export");
        // (a) magic "glTF"
        assert_eq!(u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]), GLB_MAGIC);
        assert_eq!(&buf[0..4], b"glTF");
        // (b) version == 2
        assert_eq!(u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]), 2);
        // (c) header total-length == bytes written
        let total = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]) as usize;
        assert_eq!(total, n);
    }

    #[test]
    fn glb_json_chunk_parses_expected_tokens() {
        let m = triangle();
        let mut buf = vec![0u8; 4096];
        let n = mesh_ir_to_glb(&m, &mut buf).expect("export");

        // JSON chunk starts at offset 12: [len u32][type u32][json...]
        let json_len = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]) as usize;
        let json_type = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]);
        assert_eq!(json_type, CHUNK_TYPE_JSON);
        assert_eq!(&buf[16..20], b"JSON");
        let json_start = 20;
        let json = std::str::from_utf8(&buf[json_start..json_start + json_len]).expect("utf8");
        // (d) substring checks
        assert!(json.contains("\"POSITION\""));
        assert!(json.contains("\"mode\":4"));
        assert!(json.contains("\"componentType\":5125")); // u32 indices
        assert!(json.contains("\"min\":"));
        assert!(json.contains("\"max\":"));
        // JSON chunk is 4-byte aligned.
        assert_eq!(json_len % 4, 0);
        assert!(n > json_start + json_len);
    }

    #[test]
    fn glb_bin_chunk_holds_exact_vertex_bytes() {
        let m = triangle();
        let mut buf = vec![0u8; 4096];
        let _ = mesh_ir_to_glb(&m, &mut buf).expect("export");

        // Locate the BIN chunk: header(12) + 8 + jsonChunkLen, then 8-byte chunk header.
        let json_len = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]) as usize;
        let bin_hdr = 12 + 8 + json_len;
        let bin_type = u32::from_le_bytes([
            buf[bin_hdr + 4],
            buf[bin_hdr + 5],
            buf[bin_hdr + 6],
            buf[bin_hdr + 7],
        ]);
        assert_eq!(bin_type, CHUNK_TYPE_BIN);
        assert_eq!(&buf[bin_hdr + 4..bin_hdr + 8], b"BIN\0");
        let bin_data = bin_hdr + 8;

        // Third vertex is [0.0, 2.0, 0.0]; its y float sits at vertex 2, component 1.
        let off = bin_data + (2 * 3 + 1) * 4;
        let y = f32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]);
        assert_eq!(y, 2.0);
        // First vertex x == 0.0
        let x0 = f32::from_le_bytes([
            buf[bin_data],
            buf[bin_data + 1],
            buf[bin_data + 2],
            buf[bin_data + 3],
        ]);
        assert_eq!(x0, 0.0);
        // Second vertex x == 1.0
        let off1 = bin_data + 3 * 4;
        let x1 = f32::from_le_bytes([buf[off1], buf[off1 + 1], buf[off1 + 2], buf[off1 + 3]]);
        assert_eq!(x1, 1.0);

        // First index (u32) sits right after the 3 vertices (9 floats = 36 bytes).
        let idx_off = bin_data + 9 * 4;
        let i0 = u32::from_le_bytes([
            buf[idx_off],
            buf[idx_off + 1],
            buf[idx_off + 2],
            buf[idx_off + 3],
        ]);
        assert_eq!(i0, 0);
    }

    #[test]
    fn glb_too_small_buffer_fails_closed() {
        let m = triangle();
        let mut tiny = vec![0u8; 16]; // smaller than any valid GLB for this mesh
        let r = mesh_ir_to_glb(&m, &mut tiny);
        assert_eq!(r, Err(CvError::BufferTooSmall));
    }

    #[test]
    fn glb_empty_mesh_fails_closed() {
        let m = MeshIR::empty();
        let mut buf = vec![0u8; 256];
        let r = mesh_ir_to_glb(&m, &mut buf);
        assert_eq!(r, Err(CvError::EmptyInput));
    }

    #[test]
    fn glb_out_of_range_index_rejected() {
        let mut m = triangle();
        m.indices = vec![0, 1, 99]; // 99 >= 3 vertices
        let mut buf = vec![0u8; 4096];
        let r = mesh_ir_to_glb(&m, &mut buf);
        assert_eq!(r, Err(CvError::InvalidParameter));
    }
}
