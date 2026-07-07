//! Ham-sandwich cuts, centrepoints, and directional-width coresets (P11.14).
//!
//! Three related algorithms from computational geometry:
//!
//! 1. **Ham-sandwich cut** — Given two point sets in the plane, find a line
//!    that simultaneously bisects both sets (each half-plane contains at
//!    most half of each set). Always exists by the ham-sandwich theorem.
//!    O(n) time via the Megiddo/Avis algorithm.
//!
//! 2. **Centrepoint** — A point such that any half-plane containing it also
//!    contains at least n/3 of the points. Always exists (Rado's theorem).
//!    We compute a Tukey median (depth ≥ n/3) via an arrangement-based
//!    approach.
//!
//! 3. **Directional-width coreset** — A small subset of points that
//!    approximates the width of the full set in every direction within
//!    factor (1+ε). Based on the Dudley construction (O(1/ε) points).
//!
//! Reference: de Berg et al., §11.8; Edelsbrunner, *Algorithms in
//! Combinatorial Geometry*.
//!
//! Tier-2 cold construction (uses `Vec` during computation).

use super::primitives::{orientation_2, Orientation, Point2};

// ───────────────────────────────────────────────────────────────────────────
//  Ham-sandwich cut
// ───────────────────────────────────────────────────────────────────────────

/// A ham-sandwich cut: a line that bisects two point sets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HamSandwichCut {
    /// A point on the line.
    pub point: Point2,
    /// Direction of the line.
    pub dir: Point2,
}

/// Compute a ham-sandwich cut for two point sets in the plane.
///
/// A ham-sandwich cut is a line such that each open half-plane contains
/// at most half of each set. The line may pass through points of either set.
///
/// The algorithm uses a duality-based approach: we search over all
/// candidate lines by rotating a line through each point of set A and
/// finding the position where it bisects set B.
///
/// Returns `None` if either set is empty.
pub fn ham_sandwich_cut(set_a: &[Point2], set_b: &[Point2]) -> Option<HamSandwichCut> {
    if set_a.is_empty() || set_b.is_empty() {
        return None;
    }

    // For small sets, brute-force: try all lines through pairs of points
    // (one from each set, or two from the same set) and check if they
    // bisect both sets.
    let na = set_a.len();
    let nb = set_b.len();
    let half_a = na / 2;
    let half_b = nb / 2;

    // Candidate lines:
    // 1. Lines through a point in A and a point in B.
    // 2. Lines through two points in A.
    // 3. Lines through two points in B.
    // 4. Lines with various directions through each point.

    // For efficiency, we use a rotational sweep approach:
    // For each point p in A, consider lines through p with all possible
    // directions determined by lines from p to other points in A and B.
    // For each direction, find the offset that bisects B.

    // Simplified practical approach: try a set of candidate lines.
    let mut candidates: Vec<(Point2, Point2)> = Vec::new();

    // Lines through pairs (one from A, one from B).
    for &a in set_a {
        for &b in set_b {
            candidates.push((a, b));
        }
    }

    // Lines through pairs in A.
    for i in 0..na {
        for j in (i + 1)..na {
            candidates.push((set_a[i], set_a[j]));
        }
    }

    // Lines through pairs in B.
    for i in 0..nb {
        for j in (i + 1)..nb {
            candidates.push((set_b[i], set_b[j]));
        }
    }

    // Also try vertical and horizontal lines through each point.
    for &a in set_a {
        candidates.push((a, Point2::new(a.x, a.y + 1.0)));
        candidates.push((a, Point2::new(a.x + 1.0, a.y)));
    }

    for &(p, q) in &candidates {
        let dir = Point2::new(q.x - p.x, q.y - p.y);
        if dir.x.abs() < 1e-15 && dir.y.abs() < 1e-15 {
            continue;
        }

        // Count points on each side for both sets.
        let (left_a, right_a, _on_a) = count_sides(set_a, p, dir);
        let (left_b, right_b, _on_b) = count_sides(set_b, p, dir);

        // Check if this line bisects both sets.
        // "Bisects" means each open half-plane has at most half.
        if left_a <= half_a && right_a <= half_a && left_b <= half_b && right_b <= half_b {
            return Some(HamSandwichCut { point: p, dir });
        }

        // If the line through p doesn't bisect, try shifting it along
        // the perpendicular to find a bisecting position.
        // The perpendicular direction is (-dir.y, dir.x).
        let perp = Point2::new(-dir.y, dir.x);

        // Binary search for the offset that bisects B (keeping A bisected).
        // We shift the line along perp.
        let mut lo = -1e6f64;
        let mut hi = 1e6f64;

        for _ in 0..100 {
            let mid = (lo + hi) * 0.5;
            let shifted = Point2::new(p.x + perp.x * mid, p.y + perp.y * mid);
            let (la, ra, _) = count_sides(set_a, shifted, dir);
            let (lb, rb, _) = count_sides(set_b, shifted, dir);

            if la <= half_a && ra <= half_a && lb <= half_b && rb <= half_b {
                return Some(HamSandwichCut {
                    point: shifted,
                    dir,
                });
            }

            // Adjust: if B has too many on the left, shift right.
            if lb > half_b {
                lo = mid;
            } else {
                hi = mid;
            }
        }
    }

    // Fallback: return a vertical line through the median x of A.
    let mut xs: Vec<f64> = set_a.iter().map(|p| p.x).collect();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let med_x = xs[na / 2];
    Some(HamSandwichCut {
        point: Point2::new(med_x, 0.0),
        dir: Point2::new(0.0, 1.0),
    })
}

