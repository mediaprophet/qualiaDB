//! Connectivity invariants: components, boundary loops, Euler characteristic, genus.
//!
//! Given a half-edge mesh, these functions compute graph-theoretic invariants
//! using the CSR face-adjacency view from [`super::csr_adjacency`]. All
//! functions are caller-buffered and zero-heap (no `Vec`/`String`/`Box` in
//! any hot function; test helpers may allocate for setup only).
//!
//! # Invariants
//!
//! - **Connected components** — BFS over the face-adjacency graph (faces
//!   connected via twin links belong to the same component). Deterministic:
//!   seed = face 0, BFS visits neighbours in ascending face-index order.
//! - **Boundary loops** — count by walking boundary half-edges (twin ==
//!   `INVALID_INDEX`) and grouping them into cycles via `next` links.
//! - **Euler characteristic** — χ = V − E + F, where V = vertex count,
//!   E = unique undirected edges, F = face count.
//! - **Genus** — for a closed orientable surface: g = (2 − χ) / 2. For
//!   surfaces with boundary: g = (2 − χ − b) / 2, where b = boundary loop count.
//!   Returns `None` if the surface is non-orientable or the formula doesn't
//!   yield a non-negative integer (indicating invalid/inconsistent topology).

use super::topology::{HalfEdge, INVALID_INDEX};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors raised by the connectivity invariant functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectivityError {
    /// `labels` buffer too small (needs `face_count` entries).
    LabelBufferTooSmall { required: usize },
    /// `queue` buffer too small (needs `face_count` entries).
    QueueBufferTooSmall { required: usize },
    /// `visited` buffer too small (needs `half_edge_count` entries).
    VisitedBufferTooSmall { required: usize },
    /// A half-edge link points outside the half-edge array.
    HalfEdgeOutOfRange { index: u32 },
}

/// Summary of mesh connectivity invariants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectivitySummary {
    /// Number of connected components (face-connectivity via twin links).
    pub component_count: u32,
    /// Number of boundary loops (cycles of boundary half-edges).
    pub boundary_loop_count: u32,
    /// Euler characteristic χ = V − E + F.
    pub euler_characteristic: i32,
    /// Genus (None if non-orientable or inconsistent).
    pub genus: Option<u32>,
    /// Vertex count.
    pub vertex_count: u32,
    /// Edge count (unique undirected edges).
    pub edge_count: u32,
    /// Face count.
    pub face_count: u32,
}

// ---------------------------------------------------------------------------
// Connected components via BFS over face adjacency
// ---------------------------------------------------------------------------

/// Label connected components by BFS over the face-adjacency graph.
///
/// Two faces are in the same component if connected by a path of twin-linked
/// half-edges. `labels[f]` receives the component index (0-based, assigned in
/// ascending face-index order of first encounter). `queue` is a workspace
/// buffer of length `face_count`.
///
/// Returns the number of connected components.
///
/// Zero-heap. Deterministic.
pub fn label_components(
    face_count: u32,
    half_edges: &[HalfEdge],
    labels: &mut [u32],
    queue: &mut [u32],
) -> Result<u32, ConnectivityError> {
    let fc = face_count as usize;
    if labels.len() < fc {
        return Err(ConnectivityError::LabelBufferTooSmall { required: fc });
    }
    if queue.len() < fc {
        return Err(ConnectivityError::QueueBufferTooSmall { required: fc });
    }

    for l in &mut labels[..fc] {
        *l = INVALID_INDEX;
    }

    let mut component = 0u32;
    let mut seed = 0u32;

    while (seed as usize) < fc {
        // BFS from this seed.
        let mut head = 0usize;
        let mut tail = 0usize;
        queue[tail] = seed;
        tail += 1;
        labels[seed as usize] = component;

        while head < tail {
            let face = queue[head];
            head += 1;

            // Find all half-edges of this face and follow twins.
            for he in half_edges.iter() {
                if he.face != face {
                    continue;
                }
                if he.twin == INVALID_INDEX {
                    continue;
                }
                let twin_idx = he.twin as usize;
                if twin_idx >= half_edges.len() {
                    return Err(ConnectivityError::HalfEdgeOutOfRange {
                        index: he.twin,
                    });
                }
                let neighbour = half_edges[twin_idx].face;
                if neighbour == INVALID_INDEX || (neighbour as usize) >= fc {
                    continue;
                }
                if labels[neighbour as usize] == INVALID_INDEX {
                    labels[neighbour as usize] = component;
                    queue[tail] = neighbour;
                    tail += 1;
                }
            }
        }

        component += 1;

        // Advance seed to next unlabelled face.
        while (seed as usize) < fc && labels[seed as usize] != INVALID_INDEX {
            seed += 1;
        }
    }

    Ok(component)
}

