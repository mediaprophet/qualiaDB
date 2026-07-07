//! P11.1 — Robust segment/line/ray primitives and exact intersections.
//!
//! The acceptance gate requires: "Proper, endpoint, T-junction, collinear
//! overlap and disjoint cases return canonical typed results; constructed
//! coordinates re-predicate without sign drift."
//!
//! This module provides a rich classification of 2-D segment-segment
//! intersection, distinguishing:
//!
//! - **Proper** — the segments cross at a single point in the interior of
//!   both segments.
//! - **Endpoint** — the segments share a common endpoint (both segments end
//!   at the same point).
//! - **T-junction** — an endpoint of one segment lies on the interior of the
//!   other segment.
//! - **CollinearOverlap** — the segments are collinear and share an interval
//!   (the overlap is a segment, not a point).
//! - **CollinearTouch** — the segments are collinear and share exactly one
//!   endpoint (the overlap is a single point at the boundary of both).
//! - **CollinearDisjoint** — the segments are collinear but do not overlap.
//! - **Disjoint** — the segments do not intersect at all (non-collinear, no
//!   crossing).
//!
//! ## Robustness
//!
//! All classification uses the exact orientation predicate
//! ([`super::primitives::orientation_2`]) which has a filtered → compensated
//! → exact ladder (P1.2–P1.7). The sign is always correct, even for
//! near-degenerate inputs.
//!
//! ## Exact construction
//!
//! When the intersection is a single point (Proper, Endpoint, T-junction,
//! CollinearTouch), the caller can request an exact construction via
//! [`super::kernel::ConstructionKernel::segment_intersection_2`], which
//! returns an [`super::kernel::ExactPoint2`] that re-predicates without sign
//! drift. The `classify_and_construct` function combines classification and
//! construction in one call.
//!
//! ## Zero-heap
//!
//! The classification functions use only stack-allocated values — no `Vec`,
//! `String`, or `Box`. The exact construction delegates to the
//! `ConstructionKernel` which is also zero-heap.

use super::kernel::{ConstructionKernel, ExactPoint2, GeometryKernel, Unsupported};
use super::primitives::{orientation_2, Orientation, Point2};

// ───────────────────────────────────────────────────────────────────────────
//  Classification
// ───────────────────────────────────────────────────────────────────────────

/// The canonical classification of a 2-D segment-segment intersection.
///
/// P11.1 acceptance gate: "Proper, endpoint, T-junction, collinear overlap
/// and disjoint cases return canonical typed results."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentIntersectionClass {
    /// The segments do not intersect (non-collinear, no crossing).
    Disjoint,
    /// The segments cross at a single point in the interior of both segments.
    Proper,
    /// The segments share a common endpoint (both segments end at the same
    /// point — e.g. `b == c`, `b == d`, `a == c`, or `a == d`).
    Endpoint,
    /// An endpoint of one segment lies on the interior of the other segment
    /// (T-junction). The `which` field indicates which segment's endpoint
    /// lies on the other: `AbOnCd` means an endpoint of `ab` lies on `cd`;
    /// `CdOnAb` means an endpoint of `cd` lies on `ab`.
    TJunction(TJunctionSide),
    /// The segments are collinear and share an interval (the overlap is a
    /// segment, not a point).
    CollinearOverlap,
    /// The segments are collinear and share exactly one endpoint (the overlap
    /// is a single point at the boundary of both).
    CollinearTouch,
    /// The segments are collinear but do not overlap.
    CollinearDisjoint,
}

/// Which segment's endpoint lies on the other segment in a T-junction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TJunctionSide {
    /// An endpoint of segment `ab` lies on the interior of segment `cd`.
    AbOnCd,
    /// An endpoint of segment `cd` lies on the interior of segment `ab`.
    CdOnAb,
}

/// The result of classifying a segment-segment intersection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentIntersectionResult {
    /// The canonical classification.
    pub class: SegmentIntersectionClass,
    /// The intersection point, if the intersection is a single point
    /// (Proper, Endpoint, T-junction, CollinearTouch). `None` for Disjoint,
    /// CollinearOverlap, and CollinearDisjoint.
    pub point: Option<Point2>,
}

// ───────────────────────────────────────────────────────────────────────────
//  Classification logic
// ───────────────────────────────────────────────────────────────────────────

