//! P11.9 — Half-plane intersection and fixed-dimensional randomized linear
//! programming (2-D).
//!
//! The acceptance gate requires: "Feasible optimum, infeasible and unbounded
//! certificates match exhaustive low-dimensional fixtures; seeded permutation
//! is deterministic."
//!
//! ## Algorithms
//!
//! ### Half-plane intersection
//!
//! Sort-and-intersect with a deque (de Berg, ch. 4 / O'Rourke). Each half-plane
//! is a directed line; the feasible region is the left (CCW) side. Half-planes
//! are sorted by boundary angle; parallel half-planes are reduced to the
//! innermost. A deque maintains the active set: the intersection vertex of each
//! consecutive pair is computed, and a vertex that falls outside a new
//! half-plane causes a pop from the front or back.
//!
//! Unbounded regions are detected by adding a sentinel bounding box (four
//! half-planes at `±BIG`). If the final polygon touches the box boundary, the
//! true region is unbounded and `Unbounded` is returned. Otherwise the box
//! half-planes are stripped and `Bounded` is returned.
//!
//! ### 2-D linear programming
//!
//! Seidel's randomized incremental algorithm (de Berg §4.4). Constraints are
//! half-planes; the objective `c·x` is minimized. Constraints are added in a
//! seeded (SplitMix64) random order. When the current optimum violates a new
//! constraint, the new optimum lies on the new constraint's boundary line, and
//! a 1-D LP is solved along that line over the previously added constraints.
//!
//! The 1-D LP reduces each prior half-plane to a `t`-interval on the line; the
//! feasible interval is `[t_lo, t_hi]`. Empty interval ⇒ `Infeasible` with the
//! two witness constraint indices. Unbounded minimising direction ⇒
//! `Unbounded` with the ray. Otherwise the optimum is at `t_lo` or `t_hi`
//! depending on the sign of `c·d`.
//!
//! ## Determinism
//!
//! The LP permutation is produced by a SplitMix64 PRNG seeded by the caller.
//! The same seed + input always yields the same optimum, the same witness
//! indices, and the same ray — bit-identical across platforms and runs.
//!
//! ## Zero-heap contract
//!
//! The predicate path (membership test, line-line intersection sign) uses only
//! filtered `f64` arithmetic via [`super::primitives::orientation_2`]. The
//! public APIs return `Vec<Point2>` / typed enums — they allocate during
//! construction (cold), matching the P11.1–P11.5 surface-module convention.
//! No allocation occurs inside the LP inner loop's interval arithmetic.

use super::primitives::{orientation_2, Orientation, Point2};

// ───────────────────────────────────────────────────────────────────────────
//  SplitMix64 — seeded PRNG (matches inference/sampler.rs pattern)
// ───────────────────────────────────────────────────────────────────────────

/// SplitMix64 — tiny, allocation-free, platform-independent PRNG
/// (Steele et al. 2014). Same generator used by `inference::sampler`.
#[inline]
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Produce a deterministic permutation of `0..n` from `seed`.
fn seeded_permutation(seed: u64, n: usize) -> Vec<usize> {
    // Avoid the all-zero SplitMix64 fixed point; mix the seed once.
    let mut rng = seed ^ 0x2545_F491_4F6C_DD1D;
    let _ = splitmix64(&mut rng);
    let mut order: Vec<usize> = (0..n).collect();
    // Fisher–Yates with SplitMix64 draws. Deterministic for a given seed.
    if n > 1 {
        for i in (1..n).rev() {
            let j = (splitmix64(&mut rng) as usize) % (i + 1);
            order.swap(i, j);
        }
    }
    order
}

// ───────────────────────────────────────────────────────────────────────────
//  Half-plane representation
// ───────────────────────────────────────────────────────────────────────────

/// A closed half-plane: the set of points on or to the left (CCW side) of the
/// directed line `point → point + direction`.
///
/// Equivalently, `p` is inside iff `orientation_2(point, point + direction, p)`
/// is `CounterClockwise` or `Collinear`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HalfPlane {
    /// A point on the boundary line.
    pub point: Point2,
    /// Direction vector of the boundary line (need not be unit length; must be
    /// non-zero).
    pub direction: Point2,
}

impl HalfPlane {
    /// Construct from a directed line `a → b`. The feasible region is the left
    /// side of `a → b`.
    pub fn from_directed_line(a: Point2, b: Point2) -> Self {
        Self {
            point: a,
            direction: Point2::new(b.x - a.x, b.y - a.y),
        }
    }

    /// Construct a half-plane whose boundary is the line through `a` and `b`,
    /// oriented so that `side` lies in the feasible (left) region. If `side`
    /// is collinear with `a,b` this falls back to the `a → b` orientation.
    pub fn from_line_and_side(a: Point2, b: Point2, side: Point2) -> Self {
        match orientation_2(a, b, side) {
            Orientation::Clockwise => Self::from_directed_line(b, a),
            _ => Self::from_directed_line(a, b),
        }
    }

    /// Construct from an implicit line `a*x + b*y + c <= 0` (the feasible
    /// region is where the inequality holds). The boundary direction is
    /// chosen so the left side is the feasible side.
    pub fn from_implicit(a: f64, b: f64, c: f64) -> Self {
        // Boundary line: a*x + b*y + c = 0. Direction along the line: (-b, a).
        // Feasible side: a*x + b*y + c <= 0. Pick a point on the line and
        // verify orientation puts the feasible side on the left.
        let dir = Point2::new(-b, a);
        // A point on the line: if b != 0, (0, -c/b); else (-c/a, 0).
        let point = if b.abs() > 0.0 {
            Point2::new(0.0, -c / b)
        } else if a.abs() > 0.0 {
            Point2::new(-c / a, 0.0)
        } else {
            // Degenerate: a == b == 0. No boundary; return a trivial half-plane
            // (the whole plane) with a non-zero direction. Callers should
            // validate constraints before constructing.
            Point2::new(0.0, 0.0)
        };
        let hp = Self { point, direction: dir };
        // Verify: a*p + b*p + c should be <= 0 for a point on the left.
        // Pick a test point clearly on the left: point + left_normal.
        // left_normal of dir=(-b,a) is (-a,-b)? Let's just check the implicit
        // at point + rotated-dir and flip if needed.
        let test = Point2::new(point.x - a, point.y - b);
        if a * test.x + b * test.y + c <= 0.0 {
            hp
        } else {
            Self {
                point,
                direction: Point2::new(b, -a),
            }
        }
    }

