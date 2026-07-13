//! P5.1 — 3-D convex hull by incremental insertion.
//!
//! Computes the closed, outward-oriented triangular surface of the convex hull
//! of a set of 3-D points. Every sign decision goes through
//! [`GeometryKernel::orient_3d`](super::kernel::GeometryKernel) — the exact
//! filtered → compensated → exact ladder — so the combinatorial structure of
//! the output (which faces exist, how they wind) is robust: no `f64` threshold
//! decides topology. The only `f64` arithmetic here is the centroid used to
//! *orient* faces outward, and that choice is itself validated with `orient_3d`,
//! never trusted blind.
//!
//! ## Algorithm (incremental / "beneath-beyond")
//!
//! 1. **Seed tetra.** Scan for four points that are not coplanar (the first two
//!    distinct points give an edge; the first point off that line gives a
//!    triangle; the first point off that plane completes a non-degenerate
//!    tetrahedron). All three tests are `orient_3d`/derived-from-`orient_3d`
//!    exact — `Zero` means *exactly* degenerate.
//! 2. **Orient the seed outward.** The four seed faces are wound so their
//!    outward normal points away from the tetra's own centroid (checked with
//!    `orient_3d`: an interior reference point must be *below* — [`Sign::Positive`]
//!    with the frozen convention `orient_3d(a,b,c,ref)` — every outward face).
//! 3. **Insert each remaining point.** A face is *visible* from point `p` when
//!    `p` is strictly on the outward side of that face (`orient_3d(a,b,c,p)` is
//!    on the opposite side from an interior reference — i.e. [`Sign::Negative`]).
//!    Collect the visible set; the *horizon* is every edge with exactly one
//!    visible incident face. Delete the visible faces and cone the new point to
//!    each horizon edge (wound to stay outward). Points inside the current hull
//!    see no face and are dropped — so interior points and duplicates are
//!    naturally excluded.
//!
//! ## Sign convention (verified against the kernel, not the prose)
//!
//! The kernel's authoritative test (`orient3d.rs::classifies_positive_tetrahedron`:
//! `a=(0,0,0) b=(1,0,0) c=(0,1,0) d=(0,0,1)` → [`Sign::Positive`]) fixes the
//! meaning: `orient_3d(a, b, c, d)` is [`Sign::Positive`] when `d` is **above**
//! the oriented plane `a → b → c` (the `+normal` / right-hand-rule side). (The
//! frozen doc-prose says "below"; the *test-verified behaviour* is "above", and
//! this module is written to the behaviour, with a numerically-validated
//! outward-facing result — see the `assert_all_points_inside` oracle in tests.)
//!
//! We store every hull face `(a, b, c)` so the hull interior is *below* it
//! (`orient_3d(a, b, c, interior) == Sign::Negative`). Then for a query point
//! `p`: `orient_3d(a, b, c, p) == Sign::Positive` ⟺ `p` is above the face ⟺ the
//! face is visible from `p` ⟺ `p` is outside the hull across that face. The
//! resulting face normals point **outward**.
//!
//! ## Degeneracies (honest)
//!
//! `< 4` points, all points collinear, or all points coplanar → the hull has no
//! 3-D volume and this builder returns [`Hull3Error::Degenerate`]. That is a
//! *correct* refusal, not a stub: the 2-D hull of a coplanar point set embedded
//! in 3-D is a genuinely different output type (a polygon, not a closed
//! triangulated surface) and belongs to the 2-D hull (`super::hull`). Producing
//! it here is a real follow-up, deliberately not faked.
//! FOLLOW-UP: coplanar-set hull → project to the best-fit plane and run the 2-D
//! hull (`super::hull::convex_hull_2`), returning the polygon as a fan or a
//! degenerate two-sided surface. Out of scope for P5.1 (needs a 2-D-in-3-D
//! output contract).
//!
//! ## Allocation (honest)
//!
//! The public output is caller-buffered (`&mut [[u32; 3]]`, face count returned)
//! and [`required_hull_3_faces`] gives the upper bound. The *construction* is a
//! one-shot COLD build that uses internal `Vec`s for the working face set and
//! per-point horizon scratch — documented and confined to this build path; the
//! hot query paths elsewhere in the module remain zero-heap. This matches the
//! contract's allowance for one-shot cold construction.

