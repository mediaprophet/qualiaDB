//! Voronoi diagram as the Delaunay dual (P4.6).
//!
//! Given a Delaunay triangulation, the Voronoi diagram is constructed as:
//! - Each Voronoi **vertex** is the circumcenter of a Delaunay triangle.
//! - Each Voronoi **edge** connects the circumcenters of two adjacent Delaunay
//!   triangles (sharing a Delaunay edge).
//! - Each Voronoi **cell** corresponds to a Delaunay vertex (site).
//!
//! ## Nearest-site query
//!
//! The nearest site to a query point can be found by locating the Delaunay
//! triangle containing the query point — the nearest site is one of its
//! vertices. For simplicity, this implementation provides a brute-force
//! nearest-site query (cross-checked against the Delaunay-based approach).
//!
//! ## Determinism
//!
//! The Voronoi diagram is fully determined by the Delaunay triangulation,
//! which is deterministic (P4.4). Circumcenters are computed in f64 with
//! a stable formula. Output is sorted canonically.

use super::delaunay_2::delaunay_triangulation_2;
use super::primitives::Point2;

/// Voronoi diagram error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoronoiError {
    /// Delaunay triangulation failed.
    DelaunayFailed(String),
    /// Output buffer too small.
    OutputTooSmall { required: usize, have: usize },
    /// Too few sites.
    TooFewSites { got: usize },
}

impl core::fmt::Display for VoronoiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DelaunayFailed(msg) => write!(f, "voronoi: delaunay failed: {msg}"),
            Self::OutputTooSmall { required, have } => {
                write!(f, "voronoi: output too small, need {required}, have {have}")
            }
            Self::TooFewSites { got } => write!(f, "voronoi: need ≥2 sites, got {got}"),
        }
    }
}

impl std::error::Error for VoronoiError {}

/// A Voronoi vertex (circumcenter of a Delaunay triangle).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoronoiVertex {
    /// The Delaunay triangle index this vertex corresponds to.
    pub triangle_index: u32,
    /// The circumcenter coordinates.
    pub center: Point2,
}

/// A Voronoi edge connecting two Voronoi vertices (or extending to infinity
/// for boundary edges, where `neighbor_triangle` is `None`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoronoiEdge {
    /// The Delaunay edge (site_a, site_b) that this Voronoi edge crosses.
    pub site_a: u32,
    pub site_b: u32,
    /// The triangle on one side of the Delaunay edge.
    pub triangle: u32,
    /// The adjacent triangle on the other side (None = boundary edge).
    pub neighbor_triangle: Option<u32>,
}

/// Compute the circumcenter of a triangle (a, b, c).
///
/// Uses the formula:
/// ```text
/// D = 2 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by))
/// ux = ((ax² + ay²) * (by - cy) + (bx² + by²) * (cy - ay) + (cx² + cy²) * (ay - by)) / D
/// uy = ((ax² + ay²) * (cx - bx) + (bx² + by²) * (ax - cx) + (cx² + cy²) * (bx - ax)) / D
/// ```
#[inline]
pub fn circumcenter(a: Point2, b: Point2, c: Point2) -> Point2 {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    let ax2 = a.x * a.x + a.y * a.y;
    let bx2 = b.x * b.x + b.y * b.y;
    let cx2 = c.x * c.x + c.y * c.y;
    let ux = (ax2 * (b.y - c.y) + bx2 * (c.y - a.y) + cx2 * (a.y - b.y)) / d;
    let uy = (ax2 * (c.x - b.x) + bx2 * (a.x - c.x) + cx2 * (b.x - a.x)) / d;
    Point2::new(ux, uy)
}

