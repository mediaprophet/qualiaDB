//! Exact 3-D construction — segment/plane and segment/triangle intersection (P5.4).
//!
//! This extends the P1.7 exact-construction idea ([`super::exact_kernel`]'s
//! `ExactPoint2` / `construct_segment_intersection`) into three dimensions.
//!
//! The filtered `f64` predicates ([`super::kernel::FilteredF64Kernel`]) never
//! return a wrong sign on *given* input. But a geometric *construction* — the
//! point where a segment pierces a plane — produces a **new** coordinate that,
//! if rounded to `f64`, corrupts every subsequent predicate. A downstream
//! `orient_3d` on the rounded intersection point can then mis-sign even though
//! `orient_3d` itself is exact, because its *input* was already wrong.
//!
//! This module solves that by carrying the constructed point as an **exact
//! rational**: three numerator expansions and one shared denominator expansion,
//! all zero-heap (stack `[f64; N]` arrays). A predicate on such a point
//! ([`orient_3d_exact_3`]) cross-multiplies by the denominators to eliminate
//! the division, so the whole evaluation stays in exact expansion arithmetic —
//! no rounding, ever.
//!
//! ## The construction
//!
//! Segment `(p, q)`; plane through `(a, b, c)` with normal
//! `n = (b − a) × (c − a)`. The plane is `{ x : n · (x − a) = 0 }`. The line is
//! `x(t) = p + t·(q − p)`. Substituting and solving:
//!
//! ```text
//! t = n · (p − a) / n · (p − q)
//! den = n · (p − q)        (zero ⇔ segment parallel to plane)
//! t_num = n · (p − a)
//! x_i = (p_i · den − t_num · (q_i − p_i)) / den   for i ∈ {x, y, z}
//! ```
//!
//! (using `x(t) = p + (t_num/den)·(q − p)` with `t = t_num/den`, and
//! `t·(q_i − p_i) = t_num·(q_i − p_i)/den`.)
//!
//! We keep `x_num_i = p_i · den + t_num · (q_i − p_i)` and the shared `den`
//! as separate expansions. The denominator is normalized positive.
//!
//! ## The predicate
//!
//! `orient_3d(A, B, C, P)` where `P` is exact-rational is
//! `sign(det(B − A, C − A, P − A))`. With `P = (x_num/den, y_num/den, z_num/den)`
//! and `den > 0`, every occurrence of `P − A` is
//! `(x_num − A·den)/den`. The determinant is homogeneous of degree 1 in the
//! third row, so multiplying that row by `den > 0` does not change the sign:
//!
//! ```text
//! sign(orient_3d(A,B,C,P)) = sign(det( B−A, C−A, (num − A·den) ))
//! ```
//!
//! where `num − A·den = (x_num − A.x·den, y_num − A.y·den, z_num − A.z·den)` is
//! computed as exact expansions. That 3×3 determinant is then evaluated exactly.
//!
//! ## Zero-heap contract
//!
//! [`ExactPoint3`] is `[f64; N]` stack arrays. No `Vec`/`String`/`Box` in any
//! construction or predicate path. The determinant workspace is a fixed stack
//! array sized by the worst-case expansion length (documented at
//! [`MAX_DET3`]).
//!
//! ## References
//!
//! The cascaded-construction problem motivates CGAL's `Cartesian<Exact_kernel>`
//! and the lazy-exact approach (Pion & Fabri, 2009); the expansion arithmetic is
//! Shewchuk (1996). This is an original zero-heap Rust analogue over the P1.3
//! [`super::expansion`] primitives — no CGAL or other third-party source is used.

use super::expansion::{
    compress_expansion, expansion_sum, negate_expansion, scale_expansion, sign_of_expansion,
    two_product, Sign,
};
use super::primitives::Point3;

// ──────────────────────────────────────────────────────────────────────────
//  Workspace size constants
// ──────────────────────────────────────────────────────────────────────────
//
// We derive the worst-case (uncompressed) expansion lengths so every stack
// buffer is provably large enough — the expansion ops fail-closed on a small
// buffer, so an under-sized bound would surface as a panic in tests, never a
// silent truncation. In practice compression keeps the real lengths far below
// these bounds; the constants are conservative ceilings.

/// A 2×2 determinant of coordinate differences (a normal-vector component
/// `n = (b−a) × (c−a)`): difference of two products of two length-2
/// expansions. `two_product`-then-`scale` of two diffs gives ≤ 8; the
/// difference of two such is ≤ 16 uncompressed. We size at 16.
const MAX_NORMAL: usize = 16;

/// The denominator `den = n · (p − q)` and numerator terms `t_num = n · (p − a)`:
/// a dot product of the length-≤`MAX_NORMAL` normal with an f64 difference,
/// summed over 3 axes. Each `scale_expansion(normal, diff)` is ≤ `2*MAX_NORMAL`;
/// three summed is ≤ `6*MAX_NORMAL`. Size at 128 (> 96) for headroom.
const MAX_DEN: usize = 128;

/// A constructed numerator coordinate `x_num = p_i·den + t_num·(q_i − p_i)`.
/// `p_i·den` is `scale(den)` ≤ `2*MAX_DEN`; `t_num·(q_i−p_i)` likewise.
/// The sum is ≤ `4*MAX_DEN`. Size at 512.
const MAX_NUMER: usize = 512;

/// The exact 3×3 orientation determinant on a constructed point.
///
/// Tight worst-case component derivation (all uncompressed upper bounds):
/// - normal component `n_i` (2×2 det of f64 diffs): ≤ 4
/// - `den`, `t_num` (3-term dot of `n` with f64 diffs): ≤ 3·(2·4) = 24
/// - `x_num` (`p_i·den + t_num·dq_i`, two scales + sum): ≤ 2·(2·24) = 96
/// - row-3 entry `r3 = num − A·den`: ≤ 96 + 2·24 = 144
/// - cofactor `m` (2×2 det of f64 rows): ≤ 4
/// - term `r3·m` (scale-and-sum, `len(r3)` products of len `2·m`): ≤ 144·8 = 1152
/// - `det = termx − termy + termz`: ≤ 3·1152 = 3456
///
/// 4096 (> 3456) with power-of-two headroom. At 8 bytes/f64 the determinant
/// buffers dominate the stack frame (~360 KB peak in [`orient_3d_exact_3`]),
/// well within any thread stack and the 42 MB Sentinel arena ceiling. The
/// expansion ops fail closed on an undersized buffer, so if any bound above
/// were wrong a test would panic — never silently truncate.
const MAX_DET3: usize = 4096;

// ──────────────────────────────────────────────────────────────────────────
//  Error type
// ──────────────────────────────────────────────────────────────────────────

/// Errors from exact 3-D construction. All fail-closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exact3Error {
    /// A supplied coordinate is not finite (NaN or ±∞). Exact construction is
    /// only defined over finite reals.
    NonFiniteCoordinate {
        /// Which point (0 = p, 1 = q, 2 = a, 3 = b, 4 = c).
        point: usize,
    },
    /// The plane is degenerate: `(b − a)` and `(c − a)` are parallel (or a
    /// point coincides), so `a, b, c` do not define a plane. The normal is the
    /// zero vector.
    DegeneratePlane,
    /// The segment is parallel to the plane (`den = n · (p − q) = 0`), so there
    /// is no unique intersection point (the segment either misses the plane or
    /// lies entirely within it).
    ParallelToPlane,
}

impl core::fmt::Display for Exact3Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Exact3Error::NonFiniteCoordinate { point } => {
                write!(f, "non-finite coordinate in point {point}")
            }
            Exact3Error::DegeneratePlane => write!(f, "plane a,b,c is degenerate (zero normal)"),
            Exact3Error::ParallelToPlane => write!(f, "segment is parallel to the plane"),
        }
    }
}

impl std::error::Error for Exact3Error {}

// ──────────────────────────────────────────────────────────────────────────
//  ExactPoint3 — a rational point (num/den per axis, shared denominator)
// ──────────────────────────────────────────────────────────────────────────

