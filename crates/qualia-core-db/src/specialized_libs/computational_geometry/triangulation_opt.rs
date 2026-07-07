//! P13.3 - Optimal fixed-vertex triangulation objectives.
//!
//! Given a **fixed** set of vertices (no Steiner points), find the
//! triangulation that optimises a declared quality objective via local edge
//! flips. This is the Delaunay-free counterpart to P13.2: when you cannot add
//! points, you can still improve the mesh by flipping diagonals of
//! convex quadrilaterals.
//!
//! ## Supported objectives
//!
//! - [`TriObjective::MaxMinAngle`] — maximise the minimum interior angle
//!   across all triangles (the classic "max-min angle" criterion; the Delaunay
//!   triangulation is the global optimum for this objective).
//! - [`TriObjective::MinMaxAngle`] — minimise the maximum interior angle
//!   (reduces large obtuse angles; useful for FEM stiffness conditioning).
//! - [`TriObjective::MinMaxEdgeRatio`] — minimise the maximum edge ratio
//!   (longest/shortest edge).
//! - [`TriObjective::MinMaxRadiusEdge`] — minimise the maximum radius-edge
//!   ratio (circumradius/shortest edge).
//! - [`TriObjective::MaxMinArea`] — maximise the minimum triangle area
//!   (avoids sliver-like small triangles).
//! - [`TriObjective::MinMaxAspect`] — minimise the maximum aspect ratio
//!   (circumradius / (2 * inradius)).
//!
//! ## Algorithm
//!
//! Hill-climbing via edge flips: repeatedly find the edge flip that most
//! improves the objective, apply it, and continue until no flip improves the
//! objective (local optimum). An edge flip is valid only if the two triangles
//! sharing the edge form a **convex quadrilateral** (the flip would not create
//! an inverted triangle).
//!
//! For `MaxMinAngle` the local optimum is the Delaunay triangulation (this is
//! the defining property of Delaunay). For other objectives the local optimum
//! is not guaranteed to be the global optimum, but hill-climbing from a
//! Delaunay start produces a high-quality mesh in practice.
//!
//! ## Determinism
//!
//! Edge-flip candidates are evaluated in deterministic order (sorted by edge
//! key). Ties in objective improvement are broken by the canonical edge key.
//! Identical input -> bit-identical output.

use super::primitives::{orientation_2, Orientation, Point2};

// ---------------------------------------------------------------------------
//  Errors
// ---------------------------------------------------------------------------

/// Triangulation-optimisation error.
#[derive(Debug, Clone, PartialEq)]
pub enum TriangulationOptError {
    /// Fewer than 3 input points.
    TooFewPoints { got: usize },
    /// The initial triangulation had fewer than 1 triangle.
    EmptyTriangulation,
    /// An edge referenced an out-of-range vertex.
    InvalidEdge { a: u32, b: u32, point_count: usize },
    /// The triangulation is inconsistent (an edge has no matching triangle pair
    /// or a triangle references a non-existent vertex).
    InconsistentTriangulation { detail: String },
    /// The flip iteration limit was reached without convergence.
    IterationLimitReached { iterations: usize },
}

impl core::fmt::Display for TriangulationOptError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooFewPoints { got } => {
                write!(f, "tri_opt: need >= 3 points, got {got}")
            }
            Self::EmptyTriangulation => write!(f, "tri_opt: empty triangulation"),
            Self::InvalidEdge { a, b, point_count } => {
                write!(f, "tri_opt: edge ({a},{b}) >= {point_count} points")
            }
            Self::InconsistentTriangulation { detail } => {
                write!(f, "tri_opt: inconsistent triangulation: {detail}")
            }
            Self::IterationLimitReached { iterations } => {
                write!(f, "tri_opt: iteration limit reached ({iterations})")
            }
        }
    }
}

impl std::error::Error for TriangulationOptError {}

// ---------------------------------------------------------------------------
//  Objective
// ---------------------------------------------------------------------------

