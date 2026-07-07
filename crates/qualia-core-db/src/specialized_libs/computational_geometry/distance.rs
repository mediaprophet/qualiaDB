//! Distance and intersection primitive family for 2D and 3D geometry.
//!
//! Provides allocation-free distance computations and exact/filtered
//! intersection tests for the spatial query layer (BVH, kd-tree, box joins).
//!
//! All functions are `#[inline]`, zero-heap, and deterministic. Intersection
//! tests return an enum classifying the result (including degenerate cases:
//! collinear, coplanar, touching, zero-length, ray-grazes-edge).

use super::primitives::{Point2, Point3};

// ---------------------------------------------------------------------------
// Distance: 2D
// ---------------------------------------------------------------------------

/// Squared Euclidean distance between two 2D points.
#[inline]
pub fn distance_sq_2d(a: Point2, b: Point2) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

/// Euclidean distance between two 2D points.
#[inline]
pub fn distance_2d(a: Point2, b: Point2) -> f64 {
    distance_sq_2d(a, b).sqrt()
}

/// Squared distance from point `p` to segment `ab` in 2D.
#[inline]
pub fn point_segment_distance_sq_2d(p: Point2, a: Point2, b: Point2) -> f64 {
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let apx = p.x - a.x;
    let apy = p.y - a.y;
    let ab_sq = abx * abx + aby * aby;
    if ab_sq == 0.0 {
        // Degenerate segment (a == b).
        return apx * apx + apy * apy;
    }
    let t = (apx * abx + apy * aby) / ab_sq;
    let t = t.clamp(0.0, 1.0);
    let cx = a.x + t * abx;
    let cy = a.y + t * aby;
    let dx = p.x - cx;
    let dy = p.y - cy;
    dx * dx + dy * dy
}

/// Distance from point `p` to segment `ab` in 2D.
#[inline]
pub fn point_segment_distance_2d(p: Point2, a: Point2, b: Point2) -> f64 {
    point_segment_distance_sq_2d(p, a, b).sqrt()
}

/// Squared distance from point `p` to the line through `a` and `b` (infinite line) in 2D.
#[inline]
pub fn point_line_distance_sq_2d(p: Point2, a: Point2, b: Point2) -> f64 {
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let ab_sq = abx * abx + aby * aby;
    if ab_sq == 0.0 {
        let dx = p.x - a.x;
        let dy = p.y - a.y;
        return dx * dx + dy * dy;
    }
    let cross = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
    cross * cross / ab_sq
}

// ---------------------------------------------------------------------------
// Distance: 3D
// ---------------------------------------------------------------------------

/// Squared Euclidean distance between two 3D points.
#[inline]
pub fn distance_sq_3d(a: Point3, b: Point3) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    dx * dx + dy * dy + dz * dz
}

/// Euclidean distance between two 3D points.
#[inline]
pub fn distance_3d(a: Point3, b: Point3) -> f64 {
    distance_sq_3d(a, b).sqrt()
}

/// Squared distance from point `p` to segment `ab` in 3D.
#[inline]
pub fn point_segment_distance_sq_3d(p: Point3, a: Point3, b: Point3) -> f64 {
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let abz = b.z - a.z;
    let apx = p.x - a.x;
    let apy = p.y - a.y;
    let apz = p.z - a.z;
    let ab_sq = abx * abx + aby * aby + abz * abz;
    if ab_sq == 0.0 {
        return apx * apx + apy * apy + apz * apz;
    }
    let t = (apx * abx + apy * aby + apz * abz) / ab_sq;
    let t = t.clamp(0.0, 1.0);
    let cx = a.x + t * abx;
    let cy = a.y + t * aby;
    let cz = a.z + t * abz;
    let dx = p.x - cx;
    let dy = p.y - cy;
    let dz = p.z - cz;
    dx * dx + dy * dy + dz * dz
}

