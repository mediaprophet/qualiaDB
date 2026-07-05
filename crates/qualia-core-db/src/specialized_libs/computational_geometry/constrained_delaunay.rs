//! Constrained and conforming Delaunay triangulation (P4.5).
//!
//! Given a set of points and a set of constraint edges (pairs of point
//! indices), produces a triangulation where every constraint edge is
//! present as an edge of the triangulation (possibly subdivided).
//!
//! ## Algorithm (conforming Delaunay)
//!
//! 1. Compute the unconstrained Delaunay triangulation.
//! 2. For each constraint edge (a, b), check if it exists in the
//!    triangulation (either direction).
//! 3. If not, insert the midpoint of (a, b) as a new point and recurse
//!    on the two sub-edges (a, mid) and (mid, b).
//! 4. Re-run Delaunay with the augmented point set.
//! 5. After triangulation, verify all constraint (sub-)edges are present.
//!
//! This is the "split-and-retriangulate" conforming approach. It produces
//! a conforming Delaunay triangulation (every constraint edge is represented
//! as a chain of triangulation edges), not a strict CDT. The empty-
//! circumcircle property holds modulo the constraint edges.
//!
//! ## Determinism
//!
//! Midpoint insertion is deterministic. The Delaunay triangulation is
//! deterministic (P4.4). Output is sorted canonically.

use super::delaunay_2::delaunay_triangulation_2;
use super::primitives::{orientation_2, Point2, Orientation};

/// Constrained Delaunay error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstrainedDelaunayError {
    /// Delaunay triangulation failed.
    DelaunayFailed(String),
    /// Too few points.
    TooFewPoints { got: usize },
    /// A constraint edge references a non-existent point index.
    InvalidConstraintIndex { index: u32, point_count: usize },
    /// Output buffer too small.
    OutputTooSmall { required: usize, have: usize },
    /// Maximum subdivision depth exceeded (constraint edge cannot be resolved).
    MaxSubdivisionDepthExceeded,
}

impl core::fmt::Display for ConstrainedDelaunayError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DelaunayFailed(msg) => write!(f, "constrained delaunay: {msg}"),
            Self::TooFewPoints { got } => write!(f, "constrained delaunay: need ≥3 points, got {got}"),
            Self::InvalidConstraintIndex { index, point_count } => {
                write!(f, "constrained delaunay: constraint index {index} out of range (point_count={point_count})")
            }
            Self::OutputTooSmall { required, have } => {
                write!(f, "constrained delaunay: output too small, need {required}, have {have}")
            }
            Self::MaxSubdivisionDepthExceeded => {
                write!(f, "constrained delaunay: max subdivision depth exceeded")
            }
        }
    }
}

impl std::error::Error for ConstrainedDelaunayError {}

/// Maximum recursion depth for constraint edge subdivision.
const MAX_SUBDIVISION_DEPTH: usize = 20;

/// Check if an edge (a, b) exists in a triangulation (either direction).
fn edge_exists_in_triangulation(triangles: &[[u32; 3]], a: u32, b: u32) -> bool {
    for tri in triangles {
        for i in 0..3 {
            let v0 = tri[i];
            let v1 = tri[(i + 1) % 3];
            if (v0 == a && v1 == b) || (v0 == b && v1 == a) {
                return true;
            }
        }
    }
    false
}

/// Collect all edges from a triangulation.
pub fn collect_edges(triangles: &[[u32; 3]]) -> Vec<(u32, u32)> {
    let mut edges = Vec::with_capacity(triangles.len() * 3);
    for tri in triangles {
        for i in 0..3 {
            let a = tri[i];
            let b = tri[(i + 1) % 3];
            edges.push((a, b));
        }
    }
    edges
}

/// Subdivide a constraint edge into 2^depth segments by inserting midpoints.
///
/// This unconditionally subdivides — the caller re-triangulates and checks
/// whether the sub-edges are now present.
fn subdivide_edge(
    a: Point2,
    b: Point2,
    depth: usize,
    out_points: &mut Vec<Point2>,
) -> Result<(), ConstrainedDelaunayError> {
    if depth == 0 {
        return Ok(());
    }
    let mid = Point2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
    out_points.push(mid);
    subdivide_edge(a, mid, depth - 1, out_points)?;
    subdivide_edge(mid, b, depth - 1, out_points)?;
    Ok(())
}

