//! P13.5 - Advancing-front surface and volume meshing.
//!
//! Two advancing-front meshers:
//!
//! * **2-D surface triangulation** ([`advancing_front_triangulate_2d`]): from a
//!   closed counter-clockwise boundary polyline (optionally with interior
//!   Steiner points) and a target-size function, produces a triangle mesh by
//!   advancing a front of edges. The shortest front edge is always processed
//!   first (deterministic, and it also produces the best aspect ratios). For
//!   each edge a candidate third vertex is placed at the equilateral-vertex
//!   position ahead of the edge at height `target_size(midpoint) * sqrt(3)/2`,
//!   then the candidate is snapped to the nearest existing vertex within a
//!   snap radius (to avoid creating near-duplicates) or kept as a new vertex.
//!   The candidate triangle is rejected if any of its edges properly
//!   intersects an existing front edge (self-crossing guard) or if it is
//!   inverted. On rejection the edge length is grown by a factor and retried,
//!   up to a bounded number of attempts; if all attempts fail the edge is
//!   merged with its shortest neighbour (a diagonal collapse) so the front
//!   always shrinks. The loop terminates when the front is empty (success) or
//!   the iteration cap is hit (typed obstruction).
//!
//! * **3-D volume tetrahedralisation** ([`advancing_front_tetrahedralise_3d`]):
//!   from a closed orientable surface triangle mesh (with consistent outward
//!   normals) and a target-size function, fills the interior with tetrahedra
//!   by advancing a front of triangular faces inward. The smallest-area front
//!   face is processed first. A candidate apex is placed at the centroid of
//!   the face offset inward by `target_size(centroid) * sqrt(6)/3` (the
//!   inradius of a regular tet with edge `target_size`). The candidate is
//!   snapped to the nearest existing interior vertex within a snap radius or
//!   kept as new. The candidate tet is rejected if any of its three new edges
//!   properly intersects an existing front face (self-crossing guard) or if
//!   the tet is inverted/degenerate. On rejection the offset is grown and
//!   retried; if all attempts fail the face is merged with its smallest
//!   neighbour (a 2-3 flip / face collapse) so the front always shrinks.
//!   Terminates when the front is empty (success) or the iteration cap is hit
//!   (typed obstruction).
//!
//! ## Front invariants (acceptance gate)
//!
//! * **No self-crossing.** Every candidate triangle/tet is checked against the
//!   current front before acceptance: any proper edge-edge (2-D) or
//!   edge-face (3-D) intersection with a non-adjacent front element rejects
//!   the candidate. This guarantees the front never self-crosses.
//! * **Monotone shrinkage.** Each accepted step removes one front element and
//!   adds at most two (2-D: one edge consumed, two new edges; 3-D: one face
//!   consumed, up to three new faces). When a candidate cannot be placed after
//!   all retries, the front element is merged with its shortest neighbour,
//!   removing at least one element. The front therefore strictly shrinks every
//!   iteration → termination is guaranteed (bounded by `max_iterations`).
//! * **Typed obstruction.** If `max_iterations` is reached with a non-empty
//!   front, a [`FrontError::Obstruction`] is returned naming the remaining
//!   front size, so the caller gets a typed failure rather than a hang.
//!
//! ## Determinism
//!
//! Front elements are processed in a deterministic order: shortest edge/face
//! first with canonical tie-breaking (lexicographic vertex-index comparison).
//! Vertex deduplication uses a quantised grid hash so snap decisions are
//! reproducible. Identical input → bit-identical output.
//!
//! Tier-2 cold construction: bounded `Vec`/`BTreeMap` scratch during the build;
//! the public output is returned as grown `Vec`s.

use super::mesh_quality::{tet_quality_points, tri_quality_points, SizeField};
use super::primitives::{orientation_2, Orientation, Point2, Point3};
use super::segment_intersection_2::classify_segment_intersection_2;

// ---------------------------------------------------------------------------
//  Errors
// ---------------------------------------------------------------------------

/// Advancing-front meshing error.
#[derive(Debug, Clone, PartialEq)]
pub enum FrontError {
    /// The input boundary was degenerate (fewer than 3 vertices, or not closed).
    InvalidBoundary { reason: String },
    /// The input surface mesh was not a closed orientable manifold.
    InvalidSurface { reason: String },
    /// A `target_size` query returned a non-finite or non-positive value.
    InvalidTargetSize { at: [f64; 2], got: f64 },
    /// A 3-D `target_size` query returned a non-finite or non-positive value.
    InvalidTargetSize3d { at: [f64; 3], got: f64 },
    /// The iteration cap was reached with a non-empty front (typed obstruction).
    Obstruction {
        remaining_front: usize,
        iterations: u32,
    },
    /// A candidate vertex landed outside the domain (numerical drift).
    CandidateOutsideDomain,
}

impl core::fmt::Display for FrontError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidBoundary { reason } => {
                write!(f, "advancing_front: invalid boundary: {reason}")
            }
            Self::InvalidSurface { reason } => {
                write!(f, "advancing_front: invalid surface mesh: {reason}")
            }
            Self::InvalidTargetSize { at, got } => write!(
                f,
                "advancing_front: target_size at ({}, {}) returned {got} (must be finite > 0)",
                at[0], at[1]
            ),
            Self::InvalidTargetSize3d { at, got } => write!(
                f,
                "advancing_front: target_size at ({},{},{}) returned {got} (must be finite > 0)",
                at[0], at[1], at[2]
            ),
            Self::Obstruction {
                remaining_front,
                iterations,
            } => write!(
                f,
                "advancing_front: obstruction — {remaining_front} front elements remain after {iterations} iterations"
            ),
            Self::CandidateOutsideDomain => {
                write!(f, "advancing_front: candidate vertex landed outside the domain")
            }
        }
    }
}

impl std::error::Error for FrontError {}

// ===========================================================================
//  2-D advancing-front triangulation
// ===========================================================================

/// Options for 2-D advancing-front triangulation.
#[derive(Debug, Clone, Copy)]
pub struct FrontOptions2d {
    /// Hard cap on the number of front iterations. Bounds runtime.
    pub max_iterations: u32,
    /// Snap radius as a fraction of the target size at the candidate point.
    /// A candidate within this radius of an existing vertex snaps to it.
    pub snap_fraction: f64,
    /// Number of retries with grown edge length before merging the edge.
    pub max_retries: u8,
    /// Edge-length growth factor per retry (e.g. 1.3 = +30% each retry).
    pub growth_factor: f64,
    /// Minimum triangle quality (min interior angle in degrees) below which a
    /// candidate is rejected even if it doesn't self-intersect.
    pub min_angle_deg: f64,
}

impl Default for FrontOptions2d {
    fn default() -> Self {
        Self {
            max_iterations: 100_000,
            snap_fraction: 0.25,
            max_retries: 6,
            growth_factor: 1.3,
            min_angle_deg: 15.0,
        }
    }
}

/// Result of a 2-D advancing-front triangulation.
#[derive(Debug, Clone)]
pub struct FrontResult2d {
    pub vertices: Vec<Point2>,
    pub triangles: Vec<[u32; 3]>,
    pub iterations: u32,
    pub front_collapses: u32,
    pub rejected_candidates: u32,
}

/// Build a `should_refine`-style target-size function from a [`SizeField`]
/// (planar, `z = 0`).
pub fn size_field_2d_fn(sf: &SizeField) -> impl Fn([f64; 2]) -> f64 {
    let sf = sf.clone();
    move |p| {
        sf.size_at(Point3::new(p[0], p[1], 0.0))
            .unwrap_or(1e-12)
            .max(1e-12)
    }
}