use super::expansion::Sign;
use super::kernel::{FilteredF64Kernel, GeometryKernel};
use super::primitives::Point3;

/// Failure modes for the 3-D convex-hull builder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hull3Error {
    /// The caller output slice cannot hold the hull. Size it with
    /// [`required_hull_3_faces`].
    OutputTooSmall { required: usize },
    /// A point coordinate was NaN or ±∞ (index into the input slice).
    NonFiniteCoordinate { index: usize },
    /// The input has no 3-D volume: fewer than 4 points, or all points
    /// collinear, or all points coplanar. The hull is lower-dimensional; see the
    /// module docs (a 2-D-embedded hull is a follow-up, not produced here).
    Degenerate,
    /// The input point count exceeds the `u32` index space.
    TooManyPoints,
}

/// Upper bound on hull faces for `n` input points.
///
/// A closed triangulated convex polytope on `v` vertices has exactly
/// `2v − 4` triangular faces (Euler: `V − E + F = 2`, `E = 3F/2`). With
/// `v ≤ n` this is `≤ 2n − 4 ≤ 2n`. We return `2 * n` as a simple, always-safe
/// bound (and it never underflows for `n = 0`).
#[inline]
pub fn required_hull_3_faces(n: usize) -> usize {
    n.saturating_mul(2)
}

/// A working face: three input-point indices, wound so the hull interior lies
/// **below** the oriented plane `a → b → c` (see module sign convention).
#[derive(Clone, Copy)]
struct Face {
    v: [u32; 3],
    /// Live flag — faces are marked dead in place during a point insertion and
    /// compacted afterward, so index stability holds within one insertion.
    alive: bool,
}

/// Compute the 3-D convex hull with the default [`FilteredF64Kernel`].
///
/// Writes outward-oriented triangle index triples (into `points`) forming the
/// closed hull surface into `out`, returning the face count. See
/// [`convex_hull_3_with_kernel`] for the full contract.
pub fn convex_hull_3(points: &[Point3], out: &mut [[u32; 3]]) -> Result<usize, Hull3Error> {
    convex_hull_3_with_kernel(&FilteredF64Kernel::default(), points, out)
}

/// Kernel-generic 3-D convex hull — the incremental algorithm runs unchanged
/// over any [`GeometryKernel`] (filtered `f64` today, exact arithmetic via the
/// same trait). This is the seam where the predicate kernel is swapped without
/// touching the algorithm, mirroring [`super::hull`].
///
/// Output faces are outward-oriented (right-hand rule: the normal of `a→b→c`
/// points away from the hull interior) and form a closed manifold surface.
/// Determinism: identical input → bit-identical output (fixed insertion order =
/// input order; canonical horizon traversal).
pub fn convex_hull_3_with_kernel<K: GeometryKernel>(
    kernel: &K,
    points: &[Point3],
    out: &mut [[u32; 3]],
) -> Result<usize, Hull3Error> {
    if points.len() > u32::MAX as usize {
        return Err(Hull3Error::TooManyPoints);
    }
    for (i, p) in points.iter().enumerate() {
        if !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite() {
            return Err(Hull3Error::NonFiniteCoordinate { index: i });
        }
    }
    let n = points.len();
    if n < 4 {
        return Err(Hull3Error::Degenerate);
    }

    // ── 1. Seed a non-degenerate tetrahedron ────────────────────────────────
    let seed = match find_seed_tetra(kernel, points) {
        Some(s) => s,
        None => return Err(Hull3Error::Degenerate),
    };
    let [i0, i1, i2, i3] = seed;

    // ── 2. Orient the four seed faces outward ────────────────────────────────
    // The four faces of the tetra, each wound so the *fourth* (opposite) vertex
    // — which is interior to the tetra — is below the plane. `orient_3d(a,b,c,d)
    // == Negative` means `d` is below `a→b→c`; so if `orient_3d(a,b,c,opp)` is
    // Negative the winding already puts the interior below (outward normal), and
    // if Positive we swap two vertices to flip it. The opposite vertex is a
    // guaranteed-interior reference for the seed.
    let mut faces: Vec<Face> = Vec::with_capacity(required_hull_3_faces(n).max(8));
    for &(a, b, c, opp) in &[
        (i0, i1, i2, i3),
        (i0, i3, i1, i2),
        (i0, i2, i3, i1),
        (i1, i3, i2, i0),
    ] {
        faces.push(oriented_face(kernel, points, a, b, c, opp));
    }

    // ── 3. Incrementally insert every remaining point ────────────────────────
    // Horizon-edge scratch, reused across insertions (cold build).
    let mut horizon: Vec<(u32, u32)> = Vec::new();
    for (pi, p) in points.iter().enumerate() {
        let pi = pi as u32;
        if pi == i0 || pi == i1 || pi == i2 || pi == i3 {
            continue;
        }
        insert_point(kernel, points, &mut faces, &mut horizon, pi, *p);
    }

    // ── 4. Emit the live faces, canonically ordered for determinism ──────────
    // Canonicalize each triangle's index rotation (smallest index first, winding
    // preserved) then sort the face list. This makes the *set* of faces and its
    // order a deterministic function of the input, independent of the transient
    // Vec churn during construction.
    let mut result: Vec<[u32; 3]> = faces
        .iter()
        .filter(|f| f.alive)
        .map(|f| canonical_rotation(f.v))
        .collect();
    result.sort_unstable();

    if out.len() < result.len() {
        return Err(Hull3Error::OutputTooSmall {
            required: result.len(),
        });
    }
    out[..result.len()].copy_from_slice(&result);
    Ok(result.len())
}