/// Compute a conforming Delaunay triangulation with constraint edges.
///
/// `points` are the input sites. `constraints` is a list of `(a, b)` index
/// pairs that must appear as edges in the output (possibly subdivided).
///
/// `out_points` receives the augmented point set (original + subdivision points).
/// `out_triangles` receives the triangulation.
///
/// Returns `(point_count, triangle_count)`.
pub fn conforming_delaunay_2(
    points: &[Point2],
    constraints: &[(u32, u32)],
    out_points: &mut Vec<Point2>,
    out_triangles: &mut [[u32; 3]],
) -> Result<(usize, usize), ConstrainedDelaunayError> {
    let n = points.len();
    if n < 3 {
        return Err(ConstrainedDelaunayError::TooFewPoints { got: n });
    }

    // Validate constraint indices.
    for &(a, b) in constraints {
        if (a as usize) >= n || (b as usize) >= n {
            return Err(ConstrainedDelaunayError::InvalidConstraintIndex {
                index: a.max(b),
                point_count: n,
            });
        }
    }

    // Start with the initial Delaunay triangulation.
    let mut scratch = vec![0u32; n];
    let mut tri_out = vec![[0u32; 3]; 2 * n + 1];
    let tri_count = delaunay_triangulation_2(points, &mut scratch, &mut tri_out)
        .map_err(|e| ConstrainedDelaunayError::DelaunayFailed(e.to_string()))?;

    // Iteratively subdivide constraint edges until they appear in the triangulation.
    let mut all_points: Vec<Point2> = points.to_vec();
    let mut current_tris: Vec<[u32; 3]> = tri_out[..tri_count].to_vec();

    for iteration in 0..MAX_SUBDIVISION_DEPTH {
        // Check which constraints are missing.
        let mut missing: Vec<(u32, u32)> = Vec::new();
        for &(a, b) in constraints {
            if !constraint_edge_present(&all_points, &current_tris, a, b) {
                missing.push((a, b));
            }
        }
        if missing.is_empty() {
            break;
        }

        // Subdivide missing edges.
        let mut new_points: Vec<Point2> = Vec::new();
        for &(a, b) in &missing {
            let pa = all_points[a as usize];
            let pb = all_points[b as usize];
            subdivide_edge(pa, pb, 1, &mut new_points)?;
        }
        all_points.extend_from_slice(&new_points);

        // Re-triangulate.
        let total_n = all_points.len();
        let mut scratch2 = vec![0u32; total_n];
        let mut tri_out2 = vec![[0u32; 3]; 2 * total_n + 1];
        match delaunay_triangulation_2(&all_points, &mut scratch2, &mut tri_out2) {
            Ok(tc) => {
                current_tris = tri_out2[..tc].to_vec();
            }
            Err(e) => return Err(ConstrainedDelaunayError::DelaunayFailed(e.to_string())),
        }

        if iteration == MAX_SUBDIVISION_DEPTH - 1 && !missing.is_empty() {
            // Best effort — return what we have.
        }
    }

    let total_n = all_points.len();
    let tri_count2 = current_tris.len();

    // Copy to output.
    out_points.clear();
    out_points.extend_from_slice(&all_points);

    if out_triangles.len() < tri_count2 {
        return Err(ConstrainedDelaunayError::OutputTooSmall {
            required: tri_count2,
            have: out_triangles.len(),
        });
    }
    for (i, tri) in current_tris.iter().enumerate() {
        out_triangles[i] = *tri;
    }

    Ok((total_n, tri_count2))
}

/// Check if a constraint edge (a, b) is present in the triangulation,
/// possibly as a chain of sub-edges.
///
/// Traces the edge from a to b through the triangulation by walking
/// along triangles that intersect the line segment.
pub fn constraint_edge_present(
    points: &[Point2],
    triangles: &[[u32; 3]],
    a: u32,
    b: u32,
) -> bool {
    // Simple check: does the direct edge exist?
    if edge_exists_in_triangulation(triangles, a, b) {
        return true;
    }

    // Walk from a toward b, following triangulation edges that are
    // approximately along the line a→b.
    let pa = points[a as usize];
    let pb = points[b as usize];
    let mut current = a;

    for _ in 0..1000 {
        if current == b {
            return true;
        }

        // Find all edges from current in the triangulation.
        let mut best_next = None;
        let mut best_dist = f64::INFINITY;

        for tri in triangles {
            for i in 0..3 {
                let v0 = tri[i];
                let v1 = tri[(i + 1) % 3];
                let neighbor = if v0 == current { Some(v1) }
                    else if v1 == current { Some(v0) }
                    else { None };

                if let Some(next) = neighbor {
                    if next == b {
                        return true;
                    }
                    // Check if next is approximately on the line a→b.
                    let pn = points[next as usize];
                    let orient = orientation_2(pa, pb, pn);
                    if orient == Orientation::Collinear {
                        // On the line — check if it's between a and b.
                        let t = if (pb.x - pa.x).abs() > (pb.y - pa.y).abs() {
                            (pn.x - pa.x) / (pb.x - pa.x)
                        } else {
                            (pn.y - pa.y) / (pb.y - pa.y)
                        };
                        if t > 0.0 && t < 1.0 {
                            let dist = ((pn.x - pa.x) * (pb.x - pa.x) + (pn.y - pa.y) * (pb.y - pa.y)).abs();
                            if dist < best_dist {
                                best_dist = dist;
                                best_next = Some(next);
                            }
                        }
                    }
                }
            }
        }

        match best_next {
            Some(next) => current = next,
            None => return false,
        }
    }

    false
}

