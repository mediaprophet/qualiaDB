//! P13.8 - Anisotropic surface remeshing with feature preservation.
//!
//! This is a conservative Tier-2 cold builder over the P13.1 metric tensor
//! field. It refines edges whose metric length is above the declared upper
//! band, smooths only unconstrained interior vertices with metric-weighted
//! one-ring relaxation, and projects moved vertices back onto the input surface
//! through a BVH-pruned exact closest-triangle query.
//!
//! Scope fence: this is a feature-preserving refinement/relaxation pass, not a
//! full anisotropic edge-collapse/flip optimizer. It preserves tagged corners
//! and crease endpoints exactly, preserves boundary vertices, and fails closed
//! on invalid input or indefinite metrics.

use super::bvh::{build_bvh_recursive, BvhError, BvhNode, MAX_BVH_DEPTH};
use super::distance::Aabb;
use super::mesh_quality::{
    check_field_conformance_tri, AnisotropyField, FieldConformance, MeshQualityError,
};
use super::primitives::Point3;
use super::topology::{build_triangle_half_edges, required_edge_slots, EdgeSlot, HalfEdge};

/// An undirected feature/crease edge. Endpoints are canonicalized internally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureEdge {
    pub a: u32,
    pub b: u32,
}

impl FeatureEdge {
    #[inline]
    pub fn new(a: u32, b: u32) -> Self {
        if a <= b {
            Self { a, b }
        } else {
            Self { a: b, b: a }
        }
    }
}

/// Options for [`anisotropic_remesh`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnisotropicRemeshOptions {
    /// Number of split/relax passes. `0` validates and copies through.
    pub iterations: u32,
    /// Split edges longer than this metric length. A value around 4/3 matches
    /// the isotropic remesher's long-edge threshold.
    pub max_metric_edge: f64,
    /// Relaxation step in `[0, 1]`; `0` disables smoothing.
    pub relaxation: f64,
}

impl Default for AnisotropicRemeshOptions {
    fn default() -> Self {
        Self {
            iterations: 4,
            max_metric_edge: 4.0 / 3.0,
            relaxation: 0.5,
        }
    }
}

/// Summary of an anisotropic remesh pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnisotropicRemeshReport {
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub splits: usize,
    pub relaxations: usize,
    pub bvh_projection_queries: usize,
    pub preserved_feature_vertices: usize,
    pub field_conformance: FieldConformance,
}

/// Failure modes for [`anisotropic_remesh`].
#[derive(Debug, Clone, PartialEq)]
pub enum AnisotropicRemeshError {
    InvalidOptions,
    IndexOutOfBounds { triangle: usize, vertex: u32 },
    FeatureIndexOutOfBounds { vertex: u32 },
    DegenerateInputFace { triangle: usize },
    NonFiniteCoordinate { index: usize },
    NonManifoldInput,
    Metric(MeshQualityError),
    Bvh(BvhError),
    VertexOutputTooSmall { required: usize },
    TriangleOutputTooSmall { required: usize },
}

impl From<MeshQualityError> for AnisotropicRemeshError {
    fn from(err: MeshQualityError) -> Self {
        Self::Metric(err)
    }
}

impl From<BvhError> for AnisotropicRemeshError {
    fn from(err: BvhError) -> Self {
        Self::Bvh(err)
    }
}

