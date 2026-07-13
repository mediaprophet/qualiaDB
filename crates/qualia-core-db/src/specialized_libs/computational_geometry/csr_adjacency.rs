//! CSR (Compressed Sparse Row) adjacency views for half-edge meshes.
//!
//! The half-edge graph is the canonical topology representation, but many
//! algorithms (component labelling, BFS, graph queries, GPU staging) prefer a
//! flat CSR adjacency: a `offsets` array of length `N+1` and a packed `neighbours`
//! array of length `E`, where the neighbours of entity `i` live in
//! `neighbours[offsets[i]..offsets[i+1]]`.
//!
//! Two views are provided:
//!
//! - **Vertex adjacency** — each vertex's neighbour vertices (via half-edge
//!   traversal). `offsets` has `vertex_count + 1` entries; `neighbours` has
//!   one entry per *directed* half-edge (so degree == outgoing half-edge count).
//! - **Face adjacency** — each face's neighbour faces (via twin links).
//!   `offsets` has `face_count + 1` entries; `neighbours` has one entry per
//!   half-edge, holding the neighbour face index or `INVALID_INDEX` for
//!   boundary edges.
//!
//! Both views are deterministic: identical input yields bit-identical output
//! (neighbours are emitted in half-edge-index order, which is face-order then
//! local-edge-order). All functions are caller-buffered and zero-heap.

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use super::topology::{HalfEdge, INVALID_INDEX};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors raised by the CSR adjacency builders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsrError {
    /// `offsets` buffer too small (needs `entity_count + 1`).
    OffsetBufferTooSmall { required: usize },
    /// `neighbours` buffer too small (needs `edge_count` entries).
    NeighbourBufferTooSmall { required: usize },
    /// A half-edge link points outside the half-edge array.
    HalfEdgeOutOfRange { index: u32 },
    /// A vertex/face index in the half-edge data exceeds `entity_count`.
    EntityOutOfRange { index: u32 },
}

// ---------------------------------------------------------------------------
// CSR POD header (for .10d serialization / GPU staging)
// ---------------------------------------------------------------------------

/// CSR adjacency header: describes the layout of a CSR adjacency stream.
///
/// 16 bytes, `#[repr(C)]`, `Pod`/`Zeroable` so it can be `bytemuck::cast_slice`d
/// alongside the offsets and neighbours arrays for GPU staging or `.10d`
/// section encoding.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable, Serialize, Deserialize)]
pub struct CsrHeader {
    /// Number of entities (vertices or faces).
    pub entity_count: u32,
    /// Number of directed edges in the adjacency (length of `neighbours`).
    pub edge_count: u32,
    /// Byte offset of the `offsets` array from the start of the CSR payload.
    pub offsets_byte_offset: u32,
    /// Byte offset of the `neighbours` array from the start of the CSR payload.
    pub neighbours_byte_offset: u32,
}

impl Default for CsrHeader {
    fn default() -> Self {
        Self {
            entity_count: 0,
            edge_count: 0,
            offsets_byte_offset: 0,
            neighbours_byte_offset: 0,
        }
    }
}

/// Summary of a built CSR adjacency view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CsrSummary {
    pub entity_count: u32,
    pub edge_count: u32,
    pub max_degree: u32,
}

// ---------------------------------------------------------------------------
// Vertex adjacency: vertex → neighbour vertices via half-edge traversal
// ---------------------------------------------------------------------------

