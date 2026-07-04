//! Surface-mesh view: SoA vertex→/face→half-edge maps + allocation-free circulators (P2.2).
//!
//! A read-only view over a half-edge graph ([`HalfEdge`] array) that provides:
//! - **SoA vertex→half-edge map** — for each vertex, one representative outgoing
//!   half-edge (caller-owned buffer, zero-heap).
//! - **SoA face→half-edge map** — for each face, one representative half-edge
//!   (caller-owned buffer, zero-heap).
//! - **Allocation-free circulators** — one-ring vertex circulator, face-loop
//!   circulator, boundary-loop walker. These are iterator-like structs that
//!   walk the half-edge graph without any heap allocation.
//!
//! ## Design
//!
//! The view holds only references (`&[HalfEdge]`) and pre-built index maps.
//! All maps are built into caller-owned `&mut [u32]` buffers — no `Vec`,
//! `String`, or `Box` in any function. The circulators are `Copy` structs
//! that carry a reference to the half-edge array and a current index; they
//! implement `Iterator` but do not allocate.
//!
//! ## Determinism
//!
//! The derived maps are deterministic: the vertex→half-edge map picks the
//! **lowest-indexed** outgoing half-edge for each vertex (not the last one
//! encountered), so the output is byte-identical across runs regardless of
//! face iteration order. The face→half-edge map picks the first half-edge
//! of each face (index `face * 3`).

use super::topology::{HalfEdge, TopologySummary, INVALID_INDEX};

/// Error type for surface-mesh view construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceMeshError {
    /// The vertex→half-edge buffer is too small.
    VertexMapTooSmall { required: usize },
    /// The face→half-edge buffer is too small.
    FaceMapTooSmall { required: usize },
    /// A half-edge index in the graph is out of range.
    HalfEdgeOutOfRange { index: u32 },
    /// A vertex index in the graph is out of range.
    VertexOutOfRange { index: u32 },
    /// A face index in the graph is out of range.
    FaceOutOfRange { index: u32 },
}

/// A read-only surface-mesh view over a half-edge graph.
///
/// Holds references to the half-edge array and the pre-built vertex→/face→
/// half-edge maps. The maps are built by [`build_surface_mesh_maps`] into
/// caller-owned buffers; this struct borrows them.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceMeshView<'a> {
    half_edges: &'a [HalfEdge],
    vertex_to_he: &'a [u32],
    face_to_he: &'a [u32],
}

impl<'a> SurfaceMeshView<'a> {
    /// Construct a view from the half-edge array and pre-built maps.
    ///
    /// The maps must have been built by [`build_surface_mesh_maps`].
    pub fn new(
        half_edges: &'a [HalfEdge],
        vertex_to_he: &'a [u32],
        face_to_he: &'a [u32],
    ) -> Self {
        Self {
            half_edges,
            vertex_to_he,
            face_to_he,
        }
    }

    /// The number of half-edges in the graph.
    #[inline]
    pub fn half_edge_count(&self) -> usize {
        self.half_edges.len()
    }

    /// Get a half-edge by index.
    #[inline]
    pub fn half_edge(&self, index: u32) -> Result<HalfEdge, SurfaceMeshError> {
        let he = self
            .half_edges
            .get(index as usize)
            .ok_or(SurfaceMeshError::HalfEdgeOutOfRange { index })?;
        Ok(*he)
    }

    /// The representative outgoing half-edge for a vertex, or `INVALID_INDEX`
    /// if the vertex has no incident half-edges (isolated vertex).
    #[inline]
    pub fn vertex_half_edge(&self, vertex: u32) -> Result<u32, SurfaceMeshError> {
        let he = self
            .vertex_to_he
            .get(vertex as usize)
            .ok_or(SurfaceMeshError::VertexOutOfRange { index: vertex })?;
        Ok(*he)
    }

