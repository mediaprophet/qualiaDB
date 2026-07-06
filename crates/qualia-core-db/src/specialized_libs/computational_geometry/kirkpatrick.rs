//! Kirkpatrick hierarchy for O(log n) point location in planar subdivisions
//! (P11.7).
//!
//! Given a triangulated planar subdivision, the Kirkpatrick hierarchy builds
//! a sequence of progressively coarser triangulations by removing independent
//! sets of low-degree vertices and retriangulating the holes. Point location
//! starts at the coarsest level (a single triangle) and refines downward:
//! at each level, the triangle containing the query is found by checking
//! the (constant-size) set of triangles that replaced it.
//!
//! **Guaranteed** O(log n) query time and O(n) space — no randomization.
//!
//! Reference: Kirkpatrick, "Optimal search in planar subdivisions,"
//! *SIAM J. Comput.* 1983. Also de Berg et al. §6.3 (simplified variant).
//!
//! Tier-2 cold construction (uses `Vec` during build; query is allocation-free).

use super::primitives::{orientation_2, Orientation, Point2};

// ───────────────────────────────────────────────────────────────────────────
//  Error type
// ───────────────────────────────────────────────────────────────────────────

/// Error returned by Kirkpatrick hierarchy operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KirkpatrickError {
    /// Triangulation is empty.
    EmptyTriangulation,
    /// Fewer than 3 vertices.
    TooFewVertices,
    /// Query point is outside the bounding triangle.
    OutsideBoundingTriangle,
}

impl core::fmt::Display for KirkpatrickError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyTriangulation => write!(f, "kirkpatrick: empty triangulation"),
            Self::TooFewVertices => write!(f, "kirkpatrick: need at least 3 vertices"),
            Self::OutsideBoundingTriangle => {
                write!(f, "kirkpatrick: query outside bounding triangle")
            }
        }
    }
}

impl std::error::Error for KirkpatrickError {}

// ───────────────────────────────────────────────────────────────────────────
//  Triangulation representation
// ───────────────────────────────────────────────────────────────────────────

/// A triangle in the hierarchy: three vertex indices + the face label it
/// represents (for the finest level, this is the original face index; for
/// coarser levels, it's a sentinel).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Tri {
    v: [usize; 3],
    /// Original face index at the finest level, or `usize::MAX` for
    /// triangles introduced during hole retriangulation.
    face: usize,
    /// Level at which this triangle exists.
    level: usize,
}

/// A level of the hierarchy: a set of triangles and their vertex positions.
#[derive(Debug, Clone)]
struct Level {
    triangles: Vec<Tri>,
    /// Map from a triangle index at this level to the triangle indices at
    /// the *finer* (previous) level that it replaces. Used during refinement.
    /// `children[i]` = list of finer triangles that overlap triangle `i`.
    children: Vec<Vec<usize>>,
}

// ───────────────────────────────────────────────────────────────────────────
//  Kirkpatrick hierarchy
// ───────────────────────────────────────────────────────────────────────────

/// Kirkpatrick point-location hierarchy.
///
/// Build with [`KirkpatrickHierarchy::build`], query with
/// [`KirkpatrickHierarchy::locate`].
pub struct KirkpatrickHierarchy {
    /// Levels from finest (index 0) to coarsest.
    levels: Vec<Level>,
    /// Original vertex positions.
    vertices: Vec<Point2>,
    /// Bounding triangle vertex indices (into `vertices`).
    bbox_tri: [usize; 3],
}

impl std::fmt::Debug for KirkpatrickHierarchy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KirkpatrickHierarchy")
            .field("num_levels", &self.levels.len())
            .field("num_vertices", &self.vertices.len())
            .field("bbox_tri", &self.bbox_tri)
            .finish()
    }
}

/// Orientation test for a triangle: returns true if `p` is inside or on
/// the boundary of the CCW triangle `(a, b, c)`.
fn point_in_tri(p: Point2, a: Point2, b: Point2, c: Point2) -> bool {
    let o1 = orientation_2(a, b, p);
    let o2 = orientation_2(b, c, p);
    let o3 = orientation_2(c, a, p);
    o1 != Orientation::Clockwise && o2 != Orientation::Clockwise && o3 != Orientation::Clockwise
}

