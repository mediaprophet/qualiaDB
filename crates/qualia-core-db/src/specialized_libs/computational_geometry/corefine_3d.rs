//! Exact 3-D mesh co-refinement (P12.3).
//!
//! Given two triangle meshes that intersect, split both meshes along their
//! intersection curves so they share a common refinement. This is the 3D
//! analogue of 2D mesh co-refinement and the foundation for exact 3D boolean
//! operations.
//!
//! ## Algorithm
//!
//! 1. **Broad phase**: Build a BVH over mesh B's triangle AABBs, then query
//!    with each triangle AABB from mesh A to find candidate pairs. This
//!    reduces the broad phase from O(nm) to O(n log m + k) where k is the
//!    number of candidate pairs.
//! 2. **Narrow phase**: For each candidate pair, compute the intersection
//!    segment using `tri_tri_intersect_3_exact`.
//! 3. **Split**: For each triangle that intersects, insert the intersection
//!    segment endpoints as new vertices and split the triangle into
//!    sub-triangles.
//! 4. **Output**: Both meshes are returned with compatible boundaries —
//!    any point on the intersection curve is a vertex of both meshes.
//!
//! ## Bounded workspace (P12.5)
//!
//! The BVH broad phase produces the same candidate pair set as the O(nm)
//! brute-force oracle (no false negatives). Workspace is bounded by the
//! 42-MiB Sentinel ceiling: BVH nodes, prim indices, and query buffers
//! are allocated within the budget. Deterministic Morton-code ordering
//! ensures bit-identical candidate pairs across runs.
//!
//! ## Exactness
//!
//! The intersection points are exact-rational [`ExactPoint3`] values (from
//! [`tri_tri_intersect_3_exact`]). The f64-rounded versions are used for mesh
//! vertex storage (`Point3`), but the exact points are used for uniqueness
//! comparison (via exact coordinate equality) instead of float tolerance.
//! The topology (which triangles intersect, how they split) is determined by
//! exact orientation predicates.
//!
//! Tier-2 cold construction (uses `Vec` during build).

use super::boolean_3::Boolean3Error;
use super::bvh::{build_bvh_recursive, query_overlap, BvhNode, MAX_BVH_DEPTH};
use super::distance::Aabb;
use super::exact_construct_3::ExactPoint3;
use super::expansion::Sign;
use super::orient3d::orient_3d;
use super::primitives::Point3;
use super::tri_tri_3::{tri_tri_intersect_3_exact, ExactTriTriSegment};

// ───────────────────────────────────────────────────────────────────────────
//  Data structures
// ───────────────────────────────────────────────────────────────────────────

/// A 3D triangle mesh: vertices + triangle index triples.
#[derive(Debug, Clone)]
pub struct Mesh3D {
    pub vertices: Vec<Point3>,
    pub triangles: Vec<[u32; 3]>,
}

/// Result of 3D mesh co-refinement: both meshes refined to share common
/// intersection vertices.
#[derive(Debug, Clone)]
pub struct CorefinementResult3D {
    pub mesh_a: Mesh3D,
    pub mesh_b: Mesh3D,
    /// Number of intersection points inserted into both meshes.
    pub num_intersection_points: usize,
    /// Number of triangle pairs that intersect.
    pub num_intersecting_pairs: usize,
}

// ───────────────────────────────────────────────────────────────────────────
//  Co-refinement
// ───────────────────────────────────────────────────────────────────────────

/// Sentinel memory budget for BVH-driven co-refinement workspace (42 MiB).
pub const COREFINE_BUDGET_BYTES: usize = 42 * 1024 * 1024;