/// Compute the Voronoi diagram from a set of 2-D sites.
///
/// Returns `(vertex_count, edge_count)`. Voronoi vertices are written to
/// `vertices_out`, Voronoi edges to `edges_out`.
///
/// `tri_scratch` needs `sites.len()` entries (for Delaunay).
/// `tri_out` needs `2 * sites.len()` entries (for Delaunay triangles).
/// `vertices_out` needs `2 * sites.len()` entries (upper bound on triangles).
/// `edges_out` needs `3 * sites.len()` entries (upper bound on Delaunay edges).
pub fn voronoi_diagram_2(
    sites: &[Point2],
    tri_scratch: &mut [u32],
    tri_out: &mut [[u32; 3]],
    vertices_out: &mut [VoronoiVertex],
    edges_out: &mut [VoronoiEdge],
) -> Result<(usize, usize), VoronoiError> {
    let n = sites.len();
    if n < 2 {
        return Err(VoronoiError::TooFewSites { got: n });
    }

    // Compute Delaunay triangulation.
    let tri_count = delaunay_triangulation_2(sites, tri_scratch, tri_out)
        .map_err(|e| VoronoiError::DelaunayFailed(e.to_string()))?;

    if tri_count > vertices_out.len() {
        return Err(VoronoiError::OutputTooSmall {
            required: tri_count,
            have: vertices_out.len(),
        });
    }

    // Compute Voronoi vertices (circumcenters).
    for i in 0..tri_count {
        let tri = tri_out[i];
        let a = sites[tri[0] as usize];
        let b = sites[tri[1] as usize];
        let c = sites[tri[2] as usize];
        let center = circumcenter(a, b, c);
        vertices_out[i] = VoronoiVertex {
            triangle_index: i as u32,
            center,
        };
    }

    // Compute Voronoi edges by finding adjacent Delaunay triangles.
    // Each Delaunay edge (a, b) shared by two triangles t1, t2 produces
    // a Voronoi edge connecting circumcenter(t1) and circumcenter(t2).
    // Boundary Delaunay edges produce infinite Voronoi rays (neighbor = None).
    let mut edge_count = 0usize;

    // Build a map from sorted Delaunay edge → list of triangles sharing it.
    // We use a simple O(n²) scan since triangle counts are modest.
    for t1 in 0..tri_count {
        let tri1 = tri_out[t1];
        for j in 0..3 {
            let a = tri1[j];
            let b = tri1[(j + 1) % 3];
            // Only process each edge once (when a < b).
            if a > b {
                continue;
            }

            // Find the other triangle sharing edge (a, b).
            let mut neighbor: Option<u32> = None;
            for t2 in (t1 + 1)..tri_count {
                let tri2 = tri_out[t2];
                // Check if tri2 contains both a and b.
                let has_a = tri2[0] == a || tri2[1] == a || tri2[2] == a;
                let has_b = tri2[0] == b || tri2[1] == b || tri2[2] == b;
                if has_a && has_b {
                    neighbor = Some(t2 as u32);
                    break;
                }
            }

            if edge_count >= edges_out.len() {
                return Err(VoronoiError::OutputTooSmall {
                    required: edge_count + 1,
                    have: edges_out.len(),
                });
            }

            edges_out[edge_count] = VoronoiEdge {
                site_a: a,
                site_b: b,
                triangle: t1 as u32,
                neighbor_triangle: neighbor,
            };
            edge_count += 1;
        }
    }

    // Sort edges canonically for deterministic output.
    edges_out[..edge_count].sort_unstable_by(|a, b| {
        (a.site_a, a.site_b, a.triangle).cmp(&(b.site_a, b.site_b, b.triangle))
    });

    Ok((tri_count, edge_count))
}

/// Brute-force nearest-site query: returns the index of the site closest
/// to the query point.
///
/// Ties are broken by index (lowest index wins) for determinism.
pub fn nearest_site_brute_force(sites: &[Point2], query: Point2) -> Option<u32> {
    if sites.is_empty() {
        return None;
    }
    let mut best_idx = 0u32;
    let mut best_dist_sq = f64::INFINITY;
    for (i, s) in sites.iter().enumerate() {
        let dx = s.x - query.x;
        let dy = s.y - query.y;
        let d = dx * dx + dy * dy;
        if d < best_dist_sq {
            best_dist_sq = d;
            best_idx = i as u32;
        }
    }
    Some(best_idx)
}