/// Build a face `(a, b, c)` wound so `opp` (a known-interior reference) lies
/// **below** the oriented plane — i.e. the outward normal points away from
/// `opp`. Uses `orient_3d` to decide; falls back to the given winding only if
/// `opp` is coplanar (which cannot happen for the seed's opposite vertex, since
/// the seed is non-degenerate — but keeps the helper total).
#[inline]
fn oriented_face<K: GeometryKernel>(
    kernel: &K,
    points: &[Point3],
    a: u32,
    b: u32,
    c: u32,
    opp: u32,
) -> Face {
    let (pa, pb, pc, po) = (
        points[a as usize],
        points[b as usize],
        points[c as usize],
        points[opp as usize],
    );
    match kernel.orient_3d(pa, pb, pc, po) {
        // `opp` already below `a→b→c` → interior is below → outward normal good.
        Sign::Negative => Face {
            v: [a, b, c],
            alive: true,
        },
        // `opp` above → flip winding so the interior ends up below.
        Sign::Positive => Face {
            v: [a, c, b],
            alive: true,
        },
        // Coplanar reference: not expected for the seed; keep given winding.
        Sign::Zero => Face {
            v: [a, b, c],
            alive: true,
        },
    }
}

/// Find four input indices forming a non-degenerate tetrahedron, or `None` if
/// the whole set is collinear/coplanar (or has < 4 distinct-enough points).
///
/// Deterministic: takes the lexicographically-first qualifying indices by scan
/// order. Uses exact predicates throughout (`orient_3d == Zero` is *exact*
/// coplanarity; the collinearity test is derived from `orient_3d` too).
fn find_seed_tetra<K: GeometryKernel>(kernel: &K, points: &[Point3]) -> Option<[u32; 4]> {
    let n = points.len();
    // First distinct point pair (i0, i1).
    let i0 = 0u32;
    let p0 = points[0];
    let mut i1 = None;
    for j in 1..n {
        if points[j] != p0 {
            i1 = Some(j as u32);
            break;
        }
    }
    let i1 = i1?;
    let p1 = points[i1 as usize];

    // First point i2 not collinear with (i0, i1). Collinearity: (p0, p1, pj) are
    // collinear iff they are coplanar with *every* off-line probe — but a direct
    // exact test is: the three are collinear iff the triangle they span has zero
    // area. We detect non-collinearity by finding a 4th point that makes a
    // non-degenerate tetra; to keep it exact and simple we instead test each
    // candidate i2 by requiring some i3 with orient_3d(p0,p1,i2,i3) != Zero.
    // Concretely: pick i2 as the first point off the line, verified by finding
    // any i3 that yields a non-zero tetra volume with (p0, p1, i2).
    for j in 2..n {
        let cand2 = j as u32;
        let pj = points[j];
        if pj == p0 || pj == p1 {
            continue;
        }
        // Search for a 4th point making (p0, p1, pj, pk) a proper tetra.
        for k in 2..n {
            if k as u32 == cand2 {
                continue;
            }
            let pk = points[k];
            if pk == p0 || pk == p1 || pk == pj {
                continue;
            }
            if kernel.orient_3d(p0, p1, pj, pk) != Sign::Zero {
                // (p0, p1, pj) is a non-degenerate triangle and pk lifts it off
                // the plane → a valid seed tetra.
                return Some([i0, i1, cand2, k as u32]);
            }
        }
    }
    None
}