/// Triangulation quality objective to optimise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriObjective {
    /// Maximise the minimum interior angle across all triangles.
    /// (Delaunay is the global optimum for this objective.)
    MaxMinAngle,
    /// Minimise the maximum interior angle across all triangles.
    MinMaxAngle,
    /// Minimise the maximum edge ratio (longest/shortest edge) across all
    /// triangles.
    MinMaxEdgeRatio,
    /// Minimise the maximum radius-edge ratio (circumradius/shortest edge).
    MinMaxRadiusEdge,
    /// Maximise the minimum triangle area.
    MaxMinArea,
    /// Minimise the maximum aspect ratio (circumradius / (2 * inradius)).
    MinMaxAspect,
}

impl TriObjective {
    /// Returns `true` if this is a maximisation objective.
    #[inline]
    pub fn is_maximise(&self) -> bool {
        matches!(self, Self::MaxMinAngle | Self::MaxMinArea)
    }
}

// ---------------------------------------------------------------------------
//  Geometry helpers
// ---------------------------------------------------------------------------

#[inline]
fn edge_len_sq(a: Point2, b: Point2) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

/// Interior angle at vertex `v` between edges to `p` and `q` (radians).
#[inline]
fn angle_at(v: Point2, p: Point2, q: Point2) -> f64 {
    let ux = p.x - v.x;
    let uy = p.y - v.y;
    let vx = q.x - v.x;
    let vy = q.y - v.y;
    let nu = (ux * ux + uy * uy).sqrt();
    let nv = (vx * vx + vy * vy).sqrt();
    if nu == 0.0 || nv == 0.0 {
        return 0.0;
    }
    let cos = ((ux * vx + uy * vy) / (nu * nv)).clamp(-1.0, 1.0);
    cos.acos()
}

/// Minimum interior angle of a triangle (radians).
fn tri_min_angle(a: Point2, b: Point2, c: Point2) -> f64 {
    angle_at(a, b, c)
        .min(angle_at(b, a, c))
        .min(angle_at(c, a, b))
}

/// Maximum interior angle of a triangle (radians).
fn tri_max_angle(a: Point2, b: Point2, c: Point2) -> f64 {
    angle_at(a, b, c)
        .max(angle_at(b, a, c))
        .max(angle_at(c, a, b))
}

/// Unsigned area of a triangle.
#[inline]
fn tri_area(a: Point2, b: Point2, c: Point2) -> f64 {
    0.5 * ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs()
}

/// Edge ratio = longest_edge / shortest_edge (>= 1).
fn tri_edge_ratio(a: Point2, b: Point2, c: Point2) -> f64 {
    let l0 = edge_len_sq(a, b).sqrt();
    let l1 = edge_len_sq(b, c).sqrt();
    let l2 = edge_len_sq(c, a).sqrt();
    let mn = l0.min(l1).min(l2);
    let mx = l0.max(l1).max(l2);
    if mn > 0.0 {
        mx / mn
    } else {
        f64::INFINITY
    }
}

/// Circumradius of a triangle.
fn circumradius(a: Point2, b: Point2, c: Point2) -> f64 {
    let l0 = edge_len_sq(a, b).sqrt();
    let l1 = edge_len_sq(b, c).sqrt();
    let l2 = edge_len_sq(c, a).sqrt();
    let area = tri_area(a, b, c);
    if area <= 0.0 {
        f64::INFINITY
    } else {
        (l0 * l1 * l2) / (4.0 * area)
    }
}

/// Radius-edge ratio = circumradius / shortest_edge.
fn tri_radius_edge(a: Point2, b: Point2, c: Point2) -> f64 {
    let l0 = edge_len_sq(a, b).sqrt();
    let l1 = edge_len_sq(b, c).sqrt();
    let l2 = edge_len_sq(c, a).sqrt();
    let mn = l0.min(l1).min(l2);
    let r = circumradius(a, b, c);
    if mn > 0.0 {
        r / mn
    } else {
        f64::INFINITY
    }
}

/// Inradius of a triangle.
fn inradius(a: Point2, b: Point2, c: Point2) -> f64 {
    let l0 = edge_len_sq(a, b).sqrt();
    let l1 = edge_len_sq(b, c).sqrt();
    let l2 = edge_len_sq(c, a).sqrt();
    let area = tri_area(a, b, c);
    let s = 0.5 * (l0 + l1 + l2);
    if s > 0.0 {
        area / s
    } else {
        0.0
    }
}

/// Aspect ratio = circumradius / (2 * inradius) (>= 1; 1 = equilateral).
fn tri_aspect(a: Point2, b: Point2, c: Point2) -> f64 {
    let r = inradius(a, b, c);
    let rc = circumradius(a, b, c);
    if r > 0.0 {
        rc / (2.0 * r)
    } else {
        f64::INFINITY
    }
}

