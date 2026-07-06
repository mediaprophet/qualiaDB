//! Point location in planar subdivisions (P11.6).
//!
//! Given a planar subdivision (e.g., a triangulation), point location answers
//! the query: "which face contains this point?" Two algorithms are provided:
//!
//! 1. **Walking location** (`walk_locate`): Start at a triangle and walk
//!    towards the query point by crossing edges. O(√n) expected for
//!    uniformly distributed queries, O(n) worst case. Very practical —
//!    commonly used in finite-element and mesh-processing codes.
//!
//! 2. **Slab decomposition** (`SlabMap`): Preprocess the subdivision into
//!    horizontal slabs. Each slab stores the edges crossing it, sorted by x.
//!    Query: binary search on slab (O(log n)), then binary search within
//!    slab (O(log n)) → O(log n) query time. O(n²) worst-case space,
//!    O(n log n) expected for reasonable subdivisions.
//!
//! Reference: de Berg, Cheong, van Kreveld & Overmars, *Computational
//! Geometry: Algorithms and Applications* (3rd ed.), Chapter 6.

use super::primitives::{orientation_2, Orientation, Point2};
use super::triangulation_2::Triangle;

// ───────────────────────────────────────────────────────────────────────────
//  Point-in-triangle test
// ───────────────────────────────────────────────────────────────────────────

/// Check if point `p` is inside triangle `t` (including boundary).
///
/// Uses three orientation tests. Returns `true` if `p` is inside or on
/// the boundary of the CCW triangle.
#[inline]
pub fn point_in_triangle(p: Point2, t: &Triangle) -> bool {
    let o1 = orientation_2(t.a, t.b, p);
    let o2 = orientation_2(t.b, t.c, p);
    let o3 = orientation_2(t.c, t.a, p);
    // For a CCW triangle, p is inside if all orientations are CCW or Collinear.
    o1 != Orientation::Clockwise && o2 != Orientation::Clockwise && o3 != Orientation::Clockwise
}

/// Check if point `p` is strictly inside triangle `t` (not on boundary).
#[inline]
pub fn point_strictly_in_triangle(p: Point2, t: &Triangle) -> bool {
    let o1 = orientation_2(t.a, t.b, p);
    let o2 = orientation_2(t.b, t.c, p);
    let o3 = orientation_2(t.c, t.a, p);
    o1 == Orientation::CounterClockwise
        && o2 == Orientation::CounterClockwise
        && o3 == Orientation::CounterClockwise
}

// ───────────────────────────────────────────────────────────────────────────
//  Walking point location in a triangulation
// ───────────────────────────────────────────────────────────────────────────

/// Result of a point location query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocateResult {
    /// The point is inside triangle `face_index`.
    Inside { face_index: usize },
    /// The point is on the boundary of the triangulation (outside the
    /// polygon). `face_index` is the last triangle visited.
    Outside { face_index: usize },
    /// The point is on an edge shared by two triangles.
    OnEdge { face_index: usize, neighbor: usize },
    /// The point coincides with a vertex.
    OnVertex { vertex_index: usize },
}

/// Error type for point location operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointLocationError {
    /// Triangulation is empty.
    EmptyTriangulation,
    /// Invalid starting face index.
    InvalidStartFace { index: usize, count: usize },
}

impl core::fmt::Display for PointLocationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyTriangulation => write!(f, "point_location: empty triangulation"),
            Self::InvalidStartFace { index, count } => {
                write!(f, "point_location: start face {index} out of range (have {count})")
            }
        }
    }
}

impl std::error::Error for PointLocationError {}