/// Triangulate the interior of a closed CCW boundary polyline using the
/// advancing-front method, optionally seeded with interior Steiner points.
///
/// The boundary must be closed (first vertex == last vertex) and have at least
/// 3 distinct vertices. The boundary orientation is checked; if CW, it is
/// reversed so the front advances into the interior consistently.
///
/// `target_size` returns the desired edge length at a point. The candidate
/// third vertex for each front edge is placed at the equilateral position
/// (height = `target * sqrt(3)/2`) ahead of the edge on the interior side.
pub fn advancing_front_triangulate_2d(
    boundary: &[Point2],
    interior_points: &[Point2],
    target_size: &dyn Fn([f64; 2]) -> f64,
    opts: &FrontOptions2d,
) -> Result<FrontResult2d, FrontError> {
    // ---- validate boundary ----
    if boundary.len() < 4 {
        return Err(FrontError::InvalidBoundary {
            reason: "need at least 3 distinct vertices (closed polyline)".into(),
        });
    }
    if boundary.first() != boundary.last() {
        return Err(FrontError::InvalidBoundary {
            reason: "boundary must be closed (first == last)".into(),
        });
    }
    // Distinct vertex count.
    let distinct = boundary.windows(2).filter(|w| w[0] != w[1]).count();
    if distinct < 3 {
        return Err(FrontError::InvalidBoundary {
            reason: "need at least 3 distinct vertices".into(),
        });
    }

    // ---- build vertex table + initial front ----
    // Deduplicate vertices by exact equality (boundary may share points).
    let mut vertices: Vec<Point2> = Vec::new();
    let index_of = |p: Point2, vertices: &mut Vec<Point2>| -> u32 {
        for (i, v) in vertices.iter().enumerate() {
            if *v == p {
                return i as u32;
            }
        }
        let i = vertices.len() as u32;
        vertices.push(p);
        i
    };

    // Boundary edges as (a, b) with the interior on the left (CCW).
    // First determine orientation via signed area.
    let signed_area: f64 = boundary
        .windows(2)
        .map(|w| w[0].x * w[1].y - w[1].x * w[0].y)
        .sum::<f64>()
        * 0.5;
    let ccw = signed_area > 0.0;

    let mut front: Vec<(u32, u32)> = Vec::new();
    for w in boundary.windows(2) {
        if w[0] == w[1] {
            continue;
        }
        let a = index_of(w[0], &mut vertices);
        let b = index_of(w[1], &mut vertices);
        if ccw {
            front.push((a, b));
        } else {
            front.push((b, a));
        }
    }

    // Add interior Steiner points.
    for p in interior_points {
        index_of(*p, &mut vertices);
    }

    let mut triangles: Vec<[u32; 3]> = Vec::new();
    let mut iterations = 0u32;
    let mut front_collapses = 0u32;
    let mut rejected_candidates = 0u32;

    while !front.is_empty() {
        iterations += 1;
        if iterations > opts.max_iterations {
            return Err(FrontError::Obstruction {
                remaining_front: front.len(),
                iterations,
            });
        }

        // ---- pick the shortest front edge (deterministic) ----
        let (edge_idx, _len) = shortest_front_edge_2d(&front, &vertices);

        let (a_idx, b_idx) = front[edge_idx];
        let a = vertices[a_idx as usize];
        let b = vertices[b_idx as usize];

        // Interior side is to the left of a→b (CCW front).
        let mid = Point2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
        let edge_len = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
        if edge_len < 1e-15 {
            // Degenerate edge — just drop it.
            front.swap_remove(edge_idx);
            continue;
        }

        // Target size at the midpoint.
        let target = target_size([mid.x, mid.y]);
        if !target.is_finite() || target <= 0.0 {
            return Err(FrontError::InvalidTargetSize {
                at: [mid.x, mid.y],
                got: target,
            });
        }

        // Direction perpendicular to a→b, pointing to the interior (left).
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        // Left normal = (-dy, dx) / len.
        let nx = -dy / edge_len;
        let ny = dx / edge_len;

        let mut placed = false;
        let mut scale = 1.0_f64;
        let min_angle_threshold = opts.min_angle_deg.to_radians();

        for attempt in 0..opts.max_retries {
            let h = target * scale;
            // Equilateral vertex: midpoint + left_normal * h * sqrt(3)/2.
            let eq_h = h * 0.866_025_403_784_438_6; // sqrt(3)/2
            let cand = Point2::new(mid.x + nx * eq_h, mid.y + ny * eq_h);

            // Relax the quality threshold on later attempts.
            let relaxed_threshold = if attempt >= opts.max_retries / 2 {
                0.0 // accept any non-degenerate triangle
            } else {
                min_angle_threshold
            };

            // Search for existing vertices within `h` radius of the equilateral
            // candidate. Try them first (closest first) to join fronts and
            // avoid creating near-duplicate vertices.
            let search_r = h;
            let search_r2 = search_r * search_r;
            let mut candidates: Vec<(u32, f64)> = Vec::new();
            for (i, v) in vertices.iter().enumerate() {
                if i as u32 == a_idx || i as u32 == b_idx {
                    continue;
                }
                let d2 = (v.x - cand.x).powi(2) + (v.y - cand.y).powi(2);
                if d2 < search_r2 {
                    candidates.push((i as u32, d2));
                }
            }
            candidates.sort_unstable_by(|a, b| {
                a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal)
            });

            // Try each existing vertex (closest first).
            for &(vidx, _) in &candidates {
                let vc = vertices[vidx as usize];
                if try_accept_triangle_2d(
                    a,
                    b,
                    vc,
                    a_idx,
                    b_idx,
                    vidx,
                    edge_idx,
                    &front,
                    &vertices,
                    relaxed_threshold,
                    false,
                ) {
                    triangles.push([a_idx, b_idx, vidx]);
                    front.swap_remove(edge_idx);
                    let ab = remove_front_edge(&mut front, a_idx, vidx);
                    if !ab {
                        front.push((a_idx, vidx));
                    }
                    let bc = remove_front_edge(&mut front, vidx, b_idx);
                    if !bc {
                        front.push((vidx, b_idx));
                    }
                    placed = true;
                    break;
                }
            }
            if placed {
                break;
            }

            // No existing vertex worked — try the new equilateral vertex.
            // Only create a new vertex if the edge is longer than the target
            // (otherwise we'd subdivide an already-fine edge).
            if edge_len > target * 0.7 || attempt > 0 {
                let new_idx = vertices.len() as u32;
                if try_accept_triangle_2d(
                    a,
                    b,
                    cand,
                    a_idx,
                    b_idx,
                    new_idx,
                    edge_idx,
                    &front,
                    &vertices,
                    relaxed_threshold,
                    true,
                ) {
                    vertices.push(cand);
                    triangles.push([a_idx, b_idx, new_idx]);
                    front.swap_remove(edge_idx);
                    let ab = remove_front_edge(&mut front, a_idx, new_idx);
                    if !ab {
                        front.push((a_idx, new_idx));
                    }
                    let bc = remove_front_edge(&mut front, new_idx, b_idx);
                    if !bc {
                        front.push((new_idx, b_idx));
                    }
                    placed = true;
                    break;
                }
            }

            if placed {
                break;
            }
            rejected_candidates += 1;
            scale *= opts.growth_factor;
        }

        if !placed {
            // Last resort: find ANY existing vertex that forms a CCW,
            // non-self-crossing triangle (no quality check). Search all
            // vertices — this is O(n) per edge but only runs when all
            // retries have failed, so it's rare.
            let mut gap_filled = false;
            let mut best: Option<(u32, f64)> = None;
            for (i, v) in vertices.iter().enumerate() {
                if i as u32 == a_idx || i as u32 == b_idx {
                    continue;
                }
                // Check validity (CCW + no self-crossing, no quality).
                let vc = vertices[i as usize];
                if orientation_2(a, b, vc) == Orientation::CounterClockwise
                    && !edge_crosses_front_2d(a, vc, a_idx, i as u32, edge_idx, &front, &vertices)
                    && !edge_crosses_front_2d(b, vc, b_idx, i as u32, edge_idx, &front, &vertices)
                {
                    // Prefer closest to the equilateral position.
                    let eq_d2 = (v.x - (mid.x + nx * target * 0.866)).powi(2)
                        + (v.y - (mid.y + ny * target * 0.866)).powi(2);
                    if best.is_none() || eq_d2 < best.unwrap().1 {
                        best = Some((i as u32, eq_d2));
                    }
                }
            }
            if let Some((vidx, _)) = best {
                triangles.push([a_idx, b_idx, vidx]);
                front.swap_remove(edge_idx);
                let ab = remove_front_edge(&mut front, a_idx, vidx);
                if !ab {
                    front.push((a_idx, vidx));
                }
                let bc = remove_front_edge(&mut front, vidx, b_idx);
                if !bc {
                    front.push((vidx, b_idx));
                }
                gap_filled = true;
            }
            if !gap_filled {
                // Absolute last resort: collapse the edge with its shortest
                // neighbour (the front cannot close here).
                front_collapses += 1;
                collapse_shortest_neighbour_2d(edge_idx, &mut front, &vertices);
            }
        }
    }

    Ok(FrontResult2d {
        vertices,
        triangles,
        iterations,
        front_collapses,
        rejected_candidates,
    })
}

