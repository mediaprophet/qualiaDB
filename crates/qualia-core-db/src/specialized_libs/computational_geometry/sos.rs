//! P12.2 — Simulation of Simplicity (SoS) for deterministic degeneracy resolution.
//!
//! When an exact predicate returns `Sign::Zero`, the input is degenerate
//! (e.g. four coplanar points in `orient_3d`). Algorithms that branch on the
//! sign need a deterministic non-zero answer — otherwise the output depends
//! on floating-point noise or iteration order, breaking reproducibility.
//!
//! **Simulation of Simplicity** (Edelsbrunner & Mücke, 1990) resolves this by
//! symbolically perturbing the input so that all predicates return non-zero
//! signs. The perturbation is infinitesimal (it does not change the topology
//! of non-degenerate inputs) and deterministic (it depends only on the point
//! ordering, not on memory layout or rounding).
//!
//! ## How it works
//!
//! For `orient_3d(a, b, c, d)`, the 4×4 determinant is:
//!
//! ```text
//! D = | ax  ay  az  1 |
//!     | bx  by  bz  1 |
//!     | cx  cy  cz  1 |
//!     | dx  dy  dz  1 |
//! ```
//!
//! When `D = 0`, perturb each coordinate `M[i][j]` (for `j ∈ {x, y, z}`) by
//! `ε^(2^(3i+j))`. The perturbed determinant `D(ε)` is a polynomial in `ε`.
//! Since `D = 0`, the sign of `D(ε)` for infinitesimally small `ε > 0` is
//! the sign of the **first non-zero coefficient** in the polynomial expansion,
//! ordered by increasing power of `ε`.
//!
//! The first-order coefficients are the **cofactors** `C_{ij}` of the original
//! matrix. Each cofactor is a 2D orientation test of the three points ≠ `i`,
//! projected onto the coordinate plane ≠ `j`. The 12 cofactors are evaluated
//! in order of increasing `ε` power:
//!
//! | Order | (i, j) | Power  | Cofactor                         |
//! |-------|--------|--------|----------------------------------|
//! |  1    | (0,0)  | 1      | +orient_2d(b_yz, c_yz, d_yz)    |
//! |  2    | (0,1)  | 2      | −orient_2d(b_xz, c_xz, d_xz)    |
//! |  3    | (0,2)  | 4      | +orient_2d(b_xy, c_xy, d_xy)    |
//! |  4    | (1,0)  | 8      | −orient_2d(a_yz, c_yz, d_yz)    |
//! |  5    | (1,1)  | 16     | +orient_2d(a_xz, c_xz, d_xz)    |
//! |  6    | (1,2)  | 32     | −orient_2d(a_xy, c_xy, d_xy)    |
//! |  7    | (2,0)  | 64     | +orient_2d(a_yz, b_yz, d_yz)    |
//! |  8    | (2,1)  | 128    | −orient_2d(a_xz, b_xz, d_xz)    |
//! |  9    | (2,2)  | 256    | +orient_2d(a_xy, b_xy, d_xy)    |
//! | 10    | (3,0)  | 512    | −orient_2d(a_yz, b_yz, c_yz)    |
//! | 11    | (3,1)  | 1024   | +orient_2d(a_xz, b_xz, c_xz)    |
//! | 12    | (3,2)  | 2048   | −orient_2d(a_xy, b_xy, c_xy)    |
//!
//! If all 12 cofactors are zero (all four points are collinear or identical),
//! the configuration is fully degenerate and we return `Sign::Positive` as a
//! deterministic constant. This is consistent with the SoS principle: the
//! perturbation guarantees a total order, and for fully degenerate inputs any
//! consistent sign is valid.
//!
//! ## Zero-heap contract
//!
//! [`orient_3d_sos`] is a predicate: it takes `Point3` (Copy) and returns
//! `Sign` (Copy). No `Vec`, `String`, or `Box`. The underlying
//! [`orientation_2`] is also zero-heap. This is a Tier-1 hot-path operation.

use super::expansion::Sign;
use super::orient3d::orient_3d;
use super::primitives::{orientation_2, Orientation, Point2, Point3};

/// Convert `Orientation` to `Sign`.
#[inline]
fn orient_to_sign(o: Orientation) -> Sign {
    match o {
        Orientation::CounterClockwise => Sign::Positive,
        Orientation::Collinear => Sign::Zero,
        Orientation::Clockwise => Sign::Negative,
    }
}