/// Conservative anisotropic remeshing against a metric tensor field.
///
/// The public surface is caller-buffered. Internal `Vec` scratch is used only in
/// this cold construction path. Tagged `fixed_vertices` and endpoints of
/// `crease_edges` are preserved exactly in the output.
pub fn anisotropic_remesh(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    field: &AnisotropyField,
    fixed_vertices: &[u32],
    crease_edges: &[FeatureEdge],
    options: AnisotropicRemeshOptions,
    out_vertices: &mut [Point3],
    out_triangles: &mut [[u32; 3]],
) -> Result<AnisotropicRemeshReport, AnisotropicRemeshError> {
    if !(options.max_metric_edge.is_finite()
        && options.max_metric_edge > 0.0
        && options.relaxation.is_finite()
        && options.relaxation >= 0.0
        && options.relaxation <= 1.0)
    {
        return Err(AnisotropicRemeshError::InvalidOptions);
    }
    validate_input(vertices, triangles, fixed_vertices, crease_edges)?;

    if !validate_manifold(vertices.len() as u32, triangles) {
        return Err(AnisotropicRemeshError::NonManifoldInput);
    }

    let projector = BvhSurfaceProjector::new(vertices, triangles)?;
    let mut mesh = AnisoMesh::new(vertices, triangles, fixed_vertices, crease_edges);
    let mut splits = 0usize;
    let mut relaxations = 0usize;
    let mut bvh_projection_queries = 0usize;

    for _ in 0..options.iterations {
        splits += mesh.split_metric_long_edges(field, options.max_metric_edge)?;
        let (moved, projected) = mesh.metric_relax(field, &projector, options.relaxation)?;
        relaxations += moved;
        bvh_projection_queries += projected;
    }

    mesh.compact();

    if out_vertices.len() < mesh.vertices.len() {
        return Err(AnisotropicRemeshError::VertexOutputTooSmall {
            required: mesh.vertices.len(),
        });
    }
    if out_triangles.len() < mesh.triangles.len() {
        return Err(AnisotropicRemeshError::TriangleOutputTooSmall {
            required: mesh.triangles.len(),
        });
    }

    out_vertices[..mesh.vertices.len()].copy_from_slice(&mesh.vertices);
    out_triangles[..mesh.triangles.len()].copy_from_slice(&mesh.triangles);
    let field_conformance = check_field_conformance_tri(&mesh.vertices, &mesh.triangles, field)?;

    Ok(AnisotropicRemeshReport {
        vertex_count: mesh.vertices.len(),
        triangle_count: mesh.triangles.len(),
        splits,
        relaxations,
        bvh_projection_queries,
        preserved_feature_vertices: mesh.original_fixed_count,
        field_conformance,
    })
}

/// Upper bound for output buffers. Each split can at most double faces for a
/// chosen edge; we use the same intentionally generous x4-per-pass bound as the
/// isotropic remesher.
pub fn required_anisotropic_output_capacity(
    vertex_count: usize,
    triangle_count: usize,
    iterations: u32,
) -> (usize, usize) {
    let mut v = vertex_count.max(1);
    let mut f = triangle_count.max(1);
    for _ in 0..iterations.min(16) {
        f = f.saturating_mul(4);
        v = v.saturating_add(f);
    }
    (v, f)
}

fn validate_input(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    fixed_vertices: &[u32],
    crease_edges: &[FeatureEdge],
) -> Result<(), AnisotropicRemeshError> {
    for (i, v) in vertices.iter().enumerate() {
        if !v.x.is_finite() || !v.y.is_finite() || !v.z.is_finite() {
            return Err(AnisotropicRemeshError::NonFiniteCoordinate { index: i });
        }
    }
    for (t, tri) in triangles.iter().enumerate() {
        for &vi in tri {
            if vi as usize >= vertices.len() {
                return Err(AnisotropicRemeshError::IndexOutOfBounds {
                    triangle: t,
                    vertex: vi,
                });
            }
        }
        if tri[0] == tri[1] || tri[1] == tri[2] || tri[2] == tri[0] {
            return Err(AnisotropicRemeshError::DegenerateInputFace { triangle: t });
        }
    }
    for &v in fixed_vertices {
        if v as usize >= vertices.len() {
            return Err(AnisotropicRemeshError::FeatureIndexOutOfBounds { vertex: v });
        }
    }
    for e in crease_edges {
        if e.a as usize >= vertices.len() {
            return Err(AnisotropicRemeshError::FeatureIndexOutOfBounds { vertex: e.a });
        }
        if e.b as usize >= vertices.len() {
            return Err(AnisotropicRemeshError::FeatureIndexOutOfBounds { vertex: e.b });
        }
    }
    Ok(())
}

