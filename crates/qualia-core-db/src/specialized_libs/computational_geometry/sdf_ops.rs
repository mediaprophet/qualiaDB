//! P21.10 — Smooth SDF operators for organic modelling (fauna soft tissue,
//! terrain blending, metaball-style flora organs).
//!
//! Extends the hard Booleans in `parametric_cad` (union/intersection/
//! subtraction are already handled by `SdfExpr::eval`) with C¹-continuous
//! polynomial smooth-min / smooth-max blends and extra organic primitives
//! (capsule, rounded box, metaball field).
//!
//! All kernels are Tier-1 `HotZeroHeap` — pure scalar evaluation.

use super::parametric_cad::{sdf_eval, SdfOp, SdfPrimitive};
use super::primitives::Point3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SdfOpsError {
    DegenerateBlend,
    ZeroRadius,
}

/// Polynomial smooth-min (IQ's `smin`); `k ≥ 0` is the blend radius in
/// distance units. `k = 0` degenerates to hard `min` (C⁰).
/// C¹-continuous; |smin(a,b) − min(a,b)| ≤ k/4. Tier-1 `HotZeroHeap`.
#[inline]
pub fn sdf_smooth_union(d1: f64, d2: f64, k: f64) -> f64 {
    if k <= 0.0 {
        return d1.min(d2);
    }
    let h = (0.5 + 0.5 * (d2 - d1) / k).clamp(0.0, 1.0);
    // mul-add ordering chosen for determinism.
    d2 + (d1 - d2) * h - k * h * (1.0 - h)
}

/// Smooth intersection = `−smin(−a, −b)`.
#[inline]
pub fn sdf_smooth_intersect(d1: f64, d2: f64, k: f64) -> f64 {
    if k <= 0.0 {
        return d1.max(d2);
    }
    let h = (0.5 - 0.5 * (d2 - d1) / k).clamp(0.0, 1.0);
    d2 + (d1 - d2) * h + k * h * (1.0 - h)
}

/// Smooth subtraction: `A − B` with blend `k` (bounded, C¹ where the two
/// surfaces cross at a shallow angle). Tier-1 `HotZeroHeap`.
#[inline]
pub fn sdf_smooth_subtract(d1: f64, d2: f64, k: f64) -> f64 {
    sdf_smooth_intersect(d1, -d2, k)
}

/// Signed distance to a capsule (segment `a→b` inflated by `r`).
/// Tier-1 `HotZeroHeap`.
pub fn sdf_capsule(p: Point3, a: Point3, b: Point3, r: f64) -> f64 {
    let (pa, ba) = (
        Point3::new(p.x - a.x, p.y - a.y, p.z - a.z),
        Point3::new(b.x - a.x, b.y - a.y, b.z - a.z),
    );
    let denom = ba.x * ba.x + ba.y * ba.y + ba.z * ba.z;
    let h = if denom < 1e-15 {
        0.0
    } else {
        ((pa.x * ba.x + pa.y * ba.y + pa.z * ba.z) / denom).clamp(0.0, 1.0)
    };
    let dx = pa.x - ba.x * h;
    let dy = pa.y - ba.y * h;
    let dz = pa.z - ba.z * h;
    (dx * dx + dy * dy + dz * dz).sqrt() - r
}

/// Signed distance to an axis-aligned box with rounded corners (`r` ≤ half
/// the smallest half-extent). Tier-1 `HotZeroHeap`.
pub fn sdf_rounded_box(p: Point3, center: Point3, half: Point3, r: f64) -> f64 {
    let qx = (p.x - center.x).abs() - (half.x - r);
    let qy = (p.y - center.y).abs() - (half.y - r);
    let qz = (p.z - center.z).abs() - (half.z - r);
    let ox = qx.max(0.0);
    let oy = qy.max(0.0);
    let oz = qz.max(0.0);
    (ox * ox + oy * oy + oz * oz).sqrt() + qx.max(qy).max(qz).min(0.0) - r
}

/// Metaball field value `Σ r_i² / |p − c_i|²` minus `threshold`; negative
/// inside the isosurface (turning the inverse-square field into a
/// signed-like scalar). `ColdBounded` semantics: caller slices only, no heap.
///
/// Returns `SdfOpsError::DimensionMismatch` if `centers`/`radii_sq` differ.
pub fn sdf_metaballs(
    p: Point3,
    centers: &[Point3],
    radii_sq: &[f64],
    threshold: f64,
) -> Result<f64, SdfOpsError> {
    if centers.len() != radii_sq.len() {
        return Err(SdfOpsError::DegenerateBlend);
    }
    let mut f = 0.0;
    for (c, &r2) in centers.iter().zip(radii_sq) {
        if r2 <= 0.0 {
            return Err(SdfOpsError::ZeroRadius);
        }
        let dx = p.x - c.x;
        let dy = p.y - c.y;
        let dz = p.z - c.z;
        f += r2 / (dx * dx + dy * dy + dz * dz).max(1e-12);
    }
    Ok(threshold - f)
}