// ---------------------------------------------------------------------------
// Boundary loop counting
// ---------------------------------------------------------------------------

/// Count boundary loops by walking cycles of boundary half-edges.
///
/// A boundary half-edge has `twin == INVALID_INDEX`. To walk a boundary loop,
/// from each boundary half-edge we go to `next` (same face), then if that edge
/// has a twin, cross to the twin and go to its `next`, repeating until we find
/// the next boundary edge. This rotates around the destination vertex until
/// exiting the interior.
///
/// `visited` is a workspace buffer of length `half_edges.len()`.
///
/// Zero-heap. Deterministic.
pub fn count_boundary_loops(
    half_edges: &[HalfEdge],
    visited: &mut [bool],
) -> Result<u32, ConnectivityError> {
    if visited.len() < half_edges.len() {
        return Err(ConnectivityError::VisitedBufferTooSmall {
            required: half_edges.len(),
        });
    }

    for v in &mut visited[..half_edges.len()] {
        *v = false;
    }

    let mut loop_count = 0u32;
    let he_len = half_edges.len();

    for start in 0..he_len {
        if visited[start] || half_edges[start].twin != INVALID_INDEX {
            continue;
        }

        // Walk the boundary cycle starting at this boundary half-edge.
        loop_count += 1;
        let mut cur = start as u32;
        loop {
            if (cur as usize) >= he_len {
                return Err(ConnectivityError::HalfEdgeOutOfRange { index: cur });
            }
            if visited[cur as usize] {
                break;
            }
            visited[cur as usize] = true;

            // Find the next boundary half-edge by rotating around the
            // destination vertex: go to `next`, then cross twins until we
            // exit the interior.
            let next_in_face = half_edges[cur as usize].next;
            if (next_in_face as usize) >= he_len {
                return Err(ConnectivityError::HalfEdgeOutOfRange {
                    index: next_in_face,
                });
            }

            let mut candidate = next_in_face;
            // If the candidate is interior (has a twin), rotate: cross to
            // twin, go to its next, repeat. Bounded by he_len iterations.
            let mut rotations = 0;
            while half_edges[candidate as usize].twin != INVALID_INDEX {
                let twin = half_edges[candidate as usize].twin;
                if (twin as usize) >= he_len {
                    return Err(ConnectivityError::HalfEdgeOutOfRange {
                        index: twin,
                    });
                }
                let next_after_twin = half_edges[twin as usize].next;
                if (next_after_twin as usize) >= he_len {
                    return Err(ConnectivityError::HalfEdgeOutOfRange {
                        index: next_after_twin,
                    });
                }
                candidate = next_after_twin;
                rotations += 1;
                if rotations > he_len {
                    // Malformed mesh — stuck in a cycle with no boundary.
                    return Err(ConnectivityError::HalfEdgeOutOfRange {
                        index: candidate,
                    });
                }
            }

            cur = candidate;
            if cur as usize == start {
                break;
            }
        }
    }

    Ok(loop_count)
}

// ---------------------------------------------------------------------------
// Euler characteristic and genus
// ---------------------------------------------------------------------------

