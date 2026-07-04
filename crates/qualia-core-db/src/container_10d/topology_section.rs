//! `.10d` Topology section — half-edge graph + CSR adjacency + connectivity
//! summary serialized as a self-describing section (P2.8).
//!
//! Layout:
//! ```text
//! [TopologyMiniHeader: 32 bytes]
//! [half_edges:  N * 16 bytes  (HalfEdge = 4 × u32)]
//! [v_offsets:   (V+1) * 4 bytes]
//! [v_neighbours: N * 4 bytes]
//! [f_offsets:   (F+1) * 4 bytes]
//! [f_neighbours: N * 4 bytes]
//! ```
//!
//! All arrays are little-endian, 4-byte aligned. The section is deterministic:
//! identical input yields byte-identical output. Per-section CRC-32C is
//! handled by the container's section-table encoder.

use bytemuck::{bytes_of, cast_slice, from_bytes, Pod, Zeroable};

use crate::specialized_libs::computational_geometry::{
    build_face_adjacency_csr, build_vertex_adjacency_csr, compute_connectivity, HalfEdge,
    INVALID_INDEX,
};

/// Mini-header size in bytes.
pub const TOPOLOGY_MINI_HEADER_SIZE: usize = 32;

/// Maximum half-edge count (bounded by 42MB Sentinel: 42MB / 16B = 2.75M).
pub const MAX_HALF_EDGE_COUNT: usize = 2_097_152; // 2^21

/// The 32-byte Topology-section mini-header.
///
/// ```text
/// offset  size  field
/// 0       4     vertex_count:u32
/// 4       4     face_count:u32
/// 8       4     half_edge_count:u32
/// 12      4     boundary_loop_count:u32
/// 16      4     component_count:u32
/// 20      4     euler_characteristic:i32
/// 24      4     genus:u32           (0xFFFF_FFFF = None / non-orientable)
/// 28      4     reserved_u32        (must be zero)
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct TopologyMiniHeader {
    pub vertex_count: u32,
    pub face_count: u32,
    pub half_edge_count: u32,
    pub boundary_loop_count: u32,
    pub component_count: u32,
    pub euler_characteristic: i32,
    pub genus: u32,
    pub reserved_u32: u32,
}

impl Default for TopologyMiniHeader {
    fn default() -> Self {
        Self {
            vertex_count: 0,
            face_count: 0,
            half_edge_count: 0,
            boundary_loop_count: 0,
            component_count: 0,
            euler_characteristic: 0,
            genus: INVALID_INDEX,
            reserved_u32: 0,
        }
    }
}

/// Topology-section encode/decode error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopologySectionError {
    /// Payload too short for the mini-header.
    PayloadTooShort { got: usize, need: usize },
    /// Non-zero reserved field.
    NonZeroReserved,
    /// Half-edge count exceeds the max.
    HalfEdgeCountTooLarge { got: u32, max: usize },
    /// Payload truncated relative to declared counts.
    PayloadTruncated { expected: usize, got: usize },
    /// Output buffer too small.
    OutputBufferTooSmall { needed: usize, have: usize },
    /// Connectivity computation failed.
    ConnectivityError,
}

impl std::fmt::Display for TopologySectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PayloadTooShort { got, need } => write!(f, "10d TOPOLOGY payload too short: got {got}, need {need}"),
            Self::NonZeroReserved => write!(f, "10d TOPOLOGY non-zero reserved field"),
            Self::HalfEdgeCountTooLarge { got, max } => write!(f, "10d TOPOLOGY half_edge_count {got} exceeds max {max}"),
            Self::PayloadTruncated { expected, got } => write!(f, "10d TOPOLOGY payload truncated: expected {expected}, got {got}"),
            Self::OutputBufferTooSmall { needed, have } => write!(f, "10d TOPOLOGY output buffer too small: need {needed}, have {have}"),
            Self::ConnectivityError => write!(f, "10d TOPOLOGY connectivity computation failed"),
        }
    }
}

impl std::error::Error for TopologySectionError {}

/// Compute the encoded byte length for a topology section.
#[inline]
pub fn encoded_len(vertex_count: u32, face_count: u32, half_edge_count: u32) -> usize {
    let vc = vertex_count as usize;
    let fc = face_count as usize;
    let ec = half_edge_count as usize;
    TOPOLOGY_MINI_HEADER_SIZE
        + ec * 16          // half_edges
        + (vc + 1) * 4     // v_offsets
        + ec * 4           // v_neighbours
        + (fc + 1) * 4     // f_offsets
        + ec * 4           // f_neighbours
}

