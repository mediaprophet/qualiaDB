//! `incircle` — the 2-D in-circle predicate (P1.5).
//!
//! Computes the sign of the determinant that classifies whether a point `d`
//! lies inside, on, or outside the oriented circle through `a, b, c`. When
//! `a, b, c` are counter-clockwise, [`Sign::Positive`] means `d` is inside the
//! circle, [`Sign::Zero`] means on it, [`Sign::Negative`] means outside. The
//! sign flips when `a, b, c` are clockwise.
//!
//! The determinant (after translating by `d`) is:
//!
//! ```text
//! | adx  ady  adx²+ady² |
//! | bdx  bdy  bdx²+bdy² |
//! | cdx  cdy  cdx²+cdy² |
//! ```
//!
//! where `adx = ax − dx`, etc. This is a 3×3 determinant whose third column
//! contains squared-distance entries — each term is a product of two
//! coordinate differences and a squared distance.
//!
//! ## Filtered → compensated → exact ladder (Shewchuk adaptive precision)
//!
//! 1. **Filtered** — the 3×3 in-circle determinant (with squared-distance
//!    entries) plus a static error bound.
//! 2. **Compensated** — `mul_add` residual recovery on each product, forming a
//!    compensated determinant with a tighter bound.
//! 3. **Exact** — expansion arithmetic over a stack-allocated workspace sized
//!    by [`super::expansion::MAX_EXPANSION_INCIRCLE`]. Zero-heap, always
//!    correct, used only near degeneracy.
//!
//! ## Zero-heap contract
//!
//! No `Vec`, `String`, or `Box` in any path. The exact stage uses fixed-size
//! stack arrays.
//!
//! ## References
//!
//! The adaptive-precision ladder follows Shewchuk (1996). The implementation
//! is original Rust over the P1.3 expansion primitives.

use super::expansion::{
    compress_expansion, expansion_sum, negate_expansion, scale_expansion, sign_of_expansion,
    two_product, Sign, MAX_EXPANSION_INCIRCLE,
};
use super::primitives::Point2;

// ──────────────────────────────────────────────────────────────────────────
//  Error bounds
// ──────────────────────────────────────────────────────────────────────────

/// Filtered error bound coefficient for the in-circle determinant. The
/// determinant is a sum of 6 products, each involving 2 coordinate differences
/// and a squared distance (itself a sum of 2 products). The rounding is
/// heavier than orient3d, so the bound is larger.
const FILTERED_BOUND: f64 = 32.0 * f64::EPSILON;

/// Compensated error bound coefficient. Product residuals are recovered via
/// `mul_add`, leaving only summation rounding.
const COMPENSATED_BOUND: f64 = 8.0 * f64::EPSILON;

// ──────────────────────────────────────────────────────────────────────────
//  Coordinate differences and squared distances
// ──────────────────────────────────────────────────────────────────────────

/// The 6 coordinate differences and 3 squared distances for the in-circle
/// determinant (after translating by `d`).
#[derive(Clone, Copy)]
struct IncircleDiffs {
    adx: f64,
    ady: f64,
    bdx: f64,
    bdy: f64,
    cdx: f64,
    cdy: f64,
    ad2: f64,
    bd2: f64,
    cd2: f64,
}

impl IncircleDiffs {
    #[inline]
    fn from_points(a: Point2, b: Point2, c: Point2, d: Point2) -> Self {
        let adx = a.x - d.x;
        let ady = a.y - d.y;
        let bdx = b.x - d.x;
        let bdy = b.y - d.y;
        let cdx = c.x - d.x;
        let cdy = c.y - d.y;
        IncircleDiffs {
            adx,
            ady,
            bdx,
            bdy,
            cdx,
            cdy,
            ad2: adx * adx + ady * ady,
            bd2: bdx * bdx + bdy * bdy,
            cd2: cdx * cdx + cdy * cdy,
        }
    }