/// Build a CSR vertex-adjacency view from a half-edge array.
///
/// For each vertex `v`, the neighbours are the `origin` fields of the half-edges
/// whose `next` originates at `v` — i.e. the *destination* vertices of the
/// outgoing directed edges. The neighbours are emitted in half-edge-index order
/// (deterministic).
///
/// - `offsets`: `vertex_count + 1` entries. `offsets[0] = 0`,
///   `offsets[v+1] = offsets[v] + degree(v)`.
/// - `neighbours`: `half_edges.len()` entries. The neighbours of vertex `v`
///   are `neighbours[offsets[v]..offsets[v+1]]`.
///
/// Zero-heap. Deterministic. Caller-buffered.
pub fn build_vertex_adjacency_csr(
    vertex_count: u32,
    half_edges: &[HalfEdge],
    offsets: &mut [u32],
    neighbours: &mut [u32],
) -> Result<CsrSummary, CsrError> {
    let vc = vertex_count as usize;
    let ec = half_edges.len();

    if offsets.len() < vc + 1 {
        return Err(CsrError::OffsetBufferTooSmall { required: vc + 1 });
    }
    if neighbours.len() < ec {
        return Err(CsrError::NeighbourBufferTooSmall { required: ec });
    }

    // Pass 1: count degrees.
    for slot in &mut offsets[..vc + 1] {
        *slot = 0;
    }
    for he in half_edges.iter() {
        let origin = he.origin as usize;
        if origin >= vc {
            return Err(CsrError::EntityOutOfRange { index: he.origin });
        }
        offsets[origin + 1] += 1;
    }

    // Prefix sum → offsets.
    let mut max_degree = 0u32;
    let mut acc = 0u32;
    for v in 0..vc {
        acc += offsets[v + 1];
        offsets[v + 1] = acc;
        let degree = offsets[v + 1] - offsets[v];
        if degree > max_degree {
            max_degree = degree;
        }
    }
    offsets[0] = 0;

    // Scatter pass: walk half-edges in order, placing each neighbour at
    // offsets[origin] and advancing. After scatter, offsets[v] = original
    // offsets[v+1] for v < vc, and offsets[vc] is unchanged (= ec). We then
    // shift offsets right by one and restore offsets[0] = 0.
    for he in half_edges.iter() {
        let origin = he.origin as usize;
        let next_he = &half_edges.get(he.next as usize);
        let dest = match next_he {
            Some(n) => n.origin,
            None => return Err(CsrError::HalfEdgeOutOfRange { index: he.next }),
        };
        let pos = offsets[origin] as usize;
        neighbours[pos] = dest;
        offsets[origin] += 1;
    }

    // Fix up offsets: shift right by one, restore offsets[0] = 0.
    for v in (1..=vc).rev() {
        offsets[v] = offsets[v - 1];
    }
    offsets[0] = 0;

    Ok(CsrSummary {
        entity_count: vertex_count,
        edge_count: ec as u32,
        max_degree,
    })
}

// ---------------------------------------------------------------------------
// Face adjacency: face → neighbour faces via twin links
// ---------------------------------------------------------------------------

/// Build a CSR face-adjacency view from a half-edge array.
///
/// For each face `f`, the neighbours are the face indices of the twin half-edges'
/// faces. Boundary edges (twin == `INVALID_INDEX`) produce `INVALID_INDEX` in
/// the neighbour list, so the caller can detect boundary connectivity.
///
/// - `offsets`: `face_count + 1` entries.
/// - `neighbours`: `half_edges.len()` entries (3 per triangle face).
///
/// Zero-heap. Deterministic. Caller-buffered.
pub fn build_face_adjacency_csr(
    face_count: u32,
    half_edges: &[HalfEdge],
    offsets: &mut [u32],
    neighbours: &mut [u32],
) -> Result<CsrSummary, CsrError> {
    let fc = face_count as usize;
    let ec = half_edges.len();

    if offsets.len() < fc + 1 {
        return Err(CsrError::OffsetBufferTooSmall { required: fc + 1 });
    }
    if neighbours.len() < ec {
        return Err(CsrError::NeighbourBufferTooSmall { required: ec });
    }

    // Pass 1: count edges per face.
    for slot in &mut offsets[..fc + 1] {
        *slot = 0;
    }
    for he in half_edges.iter() {
        let face = he.face as usize;
        if face >= fc {
            return Err(CsrError::EntityOutOfRange { index: he.face });
        }
        offsets[face + 1] += 1;
    }

    // Prefix sum.
    let mut max_degree = 0u32;
    let mut acc = 0u32;
    for f in 0..fc {
        acc += offsets[f + 1];
        offsets[f + 1] = acc;
        let degree = offsets[f + 1] - offsets[f];
        if degree > max_degree {
            max_degree = degree;
        }
    }
    offsets[0] = 0;

    // Scatter pass: for each half-edge, the neighbour face is the twin's face
    // (or INVALID_INDEX for boundary).
    for he in half_edges.iter() {
        let face = he.face as usize;
        let neighbour = if he.twin == INVALID_INDEX {
            INVALID_INDEX
        } else {
            let twin_idx = he.twin as usize;
            match half_edges.get(twin_idx) {
                Some(twin) => twin.face,
                None => return Err(CsrError::HalfEdgeOutOfRange { index: he.twin }),
            }
        };
        let pos = offsets[face] as usize;
        neighbours[pos] = neighbour;
        offsets[face] += 1;
    }

    // Fix up offsets.
    for f in (1..=fc).rev() {
        offsets[f] = offsets[f - 1];
    }
    offsets[0] = 0;

    Ok(CsrSummary {
        entity_count: face_count,
        edge_count: ec as u32,
        max_degree,
    })
}