    /// The representative half-edge for a face.
    #[inline]
    pub fn face_half_edge(&self, face: u32) -> Result<u32, SurfaceMeshError> {
        let he = self
            .face_to_he
            .get(face as usize)
            .ok_or(SurfaceMeshError::FaceOutOfRange { index: face })?;
        Ok(*he)
    }

    /// Create a one-ring vertex circulator around `vertex`.
    ///
    /// Visits the vertices adjacent to `vertex` in counter-clockwise order
    /// (following `twin` then `next`). For boundary vertices, the circulator
    /// stops when it reaches the boundary (it does not wrap around).
    pub fn one_ring(&self, vertex: u32) -> Result<OneRingCirculator<'a>, SurfaceMeshError> {
        let start = self.vertex_half_edge(vertex)?;
        if start == INVALID_INDEX {
            return Ok(OneRingCirculator::empty(self.half_edges));
        }
        Ok(OneRingCirculator::new(self.half_edges, start))
    }

    /// Create a face-loop circulator around `face`.
    ///
    /// Visits the 3 half-edges of the triangle `face` in order.
    pub fn face_loop(&self, face: u32) -> Result<FaceLoopCirculator<'a>, SurfaceMeshError> {
        let start = self.face_half_edge(face)?;
        if start == INVALID_INDEX {
            return Ok(FaceLoopCirculator::empty(self.half_edges));
        }
        Ok(FaceLoopCirculator::new(self.half_edges, start))
    }

    /// Create a boundary-loop walker starting from `boundary_half_edge`.
    ///
    /// Walks along boundary half-edges (those with `twin == INVALID_INDEX`)
    /// by following `next` until it returns to the start. The starting
    /// half-edge must be a boundary half-edge.
    pub fn boundary_loop(
        &self,
        boundary_half_edge: u32,
    ) -> Result<BoundaryLoopWalker<'a>, SurfaceMeshError> {
        let he = self.half_edge(boundary_half_edge)?;
        if he.twin != INVALID_INDEX {
            return Err(SurfaceMeshError::HalfEdgeOutOfRange {
                index: boundary_half_edge,
            });
        }
        Ok(BoundaryLoopWalker::new(self.half_edges, boundary_half_edge))
    }

    /// Find all boundary half-edges (those with `twin == INVALID_INDEX`).
    ///
    /// Writes the indices of boundary half-edges into `out` and returns the
    /// count. If `out` is too small, returns an error with the required size.
    pub fn collect_boundary_half_edges(
        &self,
        out: &mut [u32],
    ) -> Result<usize, SurfaceMeshError> {
        let mut count = 0;
        for (i, he) in self.half_edges.iter().enumerate() {
            if he.twin == INVALID_INDEX {
                if count >= out.len() {
                    return Err(SurfaceMeshError::HalfEdgeOutOfRange {
                        index: out.len() as u32,
                    });
                }
                out[count] = i as u32;
                count += 1;
            }
        }
        Ok(count)
    }
}