fn validate_manifold(vertex_count: u32, triangles: &[[u32; 3]]) -> bool {
    if triangles.is_empty() {
        return true;
    }
    let mut edges = vec![HalfEdge::default(); triangles.len() * 3];
    let mut slots = vec![EdgeSlot::default(); required_edge_slots(triangles.len())];
    build_triangle_half_edges(vertex_count, triangles, &mut edges, &mut slots).is_ok()
}

#[derive(Debug, Clone, Copy)]
struct EdgeCandidate {
    a: u32,
    b: u32,
    metric_len: f64,
}

struct AnisoMesh {
    vertices: Vec<Point3>,
    triangles: Vec<[u32; 3]>,
    fixed: Vec<bool>,
    feature_edges: Vec<FeatureEdge>,
    original_fixed_count: usize,
}

impl AnisoMesh {
    fn new(
        vertices: &[Point3],
        triangles: &[[u32; 3]],
        fixed_vertices: &[u32],
        crease_edges: &[FeatureEdge],
    ) -> Self {
        let mut fixed = vec![false; vertices.len()];
        for &v in fixed_vertices {
            fixed[v as usize] = true;
        }
        let mut feature_edges: Vec<FeatureEdge> = crease_edges
            .iter()
            .map(|e| FeatureEdge::new(e.a, e.b))
            .collect();
        feature_edges.sort_by_key(|e| (e.a, e.b));
        feature_edges.dedup_by_key(|e| (e.a, e.b));
        for e in &feature_edges {
            fixed[e.a as usize] = true;
            fixed[e.b as usize] = true;
        }
        let boundary = boundary_vertices(triangles, vertices.len());
        for (i, is_boundary) in boundary.into_iter().enumerate() {
            fixed[i] |= is_boundary;
        }
        let original_fixed_count = fixed.iter().filter(|&&f| f).count();
        Self {
            vertices: vertices.to_vec(),
            triangles: triangles.to_vec(),
            fixed,
            feature_edges,
            original_fixed_count,
        }
    }

    fn split_metric_long_edges(
        &mut self,
        field: &AnisotropyField,
        high: f64,
    ) -> Result<usize, AnisotropicRemeshError> {
        let mut candidates = self.edge_candidates(field)?;
        candidates.sort_by(|a, b| {
            b.metric_len
                .partial_cmp(&a.metric_len)
                .unwrap_or(core::cmp::Ordering::Equal)
                .then_with(|| a.a.cmp(&b.a))
                .then_with(|| a.b.cmp(&b.b))
        });

        let mut splits = 0usize;
        for c in candidates {
            if !self.edge_exists(c.a, c.b) {
                continue;
            }
            let len = self.metric_edge_len(field, c.a, c.b)?;
            if len <= high {
                continue;
            }
            let pa = self.vertices[c.a as usize];
            let pb = self.vertices[c.b as usize];
            let mid = midpoint(pa, pb);
            let mid_idx = self.vertices.len() as u32;
            self.vertices.push(mid);
            let is_feature = self.is_feature_edge(c.a, c.b);
            self.fixed.push(is_feature);
            if is_feature {
                self.feature_edges.push(FeatureEdge::new(c.a, mid_idx));
                self.feature_edges.push(FeatureEdge::new(mid_idx, c.b));
                self.feature_edges.sort_by_key(|e| (e.a, e.b));
                self.feature_edges.dedup_by_key(|e| (e.a, e.b));
            }
            let mut next = Vec::with_capacity(self.triangles.len() + 2);
            let mut used = false;
            for tri in &self.triangles {
                if contains_edge(*tri, c.a, c.b) {
                    let [t0, t1] = split_triangle_on_edge(*tri, c.a, c.b, mid_idx);
                    next.push(t0);
                    next.push(t1);
                    used = true;
                } else {
                    next.push(*tri);
                }
            }
            if used {
                self.triangles = next;
                splits += 1;
            } else {
                self.vertices.pop();
                self.fixed.pop();
            }
        }
        Ok(splits)
    }