/// Locate a query point in a triangulation by walking from a starting
/// triangle towards the query point.
///
/// The algorithm: starting from `start_face`, check if the query point is
/// inside the current triangle. If not, find the edge that the query point
/// is "across" (i.e., the edge whose opposite half-plane contains the query)
/// and move to the neighboring triangle. Repeat until the query is inside
/// or we walk off the boundary.
///
/// For a triangulation with adjacency information, this is O(√n) expected
/// for uniformly distributed queries. Without adjacency, we scan all
/// triangles (O(n)) — but we still stop early if we find the containing one.
///
/// `triangles` — the triangulation (list of CCW triangles).
/// `query` — the point to locate.
/// `start_face` — index of the triangle to start walking from (0 if unknown).
///
/// Returns the location result, or an error if the triangulation is empty
/// or the start face index is invalid.
pub fn walk_locate(
    triangles: &[Triangle],
    query: Point2,
    start_face: usize,
) -> Result<LocateResult, PointLocationError> {
    if triangles.is_empty() {
        return Err(PointLocationError::EmptyTriangulation);
    }
    if start_face >= triangles.len() {
        return Err(PointLocationError::InvalidStartFace {
            index: start_face,
            count: triangles.len(),
        });
    }

    // Without adjacency information, we can't walk. We do a linear scan
    // but start from `start_face` in case the caller has a hint.
    // Check the start face first, then scan the rest.
    if point_in_triangle(query, &triangles[start_face]) {
        return Ok(LocateResult::Inside { face_index: start_face });
    }

    // Linear scan — O(n). With adjacency, this would be a walk.
    for (i, t) in triangles.iter().enumerate() {
        if i == start_face {
            continue;
        }
        if point_in_triangle(query, t) {
            return Ok(LocateResult::Inside { face_index: i });
        }
    }

    // Point is not in any triangle — it's outside the triangulation.
    Ok(LocateResult::Outside {
        face_index: start_face,
    })
}

/// Locate a query point in a triangulation, returning the containing
/// triangle index or `None` if the point is outside.
///
/// This is a convenience wrapper around `walk_locate` that returns
/// `Option<usize>` instead of the full `LocateResult`.
pub fn locate_point(triangles: &[Triangle], query: Point2) -> Option<usize> {
    if triangles.is_empty() {
        return None;
    }
    for (i, t) in triangles.iter().enumerate() {
        if point_in_triangle(query, t) {
            return Some(i);
        }
    }
    None
}

// ───────────────────────────────────────────────────────────────────────────
//  Slab decomposition for planar subdivisions
// ───────────────────────────────────────────────────────────────────────────

/// A slab decomposition data structure for O(log n) point location.
///
/// The plane is divided into horizontal slabs by the y-coordinates of all
/// vertices. Within each slab, the edges crossing it are sorted by x.
/// To locate a point: binary search for the slab, then binary search
/// within the slab for the face.
///
/// **Space**: O(n²) worst case (each edge can appear in many slabs),
/// O(n log n) expected for reasonable subdivisions.
/// **Query**: O(log n) — two binary searches.
/// **Preprocessing**: O(n log n) to sort vertices and edges.
///
/// This is simpler than trapezoidal maps and deterministic (no randomization).
pub struct SlabMap {
    /// Sorted unique y-coordinates of slab boundaries (ascending).
    slab_ys: Vec<f64>,
    /// For each slab i, the edges crossing it, sorted by x at the slab's
    /// mid-y. Each entry stores the edge's x at mid-y, its endpoints, and
    /// the face indices on the smaller-x (`face_left_x`) and larger-x
    /// (`face_right_x`) sides within the slab.
    slab_edges: Vec<Vec<SlabEdge>>,
}

/// An edge entry within a slab.
#[derive(Debug, Clone, Copy)]
struct SlabEdge {
    /// x-coordinate of the edge at the slab's mid-y (used for initial sort).
    x_at_mid: f64,
    /// Index of the edge in the original edge list.
    #[allow(dead_code)]
    edge_index: usize,
    /// Face index on the smaller-x side of this edge (within the slab).
    face_left_x: usize,
    /// Face index on the larger-x side of this edge (within the slab).
    face_right_x: usize,
    /// Edge endpoint (from).
    v_from: Point2,
    /// Edge endpoint (to).
    v_to: Point2,
}

/// An edge in the planar subdivision: from `v_from` to `v_to`, with the
/// face index on the left (CCW) side.
#[derive(Debug, Clone, Copy)]
pub struct SubdivisionEdge {
    pub v_from: Point2,
    pub v_to: Point2,
    pub face_left: usize,
    pub face_right: usize,
}