/// An exact point in 3-D, stored as a rational vector with a shared
/// denominator: `x = x_num/den`, `y = y_num/den`, `z = z_num/den`, `den > 0`.
///
/// Every field is a stack-allocated expansion (zero-heap). The denominator is
/// normalized to be strictly positive by construction, which makes sign
/// reasoning in predicates a matter of cross-multiplication with no case split.
#[derive(Debug, Clone)]
pub struct ExactPoint3 {
    /// Numerator of the x-coordinate.
    pub x_num: [f64; MAX_NUMER],
    pub x_num_len: usize,
    /// Numerator of the y-coordinate.
    pub y_num: [f64; MAX_NUMER],
    pub y_num_len: usize,
    /// Numerator of the z-coordinate.
    pub z_num: [f64; MAX_NUMER],
    pub z_num_len: usize,
    /// Shared denominator. Always strictly positive after construction.
    pub den: [f64; MAX_DEN],
    pub den_len: usize,
}

impl ExactPoint3 {
    /// Create an exact point from a plain `f64` point (denominator = 1).
    pub fn from_point3(p: Point3) -> Self {
        let mut ep = ExactPoint3 {
            x_num: [0.0; MAX_NUMER],
            x_num_len: 1,
            y_num: [0.0; MAX_NUMER],
            y_num_len: 1,
            z_num: [0.0; MAX_NUMER],
            z_num_len: 1,
            den: [0.0; MAX_DEN],
            den_len: 1,
        };
        ep.x_num[0] = p.x;
        ep.y_num[0] = p.y;
        ep.z_num[0] = p.z;
        ep.den[0] = 1.0;
        ep
    }

    /// Convert to a rounded [`Point3`] (for comparison with the filtered path).
    /// This *does* round — use [`orient_3d_exact_3`] for exact predicates.
    pub fn to_point3(&self) -> Point3 {
        let den = expansion_value(&self.den[..self.den_len]);
        Point3::new(
            expansion_value(&self.x_num[..self.x_num_len]) / den,
            expansion_value(&self.y_num[..self.y_num_len]) / den,
            expansion_value(&self.z_num[..self.z_num_len]) / den,
        )
    }
}

/// Sum of an expansion's components (for converting to a single `f64`).
#[inline]
fn expansion_value(e: &[f64]) -> f64 {
    e.iter().sum()
}

// ──────────────────────────────────────────────────────────────────────────
//  Small expansion helpers (compressed sums, fixed buffers)
// ──────────────────────────────────────────────────────────────────────────

/// `out = a + b`, compressed. Returns the length. Panics only if the caller's
/// buffers are undersized, which is a programming error (all internal callers
/// size their buffers from the `MAX_*` constants above).
fn add_compressed(a: &[f64], b: &[f64], scratch: &mut [f64], out: &mut [f64]) -> usize {
    let sum_len = expansion_sum(a, b, scratch).expect("scratch sized for a.len()+b.len()");
    compress_expansion(&scratch[..sum_len], out).expect("out sized for sum length")
}

/// `out = scalar * e`, compressed. Returns the length.
fn scale_compressed(e: &[f64], scalar: f64, scratch: &mut [f64], out: &mut [f64]) -> usize {
    let n = scale_expansion(e, scalar, scratch).expect("scratch sized for 2*e.len()");
    compress_expansion(&scratch[..n], out).expect("out sized for 2*e.len()")
}

// ──────────────────────────────────────────────────────────────────────────
//  Exact 2×2 determinant of coordinate differences (normal-vector component)
// ──────────────────────────────────────────────────────────────────────────

/// Compute `u1*v2 − u2*v1` exactly, where each of `u1,u2,v1,v2` is an f64
/// coordinate difference. Writes the compressed expansion into `out` and
/// returns its length. `out` must have length ≥ [`MAX_NORMAL`].
fn exact_cross_component(u1: f64, u2: f64, v1: f64, v2: f64, out: &mut [f64]) -> usize {
    // u1*v2  (length 2)
    let (p1, e1) = two_product(u1, v2);
    // u2*v1  (length 2)
    let (p2, e2) = two_product(u2, v1);
    // det = (p1 + e1) - (p2 + e2). Build as an expansion: [e1, p1] + [-e2, -p2].
    let a = [e1, p1];
    let b = [-e2, -p2];
    let mut scratch = [0.0f64; MAX_NORMAL];
    add_compressed(&a, &b, &mut scratch, out)
}

// ──────────────────────────────────────────────────────────────────────────
//  Normal vector n = (b − a) × (c − a) as three exact expansions
// ──────────────────────────────────────────────────────────────────────────

/// The exact plane normal `n = (b − a) × (c − a)`, each component a compressed
/// expansion of length ≤ [`MAX_NORMAL`].
struct ExactNormal {
    nx: [f64; MAX_NORMAL],
    nx_len: usize,
    ny: [f64; MAX_NORMAL],
    ny_len: usize,
    nz: [f64; MAX_NORMAL],
    nz_len: usize,
}

impl ExactNormal {
    fn from_plane(a: Point3, b: Point3, c: Point3) -> Self {
        // Coordinate differences (each error-free is length 2, but the products
        // in the cross terms use the *rounded* f64 difference; the cross uses
        // two_product on those f64 differences to recover their product exactly,
        // which is what exact_cross_component does). We pass the f64 differences.
        let ux = b.x - a.x;
        let uy = b.y - a.y;
        let uz = b.z - a.z;
        let vx = c.x - a.x;
        let vy = c.y - a.y;
        let vz = c.z - a.z;

        // n = u × v = (uy*vz − uz*vy, uz*vx − ux*vz, ux*vy − uy*vx)
        let mut nx = [0.0f64; MAX_NORMAL];
        let mut ny = [0.0f64; MAX_NORMAL];
        let mut nz = [0.0f64; MAX_NORMAL];
        let nx_len = exact_cross_component(uy, uz, vy, vz, &mut nx);
        let ny_len = exact_cross_component(uz, ux, vz, vx, &mut ny);
        let nz_len = exact_cross_component(ux, uy, vx, vy, &mut nz);
        ExactNormal { nx, nx_len, ny, ny_len, nz, nz_len }
    }

    /// True iff the normal is the zero vector (degenerate plane).
    fn is_zero(&self) -> bool {
        sign_of_expansion(&self.nx[..self.nx_len]) == Sign::Zero
            && sign_of_expansion(&self.ny[..self.ny_len]) == Sign::Zero
            && sign_of_expansion(&self.nz[..self.nz_len]) == Sign::Zero
    }