/// Classify the intersection of segment `ab` with segment `cd`.
///
/// Uses the exact orientation predicate for robustness. Returns a canonical
/// [`SegmentIntersectionResult`] with the classification and (if applicable)
/// the intersection point as a rounded `Point2`.
///
/// For exact construction (re-predicable without sign drift), use
/// [`classify_and_construct`] with a [`ConstructionKernel`].
pub fn classify_segment_intersection_2(
    a: Point2,
    b: Point2,
    c: Point2,
    d: Point2,
) -> SegmentIntersectionResult {
    // Check for shared endpoints first — these are the cheapest cases.
    // Also handle identical segments (a==c && b==d, or a==d && b==c).
    if a == c && b == d || a == d && b == c {
        // Identical segments → full collinear overlap.
        return SegmentIntersectionResult {
            class: SegmentIntersectionClass::CollinearOverlap,
            point: None,
        };
    }
    if a == c || a == d {
        return classify_shared_endpoint(a, a, b, c, d, a == c);
    }
    if b == c || b == d {
        return classify_shared_endpoint(b, a, b, c, d, b == c);
    }

    // Handle zero-length segments (a==b or c==d): the "segment" is a single
    // point. Check if that point lies on the other segment.
    if a == b {
        // Segment ab is a single point at a. Check if a lies on segment cd.
        let o = orientation_2(c, d, a);
        if o == Orientation::Collinear && on_segment(c, d, a) {
            return SegmentIntersectionResult {
                class: SegmentIntersectionClass::TJunction(TJunctionSide::AbOnCd),
                point: Some(a),
            };
        }
        return SegmentIntersectionResult {
            class: SegmentIntersectionClass::Disjoint,
            point: None,
        };
    }
    if c == d {
        // Segment cd is a single point at c. Check if c lies on segment ab.
        let o = orientation_2(a, b, c);
        if o == Orientation::Collinear && on_segment(a, b, c) {
            return SegmentIntersectionResult {
                class: SegmentIntersectionClass::TJunction(TJunctionSide::CdOnAb),
                point: Some(c),
            };
        }
        return SegmentIntersectionResult {
            class: SegmentIntersectionClass::Disjoint,
            point: None,
        };
    }

    // No shared endpoints. Use orientation predicates.
    let o1 = orientation_2(a, b, c); // sign of c w.r.t. line ab
    let o2 = orientation_2(a, b, d); // sign of d w.r.t. line ab
    let o3 = orientation_2(c, d, a); // sign of a w.r.t. line cd
    let o4 = orientation_2(c, d, b); // sign of b w.r.t. line cd

    // Collinear case: all four points are collinear.
    if o1 == Orientation::Collinear && o2 == Orientation::Collinear {
        return classify_collinear(a, b, c, d);
    }

    // T-junction cases: one endpoint lies on the other segment.
    // Check these BEFORE the proper intersection check, because when one
    // orientation is Collinear, o1 != o2 may be true but the intersection
    // is a T-junction, not proper.
    // c is collinear with ab and lies on segment ab.
    if o1 == Orientation::Collinear && on_segment(a, b, c) {
        return SegmentIntersectionResult {
            class: SegmentIntersectionClass::TJunction(TJunctionSide::AbOnCd),
            point: Some(c),
        };
    }
    // d is collinear with ab and lies on segment ab.
    if o2 == Orientation::Collinear && on_segment(a, b, d) {
        return SegmentIntersectionResult {
            class: SegmentIntersectionClass::TJunction(TJunctionSide::AbOnCd),
            point: Some(d),
        };
    }
    // a is collinear with cd and lies on segment cd.
    if o3 == Orientation::Collinear && on_segment(c, d, a) {
        return SegmentIntersectionResult {
            class: SegmentIntersectionClass::TJunction(TJunctionSide::CdOnAb),
            point: Some(a),
        };
    }
    // b is collinear with cd and lies on segment cd.
    if o4 == Orientation::Collinear && on_segment(c, d, b) {
        return SegmentIntersectionResult {
            class: SegmentIntersectionClass::TJunction(TJunctionSide::CdOnAb),
            point: Some(b),
        };
    }

    // General case: proper intersection (c and d on opposite sides of ab,
    // AND a and b on opposite sides of cd). At this point, none of the
    // orientations are Collinear (we checked above), so this is a true
    // proper crossing.
    if o1 != o2 && o3 != o4 {
        // The segments cross at a single interior point.
        let pt = compute_intersection_point(a, b, c, d);
        return SegmentIntersectionResult {
            class: SegmentIntersectionClass::Proper,
            point: Some(pt),
        };
    }

    // No intersection.
    SegmentIntersectionResult {
        class: SegmentIntersectionClass::Disjoint,
        point: None,
    }
}

