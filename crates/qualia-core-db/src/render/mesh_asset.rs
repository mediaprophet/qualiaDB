//! Native **quantized mesh geometry** buffer — the geometry half of a Q42 mesh asset.
//!
//! Context (verified via git history 2026-07-03): [`assets::mesh_to_nquins`](super::assets) was written
//! in Phase 1.3 (`65a14dd7`), *before* the Q42 container / manifold format existed (Phase 6,
//! `903e8000`). So the GLB→native path only ever emitted the **semantic** layer (13 NQuins: type,
//! counts, bbox, centroid) and threw the geometry away — the mesh was re-imported from the source GLB
//! on every use. This module fills that gap: it encodes the actual vertices + triangles into a compact,
//! zero-copy native buffer, in the same house style as [`tensor::buffer_export`](crate::tensor) (magic
//! `"Q42M"`, `#[repr(C, align(4))]` bytemuck header, little-endian).
//!
//! **Why it's smaller.** Vertex positions are quantized to **u16 per axis within the mesh's bounding
//! box** — 6 bytes/vertex vs 12 for raw f32 (2×), and the bbox is exactly what the semantic quins
//! already carry, so it doubles as the dequantization frame (no information is invented). Triangle
//! indices are u16 when the mesh has ≤65 536 vertices (6 bytes/tri vs 12). Versus the *source* GLB the
//! win is larger and variable: we also drop normals, UVs, tangents, materials and textures.
//! Quantization error is `bbox_extent / 65535` per axis — sub-micron at organ scale, visually lossless.
//!
//! Bigger reductions (a low-poly LOD via decimation) are a lossy follow-up tier that slots into the
//! [`authoring`](super::authoring) budget planner's Scene3D→2D degradation — the same "works on hardware
//! people own" rail. This module is the lossless-quantization foundation.

use bytemuck::{bytes_of, Pod, Zeroable};

use super::assets::Mesh;

/// Magic for a native quantized mesh buffer ("Q42M", little-endian).
pub const MESH_BUFFER_MAGIC: u32 = 0x4D32_3451; // b"Q42M" LE
pub const MESH_BUFFER_VERSION: u16 = 1;

/// `flags` bit 0: triangle indices are u16 (else u32).
pub const FLAG_U16_INDICES: u16 = 0x0001;

/// Header for a quantized mesh buffer: 48 bytes, `align(4)`, no padding (bytemuck-`Pod`).
#[repr(C, align(4))]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct MeshBufferHeader {
    pub magic: u32,
    pub version: u16,
    pub flags: u16,
    pub vertex_count: u32,
    pub triangle_count: u32,
    /// Dequantization frame (per-axis min): position = min + (q/65535) * (max - min).
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub _reserved: [u32; 2],
}

pub const MESH_HEADER_BYTES: usize = std::mem::size_of::<MeshBufferHeader>();

/// Whether a mesh with `vertex_count` vertices can use u16 indices.
#[inline]
pub fn fits_u16_indices(vertex_count: usize) -> bool {
    vertex_count <= u16::MAX as usize + 1 // indices 0..=65535
}

/// Encoded length in bytes for a mesh of the given size (for size reporting without allocating).
#[inline]
pub fn encoded_len(vertex_count: usize, triangle_count: usize) -> usize {
    let idx_bytes = if fits_u16_indices(vertex_count) { 6 } else { 12 };
    MESH_HEADER_BYTES + vertex_count * 6 + triangle_count * idx_bytes
}

/// Raw in-memory geometry size (f32 positions + u32 triangle indices) — the baseline we shrink from.
#[inline]
pub fn raw_geometry_len(vertex_count: usize, triangle_count: usize) -> usize {
    vertex_count * 12 + triangle_count * 12
}

#[inline]
fn quantize(v: f32, min: f32, extent: f32) -> u16 {
    if extent <= 0.0 {
        return 0;
    }
    let n = ((v - min) / extent).clamp(0.0, 1.0);
    (n * 65535.0 + 0.5) as u16
}

#[inline]
fn dequantize(q: u16, min: f32, extent: f32) -> f32 {
    min + (q as f32 / 65535.0) * extent
}

fn bbox(positions: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in positions {
        for a in 0..3 {
            if p[a] < min[a] {
                min[a] = p[a];
            }
            if p[a] > max[a] {
                max[a] = p[a];
            }
        }
    }
    if positions.is_empty() {
        min = [0.0; 3];
        max = [0.0; 3];
    }
    (min, max)
}

/// Encode a [`Mesh`] into a native quantized mesh buffer. The bbox is recomputed from the positions
/// (independent of any stale `mesh.min/max`) so it is a faithful dequantization frame.
pub fn encode_mesh_q42(mesh: &Mesh) -> Vec<u8> {
    let vcount = mesh.positions.len();
    let tcount = mesh.triangles.len();
    let u16_idx = fits_u16_indices(vcount);
    let (min, max) = bbox(&mesh.positions);
    let extent = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];

    let header = MeshBufferHeader {
        magic: MESH_BUFFER_MAGIC,
        version: MESH_BUFFER_VERSION,
        flags: if u16_idx { FLAG_U16_INDICES } else { 0 },
        vertex_count: vcount as u32,
        triangle_count: tcount as u32,
        min,
        max,
        _reserved: [0; 2],
    };

    let mut out = Vec::with_capacity(encoded_len(vcount, tcount));
    out.extend_from_slice(bytes_of(&header));
    for p in &mesh.positions {
        for a in 0..3 {
            out.extend_from_slice(&quantize(p[a], min[a], extent[a]).to_le_bytes());
        }
    }
    for t in &mesh.triangles {
        for &idx in t {
            if u16_idx {
                out.extend_from_slice(&(idx as u16).to_le_bytes());
            } else {
                out.extend_from_slice(&idx.to_le_bytes());
            }
        }
    }
    out
}

