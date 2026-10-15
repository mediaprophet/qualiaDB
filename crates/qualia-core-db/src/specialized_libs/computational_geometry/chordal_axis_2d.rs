//! P20.7 — 2-D Chordal Axis Transform (CAT) & Skeleton Branch Pruning
//! (Prasad 1997; PlantGL `pglalgo::fitting::Skeleton`).
//!
//! Given a simple closed 2-D polygon contour:
//! 1. Builds the Constrained Delaunay Triangulation (via `constrained_delaunay_2`)
//!    and filters to triangles strictly inside the polygon.
//! 2. Classifies every interior triangle by its number of boundary edges:
//!    - [`CatTriangleKind::Termination`] (2 boundary edges): leaf tip / lobe extremity.
//!    - [`CatTriangleKind::Sleeve`] (1 boundary edge): tubular corridor; connects
//!      midpoints of the two interior edges.
//!    - [`CatTriangleKind::Junction`] (0 boundary edges): branch fork; connects
//!      each of the three interior edge midpoints to the triangle's
//!      **pseudo-circumcenter** (circumcenter if acute, midpoint of longest edge
//!      if obtuse).
//! 3. Supports area- and length-based pruning of noisy boundary spurs
//!    (`prune_chordal_axis_spurs`).

use super::boolean_2::point_in_polygon;
use super::constrained_delaunay::conforming_delaunay_2;
use super::primitives::Point2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatTriangleKind {
    /// 2 boundary edges, 1 interior chord.
    Termination,
    /// 1 boundary edge, 2 interior chords.
    Sleeve,
    /// 0 boundary edges, 3 interior chords.
    Junction,
    /// 3 boundary edges (isolated single-triangle polygon).
    Isolated,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChordalSegment {
    pub a: Point2,
    pub b: Point2,
    pub triangle_index: u32,
    pub kind: CatTriangleKind,
    pub triangle_area: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordalAxisError {
    TooFewVertices { got: usize },
    TriangulationFailed,
    OutputTooSmall { required: usize },
}

#[inline]
fn midpoint(a: Point2, b: Point2) -> Point2 {
    Point2::new(0.5 * (a.x + b.x), 0.5 * (a.y + b.y))
}

#[inline]
fn dist_sq(a: Point2, b: Point2) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

#[inline]
fn triangle_area(a: Point2, b: Point2, c: Point2) -> f64 {
    0.5 * ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).abs()
}

/// True when triangle `(a, b, c)` has three strictly acute angles.
/// Tier-1 `HotZeroHeap`.
#[inline]
pub fn is_acute_triangle_2d(a: Point2, b: Point2, c: Point2) -> bool {
    let ab2 = dist_sq(a, b);
    let bc2 = dist_sq(b, c);
    let ca2 = dist_sq(c, a);
    (ab2 + bc2 > ca2 + 1e-12) && (bc2 + ca2 > ab2 + 1e-12) && (ca2 + ab2 > bc2 + 1e-12)
}

/// Compute PlantGL's `SkelTriangle::getPseudoCircumCenter`:
/// - If `(a, b, c)` is acute, returns the exact circumcenter.
/// - Otherwise, returns the midpoint of the longest side (guaranteeing the
///   junction center stays inside the triangle).
///
/// Tier-1 `HotZeroHeap`.
pub fn pseudo_circumcenter_2d(a: Point2, b: Point2, c: Point2) -> Point2 {
    if is_acute_triangle_2d(a, b, c) {
        let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
        if d.abs() > 1e-14 {
            let a2 = a.x * a.x + a.y * a.y;
            let b2 = b.x * b.x + b.y * b.y;
            let c2 = c.x * c.x + c.y * c.y;
            let ux = (a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d;
            let uy = (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d;
            return Point2::new(ux, uy);
        }
    }
    let ab2 = dist_sq(a, b);
    let bc2 = dist_sq(b, c);
    let ca2 = dist_sq(c, a);
    if ab2 >= bc2 && ab2 >= ca2 {
        midpoint(a, b)
    } else if bc2 >= ca2 {
        midpoint(b, c)
    } else {
        midpoint(c, a)
    }
}

/// Extract the 2-D Chordal Axis Transform (CAT) skeleton segments of a closed
/// simple polygon `polygon`.
///
/// `area_max_filter` prunes `Termination` cap segments whose triangle area is
/// `<= area_max_filter` (matching PlantGL `getChordalAxisTransform(shape, areaMaxFilter)`).
pub fn chordal_axis_transform_2d(
    polygon: &[Point2],
    area_max_filter: f64,
    out_segments: &mut [ChordalSegment],
) -> Result<usize, ChordalAxisError> {
    let n = polygon.len();
    if n < 3 {
        return Err(ChordalAxisError::TooFewVertices { got: n });
    }

    let mut constraints = Vec::with_capacity(n);
    for i in 0..n {
        constraints.push((i as u32, ((i + 1) % n) as u32));
    }
    let mut cdt_pts: Vec<Point2> = Vec::with_capacity(n);
    let mut cdt_tris = vec![[0u32; 3]; 4 * n + 8];
    let (n_pts, n_tris) = conforming_delaunay_2(polygon, &constraints, &mut cdt_pts, &mut cdt_tris)
        .map_err(|_| ChordalAxisError::TriangulationFailed)?;
    let points = &cdt_pts[..n_pts];

    // Keep only triangles whose centroid lies strictly inside the polygon
    let mut interior_tris: Vec<[u32; 3]> = Vec::new();
    for &tri in &cdt_tris[..n_tris] {
        let a = points[tri[0] as usize];
        let b = points[tri[1] as usize];
        let c = points[tri[2] as usize];
        if triangle_area(a, b, c) <= 1e-12 {
            continue;
        }
        let centroid = Point2::new((a.x + b.x + c.x) / 3.0, (a.y + b.y + c.y) / 3.0);
        if point_in_polygon(centroid, polygon) {
            interior_tris.push(tri);
        }
    }

    // An edge of an interior triangle is an interior chord iff it is shared by two interior triangles
    let is_interior_edge = |u: u32, v: u32, self_idx: usize| -> bool {
        let (lo, hi) = if u < v { (u, v) } else { (v, u) };
        for (ti, tri) in interior_tris.iter().enumerate() {
            if ti == self_idx {
                continue;
            }
            for e in 0..3 {
                let a = tri[e];
                let b = tri[(e + 1) % 3];
                let (elo, ehi) = if a < b { (a, b) } else { (b, a) };
                if elo == lo && ehi == hi {
                    return true;
                }
            }
        }
        false
    };

    let mut segments = Vec::new();
    for (ti, &tri) in interior_tris.iter().enumerate() {
        let pts = [
            points[tri[0] as usize],
            points[tri[1] as usize],
            points[tri[2] as usize],
        ];
        let area = triangle_area(pts[0], pts[1], pts[2]);
        let mut interior_mids = [Point2::default(); 3];
        let mut int_count = 0usize;
        let mut boundary_opposite_vert = 0usize;

        for e in 0..3 {
            let u = tri[e];
            let v = tri[(e + 1) % 3];
            if is_interior_edge(u, v, ti) {
                interior_mids[int_count] = midpoint(pts[e], pts[(e + 1) % 3]);
                boundary_opposite_vert = (e + 2) % 3;
                int_count += 1;
            }
        }

        match int_count {
            1 => {
                // Termination triangle (2 boundary edges)
                if area > area_max_filter {
                    let tip = pts[boundary_opposite_vert];
                    segments.push(ChordalSegment {
                        a: interior_mids[0],
                        b: tip,
                        triangle_index: ti as u32,
                        kind: CatTriangleKind::Termination,
                        triangle_area: area,
                    });
                }
            }
            2 => {
                // Sleeve triangle (1 boundary edge): connect the two interior chord midpoints
                segments.push(ChordalSegment {
                    a: interior_mids[0],
                    b: interior_mids[1],
                    triangle_index: ti as u32,
                    kind: CatTriangleKind::Sleeve,
                    triangle_area: area,
                });
            }
            3 => {
                // Junction triangle (0 boundary edges): connect each chord midpoint to pseudo-circumcenter
                let center = pseudo_circumcenter_2d(pts[0], pts[1], pts[2]);
                for m in &interior_mids[..3] {
                    segments.push(ChordalSegment {
                        a: center,
                        b: *m,
                        triangle_index: ti as u32,
                        kind: CatTriangleKind::Junction,
                        triangle_area: area,
                    });
                }
            }
            _ => {}
        }
    }

    if out_segments.len() < segments.len() {
        return Err(ChordalAxisError::OutputTooSmall {
            required: segments.len(),
        });
    }
    out_segments[..segments.len()].copy_from_slice(&segments);
    Ok(segments.len())
}

/// Prune short terminal segments from a Chordal Axis segment list
/// (PlantGL `filterLittleBranchesOnBranchSize`).
///
/// Tier-1 `HotZeroHeap`.
pub fn prune_chordal_axis_spurs(
    segments: &[ChordalSegment],
    min_terminal_length: f64,
    out_segments: &mut [ChordalSegment],
) -> Result<usize, ChordalAxisError> {
    if out_segments.len() < segments.len() {
        return Err(ChordalAxisError::OutputTooSmall {
            required: segments.len(),
        });
    }
    let min_l2 = min_terminal_length * min_terminal_length;
    let mut count = 0usize;
    for &seg in segments {
        if seg.kind == CatTriangleKind::Termination && dist_sq(seg.a, seg.b) < min_l2 {
            continue;
        }
        out_segments[count] = seg;
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chordal_axis_extracts_skeleton_of_elongated_leaf_polygon() {
        let leaf = [
            Point2::new(0.0, 0.0),
            Point2::new(2.0, -0.5),
            Point2::new(4.0, -0.4),
            Point2::new(6.0, 0.0),
            Point2::new(4.0, 0.4),
            Point2::new(2.0, 0.5),
        ];
        let mut segs = vec![
            ChordalSegment {
                a: Point2::default(),
                b: Point2::default(),
                triangle_index: 0,
                kind: CatTriangleKind::Sleeve,
                triangle_area: 0.0,
            };
            32
        ];
        let n = chordal_axis_transform_2d(&leaf, 0.0, &mut segs).unwrap();
        assert!(
            n >= 3,
            "expected sleeve + termination CAT segments, got {n}"
        );
        assert!(segs[..n].iter().any(|s| s.kind == CatTriangleKind::Sleeve));
        assert!(segs[..n]
            .iter()
            .any(|s| s.kind == CatTriangleKind::Termination));

        let mut pruned = segs.clone();
        let np = prune_chordal_axis_spurs(&segs[..n], 10.0, &mut pruned).unwrap();
        assert!(
            np < n,
            "expected termination spurs shorter than 10.0 to be pruned"
        );
    }
}