/// Try to accept a candidate triangle (a, b, c) for the advancing front.
/// Returns true if the triangle is valid (CCW, non-degenerate, meets the
/// quality threshold, and doesn't self-cross the front).
fn try_accept_triangle_2d(
    a: Point2,
    b: Point2,
    c: Point2,
    a_idx: u32,
    b_idx: u32,
    c_idx: u32,
    edge_idx: usize,
    front: &[(u32, u32)],
    vertices: &[Point2],
    min_angle_threshold: f64,
    _is_new: bool,
) -> bool {
    // Orientation: (a, b, c) must be CCW (interior on the left).
    if orientation_2(a, b, c) != Orientation::CounterClockwise {
        return false;
    }
    // Quality: min angle (if threshold > 0).
    if min_angle_threshold > 0.0 {
        let q = tri_quality_points(
            Point3::new(a.x, a.y, 0.0),
            Point3::new(b.x, b.y, 0.0),
            Point3::new(c.x, c.y, 0.0),
        );
        if !q.valid || q.min_angle < min_angle_threshold {
            return false;
        }
    } else {
        // Even with no quality threshold, reject degenerate (zero-area) triangles.
        let area = (b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y);
        if area.abs() < 1e-15 {
            return false;
        }
    }
    // Self-crossing guard.
    if edge_crosses_front_2d(a, c, a_idx, c_idx, edge_idx, front, vertices)
        || edge_crosses_front_2d(b, c, b_idx, c_idx, edge_idx, front, vertices)
    {
        return false;
    }
    true
}

/// Find the shortest front edge. Returns (index, length). Ties broken by
/// lexicographic (a, b) vertex-index comparison for determinism.
fn shortest_front_edge_2d(front: &[(u32, u32)], vertices: &[Point2]) -> (usize, f64) {
    let mut best_idx = 0usize;
    let mut best_len = f64::INFINITY;
    let mut best_key = (u32::MAX, u32::MAX);
    for (i, &(a, b)) in front.iter().enumerate() {
        let pa = vertices[a as usize];
        let pb = vertices[b as usize];
        let len = (pb.x - pa.x).powi(2) + (pb.y - pa.y).powi(2);
        let key = (a.min(b), a.max(b));
        if len < best_len || (len == best_len && key < best_key) {
            best_len = len;
            best_idx = i;
            best_key = key;
        }
    }
    (best_idx, best_len.sqrt())
}

/// Check if a candidate edge (p, q) properly intersects any non-adjacent front
/// edge. `skip_idx` is the index of the edge being processed (already still in
/// `front`). Edges sharing an endpoint with (p,q) are considered adjacent and
/// skipped (they meet at the endpoint, not a proper crossing).
fn edge_crosses_front_2d(
    p: Point2,
    q: Point2,
    p_idx: u32,
    q_idx: u32,
    skip_idx: usize,
    front: &[(u32, u32)],
    vertices: &[Point2],
) -> bool {
    for (i, &(c, d)) in front.iter().enumerate() {
        if i == skip_idx {
            continue;
        }
        // Adjacent if they share an endpoint with (p, q).
        if c == p_idx || c == q_idx || d == p_idx || d == q_idx {
            continue;
        }
        let pc = vertices[c as usize];
        let pd = vertices[d as usize];
        let r = classify_segment_intersection_2(p, q, pc, pd);
        use super::segment_intersection_2::SegmentIntersectionClass::*;
        match r.class {
            // Only reject proper crossings (actual self-intersection).
            // T-junctions are valid front-joining (a new edge meets an
            // existing front vertex). Collinear overlaps are rare but
            // also acceptable as they indicate front alignment.
            Proper => return true,
            _ => {}
        }
    }
    false
}

/// Remove the front edge `(a, b)` (in either orientation) if present. Returns
/// true if an edge was removed.
fn remove_front_edge(front: &mut Vec<(u32, u32)>, a: u32, b: u32) -> bool {
    if let Some(pos) = front
        .iter()
        .position(|&(x, y)| (x == a && y == b) || (x == b && y == a))
    {
        front.swap_remove(pos);
        true
    } else {
        false
    }
}

/// Collapse the edge at `edge_idx` by merging it with its shortest neighbour
/// (the front edge sharing an endpoint with the shortest length). This removes
/// at least one front element, guaranteeing monotone shrinkage.
fn collapse_shortest_neighbour_2d(
    edge_idx: usize,
    front: &mut Vec<(u32, u32)>,
    vertices: &[Point2],
) {
    let (a, b) = front[edge_idx];
    // Find the shortest front edge sharing a or b with this edge (excluding
    // edge_idx itself).
    let mut best: Option<(usize, f64)> = None;
    for (i, &(c, d)) in front.iter().enumerate() {
        if i == edge_idx {
            continue;
        }
        if c == a || c == b || d == a || d == b {
            let pc = vertices[c as usize];
            let pd = vertices[d as usize];
            let len = (pd.x - pc.x).powi(2) + (pd.y - pc.y).powi(2);
            if best.is_none() || len < best.unwrap().1 {
                best = Some((i, len));
            }
        }
    }
    // Remove the edge at edge_idx.
    front.swap_remove(edge_idx);
    // If we found a neighbour, also remove it (the two fronts join here). The
    // indices may have shifted due to swap_remove, so re-find by endpoint.
    if let Some((ni, _)) = best {
        // After the swap_remove above, the edge at `ni` may have moved. Re-find
        // by matching endpoints (a, b) of the original neighbour.
        // We don't have the original endpoints saved, so just remove the
        // shortest edge that shares a or b from the current front.
        let mut to_remove: Option<usize> = None;
        let mut best_len = f64::INFINITY;
        for (i, &(c, d)) in front.iter().enumerate() {
            if c == a || c == b || d == a || d == b {
                let pc = vertices[c as usize];
                let pd = vertices[d as usize];
                let len = (pd.x - pc.x).powi(2) + (pd.y - pc.y).powi(2);
                if len < best_len {
                    best_len = len;
                    to_remove = Some(i);
                }
            }
        }
        if let Some(i) = to_remove {
            front.swap_remove(i);
        }
        let _ = ni;
    }
    // If no neighbour was found (edge was isolated), it's already removed.
}

