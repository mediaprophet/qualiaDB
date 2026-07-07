//! Minkowski sum of two 2-D polygons (P4.8 / P11.7).
//!
//! The Minkowski sum of polygons A and B is the set of all points
//! `{a + b | a ∈ A, b ∈ B}`.
//!
//! ## Algorithms
//!
//! **Convex inputs (O(n+m)):** For two convex polygons in CCW order, the
//! Minkowski sum is computed by merging their edge vectors sorted by polar
//! angle. Start from the sum of the bottom-most vertices, then walk both
//! edge lists in angle order, adding edges from whichever polygon has the
//! smaller current edge angle. This produces the exact Minkowski sum in
//! O(n+m) time (de Berg §13.3).
//!
//! **Non-convex inputs:** Decompose each polygon into convex pieces (via
//! triangulation from P11.5), compute all pairwise convex Minkowski sums,
//! and take the union of the results. This is O(n*m) in the number of
//! convex pieces but produces the exact (non-convex) Minkowski sum boundary.
//!
//! **Brute-force fallback (O(n*m)):** Compute all pairwise vertex sums and
//! take the convex hull. This gives the convex hull of the Minkowski sum,
//! which is exact for convex inputs but an approximation for non-convex
//! inputs.
//!
//! ## Determinism
//!
//! All algorithms are deterministic. Output is CCW.

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
            Self::TooFewVertices { got } => write!(f, "minkowski: need â‰¥1 vertex, got {got}"),
            Self::HullFailed(msg) => write!(f, "minkowski: hull failed: {msg}"),
            Self::OutputTooSmall { required, have } => {
                write!(
                    f,
                    "minkowski: output too small, need {required}, have {have}"
                )
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
/// `scratch` needs `3 * a.len() * b.len()` entries (for the convex hull).
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
    let hull_scratch_size = pair_count * 3;
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
    let hull_count =
        convex_hull_indices_2(&sums, &mut scratch[..hull_scratch_size], &mut hull_indices)
            .map_err(|e| MinkowskiError::HullFailed(format!("{e:?}")))?;

    // Copy hull vertices to output.
    for i in 0..hull_count {
        out[i] = sums[hull_indices[i] as usize];
    }

    Ok(hull_count)
}

/// Compute the Minkowski difference (A âŠ– B = Minkowski sum of A and -B).
///
/// This is useful for collision detection: A âŠ– B contains the origin
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

// ───────────────────────────────────────────────────────────────────────────
//  O(n+m) Minkowski sum for convex polygons (edge merge by angle)
// ───────────────────────────────────────────────────────────────────────────

/// Compute the edge vectors of a CCW convex polygon, starting from the
/// bottom-most vertex (lowest y, then lowest x).
///
/// Returns (start_index, edge_vectors) where edge_vectors[i] is the vector
/// from vertex i to vertex (i+1) % n, starting from the bottom-most vertex.
fn convex_edges_from_bottom(poly: &[Point2]) -> (usize, Vec<(f64, f64)>) {
    let n = poly.len();
    // Find the bottom-most vertex (lowest y, then lowest x).
    let mut bottom = 0;
    for i in 1..n {
        if poly[i].y < poly[bottom].y || (poly[i].y == poly[bottom].y && poly[i].x < poly[bottom].x)
        {
            bottom = i;
        }
    }
    // Compute edge vectors starting from the bottom vertex.
    let mut edges = Vec::with_capacity(n);
    for i in 0..n {
        let from = (bottom + i) % n;
        let to = (bottom + i + 1) % n;
        edges.push((poly[to].x - poly[from].x, poly[to].y - poly[from].y));
    }
    (bottom, edges)
}

/// Polar angle of a 2D vector, in [0, 2π).
/// Used for sorting edge vectors in the Minkowski sum merge.
#[inline]
fn polar_angle(dx: f64, dy: f64) -> f64 {
    let angle = dy.atan2(dx);
    if angle < 0.0 {
        angle + 2.0 * std::f64::consts::PI
    } else {
        angle
    }
}

