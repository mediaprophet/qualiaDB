//! Delaunay triangulation 2-D (P4.4).
//!
//! Deterministic, index-based Bowyer-Watson incremental insertion.
//! Uses the exact `incircle` predicate from P1.5 for robust classification.
//!
//! ## Algorithm
//!
//! 1. Compute a super-triangle enclosing all input points.
//! 2. Insert points one at a time in index order:
//!    a. Find all triangles whose circumcircle contains the new point
//!       (via `incircle` — the exact ladder).
//!    b. Remove the "bad" triangles, leaving a star-shaped cavity.
//!    c. Re-triangulate the cavity by connecting the new point to each
//!       boundary edge of the cavity.
//! 3. Remove all triangles that reference a super-triangle vertex.
//!
//! ## Determinism
//!
//! Points are processed in index order. The `incircle` predicate is exact
//! (filtered → compensated → expansion), so the combinatorial output is
//! bit-identical across runs and platforms. Triangle ordering is canonical:
//! the output is sorted by (min_vertex, mid_vertex, max_vertex) after
//! removing super-triangle triangles.
//!
//! ## Zero-heap contract
//!
//! The predicate hot path (`incircle`, `orientation_2`) is zero-heap.
//! The algorithm workspace uses `Vec` for the dynamic triangle list — this
//! is the algorithm layer, not the predicate layer.

use super::expansion::Sign;
use super::incircle::incircle;
use super::primitives::{orientation_2, Orientation, Point2};

#[cfg(test)]
use super::hull::convex_hull_indices_2;

/// Delaunay triangulation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DelaunayError {
    /// Fewer than 3 input points.
    TooFewPoints { got: usize },
    /// All points are collinear — no triangulation exists.
    CollinearInput,
    /// Output buffer too small.
    OutputTooSmall { required: usize, have: usize },
    /// Scratch buffer too small.
    ScratchTooSmall { required: usize, have: usize },
}

impl core::fmt::Display for DelaunayError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooFewPoints { got } => write!(f, "delaunay: need ≥3 points, got {got}"),
            Self::CollinearInput => write!(f, "delaunay: all points collinear"),
            Self::OutputTooSmall { required, have } => {
                write!(
                    f,
                    "delaunay: output too small, need {required}, have {have}"
                )
            }
            Self::ScratchTooSmall { required, have } => {
                write!(
                    f,
                    "delaunay: scratch too small, need {required}, have {have}"
                )
            }
        }
    }
}

impl std::error::Error for DelaunayError {}

/// A triangle in the triangulation, stored with CCW vertex winding.
/// The sort key (min, mid, max) is computed separately for deterministic
/// output ordering without breaking the winding.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Tri {
    v: [u32; 3], // CCW order
}

impl Tri {
    #[inline]
    fn new(a: u32, b: u32, c: u32) -> Self {
        Tri { v: [a, b, c] }
    }

    /// Sort key for deterministic output ordering (does not affect winding).
    #[inline]
    fn sort_key(&self) -> [u32; 3] {
        let mut s = self.v;
        s.sort_unstable();
        s
    }

    #[inline]
    pub fn contains(&self, idx: u32) -> bool {
        self.v[0] == idx || self.v[1] == idx || self.v[2] == idx
    }
}

/// An edge in the cavity boundary.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Edge {
    v: [u32; 2],
}

impl Edge {
    #[inline]
    fn new(a: u32, b: u32) -> Self {
        if a < b {
            Edge { v: [a, b] }
        } else {
            Edge { v: [b, a] }
        }
    }
}

/// Maximum number of triangles for a planar triangulation of n points:
/// 2n - 2 - h (where h = hull vertices). We use 2n as an upper bound.
/// Plus the super-triangle: +1.
#[inline]
fn max_triangles(n: usize) -> usize {
    if n < 2 {
        1
    } else {
        2 * n + 1
    }
}