/// Compute the Euler characteristic χ = V − E + F.
///
/// - `vertex_count`: number of unique vertices.
/// - `face_count`: number of faces.
/// - `half_edges`: the half-edge array. Unique undirected edges are counted
///   as (total half-edges + boundary half-edges) / 2, since each interior edge
///   contributes 2 half-edges and each boundary edge contributes 1.
#[inline]
pub fn euler_characteristic(
    vertex_count: u32,
    face_count: u32,
    half_edges: &[HalfEdge],
) -> i32 {
    let boundary = half_edges
        .iter()
        .filter(|he| he.twin == INVALID_INDEX)
        .count() as u32;
    let edge_count = (half_edges.len() as u32 + boundary) / 2;
    (vertex_count as i32) - (edge_count as i32) + (face_count as i32)
}

/// Compute genus from the Euler characteristic and boundary loop count.
///
/// For a closed orientable surface: g = (2 − χ) / 2.
/// For a surface with b boundary loops: g = (2 − χ − b) / 2.
///
/// Returns `None` if the result is negative or non-integer (indicating
/// non-orientable or inconsistent topology).
#[inline]
pub fn genus_from_euler(euler: i32, boundary_loops: u32) -> Option<u32> {
    let numerator = 2 - euler - boundary_loops as i32;
    if numerator < 0 || numerator % 2 != 0 {
        return None;
    }
    Some((numerator / 2) as u32)
}

// ---------------------------------------------------------------------------
// Full connectivity summary
// ---------------------------------------------------------------------------

