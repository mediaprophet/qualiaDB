//! Minkowski sum of two 2-D polygons (P4.8).
//!
//! The Minkowski sum of polygons A and B is the set of all points
//! `{a + b | a ∈ A, b ∈ B}`. For convex polygons, this is equivalent to
//! the convex hull of the pairwise sum of all vertex pairs. For the
//! general case, we compute the convex hull of the pairwise sums.
//!
//! ## Algorithm
//!
//! 1. For all pairs (a_i, b_j) of vertices from A and B, compute a_i + b_j.
//! 2. Compute the convex hull of the resulting point set.
//!
//! For convex inputs, the Minkowski sum is also convex and this gives
//! the exact result. For non-convex inputs, the result is the convex
//! hull of the Minkowski sum (an approximation).
//!
//! ## Determinism
//!
//! The convex hull is deterministic (P4.1). Output is CCW.

use super::hull::convex_hull_indices_2;
use super::primitives::Point2;

/// Minkowski sum error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MinkowskiError {
    /// Polygon has fewer than 1 vertex.
    TooFewVertices { got: usize },
    /// Hull computation failed.
    HullFailed(String),
    /// Output buffer too small.
    OutputTooSmall { required: usize, have: usize },
}

impl core::fmt::Display for MinkowskiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooFewVertices { got } => write!(f, "minkowski: need ≥1 vertex, got {got}"),
            Self::HullFailed(msg) => write!(f, "minkowski: hull failed: {msg}"),
            Self::OutputTooSmall { required, have } => {
                write!(f, "minkowski: output too small, need {required}, have {have}")
            }
        }
    }
}

impl std::error::Error for MinkowskiError {}

/// Compute the Minkowski sum of two convex polygons.
///
/// Returns the number of vertices written to `out`. The output is the
/// convex hull of all pairwise vertex sums, in CCW order.
///
/// `scratch` needs `a.len() * b.len()` entries.
/// `out` needs `a.len() * b.len()` entries (upper bound).
pub fn minkowski_sum_2(
    a: &[Point2],
    b: &[Point2],
    scratch: &mut [u32],
    out: &mut [Point2],
) -> Result<usize, MinkowskiError> {
    if a.is_empty() || b.is_empty() {
        return Err(MinkowskiError::TooFewVertices {
            got: a.len().min(b.len()),
        });
    }

    let na = a.len();
    let nb = b.len();
    let pair_count = na * nb;

    // Compute all pairwise sums.
    let mut sums: Vec<Point2> = Vec::with_capacity(pair_count);
    for va in a {
        for vb in b {
            sums.push(Point2::new(va.x + vb.x, va.y + vb.y));
        }
    }

    // Compute the convex hull of the sums.
    let hull_scratch_size = pair_count * 2;
    if scratch.len() < hull_scratch_size {
        return Err(MinkowskiError::OutputTooSmall {
            required: hull_scratch_size,
            have: scratch.len(),
        });
    }
    if out.len() < pair_count {
        return Err(MinkowskiError::OutputTooSmall {
            required: pair_count,
            have: out.len(),
        });
    }

    let mut hull_indices = vec![0u32; pair_count];
    let hull_count = convex_hull_indices_2(&sums, &mut scratch[..hull_scratch_size], &mut hull_indices)
        .map_err(|e| MinkowskiError::HullFailed(format!("{e:?}")))?;

    // Copy hull vertices to output.
    for i in 0..hull_count {
        out[i] = sums[hull_indices[i] as usize];
    }

    Ok(hull_count)
}

/// Compute the Minkowski difference (A ⊖ B = Minkowski sum of A and -B).
///
/// This is useful for collision detection: A ⊖ B contains the origin
/// iff A and B intersect.
pub fn minkowski_difference_2(
    a: &[Point2],
    b: &[Point2],
    scratch: &mut [u32],
    out: &mut [Point2],
) -> Result<usize, MinkowskiError> {
    let neg_b: Vec<Point2> = b.iter().map(|p| Point2::new(-p.x, -p.y)).collect();
    minkowski_sum_2(a, &neg_b, scratch, out)
}

/// Cross-check: brute-force Minkowski sum as the convex hull of all
/// pairwise point sums. This is the same algorithm but returns the
/// full set of sum points (not just the hull) for verification.
pub fn minkowski_sum_brute_force(a: &[Point2], b: &[Point2]) -> Vec<Point2> {
    let mut sums = Vec::with_capacity(a.len() * b.len());
    for va in a {
        for vb in b {
            sums.push(Point2::new(va.x + vb.x, va.y + vb.y));
        }
    }
    sums
}