/// Build the vertex→half-edge and face→half-edge maps into caller-owned buffers.
///
/// - `vertex_to_he`: one entry per vertex. Each entry is the **lowest-indexed**
///   outgoing half-edge for that vertex (deterministic). Isolated vertices get
///   `INVALID_INDEX`.
/// - `face_to_he`: one entry per face. Each entry is the first half-edge of
///   the face (index `face * 3`).
///
/// Both buffers must be initialized to `INVALID_INDEX` before calling (this
/// function fills them completely, so the initial value doesn't matter — it
/// writes every entry).
pub fn build_surface_mesh_maps(
    summary: TopologySummary,
    half_edges: &[HalfEdge],
    vertex_to_he: &mut [u32],
    face_to_he: &mut [u32],
) -> Result<(), SurfaceMeshError> {
    if vertex_to_he.len() < summary.vertex_count as usize {
        return Err(SurfaceMeshError::VertexMapTooSmall {
            required: summary.vertex_count as usize,
        });
    }
    if face_to_he.len() < summary.face_count as usize {
        return Err(SurfaceMeshError::FaceMapTooSmall {
            required: summary.face_count as usize,
        });
    }

    // Initialize all entries to INVALID_INDEX.
    for v in vertex_to_he.iter_mut() {
        *v = INVALID_INDEX;
    }
    for f in face_to_he.iter_mut() {
        *f = INVALID_INDEX;
    }

    // Build vertex→half-edge map: pick the lowest-indexed outgoing half-edge.
    for (i, he) in half_edges.iter().enumerate() {
        let origin = he.origin;
        if origin == INVALID_INDEX {
            continue;
        }
        if (origin as usize) < vertex_to_he.len() {
            // Keep the lowest index (deterministic).
            if vertex_to_he[origin as usize] == INVALID_INDEX
                || (i as u32) < vertex_to_he[origin as usize]
            {
                vertex_to_he[origin as usize] = i as u32;
            }
        }
    }

    // Build face→half-edge map: first half-edge of each face (face * 3).
    for face in 0..summary.face_count as usize {
        let he_index = face * 3;
        if he_index < half_edges.len() {
            face_to_he[face] = he_index as u32;
        }
    }

    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────
//  Circulators (allocation-free iterators over the half-edge graph)
// ──────────────────────────────────────────────────────────────────────────

/// One-ring vertex circulator: visits vertices adjacent to a center vertex.
///
/// Walks CCW around the center vertex by rotating through adjacent faces:
/// from outgoing half-edge `h` (origin = center), the neighbor is
/// `destination(h) = origin(next(h))`, and the next outgoing half-edge is
/// `next(twin(h))` (the half-edge after the twin in the adjacent face, which
/// also has origin = center). This visits one neighbor per incident face.
///
/// For **boundary vertices**, the rotation stops when `twin(h) == INVALID_INDEX`
/// (no adjacent face). The one-ring of a boundary vertex has one more vertex
/// than the number of incident faces — the extra vertex sits at the other end
/// of the boundary edge incident to the start half-edge's face. That neighbor
/// is `origin(prev(start))`, yielded as the final element. For interior
/// vertices the walk wraps fully around and no extra neighbor is needed.
#[derive(Debug, Clone, Copy)]
pub struct OneRingCirculator<'a> {
    half_edges: &'a [HalfEdge],
    start: u32,
    current: u32,
    /// Whether we still need to yield the boundary neighbor before stopping.
    pending_boundary: bool,
    /// The boundary neighbor to yield after the walk hits a boundary.
    boundary_neighbor: u32,
    done: bool,
}

impl<'a> OneRingCirculator<'a> {
    /// Create an empty circulator (for isolated vertices).
    pub fn empty(half_edges: &'a [HalfEdge]) -> Self {
        Self {
            half_edges,
            start: INVALID_INDEX,
            current: INVALID_INDEX,
            pending_boundary: false,
            boundary_neighbor: INVALID_INDEX,
            done: true,
        }
    }

    /// Create a circulator starting at half-edge `start`.
    pub fn new(half_edges: &'a [HalfEdge], start: u32) -> Self {
        Self {
            half_edges,
            start,
            current: start,
            pending_boundary: false,
            boundary_neighbor: INVALID_INDEX,
            done: false,
        }
    }
}

impl<'a> Iterator for OneRingCirculator<'a> {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        // Yield the deferred boundary neighbor (the vertex at the other end of
        // the start face's boundary edge) before stopping.
        if self.pending_boundary {
            self.pending_boundary = false;
            self.done = true;
            return Some(self.boundary_neighbor);
        }
        if self.done {
            return None;
        }
        if self.current == INVALID_INDEX {
            self.done = true;
            return None;
        }

        let he = *self
            .half_edges
            .get(self.current as usize)
            .expect("half-edge index in range");

        // The neighbor is destination(h) = origin(next(h)). This is the same
        // as origin(twin(h)) for interior edges; for boundary edges there is
        // no twin, so we always use origin(next(h)).
        let next_he = *self
            .half_edges
            .get(he.next as usize)
            .expect("next index in range");
        let neighbor = next_he.origin;