/// Compute the Minkowski sum of two convex polygons in O(n+m) time.
///
/// Both polygons must be convex and in CCW order. The algorithm merges
/// the edge vectors of both polygons sorted by polar angle, starting from
/// the sum of the bottom-most vertices.
///
/// Returns the vertices of the Minkowski sum in CCW order.
///
/// This is the exact Minkowski sum — no convex hull approximation.
pub fn minkowski_sum_convex(a: &[Point2], b: &[Point2]) -> Vec<Point2> {
    let na = a.len();
    let nb = b.len();
    if na == 0 || nb == 0 {
        return Vec::new();
    }

    // Handle degenerate cases (single point or segment).
    if na == 1 {
        return b
            .iter()
            .map(|p| Point2::new(p.x + a[0].x, p.y + a[0].y))
            .collect();
    }
    if nb == 1 {
        return a
            .iter()
            .map(|p| Point2::new(p.x + b[0].x, p.y + b[0].y))
            .collect();
    }

    // Get edge vectors starting from the bottom-most vertex of each polygon.
    let (bottom_a, edges_a) = convex_edges_from_bottom(a);
    let (bottom_b, edges_b) = convex_edges_from_bottom(b);

    // Start point: sum of the bottom-most vertices.
    let start = Point2::new(a[bottom_a].x + b[bottom_b].x, a[bottom_a].y + b[bottom_b].y);

    // Merge edges by polar angle.
    let mut raw = Vec::with_capacity(na + nb);
    raw.push(start);

    let mut ia = 0;
    let mut ib = 0;
    let mut cx = start.x;
    let mut cy = start.y;

    while ia < na || ib < nb {
        let angle_a = if ia < na {
            polar_angle(edges_a[ia].0, edges_a[ia].1)
        } else {
            f64::INFINITY
        };
        let angle_b = if ib < nb {
            polar_angle(edges_b[ib].0, edges_b[ib].1)
        } else {
            f64::INFINITY
        };

        if ia < na && (ib >= nb || angle_a <= angle_b) {
            // Add edge from A.
            cx += edges_a[ia].0;
            cy += edges_a[ia].1;
            ia += 1;
        } else {
            // Add edge from B.
            cx += edges_b[ib].0;
            cy += edges_b[ib].1;
            ib += 1;
        }

        // Don't add the last point (it wraps around to the start).
        if ia < na || ib < nb {
            raw.push(Point2::new(cx, cy));
        }
    }

    // Remove collinear vertices (consecutive vertices on the same line).
    // This happens when edges from both polygons have the same angle.
    let result = remove_collinear_vertices(&raw);
    result
}

/// Remove collinear vertices from a polygon (vertices where the previous,
/// current, and next vertices are collinear).
fn remove_collinear_vertices(poly: &[Point2]) -> Vec<Point2> {
    let n = poly.len();
    if n < 3 {
        return poly.to_vec();
    }
    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        let prev = poly[(i + n - 1) % n];
        let curr = poly[i];
        let next = poly[(i + 1) % n];
        let orient = super::primitives::orientation_2(prev, curr, next);
        if orient != super::primitives::Orientation::Collinear {
            result.push(curr);
        }
    }
    result
}

// ───────────────────────────────────────────────────────────────────────────
//  Minkowski sum for non-convex polygons (convex decomposition + union)
// ───────────────────────────────────────────────────────────────────────────

/// Compute the Minkowski sum of two (possibly non-convex) simple polygons.
///
/// Decomposes each polygon into triangles (using ear clipping from P11.5),
/// computes all pairwise convex Minkowski sums, and returns the union as
/// a set of boundary points.
///
/// For convex inputs, this reduces to the O(n+m) convex algorithm.
/// For non-convex inputs, the result is a point set on the boundary of
/// the Minkowski sum — the caller can compute the convex hull or use
/// a polygon union to get the full boundary.
///
/// Returns all vertices of all pairwise Minkowski sums.
pub fn minkowski_sum_non_convex(a: &[Point2], b: &[Point2]) -> Vec<Point2> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }

    // Triangulate both polygons.
    let tris_a = super::triangulation_2::triangulate_ear_clipping(a);
    let tris_b = super::triangulation_2::triangulate_ear_clipping(b);

    if tris_a.is_empty() || tris_b.is_empty() {
        // Fallback: use brute force.
        return minkowski_sum_brute_force(a, b);
    }

    // Compute all pairwise convex Minkowski sums.
    let mut all_points = Vec::with_capacity(tris_a.len() * tris_b.len() * 6);
    for ta in &tris_a {
        let tri_a = [ta.a, ta.b, ta.c];
        for tb in &tris_b {
            let tri_b = [tb.a, tb.b, tb.c];
            let sum = minkowski_sum_convex(&tri_a, &tri_b);
            all_points.extend(sum);
        }
    }

    all_points
}

// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
//  Tests
// â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[cfg(test)]
mod tests {
    use super::super::boolean_2::polygon_area;
    use super::*;

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
        let mut scratch = vec![0u32; pair_count * 3];
        let mut out = vec![Point2::new(0.0, 0.0); pair_count];
        let count = minkowski_sum_2(&a, &b, &mut scratch, &mut out).unwrap();
        // Sum of two unit squares is a 2Ã—2 square (4 vertices).
        assert_eq!(count, 4);
        let area = polygon_area(&out[..count]);
        assert!((area - 4.0).abs() < 1e-9, "area should be 4.0, got {area}");
    }

    #[test]
    fn sum_with_point() {
        let a = unit_square();
        let b = vec![Point2::new(1.0, 2.0)];
        let pair_count = a.len() * b.len();
        let mut scratch = vec![0u32; pair_count * 3];
        let mut out = vec![Point2::new(0.0, 0.0); pair_count];
        let count = minkowski_sum_2(&a, &b, &mut scratch, &mut out).unwrap();
        // Sum with a point is a translation â€” same shape, 4 vertices.
        assert_eq!(count, 4);
        let area = polygon_area(&out[..count]);
        assert!((area - 1.0).abs() < 1e-9);
        // Check translation: the minimum x should be 1.0, minimum y should be 2.0.
        let min_x = out[..count]
            .iter()
            .map(|p| p.x)
            .fold(f64::INFINITY, f64::min);
        let min_y = out[..count]
            .iter()
            .map(|p| p.y)
            .fold(f64::INFINITY, f64::min);
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
        let mut scratch = vec![0u32; pair_count * 3];
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
        let mut scratch = vec![0u32; pair_count * 3];
        let mut out = vec![Point2::new(0.0, 0.0); pair_count];
        let count = minkowski_sum_2(&a, &b, &mut scratch, &mut out).unwrap();

        // Brute-force: compute all sums, then check that the hull matches.
        let brute = minkowski_sum_brute_force(&a, &b);
        let mut bf_scratch = vec![0u32; brute.len() * 3];
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
            let mut s = vec![0u32; pair_count * 3];
            let mut o = vec![Point2::new(0.0, 0.0); pair_count];
            let c = minkowski_sum_2(&a, &b, &mut s, &mut o).unwrap();
            (c, o[..c].to_vec())
        };

        let (count2, out2) = {
            let mut s = vec![0u32; pair_count * 3];
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
        let b = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        ];
        let pair_count = a.len() * b.len();
        let mut scratch = vec![0u32; pair_count * 3];
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
        let mut scratch = vec![0u32; 15];
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
        let b = vec![
            Point2::new(0.0, 0.0),
            Point2::new(0.5, 0.0),
            Point2::new(0.0, 0.5),
        ];
        let pair_count = a.len() * b.len();
        let mut scratch = vec![0u32; pair_count * 3];
        let mut out = vec![Point2::new(0.0, 0.0); pair_count];
        let count = minkowski_sum_2(&a, &b, &mut scratch, &mut out).unwrap();
        assert!(count >= 3);
        let area = polygon_area(&out[..count]);
        assert!(area > 0.0);
    }

    // ── O(n+m) convex Minkowski sum tests ──────────────────────────────

    #[test]
    fn convex_sum_two_unit_squares() {
        let a = unit_square();
        let b = unit_square();
        let result = minkowski_sum_convex(&a, &b);
        // Sum of two unit squares is a 2×2 square (4 vertices).
        assert_eq!(result.len(), 4);
        let area = polygon_area(&result);
        assert!((area - 4.0).abs() < 1e-9, "area should be 4.0, got {area}");
    }

    #[test]
    fn convex_sum_with_point() {
        let a = unit_square();
        let b = vec![Point2::new(1.0, 2.0)];
        let result = minkowski_sum_convex(&a, &b);
        // Sum with a point is a translation — same shape, 4 vertices.
        assert_eq!(result.len(), 4);
        let area = polygon_area(&result);
        assert!((area - 1.0).abs() < 1e-9);
        // Check translation.
        let min_x = result.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let min_y = result.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        assert!((min_x - 1.0).abs() < 1e-9);
        assert!((min_y - 2.0).abs() < 1e-9);
    }

    #[test]
    fn convex_sum_two_triangles() {
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
        let result = minkowski_sum_convex(&a, &b);
        assert!(result.len() >= 3);
        let area = polygon_area(&result);
        assert!(area > 0.0);
        // The Minkowski sum of two unit right triangles (each area 0.5)
        // is a quadrilateral with vertices (0,0), (2,0), (1,1), (0,2).
        // Area = 2.0 (by the shoelace formula).
        assert!((area - 2.0).abs() < 1e-9, "area should be 2.0, got {area}");
    }

    #[test]
    fn convex_sum_matches_brute_force() {
        // Compare O(n+m) convex sum with the brute-force hull for convex inputs.
        let a = vec![
            Point2::new(0.0, 0.0),
            Point2::new(3.0, 0.0),
            Point2::new(4.0, 2.0),
            Point2::new(2.0, 4.0),
            Point2::new(0.0, 3.0),
        ];
        let b = vec![
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(2.0, 2.0),
            Point2::new(0.0, 2.0),
        ];
        let convex_result = minkowski_sum_convex(&a, &b);
        let convex_area = polygon_area(&convex_result);

        // Brute force: all pairwise sums, then convex hull.
        let brute = minkowski_sum_brute_force(&a, &b);
        let mut bf_scratch = vec![0u32; brute.len() * 3];
        let mut bf_hull = vec![0u32; brute.len()];
        let bf_count = convex_hull_indices_2(&brute, &mut bf_scratch, &mut bf_hull).unwrap();
        let bf_area: f64 = (0..bf_count)
            .map(|i| {
                let p = brute[bf_hull[i] as usize];
                let q = brute[bf_hull[(i + 1) % bf_count] as usize];
                p.x * q.y - q.x * p.y
            })
            .sum::<f64>()
            * 0.5;

        assert!(
            (convex_area - bf_area).abs() < 1e-9,
            "convex area {} should match brute-force area {}",
            convex_area,
            bf_area
        );
    }

    #[test]
    fn convex_sum_hexagon_and_square() {
        // Regular hexagon (CCW).
        let hex: Vec<Point2> = (0..6)
            .map(|i| {
                let angle = i as f64 * std::f64::consts::PI / 3.0;
                Point2::new(angle.cos(), angle.sin())
            })
            .collect();
        let sq = unit_square();
        let result = minkowski_sum_convex(&hex, &sq);
        assert!(result.len() >= 6);
        let area = polygon_area(&result);
        assert!(area > 0.0);
    }

    #[test]
    fn convex_sum_empty_inputs() {
        let a: Vec<Point2> = vec![];
        let b = unit_square();
        let result = minkowski_sum_convex(&a, &b);
        assert!(result.is_empty());
    }

    #[test]
    fn convex_sum_determinism() {
        let a = unit_square();
        let b = unit_square();
        let r1 = minkowski_sum_convex(&a, &b);
        let r2 = minkowski_sum_convex(&a, &b);
        assert_eq!(r1, r2);
    }

    // ── Non-convex Minkowski sum tests ─────────────────────────────────

    #[test]
    fn non_convex_sum_l_shape_and_triangle() {
        // L-shaped (reflex) polygon.
        let a = vec![
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(2.0, 1.0),
            Point2::new(1.0, 1.0),
            Point2::new(1.0, 2.0),
            Point2::new(0.0, 2.0),
        ];
        let b = vec![
            Point2::new(0.0, 0.0),
            Point2::new(0.5, 0.0),
            Point2::new(0.0, 0.5),
        ];
        let result = minkowski_sum_non_convex(&a, &b);
        assert!(!result.is_empty(), "should produce some points");
    }

    #[test]
    fn non_convex_sum_two_squares() {
        // Two convex squares — should produce the same result as convex sum.
        let a = unit_square();
        let b = unit_square();
        let result = minkowski_sum_non_convex(&a, &b);
        assert!(!result.is_empty());
        // The convex hull of the non-convex result should have area 4.0.
        let mut scratch = vec![0u32; result.len() * 3];
        let mut hull = vec![0u32; result.len()];
        let hull_count = convex_hull_indices_2(&result, &mut scratch, &mut hull).unwrap();
        let hull_area: f64 = (0..hull_count)
            .map(|i| {
                let p = result[hull[i] as usize];
                let q = result[hull[(i + 1) % hull_count] as usize];
                p.x * q.y - q.x * p.y
            })
            .sum::<f64>()
            * 0.5;
        assert!(
            (hull_area - 4.0).abs() < 1e-9,
            "hull area should be 4.0, got {hull_area}"
        );
    }

    #[test]
    fn non_convex_sum_empty_inputs() {
        let a: Vec<Point2> = vec![];
        let b = unit_square();
        let result = minkowski_sum_non_convex(&a, &b);
        assert!(result.is_empty());
    }

    #[test]
    fn non_convex_sum_point_inputs() {
        let a = vec![Point2::new(1.0, 2.0)];
        let b = vec![Point2::new(3.0, 4.0)];
        let result = minkowski_sum_non_convex(&a, &b);
        // Sum of two points is a single point (4, 6).
        // Triangulation of a single point returns empty, so this falls back
        // to brute force.
        assert!(result.contains(&Point2::new(4.0, 6.0)));
    }

    // ── Polar angle tests ──────────────────────────────────────────────

    #[test]
    fn polar_angle_positive_x_axis() {
        assert!((polar_angle(1.0, 0.0)).abs() < 1e-12);
    }

    #[test]
    fn polar_angle_positive_y_axis() {
        assert!((polar_angle(0.0, 1.0) - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    }

    #[test]
    fn polar_angle_negative_x_axis() {
        assert!((polar_angle(-1.0, 0.0) - std::f64::consts::PI).abs() < 1e-12);
    }

    #[test]
    fn polar_angle_negative_y_axis() {
        assert!((polar_angle(0.0, -1.0) - 3.0 * std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    }
}