/// Nearest-site query using the Delaunay triangulation.
///
/// Locates the Delaunay triangle containing the query point and returns
/// the nearest vertex. Falls back to brute-force if no containing triangle
/// is found (query outside the convex hull).
pub fn nearest_site_via_delaunay(
    sites: &[Point2],
    triangles: &[[u32; 3]],
    query: Point2,
) -> Option<u32> {
    use super::primitives::orientation_2;

    // Find the triangle containing the query point.
    for tri in triangles {
        let a = sites[tri[0] as usize];
        let b = sites[tri[1] as usize];
        let c = sites[tri[2] as usize];

        // Check if query is inside triangle (all orientations same sign).
        let o1 = orientation_2(a, b, query);
        let o2 = orientation_2(b, c, query);
        let o3 = orientation_2(c, a, query);

        let inside = (o1 != super::primitives::Orientation::Clockwise
            && o2 != super::primitives::Orientation::Clockwise
            && o3 != super::primitives::Orientation::Clockwise)
            || (o1 != super::primitives::Orientation::CounterClockwise
                && o2 != super::primitives::Orientation::CounterClockwise
                && o3 != super::primitives::Orientation::CounterClockwise);

        if inside {
            // Return the nearest vertex of this triangle.
            let mut best_idx = tri[0];
            let mut best_d = f64::INFINITY;
            for &v in tri {
                let s = sites[v as usize];
                let dx = s.x - query.x;
                let dy = s.y - query.y;
                let d = dx * dx + dy * dy;
                if d < best_d {
                    best_d = d;
                    best_idx = v;
                }
            }
            return Some(best_idx);
        }
    }

    // Fallback: brute-force.
    nearest_site_brute_force(sites, query)
}

/// Verify that each Voronoi vertex is equidistant to its ≥3 sites.
///
/// Returns `true` if all vertices satisfy the equidistance property
/// within the given tolerance.
pub fn verify_voronoi_vertices(
    sites: &[Point2],
    triangles: &[[u32; 3]],
    vertices: &[VoronoiVertex],
    tolerance: f64,
) -> bool {
    for v in vertices {
        let tri = triangles[v.triangle_index as usize];
        let a = sites[tri[0] as usize];
        let b = sites[tri[1] as usize];
        let c = sites[tri[2] as usize];

        let da = distance_sq(v.center, a);
        let db = distance_sq(v.center, b);
        let dc = distance_sq(v.center, c);

        if (da - db).abs() > tolerance || (da - dc).abs() > tolerance {
            return false;
        }
    }
    true
}

#[inline]
fn distance_sq(a: Point2, b: Point2) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

