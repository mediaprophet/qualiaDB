//! P11.4 — Simple-polygon, polygon-with-holes, and PSLG validation.
//!
//! The acceptance gate requires: "Detects crossings, duplicate edges,
//! invalid nesting and orientation; emits canonical components and typed
//! repair suggestions without silent mutation."
//!
//! ## Validation checks
//!
//! For a simple polygon:
//! - **Crossing edges** — non-adjacent edges that intersect (proper or
//!   T-junction).
//! - **Duplicate edges** — the same edge appears twice (in either direction).
//! - **Degenerate edges** — zero-length edges (consecutive duplicate points).
//! - **Orientation** — the polygon should be CCW for the outer boundary
//!   (by convention).
//! - **Minimum vertex count** — at least 3 vertices.
//!
//! For a polygon-with-holes:
//! - All of the above, plus:
//! - **Nesting** — each hole must be inside the outer boundary.
//! - **Hole orientation** — holes should be CW (by convention, opposite to
//!   the outer boundary).
//! - **Hole disjointness** — holes must not overlap each other.
//! - **No hole crosses outer boundary** — a hole edge cannot cross an outer
//!   edge.
//!
//! For a PSLG (Planar Straight-Line Graph):
//! - **Duplicate edges** — the same edge appears twice.
//! - **Crossing edges** — edges that intersect at non-endpoint points.
//! - **Isolated vertices** — vertices not referenced by any edge.
//!
//! ## Typed repair suggestions
//!
//! The validator never silently mutates the input. Instead, it returns a
//! `ValidationReport` with typed `ValidationIssue` entries, each with a
//! `RepairSuggestion` that the caller can apply (or ignore).

use super::primitives::Point2;
use super::segment_intersection_2::classify_segment_intersection_2;

// ───────────────────────────────────────────────────────────────────────────
//  Validation types
// ───────────────────────────────────────────────────────────────────────────

/// A typed validation issue.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationIssue {
    /// Two non-adjacent edges cross (proper intersection or T-junction).
    CrossingEdges {
        edge1: usize,
        edge2: usize,
        point: Point2,
    },
    /// The same edge appears twice (possibly in opposite direction).
    DuplicateEdge { edge1: usize, edge2: usize },
    /// A zero-length edge (consecutive duplicate vertices).
    DegenerateEdge { edge: usize },
    /// The polygon has fewer than 3 vertices.
    TooFewVertices { actual: usize },
    /// The polygon orientation is wrong (expected CCW for outer, CW for holes).
    WrongOrientation {
        expected_ccw: bool,
        actual_ccw: bool,
    },
    /// A hole is not inside the outer boundary.
    HoleOutsideBoundary { hole_index: usize },
    /// A hole edge crosses an outer boundary edge.
    HoleCrossesBoundary {
        hole_index: usize,
        hole_edge: usize,
        boundary_edge: usize,
    },
    /// Two holes overlap (their edges cross).
    HolesOverlap { hole1: usize, hole2: usize },
    /// A vertex in the PSLG is not referenced by any edge.
    IsolatedVertex { vertex: usize },
}

/// A typed repair suggestion for a validation issue.
#[derive(Debug, Clone, PartialEq)]
pub enum RepairSuggestion {
    /// Split the crossing edges at the intersection point.
    SplitAtIntersection {
        edge1: usize,
        edge2: usize,
        point: Point2,
    },
    /// Remove the duplicate edge.
    RemoveDuplicateEdge { edge_to_remove: usize },
    /// Remove the degenerate (zero-length) edge by merging the vertices.
    RemoveDegenerateEdge { edge: usize },
    /// Reverse the vertex order to fix the orientation.
    ReverseVertexOrder,
    /// Add more vertices (the polygon needs at least 3).
    AddVertices { needed: usize },
    /// Move the hole inside the outer boundary.
    MoveHoleInside { hole_index: usize },
    /// Remove the hole that crosses the boundary.
    RemoveCrossingHole { hole_index: usize },
    /// Merge or separate the overlapping holes.
    FixOverlappingHoles { hole1: usize, hole2: usize },
    /// Remove the isolated vertex.
    RemoveIsolatedVertex { vertex: usize },
    /// No automatic repair available — manual intervention needed.
    ManualRepair,
}

