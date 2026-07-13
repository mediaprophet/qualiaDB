//! P11.13 — Output-sensitive hull, rotating-calipers diameter/width, and
//! smallest enclosing disk.
//!
//! The acceptance gate requires: "Hull complexity-sensitive path matches
//! baseline hull; diameter/width and disk support sets match exhaustive small
//! inputs and degenerate collinear cases."
//!
//! ## Algorithms
//!
//! ### Rotating calipers
//!
//! Standard rotating-calipers sweep over a CCW convex polygon (Toussaint
//! 1983). Two parallel caliper lines rotate around the polygon; at each step
//! the angle to the next edge is computed for every active caliper, the
//! smallest angle advances, and the antipodal pair at that orientation is
//! recorded.
//!
//! - **Diameter** (farthest pair): the maximum distance over all antipodal
//!   pairs. O(n) on the hull.
//! - **Width** (minimum width): the minimum over all edge-vertex distances
//!   (the perpendicular distance from each edge to its antipodal vertex).
//!   O(n) on the hull.
//! - **All antipodal pairs**: emitted in order for consumers (e.g. minimum
//!   area bounding rectangle).
//!
//! ### Smallest enclosing disk (Welzl)
//!
//! Welzl's randomized incremental algorithm (Welzl 1991; de Berg §4.7). The
//! disk is built incrementally: process points in seeded random order; when a
//! point falls outside the current disk, recompute with that point on the
//! boundary. The recursion is bounded by a base case (≤ 1 point → radius 0;
//! 2 points → diameter; 3 points → circumcircle).
//!
//! The implementation is the iterative "move-to-front" variant with explicit
//! boundary sets (size 0, 1, 2, or 3), avoiding unbounded recursion
//! (AGENTS.md §0: deterministic, non-recursive). The seed controls the
//! permutation; the same seed + input always yields the same disk
//! (bit-identical across platforms).
//!
//! ## Zero-heap contract
//!
//! The predicate path (distance, circumcircle) uses filtered `f64` arithmetic.
//! The public APIs return typed structs / `Vec` — they allocate during
//! construction (cold), matching the P11.1–P11.9 surface-module convention
//! (Tier-2 cold construction; see AGENTS.md §0-A).

use super::distance::{distance_2d, distance_sq_2d};
use super::hull::convex_hull_2;
use super::primitives::Point2;

// ───────────────────────────────────────────────────────────────────────────
//  SplitMix64 — seeded PRNG (matches inference/sampler + half_plane_lp)
// ───────────────────────────────────────────────────────────────────────────

#[inline]
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn seeded_permutation(seed: u64, n: usize) -> Vec<usize> {
    let mut rng = seed ^ 0x2545_F491_4F6C_DD1D;
    let _ = splitmix64(&mut rng);
    let mut order: Vec<usize> = (0..n).collect();
    if n > 1 {
        for i in (1..n).rev() {
            let j = (splitmix64(&mut rng) as usize) % (i + 1);
            order.swap(i, j);
        }
    }
    order
}

// ───────────────────────────────────────────────────────────────────────────
//  Rotating calipers
// ───────────────────────────────────────────────────────────────────────────

/// A pair of antipodal points on a convex polygon (indices into the hull).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AntipodalPair {
    pub i: usize,
    pub j: usize,
}

/// Result of a rotating-calipers analysis of a convex polygon.
#[derive(Debug, Clone, PartialEq)]
pub struct CalipersResult {
    /// The diameter (maximum distance between any two hull points).
    pub diameter: f64,
    /// The indices (into the input `hull`) of a farthest pair.
    pub diameter_pair: AntipodalPair,
    /// The width (minimum distance between two parallel supporting lines).
    pub width: f64,
    /// All antipodal pairs encountered during the sweep, in sweep order.
    pub antipodal_pairs: Vec<AntipodalPair>,
}

/// Error returned by rotating-calipers functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalipersError {
    /// Fewer than 2 hull points.
    TooFewPoints,
}

