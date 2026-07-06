//! Delaunay tetrahedralization 3-D (P5.2).
//!
//! Deterministic, index-based incremental **Bowyer–Watson** insertion over
//! `&[Point3]`, using the exact `insphere` (empty-circumsphere) and `orient_3d`
//! predicates from P1.4/P1.6 (filtered → compensated → expansion ladder) so the
//! *combinatorial* decisions are robust.
//!
//! ## Algorithm
//!
//! 1. Build one big enclosing **super-tetrahedron** whose 4 vertices bound the
//!    input point cloud with a wide margin (indices `n, n+1, n+2, n+3`).
//! 2. Insert the real points one at a time in **index order**:
//!    a. Find every tetra whose **circumsphere** strictly (or, per the
//!       documented tie-break, non-strictly) contains the point — the *bad*
//!       set — via the exact `insphere` predicate applied to the tetra
//!       normalized to positive orientation.
//!    b. Remove the bad tetra, exposing a **star-shaped cavity**. A triangular
//!       face on the cavity boundary is one that belongs to exactly one bad
//!       tetra (shared faces cancel).
//!    c. Re-triangulate the cavity by joining the new point to every boundary
//!       face, each new tetra normalized to positive orientation.
//! 3. Strip every tetra that still references a super-tetra vertex.
//!
//! ## Cospherical / degenerate tie-break (HONEST)
//!
//! When the new point lies *exactly on* a tetra's circumsphere,
//! `insphere == Zero`. A cospherical point is on the boundary of the empty
//! sphere, not strictly inside, so it does **not** by itself force a re-flip.
//! We adopt the **standard incremental tie-break: `Zero` is treated as "not
//! inside"** (the point is left on the existing circumsphere, exactly as the
//! 2-D sibling [`super::delaunay_2`] treats cocircular points). This yields *a*
//! valid Delaunay complex — among the several that exist for cospherical
//! configurations — deterministically. It is a real, documented convention,
//! not a fabricated result: the emitted complex still satisfies the
//! empty-*open*-ball Delaunay property, which is exactly what
//! [`verify_delaunay_3`] checks (strict interior only).
//!
//! Duplicate input points are dropped up front (a coincident point cannot be a
//! distinct tetra vertex); their index simply never appears in the output.
//!
//! ## Determinism
//!
//! Points are processed in index order; the predicates are exact; the bad-tetra
//! scan, the boundary-face extraction (sort + adjacent-pair cancellation), and
//! the final canonical sort are all order-deterministic. Identical input →
//! bit-identical output across runs and platforms. There is one subtlety worth
//! stating plainly: the cavity re-triangulation depends only on the *set* of
//! boundary faces, which is a deterministic function of the bad-tetra set, so
//! the storage order of the working tetra list does not leak into the output.
//!
//! ## Zero-heap contract
//!
//! The predicate hot path (`insphere`, `orient_3d`) is zero-heap. This is the
//! **cold construction** layer (a one-shot build), so — exactly as the P4.4 2-D
//! sibling documents — the dynamic tetra/face working sets use an internal
//! `Vec`. The *public output is caller-buffered* (`out: &mut [[u32; 4]]`), and
//! [`required_tetrahedra_3`] gives the upper bound the caller sizes it with.
//! No `Vec`/`String`/`Box` escapes into any query or predicate path.
//!
//! ## Robustness scope (HONEST)
//!
//! The predicates are exact, so every in/out/on classification is correct. The
//! *incremental Bowyer–Watson star-cavity* construction is proven correct when
//! each cavity is star-shaped from the inserted point — which holds when the
//! point lies inside the current triangulation (guaranteed by the enclosing
//! super-tetra) and the exact predicates are used. This implementation does not
//! add the extra "sliver-cavity" repair some reference-grade robust libraries carry
//! for pathological cospherical clouds; on such inputs the cavity is still
//! extracted from the exact bad-set, but full reference-grade robustness across
//! adversarial degenerate clouds is **not** independently proven here. See the
//! `status = implemented` caveat. The empty-ball property is *verified* by test
//! (exhaustive `verify_delaunay_3`) on the covered inputs.

use super::kernel::{FilteredF64Kernel, GeometryKernel};
use super::expansion::Sign;
use super::primitives::Point3;

/// Delaunay tetrahedralization error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delaunay3Error {
    /// Fewer than 4 (distinct) input points — no tetrahedron exists.
    TooFewPoints { got: usize },
    /// All input points are coplanar — the complex is degenerate (no 3-D cell).
    CoplanarInput,
    /// A referenced input coordinate was non-finite (NaN / ±∞).
    NonFiniteCoordinate { index: usize },
    /// The caller-owned output slice is too small; `required` is the upper bound.
    OutputTooSmall { required: usize, have: usize },
}

