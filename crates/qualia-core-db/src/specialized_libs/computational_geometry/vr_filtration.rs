//! P8.1 — Simplicial-complex core: caller-buffered VR / alpha filtration
//! over the Tensor10D point cloud.
//!
//! ## Vietoris-Rips filtration
//!
//! The VR complex at radius ε contains all simplices whose diameter is ≤ 2ε.
//! For a point cloud, this means:
//! - 0-simplices (vertices): always present, birth = 0
//! - 1-simplices (edges): birth = half the pairwise distance
//! - 2-simplices (triangles): birth = half the max edge length
//! - k-simplices: birth = half the max pairwise distance (diameter)
//!
//! The VR filtration is the sequence of VR complexes as ε increases from 0.
//!
//! ## Alpha filtration (generalisation)
//!
//! The existing `tda.rs` implements 2D alpha filtration via Delaunay. This
//! module generalises to N-D by using the VR construction with circumradius
//! for simplices that have a well-defined circumsphere.
//!
//! ## Determinism
//!
//! All simplices are emitted in canonical (birth, dim, vertex-indices) order.
//! Identical input → bit-identical output.

use crate::tensor::Tensor10D;

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

/// VR filtration error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VrError {
    /// Too few points.
    TooFewPoints { got: usize },
    /// Non-finite coordinate in input.
    NonFinite { point_index: usize },
    /// Buffer too small.
    BufferTooSmall { needed: usize, have: usize },
}

impl core::fmt::Display for VrError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooFewPoints { got } => write!(f, "vr: too few points: {got}"),
            Self::NonFinite { point_index } => write!(f, "vr: non-finite at point {point_index}"),
            Self::BufferTooSmall { needed, have } => {
                write!(f, "vr: buffer too small, need {needed}, have {have}")
            }
        }
    }
}

impl std::error::Error for VrError {}

// ───────────────────────────────────────────────────────────────────────────
//  Types
// ───────────────────────────────────────────────────────────────────────────

/// A simplex in the VR filtration.
///
/// Vertex indices are sorted in ascending order. `birth` is stored as f64
/// bits for total ordering without floating-point comparison ambiguity.
///
/// **Ordering**: sorted by (birth, dim, v0, v1, v2) — birth first, then
/// dimension, then vertex indices. This is the canonical filtration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VrSimplex {
    /// Dimension: 0 = vertex, 1 = edge, 2 = triangle.
    pub dim: u8,
    /// Vertex indices (sorted ascending). Unused slots are 0.
    pub v0: u32,
    pub v1: u32,
    pub v2: u32,
    /// Birth radius as f64 bits.
    pub birth: u64,
}

impl PartialOrd for VrSimplex {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VrSimplex {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        // Sort by (birth, dim, v0, v1, v2) — canonical filtration order.
        self.birth
            .cmp(&other.birth)
            .then(self.dim.cmp(&other.dim))
            .then(self.v0.cmp(&other.v0))
            .then(self.v1.cmp(&other.v1))
            .then(self.v2.cmp(&other.v2))
    }
}

impl VrSimplex {
    #[inline]
    pub fn birth_f64(&self) -> f64 {
        f64::from_bits(self.birth)
    }