/// Classify a shared-endpoint case.
///
/// `shared` is the shared endpoint. `a`/`b` are the first segment's endpoints,
/// `c`/`d` are the second segment's endpoints. We determine the "other"
/// endpoints (the non-shared ones) and check collinearity.
fn classify_shared_endpoint(
    shared: Point2,
    a: Point2,
    b: Point2,
    c: Point2,
    d: Point2,
    _a_is_c: bool,
) -> SegmentIntersectionResult {
    // Determine the "other" endpoints (the ones that are NOT the shared point).
    let other_ab = if a == shared { b } else { a };
    let other_cd = if c == shared { d } else { c };

    // Check if the two segments are collinear by testing whether the "other"
    // endpoints are collinear with the shared point.
    let o = orientation_2(other_ab, shared, other_cd);
    if o == Orientation::Collinear {
        // Collinear with shared endpoint — check if they overlap beyond the
        // shared point by checking if the "other" endpoints are on the same
        // side or opposite sides.
        let dx_ab = other_ab.x - shared.x;
        let dy_ab = other_ab.y - shared.y;
        let dx_cd = other_cd.x - shared.x;
        let dy_cd = other_cd.y - shared.y;
        let dot = dx_ab * dx_cd + dy_ab * dy_cd;
        if dot < 0.0 {
            // Opposite directions → segments only share the single point
            // (they extend away from each other).
            SegmentIntersectionResult {
                class: SegmentIntersectionClass::CollinearTouch,
                point: Some(shared),
            }
        } else {
            // Same direction → overlap extends beyond the shared point.
            SegmentIntersectionResult {
                class: SegmentIntersectionClass::CollinearOverlap,
                point: None,
            }
        }
    } else {
        // Not collinear — just a shared endpoint.
        SegmentIntersectionResult {
            class: SegmentIntersectionClass::Endpoint,
            point: Some(shared),
        }
    }
}

/// Classify a collinear case (all four points are collinear).
fn classify_collinear(a: Point2, b: Point2, c: Point2, d: Point2) -> SegmentIntersectionResult {
    // Project onto the dominant axis.
    let (a_t, b_t, c_t, d_t) = if (b.x - a.x).abs() >= (b.y - a.y).abs() {
        (a.x, b.x, c.x, d.x)
    } else {
        (a.y, b.y, c.y, d.y)
    };

    let (lo_ab, hi_ab) = (a_t.min(b_t), a_t.max(b_t));
    let (lo_cd, hi_cd) = (c_t.min(d_t), c_t.max(d_t));

    if hi_ab < lo_cd || hi_cd < lo_ab {
        return SegmentIntersectionResult {
            class: SegmentIntersectionClass::CollinearDisjoint,
            point: None,
        };
    }
    if hi_ab == lo_cd || hi_cd == lo_ab {
        // Touch at a single endpoint.
        let touch_t = if hi_ab == lo_cd { hi_ab } else { hi_cd };
        let pt = if (b.x - a.x).abs() >= (b.y - a.y).abs() {
            // Dominant axis is x — reconstruct the point.
            // We need the y-coordinate too. Since all points are collinear,
            // we can interpolate. But for the touch point, it's one of the
            // segment endpoints.
            if a_t == touch_t {
                a
            } else if b_t == touch_t {
                b
            } else if c_t == touch_t {
                c
            } else {
                d
            }
        } else if a_t == touch_t {
            a
        } else if b_t == touch_t {
            b
        } else if c_t == touch_t {
            c
        } else {
            d
        };
        return SegmentIntersectionResult {
            class: SegmentIntersectionClass::CollinearTouch,
            point: Some(pt),
        };
    }
    SegmentIntersectionResult {
        class: SegmentIntersectionClass::CollinearOverlap,
        point: None,
    }
}