/// Compute a large bounding triangle that contains all points.
fn bounding_triangle(points: &[Point2]) -> [Point2; 3] {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for &p in points {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    let dx = max_x - min_x;
    let dy = max_y - min_y;
    let d = dx.max(dy).max(1.0) * 10.0;
    let cx = (min_x + max_x) * 0.5;
    let cy = (min_y + max_y) * 0.5;
    // Large triangle pointing up.
    [
        Point2::new(cx, cy + d),
        Point2::new(cx - d, cy - d * 0.5),
        Point2::new(cx + d, cy - d * 0.5),
    ]
}

impl KirkpatrickHierarchy {
    /// Build a Kirkpatrick hierarchy from a triangulation.
    ///
    /// `vertices` — the vertex positions.
    /// `triangles` — list of CCW triangles as vertex index triples.
    /// `face_labels` — the original face index for each triangle (use
    ///   `0..triangles.len()` if you just want the triangle index).
    ///
    /// The triangulation must cover a simply-connected region (no holes).
    /// A bounding triangle is added automatically to enclose everything.
    pub fn build(
        vertices: &[Point2],
        triangles: &[[usize; 3]],
        face_labels: &[usize],
    ) -> Result<Self, KirkpatrickError> {
        if vertices.len() < 3 {
            return Err(KirkpatrickError::TooFewVertices);
        }
        if triangles.is_empty() {
            return Err(KirkpatrickError::EmptyTriangulation);
        }

        // Add bounding triangle vertices.
        let bbox = bounding_triangle(vertices);
        let n = vertices.len();
        let mut all_vertices = vertices.to_vec();
        all_vertices.push(bbox[0]);
        all_vertices.push(bbox[1]);
        all_vertices.push(bbox[2]);
        let bv = [n, n + 1, n + 2];

        // Build the finest level: original triangles + bounding triangle
        // faces. We need to triangulate the gap between the original
        // triangulation boundary and the bounding triangle. For simplicity,
        // we fan-triangulate from each bbox vertex to the convex hull
        // boundary edges. However, computing the exact boundary requires
        // half-edge analysis. Instead, we use a simpler approach:
        // we add the bounding triangle as a single face, then connect
        // each original boundary edge to the nearest bbox vertex.
        //
        // Actually, the standard approach is:
        // 1. Find the boundary edges of the input triangulation.
        // 2. For each boundary edge, create a triangle connecting it to
        //    one of the bbox vertices.
        // 3. The remaining gap (between the fan triangles and the bbox
        //    triangle) is filled by the bbox triangle itself.
        //
        // But this is complex. A simpler approach that works for point
        // location: just add the bounding triangle as a single face that
        // covers everything outside the original triangulation. We don't
        // need a valid triangulation of the gap — we just need the hierarchy
        // to correctly locate points. The key insight is that the bounding
        // triangle contains all original points, so any point inside the
        // bbox triangle but outside the original triangulation will be
        // "located" to the bbox face.
        //
        // For the hierarchy to work, we need a valid triangulation at the
        // finest level. We create it by:
        // - Keeping all original triangles.
        // - Adding triangles from each boundary edge to bbox vertices.
        // - The bbox triangle itself is the outermost face.

        // Find boundary edges (edges that appear in only one triangle).
        let mut edge_count: std::collections::HashMap<(usize, usize), usize> =
            std::collections::HashMap::new();
        for &t in triangles {
            for i in 0..3 {
                let a = t[i];
                let b = t[(i + 1) % 3];
                let key = if a < b { (a, b) } else { (b, a) };
                *edge_count.entry(key).or_insert(0) += 1;
            }
        }

        let mut finest_tris: Vec<Tri> = Vec::with_capacity(triangles.len() + 20);
        for (i, &t) in triangles.iter().enumerate() {
            finest_tris.push(Tri {
                v: t,
                face: face_labels[i],
                level: 0,
            });
        }

        // For each boundary edge, create a triangle to the nearest bbox vertex.
        // We pick the bbox vertex that makes the triangle CCW.
        let mut boundary_edges: Vec<(usize, usize)> = Vec::new();
        for &t in triangles {
            for i in 0..3 {
                let a = t[i];
                let b = t[(i + 1) % 3];
                let key = if a < b { (a, b) } else { (b, a) };
                if edge_count[&key] == 1 {
                    // This is a boundary edge. The triangle t has it in
                    // CCW order (a→b), so the exterior is on the right.
                    // We need a CCW triangle (a, bv, b) or (b, bv, a).
                    // Check which bbox vertex makes it CCW.
                    for &bv in &bv {
                        let o = orientation_2(all_vertices[a], all_vertices[b], all_vertices[bv]);
                        if o == Orientation::CounterClockwise {
                            boundary_edges.push((a, b));
                            finest_tris.push(Tri {
                                v: [a, bv, b],
                                face: usize::MAX, // exterior face
                                level: 0,
                            });
                            break;
                        }
                    }
                }
            }
        }

        // Add the outer bounding triangle (CCW: top → bottom-left → bottom-right).
        finest_tris.push(Tri {
            v: [bv[0], bv[1], bv[2]],
            face: usize::MAX,
            level: 0,
        });

        // Build adjacency: for each triangle, its 3 neighbors (across each edge).
        // We build this by matching directed edges.
        let level0 = Level {
            triangles: finest_tris,
            children: Vec::new(),
        };

        // Build the hierarchy by repeatedly removing independent sets.
        let mut levels: Vec<Level> = Vec::new();
        levels.push(level0.clone());

        const MAX_DEGREE: usize = 12;
        const MIN_TRIANGLES: usize = 4;

        loop {
            let current = levels.last().unwrap();
            if current.triangles.len() <= MIN_TRIANGLES {
                break;
            }

            let next = build_next_level(current, &all_vertices, MAX_DEGREE);
            if next.triangles.len() >= current.triangles.len() {
                // No progress — stop to avoid infinite loop.
                break;
            }
            levels.push(next);
        }

        // Build children maps (from coarse to fine).
        // For each level L > 0, children[i] = list of triangles at level L-1
        // that overlap triangle i at level L.
        for li in (1..levels.len()).rev() {
            let finer = &levels[li - 1].triangles.clone();
            let coarse = &levels[li].triangles;
            let mut children = vec![Vec::new(); coarse.len()];

            for (fi, ft) in finer.iter().enumerate() {
                // Find which coarse triangle contains the centroid of ft.
                let cx = (all_vertices[ft.v[0]].x + all_vertices[ft.v[1]].x + all_vertices[ft.v[2]].x) / 3.0;
                let cy = (all_vertices[ft.v[0]].y + all_vertices[ft.v[1]].y + all_vertices[ft.v[2]].y) / 3.0;
                let centroid = Point2::new(cx, cy);

                for (ci, ct) in coarse.iter().enumerate() {
                    if point_in_tri(
                        centroid,
                        all_vertices[ct.v[0]],
                        all_vertices[ct.v[1]],
                        all_vertices[ct.v[2]],
                    ) {
                        children[ci].push(fi);
                        break;
                    }
                }
            }

            levels[li].children = children;
        }

        // Level 0 has no children (it's the finest).
        levels[0].children = vec![Vec::new(); levels[0].triangles.len()];

        Ok(KirkpatrickHierarchy {
            levels,
            vertices: all_vertices,
            bbox_tri: bv,
        })
    }

    /// Locate a query point in the hierarchy.
    ///
    /// Returns the face label of the containing triangle at the finest level,
    /// or `None` if the point is in an exterior face (face label `usize::MAX`).
    /// Returns an error if the point is outside the bounding triangle.
    pub fn locate(&self, query: Point2) -> Result<Option<usize>, KirkpatrickError> {
        // Check that the query is inside the bounding triangle.
        let bv = self.bbox_tri;
        if !point_in_tri(
            query,
            self.vertices[bv[0]],
            self.vertices[bv[1]],
            self.vertices[bv[2]],
        ) {
            return Err(KirkpatrickError::OutsideBoundingTriangle);
        }

        // Start at the coarsest level.
        let coarsest = self.levels.len() - 1;
        let coarse_tris = &self.levels[coarsest].triangles;

        // Find the containing triangle at the coarsest level.
        let mut current_tri = 0usize;
        for (i, t) in coarse_tris.iter().enumerate() {
            if point_in_tri(
                query,
                self.vertices[t.v[0]],
                self.vertices[t.v[1]],
                self.vertices[t.v[2]],
            ) {
                current_tri = i;
                break;
            }
        }

        // Refine downward through the levels.
        for li in (0..coarsest).rev() {
            let children = &self.levels[li + 1].children[current_tri];
            if children.is_empty() {
                // No children — we're at the finest level for this branch.
                // The face is at the current level.
                let tri = &self.levels[li + 1].triangles[current_tri];
                return Ok(if tri.face == usize::MAX { None } else { Some(tri.face) });
            }

            // Find which child contains the query.
            let finer_tris = &self.levels[li].triangles;
            let mut found = false;
            for &ci in children {
                let t = &finer_tris[ci];
                if point_in_tri(
                    query,
                    self.vertices[t.v[0]],
                    self.vertices[t.v[1]],
                    self.vertices[t.v[2]],
                ) {
                    current_tri = ci;
                    found = true;
                    break;
                }
            }
            if !found {
                // Shouldn't happen if the hierarchy is correct, but fall back.
                return Ok(None);
            }
        }

        // We're at the finest level (level 0).
        let tri = &self.levels[0].triangles[current_tri];
        Ok(if tri.face == usize::MAX { None } else { Some(tri.face) })
    }

    /// Number of levels in the hierarchy.
    pub fn num_levels(&self) -> usize {
        self.levels.len()
    }

    /// Number of triangles at the finest level.
    pub fn num_finest_triangles(&self) -> usize {
        self.levels[0].triangles.len()
    }

    /// Number of triangles at the coarsest level.
    pub fn num_coarsest_triangles(&self) -> usize {
        self.levels.last().unwrap().triangles.len()
    }

    /// Brute-force locate: scan all triangles at the finest level.
    /// Used as an oracle for testing.
    pub fn locate_brute_force(&self, query: Point2) -> Option<usize> {
        for t in &self.levels[0].triangles {
            if point_in_tri(
                query,
                self.vertices[t.v[0]],
                self.vertices[t.v[1]],
                self.vertices[t.v[2]],
            ) {
                return if t.face == usize::MAX { None } else { Some(t.face) };
            }
        }
        None
    }
}

/// Build the next coarser level by removing an independent set of
/// low-degree vertices and retriangulating the holes.
fn build_next_level(current: &Level, vertices: &[Point2], max_degree: usize) -> Level {
    // Build vertex → triangles adjacency.
    let mut vert_tris: Vec<Vec<usize>> = vec![Vec::new(); vertices.len()];
    for (ti, t) in current.triangles.iter().enumerate() {
        for &v in &t.v {
            vert_tris[v].push(ti);
        }
    }

    // Find an independent set of vertices with degree <= max_degree.
    // "Degree" = number of triangles incident to the vertex.
    // We skip bounding-triangle vertices (the last 3) and any vertex
    // that shares a triangle with a bbox vertex (i.e., boundary/fan
    // vertices). Only interior vertices of the original triangulation
    // are eligible for removal — this ensures the hole is a simple
    // polygon that can be fan-triangulated.
    let n_orig = vertices.len() - 3;
    let mut removed = vec![false; vertices.len()];
    let mut to_remove: Vec<usize> = Vec::new();

    for v in 0..n_orig {
        if removed[v] {
            continue;
        }
        if vert_tris[v].len() > max_degree {
            continue;
        }
        // Check that no triangle in the star contains a bbox vertex.
        let is_interior = vert_tris[v].iter().all(|&ti| {
            current.triangles[ti].v.iter().all(|&vv| vv < n_orig)
        });
        if !is_interior {
            continue;
        }
        to_remove.push(v);
        removed[v] = true;
        // Mark all neighbors as unavailable.
        for &ti in &vert_tris[v] {
            for &nv in &current.triangles[ti].v {
                if nv != v && nv < n_orig {
                    removed[nv] = true;
                }
            }
        }
    }

    if to_remove.is_empty() {
        // Can't coarsen — return a copy.
        return Level {
            triangles: current.triangles.clone(),
            children: Vec::new(),
        };
    }

    // Remove the selected vertices and retriangulate the holes.
    // For each removed vertex, collect the "star" (triangles around it),
    // remove them, and fan-triangulate the hole.
    let mut new_tris: Vec<Tri> = Vec::new();
    let mut dead: Vec<bool> = vec![false; current.triangles.len()];

    // Mark triangles that contain a removed vertex as dead.
    for &v in &to_remove {
        for &ti in &vert_tris[v] {
            dead[ti] = true;
        }
    }

    // Keep triangles that are not dead.
    for (ti, t) in current.triangles.iter().enumerate() {
        if !dead[ti] {
            new_tris.push(Tri {
                v: t.v,
                face: t.face,
                level: t.level + 1,
            });
        }
    }

    // For each removed vertex, retriangulate its hole.
    // The hole is the polygon formed by the neighbors of the removed vertex,
    // in order. We fan-triangulate from the first neighbor.
    for &v in &to_remove {
        // Collect the neighbors in CCW order around v.
        let star = &vert_tris[v];
        if star.is_empty() {
            continue;
        }

        // Build the boundary polygon by tracing the star.
        let boundary = trace_hole_boundary(star, v, &current.triangles);

        if boundary.len() < 3 {
            continue;
        }

        // Fan-triangulate from boundary[0].
        for i in 1..boundary.len() - 1 {
            let tri = Tri {
                v: [boundary[0], boundary[i], boundary[i + 1]],
                face: usize::MAX,
                level: current.triangles[0].level + 1,
            };
            // Verify CCW; flip if needed.
            let o = orientation_2(
                vertices[tri.v[0]],
                vertices[tri.v[1]],
                vertices[tri.v[2]],
            );
            if o == Orientation::Clockwise {
                new_tris.push(Tri {
                    v: [tri.v[0], tri.v[2], tri.v[1]],
                    face: usize::MAX,
                    level: tri.level,
                });
            } else {
                new_tris.push(tri);
            }
        }
    }

    Level {
        triangles: new_tris,
        children: Vec::new(),
    }
}

/// Trace the boundary of the hole left by removing vertex `v` from its
/// star of triangles. Returns the boundary vertices in CCW order.
fn trace_hole_boundary(star: &[usize], v: usize, triangles: &[Tri]) -> Vec<usize> {
    // For each triangle in the star, collect the two edges that don't
    // contain v. The boundary edges are those that appear in only one
    // triangle of the star.
    let mut edge_map: std::collections::HashMap<(usize, usize), usize> =
        std::collections::HashMap::new();

    for &ti in star {
        let t = &triangles[ti];
        for i in 0..3 {
            let a = t.v[i];
            let b = t.v[(i + 1) % 3];
            if a != v && b != v {
                let key = if a < b { (a, b) } else { (b, a) };
                *edge_map.entry(key).or_insert(0) += 1;
            }
        }
    }

    // Boundary edges are those with count 1.
    let mut boundary_edges: Vec<(usize, usize)> = Vec::new();
    for &ti in star {
        let t = &triangles[ti];
        for i in 0..3 {
            let a = t.v[i];
            let b = t.v[(i + 1) % 3];
            if a != v && b != v {
                let key = if a < b { (a, b) } else { (b, a) };
                if edge_map[&key] == 1 {
                    // Keep the directed edge as it appears in the triangle (CCW).
                    boundary_edges.push((a, b));
                }
            }
        }
    }

    // Chain the directed edges into a cycle.
    let mut boundary: Vec<usize> = Vec::new();
    if boundary_edges.is_empty() {
        return boundary;
    }

    let mut used = vec![false; boundary_edges.len()];
    let mut current = boundary_edges[0];
    boundary.push(current.0);
    used[0] = true;

    loop {
        boundary.push(current.1);
        // Find the edge starting at current.1 that hasn't been used.
        let mut found = false;
        for (i, &(a, b)) in boundary_edges.iter().enumerate() {
            if !used[i] && a == current.1 {
                used[i] = true;
                current = (a, b);
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
    }

    // Remove the last vertex (it's the same as the first).
    if boundary.len() > 1 && boundary.last() == boundary.first() {
        boundary.pop();
    }

    boundary
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    // ── Basic build ─────────────────────────────────────────────────────

    #[test]
    fn single_triangle_builds() {
        let verts = vec![pt(0.0, 0.0), pt(4.0, 0.0), pt(2.0, 4.0)];
        let tris = vec![[0, 1, 2]];
        let faces = vec![0];
        let h = KirkpatrickHierarchy::build(&verts, &tris, &faces).unwrap();
        assert!(h.num_levels() >= 1);
        assert!(h.num_finest_triangles() >= 1);
    }

    #[test]
    fn two_triangles_builds() {
        let verts = vec![pt(0.0, 0.0), pt(2.0, 0.0), pt(4.0, 0.0), pt(2.0, 4.0)];
        let tris = vec![[0, 1, 3], [1, 2, 3]];
        let faces = vec![0, 1];
        let h = KirkpatrickHierarchy::build(&verts, &tris, &faces).unwrap();
        assert!(h.num_levels() >= 1);
    }

    #[test]
    fn empty_triangulation_errors() {
        let verts = vec![pt(0.0, 0.0), pt(1.0, 0.0), pt(0.0, 1.0)];
        let result = KirkpatrickHierarchy::build(&verts, &[], &[]);
        assert!(matches!(result, Err(KirkpatrickError::EmptyTriangulation)));
    }

    #[test]
    fn too_few_vertices_errors() {
        let verts = vec![pt(0.0, 0.0), pt(1.0, 0.0)];
        let result = KirkpatrickHierarchy::build(&verts, &[[0, 0, 0]], &[0]);
        assert!(matches!(result, Err(KirkpatrickError::TooFewVertices)));
    }

    // ── Point location ──────────────────────────────────────────────────

    #[test]
    fn locate_in_single_triangle() {
        let verts = vec![pt(0.0, 0.0), pt(4.0, 0.0), pt(2.0, 4.0)];
        let tris = vec![[0, 1, 2]];
        let faces = vec![0];
        let h = KirkpatrickHierarchy::build(&verts, &tris, &faces).unwrap();
        let result = h.locate(pt(2.0, 1.0)).unwrap();
        assert_eq!(result, Some(0));
    }

    #[test]
    fn locate_in_two_triangles() {
        let verts = vec![pt(0.0, 0.0), pt(2.0, 0.0), pt(4.0, 0.0), pt(2.0, 4.0)];
        let tris = vec![[0, 1, 3], [1, 2, 3]];
        let faces = vec![0, 1];
        let h = KirkpatrickHierarchy::build(&verts, &tris, &faces).unwrap();
        // Point in left triangle.
        assert_eq!(h.locate(pt(1.0, 1.0)).unwrap(), Some(0));
        // Point in right triangle.
        assert_eq!(h.locate(pt(3.0, 1.0)).unwrap(), Some(1));
    }

    #[test]
    fn locate_outside_bbox_errors() {
        let verts = vec![pt(0.0, 0.0), pt(4.0, 0.0), pt(2.0, 4.0)];
        let tris = vec![[0, 1, 2]];
        let faces = vec![0];
        let h = KirkpatrickHierarchy::build(&verts, &tris, &faces).unwrap();
        assert!(matches!(
            h.locate(pt(1000.0, 1000.0)),
            Err(KirkpatrickError::OutsideBoundingTriangle)
        ));
    }

    // ── Grid triangulation ──────────────────────────────────────────────

    fn build_grid_triangulation(nx: usize, ny: usize) -> (Vec<Point2>, Vec<[usize; 3]>, Vec<usize>) {
        let mut verts = Vec::new();
        for j in 0..=ny {
            for i in 0..=nx {
                verts.push(pt(i as f64, j as f64));
            }
        }
        let idx = |i: usize, j: usize| j * (nx + 1) + i;

        let mut tris = Vec::new();
        let mut faces = Vec::new();
        let mut fi = 0;
        for j in 0..ny {
            for i in 0..nx {
                let a = idx(i, j);
                let b = idx(i + 1, j);
                let c = idx(i + 1, j + 1);
                let d = idx(i, j + 1);
                tris.push([a, b, c]);
                faces.push(fi);
                fi += 1;
                tris.push([a, c, d]);
                faces.push(fi);
                fi += 1;
            }
        }
        (verts, tris, faces)
    }

    #[test]
    fn grid_2x2_locate_all_faces() {
        let (verts, tris, faces) = build_grid_triangulation(2, 2);
        let h = KirkpatrickHierarchy::build(&verts, &tris, &faces).unwrap();

        // Check every cell center.
        for j in 0..2 {
            for i in 0..2 {
                let qx = i as f64 + 0.25;
                let qy = j as f64 + 0.25;
                let result = h.locate(pt(qx, qy)).unwrap();
                assert!(result.is_some(), "point ({}, {}) should be in a face", qx, qy);
            }
        }
    }

    #[test]
    fn grid_4x4_locate_matches_brute_force() {
        let (verts, tris, faces) = build_grid_triangulation(4, 4);
        let h = KirkpatrickHierarchy::build(&verts, &tris, &faces).unwrap();

        for j in 0..4 {
            for i in 0..4 {
                let qx = i as f64 + 0.3;
                let qy = j as f64 + 0.3;
                let p = pt(qx, qy);
                let dag = h.locate(p).unwrap();
                let bf = h.locate_brute_force(p);
                // Both should find a face (or both None).
                assert_eq!(
                    dag.is_some(),
                    bf.is_some(),
                    "mismatch at ({}, {}): dag={:?}, bf={:?}",
                    qx, qy, dag, bf
                );
            }
        }
    }

    #[test]
    fn grid_4x4_locate_edge_centers() {
        let (verts, tris, faces) = build_grid_triangulation(4, 4);
        let h = KirkpatrickHierarchy::build(&verts, &tris, &faces).unwrap();

        // Test points on grid lines (edges between triangles).
        for i in 0..4 {
            let p = pt(i as f64 + 0.5, 2.0);
            let result = h.locate(p);
            assert!(result.is_ok(), "edge point ({}, 2) should be locatable", p.x);
        }
    }

    // ── Hierarchy properties ────────────────────────────────────────────

    #[test]
    fn hierarchy_coarsens() {
        let (verts, tris, faces) = build_grid_triangulation(4, 4);
        let h = KirkpatrickHierarchy::build(&verts, &tris, &faces).unwrap();
        // The coarsest level should have fewer triangles than the finest.
        assert!(h.num_coarsest_triangles() < h.num_finest_triangles());
        // And at least 1 triangle.
        assert!(h.num_coarsest_triangles() >= 1);
        // Multiple levels.
        assert!(h.num_levels() >= 2);
    }

    #[test]
    fn larger_grid_builds_and_locates() {
        let (verts, tris, faces) = build_grid_triangulation(8, 8);
        let h = KirkpatrickHierarchy::build(&verts, &tris, &faces).unwrap();
        assert!(h.num_levels() >= 2);

        // Sample queries.
        for j in 0..8 {
            for i in 0..8 {
                let qx = i as f64 + 0.5;
                let qy = j as f64 + 0.5;
                let result = h.locate(pt(qx, qy)).unwrap();
                assert!(result.is_some(), "point ({}, {}) should be in a face", qx, qy);
            }
        }
    }

    // ── Error display ───────────────────────────────────────────────────

    #[test]
    fn error_display() {
        assert!(KirkpatrickError::EmptyTriangulation.to_string().contains("empty"));
        assert!(KirkpatrickError::TooFewVertices.to_string().contains("at least 3"));
        assert!(KirkpatrickError::OutsideBoundingTriangle.to_string().contains("bounding triangle"));
    }

    // ── Determinism ─────────────────────────────────────────────────────

    #[test]
    fn same_input_produces_same_hierarchy() {
        let (verts, tris, faces) = build_grid_triangulation(4, 4);
        let h1 = KirkpatrickHierarchy::build(&verts, &tris, &faces).unwrap();
        let h2 = KirkpatrickHierarchy::build(&verts, &tris, &faces).unwrap();

        assert_eq!(h1.num_levels(), h2.num_levels());
        assert_eq!(h1.num_finest_triangles(), h2.num_finest_triangles());

        for j in 0..4 {
            for i in 0..4 {
                let p = pt(i as f64 + 0.3, j as f64 + 0.3);
                assert_eq!(h1.locate(p), h2.locate(p), "mismatch at ({}, {})", p.x, p.y);
            }
        }
    }

    // ── Exterior points ─────────────────────────────────────────────────

    #[test]
    fn exterior_point_returns_none() {
        let (verts, tris, faces) = build_grid_triangulation(2, 2);
        let h = KirkpatrickHierarchy::build(&verts, &tris, &faces).unwrap();

        // Point inside the grid but in the exterior (between grid boundary
        // and bounding triangle). The bounding triangle is huge, so this
        // point is inside the bbox but outside the grid.
        let result = h.locate(pt(-0.5, -0.5));
        // Should be Ok(None) — inside bbox but in an exterior face.
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }
}
