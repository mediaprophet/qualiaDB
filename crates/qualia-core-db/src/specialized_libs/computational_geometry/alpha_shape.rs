//! P6.2 — Alpha shapes (2D) and alpha-wrap surface extraction.
//!
//! An alpha shape is a subgraph of the Delaunay triangulation: a triangle
//! (or edge) belongs to the alpha shape if its circumradius is ≤ α and it
//! is not "covered" by a larger triangle. The alpha shape generalises the
//! convex hull: as α → ∞ it becomes the convex hull; as α → 0 it becomes
//! the empty set. At intermediate α it captures the "shape" of a point set.
//!
//! ## Classification
//!
//! For each Delaunay triangle:
//! - **Interior** (regular): circumradius ≤ α and the triangle is not on
//!   the convex hull boundary.
//! - **Regular** (boundary): circumradius ≤ α and the triangle is on the
//!   convex hull boundary, or an edge is singular (belongs to only one
//!   triangle with circumradius ≤ α).
//! - **Singular**: an edge with circumradius ≤ α that belongs to no
//!   triangle with circumradius ≤ α.
//!
//! ## Determinism
//!
//! All output is deterministic: triangles/edges are sorted by canonical
//! index order. Identical input → bit-identical output.
//!
//! ## Zero heap
//!
//! All functions use caller-supplied buffers. No `Vec` in hot paths.

use super::delaunay_2::{delaunay_triangulation_2, DelaunayError};
use super::primitives::{Point2, Point3};
use super::voronoi_2::circumcenter;

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

/// Alpha shape error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlphaShapeError {
    /// Too few points for a triangulation.
    TooFewPoints { got: usize },
    /// Delaunay triangulation failed.
    DelaunayFailed(DelaunayError),
    /// Output buffer too small.
    BufferTooSmall { needed: usize, have: usize },
    /// Alpha value is not finite.
    NonFiniteAlpha,
}

impl core::fmt::Display for AlphaShapeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooFewPoints { got } => write!(f, "alpha_shape: too few points: {got}"),
            Self::DelaunayFailed(e) => write!(f, "alpha_shape: delaunay failed: {e:?}"),
            Self::BufferTooSmall { needed, have } => {
                write!(f, "alpha_shape: buffer too small, need {needed}, have {have}")
            }
            Self::NonFiniteAlpha => write!(f, "alpha_shape: alpha is not finite"),
        }
    }
}

impl std::error::Error for AlphaShapeError {}

// ───────────────────────────────────────────────────────────────────────────
//  Types
// ───────────────────────────────────────────────────────────────────────────

/// Classification of a Delaunay triangle in the alpha shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriangleClass {
    /// Interior: circumradius ≤ α, part of the alpha shape interior.
    Interior,
    /// Regular: on the boundary of the alpha shape.
    Regular,
    /// Exterior: circumradius > α, not part of the alpha shape.
    Exterior,
}

/// Classification of a Delaunay edge in the alpha shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeClass {
    /// Interior edge: shared by two alpha triangles.
    Interior,
    /// Boundary edge: belongs to exactly one alpha triangle.
    Boundary,
    /// Singular edge: no alpha triangle contains it, but its half-radius
    /// (circumradius of the edge as a "degenerate triangle") ≤ α.
    Singular,
    /// Exterior: not part of the alpha shape.
    Exterior,
}

/// An alpha-shape edge: (i, j) with i < j, plus classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct AlphaEdge {
    pub i: u32,
    pub j: u32,
    pub class: u8, // 0=Interior, 1=Boundary, 2=Singular, 3=Exterior
}

/// Alpha shape result: classified triangles and edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlphaShapeReport {
    /// Number of interior triangles.
    pub interior_triangles: usize,
    /// Number of regular (boundary) triangles.
    pub regular_triangles: usize,
    /// Number of exterior triangles.
    pub exterior_triangles: usize,
    /// Number of boundary edges.
    pub boundary_edges: usize,
    /// Number of singular edges.
    pub singular_edges: usize,
}

// ───────────────────────────────────────────────────────────────────────────
//  Alpha shape computation (2D)
// ───────────────────────────────────────────────────────────────────────────