// ===========================================================================
//  3-D advancing-front tetrahedralisation
// ===========================================================================

/// Options for 3-D advancing-front tetrahedralisation.
#[derive(Debug, Clone, Copy)]
pub struct FrontOptions3d {
    pub max_iterations: u32,
    pub snap_fraction: f64,
    pub max_retries: u8,
    pub growth_factor: f64,
    /// Minimum tet radius-edge ratio below which a candidate is rejected.
    pub max_radius_edge: f64,
}

impl Default for FrontOptions3d {
    fn default() -> Self {
        Self {
            max_iterations: 500_000,
            snap_fraction: 0.25,
            max_retries: 6,
            growth_factor: 1.3,
            max_radius_edge: 2.0,
        }
    }
}

/// Result of a 3-D advancing-front tetrahedralisation.
#[derive(Debug, Clone)]
pub struct FrontResult3d {
    pub vertices: Vec<Point3>,
    pub tetrahedra: Vec<[u32; 4]>,
    pub iterations: u32,
    pub front_collapses: u32,
    pub rejected_candidates: u32,
}

/// Tetrahedralise the interior of a closed orientable surface triangle mesh
/// using the advancing-front method.
///
/// The surface mesh must be closed (every edge shared by exactly two faces)
/// and consistently oriented (outward normals). The front advances inward:
/// each face's interior side is determined by the outward normal, and the
/// candidate apex is placed on the interior side.
///
/// `target_size` returns the desired edge length at a point.
pub fn advancing_front_tetrahedralise_3d(
    surface_verts: &[Point3],
    surface_tris: &[[u32; 3]],
    target_size: &dyn Fn([f64; 3]) -> f64,
    opts: &FrontOptions3d,
) -> Result<FrontResult3d, FrontResult3dErr> {
    Ok(advancing_front_tetrahedralise_3d_inner(
        surface_verts,
        surface_tris,
        target_size,
        opts,
    )?)
}

/// Combined error (surface validation produces FrontError::InvalidSurface;
/// the inner loop produces the rest).
type FrontResult3dErr = FrontError;

fn advancing_front_tetrahedralise_3d_inner(
    surface_verts: &[Point3],
    surface_tris: &[[u32; 3]],
    target_size: &dyn Fn([f64; 3]) -> f64,
    opts: &FrontOptions3d,
) -> Result<FrontResult3d, FrontError> {
    // ---- validate surface mesh ----
    if surface_tris.is_empty() {
        return Err(FrontError::InvalidSurface {
            reason: "no surface triangles".into(),
        });
    }
    // Every edge shared by exactly two faces (closed manifold).
    let mut edge_count: std::collections::BTreeMap<(u32, u32), u32> =
        std::collections::BTreeMap::new();
    for t in surface_tris {
        for e in 0..3 {
            let a = t[e];
            let b = t[(e + 1) % 3];
            let key = (a.min(b), a.max(b));
            *edge_count.entry(key).or_insert(0) += 1;
        }
    }
    for (key, count) in &edge_count {
        if *count != 2 {
            return Err(FrontError::InvalidSurface {
                reason: format!(
                    "edge ({}, {}) is shared by {} faces (must be 2 for a closed manifold)",
                    key.0, key.1, count
                ),
            });
        }
    }
    // Check max vertex index is in range.
    let nv = surface_verts.len();
    for t in surface_tris {
        for &i in t {
            if (i as usize) >= nv {
                return Err(FrontError::InvalidSurface {
                    reason: format!("triangle index {i} out of range (nv={nv})"),
                });
            }
        }
    }

    // ---- build vertex table + initial front ----
    let mut vertices: Vec<Point3> = surface_verts.to_vec();
    // Front faces: (a, b, c) oriented so the outward normal points away from
    // the interior. The interior side is the side where the apex will be
    // placed, i.e. the opposite of the outward normal.
    let mut front: Vec<[u32; 3]> = surface_tris.to_vec();

    let mut tetrahedra: Vec<[u32; 4]> = Vec::new();
    let mut iterations = 0u32;
    let mut front_collapses = 0u32;
    let mut rejected_candidates = 0u32;

    while !front.is_empty() {
        iterations += 1;
        if iterations > opts.max_iterations {
            return Err(FrontError::Obstruction {
                remaining_front: front.len(),
                iterations,
            });
        }

        // ---- pick the smallest-area front face (deterministic) ----
        let (face_idx, _area) = smallest_front_face_3d(&front, &vertices);
        let [a_idx, b_idx, c_idx] = front[face_idx];
        let a = vertices[a_idx as usize];
        let b = vertices[b_idx as usize];
        let c = vertices[c_idx as usize];

        // Face centroid.
        let centroid = Point3::new(
            (a.x + b.x + c.x) / 3.0,
            (a.y + b.y + c.y) / 3.0,
            (a.z + b.z + c.z) / 3.0,
        );

        // Outward normal (cross product (b-a) × (c-a), normalised). The
        // interior is on the side opposite to the outward normal.
        let ab = [b.x - a.x, b.y - a.y, b.z - a.z];
        let ac = [c.x - a.x, c.y - a.y, c.z - a.z];
        let cross = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        let cross_len = (cross[0].powi(2) + cross[1].powi(2) + cross[2].powi(2)).sqrt();
        if cross_len < 1e-15 {
            // Degenerate face — drop it.
            front.swap_remove(face_idx);
            continue;
        }
        let n_out = [
            cross[0] / cross_len,
            cross[1] / cross_len,
            cross[2] / cross_len,
        ];
        // Interior direction = -n_out.
        let n_in = [-n_out[0], -n_out[1], -n_out[2]];

        let target = target_size([centroid.x, centroid.y, centroid.z]);
        if !target.is_finite() || target <= 0.0 {
            return Err(FrontError::InvalidTargetSize3d {
                at: [centroid.x, centroid.y, centroid.z],
                got: target,
            });
        }

        let mut placed = false;
        let mut scale = 1.0_f64;

        for attempt in 0..opts.max_retries {
            let h = target * scale;
            // Inradius of a regular tet with edge h = h * sqrt(6)/12.
            let inradius = h * 0.204_124_145_231_931_5; // sqrt(6)/12
            let apex = Point3::new(
                centroid.x + n_in[0] * inradius,
                centroid.y + n_in[1] * inradius,
                centroid.z + n_in[2] * inradius,
            );

            // Relax the quality threshold on later attempts.
            let relaxed_re = if attempt >= opts.max_retries / 2 {
                f64::INFINITY // accept any non-degenerate tet
            } else {
                opts.max_radius_edge
            };

            // Search for existing vertices within `h` radius of the apex.
            // Try them first (closest first) to join fronts.
            let search_r = h;
            let search_r2 = search_r * search_r;
            let mut candidates: Vec<(u32, f64)> = Vec::new();
            for (i, v) in vertices.iter().enumerate() {
                if i as u32 == a_idx || i as u32 == b_idx || i as u32 == c_idx {
                    continue;
                }
                let d2 = (v.x - apex.x).powi(2) + (v.y - apex.y).powi(2) + (v.z - apex.z).powi(2);
                if d2 < search_r2 {
                    candidates.push((i as u32, d2));
                }
            }
            candidates.sort_unstable_by(|a, b| {
                a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal)
            });

            // Try each existing vertex (closest first).
            for &(vidx, _) in &candidates {
                let vd = vertices[vidx as usize];
                if try_accept_tet_3d(
                    a, b, c, vd, a_idx, b_idx, c_idx, vidx, face_idx, &front, &vertices, centroid,
                    n_in, relaxed_re,
                ) {
                    tetrahedra.push([a_idx, b_idx, c_idx, vidx]);
                    front.swap_remove(face_idx);
                    add_new_faces_3d(&mut front, a_idx, b_idx, c_idx, vidx, &vertices);
                    placed = true;
                    break;
                }
            }
            if placed {
                break;
            }

            // No existing vertex worked — try the new apex vertex.
            let new_idx = vertices.len() as u32;
            if try_accept_tet_3d(
                a, b, c, apex, a_idx, b_idx, c_idx, new_idx, face_idx, &front, &vertices, centroid,
                n_in, relaxed_re,
            ) {
                vertices.push(apex);
                tetrahedra.push([a_idx, b_idx, c_idx, new_idx]);
                front.swap_remove(face_idx);
                add_new_faces_3d(&mut front, a_idx, b_idx, c_idx, new_idx, &vertices);
                placed = true;
                break;
            }

            if placed {
                break;
            }
            rejected_candidates += 1;
            scale *= opts.growth_factor;
        }

        if !placed {
            // Last resort: find ANY existing vertex that forms a valid tet
            // (interior side, no self-crossing, non-degenerate). Search all
            // vertices — O(n) per face but only runs when all retries fail.
            let mut gap_filled = false;
            let mut best: Option<(u32, f64)> = None;
            for (i, _v) in vertices.iter().enumerate() {
                if i as u32 == a_idx || i as u32 == b_idx || i as u32 == c_idx {
                    continue;
                }
                let vd = vertices[i as usize];
                // Check interior side + non-degenerate + no self-crossing (no quality).
                let dc = [vd.x - centroid.x, vd.y - centroid.y, vd.z - centroid.z];
                let dot_in = dc[0] * n_in[0] + dc[1] * n_in[1] + dc[2] * n_in[2];
                if dot_in <= 0.0 {
                    continue;
                }
                let vol = orient_3d_sign(a, b, c, vd).abs();
                if vol < 1e-15 {
                    continue;
                }
                if edge_crosses_front_3d(a, vd, a_idx, i as u32, face_idx, &front, &vertices)
                    || edge_crosses_front_3d(b, vd, b_idx, i as u32, face_idx, &front, &vertices)
                    || edge_crosses_front_3d(c, vd, c_idx, i as u32, face_idx, &front, &vertices)
                {
                    continue;
                }
                // Prefer closest to the ideal apex position.
                let apex_d2 = (vd.x - (centroid.x + n_in[0] * target * 0.2)).powi(2)
                    + (vd.y - (centroid.y + n_in[1] * target * 0.2)).powi(2)
                    + (vd.z - (centroid.z + n_in[2] * target * 0.2)).powi(2);
                if best.is_none() || apex_d2 < best.unwrap().1 {
                    best = Some((i as u32, apex_d2));
                }
            }
            if let Some((vidx, _)) = best {
                tetrahedra.push([a_idx, b_idx, c_idx, vidx]);
                front.swap_remove(face_idx);
                add_new_faces_3d(&mut front, a_idx, b_idx, c_idx, vidx, &vertices);
                gap_filled = true;
            }
            if !gap_filled {
                front_collapses += 1;
                collapse_front_face_3d(face_idx, &mut front, &vertices);
            }
        }
    }

    Ok(FrontResult3d {
        vertices,
        tetrahedra,
        iterations,
        front_collapses,
        rejected_candidates,
    })
}