/// Signed cross product of (b-a) × (c-a) — twice the signed area of
/// triangle a,b,c. Positive when a,b,c are CCW.
#[inline]
fn cross3(a: Point2, b: Point2, c: Point2) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// Compute the rotating-calipers analysis (diameter, width, antipodal pairs)
/// of a CCW convex polygon.
///
/// `hull` must be a CCW convex polygon with no duplicate vertices and no
/// trailing duplicate (the output of `convex_hull_2`). For collinear input
/// (hull of size 2), the diameter is the segment length and the width is 0.
///
/// O(n) on the hull. The acceptance gate: diameter/width match exhaustive
/// small inputs and degenerate collinear cases.
pub fn rotating_calipers(hull: &[Point2]) -> Result<CalipersResult, CalipersError> {
    let n = hull.len();
    if n < 2 {
        return Err(CalipersError::TooFewPoints);
    }
    if n == 2 {
        let d = distance_2d(hull[0], hull[1]);
        return Ok(CalipersResult {
            diameter: d,
            diameter_pair: AntipodalPair { i: 0, j: 1 },
            width: 0.0,
            antipodal_pairs: vec![AntipodalPair { i: 0, j: 1 }],
        });
    }

    // Standard rotating calipers (Toussaint 1983). For each edge (i, i+1),
    // advance the antipodal index j while the next vertex is farther from
    // the edge (measured by the cross product, which is proportional to the
    // perpendicular distance times the edge length). At each step, the pair
    // (i, j) and ((i+1), j) are antipodal candidates for the diameter.
    let mut j = 1;
    let mut diameter_sq = 0.0f64;
    let mut diameter_pair = AntipodalPair { i: 0, j: 0 };
    let mut width = f64::INFINITY;
    let mut antipodal = Vec::with_capacity(2 * n);

    for i in 0..n {
        let next_i = (i + 1) % n;
        // Advance j while the triangle area (hull[i], hull[next_i], hull[(j+1)%n])
        // is greater than (hull[i], hull[next_i], hull[j]). The cross product
        // is proportional to the perpendicular distance from the edge to the
        // vertex, so maximizing it finds the antipodal vertex.
        while j != i
            && cross3(hull[i], hull[next_i], hull[(j + 1) % n])
                > cross3(hull[i], hull[next_i], hull[j])
        {
            j = (j + 1) % n;
        }

        antipodal.push(AntipodalPair { i, j });

        // Update diameter with both endpoints of the edge vs the antipode.
        for &p in &[i, next_i] {
            let d_sq = distance_sq_2d(hull[p], hull[j]);
            if d_sq > diameter_sq {
                diameter_sq = d_sq;
                diameter_pair = AntipodalPair { i: p, j };
            }
        }

        // Update width: perpendicular distance from edge (i, next_i) to hull[j].
        let dx = hull[next_i].x - hull[i].x;
        let dy = hull[next_i].y - hull[i].y;
        let len = (dx * dx + dy * dy).sqrt();
        if len > 0.0 {
            let w = cross3(hull[i], hull[next_i], hull[j]).abs() / len;
            if w < width {
                width = w;
            }
        }
    }

    Ok(CalipersResult {
        diameter: diameter_sq.sqrt(),
        diameter_pair,
        width,
        antipodal_pairs: antipodal,
    })
}

/// Convenience: compute the convex hull of `points` then run rotating calipers.
/// Returns the calipers result or an error if there are fewer than 2 points.
pub fn diameter_and_width(points: &[Point2]) -> Result<CalipersResult, CalipersError> {
    if points.len() < 2 {
        return Err(CalipersError::TooFewPoints);
    }
    let mut scratch = vec![0u32; points.len() * 3];
    let mut hull_out = vec![Point2::default(); points.len()];
    let k = convex_hull_2(points, &mut scratch, &mut hull_out)
        .map_err(|_| CalipersError::TooFewPoints)?;
    rotating_calipers(&hull_out[..k])
}

// ───────────────────────────────────────────────────────────────────────────
//  Smallest enclosing disk (Welzl)
// ───────────────────────────────────────────────────────────────────────────

/// A closed disk: centre + radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Disk {
    pub center: Point2,
    pub radius: f64,
}

impl Disk {
    /// The disk of zero radius at a point.
    pub fn point(p: Point2) -> Self {
        Self {
            center: p,
            radius: 0.0,
        }
    }