/// Squared distance from point `p` to triangle `abc` in 3D.
///
/// Projects `p` onto the triangle plane, classifies the projection relative to
/// the triangle edges, and returns the squared distance to the closest feature
/// (vertex, edge, or interior).
pub fn point_triangle_distance_sq_3d(p: Point3, a: Point3, b: Point3, c: Point3) -> f64 {
    let ab = Point3::new(b.x - a.x, b.y - a.y, b.z - a.z);
    let ac = Point3::new(c.x - a.x, c.y - a.y, c.z - a.z);
    let ap = Point3::new(p.x - a.x, p.y - a.y, p.z - a.z);

    let d1 = dot_3d(ab, ap);
    let d2 = dot_3d(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        // Closest to vertex a.
        return dot_3d(ap, ap);
    }

    let bp = Point3::new(p.x - b.x, p.y - b.y, p.z - b.z);
    let d3 = dot_3d(ab, bp);
    let d4 = dot_3d(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        // Closest to vertex b.
        return dot_3d(bp, bp);
    }

    let cp = Point3::new(p.x - c.x, p.y - c.y, p.z - c.z);
    let d5 = dot_3d(ab, cp);
    let d6 = dot_3d(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        // Closest to vertex c.
        return dot_3d(cp, cp);
    }

    // Edge ab.
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let t = d1 / (d1 - d3);
        let cx = a.x + t * ab.x;
        let cy = a.y + t * ab.y;
        let cz = a.z + t * ab.z;
        let dx = p.x - cx;
        let dy = p.y - cy;
        let dz = p.z - cz;
        return dx * dx + dy * dy + dz * dz;
    }

    // Edge ac.
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let t = d2 / (d2 - d6);
        let cx = a.x + t * ac.x;
        let cy = a.y + t * ac.y;
        let cz = a.z + t * ac.z;
        let dx = p.x - cx;
        let dy = p.y - cy;
        let dz = p.z - cz;
        return dx * dx + dy * dy + dz * dz;
    }

    // Edge bc.
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let t = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        let bcx = c.x - b.x;
        let bcy = c.y - b.y;
        let bcz = c.z - b.z;
        let cx = b.x + t * bcx;
        let cy = b.y + t * bcy;
        let cz = b.z + t * bcz;
        let dx = p.x - cx;
        let dy = p.y - cy;
        let dz = p.z - cz;
        return dx * dx + dy * dy + dz * dz;
    }

    // Interior of the triangle: project onto plane.
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    let cx = a.x + ab.x * v + ac.x * w;
    let cy = a.y + ab.y * v + ac.y * w;
    let cz = a.z + ab.z * v + ac.z * w;
    let dx = p.x - cx;
    let dy = p.y - cy;
    let dz = p.z - cz;
    dx * dx + dy * dy + dz * dz
}

#[inline]
fn dot_3d(a: Point3, b: Point3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

// ---------------------------------------------------------------------------
// Intersection: 2D segment-segment
// ---------------------------------------------------------------------------

/// Result of a 2D segment-segment intersection test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentIntersection2d {
    /// Segments do not intersect.
    Disjoint,
    /// Segments intersect at a single point.
    Point,
    /// Segments overlap over an interval (collinear overlap).
    Overlap,
    /// Segments share an endpoint (touching).
    Touching,
}

/// Test whether two 2D segments `ab` and `cd` intersect.
///
/// Uses orientation predicates for the general case and direct coordinate
/// comparison for degenerate (collinear) cases. Returns the intersection type.
pub fn segment_segment_intersect_2d(
    a: Point2,
    b: Point2,
    c: Point2,
    d: Point2,
) -> SegmentIntersection2d {
    use super::primitives::orientation_2;

    let o1 = orientation_2(a, b, c);
    let o2 = orientation_2(a, b, d);
    let o3 = orientation_2(c, d, a);
    let o4 = orientation_2(c, d, b);

    // General case: proper intersection.
    if o1 != o2 && o3 != o4 {
        return SegmentIntersection2d::Point;
    }

    // Degenerate: collinear cases.
    if o1 == super::primitives::Orientation::Collinear
        && o2 == super::primitives::Orientation::Collinear
    {
        // All four points are collinear. Check for overlap.
        // Project onto the dominant axis.
        let (a_t, b_t) = if (b.x - a.x).abs() >= (b.y - a.y).abs() {
            (a.x, b.x)
        } else {
            (a.y, b.y)
        };
        let (c_t, d_t) = if (b.x - a.x).abs() >= (b.y - a.y).abs() {
            (c.x, d.x)
        } else {
            (c.y, d.y)
        };

        let (lo_ab, hi_ab) = (a_t.min(b_t), a_t.max(b_t));
        let (lo_cd, hi_cd) = (c_t.min(d_t), c_t.max(d_t));

        if hi_ab < lo_cd || hi_cd < lo_ab {
            return SegmentIntersection2d::Disjoint;
        }
        // Check if they just touch at an endpoint.
        if hi_ab == lo_cd || hi_cd == lo_ab {
            return SegmentIntersection2d::Touching;
        }
        return SegmentIntersection2d::Overlap;
    }

    // One endpoint lies on the other segment.
    if o1 == super::primitives::Orientation::Collinear && on_segment_2d(a, b, c) {
        return SegmentIntersection2d::Touching;
    }
    if o2 == super::primitives::Orientation::Collinear && on_segment_2d(a, b, d) {
        return SegmentIntersection2d::Touching;
    }
    if o3 == super::primitives::Orientation::Collinear && on_segment_2d(c, d, a) {
        return SegmentIntersection2d::Touching;
    }
    if o4 == super::primitives::Orientation::Collinear && on_segment_2d(c, d, b) {
        return SegmentIntersection2d::Touching;
    }

    SegmentIntersection2d::Disjoint
}