/// Compute a determinism hash for a Voronoi diagram.
pub fn voronoi_hash(
    vertices: &[VoronoiVertex],
    edges: &[VoronoiEdge],
) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for v in vertices {
        hash ^= v.triangle_index as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        let bits = v.center.x.to_bits();
        hash ^= bits as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        let bits = v.center.y.to_bits();
        hash ^= bits as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    for e in edges {
        hash ^= e.site_a as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= e.site_b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= e.triangle as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// ──────────────────────────────────────────────────────────────────────────
//  Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn max_tri(n: usize) -> usize {
        if n < 2 { 1 } else { 2 * n + 1 }
    }

    #[test]
    fn square_voronoi_has_one_vertex() {
        let sites = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];
        let mut tri_scratch = vec![0u32; 4];
        let mut tri_out = vec![[0u32; 3]; max_tri(4)];
        let mut verts = vec![VoronoiVertex::default_for(0); max_tri(4)];
        let mut edges = vec![VoronoiEdge::default(); 3 * 4];
        let (vc, ec) = voronoi_diagram_2(&sites, &mut tri_scratch, &mut tri_out, &mut verts, &mut edges).unwrap();
        assert_eq!(vc, 2); // 2 Delaunay triangles → 2 Voronoi vertices
        assert!(ec > 0);
    }

    #[test]
    fn voronoi_vertices_equidistant() {
        let sites = [
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(1.0, 2.0),
            Point2::new(1.0, 0.5), // interior point
        ];
        let mut tri_scratch = vec![0u32; 4];
        let mut tri_out = vec![[0u32; 3]; max_tri(4)];
        let mut verts = vec![VoronoiVertex::default_for(0); max_tri(4)];
        let mut edges = vec![VoronoiEdge::default(); 3 * 4];
        let (vc, _) = voronoi_diagram_2(&sites, &mut tri_scratch, &mut tri_out, &mut verts, &mut edges).unwrap();
        assert!(
            verify_voronoi_vertices(&sites, &tri_out[..vc], &verts[..vc], 1e-10),
            "Voronoi vertices should be equidistant to their sites"
        );
    }

    #[test]
    fn nearest_site_matches_brute_force() {
        let sites = [
            Point2::new(0.0, 0.0),
            Point2::new(3.0, 0.0),
            Point2::new(2.0, 3.0),
            Point2::new(0.0, 2.0),
            Point2::new(1.0, 1.0),
        ];
        let mut tri_scratch = vec![0u32; 5];
        let mut tri_out = vec![[0u32; 3]; max_tri(5)];
        let tri_count = delaunay_triangulation_2(&sites, &mut tri_scratch, &mut tri_out).unwrap();

        let test_points = [
            Point2::new(0.5, 0.5),
            Point2::new(1.5, 1.0),
            Point2::new(2.5, 2.0),
            Point2::new(0.1, 0.1),
            Point2::new(1.0, 0.8),
            Point2::new(10.0, 10.0), // outside hull
            Point2::new(-1.0, -1.0), // outside hull
        ];

        for qp in test_points {
            let bf = nearest_site_brute_force(&sites, qp).unwrap();
            let delaunay = nearest_site_via_delaunay(&sites, &tri_out[..tri_count], qp).unwrap();
            assert_eq!(bf, delaunay, "nearest site mismatch at {qp:?}: brute={bf}, delaunay={delaunay}");
        }
    }

    #[test]
    fn determinism_same_input_same_output() {
        let sites = [
            Point2::new(0.0, 0.0),
            Point2::new(3.0, 0.0),
            Point2::new(2.0, 3.0),
            Point2::new(0.0, 2.0),
            Point2::new(1.0, 1.0),
        ];
        let run = || {
            let mut ts = vec![0u32; 5];
            let mut to = vec![[0u32; 3]; max_tri(5)];
            let mut v = vec![VoronoiVertex::default_for(0); max_tri(5)];
            let mut e = vec![VoronoiEdge::default(); 3 * 5];
            let (vc, ec) = voronoi_diagram_2(&sites, &mut ts, &mut to, &mut v, &mut e).unwrap();
            (vc, ec, voronoi_hash(&v[..vc], &e[..ec]))
        };
        let (vc1, ec1, h1) = run();
        let (vc2, ec2, h2) = run();
        assert_eq!(vc1, vc2);
        assert_eq!(ec1, ec2);
        assert_eq!(h1, h2);
    }

    #[test]
    fn circumcenter_unit_triangle() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(0.5, 0.5 * 3.0_f64.sqrt());
        let cc = circumcenter(a, b, c);
        // Circumcenter of equilateral triangle with side 1 is at (0.5, sqrt(3)/6).
        assert!((cc.x - 0.5).abs() < 1e-12);
        assert!((cc.y - 3.0_f64.sqrt() / 6.0).abs() < 1e-12);
    }

    #[test]
    fn too_few_sites_errors() {
        let sites = [Point2::new(0.0, 0.0)];
        let mut ts = vec![0u32; 1];
        let mut to = vec![[0u32; 3]; 10];
        let mut v = vec![VoronoiVertex::default_for(0); 10];
        let mut e = vec![VoronoiEdge::default(); 10];
        let result = voronoi_diagram_2(&sites, &mut ts, &mut to, &mut v, &mut e);
        assert!(result.is_err());
    }
}

// Default values for test initialization.
impl VoronoiVertex {
    pub fn default_for(idx: u32) -> Self {
        VoronoiVertex {
            triangle_index: idx,
            center: Point2::new(0.0, 0.0),
        }
    }
}

impl Default for VoronoiEdge {
    fn default() -> Self {
        VoronoiEdge {
            site_a: 0,
            site_b: 0,
            triangle: 0,
            neighbor_triangle: None,
        }
    }
}