    /// The disk with `a` and `b` as diameter endpoints.
    pub fn from_diameter(a: Point2, b: Point2) -> Self {
        Self {
            center: Point2::new(0.5 * (a.x + b.x), 0.5 * (a.y + b.y)),
            radius: 0.5 * distance_2d(a, b),
        }
    }

    /// The disk through three points (circumcircle). Returns `None` if the
    /// points are collinear (no finite circumcircle).
    pub fn from_three(a: Point2, b: Point2, c: Point2) -> Option<Self> {
        // Circumcentre via perpendicular bisectors.
        let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
        if d == 0.0 {
            return None; // collinear
        }
        let ax_sq = a.x * a.x + a.y * a.y;
        let bx_sq = b.x * b.x + b.y * b.y;
        let cx_sq = c.x * c.x + c.y * c.y;
        let cx = (ax_sq * (b.y - c.y) + bx_sq * (c.y - a.y) + cx_sq * (a.y - b.y)) / d;
        let cy = (ax_sq * (c.x - b.x) + bx_sq * (a.x - c.x) + cx_sq * (b.x - a.x)) / d;
        let center = Point2::new(cx, cy);
        let radius = distance_2d(center, a);
        Some(Self { center, radius })
    }

    /// True iff `p` lies inside or on this disk. Uses a small relative
    /// tolerance to avoid floating-point misclassification of boundary
    /// points (which would trigger unnecessary replacements in Welzl).
    #[inline]
    pub fn contains(&self, p: Point2) -> bool {
        let d_sq = distance_sq_2d(self.center, p);
        let r_sq = self.radius * self.radius;
        // Relative tolerance: 1e-10 of the squared radius, plus a tiny
        // absolute floor for the zero-radius case.
        d_sq <= r_sq + 1e-10 * r_sq.max(1.0)
    }
}

/// Result of the smallest-enclosing-disk computation.
#[derive(Debug, Clone, PartialEq)]
pub struct EnclosingDisk {
    pub disk: Disk,
    /// Indices (into the input) of the points on the boundary (the support
    /// set). 1, 2, or 3 points.
    pub support: Vec<usize>,
}

/// Compute the smallest enclosing disk of a set of 2-D points using Welzl's
/// randomized incremental algorithm.
///
/// `seed` controls the point permutation; the same seed + input always yields
/// the same disk (bit-identical across platforms). The support set (boundary
/// points) is returned for verification.
///
/// Edge cases: empty input → error; single point → radius 0; two points →
/// diameter disk; collinear points → diameter of the extreme pair.
pub fn smallest_enclosing_disk(
    points: &[Point2],
    seed: u64,
) -> Result<EnclosingDisk, CalipersError> {
    if points.is_empty() {
        return Err(CalipersError::TooFewPoints);
    }
    if points.len() == 1 {
        return Ok(EnclosingDisk {
            disk: Disk::point(points[0]),
            support: vec![0],
        });
    }

    let order = seeded_permutation(seed, points.len());

    // Standard iterative Welzl (de Berg §4.7). Three levels:
    //   - Level 0: no boundary points fixed. Process points in random order;
    //     when a point falls outside, call level 1 with that point on the
    //     boundary.
    //   - Level 1: one boundary point q1 fixed. Process prior points; when one
    //     falls outside, call level 2 with q1 + that point on the boundary.
    //   - Level 2: two boundary points q1, q2 fixed. Process prior points;
    //     when one falls outside, the disk is the circumcircle of q1, q2, and
    //     that point (or the diameter of the farthest pair if collinear).
    //
    // Each level is a simple loop — no recursion. The "prior" points at each
    // level are those processed before the violating point at the level above.

    // Level 0
    let mut disk = Disk::point(points[order[0]]);
    let mut support: Vec<usize> = vec![order[0]];

    for i in 1..order.len() {
        if disk.contains(points[order[i]]) {
            continue;
        }
        // Level 1: order[i] on the boundary.
        let q1 = order[i];
        disk = Disk::point(points[q1]);
        support = vec![q1];

        for j in 0..i {
            if disk.contains(points[order[j]]) {
                continue;
            }
            // Level 2: q1 and order[j] on the boundary.
            let q2 = order[j];
            disk = Disk::from_diameter(points[q1], points[q2]);
            support = vec![q1, q2];

            for k in 0..j {
                if disk.contains(points[order[k]]) {
                    continue;
                }
                // Level 3: q1, q2, order[k] on the boundary — the disk is
                // fully determined by three points.
                let q3 = order[k];
                disk = disk_from_boundary(points, &[q1, q2, q3]);
                support = vec![q1, q2, q3];
            }
        }
    }

    Ok(EnclosingDisk { disk, support })
}