/// Compute the 2D alpha shape for a point set at radius `alpha`.
///
/// Writes triangle classifications into `out_tri_classes` (one per Delaunay
/// triangle) and alpha-shape edges into `out_edges`.
///
/// `scratch_delaunay` needs `n` entries.
/// `out_triangles` needs `max_triangles(n)` entries.
/// `out_tri_classes` needs `max_triangles(n)` entries.
/// `out_edges` needs `max_triangles(n) * 3` entries (upper bound).
///
/// Returns `(triangle_count, edge_count, report)`.
pub fn alpha_shape_2d(
    points: &[Point2],
    alpha: f64,
    scratch_delaunay: &mut [u32],
    out_triangles: &mut [[u32; 3]],
    out_tri_classes: &mut [TriangleClass],
    out_edges: &mut [AlphaEdge],
) -> Result<(usize, usize, AlphaShapeReport), AlphaShapeError> {
    if !alpha.is_finite() || alpha <= 0.0 {
        return Err(AlphaShapeError::NonFiniteAlpha);
    }
    if points.len() < 3 {
        return Err(AlphaShapeError::TooFewPoints { got: points.len() });
    }

    let n = points.len();
    let max_tris = max_triangles(n);

    if out_tri_classes.len() < max_tris {
        return Err(AlphaShapeError::BufferTooSmall {
            needed: max_tris,
            have: out_tri_classes.len(),
        });
    }
    if out_edges.len() < max_tris * 3 {
        return Err(AlphaShapeError::BufferTooSmall {
            needed: max_tris * 3,
            have: out_edges.len(),
        });
    }

    // Compute Delaunay triangulation.
    let tri_count = delaunay_triangulation_2(points, scratch_delaunay, out_triangles)
        .map_err(AlphaShapeError::DelaunayFailed)?;

    let alpha_sq = alpha * alpha;

    // Classify triangles by circumradius.
    let mut interior = 0usize;
    let mut regular = 0usize;
    let mut exterior = 0usize;

    for t in 0..tri_count {
        let [ia, ib, ic] = out_triangles[t];
        let a = points[ia as usize];
        let b = points[ib as usize];
        let c = points[ic as usize];
        let cc = circumcenter(a, b, c);
        let r_sq = (cc.x - a.x).powi(2) + (cc.y - a.y).powi(2);
        if r_sq <= alpha_sq {
            out_tri_classes[t] = TriangleClass::Interior;
            interior += 1;
        } else {
            out_tri_classes[t] = TriangleClass::Exterior;
            exterior += 1;
        }
    }

    // Build edge list from triangles.
    // Each triangle has 3 edges; we collect (min, max) pairs.
    let mut edge_count = 0usize;
    for t in 0..tri_count {
        let [ia, ib, ic] = out_triangles[t];
        for &(u, v) in &[(ia, ib), (ib, ic), (ia, ic)] {
            let (a, b) = if u < v { (u, v) } else { (v, u) };
            out_edges[edge_count] = AlphaEdge {
                i: a,
                j: b,
                class: 3, // Exterior by default
            };
            edge_count += 1;
        }
    }

    // Sort edges by (i, j) and deduplicate, counting triangle membership.
    out_edges[..edge_count].sort_unstable();

    // Count how many of these edges come from interior triangles.
    // For each interior triangle, check if it contains this edge.
    let mut write = 0usize;
    let mut i = 0usize;
    while i < edge_count {
        let cur = out_edges[i];
        let mut j = i;
        while j < edge_count && out_edges[j].i == cur.i && out_edges[j].j == cur.j {
            j += 1;
        }

        let mut alpha_tri_count = 0usize;
        let ei = cur.i;
        let ej = cur.j;
        for t in 0..tri_count {
            if out_tri_classes[t] != TriangleClass::Interior {
                continue;
            }
            let [ia, ib, ic] = out_triangles[t];
            let has_edge = (ia == ei && ib == ej) || (ia == ei && ic == ej) || (ib == ei && ic == ej)
                || (ia == ej && ib == ei) || (ia == ej && ic == ei) || (ib == ej && ic == ei);
            if has_edge {
                alpha_tri_count += 1;
            }
        }

        let class = if alpha_tri_count >= 2 {
            0u8 // Interior
        } else if alpha_tri_count == 1 {
            1u8 // Boundary
        } else {
            // Check if the edge itself has "radius" ≤ alpha
            // (i.e., half the edge length ≤ alpha)
            let pa = points[ei as usize];
            let pb = points[ej as usize];
            let half_len_sq = ((pa.x - pb.x).powi(2) + (pa.y - pb.y).powi(2)) / 4.0;
            if half_len_sq <= alpha_sq {
                2u8 // Singular
            } else {
                3u8 // Exterior
            }
        };

        out_edges[write] = AlphaEdge {
            i: cur.i,
            j: cur.j,
            class,
        };
        write += 1;
        i = j;
    }
    edge_count = write;

    // Count edge classes.
    let mut boundary_edges = 0usize;
    let mut singular_edges = 0usize;
    for e in &out_edges[..edge_count] {
        match e.class {
            1 => boundary_edges += 1,
            2 => singular_edges += 1,
            _ => {}
        }
    }

    // Regular triangles: interior triangles that have at least one boundary edge.
    for t in 0..tri_count {
        if out_tri_classes[t] == TriangleClass::Interior {
            let [ia, ib, ic] = out_triangles[t];
            for &(u, v) in &[(ia, ib), (ib, ic), (ia, ic)] {
                let (a, b) = if u < v { (u, v) } else { (v, u) };
                // Binary search for this edge.
                let found = out_edges[..edge_count].binary_search_by(|e| {
                    (e.i, e.j).cmp(&(a, b))
                });
                if let Ok(idx) = found {
                    if out_edges[idx].class == 1 || out_edges[idx].class == 2 {
                        out_tri_classes[t] = TriangleClass::Regular;
                        interior -= 1;
                        regular += 1;
                        break;
                    }
                }
            }
        }
    }

    Ok((
        tri_count,
        edge_count,
        AlphaShapeReport {
            interior_triangles: interior,
            regular_triangles: regular,
            exterior_triangles: exterior,
            boundary_edges,
            singular_edges,
        },
    ))
}

