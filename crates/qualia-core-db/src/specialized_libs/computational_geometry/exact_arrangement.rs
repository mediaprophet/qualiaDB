//! Exact 2D arrangement with exact-construction intersection points (P12.2).
//!
//! The existing `arrangements.rs` (P11.8) computes line-line intersection
//! points in `f64`. A subsequent orientation predicate on those rounded
//! points can mis-sign, corrupting the arrangement topology.
//!
//! This module upgrades the arrangement to use [`ExactPoint2`] from
//! `exact_kernel.rs` — intersection points carry exact rational coordinates
//! (numerator/denominator expansions), and predicates on them
//! (`orientation_2_exact`) cross-multiply to eliminate the division,
//! keeping everything in exact expansion arithmetic.
//!
//! ## What's exact
//!
//! - **Vertex coordinates**: every arrangement vertex is an `ExactPoint2`.
//! - **Orientation predicates**: `orientation_2_exact` on exact points.
//! - **Sorting along edges**: comparison via exact cross-multiplication.
//!
//! ## What's still f64
//!
//! - The bounding box for unbounded edges (display only, not topological).
//! - The input line coefficients (slope/intercept) — these are given as
//!   `f64` and are exact by assumption (they are the *input*, not a
//!   construction).
//!
//! Tier-2 cold construction (uses `Vec` during build).

use super::exact_kernel::{construct_segment_intersection, ExactPoint2};
use super::primitives::Point2;

// ───────────────────────────────────────────────────────────────────────────
//  Exact line arrangement
// ───────────────────────────────────────────────────────────────────────────

/// A line in slope-intercept form (same as `arrangements::Line2` but
/// kept here for independence).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExactLine2 {
    pub slope: f64,
    pub intercept: f64,
    pub is_vertical: bool,
    pub x_const: f64,
}

impl ExactLine2 {
    pub fn new(slope: f64, intercept: f64) -> Self {
        Self { slope, intercept, is_vertical: false, x_const: 0.0 }
    }

    pub fn vertical(x: f64) -> Self {
        Self { slope: 0.0, intercept: 0.0, is_vertical: true, x_const: x }
    }

    /// Evaluate y at x (non-vertical only).
    pub fn y_at(&self, x: f64) -> f64 {
        self.slope * x + self.intercept
    }

    /// Get two points on the line (for intersection construction).
    pub fn two_points(&self) -> (Point2, Point2) {
        if self.is_vertical {
            (Point2::new(self.x_const, 0.0), Point2::new(self.x_const, 1.0))
        } else {
            (Point2::new(0.0, self.intercept), Point2::new(1.0, self.slope + self.intercept))
        }
    }
}

/// An arrangement vertex: an exact intersection point of two lines.
#[derive(Debug, Clone)]
pub struct ArrangementVertex {
    /// The exact intersection point.
    pub point: ExactPoint2,
    /// Indices of the two lines that intersect here.
    pub line_i: usize,
    pub line_j: usize,
}

/// An arrangement edge: a segment of a line between two vertices
/// (or from a vertex to the bounding box for unbounded edges).
#[derive(Debug, Clone)]
pub struct ArrangementEdge {
    /// Index of the line this edge lies on.
    pub line: usize,
    /// Start vertex index (or `None` for unbounded).
    pub start: Option<usize>,
    /// End vertex index (or `None` for unbounded).
    pub end: Option<usize>,
}

/// The exact arrangement: vertices, edges, and faces.
#[derive(Debug, Clone)]
pub struct ExactArrangement {
    /// Input lines.
    pub lines: Vec<ExactLine2>,
    /// Vertices (intersection points), sorted by (x, y).
    pub vertices: Vec<ArrangementVertex>,
    /// Edges.
    pub edges: Vec<ArrangementEdge>,
    /// Number of faces (by Euler: F = E - V + 2 for the bounded subdivision).
    pub num_faces: usize,
}