    fn metric_relax(
        &mut self,
        field: &AnisotropyField,
        projector: &BvhSurfaceProjector,
        relaxation: f64,
    ) -> Result<(usize, usize), AnisotropicRemeshError> {
        if relaxation == 0.0 || self.vertices.is_empty() {
            return Ok((0, 0));
        }
        let neighbours = vertex_neighbours(&self.triangles, self.vertices.len());
        let mut next = self.vertices.clone();
        let mut moved = 0usize;
        let mut projected = 0usize;
        for (i, ns) in neighbours.iter().enumerate() {
            if self.fixed[i] || ns.is_empty() {
                continue;
            }
            let p = self.vertices[i];
            let mut acc = Point3::new(0.0, 0.0, 0.0);
            let mut weight_sum = 0.0f64;
            for &j in ns {
                let q = self.vertices[j as usize];
                let len = self.metric_edge_len_points(field, p, q)?;
                let w = 1.0 / len.max(1e-12);
                acc = add(acc, scale(q, w));
                weight_sum += w;
            }
            if weight_sum == 0.0 {
                continue;
            }
            let target = scale(acc, 1.0 / weight_sum);
            let blended = add(scale(p, 1.0 - relaxation), scale(target, relaxation));
            let snapped = projector.project(blended);
            projected += 1;
            if distance_sq(p, snapped) > 1e-24 {
                next[i] = snapped;
                moved += 1;
            }
        }
        self.vertices = next;
        Ok((moved, projected))
    }

    fn edge_candidates(
        &self,
        field: &AnisotropyField,
    ) -> Result<Vec<EdgeCandidate>, AnisotropicRemeshError> {
        let mut edges = Vec::with_capacity(self.triangles.len() * 3);
        for tri in &self.triangles {
            for (a, b) in tri_edges(*tri) {
                let (a, b) = canon_edge(a, b);
                edges.push((a, b));
            }
        }
        edges.sort_unstable();
        edges.dedup();
        let mut out = Vec::with_capacity(edges.len());
        for (a, b) in edges {
            out.push(EdgeCandidate {
                a,
                b,
                metric_len: self.metric_edge_len(field, a, b)?,
            });
        }
        Ok(out)
    }

    fn metric_edge_len(
        &self,
        field: &AnisotropyField,
        a: u32,
        b: u32,
    ) -> Result<f64, AnisotropicRemeshError> {
        self.metric_edge_len_points(field, self.vertices[a as usize], self.vertices[b as usize])
    }

    fn metric_edge_len_points(
        &self,
        field: &AnisotropyField,
        a: Point3,
        b: Point3,
    ) -> Result<f64, AnisotropicRemeshError> {
        let mid = midpoint(a, b);
        Ok(field.metric_at(mid)?.length_of(a, b))
    }

    fn edge_exists(&self, a: u32, b: u32) -> bool {
        self.triangles.iter().any(|&t| contains_edge(t, a, b))
    }

    fn is_feature_edge(&self, a: u32, b: u32) -> bool {
        let e = FeatureEdge::new(a, b);
        self.feature_edges
            .binary_search_by_key(&(e.a, e.b), |x| (x.a, x.b))
            .is_ok()
    }