// ---------------------------------------------------------------------------
//  Objective evaluation
// ---------------------------------------------------------------------------

/// Evaluate the global objective value for a triangulation.
///
/// For maximisation objectives (MaxMinAngle, MaxMinArea) this returns the
/// minimum across all triangles. For minimisation objectives (MinMax*) this
/// returns the maximum across all triangles.
pub fn evaluate_objective(
    points: &[Point2],
    triangles: &[[u32; 3]],
    objective: TriObjective,
) -> f64 {
    let mut result = if objective.is_maximise() {
        f64::INFINITY
    } else {
        0.0f64
    };
    for &tri in triangles {
        let [a, b, c] = tri;
        let pa = points[a as usize];
        let pb = points[b as usize];
        let pc = points[c as usize];
        let val = match objective {
            TriObjective::MaxMinAngle => tri_min_angle(pa, pb, pc),
            TriObjective::MinMaxAngle => tri_max_angle(pa, pb, pc),
            TriObjective::MinMaxEdgeRatio => tri_edge_ratio(pa, pb, pc),
            TriObjective::MinMaxRadiusEdge => tri_radius_edge(pa, pb, pc),
            TriObjective::MaxMinArea => tri_area(pa, pb, pc),
            TriObjective::MinMaxAspect => tri_aspect(pa, pb, pc),
        };
        if objective.is_maximise() {
            if val < result {
                result = val;
            }
        } else if val > result {
            result = val;
        }
    }
    if triangles.is_empty() {
        0.0
    } else {
        result
    }
}

/// Evaluate the objective for a single triangle.
#[inline]
fn tri_objective(a: Point2, b: Point2, c: Point2, objective: TriObjective) -> f64 {
    match objective {
        TriObjective::MaxMinAngle => tri_min_angle(a, b, c),
        TriObjective::MinMaxAngle => tri_max_angle(a, b, c),
        TriObjective::MinMaxEdgeRatio => tri_edge_ratio(a, b, c),
        TriObjective::MinMaxRadiusEdge => tri_radius_edge(a, b, c),
        TriObjective::MaxMinArea => tri_area(a, b, c),
        TriObjective::MinMaxAspect => tri_aspect(a, b, c),
    }
}

// ---------------------------------------------------------------------------
//  Edge flip
// ---------------------------------------------------------------------------

/// Find the two triangles sharing edge (a, b) and the two opposite vertices.
///
/// Returns `Some((t0, t1, c, d))` where `triangles[t0]` has vertices (a, b, c)
/// and `triangles[t1]` has vertices (a, b, d), with c != d. Returns `None` if
/// the edge is on the boundary (only one triangle) or is not present.
fn find_edge_pair(triangles: &[[u32; 3]], a: u32, b: u32) -> Option<(usize, usize, u32, u32)> {
    let mut t0: Option<(usize, u32)> = None;
    let mut t1: Option<(usize, u32)> = None;
    for (ti, tri) in triangles.iter().enumerate() {
        let [x, y, z] = *tri;
        let has_a = x == a || y == a || z == a;
        let has_b = x == b || y == b || z == b;
        if has_a && has_b {
            // Find the opposite vertex (the one that is neither a nor b).
            let opp = if x != a && x != b {
                x
            } else if y != a && y != b {
                y
            } else {
                z
            };
            if t0.is_none() {
                t0 = Some((ti, opp));
            } else if t1.is_none() {
                t1 = Some((ti, opp));
                break;
            }
        }
    }
    match (t0, t1) {
        (Some((ti0, c)), Some((ti1, d))) if c != d => Some((ti0, ti1, c, d)),
        _ => None,
    }
}