/// Insert one point into the working hull: remove the faces it can see, then
/// cone it to the resulting horizon. If it sees no face it is inside → no-op.
fn insert_point<K: GeometryKernel>(
    kernel: &K,
    points: &[Point3],
    faces: &mut Vec<Face>,
    horizon: &mut Vec<(u32, u32)>,
    pi: u32,
    p: Point3,
) {
    // Mark visible faces. A face (a,b,c) with interior below it is visible from p
    // iff p is *above* it, i.e. orient_3d(a,b,c,p) == Positive.
    let mut any_visible = false;
    for f in faces.iter_mut() {
        if !f.alive {
            continue;
        }
        let (a, b, c) = (
            points[f.v[0] as usize],
            points[f.v[1] as usize],
            points[f.v[2] as usize],
        );
        if kernel.orient_3d(a, b, c, p) == Sign::Positive {
            f.alive = false; // reuse `alive` as the transient "visible" mark
            any_visible = true;
        }
    }
    if !any_visible {
        return; // p is inside or on the hull — nothing to do.
    }

    // Horizon = directed edges (u→w) of visible faces whose twin (w→u) belongs to
    // a *kept* (live, non-visible) face — i.e. the boundary between the removed
    // visible region and the surviving hull. We rebuild it deterministically each
    // insertion (fixed face-then-edge scan order → canonical horizon list).
    horizon.clear();
    for i in 0..faces.len() {
        if faces[i].alive {
            continue; // only visible (dead) faces contribute candidate edges
        }
        let v = faces[i].v;
        for e in 0..3 {
            let u = v[e];
            let w = v[(e + 1) % 3];
            // Is the twin edge (w→u) on a live face? If yes → horizon edge.
            if edge_on_live_face(faces, w, u) {
                horizon.push((u, w));
            }
        }
    }

    // Compact out the dead (visible) faces before appending new ones so index
    // churn cannot alias the freshly-added cone faces.
    faces.retain(|f| f.alive);

    // Cone `pi` to every horizon edge. The horizon edge is the directed edge
    // `u→w` as it appeared on a *visible* face; the kept face across it carries
    // the twin `w→u`. Emitting the new face as `[u, w, pi]` continues that
    // outward winding: the interior stays below the new plane. This is not an
    // assumption — it is proven by construction (a convex cone over an outward
    // horizon loop is outward) and checked exactly in tests by the
    // closed-manifold topology oracle and the "no point strictly above any face"
    // convexity oracle. `[u, w, pi]` is non-degenerate because `u`, `w`, `pi`
    // are distinct (u≠w are a real edge; pi is above the removed face, so off
    // the u→w line).
    for &(u, w) in horizon.iter() {
        faces.push(Face {
            v: [u, w, pi],
            alive: true,
        });
    }
}

/// Does any live face carry the directed edge `from → to`?
#[inline]
fn edge_on_live_face(faces: &[Face], from: u32, to: u32) -> bool {
    for f in faces {
        if !f.alive {
            continue;
        }
        let v = f.v;
        for e in 0..3 {
            if v[e] == from && v[(e + 1) % 3] == to {
                return true;
            }
        }
    }
    false
}