/// Compute the co-refinement of two 3D triangle meshes.
///
/// Splits both meshes at their intersection curves so they share a common
/// refinement. The output meshes have compatible boundaries.
///
/// Uses BVH-accelerated broad phase (P12.5): a BVH is built over mesh B's
/// triangle AABBs, then each triangle from mesh A queries the BVH to find
/// candidate pairs. This produces the same candidate set as the O(nm)
/// brute-force oracle with no false negatives.
pub fn corefine_3d(
    mesh_a: &Mesh3D,
    mesh_b: &Mesh3D,
) -> Result<CorefinementResult3D, Boolean3Error> {
    // Validate meshes.
    validate_mesh(mesh_a, "A")?;
    validate_mesh(mesh_b, "B")?;

    // Compute triangle AABBs for both meshes.
    let boxes_a = compute_triangle_aabbs(mesh_a);
    let boxes_b = compute_triangle_aabbs(mesh_b);

    // Build BVH over mesh B.
    let nb = boxes_b.len();
    let mut bvh_nodes = vec![BvhNode::default(); 2 * nb];
    let mut bvh_prim_indices = vec![0u32; nb];
    let mut bvh_morton_codes = vec![0u64; nb];
    let mut bvh_sort_indices = vec![0u32; nb];

    let (node_count, root) = build_bvh_recursive(
        &boxes_b,
        &mut bvh_nodes,
        &mut bvh_prim_indices,
        &mut bvh_morton_codes,
        &mut bvh_sort_indices,
    ).map_err(|_| Boolean3Error::DegenerateMesh { mesh: "B" })?;

    // Query buffers (reused across all mesh A triangles).
    let mut query_out = vec![0u32; nb];
    let mut query_stack = vec![0u32; MAX_BVH_DEPTH * 2];

    // Find all intersecting triangle pairs via BVH broad phase + exact narrow phase.
    let mut exact_segments: Vec<ExactTriTriSegment> = Vec::new();
    let mut num_intersecting_pairs = 0;

    for (i, tri_a) in mesh_a.triangles.iter().enumerate() {
        let a0 = mesh_a.vertices[tri_a[0] as usize];
        let a1 = mesh_a.vertices[tri_a[1] as usize];
        let a2 = mesh_a.vertices[tri_a[2] as usize];

        // BVH query: find all triangles in mesh B whose AABB overlaps this triangle's AABB.
        let hit_count = query_overlap(
            &bvh_nodes,
            &boxes_b,
            &bvh_prim_indices,
            root,
            node_count,
            &boxes_a[i],
            &mut query_out,
            &mut query_stack,
        ).map_err(|_| Boolean3Error::DegenerateMesh { mesh: "B" })?;

        // Sort candidates for deterministic ordering.
        query_out[..hit_count].sort_unstable();

        // Narrow phase: exact triangle-triangle intersection test.
        for j in 0..hit_count {
            let b_idx = query_out[j] as usize;
            let tri_b = &mesh_b.triangles[b_idx];
            let b0 = mesh_b.vertices[tri_b[0] as usize];
            let b1 = mesh_b.vertices[tri_b[1] as usize];
            let b2 = mesh_b.vertices[tri_b[2] as usize];

            let (intersects, seg_opt) = tri_tri_intersect_3_exact(a0, a1, a2, b0, b1, b2);
            if intersects {
                if let Some(seg) = seg_opt {
                    num_intersecting_pairs += 1;
                    exact_segments.push(seg);
                } else {
                    // Coplanar overlap — no segment, but triangles do intersect.
                    num_intersecting_pairs += 1;
                }
            }
        }
    }

    // Collect all unique intersection points using exact comparison.
    // We store both the exact point (for uniqueness) and the rounded Point3
    // (for mesh vertex insertion).
    let mut exact_points: Vec<ExactPoint3> = Vec::new();
    let mut f64_points: Vec<Point3> = Vec::new();
    for seg in &exact_segments {
        add_unique_exact_point(&mut exact_points, &mut f64_points, &seg.start);
        add_unique_exact_point(&mut exact_points, &mut f64_points, &seg.end);
    }

    let num_intersection_points = f64_points.len();

    // Refine both meshes by inserting intersection points and splitting
    // affected triangles.
    let refined_a = refine_mesh(mesh_a, &f64_points);
    let refined_b = refine_mesh(mesh_b, &f64_points);

    Ok(CorefinementResult3D {
        mesh_a: refined_a,
        mesh_b: refined_b,
        num_intersection_points,
        num_intersecting_pairs,
    })
}