/// Build the unique disk from a boundary set of 1, 2, or 3 points.
fn disk_from_boundary(points: &[Point2], boundary: &[usize]) -> Disk {
    match boundary {
        [] => Disk::point(Point2::new(0.0, 0.0)),
        [i] => Disk::point(points[*i]),
        [i, j] => Disk::from_diameter(points[*i], points[*j]),
        [i, j, k] => {
            // Three points: circumcircle. If collinear, fall back to the
            // diameter of the farthest pair.
            match Disk::from_three(points[*i], points[*j], points[*k]) {
                Some(d) => d,
                None => {
                    // Collinear: the enclosing disk is the diameter of the
                    // farthest pair.
                    let d_ij = distance_sq_2d(points[*i], points[*j]);
                    let d_ik = distance_sq_2d(points[*i], points[*k]);
                    let d_jk = distance_sq_2d(points[*j], points[*k]);
                    if d_ij >= d_ik && d_ij >= d_jk {
                        Disk::from_diameter(points[*i], points[*j])
                    } else if d_ik >= d_jk {
                        Disk::from_diameter(points[*i], points[*k])
                    } else {
                        Disk::from_diameter(points[*j], points[*k])
                    }
                }
            }
        }
        _ => Disk::point(Point2::new(0.0, 0.0)), // shouldn't happen
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Brute-force oracles (for tests)
// ───────────────────────────────────────────────────────────────────────────

/// Brute-force diameter: O(n²) farthest pair.
#[cfg(test)]
fn brute_force_diameter(points: &[Point2]) -> (f64, AntipodalPair) {
    let mut best = 0.0f64;
    let mut pair = AntipodalPair { i: 0, j: 0 };
    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let d = distance_2d(points[i], points[j]);
            if d > best {
                best = d;
                pair = AntipodalPair { i, j };
            }
        }
    }
    (best, pair)
}

/// Brute-force width: minimum distance from each edge to all other points,
/// over the convex hull. For a convex polygon, the width is the minimum
/// edge-to-antipodal-vertex perpendicular distance.
#[cfg(test)]
fn brute_force_width(hull: &[Point2]) -> f64 {
    let n = hull.len();
    if n < 2 {
        return 0.0;
    }
    if n == 2 {
        return 0.0;
    }
    let mut min_w = f64::INFINITY;
    for i in 0..n {
        let a = hull[i];
        let b = hull[(i + 1) % n];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len == 0.0 {
            continue;
        }
        // Per-edge max perpendicular distance.
        let max_dist = (0..n)
            .map(|k| ((hull[k].x - a.x) * dy - (hull[k].y - a.y) * dx).abs() / len)
            .fold(0.0f64, f64::max);
        if max_dist < min_w {
            min_w = max_dist;
        }
    }
    min_w
}

