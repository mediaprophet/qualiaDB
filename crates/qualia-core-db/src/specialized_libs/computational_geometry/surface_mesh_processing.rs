//! P5.3 — Surface-mesh processing core: **measures** on a triangle mesh.
//!
//! Area and signed volume over caller-owned slices, zero-heap (no `Vec`/`String`/`Box`,
//! no scratch — a single streaming pass over the triangle list). These are *metric*
//! quantities (magnitudes), not sign predicates: exact geometric predicates
//! ([`GeometryKernel::orient_3d`](super::kernel::GeometryKernel) etc.) exist to make
//! combinatorial *decisions* robust; an area or a volume is an approximate real number,
//! so `f64` accumulation is the right and honest model here.
//!
//! ## Scope (honest)
//!
//! - **Implemented + verified here:** [`surface_area`], [`signed_volume`].
//! - **Already provided elsewhere (reused, not re-implemented):** connected-component
//!   count, boundary-loop count, Euler characteristic and genus come from
//!   [`super::connectivity`] (P2.5); manifold / watertight detection and orientation
//!   come from [`super::topology::build_triangle_half_edges`] (P2.1). This module does
//!   **not** duplicate them.
//! - **Deferred (a real follow-up, NOT stubbed):** mesh **self-intersection** — it needs
//!   triangle–triangle intersection over the P3 BVH broad phase (`super::bvh`) and is a
//!   separate unit. It is deliberately absent rather than faked with a placeholder that
//!   returns a plausible-but-wrong answer.

use super::primitives::Point3;

/// Failure modes for the surface-mesh measures. Both are input-integrity faults, not
/// numeric ones — a finite mesh with in-bounds indices always yields a finite measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshMeasureError {
    /// A triangle referenced a vertex index outside `vertices`.
    IndexOutOfBounds { triangle: usize, vertex: u32 },
    /// A referenced vertex had a non-finite coordinate (NaN / ±∞).
    NonFiniteCoordinate { index: usize },
}

/// Fetch and validate the three corner points of triangle `t`.
#[inline]
fn fetch(vertices: &[Point3], tri: &[u32; 3], t: usize) -> Result<[Point3; 3], MeshMeasureError> {
    let mut out = [Point3::new(0.0, 0.0, 0.0); 3];
    for (i, &vi) in tri.iter().enumerate() {
        let v = *vertices
            .get(vi as usize)
            .ok_or(MeshMeasureError::IndexOutOfBounds {
                triangle: t,
                vertex: vi,
            })?;
        if !v.x.is_finite() || !v.y.is_finite() || !v.z.is_finite() {
            return Err(MeshMeasureError::NonFiniteCoordinate { index: vi as usize });
        }
        out[i] = v;
    }
    Ok(out)
}

/// Total surface area `= Σ ½‖(b − a) × (c − a)‖` over every triangle.
///
/// Independent of winding (uses the cross-product *magnitude*). Degenerate
/// (zero-area / collinear) triangles contribute exactly `0`. Deterministic: identical
/// input → bit-identical result (fixed summation order = the triangle order).
pub fn surface_area(vertices: &[Point3], triangles: &[[u32; 3]]) -> Result<f64, MeshMeasureError> {
    let mut area = 0.0f64;
    for (t, tri) in triangles.iter().enumerate() {
        let [a, b, c] = fetch(vertices, tri, t)?;
        let (ux, uy, uz) = (b.x - a.x, b.y - a.y, b.z - a.z);
        let (vx, vy, vz) = (c.x - a.x, c.y - a.y, c.z - a.z);
        // cross(u, v)
        let cx = uy * vz - uz * vy;
        let cy = uz * vx - ux * vz;
        let cz = ux * vy - uy * vx;
        area += 0.5 * (cx * cx + cy * cy + cz * cz).sqrt();
    }
    Ok(area)
}