    /// Exact dot product `n · (wx, wy, wz)` where `w` is an f64 vector.
    /// Writes the compressed result into `out` (≥ [`MAX_DEN`]); returns length.
    fn dot_f64(&self, wx: f64, wy: f64, wz: f64, out: &mut [f64]) -> usize {
        // term_i = n_i * w_i  (scale expansion by scalar), then sum the three.
        let mut tx = [0.0f64; MAX_DEN];
        let mut ty = [0.0f64; MAX_DEN];
        let mut tz = [0.0f64; MAX_DEN];
        let mut scratch = [0.0f64; MAX_DEN];
        let tx_len = scale_compressed(&self.nx[..self.nx_len], wx, &mut scratch, &mut tx);
        let ty_len = scale_compressed(&self.ny[..self.ny_len], wy, &mut scratch, &mut ty);
        let tz_len = scale_compressed(&self.nz[..self.nz_len], wz, &mut scratch, &mut tz);

        // sum = tx + ty + tz
        let mut partial = [0.0f64; MAX_DEN];
        let mut sum_scratch = [0.0f64; MAX_DEN];
        let partial_len =
            add_compressed(&tx[..tx_len], &ty[..ty_len], &mut sum_scratch, &mut partial);
        add_compressed(&partial[..partial_len], &tz[..tz_len], &mut sum_scratch, out)
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  Construct segment/plane intersection
// ──────────────────────────────────────────────────────────────────────────

#[inline]
fn all_finite(p: Point3) -> bool {
    p.x.is_finite() && p.y.is_finite() && p.z.is_finite()
}

/// Construct the exact intersection point of segment `(p, q)` with the plane
/// through `(a, b, c)`.
///
/// The plane normal is `n = (b − a) × (c − a)`; the intersection parameter is
/// `t = n·(p − a) / n·(p − q)`, and the point is `p + t·(q − p)`, kept as an
/// exact rational vector ([`ExactPoint3`]).
///
/// The construction treats the plane as the *infinite* plane through `a, b, c`
/// and the segment as the *infinite* line through `p, q` — it returns the
/// unique line/plane crossing. (Segment-within-bounds and triangle-within-bounds
/// membership tests are separate; see [`segment_plane_parameter_sign`] and
/// [`orient_3d_exact_3`], which let a caller decide containment without
/// rounding.)
///
/// # Errors
/// - [`Exact3Error::NonFiniteCoordinate`] if any coordinate is not finite.
/// - [`Exact3Error::DegeneratePlane`] if `a, b, c` are collinear/coincident.
/// - [`Exact3Error::ParallelToPlane`] if the segment direction is perpendicular
///   to the normal (`den = 0`) — no unique crossing.
pub fn construct_segment_plane_intersection_3(
    p: Point3,
    q: Point3,
    a: Point3,
    b: Point3,
    c: Point3,
) -> Result<ExactPoint3, Exact3Error> {
    for (idx, pt) in [p, q, a, b, c].iter().enumerate() {
        if !all_finite(*pt) {
            return Err(Exact3Error::NonFiniteCoordinate { point: idx });
        }
    }

    let normal = ExactNormal::from_plane(a, b, c);
    if normal.is_zero() {
        return Err(Exact3Error::DegeneratePlane);
    }

    // den = n · (p − q)
    let mut den = [0.0f64; MAX_DEN];
    let den_len = normal.dot_f64(p.x - q.x, p.y - q.y, p.z - q.z, &mut den);
    let den_sign = sign_of_expansion(&den[..den_len]);
    if den_sign == Sign::Zero {
        return Err(Exact3Error::ParallelToPlane);
    }

    // t_num = n · (p − a)
    let mut t_num = [0.0f64; MAX_DEN];
    let t_num_len = normal.dot_f64(p.x - a.x, p.y - a.y, p.z - a.z, &mut t_num);

    // For each axis i: x_num_i = p_i·den + t_num·(q_i − p_i)
    // (since point = p + (t_num/den)·(q − p)).
    let mut x_num = [0.0f64; MAX_NUMER];
    let mut y_num = [0.0f64; MAX_NUMER];
    let mut z_num = [0.0f64; MAX_NUMER];
    let x_num_len = construct_axis_numerator(
        p.x,
        q.x - p.x,
        &den[..den_len],
        &t_num[..t_num_len],
        &mut x_num,
    );
    let y_num_len = construct_axis_numerator(
        p.y,
        q.y - p.y,
        &den[..den_len],
        &t_num[..t_num_len],
        &mut y_num,
    );
    let z_num_len = construct_axis_numerator(
        p.z,
        q.z - p.z,
        &den[..den_len],
        &t_num[..t_num_len],
        &mut z_num,
    );

    let mut den_out = [0.0f64; MAX_DEN];
    den_out[..den_len].copy_from_slice(&den[..den_len]);

    // Normalize the denominator to be strictly positive. Negating the numerators
    // and the denominator together leaves every ratio (and thus the point)
    // unchanged, but gives predicates a fixed `den > 0` sign to reason with.
    if den_sign == Sign::Negative {
        negate_expansion(&mut x_num[..x_num_len]);
        negate_expansion(&mut y_num[..y_num_len]);
        negate_expansion(&mut z_num[..z_num_len]);
        negate_expansion(&mut den_out[..den_len]);
    }
    debug_assert_eq!(sign_of_expansion(&den_out[..den_len]), Sign::Positive);

    Ok(ExactPoint3 {
        x_num,
        x_num_len,
        y_num,
        y_num_len,
        z_num,
        z_num_len,
        den: den_out,
        den_len,
    })
}

/// `out = p_i·den + t_num·(q_i − p_i)`, compressed. Returns the length.
fn construct_axis_numerator(
    p_i: f64,
    dq_i: f64, // q_i − p_i
    den: &[f64],
    t_num: &[f64],
    out: &mut [f64],
) -> usize {
    let mut scratch = [0.0f64; MAX_NUMER];
    // term1 = p_i · den
    let mut term1 = [0.0f64; MAX_NUMER];
    let t1_len = scale_compressed(den, p_i, &mut scratch, &mut term1);
    // term2 = t_num · (q_i − p_i)
    let mut term2 = [0.0f64; MAX_NUMER];
    let t2_len = scale_compressed(t_num, dq_i, &mut scratch, &mut term2);
    // out = term1 + term2
    add_compressed(&term1[..t1_len], &term2[..t2_len], &mut scratch, out)
}

// ──────────────────────────────────────────────────────────────────────────
//  Exact orient_3d on a constructed point
// ──────────────────────────────────────────────────────────────────────────

/// Exact 3-D orientation `sign(det(B − A, C − A, P − A))` where `P` is an
/// exact-rational [`ExactPoint3`] and `A, B, C` are plain `f64` points.
///
/// Because the determinant is linear in its third row and `P.den > 0`,
/// multiplying that row by `P.den` preserves the sign. The third row becomes
/// `num − A·den` (exact expansions), and the whole 3×3 determinant is then
/// evaluated in exact expansion arithmetic — no division, no rounding.
///
/// Returns [`Sign::Positive`] if `P` lies below the oriented plane `A → B → C`
/// (matching [`super::orient3d::orient_3d`]'s convention), [`Sign::Negative`]
/// if above, [`Sign::Zero`] if coplanar.
pub fn orient_3d_exact_3(a: Point3, b: Point3, c: Point3, p: &ExactPoint3) -> Sign {
    // Row 1: (b − a), Row 2: (c − a) — plain f64.
    let r1x = b.x - a.x;
    let r1y = b.y - a.y;
    let r1z = b.z - a.z;
    let r2x = c.x - a.x;
    let r2y = c.y - a.y;
    let r2z = c.z - a.z;

    // Row 3 (scaled by den): (x_num − a.x·den, y_num − a.y·den, z_num − a.z·den).
    let mut r3x = [0.0f64; MAX_NUMER];
    let mut r3y = [0.0f64; MAX_NUMER];
    let mut r3z = [0.0f64; MAX_NUMER];
    let r3x_len = scaled_axis_minus(&p.x_num[..p.x_num_len], a.x, &p.den[..p.den_len], &mut r3x);
    let r3y_len = scaled_axis_minus(&p.y_num[..p.y_num_len], a.y, &p.den[..p.den_len], &mut r3y);
    let r3z_len = scaled_axis_minus(&p.z_num[..p.z_num_len], a.z, &p.den[..p.den_len], &mut r3z);

    // det = det( [r1x r1y r1z]
    //            [r2x r2y r2z]
    //            [r3x r3y r3z] )  where row 3 is an expansion.
    //
    // Expand along row 3 (the exact row):
    //   det = r3x·(r1y·r2z − r1z·r2y)   [cofactor for column x]
    //       − r3y·(r1x·r2z − r1z·r2x)   [cofactor for column y]
    //       + r3z·(r1x·r2y − r1y·r2x)   [cofactor for column z]
    //
    // Each cofactor m_i is a 2×2 determinant of f64 rows (rows 1 and 2), which
    // we compute exactly as a length-≤`MAX_NORMAL` expansion. Then we scale the
    // corresponding row-3 expansion by that cofactor and sum.
    let mut mx = [0.0f64; MAX_NORMAL];
    let mut my = [0.0f64; MAX_NORMAL];
    let mut mz = [0.0f64; MAX_NORMAL];
    let mx_len = exact_cross_component(r1y, r1z, r2y, r2z, &mut mx); // r1y*r2z − r1z*r2y
    let my_len = exact_cross_component(r1x, r1z, r2x, r2z, &mut my); // r1x*r2z − r1z*r2x
    let mz_len = exact_cross_component(r1x, r1y, r2x, r2y, &mut mz); // r1x*r2y − r1y*r2x

    // Multiply two expansions (row-3 entry × cofactor) into a product expansion.
    // Both operands can be multi-component, so we use a scale-and-sum loop:
    // (sum_j r3[j]) * (sum_k m[k]) = sum_j r3[j]*m.
    let mut termx = [0.0f64; MAX_DET3];
    let mut termy = [0.0f64; MAX_DET3];
    let mut termz = [0.0f64; MAX_DET3];
    let termx_len = mul_expansions(&r3x[..r3x_len], &mx[..mx_len], &mut termx);
    let termy_len = mul_expansions(&r3y[..r3y_len], &my[..my_len], &mut termy);
    let termz_len = mul_expansions(&r3z[..r3z_len], &mz[..mz_len], &mut termz);

    // det = termx − termy + termz.
    negate_expansion(&mut termy[..termy_len]);
    let mut det = [0.0f64; MAX_DET3];
    let mut partial = [0.0f64; MAX_DET3];
    let mut scratch = [0.0f64; MAX_DET3];
    let partial_len =
        add_compressed(&termx[..termx_len], &termy[..termy_len], &mut scratch, &mut partial);
    let det_len = add_compressed(&partial[..partial_len], &termz[..termz_len], &mut scratch, &mut det);

    sign_of_expansion(&det[..det_len])
}

/// `out = num − scalar·den`, compressed. Returns the length. Used to form the
/// scaled row-3 entries `x_num − A.x·den`.
fn scaled_axis_minus(num: &[f64], scalar: f64, den: &[f64], out: &mut [f64]) -> usize {
    let mut scratch = [0.0f64; MAX_NUMER];
    // s_den = scalar · den
    let mut s_den = [0.0f64; MAX_NUMER];
    let s_den_len = scale_compressed(den, scalar, &mut scratch, &mut s_den);
    negate_expansion(&mut s_den[..s_den_len]); // −scalar·den
    add_compressed(num, &s_den[..s_den_len], &mut scratch, out)
}

/// Multiply two expansions exactly: `out = (Σ a) · (Σ b)`, compressed.
/// Returns the length. `out` must be ≥ [`MAX_DET3`]. Zero-heap.
fn mul_expansions(a: &[f64], b: &[f64], out: &mut [f64]) -> usize {
    if a.is_empty() || b.is_empty() {
        out[0] = 0.0;
        return 1;
    }
    // Accumulate Σ_i (a[i] · b) via scale_expansion of b by each a[i].
    let mut acc = [0.0f64; MAX_DET3];
    let mut acc_len = 0usize;
    let mut prod = [0.0f64; MAX_DET3];
    let mut prod_scratch = [0.0f64; MAX_DET3];
    let mut sum_scratch = [0.0f64; MAX_DET3];

    for &ai in a {
        // prod = ai · b
        let prod_len = scale_compressed(b, ai, &mut prod_scratch, &mut prod);
        if acc_len == 0 {
            acc[..prod_len].copy_from_slice(&prod[..prod_len]);
            acc_len = prod_len;
        } else {
            let mut next = [0.0f64; MAX_DET3];
            let next_len =
                add_compressed(&acc[..acc_len], &prod[..prod_len], &mut sum_scratch, &mut next);
            acc[..next_len].copy_from_slice(&next[..next_len]);
            acc_len = next_len;
        }
    }
    out[..acc_len].copy_from_slice(&acc[..acc_len]);
    acc_len
}

// ──────────────────────────────────────────────────────────────────────────
//  Segment/triangle intersection (bounded)
// ──────────────────────────────────────────────────────────────────────────

/// Whether a constructed line/plane crossing lies inside triangle `(a, b, c)`,
/// decided *exactly*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriangleContainment {
    /// The point is strictly inside the triangle.
    Inside,
    /// The point is exactly on the triangle boundary (an edge or vertex).
    OnBoundary,
    /// The point is outside the triangle.
    Outside,
}

/// Construct the exact intersection of segment `(p, q)` with the plane of
/// triangle `(a, b, c)`, and classify whether that point lies inside the
/// triangle — all without rounding.
///
/// The point-in-triangle test is done exactly: the point `P` is in-plane by
/// construction, so we test its orientation against each directed edge using a
/// consistent reference. We use three [`orient_3d_exact_3`] calls of the form
/// `orient_3d(edge_start, edge_end, apex_off_plane, P)` — but since `P` is in
/// the plane, we instead reduce to in-plane side tests via the plane normal.
///
/// Concretely: for edge `(a, b)`, the signed volume of `(a, b, a + n, P)` has
/// the sign of the 2-D cross product `(b − a) × (P − a)` projected along `n`.
/// We reuse [`orient_3d_exact_3`] with the third plane point offset along the
/// normal to realize this test exactly. The three edge signs must be
/// consistent (all ≥ 0 or all ≤ 0) for containment.
///
/// Returns the [`ExactPoint3`] and its [`TriangleContainment`].
///
/// # Errors
/// Same as [`construct_segment_plane_intersection_3`].
pub fn construct_segment_triangle_intersection_3(
    p: Point3,
    q: Point3,
    a: Point3,
    b: Point3,
    c: Point3,
) -> Result<(ExactPoint3, TriangleContainment), Exact3Error> {
    let point = construct_segment_plane_intersection_3(p, q, a, b, c)?;

    // In-plane side test for each edge. For edge (u, v) of a triangle with
    // third vertex w, the point is on the "inside" side iff orient_3d over
    // (u, v, w, P) has the SAME sign as orient_3d over (u, v, w, <triangle
    // interior>). Since w is the opposite vertex (a true triangle vertex, off
    // the edge but in-plane), orient_3d(u, v, w, P) is degenerate (all in one
    // plane) — so we lift the test out of the plane using an off-plane apex.
    //
    // We build an apex = a + n (a point off the plane along the normal). Then
    // orient_3d(u, v, apex, P) gives the side of the *plane through u, v, and
    // apex* — which is the plane containing edge (u,v) and perpendicular to the
    // triangle's plane. Its sign is exactly the in-plane side of edge (u,v).
    //
    // The reference "inside" sign for edge (u, v) is orient_3d(u, v, apex, w)
    // where w is the third triangle vertex (which is genuinely inside relative
    // to that edge). Containment ⇔ P's sign matches w's sign (or is zero) for
    // all three edges.

    // Off-plane apex: a + n, computed with a plain f64 normal (only its
    // direction matters; the exact sign test below uses orient_3d_exact_3 /
    // orient_3d, which are themselves exact).
    let ux = b.x - a.x;
    let uy = b.y - a.y;
    let uz = b.z - a.z;
    let vx = c.x - a.x;
    let vy = c.y - a.y;
    let vz = c.z - a.z;
    let nx = uy * vz - uz * vy;
    let ny = uz * vx - ux * vz;
    let nz = ux * vy - uy * vx;
    let apex = Point3::new(a.x + nx, a.y + ny, a.z + nz);

    let containment = classify_triangle_containment(a, b, c, apex, &point);
    Ok((point, containment))
}

/// Exact point-in-triangle classification for an in-plane [`ExactPoint3`].
/// `apex` is an off-plane reference point (`a + n`). Uses [`orient_3d_exact_3`]
/// for the exact `P` tests and [`super::orient3d::orient_3d`] for the f64
/// reference-vertex tests.
fn classify_triangle_containment(
    a: Point3,
    b: Point3,
    c: Point3,
    apex: Point3,
    point: &ExactPoint3,
) -> TriangleContainment {
    // For each directed edge, the reference vertex is the opposite triangle
    // vertex. Its orient_3d sign against (edge, apex) defines "inside".
    let edges = [(a, b, c), (b, c, a), (c, a, b)];
    let mut any_boundary = false;
    for &(u, v, w) in &edges {
        let ref_sign = super::orient3d::orient_3d(u, v, apex, w);
        let p_sign = orient_3d_exact_3(u, v, apex, point);
        if p_sign == Sign::Zero {
            any_boundary = true;
            continue;
        }
        // ref_sign should be non-zero for a non-degenerate triangle. If the
        // triangle is degenerate (ref_sign == Zero) the plane construction
        // would already have failed, so we can assume it is non-zero here.
        if ref_sign != Sign::Zero && p_sign != ref_sign {
            return TriangleContainment::Outside;
        }
    }
    if any_boundary {
        TriangleContainment::OnBoundary
    } else {
        TriangleContainment::Inside
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  Parameter sign (is the crossing within the segment span?)
// ──────────────────────────────────────────────────────────────────────────

/// Sign classification of the intersection parameter `t = t_num/den` relative
/// to the segment span `[0, 1]`, decided exactly. This lets a caller tell
/// whether the plane crossing falls *within* the segment `(p, q)` (`t ∈ [0,1]`)
/// versus on the infinite-line extension — without rounding `t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterSpan {
    /// `t < 0`: crossing is before `p`.
    BeforeStart,
    /// `t = 0`: crossing is exactly at `p`.
    AtStart,
    /// `0 < t < 1`: crossing is strictly inside the segment.
    Interior,
    /// `t = 1`: crossing is exactly at `q`.
    AtEnd,
    /// `t > 1`: crossing is beyond `q`.
    BeyondEnd,
}

/// Classify where the exact plane crossing of segment `(p, q)` with plane
/// `(a, b, c)` falls relative to the segment span, decided exactly.
///
/// Computes `t_num` and `den` exactly (`den` normalized positive), then:
/// - `sign(t_num)` places `t` vs 0;
/// - `sign(den − t_num)` places `t` vs 1 (since `t < 1 ⇔ t_num < den`).
///
/// # Errors
/// Same as [`construct_segment_plane_intersection_3`].
pub fn segment_plane_parameter_sign(
    p: Point3,
    q: Point3,
    a: Point3,
    b: Point3,
    c: Point3,
) -> Result<ParameterSpan, Exact3Error> {
    for (idx, pt) in [p, q, a, b, c].iter().enumerate() {
        if !all_finite(*pt) {
            return Err(Exact3Error::NonFiniteCoordinate { point: idx });
        }
    }
    let normal = ExactNormal::from_plane(a, b, c);
    if normal.is_zero() {
        return Err(Exact3Error::DegeneratePlane);
    }

    let mut den = [0.0f64; MAX_DEN];
    let den_len = normal.dot_f64(p.x - q.x, p.y - q.y, p.z - q.z, &mut den);
    let den_sign = sign_of_expansion(&den[..den_len]);
    if den_sign == Sign::Zero {
        return Err(Exact3Error::ParallelToPlane);
    }

    let mut t_num = [0.0f64; MAX_DEN];
    let t_num_len = normal.dot_f64(p.x - a.x, p.y - a.y, p.z - a.z, &mut t_num);

    // Normalize so den > 0 (flip t_num's sign along with den).
    let mut den_norm = [0.0f64; MAX_DEN];
    den_norm[..den_len].copy_from_slice(&den[..den_len]);
    let mut t_norm = [0.0f64; MAX_DEN];
    t_norm[..t_num_len].copy_from_slice(&t_num[..t_num_len]);
    if den_sign == Sign::Negative {
        negate_expansion(&mut den_norm[..den_len]);
        negate_expansion(&mut t_norm[..t_num_len]);
    }

    let t_sign = sign_of_expansion(&t_norm[..t_num_len]);

    // t vs 1: sign(den − t_num) — since den > 0, t < 1 ⇔ t_num < den ⇔ den − t_num > 0.
    let mut neg_t = [0.0f64; MAX_DEN];
    neg_t[..t_num_len].copy_from_slice(&t_norm[..t_num_len]);
    negate_expansion(&mut neg_t[..t_num_len]);
    let mut diff = [0.0f64; MAX_DEN];
    let mut scratch = [0.0f64; MAX_DEN];
    let diff_len = add_compressed(&den_norm[..den_len], &neg_t[..t_num_len], &mut scratch, &mut diff);
    let one_minus_t_sign = sign_of_expansion(&diff[..diff_len]);

    Ok(match (t_sign, one_minus_t_sign) {
        (Sign::Negative, _) => ParameterSpan::BeforeStart,
        (Sign::Zero, _) => ParameterSpan::AtStart,
        (Sign::Positive, Sign::Negative) => ParameterSpan::BeyondEnd,
        (Sign::Positive, Sign::Zero) => ParameterSpan::AtEnd,
        (Sign::Positive, Sign::Positive) => ParameterSpan::Interior,
    })
}

// ──────────────────────────────────────────────────────────────────────────
//  Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::exact_test_helper::Exact;
    use crate::specialized_libs::computational_geometry::orient3d::orient_3d;

    // ── BigInt reference for orient_3d on a rational point ─────────────────
    //
    // P = (x_num/den, y_num/den, z_num/den), den > 0. Reference computes
    // sign(det(B−A, C−A, P−A)) exactly, cross-multiplying the third row by den.

    /// Exact 3×3 determinant sign with row 3 an exact-rational vector
    /// (given as numerator Exacts and a positive denominator Exact).
    #[allow(clippy::too_many_arguments)]
    fn orient3_exact_reference(
        a: Point3,
        b: Point3,
        c: Point3,
        px_num: &Exact,
        py_num: &Exact,
        pz_num: &Exact,
        den: &Exact,
    ) -> Sign {
        let ax = Exact::from_f64(a.x);
        let ay = Exact::from_f64(a.y);
        let az = Exact::from_f64(a.z);
        let r1x = Exact::from_f64(b.x).sub(ax.clone());
        let r1y = Exact::from_f64(b.y).sub(ay.clone());
        let r1z = Exact::from_f64(b.z).sub(az.clone());
        let r2x = Exact::from_f64(c.x).sub(ax.clone());
        let r2y = Exact::from_f64(c.y).sub(ay.clone());
        let r2z = Exact::from_f64(c.z).sub(az.clone());

        // Row 3 scaled by den: num − a·den.
        let r3x = px_num.clone().sub(ax.clone().mul(den.clone()));
        let r3y = py_num.clone().sub(ay.clone().mul(den.clone()));
        let r3z = pz_num.clone().sub(az.clone().mul(den.clone()));

        // det = r3x·(r1y·r2z − r1z·r2y) − r3y·(r1x·r2z − r1z·r2x)
        //     + r3z·(r1x·r2y − r1y·r2x)
        let mx = r1y.clone().mul(r2z.clone()).sub(r1z.clone().mul(r2y.clone()));
        let my = r1x.clone().mul(r2z.clone()).sub(r1z.clone().mul(r2x.clone()));
        let mz = r1x.clone().mul(r2y.clone()).sub(r1y.clone().mul(r2x.clone()));

        let det = r3x
            .mul(mx)
            .sub(r3y.mul(my))
            .add(r3z.mul(mz));

        // den > 0 by construction, so the scaled determinant has the same sign
        // as the true (divided) determinant.
        det.sign()
    }

    /// Compute the exact numerators/denominator of the segment/plane
    /// intersection using BigInt, matching the module's construction formula.
    #[allow(clippy::type_complexity)]
    fn exact_intersection_reference(
        p: Point3,
        q: Point3,
        a: Point3,
        b: Point3,
        c: Point3,
    ) -> (Exact, Exact, Exact, Exact) {
        let e = Exact::from_f64;
        // Normal n = (b−a) × (c−a).
        let ux = e(b.x).sub(e(a.x));
        let uy = e(b.y).sub(e(a.y));
        let uz = e(b.z).sub(e(a.z));
        let vx = e(c.x).sub(e(a.x));
        let vy = e(c.y).sub(e(a.y));
        let vz = e(c.z).sub(e(a.z));
        let nx = uy.clone().mul(vz.clone()).sub(uz.clone().mul(vy.clone()));
        let ny = uz.clone().mul(vx.clone()).sub(ux.clone().mul(vz.clone()));
        let nz = ux.clone().mul(vy.clone()).sub(uy.clone().mul(vx.clone()));

        // den = n · (p − q)
        let den = nx
            .clone()
            .mul(e(p.x).sub(e(q.x)))
            .add(ny.clone().mul(e(p.y).sub(e(q.y))))
            .add(nz.clone().mul(e(p.z).sub(e(q.z))));
        // t_num = n · (p − a)
        let t_num = nx
            .clone()
            .mul(e(p.x).sub(e(a.x)))
            .add(ny.clone().mul(e(p.y).sub(e(a.y))))
            .add(nz.clone().mul(e(p.z).sub(e(a.z))));

        // x_num_i = p_i·den + t_num·(q_i − p_i)
        let x_num = e(p.x).mul(den.clone()).add(t_num.clone().mul(e(q.x).sub(e(p.x))));
        let y_num = e(p.y).mul(den.clone()).add(t_num.clone().mul(e(q.y).sub(e(p.y))));
        let z_num = e(p.z).mul(den.clone()).add(t_num.clone().mul(e(q.z).sub(e(p.z))));

        // Normalize den positive (flip numerators too).
        if den.sign() == Sign::Negative {
            (x_num.neg(), y_num.neg(), z_num.neg(), den.neg())
        } else {
            (x_num, y_num, z_num, den)
        }
    }

    // ── Basic construction: a clean rational crossing ─────────────────────

    #[test]
    fn construct_axis_aligned_crossing() {
        // Segment from (0,0,-1) to (0,0,1) crosses the z=0 plane at (0,0,0).
        let p = Point3::new(0.0, 0.0, -1.0);
        let q = Point3::new(0.0, 0.0, 1.0);
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let point = construct_segment_plane_intersection_3(p, q, a, b, c).unwrap();
        let r = point.to_point3();
        assert!(r.x.abs() < 1e-12 && r.y.abs() < 1e-12 && r.z.abs() < 1e-12, "{r:?}");
    }

    #[test]
    fn construct_diagonal_crossing_value() {
        // Segment (1,1,1)→(-1,-1,-1) crosses plane through origin with normal
        // (1,1,1) at the origin. Plane: a=(1,-1,0), b=(0,1,-1), c=(-1,0,1)
        // all satisfy x+y+z=0.
        let p = Point3::new(1.0, 1.0, 1.0);
        let q = Point3::new(-1.0, -1.0, -1.0);
        let a = Point3::new(1.0, -1.0, 0.0);
        let b = Point3::new(0.0, 1.0, -1.0);
        let c = Point3::new(-1.0, 0.0, 1.0);
        let point = construct_segment_plane_intersection_3(p, q, a, b, c).unwrap();
        let r = point.to_point3();
        assert!(r.x.abs() < 1e-12 && r.y.abs() < 1e-12 && r.z.abs() < 1e-12, "{r:?}");
    }

    #[test]
    fn construct_third_plane_rational_value() {
        // A crossing at rational coords involving thirds. Plane z = 1/3 via
        // three points at z = 1/3; segment straight down through (1/2, 1/4).
        let a = Point3::new(0.0, 0.0, 1.0 / 3.0);
        let b = Point3::new(1.0, 0.0, 1.0 / 3.0);
        let c = Point3::new(0.0, 1.0, 1.0 / 3.0);
        let p = Point3::new(0.5, 0.25, 0.0);
        let q = Point3::new(0.5, 0.25, 1.0);
        let point = construct_segment_plane_intersection_3(p, q, a, b, c).unwrap();
        let r = point.to_point3();
        assert!((r.x - 0.5).abs() < 1e-12, "x={}", r.x);
        assert!((r.y - 0.25).abs() < 1e-12, "y={}", r.y);
        assert!((r.z - 1.0 / 3.0).abs() < 1e-12, "z={}", r.z);

        // Compare the exact numerators/denominator against the BigInt reference
        // (they need not be bit-identical expansions, but sign-scaled orient
        // must match — covered below). Here confirm the rounded value is right.
        let (xr, yr, zr, dr) = exact_intersection_reference(p, q, a, b, c);
        let x_expect = ratio_to_f64(&xr, &dr);
        let y_expect = ratio_to_f64(&yr, &dr);
        let z_expect = ratio_to_f64(&zr, &dr);
        assert!((r.x - x_expect).abs() < 1e-12);
        assert!((r.y - y_expect).abs() < 1e-12);
        assert!((r.z - z_expect).abs() < 1e-12);
    }

    /// Approximate a BigInt ratio as f64 (test-only).
    fn ratio_to_f64(num: &Exact, den: &Exact) -> f64 {
        // Convert via f64 by scaling: value = mantissa*2^exp. We use a coarse
        // conversion adequate for the 1e-12 checks (num and den are small).
        let n = num.mantissa.to_string().parse::<f64>().unwrap()
            * 2f64.powi(num.exponent);
        let d = den.mantissa.to_string().parse::<f64>().unwrap()
            * 2f64.powi(den.exponent);
        n / d
    }

    // ── Degeneracies ──────────────────────────────────────────────────────

    #[test]
    fn parallel_segment_errors() {
        // Segment parallel to the z=0 plane (moves only in x).
        let p = Point3::new(0.0, 0.0, 1.0);
        let q = Point3::new(1.0, 0.0, 1.0);
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        assert!(matches!(
            construct_segment_plane_intersection_3(p, q, a, b, c),
            Err(Exact3Error::ParallelToPlane)
        ));
    }

    #[test]
    fn degenerate_plane_errors() {
        // a, b, c collinear → zero normal.
        let p = Point3::new(0.0, 0.0, -1.0);
        let q = Point3::new(0.0, 0.0, 1.0);
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(2.0, 0.0, 0.0); // collinear with a, b
        assert!(matches!(
            construct_segment_plane_intersection_3(p, q, a, b, c),
            Err(Exact3Error::DegeneratePlane)
        ));
    }

    #[test]
    fn non_finite_coordinate_errors() {
        let p = Point3::new(f64::NAN, 0.0, -1.0);
        let q = Point3::new(0.0, 0.0, 1.0);
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        assert!(matches!(
            construct_segment_plane_intersection_3(p, q, a, b, c),
            Err(Exact3Error::NonFiniteCoordinate { point: 0 })
        ));
    }

    // ── Exact orientation on the constructed point ────────────────────────

    #[test]
    fn constructed_point_is_coplanar_with_its_plane() {
        // The intersection point lies in plane (a,b,c) by construction, so
        // orient_3d_exact_3(a, b, c, P) must be exactly Zero.
        let a = Point3::new(0.0, 0.0, 1.0 / 3.0);
        let b = Point3::new(1.0, 0.0, 1.0 / 3.0);
        let c = Point3::new(0.0, 1.0, 1.0 / 3.0);
        let p = Point3::new(0.3, 0.7, 0.0);
        let q = Point3::new(0.3, 0.7, 2.0);
        let point = construct_segment_plane_intersection_3(p, q, a, b, c).unwrap();
        assert_eq!(orient_3d_exact_3(a, b, c, &point), Sign::Zero);
    }

    #[test]
    fn orient_exact_matches_bigint_reference() {
        // Construct a crossing, then test orient_3d against an arbitrary test
        // tetrahedron using our exact predicate vs the BigInt reference.
        let a = Point3::new(0.0, 0.0, 1.0 / 3.0);
        let b = Point3::new(1.0, 0.0, 1.0 / 3.0);
        let c = Point3::new(0.0, 1.0, 1.0 / 3.0);
        let p = Point3::new(0.2, 0.6, -0.5);
        let q = Point3::new(0.9, 0.1, 1.7);
        let point = construct_segment_plane_intersection_3(p, q, a, b, c).unwrap();

        let (xr, yr, zr, dr) = exact_intersection_reference(p, q, a, b, c);

        // Test against several tetrahedra apex frames.
        let frames = [
            (
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ),
            (
                Point3::new(1.0, 2.0, 3.0),
                Point3::new(-1.0, 0.5, 0.25),
                Point3::new(0.0, -2.0, 1.0),
            ),
            (
                Point3::new(0.5, 0.5, 1.0 / 3.0),
                Point3::new(0.5, -0.5, 0.0),
                Point3::new(-0.5, 0.5, 2.0),
            ),
        ];
        for (fa, fb, fc) in frames {
            let ours = orient_3d_exact_3(fa, fb, fc, &point);
            let reference = orient3_exact_reference(fa, fb, fc, &xr, &yr, &zr, &dr);
            assert_eq!(
                ours, reference,
                "orient_3d_exact_3 mismatch vs BigInt for frame ({fa:?},{fb:?},{fc:?})"
            );
        }
    }

    #[test]
    fn exact_construction_beats_naive_f64_where_it_provably_mis_signs() {
        // A hand-proven construction-rounding mis-sign.
        //
        // Segment (0,0,0)→(1,1,1) crosses the plane through
        // A=(1,0,0), B=(0,1,0), C=(0,0,1) — the plane x + y + z = 1 — at the
        // EXACT rational point (1/3, 1/3, 1/3). Each coordinate 1/3 is NOT
        // representable in f64, so `to_point3()` rounds it to
        // 0.3333333333333333, whose exact value is 6004799503160661/2^54 =
        // 1/3 − 1.85e-17 (just BELOW 1/3).
        //
        // Now test orientation of that crossing against the SAME plane's three
        // points (A,B,C). The exact point is ON the plane, so the true
        // orientation is Zero. But the rounded point sums to slightly less than
        // 1, so it is strictly on one side — an exact `orient_3d` on the ROUNDED
        // point returns a non-zero sign. This is precisely the hazard the exact
        // construction path exists to defend against: the predicate is exact,
        // but its *input* was corrupted by construction rounding.
        let p = Point3::new(0.0, 0.0, 0.0);
        let q = Point3::new(1.0, 1.0, 1.0);
        let pa = Point3::new(1.0, 0.0, 0.0);
        let pb = Point3::new(0.0, 1.0, 0.0);
        let pc = Point3::new(0.0, 0.0, 1.0);

        let point = construct_segment_plane_intersection_3(p, q, pa, pb, pc).unwrap();

        // Sanity: the constructed point rounds to (1/3, 1/3, 1/3).
        let rounded = point.to_point3();
        assert!((rounded.x - 1.0 / 3.0).abs() < 1e-12);
        assert!((rounded.y - 1.0 / 3.0).abs() < 1e-12);
        assert!((rounded.z - 1.0 / 3.0).abs() < 1e-12);

        // Ground truth from the BigInt oracle: the exact point is ON the plane.
        let (xr, yr, zr, dr) = exact_intersection_reference(p, q, pa, pb, pc);
        let oracle = orient3_exact_reference(pa, pb, pc, &xr, &yr, &zr, &dr);
        assert_eq!(oracle, Sign::Zero, "exact crossing lies ON plane x+y+z=1");

        // (1) Our exact predicate matches the oracle: Zero (coplanar).
        let exact_sign = orient_3d_exact_3(pa, pb, pc, &point);
        assert_eq!(
            exact_sign, oracle,
            "exact predicate must report the crossing ON its plane"
        );

        // (2) The naive path — round the constructed point, then run the
        // (itself exact) f64 orient_3d — DISAGREES: the rounded point is off
        // the plane. This proves the construction-rounding mis-sign is real.
        let naive_sign = orient_3d(pa, pb, pc, rounded);
        assert_ne!(
            naive_sign, oracle,
            "the rounded point must fall OFF the plane, mis-signing"
        );
        assert_eq!(
            naive_sign,
            Sign::Negative,
            "rounded (1/3,1/3,1/3) sums to < 1 → strictly one side"
        );
    }

    #[test]
    fn exact_predicate_matches_bigint_over_construction_grid() {
        // Corpus: many constructed crossings against many reference planes; the
        // exact predicate must match the BigInt oracle on EVERY one (the oracle
        // decides the sign — never a hand-guessed expectation).
        let a = Point3::new(0.0, 0.0, 1.0 / 3.0);
        let b = Point3::new(1.0, 0.0, 1.0 / 3.0);
        let c = Point3::new(0.0, 1.0, 1.0 / 3.0);
        let ref_planes = [
            (Point3::new(0.0, 0.0, 0.0), Point3::new(3.0, 0.0, 1.0), Point3::new(0.0, 3.0, 1.0)),
            (Point3::new(1.0, 1.0, 0.0), Point3::new(2.0, 0.0, 1.0), Point3::new(0.0, 2.0, 0.5)),
            (Point3::new(-1.0, -1.0, 0.0), Point3::new(1.0, 0.0, 1.0), Point3::new(0.0, 1.0, 2.0)),
            (Point3::new(1.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0), Point3::new(0.0, 0.0, 1.0)),
        ];
        let mut total = 0usize;
        for i in 0..7i32 {
            for j in 0..7i32 {
                let px = (i as f64) / 6.0;
                let py = (j as f64) / 6.0;
                let p = Point3::new(px, py, -1.0);
                let q = Point3::new(px + 0.25, py - 0.5, 2.0);
                let point = match construct_segment_plane_intersection_3(p, q, a, b, c) {
                    Ok(pt) => pt,
                    Err(_) => continue,
                };
                let (xr, yr, zr, dr) = exact_intersection_reference(p, q, a, b, c);
                for &(ra, rb, rc) in &ref_planes {
                    let oracle = orient3_exact_reference(ra, rb, rc, &xr, &yr, &zr, &dr);
                    let ours = orient_3d_exact_3(ra, rb, rc, &point);
                    assert_eq!(
                        ours, oracle,
                        "exact predicate disagrees with BigInt oracle at \
                         ({px},{py}) vs plane ({ra:?},{rb:?},{rc:?})"
                    );
                    total += 1;
                }
            }
        }
        assert!(total > 0, "no cases were exercised");
    }

    #[test]
    fn adversarial_near_coplanar_construction_matches_bigint() {
        // A battery of crossings tested against near-degenerate reference
        // planes; exact predicate must match BigInt on every one, including the
        // cases where an f64 orient_3d on the rounded point disagrees.
        let a = Point3::new(0.0, 0.0, 1.0 / 3.0);
        let b = Point3::new(7.0, 0.0, 1.0 / 3.0);
        let c = Point3::new(0.0, 5.0, 1.0 / 3.0);

        let segments = [
            (Point3::new(0.1, 0.2, -0.3), Point3::new(0.4, 0.9, 1.1)),
            (Point3::new(1.0 / 7.0, 2.0 / 7.0, -1.0), Point3::new(3.0 / 7.0, 1.0 / 7.0, 1.0)),
            (Point3::new(0.5, 0.5, -0.5), Point3::new(0.5, 0.5, 0.5)),
        ];
        let ref_planes = [
            (Point3::new(0.0, 0.0, 0.0), Point3::new(3.0, 0.0, 1.0), Point3::new(0.0, 3.0, 1.0)),
            (Point3::new(1.0, 1.0, 0.0), Point3::new(2.0, 0.0, 1.0), Point3::new(0.0, 2.0, 0.5)),
            (Point3::new(-1.0, -1.0, 0.0), Point3::new(1.0, 0.0, 1.0), Point3::new(0.0, 1.0, 2.0)),
        ];

        for &(p, q) in &segments {
            let point = construct_segment_plane_intersection_3(p, q, a, b, c).unwrap();
            let (xr, yr, zr, dr) = exact_intersection_reference(p, q, a, b, c);
            for &(ra, rb, rc) in &ref_planes {
                let ours = orient_3d_exact_3(ra, rb, rc, &point);
                let reference = orient3_exact_reference(ra, rb, rc, &xr, &yr, &zr, &dr);
                assert_eq!(
                    ours, reference,
                    "mismatch: seg ({p:?},{q:?}) vs plane ({ra:?},{rb:?},{rc:?})"
                );
            }
        }
    }

    // ── Segment/triangle intersection ─────────────────────────────────────

    #[test]
    fn segment_pierces_triangle_interior() {
        // Triangle in z=0 plane; segment straight down through its centroid.
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let centroid = Point3::new(1.0 / 3.0, 1.0 / 3.0, 0.0);
        let p = Point3::new(centroid.x, centroid.y, -1.0);
        let q = Point3::new(centroid.x, centroid.y, 1.0);
        let (_pt, containment) =
            construct_segment_triangle_intersection_3(p, q, a, b, c).unwrap();
        assert_eq!(containment, TriangleContainment::Inside);
    }

    #[test]
    fn segment_misses_triangle() {
        // Crossing point well outside the triangle.
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let p = Point3::new(5.0, 5.0, -1.0);
        let q = Point3::new(5.0, 5.0, 1.0);
        let (_pt, containment) =
            construct_segment_triangle_intersection_3(p, q, a, b, c).unwrap();
        assert_eq!(containment, TriangleContainment::Outside);
    }

    #[test]
    fn segment_hits_triangle_edge() {
        // Crossing exactly on the midpoint of edge (a, b).
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(2.0, 0.0, 0.0);
        let c = Point3::new(0.0, 2.0, 0.0);
        let p = Point3::new(1.0, 0.0, -1.0);
        let q = Point3::new(1.0, 0.0, 1.0);
        let (_pt, containment) =
            construct_segment_triangle_intersection_3(p, q, a, b, c).unwrap();
        assert_eq!(containment, TriangleContainment::OnBoundary);
    }

    #[test]
    fn segment_hits_triangle_vertex() {
        // Crossing exactly at vertex c.
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(2.0, 0.0, 0.0);
        let c = Point3::new(0.0, 2.0, 0.0);
        let p = Point3::new(0.0, 2.0, -1.0);
        let q = Point3::new(0.0, 2.0, 1.0);
        let (_pt, containment) =
            construct_segment_triangle_intersection_3(p, q, a, b, c).unwrap();
        assert_eq!(containment, TriangleContainment::OnBoundary);
    }

    // ── Parameter span ────────────────────────────────────────────────────

    #[test]
    fn parameter_span_interior() {
        // Crossing at t = 1/2 (midway).
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let p = Point3::new(0.2, 0.2, -1.0);
        let q = Point3::new(0.2, 0.2, 1.0);
        assert_eq!(
            segment_plane_parameter_sign(p, q, a, b, c).unwrap(),
            ParameterSpan::Interior
        );
    }

    #[test]
    fn parameter_span_at_start_and_end() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        // p ON the plane → t = 0.
        let p0 = Point3::new(0.3, 0.3, 0.0);
        let q0 = Point3::new(0.3, 0.3, 1.0);
        assert_eq!(
            segment_plane_parameter_sign(p0, q0, a, b, c).unwrap(),
            ParameterSpan::AtStart
        );
        // q ON the plane → t = 1.
        let p1 = Point3::new(0.3, 0.3, -1.0);
        let q1 = Point3::new(0.3, 0.3, 0.0);
        assert_eq!(
            segment_plane_parameter_sign(p1, q1, a, b, c).unwrap(),
            ParameterSpan::AtEnd
        );
    }