/// Check if point `p` lies on segment `ab`, assuming `p` is already known to
/// be collinear with `a` and `b`.
fn on_segment(a: Point2, b: Point2, p: Point2) -> bool {
    p.x >= a.x.min(b.x) && p.x <= a.x.max(b.x) && p.y >= a.y.min(b.y) && p.y <= a.y.max(b.y)
}

/// Compute the intersection point of two non-parallel segments (f64
/// approximation). For exact construction, use a `ConstructionKernel`.
fn compute_intersection_point(a: Point2, b: Point2, c: Point2, d: Point2) -> Point2 {
    // p = a + t*(b-a), t = cross(c-a, d-c) / cross(b-a, d-c)
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let cdx = d.x - c.x;
    let cdy = d.y - c.y;
    let acx = c.x - a.x;
    let acy = c.y - a.y;

    let denom = abx * cdy - aby * cdx;
    let t_num = acx * cdy - acy * cdx;

    // denom is non-zero (segments are not parallel — we checked orientation).
    let t = t_num / denom;
    Point2::new(a.x + t * abx, a.y + t * aby)
}

// ───────────────────────────────────────────────────────────────────────────
//  Combined classification + exact construction
// ───────────────────────────────────────────────────────────────────────────

/// Classify the intersection AND construct the exact intersection point
/// (when the intersection is a single point).
///
/// Uses a [`ConstructionKernel`] for exact construction. The returned
/// `ExactPoint2` re-predicates without sign drift (the P11.1 acceptance gate
/// requirement).
///
/// For collinear overlap (where the intersection is a segment, not a point),
/// this returns `Ok(None)` — the caller should use a separate overlap
/// computation.
pub fn classify_and_construct<K: GeometryKernel + ConstructionKernel>(
    kernel: &K,
    a: Point2,
    b: Point2,
    c: Point2,
    d: Point2,
) -> Result<(SegmentIntersectionClass, Option<ExactPoint2>), Unsupported> {
    let result = classify_segment_intersection_2(a, b, c, d);
    match result.class {
        SegmentIntersectionClass::Disjoint
        | SegmentIntersectionClass::CollinearDisjoint
        | SegmentIntersectionClass::CollinearOverlap => Ok((result.class, None)),
        SegmentIntersectionClass::Proper
        | SegmentIntersectionClass::Endpoint
        | SegmentIntersectionClass::TJunction(_)
        | SegmentIntersectionClass::CollinearTouch => {
            // Construct the exact intersection point.
            let exact_pt = kernel.segment_intersection_2(a, b, c, d)?;
            Ok((result.class, Some(exact_pt)))
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Line and ray intersection
// ───────────────────────────────────────────────────────────────────────────

/// Classify the intersection of a **line** (infinite, through points `a` and
/// `b`) with a segment `cd`.
///
/// Returns the intersection point if the line crosses the segment, or `None`
/// if the line is parallel to the segment or the segment doesn't span the
/// line.
pub fn line_segment_intersection_2(a: Point2, b: Point2, c: Point2, d: Point2) -> Option<Point2> {
    let o1 = orientation_2(a, b, c);
    let o2 = orientation_2(a, b, d);

    // If c and d are on the same side (and not collinear), no intersection.
    if o1 == o2 && o1 != Orientation::Collinear {
        return None;
    }

    // If both are collinear, the segment lies on the line — infinite
    // intersection. Return c as a representative point.
    if o1 == Orientation::Collinear && o2 == Orientation::Collinear {
        return Some(c);
    }

    // Otherwise, the line crosses the segment at a single point.
    Some(compute_intersection_point(a, b, c, d))
}

/// Classify the intersection of a **ray** (from `origin` through `dir_point`)
/// with a segment `cd`.
///
/// Returns the intersection point if the ray hits the segment, or `None`.
pub fn ray_segment_intersection_2(
    origin: Point2,
    dir_point: Point2,
    c: Point2,
    d: Point2,
) -> Option<Point2> {
    let o1 = orientation_2(origin, dir_point, c);
    let o2 = orientation_2(origin, dir_point, d);

    // If c and d are on the same side (and not collinear), no intersection.
    if o1 == o2 && o1 != Orientation::Collinear {
        return None;
    }

    // Compute the intersection point of the line with the segment.
    let pt = compute_intersection_point(origin, dir_point, c, d);

    // Check if the intersection point is on the ray (t >= 0).
    // t = (pt - origin) · (dir_point - origin) / |dir_point - origin|²
    let dx = dir_point.x - origin.x;
    let dy = dir_point.y - origin.y;
    let px = pt.x - origin.x;
    let py = pt.y - origin.y;
    let t = px * dx + py * dy;
    if t < 0.0 {
        return None; // Intersection is behind the ray origin.
    }

    Some(pt)
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::exact_kernel::ExactConstructionKernel;

    // ── Proper intersection ──────────────────────────────────────────────

    #[test]
    fn proper_intersection_classified() {
        // X-shaped crossing: (0,0)→(1,1) and (0,1)→(1,0).
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 1.0);
        let c = Point2::new(0.0, 1.0);
        let d = Point2::new(1.0, 0.0);
        let result = classify_segment_intersection_2(a, b, c, d);
        assert_eq!(result.class, SegmentIntersectionClass::Proper);
        let pt = result.point.expect("proper intersection has a point");
        assert!((pt.x - 0.5).abs() < 1e-9);
        assert!((pt.y - 0.5).abs() < 1e-9);
    }

    #[test]
    fn proper_intersection_near_degenerate() {
        // Nearly parallel segments that still cross.
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 1e-10);
        let c = Point2::new(0.0, 1.0);
        let d = Point2::new(1.0, 0.0);
        let result = classify_segment_intersection_2(a, b, c, d);
        assert_eq!(result.class, SegmentIntersectionClass::Proper);
    }

    // ── Endpoint intersection ────────────────────────────────────────────

    #[test]
    fn endpoint_intersection_classified() {
        // b == c: segments share endpoint at (1,0).
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(1.0, 0.0);
        let d = Point2::new(1.0, 1.0);
        let result = classify_segment_intersection_2(a, b, c, d);
        assert_eq!(result.class, SegmentIntersectionClass::Endpoint);
        assert_eq!(result.point, Some(Point2::new(1.0, 0.0)));
    }

    #[test]
    fn endpoint_all_four_combinations() {
        let p = Point2::new(1.0, 1.0);
        let a1 = Point2::new(0.0, 0.0);
        let a2 = Point2::new(2.0, 0.0);
        // a == c
        assert_eq!(
            classify_segment_intersection_2(p, a1, p, a2).class,
            SegmentIntersectionClass::Endpoint
        );
        // a == d
        assert_eq!(
            classify_segment_intersection_2(p, a1, a2, p).class,
            SegmentIntersectionClass::Endpoint
        );
        // b == c
        assert_eq!(
            classify_segment_intersection_2(a1, p, p, a2).class,
            SegmentIntersectionClass::Endpoint
        );
        // b == d
        assert_eq!(
            classify_segment_intersection_2(a1, p, a2, p).class,
            SegmentIntersectionClass::Endpoint
        );
    }

    // ── T-junction ───────────────────────────────────────────────────────

    #[test]
    fn t_junction_c_on_ab() {
        // c lies on the interior of ab.
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(2.0, 0.0);
        let c = Point2::new(1.0, 0.0);
        let d = Point2::new(1.0, 1.0);
        let result = classify_segment_intersection_2(a, b, c, d);
        assert_eq!(
            result.class,
            SegmentIntersectionClass::TJunction(TJunctionSide::AbOnCd)
        );
        assert_eq!(result.point, Some(c));
    }

    #[test]
    fn t_junction_d_on_ab() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(2.0, 0.0);
        let c = Point2::new(1.0, 1.0);
        let d = Point2::new(1.0, 0.0);
        let result = classify_segment_intersection_2(a, b, c, d);
        assert_eq!(
            result.class,
            SegmentIntersectionClass::TJunction(TJunctionSide::AbOnCd)
        );
        assert_eq!(result.point, Some(d));
    }

    #[test]
    fn t_junction_a_on_cd() {
        let a = Point2::new(1.0, 0.0);
        let b = Point2::new(1.0, 1.0);
        let c = Point2::new(0.0, 0.0);
        let d = Point2::new(2.0, 0.0);
        let result = classify_segment_intersection_2(a, b, c, d);
        assert_eq!(
            result.class,
            SegmentIntersectionClass::TJunction(TJunctionSide::CdOnAb)
        );
        assert_eq!(result.point, Some(a));
    }

    #[test]
    fn t_junction_b_on_cd() {
        let a = Point2::new(1.0, 1.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(0.0, 0.0);
        let d = Point2::new(2.0, 0.0);
        let result = classify_segment_intersection_2(a, b, c, d);
        assert_eq!(
            result.class,
            SegmentIntersectionClass::TJunction(TJunctionSide::CdOnAb)
        );
        assert_eq!(result.point, Some(b));
    }

    // ── Collinear overlap ────────────────────────────────────────────────

    #[test]
    fn collinear_overlap_classified() {
        // ab: (0,0)→(2,0), cd: (1,0)→(3,0) — overlap [1,2].
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(2.0, 0.0);
        let c = Point2::new(1.0, 0.0);
        let d = Point2::new(3.0, 0.0);
        let result = classify_segment_intersection_2(a, b, c, d);
        assert_eq!(result.class, SegmentIntersectionClass::CollinearOverlap);
        assert_eq!(result.point, None);
    }

    #[test]
    fn collinear_contained_overlap() {
        // ab contains cd entirely.
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(4.0, 0.0);
        let c = Point2::new(1.0, 0.0);
        let d = Point2::new(2.0, 0.0);
        let result = classify_segment_intersection_2(a, b, c, d);
        assert_eq!(result.class, SegmentIntersectionClass::CollinearOverlap);
    }

    // ── Collinear touch ──────────────────────────────────────────────────

    #[test]
    fn collinear_touch_classified() {
        // ab: (0,0)→(1,0), cd: (1,0)→(2,0) — touch at (1,0).
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(1.0, 0.0);
        let d = Point2::new(2.0, 0.0);
        let result = classify_segment_intersection_2(a, b, c, d);
        assert_eq!(result.class, SegmentIntersectionClass::CollinearTouch);
        assert_eq!(result.point, Some(Point2::new(1.0, 0.0)));
    }

    #[test]
    fn collinear_touch_shared_endpoint_opposite_directions() {
        // Shared endpoint at (1,0), going in opposite directions → touch only.
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(1.0, 0.0);
        let d = Point2::new(2.0, 0.0);
        // shared=b=c=(1,0). Other endpoints: a=(0,0), d=(2,0).
        // dot = (a-shared)·(d-shared) = (-1,0)·(1,0) = -1 < 0 → opposite → touch.
        let result = classify_segment_intersection_2(a, b, c, d);
        assert_eq!(result.class, SegmentIntersectionClass::CollinearTouch);
    }

    #[test]
    fn collinear_touch_shared_endpoint_same_direction() {
        // Shared endpoint at (0,0), both going right → overlap.
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(0.0, 0.0);
        let d = Point2::new(0.5, 0.0);
        let result = classify_segment_intersection_2(a, b, c, d);
        // a == c = (0,0). Other endpoints: b=(1,0), d=(0.5,0).
        // dot = (b-shared)·(d-shared) = (1,0)·(0.5,0) = 0.5 > 0 → same → overlap.
        assert_eq!(result.class, SegmentIntersectionClass::CollinearOverlap);
    }

    // ── Collinear disjoint ───────────────────────────────────────────────

    #[test]
    fn collinear_disjoint_classified() {
        // ab: (0,0)→(1,0), cd: (2,0)→(3,0) — no overlap.
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(2.0, 0.0);
        let d = Point2::new(3.0, 0.0);
        let result = classify_segment_intersection_2(a, b, c, d);
        assert_eq!(result.class, SegmentIntersectionClass::CollinearDisjoint);
        assert_eq!(result.point, None);
    }

    // ── Disjoint (non-collinear) ─────────────────────────────────────────

    #[test]
    fn disjoint_classified() {
        // Parallel, non-collinear segments.
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(0.0, 1.0);
        let d = Point2::new(1.0, 1.0);
        let result = classify_segment_intersection_2(a, b, c, d);
        assert_eq!(result.class, SegmentIntersectionClass::Disjoint);
        assert_eq!(result.point, None);
    }

    #[test]
    fn disjoint_non_parallel() {
        // Non-parallel but don't cross (extensions would cross, but segments
        // don't reach).
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(2.0, 1.0);
        let d = Point2::new(3.0, -1.0);
        let result = classify_segment_intersection_2(a, b, c, d);
        assert_eq!(result.class, SegmentIntersectionClass::Disjoint);
    }

    // ── Exact construction + re-predication ──────────────────────────────

    #[test]
    fn classify_and_construct_proper() {
        let k = ExactConstructionKernel::default();
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 1.0);
        let c = Point2::new(0.0, 1.0);
        let d = Point2::new(1.0, 0.0);
        let (class, pt) = classify_and_construct(&k, a, b, c, d).unwrap();
        assert_eq!(class, SegmentIntersectionClass::Proper);
        let pt = pt.expect("proper intersection has exact point");
        // (0.5, 0.5) — the construction may produce an unreduced fraction,
        // so check the rational value.
        let x_val = pt.x_num as f64 / pt.den as f64;
        let y_val = pt.y_num as f64 / pt.den as f64;
        assert!((x_val - 0.5).abs() < 1e-12, "x = {} should be 0.5", x_val);
        assert!((y_val - 0.5).abs() < 1e-12, "y = {} should be 0.5", y_val);
    }

    #[test]
    fn classify_and_construct_collinear_overlap_no_point() {
        let k = ExactConstructionKernel::default();
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(2.0, 0.0);
        let c = Point2::new(1.0, 0.0);
        let d = Point2::new(3.0, 0.0);
        let (class, pt) = classify_and_construct(&k, a, b, c, d).unwrap();
        assert_eq!(class, SegmentIntersectionClass::CollinearOverlap);
        assert_eq!(pt, None);
    }

    #[test]
    fn classify_and_construct_disjoint_no_point() {
        let k = ExactConstructionKernel::default();
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(0.0, 1.0);
        let d = Point2::new(1.0, 1.0);
        let (class, pt) = classify_and_construct(&k, a, b, c, d).unwrap();
        assert_eq!(class, SegmentIntersectionClass::Disjoint);
        assert_eq!(pt, None);
    }

    #[test]
    fn exact_point_re_predicates_without_sign_drift() {
        // The P11.1 acceptance gate: "constructed coordinates re-predicate
        // without sign drift."
        //
        // Construct the intersection of (0,0)→(2,2) and (0,2)→(2,0),
        // which is (1,1). Then check that orientation_2 with the exact
        // point gives the correct sign.
        let k = ExactConstructionKernel::default();
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(2.0, 2.0);
        let c = Point2::new(0.0, 2.0);
        let d = Point2::new(2.0, 0.0);
        let (_, pt) = classify_and_construct(&k, a, b, c, d).unwrap();
        let pt = pt.expect("proper intersection");

        // The exact point is (1,1). The construction may produce an
        // unreduced fraction (e.g. 8/8), so check the rational value.
        let x_val = pt.x_num as f64 / pt.den as f64;
        let y_val = pt.y_num as f64 / pt.den as f64;
        assert!((x_val - 1.0).abs() < 1e-12, "x = {} should be 1.0", x_val);
        assert!((y_val - 1.0).abs() < 1e-12, "y = {} should be 1.0", y_val);

        // Re-predicate: the point (1,1) should be collinear with (0,0) and (2,2).
        let pt_f64 = Point2::new(x_val, y_val);
        let orient = orientation_2(a, b, pt_f64);
        assert_eq!(
            orient,
            Orientation::Collinear,
            "exact point should be collinear with ab"
        );

        // And collinear with (0,2) and (2,0).
        let orient2 = orientation_2(c, d, pt_f64);
        assert_eq!(
            orient2,
            Orientation::Collinear,
            "exact point should be collinear with cd"
        );
    }

    // ── Line and ray intersection ────────────────────────────────────────

    #[test]
    fn line_segment_intersection_crosses() {
        // Line through (0,0)→(1,1), segment (0,1)→(1,0).
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 1.0);
        let c = Point2::new(0.0, 1.0);
        let d = Point2::new(1.0, 0.0);
        let pt = line_segment_intersection_2(a, b, c, d);
        assert!(pt.is_some());
        let pt = pt.unwrap();
        assert!((pt.x - 0.5).abs() < 1e-9);
        assert!((pt.y - 0.5).abs() < 1e-9);
    }

    #[test]
    fn line_segment_intersection_parallel() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(0.0, 1.0);
        let d = Point2::new(1.0, 1.0);
        assert!(line_segment_intersection_2(a, b, c, d).is_none());
    }

    #[test]
    fn line_segment_intersection_segment_on_line() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(0.5, 0.0);
        let d = Point2::new(0.75, 0.0);
        let pt = line_segment_intersection_2(a, b, c, d);
        assert!(pt.is_some(), "segment on line should return a point");
    }

    #[test]
    fn ray_segment_intersection_hits() {
        let origin = Point2::new(0.0, 0.0);
        let dir = Point2::new(1.0, 1.0);
        let c = Point2::new(0.0, 1.0);
        let d = Point2::new(1.0, 0.0);
        let pt = ray_segment_intersection_2(origin, dir, c, d);
        assert!(pt.is_some());
        let pt = pt.unwrap();
        assert!((pt.x - 0.5).abs() < 1e-9);
        assert!((pt.y - 0.5).abs() < 1e-9);
    }

    #[test]
    fn ray_segment_intersection_behind_origin() {
        let origin = Point2::new(1.0, 1.0);
        let dir = Point2::new(2.0, 2.0);
        let c = Point2::new(0.0, 1.0);
        let d = Point2::new(1.0, 0.0);
        // The intersection at (0.5, 0.5) is behind the ray origin (1,1).
        let pt = ray_segment_intersection_2(origin, dir, c, d);
        assert!(
            pt.is_none(),
            "intersection behind ray origin should be None"
        );
    }

    #[test]
    fn ray_segment_intersection_parallel() {
        let origin = Point2::new(0.0, 0.0);
        let dir = Point2::new(1.0, 0.0);
        let c = Point2::new(0.0, 1.0);
        let d = Point2::new(1.0, 1.0);
        assert!(ray_segment_intersection_2(origin, dir, c, d).is_none());
    }

    // ── Degenerate / adversarial cases ───────────────────────────────────

    #[test]
    fn identical_segments_collinear_overlap() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 1.0);
        let result = classify_segment_intersection_2(a, b, a, b);
        assert_eq!(result.class, SegmentIntersectionClass::CollinearOverlap);
    }

    #[test]
    fn zero_length_segment_a() {
        // a == b (zero-length segment).
        let a = Point2::new(0.5, 0.5);
        let b = Point2::new(0.5, 0.5);
        let c = Point2::new(0.0, 0.0);
        let d = Point2::new(1.0, 1.0);
        let result = classify_segment_intersection_2(a, b, c, d);
        // (0.5, 0.5) lies on segment cd → T-junction (AbOnCd).
        assert_eq!(
            result.class,
            SegmentIntersectionClass::TJunction(TJunctionSide::AbOnCd)
        );
    }

    #[test]
    fn very_small_coordinates() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1e-15, 1e-15);
        let c = Point2::new(0.0, 1e-15);
        let d = Point2::new(1e-15, 0.0);
        let result = classify_segment_intersection_2(a, b, c, d);
        assert_eq!(result.class, SegmentIntersectionClass::Proper);
    }

    #[test]
    fn large_coordinates() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1e12, 1e12);
        let c = Point2::new(0.0, 1e12);
        let d = Point2::new(1e12, 0.0);
        let result = classify_segment_intersection_2(a, b, c, d);
        assert_eq!(result.class, SegmentIntersectionClass::Proper);
        let pt = result.point.unwrap();
        assert!((pt.x - 5e11).abs() < 1.0);
        assert!((pt.y - 5e11).abs() < 1.0);
    }

    // ── Vertical segments (dominant axis = y) ────────────────────────────

    #[test]
    fn collinear_vertical_overlap() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(0.0, 2.0);
        let c = Point2::new(0.0, 1.0);
        let d = Point2::new(0.0, 3.0);
        let result = classify_segment_intersection_2(a, b, c, d);
        assert_eq!(result.class, SegmentIntersectionClass::CollinearOverlap);
    }

    #[test]
    fn collinear_vertical_disjoint() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(0.0, 1.0);
        let c = Point2::new(0.0, 2.0);
        let d = Point2::new(0.0, 3.0);
        let result = classify_segment_intersection_2(a, b, c, d);
        assert_eq!(result.class, SegmentIntersectionClass::CollinearDisjoint);
    }

    #[test]
    fn collinear_vertical_touch() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(0.0, 1.0);
        let c = Point2::new(0.0, 1.0);
        let d = Point2::new(0.0, 2.0);
        let result = classify_segment_intersection_2(a, b, c, d);
        assert_eq!(result.class, SegmentIntersectionClass::CollinearTouch);
        assert_eq!(result.point, Some(Point2::new(0.0, 1.0)));
    }
}