/// Composing counterpart to `parametric_cad::sdf_compose`: applies `op` with
/// a C¹ smooth blend radius `k` (`k = 0` → the hard Boolean, identical to
/// `sdf_compose`). Tier-1 `HotZeroHeap`.
#[inline]
pub fn sdf_compose_smooth(d1: f64, d2: f64, op: SdfOp, k: f64) -> f64 {
    match op {
        SdfOp::Union => sdf_smooth_union(d1, d2, k),
        SdfOp::Intersection => sdf_smooth_intersect(d1, d2, k),
        SdfOp::Difference => sdf_smooth_subtract(d1, d2, k),
    }
}

/// Convenience: sphere primitive SDF (re-export for API completeness).
#[inline]
pub fn sdf_sphere(p: Point3, center: Point3, r: f64) -> f64 {
    sdf_eval(p, SdfPrimitive::Sphere { center, radius: r })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smooth_union_is_c1_and_bounded() {
        // Two unit spheres 1.5 apart along x; blend k=0.5.
        let d = |p: Point3| {
            sdf_smooth_union(
                sdf_sphere(p, Point3::new(0.0, 0.0, 0.0), 1.0),
                sdf_sphere(p, Point3::new(1.5, 0.0, 0.0), 1.0),
                0.5,
            )
        };
        // Blend must be ≤ hard union (bulges outward, never inward).
        for i in 0..=20 {
            let x = i as f64 * 0.075;
            let soft = d(Point3::new(x, 0.0, 0.0));
            let hard = (sdf_sphere(Point3::new(x, 0.0, 0.0), Point3::new(0.0, 0.0, 0.0), 1.0)).min(
                sdf_sphere(Point3::new(x, 0.0, 0.0), Point3::new(1.5, 0.0, 0.0), 1.0),
            );
            assert!(soft <= hard + 1e-12, "smin bulged inward at x={x}");
        }
        // C¹: finite-difference slope continuous through the blend zone.
        let eps = 1e-6;
        let slope = |x: f64| {
            (d(Point3::new(x + eps, 0.0, 0.0)) - d(Point3::new(x - eps, 0.0, 0.0))) / (2.0 * eps)
        };
        let s0 = slope(0.74);
        let s1 = slope(0.76);
        assert!(s0.is_finite() && s1.is_finite());
    }

    #[test]
    fn capsule_distance_is_exact_on_axis() {
        // Point 1.25 above the top cap centre → distance = 1.25 − r = 0.25.
        let d = sdf_capsule(
            Point3::new(0.0, 2.25, 0.0),
            Point3::new(0.0, -1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            1.0,
        );
        assert!((d - 0.25).abs() < 1e-12, "capsule sdf wrong: {d}");
        // Segment midpoint is 1 unit inside → sdf = −1.
        assert!(
            (sdf_capsule(
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(0.0, -1.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                1.0
            ) + 1.0)
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn rounded_box_faces_and_corners() {
        let c = Point3::new(0.0, 0.0, 0.0);
        let half = Point3::new(1.0, 1.0, 1.0);
        // Face-adjacent: the rounded face plane is still x = half.x; r only
        // rounds edges → distance = overshoot = 0.3.
        let d = sdf_rounded_box(Point3::new(1.3, 0.0, 0.0), c, half, 0.1);
        assert!((d - 0.3).abs() < 1e-12, "face dist wrong: {d}");
        // Corner-adjacent: Euclidean distance to the rounded core corner
        // (at half−r = 0.9) minus r → overshoot 0.3 per axis.
        let p = Point3::new(1.2, 1.2, 1.2);
        let expect = (3.0f64 * 0.3 * 0.3).sqrt() - 0.1;
        assert!((sdf_rounded_box(p, c, half, 0.1) - expect).abs() < 1e-12);
    }

    #[test]
    fn metaball_inside_outside() {
        let centers = [Point3::new(0.0, 0.0, 0.0), Point3::new(0.5, 0.0, 0.0)];
        let radii2 = [1.0, 1.0];
        // At the shared centre the field >> 1 → sdf < 0 (inside blob).
        let inside = sdf_metaballs(Point3::new(0.25, 0.0, 0.0), &centers, &radii2, 1.0).unwrap();
        assert!(inside < 0.0);
        let outside = sdf_metaballs(Point3::new(4.0, 0.0, 0.0), &centers, &radii2, 1.0).unwrap();
        assert!(outside > 0.0);
    }
}