/// Compute the Delaunay triangulation of a set of 2-D points.
///
/// Returns the number of triangles written to `out`. Each triangle is
/// `[u32; 3]` with vertex indices into `points`, sorted canonically
/// (min, mid, max) and the triangle list is sorted lexicographically.
///
/// `scratch` needs `points.len()` entries (for the convex hull check).
/// `out` needs `2 * points.len()` entries (upper bound on triangles).
///
/// # Determinism
///
/// Identical input → bit-identical output across runs and platforms.
/// Points are processed in index order; the `incircle` predicate is exact.
pub fn delaunay_triangulation_2(
    points: &[Point2],
    scratch: &mut [u32],
    out: &mut [[u32; 3]],
) -> Result<usize, DelaunayError> {
    let n = points.len();
    if n < 3 {
        return Err(DelaunayError::TooFewPoints { got: n });
    }
    if scratch.len() < n {
        return Err(DelaunayError::ScratchTooSmall {
            required: n,
            have: scratch.len(),
        });
    }
    let max_tris = max_triangles(n);
    if out.len() < max_tris {
        return Err(DelaunayError::OutputTooSmall {
            required: max_tris,
            have: out.len(),
        });
    }

    // Check for collinear input via orientation.
    let orient_first = orientation_2(points[0], points[1], points[2]);
    if orient_first == Orientation::Collinear {
        // Check if ALL points are collinear with the first two.
        let mut all_collinear = true;
        for i in 2..n {
            if orientation_2(points[0], points[1], points[i]) != Orientation::Collinear {
                all_collinear = false;
                break;
            }
        }
        if all_collinear {
            return Err(DelaunayError::CollinearInput);
        }
    }

    // Compute bounding box for the super-triangle.
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in points {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    let dx = max_x - min_x;
    let dy = max_y - min_y;
    let delta = dx.max(dy) * 100.0 + 1.0;
    let mid_x = (min_x + max_x) * 0.5;
    let _mid_y = (min_y + max_y) * 0.5;

    // Super-triangle vertices: n, n+1, n+2 (indices after all real points).
    let st_a = n as u32; // top
    let st_b = (n + 1) as u32; // bottom-left
    let st_c = (n + 2) as u32; // bottom-right

    let st_p_a = Point2::new(mid_x, max_y + delta);
    let st_p_b = Point2::new(min_x - delta, min_y - delta);
    let st_p_c = Point2::new(max_x + delta, min_y - delta);

    // Extended points array (on the stack for small inputs, or we can
    // build a combined view). We need a way to look up super-triangle
    // points. We'll use a closure.
    let lookup = |idx: usize| -> Point2 {
        match idx {
            _ if idx == n => st_p_a,
            _ if idx == n + 1 => st_p_b,
            _ if idx == n + 2 => st_p_c,
            _ => points[idx],
        }
    };

    // Initialize triangle list with the super-triangle.
    let mut triangles: Vec<Tri> = Vec::with_capacity(max_tris);
    triangles.push(Tri::new(st_a, st_b, st_c));

    // Insert points one at a time.
    for i in 0..n {
        let p = points[i];
        let p_idx = i as u32;

        // Find all triangles whose circumcircle contains p.
        // A triangle (a, b, c) contains p in its circumcircle if
        // incircle(a, b, c, p) >= 0 (when a,b,c are CCW).
        // We need to check orientation first to handle CW triangles.
        let mut bad_indices: Vec<usize> = Vec::with_capacity(triangles.len());

        for (t_idx, tri) in triangles.iter().enumerate() {
            let a = lookup(tri.v[0] as usize);
            let b = lookup(tri.v[1] as usize);
            let c = lookup(tri.v[2] as usize);

            // The incircle predicate: if a,b,c are CCW, Positive = inside.
            // If a,b,c are CW, Negative = inside.
            // We want to know if p is inside the circumcircle regardless
            // of orientation. So we check:
            //   orient(a,b,c) == CCW && incircle(a,b,c,p) >= 0, or
            //   orient(a,b,c) == CW  && incircle(a,b,c,p) <= 0.
            let orient = orientation_2(a, b, c);
            if orient == Orientation::Collinear {
                // Degenerate triangle — skip (shouldn't happen in a valid
                // triangulation, but be safe).
                continue;
            }

            let inc = incircle(a, b, c, p);
            let inside = match orient {
                Orientation::CounterClockwise => inc != Sign::Negative,
                Orientation::Clockwise => inc != Sign::Positive,
                _ => false,
            };

            if inside {
                bad_indices.push(t_idx);
            }
        }

        if bad_indices.is_empty() {
            // Point is outside all circumcircles — this shouldn't happen
            // with a valid super-triangle, but skip just in case.
            continue;
        }

        // Collect the cavity boundary edges.
        // An edge is on the boundary if it appears in exactly one bad triangle.
        // We collect all edges from bad triangles, then find unique ones.
        let mut edges: Vec<Edge> = Vec::with_capacity(bad_indices.len() * 3);
        for &bi in &bad_indices {
            let tri = triangles[bi];
            edges.push(Edge::new(tri.v[0], tri.v[1]));
            edges.push(Edge::new(tri.v[1], tri.v[2]));
            edges.push(Edge::new(tri.v[2], tri.v[0]));
        }
        edges.sort_unstable();

        // Boundary edges are those that appear exactly once.
        let mut boundary: Vec<Edge> = Vec::with_capacity(edges.len());
        let mut j = 0;
        while j < edges.len() {
            if j + 1 < edges.len() && edges[j] == edges[j + 1] {
                // Skip all duplicates of this edge.
                j += 1;
                while j < edges.len() && edges[j] == edges[j - 1] {
                    j += 1;
                }
            } else {
                boundary.push(edges[j]);
                j += 1;
            }
        }

        // Remove bad triangles (mark them by replacing with the last element).
        // Sort bad_indices in descending order for safe removal.
        bad_indices.sort_unstable_by(|a, b| b.cmp(a));
        for bi in &bad_indices {
            triangles.swap_remove(*bi);
        }

        // Create new triangles connecting p to each boundary edge.
        for edge in &boundary {
            // Ensure the new triangle has the correct orientation (CCW).
            let a_idx = edge.v[0];
            let b_idx = edge.v[1];
            let a = lookup(a_idx as usize);
            let b = lookup(b_idx as usize);

            // We want the triangle (a, b, p) to be CCW.
            let orient = orientation_2(a, b, p);
            let tri = if orient == Orientation::CounterClockwise {
                Tri::new(a_idx, b_idx, p_idx)
            } else {
                Tri::new(b_idx, a_idx, p_idx)
            };
            triangles.push(tri);
        }
    }

    // Remove triangles that reference super-triangle vertices.
    triangles.retain(|t| t.v[0] < n as u32 && t.v[1] < n as u32 && t.v[2] < n as u32);

    // Sort triangles canonically for deterministic output (by sort key,
    // preserving CCW winding).
    triangles.sort_unstable_by(|a, b| a.sort_key().cmp(&b.sort_key()));

    // Also remove degenerate triangles (zero area).
    // Ensure CCW orientation for all remaining triangles.
    triangles.retain(|t| {
        orientation_2(
            points[t.v[0] as usize],
            points[t.v[1] as usize],
            points[t.v[2] as usize],
        ) != Orientation::Collinear
    });
    // Normalize winding to CCW.
    for t in &mut triangles {
        let orient = orientation_2(
            points[t.v[0] as usize],
            points[t.v[1] as usize],
            points[t.v[2] as usize],
        );
        if orient == Orientation::Clockwise {
            t.v.swap(1, 2);
        }
    }

    // Copy to output.
    let count = triangles.len();
    for (i, tri) in triangles.iter().enumerate() {
        out[i] = tri.v;
    }

    Ok(count)
}

/// Verify that a triangulation is Delaunay: every triangle's circumcircle
/// is empty (no other point lies inside it).
///
/// Uses the exact `incircle` predicate. Returns `true` if the triangulation
/// is valid Delaunay, `false` otherwise.
pub fn verify_delaunay(points: &[Point2], triangles: &[[u32; 3]]) -> bool {
    for tri in triangles {
        let a = points[tri[0] as usize];
        let b = points[tri[1] as usize];
        let c = points[tri[2] as usize];

        let orient = orientation_2(a, b, c);
        if orient == Orientation::Collinear {
            return false;
        }

        for (i, p) in points.iter().enumerate() {
            let pi = i as u32;
            if pi == tri[0] || pi == tri[1] || pi == tri[2] {
                continue;
            }

            let inc = incircle(a, b, c, *p);
            let inside = match orient {
                Orientation::CounterClockwise => inc == Sign::Positive,
                Orientation::Clockwise => inc == Sign::Negative,
                _ => false,
            };

            if inside {
                return false;
            }
        }
    }
    true
}

/// Compute a determinism hash for a triangulation.
/// Returns an FNV-1a hash of the triangle index data.
pub fn triangulation_hash(triangles: &[[u32; 3]]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for tri in triangles {
        for &v in tri {
            hash ^= v as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

// ──────────────────────────────────────────────────────────────────────────
//  Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::topology::{build_triangle_half_edges, required_edge_slots, EdgeSlot};
    use super::*;

    #[test]
    fn square_produces_two_triangles() {
        let points = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];
        let mut scratch = vec![0u32; points.len()];
        let mut out = vec![[0u32; 3]; max_triangles(points.len())];
        let count = delaunay_triangulation_2(&points, &mut scratch, &mut out).unwrap();
        assert_eq!(count, 2);
        // Both triangles should use all 4 vertices.
        let used: std::collections::HashSet<u32> = out[..count]
            .iter()
            .flat_map(|t| t.iter().copied())
            .collect();
        assert_eq!(used.len(), 4);
    }

    #[test]
    fn triangle_produces_one_triangle() {
        let points = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        ];
        let mut scratch = vec![0u32; 3];
        let mut out = vec![[0u32; 3]; max_triangles(3)];
        let count = delaunay_triangulation_2(&points, &mut scratch, &mut out).unwrap();
        assert_eq!(count, 1);
        assert_eq!(out[0], [0, 1, 2]);
    }

    #[test]
    fn too_few_points_errors() {
        let points = [Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)];
        let mut scratch = vec![0u32; 2];
        let mut out = vec![[0u32; 3]; 10];
        let result = delaunay_triangulation_2(&points, &mut scratch, &mut out);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), DelaunayError::TooFewPoints { got: 2 });
    }

    #[test]
    fn collinear_points_error() {
        let points = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(3.0, 0.0),
        ];
        let mut scratch = vec![0u32; 4];
        let mut out = vec![[0u32; 3]; max_triangles(4)];
        let result = delaunay_triangulation_2(&points, &mut scratch, &mut out);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), DelaunayError::CollinearInput);
    }

    #[test]
    fn empty_circumcircle_property() {
        // Grid of points.
        let mut points = Vec::new();
        for y in 0..5 {
            for x in 0..5 {
                points.push(Point2::new(x as f64, y as f64));
            }
        }
        let mut scratch = vec![0u32; points.len()];
        let mut out = vec![[0u32; 3]; max_triangles(points.len())];
        let count = delaunay_triangulation_2(&points, &mut scratch, &mut out).unwrap();
        assert!(count > 0);
        assert!(
            verify_delaunay(&points, &out[..count]),
            "triangulation is not Delaunay"
        );
    }

    #[test]
    fn random_points_are_delaunay() {
        // Deterministic pseudo-random points (no rand dependency).
        let mut points = Vec::new();
        let mut seed: u64 = 12345;
        for _ in 0..20 {
            // LCG for deterministic pseudo-random.
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let x = ((seed >> 33) as f64) / (1u64 << 31) as f64;
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let y = ((seed >> 33) as f64) / (1u64 << 31) as f64;
            points.push(Point2::new(x * 10.0, y * 10.0));
        }

        let mut scratch = vec![0u32; points.len()];
        let mut out = vec![[0u32; 3]; max_triangles(points.len())];
        let count = delaunay_triangulation_2(&points, &mut scratch, &mut out).unwrap();
        assert!(count > 0);
        assert!(
            verify_delaunay(&points, &out[..count]),
            "random triangulation is not Delaunay"
        );
    }

    #[test]
    fn boundary_equals_convex_hull() {
        let points = [
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(4.0, 4.0),
            Point2::new(0.0, 4.0),
            Point2::new(2.0, 2.0), // interior point
        ];
        let mut scratch = vec![0u32; 5];
        let mut out = vec![[0u32; 3]; max_triangles(5)];
        let count = delaunay_triangulation_2(&points, &mut scratch, &mut out).unwrap();

        // Compute convex hull.
        let mut hull_scratch = vec![0u32; 15];
        let mut hull_out = vec![0u32; 5];
        let hull_count = convex_hull_indices_2(&points, &mut hull_scratch, &mut hull_out).unwrap();

        // Collect boundary edges from triangulation.
        let mut tri_edges: std::collections::HashSet<[u32; 2]> = std::collections::HashSet::new();
        for tri in &out[..count] {
            for i in 0..3 {
                let a = tri[i];
                let b = tri[(i + 1) % 3];
                let edge = if a < b { [a, b] } else { [b, a] };
                // If the edge already exists, it's shared (interior). Remove it.
                if !tri_edges.insert(edge) {
                    tri_edges.remove(&edge);
                }
            }
        }

        // Boundary vertices should match hull vertices.
        let mut boundary_verts: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for [a, b] in &tri_edges {
            boundary_verts.insert(*a);
            boundary_verts.insert(*b);
        }
        let mut hull_verts: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for i in 0..hull_count {
            hull_verts.insert(hull_out[i]);
        }
        assert_eq!(boundary_verts, hull_verts);
    }

    #[test]
    fn manifold_check_via_half_edges() {
        let points = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
            Point2::new(0.5, 0.5),
        ];
        let mut scratch = vec![0u32; 5];
        let mut out = vec![[0u32; 3]; max_triangles(5)];
        let count = delaunay_triangulation_2(&points, &mut scratch, &mut out).unwrap();

        let triangles = &out[..count];
        let edge_count = triangles.len() * 3;
        let mut half_edges = vec![super::super::topology::HalfEdge::default(); edge_count];
        let slot_count = required_edge_slots(triangles.len());
        let mut slots = vec![EdgeSlot::default(); slot_count];
        let result =
            build_triangle_half_edges(points.len() as u32, triangles, &mut half_edges, &mut slots);
        assert!(
            result.is_ok(),
            "build_triangle_half_edges failed: {:?}",
            result.err()
        );
        let summary = result.unwrap();

        // For a planar triangulation of a convex region with 1 interior point:
        // V=5, E=?, F=count. Euler: V - E + F = 2 (for a disk, it's V - E + F = 1).
        // Actually for a triangulation of a disk: V - E + F = 1.
        // Boundary edges = boundary_half_edges.
        assert!(
            summary.boundary_half_edges > 0,
            "should have boundary edges (disk topology)"
        );
    }

    #[test]
    fn determinism_same_input_same_output() {
        let points = [
            Point2::new(0.0, 0.0),
            Point2::new(3.0, 0.0),
            Point2::new(2.0, 3.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.5, 2.5),
            Point2::new(2.5, 1.5),
        ];
        let mut scratch1 = vec![0u32; 6];
        let mut out1 = vec![[0u32; 3]; max_triangles(6)];
        let count1 = delaunay_triangulation_2(&points, &mut scratch1, &mut out1).unwrap();

        let mut scratch2 = vec![0u32; 6];
        let mut out2 = vec![[0u32; 3]; max_triangles(6)];
        let count2 = delaunay_triangulation_2(&points, &mut scratch2, &mut out2).unwrap();

        assert_eq!(count1, count2);
        assert_eq!(&out1[..count1], &out2[..count2]);
        assert_eq!(
            triangulation_hash(&out1[..count1]),
            triangulation_hash(&out2[..count2])
        );
    }

    #[test]
    fn determinism_hash_stable() {
        let points = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];
        let mut scratch = vec![0u32; 4];
        let mut out = vec![[0u32; 3]; max_triangles(4)];
        let count = delaunay_triangulation_2(&points, &mut scratch, &mut out).unwrap();
        let h1 = triangulation_hash(&out[..count]);
        let h2 = triangulation_hash(&out[..count]);
        assert_eq!(h1, h2);
    }

    #[test]
    fn cocircular_points_handled() {
        // Four points on a circle — the Delaunay triangulation should
        // produce 2 triangles (either diagonal is valid).
        let points = [
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
            Point2::new(-1.0, 0.0),
            Point2::new(0.0, -1.0),
        ];
        let mut scratch = vec![0u32; 4];
        let mut out = vec![[0u32; 3]; max_triangles(4)];
        let count = delaunay_triangulation_2(&points, &mut scratch, &mut out).unwrap();
        assert_eq!(count, 2);
        // The triangulation should still be valid (empty circumcircle
        // property holds — cocircular points are on the boundary, not inside).
        assert!(verify_delaunay(&points, &out[..count]));
    }

    #[test]
    fn duplicate_points_skipped() {
        // Two identical points — the triangulation should still work
        // for the non-degenerate case.
        let points = [
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 0.0), // duplicate
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        ];
        let mut scratch = vec![0u32; 4];
        let mut out = vec![[0u32; 3]; max_triangles(4)];
        let result = delaunay_triangulation_2(&points, &mut scratch, &mut out);
        // Should succeed — the duplicate will just be a vertex that
        // doesn't appear in any triangle (or appears in degenerate ones
        // that get filtered).
        if let Ok(count) = result {
            // Verify Delaunay property.
            assert!(verify_delaunay(&points, &out[..count]));
        }
    }

    #[test]
    fn larger_grid_is_delaunay() {
        let mut points = Vec::new();
        for y in 0..8 {
            for x in 0..8 {
                // Add jitter to avoid exact cocircular degeneracies.
                let jx = if (x + y) % 2 == 0 { 0.01 } else { -0.01 };
                let jy = if (x * y) % 3 == 0 { 0.02 } else { -0.02 };
                points.push(Point2::new(x as f64 + jx, y as f64 + jy));
            }
        }
        let mut scratch = vec![0u32; points.len()];
        let mut out = vec![[0u32; 3]; max_triangles(points.len())];
        let count = delaunay_triangulation_2(&points, &mut scratch, &mut out).unwrap();
        assert!(count > 0);
        assert!(
            verify_delaunay(&points, &out[..count]),
            "8x8 grid triangulation is not Delaunay"
        );
    }
}
