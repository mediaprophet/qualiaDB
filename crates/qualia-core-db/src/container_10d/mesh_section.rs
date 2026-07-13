//! `.10d` QuantizedMesh section — the geometry half of a mesh asset in the
//! container (P0.4).
//!
//! A QuantizedMesh section wraps a [`Mesh`](crate::render::assets::Mesh) as a
//! self-describing `.10d` section. Vertex positions are quantized to **u16
//! per axis within the mesh's bounding box** — 6 bytes/vertex vs 12 for raw
//! f32 (2×), and the bbox is exactly what the semantic quins already carry,
//! so it doubles as the dequantization frame (no information is invented).
//! Triangle indices are u16 when the mesh has ≤65 536 vertices (6 bytes/tri
//! vs 12). Quantization error is `bbox_extent / 65535` per axis — sub-micron
//! at organ scale, visually lossless.
//!
//! **Layout:** a 40-byte [`MeshMiniHeader`] (flags + counts + dequantization
//! bbox + reserved) followed by the quantized vertex data (N × 6 bytes:
//! u16×3 per vertex) and then the triangle indices (u16×3 or u32×3 per
//! triangle, selected by the `FLAG_U16_INDICES` flag). The mini-header is
//! `repr(C)`, naturally aligned, no implicit padding.
//!
//! **This replaces the erroneous legacy mesh build artifact** that lived in
//! `render/mesh_asset.rs` — a pre-release format that was never shipped and
//! has been refactored out rather than carried forward. The legacy 48-byte
//! header with its per-format magic is gone; the `.10d` section-type tag
//! (`SectionType::QuantizedMesh = 1`) replaces the magic, and the `.10d`
//! container version replaces the per-format version. No backward-compat is
//! provided — the legacy format was an erroneous build artifact, not a
//! shipped format anyone depends on.
//!
//! **Determinism + CRC:** two encodes of the same mesh are byte-identical
//! (the quantization is deterministic). The per-section CRC-32C (P0.2)
//! catches a flipped bit in the payload. The whole-file CRC-32C (P0.3)
//! catches header corruption.

use bytemuck::{bytes_of, from_bytes, Pod, Zeroable};

use crate::render::assets::Mesh;

/// Section payload mini-header size in bytes.
pub const MESH_MINI_HEADER_SIZE: usize = 40;

/// `flags` bit 0: triangle indices are u16 (else u32).
pub const FLAG_U16_INDICES: u16 = 0x0001;

/// Maximum vertex count the mesh section will accept. Bounds against a
/// hostile/malformed file. u16 indices cap at 65 536; above that the encoder
/// switches to u32. The practical ceiling is the 42MB Sentinel: 40MB of
/// vertex data / 6 bytes per vertex ≈ 6.7M vertices. 4M (2^22) is a
/// comfortable upper bound.
pub const MAX_VERTEX_COUNT: usize = 4_194_304; // 2^22

/// Maximum triangle count. Similarly bounded by the Sentinel: 40MB / 6 bytes
/// per u16-indexed triangle ≈ 6.7M triangles. 4M is the matching ceiling.
pub const MAX_TRIANGLE_COUNT: usize = 4_194_304; // 2^22

/// The 40-byte QuantizedMesh-section mini-header. `repr(C)`, naturally
/// aligned, no implicit padding.
///
/// ```text
/// offset  size  field
/// 0       2     flags:u16         (bit 0 = u16 indices, else u32)
/// 2       2     reserved_u16      (must be zero)
/// 4       4     vertex_count:u32
/// 8       4     triangle_count:u32
/// 12      12    min:[f32;3]       (dequantization frame: position = min + (q/65535)*(max-min))
/// 24      12    max:[f32;3]
/// 36      4     reserved_u32      (must be zero — future: LOD tier, material index)
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct MeshMiniHeader {
    pub flags: u16,
    pub reserved_u16: u16,
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub reserved_u32: u32,
}

impl MeshMiniHeader {
    /// Total payload byte length (mini-header + vertex data + index data).
    #[inline]
    pub fn payload_bytes(vertex_count: usize, triangle_count: usize, u16_idx: bool) -> usize {
        let idx_bytes = if u16_idx { 6 } else { 12 };
        MESH_MINI_HEADER_SIZE + vertex_count * 6 + triangle_count * idx_bytes
    }
}