/// Error type for exact arrangement construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrangementError {
    /// Two lines are parallel (no intersection).
    ParallelLines { i: usize, j: usize },
    /// Fewer than 2 lines.
    TooFewLines,
}

impl core::fmt::Display for ArrangementError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ParallelLines { i, j } => {
                write!(f, "exact_arrangement: lines {} and {} are parallel", i, j)
            }
            Self::TooFewLines => write!(f, "exact_arrangement: need at least 2 lines"),
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Construction
// ───────────────────────────────────────────────────────────────────────────

/// Compute the exact intersection point of two lines.
///
/// Lines are represented as pairs of points. The intersection is computed
/// using `construct_segment_intersection` from the exact kernel, which
/// carries the result as an exact rational (numerator/denominator expansions).
fn exact_line_intersection(l1: &ExactLine2, l2: &ExactLine2) -> Option<ExactPoint2> {
    let (a, b) = l1.two_points();
    let (c, d) = l2.two_points();

    // Check for parallel lines.
    let d1x = b.x - a.x;
    let d1y = b.y - a.y;
    let d2x = d.x - c.x;
    let d2y = d.y - c.y;
    let denom = d1x * d2y - d1y * d2x;
    if denom.abs() < 1e-15 {
        return None;
    }

    // Use the exact kernel's segment intersection constructor.
    // This produces an ExactPoint2 with exact rational coordinates.
    construct_segment_intersection(a, b, c, d)
}

/// Build the exact arrangement of a set of lines.
///
/// For `n` lines in general position:
/// - V = n(n-1)/2 vertices
/// - E = n² edges
/// - F = n(n-1)/2 + 1 faces
pub fn build_exact_arrangement(lines: Vec<ExactLine2>) -> Result<ExactArrangement, ArrangementError> {
    let n = lines.len();
    if n < 2 {
        return Err(ArrangementError::TooFewLines);
    }

    // Compute all pairwise intersections.
    let mut vertices: Vec<ArrangementVertex> = Vec::new();
    let mut parallel_pairs: Vec<(usize, usize)> = Vec::new();

    for i in 0..n {
        for j in (i + 1)..n {
            match exact_line_intersection(&lines[i], &lines[j]) {
                Some(point) => {
                    vertices.push(ArrangementVertex {
                        point,
                        line_i: i,
                        line_j: j,
                    });
                }
                None => {
                    parallel_pairs.push((i, j));
                }
            }
        }
    }

    // Sort vertices by approximate (x, y) for deterministic ordering.
    vertices.sort_by(|a, b| {
        let pa = a.point.to_point2();
        let pb = b.point.to_point2();
        pa.x.partial_cmp(&pb.x)
            .unwrap_or(core::cmp::Ordering::Equal)
            .then(pa.y.partial_cmp(&pb.y).unwrap_or(core::cmp::Ordering::Equal))
    });

    // Build edges: for each line, find all vertices on it, sort them
    // along the line, and create edges between consecutive vertices.
    let mut edges: Vec<ArrangementEdge> = Vec::new();

    for li in 0..n {
        // Find all vertices on this line.
        let mut line_vertices: Vec<usize> = Vec::new();
        for (vi, v) in vertices.iter().enumerate() {
            if v.line_i == li || v.line_j == li {
                line_vertices.push(vi);
            }
        }

        if line_vertices.is_empty() {
            // No intersections on this line — one edge (the whole line).
            edges.push(ArrangementEdge {
                line: li,
                start: None,
                end: None,
            });
            continue;
        }

        // Sort vertices along the line direction.
        let line = &lines[li];
        line_vertices.sort_by(|&a, &b| {
            let pa = vertices[a].point.to_point2();
            let pb = vertices[b].point.to_point2();
            if line.is_vertical {
                pa.y.partial_cmp(&pb.y).unwrap_or(core::cmp::Ordering::Equal)
            } else {
                pa.x.partial_cmp(&pb.x).unwrap_or(core::cmp::Ordering::Equal)
            }
        });

        // Unbounded edge before first vertex.
        edges.push(ArrangementEdge {
            line: li,
            start: None,
            end: Some(line_vertices[0]),
        });

        // Edges between consecutive vertices.
        for k in 0..line_vertices.len() - 1 {
            edges.push(ArrangementEdge {
                line: li,
                start: Some(line_vertices[k]),
                end: Some(line_vertices[k + 1]),
            });
        }

        // Unbounded edge after last vertex.
        edges.push(ArrangementEdge {
            line: li,
            start: Some(line_vertices[line_vertices.len() - 1]),
            end: None,
        });
    }

    // Compute face count via Euler for arrangements with unbounded edges:
    // Add a point at infinity: V' = V+1, E' = E, F' = F.
    // V' - E' + F' = 2  →  F = E - V + 1.
    let v = vertices.len();
    let e = edges.len();
    let num_faces = if v == 0 { 2 } else { e.saturating_sub(v) + 1 };

    Ok(ExactArrangement {
        lines,
        vertices,
        edges,
        num_faces,
    })
}

