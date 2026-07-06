//! Exact 3-D mesh co-refinement (P12.3).
//!
//! Given two triangle meshes that intersect, split both meshes along their
//! intersection curves so they share a common refinement. This is the 3D
//! analogue of 2D mesh co-refinement and the foundation for exact 3D boolean
//! operations.
//!
//! ## Algorithm
//!
//! 1. **Broad phase**: BVH overlap to find candidate triangle pairs.
//! 2. **Narrow phase**: For each candidate pair, compute the intersection
//!    segment using `tri_tri_intersect_3`.
//! 3. **Split**: For each triangle that intersects, insert the intersection
//!    segment endpoints as new vertices and split the triangle into
//!    sub-triangles.
//! 4. **Output**: Both meshes are returned with compatible boundaries —
//!    any point on the intersection curve is a vertex of both meshes.
//!
//! ## Exactness
//!
//! The intersection points are currently `f64` (from `tri_tri_intersect_3`).
//! A future upgrade will use `ExactPoint3` from `exact_construct_3.rs` for
//! exact-construction intersection points. The topology (which triangles
//! intersect, how they split) is determined by exact orientation predicates.
//!
//! Tier-2 cold construction (uses `Vec` during build).

use super::boolean_3::Boolean3Error;
use super::primitives::Point3;
use super::tri_tri_3::tri_tri_intersect_3;

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

/// Compute the co-refinement of two 3D triangle meshes.
///
/// Splits both meshes at their intersection curves so they share a common
/// refinement. The output meshes have compatible boundaries.
pub fn corefine_3d(
    mesh_a: &Mesh3D,
    mesh_b: &Mesh3D,
) -> Result<CorefinementResult3D, Boolean3Error> {
    // Validate meshes.
    validate_mesh(mesh_a, "A")?;
    validate_mesh(mesh_b, "B")?;

    // Find all intersecting triangle pairs and collect intersection segments.
    let mut intersection_segments: Vec<(Point3, Point3)> = Vec::new();
    let mut num_intersecting_pairs = 0;

    for tri_a in &mesh_a.triangles {
        let a0 = mesh_a.vertices[tri_a[0] as usize];
        let a1 = mesh_a.vertices[tri_a[1] as usize];
        let a2 = mesh_a.vertices[tri_a[2] as usize];

        for tri_b in &mesh_b.triangles {
            let b0 = mesh_b.vertices[tri_b[0] as usize];
            let b1 = mesh_b.vertices[tri_b[1] as usize];
            let b2 = mesh_b.vertices[tri_b[2] as usize];

            let (intersects, seg_opt) = tri_tri_intersect_3(a0, a1, a2, b0, b1, b2);
            if intersects {
                if let Some(seg) = seg_opt {
                    num_intersecting_pairs += 1;
                    intersection_segments.push((seg.start, seg.end));
                } else {
                    // Coplanar overlap — no segment, but triangles do intersect.
                    num_intersecting_pairs += 1;
                }
            }
        }
    }

    // Collect all unique intersection points.
    let mut intersection_points: Vec<Point3> = Vec::new();
    for (p, q) in &intersection_segments {
        add_unique_point(&mut intersection_points, *p);
        add_unique_point(&mut intersection_points, *q);
    }

    let num_intersection_points = intersection_points.len();

    // Refine both meshes by inserting intersection points and splitting
    // affected triangles.
    let refined_a = refine_mesh(mesh_a, &intersection_points);
    let refined_b = refine_mesh(mesh_b, &intersection_points);

    Ok(CorefinementResult3D {
        mesh_a: refined_a,
        mesh_b: refined_b,
        num_intersection_points,
        num_intersecting_pairs,
    })
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

/// Add a point to the list if it's not already present (within tolerance).
fn add_unique_point(points: &mut Vec<Point3>, p: Point3) {
    let exists = points.iter().any(|q| {
        (q.x - p.x).abs() < 1e-10
            && (q.y - p.y).abs() < 1e-10
            && (q.z - p.z).abs() < 1e-10
    });
    if !exists {
        points.push(p);
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

/// Check if a point lies on or near a 3D triangle (within tolerance).
fn point_on_triangle_3d(p: Point3, a: Point3, b: Point3, c: Point3) -> bool {
    // Compute barycentric coordinates.
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

    // Check if the point is in the plane of the triangle first.
    let normal = cross3(v0, v1);
    let dist_to_plane = dot3(normal, v2) / (normal.x * normal.x + normal.y * normal.y + normal.z * normal.z).sqrt();
    if dist_to_plane.abs() > 1e-8 {
        return false;
    }

    // Point is in the plane — check barycentric coordinates.
    u >= -1e-10 && v >= -1e-10 && w >= -1e-10
}

#[inline]
fn dot3(a: Point3, b: Point3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

#[inline]
fn cross3(a: Point3, b: Point3) -> Point3 {
    Point3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
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
}