// ───────────────────────────────────────────────────────────────────────────
//  3D alpha shape (via Delaunay tetrahedralization)
// ───────────────────────────────────────────────────────────────────────────

/// Compute the 3D alpha shape classification for a point set.
///
/// This classifies Delaunay tetrahedra by their circumsphere radius:
/// tetrahedra with circumsphere radius ≤ α are interior; the boundary
/// triangles (faces of interior tetrahedra not shared with another interior
/// tetrahedron) form the alpha surface.
///
/// `out_tetra_classes` needs `max_tetrahedra(n)` entries.
/// `out_boundary_tris` needs `max_tetrahedra(n) * 4` entries (upper bound).
///
/// Returns `(tetra_count, boundary_tri_count)`.
pub fn alpha_shape_3d(
    points: &[Point3],
    alpha: f64,
    tetrahedra: &[[u32; 4]],
    out_tetra_classes: &mut [bool], // true = interior (alpha tetrahedron)
    out_boundary_tris: &mut [[u32; 3]],
) -> Result<(usize, usize), AlphaShapeError> {
    if !alpha.is_finite() || alpha <= 0.0 {
        return Err(AlphaShapeError::NonFiniteAlpha);
    }
    if points.len() < 4 {
        return Err(AlphaShapeError::TooFewPoints { got: points.len() });
    }

    let alpha_sq = alpha * alpha;
    let tetra_count = tetrahedra.len();

    if out_tetra_classes.len() < tetra_count {
        return Err(AlphaShapeError::BufferTooSmall {
            needed: tetra_count,
            have: out_tetra_classes.len(),
        });
    }
    if out_boundary_tris.len() < tetra_count * 4 {
        return Err(AlphaShapeError::BufferTooSmall {
            needed: tetra_count * 4,
            have: out_boundary_tris.len(),
        });
    }

    // Classify tetrahedra by circumsphere radius.
    for t in 0..tetra_count {
        let [ia, ib, ic, id] = tetrahedra[t];
        let a = points[ia as usize];
        let b = points[ib as usize];
        let c = points[ic as usize];
        let d = points[id as usize];
        let r_sq = circumsphere_radius_sq(a, b, c, d);
        out_tetra_classes[t] = r_sq <= alpha_sq;
    }

    // Collect boundary triangles: faces of interior tetrahedra not shared
    // with another interior tetrahedron.
    let mut tri_count = 0usize;
    for t in 0..tetra_count {
        if !out_tetra_classes[t] {
            continue;
        }
        let [ia, ib, ic, id] = tetrahedra[t];
        // Each tetrahedron has 4 faces (oriented outward):
        for &(u, v, w) in &[(ia, ib, ic), (ia, ic, id), (ia, id, ib), (ib, id, ic)] {
            let mut face = [u, v, w];
            face.sort_unstable();
            // Check if any other interior tetrahedron shares this face.
            let mut shared = false;
            for t2 in 0..tetra_count {
                if t2 == t || !out_tetra_classes[t2] {
                    continue;
                }
                let [ja, jb, jc, jd] = tetrahedra[t2];
                let mut faces2 = [
                    [ja.min(jb), ja.max(jb), jc], // wrong — need proper face extraction
                ];
                // Actually, let's just check all 4 faces of t2.
                for &(u2, v2, w2) in &[(ja, jb, jc), (ja, jc, jd), (ja, jd, jb), (jb, jd, ic)] {
                    let mut f2 = [u2, v2, w2];
                    f2.sort_unstable();
                    if f2 == face {
                        shared = true;
                        break;
                    }
                }
                if shared {
                    break;
                }
                let _ = &mut faces2; // suppress warning
            }
            if !shared {
                out_boundary_tris[tri_count] = face;
                tri_count += 1;
            }
        }
    }

    // Sort boundary triangles for determinism.
    out_boundary_tris[..tri_count].sort_unstable();
    if tri_count > 1 {
        let mut write = 1usize;
        for read in 1..tri_count {
            if out_boundary_tris[read] != out_boundary_tris[write - 1] {
                out_boundary_tris[write] = out_boundary_tris[read];
                write += 1;
            }
        }
        tri_count = write;
    }

    Ok((tetra_count, tri_count))
}