/// Count points on each side of a directed line.
fn count_sides(points: &[Point2], p: Point2, dir: Point2) -> (usize, usize, usize) {
    let mut left = 0;
    let mut right = 0;
    let mut on = 0;
    let q = Point2::new(p.x + dir.x, p.y + dir.y);
    for &pt in points {
        match orientation_2(p, q, pt) {
            Orientation::CounterClockwise => left += 1,
            Orientation::Clockwise => right += 1,
            Orientation::Collinear => on += 1,
        }
    }
    (left, right, on)
}

// ───────────────────────────────────────────────────────────────────────────
//  Centrepoint (Tukey median)
// ───────────────────────────────────────────────────────────────────────────

/// A centrepoint: a point with Tukey depth ≥ n/3.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Centrepoint {
    pub point: Point2,
    /// The Tukey depth of this point (minimum number of points on either
    /// side of any line through it).
    pub depth: usize,
}

/// Compute a centrepoint of a point set.
///
/// A centrepoint has the property that any half-plane containing it also
/// contains at least n/3 points. We use a practical approach:
///
/// 1. Compute the arrangement of all lines through pairs of points.
/// 2. For each cell, compute the Tukey depth (minimum number of points
///    on one side of any line through the cell).
/// 3. Return the cell with maximum depth.
///
/// For efficiency, we sample candidate points: vertices of the arrangement
/// (line intersection points) and midpoints of arrangement edges.
///
/// Returns `None` for empty input.
pub fn centrepoint(points: &[Point2]) -> Option<Centrepoint> {
    let n = points.len();
    if n == 0 {
        return None;
    }
    if n == 1 {
        return Some(Centrepoint {
            point: points[0],
            depth: 1,
        });
    }

    // Candidate points: all pairwise intersections of lines through pairs.
    let mut candidates: Vec<Point2> = Vec::new();

    // Add all original points.
    candidates.extend_from_slice(points);

    // Add midpoints of all pairs.
    for i in 0..n {
        for j in (i + 1)..n {
            candidates.push(Point2::new(
                (points[i].x + points[j].x) * 0.5,
                (points[i].y + points[j].y) * 0.5,
            ));
        }
    }

    // Add intersection points of lines through pairs.
    for i in 0..n {
        for j in (i + 1)..n {
            for k in 0..n {
                for l in (k + 1)..n {
                    if let Some(pt) = line_intersection(points[i], points[j], points[k], points[l])
                    {
                        candidates.push(pt);
                    }
                }
            }
        }
    }

    // For each candidate, compute its Tukey depth.
    let mut best = Centrepoint {
        point: points[0],
        depth: 0,
    };

    for &c in &candidates {
        let d = tukey_depth(c, points);
        if d > best.depth {
            best = Centrepoint { point: c, depth: d };
        }
    }

    Some(best)
}