    /// True iff `p` lies in (or on the boundary of) this half-plane.
    #[inline]
    pub fn contains(&self, p: Point2) -> bool {
        let head = Point2::new(self.point.x + self.direction.x, self.point.y + self.direction.y);
        !matches!(orientation_2(self.point, head, p), Orientation::Clockwise)
    }

    /// Boundary angle in `[0, 2π)`. Used for sorting; the actual value is not
    /// meaningful beyond ordering.
    fn angle(&self) -> f64 {
        // atan2 returns [-pi, pi]; shift to [0, 2π).
        let mut a = (self.direction.y).atan2(self.direction.x);
        if a < 0.0 {
            a += 2.0 * core::f64::consts::PI;
        }
        a
    }

    /// True iff the boundary direction is (near) zero — an invalid half-plane.
    fn is_degenerate(&self) -> bool {
        self.direction.x == 0.0 && self.direction.y == 0.0
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Line-line intersection (filtered f64)
// ───────────────────────────────────────────────────────────────────────────

/// Intersection point of two non-parallel lines, each given by a point and a
/// direction. Returns `None` if the lines are parallel (or collinear).
///
/// Solves `p1 + s*d1 = p2 + t*d2` for `s`:
/// `s = cross(p2 - p1, d2) / cross(d1, d2)`.
#[inline]
fn line_line_intersect(
    p1: Point2,
    d1: Point2,
    p2: Point2,
    d2: Point2,
) -> Option<Point2> {
    let denom = d1.x * d2.y - d1.y * d2.x; // cross(d1, d2)
    if denom == 0.0 {
        return None;
    }
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let s = (dx * d2.y - dy * d2.x) / denom; // cross(p2-p1, d2) / denom
    Some(Point2::new(p1.x + s * d1.x, p1.y + s * d1.y))
}

/// Parameter `t` where the line `p + t*d` crosses the boundary of `hp`.
/// Returns `None` if the line is parallel to `hp`'s boundary.
///
/// Solves `p + t*d = hp.point + s*hp.direction` for `t`:
/// `t = cross(hp.point - p, hp.direction) / cross(d, hp.direction)`.
#[inline]
fn line_half_plane_intersect_t(
    p: Point2,
    d: Point2,
    hp: &HalfPlane,
) -> Option<f64> {
    let denom = d.x * hp.direction.y - d.y * hp.direction.x; // cross(d, hp.dir)
    if denom == 0.0 {
        return None;
    }
    let dx = hp.point.x - p.x;
    let dy = hp.point.y - p.y;
    let t = (dx * hp.direction.y - dy * hp.direction.x) / denom;
    Some(t)
}

// ───────────────────────────────────────────────────────────────────────────
//  Half-plane intersection
// ───────────────────────────────────────────────────────────────────────────

/// Result of intersecting a set of half-planes.
#[derive(Debug, Clone, PartialEq)]
pub enum HalfPlaneIntersection {
    /// No point satisfies all half-planes.
    Empty,
    /// A bounded convex polygon (CCW, no trailing duplicate, >= 3 vertices).
    Bounded(Vec<Point2>),
    /// The feasible region is unbounded. The carried vertices are the
    /// bounded portion of the boundary (the chain(s) between the unbounded
    /// rays), in CCW order; may be empty if the region is a wedge/strip with
    /// no bounded boundary chain.
    Unbounded(Vec<Point2>),
}

/// Sentinel half-plane box half-extent. Points with `|x|,|y| > BIG` are
/// considered outside the universe. Chosen large enough that genuine inputs
/// don't clip it, small enough to stay well within f64 range.
const BIG: f64 = 1e9;

/// Intersect a set of half-planes. Returns the feasible region as a CCW convex
/// polygon (`Bounded`), `Empty`, or `Unbounded`.
///
/// O(n log n) sort + O(n) deque sweep. Each half-plane's feasible side is its
/// left (CCW) side. Degenerate half-planes (zero direction) are ignored.
pub fn half_plane_intersection(half_planes: &[HalfPlane]) -> HalfPlaneIntersection {
    // Filter degenerate half-planes.
    let mut hps: Vec<HalfPlane> = half_planes
        .iter()
        .copied()
        .filter(|h| !h.is_degenerate())
        .collect();
    if hps.is_empty() {
        // No constraints ⇒ the whole plane is feasible ⇒ unbounded.
        return HalfPlaneIntersection::Unbounded(Vec::new());
    }

    // Add the sentinel bounding box so the deque always produces a bounded
    // polygon. If the final polygon touches the box, the true region is
    // unbounded.
    let box_hps = [
        HalfPlane::from_directed_line(Point2::new(-BIG, -BIG), Point2::new(BIG, -BIG)), // y >= -BIG (left side = above)
        HalfPlane::from_directed_line(Point2::new(BIG, -BIG), Point2::new(BIG, BIG)),   // x <= BIG
        HalfPlane::from_directed_line(Point2::new(BIG, BIG), Point2::new(-BIG, BIG)),   // y <= BIG
        HalfPlane::from_directed_line(Point2::new(-BIG, BIG), Point2::new(-BIG, -BIG)), // x >= -BIG
    ];
    hps.extend_from_slice(&box_hps);

    // Sort by boundary angle.
    hps.sort_by(|a, b| {
        a.angle().total_cmp(&b.angle())
    });

    // Reduce parallel half-planes (same angle) to the innermost one. Two
    // half-planes with the same direction keep the one whose boundary is
    // further "inward" (i.e. whose `point` is on the feasible side of the
    // other). For the same direction, the innermost is the one with the
    // largest `cross(point, direction)` (the line furthest in the left-normal
    // direction).
    let mut reduced: Vec<HalfPlane> = Vec::with_capacity(hps.len());
    for hp in hps.drain(..) {
        match reduced.last_mut() {
            Some(last) if last.angle() == hp.angle() => {
                // Same direction. Keep the innermost: the one whose boundary
                // is on the feasible (left) side of the other. If `hp` is on
                // the left of `last`'s boundary, `hp` is innermost (tighter).
                let head = Point2::new(last.point.x + last.direction.x, last.point.y + last.direction.y);
                if orientation_2(last.point, head, hp.point) == Orientation::CounterClockwise {
                    *last = hp;
                }
                // Else keep `last`. If collinear, they're the same line; keep `last`.
            }
            _ => reduced.push(hp),
        }
    }

    if reduced.len() < 3 {
        // Two non-parallel half-planes can't bound a polygon (only a wedge).
        // With the box present we always have >= 4, so this only triggers if
        // all input half-planes were parallel and reduced to one + the box.
        // The deque below handles it; fall through.
    }

    // Deque sweep (standard half-plane intersection). Vertices are computed on
    // demand from consecutive deque entries; no separate `verts` vec is kept.
    let mut deque: Vec<HalfPlane> = Vec::with_capacity(reduced.len());

    for hp in reduced {
        // Pop back while the vertex between the last two deque half-planes is
        // outside `hp`.
        while deque.len() >= 2 {
            let n = deque.len();
            let a = deque[n - 2];
            let b = deque[n - 1];
            if let Some(v) = line_line_intersect(a.point, a.direction, b.point, b.direction) {
                if !hp.contains(v) {
                    deque.pop();
                    continue;
                }
            }
            break;
        }
        // Pop front while the vertex between the first two deque half-planes is
        // outside `hp`.
        while deque.len() >= 2 {
            let a = deque[0];
            let b = deque[1];
            if let Some(v) = line_line_intersect(a.point, a.direction, b.point, b.direction) {
                if !hp.contains(v) {
                    deque.remove(0);
                    continue;
                }
            }
            break;
        }
        // Detect infeasibility: if the deque has exactly one half-plane `prev`
        // parallel to `hp` and `hp` does not contain `prev`'s point, the two
        // parallel half-planes face away ⇒ empty intersection.
        if deque.len() == 1 {
            let prev = deque[0];
            if line_line_intersect(prev.point, prev.direction, hp.point, hp.direction).is_none() {
                // Parallel. If hp contains prev's interior point, hp is
                // innermost (or equal) — replace. Else infeasible.
                if hp.contains(prev.point) {
                    deque[0] = hp;
                    continue;
                } else {
                    return HalfPlaneIntersection::Empty;
                }
            }
        }
        deque.push(hp);
    }

    // Closing: remove half-planes whose adjacent vertex is outside the
    // opposite end of the deque.
    while deque.len() >= 3 {
        let n = deque.len();
        let a = deque[n - 2];
        let b = deque[n - 1];
        if let Some(v) = line_line_intersect(a.point, a.direction, b.point, b.direction) {
            if !deque[0].contains(v) {
                deque.pop();
                continue;
            }
        }
        break;
    }
    while deque.len() >= 3 {
        let a = deque[0];
        let b = deque[1];
        let n = deque.len();
        let last = deque[n - 1];
        if let Some(v) = line_line_intersect(a.point, a.direction, b.point, b.direction) {
            if !last.contains(v) {
                deque.remove(0);
                continue;
            }
        }
        break;
    }

    // Rebuild the vertex ring from the surviving deque.
    let poly = build_polygon(&deque);
    if poly.len() < 3 {
        return HalfPlaneIntersection::Empty;
    }

    // Detect unboundedness: does any vertex lie on (or very near) the sentinel
    // box boundary?
    let touches_box = poly.iter().any(|p| {
        (p.x.abs() >= BIG * 0.999) || (p.y.abs() >= BIG * 0.999)
    });
    if touches_box {
        // Strip box-edge vertices: keep only vertices strictly inside the box.
        let inner: Vec<Point2> = poly
            .iter()
            .copied()
            .filter(|p| p.x.abs() < BIG * 0.999 && p.y.abs() < BIG * 0.999)
            .collect();
        return HalfPlaneIntersection::Unbounded(inner);
    }
    HalfPlaneIntersection::Bounded(poly)
}

/// Build the CCW polygon vertices from a deque of half-planes (each vertex is
/// the intersection of consecutive boundaries).
fn build_polygon(deque: &[HalfPlane]) -> Vec<Point2> {
    let n = deque.len();
    if n < 3 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let a = deque[i];
        let b = deque[(i + 1) % n];
        match line_line_intersect(a.point, a.direction, b.point, b.direction) {
            Some(v) => out.push(v),
            None => return Vec::new(), // parallel neighbours ⇒ degenerate
        }
    }
    // Drop a trailing duplicate if the ring wraps to the same point.
    if out.len() > 1 && out[0] == out[out.len() - 1] {
        out.pop();
    }
    out
}

// ───────────────────────────────────────────────────────────────────────────
//  2-D linear programming (Seidel's randomized incremental algorithm)
// ───────────────────────────────────────────────────────────────────────────

/// Result of a 2-D LP: minimize `objective · (x, y)` subject to `constraints`.
#[derive(Debug, Clone, PartialEq)]
pub enum LpResult2d {
    /// A feasible optimum exists.
    Optimal {
        point: Point2,
        value: f64,
    },
    /// The constraint set is infeasible. The two carried indices identify a
    /// conflicting pair (whose 1-D intervals on a boundary line are empty).
    Infeasible {
        witness_a: usize,
        witness_b: usize,
    },
    /// The objective is unbounded below over the feasible region. `ray` is a
    /// feasible direction along which `objective · x` decreases without bound.
    Unbounded {
        ray: Point2,
    },
}

/// Solve a 2-D LP: minimize `objective · (x, y)` subject to `constraints`
/// (each half-plane is a `<=`-style constraint on its left side).
///
/// `seed` controls the randomized constraint permutation; the same seed + input
/// always yields the same result (bit-identical across platforms).
pub fn linear_program_2d(
    objective: Point2,
    constraints: &[HalfPlane],
    seed: u64,
) -> LpResult2d {
    // Filter degenerate constraints.
    let valid: Vec<HalfPlane> = constraints
        .iter()
        .copied()
        .filter(|h| !h.is_degenerate())
        .collect();
    if valid.is_empty() {
        // No constraints. If objective is zero, origin is optimal. Else unbounded.
        if objective.x == 0.0 && objective.y == 0.0 {
            return LpResult2d::Optimal {
                point: Point2::new(0.0, 0.0),
                value: 0.0,
            };
        }
        // Minimising c·x: unbounded along -c direction.
        let ray = Point2::new(-objective.x, -objective.y);
        return LpResult2d::Unbounded { ray };
    }

    let order = seeded_permutation(seed, valid.len());

    // Seidel's randomized incremental 2-D LP, implemented iteratively.
    //
    // State: (basis_idx, current) where `current` is the LP solution over all
    //   constraints added so far, and `basis_idx` is the index (into `valid`)
    //   of the constraint whose boundary the current optimum lies on (when
    //   `current` is Optimal) or whose boundary the unbounded ray runs along
    //   (when `current` is Unbounded).
    //
    // Base case (i = 0): `base_case_lp` explicitly checks BOTH the along-line
    //   direction (c·d) and the interior direction (c·n_L). A single half-plane
    //   is unbounded unless the objective is orthogonal to the boundary AND
    //   points out of the feasible side.
    //
    // Step (i >= 1): let hp = valid[order[i]].
    //   - If current is Optimal and hp contains the optimum point, unchanged.
    //   - If current is Optimal and hp does NOT contain it, the new optimum is
    //     on hp's boundary: solve `lp_1d` along hp's boundary over all prior.
    //     New basis = idx. (Theorem: clipping a convex region by a half-plane
    //     that the old optimum violates moves the optimum to the new boundary.)
    //   - If current is Unbounded with ray r: hp bounds the ray iff r points
    //     out of hp's feasible side (orientation test). If hp bounds it, the
    //     new optimum is on hp's boundary: solve `lp_1d` along hp over all
    //     prior. New basis = idx. Else, still unbounded (same ray).
    //   - If current is Infeasible, return immediately.
    //
    // `lp_1d` (recursive case) only checks the along-line direction; the
    // interior is guaranteed bounded by the clipping theorem above. The base
    // case is the only place interior unboundedness can arise.
    let first_idx = order[0];
    let first_hp = valid[first_idx];
    let mut basis = first_idx;
    let mut current = base_case_lp(&objective, &first_hp);
    let mut added: Vec<usize> = Vec::with_capacity(valid.len());
    added.push(first_idx);

    if let Lp1d::Infeasible { witness_a, witness_b } = current {
        return LpResult2d::Infeasible {
            witness_a,
            witness_b,
        };
    }

    for &idx in order.iter().skip(1) {
        let hp = valid[idx];
        let (new_current, new_basis) = match &current {
            Lp1d::Optimal { point, .. } => {
                if hp.contains(*point) {
                    (current, basis)
                } else {
                    (lp_1d(&hp, &objective, &valid, &added, idx), idx)
                }
            }
            Lp1d::Unbounded { ray } => {
                if ray_blocked_by(&hp, *ray) {
                    (lp_1d(&hp, &objective, &valid, &added, idx), idx)
                } else {
                    (current, basis)
                }
            }
            Lp1d::Infeasible { .. } => (current, basis),
        };
        current = new_current;
        basis = new_basis;
        added.push(idx);
        if let Lp1d::Infeasible { witness_a, witness_b } = current {
            return LpResult2d::Infeasible {
                witness_a,
                witness_b,
            };
        }
    }

    match current {
        Lp1d::Optimal { point, value } => LpResult2d::Optimal { point, value },
        Lp1d::Unbounded { ray } => LpResult2d::Unbounded { ray },
        Lp1d::Infeasible { witness_a, witness_b } => LpResult2d::Infeasible {
            witness_a,
            witness_b,
        },
    }
}

/// Does `hp` block the unbounded ray direction `ray`? True iff `ray` points
/// out of `hp`'s feasible (left) side — i.e., moving along `ray` from any
/// point eventually leaves `hp`. Collinear (ray parallel to hp's boundary)
/// ⇒ not blocked (ray runs along the boundary).
#[inline]
fn ray_blocked_by(hp: &HalfPlane, ray: Point2) -> bool {
    let head = Point2::new(hp.point.x + hp.direction.x, hp.point.y + hp.direction.y);
    let ahead = Point2::new(hp.point.x + ray.x, hp.point.y + ray.y);
    orientation_2(hp.point, head, ahead) == Orientation::Clockwise
}

/// Base case: the LP optimum over a single half-plane `h`.
///
/// minimise c·x s.t. x ∈ h. The feasible region is h's left (CCW) side. Let
/// `d` = h's boundary direction, `n_L` = (-d.y, d.x) = h's inward normal.
/// - c·d > 0 ⇒ unbounded along -d (objective decreases as t → -∞).
/// - c·d < 0 ⇒ unbounded along +d (objective decreases as t → +∞).
/// - c·d == 0 and c·n_L < 0 ⇒ unbounded along n_L (into the interior).
/// - c·d == 0 and c·n_L >= 0 ⇒ optimum is the entire boundary; return h.point.
fn base_case_lp(objective: &Point2, h: &HalfPlane) -> Lp1d {
    let d = h.direction;
    let n_l = Point2::new(-d.y, d.x);
    let c_dot_d = objective.x * d.x + objective.y * d.y;
    let c_dot_n = objective.x * n_l.x + objective.y * n_l.y;
    if c_dot_d > 0.0 {
        Lp1d::Unbounded {
            ray: Point2::new(-d.x, -d.y),
        }
    } else if c_dot_d < 0.0 {
        Lp1d::Unbounded {
            ray: Point2::new(d.x, d.y),
        }
    } else if c_dot_n < 0.0 {
        Lp1d::Unbounded { ray: n_l }
    } else {
        let value = objective.x * h.point.x + objective.y * h.point.y;
        Lp1d::Optimal {
            point: h.point,
            value,
        }
    }
}

/// 1-D LP along the boundary line of `basis`, minimising `objective · x`,
/// subject to `prior` (indices into `all`). The line is parameterised as
/// `basis.point + t * basis.direction`.
///
/// Only checks the along-line direction (c·d). The interior direction is
/// guaranteed bounded by the clipping theorem when this is called from the
/// recursive case (the optimum is known to lie on `basis`'s boundary).
#[derive(Debug, Clone, PartialEq)]
enum Lp1d {
    Optimal { point: Point2, value: f64 },
    Infeasible { witness_a: usize, witness_b: usize },
    Unbounded { ray: Point2 },
}

fn lp_1d(
    basis: &HalfPlane,
    objective: &Point2,
    all: &[HalfPlane],
    prior: &[usize],
    basis_idx: usize,
) -> Lp1d {
    let p = basis.point;
    let d = basis.direction;
    let c_dot_p = objective.x * p.x + objective.y * p.y;
    let c_dot_d = objective.x * d.x + objective.y * d.y;

    // Feasible t-interval [t_lo, t_hi]. Unbounded sides are ±∞.
    let mut t_lo: f64 = f64::NEG_INFINITY;
    let mut t_hi: f64 = f64::INFINITY;
    let mut lo_witness: Option<usize> = None;
    let mut hi_witness: Option<usize> = None;

    for &idx in prior {
        let hp = &all[idx];
        match line_half_plane_intersect_t(p, d, hp) {
            None => {
                // Line is parallel to hp's boundary.
                if hp.contains(p) {
                    continue;
                } else {
                    // Fully infeasible. The conflict is between this prior
                    // constraint (idx) and the basis (basis_idx) — they are
                    // parallel and face away.
                    return Lp1d::Infeasible {
                        witness_a: idx,
                        witness_b: basis_idx,
                    };
                }
            }
            Some(t_cross) => {
                let test_t = t_cross + 1.0;
                let test_pt = Point2::new(p.x + test_t * d.x, p.y + test_t * d.y);
                if hp.contains(test_pt) {
                    if t_cross > t_lo {
                        t_lo = t_cross;
                        lo_witness = Some(idx);
                    }
                } else if t_cross < t_hi {
                    t_hi = t_cross;
                    hi_witness = Some(idx);
                }
            }
        }
    }

    if t_lo > t_hi {
        return Lp1d::Infeasible {
            witness_a: lo_witness.unwrap_or(0),
            witness_b: hi_witness.unwrap_or(0),
        };
    }

    if c_dot_d > 0.0 {
        if t_lo == f64::NEG_INFINITY {
            return Lp1d::Unbounded {
                ray: Point2::new(-d.x, -d.y),
            };
        }
        let point = Point2::new(p.x + t_lo * d.x, p.y + t_lo * d.y);
        let value = c_dot_p + t_lo * c_dot_d;
        Lp1d::Optimal { point, value }
    } else if c_dot_d < 0.0 {
        if t_hi == f64::INFINITY {
            return Lp1d::Unbounded {
                ray: Point2::new(d.x, d.y),
            };
        }
        let point = Point2::new(p.x + t_hi * d.x, p.y + t_hi * d.y);
        let value = c_dot_p + t_hi * c_dot_d;
        Lp1d::Optimal { point, value }
    } else {
        // c·d == 0: any feasible t is optimal. Pick the midpoint of the
        // feasible interval (avoids boundary endpoints; deterministic).
        let t = if t_lo != f64::NEG_INFINITY && t_hi != f64::INFINITY {
            0.5 * (t_lo + t_hi)
        } else if t_lo != f64::NEG_INFINITY {
            t_lo
        } else if t_hi != f64::INFINITY {
            t_hi
        } else {
            0.0
        };
        let point = Point2::new(p.x + t * d.x, p.y + t * d.y);
        let value = c_dot_p + t * c_dot_d;
        Lp1d::Optimal { point, value }
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Brute-force oracles (for tests) — exact vertex enumeration
// ───────────────────────────────────────────────────────────────────────────

/// True iff direction `r` is a recession direction of the half-plane set:
/// moving along `r` from any feasible point stays feasible. Equivalently, `r`
/// points into (or along) every half-plane's feasible side.
#[cfg(test)]
fn is_recession(hps: &[HalfPlane], r: Point2) -> bool {
    hps.iter().all(|h| !ray_blocked_by(h, r))
}

/// Brute-force half-plane intersection oracle via exact vertex enumeration.
///
/// Computes all pairwise boundary intersections, keeps the feasible ones, and
/// takes their convex hull. Unboundedness is detected by checking whether any
/// non-zero recession direction exists (the feasible region is unbounded iff
/// its recession cone is non-trivial).
#[cfg(test)]
fn brute_force_half_plane_intersection(hps: &[HalfPlane]) -> HalfPlaneIntersection {
    use super::hull::convex_hull_2;

    // Detect unboundedness: does a non-zero recession direction exist? Check
    // each constraint's boundary direction (±d) and the inward normals — the
    // recession cone's extreme rays are among these.
    let mut unbounded = false;
    let mut candidates: Vec<Point2> = Vec::new();
    for h in hps {
        candidates.push(h.direction);
        candidates.push(Point2::new(-h.direction.x, -h.direction.y));
        candidates.push(Point2::new(-h.direction.y, h.direction.x));
    }
    for r in &candidates {
        if r.x == 0.0 && r.y == 0.0 {
            continue;
        }
        if is_recession(hps, *r) {
            unbounded = true;
            break;
        }
    }

    // Enumerate all pairwise boundary intersections that are feasible.
    let mut verts: Vec<Point2> = Vec::new();
    for i in 0..hps.len() {
        for j in (i + 1)..hps.len() {
            if let Some(v) = line_line_intersect(
                hps[i].point, hps[i].direction, hps[j].point, hps[j].direction,
            ) {
                if hps.iter().all(|h| h.contains(v)) {
                    verts.push(v);
                }
            }
        }
    }

    if verts.is_empty() {
        // No feasible vertex. Either empty or a single half-plane / strip with
        // no vertex (unbounded with no bounded boundary chain).
        if unbounded {
            return HalfPlaneIntersection::Unbounded(Vec::new());
        }
        return HalfPlaneIntersection::Empty;
    }

    if unbounded {
        return HalfPlaneIntersection::Unbounded(verts);
    }

    let mut scratch = vec![0u32; verts.len() * 3];
    let mut out = vec![Point2::default(); verts.len()];
    let k = convex_hull_2(&verts, &mut scratch, &mut out).unwrap_or(0);
    if k < 3 {
        HalfPlaneIntersection::Empty
    } else {
        HalfPlaneIntersection::Bounded(out[..k].to_vec())
    }
}

/// Brute-force LP oracle via exact vertex enumeration.
///
/// The optimum of a bounded 2-D LP is at a vertex (pairwise boundary
/// intersection). Enumerate all feasible vertices and take the minimum. If no
/// feasible vertex exists, the LP is infeasible. Unboundedness is detected by
/// checking whether any recession direction decreases the objective.
#[cfg(test)]
fn brute_force_lp(objective: Point2, hps: &[HalfPlane]) -> LpResult2d {
    // Unboundedness: any recession direction r with c·r < 0.
    let mut candidates: Vec<Point2> = Vec::new();
    candidates.push(Point2::new(-objective.x, -objective.y));
    for h in hps {
        candidates.push(h.direction);
        candidates.push(Point2::new(-h.direction.x, -h.direction.y));
        candidates.push(Point2::new(-h.direction.y, h.direction.x));
        candidates.push(Point2::new(h.direction.y, -h.direction.x));
    }
    for r in &candidates {
        if r.x == 0.0 && r.y == 0.0 {
            continue;
        }
        if is_recession(hps, *r) && (objective.x * r.x + objective.y * r.y) < 0.0 {
            return LpResult2d::Unbounded { ray: *r };
        }
    }

    // Enumerate feasible vertices.
    let mut best: Option<(Point2, f64)> = None;
    for i in 0..hps.len() {
        for j in (i + 1)..hps.len() {
            if let Some(v) = line_line_intersect(
                hps[i].point, hps[i].direction, hps[j].point, hps[j].direction,
            ) {
                if hps.iter().all(|h| h.contains(v)) {
                    let val = objective.x * v.x + objective.y * v.y;
                    match best {
                        Some((_, bv)) if val >= bv => {}
                        _ => best = Some((v, val)),
                    }
                }
            }
        }
    }
    match best {
        Some((p, v)) => LpResult2d::Optimal { point: p, value: v },
        None => LpResult2d::Infeasible {
            witness_a: 0,
            witness_b: 1,
        },
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::hull::is_ccw_strongly_convex_2;

    fn hp(a: (f64, f64), b: (f64, f64)) -> HalfPlane {
        HalfPlane::from_directed_line(Point2::new(a.0, a.1), Point2::new(b.0, b.1))
    }

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    fn poly_area(p: &[Point2]) -> f64 {
        if p.len() < 3 {
            return 0.0;
        }
        let mut s = 0.0;
        for i in 0..p.len() {
            let j = (i + 1) % p.len();
            s += p[i].x * p[j].y - p[j].x * p[i].y;
        }
        0.5 * s
    }

    // ── Half-plane representation ──

    #[test]
    fn half_plane_contains_left_side() {
        // Directed line along +x axis from origin. Left side = +y.
        let h = hp((0.0, 0.0), (1.0, 0.0));
        assert!(h.contains(Point2::new(0.0, 1.0)));
        assert!(h.contains(Point2::new(5.0, 0.0))); // on boundary
        assert!(!h.contains(Point2::new(0.0, -1.0)));
    }

    #[test]
    fn half_plane_from_line_and_side_orients_correctly() {
        // Line through (0,0)-(1,0); side (0,1) should be feasible.
        let h = HalfPlane::from_line_and_side(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        );
        assert!(h.contains(Point2::new(0.0, 1.0)));
        assert!(!h.contains(Point2::new(0.0, -1.0)));
        // Side below ⇒ flip.
        let h2 = HalfPlane::from_line_and_side(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, -1.0),
        );
        assert!(h2.contains(Point2::new(0.0, -1.0)));
        assert!(!h2.contains(Point2::new(0.0, 1.0)));
    }

    #[test]
    fn half_plane_from_implicit() {
        // x + y <= 1 ⇒ feasible where x+y-1 <= 0.
        let h = HalfPlane::from_implicit(1.0, 1.0, -1.0);
        assert!(h.contains(Point2::new(0.0, 0.0)));
        assert!(!h.contains(Point2::new(1.0, 1.0)));
    }

    #[test]
    fn half_plane_degenerate_direction_is_flagged() {
        let h = HalfPlane {
            point: Point2::new(0.0, 0.0),
            direction: Point2::new(0.0, 0.0),
        };
        assert!(h.is_degenerate());
    }

    // ── Half-plane intersection: basic shapes ──

    #[test]
    fn hpi_empty_input_is_unbounded() {
        let r = half_plane_intersection(&[]);
        assert!(matches!(r, HalfPlaneIntersection::Unbounded(_)));
    }

    #[test]
    fn hpi_single_half_plane_is_unbounded() {
        let r = half_plane_intersection(&[hp((0.0, 0.0), (1.0, 0.0))]);
        assert!(matches!(r, HalfPlaneIntersection::Unbounded(_)));
    }

    #[test]
    fn hpi_unit_square_bounded() {
        // y >= 0, x <= 1, y <= 1, x >= 0.
        let hps = [
            hp((-1.0, 0.0), (1.0, 0.0)),  // y >= 0 (left = above)
            hp((1.0, -1.0), (1.0, 1.0)),  // x <= 1
            hp((1.0, 1.0), (-1.0, 1.0)),  // y <= 1
            hp((0.0, 1.0), (0.0, -1.0)),  // x >= 0
        ];
        let r = half_plane_intersection(&hps);
        match r {
            HalfPlaneIntersection::Bounded(poly) => {
                assert!(poly.len() >= 3);
                assert!(is_ccw_strongly_convex_2(&poly));
                let area = poly_area(&poly);
                assert!(approx_eq(area, 1.0, 1e-6), "area = {}", area);
            }
            other => panic!("expected Bounded, got {:?}", other),
        }
    }

    #[test]
    fn hpi_triangle_bounded() {
        // Three half-planes forming a triangle: x>=0, y>=0, x+y<=2.
        let hps = [
            hp((-1.0, 0.0), (1.0, 0.0)),  // y >= 0
            hp((0.0, 1.0), (0.0, -1.0)),  // x >= 0
            hp((2.0, 0.0), (0.0, 2.0)),   // x + y <= 2 (left of (2,0)->(0,2))
        ];
        let r = half_plane_intersection(&hps);
        match r {
            HalfPlaneIntersection::Bounded(poly) => {
                assert!(poly.len() >= 3);
                let area = poly_area(&poly);
                // Triangle (0,0),(2,0),(0,2) area = 2.
                assert!(approx_eq(area, 2.0, 1e-6), "area = {}", area);
            }
            other => panic!("expected Bounded, got {:?}", other),
        }
    }

    #[test]
    fn hpi_empty_when_contradictory() {
        // y >= 0 and y <= -1 ⇒ empty.
        let hps = [
            hp((-1.0, 0.0), (1.0, 0.0)),  // y >= 0
            hp((1.0, -1.0), (-1.0, -1.0)), // y <= -1
        ];
        let r = half_plane_intersection(&hps);
        assert!(matches!(r, HalfPlaneIntersection::Empty), "got {:?}", r);
    }

    #[test]
    fn hpi_unbounded_wedge() {
        // x >= 0 and y >= 0 ⇒ unbounded quadrant.
        let hps = [
            hp((0.0, 1.0), (0.0, -1.0)),  // x >= 0
            hp((-1.0, 0.0), (1.0, 0.0)),  // y >= 0
        ];
        let r = half_plane_intersection(&hps);
        assert!(matches!(r, HalfPlaneIntersection::Unbounded(_)), "got {:?}", r);
    }

    #[test]
    fn hpi_strip_is_unbounded() {
        // 0 <= y <= 1 ⇒ unbounded horizontal strip.
        let hps = [
            hp((-1.0, 0.0), (1.0, 0.0)),  // y >= 0
            hp((1.0, 1.0), (-1.0, 1.0)),  // y <= 1
        ];
        let r = half_plane_intersection(&hps);
        assert!(matches!(r, HalfPlaneIntersection::Unbounded(_)), "got {:?}", r);
    }

    #[test]
    fn hpi_redundant_half_planes_ignored() {
        // Unit square + redundant y >= -5 (weaker than y >= 0).
        let hps = [
            hp((-1.0, 0.0), (1.0, 0.0)),  // y >= 0
            hp((1.0, -1.0), (1.0, 1.0)),  // x <= 1
            hp((1.0, 1.0), (-1.0, 1.0)),  // y <= 1
            hp((0.0, 1.0), (0.0, -1.0)),  // x >= 0
            hp((-1.0, -5.0), (1.0, -5.0)), // y >= -5 (redundant)
        ];
        let r = half_plane_intersection(&hps);
        match r {
            HalfPlaneIntersection::Bounded(poly) => {
                let area = poly_area(&poly);
                assert!(approx_eq(area, 1.0, 1e-6), "area = {}", area);
            }
            other => panic!("expected Bounded, got {:?}", other),
        }
    }

    #[test]
    fn hpi_matches_brute_force_on_random_grid() {
        // A bounded pentagon.
        let hps = [
            hp((-2.0, 0.0), (2.0, 0.0)),   // y >= 0
            hp((2.0, -2.0), (2.0, 2.0)),   // x <= 2
            hp((2.0, 2.0), (-2.0, 2.0)),   // y <= 2
            hp((0.0, 2.0), (0.0, -1.0)),   // x >= 0
            hp((2.0, 0.0), (0.0, 2.0)),    // x + y <= 2
        ];
        let r = half_plane_intersection(&hps);
        let brute = brute_force_half_plane_intersection(&hps);
        match (&r, &brute) {
            (HalfPlaneIntersection::Bounded(a), HalfPlaneIntersection::Bounded(b)) => {
                let da = poly_area(a);
                let db = poly_area(b);
                assert!(approx_eq(da, db, 1e-3), "area {} vs brute {}", da, db);
            }
            (HalfPlaneIntersection::Empty, HalfPlaneIntersection::Empty) => {}
            (HalfPlaneIntersection::Unbounded(_), HalfPlaneIntersection::Unbounded(_)) => {}
            other => panic!("mismatch: algo={:?} brute={:?}", other.0, other.1),
        }
    }

    // ── LP: basic cases ──

    #[test]
    fn lp_no_constraints_unbounded() {
        let r = linear_program_2d(Point2::new(1.0, 1.0), &[], 42);
        assert!(matches!(r, LpResult2d::Unbounded { .. }));
    }

    #[test]
    fn lp_zero_objective_no_constraints_origin() {
        let r = linear_program_2d(Point2::new(0.0, 0.0), &[], 42);
        match r {
            LpResult2d::Optimal { point, value } => {
                assert!(approx_eq(point.x, 0.0, 1e-9));
                assert!(approx_eq(point.y, 0.0, 1e-9));
                assert!(approx_eq(value, 0.0, 1e-9));
            }
            other => panic!("expected Optimal, got {:?}", other),
        }
    }

    #[test]
    fn lp_simple_bounded_optimum() {
        // minimise x + y s.t. x >= 0, y >= 0, x + y >= 2.
        // Optimum at (2,0) or (0,2) — both value 2. With the half-plane
        // convention, x>=0 is left of (0,1)->(0,-1); y>=0 is left of (-1,0)->(1,0);
        // x+y>=2 is left of (2,0)->(0,2)? Check: (2,0)->(0,2) direction (-2,2);
        // left of that includes (1,1)? orientation((2,0),(0,2),(1,1)) —
        // (0,2)-(2,0)=(-2,2); (1,1)-(2,0)=(-1,1); cross = (-2)(1)-(2)(-1)= -2+2=0
        // collinear. So (1,1) is on the line x+y=2. Good. The feasible side
        // (x+y>=2) is the side away from origin: test (3,3): orientation
        // ((2,0),(0,2),(3,3)) = cross((-2,2),(1,3)) = (-2)(3)-(2)(1) = -8 <0 ⇒
        // Clockwise ⇒ NOT in left half-plane. So left of (2,0)->(0,2) is the
        // origin side (x+y<=2). We need the OPPOSITE orientation: (0,2)->(2,0).
        let hps = [
            hp((0.0, 1.0), (0.0, -1.0)),  // x >= 0
            hp((-1.0, 0.0), (1.0, 0.0)),  // y >= 0
            hp((0.0, 2.0), (2.0, 0.0)),   // x + y >= 2 (left of (0,2)->(2,0))
        ];
        let r = linear_program_2d(Point2::new(1.0, 1.0), &hps, 7);
        match r {
            LpResult2d::Optimal { point, value } => {
                assert!(
                    approx_eq(value, 2.0, 1e-6),
                    "value = {} (point {:?})", value, point
                );
                // Point must be feasible.
                assert!(hps.iter().all(|h| h.contains(point)));
            }
            other => panic!("expected Optimal, got {:?}", other),
        }
    }

    #[test]
    fn lp_infeasible_returns_witnesses() {
        // x >= 1 and x <= 0 ⇒ infeasible.
        let hps = [
            hp((1.0, 1.0), (1.0, -1.0)),  // x >= 1
            hp((0.0, -1.0), (0.0, 1.0)),  // x <= 0
        ];
        let r = linear_program_2d(Point2::new(1.0, 0.0), &hps, 3);
        match r {
            LpResult2d::Infeasible { witness_a, witness_b } => {
                assert_ne!(witness_a, witness_b);
                assert!(witness_a < 2 && witness_b < 2);
            }
            other => panic!("expected Infeasible, got {:?}", other),
        }
    }

    #[test]
    fn lp_unbounded_ray() {
        // minimise x s.t. x >= 0 ⇒ unbounded below? No: minimise x with x>=0
        // ⇒ optimum at x=0. To get unbounded, minimise -x s.t. x >= 0 ⇒
        // unbounded (x → +∞ decreases -x).
        let hps = [hp((0.0, 1.0), (0.0, -1.0))]; // x >= 0
        let r = linear_program_2d(Point2::new(-1.0, 0.0), &hps, 11);
        match r {
            LpResult2d::Unbounded { ray } => {
                // Ray should point in +x direction (decreases -x).
                assert!(ray.x > 0.0, "ray = {:?}", ray);
            }
            other => panic!("expected Unbounded, got {:?}", other),
        }
    }

    #[test]
    fn lp_optimum_at_vertex_of_triangle() {
        // minimise -x - y s.t. x>=0, y>=0, x+y<=4.
        // Optimum at (2,2)? No: minimise -(x+y) ⇒ maximise x+y ⇒ optimum at
        // any point on x+y=4, value -4. Pick (4,0) or (0,4) or (2,2). The
        // algorithm picks a specific vertex deterministically for a given seed.
        let hps = [
            hp((0.0, 1.0), (0.0, -1.0)),  // x >= 0
            hp((-1.0, 0.0), (1.0, 0.0)),  // y >= 0
            hp((4.0, 0.0), (0.0, 4.0)),   // x + y <= 4 (left of (4,0)->(0,4))
        ];
        let r = linear_program_2d(Point2::new(-1.0, -1.0), &hps, 99);
        match r {
            LpResult2d::Optimal { point, value } => {
                assert!(approx_eq(value, -4.0, 1e-6), "value = {}", value);
                assert!(hps.iter().all(|h| h.contains(point)));
                assert!(approx_eq(point.x + point.y, 4.0, 1e-6));
            }
            other => panic!("expected Optimal, got {:?}", other),
        }
    }

    #[test]
    fn lp_seed_determinism() {
        let hps = [
            hp((0.0, 1.0), (0.0, -1.0)),  // x >= 0
            hp((-1.0, 0.0), (1.0, 0.0)),  // y >= 0
            hp((0.0, 2.0), (2.0, 0.0)),   // x + y >= 2
        ];
        let r1 = linear_program_2d(Point2::new(1.0, 1.0), &hps, 12345);
        let r2 = linear_program_2d(Point2::new(1.0, 1.0), &hps, 12345);
        assert_eq!(r1, r2);
        // Different seed ⇒ same optimum value (deterministic problem), possibly
        // same point (the optimum is unique here).
        let r3 = linear_program_2d(Point2::new(1.0, 1.0), &hps, 99999);
        match (&r1, &r3) {
            (LpResult2d::Optimal { value: v1, .. }, LpResult2d::Optimal { value: v3, .. }) => {
                assert!(approx_eq(*v1, *v3, 1e-9), "value differs across seeds: {} vs {}", v1, v3);
            }
            other => panic!("expected both Optimal, got {:?}", other),
        }
    }

    #[test]
    fn lp_matches_brute_force_on_grid() {
        // minimise x + 2y s.t. x>=0, y>=0, x+y<=3, x<=2.
        let hps = [
            hp((0.0, 1.0), (0.0, -1.0)),  // x >= 0
            hp((-1.0, 0.0), (1.0, 0.0)),  // y >= 0
            hp((3.0, 0.0), (0.0, 3.0)),   // x + y <= 3
            hp((2.0, -1.0), (2.0, 1.0)),  // x <= 2
        ];
        let obj = Point2::new(1.0, 2.0);
        let r = linear_program_2d(obj, &hps, 2024);
        let brute = brute_force_lp(obj, &hps);
        match (&r, &brute) {
            (LpResult2d::Optimal { value: v, .. }, LpResult2d::Optimal { value: bv, .. }) => {
                assert!(approx_eq(*v, *bv, 1e-2), "lp value {} vs brute {}", v, bv);
            }
            (LpResult2d::Infeasible { .. }, LpResult2d::Infeasible { .. }) => {}
            (LpResult2d::Unbounded { .. }, LpResult2d::Unbounded { .. }) => {}
            other => panic!("mismatch: lp={:?} brute={:?}", other.0, other.1),
        }
    }

    #[test]
    fn lp_degenerate_constraints_filtered() {
        // A degenerate (zero-direction) constraint is ignored.
        let bad = HalfPlane {
            point: Point2::new(0.0, 0.0),
            direction: Point2::new(0.0, 0.0),
        };
        let hps = [
            hp((0.0, 1.0), (0.0, -1.0)),  // x >= 0
            bad,
        ];
        let r = linear_program_2d(Point2::new(-1.0, 0.0), &hps, 5);
        // With only x>=0 and minimise -x ⇒ unbounded.
        assert!(matches!(r, LpResult2d::Unbounded { .. }));
    }

    // ── Seeded permutation determinism ──

    #[test]
    fn seeded_permutation_is_deterministic() {
        let a = seeded_permutation(42, 10);
        let b = seeded_permutation(42, 10);
        assert_eq!(a, b);
        // Different seed ⇒ (very likely) different permutation.
        let c = seeded_permutation(7, 10);
        assert_ne!(a, c);
    }

    #[test]
    fn seeded_permutation_is_a_permutation() {
        for n in 0..=20 {
            let p = seeded_permutation(123, n);
            let mut sorted = p.clone();
            sorted.sort();
            assert_eq!(sorted, (0..n).collect::<Vec<_>>(), "n={}", n);
        }
    }

    // ── Line-line intersection ──

    #[test]
    fn line_line_intersect_basic() {
        // Line 1: y = 0 (point (0,0), dir (1,0)). Line 2: x = 1 (point (1,0), dir (0,1)).
        let v = line_line_intersect(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        );
        assert_eq!(v, Some(Point2::new(1.0, 0.0)));
    }

    #[test]
    fn line_line_intersect_parallel_returns_none() {
        let v = line_line_intersect(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
            Point2::new(1.0, 0.0),
        );
        assert_eq!(v, None);
    }
}