/// Parse a mesh buffer header (zero-copy, validated).
pub fn parse_header(bytes: &[u8]) -> Result<MeshBufferHeader, String> {
    if bytes.len() < MESH_HEADER_BYTES {
        return Err("mesh buffer: too small for header".to_string());
    }
    let header: MeshBufferHeader = bytemuck::pod_read_unaligned(&bytes[..MESH_HEADER_BYTES]);
    if header.magic != MESH_BUFFER_MAGIC {
        return Err("mesh buffer: bad magic".to_string());
    }
    if header.version != MESH_BUFFER_VERSION {
        return Err(format!("mesh buffer: unsupported version {}", header.version));
    }
    Ok(header)
}

/// Decode a native quantized mesh buffer back into a [`Mesh`] (dequantized positions, exact indices).
pub fn decode_mesh_q42(bytes: &[u8]) -> Result<Mesh, String> {
    let header = parse_header(bytes)?;
    let vcount = header.vertex_count as usize;
    let tcount = header.triangle_count as usize;
    let u16_idx = header.flags & FLAG_U16_INDICES != 0;
    let idx_bytes = if u16_idx { 6 } else { 12 };

    let need = MESH_HEADER_BYTES + vcount * 6 + tcount * idx_bytes;
    if bytes.len() < need {
        return Err(format!("mesh buffer: truncated (need {need}, have {})", bytes.len()));
    }
    let extent = [
        header.max[0] - header.min[0],
        header.max[1] - header.min[1],
        header.max[2] - header.min[2],
    ];

    let mut positions = Vec::with_capacity(vcount);
    let mut off = MESH_HEADER_BYTES;
    for _ in 0..vcount {
        let mut p = [0.0f32; 3];
        for a in 0..3 {
            let q = u16::from_le_bytes([bytes[off], bytes[off + 1]]);
            p[a] = dequantize(q, header.min[a], extent[a]);
            off += 2;
        }
        positions.push(p);
    }

    let mut triangles = Vec::with_capacity(tcount);
    for _ in 0..tcount {
        let mut t = [0u32; 3];
        for corner in t.iter_mut() {
            if u16_idx {
                *corner = u16::from_le_bytes([bytes[off], bytes[off + 1]]) as u32;
                off += 2;
            } else {
                *corner =
                    u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]]);
                off += 4;
            }
        }
        triangles.push(t);
    }

    Ok(Mesh { positions, triangles, min: header.min, max: header.max })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unit cube: 8 vertices, 12 triangles.
    fn cube() -> Mesh {
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ];
        let triangles = vec![
            [0, 1, 2],
            [0, 2, 3],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [2, 3, 7],
            [2, 7, 6],
            [1, 2, 6],
            [1, 6, 5],
            [0, 3, 7],
            [0, 7, 4],
        ];
        Mesh { positions, triangles, min: [0.0; 3], max: [1.0; 3] }
    }

    #[test]
    fn round_trips_within_quantization_tolerance_and_indices_exact() {
        let mesh = cube();
        let encoded = encode_mesh_q42(&mesh);
        let back = decode_mesh_q42(&encoded).unwrap();

        assert_eq!(back.positions.len(), mesh.positions.len());
        assert_eq!(back.triangles, mesh.triangles, "indices are exact");

        // Positions are within one quantization step of the original (extent/65535 per axis).
        let extent = 1.0f32; // unit cube
        let tol = extent / 65535.0 * 2.0;
        for (a, b) in mesh.positions.iter().zip(&back.positions) {
            for k in 0..3 {
                assert!((a[k] - b[k]).abs() <= tol, "axis {k}: {} vs {}", a[k], b[k]);
            }
        }
    }

    #[test]
    fn encoded_is_smaller_than_raw_f32_geometry() {
        // A realistically-sized organ-ish mesh (50k verts, 100k tris) — the ratio is deterministic.
        let vcount = 50_000usize;
        let tcount = 100_000usize;
        let raw = raw_geometry_len(vcount, tcount);
        let enc = encoded_len(vcount, tcount);
        assert!(enc < raw, "encoded {enc} !< raw {enc}");
        // u16 indices apply (>65536? no, 50k ≤ 65536) → 6B/vert + 6B/tri vs 12+12 → ~2× smaller.
        let ratio = enc as f64 / raw as f64;
        assert!(ratio < 0.52 && ratio > 0.48, "ratio {ratio} not ~0.5");
    }

    #[test]
    fn u32_indices_when_over_65k_vertices() {
        // Just past the u16 index ceiling → header must select u32 indices and still round-trip counts.
        let positions = vec![[0.0f32, 0.0, 0.0]; 70_000];
        let triangles = vec![[0u32, 1, 69_999]];
        let mesh = Mesh { positions, triangles: triangles.clone(), min: [0.0; 3], max: [0.0; 3] };
        let encoded = encode_mesh_q42(&mesh);
        let header = parse_header(&encoded).unwrap();
        assert_eq!(header.flags & FLAG_U16_INDICES, 0, "u32 indices selected");
        let back = decode_mesh_q42(&encoded).unwrap();
        assert_eq!(back.triangles, triangles, "large indices survive");
    }

    #[test]
    fn rejects_bad_magic_and_truncation() {
        assert!(decode_mesh_q42(&[0u8; 8]).is_err());
        let mut good = encode_mesh_q42(&cube());
        good.truncate(MESH_HEADER_BYTES + 4); // header ok, body truncated
        assert!(decode_mesh_q42(&good).is_err());
    }

    #[test]
    fn header_is_exactly_48_bytes() {
        assert_eq!(MESH_HEADER_BYTES, 48);
    }
}