    #[test]
    fn parameter_span_before_and_beyond() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        // Both endpoints above the plane, plane crossing is before p (t<0).
        let p = Point3::new(0.3, 0.3, 1.0);
        let q = Point3::new(0.3, 0.3, 2.0);
        assert_eq!(
            segment_plane_parameter_sign(p, q, a, b, c).unwrap(),
            ParameterSpan::BeforeStart
        );
        // Both endpoints below the plane, crossing is beyond q (t>1).
        let p2 = Point3::new(0.3, 0.3, -2.0);
        let q2 = Point3::new(0.3, 0.3, -1.0);
        assert_eq!(
            segment_plane_parameter_sign(p2, q2, a, b, c).unwrap(),
            ParameterSpan::BeyondEnd
        );
    }

    // ── Determinism (bit-identical) ───────────────────────────────────────

    #[test]
    fn construction_is_bit_identical_across_calls() {
        let a = Point3::new(0.0, 0.0, 1.0 / 3.0);
        let b = Point3::new(1.0, 0.0, 1.0 / 3.0);
        let c = Point3::new(0.0, 1.0, 1.0 / 3.0);
        let p = Point3::new(0.2, 0.6, -0.5);
        let q = Point3::new(0.9, 0.1, 1.7);
        let p1 = construct_segment_plane_intersection_3(p, q, a, b, c).unwrap();
        let p2 = construct_segment_plane_intersection_3(p, q, a, b, c).unwrap();
        assert_eq!(p1.x_num_len, p2.x_num_len);
        assert_eq!(p1.y_num_len, p2.y_num_len);
        assert_eq!(p1.z_num_len, p2.z_num_len);
        assert_eq!(p1.den_len, p2.den_len);
        for i in 0..p1.x_num_len {
            assert_eq!(p1.x_num[i].to_bits(), p2.x_num[i].to_bits(), "x[{i}]");
        }
        for i in 0..p1.y_num_len {
            assert_eq!(p1.y_num[i].to_bits(), p2.y_num[i].to_bits(), "y[{i}]");
        }
        for i in 0..p1.z_num_len {
            assert_eq!(p1.z_num[i].to_bits(), p2.z_num[i].to_bits(), "z[{i}]");
        }
        for i in 0..p1.den_len {
            assert_eq!(p1.den[i].to_bits(), p2.den[i].to_bits(), "den[{i}]");
        }
    }

    #[test]
    fn predicate_is_deterministic() {
        let a = Point3::new(0.0, 0.0, 1.0 / 3.0);
        let b = Point3::new(1.0, 0.0, 1.0 / 3.0);
        let c = Point3::new(0.0, 1.0, 1.0 / 3.0);
        let p = Point3::new(0.2, 0.6, -0.5);
        let q = Point3::new(0.9, 0.1, 1.7);
        let point = construct_segment_plane_intersection_3(p, q, a, b, c).unwrap();
        let fa = Point3::new(0.0, 0.0, 0.0);
        let fb = Point3::new(1.0, 0.0, 0.0);
        let fc = Point3::new(0.0, 1.0, 0.0);
        let s1 = orient_3d_exact_3(fa, fb, fc, &point);
        let s2 = orient_3d_exact_3(fa, fb, fc, &point);
        assert_eq!(s1, s2);
    }

    // ── from_point3 round-trip ────────────────────────────────────────────

    #[test]
    fn from_point3_round_trips_and_orient_matches_f64() {
        // An ExactPoint3 built from a plain point (den=1) must give the same
        // orient_3d sign as the plain f64 predicate for exactly-representable
        // coordinates.
        let d = Point3::new(0.0, 0.0, 1.0);
        let ep = ExactPoint3::from_point3(d);
        let r = ep.to_point3();
        assert_eq!(r, d);

        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        assert_eq!(orient_3d_exact_3(a, b, c, &ep), orient_3d(a, b, c, d));
    }

    #[test]
    fn from_point3_matches_f64_over_integer_grid() {
        // Exhaustive small integer grid: ExactPoint3::from_point3 predicate must
        // agree with the f64 orient_3d on every non-degenerate quadruple.
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        for dx in -2..=2 {
            for dy in -2..=2 {
                for dz in -2..=2 {
                    let d = Point3::new(dx as f64, dy as f64, dz as f64);
                    let ep = ExactPoint3::from_point3(d);
                    assert_eq!(
                        orient_3d_exact_3(a, b, c, &ep),
                        orient_3d(a, b, c, d),
                        "grid mismatch at ({dx},{dy},{dz})"
                    );
                }
            }
        }
    }

    // ── Zero-heap smoke ───────────────────────────────────────────────────

    #[test]
    fn no_heap_in_construction_and_predicate() {
        let a = Point3::new(0.0, 0.0, 1.0 / 3.0);
        let b = Point3::new(1.0, 0.0, 1.0 / 3.0);
        let c = Point3::new(0.0, 1.0, 1.0 / 3.0);
        let p = Point3::new(0.5, 0.25, 0.0);
        let q = Point3::new(0.5, 0.25, 1.0);
        let point = construct_segment_plane_intersection_3(p, q, a, b, c).unwrap();
        let _ = orient_3d_exact_3(a, b, c, &point);
        let _ = segment_plane_parameter_sign(p, q, a, b, c).unwrap();
        let _ = construct_segment_triangle_intersection_3(p, q, a, b, c).unwrap();
    }
}