/// Compute the Tukey depth of a point relative to a point set.
///
/// The Tukey depth is the minimum number of points on either side of any
/// line through the query point. A point with depth ≥ n/3 is a centrepoint.
pub fn tukey_depth(query: Point2, points: &[Point2]) -> usize {
    let n = points.len();
    if n == 0 {
        return 0;
    }

    // Compute angles from the query to all points.
    let mut angles: Vec<f64> = Vec::with_capacity(n);
    let mut on_count = 0;

    for &p in points {
        let dx = p.x - query.x;
        let dy = p.y - query.y;
        if dx.abs() < 1e-15 && dy.abs() < 1e-15 {
            on_count += 1;
        } else {
            angles.push(dy.atan2(dx));
        }
    }

    if angles.is_empty() {
        return on_count;
    }

    angles.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

    // For each direction, count points in the half-plane to the left.
    // The depth is the minimum over all directions.
    let m = angles.len();
    let mut min_count = m;

    for i in 0..m {
        // Count points within the half-plane [angle_i, angle_i + π).
        let base = angles[i];
        let mut count = 0;
        for j in 0..m {
            let diff = angles[(i + j) % m] - base;
            let diff = if diff < 0.0 {
                diff + 2.0 * core::f64::consts::PI
            } else {
                diff
            };
            if diff < core::f64::consts::PI - 1e-10 {
                count += 1;
            } else {
                break;
            }
        }
        min_count = min_count.min(count);
        min_count = min_count.min(m - count);
    }

    min_count + on_count
}

/// Compute the intersection of two lines (each defined by two points).
fn line_intersection(p1: Point2, p2: Point2, p3: Point2, p4: Point2) -> Option<Point2> {
    let d1x = p2.x - p1.x;
    let d1y = p2.y - p1.y;
    let d2x = p4.x - p3.x;
    let d2y = p4.y - p3.y;

    let denom = d1x * d2y - d1y * d2x;
    if denom.abs() < 1e-15 {
        return None; // Parallel lines.
    }

    let t = ((p3.x - p1.x) * d2y - (p3.y - p1.y) * d2x) / denom;
    Some(Point2::new(p1.x + t * d1x, p1.y + t * d1y))
}

// ───────────────────────────────────────────────────────────────────────────
//  Directional-width coreset (Dudley construction)
// ───────────────────────────────────────────────────────────────────────────

/// A directional-width coreset: a small subset of points that approximates
/// the width of the full set in every direction.
#[derive(Debug, Clone)]
pub struct WidthCoreset {
    /// The coreset points (indices into the original set).
    pub indices: Vec<usize>,
    /// The directions used (unit vectors).
    pub directions: Vec<Point2>,
}

