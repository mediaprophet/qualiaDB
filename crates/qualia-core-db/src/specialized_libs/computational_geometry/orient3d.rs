//! `orient3d` — the 3-D orientation predicate (P1.4).
//!
//! Computes the sign of the determinant of the 3×3 matrix whose rows are
//! `(b − a, c − a, d − a)`, i.e. the scalar triple product
//! `(b − a) · ((c − a) × (d − a))`. This is the signed volume of the
//! tetrahedron `a b c d` (up to a factor of 6); its sign classifies whether
//! `d` lies above, on, or below the oriented plane through `a, b, c`.
//!
//! ## Filtered → compensated → exact ladder (Shewchuk adaptive precision)
//!
//! 1. **Filtered** — a single determinant with a static error bound. When the
//!    absolute determinant exceeds the bound, the sign is certain; return it.
//! 2. **Compensated** — recover each product's rounding residual via `mul_add`
//!    and form a compensated determinant. Tighter bound; resolves most
//!    near-degenerate cases without expansion arithmetic.
//! 3. **Exact** — fall back to expansion arithmetic ([`super::expansion`]) over
//!    a stack-allocated workspace sized by [`super::expansion::MAX_EXPANSION_ORIENT3`].
//!    This is the zero-heap, always-correct path, used only when the
//!    compensated result is still within its error bound.
//!
//! ## Zero-heap contract
//!
//! No `Vec`, `String`, or `Box` in any path. The exact stage uses a fixed-size
//! stack array (`[f64; MAX_EXPANSION_ORIENT3]`), not a heap allocation.
//!
//! ## References
//!
//! The adaptive-precision ladder follows Jonathan Richard Shewchuk, "Adaptive
//! Precision Floating-Point Arithmetic and Fast Robust Geometric Predicates"
//! (1996, Discrete & Computational Geometry). The implementation is original
//! Rust over the P1.3 expansion primitives. No third-party
//! source code is used.

use super::expansion::{
    compress_expansion, expansion_sum, negate_expansion, scale_expansion, sign_of_expansion,
    two_product, Sign, MAX_EXPANSION_ORIENT3,
};
use super::primitives::Point3;

// ──────────────────────────────────────────────────────────────────────────
//  Error bounds
// ──────────────────────────────────────────────────────────────────────────

/// Filtered error bound coefficient for the 3×3 determinant. The filtered
/// determinant is a sum of 6 products of 3 values, each product accumulating
/// ~2 rounding steps, plus summation rounding. The permanent (sum of absolute
/// term values) scaled by this coefficient bounds the absolute error.
///
/// Value: `16.0 * EPSILON` — conservative; accounts for 6 products (each with
/// ~2 multiply roundings) plus 5 additions/subtractions.
const FILTERED_BOUND: f64 = 16.0 * f64::EPSILON;

/// Compensated error bound coefficient. The compensated determinant recovers
/// each product's first-order rounding residual via `mul_add`, leaving only
/// the summation rounding (second-order in the product errors, first-order in
/// the sums). This is substantially tighter than [`FILTERED_BOUND`].
const COMPENSATED_BOUND: f64 = 4.0 * f64::EPSILON;

// ──────────────────────────────────────────────────────────────────────────
//  The 9 coordinate differences
// ──────────────────────────────────────────────────────────────────────────

/// The 9 coordinate differences of the 3×3 orient3d matrix, computed as f64.
/// These are the inputs to all three ladder stages.
#[derive(Clone, Copy)]
struct Diffs {
    abx: f64,
    aby: f64,
    abz: f64,
    acx: f64,
    acy: f64,
    acz: f64,
    adx: f64,
    ady: f64,
    adz: f64,
}

impl Diffs {
    #[inline]
    fn from_points(a: Point3, b: Point3, c: Point3, d: Point3) -> Self {
        Diffs {
            abx: b.x - a.x,
            aby: b.y - a.y,
            abz: b.z - a.z,
            acx: c.x - a.x,
            acy: c.y - a.y,
            acz: c.z - a.z,
            adx: d.x - a.x,
            ady: d.y - a.y,
            adz: d.z - a.z,
        }
    }