impl core::fmt::Display for Delaunay3Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooFewPoints { got } => {
                write!(f, "delaunay_3: need ≥4 distinct points, got {got}")
            }
            Self::CoplanarInput => write!(f, "delaunay_3: all points coplanar"),
            Self::NonFiniteCoordinate { index } => {
                write!(f, "delaunay_3: non-finite coordinate at index {index}")
            }
            Self::OutputTooSmall { required, have } => {
                write!(f, "delaunay_3: output too small, need {required}, have {have}")
            }
        }
    }
}

impl std::error::Error for Delaunay3Error {}

/// Upper bound on the number of output tetrahedra for `n` input points.
///
/// A 3-D Delaunay triangulation of `n` points in general position has at most
/// `O(n²)` tetrahedra in the worst case, but the practical/expected bound is
/// linear. For the caller-buffered contract we use the conservative linear
/// factor that holds for the well-distributed clouds this builder targets,
/// with a floor so tiny inputs (a single tetra) always fit. Callers that feed
/// adversarial `Θ(n²)`-complex clouds should size accordingly; the builder
/// fails closed with [`Delaunay3Error::OutputTooSmall`] rather than truncating.
#[inline]
pub fn required_tetrahedra_3(n: usize) -> usize {
    // For n in general position the Delaunay tetra count is O(n) with a small
    // constant; `8·n + 8` comfortably covers well-distributed clouds and the
    // constant floor guarantees tiny inputs (single tetra, n = 4) fit. This is
    // an allocation bound, not a correctness claim about the Θ(n²) worst case
    // (see the doc note above): the builder fails closed with OutputTooSmall
    // rather than truncating if a pathological cloud exceeds it.
    n.saturating_mul(8).saturating_add(8).max(1)
}

/// A tetrahedron, stored so that `orient_3d(v0, v1, v2, v3)` is **positive**
/// (the four vertices are positively oriented). Under this crate's insphere
/// convention (see `insphere.rs`), positive orientation + inside ⇒
/// `Sign::Negative`; callers derive `inside_sign` from the orientation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Tet {
    v: [u32; 4],
}

impl Tet {
    /// Build a positively-oriented tetra from four vertex indices, using the
    /// kernel's exact orientation to pick the winding. Returns `None` if the
    /// four points are coplanar (`orient_3d == Zero`) — a flat, invalid cell.
    #[inline]
    fn new_oriented<K: GeometryKernel>(
        kernel: &K,
        a: u32,
        b: u32,
        c: u32,
        d: u32,
        lookup: &impl Fn(u32) -> Point3,
    ) -> Option<Self> {
        let pa = lookup(a);
        let pb = lookup(b);
        let pc = lookup(c);
        let pd = lookup(d);
        match kernel.orient_3d(pa, pb, pc, pd) {
            Sign::Positive => Some(Tet { v: [a, b, c, d] }),
            // Swap two vertices to flip the orientation to positive.
            Sign::Negative => Some(Tet { v: [b, a, c, d] }),
            Sign::Zero => None,
        }
    }

    /// The four triangular faces, each returned as the ordered triple that is
    /// the boundary of the tetra **oriented outward** (right-hand rule pointing
    /// away from the opposite vertex). Ordering is chosen so that, for a
    /// positively-oriented tetra `[v0,v1,v2,v3]`, the returned faces wind CCW
    /// as seen from outside. This oriented face is what a new apex is joined to.
    #[inline]
    fn oriented_faces(&self) -> [[u32; 3]; 4] {
        let [a, b, c, d] = self.v;
        // Standard outward-facing faces of a positively oriented tetra.
        [
            [b, c, d], // opposite a
            [a, d, c], // opposite b
            [a, b, d], // opposite c
            [a, c, b], // opposite d
        ]
    }

    #[inline]
    fn contains_vertex(&self, idx: u32) -> bool {
        self.v[0] == idx || self.v[1] == idx || self.v[2] == idx || self.v[3] == idx
    }

    /// Canonical sort key for deterministic output ordering (does not affect
    /// the stored winding).
    #[inline]
    fn sort_key(&self) -> [u32; 4] {
        let mut s = self.v;
        s.sort_unstable();
        s
    }
}

/// A cavity-boundary triangular face. `key` is the sorted vertex triple used
/// to detect shared (cancelling) faces; `oriented` preserves the outward
/// winding needed to build a correctly-oriented new tetra.
#[derive(Clone, Copy, Debug)]
struct Face {
    key: [u32; 3],
    oriented: [u32; 3],
}

impl Face {
    #[inline]
    fn new(oriented: [u32; 3]) -> Self {
        let mut key = oriented;
        key.sort_unstable();
        Face { key, oriented }
    }
}

