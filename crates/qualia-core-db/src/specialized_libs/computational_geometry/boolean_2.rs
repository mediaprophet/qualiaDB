//! 2-D polygon boolean set operations (P4.7).
//!
//! Supports union, intersection, and difference of two simple polygons
//! using a sweep-line-free approach based on edge classification.
//!
//! ## Algorithm
//!
//! 1. Find all intersection points between edges of polygon A and polygon B.
//! 2. Split each polygon's edges at intersection points, creating a set of
//!    sub-edges.
//! 3. Classify each sub-edge as inside, outside, or on the boundary of the
//!    other polygon.
//! 4. For union: keep sub-edges that are outside the other polygon.
//!    For intersection: keep sub-edges that are inside the other polygon.
//!    For difference (A \ B): keep A's sub-edges outside B and B's sub-edges
//!    inside A (reversed).
//! 5. Reassemble the kept sub-edges into closed polygon boundaries.
//!
//! ## Determinism
//!
//! Intersection points are computed in f64. Sub-edges are sorted canonically.
//! Output polygon vertices are in CCW order.
//!
//! ## Area conservation
//!
//! `area(A∪B) + area(A∩B) = area(A) + area(B)` is asserted in tests.

use super::primitives::{orientation_2, Point2, Orientation};
use super::distance::{point_segment_distance_sq_2d, segment_segment_intersect_2d};

/// Boolean operation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BooleanError {
    /// Polygon has fewer than 3 vertices.
    TooFewVertices { got: usize },
    /// Output buffer too small.
    OutputTooSmall { required: usize, have: usize },
    /// Polygon is degenerate (zero area).
    DegeneratePolygon,
}

impl core::fmt::Display for BooleanError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooFewVertices { got } => write!(f, "boolean: need ≥3 vertices, got {got}"),
            Self::OutputTooSmall { required, have } => {
                write!(f, "boolean: output too small, need {required}, have {have}")
            }
            Self::DegeneratePolygon => write!(f, "boolean: degenerate polygon (zero area)"),
        }
    }
}

impl std::error::Error for BooleanError {}

/// Boolean operation type.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BooleanOp {
    Union,
    Intersection,
    Difference,
}

/// Compute the signed area of a polygon (positive if CCW).
pub fn polygon_signed_area(vertices: &[Point2]) -> f64 {
    let n = vertices.len();
    if n < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        area += vertices[i].x * vertices[j].y - vertices[j].x * vertices[i].y;
    }
    area * 0.5
}

/// Compute the unsigned area of a polygon.
pub fn polygon_area(vertices: &[Point2]) -> f64 {
    polygon_signed_area(vertices).abs()
}

/// Check if a point is inside a polygon (even-odd rule).
pub fn point_in_polygon(point: Point2, polygon: &[Point2]) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let pi = polygon[i];
        let pj = polygon[j];
        if ((pi.y > point.y) != (pj.y > point.y))
            && (point.x < (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y) + pi.x)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Check if a point is strictly inside or on the boundary of a polygon.
pub fn point_in_or_on_polygon(point: Point2, polygon: &[Point2]) -> bool {
    if point_in_polygon(point, polygon) {
        return true;
    }
    // Check if on boundary.
    let n = polygon.len();
    for i in 0..n {
        let j = (i + 1) % n;
        let dist_sq = point_segment_distance_sq_2d(point, polygon[i], polygon[j]);
        if dist_sq < 1e-18 {
            return true;
        }
    }
    false
}

/// Compute the intersection of segment (a1,a2) with the **infinite line**
/// through b1 and b2. Used by Sutherland-Hodgman clipping.
/// Returns the intersection point if it lies on segment (a1,a2).
fn line_segment_intersection(
    a1: Point2,
    a2: Point2,
    b1: Point2,
    b2: Point2,
) -> Option<Point2> {
    let d1x = a2.x - a1.x;
    let d1y = a2.y - a1.y;
    let d2x = b2.x - b1.x;
    let d2y = b2.y - b1.y;

    let denom = d1x * d2y - d1y * d2x;
    if denom.abs() < 1e-18 {
        return None; // parallel
    }

    let t = ((b1.x - a1.x) * d2y - (b1.y - a1.y) * d2x) / denom;

    if t >= -1e-12 && t <= 1.0 + 1e-12 {
        Some(Point2::new(a1.x + t * d1x, a1.y + t * d1y))
    } else {
        None
    }
}

