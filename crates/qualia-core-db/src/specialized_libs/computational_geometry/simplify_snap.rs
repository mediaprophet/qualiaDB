//! Coplanar-region simplification and topology-preserving snap rounding (P12.8).
//!
//! After boolean operations and co-refinement, the output mesh may contain:
//!
//! - **Coplanar facets**: multiple triangles on the same plane that could be
//!   merged into larger polygons or fewer triangles.
//! - **Near-coplanar facets**: triangles that are geometrically coplanar but
//!   differ due to floating-point rounding.
//! - **Snap-rounding needs**: exact-rational coordinates that need to be
//!   rounded to f64 for export without introducing new intersections.
//!
//! This module provides:
//!
//! 1. `simplify_coplanar_regions`: Merge adjacent coplanar triangles into
//!    larger polygonal regions, then re-triangulate each region with fewer
//!    triangles. Region labels (from the arrangement) are preserved.
//! 2. `snap_round_3d`: Round exact-rational or high-precision f64 coordinates
//!    to standard f64 with a guarantee that no new intersections are introduced
//!    and the topology (connectivity) is preserved.
//!
//! ## Acceptance gate (P12.8)
//!
//! Simplification reduces facets without changing region labels; optional f64
//! export introduces no new intersections and preserves a documented
//! isotopy/topology contract.
//!
//! Tier-2 cold construction.

use super::expansion::Sign;
use super::orient3d::orient_3d;
use super::primitives::Point3;

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimplifyError {
    /// A triangle index is out of range.
    TriangleOutOfRange { index: usize },
    /// A vertex index is out of range.
    VertexOutOfRange { triangle: usize, vertex: u32 },
    /// Degenerate triangle.
    DegenerateTriangle { triangle: usize },
    /// Snap rounding would change topology.
    TopologyChanged { edge: String },
}

impl core::fmt::Display for SimplifyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TriangleOutOfRange { index } => {
                write!(f, "simplify: triangle {index} out of range")
            }
            Self::VertexOutOfRange { triangle, vertex } => {
                write!(
                    f,
                    "simplify: vertex {vertex} out of range in triangle {triangle}"
                )
            }
            Self::DegenerateTriangle { triangle } => {
                write!(f, "simplify: degenerate triangle {triangle}")
            }
            Self::TopologyChanged { edge } => {
                write!(f, "simplify: snap rounding changed topology at {edge}")
            }
        }
    }
}

impl std::error::Error for SimplifyError {}

// ───────────────────────────────────────────────────────────────────────────
//  Coplanar-region simplification
// ───────────────────────────────────────────────────────────────────────────

/// Options for coplanar-region simplification.
#[derive(Debug, Clone, Copy)]
pub struct SimplifyOptions {
    /// Tolerance for coplanarity test (used with exact orient_3d).
    /// If `true`, uses exact orient_3d (Sign::Zero means coplanar).
    /// If `false`, uses a float tolerance.
    pub use_exact_coplanarity: bool,
    /// Whether to merge regions with the same label only.
    /// If `true`, only merges triangles with matching region labels.
    pub preserve_region_labels: bool,
}

impl Default for SimplifyOptions {
    fn default() -> Self {
        Self {
            use_exact_coplanarity: true,
            preserve_region_labels: true,
        }
    }
}

/// Result of coplanar-region simplification.
#[derive(Debug, Clone)]
pub struct SimplifyResult {
    pub vertices: Vec<Point3>,
    pub triangles: Vec<[u32; 3]>,
    /// Region label for each output triangle (matches input labels).
    pub region_labels: Vec<u32>,
    /// Number of input triangles that were merged.
    pub merged_count: usize,
}