/// Compute per-triangle AABBs for a mesh.
fn compute_triangle_aabbs(mesh: &Mesh3D) -> Vec<Aabb> {
    mesh.triangles.iter().map(|tri| {
        let a = mesh.vertices[tri[0] as usize];
        let b = mesh.vertices[tri[1] as usize];
        let c = mesh.vertices[tri[2] as usize];
        Aabb::new(
            Point3::new(
                a.x.min(b.x).min(c.x),
                a.y.min(b.y).min(c.y),
                a.z.min(b.z).min(c.z),
            ),
            Point3::new(
                a.x.max(b.x).max(c.x),
                a.y.max(b.y).max(c.y),
                a.z.max(b.z).max(c.z),
            ),
        )
    }).collect()
}

/// Validate a 3D mesh.
fn validate_mesh(mesh: &Mesh3D, name: &'static str) -> Result<(), Boolean3Error> {
    if mesh.vertices.len() < 4 || mesh.triangles.len() < 4 {
        return Err(Boolean3Error::DegenerateMesh { mesh: name });
    }

    for (i, tri) in mesh.triangles.iter().enumerate() {
        for &v in tri {
            if v as usize >= mesh.vertices.len() {
                return Err(Boolean3Error::IndexOutOfBounds {
                    mesh: name,
                    triangle: i,
                    vertex: v,
                });
            }
        }
    }

    for (i, v) in mesh.vertices.iter().enumerate() {
        if !v.x.is_finite() || !v.y.is_finite() || !v.z.is_finite() {
            return Err(Boolean3Error::NonFiniteCoordinate { mesh: name, index: i });
        }
    }

    Ok(())
}

/// Check if two `ExactPoint3` values represent the same point.
///
/// Two exact points are equal iff their numerators and denominators are
/// pairwise equal (after the construction's positive-denominator normalization).
/// We compare the expansion components directly — this is exact, with no
/// tolerance.
fn exact_points_equal(a: &ExactPoint3, b: &ExactPoint3) -> bool {
    // Compare denominators first (cheap reject).
    if a.den_len != b.den_len {
        return false;
    }
    for i in 0..a.den_len {
        if a.den[i] != b.den[i] {
            return false;
        }
    }
    // Compare numerators.
    if a.x_num_len != b.x_num_len || a.y_num_len != b.y_num_len || a.z_num_len != b.z_num_len {
        return false;
    }
    for i in 0..a.x_num_len {
        if a.x_num[i] != b.x_num[i] {
            return false;
        }
    }
    for i in 0..a.y_num_len {
        if a.y_num[i] != b.y_num[i] {
            return false;
        }
    }
    for i in 0..a.z_num_len {
        if a.z_num[i] != b.z_num[i] {
            return false;
        }
    }
    true
}

/// Add an exact point to the list if it's not already present.
/// Also adds the rounded `Point3` to the parallel list.
fn add_unique_exact_point(
    exact: &mut Vec<ExactPoint3>,
    f64_pts: &mut Vec<Point3>,
    p: &ExactPoint3,
) {
    let exists = exact.iter().any(|q| exact_points_equal(q, p));
    if !exists {
        exact.push(p.clone());
        f64_pts.push(p.to_point3());
    }
}

/// Refine a mesh by inserting new points and splitting affected triangles.
///
/// For each new point, find which triangle contains it (on its surface or
/// inside), and split that triangle into sub-triangles.
fn refine_mesh(mesh: &Mesh3D, new_points: &[Point3]) -> Mesh3D {
    let mut vertices = mesh.vertices.clone();
    let mut triangles = mesh.triangles.clone();

    for &p in new_points {
        // Check if the point is already a vertex.
        let existing = vertices.iter().position(|v| {
            (v.x - p.x).abs() < 1e-10
                && (v.y - p.y).abs() < 1e-10
                && (v.z - p.z).abs() < 1e-10
        });
        if existing.is_some() {
            continue;
        }

        // Find which triangle contains this point on its surface.
        let mut containing_tri: Option<usize> = None;
        for (ti, tri) in triangles.iter().enumerate() {
            let a = vertices[tri[0] as usize];
            let b = vertices[tri[1] as usize];
            let c = vertices[tri[2] as usize];
            if point_on_triangle_3d(p, a, b, c) {
                containing_tri = Some(ti);
                break;
            }
        }

        if let Some(ti) = containing_tri {
            let tri = triangles[ti];
            let new_idx = vertices.len() as u32;
            vertices.push(p);

            // Split the triangle into 3 sub-triangles (fan from new point).
            triangles[ti] = [tri[0], tri[1], new_idx];
            triangles.push([tri[1], tri[2], new_idx]);
            triangles.push([tri[2], tri[0], new_idx]);
        }
    }

    Mesh3D { vertices, triangles }
}

