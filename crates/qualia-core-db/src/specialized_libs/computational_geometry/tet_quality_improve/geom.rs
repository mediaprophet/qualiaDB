//! Low-level geometric primitives: vector algebra on `Point3`, signed volume,
//! and tet circumcenter. `Point3` exposes only `new`, so these helpers stand
//! in for the arithmetic the rest of the module relies on.

use super::*;

// ---------------------------------------------------------------------------
//  Vector helpers (private; Point3 has only `new`)
// ---------------------------------------------------------------------------

#[inline]
pub(super) fn sub(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
#[inline]
pub(super) fn add(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
#[inline]
pub(super) fn scale(a: Point3, s: f64) -> Point3 {
    Point3::new(a.x * s, a.y * s, a.z * s)
}
#[inline]
pub(super) fn dot(a: Point3, b: Point3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}
#[inline]
pub(super) fn cross(a: Point3, b: Point3) -> Point3 {
    Point3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}
#[inline]
pub(super) fn norm(a: Point3) -> f64 {
    dot(a, a).sqrt()
}

/// Signed volume of a tet: `det(v1-v0, v2-v0, v3-v0) / 6`. Positive for the
/// standard (positively-oriented) winding.
#[inline]
pub(super) fn signed_volume(a: Point3, b: Point3, c: Point3, d: Point3) -> f64 {
    let v1 = sub(b, a);
    let v2 = sub(c, a);
    let v3 = sub(d, a);
    dot(cross(v1, v2), v3) / 6.0
}

/// Circumcenter of a tet. Returns `None` for a degenerate (coplanar) tet.
pub(super) fn circumcenter(a: Point3, b: Point3, c: Point3, d: Point3) -> Option<Point3> {
    let v1 = sub(b, a);
    let v2 = sub(c, a);
    let v3 = sub(d, a);
    let rhs = Point3::new(0.5 * dot(v1, v1), 0.5 * dot(v2, v2), 0.5 * dot(v3, v3));
    let cr23 = cross(v2, v3);
    let cr31 = cross(v3, v1);
    let cr12 = cross(v1, v2);
    let det_m = dot(v1, cr23);
    if det_m == 0.0 {
        return None;
    }
    let inv_det = 1.0 / det_m;
    let cx = (rhs.x * cr23.x + rhs.y * cr31.x + rhs.z * cr12.x) * inv_det;
    let cy = (rhs.x * cr23.y + rhs.y * cr31.y + rhs.z * cr12.y) * inv_det;
    let cz = (rhs.x * cr23.z + rhs.y * cr31.z + rhs.z * cr12.z) * inv_det;
    Some(Point3::new(a.x + cx, a.y + cy, a.z + cz))
}