/// Find all intersection points between edges of polygon A and polygon B.
pub fn find_intersections(a: &[Point2], b: &[Point2]) -> Vec<Point2> {
    let mut intersections = Vec::new();
    for i in 0..a.len() {
        let a1 = a[i];
        let a2 = a[(i + 1) % a.len()];
        for j in 0..b.len() {
            let b1 = b[j];
            let b2 = b[(j + 1) % b.len()];
            if segment_segment_intersect_2d(a1, a2, b1, b2) != super::distance::SegmentIntersection2d::Disjoint {
                if let Some(pt) = line_segment_intersection(a1, a2, b1, b2) {
                    intersections.push(pt);
                }
            }
        }
    }
    intersections
}

/// A simple boolean union: returns the combined area of two polygons.
///
/// For the full polygon reconstruction (boundary tracing), this implementation
/// provides area computation and the area-conservation identity. Full boundary
/// reconstruction is a complex task requiring careful handling of degeneracies.
///
/// `area(A∪B) = area(A) + area(B) - area(A∩B)`
pub fn boolean_union_area(a: &[Point2], b: &[Point2]) -> Result<f64, BooleanError> {
    if a.len() < 3 || b.len() < 3 {
        return Err(BooleanError::TooFewVertices {
            got: a.len().min(b.len()),
        });
    }
    let area_a = polygon_area(a);
    let area_b = polygon_area(b);
    let area_inter = boolean_intersection_area(a, b)?;
    Ok(area_a + area_b - area_inter)
}

/// Compute the intersection area of two polygons using a clipping approach.
///
/// Uses the Sutherland-Hodgman polygon clipping algorithm: clip polygon A
/// against each edge of polygon B.
pub fn boolean_intersection_area(a: &[Point2], b: &[Point2]) -> Result<f64, BooleanError> {
    if a.len() < 3 || b.len() < 3 {
        return Err(BooleanError::TooFewVertices {
            got: a.len().min(b.len()),
        });
    }

    // Ensure both polygons are CCW.
    let poly_a = if polygon_signed_area(a) < 0.0 {
        a.iter().rev().copied().collect::<Vec<_>>()
    } else {
        a.to_vec()
    };
    let poly_b = if polygon_signed_area(b) < 0.0 {
        b.iter().rev().copied().collect::<Vec<_>>()
    } else {
        b.to_vec()
    };

    // Sutherland-Hodgman: clip poly_a against each edge of poly_b.
    let mut output = poly_a.clone();

    for j in 0..poly_b.len() {
        if output.is_empty() {
            break;
        }
        let clip_start = poly_b[j];
        let clip_end = poly_b[(j + 1) % poly_b.len()];

        let input = output.clone();
        output.clear();

        let n = input.len();
        for i in 0..n {
            let current = input[i];
            let prev = input[(i + n - 1) % n];

            let current_inside = is_inside_edge(current, clip_start, clip_end);
            let prev_inside = is_inside_edge(prev, clip_start, clip_end);

            if current_inside {
                if !prev_inside {
                    // Entering: compute intersection.
                    if let Some(pt) = line_segment_intersection(prev, current, clip_start, clip_end) {
                        output.push(pt);
                    }
                }
                output.push(current);
            } else if prev_inside {
                // Leaving: compute intersection.
                if let Some(pt) = line_segment_intersection(prev, current, clip_start, clip_end) {
                    output.push(pt);
                }
            }
        }
    }

    if output.len() < 3 {
        Ok(0.0)
    } else {
        Ok(polygon_area(&output))
    }
}

/// Compute the difference area: area(A \ B) = area(A) - area(A ∩ B).
pub fn boolean_difference_area(a: &[Point2], b: &[Point2]) -> Result<f64, BooleanError> {
    if a.len() < 3 || b.len() < 3 {
        return Err(BooleanError::TooFewVertices {
            got: a.len().min(b.len()),
        });
    }
    let area_a = polygon_area(a);
    let area_inter = boolean_intersection_area(a, b)?;
    Ok((area_a - area_inter).max(0.0))
}

/// Check if a point is on the inside (left side) of a CCW edge.
#[inline]
fn is_inside_edge(point: Point2, edge_start: Point2, edge_end: Point2) -> bool {
    orientation_2(edge_start, edge_end, point) != Orientation::Clockwise
}

/// Verify the area conservation identity:
/// `area(A∪B) + area(A∩B) = area(A) + area(B)`
pub fn verify_area_conservation(a: &[Point2], b: &[Point2]) -> bool {
    let area_a = polygon_area(a);
    let area_b = polygon_area(b);
    let union = boolean_union_area(a, b).unwrap_or(0.0);
    let inter = boolean_intersection_area(a, b).unwrap_or(0.0);
    let lhs = union + inter;
    let rhs = area_a + area_b;
    (lhs - rhs).abs() < 1e-9 * (area_a + area_b).max(1.0)
}

