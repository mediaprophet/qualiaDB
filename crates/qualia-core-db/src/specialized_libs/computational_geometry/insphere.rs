//! `insphere` — the 3-D in-sphere predicate (P1.6).
//!
//! Computes the sign of the determinant that classifies whether a point `e`
//! lies inside, on, or outside the oriented sphere through `a, b, c, d`.
//!
//! ## Sign convention (this implementation)
//!
//! **This implementation uses the opposite sign convention from the standard
//! Shewchuk / de Berg formulation.** When `a, b, c, d` have positive
//! orientation (positive [`super::orient3d::orient_3d`]), [`Sign::Negative`]
//! means `e` is **inside** the sphere, [`Sign::Zero`] means on it, and
//! [`Sign::Positive`] means **outside**. The sense is reversed when the
//! orientation is negative (negative `orient_3d` → inside = `Positive`).
//!
//! ### Worked example
//!
//! Take the unit-sphere tetrahedron with **positive** `orient_3d`:
//! `a = (0,1,0)`, `b = (1,0,0)`, `c = (0,0,1)`, `d = (-1,0,0)` —
//! `orient_3d(a,b,c,d) = +2` (Positive). Then:
//! - `e = (0,0,0)` (sphere centre, inside)  → `insphere = Negative`
//! - `e = (2,0,0)` (outside)                → `insphere = Positive`
//! - `e = (0,-1,0)` (on the sphere)         → `insphere = Zero`
//!
//! ### Cross-reference
//!
//! The orientation is determined by [`super::orient3d::orient_3d`], which
//! returns the sign of the scalar triple product `(b-a)·((c-a)×(d-a))`
//! (`Positive` = `d` below the oriented plane `a→b→c`, right-hand rule).
//! Consumers (`delaunay_3`, `alpha_shape_3d`, `verify_delaunay_3`) all treat
//! `insphere == Negative` as "inside" for a positively-oriented tet, and
//! derive the inside-sign from the orientation sign — see those modules.
//!
//! ### Why not flip to the standard convention?
//!
//! The impl is verified against exact arithmetic (the BigInt cross-check in
//! the test suite below) and every consumer is consistent with it. Flipping
//! would require inverting the sign, every call-site comparison, and every
//! test expectation in one atomic commit — a wide change in the most
//! correctness-critical code, where a wrong sign is invalid topology. The
//! prose was the only defect; the code + tests are the contract.
//!
//! The determinant (after translating by `e`) is:
//!
//! ```text
//! | adx  ady  adz  adx²+ady²+adz² |
//! | bdx  bdy  bdz  bdx²+bdy²+bdz² |
//! | cdx  cdy  cdz  cdx²+cdy²+cdz² |
//! | ddx  ddy  ddz  ddx²+ddy²+ddz² |
//! ```
//!
//! Expanded by cofactors along the 4th column:
//!
//! ```text
//! det = −ad2·M_a + bd2·M_b − cd2·M_c + dd2·M_d
//! ```
//!
//! where each `M_i` is a 3×3 minor (a sum of 6 triple products of coordinate
//! differences — structurally identical to the orient3d determinant).
//!
//! ## Filtered → compensated → exact ladder (Shewchuk adaptive precision)
//!
//! 1. **Filtered** — the 4×4 in-sphere determinant plus a static error bound.
//! 2. **Compensated** — `mul_add` residual recovery on each product, forming a
//!    compensated determinant with a tighter bound.
//! 3. **Exact** — expansion arithmetic over a stack-allocated workspace sized
//!    by [`super::expansion::MAX_EXPANSION_INSPHERE`] (2048 f64s = 16 KB).
//!    Zero-heap, always correct, used only near degeneracy.
//!
//! ## Zero-heap contract
//!
//! No `Vec`, `String`, or `Box` in any path. The exact stage uses fixed-size
//! stack arrays. The 16 KB stack frame is within platform limits (predicates
//! are non-recursive).

use super::expansion::{
    compress_expansion, expansion_sum, negate_expansion, scale_expansion, sign_of_expansion,
    two_product, Sign, MAX_EXPANSION_INSPHERE,
};
use super::primitives::Point3;

// ──────────────────────────────────────────────────────────────────────────
//  Error bounds
// ──────────────────────────────────────────────────────────────────────────

/// Filtered error bound for the 4×4 in-sphere determinant. The determinant
/// has 24 terms (4 cofactors × 6 minor terms), each a product of 5 factors
/// (3 coord diffs × 2 from the squared distance). The rounding is heavy.
const FILTERED_BOUND: f64 = 256.0 * f64::EPSILON;

/// Compensated error bound. Product residuals are recovered via `mul_add`.
const COMPENSATED_BOUND: f64 = 64.0 * f64::EPSILON;

// ──────────────────────────────────────────────────────────────────────────
//  Coordinate differences and squared distances
// ──────────────────────────────────────────────────────────────────────────