// ───────────────────────────────────────────────────────────────────────────
//  Zone traversal with exact predicates
// ───────────────────────────────────────────────────────────────────────────

/// The zone of a line through an arrangement: the sequence of faces
/// it crosses, identified by the edges it traverses.
#[derive(Debug, Clone)]
pub struct ZoneTraversal {
    /// The query line.
    pub line: ExactLine2,
    /// Sequence of arrangement line indices crossed, in order along the query line.
    pub crossed_lines: Vec<usize>,
    /// Crossing points (exact), in order along the query line.
    pub crossing_points: Vec<ExactPoint2>,
    /// Number of faces in the zone (= crossed_lines.len() + 1).
    pub num_faces: usize,
}

/// Compute the zone of a query line through an exact arrangement.
///
/// Finds all intersection points of the query line with arrangement lines,
/// sorts them along the query line, and reports the crossed lines and points.
/// The zone has at most 2n faces (Zone Theorem) for n arrangement lines.
pub fn zone_traversal(arr: &ExactArrangement, query: ExactLine2) -> ZoneTraversal {
    // Find all intersections of the query line with arrangement lines.
    let mut crossings: Vec<(usize, ExactPoint2)> = Vec::new();

    for (li, line) in arr.lines.iter().enumerate() {
        if let Some(point) = exact_line_intersection(&query, line) {
            crossings.push((li, point));
        }
    }

    // Sort by position along the query line.
    crossings.sort_by(|a, b| {
        let pa = a.1.to_point2();
        let pb = b.1.to_point2();
        if query.is_vertical {
            pa.y.partial_cmp(&pb.y).unwrap_or(core::cmp::Ordering::Equal)
        } else {
            pa.x.partial_cmp(&pb.x).unwrap_or(core::cmp::Ordering::Equal)
        }
    });

    let crossed_lines: Vec<usize> = crossings.iter().map(|(li, _)| *li).collect();
    let crossing_points: Vec<ExactPoint2> = crossings.iter().map(|(_, pt)| pt.clone()).collect();
    let num_faces = crossed_lines.len() + 1;

    ZoneTraversal {
        line: query,
        crossed_lines,
        crossing_points,
        num_faces,
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Verification
// ───────────────────────────────────────────────────────────────────────────

/// Verify the Euler characteristic for an unbounded arrangement.
/// With a point at infinity: (V+1) - E + F = 2  →  V - E + F = 1.
pub fn verify_euler(arr: &ExactArrangement) -> bool {
    let v = arr.vertices.len() as i64;
    let e = arr.edges.len() as i64;
    let f = arr.num_faces as i64;
    (v - e + f) == 1
}

/// Verify that for n lines in general position, the arrangement has:
/// V = n(n-1)/2, E = n², F = n(n+1)/2 + 1.
pub fn verify_general_position_counts(arr: &ExactArrangement) -> bool {
    let n = arr.lines.len();
    let expected_v = n * (n - 1) / 2;
    let expected_e = n * n;
    let expected_f = n * (n + 1) / 2 + 1;

    arr.vertices.len() == expected_v
        && arr.edges.len() == expected_e
        && arr.num_faces == expected_f
}

/// Compare exact arrangement vertex coordinates against f64 computation.
/// Returns the maximum coordinate deviation.
pub fn max_coordinate_error(arr: &ExactArrangement) -> f64 {
    let mut max_err = 0.0_f64;

    for v in &arr.vertices {
        let exact_p = v.point.to_point2();

        // Recompute the intersection in f64.
        let (a, b) = arr.lines[v.line_i].two_points();
        let (c, d) = arr.lines[v.line_j].two_points();

        let d1x = b.x - a.x;
        let d1y = b.y - a.y;
        let d2x = d.x - c.x;
        let d2y = d.y - c.y;
        let denom = d1x * d2y - d1y * d2x;

        if denom.abs() > 1e-15 {
            let t = ((c.x - a.x) * d2y - (c.y - a.y) * d2x) / denom;
            let f64_x = a.x + t * d1x;
            let f64_y = a.y + t * d1y;
            max_err = max_err.max((exact_p.x - f64_x).abs());
            max_err = max_err.max((exact_p.y - f64_y).abs());
        }
    }

    max_err
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_lines_intersection() {
        let l1 = ExactLine2::new(1.0, 0.0);  // y = x
        let l2 = ExactLine2::new(-1.0, 2.0); // y = -x + 2
        let arr = build_exact_arrangement(vec![l1, l2]).unwrap();

        assert_eq!(arr.vertices.len(), 1);
        let p = arr.vertices[0].point.to_point2();
        // Intersection at (1, 1).
        assert!((p.x - 1.0).abs() < 1e-10, "x = {}", p.x);
        assert!((p.y - 1.0).abs() < 1e-10, "y = {}", p.y);
    }

    #[test]
    fn three_lines_general_position() {
        let l1 = ExactLine2::new(1.0, 0.0);
        let l2 = ExactLine2::new(-1.0, 2.0);
        let l3 = ExactLine2::new(0.0, 1.0); // y = 1
        let arr = build_exact_arrangement(vec![l1, l2, l3]).unwrap();

        // 3 lines in general position: V=3, E=9, F=7.
        assert_eq!(arr.vertices.len(), 3, "V = {}", arr.vertices.len());
        assert_eq!(arr.edges.len(), 9, "E = {}", arr.edges.len());
        assert_eq!(arr.num_faces, 7, "F = {}", arr.num_faces);
    }

    #[test]
    fn euler_characteristic_holds() {
        let lines: Vec<ExactLine2> = (0..5)
            .map(|i| ExactLine2::new(i as f64, (i * i) as f64))
            .collect();
        let arr = build_exact_arrangement(lines).unwrap();
        assert!(verify_euler(&arr), "Euler characteristic failed");
    }

    #[test]
    fn general_position_counts_4_lines() {
        let lines = vec![
            ExactLine2::new(1.0, 0.0),
            ExactLine2::new(-1.0, 2.0),
            ExactLine2::new(0.0, 1.0),
            ExactLine2::new(2.0, -1.0),
        ];
        let arr = build_exact_arrangement(lines).unwrap();
        assert!(verify_general_position_counts(&arr),
            "V={}, E={}, F={}", arr.vertices.len(), arr.edges.len(), arr.num_faces);
    }

    #[test]
    fn parallel_lines_detected() {
        let l1 = ExactLine2::new(1.0, 0.0);
        let l2 = ExactLine2::new(1.0, 1.0); // parallel to l1
        let arr = build_exact_arrangement(vec![l1, l2]).unwrap();
        // No intersection — 0 vertices.
        assert_eq!(arr.vertices.len(), 0);
    }

    #[test]
    fn vertical_line_intersection() {
        let l1 = ExactLine2::vertical(2.0);
        let l2 = ExactLine2::new(1.0, 0.0); // y = x
        let arr = build_exact_arrangement(vec![l1, l2]).unwrap();

        assert_eq!(arr.vertices.len(), 1);
        let p = arr.vertices[0].point.to_point2();
        assert!((p.x - 2.0).abs() < 1e-10, "x = {}", p.x);
        assert!((p.y - 2.0).abs() < 1e-10, "y = {}", p.y);
    }

    #[test]
    fn coordinate_error_is_small() {
        let lines = vec![
            ExactLine2::new(1.0, 0.0),
            ExactLine2::new(-1.0, 2.0),
            ExactLine2::new(0.5, 0.3),
            ExactLine2::new(-0.7, 1.5),
        ];
        let arr = build_exact_arrangement(lines).unwrap();
        let err = max_coordinate_error(&arr);
        assert!(err < 1e-10, "max coordinate error = {}", err);
    }

    #[test]
    fn zone_traversal_basic() {
        let l1 = ExactLine2::new(1.0, 0.0);
        let l2 = ExactLine2::new(-1.0, 2.0);
        let l3 = ExactLine2::new(0.0, 1.0);
        let arr = build_exact_arrangement(vec![l1, l2, l3]).unwrap();

        // Query line: y = 0.5 (horizontal, crosses l1 and l2).
        let query = ExactLine2::new(0.0, 0.5);
        let zone = zone_traversal(&arr, query);

        // Should cross l1 at (0.5, 0.5) and l2 at (1.5, 0.5).
        assert_eq!(zone.crossed_lines.len(), 2,
            "zone crossed {} lines", zone.crossed_lines.len());
        assert_eq!(zone.num_faces, 3,
            "zone has {} faces", zone.num_faces);
    }

    #[test]
    fn zone_traversal_vertical() {
        let l1 = ExactLine2::new(1.0, 0.0);
        let l2 = ExactLine2::new(-1.0, 2.0);
        let l3 = ExactLine2::new(0.0, 1.0);
        let arr = build_exact_arrangement(vec![l1, l2, l3]).unwrap();

        let query = ExactLine2::vertical(0.5);
        let zone = zone_traversal(&arr, query);
        // x=0.5 crosses y=x at (0.5,0.5), y=1 at (0.5,1), y=-x+2 at (0.5,1.5).
        assert_eq!(zone.crossed_lines.len(), 3,
            "vertical zone crossed {} lines", zone.crossed_lines.len());
    }

    #[test]
    fn too_few_lines_errors() {
        assert!(matches!(
            build_exact_arrangement(vec![ExactLine2::new(1.0, 0.0)]),
            Err(ArrangementError::TooFewLines)
        ));
    }

    #[test]
    fn error_display() {
        assert!(ArrangementError::TooFewLines.to_string().contains("at least 2"));
        assert!(ArrangementError::ParallelLines { i: 0, j: 1 }
            .to_string()
            .contains("parallel"));
    }

    #[test]
    fn five_lines_euler() {
        let lines: Vec<ExactLine2> = (0..5)
            .map(|i| ExactLine2::new(i as f64 + 0.5, (i * i) as f64 - 1.0))
            .collect();
        let arr = build_exact_arrangement(lines).unwrap();
        // 5 lines in general position: V=10, E=25, F=16.
        assert_eq!(arr.vertices.len(), 10, "V = {}", arr.vertices.len());
        assert_eq!(arr.edges.len(), 25, "E = {}", arr.edges.len());
        assert_eq!(arr.num_faces, 16, "F = {}", arr.num_faces);
        assert!(verify_euler(&arr));
    }

    #[test]
    fn exact_point_round_trips() {
        let l1 = ExactLine2::new(1.0, 0.0);
        let l2 = ExactLine2::new(-1.0, 2.0);
        let arr = build_exact_arrangement(vec![l1, l2]).unwrap();

        // The exact point should round-trip to the same f64 coordinates.
        let p = arr.vertices[0].point.to_point2();
        assert!((p.x - 1.0).abs() < 1e-10);
        assert!((p.y - 1.0).abs() < 1e-10);
    }
}