// ---------------------------------------------------------------------------
// Required buffer sizes
// ---------------------------------------------------------------------------

/// Required length of the `offsets` buffer for vertex adjacency.
#[inline]
pub fn required_vertex_offsets(vertex_count: u32) -> usize {
    vertex_count as usize + 1
}

/// Required length of the `neighbours` buffer for vertex adjacency.
#[inline]
pub fn required_vertex_neighbours(half_edge_count: usize) -> usize {
    half_edge_count
}

/// Required length of the `offsets` buffer for face adjacency.
#[inline]
pub fn required_face_offsets(face_count: u32) -> usize {
    face_count as usize + 1
}

/// Required length of the `neighbours` buffer for face adjacency.
#[inline]
pub fn required_face_neighbours(half_edge_count: usize) -> usize {
    half_edge_count
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::topology::{
        build_triangle_half_edges, required_edge_slots, EdgeSlot,
    };

    fn build_he(vertex_count: u32, triangles: &[[u32; 3]]) -> Vec<HalfEdge> {
        let n = triangles.len() * 3;
        let mut edges = vec![HalfEdge::default(); n];
        let mut slots = vec![EdgeSlot::default(); required_edge_slots(triangles.len())];
        build_triangle_half_edges(vertex_count, triangles, &mut edges, &mut slots).unwrap();
        edges
    }

    // --- vertex adjacency --------------------------------------------------

    #[test]
    fn vertex_csr_single_triangle() {
        let edges = build_he(3, &[[0, 1, 2]]);
        let mut offsets = [0u32; 4];
        let mut neighbours = [0u32; 3];
        let summary = build_vertex_adjacency_csr(3, &edges, &mut offsets, &mut neighbours).unwrap();
        assert_eq!(summary.entity_count, 3);
        assert_eq!(summary.edge_count, 3);
        assert_eq!(summary.max_degree, 1);
        // offsets: [0, 1, 2, 3]
        assert_eq!(offsets, [0, 1, 2, 3]);
        // Vertex 0 → 1, vertex 1 → 2, vertex 2 → 0
        assert_eq!(neighbours[0], 1);
        assert_eq!(neighbours[1], 2);
        assert_eq!(neighbours[2], 0);
    }

    #[test]
    fn vertex_csr_two_triangles_shared_edge() {
        // Triangles (0,1,2) and (2,1,3) — shared edge 1↔2.
        let edges = build_he(4, &[[0, 1, 2], [2, 1, 3]]);
        let mut offsets = [0u32; 5];
        let mut neighbours = [0u32; 6];
        let summary = build_vertex_adjacency_csr(4, &edges, &mut offsets, &mut neighbours).unwrap();
        assert_eq!(summary.edge_count, 6);
        assert_eq!(summary.max_degree, 2); // vertices 1 and 2 have degree 2
                                           // offsets: vertex 0 → 1 edge, vertex 1 → 2 edges, vertex 2 → 2 edges, vertex 3 → 1 edge
        assert_eq!(offsets, [0, 1, 3, 5, 6]);
        // Vertex 0: [1]
        assert_eq!(neighbours[0], 1);
        // Vertex 1: [2, 3] (from face 0: 1→2, from face 1: 1→3)
        assert_eq!(neighbours[1], 2);
        assert_eq!(neighbours[2], 3);
        // Vertex 2: [0, 1] (from face 0: 2→0, from face 1: 2→1)
        assert_eq!(neighbours[3], 0);
        assert_eq!(neighbours[4], 1);
        // Vertex 3: [2]
        assert_eq!(neighbours[5], 2);
    }

    #[test]
    fn vertex_csr_tetrahedron() {
        let edges = build_he(4, &[[0, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]]);
        let mut offsets = [0u32; 5];
        let mut neighbours = [0u32; 12];
        let summary = build_vertex_adjacency_csr(4, &edges, &mut offsets, &mut neighbours).unwrap();
        assert_eq!(summary.edge_count, 12);
        assert_eq!(summary.max_degree, 3); // every vertex has degree 3
        assert_eq!(offsets, [0, 3, 6, 9, 12]);
    }

    #[test]
    fn vertex_csr_rejects_small_buffers() {
        let edges = build_he(3, &[[0, 1, 2]]);
        let mut offsets = [0u32; 3]; // too small (needs 4)
        let mut neighbours = [0u32; 3];
        let err = build_vertex_adjacency_csr(3, &edges, &mut offsets, &mut neighbours).unwrap_err();
        assert_eq!(err, CsrError::OffsetBufferTooSmall { required: 4 });
    }

    #[test]
    fn vertex_csr_deterministic_across_runs() {
        let edges = build_he(4, &[[0, 1, 2], [2, 1, 3]]);
        let mut o1 = [0u32; 5];
        let mut n1 = [0u32; 6];
        let mut o2 = [0u32; 5];
        let mut n2 = [0u32; 6];
        build_vertex_adjacency_csr(4, &edges, &mut o1, &mut n1).unwrap();
        build_vertex_adjacency_csr(4, &edges, &mut o2, &mut n2).unwrap();
        assert_eq!(o1, o2);
        assert_eq!(n1, n2);
    }

    // --- face adjacency ----------------------------------------------------

    #[test]
    fn face_csr_single_triangle_all_boundary() {
        let edges = build_he(3, &[[0, 1, 2]]);
        let mut offsets = [0u32; 2];
        let mut neighbours = [0u32; 3];
        let summary = build_face_adjacency_csr(1, &edges, &mut offsets, &mut neighbours).unwrap();
        assert_eq!(summary.entity_count, 1);
        assert_eq!(summary.edge_count, 3);
        assert_eq!(summary.max_degree, 3);
        assert_eq!(offsets, [0, 3]);
        // All boundary → all INVALID_INDEX
        assert_eq!(neighbours, [INVALID_INDEX, INVALID_INDEX, INVALID_INDEX]);
    }

    #[test]
    fn face_csr_two_triangles_shared_edge() {
        let edges = build_he(4, &[[0, 1, 2], [2, 1, 3]]);
        let mut offsets = [0u32; 3];
        let mut neighbours = [0u32; 6];
        let summary = build_face_adjacency_csr(2, &edges, &mut offsets, &mut neighbours).unwrap();
        assert_eq!(summary.edge_count, 6);
        assert_eq!(summary.max_degree, 3);
        assert_eq!(offsets, [0, 3, 6]);
        // Face 0: edges 0,1,2. Edge 1 is twinned with edge 3 (face 1).
        //   edge 0 → boundary, edge 1 → face 1, edge 2 → boundary
        assert_eq!(neighbours[0], INVALID_INDEX);
        assert_eq!(neighbours[1], 1);
        assert_eq!(neighbours[2], INVALID_INDEX);
        // Face 1: edges 3,4,5. Edge 3 is twinned with edge 1 (face 0).
        //   edge 3 → face 0, edge 4 → boundary, edge 5 → boundary
        assert_eq!(neighbours[3], 0);
        assert_eq!(neighbours[4], INVALID_INDEX);
        assert_eq!(neighbours[5], INVALID_INDEX);
    }

    #[test]
    fn face_csr_tetrahedron_all_interior() {
        let edges = build_he(4, &[[0, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]]);
        let mut offsets = [0u32; 5];
        let mut neighbours = [0u32; 12];
        let summary = build_face_adjacency_csr(4, &edges, &mut offsets, &mut neighbours).unwrap();
        assert_eq!(summary.edge_count, 12);
        assert_eq!(summary.max_degree, 3);
        assert_eq!(offsets, [0, 3, 6, 9, 12]);
        // No boundary edges in a closed tetrahedron.
        assert!(
            neighbours.iter().all(|&n| n != INVALID_INDEX),
            "all faces should have valid neighbours"
        );
    }

    #[test]
    fn face_csr_deterministic_across_runs() {
        let edges = build_he(4, &[[0, 1, 2], [2, 1, 3]]);
        let mut o1 = [0u32; 3];
        let mut n1 = [0u32; 6];
        let mut o2 = [0u32; 3];
        let mut n2 = [0u32; 6];
        build_face_adjacency_csr(2, &edges, &mut o1, &mut n1).unwrap();
        build_face_adjacency_csr(2, &edges, &mut o2, &mut n2).unwrap();
        assert_eq!(o1, o2);
        assert_eq!(n1, n2);
    }

    // --- bytemuck cast round-trip ------------------------------------------

    #[test]
    fn csr_header_is_pod_and_round_trips() {
        let header = CsrHeader {
            entity_count: 42,
            edge_count: 108,
            offsets_byte_offset: 16,
            neighbours_byte_offset: 184,
        };
        let bytes: &[u8] = bytemuck::bytes_of(&header);
        let back: CsrHeader = *bytemuck::from_bytes(bytes);
        assert_eq!(header, back);
        assert_eq!(std::mem::size_of::<CsrHeader>(), 16);
    }

    #[test]
    fn csr_buffers_are_pod_castable() {
        let edges = build_he(4, &[[0, 1, 2], [2, 1, 3]]);
        let mut offsets = [0u32; 5];
        let mut neighbours = [0u32; 6];
        build_vertex_adjacency_csr(4, &edges, &mut offsets, &mut neighbours).unwrap();
        // bytemuck cast round-trip
        let off_bytes: &[u8] = bytemuck::cast_slice(&offsets);
        let nbr_bytes: &[u8] = bytemuck::cast_slice(&neighbours);
        let off_back: &[u32] = bytemuck::cast_slice(off_bytes);
        let nbr_back: &[u32] = bytemuck::cast_slice(nbr_bytes);
        assert_eq!(off_back, &offsets);
        assert_eq!(nbr_back, &neighbours);
    }

    // --- differential vs naive oracle --------------------------------------

    #[test]
    fn vertex_csr_matches_naive_oracle() {
        // Build a non-trivial mesh and compare CSR adjacency against a naive
        // O(V × E) scan.
        let triangles = [[0, 1, 2], [2, 1, 3], [0, 3, 1], [0, 2, 3]];
        let edges = build_he(4, &triangles);
        let vc = 4;
        let ec = edges.len();

        // Naive oracle: for each vertex, collect destinations in half-edge order.
        let mut naive: Vec<Vec<u32>> = vec![vec![]; vc];
        for he in &edges {
            let dest = edges[he.next as usize].origin;
            naive[he.origin as usize].push(dest);
        }

        // CSR build.
        let mut offsets = vec![0u32; vc + 1];
        let mut neighbours = vec![0u32; ec];
        build_vertex_adjacency_csr(vc as u32, &edges, &mut offsets, &mut neighbours).unwrap();

        // Compare.
        for v in 0..vc {
            let start = offsets[v] as usize;
            let end = offsets[v + 1] as usize;
            let csr_slice = &neighbours[start..end];
            assert_eq!(csr_slice, naive[v].as_slice(), "vertex {v} mismatch");
        }
    }

    #[test]
    fn face_csr_matches_naive_oracle() {
        let triangles = [[0, 1, 2], [2, 1, 3], [0, 3, 1], [0, 2, 3]];
        let edges = build_he(4, &triangles);
        let fc = triangles.len();
        let ec = edges.len();

        // Naive oracle.
        let mut naive: Vec<Vec<u32>> = vec![vec![]; fc];
        for he in &edges {
            let n = if he.twin == INVALID_INDEX {
                INVALID_INDEX
            } else {
                edges[he.twin as usize].face
            };
            naive[he.face as usize].push(n);
        }

        // CSR build.
        let mut offsets = vec![0u32; fc + 1];
        let mut neighbours = vec![0u32; ec];
        build_face_adjacency_csr(fc as u32, &edges, &mut offsets, &mut neighbours).unwrap();

        // Compare.
        for f in 0..fc {
            let start = offsets[f] as usize;
            let end = offsets[f + 1] as usize;
            let csr_slice = &neighbours[start..end];
            assert_eq!(csr_slice, naive[f].as_slice(), "face {f} mismatch");
        }
    }

    #[test]
    fn vertex_csr_boundary_mesh() {
        // Two triangles sharing only a vertex — open mesh, boundary edges.
        let edges = build_he(5, &[[0, 1, 2], [0, 3, 4]]);
        let mut offsets = [0u32; 6];
        let mut neighbours = [0u32; 6];
        let summary = build_vertex_adjacency_csr(5, &edges, &mut offsets, &mut neighbours).unwrap();
        assert_eq!(summary.edge_count, 6);
        // Vertex 0 has degree 2 (one from each triangle).
        assert_eq!(offsets[1] - offsets[0], 2);
        // Vertices 1,2,3,4 have degree 1 each.
        for v in 1..5 {
            assert_eq!(offsets[v + 1] - offsets[v], 1, "vertex {v} degree");
        }
    }
}