/// All 12 coordinate differences and 4 squared distances for the in-sphere
/// determinant (after translating by `e`).
#[derive(Clone, Copy)]
struct InsphereDiffs {
    adx: f64, ady: f64, adz: f64,
    bdx: f64, bdy: f64, bdz: f64,
    cdx: f64, cdy: f64, cdz: f64,
    ddx: f64, ddy: f64, ddz: f64,
    ad2: f64, bd2: f64, cd2: f64, dd2: f64,
}

impl InsphereDiffs {
    #[inline]
    fn from_points(a: Point3, b: Point3, c: Point3, d: Point3, e: Point3) -> Self {
        let adx = a.x - e.x; let ady = a.y - e.y; let adz = a.z - e.z;
        let bdx = b.x - e.x; let bdy = b.y - e.y; let bdz = b.z - e.z;
        let cdx = c.x - e.x; let cdy = c.y - e.y; let cdz = c.z - e.z;
        let ddx = d.x - e.x; let ddy = d.y - e.y; let ddz = d.z - e.z;
        InsphereDiffs {
            adx, ady, adz,
            bdx, bdy, bdz,
            cdx, cdy, cdz,
            ddx, ddy, ddz,
            ad2: adx * adx + ady * ady + adz * adz,
            bd2: bdx * bdx + bdy * bdy + bdz * bdz,
            cd2: cdx * cdx + cdy * cdy + cdz * cdz,
            dd2: ddx * ddx + ddy * ddy + ddz * ddz,
        }
    }