        if he.twin == INVALID_INDEX {
            // Hit a boundary: the center vertex is a boundary vertex. The
            // one-ring has one extra neighbor — the vertex at the other end
            // of the boundary edge in the start half-edge's face. For a
            // triangle face, prev(start) = next(next(start)), and the extra
            // neighbor is origin(prev(start)).
            let start_he = self.half_edges[self.start as usize];
            let start_next = self.half_edges[start_he.next as usize];
            let start_prev = self.half_edges[start_next.next as usize];
            self.boundary_neighbor = start_prev.origin;
            self.pending_boundary = true;
            return Some(neighbor);
        }

        // Rotate CCW: next(twin(h)) is the outgoing half-edge from the center
        // vertex in the adjacent face.
        let twin_he = self.half_edges[he.twin as usize];
        self.current = twin_he.next;

        if self.current == self.start {
            // Full circle — interior vertex, all neighbors visited.
            self.done = true;
        }

        Some(neighbor)
    }
}

/// Face-loop circulator: visits the half-edges of a triangle face in order.
#[derive(Debug, Clone, Copy)]
pub struct FaceLoopCirculator<'a> {
    half_edges: &'a [HalfEdge],
    /// The starting half-edge. Retained for debugging; the iterator only
    /// needs `current` and `count` to walk a triangle face.
    #[allow(dead_code)]
    start: u32,
    current: u32,
    count: u8,
}

impl<'a> FaceLoopCirculator<'a> {
    pub fn empty(half_edges: &'a [HalfEdge]) -> Self {
        Self {
            half_edges,
            start: INVALID_INDEX,
            current: INVALID_INDEX,
            count: 3,
        }
    }

    pub fn new(half_edges: &'a [HalfEdge], start: u32) -> Self {
        Self {
            half_edges,
            start,
            current: start,
            count: 0,
        }
    }
}

impl<'a> Iterator for FaceLoopCirculator<'a> {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.count >= 3 || self.current == INVALID_INDEX {
            return None;
        }
        let result = self.current;
        let he = self.half_edges[self.current as usize];
        self.current = he.next;
        self.count += 1;
        Some(result)
    }
}

/// Boundary-loop walker: walks along boundary half-edges.
///
/// Starting from a boundary half-edge, follows `next` until it returns to
/// the start. Each iteration yields the next boundary half-edge index.
#[derive(Debug, Clone, Copy)]
pub struct BoundaryLoopWalker<'a> {
    half_edges: &'a [HalfEdge],
    start: u32,
    current: u32,
    first: bool,
}

impl<'a> BoundaryLoopWalker<'a> {
    pub fn new(half_edges: &'a [HalfEdge], start: u32) -> Self {
        Self {
            half_edges,
            start,
            current: start,
            first: true,
        }
    }
}

impl<'a> Iterator for BoundaryLoopWalker<'a> {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if !self.first && self.current == self.start {
            return None;
        }
        self.first = false;

        let he = self.half_edges.get(self.current as usize)?;
        if he.twin != INVALID_INDEX {
            return None; // Not a boundary half-edge.
        }

        let result = self.current;
        // Advance to the next boundary half-edge: follow next until we find
        // a boundary half-edge.
        let mut next = he.next;
        loop {
            let next_he = self.half_edges.get(next as usize)?;
            if next_he.twin == INVALID_INDEX {
                break;
            }
            next = next_he.twin;
            let twin_he = self.half_edges[next as usize];
            next = twin_he.next;
        }
        self.current = next;
        Some(result)
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::topology::{
        build_triangle_half_edges, required_edge_slots, EdgeSlot, HalfEdge, INVALID_INDEX,
    };