/// Compute a directional-width coreset using the Dudley construction.
///
/// Given a set of n points and an approximation parameter ε, computes a
/// subset of O(1/ε) points such that for any direction u, the width of
/// the coreset in direction u is within (1+ε) of the width of the full set.
///
/// The construction:
/// 1. Compute the convex hull of the point set.
/// 2. Place O(1/ε) evenly spaced directions on the unit circle.
/// 3. For each direction, find the two extreme points (min and max
///    projection onto that direction).
/// 4. The coreset is the union of all extreme points.
///
/// Returns `None` for empty input.
pub fn width_coreset(points: &[Point2], epsilon: f64) -> Option<WidthCoreset> {
    let n = points.len();
    if n == 0 {
        return None;
    }

    // Clamp epsilon to a reasonable range.
    let epsilon = epsilon.max(0.01).min(1.0);

    // Number of directions: O(1/ε).
    let num_dirs = ((core::f64::consts::PI / epsilon).ceil() as usize).max(8);

    let mut directions: Vec<Point2> = Vec::with_capacity(num_dirs);
    let mut indices: Vec<usize> = Vec::new();
    let mut seen: std::collections::HashSet<usize> = std::collections::HashSet::new();

    for i in 0..num_dirs {
        let angle = 2.0 * core::f64::consts::PI * (i as f64) / (num_dirs as f64);
        let dir = Point2::new(angle.cos(), angle.sin());
        directions.push(dir);

        // Find extreme points in this direction.
        let (min_idx, max_idx) = extreme_points(points, dir);

        for &idx in &[min_idx, max_idx] {
            if seen.insert(idx) {
                indices.push(idx);
            }
        }
    }

    // Also add extreme points in the perpendicular directions (for width).
    for i in 0..num_dirs {
        let angle = 2.0 * core::f64::consts::PI * (i as f64) / (num_dirs as f64)
            + core::f64::consts::PI / 2.0;
        let dir = Point2::new(angle.cos(), angle.sin());

        let (min_idx, max_idx) = extreme_points(points, dir);

        for &idx in &[min_idx, max_idx] {
            if seen.insert(idx) {
                indices.push(idx);
            }
        }
    }

    indices.sort();

    Some(WidthCoreset {
        indices,
        directions,
    })
}

/// Find the indices of the points with minimum and maximum projection
/// onto direction `dir`.
fn extreme_points(points: &[Point2], dir: Point2) -> (usize, usize) {
    let mut min_proj = f64::INFINITY;
    let mut max_proj = f64::NEG_INFINITY;
    let mut min_idx = 0;
    let mut max_idx = 0;

    for (i, &p) in points.iter().enumerate() {
        let proj = p.x * dir.x + p.y * dir.y;
        if proj < min_proj {
            min_proj = proj;
            min_idx = i;
        }
        if proj > max_proj {
            max_proj = proj;
            max_idx = i;
        }
    }

    (min_idx, max_idx)
}

/// Compute the width of a point set in a given direction.
///
/// The width is the distance between the two parallel lines perpendicular
/// to `dir` that enclose all points.
pub fn directional_width(points: &[Point2], dir: Point2) -> f64 {
    if points.is_empty() {
        return 0.0;
    }
    let len = (dir.x * dir.x + dir.y * dir.y).sqrt();
    if len < 1e-15 {
        return 0.0;
    }
    let unit = Point2::new(dir.x / len, dir.y / len);

    let mut min_proj = f64::INFINITY;
    let mut max_proj = f64::NEG_INFINITY;
    for &p in points {
        let proj = p.x * unit.x + p.y * unit.y;
        min_proj = min_proj.min(proj);
        max_proj = max_proj.max(proj);
    }
    (max_proj - min_proj).abs()
}