// ───────────────────────────────────────────────────────────────────────────
//  Helpers
// ───────────────────────────────────────────────────────────────────────────

/// Maximum number of triangles in a 2D Delaunay triangulation of n points.
/// Matches delaunay_2's internal formula: 2n + 1 (upper bound including super-triangle).
#[inline]
pub fn max_triangles(n: usize) -> usize {
    if n < 2 { 1 } else { 2 * n + 1 }
}

/// Circumsphere radius squared for a tetrahedron (f64 approximation).
fn circumsphere_radius_sq(a: Point3, b: Point3, c: Point3, d: Point3) -> f64 {
    // Use the Cayley-Menger determinant for the circumradius.
    // R² = |M| / (2 * |N|²) where M and N are sub-determinants.
    // For simplicity, use the formula via the circumcenter.
    let ab = Point3::new(b.x - a.x, b.y - a.y, b.z - a.z);
    let ac = Point3::new(c.x - a.x, c.y - a.y, c.z - a.z);
    let ad = Point3::new(d.x - a.x, d.y - a.y, d.z - a.z);

    // Solve for circumcenter: o = a + x*ab + y*ac + z*ad
    // where |o - a|² = |o - b|² = |o - c|² = |o - d|²
    // This gives: 2*ab·(o-a) = |ab|², etc.
    let ab_sq = dot_3d(ab, ab);
    let ac_sq = dot_3d(ac, ac);
    let ad_sq = dot_3d(ad, ad);

    let det = dot_3d(ab, cross_3d(ac, ad));
    if det.abs() < 1e-20 {
        return f64::INFINITY; // Degenerate
    }

    // Circumcenter relative to a.
    // We compute the circumcenter directly.
    // For a tetrahedron, the circumcenter is equidistant from all 4 vertices.
    // We solve the linear system: 2*(b-a)·x = |b|²-|a|², etc.
    // With a as origin: 2*ab·x = |ab|², 2*ac·x = |ac|², 2*ad·x = |ad|².
    // x = M⁻¹ * rhs where M = [2*ab; 2*ac; 2*ad], rhs = [|ab|²; |ac|²; |ad|²].

    // Using Cramer's rule:
    let rhs = [ab_sq, ac_sq, ad_sq];
    let m00 = 2.0 * ab.x;
    let m01 = 2.0 * ab.y;
    let m02 = 2.0 * ab.z;
    let m10 = 2.0 * ac.x;
    let m11 = 2.0 * ac.y;
    let m12 = 2.0 * ac.z;
    let m20 = 2.0 * ad.x;
    let m21 = 2.0 * ad.y;
    let m22 = 2.0 * ad.z;

    let det_m = m00 * (m11 * m22 - m12 * m21)
        - m01 * (m10 * m22 - m12 * m20)
        + m02 * (m10 * m21 - m11 * m20);

    if det_m.abs() < 1e-20 {
        return f64::INFINITY;
    }

    let inv_det_m = 1.0 / det_m;
    let x = (rhs[0] * (m11 * m22 - m12 * m21) - m01 * (rhs[1] * m22 - m12 * rhs[2]) + m02 * (rhs[1] * m21 - m11 * rhs[2])) * inv_det_m;
    let y = (m00 * (rhs[1] * m22 - m12 * rhs[2]) - rhs[0] * (m10 * m22 - m12 * m20) + m02 * (m10 * rhs[2] - rhs[1] * m20)) * inv_det_m;
    let z = (m00 * (m11 * rhs[2] - rhs[1] * m21) - m01 * (m10 * rhs[2] - rhs[1] * m20) + rhs[0] * (m10 * m21 - m11 * m20)) * inv_det_m;

    // R² = |x|² (since a is origin, circumcenter = a + x, R = |x|)
    x * x + y * y + z * z
}