// ──────────────────────────────────────────────────────────────────────────
//  Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::boolean_2::polygon_area;

    fn unit_square() -> Vec<Point2> {
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ]
    }

    #[test]
    fn sum_of_two_unit_squares() {
        let a = unit_square();
        let b = unit_square();
        let pair_count = a.len() * b.len();
        let mut scratch = vec![0u32; pair_count * 2];
        let mut out = vec![Point2::new(0.0, 0.0); pair_count];
        let count = minkowski_sum_2(&a, &b, &mut scratch, &mut out).unwrap();
        // Sum of two unit squares is a 2×2 square (4 vertices).
        assert_eq!(count, 4);
        let area = polygon_area(&out[..count]);
        assert!((area - 4.0).abs() < 1e-9, "area should be 4.0, got {area}");
    }

    #[test]
    fn sum_with_point() {
        let a = unit_square();
        let b = vec![Point2::new(1.0, 2.0)];
        let pair_count = a.len() * b.len();
        let mut scratch = vec![0u32; pair_count * 2];
        let mut out = vec![Point2::new(0.0, 0.0); pair_count];
        let count = minkowski_sum_2(&a, &b, &mut scratch, &mut out).unwrap();
        // Sum with a point is a translation — same shape, 4 vertices.
        assert_eq!(count, 4);
        let area = polygon_area(&out[..count]);
        assert!((area - 1.0).abs() < 1e-9);
        // Check translation: the minimum x should be 1.0, minimum y should be 2.0.
        let min_x = out[..count].iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let min_y = out[..count].iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        assert!((min_x - 1.0).abs() < 1e-9);
        assert!((min_y - 2.0).abs() < 1e-9);
    }

    #[test]
    fn sum_of_triangles() {
        let a = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        ];
        let b = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        ];
        let pair_count = a.len() * b.len();
        let mut scratch = vec![0u32; pair_count * 2];
        let mut out = vec![Point2::new(0.0, 0.0); pair_count];
        let count = minkowski_sum_2(&a, &b, &mut scratch, &mut out).unwrap();
        assert!(count >= 3);
        // The Minkowski sum of two right triangles is a larger shape.
        // Verify it's convex and has positive area.
        let area = polygon_area(&out[..count]);
        assert!(area > 0.0);
    }

    #[test]
    fn cross_check_with_brute_force_hull() {
        let a = unit_square();
        let b = unit_square();
        let pair_count = a.len() * b.len();
        let mut scratch = vec![0u32; pair_count * 2];
        let mut out = vec![Point2::new(0.0, 0.0); pair_count];
        let count = minkowski_sum_2(&a, &b, &mut scratch, &mut out).unwrap();

        // Brute-force: compute all sums, then check that the hull matches.
        let brute = minkowski_sum_brute_force(&a, &b);
        let mut bf_scratch = vec![0u32; brute.len() * 2];
        let mut bf_hull = vec![0u32; brute.len()];
        let bf_count = convex_hull_indices_2(&brute, &mut bf_scratch, &mut bf_hull).unwrap();

        assert_eq!(count, bf_count);
        for i in 0..count {
            let hull_pt = out[i];
            let bf_pt = brute[bf_hull[i] as usize];
            assert!((hull_pt.x - bf_pt.x).abs() < 1e-12);
            assert!((hull_pt.y - bf_pt.y).abs() < 1e-12);
        }
    }

    #[test]
    fn determinism_same_input_same_output() {
        let a = unit_square();
        let b = unit_square();
        let pair_count = a.len() * b.len();

        let (count1, out1) = {
            let mut s = vec![0u32; pair_count * 2];
            let mut o = vec![Point2::new(0.0, 0.0); pair_count];
            let c = minkowski_sum_2(&a, &b, &mut s, &mut o).unwrap();
            (c, o[..c].to_vec())
        };

        let (count2, out2) = {
            let mut s = vec![0u32; pair_count * 2];
            let mut o = vec![Point2::new(0.0, 0.0); pair_count];
            let c = minkowski_sum_2(&a, &b, &mut s, &mut o).unwrap();
            (c, o[..c].to_vec())
        };

        assert_eq!(count1, count2);
        assert_eq!(out1, out2);
    }

    #[test]
    fn collinear_edges_handled() {
        // Polygon with collinear edges.
        let a = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(2.0, 0.0), // collinear
            Point2::new(2.0, 1.0),
            Point2::new(0.0, 1.0),
        ];
        let b = vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0), Point2::new(0.0, 1.0)];
        let pair_count = a.len() * b.len();
        let mut scratch = vec![0u32; pair_count * 2];
        let mut out = vec![Point2::new(0.0, 0.0); pair_count];
        let result = minkowski_sum_2(&a, &b, &mut scratch, &mut out);
        assert!(result.is_ok());
        let count = result.unwrap();
        assert!(count >= 3);
    }

    #[test]
    fn empty_polygon_errors() {
        let a: Vec<Point2> = vec![];
        let b = unit_square();
        let mut scratch = vec![0u32; 10];
        let mut out = vec![Point2::new(0.0, 0.0); 10];
        let result = minkowski_sum_2(&a, &b, &mut scratch, &mut out);
        assert!(result.is_err());
    }

    #[test]
    fn reflex_polygon_sum() {
        // L-shaped (reflex) polygon.
        let a = vec![
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(2.0, 1.0),
            Point2::new(1.0, 1.0),
            Point2::new(1.0, 2.0),
            Point2::new(0.0, 2.0),
        ];
        let b = vec![Point2::new(0.0, 0.0), Point2::new(0.5, 0.0), Point2::new(0.0, 0.5)];
        let pair_count = a.len() * b.len();
        let mut scratch = vec![0u32; pair_count * 2];
        let mut out = vec![Point2::new(0.0, 0.0); pair_count];
        let count = minkowski_sum_2(&a, &b, &mut scratch, &mut out).unwrap();
        assert!(count >= 3);
        let area = polygon_area(&out[..count]);
        assert!(area > 0.0);
    }
}