    fn compact(&mut self) {
        let mut used = vec![false; self.vertices.len()];
        for tri in &self.triangles {
            used[tri[0] as usize] = true;
            used[tri[1] as usize] = true;
            used[tri[2] as usize] = true;
        }
        let mut remap = vec![u32::MAX; self.vertices.len()];
        let mut vertices = Vec::with_capacity(self.vertices.len());
        let mut fixed = Vec::with_capacity(self.fixed.len());
        for (i, &u) in used.iter().enumerate() {
            if u {
                remap[i] = vertices.len() as u32;
                vertices.push(self.vertices[i]);
                fixed.push(self.fixed[i]);
            }
        }
        for tri in &mut self.triangles {
            tri[0] = remap[tri[0] as usize];
            tri[1] = remap[tri[1] as usize];
            tri[2] = remap[tri[2] as usize];
        }
        self.vertices = vertices;
        self.fixed = fixed;
    }
}

struct BvhSurfaceProjector {
    tris: Vec<[Point3; 3]>,
    aabbs: Vec<Aabb>,
    nodes: Vec<BvhNode>,
    prim_indices: Vec<u32>,
    node_count: usize,
    root: usize,
}

impl BvhSurfaceProjector {
    fn new(vertices: &[Point3], triangles: &[[u32; 3]]) -> Result<Self, AnisotropicRemeshError> {
        let tris: Vec<[Point3; 3]> = triangles
            .iter()
            .map(|t| {
                [
                    vertices[t[0] as usize],
                    vertices[t[1] as usize],
                    vertices[t[2] as usize],
                ]
            })
            .collect();
        let aabbs: Vec<Aabb> = tris.iter().map(|&t| tri_aabb(t)).collect();
        if aabbs.is_empty() {
            return Ok(Self {
                tris,
                aabbs,
                nodes: Vec::new(),
                prim_indices: Vec::new(),
                node_count: 0,
                root: 0,
            });
        }
        let n = aabbs.len();
        let mut nodes = vec![BvhNode::default(); 2 * n];
        let mut prim_indices = vec![0u32; n];
        let mut morton_codes = vec![0u64; n];
        let mut sort_indices = vec![0u32; n];
        let (node_count, root) = build_bvh_recursive(
            &aabbs,
            &mut nodes,
            &mut prim_indices,
            &mut morton_codes,
            &mut sort_indices,
        )?;
        nodes.truncate(node_count);
        Ok(Self {
            tris,
            aabbs,
            nodes,
            prim_indices,
            node_count,
            root,
        })
    }

    fn project(&self, p: Point3) -> Point3 {
        if self.node_count == 0 {
            return p;
        }
        let mut best = p;
        let mut best_sq = f64::INFINITY;
        let mut best_idx = u32::MAX;
        let mut stack = [0u32; MAX_BVH_DEPTH * 2];
        let mut top = 0usize;
        stack[top] = self.root as u32;
        top += 1;
        while top > 0 {
            top -= 1;
            let node = self.nodes[stack[top] as usize];
            let bbox = Aabb::new(
                Point3::new(
                    node.bbox_min[0] as f64,
                    node.bbox_min[1] as f64,
                    node.bbox_min[2] as f64,
                ),
                Point3::new(
                    node.bbox_max[0] as f64,
                    node.bbox_max[1] as f64,
                    node.bbox_max[2] as f64,
                ),
            );
            if bbox.distance_sq_to_point(p) > best_sq {
                continue;
            }
            if node.node_type == 1 {
                let start = node.left_or_first as usize;
                let count = node.right_or_count as usize;
                for i in 0..count {
                    let prim = self.prim_indices[start + i];
                    if self.aabbs[prim as usize].distance_sq_to_point(p) > best_sq {
                        continue;
                    }
                    let tri = self.tris[prim as usize];
                    let q = closest_point_on_triangle(p, tri[0], tri[1], tri[2]);
                    let d = distance_sq(p, q);
                    if d < best_sq || (d == best_sq && prim < best_idx) {
                        best_sq = d;
                        best_idx = prim;
                        best = q;
                    }
                }
            } else if top + 2 <= stack.len() {
                stack[top] = node.right_or_count;
                top += 1;
                stack[top] = node.left_or_first;
                top += 1;
            }
        }
        best
    }
}