/// Check if flipping edge (a, b) to edge (c, d) is valid: the quadrilateral
/// (a, c, b, d) must be convex (the diagonal flip does not create an inverted
/// triangle).
///
/// The quad (a, c, b, d) is convex iff all four corner turns have the same
/// orientation (all CCW or all CW). This correctly rejects cases where one
/// vertex is inside the triangle of the other three (which would pass a
/// naive "opposite sides of both diagonals" test).
fn flip_is_valid(points: &[Point2], a: u32, b: u32, c: u32, d: u32) -> bool {
    let pa = points[a as usize];
    let pb = points[b as usize];
    let pc = points[c as usize];
    let pd = points[d as usize];
    // Quad order: a, c, b, d. Check orientation at each consecutive triple.
    let o_acb = orientation_2(pa, pc, pb); // turn at c: a->c->b
    let o_cbd = orientation_2(pc, pb, pd); // turn at b: c->b->d
    let o_bda = orientation_2(pb, pd, pa); // turn at d: b->d->a
    let o_dac = orientation_2(pd, pa, pc); // turn at a: d->a->c
    if o_acb == Orientation::Collinear
        || o_cbd == Orientation::Collinear
        || o_bda == Orientation::Collinear
        || o_dac == Orientation::Collinear
    {
        return false;
    }
    // All four must have the same orientation (all CCW or all CW).
    o_acb == o_cbd && o_cbd == o_bda && o_bda == o_dac
}

/// Compute the objective contribution of the two triangles sharing edge (a, b)
/// with opposite vertices c and d.
///
/// For maximisation: returns the minimum of the two triangle objectives.
/// For minimisation: returns the maximum of the two triangle objectives.
fn pair_objective(
    points: &[Point2],
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    objective: TriObjective,
) -> f64 {
    let pa = points[a as usize];
    let pb = points[b as usize];
    let pc = points[c as usize];
    let pd = points[d as usize];
    let v0 = tri_objective(pa, pb, pc, objective);
    let v1 = tri_objective(pa, pb, pd, objective);
    if objective.is_maximise() {
        v0.min(v1)
    } else {
        v0.max(v1)
    }
}

/// Compute the objective contribution after flipping edge (a, b) to (c, d):
/// the two new triangles are (a, c, d) and (b, c, d).
fn flipped_pair_objective(
    points: &[Point2],
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    objective: TriObjective,
) -> f64 {
    let pa = points[a as usize];
    let pb = points[b as usize];
    let pc = points[c as usize];
    let pd = points[d as usize];
    let v0 = tri_objective(pa, pc, pd, objective);
    let v1 = tri_objective(pb, pc, pd, objective);
    if objective.is_maximise() {
        v0.min(v1)
    } else {
        v0.max(v1)
    }
}

/// Apply an edge flip: replace edge (a, b) with edge (c, d) in the two
/// triangles that share (a, b). Modifies `triangles` in place.
fn apply_flip(triangles: &mut [[u32; 3]], a: u32, b: u32, c: u32, d: u32) {
    // Find the two triangles and rewrite them.
    let mut flipped = 0;
    for tri in triangles.iter_mut() {
        let [x, y, z] = *tri;
        let has_a = x == a || y == a || z == a;
        let has_b = x == b || y == b || z == b;
        if has_a && has_b {
            // The opposite vertex determines which new triangle this becomes.
            let opp = if x != a && x != b {
                x
            } else if y != a && y != b {
                y
            } else {
                z
            };
            if opp == c {
                // New triangle: (b, c, d) — replace a with d.
                *tri = replace_vertex(*tri, a, d);
                // Ensure CCW orientation preserved.
                ensure_ccw(tri);
                flipped += 1;
            } else if opp == d {
                // New triangle: (a, c, d) — replace b with c.
                *tri = replace_vertex(*tri, b, c);
                ensure_ccw(tri);
                flipped += 1;
            }
        }
        if flipped == 2 {
            break;
        }
    }
}

/// Replace vertex `old` with `new` in a triangle.
#[inline]
fn replace_vertex(tri: [u32; 3], old: u32, new: u32) -> [u32; 3] {
    [
        if tri[0] == old { new } else { tri[0] },
        if tri[1] == old { new } else { tri[1] },
        if tri[2] == old { new } else { tri[2] },
    ]
}

/// Ensure a triangle is CCW; if not, swap two vertices.
fn ensure_ccw(tri: &mut [u32; 3]) {
    // We cannot check orientation without points here; the caller's flip logic
    // already preserves orientation by construction (replacing the correct
    // vertex in the correct triangle). This is a no-op placeholder for safety.
    let _ = tri;
}

// ---------------------------------------------------------------------------
//  Optimisation
// ---------------------------------------------------------------------------