/// Encode a topology section from a half-edge array.
///
/// Builds the CSR adjacency views and connectivity summary, then serializes
/// everything into `out`. The caller supplies the half-edge array and
/// vertex/face counts. Workspace buffers (`labels`, `queue`, `visited`) are
/// allocated here since this is the ingest path, not a hot path.
///
/// Returns the number of bytes written.
pub fn encode_topology_section(
    vertex_count: u32,
    face_count: u32,
    half_edges: &[HalfEdge],
    out: &mut [u8],
) -> Result<usize, TopologySectionError> {
    let ec = half_edges.len();
    if ec > MAX_HALF_EDGE_COUNT {
        return Err(TopologySectionError::HalfEdgeCountTooLarge {
            got: ec as u32,
            max: MAX_HALF_EDGE_COUNT,
        });
    }
    if ec > (u32::MAX as usize) / 4 {
        return Err(TopologySectionError::HalfEdgeCountTooLarge {
            got: ec as u32,
            max: MAX_HALF_EDGE_COUNT,
        });
    }

    let vc = vertex_count as usize;
    let fc = face_count as usize;

    // Build CSR adjacency views.
    let mut v_offsets = vec![0u32; vc + 1];
    let mut v_neighbours = vec![0u32; ec];
    build_vertex_adjacency_csr(vertex_count, half_edges, &mut v_offsets, &mut v_neighbours)
        .map_err(|_| TopologySectionError::ConnectivityError)?;

    let mut f_offsets = vec![0u32; fc + 1];
    let mut f_neighbours = vec![0u32; ec];
    build_face_adjacency_csr(face_count, half_edges, &mut f_offsets, &mut f_neighbours)
        .map_err(|_| TopologySectionError::ConnectivityError)?;

    // Compute connectivity invariants.
    let mut labels = vec![0u32; fc];
    let mut queue = vec![0u32; fc];
    let mut visited = vec![false; ec];
    let summary = compute_connectivity(
        vertex_count,
        face_count,
        half_edges,
        &mut labels,
        &mut queue,
        &mut visited,
    )
    .map_err(|_| TopologySectionError::ConnectivityError)?;

    let need = encoded_len(vertex_count, face_count, ec as u32);
    if out.len() < need {
        return Err(TopologySectionError::OutputBufferTooSmall {
            needed: need,
            have: out.len(),
        });
    }

    let header = TopologyMiniHeader {
        vertex_count,
        face_count,
        half_edge_count: ec as u32,
        boundary_loop_count: summary.boundary_loop_count,
        component_count: summary.component_count,
        euler_characteristic: summary.euler_characteristic,
        genus: summary.genus.unwrap_or(INVALID_INDEX),
        reserved_u32: 0,
    };

    let mut off = 0usize;

    // Mini-header.
    out[off..off + TOPOLOGY_MINI_HEADER_SIZE]
        .copy_from_slice(bytes_of(&header));
    off += TOPOLOGY_MINI_HEADER_SIZE;

    // Half-edges.
    let he_bytes: &[u8] = cast_slice(half_edges);
    out[off..off + he_bytes.len()].copy_from_slice(he_bytes);
    off += he_bytes.len();

    // Vertex-adjacency CSR: offsets then neighbours.
    let vo_bytes: &[u8] = cast_slice(&v_offsets);
    out[off..off + vo_bytes.len()].copy_from_slice(vo_bytes);
    off += vo_bytes.len();
    let vn_bytes: &[u8] = cast_slice(&v_neighbours);
    out[off..off + vn_bytes.len()].copy_from_slice(vn_bytes);
    off += vn_bytes.len();

    // Face-adjacency CSR: offsets then neighbours.
    let fo_bytes: &[u8] = cast_slice(&f_offsets);
    out[off..off + fo_bytes.len()].copy_from_slice(fo_bytes);
    off += fo_bytes.len();
    let fn_bytes: &[u8] = cast_slice(&f_neighbours);
    out[off..off + fn_bytes.len()].copy_from_slice(fn_bytes);
    off += fn_bytes.len();

    debug_assert_eq!(off, need);
    Ok(off)
}

/// Decoded topology section data.
#[derive(Debug, Clone, PartialEq)]
pub struct TopologySectionData {
    pub header: TopologyMiniHeader,
    pub half_edges: Vec<HalfEdge>,
    pub v_offsets: Vec<u32>,
    pub v_neighbours: Vec<u32>,
    pub f_offsets: Vec<u32>,
    pub f_neighbours: Vec<u32>,
}