/// Signed volume `= Σ (1/6) · a · (b × c)` (divergence theorem, tetrahedra to the origin).
///
/// For a **closed, consistently-oriented** mesh this is the enclosed volume, and its
/// **sign encodes global orientation** (positive when triangles wind outward / CCW seen
/// from outside). For a closed mesh the result is origin-independent; for an *open* mesh
/// the value is origin-dependent and not a meaningful volume — the caller must confirm
/// closure (via [`super::topology::build_triangle_half_edges`], boundary-edge count `= 0`)
/// before interpreting it as such. Deterministic (fixed summation order).
pub fn signed_volume(vertices: &[Point3], triangles: &[[u32; 3]]) -> Result<f64, MeshMeasureError> {
    let mut vol6 = 0.0f64;
    for (t, tri) in triangles.iter().enumerate() {
        let [a, b, c] = fetch(vertices, tri, t)?;
        // scalar triple product a · (b × c)
        let bxcx = b.y * c.z - b.z * c.y;
        let bxcy = b.z * c.x - b.x * c.z;
        let bxcz = b.x * c.y - b.y * c.x;
        vol6 += a.x * bxcx + a.y * bxcy + a.z * bxcz;
    }
    Ok(vol6 / 6.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unit cube [0,1]³, 12 triangles, all wound outward (verified per-face by hand).
    fn unit_cube() -> (Vec<Point3>, Vec<[u32; 3]>) {
        let v = vec![
            Point3::new(0.0, 0.0, 0.0), // 0
            Point3::new(1.0, 0.0, 0.0), // 1
            Point3::new(1.0, 1.0, 0.0), // 2
            Point3::new(0.0, 1.0, 0.0), // 3
            Point3::new(0.0, 0.0, 1.0), // 4
            Point3::new(1.0, 0.0, 1.0), // 5
            Point3::new(1.0, 1.0, 1.0), // 6
            Point3::new(0.0, 1.0, 1.0), // 7
        ];
        let t = vec![
            [0, 3, 2],
            [0, 2, 1], // -Z
            [4, 5, 6],
            [4, 6, 7], // +Z
            [0, 1, 5],
            [0, 5, 4], // -Y
            [3, 7, 6],
            [3, 6, 2], // +Y
            [0, 4, 7],
            [0, 7, 3], // -X
            [1, 2, 6],
            [1, 6, 5], // +X
        ];
        (v, t)
    }

    /// Tetrahedron on the origin + unit axes, wound outward. Volume = 1/6.
    fn unit_tetra() -> (Vec<Point3>, Vec<[u32; 3]>) {
        let v = vec![
            Point3::new(0.0, 0.0, 0.0), // 0
            Point3::new(1.0, 0.0, 0.0), // 1
            Point3::new(0.0, 1.0, 0.0), // 2
            Point3::new(0.0, 0.0, 1.0), // 3
        ];
        let t = vec![[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]];
        (v, t)
    }

    #[test]
    fn cube_area_is_six() {
        let (v, t) = unit_cube();
        assert!((surface_area(&v, &t).unwrap() - 6.0).abs() < 1e-12);
    }

    #[test]
    fn cube_volume_is_one_and_outward() {
        let (v, t) = unit_cube();
        let vol = signed_volume(&v, &t).unwrap();
        assert!(
            (vol - 1.0).abs() < 1e-12,
            "outward cube volume should be +1, got {vol}"
        );
    }

    #[test]
    fn reversed_winding_flips_volume_sign() {
        let (v, t) = unit_cube();
        let reversed: Vec<[u32; 3]> = t.iter().map(|tr| [tr[0], tr[2], tr[1]]).collect();
        let vol = signed_volume(&v, &reversed).unwrap();
        assert!(
            (vol + 1.0).abs() < 1e-12,
            "reversed cube volume should be -1, got {vol}"
        );
    }

    #[test]
    fn closed_mesh_volume_is_origin_independent() {
        // Translate the cube far from the origin; enclosed volume must not change.
        let (mut v, t) = unit_cube();
        for p in &mut v {
            p.x += 1000.0;
            p.y -= 500.0;
            p.z += 7.5;
        }
        assert!((signed_volume(&v, &t).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn tetra_area_and_volume() {
        let (v, t) = unit_tetra();
        let expected_area = 1.5 + 3.0f64.sqrt() / 2.0; // three ½-area faces + slanted face
        assert!((surface_area(&v, &t).unwrap() - expected_area).abs() < 1e-12);
        assert!((signed_volume(&v, &t).unwrap() - 1.0 / 6.0).abs() < 1e-12);
    }

    #[test]
    fn degenerate_triangle_contributes_zero_area() {
        // Collinear triple → zero area, no NaN.
        let v = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ];
        assert_eq!(surface_area(&v, &[[0, 1, 2]]).unwrap(), 0.0);
    }

    #[test]
    fn empty_mesh_is_zero() {
        assert_eq!(surface_area(&[], &[]).unwrap(), 0.0);
        assert_eq!(signed_volume(&[], &[]).unwrap(), 0.0);
    }

    #[test]
    fn out_of_bounds_index_errors() {
        let v = vec![Point3::new(0.0, 0.0, 0.0)];
        assert_eq!(
            surface_area(&v, &[[0, 1, 2]]),
            Err(MeshMeasureError::IndexOutOfBounds {
                triangle: 0,
                vertex: 1
            })
        );
    }

    #[test]
    fn non_finite_coordinate_errors() {
        let v = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(f64::NAN, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        assert_eq!(
            signed_volume(&v, &[[0, 1, 2]]),
            Err(MeshMeasureError::NonFiniteCoordinate { index: 1 })
        );
    }

    #[test]
    fn deterministic_bit_identical() {
        let (v, t) = unit_cube();
        let a = surface_area(&v, &t).unwrap();
        let b = surface_area(&v, &t).unwrap();
        assert_eq!(a.to_bits(), b.to_bits());
        let va = signed_volume(&v, &t).unwrap();
        let vb = signed_volume(&v, &t).unwrap();
        assert_eq!(va.to_bits(), vb.to_bits());
    }
}