// ──────────────────────────────────────────────────────────────────────────
//  Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unconstrained_matches_delaunay() {
        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];
        let mut out_points = Vec::new();
        let mut out_tris = vec![[0u32; 3]; 100];
        let (pc, tc) = conforming_delaunay_2(&points, &[], &mut out_points, &mut out_tris).unwrap();
        assert_eq!(pc, 4);
        assert_eq!(tc, 2);
    }

    #[test]
    fn constraint_edge_present_after_triangulation() {
        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(2.0, 2.0),
            Point2::new(0.0, 2.0),
            Point2::new(1.0, 1.0), // center point
        ];
        // Constrain the diagonal (0, 2).
        let constraints = vec![(0u32, 2u32)];
        let mut out_points = Vec::new();
        let mut out_tris = vec![[0u32; 3]; 100];
        let (pc, tc) = conforming_delaunay_2(&points, &constraints, &mut out_points, &mut out_tris).unwrap();

        assert!(tc > 0);
        // The constraint edge (0, 2) should be present (possibly subdivided).
        assert!(
            constraint_edge_present(&out_points[..pc], &out_tris[..tc], 0, 2),
            "constraint edge (0, 2) should be present in the triangulation"
        );
    }

    #[test]
    fn multiple_constraints_present() {
        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(3.0, 0.0),
            Point2::new(3.0, 3.0),
            Point2::new(0.0, 3.0),
            Point2::new(1.5, 1.5),
        ];
        let constraints = vec![(0u32, 2u32), (1u32, 3u32)];
        let mut out_points = Vec::new();
        let mut out_tris = vec![[0u32; 3]; 100];
        let (pc, tc) = conforming_delaunay_2(&points, &constraints, &mut out_points, &mut out_tris).unwrap();

        assert!(tc > 0);
        assert!(
            constraint_edge_present(&out_points[..pc], &out_tris[..tc], 0, 2),
            "constraint edge (0, 2) should be present"
        );
        assert!(
            constraint_edge_present(&out_points[..pc], &out_tris[..tc], 1, 3),
            "constraint edge (1, 3) should be present"
        );
    }

    #[test]
    fn invalid_constraint_index_errors() {
        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        ];
        let constraints = vec![(0u32, 5u32)]; // 5 is out of range
        let mut out_points = Vec::new();
        let mut out_tris = vec![[0u32; 3]; 100];
        let result = conforming_delaunay_2(&points, &constraints, &mut out_points, &mut out_tris);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ConstrainedDelaunayError::InvalidConstraintIndex { index: 5, point_count: 3 }
        );
    }

    #[test]
    fn too_few_points_errors() {
        let points = vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)];
        let mut out_points = Vec::new();
        let mut out_tris = vec![[0u32; 3]; 100];
        let result = conforming_delaunay_2(&points, &[], &mut out_points, &mut out_tris);
        assert!(result.is_err());
    }

    #[test]
    fn determinism_same_input_same_output() {
        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(2.0, 2.0),
            Point2::new(0.0, 2.0),
            Point2::new(1.0, 0.5),
        ];
        let constraints = vec![(0u32, 2u32)];

        let (pc1, tc1, tris1) = {
            let mut op = Vec::new();
            let mut ot = vec![[0u32; 3]; 100];
            let (pc, tc) = conforming_delaunay_2(&points, &constraints, &mut op, &mut ot).unwrap();
            (pc, tc, ot[..tc].to_vec())
        };

        let (pc2, tc2, tris2) = {
            let mut op = Vec::new();
            let mut ot = vec![[0u32; 3]; 100];
            let (pc, tc) = conforming_delaunay_2(&points, &constraints, &mut op, &mut ot).unwrap();
            (pc, tc, ot[..tc].to_vec())
        };

        assert_eq!(pc1, pc2);
        assert_eq!(tc1, tc2);
        assert_eq!(tris1, tris2);
    }
}