/// Mesh-section read/write error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshSectionError {
    /// The payload is too short for the mini-header.
    PayloadTooShort { got: usize, need: usize },
    /// A reserved field in the mini-header is non-zero.
    NonZeroReserved { field: &'static str },
    /// `vertex_count` exceeds `MAX_VERTEX_COUNT`.
    VertexCountTooLarge { got: u32, max: usize },
    /// `triangle_count` exceeds `MAX_TRIANGLE_COUNT`.
    TriangleCountTooLarge { got: u32, max: usize },
    /// The payload is too short for the declared counts.
    PayloadTruncated { expected: usize, got: usize },
    /// The output buffer is too small.
    OutputBufferTooSmall { needed: usize, have: usize },
    /// Unknown flags bit set (only bit 0 is defined in v1).
    UnknownFlags { got: u16 },
}

impl std::fmt::Display for MeshSectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PayloadTooShort { got, need } => {
                write!(f, "10d MESH payload too short: got {got}, need {need}")
            }
            Self::NonZeroReserved { field } => {
                write!(f, "10d MESH non-zero reserved field {field:?}")
            }
            Self::VertexCountTooLarge { got, max } => {
                write!(f, "10d MESH vertex_count {got} exceeds max {max}")
            }
            Self::TriangleCountTooLarge { got, max } => {
                write!(f, "10d MESH triangle_count {got} exceeds max {max}")
            }
            Self::PayloadTruncated { expected, got } => write!(
                f,
                "10d MESH payload truncated: expected {expected}, got {got}"
            ),
            Self::OutputBufferTooSmall { needed, have } => write!(
                f,
                "10d MESH output buffer too small: need {needed}, have {have}"
            ),
            Self::UnknownFlags { got } => write!(
                f,
                "10d MESH unknown flags bits {got:#06x} (only bit 0 defined in v1)"
            ),
        }
    }
}

impl std::error::Error for MeshSectionError {}

/// Whether a mesh with `vertex_count` vertices can use u16 indices.
#[inline]
pub fn fits_u16_indices(vertex_count: usize) -> bool {
    vertex_count <= u16::MAX as usize + 1 // indices 0..=65535
}