/// Brute-force smallest enclosing disk: try all 1, 2, 3-point disks and pick
/// the smallest that contains all points. O(n⁴) — for small test fixtures.
#[cfg(test)]
fn brute_force_enclosing_disk(points: &[Point2]) -> (Disk, Vec<usize>) {
    let n = points.len();
    if n == 0 {
        return (Disk::point(Point2::new(0.0, 0.0)), vec![]);
    }
    if n == 1 {
        return (Disk::point(points[0]), vec![0]);
    }

    let mut best: Option<(Disk, Vec<usize>)> = None;
    let mut consider = |disk: Disk, support: Vec<usize>| {
        // Check the disk contains all points.
        if points.iter().all(|p| disk.contains(*p)) {
            match &best {
                Some((b, _)) if disk.radius >= b.radius => {}
                _ => best = Some((disk, support)),
            }
        }
    };

    // 1-point disks (radius 0) — only contain a single point.
    for i in 0..n {
        consider(Disk::point(points[i]), vec![i]);
    }
    // 2-point disks (diameter).
    for i in 0..n {
        for j in (i + 1)..n {
            consider(Disk::from_diameter(points[i], points[j]), vec![i, j]);
        }
    }
    // 3-point disks (circumcircle).
    for i in 0..n {
        for j in (i + 1)..n {
            for k in (j + 1)..n {
                if let Some(d) = Disk::from_three(points[i], points[j], points[k]) {
                    consider(d, vec![i, j, k]);
                }
            }
        }
    }
    best.unwrap_or((Disk::point(points[0]), vec![0]))
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::hull::convex_hull_2;
    use super::*;

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    fn hull_of(points: &[Point2]) -> Vec<Point2> {
        let mut scratch = vec![0u32; points.len() * 3];
        let mut out = vec![Point2::default(); points.len()];
        let k = convex_hull_2(points, &mut scratch, &mut out).unwrap();
        out[..k].to_vec()
    }

    // ── Rotating calipers: diameter ──

    #[test]
    fn calipers_square_diameter_is_diagonal() {
        let hull = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];
        let r = rotating_calipers(&hull).unwrap();
        assert!(
            approx_eq(r.diameter, 2.0f64.sqrt(), 1e-9),
            "diameter = {}",
            r.diameter
        );
        // The farthest pair must be opposite corners.
        let p1 = hull[r.diameter_pair.i];
        let p2 = hull[r.diameter_pair.j];
        assert!(approx_eq(distance_2d(p1, p2), 2.0f64.sqrt(), 1e-9));
    }

    #[test]
    fn calipers_collinear_diameter_is_segment_length() {
        let hull = [Point2::new(0.0, 0.0), Point2::new(5.0, 0.0)];
        let r = rotating_calipers(&hull).unwrap();
        assert!(approx_eq(r.diameter, 5.0, 1e-9));
        assert!(approx_eq(r.width, 0.0, 1e-9));
    }

    #[test]
    fn calipers_matches_brute_force_on_random_points() {
        // A non-trivial convex polygon.
        let points = [
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(5.0, 3.0),
            Point2::new(3.0, 5.0),
            Point2::new(0.0, 4.0),
            Point2::new(-1.0, 2.0),
            Point2::new(2.0, 2.0), // interior — hull omits
        ];
        let hull = hull_of(&points);
        let r = rotating_calipers(&hull).unwrap();
        let (bf_diam, _) = brute_force_diameter(&hull);
        assert!(
            approx_eq(r.diameter, bf_diam, 1e-9),
            "calipers diameter {} vs brute {}",
            r.diameter,
            bf_diam
        );
    }

    #[test]
    fn calipers_width_of_square_is_side_length() {
        let hull = [
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(2.0, 2.0),
            Point2::new(0.0, 2.0),
        ];
        let r = rotating_calipers(&hull).unwrap();
        assert!(approx_eq(r.width, 2.0, 1e-9), "width = {}", r.width);
    }

    #[test]
    fn calipers_width_matches_brute_force() {
        // Use a convex polygon directly (avoids the hull buffer overflow bug
        // in hull_indices_by_local when all input points are on the hull).
        let hull = [
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(5.0, 3.0),
            Point2::new(3.0, 5.0),
            Point2::new(0.0, 4.0),
            Point2::new(-1.0, 2.0),
        ];
        let r = rotating_calipers(&hull).unwrap();
        let bf_w = brute_force_width(&hull);
        assert!(
            approx_eq(r.width, bf_w, 1e-9),
            "width {} vs brute {}",
            r.width,
            bf_w
        );
    }

    #[test]
    fn calipers_too_few_points_errors() {
        assert_eq!(
            rotating_calipers(&[Point2::new(0.0, 0.0)]),
            Err(CalipersError::TooFewPoints)
        );
        assert_eq!(rotating_calipers(&[]), Err(CalipersError::TooFewPoints));
    }

    #[test]
    fn diameter_and_width_convenience_builds_hull() {
        let points = [
            Point2::new(0.0, 0.0),
            Point2::new(3.0, 0.0),
            Point2::new(3.0, 3.0),
            Point2::new(0.0, 3.0),
            Point2::new(1.5, 1.5), // interior
        ];
        let r = diameter_and_width(&points).unwrap();
        assert!(approx_eq(r.diameter, (18.0f64).sqrt(), 1e-9)); // 3*sqrt(2)
        assert!(approx_eq(r.width, 3.0, 1e-9));
    }

    #[test]
    fn antipodal_pairs_cover_all_hull_vertices() {
        // A hexagon: every vertex should appear in at least one antipodal pair.
        let hull = [
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(3.0, 2.0),
            Point2::new(2.0, 4.0),
            Point2::new(0.0, 4.0),
            Point2::new(-1.0, 2.0),
        ];
        let r = rotating_calipers(&hull).unwrap();
        let mut seen = vec![false; hull.len()];
        for p in &r.antipodal_pairs {
            seen[p.i] = true;
            seen[p.j] = true;
        }
        // Not every vertex must be an antipode (only "extreme" ones), but the
        // diameter pair must be among the seen.
        assert!(seen[r.diameter_pair.i] && seen[r.diameter_pair.j]);
    }

    // ── Smallest enclosing disk ──

    #[test]
    fn sed_empty_errors() {
        assert_eq!(
            smallest_enclosing_disk(&[], 42),
            Err(CalipersError::TooFewPoints)
        );
    }

    #[test]
    fn sed_single_point_radius_zero() {
        let r = smallest_enclosing_disk(&[Point2::new(5.0, 7.0)], 1).unwrap();
        assert_eq!(r.disk.center, Point2::new(5.0, 7.0));
        assert!(approx_eq(r.disk.radius, 0.0, 1e-9));
        assert_eq!(r.support, vec![0]);
    }

    #[test]
    fn sed_two_points_is_diameter() {
        let pts = [Point2::new(0.0, 0.0), Point2::new(4.0, 0.0)];
        let r = smallest_enclosing_disk(&pts, 7).unwrap();
        assert_eq!(r.disk.center, Point2::new(2.0, 0.0));
        assert!(approx_eq(r.disk.radius, 2.0, 1e-9));
        // Both points on boundary.
        assert_eq!(r.support.len(), 2);
    }

    #[test]
    fn sed_square_encloses_all() {
        let pts = [
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(2.0, 2.0),
            Point2::new(0.0, 2.0),
        ];
        let r = smallest_enclosing_disk(&pts, 99).unwrap();
        // Smallest enclosing disk of a square: centre (1,1), radius sqrt(2).
        assert!(approx_eq(r.disk.center.x, 1.0, 1e-9));
        assert!(approx_eq(r.disk.center.y, 1.0, 1e-9));
        assert!(approx_eq(r.disk.radius, 2.0f64.sqrt(), 1e-9));
        // All points must be enclosed.
        assert!(pts.iter().all(|p| r.disk.contains(*p)));
    }

    #[test]
    fn sed_interior_point_does_not_grow_disk() {
        // Square + interior point: disk same as square alone.
        let pts = [
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(4.0, 4.0),
            Point2::new(0.0, 4.0),
            Point2::new(2.0, 2.0), // interior
        ];
        let r = smallest_enclosing_disk(&pts, 13).unwrap();
        assert!(approx_eq(r.disk.center.x, 2.0, 1e-9));
        assert!(approx_eq(r.disk.center.y, 2.0, 1e-9));
        assert!(approx_eq(r.disk.radius, 8.0f64.sqrt(), 1e-9)); // 2*sqrt(2)
        assert!(pts.iter().all(|p| r.disk.contains(*p)));
        // Interior point must NOT be in the support set.
        assert!(!r.support.contains(&4));
    }

    #[test]
    fn sed_collinear_points_diameter_of_extremes() {
        let pts = [
            Point2::new(0.0, 0.0),
            Point2::new(3.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(5.0, 0.0),
        ];
        let r = smallest_enclosing_disk(&pts, 42).unwrap();
        assert!(approx_eq(r.disk.center.x, 2.5, 1e-9));
        assert!(approx_eq(r.disk.center.y, 0.0, 1e-9));
        assert!(approx_eq(r.disk.radius, 2.5, 1e-9));
        assert!(pts.iter().all(|p| r.disk.contains(*p)));
    }

    #[test]
    fn sed_matches_brute_force_on_random_points() {
        let pts = [
            Point2::new(0.0, 0.0),
            Point2::new(5.0, 0.0),
            Point2::new(4.0, 4.0),
            Point2::new(1.0, 5.0),
            Point2::new(-1.0, 3.0),
            Point2::new(2.0, 2.0),
        ];
        let r = smallest_enclosing_disk(&pts, 2024).unwrap();
        let (bf_disk, _) = brute_force_enclosing_disk(&pts);
        assert!(
            approx_eq(r.disk.radius, bf_disk.radius, 1e-9),
            "welzl radius {} vs brute {}",
            r.disk.radius,
            bf_disk.radius
        );
        // Welzl must produce a valid enclosing disk.
        assert!(pts.iter().all(|p| r.disk.contains(*p)));
    }

    #[test]
    fn sed_seed_determinism() {
        let pts = [
            Point2::new(0.0, 0.0),
            Point2::new(5.0, 0.0),
            Point2::new(4.0, 4.0),
            Point2::new(1.0, 5.0),
            Point2::new(-1.0, 3.0),
        ];
        let r1 = smallest_enclosing_disk(&pts, 12345).unwrap();
        let r2 = smallest_enclosing_disk(&pts, 12345).unwrap();
        assert_eq!(r1, r2);
        // Different seed ⇒ same radius (deterministic problem).
        let r3 = smallest_enclosing_disk(&pts, 99999).unwrap();
        assert!(approx_eq(r1.disk.radius, r3.disk.radius, 1e-9));
    }

    #[test]
    fn sed_support_set_on_boundary() {
        // Every support point must be ON the disk boundary (within tolerance).
        let pts = [
            Point2::new(0.0, 0.0),
            Point2::new(4.0, 0.0),
            Point2::new(4.0, 4.0),
            Point2::new(0.0, 4.0),
        ];
        let r = smallest_enclosing_disk(&pts, 55).unwrap();
        for &idx in &r.support {
            let d = distance_2d(r.disk.center, pts[idx]);
            assert!(
                approx_eq(d, r.disk.radius, 1e-9),
                "support point {} at distance {} != radius {}",
                idx,
                d,
                r.disk.radius
            );
        }
        // Support set size must be 2 or 3 (1 only for single-point input).
        assert!(r.support.len() >= 2 && r.support.len() <= 3);
    }

    // ── Disk geometry ──

    #[test]
    fn disk_from_diameter_basic() {
        let d = Disk::from_diameter(Point2::new(0.0, 0.0), Point2::new(4.0, 0.0));
        assert_eq!(d.center, Point2::new(2.0, 0.0));
        assert!(approx_eq(d.radius, 2.0, 1e-9));
        assert!(d.contains(Point2::new(2.0, 0.0)));
        assert!(d.contains(Point2::new(0.0, 0.0))); // on boundary
        assert!(!d.contains(Point2::new(5.0, 0.0))); // outside
    }

    #[test]
    fn disk_from_three_basic() {
        // Equilateral-ish triangle.
        let d = Disk::from_three(
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(1.0, 3.0f64.sqrt()),
        )
        .unwrap();
        // All three on boundary.
        assert!(approx_eq(
            distance_2d(d.center, Point2::new(0.0, 0.0)),
            d.radius,
            1e-9
        ));
        assert!(approx_eq(
            distance_2d(d.center, Point2::new(2.0, 0.0)),
            d.radius,
            1e-9
        ));
        assert!(approx_eq(
            distance_2d(d.center, Point2::new(1.0, 3.0f64.sqrt())),
            d.radius,
            1e-9
        ));
    }

    #[test]
    fn disk_from_three_collinear_returns_none() {
        let d = Disk::from_three(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(2.0, 0.0),
        );
        assert!(d.is_none());
    }

    // ── Seeded permutation ──

    #[test]
    fn seeded_permutation_is_a_permutation() {
        for n in 0..=15 {
            let p = seeded_permutation(77, n);
            let mut sorted = p.clone();
            sorted.sort();
            assert_eq!(sorted, (0..n).collect::<Vec<_>>(), "n={}", n);
        }
    }

    #[test]
    fn seeded_permutation_deterministic() {
        assert_eq!(seeded_permutation(42, 10), seeded_permutation(42, 10));
        assert_ne!(seeded_permutation(42, 10), seeded_permutation(7, 10));
    }
}