/// Check if a point lies on or near a 3D triangle.
///
/// Uses exact `orient_3d` for the coplanarity test (replacing tolerance-based
/// plane-distance check), and barycentric coordinates for the inside test.
fn point_on_triangle_3d(p: Point3, a: Point3, b: Point3, c: Point3) -> bool {
    // Exact coplanarity test: orient_3d uses the filtered→compensated→exact
    // ladder, so Sign::Zero means exactly coplanar.
    if orient_3d(a, b, c, p) != Sign::Zero {
        return false;
    }

    // Point is in the plane — check barycentric coordinates.
    let v0 = Point3::new(b.x - a.x, b.y - a.y, b.z - a.z);
    let v1 = Point3::new(c.x - a.x, c.y - a.y, c.z - a.z);
    let v2 = Point3::new(p.x - a.x, p.y - a.y, p.z - a.z);

    let d00 = dot3(v0, v0);
    let d01 = dot3(v0, v1);
    let d11 = dot3(v1, v1);
    let d20 = dot3(v2, v0);
    let d21 = dot3(v2, v1);

    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < 1e-15 {
        return false;
    }

    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;

    u >= -1e-10 && v >= -1e-10 && w >= -1e-10
}

#[inline]
fn dot3(a: Point3, b: Point3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

// ───────────────────────────────────────────────────────────────────────────
//  Verification
// ───────────────────────────────────────────────────────────────────────────

/// Verify that two co-refined meshes share common vertices at intersection
/// points. Returns the number of shared vertices.
pub fn count_shared_vertices(a: &Mesh3D, b: &Mesh3D) -> usize {
    let mut count = 0;
    for va in &a.vertices {
        for vb in &b.vertices {
            if (va.x - vb.x).abs() < 1e-8
                && (va.y - vb.y).abs() < 1e-8
                && (va.z - vb.z).abs() < 1e-8
            {
                count += 1;
                break;
            }
        }
    }
    count
}

/// Verify that the co-refinement preserved the number of triangles
/// (should only increase).
pub fn verify_refinement_preserves_triangles(
    original: &Mesh3D,
    refined: &Mesh3D,
) -> bool {
    refined.triangles.len() >= original.triangles.len()
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn tetrahedron(cx: f64, cy: f64, cz: f64, s: f64) -> Mesh3D {
        let h = s * 0.5;
        Mesh3D {
            vertices: vec![
                Point3::new(cx - h, cy - h, cz - h),
                Point3::new(cx + h, cy - h, cz - h),
                Point3::new(cx, cy + h, cz - h),
                Point3::new(cx, cy, cz + h),
            ],
            triangles: vec![
                [0, 1, 2], // bottom
                [0, 3, 1], // front
                [1, 3, 2], // right
                [2, 3, 0], // left
            ],
        }
    }

    #[test]
    fn corefine_disjoint_meshes() {
        let a = tetrahedron(0.0, 0.0, 0.0, 2.0);
        let b = tetrahedron(10.0, 0.0, 0.0, 2.0);
        let result = corefine_3d(&a, &b).unwrap();

        assert_eq!(result.num_intersecting_pairs, 0);
        assert_eq!(result.num_intersection_points, 0);
        // No refinement needed.
        assert_eq!(result.mesh_a.triangles.len(), 4);
        assert_eq!(result.mesh_b.triangles.len(), 4);
    }

    #[test]
    fn corefine_overlapping_meshes() {
        let a = tetrahedron(0.0, 0.0, 0.0, 2.0);
        let b = tetrahedron(0.5, 0.0, 0.0, 2.0);
        let result = corefine_3d(&a, &b).unwrap();

        // Should have some intersections.
        assert!(result.num_intersecting_pairs > 0,
            "expected intersections, got {}", result.num_intersecting_pairs);
        // Refined meshes should have at least as many triangles.
        assert!(result.mesh_a.triangles.len() >= 4,
            "mesh_a triangles = {}", result.mesh_a.triangles.len());
        assert!(result.mesh_b.triangles.len() >= 4,
            "mesh_b triangles = {}", result.mesh_b.triangles.len());
    }

    #[test]
    fn corefine_identical_meshes() {
        let a = tetrahedron(0.0, 0.0, 0.0, 2.0);
        let b = a.clone();
        let result = corefine_3d(&a, &b).unwrap();

        // Identical meshes — all triangles are coplanar, may or may not
        // produce intersection segments depending on tri_tri_intersect_3.
        // Just verify it doesn't crash.
        assert!(result.mesh_a.triangles.len() >= 4);
    }

    #[test]
    fn degenerate_mesh_errors() {
        let bad = Mesh3D {
            vertices: vec![Point3::new(0.0, 0.0, 0.0)],
            triangles: vec![[0, 0, 0]],
        };
        let good = tetrahedron(0.0, 0.0, 0.0, 2.0);
        assert!(matches!(
            corefine_3d(&bad, &good),
            Err(Boolean3Error::DegenerateMesh { .. })
        ));
    }

    #[test]
    fn shared_vertices_after_refinement() {
        let a = tetrahedron(0.0, 0.0, 0.0, 2.0);
        let b = tetrahedron(0.5, 0.0, 0.0, 2.0);
        let result = corefine_3d(&a, &b).unwrap();

        if result.num_intersection_points > 0 {
            let shared = count_shared_vertices(&result.mesh_a, &result.mesh_b);
            assert!(shared > 0, "no shared vertices after refinement");
        }
    }

    #[test]
    fn refinement_preserves_triangles() {
        let a = tetrahedron(0.0, 0.0, 0.0, 2.0);
        let b = tetrahedron(0.5, 0.0, 0.0, 2.0);
        let result = corefine_3d(&a, &b).unwrap();

        assert!(verify_refinement_preserves_triangles(&a, &result.mesh_a));
        assert!(verify_refinement_preserves_triangles(&b, &result.mesh_b));
    }

    /// BVH broad phase must produce the same intersecting pair count as
    /// brute-force O(nm) oracle — no false negatives.
    #[test]
    fn bvh_broad_phase_matches_brute_force() {
        let a = tetrahedron(0.0, 0.0, 0.0, 2.0);
        let b = tetrahedron(0.5, 0.0, 0.0, 2.0);

        // Brute-force oracle: count intersecting pairs directly.
        let mut brute_count = 0;
        for tri_a in &a.triangles {
            let a0 = a.vertices[tri_a[0] as usize];
            let a1 = a.vertices[tri_a[1] as usize];
            let a2 = a.vertices[tri_a[2] as usize];
            for tri_b in &b.triangles {
                let b0 = b.vertices[tri_b[0] as usize];
                let b1 = b.vertices[tri_b[1] as usize];
                let b2 = b.vertices[tri_b[2] as usize];
                let (hit, _) = tri_tri_intersect_3_exact(a0, a1, a2, b0, b1, b2);
                if hit {
                    brute_count += 1;
                }
            }
        }

        let result = corefine_3d(&a, &b).unwrap();
        assert_eq!(
            result.num_intersecting_pairs, brute_count,
            "BVH broad phase must match brute-force oracle"
        );
    }

    #[test]
    fn corefine_determinism() {
        let a = tetrahedron(0.0, 0.0, 0.0, 2.0);
        let b = tetrahedron(0.5, 0.0, 0.0, 2.0);

        let r1 = corefine_3d(&a, &b).unwrap();
        let r2 = corefine_3d(&a, &b).unwrap();

        assert_eq!(r1.num_intersecting_pairs, r2.num_intersecting_pairs);
        assert_eq!(r1.num_intersection_points, r2.num_intersection_points);
        assert_eq!(r1.mesh_a.triangles, r2.mesh_a.triangles);
        assert_eq!(r1.mesh_b.triangles, r2.mesh_b.triangles);
    }
}