    /// Build a half-edge graph and surface-mesh view for testing.
    fn build_view(
        vertex_count: u32,
        triangles: &[[u32; 3]],
    ) -> (Vec<HalfEdge>, Vec<u32>, Vec<u32>, SurfaceMeshView<'static>) {
        let mut edges = vec![HalfEdge::default(); triangles.len() * 3];
        let slot_count = required_edge_slots(triangles.len());
        let mut slots = vec![EdgeSlot::default(); slot_count];
        let summary = build_triangle_half_edges(vertex_count, triangles, &mut edges, &mut slots)
            .expect("build half-edges");

        let mut v2he = vec![INVALID_INDEX; vertex_count as usize];
        let mut f2he = vec![INVALID_INDEX; triangles.len()];
        build_surface_mesh_maps(summary, &edges, &mut v2he, &mut f2he).expect("build maps");

        // SAFETY: we extend the lifetime to 'static for the test; the view
        // only borrows during the test function's scope.
        let view = SurfaceMeshView::new(
            unsafe { std::mem::transmute(&edges[..]) },
            unsafe { std::mem::transmute(&v2he[..]) },
            unsafe { std::mem::transmute(&f2he[..]) },
        );
        (edges, v2he, f2he, view)
    }

    #[test]
    fn single_triangle_face_loop() {
        let triangles = [[0, 1, 2]];
        let (_edges, _v2he, _f2he, view) = build_view(3, &triangles);

        let face_loop: Vec<u32> = view.face_loop(0).unwrap().collect();
        assert_eq!(face_loop, vec![0, 1, 2]);
    }

    #[test]
    fn single_triangle_one_ring_vertex_1() {
        // Triangle (0,1,2): vertex 1's one-ring should be {0, 2}.
        let triangles = [[0, 1, 2]];
        let (_edges, _v2he, _f2he, view) = build_view(3, &triangles);

        let ring: Vec<u32> = view.one_ring(1).unwrap().collect();
        assert_eq!(ring.len(), 2);
        assert!(ring.contains(&0));
        assert!(ring.contains(&2));
    }

    #[test]
    fn single_triangle_one_ring_vertex_0() {
        let triangles = [[0, 1, 2]];
        let (_edges, _v2he, _f2he, view) = build_view(3, &triangles);

        let ring: Vec<u32> = view.one_ring(0).unwrap().collect();
        assert_eq!(ring.len(), 2);
        assert!(ring.contains(&1));
        assert!(ring.contains(&2));
    }

    #[test]
    fn two_triangles_one_ring_shared_vertex() {
        // Two triangles sharing edge (1,2):
        //   T0: (0,1,2), T1: (2,1,3)
        // Vertex 1's one-ring: {0, 2, 3}
        let triangles = [[0, 1, 2], [2, 1, 3]];
        let (_edges, _v2he, _f2he, view) = build_view(4, &triangles);

        let ring: Vec<u32> = view.one_ring(1).unwrap().collect();
        assert_eq!(ring.len(), 3);
        assert!(ring.contains(&0));
        assert!(ring.contains(&2));
        assert!(ring.contains(&3));
    }

    #[test]
    fn two_triangles_one_ring_boundary_vertex() {
        // Vertex 0 is a boundary vertex (only in T0).
        // Its one-ring: {1, 2}
        let triangles = [[0, 1, 2], [2, 1, 3]];
        let (_edges, _v2he, _f2he, view) = build_view(4, &triangles);

        let ring: Vec<u32> = view.one_ring(0).unwrap().collect();
        assert_eq!(ring.len(), 2);
        assert!(ring.contains(&1));
        assert!(ring.contains(&2));
    }