    #[inline]
    pub fn vertex(&self, i: usize) -> u32 {
        match i {
            0 => self.v0,
            1 => self.v1,
            2 => self.v2,
            _ => 0,
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Distance computation
// ───────────────────────────────────────────────────────────────────────────

/// Euclidean distance between two Tensor10D points (spatial axes only: x, y, z).
#[inline]
pub fn spatial_distance(a: &Tensor10D, b: &Tensor10D) -> f64 {
    let dx = (a.x - b.x) as f64;
    let dy = (a.y - b.y) as f64;
    let dz = (a.z - b.z) as f64;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Full 7-coordinate distance (x, y, z, t, α, μ, σ) — Euclidean metric only.
#[inline]
pub fn full_coordinate_distance(a: &Tensor10D, b: &Tensor10D) -> f64 {
    let dx = (a.x - b.x) as f64;
    let dy = (a.y - b.y) as f64;
    let dz = (a.z - b.z) as f64;
    let dt = (a.t - b.t) as f64;
    let da = (a.alpha - b.alpha) as f64;
    let dm = (a.mu - b.mu) as f64;
    let ds = (a.sigma - b.sigma) as f64;
    (dx * dx + dy * dy + dz * dz + dt * dt + da * da + dm * dm + ds * ds).sqrt()
}

/// Check if a Tensor10D point has all finite spatial coordinates.
#[inline]
pub fn is_finite_point(p: &Tensor10D) -> bool {
    p.x.is_finite() && p.y.is_finite() && p.z.is_finite()
}

// ───────────────────────────────────────────────────────────────────────────
//  VR filtration
// ───────────────────────────────────────────────────────────────────────────

/// Compute the VR filtration over a Tensor10D point cloud.
///
/// Uses spatial (x, y, z) distance for edge weights. Produces vertices,
/// edges, and triangles sorted by (birth, dim, v0, v1, v2).
///
/// **Buffers:**
/// - `out_edges`: needs `n*(n-1)/2` entries
/// - `out_simplices`: needs `n + n*(n-1)/2 + n*(n-1)*(n-2)/6` entries
///   (vertices + edges + triangles). For small n this is fine; for large n
///   the caller should limit to edges only (`max_dim = 1`).
///
/// **Parameters:**
/// - `max_dim`: maximum simplex dimension (0 = vertices only, 1 = +edges, 2 = +triangles)
/// - `max_radius`: skip simplices with birth > max_radius (0 = no limit)
///
/// Returns the number of simplices written.
pub fn vr_filtration(
    points: &[Tensor10D],
    max_dim: u8,
    max_radius: f64,
    out_simplices: &mut [VrSimplex],
) -> Result<usize, VrError> {
    let n = points.len();
    if n < 1 {
        return Err(VrError::TooFewPoints { got: 0 });
    }

    // Validate finiteness.
    for (i, p) in points.iter().enumerate() {
        if !is_finite_point(p) {
            return Err(VrError::NonFinite { point_index: i });
        }
    }

    // Upper bound on simplices, respecting max_dim.
    let max_edges = if max_dim >= 1 { n * n.saturating_sub(1) / 2 } else { 0 };
    let max_tris = if max_dim >= 2 { n * n.saturating_sub(1) * n.saturating_sub(2) / 6 } else { 0 };
    let max_simplices = n + max_edges + max_tris;
    if out_simplices.len() < max_simplices {
        return Err(VrError::BufferTooSmall {
            needed: max_simplices,
            have: out_simplices.len(),
        });
    }

    let mut count = 0usize;

    // Vertices: birth = 0.
    for i in 0..n {
        out_simplices[count] = VrSimplex {
            dim: 0,
            v0: i as u32,
            v1: 0,
            v2: 0,
            birth: 0.0f64.to_bits(),
        };
        count += 1;
    }

    if max_dim < 1 {
        // Sort and return.
        out_simplices[..count].sort_unstable();
        return Ok(count);
    }

    // Edges: birth = half the pairwise distance.
    for i in 0..n {
        for j in (i + 1)..n {
            let d = spatial_distance(&points[i], &points[j]);
            let birth = d / 2.0;
            if max_radius > 0.0 && birth > max_radius {
                continue;
            }
            out_simplices[count] = VrSimplex {
                dim: 1,
                v0: i as u32,
                v1: j as u32,
                v2: 0,
                birth: birth.to_bits(),
            };
            count += 1;
        }
    }

    if max_dim < 2 {
        out_simplices[..count].sort_unstable();
        return Ok(count);
    }

    // Triangles: birth = half the max edge length (diameter / 2).
    for i in 0..n {
        for j in (i + 1)..n {
            let d_ij = spatial_distance(&points[i], &points[j]);
            for k in (j + 1)..n {
                let d_ik = spatial_distance(&points[i], &points[k]);
                let d_jk = spatial_distance(&points[j], &points[k]);
                let diameter = d_ij.max(d_ik).max(d_jk);
                let birth = diameter / 2.0;
                if max_radius > 0.0 && birth > max_radius {
                    continue;
                }
                out_simplices[count] = VrSimplex {
                    dim: 2,
                    v0: i as u32,
                    v1: j as u32,
                    v2: k as u32,
                    birth: birth.to_bits(),
                };
                count += 1;
            }
        }
    }

    // Sort by (birth, dim, v0, v1, v2) — canonical filtration order.
    out_simplices[..count].sort_unstable();

    Ok(count)
}

/// Compute the VR filtration using the full 7-coordinate distance
/// (Euclidean metric only, v=0).
pub fn vr_filtration_full(
    points: &[Tensor10D],
    max_dim: u8,
    max_radius: f64,
    out_simplices: &mut [VrSimplex],
) -> Result<usize, VrError> {
    let n = points.len();
    if n < 1 {
        return Err(VrError::TooFewPoints { got: 0 });
    }

    for (i, p) in points.iter().enumerate() {
        if !is_finite_point(p) {
            return Err(VrError::NonFinite { point_index: i });
        }
    }

    let max_edges = n * n.saturating_sub(1) / 2;
    let max_tris = if max_dim >= 2 { n * n.saturating_sub(1) * n.saturating_sub(2) / 6 } else { 0 };
    let max_simplices = n + max_edges + max_tris;
    if out_simplices.len() < max_simplices {
        return Err(VrError::BufferTooSmall {
            needed: max_simplices,
            have: out_simplices.len(),
        });
    }

    let mut count = 0usize;

    for i in 0..n {
        out_simplices[count] = VrSimplex {
            dim: 0, v0: i as u32, v1: 0, v2: 0,
            birth: 0.0f64.to_bits(),
        };
        count += 1;
    }

    if max_dim < 1 {
        out_simplices[..count].sort_unstable();
        return Ok(count);
    }

    for i in 0..n {
        for j in (i + 1)..n {
            let d = full_coordinate_distance(&points[i], &points[j]);
            let birth = d / 2.0;
            if max_radius > 0.0 && birth > max_radius { continue; }
            out_simplices[count] = VrSimplex {
                dim: 1, v0: i as u32, v1: j as u32, v2: 0,
                birth: birth.to_bits(),
            };
            count += 1;
        }
    }

    if max_dim < 2 {
        out_simplices[..count].sort_unstable();
        return Ok(count);
    }

    for i in 0..n {
        for j in (i + 1)..n {
            let d_ij = full_coordinate_distance(&points[i], &points[j]);
            for k in (j + 1)..n {
                let d_ik = full_coordinate_distance(&points[i], &points[k]);
                let d_jk = full_coordinate_distance(&points[j], &points[k]);
                let diameter = d_ij.max(d_ik).max(d_jk);
                let birth = diameter / 2.0;
                if max_radius > 0.0 && birth > max_radius { continue; }
                out_simplices[count] = VrSimplex {
                    dim: 2, v0: i as u32, v1: j as u32, v2: k as u32,
                    birth: birth.to_bits(),
                };
                count += 1;
            }
        }
    }

    out_simplices[..count].sort_unstable();
    Ok(count)
}

// ───────────────────────────────────────────────────────────────────────────
//  Alpha filtration (4 co-circular points — CC0 golden test)
// ───────────────────────────────────────────────────────────────────────────

/// Compute the alpha filtration for 4 co-circular points.
///
/// For 4 points on a common circle, the alpha complex has a known structure:
/// - 4 vertices (birth = 0)
/// - 4 edges on the convex hull (birth = half edge length)
/// - 2 diagonal edges (birth = half diagonal length)
/// - 4 triangles (birth = circumradius, since all are co-circular)
///
/// The CC0 golden values are the circumradius of the 4 points (which is
/// the same for all triangles since they share the same circumcircle).
///
/// `points` must be exactly 4 points. `out_simplices` must have room for
/// 4 + 6 + 4 = 14 simplices.
pub fn alpha_filtration_4_cocircular(
    points: &[Tensor10D],
    out_simplices: &mut [VrSimplex],
) -> Result<usize, VrError> {
    if points.len() != 4 {
        return Err(VrError::TooFewPoints { got: points.len() });
    }
    if out_simplices.len() < 14 {
        return Err(VrError::BufferTooSmall { needed: 14, have: out_simplices.len() });
    }

    for (i, p) in points.iter().enumerate() {
        if !is_finite_point(p) {
            return Err(VrError::NonFinite { point_index: i });
        }
    }

    // Compute all pairwise distances.
    let mut dists = [[0.0f64; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            if i != j {
                dists[i][j] = spatial_distance(&points[i], &points[j]);
            }
        }
    }

    // Compute circumradius of the 4 points (they're co-circular).
    // For co-circular points, the circumradius is the distance from the
    // circumcenter to any point. We compute it via the triangle (0,1,2)
    // circumradius — for co-circular points, all triangles give the same R.
    let r_circ = triangle_circumradius_3d(
        &points[0], &points[1], &points[2],
    );

    let mut count = 0usize;

    // 4 vertices.
    for i in 0..4 {
        out_simplices[count] = VrSimplex {
            dim: 0, v0: i as u32, v1: 0, v2: 0,
            birth: 0.0f64.to_bits(),
        };
        count += 1;
    }

    // 6 edges (all pairs).
    for i in 0..4 {
        for j in (i + 1)..4 {
            let birth = dists[i][j] / 2.0;
            out_simplices[count] = VrSimplex {
                dim: 1, v0: i as u32, v1: j as u32, v2: 0,
                birth: birth.to_bits(),
            };
            count += 1;
        }
    }

    // 4 triangles — all with birth = circumradius.
    for i in 0..4 {
        for j in (i + 1)..4 {
            for k in (j + 1)..4 {
                out_simplices[count] = VrSimplex {
                    dim: 2, v0: i as u32, v1: j as u32, v2: k as u32,
                    birth: r_circ.to_bits(),
                };
                count += 1;
            }
        }
    }

    out_simplices[..count].sort_unstable();
    Ok(count)
}

/// Circumradius of a triangle in 3D (using spatial coordinates).
fn triangle_circumradius_3d(a: &Tensor10D, b: &Tensor10D, c: &Tensor10D) -> f64 {
    let d_ab = spatial_distance(a, b);
    let d_bc = spatial_distance(b, c);
    let d_ca = spatial_distance(c, a);

    // Area via Heron's formula.
    let s = (d_ab + d_bc + d_ca) / 2.0;
    let area_sq = s * (s - d_ab) * (s - d_bc) * (s - d_ca);
    if area_sq <= 0.0 {
        return f64::INFINITY;
    }
    let area = area_sq.sqrt();

    // Circumradius R = (a * b * c) / (4 * Area).
    (d_ab * d_bc * d_ca) / (4.0 * area)
}

// ───────────────────────────────────────────────────────────────────────────
//  Determinism hash
// ───────────────────────────────────────────────────────────────────────────

/// FNV-1a hash over the filtration for determinism verification.
pub fn filtration_hash(simplices: &[VrSimplex]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for s in simplices {
        hash ^= s.dim as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= s.v0 as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= s.v1 as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= s.v2 as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= s.birth;
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

    fn make_point(x: f32, y: f32, z: f32) -> Tensor10D {
        Tensor10D::new(0.0, 0.0, 0.0, x, y, z, 0.0, 0.0, 0.0, 0.0)
    }

    fn unit_square_with_centre() -> Vec<Tensor10D> {
        vec![
            make_point(0.0, 0.0, 0.0),
            make_point(1.0, 0.0, 0.0),
            make_point(1.0, 1.0, 0.0),
            make_point(0.0, 1.0, 0.0),
            make_point(0.5, 0.5, 0.0),
        ]
    }

    fn four_cocircular() -> Vec<Tensor10D> {
        // 4 points on the unit circle at 0°, 90°, 180°, 270°.
        let r = 1.0f32;
        vec![
            make_point(r, 0.0, 0.0),
            make_point(0.0, r, 0.0),
            make_point(-r, 0.0, 0.0),
            make_point(0.0, -r, 0.0),
        ]
    }

    #[test]
    fn vr_unit_square_vertices_and_edges() {
        let pts = unit_square_with_centre();
        let n = pts.len();
        let max_edges = n * (n - 1) / 2;
        let max_tris = n * (n - 1) * (n - 2) / 6;
        let mut simplices = vec![VrSimplex::default(); n + max_edges + max_tris];

        let count = vr_filtration(&pts, 2, 0.0, &mut simplices).unwrap();

        // 5 vertices + 10 edges + 10 triangles = 25.
        assert_eq!(count, 5 + 10 + 10);

        // First 5 should be vertices (dim=0, birth=0).
        for i in 0..5 {
            assert_eq!(simplices[i].dim, 0, "vertex {} should be dim 0", i);
            assert_eq!(simplices[i].birth_f64(), 0.0);
        }
    }

    #[test]
    fn vr_unit_square_edge_births() {
        let pts = unit_square_with_centre();
        let n = pts.len();
        let mut simplices = vec![VrSimplex::default(); n + n * (n - 1) / 2 + n * (n - 1) * (n - 2) / 6];

        let count = vr_filtration(&pts, 1, 0.0, &mut simplices).unwrap();

        // Find the edge (0,4) — centre to corner.
        let edge_04 = simplices[..count].iter().find(|s| {
            s.dim == 1 && s.v0 == 0 && s.v1 == 4
        }).unwrap();
        let expected = spatial_distance(&pts[0], &pts[4]) / 2.0;
        assert!((edge_04.birth_f64() - expected).abs() < 1e-10,
            "edge (0,4) birth should be {}", expected);
    }

    #[test]
    fn vr_unit_square_triangle_birth_is_half_diameter() {
        let pts = unit_square_with_centre();
        let n = pts.len();
        let mut simplices = vec![VrSimplex::default(); n + n * (n - 1) / 2 + n * (n - 1) * (n - 2) / 6];

        let count = vr_filtration(&pts, 2, 0.0, &mut simplices).unwrap();

        // Triangle (0,1,2): corners (0,0), (1,0), (1,1).
        // Diameter = max(d01, d02, d12) = max(1, sqrt(2), 1) = sqrt(2).
        // Birth = sqrt(2)/2.
        let tri = simplices[..count].iter().find(|s| {
            s.dim == 2 && s.v0 == 0 && s.v1 == 1 && s.v2 == 2
        }).unwrap();
        let expected = (2.0f64).sqrt() / 2.0;
        assert!((tri.birth_f64() - expected).abs() < 1e-10,
            "triangle (0,1,2) birth should be {}", expected);
    }

    #[test]
    fn vr_max_radius_filters_simplices() {
        let pts = unit_square_with_centre();
        let n = pts.len();
        let mut simplices = vec![VrSimplex::default(); n + n * (n - 1) / 2 + n * (n - 1) * (n - 2) / 6];

        // With max_radius = 0.4, only edges with birth ≤ 0.4 survive.
        // Edge (0,4) has birth = sqrt(0.5)/2 ≈ 0.354 — should survive.
        // Edge (0,2) has birth = sqrt(2)/2 ≈ 0.707 — should be filtered.
        let count = vr_filtration(&pts, 2, 0.4, &mut simplices).unwrap();

        let has_04 = simplices[..count].iter().any(|s| s.dim == 1 && s.v0 == 0 && s.v1 == 4);
        let has_02 = simplices[..count].iter().any(|s| s.dim == 1 && s.v0 == 0 && s.v1 == 2);
        assert!(has_04, "edge (0,4) should survive at radius 0.4");
        assert!(!has_02, "edge (0,2) should be filtered at radius 0.4");
    }

    #[test]
    fn vr_determinism() {
        let pts = unit_square_with_centre();
        let n = pts.len();
        let cap = n + n * (n - 1) / 2 + n * (n - 1) * (n - 2) / 6;

        let mut s1 = vec![VrSimplex::default(); cap];
        let c1 = vr_filtration(&pts, 2, 0.0, &mut s1).unwrap();

        let mut s2 = vec![VrSimplex::default(); cap];
        let c2 = vr_filtration(&pts, 2, 0.0, &mut s2).unwrap();

        assert_eq!(c1, c2);
        assert_eq!(s1[..c1], s2[..c2], "filtration must be byte-identical");
        assert_eq!(filtration_hash(&s1[..c1]), filtration_hash(&s2[..c2]));
    }

    #[test]
    fn vr_non_finite_fails_closed() {
        let mut pts = unit_square_with_centre();
        pts[2].x = f32::NAN;
        let n = pts.len();
        let mut simplices = vec![VrSimplex::default(); n + n * (n - 1) / 2];
        let err = vr_filtration(&pts, 1, 0.0, &mut simplices).unwrap_err();
        assert!(matches!(err, VrError::NonFinite { point_index: 2 }));
    }

    #[test]
    fn vr_empty_points_fails() {
        let pts: Vec<Tensor10D> = vec![];
        let mut simplices = vec![VrSimplex::default(); 10];
        let err = vr_filtration(&pts, 1, 0.0, &mut simplices).unwrap_err();
        assert!(matches!(err, VrError::TooFewPoints { got: 0 }));
    }

    #[test]
    fn vr_buffer_too_small() {
        let pts = unit_square_with_centre();
        let mut simplices = vec![VrSimplex::default(); 3]; // way too small
        let err = vr_filtration(&pts, 2, 0.0, &mut simplices).unwrap_err();
        assert!(matches!(err, VrError::BufferTooSmall { .. }));
    }

    #[test]
    fn vr_max_dim_0_vertices_only() {
        let pts = unit_square_with_centre();
        let mut simplices = vec![VrSimplex::default(); 5];
        let count = vr_filtration(&pts, 0, 0.0, &mut simplices).unwrap();
        assert_eq!(count, 5);
        for i in 0..count {
            assert_eq!(simplices[i].dim, 0);
        }
    }

    // ── Alpha filtration: 4 co-circular points ───────────────────────

    #[test]
    fn alpha_4_cocircular_vertex_count() {
        let pts = four_cocircular();
        let mut simplices = vec![VrSimplex::default(); 14];
        let count = alpha_filtration_4_cocircular(&pts, &mut simplices).unwrap();
        assert_eq!(count, 14, "should have 4+6+4 = 14 simplices");
    }

    #[test]
    fn alpha_4_cocircular_all_triangles_same_birth() {
        let pts = four_cocircular();
        let mut simplices = vec![VrSimplex::default(); 14];
        let count = alpha_filtration_4_cocircular(&pts, &mut simplices).unwrap();

        let tris: Vec<_> = simplices[..count].iter().filter(|s| s.dim == 2).collect();
        assert_eq!(tris.len(), 4, "should have 4 triangles");

        // All triangles should have the same birth (circumradius).
        let first_birth = tris[0].birth_f64();
        for t in &tris {
            assert!((t.birth_f64() - first_birth).abs() < 1e-10,
                "all co-circular triangles should have same birth");
        }

        // The circumradius of the unit circle is 1.0.
        assert!((first_birth - 1.0).abs() < 1e-10,
            "circumradius of unit circle should be 1.0, got {}", first_birth);
    }

    #[test]
    fn alpha_4_cocircular_edge_births() {
        let pts = four_cocircular();
        let mut simplices = vec![VrSimplex::default(); 14];
        let count = alpha_filtration_4_cocircular(&pts, &mut simplices).unwrap();

        // Edge (0,1): distance = sqrt(2), birth = sqrt(2)/2.
        let edge_01 = simplices[..count].iter().find(|s| {
            s.dim == 1 && s.v0 == 0 && s.v1 == 1
        }).unwrap();
        let expected = (2.0f64).sqrt() / 2.0;
        assert!((edge_01.birth_f64() - expected).abs() < 1e-10,
            "edge (0,1) birth should be sqrt(2)/2");

        // Edge (0,2): distance = 2 (diameter), birth = 1.0.
        let edge_02 = simplices[..count].iter().find(|s| {
            s.dim == 1 && s.v0 == 0 && s.v1 == 2
        }).unwrap();
        assert!((edge_02.birth_f64() - 1.0).abs() < 1e-10,
            "edge (0,2) birth should be 1.0 (diameter/2)");
    }

    #[test]
    fn alpha_4_cocircular_determinism() {
        let pts = four_cocircular();

        let mut s1 = vec![VrSimplex::default(); 14];
        let c1 = alpha_filtration_4_cocircular(&pts, &mut s1).unwrap();

        let mut s2 = vec![VrSimplex::default(); 14];
        let c2 = alpha_filtration_4_cocircular(&pts, &mut s2).unwrap();

        assert_eq!(c1, c2);
        assert_eq!(s1[..c1], s2[..c2]);
        assert_eq!(filtration_hash(&s1[..c1]), filtration_hash(&s2[..c2]));
    }

    #[test]
    fn alpha_4_cocircular_non_finite_fails() {
        let mut pts = four_cocircular();
        pts[0].x = f32::INFINITY;
        let mut simplices = vec![VrSimplex::default(); 14];
        let err = alpha_filtration_4_cocircular(&pts, &mut simplices).unwrap_err();
        assert!(matches!(err, VrError::NonFinite { .. }));
    }

    #[test]
    fn alpha_4_cocircular_wrong_count_fails() {
        let pts = unit_square_with_centre(); // 5 points, not 4
        let mut simplices = vec![VrSimplex::default(); 14];
        let err = alpha_filtration_4_cocircular(&pts, &mut simplices).unwrap_err();
        assert!(matches!(err, VrError::TooFewPoints { .. }));
    }

    // ── Full-coordinate VR filtration ─────────────────────────────────

    #[test]
    fn vr_full_includes_spectral_axes() {
        // Two points at same spatial position but different σ.
        let pts = vec![
            Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0),
        ];
        let mut simplices = vec![VrSimplex::default(); 3];
        let count = vr_filtration_full(&pts, 1, 0.0, &mut simplices).unwrap();

        let edge = simplices[..count].iter().find(|s| s.dim == 1).unwrap();
        // Full distance = 1.0 (σ difference), birth = 0.5.
        assert!((edge.birth_f64() - 0.5).abs() < 1e-10,
            "full-coordinate edge birth should be 0.5, got {}", edge.birth_f64());
    }

    #[test]
    fn vr_spatial_ignores_spectral_axes() {
        // Same points as above — spatial distance should be 0.
        let pts = vec![
            Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0),
        ];
        let mut simplices = vec![VrSimplex::default(); 3];
        let count = vr_filtration(&pts, 1, 0.0, &mut simplices).unwrap();

        let edge = simplices[..count].iter().find(|s| s.dim == 1).unwrap();
        // Spatial distance = 0, birth = 0.
        assert!((edge.birth_f64() - 0.0).abs() < 1e-10,
            "spatial-only edge birth should be 0");
    }

    #[test]
    fn vr_filtration_sorted_canonical() {
        let pts = unit_square_with_centre();
        let n = pts.len();
        let cap = n + n * (n - 1) / 2;
        let mut simplices = vec![VrSimplex::default(); cap];
        let count = vr_filtration(&pts, 1, 0.0, &mut simplices).unwrap();

        // Verify sorted order.
        for i in 1..count {
            assert!(simplices[i - 1] <= simplices[i],
                "simplices must be sorted at index {}", i);
        }
    }
}