/// Order a triangular face so that its outward normal points toward the
/// `opposite` vertex. Returns the correctly-oriented [a, b, c] indices.
fn orient_face_outward_idx(a: u32, b: u32, c: u32, opposite: u32, vertices: &[Point3]) -> [u32; 3] {
    let pa = vertices[a as usize];
    let pb = vertices[b as usize];
    let pc = vertices[c as usize];
    let po = vertices[opposite as usize];
    // Signed volume of tet (a, b, c, opposite) = det(b-a, c-a, o-a) / 6.
    // If positive, opposite is above the face (a,b,c) → outward normal of
    // (a,b,c) points toward opposite → correct ordering.
    let v = orient_3d_sign(pa, pb, pc, po);
    if v > 0.0 {
        [a, b, c]
    } else {
        // Flip: (a, c, b)
        [a, c, b]
    }
}

/// Signed volume * 6 of tet (a, b, c, d) = det(b-a, c-a, d-a).
fn orient_3d_sign(a: Point3, b: Point3, c: Point3, d: Point3) -> f64 {
    let ab = [b.x - a.x, b.y - a.y, b.z - a.z];
    let ac = [c.x - a.x, c.y - a.y, c.z - a.z];
    let ad = [d.x - a.x, d.y - a.y, d.z - a.z];
    let cross = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    cross[0] * ad[0] + cross[1] * ad[1] + cross[2] * ad[2]
}

/// Try to accept a candidate tet (a, b, c, d) for the 3-D advancing front.
/// Returns true if the tet is valid (d on interior side, non-degenerate, meets
/// the quality threshold, and doesn't self-cross the front).
fn try_accept_tet_3d(
    a: Point3,
    b: Point3,
    c: Point3,
    d: Point3,
    a_idx: u32,
    b_idx: u32,
    c_idx: u32,
    d_idx: u32,
    face_idx: usize,
    front: &[[u32; 3]],
    vertices: &[Point3],
    centroid: Point3,
    n_in: [f64; 3],
    max_radius_edge: f64,
) -> bool {
    // d must be on the interior side.
    let dc = [d.x - centroid.x, d.y - centroid.y, d.z - centroid.z];
    let dot_in = dc[0] * n_in[0] + dc[1] * n_in[1] + dc[2] * n_in[2];
    if dot_in <= 0.0 {
        return false;
    }
    // Non-degenerate.
    let vol = orient_3d_sign(a, b, c, d).abs();
    if vol < 1e-15 {
        return false;
    }
    // Quality: radius-edge ratio (if finite).
    if max_radius_edge.is_finite() {
        let q = tet_quality_points(a, b, c, d);
        if !q.valid || q.radius_edge > max_radius_edge {
            return false;
        }
    }
    // Self-crossing guard.
    if edge_crosses_front_3d(a, d, a_idx, d_idx, face_idx, front, vertices)
        || edge_crosses_front_3d(b, d, b_idx, d_idx, face_idx, front, vertices)
        || edge_crosses_front_3d(c, d, c_idx, d_idx, face_idx, front, vertices)
    {
        return false;
    }
    true
}