    #[test]
    fn two_triangles_boundary_loop() {
        // Two triangles sharing edge (1,2):
        //   T0: (0,1,2), T1: (2,1,3)
        // Boundary: 0→1, 2→0 (from T0) and 1→3, 3→2 (from T1)
        // Boundary loop starting from edge 0 (origin=0, next→1):
        //   he0: origin=0, next=he1 (origin=1) → boundary
        //   he1: origin=1, twin=he3 → NOT boundary
        //   he2: origin=2, next=he0 → boundary
        //   he3: origin=1, ... wait let me recompute.
        // T0 edges: he0=(0→1), he1=(1→2), he2=(2→0)
        // T1 edges: he3=(2→1), he4=(1→3), he5=(3→2)
        // Twin: he1(1→2) ↔ he3(2→1)
        // Boundary: he0(0→1), he2(2→0), he4(1→3), he5(3→2)
        // Boundary loop: he0 → he4 → he5 → he2 → he0
        //   (0→1, then 1→3, then 3→2, then 2→0, back to 0→1)
        let triangles = [[0, 1, 2], [2, 1, 3]];
        let (_edges, _v2he, _f2he, view) = build_view(4, &triangles);

        // Find boundary half-edges
        let mut boundary = [0u32; 8];
        let count = view.collect_boundary_half_edges(&mut boundary).unwrap();
        assert_eq!(count, 4);

        // Walk the boundary loop starting from he0 (origin=0)
        let loop_edges: Vec<u32> = view.boundary_loop(0).unwrap().collect();
        assert_eq!(loop_edges.len(), 4);
        // Should visit all 4 boundary half-edges
        for &he in &loop_edges {
            assert!(boundary[..count].contains(&he));
        }
    }

    #[test]
    fn vertex_map_is_deterministic_across_runs() {
        // Build the same mesh twice and verify byte-identical maps.
        let triangles = [[0, 1, 2], [2, 1, 3], [0, 2, 4], [0, 4, 5]];

        let (edges1, v2he1, f2he1, _) = build_view(6, &triangles);
        let (edges2, v2he2, f2he2, _) = build_view(6, &triangles);

        assert_eq!(v2he1, v2he2, "vertex→half-edge maps must be byte-identical");
        assert_eq!(f2he1, f2he2, "face→half-edge maps must be byte-identical");
        assert_eq!(edges1, edges2, "half-edge arrays must be byte-identical");
    }

    #[test]
    fn vertex_map_picks_lowest_indexed_half_edge() {
        // Vertex 0 appears in multiple triangles. The map should pick the
        // lowest-indexed outgoing half-edge.
        let triangles = [[0, 1, 2], [0, 2, 3]];
        let (edges, v2he, _f2he, _view) = build_view(4, &triangles);

        // he0 = (0→1) is the lowest-indexed outgoing half-edge for vertex 0.
        assert_eq!(v2he[0], 0);
        // Verify it's actually an outgoing half-edge from vertex 0.
        assert_eq!(edges[v2he[0] as usize].origin, 0);
    }

    #[test]
    fn isolated_vertex_gets_invalid_index() {
        // Vertex 3 is not in any triangle.
        let triangles = [[0, 1, 2]];
        let (_edges, v2he, _f2he, _view) = build_view(4, &triangles);

        assert_eq!(v2he[3], INVALID_INDEX);
    }

    #[test]
    fn fan_mesh_one_ring_center_vertex() {
        // Fan of 4 triangles around vertex 0:
        //   T0: (0,1,2), T1: (0,2,3), T2: (0,3,4), T3: (0,4,1)
        // Vertex 0's one-ring: {1, 2, 3, 4}
        let triangles = [[0, 1, 2], [0, 2, 3], [0, 3, 4], [0, 4, 1]];
        let (_edges, _v2he, _f2he, view) = build_view(5, &triangles);

        let ring: Vec<u32> = view.one_ring(0).unwrap().collect();
        assert_eq!(ring.len(), 4);
        assert!(ring.contains(&1));
        assert!(ring.contains(&2));
        assert!(ring.contains(&3));
        assert!(ring.contains(&4));
    }

    #[test]
    fn closed_mesh_no_boundary() {
        // Tetrahedron (closed surface, no boundary):
        //   T0: (0,1,2), T1: (0,3,1), T2: (0,2,3), T3: (1,3,2)
        // All edges should have twins.
        let triangles = [[0, 1, 2], [0, 3, 1], [0, 2, 3], [1, 3, 2]];
        let (_edges, _v2he, _f2he, view) = build_view(4, &triangles);

        let mut boundary = [0u32; 16];
        let count = view.collect_boundary_half_edges(&mut boundary).unwrap();
        assert_eq!(count, 0, "tetrahedron should have no boundary");
    }