#[inline]
fn dot_3d(a: Point3, b: Point3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

#[inline]
fn cross_3d(a: Point3, b: Point3) -> Point3 {
    Point3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

// ───────────────────────────────────────────────────────────────────────────
//  Determinism hash
// ───────────────────────────────────────────────────────────────────────────

/// FNV-1a hash over alpha-shape edges for determinism verification.
pub fn alpha_shape_hash(edges: &[AlphaEdge]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for e in edges {
        hash ^= e.i as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= e.j as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= e.class as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn square_points() -> Vec<Point2> {
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
            Point2::new(0.5, 0.5),
        ]
    }

    #[allow(dead_code)]
    fn circle_points(n: usize, r: f64) -> Vec<Point2> {
        let mut pts = Vec::with_capacity(n);
        for i in 0..n {
            let angle = 2.0 * core::f64::consts::PI * i as f64 / n as f64;
            pts.push(Point2::new(r * angle.cos(), r * angle.sin()));
        }
        pts
    }

    #[test]
    fn alpha_shape_large_alpha_is_convex_hull() {
        let pts = square_points();
        let n = pts.len();
        let max_tris = max_triangles(n);
        let mut scratch = vec![0u32; n];
        let mut tris = vec![[0u32; 3]; max_tris];
        let mut tri_classes = vec![TriangleClass::Exterior; max_tris];
        let mut edges = vec![AlphaEdge::default(); max_tris * 3];

        // Very large alpha → all triangles are interior → convex hull.
        let (tc, _ec, report) = alpha_shape_2d(
            &pts, 100.0, &mut scratch, &mut tris, &mut tri_classes, &mut edges,
        ).unwrap();

        assert!(tc > 0, "should have triangles");
        assert_eq!(report.exterior_triangles, 0, "large alpha → no exterior triangles");
    }

    #[test]
    fn alpha_shape_small_alpha_is_empty() {
        let pts = square_points();
        let n = pts.len();
        let max_tris = max_triangles(n);
        let mut scratch = vec![0u32; n];
        let mut tris = vec![[0u32; 3]; max_tris];
        let mut tri_classes = vec![TriangleClass::Exterior; max_tris];
        let mut edges = vec![AlphaEdge::default(); max_tris * 3];

        // Very small alpha → no triangles are interior.
        let (tc, _ec, report) = alpha_shape_2d(
            &pts, 0.01, &mut scratch, &mut tris, &mut tri_classes, &mut edges,
        ).unwrap();

        assert!(tc > 0, "Delaunay should produce triangles");
        assert_eq!(report.interior_triangles + report.regular_triangles, 0,
            "small alpha → no alpha triangles");
    }

    #[test]
    fn alpha_shape_determinism() {
        // Use jittered circle points to avoid cocircular degeneracy.
        let pts: Vec<Point2> = (0..20).map(|i| {
            let angle = 2.0 * core::f64::consts::PI * i as f64 / 20.0;
            let r = 1.0 + (i as f64 * 0.0001).sin() * 0.01;
            Point2::new(r * angle.cos(), r * angle.sin())
        }).collect();
        let n = pts.len();
        let max_tris = max_triangles(n);

        let mut s1 = vec![0u32; n];
        let mut t1 = vec![[0u32; 3]; max_tris];
        let mut c1 = vec![TriangleClass::Exterior; max_tris];
        let mut e1 = vec![AlphaEdge::default(); max_tris * 3];

        let mut s2 = vec![0u32; n];
        let mut t2 = vec![[0u32; 3]; max_tris];
        let mut c2 = vec![TriangleClass::Exterior; max_tris];
        let mut e2 = vec![AlphaEdge::default(); max_tris * 3];

        let (tc1, ec1, r1) = alpha_shape_2d(&pts, 0.8, &mut s1, &mut t1, &mut c1, &mut e1).unwrap();
        let (tc2, ec2, r2) = alpha_shape_2d(&pts, 0.8, &mut s2, &mut t2, &mut c2, &mut e2).unwrap();

        assert_eq!(tc1, tc2);
        assert_eq!(ec1, ec2);
        assert_eq!(r1, r2);
        assert_eq!(alpha_shape_hash(&e1[..ec1]), alpha_shape_hash(&e2[..ec2]));
    }

    #[test]
    fn alpha_shape_circle_captures_boundary() {
        // Use jittered circle points to avoid cocircular degeneracy.
        let pts: Vec<Point2> = (0..30).map(|i| {
            let angle = 2.0 * core::f64::consts::PI * i as f64 / 30.0;
            let r = 1.0 + (i as f64 * 0.0001).sin() * 0.01; // small jitter
            Point2::new(r * angle.cos(), r * angle.sin())
        }).collect();
        let n = pts.len();
        let max_tris = max_triangles(n);
        let mut scratch = vec![0u32; n];
        let mut tris = vec![[0u32; 3]; max_tris];
        let mut tri_classes = vec![TriangleClass::Exterior; max_tris];
        let mut edges = vec![AlphaEdge::default(); max_tris * 3];

        // Alpha slightly larger than the circumradius of boundary triangles.
        let (_tc, ec, report) = alpha_shape_2d(
            &pts, 1.5, &mut scratch, &mut tris, &mut tri_classes, &mut edges,
        ).unwrap();

        // Should have some boundary edges forming the circle boundary.
        assert!(report.boundary_edges > 0, "circle should have boundary edges");
        assert!(ec > 0, "should have alpha-shape edges");
    }

    #[test]
    fn alpha_shape_too_few_points() {
        let pts = vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)];
        let mut scratch = vec![0u32; 2];
        let mut tris = vec![[0u32; 3]; 1];
        let mut tri_classes = vec![TriangleClass::Exterior; 1];
        let mut edges = vec![AlphaEdge::default(); 3];
        assert!(matches!(
            alpha_shape_2d(&pts, 1.0, &mut scratch, &mut tris, &mut tri_classes, &mut edges),
            Err(AlphaShapeError::TooFewPoints { .. })
        ));
    }

    #[test]
    fn alpha_shape_non_finite_alpha() {
        let pts = square_points();
        let n = pts.len();
        let max_tris = max_triangles(n);
        let mut scratch = vec![0u32; n];
        let mut tris = vec![[0u32; 3]; max_tris];
        let mut tri_classes = vec![TriangleClass::Exterior; max_tris];
        let mut edges = vec![AlphaEdge::default(); max_tris * 3];
        assert!(matches!(
            alpha_shape_2d(&pts, f64::NAN, &mut scratch, &mut tris, &mut tri_classes, &mut edges),
            Err(AlphaShapeError::NonFiniteAlpha)
        ));
    }

    #[test]
    fn alpha_shape_3d_basic() {
        // 5 points: 4 corners of a tetrahedron + 1 center.
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(0.25, 0.25, 0.25),
        ];
        // Simple tetrahedron: first 4 points.
        let tetras = vec![[0u32, 1, 2, 3]];
        let mut classes = vec![false; 1];
        let mut boundary = vec![[0u32; 3]; 4];

        // Large alpha → tetrahedron is interior.
        let (tc, bc) = alpha_shape_3d(&pts, 10.0, &tetras, &mut classes, &mut boundary).unwrap();
        assert_eq!(tc, 1);
        assert!(classes[0], "tetrahedron should be interior with large alpha");
        assert_eq!(bc, 4, "single tetrahedron has 4 boundary faces");
    }

    #[test]
    fn alpha_shape_3d_small_alpha() {
        let pts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        ];
        let tetras = vec![[0u32, 1, 2, 3]];
        let mut classes = vec![false; 1];
        let mut boundary = vec![[0u32; 3]; 4];

        let (tc, bc) = alpha_shape_3d(&pts, 0.01, &tetras, &mut classes, &mut boundary).unwrap();
        assert_eq!(tc, 1);
        assert!(!classes[0], "tetrahedron should be exterior with small alpha");
        assert_eq!(bc, 0, "no boundary faces with small alpha");
    }
}