fn boundary_vertices(triangles: &[[u32; 3]], vertex_count: usize) -> Vec<bool> {
    let mut edges = Vec::with_capacity(triangles.len() * 3);
    for tri in triangles {
        for (a, b) in tri_edges(*tri) {
            edges.push(canon_edge(a, b));
        }
    }
    edges.sort_unstable();
    let mut out = vec![false; vertex_count];
    let mut i = 0usize;
    while i < edges.len() {
        let edge = edges[i];
        let mut count = 1usize;
        while i + count < edges.len() && edges[i + count] == edge {
            count += 1;
        }
        if count == 1 {
            out[edge.0 as usize] = true;
            out[edge.1 as usize] = true;
        }
        i += count;
    }
    out
}

fn vertex_neighbours(triangles: &[[u32; 3]], vertex_count: usize) -> Vec<Vec<u32>> {
    let mut out = vec![Vec::<u32>::new(); vertex_count];
    for tri in triangles {
        for (a, b) in tri_edges(*tri) {
            push_unique(&mut out[a as usize], b);
            push_unique(&mut out[b as usize], a);
        }
    }
    for ns in &mut out {
        ns.sort_unstable();
    }
    out
}

fn push_unique(xs: &mut Vec<u32>, x: u32) {
    if !xs.contains(&x) {
        xs.push(x);
    }
}

#[inline]
fn tri_edges(tri: [u32; 3]) -> [(u32, u32); 3] {
    [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])]
}

#[inline]
fn canon_edge(a: u32, b: u32) -> (u32, u32) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

fn contains_edge(tri: [u32; 3], a: u32, b: u32) -> bool {
    tri_edges(tri)
        .iter()
        .any(|&(x, y)| (x == a && y == b) || (x == b && y == a))
}

fn split_triangle_on_edge(tri: [u32; 3], a: u32, b: u32, mid: u32) -> [[u32; 3]; 2] {
    for k in 0..3 {
        let x = tri[k];
        let y = tri[(k + 1) % 3];
        let z = tri[(k + 2) % 3];
        if (x == a && y == b) || (x == b && y == a) {
            return [[x, mid, z], [mid, y, z]];
        }
    }
    [tri, tri]
}

fn tri_aabb(t: [Point3; 3]) -> Aabb {
    let min = Point3::new(
        t[0].x.min(t[1].x).min(t[2].x),
        t[0].y.min(t[1].y).min(t[2].y),
        t[0].z.min(t[1].z).min(t[2].z),
    );
    let max = Point3::new(
        t[0].x.max(t[1].x).max(t[2].x),
        t[0].y.max(t[1].y).max(t[2].y),
        t[0].z.max(t[1].z).max(t[2].z),
    );
    Aabb::new(min, max)
}

fn closest_point_on_triangle(p: Point3, a: Point3, b: Point3, c: Point3) -> Point3 {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = sub(p, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return add(a, scale(ab, d1 / (d1 - d3)));
    }
    let cp = sub(p, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return add(a, scale(ac, d2 / (d2 - d6)));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return add(b, scale(sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6))));
    }
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    add(a, add(scale(ab, v), scale(ac, w)))
}

#[inline]
fn midpoint(a: Point3, b: Point3) -> Point3 {
    Point3::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5, (a.z + b.z) * 0.5)
}