/// Compute the full connectivity summary in one call.
///
/// Requires workspace buffers:
/// - `labels`: `face_count` entries.
/// - `queue`: `face_count` entries.
/// - `visited`: `half_edges.len()` entries.
///
/// Zero-heap. Deterministic.
pub fn compute_connectivity(
    vertex_count: u32,
    face_count: u32,
    half_edges: &[HalfEdge],
    labels: &mut [u32],
    queue: &mut [u32],
    visited: &mut [bool],
) -> Result<ConnectivitySummary, ConnectivityError> {
    let component_count = label_components(face_count, half_edges, labels, queue)?;
    let boundary_loop_count = count_boundary_loops(half_edges, visited)?;

    let boundary = half_edges
        .iter()
        .filter(|he| he.twin == INVALID_INDEX)
        .count() as u32;
    let edge_count = (half_edges.len() as u32 + boundary) / 2;
    let euler = euler_characteristic(vertex_count, face_count, half_edges);
    let genus = genus_from_euler(euler, boundary_loop_count);

    Ok(ConnectivitySummary {
        component_count,
        boundary_loop_count,
        euler_characteristic: euler,
        genus,
        vertex_count,
        edge_count,
        face_count,
    })
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

    fn build_he(
        vertex_count: u32,
        triangles: &[[u32; 3]],
    ) -> (Vec<HalfEdge>, u32) {
        let n = triangles.len() * 3;
        let mut edges = vec![HalfEdge::default(); n];
        let mut slots = vec![EdgeSlot::default(); required_edge_slots(triangles.len())];
        let summary =
            build_triangle_half_edges(vertex_count, triangles, &mut edges, &mut slots).unwrap();
        (edges, summary.boundary_half_edges)
    }

    // --- connected components ----------------------------------------------

    #[test]
    fn single_triangle_one_component() {
        let (edges, _) = build_he(3, &[[0, 1, 2]]);
        let mut labels = [0u32; 1];
        let mut queue = [0u32; 1];
        let count = label_components(1, &edges, &mut labels, &mut queue).unwrap();
        assert_eq!(count, 1);
        assert_eq!(labels, [0]);
    }

    #[test]
    fn two_disjoint_triangles_two_components() {
        let (edges, _) = build_he(6, &[[0, 1, 2], [3, 4, 5]]);
        let mut labels = [0u32; 2];
        let mut queue = [0u32; 2];
        let count = label_components(2, &edges, &mut labels, &mut queue).unwrap();
        assert_eq!(count, 2);
        assert_eq!(labels[0], 0);
        assert_eq!(labels[1], 1);
    }

    #[test]
    fn two_shared_edge_triangles_one_component() {
        let (edges, _) = build_he(4, &[[0, 1, 2], [2, 1, 3]]);
        let mut labels = [0u32; 2];
        let mut queue = [0u32; 2];
        let count = label_components(2, &edges, &mut labels, &mut queue).unwrap();
        assert_eq!(count, 1);
        assert_eq!(labels, [0, 0]);
    }

    #[test]
    fn tetrahedron_one_component() {
        let (edges, _) = build_he(4, &[[0, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]]);
        let mut labels = [0u32; 4];
        let mut queue = [0u32; 4];
        let count = label_components(4, &edges, &mut labels, &mut queue).unwrap();
        assert_eq!(count, 1);
        assert!(labels.iter().all(|&l| l == 0));
    }

    #[test]
    fn components_deterministic() {
        let (edges, _) = build_he(6, &[[0, 1, 2], [3, 4, 5]]);
        let mut l1 = [0u32; 2];
        let mut q1 = [0u32; 2];
        let mut l2 = [0u32; 2];
        let mut q2 = [0u32; 2];
        label_components(2, &edges, &mut l1, &mut q1).unwrap();
        label_components(2, &edges, &mut l2, &mut q2).unwrap();
        assert_eq!(l1, l2);
    }

    // --- boundary loops ----------------------------------------------------

    #[test]
    fn single_triangle_one_boundary_loop() {
        let (edges, _) = build_he(3, &[[0, 1, 2]]);
        let mut visited = [false; 3];
        let loops = count_boundary_loops(&edges, &mut visited).unwrap();
        assert_eq!(loops, 1);
    }

    #[test]
    fn two_shared_edge_triangles_one_boundary_loop() {
        // Two triangles sharing an edge → one boundary loop of 4 edges.
        let (edges, _) = build_he(4, &[[0, 1, 2], [2, 1, 3]]);
        let mut visited = [false; 6];
        let loops = count_boundary_loops(&edges, &mut visited).unwrap();
        assert_eq!(loops, 1);
    }

    #[test]
    fn tetrahedron_zero_boundary_loops() {
        let (edges, _) = build_he(4, &[[0, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]]);
        let mut visited = [false; 12];
        let loops = count_boundary_loops(&edges, &mut visited).unwrap();
        assert_eq!(loops, 0);
    }

    #[test]
    fn two_disjoint_triangles_two_boundary_loops() {
        let (edges, _) = build_he(6, &[[0, 1, 2], [3, 4, 5]]);
        let mut visited = [false; 6];
        let loops = count_boundary_loops(&edges, &mut visited).unwrap();
        assert_eq!(loops, 2);
    }

    // --- Euler characteristic ----------------------------------------------

    #[test]
    fn single_triangle_euler_1() {
        // V=3, E=3, F=1 → χ=1
        let (edges, _) = build_he(3, &[[0, 1, 2]]);
        let chi = euler_characteristic(3, 1, &edges);
        assert_eq!(chi, 1);
    }

    #[test]
    fn tetrahedron_euler_2() {
        // V=4, E=6, F=4 → χ=2
        let (edges, _) = build_he(4, &[[0, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]]);
        let chi = euler_characteristic(4, 4, &edges);
        assert_eq!(chi, 2);
    }

    #[test]
    fn two_shared_edge_triangles_euler_1() {
        // V=4, E=5, F=2 → χ=1
        let (edges, _) = build_he(4, &[[0, 1, 2], [2, 1, 3]]);
        let chi = euler_characteristic(4, 2, &edges);
        assert_eq!(chi, 1);
    }

    // --- genus --------------------------------------------------------------

    #[test]
    fn tetrahedron_genus_0() {
        // χ=2, b=0 → g=(2-2-0)/2=0
        let g = genus_from_euler(2, 0);
        assert_eq!(g, Some(0));
    }

    #[test]
    fn disk_genus_0() {
        // Single triangle: χ=1, b=1 → g=(2-1-1)/2=0
        let g = genus_from_euler(1, 1);
        assert_eq!(g, Some(0));
    }

    #[test]
    fn torus_genus_1() {
        // Torus: χ=0, b=0 → g=(2-0-0)/2=1
        let g = genus_from_euler(0, 0);
        assert_eq!(g, Some(1));
    }

    #[test]
    fn invalid_topology_genus_none() {
        // χ=3, b=0 → g=(2-3-0)/2 = -1/2 → None
        let g = genus_from_euler(3, 0);
        assert_eq!(g, None);
    }

    // --- full summary -------------------------------------------------------

    #[test]
    fn summary_tetrahedron() {
        let (edges, _) = build_he(4, &[[0, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]]);
        let mut labels = [0u32; 4];
        let mut queue = [0u32; 4];
        let mut visited = [false; 12];
        let summary = compute_connectivity(4, 4, &edges, &mut labels, &mut queue, &mut visited)
            .unwrap();
        assert_eq!(summary.component_count, 1);
        assert_eq!(summary.boundary_loop_count, 0);
        assert_eq!(summary.euler_characteristic, 2);
        assert_eq!(summary.genus, Some(0));
        assert_eq!(summary.vertex_count, 4);
        assert_eq!(summary.edge_count, 6);
        assert_eq!(summary.face_count, 4);
    }

    #[test]
    fn summary_single_triangle() {
        let (edges, _) = build_he(3, &[[0, 1, 2]]);
        let mut labels = [0u32; 1];
        let mut queue = [0u32; 1];
        let mut visited = [false; 3];
        let summary = compute_connectivity(3, 1, &edges, &mut labels, &mut queue, &mut visited)
            .unwrap();
        assert_eq!(summary.component_count, 1);
        assert_eq!(summary.boundary_loop_count, 1);
        assert_eq!(summary.euler_characteristic, 1);
        assert_eq!(summary.genus, Some(0));
        assert_eq!(summary.edge_count, 3);
    }

    #[test]
    fn summary_two_disjoint_triangles() {
        let (edges, _) = build_he(6, &[[0, 1, 2], [3, 4, 5]]);
        let mut labels = [0u32; 2];
        let mut queue = [0u32; 2];
        let mut visited = [false; 6];
        let summary = compute_connectivity(6, 2, &edges, &mut labels, &mut queue, &mut visited)
            .unwrap();
        assert_eq!(summary.component_count, 2);
        assert_eq!(summary.boundary_loop_count, 2);
        // V=6, E=6, F=2 → χ=2
        assert_eq!(summary.euler_characteristic, 2);
        // g=(2-2-2)/2 = -1 → None (two disjoint disks, not a single surface)
        assert_eq!(summary.genus, None);
    }

    #[test]
    fn summary_deterministic_across_runs() {
        let (edges, _) = build_he(4, &[[0, 1, 2], [2, 1, 3]]);
        let mut l1 = [0u32; 2];
        let mut q1 = [0u32; 2];
        let mut v1 = [false; 6];
        let mut l2 = [0u32; 2];
        let mut q2 = [0u32; 2];
        let mut v2 = [false; 6];
        let s1 = compute_connectivity(4, 2, &edges, &mut l1, &mut q1, &mut v1).unwrap();
        let s2 = compute_connectivity(4, 2, &edges, &mut l2, &mut q2, &mut v2).unwrap();
        assert_eq!(s1, s2);
        assert_eq!(l1, l2);
    }

    // --- buffer size errors ------------------------------------------------

    #[test]
    fn label_components_rejects_small_label_buffer() {
        let (edges, _) = build_he(3, &[[0, 1, 2]]);
        let mut labels = [0u32; 0];
        let mut queue = [0u32; 1];
        let err = label_components(1, &edges, &mut labels, &mut queue).unwrap_err();
        assert_eq!(err, ConnectivityError::LabelBufferTooSmall { required: 1 });
    }

    #[test]
    fn count_boundary_loops_rejects_small_visited() {
        let (edges, _) = build_he(3, &[[0, 1, 2]]);
        let mut visited = [false; 2];
        let err = count_boundary_loops(&edges, &mut visited).unwrap_err();
        assert_eq!(err, ConnectivityError::VisitedBufferTooSmall { required: 3 });
    }
}