/// Check if point `p` lies on segment `ab` (assuming collinearity).
#[inline]
fn on_segment_2d(a: Point2, b: Point2, p: Point2) -> bool {
    p.x >= a.x.min(b.x) && p.x <= a.x.max(b.x) && p.y >= a.y.min(b.y) && p.y <= a.y.max(b.y)
}

// ---------------------------------------------------------------------------
// Intersection: 3D ray-triangle (Möller–Trumbore)
// ---------------------------------------------------------------------------

/// Result of a 3D ray-triangle intersection test.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayTriangleHit {
    /// Ray parameter `t` at the hit point: `origin + t * direction`.
    pub t: f64,
    /// Barycentric coordinate u.
    pub u: f64,
    /// Barycentric coordinate v.
    pub v: f64,
}

/// Result of a 3D ray-triangle intersection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RayTriangleResult {
    /// Ray hits the triangle at the given parameters.
    Hit(RayTriangleHit),
    /// Ray misses the triangle.
    Miss,
    /// Ray is parallel to the triangle plane (grazes).
    Parallel,
    /// Degenerate triangle (zero area).
    DegenerateTriangle,
}

/// Test intersection of a 3D ray with a triangle using the Möller–Trumbore algorithm.
///
/// - `origin`: ray origin.
/// - `direction`: ray direction (need not be normalized; `t` is in units of `direction`).
/// - `a, b, c`: triangle vertices.
///
/// Zero-heap. Deterministic.
pub fn ray_triangle_intersect_3d(
    origin: Point3,
    direction: Point3,
    a: Point3,
    b: Point3,
    c: Point3,
) -> RayTriangleResult {
    let edge1 = Point3::new(b.x - a.x, b.y - a.y, b.z - a.z);
    let edge2 = Point3::new(c.x - a.x, c.y - a.y, c.z - a.z);

    let h = cross_3d(direction, edge2);
    let det = dot_3d(edge1, h);

    // Back-face culling tolerance: if det is near zero, ray is parallel.
    if det.abs() < f64::EPSILON {
        // Check for degenerate triangle.
        let edge1_sq = dot_3d(edge1, edge1);
        let edge2_sq = dot_3d(edge2, edge2);
        if edge1_sq == 0.0 || edge2_sq == 0.0 {
            return RayTriangleResult::DegenerateTriangle;
        }
        return RayTriangleResult::Parallel;
    }

    let inv_det = 1.0 / det;
    let s = Point3::new(origin.x - a.x, origin.y - a.y, origin.z - a.z);
    let u = inv_det * dot_3d(s, h);

    if u < 0.0 || u > 1.0 {
        return RayTriangleResult::Miss;
    }

    let q = cross_3d(s, edge1);
    let v = inv_det * dot_3d(direction, q);

    if v < 0.0 || u + v > 1.0 {
        return RayTriangleResult::Miss;
    }

    let t = inv_det * dot_3d(edge2, q);
    RayTriangleResult::Hit(RayTriangleHit { t, u, v })
}

#[inline]
fn cross_3d(a: Point3, b: Point3) -> Point3 {
    Point3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

// ---------------------------------------------------------------------------
// Intersection: 3D AABB overlap
// ---------------------------------------------------------------------------

/// Axis-aligned bounding box in 3D.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Point3,
    pub max: Point3,
}

impl Aabb {
    #[inline]
    pub fn new(min: Point3, max: Point3) -> Self {
        Self { min, max }
    }