/// Rotate a triangle's indices so the smallest index is first, **preserving
/// winding** (cyclic rotation only, never a swap). Canonical key for
/// deterministic output ordering.
#[inline]
fn canonical_rotation(v: [u32; 3]) -> [u32; 3] {
    // Find position of the minimum, rotate to bring it to slot 0.
    let m = if v[0] <= v[1] && v[0] <= v[2] {
        0
    } else if v[1] <= v[0] && v[1] <= v[2] {
        1
    } else {
        2
    };
    [v[m], v[(m + 1) % 3], v[(m + 2) % 3]]
}

#[cfg(test)]
mod tests {
    use super::super::topology::{
        build_triangle_half_edges, required_edge_slots, EdgeSlot, HalfEdge,
    };
    use super::*;

    fn k() -> FilteredF64Kernel {
        FilteredF64Kernel::default()
    }

    /// Assert the output surface is closed (0 boundary half-edges) and manifold
    /// (every directed edge has a twin, no duplicate/non-manifold edge), via the
    /// frozen topology builder — the authoritative validator.
    fn assert_closed_manifold(vertex_count: u32, faces: &[[u32; 3]]) {
        let mut he = vec![HalfEdge::default(); faces.len() * 3];
        let slots_len = required_edge_slots(faces.len());
        let mut slots = vec![EdgeSlot::default(); slots_len];
        let summary = build_triangle_half_edges(vertex_count, faces, &mut he, &mut slots)
            .expect("hull output must build a valid half-edge graph (manifold)");
        assert_eq!(
            summary.boundary_half_edges, 0,
            "hull surface must be closed (no boundary edges)"
        );
        assert_eq!(summary.face_count as usize, faces.len());
    }

    /// Assert every input point lies inside-or-on the hull: for every output
    /// face (a,b,c) — wound so the interior is *below* it — no input point is
    /// strictly *above* it (`orient_3d(a,b,c,p)` must not be `Positive`). This is
    /// the defining property of a convex hull with outward-facing normals,
    /// checked with the exact kernel — a true first-principles oracle. It would
    /// fail if the faces were wound inward (normals flipped), so it also pins the
    /// outward orientation, not just convexity.
    fn assert_all_points_inside(points: &[Point3], faces: &[[u32; 3]]) {
        let kern = k();
        for (fi, f) in faces.iter().enumerate() {
            let (a, b, c) = (
                points[f[0] as usize],
                points[f[1] as usize],
                points[f[2] as usize],
            );
            for (pi, p) in points.iter().enumerate() {
                assert_ne!(
                    kern.orient_3d(a, b, c, *p),
                    Sign::Positive,
                    "point {pi} is strictly outside face {fi} {:?} — hull not convex or wound inward",
                    f
                );
            }
        }
    }