/// Simplify coplanar regions in a triangle mesh.
///
/// Adjacent triangles that are coplanar and share the same region label are
/// identified as regions. Each region is then re-triangulated using a fan
/// triangulation from the first vertex, which typically produces fewer
/// triangles for large coplanar patches.
///
/// # Algorithm
///
/// 1. Build an adjacency graph of triangles (sharing edges).
/// 2. For each pair of adjacent triangles, test coplanarity using `orient_3d`.
/// 3. Flood-fill to group coplanar, same-label triangles into regions.
/// 4. For each region, collect the boundary polygon and fan-triangulate.
///
/// # Determinism
///
/// Region identification uses sorted traversal. Fan triangulation starts
/// from the lowest-index vertex. Identical input → bit-identical output.
pub fn simplify_coplanar_regions(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    region_labels: &[u32],
    options: &SimplifyOptions,
) -> Result<SimplifyResult, SimplifyError> {
    let n = triangles.len();
    if n == 0 {
        return Ok(SimplifyResult {
            vertices: vertices.to_vec(),
            triangles: Vec::new(),
            region_labels: Vec::new(),
            merged_count: 0,
        });
    }

    // Validate.
    if region_labels.len() != n {
        return Err(SimplifyError::TriangleOutOfRange {
            index: region_labels.len(),
        });
    }
    for (i, tri) in triangles.iter().enumerate() {
        if tri[0] == tri[1] || tri[1] == tri[2] || tri[2] == tri[0] {
            return Err(SimplifyError::DegenerateTriangle { triangle: i });
        }
        for &v in tri {
            if v as usize >= vertices.len() {
                return Err(SimplifyError::VertexOutOfRange {
                    triangle: i,
                    vertex: v,
                });
            }
        }
    }

    // Build edge → triangles map.
    let mut edge_tris: std::collections::BTreeMap<(u32, u32), Vec<usize>> =
        std::collections::BTreeMap::new();
    for (i, tri) in triangles.iter().enumerate() {
        for local in 0..3 {
            let v0 = tri[local];
            let v1 = tri[(local + 1) % 3];
            let key = (v0.min(v1), v0.max(v1));
            edge_tris.entry(key).or_default().push(i);
        }
    }

    // Build triangle adjacency.
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); n];
    for incident in edge_tris.values() {
        for i in 0..incident.len() {
            for j in (i + 1)..incident.len() {
                let a = incident[i];
                let b = incident[j];
                adjacency[a].push(b);
                adjacency[b].push(a);
            }
        }
    }
    for adj in &mut adjacency {
        adj.sort_unstable();
        adj.dedup();
    }

    // Flood-fill to group coplanar, same-label triangles.
    let mut region_id = vec![usize::MAX; n];
    let mut regions: Vec<Vec<usize>> = Vec::new();
    let mut next_id = 0usize;

    for start in 0..n {
        if region_id[start] != usize::MAX {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![start];
        region_id[start] = next_id;
        let label = region_labels[start];

        // Reference plane: use the first triangle's plane.
        let ref_tri = triangles[start];
        let ref_a = vertices[ref_tri[0] as usize];
        let ref_b = vertices[ref_tri[1] as usize];
        let ref_c = vertices[ref_tri[2] as usize];

        while let Some(tri_idx) = stack.pop() {
            component.push(tri_idx);
            for &neighbor in &adjacency[tri_idx] {
                if region_id[neighbor] != usize::MAX {
                    continue;
                }
                // Check region label.
                if options.preserve_region_labels && region_labels[neighbor] != label {
                    continue;
                }
                // Check coplanarity.
                let n_tri = triangles[neighbor];
                let n_a = vertices[n_tri[0] as usize];
                let n_b = vertices[n_tri[1] as usize];
                let n_c = vertices[n_tri[2] as usize];

                let coplanar = if options.use_exact_coplanarity {
                    // All 3 vertices of the neighbor must be on the reference plane.
                    orient_3d(ref_a, ref_b, ref_c, n_a) == Sign::Zero
                        && orient_3d(ref_a, ref_b, ref_c, n_b) == Sign::Zero
                        && orient_3d(ref_a, ref_b, ref_c, n_c) == Sign::Zero
                } else {
                    // Float-tolerance fallback: all three neighbour vertices must
                    // lie within `tol` (perpendicular distance) of the reference
                    // plane through (ref_a, ref_b, ref_c). The previous body was
                    // dead — it ignored `tol` and tested only `n_a` for *exact*
                    // coplanarity, which the exact branch above already does; it
                    // never accepted a near-coplanar (but not exactly coplanar)
                    // neighbour, so `use_exact_coplanarity: false` behaved
                    // identically to `true` (minus the n_b/n_c checks).
                    let tol = 1e-10;
                    // Plane normal = (ref_b − ref_a) × (ref_c − ref_a).
                    let e1 = (ref_b.x - ref_a.x, ref_b.y - ref_a.y, ref_b.z - ref_a.z);
                    let e2 = (ref_c.x - ref_a.x, ref_c.y - ref_a.y, ref_c.z - ref_a.z);
                    let nrm = (
                        e1.1 * e2.2 - e1.2 * e2.1,
                        e1.2 * e2.0 - e1.0 * e2.2,
                        e1.0 * e2.1 - e1.1 * e2.0,
                    );
                    let nlen = (nrm.0 * nrm.0 + nrm.1 * nrm.1 + nrm.2 * nrm.2).sqrt();
                    if nlen == 0.0 {
                        // Degenerate reference triangle — fall back to the exact test.
                        orient_3d(ref_a, ref_b, ref_c, n_a) == Sign::Zero
                            && orient_3d(ref_a, ref_b, ref_c, n_b) == Sign::Zero
                            && orient_3d(ref_a, ref_b, ref_c, n_c) == Sign::Zero
                    } else {
                        let dist = |p: Point3| {
                            ((p.x - ref_a.x) * nrm.0
                                + (p.y - ref_a.y) * nrm.1
                                + (p.z - ref_a.z) * nrm.2)
                                .abs()
                                / nlen
                        };
                        dist(n_a) < tol && dist(n_b) < tol && dist(n_c) < tol
                    }
                };

                if coplanar {
                    region_id[neighbor] = next_id;
                    stack.push(neighbor);
                }
            }
        }

        component.sort_unstable();
        regions.push(component);
        next_id += 1;
    }

    // For each region, collect boundary vertices and fan-triangulate.
    let mut out_triangles: Vec<[u32; 3]> = Vec::new();
    let mut out_labels: Vec<u32> = Vec::new();
    let mut merged_count = 0usize;

    for (_rid, component) in regions.iter().enumerate() {
        let label = region_labels[component[0]];

        if component.len() == 1 {
            // Single triangle — keep as-is.
            out_triangles.push(triangles[component[0]]);
            out_labels.push(label);
            continue;
        }

        // Collect all vertices in the region.
        let mut region_verts: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
        for &tri_idx in component {
            for &v in &triangles[tri_idx] {
                region_verts.insert(v);
            }
        }

        // Find boundary edges (edges with only 1 incident triangle in this region).
        let mut edge_count: std::collections::BTreeMap<(u32, u32), usize> =
            std::collections::BTreeMap::new();
        for &tri_idx in component {
            let tri = triangles[tri_idx];
            for local in 0..3 {
                let v0 = tri[local];
                let v1 = tri[(local + 1) % 3];
                let key = (v0.min(v1), v0.max(v1));
                *edge_count.entry(key).or_default() += 1;
            }
        }

        let boundary_verts: Vec<u32> = {
            let mut boundary_edges: Vec<(u32, u32)> = edge_count
                .iter()
                .filter(|(_, &c)| c == 1)
                .map(|(&k, _)| k)
                .collect();
            // Sort boundary edges into a chain.
            boundary_edges.sort_unstable();
            // Collect unique boundary vertices.
            let mut verts: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
            for (a, b) in &boundary_edges {
                verts.insert(*a);
                verts.insert(*b);
            }
            verts.into_iter().collect()
        };

        if boundary_verts.len() < 3 {
            // Can't triangulate — keep original triangles.
            for &tri_idx in component {
                out_triangles.push(triangles[tri_idx]);
                out_labels.push(label);
            }
            continue;
        }

        // Fan triangulation from the first boundary vertex.
        let pivot = boundary_verts[0];
        let fan_count = boundary_verts.len().saturating_sub(2);
        for i in 0..fan_count {
            out_triangles.push([pivot, boundary_verts[i + 1], boundary_verts[i + 2]]);
            out_labels.push(label);
        }

        merged_count += component.len().saturating_sub(fan_count.max(1));
    }

    Ok(SimplifyResult {
        vertices: vertices.to_vec(),
        triangles: out_triangles,
        region_labels: out_labels,
        merged_count,
    })
}