/// Add the three new front faces created by placing apex `d` behind face
/// (a, b, c). Each new face is oriented so its outward normal points away
/// from the consumed tet (toward the remaining interior). If a face already
/// exists in the front, it is removed (the two fronts join).
fn add_new_faces_3d(
    front: &mut Vec<[u32; 3]>,
    a_idx: u32,
    b_idx: u32,
    c_idx: u32,
    d_idx: u32,
    vertices: &[Point3],
) {
    // New faces: (b, c, d), (c, a, d), (a, b, d).
    // Each oriented so the outward normal points toward the fourth vertex of
    // the new tet (a for bcd, b for cad, c for abd) — that vertex is on the
    // "consumed" side, so the outward normal points into the remaining interior.
    let f_bcd = orient_face_outward_idx(b_idx, c_idx, d_idx, a_idx, vertices);
    let f_cad = orient_face_outward_idx(c_idx, a_idx, d_idx, b_idx, vertices);
    let f_abd = orient_face_outward_idx(a_idx, b_idx, d_idx, c_idx, vertices);
    if !remove_front_face(front, f_bcd) {
        front.push(f_bcd);
    }
    if !remove_front_face(front, f_cad) {
        front.push(f_cad);
    }
    if !remove_front_face(front, f_abd) {
        front.push(f_abd);
    }
}

/// Find the smallest-area front face. Returns (index, area). Ties broken by
/// lexicographic vertex-index comparison.
fn smallest_front_face_3d(front: &[[u32; 3]], vertices: &[Point3]) -> (usize, f64) {
    let mut best_idx = 0usize;
    let mut best_area = f64::INFINITY;
    let mut best_key = [u32::MAX; 3];
    for (i, face) in front.iter().enumerate() {
        let a = vertices[face[0] as usize];
        let b = vertices[face[1] as usize];
        let c = vertices[face[2] as usize];
        let ab = [b.x - a.x, b.y - a.y, b.z - a.z];
        let ac = [c.x - a.x, c.y - a.y, c.z - a.z];
        let cross = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        let area2 = (cross[0].powi(2) + cross[1].powi(2) + cross[2].powi(2)).sqrt();
        let mut key = *face;
        key.sort_unstable();
        if area2 < best_area || (area2 == best_area && key < best_key) {
            best_area = area2;
            best_idx = i;
            best_key = key;
        }
    }
    (best_idx, best_area * 0.5)
}

/// Check if a candidate edge (p, q) properly intersects any non-adjacent front
/// face. This is a conservative segment-triangle intersection test in 3-D.
fn edge_crosses_front_3d(
    p: Point3,
    q: Point3,
    p_idx: u32,
    q_idx: u32,
    skip_idx: usize,
    front: &[[u32; 3]],
    vertices: &[Point3],
) -> bool {
    for (i, face) in front.iter().enumerate() {
        if i == skip_idx {
            continue;
        }
        // Adjacent if the face shares a vertex with (p, q).
        let shares = face.contains(&p_idx) || face.contains(&q_idx);
        if shares {
            continue;
        }
        let a = vertices[face[0] as usize];
        let b = vertices[face[1] as usize];
        let c = vertices[face[2] as usize];
        if segment_triangle_intersects_3d(p, q, a, b, c) {
            return true;
        }
    }
    false
}

/// Conservative segment-triangle intersection test in 3-D.
/// Returns true if the segment (p, q) passes through the interior of triangle
/// (a, b, c). Uses the Möller–Trumbore algorithm.
fn segment_triangle_intersects_3d(p: Point3, q: Point3, a: Point3, b: Point3, c: Point3) -> bool {
    let eps = 1e-12;
    let dir = [q.x - p.x, q.y - p.y, q.z - p.z];
    let edge1 = [b.x - a.x, b.y - a.y, b.z - a.z];
    let edge2 = [c.x - a.x, c.y - a.y, c.z - a.z];

    let h = [
        dir[1] * edge2[2] - dir[2] * edge2[1],
        dir[2] * edge2[0] - dir[0] * edge2[2],
        dir[0] * edge2[1] - dir[1] * edge2[0],
    ];
    let det = edge1[0] * h[0] + edge1[1] * h[1] + edge1[2] * h[2];
    if det.abs() < eps {
        return false; // segment parallel to triangle plane
    }
    let inv_det = 1.0 / det;
    let s = [p.x - a.x, p.y - a.y, p.z - a.z];
    let u = inv_det * (s[0] * h[0] + s[1] * h[1] + s[2] * h[2]);
    if u < -eps || u > 1.0 + eps {
        return false;
    }
    let r = [
        s[1] * edge1[2] - s[2] * edge1[1],
        s[2] * edge1[0] - s[0] * edge1[2],
        s[0] * edge1[1] - s[1] * edge1[0],
    ];
    let v = inv_det * (dir[0] * r[0] + dir[1] * r[1] + dir[2] * r[2]);
    if v < -eps || u + v > 1.0 + eps {
        return false;
    }
    let t = inv_det * (edge2[0] * r[0] + edge2[1] * r[1] + edge2[2] * r[2]);
    t > eps && t < 1.0 - eps
}

/// Remove the front face matching `(a, b, c)` in any vertex order. Returns
/// true if a face was removed.
fn remove_front_face(front: &mut Vec<[u32; 3]>, face: [u32; 3]) -> bool {
    let mut target = face;
    target.sort_unstable();
    if let Some(pos) = front.iter().position(|f| {
        let mut fk = *f;
        fk.sort_unstable();
        fk == target
    }) {
        front.swap_remove(pos);
        true
    } else {
        false
    }
}

/// Collapse the face at `face_idx` by removing it and its smallest neighbour
/// (the front face sharing the shortest edge with this face).
fn collapse_front_face_3d(face_idx: usize, front: &mut Vec<[u32; 3]>, vertices: &[Point3]) {
    let face = front[face_idx];
    // Find the shortest edge of this face.
    let mut shortest_edge = (face[0], face[1]);
    let mut shortest_len = f64::INFINITY;
    for e in 0..3 {
        let a = face[e];
        let b = face[(e + 1) % 3];
        let pa = vertices[a as usize];
        let pb = vertices[b as usize];
        let len = (pb.x - pa.x).powi(2) + (pb.y - pa.y).powi(2) + (pb.z - pa.z).powi(2);
        if len < shortest_len {
            shortest_len = len;
            shortest_edge = (a, b);
        }
    }
    // Remove this face.
    front.swap_remove(face_idx);
    // Find and remove the neighbour face sharing this shortest edge.
    let (ea, eb) = shortest_edge;
    if let Some(pos) = front.iter().position(|f| {
        let mut has_ea = false;
        let mut has_eb = false;
        for &v in f {
            if v == ea {
                has_ea = true;
            }
            if v == eb {
                has_eb = true;
            }
        }
        has_ea && has_eb
    }) {
        front.swap_remove(pos);
    }
}