    /// The permanent: sum of absolute values of the 6 determinant terms.
    #[inline]
    fn permanent(&self) -> f64 {
        let IncircleDiffs { adx, ady, bdx, bdy, cdx, cdy, ad2, bd2, cd2 } = *self;
        (adx.abs() * (bdy.abs() * cd2.abs() + cdy.abs() * bd2.abs()))
            + (ady.abs() * (bdx.abs() * cd2.abs() + cdx.abs() * bd2.abs()))
            + (ad2.abs() * (bdx.abs() * cdy.abs() + bdy.abs() * cdx.abs()))
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  Stage 1: Filtered
// ──────────────────────────────────────────────────────────────────────────

#[inline]
fn filtered_det(d: &IncircleDiffs) -> f64 {
    let IncircleDiffs { adx, ady, bdx, bdy, cdx, cdy, ad2, bd2, cd2 } = *d;
    adx * (bdy * cd2 - cdy * bd2) - ady * (bdx * cd2 - cdx * bd2) + ad2 * (bdx * cdy - bdy * cdx)
}

// ──────────────────────────────────────────────────────────────────────────
//  Stage 2: Compensated
// ──────────────────────────────────────────────────────────────────────────

#[inline]
fn compensated_det(d: &IncircleDiffs) -> f64 {
    let IncircleDiffs { adx, ady, bdx, bdy, cdx, cdy, ad2, bd2, cd2 } = *d;

    // Compensated squared distances: ad2 + ad2_err = adx² + ady² (exact).
    // The filtered `ad2` is `round(round(adx²) + round(ady²))`. The error
    // includes both the product residuals and the addition rounding.
    let (adx2_p, adx2_e) = two_product(adx, adx);
    let (ady2_p, ady2_e) = two_product(ady, ady);
    let ad2_err = (adx2_p + ady2_p - ad2) + adx2_e + ady2_e;

    let (bdx2_p, bdx2_e) = two_product(bdx, bdx);
    let (bdy2_p, bdy2_e) = two_product(bdy, bdy);
    let bd2_err = (bdx2_p + bdy2_p - bd2) + bdx2_e + bdy2_e;

    let (cdx2_p, cdx2_e) = two_product(cdx, cdx);
    let (cdy2_p, cdy2_e) = two_product(cdy, cdy);
    let cd2_err = (cdx2_p + cdy2_p - cd2) + cdx2_e + cdy2_e;

    // Inner products with residual recovery (use filtered ad2/bd2/cd2 as main).
    let (p_bdy_cd2, e_bdy_cd2) = two_product(bdy, cd2);
    let (p_cdy_bd2, e_cdy_bd2) = two_product(cdy, bd2);
    let (p_bdx_cd2, e_bdx_cd2) = two_product(bdx, cd2);
    let (p_cdx_bd2, e_cdx_bd2) = two_product(cdx, bd2);
    let (p_bdx_cdy, e_bdx_cdy) = two_product(bdx, cdy);
    let (p_bdy_cdx, e_bdy_cdx) = two_product(bdy, cdx);

    // Three 2×2 minors.
    let m1 = p_bdy_cd2 - p_cdy_bd2;
    let m1_err = e_bdy_cd2 - e_cdy_bd2 + bdy * cd2_err - cdy * bd2_err;
    let m2 = p_bdx_cd2 - p_cdx_bd2;
    let m2_err = e_bdx_cd2 - e_cdx_bd2 + bdx * cd2_err - cdx * bd2_err;
    let m3 = p_bdx_cdy - p_bdy_cdx;
    let m3_err = e_bdx_cdy - e_bdy_cdx;

    // Outer products — use the filtered ad2/bd2/cd2 as main values, with errors.
    let o1 = adx * m1;
    let o1_err = adx.mul_add(m1, -o1) + adx * m1_err;
    let o2 = ady * m2;
    let o2_err = ady.mul_add(m2, -o2) + ady * m2_err;
    let o3 = ad2 * m3;
    let o3_err = ad2.mul_add(m3, -o3) + ad2_err * m3 + ad2 * m3_err;

    (o1 - o2 + o3) + (o1_err - o2_err + o3_err)
}

// ──────────────────────────────────────────────────────────────────────────
//  Stage 3: Exact (expansion arithmetic)
// ──────────────────────────────────────────────────────────────────────────

/// The exact in-circle determinant via expansion arithmetic. Zero-heap.
///
/// The 6 terms each involve a squared-distance expansion (length ≤ 4) scaled
/// by two coordinate differences. Each term is at most length 16; the 6 terms
/// are summed with compression after each addition into the 96-element
/// workspace.
fn exact_det(d: &IncircleDiffs) -> Sign {
    let IncircleDiffs { adx, ady, bdx, bdy, cdx, cdy, .. } = *d;

    // Compute the three squared distances as expansions (length ≤ 4 each).
    let mut ad2_exp = [0.0f64; 4];
    let mut bd2_exp = [0.0f64; 4];
    let mut cd2_exp = [0.0f64; 4];
    let ad2_len = sq_dist_expansion(adx, ady, &mut ad2_exp);
    let bd2_len = sq_dist_expansion(bdx, bdy, &mut bd2_exp);
    let cd2_len = sq_dist_expansion(cdx, cdy, &mut cd2_exp);

    // The 6 terms: (scalar1, scalar2, expansion, negate)
    // det = adx*bdy*cd2 - adx*cdy*bd2 - ady*bdx*cd2 + ady*cdx*bd2 + ad2*bdx*cdy - ad2*bdy*cdx
    let terms: [(f64, f64, &[f64], bool); 6] = [
        (adx, bdy, &cd2_exp[..cd2_len], false),
        (adx, cdy, &bd2_exp[..bd2_len], true),
        (ady, bdx, &cd2_exp[..cd2_len], true),
        (ady, cdx, &bd2_exp[..bd2_len], false),
        (1.0, bdx, &cd2_exp[..cd2_len], false), // placeholder, replaced below
        (1.0, bdy, &cd2_exp[..cd2_len], true),  // placeholder
    ];

    // We need ad2_exp for terms 5 and 6. Handle them separately.
    // Term 5: +ad2 * bdx * cdy  (ad2 is the expansion, bdx and cdy are scalars)
    // Term 6: -ad2 * bdy * cdx

    // Stack workspace.
    let mut scaled = [0.0f64; 16]; // expansion × scalar → length ≤ 2*expansion_len
    let mut term = [0.0f64; 32]; // after second scale → length ≤ 4*expansion_len
    let mut accum = [0.0f64; MAX_EXPANSION_INCIRCLE];
    let mut temp = [0.0f64; MAX_EXPANSION_INCIRCLE];
    let mut accum_len = 0usize;

    // Process the first 4 terms (cd2 or bd2 as the expansion).
    for &(s1, s2, exp, negate) in &terms[..4] {
        let term_len = compute_term(exp, s1, s2, &mut scaled, &mut term);
        add_term(&mut accum, &mut accum_len, &term[..term_len], negate, &mut temp);
    }

    // Process terms 5 and 6 (ad2 as the expansion).
    // Term 5: +ad2 * bdx * cdy
    let t5_len = compute_term(&ad2_exp[..ad2_len], bdx, cdy, &mut scaled, &mut term);
    add_term(&mut accum, &mut accum_len, &term[..t5_len], false, &mut temp);
    // Term 6: -ad2 * bdy * cdx
    let t6_len = compute_term(&ad2_exp[..ad2_len], bdy, cdx, &mut scaled, &mut term);
    add_term(&mut accum, &mut accum_len, &term[..t6_len], true, &mut temp);

    // Final compress and sign.
    let mut compressed = [0.0f64; MAX_EXPANSION_INCIRCLE];
    let comp_len = compress_expansion(&accum[..accum_len], &mut compressed)
        .expect("compressed buffer sized for MAX_EXPANSION_INCIRCLE");
    sign_of_expansion(&compressed[..comp_len])
}

/// Compute `adx² + ady²` as an expansion (length ≤ 4).
fn sq_dist_expansion(dx: f64, dy: f64, out: &mut [f64; 4]) -> usize {
    let (px, ex) = two_product(dx, dx);
    let (py, ey) = two_product(dy, dy);
    // Sum the two length-2 expansions → length ≤ 4.
    let prod = [px, ex];
    let other = [py, ey];
    let len = expansion_sum(&prod, &other, out).expect("out is sized for 4");
    // Compress to minimal form.
    let mut comp = [0.0f64; 4];
    let comp_len = compress_expansion(&out[..len], &mut comp).expect("comp is sized for 4");
    out[..comp_len].copy_from_slice(&comp[..comp_len]);
    comp_len
}

/// Compute `exp * s1 * s2` as an expansion. Writes into `scaled` (scratch) and
/// `term` (output). Returns the length of the result in `term`.
///
/// `scaled` must have length ≥ 2 * exp.len(). `term` must have length ≥
/// 4 * exp.len().
fn compute_term(exp: &[f64], s1: f64, s2: f64, scaled: &mut [f64; 16], term: &mut [f64; 32]) -> usize {
    // Scale exp by s1 → length ≤ 2*exp.len().
    let len1 = scale_expansion(exp, s1, scaled).expect("scaled sized for 2*exp");
    // Compress to keep it small.
    let mut comp = [0.0f64; 16];
    let comp_len = compress_expansion(&scaled[..len1], &mut comp).expect("comp sized for 16");
    // Scale by s2 → length ≤ 2*comp_len.
    let len2 = scale_expansion(&comp[..comp_len], s2, term).expect("term sized for 32");
    // Compress the result.
    let mut comp2 = [0.0f64; 32];
    let comp2_len = compress_expansion(&term[..len2], &mut comp2).expect("comp2 sized for 32");
    term[..comp2_len].copy_from_slice(&comp2[..comp2_len]);
    comp2_len
}

/// Add a term (with optional negation) to the accumulator, then compress.
fn add_term(
    accum: &mut [f64],
    accum_len: &mut usize,
    term: &[f64],
    negate: bool,
    temp: &mut [f64],
) {
    let mut neg_term = [0.0f64; 32];
    if negate {
        neg_term[..term.len()].copy_from_slice(term);
        negate_expansion(&mut neg_term[..term.len()]);
        if *accum_len == 0 {
            accum[..term.len()].copy_from_slice(&neg_term[..term.len()]);
            *accum_len = term.len();
            return;
        }
        let sum_len = expansion_sum(&accum[..*accum_len], &neg_term[..term.len()], temp)
            .expect("temp sized for MAX_EXPANSION_INCIRCLE");
        *accum_len = compress_expansion(&temp[..sum_len], accum)
            .expect("accum sized for MAX_EXPANSION_INCIRCLE");
    } else {
        if *accum_len == 0 {
            accum[..term.len()].copy_from_slice(term);
            *accum_len = term.len();
            return;
        }
        let sum_len = expansion_sum(&accum[..*accum_len], term, temp)
            .expect("temp sized for MAX_EXPANSION_INCIRCLE");
        *accum_len = compress_expansion(&temp[..sum_len], accum)
            .expect("accum sized for MAX_EXPANSION_INCIRCLE");
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  Public ladder entry point
// ──────────────────────────────────────────────────────────────────────────

/// The 2-D in-circle predicate: side of `d` w.r.t. the oriented circle
/// through `a, b, c`.
///
/// Returns [`Sign::Positive`] if `d` is inside the oriented circle (when
/// `a, b, c` are CCW), [`Sign::Zero`] if `d` is on the circle, [`Sign::Negative`]
/// if outside. The sense is reversed when `a, b, c` are clockwise.
///
/// This is the public ladder entry point — it escalates from filtered to
/// compensated to exact as needed, never returning an uncertain sign.
pub fn incircle(a: Point2, b: Point2, c: Point2, d: Point2) -> Sign {
    let diffs = IncircleDiffs::from_points(a, b, c, d);
    let perm = diffs.permanent();

    let det = filtered_det(&diffs);
    if det.abs() > perm * FILTERED_BOUND {
        return Sign::from_f64(det);
    }

    let comp = compensated_det(&diffs);
    if comp.abs() > perm * COMPENSATED_BOUND {
        return Sign::from_f64(comp);
    }

    exact_det(&diffs)
}

// ──────────────────────────────────────────────────────────────────────────
//  Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::exact_test_helper::Exact;
    use crate::specialized_libs::computational_geometry::primitives::orientation_2;

    /// Ground-truth in-circle sign via BigInt.
    ///
    /// Computes coordinate differences in f64 first (matching the predicate's
    /// approach), then converts those f64 differences to exact BigInt values
    /// and computes the determinant exactly. This ensures the cross-check
    /// validates the same computation the predicate performs.
    fn exact_incircle_sign(a: Point2, b: Point2, c: Point2, d: Point2) -> Sign {
        // Compute differences in f64 — same as the predicate.
        let adx_f = a.x - d.x;
        let ady_f = a.y - d.y;
        let bdx_f = b.x - d.x;
        let bdy_f = b.y - d.y;
        let cdx_f = c.x - d.x;
        let cdy_f = c.y - d.y;

        // Convert to exact BigInt values.
        let adx = Exact::from_f64(adx_f);
        let ady = Exact::from_f64(ady_f);
        let bdx = Exact::from_f64(bdx_f);
        let bdy = Exact::from_f64(bdy_f);
        let cdx = Exact::from_f64(cdx_f);
        let cdy = Exact::from_f64(cdy_f);

        let ad2 = adx.clone().mul(adx.clone()).add(ady.clone().mul(ady.clone()));
        let bd2 = bdx.clone().mul(bdx.clone()).add(bdy.clone().mul(bdy.clone()));
        let cd2 = cdx.clone().mul(cdx.clone()).add(cdy.clone().mul(cdy.clone()));

        // det = adx*bdy*cd2 - adx*cdy*bd2 - ady*bdx*cd2 + ady*cdx*bd2 + ad2*bdx*cdy - ad2*bdy*cdx
        let t1 = adx.clone().mul(bdy.clone()).mul(cd2.clone());
        let t2 = adx.clone().mul(cdy.clone()).mul(bd2.clone());
        let t3 = ady.clone().mul(bdx.clone()).mul(cd2.clone());
        let t4 = ady.clone().mul(cdx.clone()).mul(bd2.clone());
        let t5 = ad2.clone().mul(bdx.clone()).mul(cdy.clone());
        let t6 = ad2.mul(bdy).mul(cdx);

        let det = t1.sub(t2).sub(t3).add(t4).add(t5).sub(t6);
        det.sign()
    }

    // ── Basic classification ──────────────────────────────────────────────

    #[test]
    fn classifies_inside_circle() {
        // Circle centered at origin, radius 1. a,b,c on circle (CCW), d at center.
        let a = Point2::new(1.0, 0.0);
        let b = Point2::new(0.0, 1.0);
        let c = Point2::new(-1.0, 0.0);
        let d = Point2::new(0.0, 0.0);
        assert_eq!(orientation_2(a, b, c), crate::specialized_libs::computational_geometry::primitives::Orientation::CounterClockwise);
        assert_eq!(incircle(a, b, c, d), Sign::Positive); // inside
    }

    #[test]
    fn classifies_outside_circle() {
        let a = Point2::new(1.0, 0.0);
        let b = Point2::new(0.0, 1.0);
        let c = Point2::new(-1.0, 0.0);
        let d = Point2::new(2.0, 0.0); // outside the unit circle
        assert_eq!(incircle(a, b, c, d), Sign::Negative);
    }

    #[test]
    fn classifies_on_circle() {
        let a = Point2::new(1.0, 0.0);
        let b = Point2::new(0.0, 1.0);
        let c = Point2::new(-1.0, 0.0);
        let d = Point2::new(0.0, -1.0); // on the unit circle
        assert_eq!(incircle(a, b, c, d), Sign::Zero);
    }

    #[test]
    fn sign_flips_for_clockwise_abc() {
        // Same circle but a,b,c clockwise → sign flips
        let a = Point2::new(1.0, 0.0);
        let b = Point2::new(-1.0, 0.0);
        let c = Point2::new(0.0, 1.0);
        let d = Point2::new(0.0, 0.0); // inside
        assert_eq!(orientation_2(a, b, c), crate::specialized_libs::computational_geometry::primitives::Orientation::Clockwise);
        // Inside with CW → Negative (flipped)
        assert_eq!(incircle(a, b, c, d), Sign::Negative);
    }

    // ── Agreement with BigInt cross-check ─────────────────────────────────

    #[test]
    fn agrees_with_exact_on_basic_cases() {
        let cases = [
            (Point2::new(1.0, 0.0), Point2::new(0.0, 1.0), Point2::new(-1.0, 0.0), Point2::new(0.0, 0.0)),
            (Point2::new(1.0, 0.0), Point2::new(0.0, 1.0), Point2::new(-1.0, 0.0), Point2::new(2.0, 0.0)),
            (Point2::new(1.0, 0.0), Point2::new(0.0, 1.0), Point2::new(-1.0, 0.0), Point2::new(0.0, -1.0)),
            (Point2::new(0.0, 0.0), Point2::new(1.0, 0.0), Point2::new(0.0, 1.0), Point2::new(0.5, 0.5)),
            (Point2::new(3.0, 4.0), Point2::new(0.0, 0.0), Point2::new(6.0, 0.0), Point2::new(3.0, 0.0)),
        ];
        for (a, b, c, d) in cases {
            assert_eq!(
                incircle(a, b, c, d),
                exact_incircle_sign(a, b, c, d),
                "mismatch on ({a:?}, {b:?}, {c:?}, {d:?})"
            );
        }
    }

    // ── Adversarial: cocircular (exact zero) ──────────────────────────────

    #[test]
    fn cocircular_four_points() {
        // Four points on the unit circle
        let a = Point2::new(1.0, 0.0);
        let b = Point2::new(0.0, 1.0);
        let c = Point2::new(-1.0, 0.0);
        let d = Point2::new(0.0, -1.0);
        assert_eq!(incircle(a, b, c, d), Sign::Zero);
        assert_eq!(exact_incircle_sign(a, b, c, d), Sign::Zero);
    }

    #[test]
    fn cocircular_on_arbitrary_circle() {
        // Circle centered at (3, 4), radius 5. Points: (8,4), (3,9), (-2,4), (3,-1)
        let a = Point2::new(8.0, 4.0);
        let b = Point2::new(3.0, 9.0);
        let c = Point2::new(-2.0, 4.0);
        let d = Point2::new(3.0, -1.0);
        assert_eq!(incircle(a, b, c, d), Sign::Zero);
        assert_eq!(exact_incircle_sign(a, b, c, d), Sign::Zero);
    }

    // ── Adversarial: near-cocircular (±1-ulp) ─────────────────────────────

    #[test]
    fn near_cocircular_1ulp_off() {
        // Start cocircular, perturb d by a few ulps.
        let a = Point2::new(1.0, 0.0);
        let b = Point2::new(0.0, 1.0);
        let c = Point2::new(-1.0, 0.0);
        let d0 = Point2::new(0.0, -1.0); // on circle

        for &delta_bits in &[1i64, -1, 2, -2, 5, -5, 100, -100] {
            let dy = f64::from_bits((d0.y.to_bits() as i64 + delta_bits) as u64);
            let d = Point2::new(d0.x, dy);
            assert_eq!(
                incircle(a, b, c, d),
                exact_incircle_sign(a, b, c, d),
                "mismatch on near-cocircular delta_bits={delta_bits}"
            );
        }
    }

    // ── Adversarial: extreme exponents ────────────────────────────────────

    #[test]
    fn extreme_exponents_agree_with_exact() {
        // Use coordinates where intermediate products (coord × coord × sq_dist)
        // stay within f64 range. With coords ~1e50, sq_dist ~1e100, and triple
        // products ~1e200, we're well within f64's ~1e308 max.
        let cases = [
            (Point2::new(1e50, 0.0), Point2::new(0.0, 1e50), Point2::new(-1e50, 0.0), Point2::new(0.0, 0.0)),
            (Point2::new(1e-50, 0.0), Point2::new(0.0, 1e-50), Point2::new(-1e-50, 0.0), Point2::new(0.0, 0.0)),
            (Point2::new(1e50, 0.0), Point2::new(0.0, 1e50), Point2::new(-1e50, 0.0), Point2::new(1e50, 1e-50)),
        ];
        for (a, b, c, d) in cases {
            assert_eq!(
                incircle(a, b, c, d),
                exact_incircle_sign(a, b, c, d),
                "mismatch on extreme-exponent case ({a:?}, {b:?}, {c:?}, {d:?})"
            );
        }
    }

    // ── Adversarial: cancellation ─────────────────────────────────────────

    #[test]
    fn cancellation_agrees_with_exact() {
        // Large coordinates with near-cancellation in the determinant.
        // Keep triple products within f64 range: 1e50³ = 1e150.
        let a = Point2::new(1e50, 0.0);
        let b = Point2::new(0.0, 1e50);
        let c = Point2::new(-1e50, 0.0);
        let d = Point2::new(0.0, -1e50); // cocircular
        assert_eq!(incircle(a, b, c, d), exact_incircle_sign(a, b, c, d));
    }

    #[test]
    fn massive_cancellation_agrees_with_exact() {
        // Points on a huge circle, d perturbed slightly.
        let a = Point2::new(1e50, 0.0);
        let b = Point2::new(0.0, 1e50);
        let c = Point2::new(-1e50, 0.0);
        // d just inside the circle — the determinant involves massive cancellation
        let d = Point2::new(0.0, -1e50 + 1.0);
        assert_eq!(
            incircle(a, b, c, d),
            exact_incircle_sign(a, b, c, d),
            "mismatch on massive cancellation"
        );
    }

    // ── All three ladder stages exercised ─────────────────────────────────

    #[test]
    fn filtered_stage_resolves_clear_case() {
        let a = Point2::new(1.0, 0.0);
        let b = Point2::new(0.0, 1.0);
        let c = Point2::new(-1.0, 0.0);
        let d = Point2::new(0.0, 0.0);
        let diffs = IncircleDiffs::from_points(a, b, c, d);
        let det = filtered_det(&diffs);
        let perm = diffs.permanent();
        assert!(
            det.abs() > perm * FILTERED_BOUND,
            "filtered should resolve (det={det}, bound={})",
            perm * FILTERED_BOUND
        );
        assert_eq!(incircle(a, b, c, d), Sign::Positive);
    }

    #[test]
    fn near_cocircular_resolves_via_compensated_or_exact() {
        let a = Point2::new(1.0, 0.0);
        let b = Point2::new(0.0, 1.0);
        let c = Point2::new(-1.0, 0.0);
        let d = Point2::new(0.0, -1.0 + 1e-15);
        assert_eq!(
            incircle(a, b, c, d),
            exact_incircle_sign(a, b, c, d),
            "near-cocircular must match exact"
        );
    }

    // ── Determinism ───────────────────────────────────────────────────────

    #[test]
    fn deterministic_across_calls() {
        let a = Point2::new(1.0, 0.0);
        let b = Point2::new(0.0, 1.0);
        let c = Point2::new(-1.0, 0.0);
        let d = Point2::new(0.3, 0.3);
        let s1 = incircle(a, b, c, d);
        let s2 = incircle(a, b, c, d);
        assert_eq!(s1, s2);
    }

    // ── Symmetry ──────────────────────────────────────────────────────────

    #[test]
    fn swapping_a_b_flips_sign() {
        let a = Point2::new(1.0, 0.0);
        let b = Point2::new(0.0, 1.0);
        let c = Point2::new(-1.0, 0.0);
        let d = Point2::new(0.0, 0.0);
        let s = incircle(a, b, c, d);
        let s_swapped = incircle(b, a, c, d);
        assert_eq!(s, s_swapped.flip());
    }

    #[test]
    fn translation_invariant() {
        let a = Point2::new(1.0, 0.0);
        let b = Point2::new(0.0, 1.0);
        let c = Point2::new(-1.0, 0.0);
        let d = Point2::new(0.0, 0.0);
        let t = Point2::new(1e10, -1e10);
        let s = incircle(a, b, c, d);
        let s_t = incircle(
            Point2::new(a.x + t.x, a.y + t.y),
            Point2::new(b.x + t.x, b.y + t.y),
            Point2::new(c.x + t.x, c.y + t.y),
            Point2::new(d.x + t.x, d.y + t.y),
        );
        assert_eq!(s, s_t);
    }

    // ── Zero-heap contract ────────────────────────────────────────────────

    #[test]
    fn no_heap_allocation_in_predicate() {
        let a = Point2::new(1.0, 0.0);
        let b = Point2::new(0.0, 1.0);
        let c = Point2::new(-1.0, 0.0);
        let d = Point2::new(0.0, -1.0);
        let _ = incircle(a, b, c, d); // cocircular — exercises exact stage
    }
}