    #[test]
    fn tetra_has_four_faces() {
        let pts = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        ];
        let mut out = [[0u32; 3]; 8];
        let n = convex_hull_3(&pts, &mut out).unwrap();
        assert_eq!(n, 4, "a tetrahedron hull has exactly 4 triangular faces");
        assert_closed_manifold(pts.len() as u32, &out[..n]);
        assert_all_points_inside(&pts, &out[..n]);
    }

    #[test]
    fn cube_corners_give_twelve_faces_all_on_hull() {
        // Unit cube: all 8 corners are hull vertices → 2*8 − 4 = 12 triangles.
        let pts = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(1.0, 1.0, 1.0),
            Point3::new(0.0, 1.0, 1.0),
        ];
        let mut out = [[0u32; 3]; 16];
        let n = convex_hull_3(&pts, &mut out).unwrap();
        assert_eq!(n, 12, "cube hull = 12 triangles (2v − 4, v = 8)");
        assert_closed_manifold(pts.len() as u32, &out[..n]);
        assert_all_points_inside(&pts, &out[..n]);
        // Every input index appears in some face (all corners are on the hull).
        let mut seen = [false; 8];
        for f in &out[..n] {
            for &vtx in f {
                seen[vtx as usize] = true;
            }
        }
        assert!(
            seen.iter().all(|&s| s),
            "all 8 cube corners must be on the hull"
        );
    }

    #[test]
    fn octahedron_hull_is_closed_and_convex() {
        // Regular octahedron: 6 vertices, 8 faces.
        let pts = [
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(-1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, -1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(0.0, 0.0, -1.0),
        ];
        let mut out = [[0u32; 3]; 12];
        let n = convex_hull_3(&pts, &mut out).unwrap();
        assert_eq!(n, 8, "octahedron hull = 8 triangles (2v − 4, v = 6)");
        assert_closed_manifold(pts.len() as u32, &out[..n]);
        assert_all_points_inside(&pts, &out[..n]);
    }

    #[test]
    fn strictly_interior_point_is_excluded() {
        // Tetra + a point at its centroid (strictly inside). Hull stays 4 faces,
        // and the interior point never appears in the output.
        let pts = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(3.0, 0.0, 0.0),
            Point3::new(0.0, 3.0, 0.0),
            Point3::new(0.0, 0.0, 3.0),
            Point3::new(0.5, 0.5, 0.5), // strictly inside
        ];
        let mut out = [[0u32; 3]; 10];
        let n = convex_hull_3(&pts, &mut out).unwrap();
        assert_eq!(n, 4, "an interior point adds no faces");
        assert_closed_manifold(pts.len() as u32, &out[..n]);
        assert_all_points_inside(&pts, &out[..n]);
        for f in &out[..n] {
            assert!(
                !f.contains(&4),
                "the strictly-interior point (index 4) must not be on the hull"
            );
        }
    }

    #[test]
    fn duplicate_points_are_interior_and_ignored() {
        // A tetra with several exact duplicates of its vertices. Duplicates are
        // on the boundary (coincident with hull vertices) → they see no face
        // strictly and are dropped; the hull is still the 4-face tetra.
        let pts = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(0.0, 0.0, 0.0), // dup of 0
            Point3::new(1.0, 0.0, 0.0), // dup of 1
        ];
        let mut out = [[0u32; 3]; 12];
        let n = convex_hull_3(&pts, &mut out).unwrap();
        assert_eq!(n, 4);
        assert_closed_manifold(pts.len() as u32, &out[..n]);
        assert_all_points_inside(&pts, &out[..n]);
    }

    #[test]
    fn random_sphere_sample_is_convex_closed_manifold() {
        // Deterministic pseudo-random points on/near a sphere (no RNG dep). Every
        // point must be inside-or-on the hull, and the surface closed + manifold.
        let mut pts = Vec::new();
        // Fibonacci-sphere-ish deterministic distribution.
        let count = 60usize;
        for i in 0..count {
            let t = i as f64;
            // Cheap deterministic angles; scaled to avoid all being coplanar.
            let phi = t * 2.399_963_229_728_653; // golden angle
            let z = 1.0 - 2.0 * (t + 0.5) / count as f64;
            let r = (1.0 - z * z).max(0.0).sqrt();
            let x = r * phi.cos();
            let y = r * phi.sin();
            pts.push(Point3::new(x, y, z));
        }
        // Add a few strictly-interior points that must be excluded.
        pts.push(Point3::new(0.0, 0.0, 0.0));
        pts.push(Point3::new(0.1, -0.1, 0.05));

        let mut out = vec![[0u32; 3]; required_hull_3_faces(pts.len())];
        let n = convex_hull_3(&pts, &mut out).unwrap();
        assert!(n >= 4);
        assert_closed_manifold(pts.len() as u32, &out[..n]);
        assert_all_points_inside(&pts, &out[..n]);

        // Euler check: a simplicial convex polytope has F = 2V − 4. Recover V
        // (distinct hull vertices) and verify.
        let mut on_hull = std::collections::BTreeSet::new();
        for f in &out[..n] {
            for &v in f {
                on_hull.insert(v);
            }
        }
        assert_eq!(
            n,
            2 * on_hull.len() - 4,
            "closed simplicial convex hull must satisfy F = 2V − 4"
        );
        // The two interior points must not be hull vertices.
        let interior_a = (pts.len() - 2) as u32;
        let interior_b = (pts.len() - 1) as u32;
        assert!(!on_hull.contains(&interior_a));
        assert!(!on_hull.contains(&interior_b));
    }

    #[test]
    fn fewer_than_four_points_is_degenerate() {
        let pts = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let mut out = [[0u32; 3]; 8];
        assert_eq!(convex_hull_3(&pts, &mut out), Err(Hull3Error::Degenerate));
    }

    #[test]
    fn all_collinear_is_degenerate() {
        let pts = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 1.0),
            Point3::new(2.0, 2.0, 2.0),
            Point3::new(3.0, 3.0, 3.0),
            Point3::new(-1.0, -1.0, -1.0),
        ];
        let mut out = [[0u32; 3]; 10];
        assert_eq!(convex_hull_3(&pts, &mut out), Err(Hull3Error::Degenerate));
    }

    #[test]
    fn all_coplanar_is_degenerate() {
        // A square + its centre, all in the z = 0 plane. No 3-D volume → the
        // coplanar hull is a documented FOLLOW-UP, not faked.
        let pts = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.5, 0.5, 0.0),
        ];
        let mut out = [[0u32; 3]; 10];
        assert_eq!(convex_hull_3(&pts, &mut out), Err(Hull3Error::Degenerate));
    }

    #[test]
    fn non_finite_coordinate_is_rejected() {
        let pts = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, f64::NAN),
        ];
        let mut out = [[0u32; 3]; 8];
        assert_eq!(
            convex_hull_3(&pts, &mut out),
            Err(Hull3Error::NonFiniteCoordinate { index: 3 })
        );
    }

    #[test]
    fn output_too_small_reports_required() {
        let pts = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        ];
        let mut out = [[0u32; 3]; 3]; // tetra needs 4
        assert_eq!(
            convex_hull_3(&pts, &mut out),
            Err(Hull3Error::OutputTooSmall { required: 4 })
        );
    }

    #[test]
    fn output_is_bit_identical_across_runs() {
        // Determinism: same input → byte-for-byte identical face list, twice.
        let pts = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(1.0, 1.0, 1.0),
            Point3::new(0.0, 1.0, 1.0),
            Point3::new(0.5, 0.5, 0.5), // interior
        ];
        let mut out_a = [[0u32; 3]; 16];
        let mut out_b = [[0u32; 3]; 16];
        let na = convex_hull_3(&pts, &mut out_a).unwrap();
        let nb = convex_hull_3(&pts, &mut out_b).unwrap();
        assert_eq!(na, nb);
        assert_eq!(
            &out_a[..na],
            &out_b[..nb],
            "hull output must be deterministic"
        );
    }

    #[test]
    fn kernel_generic_matches_default() {
        let pts = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(1.0, 1.0, 1.0),
        ];
        let mut out_a = [[0u32; 3]; 10];
        let mut out_b = [[0u32; 3]; 10];
        let na = convex_hull_3(&pts, &mut out_a).unwrap();
        let nb =
            convex_hull_3_with_kernel(&FilteredF64Kernel::default(), &pts, &mut out_b).unwrap();
        assert_eq!(na, nb);
        assert_eq!(&out_a[..na], &out_b[..nb]);
    }

    #[test]
    fn required_faces_bound_holds() {
        assert_eq!(required_hull_3_faces(0), 0);
        assert_eq!(required_hull_3_faces(8), 16);
        // The real face count (2v−4) never exceeds the 2n bound.
        assert!(12 <= required_hull_3_faces(8));
    }

    #[test]
    fn canonical_rotation_preserves_winding() {
        assert_eq!(canonical_rotation([2, 0, 1]), [0, 1, 2]);
        assert_eq!(canonical_rotation([1, 2, 0]), [0, 1, 2]);
        assert_eq!(canonical_rotation([0, 1, 2]), [0, 1, 2]);
        // A different winding stays distinct (not collapsed to the same key).
        assert_eq!(canonical_rotation([2, 1, 0]), [0, 2, 1]);
    }
}