/// Encoded length in bytes for a mesh of the given size (for size reporting without allocating).
#[inline]
pub fn encoded_len(vertex_count: usize, triangle_count: usize) -> usize {
    let idx_bytes = if fits_u16_indices(vertex_count) {
        6
    } else {
        12
    };
    MESH_MINI_HEADER_SIZE + vertex_count * 6 + triangle_count * idx_bytes
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

/// Parse and validate the mesh-section mini-header. Returns the header and the
/// total payload byte length it claims.
pub fn parse_mesh_header(bytes: &[u8]) -> Result<(MeshMiniHeader, usize), MeshSectionError> {
    if bytes.len() < MESH_MINI_HEADER_SIZE {
        return Err(MeshSectionError::PayloadTooShort {
            got: bytes.len(),
            need: MESH_MINI_HEADER_SIZE,
        });
    }
    let header: MeshMiniHeader = {
        let mut buf = [0u8; MESH_MINI_HEADER_SIZE];
        buf.copy_from_slice(&bytes[..MESH_MINI_HEADER_SIZE]);
        *from_bytes(&buf)
    };
    if header.reserved_u16 != 0 {
        return Err(MeshSectionError::NonZeroReserved {
            field: "reserved_u16",
        });
    }
    if header.reserved_u32 != 0 {
        return Err(MeshSectionError::NonZeroReserved {
            field: "reserved_u32",
        });
    }
    // Only bit 0 (FLAG_U16_INDICES) is defined in v1.
    if header.flags & !FLAG_U16_INDICES != 0 {
        return Err(MeshSectionError::UnknownFlags { got: header.flags });
    }
    let vcount = header.vertex_count as usize;
    let tcount = header.triangle_count as usize;
    if vcount > MAX_VERTEX_COUNT {
        return Err(MeshSectionError::VertexCountTooLarge {
            got: header.vertex_count,
            max: MAX_VERTEX_COUNT,
        });
    }
    if tcount > MAX_TRIANGLE_COUNT {
        return Err(MeshSectionError::TriangleCountTooLarge {
            got: header.triangle_count,
            max: MAX_TRIANGLE_COUNT,
        });
    }
    let u16_idx = header.flags & FLAG_U16_INDICES != 0;
    let total = MeshMiniHeader::payload_bytes(vcount, tcount, u16_idx);
    if bytes.len() < total {
        return Err(MeshSectionError::PayloadTruncated {
            expected: total,
            got: bytes.len(),
        });
    }
    Ok((header, total))
}

/// Encode a [`Mesh`] into a `.10d` QuantizedMesh section payload in a
/// caller-supplied buffer. Returns the bytes written. Zero-heap. The bbox is
/// recomputed from the positions (independent of any stale `mesh.min/max`)
/// so it is a faithful dequantization frame.
pub fn encode_mesh_section(mesh: &Mesh, out: &mut [u8]) -> Result<usize, MeshSectionError> {
    let vcount = mesh.positions.len();
    let tcount = mesh.triangles.len();
    if vcount > MAX_VERTEX_COUNT {
        return Err(MeshSectionError::VertexCountTooLarge {
            got: vcount as u32,
            max: MAX_VERTEX_COUNT,
        });
    }
    if tcount > MAX_TRIANGLE_COUNT {
        return Err(MeshSectionError::TriangleCountTooLarge {
            got: tcount as u32,
            max: MAX_TRIANGLE_COUNT,
        });
    }
    let u16_idx = fits_u16_indices(vcount);
    let (min, max) = bbox(&mesh.positions);
    let extent = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    let need = encoded_len(vcount, tcount);
    if out.len() < need {
        return Err(MeshSectionError::OutputBufferTooSmall {
            needed: need,
            have: out.len(),
        });
    }
    let header = MeshMiniHeader {
        flags: if u16_idx { FLAG_U16_INDICES } else { 0 },
        reserved_u16: 0,
        vertex_count: vcount as u32,
        triangle_count: tcount as u32,
        min,
        max,
        reserved_u32: 0,
    };
    let header_bytes = bytes_of(&header);
    out[..MESH_MINI_HEADER_SIZE].copy_from_slice(header_bytes);
    let mut off = MESH_MINI_HEADER_SIZE;
    for p in &mesh.positions {
        for a in 0..3 {
            out[off..off + 2].copy_from_slice(&quantize(p[a], min[a], extent[a]).to_le_bytes());
            off += 2;
        }
    }
    for t in &mesh.triangles {
        for &idx in t {
            if u16_idx {
                out[off..off + 2].copy_from_slice(&(idx as u16).to_le_bytes());
                off += 2;
            } else {
                out[off..off + 4].copy_from_slice(&idx.to_le_bytes());
                off += 4;
            }
        }
    }
    debug_assert_eq!(off, need, "encode must fill exactly the computed length");
    Ok(off)
}

/// Decode a `.10d` QuantizedMesh section payload back into a [`Mesh`]
/// (dequantized positions, exact indices). This is the ingest path (not a
/// hot path), so `Vec` allocation is fine per AGENTS.md §2-B.
pub fn decode_mesh_section(bytes: &[u8]) -> Result<Mesh, MeshSectionError> {
    let (header, total) = parse_mesh_header(bytes)?;
    let vcount = header.vertex_count as usize;
    let tcount = header.triangle_count as usize;
    let u16_idx = header.flags & FLAG_U16_INDICES != 0;
    let extent = [
        header.max[0] - header.min[0],
        header.max[1] - header.min[1],
        header.max[2] - header.min[2],
    ];
    debug_assert_eq!(
        total,
        MESH_MINI_HEADER_SIZE + vcount * 6 + tcount * if u16_idx { 6 } else { 12 }
    );

    let mut positions = Vec::with_capacity(vcount);
    let mut off = MESH_MINI_HEADER_SIZE;
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
                *corner = u32::from_le_bytes([
                    bytes[off],
                    bytes[off + 1],
                    bytes[off + 2],
                    bytes[off + 3],
                ]);
                off += 4;
            }
        }
        triangles.push(t);
    }

    Ok(Mesh {
        positions,
        triangles,
        min: header.min,
        max: header.max,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_10d::header::Container10dHeader;
    use crate::container_10d::integrity::{seal_whole_file_crc32c, verify_whole_file_crc32c};
    use crate::container_10d::section::{
        encode_container, parse_section_table, AlignmentTier, SectionInput, SectionType,
    };

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
        Mesh {
            positions,
            triangles,
            min: [0.0; 3],
            max: [1.0; 3],
        }
    }

    #[test]
    fn mini_header_is_pod_with_exact_size() {
        assert_eq!(std::mem::size_of::<MeshMiniHeader>(), MESH_MINI_HEADER_SIZE);
        assert_eq!(std::mem::offset_of!(MeshMiniHeader, flags), 0);
        assert_eq!(std::mem::offset_of!(MeshMiniHeader, reserved_u16), 2);
        assert_eq!(std::mem::offset_of!(MeshMiniHeader, vertex_count), 4);
        assert_eq!(std::mem::offset_of!(MeshMiniHeader, triangle_count), 8);
        assert_eq!(std::mem::offset_of!(MeshMiniHeader, min), 12);
        assert_eq!(std::mem::offset_of!(MeshMiniHeader, max), 24);
        assert_eq!(std::mem::offset_of!(MeshMiniHeader, reserved_u32), 36);
    }

    #[test]
    fn round_trips_within_quantization_tolerance_and_indices_exact() {
        let mesh = cube();
        let need = encoded_len(mesh.positions.len(), mesh.triangles.len());
        let mut buf = vec![0u8; need];
        let n = encode_mesh_section(&mesh, &mut buf).expect("encode");
        assert_eq!(n, need);
        let back = decode_mesh_section(&buf).expect("decode");

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
        let vcount = 50_000usize;
        let tcount = 100_000usize;
        let raw = raw_geometry_len(vcount, tcount);
        let enc = encoded_len(vcount, tcount);
        assert!(enc < raw, "encoded {enc} !< raw {raw}");
        let ratio = enc as f64 / raw as f64;
        assert!(ratio < 0.52 && ratio > 0.48, "ratio {ratio} not ~0.5");
    }

    #[test]
    fn u32_indices_when_over_65k_vertices() {
        let positions = vec![[0.0f32, 0.0, 0.0]; 70_000];
        let triangles = vec![[0u32, 1, 69_999]];
        let mesh = Mesh {
            positions,
            triangles: triangles.clone(),
            min: [0.0; 3],
            max: [0.0; 3],
        };
        let need = encoded_len(mesh.positions.len(), mesh.triangles.len());
        let mut buf = vec![0u8; need];
        encode_mesh_section(&mesh, &mut buf).expect("encode");
        let (header, _) = parse_mesh_header(&buf).expect("parse header");
        assert_eq!(header.flags & FLAG_U16_INDICES, 0, "u32 indices selected");
        let back = decode_mesh_section(&buf).expect("decode");
        assert_eq!(back.triangles, triangles, "large indices survive");
    }

    #[test]
    fn rejects_bad_payload_and_truncation() {
        assert!(parse_mesh_header(&[0u8; 8]).is_err());
        let mesh = cube();
        let need = encoded_len(mesh.positions.len(), mesh.triangles.len());
        let mut buf = vec![0u8; need];
        encode_mesh_section(&mesh, &mut buf).expect("encode");
        buf.truncate(MESH_MINI_HEADER_SIZE + 4); // header ok, body truncated
        assert!(parse_mesh_header(&buf).is_err());
    }

    #[test]
    fn rejects_non_zero_reserved() {
        let mesh = cube();
        let need = encoded_len(mesh.positions.len(), mesh.triangles.len());
        let mut buf = vec![0u8; need];
        encode_mesh_section(&mesh, &mut buf).expect("encode");
        buf[2] = 1; // reserved_u16
        let err = parse_mesh_header(&buf).expect_err("non-zero reserved_u16 must reject");
        assert!(
            matches!(
                err,
                MeshSectionError::NonZeroReserved {
                    field: "reserved_u16"
                }
            ),
            "{err}"
        );
        // Restore and corrupt reserved_u32 (offset 36).
        buf[2] = 0;
        buf[36] = 1;
        let err = parse_mesh_header(&buf).expect_err("non-zero reserved_u32 must reject");
        assert!(
            matches!(
                err,
                MeshSectionError::NonZeroReserved {
                    field: "reserved_u32"
                }
            ),
            "{err}"
        );
    }

    #[test]
    fn rejects_unknown_flags() {
        let mesh = cube();
        let need = encoded_len(mesh.positions.len(), mesh.triangles.len());
        let mut buf = vec![0u8; need];
        encode_mesh_section(&mesh, &mut buf).expect("encode");
        buf[0] = 0x02; // bit 1 is undefined in v1
        let err = parse_mesh_header(&buf).expect_err("unknown flags must reject");
        assert!(
            matches!(err, MeshSectionError::UnknownFlags { .. }),
            "{err}"
        );
    }

    #[test]
    fn rejects_vertex_count_too_large() {
        let header = MeshMiniHeader {
            flags: FLAG_U16_INDICES,
            reserved_u16: 0,
            vertex_count: (MAX_VERTEX_COUNT + 1) as u32,
            triangle_count: 0,
            min: [0.0; 3],
            max: [0.0; 3],
            reserved_u32: 0,
        };
        let mut buf = vec![0u8; MESH_MINI_HEADER_SIZE];
        buf[..MESH_MINI_HEADER_SIZE].copy_from_slice(bytemuck::bytes_of(&header));
        let err = parse_mesh_header(&buf).expect_err("too-large vertex count must reject");
        assert!(
            matches!(err, MeshSectionError::VertexCountTooLarge { .. }),
            "{err}"
        );
    }

    #[test]
    fn determinism_two_encodes_byte_identical() {
        let mesh = cube();
        let need = encoded_len(mesh.positions.len(), mesh.triangles.len());
        let mut a = vec![0u8; need];
        let mut b = vec![0u8; need];
        encode_mesh_section(&mesh, &mut a).expect("encode a");
        encode_mesh_section(&mesh, &mut b).expect("encode b");
        assert_eq!(a, b, "two encodes of the same mesh must be byte-identical");
    }

    #[test]
    fn mesh_section_round_trips_through_10d_container_with_crc() {
        let mesh = cube();
        let mesh_need = encoded_len(mesh.positions.len(), mesh.triangles.len());
        let mut mesh_payload = vec![0u8; mesh_need];
        encode_mesh_section(&mesh, &mut mesh_payload).expect("mesh encode");

        let h = Container10dHeader::proposed();
        let inputs = [SectionInput {
            section_type: SectionType::QuantizedMesh,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: &mesh_payload,
        }];
        let mut out = vec![0u8; 512];
        let n = encode_container(&h, &inputs, &mut out).expect("container encode");
        seal_whole_file_crc32c(&mut out[..n]);
        verify_whole_file_crc32c(&mut out[..n]).expect("whole-file CRC");

        let parsed_h = Container10dHeader::parse(&out[..n]).expect("header parse");
        let descs = parse_section_table(&out[..n], &parsed_h).expect("table parse");
        assert_eq!(descs.len(), 1);
        assert_eq!(descs[0].section_type, SectionType::QuantizedMesh as u8);

        let p_off = descs[0].byte_offset as usize;
        let p_len = descs[0].byte_length as usize;
        let mesh_back = decode_mesh_section(&out[p_off..p_off + p_len]).expect("mesh decode");
        assert_eq!(
            mesh_back.triangles, mesh.triangles,
            "indices exact through container"
        );
        // Positions within quantization tolerance.
        let tol = 1.0f32 / 65535.0 * 2.0;
        for (a, b) in mesh.positions.iter().zip(&mesh_back.positions) {
            for k in 0..3 {
                assert!((a[k] - b[k]).abs() <= tol, "axis {k}: {} vs {}", a[k], b[k]);
            }
        }
    }

    #[test]
    fn flipped_payload_bit_in_mesh_section_is_caught_by_per_section_crc() {
        let mesh = cube();
        let mesh_need = encoded_len(mesh.positions.len(), mesh.triangles.len());
        let mut mesh_payload = vec![0u8; mesh_need];
        encode_mesh_section(&mesh, &mut mesh_payload).expect("mesh encode");

        let h = Container10dHeader::proposed();
        let inputs = [SectionInput {
            section_type: SectionType::QuantizedMesh,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: &mesh_payload,
        }];
        let mut out = vec![0u8; 512];
        let n = encode_container(&h, &inputs, &mut out).expect("encode");
        let parsed_h = Container10dHeader::parse(&out[..n]).expect("header parse");
        let descs = parse_section_table(&out[..n], &parsed_h).expect("clean table parses");
        let p_off = descs[0].byte_offset as usize;
        // Flip a bit in the mesh payload (past the mini-header, in vertex data).
        out[p_off + MESH_MINI_HEADER_SIZE + 1] ^= 0x01;
        let err =
            parse_section_table(&out[..n], &parsed_h).expect_err("flipped bit must be caught");
        assert!(
            matches!(
                err,
                crate::container_10d::section::SectionTableError::CrcMismatch { .. }
            ),
            "{err}"
        );
    }

    #[test]
    fn empty_mesh_round_trips() {
        let mesh = Mesh {
            positions: vec![],
            triangles: vec![],
            min: [0.0; 3],
            max: [0.0; 3],
        };
        let need = encoded_len(0, 0);
        assert_eq!(need, MESH_MINI_HEADER_SIZE);
        let mut buf = vec![0u8; need];
        encode_mesh_section(&mesh, &mut buf).expect("empty encode");
        let back = decode_mesh_section(&buf).expect("empty decode");
        assert!(back.positions.is_empty());
        assert!(back.triangles.is_empty());
    }
}