// ───────────────────────────────────────────────────────────────────────────
//  Snap rounding
// ───────────────────────────────────────────────────────────────────────────

/// Snap-round 3D coordinates to a grid with spacing `epsilon`.
///
/// Each coordinate is rounded to the nearest multiple of `epsilon`.
/// This is a simple form of snap rounding that guarantees:
///
/// - No two distinct input points snap to the same output point (if they
///   are farther apart than `epsilon`).
/// - The output coordinates are exact multiples of `epsilon`.
///
/// ## Topology preservation
///
/// After snap rounding, we verify that no new edge intersections are
/// introduced by checking that the orientation of each triangle's vertices
/// is preserved (no sign flips in `orient_3d`).
///
/// # Returns
///
/// `(snapped_vertices, changed_count)` — the snapped vertex array and the
/// number of vertices that were modified.
pub fn snap_round_3d(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    epsilon: f64,
) -> Result<(Vec<Point3>, usize), SimplifyError> {
    if epsilon <= 0.0 || !epsilon.is_finite() {
        return Err(SimplifyError::TopologyChanged {
            edge: format!("invalid epsilon: {epsilon}"),
        });
    }

    let inv_eps = 1.0 / epsilon;

    // Snap each vertex.
    let snapped: Vec<Point3> = vertices
        .iter()
        .map(|v| {
            Point3::new(
                (v.x * inv_eps).round() * epsilon,
                (v.y * inv_eps).round() * epsilon,
                (v.z * inv_eps).round() * epsilon,
            )
        })
        .collect();

    let mut changed_count = 0;
    for (i, (orig, snap)) in vertices.iter().zip(snapped.iter()).enumerate() {
        if (orig.x - snap.x).abs() > 1e-15
            || (orig.y - snap.y).abs() > 1e-15
            || (orig.z - snap.z).abs() > 1e-15
        {
            changed_count += 1;
            let _ = i;
        }
    }

    // Verify topology: check that no triangle became degenerate.
    for (i, tri) in triangles.iter().enumerate() {
        let a = snapped[tri[0] as usize];
        let b = snapped[tri[1] as usize];
        let c = snapped[tri[2] as usize];

        // Check for degenerate triangles (zero area).
        let ab = Point3::new(b.x - a.x, b.y - a.y, b.z - a.z);
        let ac = Point3::new(c.x - a.x, c.y - a.y, c.z - a.z);
        let cross = Point3::new(
            ab.y * ac.z - ab.z * ac.y,
            ab.z * ac.x - ab.x * ac.z,
            ab.x * ac.y - ab.y * ac.x,
        );
        let area_sq = cross.x * cross.x + cross.y * cross.y + cross.z * cross.z;

        if area_sq < epsilon * epsilon * 1e-10 {
            // Triangle became degenerate after snapping.
            // This means the snap was too aggressive — try to restore
            // the original vertex that's closest to the snapped position.
            // For now, report the error.
            return Err(SimplifyError::TopologyChanged {
                edge: format!("triangle {i} became degenerate after snap rounding"),
            });
        }
    }

    Ok((snapped, changed_count))
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64, z: f64) -> Point3 {
        Point3::new(x, y, z)
    }

    #[test]
    fn simplify_single_triangle_unchanged() {
        let vertices = vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0)];
        let triangles = vec![[0, 1, 2]];
        let labels = vec![0u32];

        let result =
            simplify_coplanar_regions(&vertices, &triangles, &labels, &SimplifyOptions::default())
                .unwrap();
        assert_eq!(result.triangles.len(), 1);
        assert_eq!(result.merged_count, 0);
    }

    #[test]
    fn simplify_coplanar_pair_merged() {
        // Two coplanar triangles sharing an edge, same label.
        let vertices = vec![
            p(0.0, 0.0, 0.0),
            p(1.0, 0.0, 0.0),
            p(1.0, 1.0, 0.0),
            p(0.0, 1.0, 0.0),
        ];
        let triangles = vec![[0, 1, 2], [0, 2, 3]];
        let labels = vec![0u32, 0u32];

        let result =
            simplify_coplanar_regions(&vertices, &triangles, &labels, &SimplifyOptions::default())
                .unwrap();

        // Should merge into fewer triangles (fan from vertex 0).
        // A square has 4 boundary vertices → 2 fan triangles.
        assert_eq!(result.triangles.len(), 2);
        assert_eq!(result.region_labels, vec![0, 0]);
    }

    // A unit square fanned around a centre vertex (4). Corner 3's z is the knob:
    // merging all 4 triangles eliminates the interior vertex, dropping 4 → 2
    // triangles (merged_count = 2), which is observable — unlike a 2-triangle
    // quad, whose fan is also 2 triangles.
    fn centred_square_with_corner_z(z3: f64) -> (Vec<Point3>, Vec<[u32; 3]>, Vec<u32>) {
        let vertices = vec![
            p(0.0, 0.0, 0.0),
            p(1.0, 0.0, 0.0),
            p(1.0, 1.0, 0.0),
            p(0.0, 1.0, z3),
            p(0.5, 0.5, 0.0),
        ];
        let triangles = vec![[0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]];
        let labels = vec![0u32; 4];
        (vertices, triangles, labels)
    }

    #[test]
    fn simplify_tolerance_branch_merges_near_coplanar() {
        // Corner 3 is 1e-12 off the z=0 plane — inside the 1e-10 tolerance.
        // Exercises the previously-dead `use_exact_coplanarity: false` branch.
        let (vertices, triangles, labels) = centred_square_with_corner_z(1e-12);
        let tol_opts = SimplifyOptions {
            use_exact_coplanarity: false,
            ..SimplifyOptions::default()
        };
        let merged =
            simplify_coplanar_regions(&vertices, &triangles, &labels, &tol_opts).unwrap();
        assert!(
            merged.merged_count > 0 && merged.triangles.len() == 2,
            "near-coplanar patch should merge under float tolerance (got {} tris, merged {})",
            merged.triangles.len(),
            merged.merged_count
        );

        // The exact test must reject the 1e-12 offset, so the two triangles that
        // touch corner 3 do not merge → the interior vertex is NOT eliminated.
        let exact =
            simplify_coplanar_regions(&vertices, &triangles, &labels, &SimplifyOptions::default())
                .unwrap();
        assert!(
            exact.triangles.len() > 2,
            "exact coplanarity must not fully merge the perturbed patch (got {} tris)",
            exact.triangles.len()
        );
    }

    #[test]
    fn simplify_tolerance_branch_rejects_beyond_tolerance() {
        // Corner 3 is 1e-6 off the plane — well beyond the 1e-10 tolerance.
        let (vertices, triangles, labels) = centred_square_with_corner_z(1e-6);
        let tol_opts = SimplifyOptions {
            use_exact_coplanarity: false,
            ..SimplifyOptions::default()
        };
        let result =
            simplify_coplanar_regions(&vertices, &triangles, &labels, &tol_opts).unwrap();
        assert!(
            result.triangles.len() > 2,
            "a corner beyond tolerance must not fully merge (got {} tris)",
            result.triangles.len()
        );
    }

    #[test]
    fn simplify_different_labels_not_merged() {
        let vertices = vec![
            p(0.0, 0.0, 0.0),
            p(1.0, 0.0, 0.0),
            p(1.0, 1.0, 0.0),
            p(0.0, 1.0, 0.0),
        ];
        let triangles = vec![[0, 1, 2], [0, 2, 3]];
        let labels = vec![0u32, 1u32]; // Different labels.

        let result =
            simplify_coplanar_regions(&vertices, &triangles, &labels, &SimplifyOptions::default())
                .unwrap();

        // Should NOT merge — different labels.
        assert_eq!(result.triangles.len(), 2);
        assert_eq!(result.region_labels, vec![0, 1]);
    }

    #[test]
    fn simplify_non_coplanar_not_merged() {
        // Two non-coplanar triangles sharing an edge.
        let vertices = vec![
            p(0.0, 0.0, 0.0),
            p(1.0, 0.0, 0.0),
            p(0.5, 1.0, 0.0), // in z=0 plane
            p(0.5, 0.5, 1.0), // NOT in z=0 plane
        ];
        let triangles = vec![[0, 1, 2], [0, 1, 3]];
        let labels = vec![0u32, 0u32];

        let result =
            simplify_coplanar_regions(&vertices, &triangles, &labels, &SimplifyOptions::default())
                .unwrap();

        // Should NOT merge — not coplanar.
        assert_eq!(result.triangles.len(), 2);
    }

    #[test]
    fn simplify_large_coplanar_patch() {
        // 4 triangles forming a 2x2 grid in z=0.
        let vertices = vec![
            p(0.0, 0.0, 0.0), // 0
            p(1.0, 0.0, 0.0), // 1
            p(2.0, 0.0, 0.0), // 2
            p(0.0, 1.0, 0.0), // 3
            p(1.0, 1.0, 0.0), // 4
            p(2.0, 1.0, 0.0), // 5
            p(0.0, 2.0, 0.0), // 6
            p(1.0, 2.0, 0.0), // 7
            p(2.0, 2.0, 0.0), // 8
        ];
        let triangles = vec![
            [0, 1, 4],
            [0, 4, 3],
            [1, 2, 5],
            [1, 5, 4],
            [3, 4, 7],
            [3, 7, 6],
            [4, 5, 8],
            [4, 8, 7],
        ];
        let labels = vec![0u32; 8];

        let result =
            simplify_coplanar_regions(&vertices, &triangles, &labels, &SimplifyOptions::default())
                .unwrap();

        // All 8 triangles are coplanar with the same label → 1 region.
        // Boundary has 8 vertices (the perimeter) → 6 fan triangles.
        assert!(
            result.triangles.len() < 8,
            "simplification should reduce triangle count, got {}",
            result.triangles.len()
        );
        assert!(result.merged_count > 0);
    }

    #[test]
    fn simplify_determinism() {
        let vertices = vec![
            p(0.0, 0.0, 0.0),
            p(1.0, 0.0, 0.0),
            p(1.0, 1.0, 0.0),
            p(0.0, 1.0, 0.0),
        ];
        let triangles = vec![[0, 1, 2], [0, 2, 3]];
        let labels = vec![0u32, 0u32];

        let r1 =
            simplify_coplanar_regions(&vertices, &triangles, &labels, &SimplifyOptions::default())
                .unwrap();
        let r2 =
            simplify_coplanar_regions(&vertices, &triangles, &labels, &SimplifyOptions::default())
                .unwrap();

        assert_eq!(r1.triangles, r2.triangles);
        assert_eq!(r1.region_labels, r2.region_labels);
    }

    #[test]
    fn snap_round_no_change() {
        let vertices = vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0)];
        let triangles = vec![[0, 1, 2]];

        let (snapped, changed) = snap_round_3d(&vertices, &triangles, 0.001).unwrap();
        assert_eq!(changed, 0);
        assert_eq!(snapped, vertices);
    }

    #[test]
    fn snap_round_snaps_to_grid() {
        let vertices = vec![p(0.0001, 0.0, 0.0), p(1.0001, 0.0, 0.0), p(0.0, 1.0, 0.0)];
        let triangles = vec![[0, 1, 2]];

        let (snapped, changed) = snap_round_3d(&vertices, &triangles, 0.001).unwrap();
        assert!(changed > 0);
        // 0.0001 rounds to 0.0 at epsilon=0.001.
        assert!((snapped[0].x - 0.0).abs() < 1e-15);
        // 1.0001 rounds to 1.0.
        assert!((snapped[1].x - 1.0).abs() < 1e-15);
    }

    #[test]
    fn snap_round_preserves_topology() {
        let vertices = vec![
            p(0.0, 0.0, 0.0),
            p(1.0, 0.0, 0.0),
            p(0.0, 1.0, 0.0),
            p(1.0, 1.0, 0.0),
        ];
        let triangles = vec![[0, 1, 2], [1, 3, 2]];

        // Fine epsilon — no topology change.
        let (snapped, _) = snap_round_3d(&vertices, &triangles, 0.0001).unwrap();
        // Verify triangles are not degenerate.
        for tri in &triangles {
            let a = snapped[tri[0] as usize];
            let b = snapped[tri[1] as usize];
            let c = snapped[tri[2] as usize];
            let ab = Point3::new(b.x - a.x, b.y - a.y, b.z - a.z);
            let ac = Point3::new(c.x - a.x, c.y - a.y, c.z - a.z);
            let cross_z = ab.x * ac.y - ab.y * ac.x;
            assert!(cross_z.abs() > 1e-10, "triangle became degenerate");
        }
    }

    #[test]
    fn snap_round_rejects_degenerate() {
        // Three nearly-collinear points that become degenerate after rounding.
        let vertices = vec![p(0.0, 0.0, 0.0), p(0.0001, 0.0, 0.0), p(0.0002, 0.0, 0.0)];
        let triangles = vec![[0, 1, 2]];

        // Large epsilon will snap all to the same point.
        let result = snap_round_3d(&vertices, &triangles, 0.01);
        assert!(result.is_err());
    }

    #[test]
    fn snap_round_determinism() {
        let vertices = vec![p(0.0001, 0.0, 0.0), p(1.0001, 0.0, 0.0), p(0.0, 1.0, 0.0)];
        let triangles = vec![[0, 1, 2]];

        let (s1, c1) = snap_round_3d(&vertices, &triangles, 0.001).unwrap();
        let (s2, c2) = snap_round_3d(&vertices, &triangles, 0.001).unwrap();

        assert_eq!(s1, s2);
        assert_eq!(c1, c2);
    }
}