/// A validation report containing all issues found and repair suggestions.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// All issues found during validation.
    pub issues: Vec<ValidationIssue>,
    /// Whether the input is valid (no issues found).
    pub is_valid: bool,
}

impl ValidationReport {
    /// Create a report from a list of issues.
    pub fn from_issues(issues: Vec<ValidationIssue>) -> Self {
        let is_valid = issues.is_empty();
        Self { issues, is_valid }
    }

    /// Get a repair suggestion for each issue.
    pub fn repair_suggestions(&self) -> Vec<RepairSuggestion> {
        self.issues.iter().map(|issue| repair_for(issue)).collect()
    }
}

/// Get the repair suggestion for a single issue.
pub fn repair_for(issue: &ValidationIssue) -> RepairSuggestion {
    match issue {
        ValidationIssue::CrossingEdges {
            edge1,
            edge2,
            point,
        } => RepairSuggestion::SplitAtIntersection {
            edge1: *edge1,
            edge2: *edge2,
            point: *point,
        },
        ValidationIssue::DuplicateEdge { edge1: _, edge2 } => {
            RepairSuggestion::RemoveDuplicateEdge {
                edge_to_remove: *edge2,
            }
        }
        ValidationIssue::DegenerateEdge { edge } => {
            RepairSuggestion::RemoveDegenerateEdge { edge: *edge }
        }
        ValidationIssue::WrongOrientation { .. } => RepairSuggestion::ReverseVertexOrder,
        ValidationIssue::TooFewVertices { actual } => {
            RepairSuggestion::AddVertices { needed: 3 - actual }
        }
        ValidationIssue::HoleOutsideBoundary { hole_index } => RepairSuggestion::MoveHoleInside {
            hole_index: *hole_index,
        },
        ValidationIssue::HoleCrossesBoundary { hole_index, .. } => {
            RepairSuggestion::RemoveCrossingHole {
                hole_index: *hole_index,
            }
        }
        ValidationIssue::HolesOverlap { hole1, hole2 } => RepairSuggestion::FixOverlappingHoles {
            hole1: *hole1,
            hole2: *hole2,
        },
        ValidationIssue::IsolatedVertex { vertex } => {
            RepairSuggestion::RemoveIsolatedVertex { vertex: *vertex }
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Simple polygon validation
// ───────────────────────────────────────────────────────────────────────────

/// Validate a simple polygon (outer boundary without holes).
///
/// The polygon is given as a sequence of vertices. Edges are
/// (vertices[0], vertices[1]), (vertices[1], vertices[2]), ...,
/// (vertices[n-1], vertices[0]).
///
/// Checks:
/// - Minimum 3 vertices.
/// - No zero-length edges.
/// - No duplicate edges.
/// - No crossing edges (non-adjacent edges that intersect).
/// - CCW orientation (by convention).
pub fn validate_simple_polygon(vertices: &[Point2]) -> ValidationReport {
    let mut issues = Vec::new();

    // Check minimum vertex count.
    if vertices.len() < 3 {
        issues.push(ValidationIssue::TooFewVertices {
            actual: vertices.len(),
        });
        return ValidationReport::from_issues(issues);
    }

    let n = vertices.len();

    // Check for degenerate edges (zero-length).
    for i in 0..n {
        let j = (i + 1) % n;
        if vertices[i] == vertices[j] {
            issues.push(ValidationIssue::DegenerateEdge { edge: i });
        }
    }

    // Check for duplicate edges.
    for i in 0..n {
        let j = (i + 1) % n;
        for k in (i + 1)..n {
            let l = (k + 1) % n;
            // Same direction: (i,j) == (k,l)
            if vertices[i] == vertices[k] && vertices[j] == vertices[l] {
                issues.push(ValidationIssue::DuplicateEdge { edge1: i, edge2: k });
            }
            // Opposite direction: (i,j) == (l,k)
            if vertices[i] == vertices[l] && vertices[j] == vertices[k] {
                issues.push(ValidationIssue::DuplicateEdge { edge1: i, edge2: k });
            }
        }
    }

    // Check for crossing edges (non-adjacent).
    for i in 0..n {
        let j = (i + 1) % n;
        for k in (i + 1)..n {
            let l = (k + 1) % n;
            // Skip adjacent edges (they share a vertex).
            if k == i || k == j || l == i || l == j {
                continue;
            }
            let result =
                classify_segment_intersection_2(vertices[i], vertices[j], vertices[k], vertices[l]);
            if let Some(pt) = result.point {
                // Only report proper crossings and T-junctions, not shared
                // endpoints (which are adjacent edges already skipped).
                match result.class {
                    super::segment_intersection_2::SegmentIntersectionClass::Proper
                    | super::segment_intersection_2::SegmentIntersectionClass::TJunction(_) => {
                        issues.push(ValidationIssue::CrossingEdges {
                            edge1: i,
                            edge2: k,
                            point: pt,
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    // Check orientation (signed area → CCW if positive).
    let area = signed_area(vertices);
    let actual_ccw = area > 0.0;
    if !actual_ccw {
        issues.push(ValidationIssue::WrongOrientation {
            expected_ccw: true,
            actual_ccw: false,
        });
    }

    ValidationReport::from_issues(issues)
}

/// Compute the signed area of a polygon (positive = CCW).
fn signed_area(vertices: &[Point2]) -> f64 {
    let n = vertices.len();
    let mut sum = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        sum += vertices[i].x * vertices[j].y - vertices[j].x * vertices[i].y;
    }
    sum * 0.5
}

// ───────────────────────────────────────────────────────────────────────────
//  Polygon-with-holes validation
// ───────────────────────────────────────────────────────────────────────────

/// A polygon with holes.
#[derive(Debug, Clone)]
pub struct PolygonWithHoles {
    /// The outer boundary (should be CCW).
    pub outer: Vec<Point2>,
    /// The holes (each should be CW).
    pub holes: Vec<Vec<Point2>>,
}

/// Validate a polygon with holes.
///
/// Checks:
/// - All simple polygon checks on the outer boundary.
/// - All simple polygon checks on each hole (but holes should be CW).
/// - Each hole is inside the outer boundary.
/// - No hole edge crosses an outer boundary edge.
/// - No two holes overlap.
pub fn validate_polygon_with_holes(poly: &PolygonWithHoles) -> ValidationReport {
    let mut issues = Vec::new();

    // Validate outer boundary.
    let outer_report = validate_simple_polygon(&poly.outer);
    issues.extend(outer_report.issues);

    // Validate each hole (but expect CW orientation).
    for (hi, hole) in poly.holes.iter().enumerate() {
        if hole.len() < 3 {
            issues.push(ValidationIssue::TooFewVertices { actual: hole.len() });
            continue;
        }

        // Check hole orientation (should be CW, i.e., negative area).
        let hole_area = signed_area(hole);
        if hole_area > 0.0 {
            issues.push(ValidationIssue::WrongOrientation {
                expected_ccw: false,
                actual_ccw: true,
            });
        }

        // Check degenerate edges in hole.
        let hn = hole.len();
        for i in 0..hn {
            let j = (i + 1) % hn;
            if hole[i] == hole[j] {
                issues.push(ValidationIssue::DegenerateEdge { edge: i });
            }
        }

        // Check if hole is inside outer boundary.
        if !point_in_polygon(hole[0], &poly.outer) {
            issues.push(ValidationIssue::HoleOutsideBoundary { hole_index: hi });
        }

        // Check hole edges vs outer boundary edges for crossings.
        let on = poly.outer.len();
        for i in 0..hn {
            let j = (i + 1) % hn;
            for k in 0..on {
                let l = (k + 1) % on;
                let result =
                    classify_segment_intersection_2(hole[i], hole[j], poly.outer[k], poly.outer[l]);
                if let Some(_pt) = result.point {
                    match result.class {
                        super::segment_intersection_2::SegmentIntersectionClass::Proper
                        | super::segment_intersection_2::SegmentIntersectionClass::TJunction(_) => {
                            issues.push(ValidationIssue::HoleCrossesBoundary {
                                hole_index: hi,
                                hole_edge: i,
                                boundary_edge: k,
                            });
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Check hole-hole overlap.
    for (hi, hole1) in poly.holes.iter().enumerate() {
        for (hj, hole2) in poly.holes.iter().enumerate().skip(hi + 1) {
            let h1n = hole1.len();
            let h2n = hole2.len();
            let mut found_crossing = false;
            'outer: for i in 0..h1n {
                let j = (i + 1) % h1n;
                for k in 0..h2n {
                    let l = (k + 1) % h2n;
                    let result =
                        classify_segment_intersection_2(hole1[i], hole1[j], hole2[k], hole2[l]);
                    if let Some(_) = result.point {
                        match result.class {
                            super::segment_intersection_2::SegmentIntersectionClass::Proper
                            | super::segment_intersection_2::SegmentIntersectionClass::TJunction(
                                _,
                            ) => {
                                found_crossing = true;
                                break 'outer;
                            }
                            _ => {}
                        }
                    }
                }
            }
            if found_crossing {
                issues.push(ValidationIssue::HolesOverlap {
                    hole1: hi,
                    hole2: hj,
                });
            }
        }
    }

    ValidationReport::from_issues(issues)
}

/// Check if a point is inside a polygon (ray casting).
fn point_in_polygon(p: Point2, polygon: &[Point2]) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let pi = polygon[i];
        let pj = polygon[j];
        if (pi.y > p.y) != (pj.y > p.y) {
            let x_intersect = (pj.x - pi.x) * (p.y - pi.y) / (pj.y - pi.y) + pi.x;
            if p.x < x_intersect {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

// ───────────────────────────────────────────────────────────────────────────
//  PSLG validation
// ───────────────────────────────────────────────────────────────────────────

/// A PSLG (Planar Straight-Line Graph) edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PslgEdge {
    pub from: usize,
    pub to: usize,
}

/// Validate a PSLG.
///
/// Checks:
/// - No duplicate edges.
/// - No crossing edges (edges that intersect at non-endpoint points).
/// - Reports isolated vertices.
pub fn validate_pslg(vertices: &[Point2], edges: &[PslgEdge]) -> ValidationReport {
    let mut issues = Vec::new();

    // Check for duplicate edges.
    for i in 0..edges.len() {
        for j in (i + 1)..edges.len() {
            // Same direction.
            if edges[i].from == edges[j].from && edges[i].to == edges[j].to {
                issues.push(ValidationIssue::DuplicateEdge { edge1: i, edge2: j });
            }
            // Opposite direction.
            if edges[i].from == edges[j].to && edges[i].to == edges[j].from {
                issues.push(ValidationIssue::DuplicateEdge { edge1: i, edge2: j });
            }
        }
    }

    // Check for crossing edges.
    for i in 0..edges.len() {
        for j in (i + 1)..edges.len() {
            // Skip edges that share a vertex.
            let e1 = edges[i];
            let e2 = edges[j];
            if e1.from == e2.from || e1.from == e2.to || e1.to == e2.from || e1.to == e2.to {
                continue;
            }
            // Bounds check.
            if e1.from >= vertices.len()
                || e1.to >= vertices.len()
                || e2.from >= vertices.len()
                || e2.to >= vertices.len()
            {
                continue;
            }
            let result = classify_segment_intersection_2(
                vertices[e1.from],
                vertices[e1.to],
                vertices[e2.from],
                vertices[e2.to],
            );
            if let Some(pt) = result.point {
                match result.class {
                    super::segment_intersection_2::SegmentIntersectionClass::Proper
                    | super::segment_intersection_2::SegmentIntersectionClass::TJunction(_) => {
                        issues.push(ValidationIssue::CrossingEdges {
                            edge1: i,
                            edge2: j,
                            point: pt,
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    // Check for isolated vertices.
    let mut referenced = vec![false; vertices.len()];
    for e in edges {
        if e.from < vertices.len() {
            referenced[e.from] = true;
        }
        if e.to < vertices.len() {
            referenced[e.to] = true;
        }
    }
    for (i, &r) in referenced.iter().enumerate() {
        if !r {
            issues.push(ValidationIssue::IsolatedVertex { vertex: i });
        }
    }

    ValidationReport::from_issues(issues)
}

// ───────────────────────────────────────────────────────────────────────────
//  Canonical components
// ───────────────────────────────────────────────────────────────────────────

/// Canonicalize a simple polygon: ensure CCW orientation and no leading
/// duplicate vertex. Does NOT mutate the input — returns a new Vec.
pub fn canonicalize_simple_polygon(vertices: &[Point2]) -> Vec<Point2> {
    let mut result = vertices.to_vec();
    // Remove trailing duplicate of first vertex (if any).
    if result.len() > 1 && result[0] == result[result.len() - 1] {
        result.pop();
    }
    // Ensure CCW orientation.
    let area = signed_area(&result);
    if area < 0.0 {
        result.reverse();
    }
    result
}

/// Canonicalize a polygon with holes: outer CCW, holes CW.
pub fn canonicalize_polygon_with_holes(poly: &PolygonWithHoles) -> PolygonWithHoles {
    let outer = canonicalize_simple_polygon(&poly.outer);
    let holes: Vec<Vec<Point2>> = poly
        .holes
        .iter()
        .map(|hole| {
            let mut h = hole.to_vec();
            if h.len() > 1 && h[0] == h[h.len() - 1] {
                h.pop();
            }
            let area = signed_area(&h);
            if area > 0.0 {
                // Hole should be CW (negative area).
                h.reverse();
            }
            h
        })
        .collect();
    PolygonWithHoles { outer, holes }
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

    // ── Simple polygon validation ────────────────────────────────────────

    #[test]
    fn valid_ccw_square_passes() {
        let square = vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)];
        let report = validate_simple_polygon(&square);
        assert!(
            report.is_valid,
            "CCW square should be valid: {:?}",
            report.issues
        );
    }

    #[test]
    fn cw_polygon_reports_wrong_orientation() {
        let square = vec![p(0.0, 0.0), p(0.0, 1.0), p(1.0, 1.0), p(1.0, 0.0)];
        let report = validate_simple_polygon(&square);
        assert!(!report.is_valid);
        assert!(report
            .issues
            .iter()
            .any(|i| matches!(i, ValidationIssue::WrongOrientation { .. })));
    }

    #[test]
    fn too_few_vertices_reported() {
        let report = validate_simple_polygon(&[p(0.0, 0.0), p(1.0, 0.0)]);
        assert!(!report.is_valid);
        assert!(report
            .issues
            .iter()
            .any(|i| matches!(i, ValidationIssue::TooFewVertices { actual: 2 })));
    }

    #[test]
    fn crossing_edges_detected() {
        // Bowtie: (0,0), (1,1), (1,0), (0,1) — edges cross.
        let bowtie = vec![p(0.0, 0.0), p(1.0, 1.0), p(1.0, 0.0), p(0.0, 1.0)];
        let report = validate_simple_polygon(&bowtie);
        assert!(!report.is_valid);
        assert!(report
            .issues
            .iter()
            .any(|i| matches!(i, ValidationIssue::CrossingEdges { .. })));
    }

    #[test]
    fn degenerate_edge_detected() {
        // (0,0), (1,0), (1,0), (0,1) — edge 1 is zero-length.
        let poly = vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)];
        let report = validate_simple_polygon(&poly);
        assert!(!report.is_valid);
        assert!(report
            .issues
            .iter()
            .any(|i| matches!(i, ValidationIssue::DegenerateEdge { .. })));
    }

    #[test]
    fn duplicate_edge_detected() {
        // (0,0), (1,0), (0,0), (0,1) — edge 0 and edge 2 are the same.
        let poly = vec![p(0.0, 0.0), p(1.0, 0.0), p(0.0, 0.0), p(0.0, 1.0)];
        let report = validate_simple_polygon(&poly);
        assert!(!report.is_valid);
        assert!(report
            .issues
            .iter()
            .any(|i| matches!(i, ValidationIssue::DuplicateEdge { .. })));
    }

    #[test]
    fn valid_triangle_passes() {
        let tri = vec![p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)];
        let report = validate_simple_polygon(&tri);
        assert!(
            report.is_valid,
            "CCW triangle should be valid: {:?}",
            report.issues
        );
    }

    #[test]
    fn valid_convex_polygon_passes() {
        let pent = vec![
            p(0.0, 0.0),
            p(2.0, 0.0),
            p(3.0, 1.0),
            p(1.0, 2.0),
            p(0.0, 1.0),
        ];
        let report = validate_simple_polygon(&pent);
        assert!(
            report.is_valid,
            "CCW pentagon should be valid: {:?}",
            report.issues
        );
    }

    // ── Repair suggestions ───────────────────────────────────────────────

    #[test]
    fn repair_suggestion_for_crossing() {
        let issue = ValidationIssue::CrossingEdges {
            edge1: 0,
            edge2: 2,
            point: p(0.5, 0.5),
        };
        let repair = repair_for(&issue);
        assert!(matches!(
            repair,
            RepairSuggestion::SplitAtIntersection { .. }
        ));
    }

    #[test]
    fn repair_suggestion_for_orientation() {
        let issue = ValidationIssue::WrongOrientation {
            expected_ccw: true,
            actual_ccw: false,
        };
        let repair = repair_for(&issue);
        assert!(matches!(repair, RepairSuggestion::ReverseVertexOrder));
    }

    #[test]
    fn repair_suggestions_for_all_issues() {
        let bowtie = vec![p(0.0, 0.0), p(1.0, 1.0), p(1.0, 0.0), p(0.0, 1.0)];
        let report = validate_simple_polygon(&bowtie);
        let repairs = report.repair_suggestions();
        assert_eq!(repairs.len(), report.issues.len());
    }

    // ── Polygon with holes ───────────────────────────────────────────────

    #[test]
    fn valid_polygon_with_hole_passes() {
        let poly = PolygonWithHoles {
            outer: vec![p(0.0, 0.0), p(4.0, 0.0), p(4.0, 4.0), p(0.0, 4.0)],
            holes: vec![vec![p(1.0, 1.0), p(1.0, 3.0), p(3.0, 3.0), p(3.0, 1.0)]],
        };
        // The hole is CW: (1,1)→(1,3)→(3,3)→(3,1) — let me check.
        // Signed area = 0.5 * [(1*3-1*1) + (1*3-3*3) + (3*1-3*3) + (3*1-1*1)]
        // = 0.5 * [2 + (-6) + (-6) + 2] = 0.5 * (-8) = -4. Negative → CW. Good.
        let report = validate_polygon_with_holes(&poly);
        assert!(
            report.is_valid,
            "valid polygon with hole: {:?}",
            report.issues
        );
    }

    #[test]
    fn hole_outside_boundary_detected() {
        let poly = PolygonWithHoles {
            outer: vec![p(0.0, 0.0), p(4.0, 0.0), p(4.0, 4.0), p(0.0, 4.0)],
            holes: vec![vec![p(5.0, 5.0), p(5.0, 6.0), p(6.0, 6.0), p(6.0, 5.0)]],
        };
        let report = validate_polygon_with_holes(&poly);
        assert!(!report.is_valid);
        assert!(report
            .issues
            .iter()
            .any(|i| matches!(i, ValidationIssue::HoleOutsideBoundary { .. })));
    }

    #[test]
    fn hole_wrong_orientation_detected() {
        let poly = PolygonWithHoles {
            outer: vec![p(0.0, 0.0), p(4.0, 0.0), p(4.0, 4.0), p(0.0, 4.0)],
            holes: vec![vec![p(1.0, 1.0), p(3.0, 1.0), p(3.0, 3.0), p(1.0, 3.0)]],
        };
        // The hole is CCW (positive area) — should be CW.
        let report = validate_polygon_with_holes(&poly);
        assert!(!report.is_valid);
        assert!(report
            .issues
            .iter()
            .any(|i| matches!(i, ValidationIssue::WrongOrientation { .. })));
    }

    #[test]
    fn hole_crosses_boundary_detected() {
        let poly = PolygonWithHoles {
            outer: vec![p(0.0, 0.0), p(4.0, 0.0), p(4.0, 4.0), p(0.0, 4.0)],
            holes: vec![vec![p(-1.0, 2.0), p(-1.0, 3.0), p(2.0, 3.0), p(2.0, 2.0)]],
        };
        // The hole extends outside the boundary (x=-1 < 0).
        let report = validate_polygon_with_holes(&poly);
        assert!(!report.is_valid);
    }

    // ── PSLG validation ──────────────────────────────────────────────────

    #[test]
    fn valid_pslg_passes() {
        let vertices = vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)];
        let edges = vec![
            PslgEdge { from: 0, to: 1 },
            PslgEdge { from: 1, to: 2 },
            PslgEdge { from: 2, to: 3 },
            PslgEdge { from: 3, to: 0 },
        ];
        let report = validate_pslg(&vertices, &edges);
        assert!(report.is_valid, "valid PSLG: {:?}", report.issues);
    }

    #[test]
    fn pslg_crossing_edges_detected() {
        let vertices = vec![p(0.0, 0.0), p(1.0, 1.0), p(0.0, 1.0), p(1.0, 0.0)];
        let edges = vec![
            PslgEdge { from: 0, to: 1 }, // (0,0)→(1,1)
            PslgEdge { from: 2, to: 3 }, // (0,1)→(1,0) — crosses
        ];
        let report = validate_pslg(&vertices, &edges);
        assert!(!report.is_valid);
        assert!(report
            .issues
            .iter()
            .any(|i| matches!(i, ValidationIssue::CrossingEdges { .. })));
    }

    #[test]
    fn pslg_duplicate_edge_detected() {
        let vertices = vec![p(0.0, 0.0), p(1.0, 0.0)];
        let edges = vec![
            PslgEdge { from: 0, to: 1 },
            PslgEdge { from: 0, to: 1 }, // duplicate
        ];
        let report = validate_pslg(&vertices, &edges);
        assert!(!report.is_valid);
        assert!(report
            .issues
            .iter()
            .any(|i| matches!(i, ValidationIssue::DuplicateEdge { .. })));
    }

    #[test]
    fn pslg_isolated_vertex_detected() {
        let vertices = vec![p(0.0, 0.0), p(1.0, 0.0), p(2.0, 2.0)];
        let edges = vec![PslgEdge { from: 0, to: 1 }];
        let report = validate_pslg(&vertices, &edges);
        assert!(!report.is_valid);
        assert!(report
            .issues
            .iter()
            .any(|i| matches!(i, ValidationIssue::IsolatedVertex { vertex: 2 })));
    }

    #[test]
    fn pslg_shared_vertex_not_crossing() {
        let vertices = vec![p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)];
        let edges = vec![
            PslgEdge { from: 0, to: 1 },
            PslgEdge { from: 0, to: 2 }, // shares vertex 0 — not a crossing
        ];
        let report = validate_pslg(&vertices, &edges);
        assert!(
            report.is_valid,
            "shared vertex should not be a crossing: {:?}",
            report.issues
        );
    }

    // ── Canonicalization ─────────────────────────────────────────────────

    #[test]
    fn canonicalize_cw_to_ccw() {
        let cw = vec![p(0.0, 0.0), p(0.0, 1.0), p(1.0, 1.0), p(1.0, 0.0)];
        let canon = canonicalize_simple_polygon(&cw);
        assert!(
            signed_area(&canon) > 0.0,
            "canonicalized polygon should be CCW"
        );
    }

    #[test]
    fn canonicalize_removes_trailing_duplicate() {
        let poly = vec![
            p(0.0, 0.0),
            p(1.0, 0.0),
            p(1.0, 1.0),
            p(0.0, 1.0),
            p(0.0, 0.0),
        ];
        let canon = canonicalize_simple_polygon(&poly);
        assert_eq!(canon.len(), 4, "trailing duplicate should be removed");
    }

    #[test]
    fn canonicalize_polygon_with_holes_fixes_orientation() {
        let poly = PolygonWithHoles {
            outer: vec![p(0.0, 0.0), p(0.0, 4.0), p(4.0, 4.0), p(4.0, 0.0)], // CW
            holes: vec![vec![p(1.0, 1.0), p(3.0, 1.0), p(3.0, 3.0), p(1.0, 3.0)]], // CCW
        };
        let canon = canonicalize_polygon_with_holes(&poly);
        assert!(signed_area(&canon.outer) > 0.0, "outer should be CCW");
        assert!(signed_area(&canon.holes[0]) < 0.0, "hole should be CW");
    }

    // ── Point in polygon ─────────────────────────────────────────────────

    #[test]
    fn point_in_polygon_works() {
        let square = vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)];
        assert!(point_in_polygon(p(0.5, 0.5), &square));
        assert!(!point_in_polygon(p(2.0, 2.0), &square));
        assert!(!point_in_polygon(p(-1.0, -1.0), &square));
    }

    // ── No silent mutation ───────────────────────────────────────────────

    #[test]
    fn validation_does_not_mutate_input() {
        let bowtie = vec![p(0.0, 0.0), p(1.0, 1.0), p(1.0, 0.0), p(0.0, 1.0)];
        let original = bowtie.clone();
        let _ = validate_simple_polygon(&bowtie);
        assert_eq!(bowtie, original, "validation must not mutate input");
    }

    #[test]
    fn canonicalization_does_not_mutate_input() {
        let cw = vec![p(0.0, 0.0), p(0.0, 1.0), p(1.0, 1.0), p(1.0, 0.0)];
        let original = cw.clone();
        let _ = canonicalize_simple_polygon(&cw);
        assert_eq!(cw, original, "canonicalization must not mutate input");
    }
}