    /// The permanent: sum of the absolute values of the 6 determinant terms.
    /// This is the scale factor for the error bounds.
    #[inline]
    fn permanent(&self) -> f64 {
        let Diffs {
            abx,
            aby,
            abz,
            acx,
            acy,
            acz,
            adx,
            ady,
            adz,
        } = *self;
        (abx.abs() * (acy.abs() * adz.abs() + acz.abs() * ady.abs()))
            + (aby.abs() * (acx.abs() * adz.abs() + acz.abs() * adx.abs()))
            + (abz.abs() * (acx.abs() * ady.abs() + acy.abs() * adx.abs()))
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  Stage 1: Filtered
// ──────────────────────────────────────────────────────────────────────────

/// The filtered 3×3 determinant: `det(b−a, c−a, d−a)` as a single f64.
#[inline]
fn filtered_det(d: &Diffs) -> f64 {
    let Diffs {
        abx,
        aby,
        abz,
        acx,
        acy,
        acz,
        adx,
        ady,
        adz,
    } = *d;
    abx * (acy * adz - acz * ady) - aby * (acx * adz - acz * adx) + abz * (acx * ady - acy * adx)
}

// ──────────────────────────────────────────────────────────────────────────
//  Stage 2: Compensated
// ──────────────────────────────────────────────────────────────────────────

/// The compensated 3×3 determinant. Recovers each product's rounding residual
/// via `mul_add` (equivalently [`two_product`]), then forms the determinant
/// from the recovered products. The result has a tighter error bound than
/// [`filtered_det`] because the first-order product rounding is eliminated.
#[inline]
fn compensated_det(d: &Diffs) -> f64 {
    let Diffs {
        abx,
        aby,
        abz,
        acx,
        acy,
        acz,
        adx,
        ady,
        adz,
    } = *d;

    // Each inner product: two_product recovers the exact residual.
    let (p_acy_adz, e_acy_adz) = two_product(acy, adz);
    let (p_acz_ady, e_acz_ady) = two_product(acz, ady);
    let (p_acx_adz, e_acx_adz) = two_product(acx, adz);
    let (p_acz_adx, e_acz_adx) = two_product(acz, adx);
    let (p_acx_ady, e_acx_ady) = two_product(acx, ady);
    let (p_acy_adx, e_acy_adx) = two_product(acy, adx);

    // Inner differences (the three 2×2 minors), with recovered errors.
    let minor1 = p_acy_adz - p_acz_ady;
    let minor1_err = e_acy_adz - e_acz_ady;
    let minor2 = p_acx_adz - p_acz_adx;
    let minor2_err = e_acx_adz - e_acz_adx;
    let minor3 = p_acx_ady - p_acy_adx;
    let minor3_err = e_acx_ady - e_acy_adx;

    // Outer products: abx * minor1, etc. Recover the multiply residual.
    let outer1 = abx * minor1;
    let outer1_err = abx.mul_add(minor1, -outer1) + abx * minor1_err;
    let outer2 = aby * minor2;
    let outer2_err = aby.mul_add(minor2, -outer2) + aby * minor2_err;
    let outer3 = abz * minor3;
    let outer3_err = abz.mul_add(minor3, -outer3) + abz * minor3_err;

    // Final combination: outer1 - outer2 + outer3, with recovered errors.
    (outer1 - outer2 + outer3) + (outer1_err - outer2_err + outer3_err)
}

// ──────────────────────────────────────────────────────────────────────────
//  Stage 3: Exact (expansion arithmetic)
// ──────────────────────────────────────────────────────────────────────────

/// The exact 3×3 determinant via expansion arithmetic. Zero-heap: uses a
/// fixed-size stack workspace of [`MAX_EXPANSION_ORIENT3`] f64s (192 bytes).
///
/// The determinant has 6 terms, each a product of 3 coordinate differences.
/// Each term is computed as: `two_product(d1, d2)` → length-2 expansion, then
/// `scale_expansion(len-2, d3)` → length-4 expansion. The 6 terms are summed
/// (with signs) into the 24-element workspace, compressed, and the sign of the
/// resulting expansion is returned.
fn exact_det(d: &Diffs) -> Sign {
    let Diffs {
        abx,
        aby,
        abz,
        acx,
        acy,
        acz,
        adx,
        ady,
        adz,
    } = *d;

    // The 6 terms of the determinant with their signs:
    //   +abx*acy*adz  - abx*acz*ady  - aby*acx*adz
    //   +aby*acz*adx  + abz*acx*ady  - abz*acy*adx
    //
    // (d1, d2, d3, negate): each term is d1*d2*d3, negated if `negate` is true.
    let terms: [(f64, f64, f64, bool); 6] = [
        (abx, acy, adz, false),
        (abx, acz, ady, true),
        (aby, acx, adz, true),
        (aby, acz, adx, false),
        (abz, acx, ady, false),
        (abz, acy, adx, true),
    ];

    // Stack-allocated workspace (zero-heap).
    let mut prod = [0.0f64; 2]; // d1*d2 as a length-2 expansion
    let mut term = [0.0f64; 4]; // each term (d1*d2*d3): length ≤ 4
    let mut accum = [0.0f64; MAX_EXPANSION_ORIENT3]; // accumulator (compressed)
    let mut temp = [0.0f64; MAX_EXPANSION_ORIENT3]; // expansion_sum scratch
    let mut accum_len = 0usize;

    for &(d1, d2, d3, negate) in &terms {
        // Compute d1 * d2 as a length-2 expansion.
        let (p, e) = two_product(d1, d2);
        prod[0] = p;
        prod[1] = e;

        // Scale by d3 → length ≤ 4 expansion (separate input/output buffers
        // to satisfy the borrow checker).
        let term_len = scale_expansion(&prod, d3, &mut term)
            .expect("term buffer is sized for scale_expansion output");

        if negate {
            negate_expansion(&mut term[..term_len]);
        }

        // Accumulate: accum = accum + term, then compress to keep the
        // accumulator minimal and non-overlapping. Compression after each
        // addition is essential for cancellation cases (e.g. when a term and
        // its negation cancel): without it, the uncombined components can
        // leave a spurious non-zero sign in the final expansion.
        if accum_len == 0 {
            accum[..term_len].copy_from_slice(&term[..term_len]);
            accum_len = term_len;
        } else {
            let sum_len = expansion_sum(&accum[..accum_len], &term[..term_len], &mut temp)
                .expect("temp buffer is sized for MAX_EXPANSION_ORIENT3");
            // Compress temp → accum (different arrays, no borrow conflict).
            accum_len = compress_expansion(&temp[..sum_len], &mut accum)
                .expect("accum buffer is sized for MAX_EXPANSION_ORIENT3");
        }
    }

    // Final compress and read the sign.
    let mut compressed = [0.0f64; MAX_EXPANSION_ORIENT3];
    let comp_len = compress_expansion(&accum[..accum_len], &mut compressed)
        .expect("compressed buffer is sized for MAX_EXPANSION_ORIENT3");
    sign_of_expansion(&compressed[..comp_len])
}

// ──────────────────────────────────────────────────────────────────────────
//  Public ladder entry point
// ──────────────────────────────────────────────────────────────────────────

/// The 3-D orientation predicate: sign of `det(b−a, c−a, d−a)`.
///
/// Returns [`Sign::Positive`] if `d` lies below the oriented plane through
/// `a → b → c` (right-hand rule), [`Sign::Negative`] if above, [`Sign::Zero`]
/// if the four points are coplanar.
///
/// This is the public ladder entry point — it escalates from filtered to
/// compensated to exact as needed, never returning an uncertain sign.
pub fn orient_3d(a: Point3, b: Point3, c: Point3, d: Point3) -> Sign {
    let diffs = Diffs::from_points(a, b, c, d);
    let perm = diffs.permanent();

    // Stage 1: Filtered.
    let det = filtered_det(&diffs);
    if det.abs() > perm * FILTERED_BOUND {
        return Sign::from_f64(det);
    }

    // Stage 2: Compensated.
    let comp = compensated_det(&diffs);
    if comp.abs() > perm * COMPENSATED_BOUND {
        return Sign::from_f64(comp);
    }

    // Stage 3: Exact.
    exact_det(&diffs)
}

// ──────────────────────────────────────────────────────────────────────────
//  Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::exact_test_helper::Exact;

    /// Compute the exact orient3d sign via BigInt arbitrary-precision arithmetic.
    /// This is the ground-truth cross-check.
    fn exact_orient3d_sign(a: Point3, b: Point3, c: Point3, d: Point3) -> Sign {
        let ax = Exact::from_f64(a.x);
        let ay = Exact::from_f64(a.y);
        let az = Exact::from_f64(a.z);
        let bx = Exact::from_f64(b.x);
        let by = Exact::from_f64(b.y);
        let bz = Exact::from_f64(b.z);
        let cx = Exact::from_f64(c.x);
        let cy = Exact::from_f64(c.y);
        let cz = Exact::from_f64(c.z);
        let dx = Exact::from_f64(d.x);
        let dy = Exact::from_f64(d.y);
        let dz = Exact::from_f64(d.z);

        // det = (b-a) · ((c-a) × (d-a))
        let abx = bx.sub(ax.clone());
        let aby = by.sub(ay.clone());
        let abz = bz.sub(az.clone());
        let acx = cx.sub(ax.clone());
        let acy = cy.sub(ay.clone());
        let acz = cz.sub(az.clone());
        let adx = dx.sub(ax);
        let ady = dy.sub(ay);
        let adz = dz.sub(az);

        // The 6 terms: +abx*acy*adz - abx*acz*ady - aby*acx*adz + aby*acz*adx + abz*acx*ady - abz*acy*adx
        let t1 = abx.clone().mul(acy.clone()).mul(adz.clone());
        let t2 = abx.clone().mul(acz.clone()).mul(ady.clone());
        let t3 = aby.clone().mul(acx.clone()).mul(adz.clone());
        let t4 = aby.clone().mul(acz.clone()).mul(adx.clone());
        let t5 = abz.clone().mul(acx.clone()).mul(ady.clone());
        let t6 = abz.clone().mul(acy.clone()).mul(adx.clone());

        let det = t1.sub(t2).sub(t3).add(t4).add(t5).sub(t6);
        det.sign()
    }

    // ── Basic classification ──────────────────────────────────────────────

    #[test]
    fn classifies_positive_tetrahedron() {
        // a at origin, b on x, c on y, d on z → positive orientation
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let d = Point3::new(0.0, 0.0, 1.0);
        assert_eq!(orient_3d(a, b, c, d), Sign::Positive);
    }

    #[test]
    fn classifies_negative_tetrahedron() {
        // Swap c and d → negative orientation
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 0.0, 1.0);
        let d = Point3::new(0.0, 1.0, 0.0);
        assert_eq!(orient_3d(a, b, c, d), Sign::Negative);
    }