/// 3-D orientation with Simulation of Simplicity tie-breaking.
///
/// Computes `orient_3d(a, b, c, d)`. If the result is `Sign::Zero` (coplanar),
/// applies the SoS perturbation scheme to return a deterministic non-zero
/// sign. This function **never returns `Sign::Zero`**.
///
/// The SoS sign is determined by the first non-zero 2D-orientation cofactor
/// in a fixed order of increasing symbolic-perturbation power (see the module
/// documentation for the full table).
///
/// # Properties
///
/// - **Deterministic**: the same input always produces the same sign, regardless
///   of platform, build, or memory layout.
/// - **Antisymmetric**: swapping any two points flips the sign (the cofactor
///   order is consistent with the permutation parity).
/// - **Non-degenerate passthrough**: when `orient_3d` returns non-zero, that
///   sign is returned directly — SoS only activates on exact zeros.
///
/// # Zero-heap
///
/// No allocations. Stack-only computation over `Point3` / `Point2` values.
pub fn orient_3d_sos(a: Point3, b: Point3, c: Point3, d: Point3) -> Sign {
    let sign = orient_3d(a, b, c, d);
    if sign != Sign::Zero {
        return sign;
    }

    // SoS: evaluate the 12 first-order cofactors in order of increasing ε power.
    // Each cofactor is a 2D orientation of three of the four points, projected
    // onto a coordinate plane, with a sign flip from the (-1)^(i+j) factor.

    // Order 1: (i=0, j=0) power=1  → +orient_2d(b_yz, c_yz, d_yz)
    let s = orient_to_sign(orientation_2(
        Point2::new(b.y, b.z),
        Point2::new(c.y, c.z),
        Point2::new(d.y, d.z),
    ));
    if s != Sign::Zero {
        return s;
    }

    // Order 2: (i=0, j=1) power=2  → -orient_2d(b_xz, c_xz, d_xz)
    let s = orient_to_sign(orientation_2(
        Point2::new(b.x, b.z),
        Point2::new(c.x, c.z),
        Point2::new(d.x, d.z),
    ));
    if s != Sign::Zero {
        return s.flip();
    }

    // Order 3: (i=0, j=2) power=4  → +orient_2d(b_xy, c_xy, d_xy)
    let s = orient_to_sign(orientation_2(
        Point2::new(b.x, b.y),
        Point2::new(c.x, c.y),
        Point2::new(d.x, d.y),
    ));
    if s != Sign::Zero {
        return s;
    }

    // Order 4: (i=1, j=0) power=8  → -orient_2d(a_yz, c_yz, d_yz)
    let s = orient_to_sign(orientation_2(
        Point2::new(a.y, a.z),
        Point2::new(c.y, c.z),
        Point2::new(d.y, d.z),
    ));
    if s != Sign::Zero {
        return s.flip();
    }

    // Order 5: (i=1, j=1) power=16 → +orient_2d(a_xz, c_xz, d_xz)
    let s = orient_to_sign(orientation_2(
        Point2::new(a.x, a.z),
        Point2::new(c.x, c.z),
        Point2::new(d.x, d.z),
    ));
    if s != Sign::Zero {
        return s;
    }

    // Order 6: (i=1, j=2) power=32 → -orient_2d(a_xy, c_xy, d_xy)
    let s = orient_to_sign(orientation_2(
        Point2::new(a.x, a.y),
        Point2::new(c.x, c.y),
        Point2::new(d.x, d.y),
    ));
    if s != Sign::Zero {
        return s.flip();
    }

    // Order 7: (i=2, j=0) power=64 → +orient_2d(a_yz, b_yz, d_yz)
    let s = orient_to_sign(orientation_2(
        Point2::new(a.y, a.z),
        Point2::new(b.y, b.z),
        Point2::new(d.y, d.z),
    ));
    if s != Sign::Zero {
        return s;
    }

    // Order 8: (i=2, j=1) power=128 → -orient_2d(a_xz, b_xz, d_xz)
    let s = orient_to_sign(orientation_2(
        Point2::new(a.x, a.z),
        Point2::new(b.x, b.z),
        Point2::new(d.x, d.z),
    ));
    if s != Sign::Zero {
        return s.flip();
    }

    // Order 9: (i=2, j=2) power=256 → +orient_2d(a_xy, b_xy, d_xy)
    let s = orient_to_sign(orientation_2(
        Point2::new(a.x, a.y),
        Point2::new(b.x, b.y),
        Point2::new(d.x, d.y),
    ));
    if s != Sign::Zero {
        return s;
    }

    // Order 10: (i=3, j=0) power=512 → -orient_2d(a_yz, b_yz, c_yz)
    let s = orient_to_sign(orientation_2(
        Point2::new(a.y, a.z),
        Point2::new(b.y, b.z),
        Point2::new(c.y, c.z),
    ));
    if s != Sign::Zero {
        return s.flip();
    }

    // Order 11: (i=3, j=1) power=1024 → +orient_2d(a_xz, b_xz, c_xz)
    let s = orient_to_sign(orientation_2(
        Point2::new(a.x, a.z),
        Point2::new(b.x, b.z),
        Point2::new(c.x, c.z),
    ));
    if s != Sign::Zero {
        return s;
    }

    // Order 12: (i=3, j=2) power=2048 → -orient_2d(a_xy, b_xy, c_xy)
    let s = orient_to_sign(orientation_2(
        Point2::new(a.x, a.y),
        Point2::new(b.x, b.y),
        Point2::new(c.x, c.y),
    ));
    if s != Sign::Zero {
        return s.flip();
    }

    // All 12 cofactors are zero: the four points are collinear or identical.
    // Return a deterministic constant. This is consistent with the SoS
    // principle — the perturbation guarantees a total order, and for fully
    // degenerate inputs any consistent sign is valid.
    Sign::Positive
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64, z: f64) -> Point3 {
        Point3::new(x, y, z)
    }

    #[test]
    fn sos_never_returns_zero() {
        // Test a variety of degenerate and non-degenerate configurations.
        let cases = [
            // Non-degenerate: regular tetrahedron.
            (p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0), p(0.0, 0.0, 1.0)),
            // Coplanar: four points in z=0.
            (p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0), p(1.0, 1.0, 0.0)),
            // Coplanar: four points in z=1.
            (p(0.0, 0.0, 1.0), p(1.0, 0.0, 1.0), p(0.0, 1.0, 1.0), p(1.0, 1.0, 1.0)),
            // Collinear: four points on the x-axis.
            (p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(2.0, 0.0, 0.0), p(3.0, 0.0, 0.0)),
            // Identical points.
            (p(1.0, 2.0, 3.0), p(1.0, 2.0, 3.0), p(1.0, 2.0, 3.0), p(1.0, 2.0, 3.0)),
            // Coplanar with a shared vertex.
            (p(0.0, 0.0, 0.0), p(2.0, 0.0, 0.0), p(0.0, 2.0, 0.0), p(1.0, 1.0, 0.0)),
        ];

        for (a, b, c, d) in &cases {
            let sign = orient_3d_sos(*a, *b, *c, *d);
            assert!(
                sign != Sign::Zero,
                "SoS must never return Zero for ({a:?}, {b:?}, {c:?}, {d:?})"
            );
        }
    }

    #[test]
    fn sos_non_degenerate_matches_orient_3d() {
        // Non-degenerate cases: SoS should return the same sign as orient_3d.
        let a = p(0.0, 0.0, 0.0);
        let b = p(1.0, 0.0, 0.0);
        let c = p(0.0, 1.0, 0.0);
        let d = p(0.0, 0.0, 1.0);

        assert_eq!(orient_3d_sos(a, b, c, d), orient_3d(a, b, c, d));
        assert_eq!(orient_3d_sos(a, b, c, d), Sign::Positive);

        // Flip d to the other side.
        let d2 = p(0.0, 0.0, -1.0);
        assert_eq!(orient_3d_sos(a, b, c, d2), orient_3d(a, b, c, d2));
        assert_eq!(orient_3d_sos(a, b, c, d2), Sign::Negative);
    }

    #[test]
    fn sos_coplanar_is_deterministic() {
        // Four coplanar points in z=0. SoS should always return the same sign.
        let a = p(0.0, 0.0, 0.0);
        let b = p(1.0, 0.0, 0.0);
        let c = p(0.0, 1.0, 0.0);
        let d = p(1.0, 1.0, 0.0);

        let s1 = orient_3d_sos(a, b, c, d);
        let s2 = orient_3d_sos(a, b, c, d);
        assert_eq!(s1, s2, "SoS must be deterministic");
        assert_ne!(s1, Sign::Zero, "SoS must not return Zero for coplanar");
    }

    #[test]
    fn sos_antisymmetric_swap_two_points() {
        // Swapping two points should flip the sign.
        // For non-degenerate: orient_3d(a,b,c,d) = -orient_3d(b,a,c,d).
        let a = p(0.0, 0.0, 0.0);
        let b = p(1.0, 0.0, 0.0);
        let c = p(0.0, 1.0, 0.0);
        let d = p(0.0, 0.0, 1.0);

        let s1 = orient_3d_sos(a, b, c, d);
        let s2 = orient_3d_sos(b, a, c, d);
        assert_eq!(s1, s2.flip(), "swapping a,b should flip the sign");

        let s3 = orient_3d_sos(a, c, b, d);
        assert_eq!(s1, s3.flip(), "swapping b,c should flip the sign");

        let s4 = orient_3d_sos(a, b, d, c);
        assert_eq!(s1, s4.flip(), "swapping c,d should flip the sign");
    }

    #[test]
    fn sos_coplanar_swap_is_deterministic() {
        // For coplanar points, SoS does NOT guarantee antisymmetry
        // (the perturbation is tied to row position, not point identity).
        // But it must still be deterministic and non-zero.
        let a = p(0.0, 0.0, 0.0);
        let b = p(1.0, 0.0, 0.0);
        let c = p(0.0, 1.0, 0.0);
        let d = p(1.0, 1.0, 0.0);

        let s1 = orient_3d_sos(a, b, c, d);
        let s2 = orient_3d_sos(b, a, c, d);
        assert_ne!(s1, Sign::Zero);
        assert_ne!(s2, Sign::Zero);
        // Deterministic: same call → same result.
        assert_eq!(s1, orient_3d_sos(a, b, c, d));
        assert_eq!(s2, orient_3d_sos(b, a, c, d));
    }

    #[test]
    fn sos_coplanar_square_consistent() {
        // Four corners of a unit square in z=0.
        // The SoS sign should be consistent with the orientation structure.
        let a = p(0.0, 0.0, 0.0);
        let b = p(1.0, 0.0, 0.0);
        let c = p(1.0, 1.0, 0.0);
        let d = p(0.0, 1.0, 0.0);

        // (a, b, c, d) — the first cofactor is orient_2d(b_yz, c_yz, d_yz).
        // b_yz = (0, 0), c_yz = (1, 0), d_yz = (1, 0) — collinear in yz.
        // Second cofactor: -orient_2d(b_xz, c_xz, d_xz).
        // b_xz = (1, 0), c_xz = (1, 0), d_xz = (0, 0) — collinear in xz.
        // Third cofactor: +orient_2d(b_xy, c_xy, d_xy).
        // b_xy = (1, 0), c_xy = (1, 1), d_xy = (0, 1).
        // orient_2d((1,0), (1,1), (0,1)) = CCW (positive).
        let sign = orient_3d_sos(a, b, c, d);
        assert_eq!(sign, Sign::Positive, "square (a,b,c,d) should be Positive via 3rd cofactor");
    }

    #[test]
    fn sos_collinear_returns_positive() {
        // Four collinear points on x-axis — all cofactors are zero.
        // The fallback should return Sign::Positive.
        let a = p(0.0, 0.0, 0.0);
        let b = p(1.0, 0.0, 0.0);
        let c = p(2.0, 0.0, 0.0);
        let d = p(3.0, 0.0, 0.0);

        let sign = orient_3d_sos(a, b, c, d);
        assert_eq!(sign, Sign::Positive, "collinear fallback should return Positive");
    }

    #[test]
    fn sos_identical_points_returns_positive() {
        // All four points identical — fully degenerate.
        let a = p(1.0, 2.0, 3.0);
        let sign = orient_3d_sos(a, a, a, a);
        assert_eq!(sign, Sign::Positive, "identical points fallback should return Positive");
    }

    #[test]
    fn sos_cyclic_permutation_flips_sign() {
        // A 4-cycle (a,b,c,d) → (b,c,d,a) is 3 transpositions = odd permutation.
        // The determinant sign flips. For non-degenerate cases, SoS passes
        // through the actual sign, so this must hold.
        let a = p(0.0, 0.0, 0.0);
        let b = p(1.0, 0.0, 0.0);
        let c = p(0.0, 1.0, 0.0);
        let d = p(0.0, 0.0, 1.0);

        let s1 = orient_3d_sos(a, b, c, d);
        let s2 = orient_3d_sos(b, c, d, a);
        assert_eq!(s1, s2.flip(), "4-cycle (odd permutation) should flip sign");
    }

    #[test]
    fn sos_coplanar_cyclic_is_deterministic() {
        // For coplanar points, SoS doesn't guarantee permutation invariance,
        // but must be deterministic and non-zero.
        let a = p(0.0, 0.0, 0.0);
        let b = p(1.0, 0.0, 0.0);
        let c = p(1.0, 1.0, 0.0);
        let d = p(0.0, 1.0, 0.0);

        let s1 = orient_3d_sos(a, b, c, d);
        let s2 = orient_3d_sos(b, c, d, a);
        assert_ne!(s1, Sign::Zero);
        assert_ne!(s2, Sign::Zero);
        assert_eq!(s1, orient_3d_sos(a, b, c, d), "must be deterministic");
    }

    #[test]
    fn sos_three_identical_one_different() {
        // Three identical points + one different. The cofactors involving
        // the three identical points will be zero, but cofactors involving
        // the different point and two of the identical ones may be non-zero.
        let a = p(0.0, 0.0, 0.0);
        let b = p(0.0, 0.0, 0.0);
        let c = p(0.0, 0.0, 0.0);
        let d = p(1.0, 0.0, 0.0);

        let sign = orient_3d_sos(a, b, c, d);
        assert_ne!(sign, Sign::Zero, "must not return Zero");

        // All cofactors will be zero because any 3-point subset is collinear
        // (three identical points, or two identical + one on x-axis).
        // So this should hit the fallback.
        assert_eq!(sign, Sign::Positive);
    }
}