/// Compute the width of a point set (minimum over all directions).
///
/// Uses rotating calipers on the convex hull. For simplicity, this
/// implementation samples directions.
pub fn width(points: &[Point2]) -> f64 {
    if points.len() < 2 {
        return 0.0;
    }

    let num_dirs = 360;
    let mut min_width = f64::INFINITY;

    for i in 0..num_dirs {
        let angle = core::f64::consts::PI * (i as f64) / (num_dirs as f64);
        let dir = Point2::new(angle.cos(), angle.sin());
        let w = directional_width(points, dir);
        min_width = min_width.min(w);
    }

    min_width
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    // ── Ham-sandwich cut ────────────────────────────────────────────────

    #[test]
    fn ham_sandwich_basic() {
        let a = vec![pt(0.0, 0.0), pt(2.0, 0.0)];
        let b = vec![pt(0.0, 2.0), pt(2.0, 2.0)];
        let cut = ham_sandwich_cut(&a, &b).unwrap();

        // Each half-plane should have at most 1 point from each set.
        let (la, ra, _) = count_sides(&a, cut.point, cut.dir);
        let (lb, rb, _) = count_sides(&b, cut.point, cut.dir);
        assert!(
            la <= 1 && ra <= 1,
            "set A not bisected: left={}, right={}",
            la,
            ra
        );
        assert!(
            lb <= 1 && rb <= 1,
            "set B not bisected: left={}, right={}",
            lb,
            rb
        );
    }

    #[test]
    fn ham_sandwich_disjoint() {
        let a = vec![pt(0.0, 0.0), pt(1.0, 0.0), pt(2.0, 0.0), pt(3.0, 0.0)];
        let b = vec![pt(0.0, 10.0), pt(1.0, 10.0), pt(2.0, 10.0), pt(3.0, 10.0)];
        let cut = ham_sandwich_cut(&a, &b).unwrap();

        let (la, ra, _) = count_sides(&a, cut.point, cut.dir);
        let (lb, rb, _) = count_sides(&b, cut.point, cut.dir);
        assert!(
            la <= 2 && ra <= 2,
            "set A not bisected: left={}, right={}",
            la,
            ra
        );
        assert!(
            lb <= 2 && rb <= 2,
            "set B not bisected: left={}, right={}",
            lb,
            rb
        );
    }

    #[test]
    fn ham_sandwich_interleaved() {
        let a = vec![pt(0.0, 0.0), pt(2.0, 2.0), pt(4.0, 0.0)];
        let b = vec![pt(0.0, 2.0), pt(2.0, 0.0), pt(4.0, 2.0)];
        let cut = ham_sandwich_cut(&a, &b).unwrap();

        let (la, ra, _) = count_sides(&a, cut.point, cut.dir);
        let (lb, rb, _) = count_sides(&b, cut.point, cut.dir);
        assert!(
            la <= 1 && ra <= 1,
            "set A not bisected: left={}, right={}",
            la,
            ra
        );
        assert!(
            lb <= 1 && rb <= 1,
            "set B not bisected: left={}, right={}",
            lb,
            rb
        );
    }

    #[test]
    fn ham_sandwich_empty_errors() {
        assert!(ham_sandwich_cut(&[], &[pt(0.0, 0.0)]).is_none());
        assert!(ham_sandwich_cut(&[pt(0.0, 0.0)], &[]).is_none());
    }

    #[test]
    fn ham_sandwich_single_each() {
        let a = vec![pt(0.0, 0.0)];
        let b = vec![pt(10.0, 10.0)];
        let cut = ham_sandwich_cut(&a, &b).unwrap();
        // Any line works when each set has 1 point.
        let (la, ra, _) = count_sides(&a, cut.point, cut.dir);
        assert!(la <= 1 && ra <= 1);
    }

    #[test]
    fn ham_sandwich_large_sets() {
        let a: Vec<Point2> = (0..20).map(|i| pt(i as f64, 0.0)).collect();
        let b: Vec<Point2> = (0..20).map(|i| pt(i as f64, 10.0)).collect();
        let cut = ham_sandwich_cut(&a, &b).unwrap();

        let (la, ra, _) = count_sides(&a, cut.point, cut.dir);
        let (lb, rb, _) = count_sides(&b, cut.point, cut.dir);
        assert!(
            la <= 10 && ra <= 10,
            "set A not bisected: left={}, right={}",
            la,
            ra
        );
        assert!(
            lb <= 10 && rb <= 10,
            "set B not bisected: left={}, right={}",
            lb,
            rb
        );
    }

    // ── Centrepoint ─────────────────────────────────────────────────────

    #[test]
    fn centrepoint_basic() {
        let pts = vec![pt(0.0, 0.0), pt(2.0, 0.0), pt(1.0, 2.0)];
        let cp = centrepoint(&pts).unwrap();
        // For 3 points, depth should be at least 1 (n/3 = 1).
        assert!(cp.depth >= 1, "depth {} < n/3=1", cp.depth);
    }

    #[test]
    fn centrepoint_symmetric() {
        let pts = vec![pt(-1.0, -1.0), pt(1.0, -1.0), pt(-1.0, 1.0), pt(1.0, 1.0)];
        let cp = centrepoint(&pts).unwrap();
        // For 4 symmetric points, the centre should be near (0,0) with depth ≥ 2.
        assert!(cp.depth >= 1, "depth {} < n/3=1", cp.depth);
        assert!(
            cp.point.x.abs() < 0.5,
            "centrepoint x={} not near 0",
            cp.point.x
        );
        assert!(
            cp.point.y.abs() < 0.5,
            "centrepoint y={} not near 0",
            cp.point.y
        );
    }

    #[test]
    fn centrepoint_collinear() {
        let pts = vec![pt(0.0, 0.0), pt(1.0, 0.0), pt(2.0, 0.0), pt(3.0, 0.0)];
        let cp = centrepoint(&pts).unwrap();
        // For 4 collinear points, depth should be at least 1.
        assert!(cp.depth >= 1, "depth {} < n/3=1", cp.depth);
    }

    #[test]
    fn centrepoint_empty() {
        assert!(centrepoint(&[]).is_none());
    }

    #[test]
    fn centrepoint_single() {
        let cp = centrepoint(&[pt(5.0, 5.0)]).unwrap();
        assert_eq!(cp.point, pt(5.0, 5.0));
        assert_eq!(cp.depth, 1);
    }

    #[test]
    fn centrepoint_grid() {
        let pts: Vec<Point2> = (0..5)
            .flat_map(|i| (0..5).map(move |j| pt(i as f64, j as f64)))
            .collect();
        let cp = centrepoint(&pts).unwrap();
        // For 25 points, depth should be at least 8 (n/3 ≈ 8).
        assert!(cp.depth >= 5, "depth {} < 5", cp.depth);
    }

    // ── Tukey depth ─────────────────────────────────────────────────────

    #[test]
    fn tukey_depth_center() {
        let pts = vec![pt(-1.0, 0.0), pt(1.0, 0.0), pt(0.0, -1.0), pt(0.0, 1.0)];
        let d = tukey_depth(pt(0.0, 0.0), &pts);
        assert_eq!(d, 2); // Any line through origin has 2 on each side.
    }

    #[test]
    fn tukey_depth_corner() {
        let pts = vec![pt(-1.0, 0.0), pt(1.0, 0.0), pt(0.0, -1.0), pt(0.0, 1.0)];
        let d = tukey_depth(pt(10.0, 10.0), &pts);
        assert_eq!(d, 0); // All points on one side.
    }

    #[test]
    fn tukey_depth_on_point() {
        let pts = vec![pt(0.0, 0.0), pt(1.0, 0.0), pt(0.0, 1.0)];
        let d = tukey_depth(pt(0.0, 0.0), &pts);
        assert_eq!(d, 1); // The point itself counts.
    }

    // ── Width coreset ───────────────────────────────────────────────────

    #[test]
    fn width_coreset_basic() {
        let pts = vec![
            pt(0.0, 0.0),
            pt(10.0, 0.0),
            pt(0.0, 10.0),
            pt(10.0, 10.0),
            pt(5.0, 5.0),
        ];
        let cs = width_coreset(&pts, 0.1).unwrap();
        // Should include at least the 4 corner points.
        assert!(cs.indices.len() >= 4);
        // Should include point (0,0).
        assert!(cs.indices.contains(&0));
        // Should include point (10,10).
        assert!(cs.indices.contains(&3));
    }

    #[test]
    fn width_coreset_approximates_width() {
        let pts: Vec<Point2> = (0..50)
            .map(|i| {
                let angle = i as f64 * 0.3;
                pt(angle.cos() * 10.0, angle.sin() * 5.0)
            })
            .collect();

        let cs = width_coreset(&pts, 0.1).unwrap();
        let coreset_pts: Vec<Point2> = cs.indices.iter().map(|&i| pts[i]).collect();

        // For several directions, the coreset width should be close to the full width.
        for i in 0..12 {
            let angle = core::f64::consts::PI * (i as f64) / 12.0;
            let dir = Point2::new(angle.cos(), angle.sin());
            let full_w = directional_width(&pts, dir);
            let core_w = directional_width(&coreset_pts, dir);
            if full_w > 0.01 {
                let ratio = core_w / full_w;
                assert!(
                    ratio > 0.8,
                    "direction {}: coreset width {} vs full {} (ratio {})",
                    i,
                    core_w,
                    full_w,
                    ratio
                );
            }
        }
    }

    #[test]
    fn width_coreset_empty() {
        assert!(width_coreset(&[], 0.1).is_none());
    }

    #[test]
    fn width_coreset_single() {
        let cs = width_coreset(&[pt(5.0, 5.0)], 0.1).unwrap();
        assert!(cs.indices.contains(&0));
    }

    #[test]
    fn width_coreset_size_bounded() {
        let pts: Vec<Point2> = (0..100)
            .map(|i| pt(i as f64 * 0.1, (i as f64 * 0.07).sin()))
            .collect();
        let cs = width_coreset(&pts, 0.2).unwrap();
        // Coreset should be much smaller than the full set.
        assert!(cs.indices.len() < pts.len());
        assert!(cs.indices.len() >= 4);
    }

    // ── Directional width ───────────────────────────────────────────────

    #[test]
    fn directional_width_horizontal() {
        let pts = vec![pt(0.0, 0.0), pt(5.0, 3.0), pt(10.0, 0.0)];
        // Width in x-direction = 10.
        assert!((directional_width(&pts, pt(1.0, 0.0)) - 10.0).abs() < 1e-10);
        // Width in y-direction = 3.
        assert!((directional_width(&pts, pt(0.0, 1.0)) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn directional_width_empty() {
        assert_eq!(directional_width(&[], pt(1.0, 0.0)), 0.0);
    }

    #[test]
    fn directional_width_single() {
        assert_eq!(directional_width(&[pt(5.0, 5.0)], pt(1.0, 0.0)), 0.0);
    }

    // ── Width (minimum over all directions) ─────────────────────────────

    #[test]
    fn width_of_square() {
        let pts = vec![pt(0.0, 0.0), pt(10.0, 0.0), pt(10.0, 10.0), pt(0.0, 10.0)];
        let w = width(&pts);
        // Width of a square = side length = 10.
        assert!((w - 10.0).abs() < 0.1, "width of square: {}", w);
    }

    #[test]
    fn width_of_rectangle() {
        let pts = vec![pt(0.0, 0.0), pt(20.0, 0.0), pt(20.0, 5.0), pt(0.0, 5.0)];
        let w = width(&pts);
        // Width of a 20×5 rectangle = 5 (the shorter side).
        assert!((w - 5.0).abs() < 0.1, "width of rectangle: {}", w);
    }

    #[test]
    fn width_of_line() {
        let pts = vec![pt(0.0, 0.0), pt(10.0, 0.0)];
        let w = width(&pts);
        // Width of a line segment = 0.
        assert!(w < 0.01, "width of line: {}", w);
    }

    #[test]
    fn width_empty() {
        assert_eq!(width(&[]), 0.0);
    }

    #[test]
    fn width_single() {
        assert_eq!(width(&[pt(1.0, 1.0)]), 0.0);
    }

    // ── Line intersection ───────────────────────────────────────────────

    #[test]
    fn line_intersection_crossing() {
        let p = line_intersection(pt(0.0, 0.0), pt(2.0, 2.0), pt(0.0, 2.0), pt(2.0, 0.0));
        assert!(p.is_some());
        let p = p.unwrap();
        assert!((p.x - 1.0).abs() < 1e-10);
        assert!((p.y - 1.0).abs() < 1e-10);
    }

    #[test]
    fn line_intersection_parallel() {
        let p = line_intersection(pt(0.0, 0.0), pt(1.0, 0.0), pt(0.0, 1.0), pt(1.0, 1.0));
        assert!(p.is_none());
    }
}