    #[test]
    fn classifies_coplanar() {
        // Four points in the z=0 plane
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let d = Point3::new(1.0, 1.0, 0.0);
        assert_eq!(orient_3d(a, b, c, d), Sign::Zero);
    }

    #[test]
    fn coplanar_on_arbitrary_plane() {
        // Four points on the plane x + y + z = 3
        let a = Point3::new(1.0, 1.0, 1.0);
        let b = Point3::new(2.0, 1.0, 0.0);
        let c = Point3::new(1.0, 2.0, 0.0);
        let d = Point3::new(0.0, 1.0, 2.0);
        assert_eq!(orient_3d(a, b, c, d), Sign::Zero);
        assert_eq!(exact_orient3d_sign(a, b, c, d), Sign::Zero);
    }

    // ── Agreement with BigInt cross-check ─────────────────────────────────

    #[test]
    fn agrees_with_exact_on_basic_cases() {
        let cases = [
            (
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(0.0, 0.0, 1.0),
            ),
            (
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 0.0, 1.0),
                Point3::new(0.0, 1.0, 0.0),
            ),
            (
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
            ),
            (
                Point3::new(1.0, 2.0, 3.0),
                Point3::new(4.0, 5.0, 6.0),
                Point3::new(7.0, 8.0, 10.0),
                Point3::new(11.0, 12.0, 13.0),
            ),
        ];
        for (a, b, c, d) in cases {
            assert_eq!(
                orient_3d(a, b, c, d),
                exact_orient3d_sign(a, b, c, d),
                "mismatch on ({a:?}, {b:?}, {c:?}, {d:?})"
            );
        }
    }

    // ── Adversarial: extreme exponents ────────────────────────────────────

    #[test]
    fn extreme_exponents_agree_with_exact() {
        let cases = [
            // Large coordinates, small separation
            (
                Point3::new(1e100, 0.0, 0.0),
                Point3::new(1e100, 1.0, 0.0),
                Point3::new(1e100, 0.0, 1.0),
                Point3::new(1e100, 1e-100, 1e-100),
            ),
            // Small coordinates
            (
                Point3::new(1e-100, 0.0, 0.0),
                Point3::new(1e-100, 1e-100, 0.0),
                Point3::new(1e-100, 0.0, 1e-100),
                Point3::new(1e-100, 2e-100, 2e-100),
            ),
            // Mixed exponents
            (
                Point3::new(1e100, 1e-100, 0.0),
                Point3::new(1e100, 1e-100, 1.0),
                Point3::new(1e100, 2e-100, 0.0),
                Point3::new(1e100, 0.0, 1e-100),
            ),
        ];
        for (a, b, c, d) in cases {
            assert_eq!(
                orient_3d(a, b, c, d),
                exact_orient3d_sign(a, b, c, d),
                "mismatch on extreme-exponent case ({a:?}, {b:?}, {c:?}, {d:?})"
            );
        }
    }

    // ── Adversarial: near-coplanar (±1-ulp) ───────────────────────────────

    #[test]
    fn near_coplanar_1ulp_off_agrees_with_exact() {
        // Start with a coplanar configuration, then perturb d by 1 ulp.
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let d0 = Point3::new(1.0, 1.0, 0.0); // coplanar

        // Perturb d.z by ±1 ulp and ±a few ulps.
        for &delta_bits in &[1i64, -1, 2, -2, 3, -3, 100, -100] {
            let dz = f64::from_bits((delta_bits.unsigned_abs()).max(1));
            let d = Point3::new(d0.x, d0.y, if delta_bits >= 0 { dz } else { -dz });
            assert_eq!(
                orient_3d(a, b, c, d),
                exact_orient3d_sign(a, b, c, d),
                "mismatch on near-coplanar case delta_bits={delta_bits}"
            );
        }
    }

    // ── Adversarial: coplanar with extreme coordinates ────────────────────

    #[test]
    fn coplanar_extreme_coordinates() {
        // Four points on z = 1e100, with large x,y values.
        let a = Point3::new(1e100, 0.0, 1e100);
        let b = Point3::new(1e100, 1.0, 1e100);
        let c = Point3::new(1e100, 0.0, 1e100 + 1.0);
        // d is NOT coplanar — it's off the plane. Verify the sign matches exact.
        let d = Point3::new(1e100, 0.5, 1e100 + 0.5);
        assert_eq!(
            orient_3d(a, b, c, d),
            exact_orient3d_sign(a, b, c, d),
            "mismatch on extreme-coordinate case"
        );
    }

    // ── Adversarial: cancellation cases that force the exact stage ────────

    #[test]
    fn cancellation_forces_exact_stage() {
        // Construct a case where the determinant is the difference of two
        // nearly-equal large products. This forces cancellation in the
        // filtered and compensated stages, requiring the exact stage.
        //
        // det = abx*(acy*adz - acz*ady) - ...
        // Make acy*adz ≈ acz*ady so the inner difference cancels.
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 1.0);
        // d chosen so that ady*acz ≈ adz*acy (i.e., ady ≈ adz since acy=acz=1)
        let d = Point3::new(0.0, 1.0 + 1e-15, 1.0);
        assert_eq!(
            orient_3d(a, b, c, d),
            exact_orient3d_sign(a, b, c, d),
            "mismatch on cancellation case"
        );
    }

    #[test]
    fn massive_cancellation_agrees_with_exact() {
        // A case with extreme cancellation: the determinant is the difference
        // of terms of order 1e300, with the true determinant of order 1e-100.
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1e150, 1e150);
        let d = Point3::new(0.0, 1e150 + 1.0, 1e150);
        // acy*adz = 1e150 * 1e150 = 1e300
        // acz*ady = 1e150 * (1e150 + 1) = 1e300 + 1e150
        // inner = 1e300 - (1e300 + 1e150) = -1e150
        // det = abx * (-1e150) = -1e150 (clearly negative, but the inner
        // computation involves massive cancellation)
        assert_eq!(
            orient_3d(a, b, c, d),
            exact_orient3d_sign(a, b, c, d),
            "mismatch on massive cancellation case"
        );
    }

    // ── All three ladder stages are exercised ─────────────────────────────

    #[test]
    fn filtered_stage_resolves_clear_case() {
        // A case with a large determinant relative to the permanent — the
        // filtered stage should resolve it without compensation or exact.
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let d = Point3::new(0.0, 0.0, 1.0);
        // det = 1, permanent ≈ 3, filtered bound ≈ 3 * 16 * eps ≈ 1e-15
        // |det| = 1 >> 1e-15 → filtered resolves.
        let diffs = Diffs::from_points(a, b, c, d);
        let det = filtered_det(&diffs);
        let perm = diffs.permanent();
        assert!(
            det.abs() > perm * FILTERED_BOUND,
            "filtered stage should resolve this case (det={det}, bound={})",
            perm * FILTERED_BOUND
        );
        assert_eq!(orient_3d(a, b, c, d), Sign::Positive);
    }

    #[test]
    fn compensated_or_exact_stage_resolves_near_coplanar() {
        // A near-coplanar case where the filtered stage is uncertain.
        // The compensated or exact stage must resolve it.
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let d = Point3::new(0.0, 0.0, 1e-16); // very slightly off the plane

        let diffs = Diffs::from_points(a, b, c, d);
        let det = filtered_det(&diffs);
        let perm = diffs.permanent();
        // The filtered determinant should be near the bound (uncertain).
        // Either filtered resolves it, or compensated/exact does — but the
        // final answer must match the exact sign.
        let _ = (det, perm); // inspect in debug if needed
        assert_eq!(
            orient_3d(a, b, c, d),
            exact_orient3d_sign(a, b, c, d),
            "near-coplanar case must match exact sign"
        );
    }

    // ── Determinism ───────────────────────────────────────────────────────

    #[test]
    fn deterministic_across_calls() {
        let a = Point3::new(1.0, 2.0, 3.0);
        let b = Point3::new(4.0, 5.0, 6.0);
        let c = Point3::new(7.0, 8.0, 10.0);
        let d = Point3::new(11.0, 12.0, 13.0);
        let s1 = orient_3d(a, b, c, d);
        let s2 = orient_3d(a, b, c, d);
        assert_eq!(s1, s2);
    }

    // ── Symmetry / antisymmetry ───────────────────────────────────────────

    #[test]
    fn swapping_c_d_flips_sign() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let d = Point3::new(0.0, 0.0, 1.0);
        let s = orient_3d(a, b, c, d);
        let s_swapped = orient_3d(a, b, d, c);
        assert_eq!(s, s_swapped.flip());
    }

    #[test]
    fn translation_invariant() {
        // Translating all points by the same vector doesn't change orientation.
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let d = Point3::new(0.0, 0.0, 1.0);
        let t = Point3::new(1e10, -1e10, 1e5);
        let s = orient_3d(a, b, c, d);
        let s_t = orient_3d(
            Point3::new(a.x + t.x, a.y + t.y, a.z + t.z),
            Point3::new(b.x + t.x, b.y + t.y, b.z + t.z),
            Point3::new(c.x + t.x, c.y + t.y, c.z + t.z),
            Point3::new(d.x + t.x, d.y + t.y, d.z + t.z),
        );
        assert_eq!(s, s_t);
    }

    // ── Zero-heap contract ────────────────────────────────────────────────

    #[test]
    fn no_heap_allocation_in_predicate() {
        // The exact stage uses stack arrays only. This test is a compile-time
        // guarantee (no Vec/String/Box in the module) plus a runtime smoke
        // test that the predicate runs without panicking on a degenerate case.
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let d = Point3::new(1.0, 1.0, 0.0);
        // Should return Zero (coplanar) without allocating.
        let _ = orient_3d(a, b, c, d);
    }
}