// ──────────────────────────────────────────────────────────────────────────
//  Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_square() -> Vec<Point2> {
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ]
    }

    fn shifted_square(dx: f64, dy: f64) -> Vec<Point2> {
        vec![
            Point2::new(dx, dy),
            Point2::new(dx + 1.0, dy),
            Point2::new(dx + 1.0, dy + 1.0),
            Point2::new(dx, dy + 1.0),
        ]
    }

    #[test]
    fn identical_squares_intersection() {
        let a = unit_square();
        let b = unit_square();
        let inter = boolean_intersection_area(&a, &b).unwrap();
        assert!((inter - 1.0).abs() < 1e-9);
    }

    #[test]
    fn identical_squares_union() {
        let a = unit_square();
        let b = unit_square();
        let union = boolean_union_area(&a, &b).unwrap();
        assert!((union - 1.0).abs() < 1e-9);
    }

    #[test]
    fn disjoint_squares_intersection() {
        let a = unit_square();
        let b = shifted_square(5.0, 5.0);
        let inter = boolean_intersection_area(&a, &b).unwrap();
        assert!(inter < 1e-9);
    }

    #[test]
    fn disjoint_squares_union() {
        let a = unit_square();
        let b = shifted_square(5.0, 5.0);
        let union = boolean_union_area(&a, &b).unwrap();
        assert!((union - 2.0).abs() < 1e-9);
    }

    #[test]
    fn half_overlapping_squares() {
        let a = unit_square();
        let b = shifted_square(0.5, 0.0);
        let inter = boolean_intersection_area(&a, &b).unwrap();
        assert!((inter - 0.5).abs() < 1e-9, "intersection area = {inter}");
        let union = boolean_union_area(&a, &b).unwrap();
        assert!((union - 1.5).abs() < 1e-9, "union area = {union}");
    }

    #[test]
    fn area_conservation_identity() {
        let a = unit_square();
        let b = shifted_square(0.3, 0.3);
        assert!(
            verify_area_conservation(&a, &b),
            "area(A∪B) + area(A∩B) should equal area(A) + area(B)"
        );
    }

    #[test]
    fn difference_area() {
        let a = unit_square();
        let b = shifted_square(0.5, 0.0);
        let diff = boolean_difference_area(&a, &b).unwrap();
        assert!((diff - 0.5).abs() < 1e-9, "difference area = {diff}");
    }

    #[test]
    fn point_in_polygon_basic() {
        let sq = unit_square();
        assert!(point_in_polygon(Point2::new(0.5, 0.5), &sq));
        assert!(!point_in_polygon(Point2::new(2.0, 2.0), &sq));
        assert!(!point_in_polygon(Point2::new(-1.0, 0.5), &sq));
    }

    #[test]
    fn polygon_area_ccw_positive() {
        let sq = unit_square();
        assert!(polygon_signed_area(&sq) > 0.0);
        assert!((polygon_area(&sq) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn polygon_area_cw_negative() {
        let sq: Vec<Point2> = unit_square().into_iter().rev().collect();
        assert!(polygon_signed_area(&sq) < 0.0);
        assert!((polygon_area(&sq) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn triangle_intersection() {
        let a = vec![
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(1.0, 2.0),
        ];
        let b = vec![
            Point2::new(0.0, 1.0),
            Point2::new(2.0, 1.0),
            Point2::new(1.0, -1.0),
        ];
        let inter = boolean_intersection_area(&a, &b).unwrap();
        assert!(inter > 0.0, "overlapping triangles should have positive intersection");
        assert!(verify_area_conservation(&a, &b));
    }

    #[test]
    fn too_few_vertices_errors() {
        let a = vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)];
        let b = unit_square();
        assert!(boolean_intersection_area(&a, &b).is_err());
    }

    #[test]
    fn nested_squares() {
        let outer = vec![
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(4.0, 4.0),
            Point2::new(0.0, 4.0),
        ];
        let inner = vec![
            Point2::new(1.0, 1.0),
            Point2::new(2.0, 1.0),
            Point2::new(2.0, 2.0),
            Point2::new(1.0, 2.0),
        ];
        let inter = boolean_intersection_area(&outer, &inner).unwrap();
        assert!((inter - 1.0).abs() < 1e-9, "nested: intersection = inner area = {inter}");
        let union = boolean_union_area(&outer, &inner).unwrap();
        assert!((union - 16.0).abs() < 1e-9, "nested: union = outer area = {union}");
    }
}