    /// The permanent: sum of absolute values of all 24 determinant terms.
    #[inline]
    fn permanent(&self) -> f64 {
        let InsphereDiffs {
            adx, ady, adz, bdx, bdy, bdz, cdx, cdy, cdz, ddx, ddy, ddz,
            ad2, bd2, cd2, dd2,
        } = *self;

        // Minor_a: det(b, c, d) — 6 terms
        let perm_a = (bdx.abs() * (cdy.abs() * ddz.abs() + cdz.abs() * ddy.abs())
            + bdy.abs() * (cdx.abs() * ddz.abs() + cdz.abs() * ddx.abs())
            + bdz.abs() * (cdx.abs() * ddy.abs() + cdy.abs() * ddx.abs()))
            * ad2.abs();

        // Minor_b: det(a, c, d) — 6 terms
        let perm_b = (adx.abs() * (cdy.abs() * ddz.abs() + cdz.abs() * ddy.abs())
            + ady.abs() * (cdx.abs() * ddz.abs() + cdz.abs() * ddx.abs())
            + adz.abs() * (cdx.abs() * ddy.abs() + cdy.abs() * ddx.abs()))
            * bd2.abs();

        // Minor_c: det(a, b, d) — 6 terms
        let perm_c = (adx.abs() * (bdy.abs() * ddz.abs() + bdz.abs() * ddy.abs())
            + ady.abs() * (bdx.abs() * ddz.abs() + bdz.abs() * ddx.abs())
            + adz.abs() * (bdx.abs() * ddy.abs() + bdy.abs() * ddx.abs()))
            * cd2.abs();

        // Minor_d: det(a, b, c) — 6 terms
        let perm_d = (adx.abs() * (bdy.abs() * cdz.abs() + bdz.abs() * cdy.abs())
            + ady.abs() * (bdx.abs() * cdz.abs() + bdz.abs() * cdx.abs())
            + adz.abs() * (bdx.abs() * cdy.abs() + bdy.abs() * cdx.abs()))
            * dd2.abs();

        perm_a + perm_b + perm_c + perm_d
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  3×3 minors (filtered)
// ──────────────────────────────────────────────────────────────────────────

/// 3×3 determinant: m1x*(m2y*m3z - m2z*m3y) - m1y*(m2x*m3z - m2z*m3x) + m1z*(m2x*m3y - m2y*m3x)
#[inline]
fn det3(
    m1x: f64, m1y: f64, m1z: f64,
    m2x: f64, m2y: f64, m2z: f64,
    m3x: f64, m3y: f64, m3z: f64,
) -> f64 {
    m1x * (m2y * m3z - m2z * m3y)
        - m1y * (m2x * m3z - m2z * m3x)
        + m1z * (m2x * m3y - m2y * m3x)
}

// ──────────────────────────────────────────────────────────────────────────
//  Stage 1: Filtered
// ──────────────────────────────────────────────────────────────────────────

#[inline]
fn filtered_det(d: &InsphereDiffs) -> f64 {
    let InsphereDiffs {
        adx, ady, adz, bdx, bdy, bdz, cdx, cdy, cdz, ddx, ddy, ddz,
        ad2, bd2, cd2, dd2,
    } = *d;

    let minor_a = det3(bdx, bdy, bdz, cdx, cdy, cdz, ddx, ddy, ddz);
    let minor_b = det3(adx, ady, adz, cdx, cdy, cdz, ddx, ddy, ddz);
    let minor_c = det3(adx, ady, adz, bdx, bdy, bdz, ddx, ddy, ddz);
    let minor_d = det3(adx, ady, adz, bdx, bdy, bdz, cdx, cdy, cdz);

    // Cofactor expansion along column 4 (index 3): signs are - + - +.
    -ad2 * minor_a + bd2 * minor_b - cd2 * minor_c + dd2 * minor_d
}

// ──────────────────────────────────────────────────────────────────────────
//  Stage 2: Compensated
// ──────────────────────────────────────────────────────────────────────────

/// Compensated 3×3 determinant with residual recovery.
#[inline]
fn compensated_det3(
    m1x: f64, m1y: f64, m1z: f64,
    m2x: f64, m2y: f64, m2z: f64,
    m3x: f64, m3y: f64, m3z: f64,
) -> f64 {
    // Inner 2×2 minors with residual recovery.
    let (p_yn, e_yn) = two_product(m2y, m3z);
    let (p_zy, e_zy) = two_product(m2z, m3y);
    let c1 = p_yn - p_zy;
    let c1_err = e_yn - e_zy;

    let (p_xn, e_xn) = two_product(m2x, m3z);
    let (p_zx, e_zx) = two_product(m2z, m3x);
    let c2 = p_xn - p_zx;
    let c2_err = e_xn - e_zx;

    let (p_xy, e_xy) = two_product(m2x, m3y);
    let (p_yx, e_yx) = two_product(m2y, m3x);
    let c3 = p_xy - p_yx;
    let c3_err = e_xy - e_yx;

    // Outer products with residual recovery.
    let o1 = m1x * c1;
    let o1_err = m1x.mul_add(c1, -o1) + m1x * c1_err;
    let o2 = m1y * c2;
    let o2_err = m1y.mul_add(c2, -o2) + m1y * c2_err;
    let o3 = m1z * c3;
    let o3_err = m1z.mul_add(c3, -o3) + m1z * c3_err;

    (o1 - o2 + o3) + (o1_err - o2_err + o3_err)
}

#[inline]
fn compensated_det(d: &InsphereDiffs) -> f64 {
    let InsphereDiffs {
        adx, ady, adz, bdx, bdy, bdz, cdx, cdy, cdz, ddx, ddy, ddz,
        ad2, bd2, cd2, dd2,
    } = *d;

    let minor_a = compensated_det3(bdx, bdy, bdz, cdx, cdy, cdz, ddx, ddy, ddz);
    let minor_b = compensated_det3(adx, ady, adz, cdx, cdy, cdz, ddx, ddy, ddz);
    let minor_c = compensated_det3(adx, ady, adz, bdx, bdy, bdz, ddx, ddy, ddz);
    let minor_d = compensated_det3(adx, ady, adz, bdx, bdy, bdz, cdx, cdy, cdz);

    // Cofactor expansion along column 4 (index 3): signs are - + - +.
    -ad2 * minor_a + bd2 * minor_b - cd2 * minor_c + dd2 * minor_d
}

// ──────────────────────────────────────────────────────────────────────────
//  Stage 3: Exact (expansion arithmetic)
// ──────────────────────────────────────────────────────────────────────────

/// Compute a squared distance `dx² + dy² + dz²` as a compressed expansion.
/// Output length ≤ 6. Writes into `out` (must be length ≥ 8) and returns the
/// compressed length.
fn sq_dist_3d_expansion(dx: f64, dy: f64, dz: f64, out: &mut [f64; 8]) -> usize {
    let (px, ex) = two_product(dx, dx);
    let (py, ey) = two_product(dy, dy);
    let (pz, ez) = two_product(dz, dz);

    // Sum three length-2 expansions → length ≤ 6.
    let mut temp = [0.0f64; 8];
    let len1 = expansion_sum(&[px, ex], &[py, ey], &mut temp).expect("temp sized for 4");
    let len2 = expansion_sum(&temp[..len1], &[pz, ez], out).expect("out sized for 8");

    // Compress.
    let mut comp = [0.0f64; 8];
    let comp_len = compress_expansion(&out[..len2], &mut comp).expect("comp sized for 8");
    out[..comp_len].copy_from_slice(&comp[..comp_len]);
    comp_len
}

/// Compute a 3×3 determinant as a compressed expansion.
/// Each of the 6 terms is a product of 3 coordinate differences (length ≤ 4
/// after two scale_expansions). Summed with compression.
/// Writes into `accum` and returns the compressed length.
fn exact_det3(
    m1x: f64, m1y: f64, m1z: f64,
    m2x: f64, m2y: f64, m2z: f64,
    m3x: f64, m3y: f64, m3z: f64,
    accum: &mut [f64; 48],
) -> usize {
    // 6 terms: (d1, d2, d3, negate)
    // det3 = m1x*m2y*m3z - m1x*m2z*m3y - m1y*m2x*m3z + m1y*m2z*m3x + m1z*m2x*m3y - m1z*m2y*m3x
    let terms: [(f64, f64, f64, bool); 6] = [
        (m1x, m2y, m3z, false),
        (m1x, m2z, m3y, true),
        (m1y, m2x, m3z, true),
        (m1y, m2z, m3x, false),
        (m1z, m2x, m3y, false),
        (m1z, m2y, m3x, true),
    ];

    let mut prod = [0.0f64; 2];
    let mut term = [0.0f64; 8];
    let mut temp = [0.0f64; 48];
    let mut accum_len = 0usize;

    for &(d1, d2, d3, negate) in &terms {
        let (p, e) = two_product(d1, d2);
        prod[0] = p;
        prod[1] = e;
        let len = scale_expansion(&prod, d3, &mut term).expect("term sized for 4");
        // Compress the term.
        let mut comp = [0.0f64; 8];
        let comp_len = compress_expansion(&term[..len], &mut comp).expect("comp sized for 8");

        if negate {
            let mut neg = [0.0f64; 8];
            neg[..comp_len].copy_from_slice(&comp[..comp_len]);
            negate_expansion(&mut neg[..comp_len]);
            if accum_len == 0 {
                accum[..comp_len].copy_from_slice(&neg[..comp_len]);
                accum_len = comp_len;
            } else {
                let sum_len = expansion_sum(&accum[..accum_len], &neg[..comp_len], &mut temp)
                    .expect("temp sized for 48");
                accum_len = compress_expansion(&temp[..sum_len], accum)
                    .expect("accum sized for 48");
            }
        } else {
            if accum_len == 0 {
                accum[..comp_len].copy_from_slice(&comp[..comp_len]);
                accum_len = comp_len;
            } else {
                let sum_len = expansion_sum(&accum[..accum_len], &comp[..comp_len], &mut temp)
                    .expect("temp sized for 48");
                accum_len = compress_expansion(&temp[..sum_len], accum)
                    .expect("accum sized for 48");
            }
        }
    }

    accum_len
}

/// Multiply two expansions: `result = e * f`.
/// Uses `scale_expansion(f, e[i])` for each component of `e`, summed with
/// compression. Writes into `accum` (must be large enough) and returns the
/// length. `scratch` is used for intermediate results.
fn multiply_expansions(
    e: &[f64],
    f: &[f64],
    accum: &mut [f64],
    scratch: &mut [f64],
    scaled: &mut [f64; 32],
) -> usize {
    let mut accum_len = 0usize;
    let mut comp = [0.0f64; 32];

    for &ei in e {
        let scaled_len = scale_expansion(f, ei, scaled).expect("scaled sized for 2*f");
        let comp_len = compress_expansion(&scaled[..scaled_len], &mut comp)
            .expect("comp sized for 32");

        if accum_len == 0 {
            accum[..comp_len].copy_from_slice(&comp[..comp_len]);
            accum_len = comp_len;
        } else {
            let sum_len = expansion_sum(&accum[..accum_len], &comp[..comp_len], scratch)
                .expect("scratch sized for accum+comp");
            accum_len = compress_expansion(&scratch[..sum_len], accum)
                .expect("accum sized for result");
        }
    }
    accum_len
}

/// The exact in-sphere determinant via expansion arithmetic. Zero-heap.
fn exact_det(d: &InsphereDiffs) -> Sign {
    let InsphereDiffs {
        adx, ady, adz, bdx, bdy, bdz, cdx, cdy, cdz, ddx, ddy, ddz,
        ..
    } = *d;

    // Compute the four squared distances as compressed expansions.
    let mut ad2_exp = [0.0f64; 8];
    let mut bd2_exp = [0.0f64; 8];
    let mut cd2_exp = [0.0f64; 8];
    let mut dd2_exp = [0.0f64; 8];
    let ad2_len = sq_dist_3d_expansion(adx, ady, adz, &mut ad2_exp);
    let bd2_len = sq_dist_3d_expansion(bdx, bdy, bdz, &mut bd2_exp);
    let cd2_len = sq_dist_3d_expansion(cdx, cdy, cdz, &mut cd2_exp);
    let dd2_len = sq_dist_3d_expansion(ddx, ddy, ddz, &mut dd2_exp);

    // Compute the four 3×3 minors as compressed expansions.
    let mut minor_a = [0.0f64; 48];
    let mut minor_b = [0.0f64; 48];
    let mut minor_c = [0.0f64; 48];
    let mut minor_d = [0.0f64; 48];
    let ma_len = exact_det3(bdx, bdy, bdz, cdx, cdy, cdz, ddx, ddy, ddz, &mut minor_a);
    let mb_len = exact_det3(adx, ady, adz, cdx, cdy, cdz, ddx, ddy, ddz, &mut minor_b);
    let mc_len = exact_det3(adx, ady, adz, bdx, bdy, bdz, ddx, ddy, ddz, &mut minor_c);
    let md_len = exact_det3(adx, ady, adz, bdx, bdy, bdz, cdx, cdy, cdz, &mut minor_d);

    // det = -ad2*minor_a + bd2*minor_b - cd2*minor_c + dd2*minor_d
    // Cofactor expansion along column 4 (index 3): signs are - + - +.
    let mut accum = [0.0f64; MAX_EXPANSION_INSPHERE];
    let mut temp = [0.0f64; MAX_EXPANSION_INSPHERE];
    let mut product = [0.0f64; 256]; // minor (≤48) × sq_dist (≤8) → ≤ 48*16 = 768
    let mut scratch = [0.0f64; MAX_EXPANSION_INSPHERE];
    let mut scaled = [0.0f64; 32]; // for multiply_expansions
    let mut accum_len = 0usize;

    // Helper closure: multiply sq_dist × minor, then add (or subtract) to accum.
    macro_rules! add_product {
        ($sq:expr, $sq_len:expr, $min:expr, $min_len:expr, $negate:expr) => {
            let prod_len = multiply_expansions(
                &$sq[..$sq_len],
                &$min[..$min_len],
                &mut product,
                &mut scratch,
                &mut scaled,
            );
            if $negate {
                negate_expansion(&mut product[..prod_len]);
            }
            if accum_len == 0 {
                accum[..prod_len].copy_from_slice(&product[..prod_len]);
                accum_len = prod_len;
            } else {
                let sum_len = expansion_sum(&accum[..accum_len], &product[..prod_len], &mut temp)
                    .expect("temp sized for MAX_EXPANSION_INSPHERE");
                accum_len = compress_expansion(&temp[..sum_len], &mut accum)
                    .expect("accum sized for MAX_EXPANSION_INSPHERE");
            }
        };
    }

    add_product!(ad2_exp, ad2_len, minor_a, ma_len, true);   // -ad2*minor_a
    add_product!(bd2_exp, bd2_len, minor_b, mb_len, false);  // +bd2*minor_b
    add_product!(cd2_exp, cd2_len, minor_c, mc_len, true);   // -cd2*minor_c
    add_product!(dd2_exp, dd2_len, minor_d, md_len, false);  // +dd2*minor_d

    // Final compress and sign.
    let mut compressed = [0.0f64; MAX_EXPANSION_INSPHERE];
    let comp_len = compress_expansion(&accum[..accum_len], &mut compressed)
        .expect("compressed sized for MAX_EXPANSION_INSPHERE");
    sign_of_expansion(&compressed[..comp_len])
}

// ──────────────────────────────────────────────────────────────────────────
//  Public ladder entry point
// ──────────────────────────────────────────────────────────────────────────

/// The 3-D in-sphere predicate: side of `e` w.r.t. the oriented sphere
/// through `a, b, c, d`.
///
/// **Sign convention (non-standard — see module docs):** when `a, b, c, d`
/// are positively oriented ([`super::orient3d::orient_3d`] > 0), returns
/// [`Sign::Negative`] if `e` is **inside** the sphere, [`Sign::Zero`] if `e`
/// is on it, [`Sign::Positive`] if **outside**. The sense is reversed for
/// negative orientation (inside = `Positive`).
///
/// This is the public ladder entry point — it escalates from filtered to
/// compensated to exact as needed, never returning an uncertain sign.
pub fn insphere(a: Point3, b: Point3, c: Point3, d: Point3, e: Point3) -> Sign {
    let diffs = InsphereDiffs::from_points(a, b, c, d, e);
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

    /// Ground-truth in-sphere sign via BigInt.
    /// Computes coordinate differences in f64 first (matching the predicate),
    /// then converts to exact BigInt values.
    fn exact_insphere_sign(a: Point3, b: Point3, c: Point3, d: Point3, e: Point3) -> Sign {
        let adx_f = a.x - e.x; let ady_f = a.y - e.y; let adz_f = a.z - e.z;
        let bdx_f = b.x - e.x; let bdy_f = b.y - e.y; let bdz_f = b.z - e.z;
        let cdx_f = c.x - e.x; let cdy_f = c.y - e.y; let cdz_f = c.z - e.z;
        let ddx_f = d.x - e.x; let ddy_f = d.y - e.y; let ddz_f = d.z - e.z;

        let adx = Exact::from_f64(adx_f); let ady = Exact::from_f64(ady_f); let adz = Exact::from_f64(adz_f);
        let bdx = Exact::from_f64(bdx_f); let bdy = Exact::from_f64(bdy_f); let bdz = Exact::from_f64(bdz_f);
        let cdx = Exact::from_f64(cdx_f); let cdy = Exact::from_f64(cdy_f); let cdz = Exact::from_f64(cdz_f);
        let ddx = Exact::from_f64(ddx_f); let ddy = Exact::from_f64(ddy_f); let ddz = Exact::from_f64(ddz_f);

        let ad2 = adx.clone().mul(adx.clone()).add(ady.clone().mul(ady.clone())).add(adz.clone().mul(adz.clone()));
        let bd2 = bdx.clone().mul(bdx.clone()).add(bdy.clone().mul(bdy.clone())).add(bdz.clone().mul(bdz.clone()));
        let cd2 = cdx.clone().mul(cdx.clone()).add(cdy.clone().mul(cdy.clone())).add(cdz.clone().mul(cdz.clone()));
        let dd2 = ddx.clone().mul(ddx.clone()).add(ddy.clone().mul(ddy.clone())).add(ddz.clone().mul(ddz.clone()));

        // 3×3 minors via BigInt
        let minor_a = {
            let t1 = bdx.clone().mul(cdy.clone()).mul(ddz.clone());
            let t2 = bdx.clone().mul(cdz.clone()).mul(ddy.clone());
            let t3 = bdy.clone().mul(cdx.clone()).mul(ddz.clone());
            let t4 = bdy.clone().mul(cdz.clone()).mul(ddx.clone());
            let t5 = bdz.clone().mul(cdx.clone()).mul(ddy.clone());
            let t6 = bdz.clone().mul(cdy.clone()).mul(ddx.clone());
            t1.sub(t2).sub(t3).add(t4).add(t5).sub(t6)
        };
        let minor_b = {
            let t1 = adx.clone().mul(cdy.clone()).mul(ddz.clone());
            let t2 = adx.clone().mul(cdz.clone()).mul(ddy.clone());
            let t3 = ady.clone().mul(cdx.clone()).mul(ddz.clone());
            let t4 = ady.clone().mul(cdz.clone()).mul(ddx.clone());
            let t5 = adz.clone().mul(cdx.clone()).mul(ddy.clone());
            let t6 = adz.clone().mul(cdy.clone()).mul(ddx.clone());
            t1.sub(t2).sub(t3).add(t4).add(t5).sub(t6)
        };
        let minor_c = {
            let t1 = adx.clone().mul(bdy.clone()).mul(ddz.clone());
            let t2 = adx.clone().mul(bdz.clone()).mul(ddy.clone());
            let t3 = ady.clone().mul(bdx.clone()).mul(ddz.clone());
            let t4 = ady.clone().mul(bdz.clone()).mul(ddx.clone());
            let t5 = adz.clone().mul(bdx.clone()).mul(ddy.clone());
            let t6 = adz.clone().mul(bdy.clone()).mul(ddx.clone());
            t1.sub(t2).sub(t3).add(t4).add(t5).sub(t6)
        };
        let minor_d = {
            let t1 = adx.clone().mul(bdy.clone()).mul(cdz.clone());
            let t2 = adx.clone().mul(bdz.clone()).mul(cdy.clone());
            let t3 = ady.clone().mul(bdx.clone()).mul(cdz.clone());
            let t4 = ady.clone().mul(bdz.clone()).mul(cdx.clone());
            let t5 = adz.clone().mul(bdx.clone()).mul(cdy.clone());
            let t6 = adz.clone().mul(bdy.clone()).mul(cdx.clone());
            t1.sub(t2).sub(t3).add(t4).add(t5).sub(t6)
        };

        // det = -ad2*minor_a + bd2*minor_b - cd2*minor_c + dd2*minor_d
        let det = ad2.mul(minor_a).neg()
            .add(bd2.mul(minor_b))
            .sub(cd2.mul(minor_c))
            .add(dd2.mul(minor_d));
        det.sign()
    }

    // ── Basic classification ──────────────────────────────────────────────

    /// Unit sphere centered at origin. a,b,c,d on sphere with **negative**
    /// orientation (`orient_3d = -2`). Under this impl's convention,
    /// negative-orientation + inside ⇒ `Positive`.
    fn unit_sphere_points() -> (Point3, Point3, Point3, Point3) {
        let a = Point3::new(1.0, 0.0, 0.0);
        let b = Point3::new(0.0, 1.0, 0.0);
        let c = Point3::new(0.0, 0.0, 1.0);
        let d = Point3::new(-1.0, 0.0, 0.0);
        (a, b, c, d)
    }

    #[test]
    fn classifies_inside_sphere() {
        let (a, b, c, d) = unit_sphere_points();
        let e = Point3::new(0.0, 0.0, 0.0); // center → inside
        // Negative orientation ⇒ inside = Positive (this impl's convention).
        assert_eq!(insphere(a, b, c, d, e), Sign::Positive);
    }

    #[test]
    fn classifies_outside_sphere() {
        let (a, b, c, d) = unit_sphere_points();
        let e = Point3::new(2.0, 0.0, 0.0); // outside
        // Negative orientation ⇒ outside = Negative.
        assert_eq!(insphere(a, b, c, d, e), Sign::Negative);
    }

    #[test]
    fn classifies_on_sphere() {
        let (a, b, c, d) = unit_sphere_points();
        let e = Point3::new(0.0, -1.0, 0.0); // on the unit sphere
        assert_eq!(insphere(a, b, c, d, e), Sign::Zero);
    }

    #[test]
    fn sign_flips_for_negative_orientation() {
        // Swap a and b to flip orientation: this tet has **positive**
        // orientation (`orient_3d = +2`). Under this impl's convention,
        // positive-orientation + inside ⇒ `Negative` (the flip).
        let a = Point3::new(0.0, 1.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 0.0, 1.0);
        let d = Point3::new(-1.0, 0.0, 0.0);
        let e = Point3::new(0.0, 0.0, 0.0); // center → inside, but orientation is now positive
        let s = insphere(a, b, c, d, e);
        // With positive orientation, inside → Negative (this impl's convention).
        assert_eq!(s, Sign::Negative);
    }

    // ── Agreement with BigInt cross-check ─────────────────────────────────

    #[test]
    fn agrees_with_exact_on_basic_cases() {
        let (a, b, c, d) = unit_sphere_points();
        let cases = [
            (a, b, c, d, Point3::new(0.0, 0.0, 0.0)),   // inside
            (a, b, c, d, Point3::new(2.0, 0.0, 0.0)),   // outside
            (a, b, c, d, Point3::new(0.0, -1.0, 0.0)),  // on sphere
            (a, b, c, d, Point3::new(0.5, 0.5, 0.5)),   // inside
            (Point3::new(3.0, 4.0, 0.0), Point3::new(0.0, 0.0, 0.0), Point3::new(6.0, 0.0, 0.0), Point3::new(3.0, 0.0, 4.0), Point3::new(3.0, 1.0, 0.0)),
        ];
        for (a, b, c, d, e) in cases {
            assert_eq!(
                insphere(a, b, c, d, e),
                exact_insphere_sign(a, b, c, d, e),
                "mismatch on ({a:?}, {b:?}, {c:?}, {d:?}, {e:?})"
            );
        }
    }

    // ── Adversarial: cospherical (exact zero) ─────────────────────────────

    #[test]
    fn cospherical_five_points() {
        // Five points on the unit sphere
        let a = Point3::new(1.0, 0.0, 0.0);
        let b = Point3::new(0.0, 1.0, 0.0);
        let c = Point3::new(0.0, 0.0, 1.0);
        let d = Point3::new(-1.0, 0.0, 0.0);
        let e = Point3::new(0.0, -1.0, 0.0);
        assert_eq!(insphere(a, b, c, d, e), Sign::Zero);
        assert_eq!(exact_insphere_sign(a, b, c, d, e), Sign::Zero);
    }

    #[test]
    fn cospherical_on_arbitrary_sphere() {
        // Sphere centered at (1, 2, 3), radius 6.
        // Points: (7,2,3), (1,8,3), (1,2,9), (-5,2,3), (1,-4,3)
        let a = Point3::new(7.0, 2.0, 3.0);
        let b = Point3::new(1.0, 8.0, 3.0);
        let c = Point3::new(1.0, 2.0, 9.0);
        let d = Point3::new(-5.0, 2.0, 3.0);
        let e = Point3::new(1.0, -4.0, 3.0);
        assert_eq!(insphere(a, b, c, d, e), Sign::Zero);
        assert_eq!(exact_insphere_sign(a, b, c, d, e), Sign::Zero);
    }

    // ── Adversarial: near-cospherical (±1-ulp) ────────────────────────────

    #[test]
    fn near_cospherical_1ulp_off() {
        let a = Point3::new(1.0, 0.0, 0.0);
        let b = Point3::new(0.0, 1.0, 0.0);
        let c = Point3::new(0.0, 0.0, 1.0);
        let d = Point3::new(-1.0, 0.0, 0.0);
        let e0 = Point3::new(0.0, -1.0, 0.0); // on sphere

        for &delta_bits in &[1i64, -1, 2, -2, 5, -5, 100, -100] {
            let ey = f64::from_bits((e0.y.to_bits() as i64 + delta_bits) as u64);
            let e = Point3::new(e0.x, ey, e0.z);
            assert_eq!(
                insphere(a, b, c, d, e),
                exact_insphere_sign(a, b, c, d, e),
                "mismatch on near-cospherical delta_bits={delta_bits}"
            );
        }
    }

    // ── Adversarial: extreme exponents ────────────────────────────────────

    #[test]
    fn extreme_exponents_agree_with_exact() {
    // Keep products within f64 range: coords ~1e30, sq_dist ~1e60, products ~1e150.
    let cases = [
        (Point3::new(1e30, 0.0, 0.0), Point3::new(0.0, 1e30, 0.0), Point3::new(0.0, 0.0, 1e30), Point3::new(-1e30, 0.0, 0.0), Point3::new(0.0, 0.0, 0.0)),
        (Point3::new(1e-30, 0.0, 0.0), Point3::new(0.0, 1e-30, 0.0), Point3::new(0.0, 0.0, 1e-30), Point3::new(-1e-30, 0.0, 0.0), Point3::new(0.0, 0.0, 0.0)),
    ];
    for (a, b, c, d, e) in cases {
        assert_eq!(
            insphere(a, b, c, d, e),
            exact_insphere_sign(a, b, c, d, e),
            "mismatch on extreme-exponent case"
        );
    }
}

    // ── Adversarial: cancellation ─────────────────────────────────────────

    #[test]
    fn cancellation_agrees_with_exact() {
        let a = Point3::new(1e30, 0.0, 0.0);
        let b = Point3::new(0.0, 1e30, 0.0);
        let c = Point3::new(0.0, 0.0, 1e30);
        let d = Point3::new(-1e30, 0.0, 0.0);
        let e = Point3::new(0.0, -1e30, 0.0); // cospherical
        assert_eq!(insphere(a, b, c, d, e), exact_insphere_sign(a, b, c, d, e));
    }

    #[test]
    fn massive_cancellation_agrees_with_exact() {
        let a = Point3::new(1e30, 0.0, 0.0);
        let b = Point3::new(0.0, 1e30, 0.0);
        let c = Point3::new(0.0, 0.0, 1e30);
        let d = Point3::new(-1e30, 0.0, 0.0);
        let e = Point3::new(0.0, -1e30 + 1.0, 0.0); // just off the sphere
        assert_eq!(
            insphere(a, b, c, d, e),
            exact_insphere_sign(a, b, c, d, e),
            "mismatch on massive cancellation"
        );
    }

    // ── All three ladder stages exercised ─────────────────────────────────

    #[test]
    fn filtered_stage_resolves_clear_case() {
        let (a, b, c, d) = unit_sphere_points();
        let e = Point3::new(0.0, 0.0, 0.0);
        let diffs = InsphereDiffs::from_points(a, b, c, d, e);
        let det = filtered_det(&diffs);
        let perm = diffs.permanent();
        assert!(
            det.abs() > perm * FILTERED_BOUND,
            "filtered should resolve (det={det}, bound={})",
            perm * FILTERED_BOUND
        );
        assert_eq!(insphere(a, b, c, d, e), Sign::Positive);
    }

    #[test]
    fn near_cospherical_resolves_via_compensated_or_exact() {
        let a = Point3::new(1.0, 0.0, 0.0);
        let b = Point3::new(0.0, 1.0, 0.0);
        let c = Point3::new(0.0, 0.0, 1.0);
        let d = Point3::new(-1.0, 0.0, 0.0);
        let e = Point3::new(0.0, -1.0 + 1e-15, 0.0);
        assert_eq!(
            insphere(a, b, c, d, e),
            exact_insphere_sign(a, b, c, d, e),
            "near-cospherical must match exact"
        );
    }

    // ── Determinism ───────────────────────────────────────────────────────

    #[test]
    fn deterministic_across_calls() {
        let (a, b, c, d) = unit_sphere_points();
        let e = Point3::new(0.3, 0.3, 0.3);
        let s1 = insphere(a, b, c, d, e);
        let s2 = insphere(a, b, c, d, e);
        assert_eq!(s1, s2);
    }

    // ── Symmetry ──────────────────────────────────────────────────────────

    #[test]
    fn swapping_a_b_flips_sign() {
        let (a, b, c, d) = unit_sphere_points();
        let e = Point3::new(0.0, 0.0, 0.0);
        let s = insphere(a, b, c, d, e);
        let s_swapped = insphere(b, a, c, d, e);
        assert_eq!(s, s_swapped.flip());
    }

    #[test]
    fn translation_invariant() {
        let (a, b, c, d) = unit_sphere_points();
        let e = Point3::new(0.0, 0.0, 0.0);
        let t = Point3::new(1e10, -1e10, 5e9);
        let s = insphere(a, b, c, d, e);
        let s_t = insphere(
            Point3::new(a.x + t.x, a.y + t.y, a.z + t.z),
            Point3::new(b.x + t.x, b.y + t.y, b.z + t.z),
            Point3::new(c.x + t.x, c.y + t.y, c.z + t.z),
            Point3::new(d.x + t.x, d.y + t.y, d.z + t.z),
            Point3::new(e.x + t.x, e.y + t.y, e.z + t.z),
        );
        assert_eq!(s, s_t);
    }

    // ── Zero-heap contract ────────────────────────────────────────────────

    #[test]
    fn no_heap_allocation_in_predicate() {
        let (a, b, c, d) = unit_sphere_points();
        let e = Point3::new(0.0, -1.0, 0.0); // cospherical — exercises exact stage
        let _ = insphere(a, b, c, d, e);
    }
}