/// Optimise a triangulation by hill-climbing via edge flips.
///
/// Given a fixed vertex set (`points`) and an initial triangulation
/// (`triangles`), repeatedly find and apply the edge flip that most improves
/// the declared `objective`. Stops when no flip improves the objective (local
/// optimum) or `max_iterations` is reached.
///
/// Returns the number of flips applied. The `triangles` slice is modified in
/// place.
pub fn optimise_triangulation(
    points: &[Point2],
    triangles: &mut [[u32; 3]],
    objective: TriObjective,
    max_iterations: usize,
) -> Result<usize, TriangulationOptError> {
    let n = points.len();
    if n < 3 {
        return Err(TriangulationOptError::TooFewPoints { got: n });
    }
    if triangles.is_empty() {
        return Err(TriangulationOptError::EmptyTriangulation);
    }
    // Validate triangle vertex indices.
    for (ti, tri) in triangles.iter().enumerate() {
        for &vi in tri {
            if (vi as usize) >= n {
                return Err(TriangulationOptError::InconsistentTriangulation {
                    detail: format!("triangle {ti} references vertex {vi} >= {n}"),
                });
            }
        }
    }

    let mut flips = 0usize;
    for _iter in 0..max_iterations {
        // Collect all interior edges (shared by exactly 2 triangles).
        let mut edges: Vec<(u32, u32)> = Vec::new();
        for tri in triangles.iter() {
            let [a, b, c] = *tri;
            for &(u, v) in &[
                (a.min(b), a.max(b)),
                (b.min(c), b.max(c)),
                (c.min(a), c.max(a)),
            ] {
                // Check if this edge is interior (shared by 2 triangles).
                if let Some((_, _, _, _)) = find_edge_pair(triangles, u, v) {
                    // Interior edge — add if not already present.
                    let key = (u.min(v), u.max(v));
                    if !edges.contains(&key) {
                        edges.push(key);
                    }
                }
            }
        }
        if edges.is_empty() {
            break;
        }
        // Sort edges deterministically.
        edges.sort_unstable();

        // Find the best flip.
        let mut best_flip: Option<(u32, u32, u32, u32, f64)> = None;
        for &(a, b) in &edges {
            let (ti0, ti1, c, d) = match find_edge_pair(triangles, a, b) {
                Some(p) => p,
                None => continue,
            };
            let _ = (ti0, ti1);
            if !flip_is_valid(points, a, b, c, d) {
                continue;
            }
            let current = pair_objective(points, a, b, c, d, objective);
            let flipped = flipped_pair_objective(points, a, b, c, d, objective);
            // Improvement: for maximise, flipped > current; for minimise,
            // flipped < current.
            let improves = if objective.is_maximise() {
                flipped > current
            } else {
                flipped < current
            };
            if !improves {
                continue;
            }
            let improvement = if objective.is_maximise() {
                flipped - current
            } else {
                current - flipped
            };
            match best_flip {
                None => best_flip = Some((a, b, c, d, improvement)),
                Some((_, _, _, _, best_imp)) if improvement > best_imp => {
                    best_flip = Some((a, b, c, d, improvement));
                }
                _ => {}
            }
        }
        match best_flip {
            Some((a, b, c, d, _)) => {
                apply_flip(triangles, a, b, c, d);
                flips += 1;
            }
            None => break, // local optimum
        }
    }
    Ok(flips)
}

/// Optimise and also return the final objective value.
pub fn optimise_and_evaluate(
    points: &[Point2],
    triangles: &mut [[u32; 3]],
    objective: TriObjective,
    max_iterations: usize,
) -> Result<(usize, f64), TriangulationOptError> {
    let flips = optimise_triangulation(points, triangles, objective, max_iterations)?;
    let val = evaluate_objective(points, triangles, objective);
    Ok((flips, val))
}