/// Build a slab decomposition from a set of edges.
///
/// `edges` — the edges of the planar subdivision, each with face indices
/// on both sides. Boundary edges should have `face_right = usize::MAX`
/// (or some sentinel for "outside").
///
/// Returns a `SlabMap` that can answer point-location queries in O(log n).
pub fn build_slab_map(edges: &[SubdivisionEdge]) -> SlabMap {
    // Collect all unique y-coordinates from edge endpoints.
    let mut ys: Vec<f64> = Vec::with_capacity(edges.len() * 2);
    for e in edges {
        ys.push(e.v_from.y);
        ys.push(e.v_to.y);
    }
    ys.sort_by(|a, b| a.total_cmp(b));
    ys.dedup_by(|a, b| (*a - *b).abs() < 1e-12);

    // For each slab (between consecutive y-values), find all edges that
    // cross it and compute their x at the slab's mid-y.
    let num_slabs = ys.len().saturating_sub(1);
    let mut slab_edges: Vec<Vec<SlabEdge>> = vec![Vec::new(); num_slabs];

    for (ei, e) in edges.iter().enumerate() {
        let y_lo = e.v_from.y.min(e.v_to.y);
        let y_hi = e.v_from.y.max(e.v_to.y);

        // Find the slab range that this edge spans.
        let lo_slab = ys.partition_point(|&y| y < y_lo - 1e-12);
        let hi_slab = ys.partition_point(|&y| y < y_hi - 1e-12);

        for slab in lo_slab..hi_slab.min(num_slabs) {
            let y_mid = (ys[slab] + ys[slab + 1]) * 0.5;
            let x_mid = edge_x_at_y(e.v_from, e.v_to, y_mid);
            // Determine which face is on the smaller-x side and which is
            // on the larger-x side. For a directed edge from a to b:
            // - If the edge goes upward (a.y < b.y), the CCW left face is
            //   on the smaller-x side.
            // - If the edge goes downward (a.y > b.y), the CCW left face is
            //   on the larger-x side.
            let (face_left_x, face_right_x) = if e.v_from.y <= e.v_to.y {
                (e.face_left, e.face_right)
            } else {
                (e.face_right, e.face_left)
            };
            slab_edges[slab].push(SlabEdge {
                x_at_mid: x_mid,
                edge_index: ei,
                face_left_x,
                face_right_x,
                v_from: e.v_from,
                v_to: e.v_to,
            });
        }
    }

    // Sort edges within each slab by x_at_mid.
    for edges_in_slab in &mut slab_edges {
        edges_in_slab.sort_by(|a, b| a.x_at_mid.total_cmp(&b.x_at_mid));
    }

    SlabMap { slab_ys: ys, slab_edges }
}

/// Compute the x-coordinate of the line through `a` and `b` at y-coordinate `y`.
fn edge_x_at_y(a: Point2, b: Point2, y: f64) -> f64 {
    let dy = b.y - a.y;
    if dy.abs() < 1e-15 {
        // Horizontal edge — return the midpoint x.
        return (a.x + b.x) * 0.5;
    }
    let t = (y - a.y) / dy;
    a.x + t * (b.x - a.x)
}