    #[test]
    fn tetrahedron_one_ring_vertex_0() {
        // Tetrahedron: vertex 0's one-ring should be {1, 2, 3}.
        let triangles = [[0, 1, 2], [0, 3, 1], [0, 2, 3], [1, 3, 2]];
        let (_edges, _v2he, _f2he, view) = build_view(4, &triangles);

        let ring: Vec<u32> = view.one_ring(0).unwrap().collect();
        assert_eq!(ring.len(), 3);
        assert!(ring.contains(&1));
        assert!(ring.contains(&2));
        assert!(ring.contains(&3));
    }

    #[test]
    fn face_loop_visits_three_half_edges() {
        let triangles = [[0, 1, 2], [2, 1, 3]];
        let (_edges, _v2he, _f2he, view) = build_view(4, &triangles);

        let loop0: Vec<u32> = view.face_loop(0).unwrap().collect();
        let loop1: Vec<u32> = view.face_loop(1).unwrap().collect();
        assert_eq!(loop0.len(), 3);
        assert_eq!(loop1.len(), 3);
    }

    #[test]
    fn build_maps_rejects_small_buffers() {
        let triangles = [[0, 1, 2]];
        let mut edges = [HalfEdge::default(); 3];
        let mut slots = [EdgeSlot::default(); 8];
        let summary = build_triangle_half_edges(3, &triangles, &mut edges, &mut slots).unwrap();

        let mut v2he = [INVALID_INDEX; 2]; // too small (need 3)
        let mut f2he = [INVALID_INDEX; 1];
        assert_eq!(
            build_surface_mesh_maps(summary, &edges, &mut v2he, &mut f2he),
            Err(SurfaceMeshError::VertexMapTooSmall { required: 3 })
        );

        let mut v2he = [INVALID_INDEX; 3];
        let mut f2he = [INVALID_INDEX; 0]; // too small (need 1)
        assert_eq!(
            build_surface_mesh_maps(summary, &edges, &mut v2he, &mut f2he),
            Err(SurfaceMeshError::FaceMapTooSmall { required: 1 })
        );
    }

    #[test]
    fn boundary_loop_rejects_non_boundary_half_edge() {
        let triangles = [[0, 1, 2], [2, 1, 3]];
        let (_edges, _v2he, _f2he, view) = build_view(4, &triangles);

        // he1 = (1→2) has a twin (he3), so it's not a boundary half-edge.
        let result = view.boundary_loop(1);
        assert!(result.is_err());
    }

    #[test]
    fn single_triangle_boundary_loop() {
        // Single triangle: all 3 edges are boundary.
        let triangles = [[0, 1, 2]];
        let (_edges, _v2he, _f2he, view) = build_view(3, &triangles);

        let loop_edges: Vec<u32> = view.boundary_loop(0).unwrap().collect();
        assert_eq!(loop_edges.len(), 3);
        // Should visit he0, he1, he2
        assert!(loop_edges.contains(&0));
        assert!(loop_edges.contains(&1));
        assert!(loop_edges.contains(&2));
    }

    #[test]
    fn grid_mesh_one_ring() {
        // 2×2 grid of triangles (4 triangles, 4 vertices in a square):
        //   0---1
        //   |\  |
        //   | \ |
        //   |  \|
        //   2---3
        // T0: (0,2,1), T1: (1,2,3)
        // Vertex 2's one-ring: {0, 1, 3}
        let triangles = [[0, 2, 1], [1, 2, 3]];
        let (_edges, _v2he, _f2he, view) = build_view(4, &triangles);

        let ring: Vec<u32> = view.one_ring(2).unwrap().collect();
        assert_eq!(ring.len(), 3);
        assert!(ring.contains(&0));
        assert!(ring.contains(&1));
        assert!(ring.contains(&3));
    }
}