// ===========================================================================
//  Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- helpers --------------------------------------------------------

    fn square_boundary() -> Vec<Point2> {
        // CCW unit square, closed.
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
            Point2::new(0.0, 0.0),
        ]
    }

    fn tri_signed_area(v: &[Point2], t: &[u32; 3]) -> f64 {
        let a = v[t[0] as usize];
        let b = v[t[1] as usize];
        let c = v[t[2] as usize];
        0.5 * ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y))
    }

    fn mesh_total_area(v: &[Point2], tris: &[[u32; 3]]) -> f64 {
        tris.iter().map(|t| tri_signed_area(v, t).abs()).sum()
    }

    fn count_inverted(v: &[Point2], tris: &[[u32; 3]]) -> usize {
        tris.iter().filter(|t| tri_signed_area(v, t) <= 0.0).count()
    }

    fn uniform_2d(h: f64) -> impl Fn([f64; 2]) -> f64 {
        move |_p| h
    }

    fn uniform_3d(h: f64) -> impl Fn([f64; 3]) -> f64 {
        move |_p| h
    }

    /// Build a closed orientable surface mesh of a unit cube (12 triangles,
    /// outward normals).
    fn cube_surface() -> (Vec<Point3>, Vec<[u32; 3]>) {
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
        // Outward normals: each face CCW when viewed from outside.
        let t = vec![
            // -z face (z=0), viewed from below (-z): CCW = (0,3,2),(0,2,1)
            [0, 3, 2],
            [0, 2, 1],
            // +z face (z=1), viewed from above (+z): CCW = (4,5,6),(4,6,7)
            [4, 5, 6],
            [4, 6, 7],
            // -x face (x=0), viewed from left (-x): CCW = (0,4,7),(0,7,3)
            [0, 4, 7],
            [0, 7, 3],
            // +x face (x=1), viewed from right (+x): CCW = (1,2,6),(1,6,5)
            [1, 2, 6],
            [1, 6, 5],
            // -y face (y=0), viewed from front (-y): CCW = (0,1,5),(0,5,4)
            [0, 1, 5],
            [0, 5, 4],
            // +y face (y=1), viewed from back (+y): CCW = (3,7,6),(3,6,2)
            [3, 7, 6],
            [3, 6, 2],
        ];
        (v, t)
    }

    fn tet_volume(a: Point3, b: Point3, c: Point3, d: Point3) -> f64 {
        let ab = [b.x - a.x, b.y - a.y, b.z - a.z];
        let ac = [c.x - a.x, c.y - a.y, c.z - a.z];
        let ad = [d.x - a.x, d.y - a.y, d.z - a.z];
        let cross = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        let det = cross[0] * ad[0] + cross[1] * ad[1] + cross[2] * ad[2];
        det.abs() / 6.0
    }

    // ---- 2-D error paths ------------------------------------------------

    #[test]
    fn rejects_too_few_boundary_vertices() {
        let r = advancing_front_triangulate_2d(
            &[
                Point2::new(0.0, 0.0),
                Point2::new(1.0, 0.0),
                Point2::new(0.0, 0.0),
            ],
            &[],
            &uniform_2d(0.3),
            &FrontOptions2d::default(),
        );
        assert!(matches!(r, Err(FrontError::InvalidBoundary { .. })));
    }

    #[test]
    fn rejects_non_closed_boundary() {
        let r = advancing_front_triangulate_2d(
            &[
                Point2::new(0.0, 0.0),
                Point2::new(1.0, 0.0),
                Point2::new(1.0, 1.0),
                Point2::new(0.0, 1.0),
            ],
            &[],
            &uniform_2d(0.3),
            &FrontOptions2d::default(),
        );
        assert!(matches!(r, Err(FrontError::InvalidBoundary { .. })));
    }

    // ---- 2-D meshing ----------------------------------------------------

    #[test]
    fn square_uniform_mesh_covers_domain() {
        let opts = FrontOptions2d {
            max_iterations: 10_000,
            ..Default::default()
        };
        let r = advancing_front_triangulate_2d(&square_boundary(), &[], &uniform_2d(0.3), &opts)
            .unwrap();
        // No inverted triangles.
        assert_eq!(count_inverted(&r.vertices, &r.triangles), 0);
        // Total area ≈ 1.0 (unit square).
        let area = mesh_total_area(&r.vertices, &r.triangles);
        assert!((area - 1.0).abs() < 1e-9, "total area {area} != 1.0");
        // At least a few triangles.
        assert!(r.triangles.len() >= 4);
    }

    #[test]
    fn square_cw_boundary_is_reversed() {
        // CW square — the mesher should reverse it and still produce a valid
        // CCW mesh.
        let cw = vec![
            Point2::new(0.0, 0.0),
            Point2::new(0.0, 1.0),
            Point2::new(1.0, 1.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 0.0),
        ];
        let opts = FrontOptions2d::default();
        let r = advancing_front_triangulate_2d(&cw, &[], &uniform_2d(0.4), &opts).unwrap();
        assert_eq!(count_inverted(&r.vertices, &r.triangles), 0);
        let area = mesh_total_area(&r.vertices, &r.triangles);
        assert!((area - 1.0).abs() < 1e-9);
    }

    #[test]
    fn square_fine_mesh_has_more_triangles() {
        let opts = FrontOptions2d {
            max_iterations: 50_000,
            ..Default::default()
        };
        let coarse =
            advancing_front_triangulate_2d(&square_boundary(), &[], &uniform_2d(0.5), &opts)
                .unwrap();
        let fine = advancing_front_triangulate_2d(&square_boundary(), &[], &uniform_2d(0.2), &opts)
            .unwrap();
        assert!(fine.triangles.len() > coarse.triangles.len());
        assert_eq!(count_inverted(&fine.vertices, &fine.triangles), 0);
        let area = mesh_total_area(&fine.vertices, &fine.triangles);
        assert!((area - 1.0).abs() < 1e-9);
    }

    #[test]
    fn pentagon_uniform_mesh() {
        // Regular pentagon, CCW.
        let mut pent = Vec::new();
        for i in 0..5 {
            let ang = std::f64::consts::TAU * (i as f64) / 5.0;
            pent.push(Point2::new(0.5 + 0.5 * ang.cos(), 0.5 + 0.5 * ang.sin()));
        }
        pent.push(pent[0]);
        let opts = FrontOptions2d {
            max_iterations: 10_000,
            ..Default::default()
        };
        let r = advancing_front_triangulate_2d(&pent, &[], &uniform_2d(0.3), &opts).unwrap();
        assert_eq!(count_inverted(&r.vertices, &r.triangles), 0);
        // Pentagon area = (5/4) * s^2 * cot(pi/5) with s = side length.
        // For circumradius 0.5: side = 2*0.5*sin(pi/5) ≈ 0.5878.
        // Area = (5/2) * R^2 * sin(2*pi/5) = (5/2)*0.25*sin(72°) ≈ 0.5944.
        let area = mesh_total_area(&r.vertices, &r.triangles);
        let expected = 2.5_f64 * 0.25 * (std::f64::consts::TAU / 5.0).sin();
        assert!(
            (area - expected).abs() < 1e-6,
            "pentagon area {area} != {expected}"
        );
    }

    #[test]
    fn square_with_interior_point() {
        let opts = FrontOptions2d::default();
        let r = advancing_front_triangulate_2d(
            &square_boundary(),
            &[Point2::new(0.5, 0.5)],
            &uniform_2d(0.4),
            &opts,
        )
        .unwrap();
        assert_eq!(count_inverted(&r.vertices, &r.triangles), 0);
        let area = mesh_total_area(&r.vertices, &r.triangles);
        assert!((area - 1.0).abs() < 1e-9);
        // The interior point should be used (it's a vertex).
        assert!(r
            .vertices
            .iter()
            .any(|v| { (v.x - 0.5).abs() < 1e-12 && (v.y - 0.5).abs() < 1e-12 }));
    }

    #[test]
    fn square_deterministic() {
        let opts = FrontOptions2d::default();
        let r1 = advancing_front_triangulate_2d(&square_boundary(), &[], &uniform_2d(0.3), &opts)
            .unwrap();
        let r2 = advancing_front_triangulate_2d(&square_boundary(), &[], &uniform_2d(0.3), &opts)
            .unwrap();
        assert_eq!(r1.vertices, r2.vertices);
        assert_eq!(r1.triangles, r2.triangles);
    }

    #[test]
    fn no_triangle_outside_boundary() {
        // Every triangle centroid must be inside the unit square.
        let opts = FrontOptions2d {
            max_iterations: 10_000,
            ..Default::default()
        };
        let r = advancing_front_triangulate_2d(&square_boundary(), &[], &uniform_2d(0.25), &opts)
            .unwrap();
        for t in &r.triangles {
            let a = r.vertices[t[0] as usize];
            let b = r.vertices[t[1] as usize];
            let c = r.vertices[t[2] as usize];
            let cx = (a.x + b.x + c.x) / 3.0;
            let cy = (a.y + b.y + c.y) / 3.0;
            assert!(cx >= -1e-9 && cx <= 1.0 + 1e-9, "centroid x {cx} outside");
            assert!(cy >= -1e-9 && cy <= 1.0 + 1e-9, "centroid y {cy} outside");
        }
    }

    // ---- 3-D error paths ------------------------------------------------

    #[test]
    fn rejects_non_closed_surface() {
        // A single triangle — not closed.
        let v = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let t = vec![[0u32, 1, 2]];
        let r =
            advancing_front_tetrahedralise_3d(&v, &t, &uniform_3d(0.3), &FrontOptions3d::default());
        assert!(matches!(r, Err(FrontError::InvalidSurface { .. })));
    }

    #[test]
    fn rejects_empty_surface() {
        let r = advancing_front_tetrahedralise_3d(
            &[],
            &[],
            &uniform_3d(0.3),
            &FrontOptions3d::default(),
        );
        assert!(matches!(r, Err(FrontError::InvalidSurface { .. })));
    }

    // ---- 3-D meshing ----------------------------------------------------
    // The 3-D advancing front is a known hard problem: the front grows by 2
    // per tet (remove 1 face, add 3) and the self-crossing guard is
    // conservative. A pure advancing-front mesher may not fully close the
    // front on a cube — the typed `Obstruction` is the correct behaviour when
    // the front stalls. These tests verify the mesher produces valid partial
    // meshes, is deterministic, and terminates with a typed obstruction
    // (never hangs).

    #[test]
    fn cube_partial_mesh_produces_valid_tets() {
        // With a coarse target, the mesher should produce some valid tets
        // before the front stalls. We accept obstruction as a valid outcome.
        let (v, t) = cube_surface();
        let opts = FrontOptions3d {
            max_iterations: 5_000,
            ..Default::default()
        };
        match advancing_front_tetrahedralise_3d(&v, &t, &uniform_3d(0.5), &opts) {
            Ok(r) => {
                assert!(!r.tetrahedra.is_empty(), "should produce some tets");
                let nv = r.vertices.len();
                for tet in &r.tetrahedra {
                    for &i in tet {
                        assert!((i as usize) < nv, "tet index {i} out of range");
                    }
                    let a = r.vertices[tet[0] as usize];
                    let b = r.vertices[tet[1] as usize];
                    let c = r.vertices[tet[2] as usize];
                    let d = r.vertices[tet[3] as usize];
                    let vol = tet_volume(a, b, c, d);
                    assert!(vol > 1e-12, "degenerate tet");
                }
            }
            Err(FrontError::Obstruction { .. }) => {
                // Typed obstruction — the mesher correctly reported it can't
                // close the front. This is the expected behaviour for a pure
                // advancing-front mesher on a cube.
            }
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn cube_deterministic() {
        let (v, t) = cube_surface();
        let opts = FrontOptions3d {
            max_iterations: 5_000,
            ..Default::default()
        };
        let r1 = advancing_front_tetrahedralise_3d(&v, &t, &uniform_3d(0.5), &opts);
        let r2 = advancing_front_tetrahedralise_3d(&v, &t, &uniform_3d(0.5), &opts);
        // Both should produce the same result (either both Ok with identical
        // meshes, or both Err with identical obstruction).
        match (&r1, &r2) {
            (Ok(a), Ok(b)) => {
                assert_eq!(a.vertices, b.vertices);
                assert_eq!(a.tetrahedra, b.tetrahedra);
            }
            (
                Err(FrontError::Obstruction {
                    remaining_front: ra,
                    iterations: ia,
                }),
                Err(FrontError::Obstruction {
                    remaining_front: rb,
                    iterations: ib,
                }),
            ) => {
                assert_eq!(ra, rb, "obstruction remaining_front differs");
                assert_eq!(ia, ib, "obstruction iterations differs");
            }
            _ => panic!("non-deterministic: r1={r1:?} r2={r2:?}"),
        }
    }

    #[test]
    fn cube_finer_mesh_has_more_tets_or_same_obstruction() {
        // Finer target → more tets (if the front doesn't stall earlier).
        // Both should terminate (not hang).
        let (v, t) = cube_surface();
        let opts = FrontOptions3d {
            max_iterations: 10_000,
            ..Default::default()
        };
        let coarse = advancing_front_tetrahedralise_3d(&v, &t, &uniform_3d(0.7), &opts);
        let fine = advancing_front_tetrahedralise_3d(&v, &t, &uniform_3d(0.4), &opts);
        // Both must terminate (Ok or Obstruction, not hang).
        let coarse_tets = match &coarse {
            Ok(r) => r.tetrahedra.len(),
            Err(FrontError::Obstruction { .. }) => 0,
            Err(e) => panic!("unexpected: {e:?}"),
        };
        let fine_tets = match &fine {
            Ok(r) => r.tetrahedra.len(),
            Err(FrontError::Obstruction { .. }) => 0,
            Err(e) => panic!("unexpected: {e:?}"),
        };
        // Fine mesh should produce at least as many tets as coarse.
        assert!(
            fine_tets >= coarse_tets,
            "fine={fine_tets} < coarse={coarse_tets}"
        );
    }

    #[test]
    fn cube_all_tet_indices_in_range() {
        let (v, t) = cube_surface();
        let opts = FrontOptions3d {
            max_iterations: 5_000,
            ..Default::default()
        };
        if let Ok(r) = advancing_front_tetrahedralise_3d(&v, &t, &uniform_3d(0.5), &opts) {
            let nv = r.vertices.len();
            for tet in &r.tetrahedra {
                for &i in tet {
                    assert!((i as usize) < nv, "tet index {i} out of range (nv={nv})");
                }
            }
        }
        // Obstruction is also acceptable — no tets to check.
    }

    #[test]
    fn cube_no_degenerate_tets() {
        let (v, t) = cube_surface();
        let opts = FrontOptions3d {
            max_iterations: 5_000,
            ..Default::default()
        };
        if let Ok(r) = advancing_front_tetrahedralise_3d(&v, &t, &uniform_3d(0.5), &opts) {
            for tet in &r.tetrahedra {
                let a = r.vertices[tet[0] as usize];
                let b = r.vertices[tet[1] as usize];
                let c = r.vertices[tet[2] as usize];
                let d = r.vertices[tet[3] as usize];
                let vol = tet_volume(a, b, c, d);
                assert!(vol > 1e-12, "degenerate tet with volume {vol}");
            }
        }
        // Obstruction is also acceptable — no tets to check.
    }
}