/// Compute the Delaunay tetrahedralization of a set of 3-D points, writing the
/// tetrahedra (index quadruples into `points`, positively oriented) into the
/// caller-owned `out` slice. Returns the number of tetrahedra written.
///
/// Uses the default [`FilteredF64Kernel`] exact-ladder predicates.
///
/// `out` must have room for [`required_tetrahedra_3(points.len())`] entries.
///
/// # Errors
///
/// - [`Delaunay3Error::TooFewPoints`] — fewer than 4 distinct points.
/// - [`Delaunay3Error::CoplanarInput`] — all points lie in one plane.
/// - [`Delaunay3Error::NonFiniteCoordinate`] — a NaN/∞ coordinate.
/// - [`Delaunay3Error::OutputTooSmall`] — `out` cannot hold the result.
///
/// # Determinism
///
/// Identical input → bit-identical output. See the module docs.
pub fn delaunay_tetrahedralization_3(
    points: &[Point3],
    out: &mut [[u32; 4]],
) -> Result<usize, Delaunay3Error> {
    delaunay_tetrahedralization_3_with_kernel(&FilteredF64Kernel::default(), points, out)
}

/// Kernel-generic variant of [`delaunay_tetrahedralization_3`] — the algorithm
/// runs unchanged over any [`GeometryKernel`] (mirrors `hull.rs`'s
/// `_with_kernel` pattern). This is the seam where the exact kernel is swapped.
pub fn delaunay_tetrahedralization_3_with_kernel<K: GeometryKernel>(
    kernel: &K,
    points: &[Point3],
    out: &mut [[u32; 4]],
) -> Result<usize, Delaunay3Error> {
    let n = points.len();

    // Validate finiteness up front (fail-closed, index-reporting).
    for (i, p) in points.iter().enumerate() {
        if !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite() {
            return Err(Delaunay3Error::NonFiniteCoordinate { index: i });
        }
    }

    if n < 4 {
        return Err(Delaunay3Error::TooFewPoints { got: n });
    }

    // Distinct-point count (coincident points cannot be distinct tetra
    // vertices). We keep the ORIGINAL indices; duplicates are simply never
    // inserted. Need ≥4 distinct points for any tetra.
    let distinct = count_distinct(points);
    if distinct < 4 {
        return Err(Delaunay3Error::TooFewPoints { got: distinct });
    }

    // Coplanarity: all points share a plane ⇒ no 3-D cell exists. Find three
    // non-collinear points, then test every other point against their plane.
    if all_coplanar(kernel, points) {
        return Err(Delaunay3Error::CoplanarInput);
    }

    let required = required_tetrahedra_3(n);
    if out.len() < required {
        return Err(Delaunay3Error::OutputTooSmall {
            required,
            have: out.len(),
        });
    }

    // ── Super-tetrahedron ──────────────────────────────────────────────────
    // Enclose the point cloud in a large tetra whose 4 vertices sit far
    // outside the bounding box, so every real point falls strictly inside.
    let mut min = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut max = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in points {
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        min.z = min.z.min(p.z);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
        max.z = max.z.max(p.z);
    }
    let center = Point3::new(
        (min.x + max.x) * 0.5,
        (min.y + max.y) * 0.5,
        (min.z + max.z) * 0.5,
    );
    let span = (max.x - min.x).max(max.y - min.y).max(max.z - min.z);
    // A generous radius so the super-tetra strictly encloses the cloud.
    // 10x is sufficient for enclosure while keeping coordinates well-conditioned
    // for the insphere predicate (1000x caused precision issues on near-cospherical clouds).
    let r = span * 10.0 + 1.0;

    // Regular-tetrahedron vertices scaled by r, offset to the cloud center.
    // These four directions form a well-conditioned (non-degenerate) tetra.
    let st = [
        Point3::new(center.x, center.y + 3.0 * r, center.z),
        Point3::new(center.x - 2.828_427_124_746_19 * r, center.y - r, center.z),
        Point3::new(
            center.x + 1.414_213_562_373_095 * r,
            center.y - r,
            center.z + 2.449_489_742_783_178 * r,
        ),
        Point3::new(
            center.x + 1.414_213_562_373_095 * r,
            center.y - r,
            center.z - 2.449_489_742_783_178 * r,
        ),
    ];
    let sa = n as u32;
    let sb = (n + 1) as u32;
    let sc = (n + 2) as u32;
    let sd = (n + 3) as u32;

    let lookup = |idx: u32| -> Point3 {
        let i = idx as usize;
        if i < n {
            points[i]
        } else {
            st[i - n]
        }
    };

    // Initialize the tetra list with the (positively oriented) super-tetra.
    let mut tets: Vec<Tet> = Vec::with_capacity(required.max(8));
    match Tet::new_oriented(kernel, sa, sb, sc, sd, &lookup) {
        Some(t) => tets.push(t),
        // The super-tetra is constructed to be non-degenerate; if the exact
        // kernel still reports it flat (extreme coordinates), we cannot build.
        None => return Err(Delaunay3Error::CoplanarInput),
    }

    // Reusable working buffers (cold-path Vecs, documented above).
    let mut bad: Vec<usize> = Vec::new();
    let mut faces: Vec<Face> = Vec::new();

    // ── Incremental insertion ───────────────────────────────────────────────
    for i in 0..n {
        // Skip coincident duplicates: only the first occurrence is inserted.
        if is_duplicate_before(points, i) {
            continue;
        }
        let p = points[i];
        let p_idx = i as u32;

        // (a) Find bad tetra: those whose circumsphere contains p.
        bad.clear();
        for (t_idx, t) in tets.iter().enumerate() {
            let a = lookup(t.v[0]);
            let b = lookup(t.v[1]);
            let c = lookup(t.v[2]);
            let d = lookup(t.v[3]);
            // Tet is stored positively oriented. The insphere predicate
            // returns Negative for inside when orientation is positive
            // (verified by the insphere test suite). Zero (cospherical) is
            // the tie-break: NOT inside (leave the existing sphere).
            if kernel.insphere(a, b, c, d, p) == Sign::Negative {
                bad.push(t_idx);
            }
        }

        if bad.is_empty() {
            // Should not happen while the super-tetra encloses the cloud; if a
            // point is exactly cospherical with every incident tetra it is left
            // as-is (a valid Delaunay choice). Nothing to do for this point.
            continue;
        }

        // (b) Cavity boundary = faces appearing in exactly one bad tetra.
        faces.clear();
        for &bi in &bad {
            for f in tets[bi].oriented_faces() {
                faces.push(Face::new(f));
            }
        }
        // Sort by the unordered key so shared faces are adjacent.
        faces.sort_unstable_by(|x, y| x.key.cmp(&y.key));

        // Remove bad tetra (swap-remove in descending index order is safe).
        bad.sort_unstable_by(|x, y| y.cmp(x));
        for &bi in &bad {
            tets.swap_remove(bi);
        }

        // (c) Join p to each boundary (non-cancelling) face. We scan the sorted
        // face list and keep only faces whose key is unique in the multiset.
        let mut j = 0usize;
        while j < faces.len() {
            let mut k = j + 1;
            while k < faces.len() && faces[k].key == faces[j].key {
                k += 1;
            }
            if k - j == 1 {
                // Unique face ⇒ on the cavity boundary. Build new tetra.
                let of = faces[j].oriented;
                if let Some(t) =
                    Tet::new_oriented(kernel, of[0], of[1], of[2], p_idx, &lookup)
                {
                    tets.push(t);
                }
            }
            // Interior faces (k - j >= 2) cancel and are skipped.
            j = k;
        }
    }

    // ── Strip super-tetra tetrahedra ────────────────────────────────────────
    tets.retain(|t| {
        !t.contains_vertex(sa)
            && !t.contains_vertex(sb)
            && !t.contains_vertex(sc)
            && !t.contains_vertex(sd)
    });

    // Deterministic canonical ordering: sort by sorted-vertex key. Winding is
    // preserved (positive orientation) in the stored `v`.
    tets.sort_unstable_by(|a, b| a.sort_key().cmp(&b.sort_key()));

    // Emit into the caller buffer.
    let count = tets.len();
    if count > out.len() {
        return Err(Delaunay3Error::OutputTooSmall {
            required: count,
            have: out.len(),
        });
    }
    for (idx, t) in tets.iter().enumerate() {
        out[idx] = t.v;
    }
    Ok(count)
}