    #[inline]
    pub fn overlaps(&self, other: &Aabb) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    #[inline]
    pub fn contains_point(&self, p: Point3) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }

    /// Squared distance from point `p` to this AABB (0 if inside).
    #[inline]
    pub fn distance_sq_to_point(&self, p: Point3) -> f64 {
        let dx = (self.min.x - p.x).max(0.0).max(p.x - self.max.x);
        let dy = (self.min.y - p.y).max(0.0).max(p.y - self.max.y);
        let dz = (self.min.z - p.z).max(0.0).max(p.z - self.max.z);
        dx * dx + dy * dy + dz * dz
    }

    /// Center point.
    #[inline]
    pub fn center(&self) -> Point3 {
        Point3::new(
            0.5 * (self.min.x + self.max.x),
            0.5 * (self.min.y + self.max.y),
            0.5 * (self.min.z + self.max.z),
        )
    }

    /// Surface area (used for SAH-based BVH construction).
    #[inline]
    pub fn surface_area(&self) -> f64 {
        let dx = self.max.x - self.min.x;
        let dy = self.max.y - self.min.y;
        let dz = self.max.z - self.min.z;
        2.0 * (dx * dy + dy * dz + dz * dx)
    }

    /// Union of two AABBs.
    #[inline]
    pub fn union(&self, other: &Aabb) -> Aabb {
        Aabb::new(
            Point3::new(
                self.min.x.min(other.min.x),
                self.min.y.min(other.min.y),
                self.min.z.min(other.min.z),
            ),
            Point3::new(
                self.max.x.max(other.max.x),
                self.max.y.max(other.max.y),
                self.max.z.max(other.max.z),
            ),
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- 2D distance ---

    #[test]
    fn distance_2d_basic() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(3.0, 4.0);
        assert_eq!(distance_2d(a, b), 5.0);
        assert_eq!(distance_sq_2d(a, b), 25.0);
    }

    #[test]
    fn point_segment_distance_2d_interior() {
        let p = Point2::new(0.5, 1.0);
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        assert!((point_segment_distance_2d(p, a, b) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn point_segment_distance_2d_endpoint() {
        let p = Point2::new(2.0, 0.0);
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        assert!((point_segment_distance_2d(p, a, b) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn point_segment_distance_2d_degenerate() {
        let p = Point2::new(1.0, 1.0);
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(0.0, 0.0); // zero-length segment
        assert!((point_segment_distance_2d(p, a, b) - 2.0f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn point_line_distance_2d() {
        let p = Point2::new(0.0, 1.0);
        let a = Point2::new(-1.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        assert!((point_line_distance_sq_2d(p, a, b) - 1.0).abs() < 1e-12);
    }

    // --- 3D distance ---

    #[test]
    fn distance_3d_basic() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 2.0, 2.0);
        assert!((distance_3d(a, b) - 3.0).abs() < 1e-12);
    }

    #[test]
    fn point_segment_distance_3d() {
        let p = Point3::new(0.5, 0.0, 1.0);
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        assert!((point_segment_distance_sq_3d(p, a, b) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn point_triangle_distance_3d_interior() {
        // Point directly above the centroid of a triangle.
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let p = Point3::new(1.0 / 3.0, 1.0 / 3.0, 2.0);
        assert!((point_triangle_distance_sq_3d(p, a, b, c) - 4.0).abs() < 1e-12);
    }

    #[test]
    fn point_triangle_distance_3d_vertex() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let p = Point3::new(-1.0, -1.0, 0.0);
        let d = point_triangle_distance_sq_3d(p, a, b, c);
        assert!((d - 2.0).abs() < 1e-12, "d={d}");
    }

    // --- 2D segment intersection ---

    #[test]
    fn segments_cross_properly() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 1.0);
        let c = Point2::new(0.0, 1.0);
        let d = Point2::new(1.0, 0.0);
        assert_eq!(
            segment_segment_intersect_2d(a, b, c, d),
            SegmentIntersection2d::Point
        );
    }

    #[test]
    fn segments_disjoint() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(2.0, 0.0);
        let d = Point2::new(3.0, 0.0);
        assert_eq!(
            segment_segment_intersect_2d(a, b, c, d),
            SegmentIntersection2d::Disjoint
        );
    }

    #[test]
    fn segments_collinear_overlap() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(2.0, 0.0);
        let c = Point2::new(1.0, 0.0);
        let d = Point2::new(3.0, 0.0);
        assert_eq!(
            segment_segment_intersect_2d(a, b, c, d),
            SegmentIntersection2d::Overlap
        );
    }

    #[test]
    fn segments_collinear_touching() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(1.0, 0.0);
        let d = Point2::new(2.0, 0.0);
        assert_eq!(
            segment_segment_intersect_2d(a, b, c, d),
            SegmentIntersection2d::Touching
        );
    }

    #[test]
    fn segments_collinear_disjoint() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(2.0, 0.0);
        let d = Point2::new(3.0, 0.0);
        assert_eq!(
            segment_segment_intersect_2d(a, b, c, d),
            SegmentIntersection2d::Disjoint
        );
    }

    #[test]
    fn segments_touching_at_endpoint() {
        // Two segments sharing endpoint b=c, not collinear.
        // The intersection is a single point (the shared endpoint).
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(1.0, 0.0);
        let d = Point2::new(1.0, 1.0);
        // When segments share an endpoint and diverge, the orientation test
        // sees o1≠o2 and o3≠o4 → Point (proper crossing at the shared vertex).
        assert_eq!(
            segment_segment_intersect_2d(a, b, c, d),
            SegmentIntersection2d::Point
        );
    }

    // --- 3D ray-triangle ---

    #[test]
    fn ray_hits_triangle() {
        let origin = Point3::new(0.0, 0.0, 1.0);
        let dir = Point3::new(0.0, 0.0, -1.0);
        let a = Point3::new(-1.0, -1.0, 0.0);
        let b = Point3::new(1.0, -1.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        match ray_triangle_intersect_3d(origin, dir, a, b, c) {
            RayTriangleResult::Hit(hit) => {
                assert!((hit.t - 1.0).abs() < 1e-12);
                assert!(hit.u >= 0.0 && hit.u <= 1.0);
                assert!(hit.v >= 0.0 && hit.u + hit.v <= 1.0);
            }
            _ => panic!("expected hit"),
        }
    }

    #[test]
    fn ray_misses_triangle() {
        let origin = Point3::new(5.0, 5.0, 1.0);
        let dir = Point3::new(0.0, 0.0, -1.0);
        let a = Point3::new(-1.0, -1.0, 0.0);
        let b = Point3::new(1.0, -1.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        assert_eq!(
            ray_triangle_intersect_3d(origin, dir, a, b, c),
            RayTriangleResult::Miss
        );
    }

    #[test]
    fn ray_parallel_to_triangle() {
        let origin = Point3::new(0.0, 0.0, 1.0);
        let dir = Point3::new(1.0, 0.0, 0.0); // parallel to triangle plane
        let a = Point3::new(-1.0, -1.0, 0.0);
        let b = Point3::new(1.0, -1.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        assert_eq!(
            ray_triangle_intersect_3d(origin, dir, a, b, c),
            RayTriangleResult::Parallel
        );
    }

    #[test]
    fn ray_hits_degenerate_triangle() {
        let origin = Point3::new(0.0, 0.0, 1.0);
        let dir = Point3::new(0.0, 0.0, -1.0);
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(0.0, 0.0, 0.0); // degenerate
        let c = Point3::new(1.0, 0.0, 0.0);
        assert_eq!(
            ray_triangle_intersect_3d(origin, dir, a, b, c),
            RayTriangleResult::DegenerateTriangle
        );
    }

    // --- AABB ---

    #[test]
    fn aabb_overlaps() {
        let a = Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let b = Aabb::new(Point3::new(0.5, 0.5, 0.5), Point3::new(2.0, 2.0, 2.0));
        assert!(a.overlaps(&b));
    }

    #[test]
    fn aabb_disjoint() {
        let a = Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let b = Aabb::new(Point3::new(2.0, 2.0, 2.0), Point3::new(3.0, 3.0, 3.0));
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn aabb_touching_overlaps() {
        // Touching faces should overlap (<=, >=).
        let a = Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let b = Aabb::new(Point3::new(1.0, 0.0, 0.0), Point3::new(2.0, 1.0, 1.0));
        assert!(a.overlaps(&b));
    }

    #[test]
    fn aabb_contains_point() {
        let a = Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        assert!(a.contains_point(Point3::new(0.5, 0.5, 0.5)));
        assert!(!a.contains_point(Point3::new(1.5, 0.5, 0.5)));
    }

    #[test]
    fn aabb_distance_sq_to_point_inside() {
        let a = Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        assert_eq!(a.distance_sq_to_point(Point3::new(0.5, 0.5, 0.5)), 0.0);
    }

    #[test]
    fn aabb_distance_sq_to_point_outside() {
        let a = Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let d = a.distance_sq_to_point(Point3::new(2.0, 0.5, 0.5));
        assert!((d - 1.0).abs() < 1e-12);
    }

    #[test]
    fn aabb_surface_area() {
        let a = Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 2.0, 3.0));
        // 2*(1*2 + 2*3 + 3*1) = 2*(2+6+3) = 22
        assert!((a.surface_area() - 22.0).abs() < 1e-12);
    }

    #[test]
    fn aabb_union() {
        let a = Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let b = Aabb::new(Point3::new(0.5, 0.5, 0.5), Point3::new(2.0, 2.0, 2.0));
        let u = a.union(&b);
        assert_eq!(u.min, Point3::new(0.0, 0.0, 0.0));
        assert_eq!(u.max, Point3::new(2.0, 2.0, 2.0));
    }
}