// ---------------------------------------------------------------------------
//  Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::FRAC_PI_4;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    /// A convex quadrilateral where the "bad" diagonal (1,3) produces a
    /// very thin triangle (min angle ~8 deg) and the "good" diagonal (0,2)
    /// produces better triangles (min angle ~18 deg).
    /// Points: (0,0), (4,0), (3,1), (0,2) — all on the convex hull.
    fn convex_quad() -> Vec<Point2> {
        vec![
            Point2::new(0.0, 0.0), // 0
            Point2::new(4.0, 0.0), // 1
            Point2::new(3.0, 1.0), // 2
            Point2::new(0.0, 2.0), // 3
        ]
    }

    #[test]
    fn rejects_too_few_points() {
        let pts = vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)];
        let mut tris: Vec<[u32; 3]> = vec![];
        let r = optimise_triangulation(&pts, &mut tris, TriObjective::MaxMinAngle, 100);
        assert!(matches!(r, Err(TriangulationOptError::TooFewPoints { .. })));
    }

    #[test]
    fn rejects_empty_triangulation() {
        let pts = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        ];
        let mut tris: Vec<[u32; 3]> = vec![];
        let r = optimise_triangulation(&pts, &mut tris, TriObjective::MaxMinAngle, 100);
        assert!(matches!(
            r,
            Err(TriangulationOptError::EmptyTriangulation { .. })
        ));
    }

    #[test]
    fn rejects_invalid_vertex_index() {
        let pts = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        ];
        let mut tris = vec![[0u32, 1, 5]]; // 5 is out of range
        let r = optimise_triangulation(&pts, &mut tris, TriObjective::MaxMinAngle, 100);
        assert!(matches!(
            r,
            Err(TriangulationOptError::InconsistentTriangulation { .. })
        ));
    }

    #[test]
    fn flip_improves_max_min_angle() {
        let pts = convex_quad();
        // Initial triangulation with the "bad" diagonal (1,3).
        let mut tris = vec![[0u32, 1, 3], [1u32, 2, 3]];
        let initial_obj = evaluate_objective(&pts, &tris, TriObjective::MaxMinAngle);
        let (flips, final_obj) =
            optimise_and_evaluate(&pts, &mut tris, TriObjective::MaxMinAngle, 100).unwrap();
        assert!(flips > 0, "should flip at least once");
        assert!(final_obj > initial_obj, "objective should improve");
    }

    #[test]
    fn flip_improves_min_max_angle() {
        let pts = convex_quad();
        let mut tris = vec![[0u32, 1, 3], [1u32, 2, 3]];
        let initial_obj = evaluate_objective(&pts, &tris, TriObjective::MinMaxAngle);
        let (flips, final_obj) =
            optimise_and_evaluate(&pts, &mut tris, TriObjective::MinMaxAngle, 100).unwrap();
        assert!(flips > 0, "should flip at least once");
        assert!(final_obj < initial_obj, "max angle should decrease");
    }

    #[test]
    fn no_flip_needed_for_already_optimal() {
        // Equilateral-ish triangulation: two equilateral triangles sharing an
        // edge. No flip should improve.
        let s = 3.0f64.sqrt();
        let pts = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.5, s / 2.0),
            Point2::new(1.5, s / 2.0),
        ];
        let mut tris = vec![[0u32, 1, 2], [1u32, 3, 2]];
        let (flips, _obj) =
            optimise_and_evaluate(&pts, &mut tris, TriObjective::MaxMinAngle, 100).unwrap();
        assert_eq!(flips, 0, "no flip should be needed for equilateral pair");
    }

    #[test]
    fn flip_validity_convex_check() {
        let pts = convex_quad();
        // Edge (1,3) with opposites 0 and 2: the quad is convex.
        assert!(flip_is_valid(&pts, 1, 3, 0, 2));
        // Edge (0,2) with opposites 1 and 3: also convex (same quad, other diagonal).
        assert!(flip_is_valid(&pts, 0, 2, 1, 3));
    }

    #[test]
    fn flip_validity_rejects_non_convex() {
        // Non-convex quad: (0,0), (2,0), (1,0.5), (0.5, 0.1).
        // Vertex 3 (0.5, 0.1) is inside the triangle (0,1,2).
        let pts = vec![
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(1.0, 0.5),
            Point2::new(0.5, 0.1),
        ];
        // Edge (0,2) with opposites 1 and 3: 3 is inside triangle (0,1,2),
        // so the quad is non-convex (turn at d is CW while others are CCW).
        assert!(!flip_is_valid(&pts, 0, 2, 1, 3));
    }

    #[test]
    fn evaluate_objective_max_min_angle() {
        let pts = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        ];
        let tris = vec![[0u32, 1, 2]];
        let val = evaluate_objective(&pts, &tris, TriObjective::MaxMinAngle);
        // Right isosceles: min angle = 45 deg = pi/4.
        assert!(approx(val, FRAC_PI_4, 1e-9));
    }

    #[test]
    fn evaluate_objective_min_max_angle() {
        let pts = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
        ];
        let tris = vec![[0u32, 1, 2]];
        let val = evaluate_objective(&pts, &tris, TriObjective::MinMaxAngle);
        // Right isosceles: max angle = 90 deg = pi/2.
        assert!(approx(val, core::f64::consts::FRAC_PI_2, 1e-9));
    }

    #[test]
    fn evaluate_objective_max_min_area() {
        let pts = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
            Point2::new(1.0, 1.0),
        ];
        let tris = vec![[0u32, 1, 2], [1u32, 3, 2]];
        let val = evaluate_objective(&pts, &tris, TriObjective::MaxMinArea);
        // Both triangles have area 0.5.
        assert!(approx(val, 0.5, 1e-12));
    }

    #[test]
    fn edge_ratio_equilateral() {
        let s = 3.0f64.sqrt();
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(0.5, s / 2.0);
        assert!(approx(tri_edge_ratio(a, b, c), 1.0, 1e-12));
    }

    #[test]
    fn radius_edge_equilateral() {
        let s = 3.0f64.sqrt();
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(0.5, s / 2.0);
        // R = 1/sqrt(3), shortest edge = 1, so radius_edge = 1/sqrt(3).
        assert!(approx(tri_radius_edge(a, b, c), 1.0 / 3.0f64.sqrt(), 1e-9));
    }

    #[test]
    fn aspect_equilateral() {
        let s = 3.0f64.sqrt();
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(0.5, s / 2.0);
        assert!(approx(tri_aspect(a, b, c), 1.0, 1e-9));
    }

    #[test]
    fn find_edge_pair_interior() {
        let tris = vec![[0u32, 1, 2], [0u32, 2, 3]];
        // Edge (0,2) is shared by both triangles.
        let pair = find_edge_pair(&tris, 0, 2);
        assert!(pair.is_some());
        let (_, _, c, d) = pair.unwrap();
        assert!(c == 1 && d == 3 || c == 3 && d == 1);
    }

    #[test]
    fn find_edge_pair_boundary() {
        let tris = vec![[0u32, 1, 2], [0u32, 2, 3]];
        // Edge (0,1) is a boundary edge — only in triangle 0.
        let pair = find_edge_pair(&tris, 0, 1);
        assert!(pair.is_none());
    }

    #[test]
    fn apply_flip_swaps_diagonal() {
        let mut tris = vec![[0u32, 1, 2], [0u32, 2, 3]];
        // Flip edge (0,2) to (1,3).
        apply_flip(&mut tris, 0, 2, 1, 3);
        // After flip, the triangles should be (1,3,2) and (0,1,3) or similar.
        // Check that edge (0,2) is no longer present and (1,3) is.
        let has_02 = tris.iter().any(|t| {
            let edges = [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])];
            edges
                .iter()
                .any(|&(u, v)| (u == 0 && v == 2) || (u == 2 && v == 0))
        });
        assert!(!has_02, "edge (0,2) should be gone after flip");
        let has_13 = tris.iter().any(|t| {
            let edges = [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])];
            edges
                .iter()
                .any(|&(u, v)| (u == 1 && v == 3) || (u == 3 && v == 1))
        });
        assert!(has_13, "edge (1,3) should be present after flip");
    }

    #[test]
    fn iteration_limit_respected() {
        let pts = convex_quad();
        let mut tris = vec![[0u32, 1, 3], [1u32, 2, 3]];
        let flips = optimise_triangulation(&pts, &mut tris, TriObjective::MaxMinAngle, 0).unwrap();
        assert_eq!(flips, 0, "0 iterations = 0 flips");
    }

    #[test]
    fn objective_is_maximise_flag() {
        assert!(TriObjective::MaxMinAngle.is_maximise());
        assert!(TriObjective::MaxMinArea.is_maximise());
        assert!(!TriObjective::MinMaxAngle.is_maximise());
        assert!(!TriObjective::MinMaxEdgeRatio.is_maximise());
        assert!(!TriObjective::MinMaxRadiusEdge.is_maximise());
        assert!(!TriObjective::MinMaxAspect.is_maximise());
    }
}