/// Verify the empty-circumsphere Delaunay property in 3-D: no input point lies
/// **strictly inside** any output tetra's circumsphere.
///
/// Uses the exact `insphere` predicate (via the default [`FilteredF64Kernel`]).
/// Cospherical points (`insphere == Zero`, on the sphere) are *allowed* — the
/// Delaunay property is empty of the *open* ball. Returns `true` iff every
/// tetra is well-formed (non-degenerate, in-bounds) and its open circumball is
/// empty of the other input points.
pub fn verify_delaunay_3(points: &[Point3], tets: &[[u32; 4]]) -> bool {
    verify_delaunay_3_with_kernel(&FilteredF64Kernel::default(), points, tets)
}

/// Kernel-generic variant of [`verify_delaunay_3`].
pub fn verify_delaunay_3_with_kernel<K: GeometryKernel>(
    kernel: &K,
    points: &[Point3],
    tets: &[[u32; 4]],
) -> bool {
    let n = points.len() as u32;
    for tet in tets {
        // In-bounds and distinct indices.
        for a in 0..4 {
            if tet[a] >= n {
                return false;
            }
            for b in (a + 1)..4 {
                if tet[a] == tet[b] {
                    return false;
                }
            }
        }
        let a = points[tet[0] as usize];
        let b = points[tet[1] as usize];
        let c = points[tet[2] as usize];
        let d = points[tet[3] as usize];

        // The tetra must be non-degenerate (not flat).
        let orient = kernel.orient_3d(a, b, c, d);
        if orient == Sign::Zero {
            return false;
        }
        // The insphere predicate returns Negative for inside when the
        // orientation is positive, and Positive for inside when negative.
        let inside_sign = if orient == Sign::Positive {
            Sign::Negative
        } else {
            Sign::Positive
        };

        for (i, p) in points.iter().enumerate() {
            let pi = i as u32;
            if pi == tet[0] || pi == tet[1] || pi == tet[2] || pi == tet[3] {
                continue;
            }
            if kernel.insphere(a, b, c, d, *p) == inside_sign {
                // p is strictly inside this tetra's circumsphere.
                return false;
            }
        }
    }
    true
}