/// Decode a `.10d` Topology section payload.
pub fn decode_topology_section(bytes: &[u8]) -> Result<TopologySectionData, TopologySectionError> {
    if bytes.len() < TOPOLOGY_MINI_HEADER_SIZE {
        return Err(TopologySectionError::PayloadTooShort {
            got: bytes.len(),
            need: TOPOLOGY_MINI_HEADER_SIZE,
        });
    }

    let header: TopologyMiniHeader =
        *from_bytes(&bytes[..TOPOLOGY_MINI_HEADER_SIZE]);
    if header.reserved_u32 != 0 {
        return Err(TopologySectionError::NonZeroReserved);
    }

    let vc = header.vertex_count as usize;
    let fc = header.face_count as usize;
    let ec = header.half_edge_count as usize;

    if ec > MAX_HALF_EDGE_COUNT {
        return Err(TopologySectionError::HalfEdgeCountTooLarge {
            got: header.half_edge_count,
            max: MAX_HALF_EDGE_COUNT,
        });
    }

    let need = encoded_len(header.vertex_count, header.face_count, header.half_edge_count);
    if bytes.len() < need {
        return Err(TopologySectionError::PayloadTruncated {
            expected: need,
            got: bytes.len(),
        });
    }

    let mut off = TOPOLOGY_MINI_HEADER_SIZE;

    // Half-edges.
    let he_bytes = &bytes[off..off + ec * 16];
    let half_edges: Vec<HalfEdge> = cast_slice(he_bytes).to_vec();
    off += ec * 16;

    // Vertex-adjacency CSR.
    let vo_bytes = &bytes[off..off + (vc + 1) * 4];
    let v_offsets: Vec<u32> = cast_slice(vo_bytes).to_vec();
    off += (vc + 1) * 4;
    let vn_bytes = &bytes[off..off + ec * 4];
    let v_neighbours: Vec<u32> = cast_slice(vn_bytes).to_vec();
    off += ec * 4;

    // Face-adjacency CSR.
    let fo_bytes = &bytes[off..off + (fc + 1) * 4];
    let f_offsets: Vec<u32> = cast_slice(fo_bytes).to_vec();
    off += (fc + 1) * 4;
    let fn_bytes = &bytes[off..off + ec * 4];
    let f_neighbours: Vec<u32> = cast_slice(fn_bytes).to_vec();
    off += ec * 4;

    debug_assert_eq!(off, need);

    Ok(TopologySectionData {
        header,
        half_edges,
        v_offsets,
        v_neighbours,
        f_offsets,
        f_neighbours,
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
    use crate::specialized_libs::computational_geometry::{
        build_triangle_half_edges, required_edge_slots, EdgeSlot,
    };

    fn build_he(vertex_count: u32, triangles: &[[u32; 3]]) -> (Vec<HalfEdge>, u32, u32) {
        let n = triangles.len() * 3;
        let mut edges = vec![HalfEdge::default(); n];
        let mut slots = vec![EdgeSlot::default(); required_edge_slots(triangles.len())];
        let summary = build_triangle_half_edges(vertex_count, triangles, &mut edges, &mut slots)
            .unwrap();
        (edges, vertex_count, triangles.len() as u32)
    }

    #[test]
    fn mini_header_is_pod_with_exact_size() {
        assert_eq!(std::mem::size_of::<TopologyMiniHeader>(), TOPOLOGY_MINI_HEADER_SIZE);
        assert_eq!(std::mem::offset_of!(TopologyMiniHeader, vertex_count), 0);
        assert_eq!(std::mem::offset_of!(TopologyMiniHeader, face_count), 4);
        assert_eq!(std::mem::offset_of!(TopologyMiniHeader, half_edge_count), 8);
        assert_eq!(std::mem::offset_of!(TopologyMiniHeader, boundary_loop_count), 12);
        assert_eq!(std::mem::offset_of!(TopologyMiniHeader, component_count), 16);
        assert_eq!(std::mem::offset_of!(TopologyMiniHeader, euler_characteristic), 20);
        assert_eq!(std::mem::offset_of!(TopologyMiniHeader, genus), 24);
        assert_eq!(std::mem::offset_of!(TopologyMiniHeader, reserved_u32), 28);
    }

    #[test]
    fn round_trip_tetrahedron() {
        let (edges, vc, fc) = build_he(4, &[[0, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]]);
        let need = encoded_len(vc, fc, edges.len() as u32);
        let mut buf = vec![0u8; need];
        let n = encode_topology_section(vc, fc, &edges, &mut buf).unwrap();
        assert_eq!(n, need);

        let back = decode_topology_section(&buf).unwrap();
        assert_eq!(back.header.vertex_count, 4);
        assert_eq!(back.header.face_count, 4);
        assert_eq!(back.header.half_edge_count, 12);
        assert_eq!(back.header.boundary_loop_count, 0);
        assert_eq!(back.header.component_count, 1);
        assert_eq!(back.header.euler_characteristic, 2);
        assert_eq!(back.header.genus, 0);
        assert_eq!(back.half_edges, edges);
        assert_eq!(back.v_offsets.len(), 5);
        assert_eq!(back.v_neighbours.len(), 12);
        assert_eq!(back.f_offsets.len(), 5);
        assert_eq!(back.f_neighbours.len(), 12);
    }

    #[test]
    fn round_trip_single_triangle() {
        let (edges, vc, fc) = build_he(3, &[[0, 1, 2]]);
        let need = encoded_len(vc, fc, edges.len() as u32);
        let mut buf = vec![0u8; need];
        encode_topology_section(vc, fc, &edges, &mut buf).unwrap();
        let back = decode_topology_section(&buf).unwrap();
        assert_eq!(back.header.boundary_loop_count, 1);
        assert_eq!(back.header.euler_characteristic, 1);
        assert_eq!(back.header.genus, 0);
        assert_eq!(back.half_edges, edges);
    }

    #[test]
    fn determinism_two_encodes_byte_identical() {
        let (edges, vc, fc) = build_he(4, &[[0, 1, 2], [2, 1, 3]]);
        let need = encoded_len(vc, fc, edges.len() as u32);
        let mut a = vec![0u8; need];
        let mut b = vec![0u8; need];
        encode_topology_section(vc, fc, &edges, &mut a).unwrap();
        encode_topology_section(vc, fc, &edges, &mut b).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn rejects_non_zero_reserved() {
        let (edges, vc, fc) = build_he(3, &[[0, 1, 2]]);
        let need = encoded_len(vc, fc, edges.len() as u32);
        let mut buf = vec![0u8; need];
        encode_topology_section(vc, fc, &edges, &mut buf).unwrap();
        buf[28] = 1; // reserved_u32
        assert!(decode_topology_section(&buf).is_err());
    }

    #[test]
    fn rejects_truncated_payload() {
        let (edges, vc, fc) = build_he(3, &[[0, 1, 2]]);
        let need = encoded_len(vc, fc, edges.len() as u32);
        let mut buf = vec![0u8; need];
        encode_topology_section(vc, fc, &edges, &mut buf).unwrap();
        buf.truncate(TOPOLOGY_MINI_HEADER_SIZE + 4);
        assert!(decode_topology_section(&buf).is_err());
    }

    #[test]
    fn topology_section_round_trips_through_10d_container_with_crc() {
        let (edges, vc, fc) = build_he(4, &[[0, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]]);
        let need = encoded_len(vc, fc, edges.len() as u32);
        let mut topo_payload = vec![0u8; need];
        encode_topology_section(vc, fc, &edges, &mut topo_payload).unwrap();

        let h = Container10dHeader::proposed();
        let inputs = [SectionInput {
            section_type: SectionType::Topology,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: &topo_payload,
        }];
        let mut out = vec![0u8; 1024];
        let n = encode_container(&h, &inputs, &mut out).unwrap();
        seal_whole_file_crc32c(&mut out[..n]);
        verify_whole_file_crc32c(&mut out[..n]).unwrap();

        let parsed_h = Container10dHeader::parse(&out[..n]).unwrap();
        let descs = parse_section_table(&out[..n], &parsed_h).unwrap();
        assert_eq!(descs.len(), 1);
        assert_eq!(descs[0].section_type, SectionType::Topology as u8);

        let p_off = descs[0].byte_offset as usize;
        let p_len = descs[0].byte_length as usize;
        let back = decode_topology_section(&out[p_off..p_off + p_len]).unwrap();
        assert_eq!(back.half_edges, edges);
        assert_eq!(back.header.vertex_count, 4);
        assert_eq!(back.header.face_count, 4);
    }

    #[test]
    fn flipped_payload_bit_caught_by_per_section_crc() {
        let (edges, vc, fc) = build_he(4, &[[0, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]]);
        let need = encoded_len(vc, fc, edges.len() as u32);
        let mut topo_payload = vec![0u8; need];
        encode_topology_section(vc, fc, &edges, &mut topo_payload).unwrap();

        let h = Container10dHeader::proposed();
        let inputs = [SectionInput {
            section_type: SectionType::Topology,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: &topo_payload,
        }];
        let mut out = vec![0u8; 1024];
        let n = encode_container(&h, &inputs, &mut out).unwrap();
        let parsed_h = Container10dHeader::parse(&out[..n]).unwrap();
        let descs = parse_section_table(&out[..n], &parsed_h).unwrap();
        let p_off = descs[0].byte_offset as usize;
        out[p_off + TOPOLOGY_MINI_HEADER_SIZE + 1] ^= 0x01;
        let err = parse_section_table(&out[..n], &parsed_h).unwrap_err();
        assert!(matches!(
            err,
            crate::container_10d::section::SectionTableError::CrcMismatch { .. }
        ));
    }
}