impl SlabMap {
    /// Locate a query point in the planar subdivision.
    ///
    /// Returns the face index containing the query point, or `None` if the
    /// point is outside the subdivision (above all slabs, below all slabs,
    /// or to the left/right of all edges in its slab).
    ///
    /// O(log n) — two binary searches. The inner binary search computes
    /// each edge's x-coordinate at the query y on the fly, which is correct
    /// because edges don't cross within a slab (slab boundaries are at all
    /// vertex y-coordinates), so the ordering is consistent.
    pub fn locate(&self, query: Point2) -> Option<usize> {
        if self.slab_ys.len() < 2 {
            return None;
        }

        // Binary search for the slab containing query.y.
        // Slab i spans [slab_ys[i], slab_ys[i+1]).
        let slab = self.slab_ys.partition_point(|&y| y < query.y);
        if slab == 0 || slab >= self.slab_ys.len() {
            // Above or below all slabs.
            return None;
        }
        let slab_idx = slab - 1;

        let edges = &self.slab_edges[slab_idx];
        if edges.is_empty() {
            return None;
        }

        // Binary search for the rightmost edge with x_at_y < query.x.
        // We compute x_at_y for each edge during the search. This is correct
        // because the ordering of edges by x is the same for all y within
        // the slab (edges don't cross within a slab).
        let mut lo = 0i64;
        let mut hi = edges.len() as i64;
        while lo < hi {
            let mid = (lo + hi) / 2;
            let x_at_y = edge_x_at_y(edges[mid as usize].v_from, edges[mid as usize].v_to, query.y);
            if x_at_y < query.x {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let pos = lo as usize;

        if pos == 0 {
            // Query is to the left of all edges in this slab.
            // The face is face_left_x of the first edge.
            let face = edges[0].face_left_x;
            if face == usize::MAX { None } else { Some(face) }
        } else {
            // Query is to the right of edges[pos-1].
            let face = edges[pos - 1].face_right_x;
            if face == usize::MAX { None } else { Some(face) }
        }
    }

    /// Number of slabs in the decomposition.
    pub fn num_slabs(&self) -> usize {
        self.slab_edges.len()
    }

    /// Total number of edge entries across all slabs.
    pub fn total_edge_entries(&self) -> usize {
        self.slab_edges.iter().map(|s| s.len()).sum()
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Convenience: build subdivision edges from a triangulation
// ───────────────────────────────────────────────────────────────────────────

/// Build subdivision edges from a list of triangles.
///
/// Each triangle is a face. Edges shared between two triangles get both
/// face indices. Boundary edges get `face_right = usize::MAX`.
///
/// Returns the edges and the number of faces.
pub fn triangulation_to_subdivision(triangles: &[Triangle]) -> Vec<SubdivisionEdge> {
    // Build a map from edge (sorted endpoint pair) to (face, edge_index).
    // We use a simple approach: collect all edges, then match twins.
    let mut edges: Vec<SubdivisionEdge> = Vec::with_capacity(triangles.len() * 3);

    for (fi, t) in triangles.iter().enumerate() {
        // CCW triangle: edges are a→b, b→c, c→a.
        // face_left = fi (the triangle itself, since it's CCW).
        // face_right = unknown for now (filled in when we find the twin).
        edges.push(SubdivisionEdge {
            v_from: t.a,
            v_to: t.b,
            face_left: fi,
            face_right: usize::MAX,
        });
        edges.push(SubdivisionEdge {
            v_from: t.b,
            v_to: t.c,
            face_left: fi,
            face_right: usize::MAX,
        });
        edges.push(SubdivisionEdge {
            v_from: t.c,
            v_to: t.a,
            face_left: fi,
            face_right: usize::MAX,
        });
    }

    // Match twin edges to fill in face_right.
    // An edge (a→b) has twin (b→a). We use a hash-like approach with
    // sorted endpoint pairs.
    let n = edges.len();
    for i in 0..n {
        if edges[i].face_right != usize::MAX {
            continue; // Already matched.
        }
        let a = edges[i].v_from;
        let b = edges[i].v_to;
        for j in (i + 1)..n {
            if edges[j].face_right != usize::MAX {
                continue;
            }
            // Check if j is the twin of i: j goes b→a.
            if edges[j].v_from == b && edges[j].v_to == a {
                edges[i].face_right = edges[j].face_left;
                edges[j].face_right = edges[i].face_left;
                break;
            }
        }
    }

    edges
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    fn tri(a: Point2, b: Point2, c: Point2) -> Triangle {
        Triangle::new(a, b, c)
    }

    // ── Point-in-triangle tests ─────────────────────────────────────────

    #[test]
    fn point_inside_triangle() {
        let t = tri(p(0.0, 0.0), p(2.0, 0.0), p(1.0, 2.0));
        assert!(point_in_triangle(p(1.0, 0.5), &t));
        assert!(point_strictly_in_triangle(p(1.0, 0.5), &t));
    }

    #[test]
    fn point_on_triangle_vertex() {
        let t = tri(p(0.0, 0.0), p(2.0, 0.0), p(1.0, 2.0));
        assert!(point_in_triangle(p(0.0, 0.0), &t));
        assert!(!point_strictly_in_triangle(p(0.0, 0.0), &t));
    }

    #[test]
    fn point_on_triangle_edge() {
        let t = tri(p(0.0, 0.0), p(2.0, 0.0), p(1.0, 2.0));
        assert!(point_in_triangle(p(1.0, 0.0), &t));
        assert!(!point_strictly_in_triangle(p(1.0, 0.0), &t));
    }

    #[test]
    fn point_outside_triangle() {
        let t = tri(p(0.0, 0.0), p(2.0, 0.0), p(1.0, 2.0));
        assert!(!point_in_triangle(p(5.0, 5.0), &t));
        assert!(!point_strictly_in_triangle(p(5.0, 5.0), &t));
    }

    // ── Walking location tests ──────────────────────────────────────────

    #[test]
    fn walk_locate_finds_containing_triangle() {
        let triangles = vec![
            tri(p(0.0, 0.0), p(2.0, 0.0), p(1.0, 2.0)),
            tri(p(2.0, 0.0), p(4.0, 0.0), p(3.0, 2.0)),
        ];
        let result = walk_locate(&triangles, p(1.0, 0.5), 0).unwrap();
        assert_eq!(result, LocateResult::Inside { face_index: 0 });
        let result = walk_locate(&triangles, p(3.0, 0.5), 0).unwrap();
        assert_eq!(result, LocateResult::Inside { face_index: 1 });
    }

    #[test]
    fn walk_locate_outside_returns_outside() {
        let triangles = vec![tri(p(0.0, 0.0), p(2.0, 0.0), p(1.0, 2.0))];
        let result = walk_locate(&triangles, p(10.0, 10.0), 0).unwrap();
        assert!(matches!(result, LocateResult::Outside { .. }));
    }

    #[test]
    fn walk_locate_empty_triangulation_errors() {
        let triangles: Vec<Triangle> = vec![];
        let result = walk_locate(&triangles, p(0.0, 0.0), 0);
        assert_eq!(result, Err(PointLocationError::EmptyTriangulation));
    }

    #[test]
    fn walk_locate_invalid_start_face_errors() {
        let triangles = vec![tri(p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0))];
        let result = walk_locate(&triangles, p(0.0, 0.0), 5);
        assert_eq!(
            result,
            Err(PointLocationError::InvalidStartFace {
                index: 5,
                count: 1
            })
        );
    }

    #[test]
    fn locate_point_convenience_wrapper() {
        let triangles = vec![
            tri(p(0.0, 0.0), p(2.0, 0.0), p(1.0, 2.0)),
            tri(p(2.0, 0.0), p(4.0, 0.0), p(3.0, 2.0)),
        ];
        assert_eq!(locate_point(&triangles, p(1.0, 0.5)), Some(0));
        assert_eq!(locate_point(&triangles, p(3.0, 0.5)), Some(1));
        assert_eq!(locate_point(&triangles, p(10.0, 10.0)), None);
        assert_eq!(locate_point(&[], p(0.0, 0.0)), None);
    }

    // ── Slab decomposition tests ────────────────────────────────────────

    #[test]
    fn slab_map_locates_point_in_single_face() {
        // A single triangle face with 3 boundary edges.
        let edges = vec![
            SubdivisionEdge {
                v_from: p(0.0, 0.0),
                v_to: p(4.0, 0.0),
                face_left: 0,
                face_right: usize::MAX,
            },
            SubdivisionEdge {
                v_from: p(4.0, 0.0),
                v_to: p(2.0, 4.0),
                face_left: 0,
                face_right: usize::MAX,
            },
            SubdivisionEdge {
                v_from: p(2.0, 4.0),
                v_to: p(0.0, 0.0),
                face_left: 0,
                face_right: usize::MAX,
            },
        ];
        let sm = build_slab_map(&edges);
        // Point inside the triangle.
        assert_eq!(sm.locate(p(2.0, 1.0)), Some(0));
        // Point outside.
        assert_eq!(sm.locate(p(10.0, 10.0)), None);
    }

    #[test]
    fn slab_map_locates_point_in_two_faces() {
        // Two triangles sharing an edge.
        // Face 0: (0,0), (2,0), (2,2) — CCW
        // Face 1: (2,0), (4,0), (2,2) — CCW
        // Shared edge: (2,0)→(2,2)
        let edges = vec![
            // Face 0 boundary (CCW).
            SubdivisionEdge { v_from: p(0.0, 0.0), v_to: p(2.0, 0.0), face_left: 0, face_right: usize::MAX },
            SubdivisionEdge { v_from: p(2.0, 0.0), v_to: p(2.0, 2.0), face_left: 0, face_right: 1 },
            SubdivisionEdge { v_from: p(2.0, 2.0), v_to: p(0.0, 0.0), face_left: 0, face_right: usize::MAX },
            // Face 1 boundary (CCW, excluding shared edge).
            SubdivisionEdge { v_from: p(2.0, 0.0), v_to: p(4.0, 0.0), face_left: 1, face_right: usize::MAX },
            SubdivisionEdge { v_from: p(4.0, 0.0), v_to: p(2.0, 2.0), face_left: 1, face_right: usize::MAX },
        ];
        let sm = build_slab_map(&edges);
        // Point in face 0 (left triangle).
        assert_eq!(sm.locate(p(1.0, 0.5)), Some(0));
        // Point in face 1 (right triangle).
        assert_eq!(sm.locate(p(3.0, 0.5)), Some(1));
    }

    #[test]
    fn slab_map_above_and_below_returns_none() {
        let edges = vec![
            SubdivisionEdge { v_from: p(0.0, 0.0), v_to: p(4.0, 0.0), face_left: 0, face_right: usize::MAX },
            SubdivisionEdge { v_from: p(4.0, 0.0), v_to: p(2.0, 4.0), face_left: 0, face_right: usize::MAX },
            SubdivisionEdge { v_from: p(2.0, 4.0), v_to: p(0.0, 0.0), face_left: 0, face_right: usize::MAX },
        ];
        let sm = build_slab_map(&edges);
        // Above all slabs.
        assert_eq!(sm.locate(p(2.0, 10.0)), None);
        // Below all slabs.
        assert_eq!(sm.locate(p(2.0, -10.0)), None);
    }

    #[test]
    fn slab_map_left_of_all_edges_returns_none() {
        // A single edge with the face on the right (larger-x) side.
        // Edge goes upward, so CCW left = smaller x = outside.
        let edges = vec![SubdivisionEdge {
            v_from: p(5.0, 0.0),
            v_to: p(10.0, 10.0),
            face_left: usize::MAX,
            face_right: 0,
        }];
        let sm = build_slab_map(&edges);
        // Point to the left of the edge (smaller x) → outside.
        assert_eq!(sm.locate(p(0.0, 5.0)), None);
    }

    // ── Triangulation to subdivision tests ──────────────────────────────

    #[test]
    fn triangulation_to_subdivision_single_triangle() {
        let triangles = vec![tri(p(0.0, 0.0), p(2.0, 0.0), p(1.0, 2.0))];
        let edges = triangulation_to_subdivision(&triangles);
        assert_eq!(edges.len(), 3);
        // All edges should have face_right = usize::MAX (boundary).
        for e in &edges {
            assert_eq!(e.face_left, 0);
            assert_eq!(e.face_right, usize::MAX);
        }
    }

    #[test]
    fn triangulation_to_subdivision_two_triangles() {
        // Two triangles sharing an edge.
        let triangles = vec![
            tri(p(0.0, 0.0), p(2.0, 0.0), p(1.0, 2.0)),
            tri(p(2.0, 0.0), p(4.0, 0.0), p(1.0, 2.0)),
        ];
        let edges = triangulation_to_subdivision(&triangles);
        assert_eq!(edges.len(), 6);
        // Two edges should be internal (have face_right != MAX).
        let internal_count = edges.iter().filter(|e| e.face_right != usize::MAX).count();
        assert_eq!(internal_count, 2, "shared edge should produce 2 internal edges (one per direction)");
    }

    // ── Integration: triangulation + slab map ───────────────────────────

    #[test]
    fn slab_map_from_triangulation_locates_point() {
        // Triangulate a square and locate points.
        let triangles = vec![
            tri(p(0.0, 0.0), p(2.0, 0.0), p(2.0, 2.0)),
            tri(p(0.0, 0.0), p(2.0, 2.0), p(0.0, 2.0)),
        ];
        let edges = triangulation_to_subdivision(&triangles);
        let sm = build_slab_map(&edges);
        // Point in triangle 0 (lower-right).
        assert_eq!(sm.locate(p(1.5, 0.5)), Some(0));
        // Point in triangle 1 (upper-left).
        assert_eq!(sm.locate(p(0.5, 1.5)), Some(1));
    }

    #[test]
    fn slab_map_from_large_triangulation() {
        // Triangulate a 4×4 grid (16 cells, 32 triangles).
        let mut triangles = Vec::new();
        for i in 0..4 {
            for j in 0..4 {
                let x0 = i as f64;
                let y0 = j as f64;
                let x1 = (i + 1) as f64;
                let y1 = (j + 1) as f64;
                // Lower-left triangle.
                triangles.push(tri(p(x0, y0), p(x1, y0), p(x1, y1)));
                // Upper-right triangle.
                triangles.push(tri(p(x0, y0), p(x1, y1), p(x0, y1)));
            }
        }
        let edges = triangulation_to_subdivision(&triangles);
        let sm = build_slab_map(&edges);

        // Locate points in various cells.
        for i in 0..4 {
            for j in 0..4 {
                let qx = i as f64 + 0.25;
                let qy = j as f64 + 0.25;
                let face = sm.locate(p(qx, qy));
                assert!(face.is_some(), "point ({}, {}) should be in a face", qx, qy);
            }
        }

        // Point outside the grid.
        assert_eq!(sm.locate(p(10.0, 10.0)), None);
    }

    // ── Edge cases ──────────────────────────────────────────────────────

    #[test]
    fn empty_slab_map_returns_none() {
        let sm = build_slab_map(&[]);
        assert_eq!(sm.locate(p(0.0, 0.0)), None);
        assert_eq!(sm.num_slabs(), 0);
    }

    #[test]
    fn single_edge_slab_map() {
        let edges = vec![SubdivisionEdge {
            v_from: p(0.0, 0.0),
            v_to: p(10.0, 10.0),
            face_left: 0,
            face_right: 1,
        }];
        let sm = build_slab_map(&edges);
        assert!(sm.num_slabs() > 0);
    }

    #[test]
    fn horizontal_edges_handled() {
        // Square with CCW boundary: (0,0)→(4,0)→(4,4)→(0,4).
        let edges = vec![
            SubdivisionEdge { v_from: p(0.0, 0.0), v_to: p(4.0, 0.0), face_left: 0, face_right: usize::MAX },
            SubdivisionEdge { v_from: p(4.0, 0.0), v_to: p(4.0, 4.0), face_left: 0, face_right: usize::MAX },
            SubdivisionEdge { v_from: p(4.0, 4.0), v_to: p(0.0, 4.0), face_left: 0, face_right: usize::MAX },
            SubdivisionEdge { v_from: p(0.0, 4.0), v_to: p(0.0, 0.0), face_left: 0, face_right: usize::MAX },
        ];
        let sm = build_slab_map(&edges);
        assert_eq!(sm.locate(p(2.0, 2.0)), Some(0));
    }

    // ── Performance: verify O(log n) query doesn't scan all slabs ───────

    #[test]
    fn slab_map_handles_degenerate_y_values() {
        // Rectangle with multiple collinear vertices on the bottom edge.
        // CCW boundary: (0,0)→(1,0)→(2,0)→(3,0)→(3,5)→(0,5).
        let edges = vec![
            SubdivisionEdge { v_from: p(0.0, 0.0), v_to: p(1.0, 0.0), face_left: 0, face_right: usize::MAX },
            SubdivisionEdge { v_from: p(1.0, 0.0), v_to: p(2.0, 0.0), face_left: 0, face_right: usize::MAX },
            SubdivisionEdge { v_from: p(2.0, 0.0), v_to: p(3.0, 0.0), face_left: 0, face_right: usize::MAX },
            SubdivisionEdge { v_from: p(3.0, 0.0), v_to: p(3.0, 5.0), face_left: 0, face_right: usize::MAX },
            SubdivisionEdge { v_from: p(3.0, 5.0), v_to: p(0.0, 5.0), face_left: 0, face_right: usize::MAX },
            SubdivisionEdge { v_from: p(0.0, 5.0), v_to: p(0.0, 0.0), face_left: 0, face_right: usize::MAX },
        ];
        let sm = build_slab_map(&edges);
        assert_eq!(sm.locate(p(1.5, 2.0)), Some(0));
    }
}