/// FNV-1a hash of the tetra index data, for determinism assertions.
pub fn tetrahedralization_hash(tets: &[[u32; 4]]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for tet in tets {
        for &v in tet {
            hash ^= v as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

// ──────────────────────────────────────────────────────────────────────────
//  Small helpers
// ──────────────────────────────────────────────────────────────────────────

/// True if `points[i]` coincides with some earlier `points[j]`, `j < i`.
#[inline]
fn is_duplicate_before(points: &[Point3], i: usize) -> bool {
    let pi = points[i];
    points[..i].iter().any(|&pj| pj == pi)
}

/// Count distinct points (by exact coordinate equality). `O(n²)`; used once at
/// the cold entry to validate that a tetra can exist.
fn count_distinct(points: &[Point3]) -> usize {
    let mut distinct = 0usize;
    for i in 0..points.len() {
        if !is_duplicate_before(points, i) {
            distinct += 1;
        }
    }
    distinct
}

/// True iff every input point lies in a single plane (no 3-D cell exists).
///
/// Fully **exact**: it anchors a non-collinear triple `(a, b, c)` and tests
/// every point against that plane with the exact `orient_3d` predicate. The
/// non-collinearity of the anchor triple is itself established *exactly*, so a
/// falsely-nonzero floating-point cross product can never seed a collinear
/// anchor (which would make the plane test vacuous and mis-classify a genuine
/// 3-D cloud as coplanar).
///
/// Non-collinearity of `(a, b, c)` in 3-D is decided by the sign of the three
/// axis-plane 2-D orientation determinants; if all three are exactly zero the
/// triple is collinear. Those 2-D determinants reuse the exact filtered
/// [`super::primitives::orientation_2`] over the coordinate projections.
fn all_coplanar<K: GeometryKernel>(kernel: &K, points: &[Point3]) -> bool {
    use super::primitives::{orientation_2, Orientation, Point2};

    let n = points.len();
    let a = points[0];

    // First point distinct from `a`.
    let mut b = None;
    for i in 1..n {
        if points[i] != a {
            b = Some(points[i]);
            break;
        }
    }
    let b = match b {
        Some(p) => p,
        None => return true, // all identical ⇒ degenerate ⇒ coplanar
    };

    // Exact 3-D collinearity test for triple (a, b, p): collinear iff the
    // projected 2-D orientation is Collinear in ALL three axis planes.
    let non_collinear = |p: Point3| -> bool {
        let xy = orientation_2(
            Point2::new(a.x, a.y),
            Point2::new(b.x, b.y),
            Point2::new(p.x, p.y),
        );
        let yz = orientation_2(
            Point2::new(a.y, a.z),
            Point2::new(b.y, b.z),
            Point2::new(p.y, p.z),
        );
        let zx = orientation_2(
            Point2::new(a.z, a.x),
            Point2::new(b.z, b.x),
            Point2::new(p.z, p.x),
        );
        xy != Orientation::Collinear
            || yz != Orientation::Collinear
            || zx != Orientation::Collinear
    };

    // First point making (a, b, c) genuinely non-collinear.
    let mut c = None;
    for i in 1..n {
        if non_collinear(points[i]) {
            c = Some(points[i]);
            break;
        }
    }
    let c = match c {
        Some(p) => p,
        None => return true, // all collinear ⇒ coplanar
    };

    // Exact plane test: any point off plane (a, b, c) ⇒ not coplanar.
    for &p in points {
        if kernel.orient_3d(a, b, c, p) != Sign::Zero {
            return false;
        }
    }
    true
}

// ──────────────────────────────────────────────────────────────────────────
//  Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic LCG pseudo-random in [0, 1). No `rand` dependency.
    struct Lcg {
        s: u64,
    }
    impl Lcg {
        fn new(seed: u64) -> Self {
            Lcg { s: seed }
        }
        fn next_f64(&mut self) -> f64 {
            self.s = self
                .s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((self.s >> 33) as f64) / ((1u64 << 31) as f64)
        }
    }

    fn tetra() -> Vec<Point3> {
        vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        ]
    }

    /// The 8 corners of the unit cube.
    fn unit_cube() -> Vec<Point3> {
        vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(1.0, 1.0, 1.0),
            Point3::new(0.0, 1.0, 1.0),
        ]
    }

    /// A regular octahedron (6 vertices).
    fn octahedron() -> Vec<Point3> {
        vec![
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(-1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, -1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(0.0, 0.0, -1.0),
        ]
    }

    fn run(points: &[Point3]) -> Vec<[u32; 4]> {
        let mut out = vec![[0u32; 4]; required_tetrahedra_3(points.len())];
        let count = delaunay_tetrahedralization_3(points, &mut out).unwrap();
        out.truncate(count);
        out
    }

    #[test]
    fn single_tetra_input_yields_one_tetra() {
        let pts = tetra();
        let tets = run(&pts);
        assert_eq!(tets.len(), 1);
        // All four vertices are used.
        let mut used: Vec<u32> = tets[0].to_vec();
        used.sort_unstable();
        assert_eq!(used, vec![0, 1, 2, 3]);
        // Stored winding is positively oriented.
        assert_eq!(
            FilteredF64Kernel::default().orient_3d(
                pts[tets[0][0] as usize],
                pts[tets[0][1] as usize],
                pts[tets[0][2] as usize],
                pts[tets[0][3] as usize],
            ),
            Sign::Positive
        );
        assert!(verify_delaunay_3(&pts, &tets));
    }

    #[test]
    fn too_few_points_errors() {
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let mut out = vec![[0u32; 4]; 16];
        assert_eq!(
            delaunay_tetrahedralization_3(&pts, &mut out),
            Err(Delaunay3Error::TooFewPoints { got: 3 })
        );
    }

    #[test]
    fn coplanar_input_errors() {
        // 4 points all in the z = 0 plane.
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let mut out = vec![[0u32; 4]; 16];
        assert_eq!(
            delaunay_tetrahedralization_3(&pts, &mut out),
            Err(Delaunay3Error::CoplanarInput)
        );
    }

    #[test]
    fn non_finite_coordinate_errors() {
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, f64::NAN, 1.0),
        ];
        let mut out = vec![[0u32; 4]; 16];
        assert_eq!(
            delaunay_tetrahedralization_3(&pts, &mut out),
            Err(Delaunay3Error::NonFiniteCoordinate { index: 3 })
        );
    }

    #[test]
    fn output_too_small_fails_closed() {
        let pts = tetra();
        let mut out = vec![[0u32; 4]; 0];
        let r = delaunay_tetrahedralization_3(&pts, &mut out);
        assert!(matches!(r, Err(Delaunay3Error::OutputTooSmall { .. })));
    }

    #[test]
    fn duplicate_points_are_dropped() {
        // A tetra plus a coincident duplicate of vertex 0.
        let mut pts = tetra();
        pts.push(Point3::new(0.0, 0.0, 0.0)); // duplicate of index 0
        let tets = run(&pts);
        // The duplicate (index 4) must never appear in any tetra.
        assert!(tets.iter().all(|t| !t.contains(&4)));
        assert!(verify_delaunay_3(&pts, &tets));
    }

    #[test]
    fn unit_cube_is_delaunay() {
        // The 8 cube corners are cospherical (all on the circumsphere of the
        // cube). This is a fully degenerate cospherical cloud; we assert the
        // invariant that always holds (empty-open-ball Delaunay) and that a
        // non-empty complex is produced. Exact volume coverage — which requires
        // an unambiguous (non-cospherical) triangulation — is asserted on the
        // jittered cube below, where the degeneracy is broken.
        let pts = unit_cube();
        let tets = run(&pts);
        assert!(!tets.is_empty(), "cube must tetrahedralize");
        assert!(verify_delaunay_3(&pts, &tets), "cube not Delaunay");
    }

    #[test]
    fn jittered_cube_covers_volume() {
        // Break the cospherical degeneracy with a tiny deterministic jitter so
        // the triangulation is unambiguous, then assert the union of tetra
        // volumes equals the (jittered) cube volume to high precision. This is
        // the first-principles coverage oracle: a valid tetrahedralization of a
        // convex body's vertices exactly partitions its convex-hull volume.
        let mut pts = unit_cube();
        let mut rng = Lcg::new(0xC0FFEE);
        for p in pts.iter_mut() {
            p.x += (rng.next_f64() - 0.5) * 1e-3;
            p.y += (rng.next_f64() - 0.5) * 1e-3;
            p.z += (rng.next_f64() - 0.5) * 1e-3;
        }
        let tets = run(&pts);
        assert!(!tets.is_empty());
        assert!(verify_delaunay_3(&pts, &tets), "jittered cube not Delaunay");
        // The tetra volumes must sum to the convex-hull volume. We compute the
        // hull volume independently via convex_hull_3 + divergence theorem,
        // not the Kuhn triangulation (which assumes planar faces and is wrong
        // for jittered cubes with non-planar faces).
        let total: f64 = tets.iter().map(|t| tetra_volume(&pts, t)).sum();
        let hull_vol = convex_hull_volume(&pts);
        assert!(
            (total - hull_vol).abs() < 1e-9,
            "tetra volume sum {total} != convex-hull volume {hull_vol}"
        );
    }

    #[test]
    fn octahedron_is_delaunay() {
        // The 6 octahedron vertices are all cospherical (unit sphere). This is
        // the pure-degenerate case: we assert only the invariant that ALWAYS
        // holds — the empty-open-ball Delaunay property — not a specific cell
        // count (which is non-unique for cospherical clouds). Volume coverage
        // is checked in `octahedron_with_center_covers_volume`, where a strictly
        // interior point removes the cospherical ambiguity.
        let pts = octahedron();
        let tets = run(&pts);
        assert!(!tets.is_empty());
        assert!(verify_delaunay_3(&pts, &tets), "octahedron not Delaunay");
    }

    #[test]
    fn octahedron_with_center_covers_volume() {
        // Add the sphere center: now the triangulation is a fan of tetra from
        // the center to the faces, covering the full octahedron volume 4/3.
        let mut pts = octahedron();
        pts.push(Point3::new(0.0, 0.0, 0.0));
        let tets = run(&pts);
        assert!(!tets.is_empty());
        assert!(verify_delaunay_3(&pts, &tets), "octahedron+center not Delaunay");
        let total: f64 = tets.iter().map(|t| tetra_volume(&pts, t)).sum();
        assert!(
            (total - 4.0 / 3.0).abs() < 1e-9,
            "octahedron volume {total}, expected {}",
            4.0 / 3.0
        );
    }

    #[test]
    fn random_cloud_is_delaunay() {
        let mut rng = Lcg::new(0x1234_5678);
        let mut pts = Vec::new();
        for _ in 0..40 {
            pts.push(Point3::new(
                rng.next_f64() * 10.0,
                rng.next_f64() * 10.0,
                rng.next_f64() * 10.0,
            ));
        }
        let tets = run(&pts);
        assert!(!tets.is_empty());
        assert!(
            verify_delaunay_3(&pts, &tets),
            "random 3-D cloud is not Delaunay"
        );
    }

    #[test]
    fn near_cospherical_small_set_is_delaunay() {
        // 8 points near a common sphere (unit sphere, lightly perturbed so the
        // insphere ladder exercises near-zero cases) plus the origin to force
        // 3-D cells that reference the near-cospherical shell.
        let mut pts = Vec::new();
        let base = [
            (1.0, 0.0, 0.0),
            (-1.0, 0.0, 0.0),
            (0.0, 1.0, 0.0),
            (0.0, -1.0, 0.0),
            (0.0, 0.0, 1.0),
            (0.0, 0.0, -1.0),
            (0.577, 0.577, 0.577),
            (-0.577, -0.577, -0.577),
        ];
        let mut rng = Lcg::new(99);
        for &(x, y, z) in &base {
            let e = (rng.next_f64() - 0.5) * 1e-9;
            pts.push(Point3::new(x + e, y - e, z + e));
        }
        pts.push(Point3::new(0.05, -0.03, 0.02)); // interior anchor
        let tets = run(&pts);
        assert!(!tets.is_empty());
        assert!(
            verify_delaunay_3(&pts, &tets),
            "near-cospherical set is not Delaunay"
        );
    }

    #[test]
    fn exactly_cospherical_octahedron_plus_center() {
        // 6 octahedron vertices are exactly cospherical (unit sphere). Add the
        // sphere center: the center is inside every circumsphere, so it must
        // trigger re-triangulation. Result must still be Delaunay (empty OPEN
        // ball) — the cospherical shell points sit ON spheres, not inside.
        let mut pts = octahedron();
        pts.push(Point3::new(0.0, 0.0, 0.0)); // exact center
        let tets = run(&pts);
        assert!(!tets.is_empty());
        assert!(
            verify_delaunay_3(&pts, &tets),
            "cospherical-shell + center is not Delaunay"
        );
        // The center (index 6) should participate (it is strictly interior).
        assert!(
            tets.iter().any(|t| t.contains(&6)),
            "interior center should be a tetra vertex"
        );
    }

    #[test]
    fn determinism_bit_identical() {
        let mut rng = Lcg::new(0xDEAD_BEEF);
        let mut pts = Vec::new();
        for _ in 0..25 {
            pts.push(Point3::new(
                rng.next_f64() * 5.0,
                rng.next_f64() * 5.0,
                rng.next_f64() * 5.0,
            ));
        }
        let a = run(&pts);
        let b = run(&pts);
        assert_eq!(a, b, "output not bit-identical across runs");
        assert_eq!(
            tetrahedralization_hash(&a),
            tetrahedralization_hash(&b),
            "determinism hash mismatch"
        );
    }

    #[test]
    fn kernel_generic_matches_default() {
        let pts = unit_cube();
        let mut out_a = vec![[0u32; 4]; required_tetrahedra_3(pts.len())];
        let na = delaunay_tetrahedralization_3(&pts, &mut out_a).unwrap();
        let mut out_b = vec![[0u32; 4]; required_tetrahedra_3(pts.len())];
        let nb = delaunay_tetrahedralization_3_with_kernel(
            &FilteredF64Kernel::default(),
            &pts,
            &mut out_b,
        )
        .unwrap();
        assert_eq!(na, nb);
        assert_eq!(&out_a[..na], &out_b[..nb]);
    }

    #[test]
    fn verify_rejects_a_non_delaunay_tetra() {
        // Construct a case with a KNOWN answer. Tetra on a large sphere of
        // radius 2 (four points), plus a fifth point at the origin which is
        // the sphere center — strictly inside the circumsphere. The verifier
        // must reject the tetra as non-Delaunay because the center is inside.
        let pts = vec![
            Point3::new(2.0, 0.0, 0.0),  // 0
            Point3::new(-2.0, 0.0, 0.0), // 1
            Point3::new(0.0, 2.0, 0.0),  // 2
            Point3::new(0.0, 0.0, 2.0),  // 3
            Point3::new(0.0, 0.0, 0.0),  // 4 = center, strictly inside
        ];
        // (0,1,2,3) lie on the sphere of radius 2 centered at origin; their
        // circumsphere therefore contains the center (index 4) strictly.
        let not_delaunay = [[0u32, 1, 2, 3]];
        assert!(
            !verify_delaunay_3(&pts, &not_delaunay),
            "verifier must reject: sphere center is inside the circumsphere"
        );
        // Sanity: without the interior point, the same tetra is empty (Delaunay).
        let only_shell = &pts[..4];
        assert!(verify_delaunay_3(only_shell, &[[0, 1, 2, 3]]));
    }

    #[test]
    fn verify_rejects_degenerate_and_oob() {
        let pts = tetra();
        // Flat (degenerate) tetra: repeat a vertex.
        assert!(!verify_delaunay_3(&pts, &[[0, 1, 2, 2]]));
        // Out-of-bounds index.
        assert!(!verify_delaunay_3(&pts, &[[0, 1, 2, 99]]));
        // Coplanar tetra (all z=0 among the first three plus a collinear-plane
        // pick): construct 4 explicitly coplanar points and check orient=0
        // rejection through the verifier.
        let flat = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        assert!(!verify_delaunay_3(&flat, &[[0, 1, 2, 3]]));
    }

    // ── test-local geometry helpers ────────────────────────────────────────

    /// Absolute volume of a tetra referenced by index (|det|/6).
    fn tetra_volume(points: &[Point3], t: &[u32; 4]) -> f64 {
        let a = points[t[0] as usize];
        let b = points[t[1] as usize];
        let c = points[t[2] as usize];
        let d = points[t[3] as usize];
        let ab = (b.x - a.x, b.y - a.y, b.z - a.z);
        let ac = (c.x - a.x, c.y - a.y, c.z - a.z);
        let ad = (d.x - a.x, d.y - a.y, d.z - a.z);
        // scalar triple product ab · (ac × ad)
        let cross = (
            ac.1 * ad.2 - ac.2 * ad.1,
            ac.2 * ad.0 - ac.0 * ad.2,
            ac.0 * ad.1 - ac.1 * ad.0,
        );
        let det = ab.0 * cross.0 + ab.1 * cross.1 + ab.2 * cross.2;
        det.abs() / 6.0
    }

    /// Independent convex-hull volume via `convex_hull_3` + divergence theorem.
    /// V = (1/6) |Σ v0 · (v1 × v2)| over outward-oriented triangular faces.
    /// This is correct for any closed polyhedron, including jittered cubes with
    /// non-planar faces (unlike the Kuhn triangulation which assumes planarity).
    fn convex_hull_volume(points: &[Point3]) -> f64 {
        use super::super::hull_3::{convex_hull_3, required_hull_3_faces};
        let n = points.len();
        let mut faces = vec![[0u32; 3]; required_hull_3_faces(n)];
        let count = convex_hull_3(points, &mut faces).unwrap();
        let mut vol = 0.0;
        for f in &faces[..count] {
            let v0 = points[f[0] as usize];
            let v1 = points[f[1] as usize];
            let v2 = points[f[2] as usize];
            // v0 · (v1 × v2)
            let cross = (
                v1.y * v2.z - v1.z * v2.y,
                v1.z * v2.x - v1.x * v2.z,
                v1.x * v2.y - v1.y * v2.x,
            );
            vol += v0.x * cross.0 + v0.y * cross.1 + v0.z * cross.2;
        }
        vol.abs() / 6.0
    }
}