#[inline]
fn sub(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

#[inline]
fn add(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

#[inline]
fn scale(a: Point3, s: f64) -> Point3 {
    Point3::new(a.x * s, a.y * s, a.z * s)
}

#[inline]
fn dot(a: Point3, b: Point3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

#[inline]
fn distance_sq(a: Point3, b: Point3) -> f64 {
    let d = sub(a, b);
    dot(d, d)
}

#[cfg(test)]
mod tests {
    use super::super::mesh_quality::{MetricTensor, TriQuality};
    use super::*;

    fn square() -> (Vec<Point3>, Vec<[u32; 3]>) {
        (
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2], [0, 2, 3]],
        )
    }

    #[test]
    fn invalid_options_rejected() {
        let (v, t) = square();
        let f = AnisotropyField::Uniform {
            metric: MetricTensor::IDENTITY,
        };
        let mut ov = vec![Point3::new(0.0, 0.0, 0.0); 16];
        let mut ot = vec![[0u32; 3]; 16];
        let err = anisotropic_remesh(
            &v,
            &t,
            &f,
            &[],
            &[],
            AnisotropicRemeshOptions {
                iterations: 1,
                max_metric_edge: 0.0,
                relaxation: 0.5,
            },
            &mut ov,
            &mut ot,
        )
        .unwrap_err();
        assert_eq!(err, AnisotropicRemeshError::InvalidOptions);
    }

    #[test]
    fn metric_long_edges_are_split_until_within_band() {
        let (v, t) = square();
        let f = AnisotropyField::Uniform {
            metric: MetricTensor::isotropic(0.5),
        };
        let (vc, tc) = required_anisotropic_output_capacity(v.len(), t.len(), 3);
        let mut ov = vec![Point3::new(0.0, 0.0, 0.0); vc];
        let mut ot = vec![[0u32; 3]; tc];
        let report = anisotropic_remesh(
            &v,
            &t,
            &f,
            &[],
            &[],
            AnisotropicRemeshOptions {
                iterations: 3,
                max_metric_edge: 4.0 / 3.0,
                relaxation: 0.0,
            },
            &mut ov,
            &mut ot,
        )
        .unwrap();
        assert!(report.splits > 0);
        assert!(report.field_conformance.max_ratio <= 4.0 / 3.0 + 1e-12);
    }

    #[test]
    fn fixed_corners_and_crease_endpoints_are_preserved() {
        let (v, t) = square();
        let f = AnisotropyField::Uniform {
            metric: MetricTensor::isotropic(0.75),
        };
        let (vc, tc) = required_anisotropic_output_capacity(v.len(), t.len(), 2);
        let mut ov = vec![Point3::new(0.0, 0.0, 0.0); vc];
        let mut ot = vec![[0u32; 3]; tc];
        let report = anisotropic_remesh(
            &v,
            &t,
            &f,
            &[0],
            &[FeatureEdge::new(1, 2)],
            AnisotropicRemeshOptions {
                iterations: 2,
                max_metric_edge: 4.0 / 3.0,
                relaxation: 0.8,
            },
            &mut ov,
            &mut ot,
        )
        .unwrap();
        let out = &ov[..report.vertex_count];
        for original in [v[0], v[1], v[2]] {
            assert!(
                out.iter().any(|&p| distance_sq(p, original) < 1e-24),
                "missing preserved feature vertex {original:?}"
            );
        }
    }

    #[test]
    fn bvh_projection_keeps_relaxed_vertices_on_source_plane() {
        let v = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.45, 0.45, 0.0),
        ];
        let t = vec![[0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]];
        let f = AnisotropyField::Uniform {
            metric: MetricTensor::IDENTITY,
        };
        let (vc, tc) = required_anisotropic_output_capacity(v.len(), t.len(), 1);
        let mut ov = vec![Point3::new(0.0, 0.0, 0.0); vc];
        let mut ot = vec![[0u32; 3]; tc];
        let report = anisotropic_remesh(
            &v,
            &t,
            &f,
            &[],
            &[],
            AnisotropicRemeshOptions {
                iterations: 1,
                max_metric_edge: 10.0,
                relaxation: 0.75,
            },
            &mut ov,
            &mut ot,
        )
        .unwrap();
        assert!(report.bvh_projection_queries > 0);
        for p in &ov[..report.vertex_count] {
            assert!(p.z.abs() < 1e-12);
        }
    }

    #[test]
    fn deterministic_output() {
        let (v, t) = square();
        let f = AnisotropyField::Uniform {
            metric: MetricTensor::isotropic(0.5),
        };
        let (vc, tc) = required_anisotropic_output_capacity(v.len(), t.len(), 3);
        let mut av = vec![Point3::new(0.0, 0.0, 0.0); vc];
        let mut at = vec![[0u32; 3]; tc];
        let mut bv = vec![Point3::new(0.0, 0.0, 0.0); vc];
        let mut bt = vec![[0u32; 3]; tc];
        let opts = AnisotropicRemeshOptions {
            iterations: 3,
            max_metric_edge: 4.0 / 3.0,
            relaxation: 0.3,
        };
        let ra = anisotropic_remesh(&v, &t, &f, &[], &[], opts, &mut av, &mut at).unwrap();
        let rb = anisotropic_remesh(&v, &t, &f, &[], &[], opts, &mut bv, &mut bt).unwrap();
        assert_eq!(ra, rb);
        assert_eq!(&av[..ra.vertex_count], &bv[..rb.vertex_count]);
        assert_eq!(&at[..ra.triangle_count], &bt[..rb.triangle_count]);
    }

    #[test]
    fn output_buffer_bounds_fail_closed() {
        let (v, t) = square();
        let f = AnisotropyField::Uniform {
            metric: MetricTensor::IDENTITY,
        };
        let mut ov = vec![Point3::new(0.0, 0.0, 0.0); 1];
        let mut ot = vec![[0u32; 3]; 1];
        let err = anisotropic_remesh(
            &v,
            &t,
            &f,
            &[],
            &[],
            AnisotropicRemeshOptions::default(),
            &mut ov,
            &mut ot,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            AnisotropicRemeshError::VertexOutputTooSmall { .. }
                | AnisotropicRemeshError::TriangleOutputTooSmall { .. }
        ));
    }

    #[test]
    fn indefinite_metric_fails_closed() {
        let (v, t) = square();
        let f = AnisotropyField::Uniform {
            metric: MetricTensor {
                m00: -1.0,
                m10: 0.0,
                m11: 1.0,
                m20: 0.0,
                m21: 0.0,
                m22: 1.0,
            },
        };
        let mut ov = vec![Point3::new(0.0, 0.0, 0.0); 32];
        let mut ot = vec![[0u32; 3]; 32];
        let err = anisotropic_remesh(
            &v,
            &t,
            &f,
            &[],
            &[],
            AnisotropicRemeshOptions {
                iterations: 1,
                max_metric_edge: 10.0,
                relaxation: 0.0,
            },
            &mut ov,
            &mut ot,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            AnisotropicRemeshError::Metric(MeshQualityError::IndefiniteMetric { .. })
        ));
    }

    #[test]
    fn triangle_quality_remains_valid_after_refinement() {
        let (v, t) = square();
        let f = AnisotropyField::Uniform {
            metric: MetricTensor::isotropic(0.5),
        };
        let (vc, tc) = required_anisotropic_output_capacity(v.len(), t.len(), 2);
        let mut ov = vec![Point3::new(0.0, 0.0, 0.0); vc];
        let mut ot = vec![[0u32; 3]; tc];
        let report = anisotropic_remesh(
            &v,
            &t,
            &f,
            &[],
            &[],
            AnisotropicRemeshOptions {
                iterations: 2,
                max_metric_edge: 4.0 / 3.0,
                relaxation: 0.0,
            },
            &mut ov,
            &mut ot,
        )
        .unwrap();
        for tri in &ot[..report.triangle_count] {
            let q: TriQuality =
                super::super::mesh_quality::tri_quality(&ov[..report.vertex_count], tri).unwrap();
            assert!(q.valid);
            assert!(q.area > 0.0);
        }
    }
}
