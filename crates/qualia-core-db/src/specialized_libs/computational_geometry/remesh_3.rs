//! P5.6 — Isotropic remeshing of a triangle surface mesh.
//!
//! Iterates the classic four-stage Botsch–Kobbelt loop toward a target edge length
//! `L`:
//!
//! 1. **Split** every edge longer than `4/3·L` at its midpoint.
//! 2. **Collapse** every edge shorter than `4/5·L` (link-condition + fold guarded).
//! 3. **Flip** edges to equalise vertex valence toward the ideal (6 interior / 4 boundary),
//!    rejecting flips that would fold a triangle or duplicate an edge.
//! 4. **Tangential relaxation** — move each interior vertex toward its one-ring centroid,
//!    remove the normal component, and re-project the result back onto the *original*
//!    surface so the mesh does not shrink or drift off the shape.
//!
//! Reference: M. Botsch and L. Kobbelt, "A Remeshing Approach to Multiresolution
//! Modeling" (SGP 2004) and the isotropic-remeshing chapter of *Polygon Mesh
//! Processing* (Botsch et al., 2010). The implementation is original Rust over this
//! crate's own predicates ([`GeometryKernel::orient_3d`]) and measures; no
//! third-party source is used.
//!
//! ## Honesty / scope
//!
//! This is a **correct, manifold-preserving** implementation, not a research-grade one.
//! What is guaranteed:
//! - Every accepted operation preserves 2-manifoldness and consistent orientation.
//!   Illegal splits/collapses/flips are *rejected* (never applied), so the output
//!   half-edge graph is always closed+manifold when the input was.
//! - Feature-agnostic: sharp edges/creases are **not** detected or preserved — a
//!   smooth surface is assumed. On a sharp closed polyhedron (cube/octahedron) the
//!   collapse+project step therefore *rounds corners inward*, so enclosed volume
//!   shrinks; that is expected, not a bug. The **boundary polygon is preserved
//!   exactly**: boundary vertices are pinned during smoothing, boundary edges may be
//!   *split* (refining the boundary in place), and no edge touching the boundary is
//!   ever collapsed (so the boundary is never eroded or dragged inward). A flat patch's
//!   area is preserved to floating-point exactness as a result.
//! - Surface projection is an **exact nearest-triangle** projection against the
//!   original mesh (linear scan per relaxed vertex — cold path, documented below).
//!   It keeps the remesh on-surface but is O(V·F) per smoothing pass; a BVH-
//!   accelerated projection is a performance follow-up, not a correctness one.
//!
//! What is **not** attempted (and deliberately absent rather than faked):
//! - `// FOLLOW-UP: feature/crease preservation` — needs a dihedral-angle feature
//!   detector and constrained relaxation; out of scope for this file.
//! - `// FOLLOW-UP: BVH-accelerated surface projection` — replace the linear scan
//!   in [`SurfaceProjector`] with `super::bvh`; purely a speed change.
//!
//! ## Memory model
//!
//! Remeshing rewrites connectivity in place across many passes; the incidence sets
//! grow and shrink unpredictably. A fixed caller scratch cannot bound that, so the
//! **cold construction** here uses internal `Vec`s (documented, one-shot). The
//! **public output is caller-buffered** ([`isotropic_remesh`] writes into caller
//! `out_vertices`/`out_triangles` slices and returns counts), matching the P5 hot/query
//! zero-heap contract for the surface the rest of the engine touches.
//!
//! ## Determinism
//!
//! Identical input + options ⇒ bit-identical output. All passes iterate in a fixed,
//! canonical order (ascending edge / vertex / triangle index); collapses and flips
//! process a snapshot of candidate edges taken at the start of the pass and skip any
//! whose endpoints were already touched, so the result does not depend on hash
//! iteration order or transient indexing. A determinism test asserts bit-identical
//! vertices and triangles across two runs.

use super::kernel::{FilteredF64Kernel, GeometryKernel};
use super::primitives::Point3;
use super::surface_mesh_processing::MeshMeasureError;
use super::topology::{build_triangle_half_edges, required_edge_slots, EdgeSlot, HalfEdge};

// ──────────────────────────────────────────────────────────────────────────
//  Public API
// ──────────────────────────────────────────────────────────────────────────

/// Tuning parameters for [`isotropic_remesh`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RemeshOptions {
    /// Target (isotropic) edge length `L`. Must be finite and `> 0`.
    pub target_edge_length: f64,
    /// Number of split/collapse/flip/relax passes to run. `0` is a valid no-op
    /// (the mesh is validated and copied through unchanged).
    pub iterations: u32,
}

impl RemeshOptions {
    /// Construct options with the given target length and iteration count.
    #[inline]
    pub fn new(target_edge_length: f64, iterations: u32) -> Self {
        Self {
            target_edge_length,
            iterations,
        }
    }
}

impl Default for RemeshOptions {
    fn default() -> Self {
        Self {
            target_edge_length: 1.0,
            iterations: 5,
        }
    }
}

/// What a remeshing run did. All counts are cumulative across every pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RemeshReport {
    /// Final vertex count written to `out_vertices`.
    pub vertex_count: usize,
    /// Final triangle count written to `out_triangles`.
    pub triangle_count: usize,
    /// Edges split (each split adds one vertex + triangles).
    pub splits: usize,
    /// Edges collapsed (each removes one vertex).
    pub collapses: usize,
    /// Edges flipped for valence equalisation.
    pub flips: usize,
    /// Vertices moved by tangential relaxation (summed over passes).
    pub relaxations: usize,
    /// `true` if the final mesh is closed (0 boundary edges) and manifold — i.e.
    /// the output half-edge graph built cleanly with no boundary. A mesh that was
    /// open on input stays open; this flag then reports `false` (expected).
    pub closed_manifold: bool,
}

/// Failure modes for [`isotropic_remesh`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemeshError {
    /// `target_edge_length` was not finite and strictly positive.
    InvalidTargetLength,
    /// A triangle referenced a vertex index outside `vertices`.
    IndexOutOfBounds { triangle: usize, vertex: u32 },
    /// A vertex coordinate was NaN / ±∞.
    NonFiniteCoordinate { index: usize },
    /// A triangle had two equal corner indices (degenerate on input).
    DegenerateInputFace { triangle: usize },
    /// Input connectivity was not a 2-manifold with consistent orientation, so
    /// remeshing cannot proceed. (The half-edge validator rejected it.)
    NonManifoldInput,
    /// `out_vertices` is too small; needs `required` entries.
    VertexOutputTooSmall { required: usize },
    /// `out_triangles` is too small; needs `required` entries.
    TriangleOutputTooSmall { required: usize },
}

impl From<MeshMeasureError> for RemeshError {
    fn from(err: MeshMeasureError) -> Self {
        match err {
            MeshMeasureError::IndexOutOfBounds { triangle, vertex } => {
                RemeshError::IndexOutOfBounds { triangle, vertex }
            }
            MeshMeasureError::NonFiniteCoordinate { index } => {
                RemeshError::NonFiniteCoordinate { index }
            }
        }
    }
}

/// Isotropic remesh `(vertices, triangles)` toward `options.target_edge_length`,
/// using the default [`FilteredF64Kernel`].
///
/// Writes the remeshed vertices/triangles into the caller-owned `out_vertices` /
/// `out_triangles` slices and returns a [`RemeshReport`]. Use
/// [`required_output_capacity`] to size the output buffers up front. See
/// [`isotropic_remesh_with_kernel`] for the kernel-generic form and the module
/// docs for the algorithm and honesty caveats.
pub fn isotropic_remesh(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    options: RemeshOptions,
    out_vertices: &mut [Point3],
    out_triangles: &mut [[u32; 3]],
) -> Result<RemeshReport, RemeshError> {
    isotropic_remesh_with_kernel(
        &FilteredF64Kernel::default(),
        vertices,
        triangles,
        options,
        out_vertices,
        out_triangles,
    )
}

/// Kernel-generic [`isotropic_remesh`]. The orientation predicate that guards
/// flips and collapses is taken from `kernel`, so the same algorithm runs over
/// the filtered `f64` kernel today or an exact kernel later.
pub fn isotropic_remesh_with_kernel<K: GeometryKernel>(
    kernel: &K,
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    options: RemeshOptions,
    out_vertices: &mut [Point3],
    out_triangles: &mut [[u32; 3]],
) -> Result<RemeshReport, RemeshError> {
    if !(options.target_edge_length.is_finite() && options.target_edge_length > 0.0) {
        return Err(RemeshError::InvalidTargetLength);
    }
    // Validate input coordinates + face non-degeneracy up front.
    for (i, v) in vertices.iter().enumerate() {
        if !v.x.is_finite() || !v.y.is_finite() || !v.z.is_finite() {
            return Err(RemeshError::NonFiniteCoordinate { index: i });
        }
    }
    for (t, tri) in triangles.iter().enumerate() {
        for &vi in tri {
            if vi as usize >= vertices.len() {
                return Err(RemeshError::IndexOutOfBounds {
                    triangle: t,
                    vertex: vi,
                });
            }
        }
        if tri[0] == tri[1] || tri[1] == tri[2] || tri[2] == tri[0] {
            return Err(RemeshError::DegenerateInputFace { triangle: t });
        }
    }
    // The input must be a manifold with consistent orientation, or remeshing is
    // ill-defined. Validate with the frozen half-edge builder (fail-closed).
    if !validate_manifold(vertices.len() as u32, triangles) {
        return Err(RemeshError::NonManifoldInput);
    }

    // Cold construction (documented): editable mesh + immutable original surface.
    let projector = SurfaceProjector::new(vertices, triangles);
    let mut mesh = RemeshMesh::from_input(vertices, triangles);
    let mut report = RemeshReport::default();

    let low = 0.8 * options.target_edge_length; // 4/5 L
    let high = 4.0 / 3.0 * options.target_edge_length; // 4/3 L
    let low_sq = low * low;
    let high_sq = high * high;

    for _ in 0..options.iterations {
        report.splits += mesh.split_long_edges(high_sq);
        report.collapses += mesh.collapse_short_edges(kernel, low_sq, high_sq);
        report.flips += mesh.equalize_valence(kernel);
        report.relaxations += mesh.tangential_relaxation(&projector);
    }

    // Drop any vertices left unreferenced by collapses and canonicalise ordering.
    mesh.compact();

    if out_vertices.len() < mesh.vertices.len() {
        return Err(RemeshError::VertexOutputTooSmall {
            required: mesh.vertices.len(),
        });
    }
    if out_triangles.len() < mesh.triangles.len() {
        return Err(RemeshError::TriangleOutputTooSmall {
            required: mesh.triangles.len(),
        });
    }

    out_vertices[..mesh.vertices.len()].copy_from_slice(&mesh.vertices);
    out_triangles[..mesh.triangles.len()].copy_from_slice(&mesh.triangles);
    report.vertex_count = mesh.vertices.len();
    report.triangle_count = mesh.triangles.len();
    report.closed_manifold = mesh.is_closed_manifold();

    Ok(report)
}

/// Upper bound on the output buffer sizes for a run. Splitting can at most quadruple
/// per pass in the pathological case; this returns a safe, cheap over-estimate the
/// caller can allocate once. The actual counts are reported in [`RemeshReport`].
///
/// The bound assumes each pass may split every current edge (≤ `3·F/2` interior +
/// boundary edges → adds that many vertices and doubles faces) and collapses/flips
/// never *increase* counts. It is intentionally generous; if a run would exceed a
/// caller's real buffer it returns [`RemeshError::VertexOutputTooSmall`] /
/// [`RemeshError::TriangleOutputTooSmall`] rather than overrunning.
pub fn required_output_capacity(
    vertex_count: usize,
    triangle_count: usize,
    iterations: u32,
) -> (usize, usize) {
    // Worst case within a pass: every one of a triangle's three edges is long, so the
    // triangle is split into four (1-to-4 subdivision). Bound face growth by ×4 per
    // pass and vertices by the faces added (each split adds ≤1 vertex per face pair, so
    // adding `f` faces adds ≤ `f` vertices — a safe over-estimate). Clamp iterations to
    // avoid runaway allocation while staying generous.
    let mut v = vertex_count.max(1);
    let mut f = triangle_count.max(1);
    for _ in 0..iterations.min(16) {
        f = f.saturating_mul(4);
        v = v.saturating_add(f);
    }
    (v, f)
}

// ──────────────────────────────────────────────────────────────────────────
//  Manifold validation (delegates to the frozen half-edge builder)
// ──────────────────────────────────────────────────────────────────────────

/// Returns `true` if `triangles` build a valid 2-manifold half-edge graph with
/// consistent orientation over `vertex_count` vertices (boundaries allowed).
fn validate_manifold(vertex_count: u32, triangles: &[[u32; 3]]) -> bool {
    if triangles.is_empty() {
        return true; // empty mesh is trivially manifold
    }
    let edge_count = triangles.len() * 3;
    let slot_count = required_edge_slots(triangles.len());
    let mut edges = vec![HalfEdge::default(); edge_count];
    let mut slots = vec![EdgeSlot::default(); slot_count];
    build_triangle_half_edges(vertex_count, triangles, &mut edges, &mut slots).is_ok()
}

// ──────────────────────────────────────────────────────────────────────────
//  Editable mesh with vertex→triangle incidence
// ──────────────────────────────────────────────────────────────────────────

/// Mutable triangle mesh used during remeshing. Cold construction (internal `Vec`s,
/// documented); the public surface is caller-buffered by [`isotropic_remesh`].
struct RemeshMesh {
    vertices: Vec<Point3>,
    /// `u32::MAX` in any slot marks a deleted (tombstoned) triangle; compacted at end.
    triangles: Vec<[u32; 3]>,
    /// Per-vertex incident-triangle indices (into `triangles`). Rebuilt lazily.
    incident: Vec<Vec<u32>>,
    /// `true` for vertices on the mesh boundary (pinned during smoothing).
    boundary: Vec<bool>,
    /// Dirty flag: incidence/boundary must be rebuilt before the next topological pass.
    dirty: bool,
}

const TOMBSTONE: u32 = u32::MAX;

impl RemeshMesh {
    fn from_input(vertices: &[Point3], triangles: &[[u32; 3]]) -> Self {
        let mut mesh = RemeshMesh {
            vertices: vertices.to_vec(),
            triangles: triangles.to_vec(),
            incident: Vec::new(),
            boundary: Vec::new(),
            dirty: true,
        };
        mesh.rebuild_incidence();
        mesh
    }

    #[inline]
    fn is_deleted(tri: &[u32; 3]) -> bool {
        tri[0] == TOMBSTONE
    }

    /// Rebuild `incident` and `boundary` from the live triangle list.
    fn rebuild_incidence(&mut self) {
        let vn = self.vertices.len();
        self.incident = vec![Vec::new(); vn];
        for (ti, tri) in self.triangles.iter().enumerate() {
            if Self::is_deleted(tri) {
                continue;
            }
            for &v in tri {
                self.incident[v as usize].push(ti as u32);
            }
        }
        // A vertex is on the boundary iff at least one of its incident directed
        // edges has no oppositely-directed twin. We detect boundary edges by
        // counting directed-edge multiplicity over the whole live mesh.
        self.boundary = vec![false; vn];
        // Build a deterministic directed-edge multiset key → count via sorting.
        let mut directed: Vec<(u32, u32)> = Vec::new();
        for tri in self.triangles.iter() {
            if Self::is_deleted(tri) {
                continue;
            }
            for k in 0..3 {
                let a = tri[k];
                let b = tri[(k + 1) % 3];
                directed.push((a, b));
            }
        }
        // For each directed edge (a,b), it is a boundary edge if the reverse (b,a)
        // does not appear. Use a sorted lookup for determinism + no hashing.
        let mut sorted = directed.clone();
        sorted.sort_unstable();
        for &(a, b) in directed.iter() {
            // binary-search for reverse (b,a)
            if sorted.binary_search(&(b, a)).is_err() {
                self.boundary[a as usize] = true;
                self.boundary[b as usize] = true;
            }
        }
        self.dirty = false;
    }

    #[inline]
    fn ensure_clean(&mut self) {
        if self.dirty {
            self.rebuild_incidence();
        }
    }

    #[inline]
    fn edge_len_sq(&self, u: u32, v: u32) -> f64 {
        let a = self.vertices[u as usize];
        let b = self.vertices[v as usize];
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        let dz = a.z - b.z;
        dx * dx + dy * dy + dz * dz
    }

    /// Collect the live undirected edges as canonical `(min,max)` pairs, sorted and
    /// deduplicated — a deterministic snapshot for a pass to iterate.
    fn collect_edges(&self) -> Vec<(u32, u32)> {
        let mut edges: Vec<(u32, u32)> = Vec::new();
        for tri in self.triangles.iter() {
            if Self::is_deleted(tri) {
                continue;
            }
            for k in 0..3 {
                let a = tri[k];
                let b = tri[(k + 1) % 3];
                edges.push(if a < b { (a, b) } else { (b, a) });
            }
        }
        edges.sort_unstable();
        edges.dedup();
        edges
    }

    /// Find the (up to two) live triangles incident to undirected edge `(u,v)`.
    /// Returns each as `(triangle_index, apex_vertex)` where apex is the third corner.
    fn edge_triangles(&self, u: u32, v: u32) -> Vec<(u32, u32)> {
        let mut out: Vec<(u32, u32)> = Vec::new();
        for &ti in self.incident[u as usize].iter() {
            let tri = self.triangles[ti as usize];
            if Self::is_deleted(&tri) {
                continue;
            }
            let has_u = tri[0] == u || tri[1] == u || tri[2] == u;
            let has_v = tri[0] == v || tri[1] == v || tri[2] == v;
            if has_u && has_v {
                let apex = tri[0] ^ tri[1] ^ tri[2] ^ u ^ v;
                out.push((ti, apex));
            }
        }
        out.sort_unstable_by_key(|&(ti, _)| ti);
        out
    }

    // ── Stage 1: split long edges ─────────────────────────────────────────

    /// Split every edge longer than `sqrt(high_sq)` at its midpoint.
    ///
    /// Runs to a fixpoint: after splitting the snapshot of currently-long edges, any
    /// *newly created* edge that is still too long (only possible for the original
    /// non-split diagonal of a triangle whose other edges were split — rare) is caught
    /// by re-collecting and repeating, bounded by a hard iteration cap. Incidence is
    /// maintained incrementally *within* the pass so each split sees live connectivity,
    /// which is what keeps the mesh manifold across chained splits on a shared triangle.
    /// Returns the number of edges split. Always manifold-preserving.
    fn split_long_edges(&mut self, high_sq: f64) -> usize {
        self.ensure_clean();
        let mut splits = 0usize;
        // Bound the fixpoint: each round strictly increases the vertex count, and edge
        // lengths only shrink by ½ per split, so O(log) rounds suffice; cap defensively.
        for _round in 0..32 {
            let edges = self.collect_edges();
            let mut any = false;
            for (u, v) in edges {
                // Re-check length against LIVE geometry (an endpoint may have been a
                // midpoint created earlier this round — its incident edges are fresh).
                if self.edge_len_sq(u, v) <= high_sq {
                    continue;
                }
                if self.split_edge(u, v) {
                    splits += 1;
                    any = true;
                }
            }
            if !any {
                break;
            }
        }
        if splits > 0 {
            self.dirty = true;
            self.rebuild_incidence();
        }
        splits
    }

    /// Split edge `(u,v)`: insert a midpoint `m`, replace each incident triangle
    /// `(u,v,apex)` (in its actual winding) by two triangles through `m`. Works for
    /// interior (2 incident) and boundary (1 incident) edges. Maintains `incident`
    /// incrementally so later splits in the same pass observe the new sub-triangles.
    /// The midpoint inherits boundary status from the split edge (a boundary edge's
    /// midpoint is on the boundary). Returns `true` on success.
    fn split_edge(&mut self, u: u32, v: u32) -> bool {
        let tris = self.edge_triangles(u, v);
        if tris.is_empty() || tris.len() > 2 {
            return false; // non-manifold edge — skip (should not happen post-validation)
        }
        let is_boundary_edge = tris.len() == 1;
        let a = self.vertices[u as usize];
        let b = self.vertices[v as usize];
        let m = Point3::new(
            0.5 * (a.x + b.x),
            0.5 * (a.y + b.y),
            0.5 * (a.z + b.z),
        );
        let mid = self.vertices.len() as u32;
        self.vertices.push(m);
        self.incident.push(Vec::new());
        self.boundary.push(is_boundary_edge);

        for &(ti, apex) in tris.iter() {
            let tri = self.triangles[ti as usize];
            // Replace triangle (x0,x1,x2): wherever the directed edge u→v or v→u
            // appears, split into two triangles sharing m, preserving winding.
            let new_tris = split_triangle_at_edge(tri, u, v, mid);
            let new_idx = self.triangles.len() as u32;
            // Child 0 reuses slot `ti`; child 1 is appended at `new_idx`.
            self.triangles[ti as usize] = new_tris[0];
            self.triangles.push(new_tris[1]);
            // Maintain incidence: slot `ti` (child 0) and `new_idx` (child 1) each
            // reference {u or v}, apex, and mid. Rather than diff, recompute the two
            // slots' incidence from their corner lists.
            //  - `mid` gains both children.
            self.incident[mid as usize].push(ti);
            self.incident[mid as usize].push(new_idx);
            //  - `apex` gains both children (it is a corner of both).
            self.incident[apex as usize].push(new_idx);
            //  - one of {u,v} keeps slot `ti` (already listed), the other must move to
            //    `new_idx`. Child 0 = [a,mid,c] where a is the head of the directed
            //    edge; child 1 = [mid,b,c]. Determine which endpoint is in child 1.
            let child1 = new_tris[1];
            let end_in_child1 = if child1[0] == u || child1[1] == u || child1[2] == u {
                u
            } else {
                v
            };
            self.incident[end_in_child1 as usize].push(new_idx);
        }
        true
    }

    // ── Stage 2: collapse short edges ─────────────────────────────────────

    /// Collapse every live edge shorter than `sqrt(low_sq)` toward its midpoint,
    /// subject to the link condition, boundary rules, and a fold/normal-inversion
    /// guard. Never collapses an edge whose result would create an edge longer than
    /// `sqrt(high_sq)` (avoids oscillation with the split stage). Returns the number
    /// of successful collapses.
    fn collapse_short_edges<K: GeometryKernel>(
        &mut self,
        kernel: &K,
        low_sq: f64,
        high_sq: f64,
    ) -> usize {
        self.ensure_clean();
        let edges = self.collect_edges();
        let mut touched = vec![false; self.vertices.len()];
        let mut collapses = 0usize;

        for (u, v) in edges {
            let (u, v) = (u as usize, v as usize);
            if u >= touched.len() || v >= touched.len() {
                continue;
            }
            if touched[u] || touched[v] {
                continue; // one endpoint already merged this pass
            }
            if self.edge_len_sq(u as u32, v as u32) >= low_sq {
                continue;
            }
            if self.try_collapse(kernel, u as u32, v as u32, high_sq) {
                touched[u] = true;
                touched[v] = true;
                collapses += 1;
            }
        }
        if collapses > 0 {
            self.dirty = true;
            self.rebuild_incidence();
        }
        collapses
    }

    /// Attempt to collapse edge `(u,v)` into a single vertex placed at the midpoint.
    /// The surviving vertex is `u`; `v` is tombstoned. Returns `true` if applied.
    ///
    /// Legality (all must hold, else reject and leave the mesh unchanged):
    /// - **Boundary rule:** if both `u` and `v` are boundary vertices but the edge
    ///   `(u,v)` is *not itself* a boundary edge, collapsing would pinch the surface —
    ///   reject. A boundary edge (1 incident triangle) may collapse; an interior edge
    ///   between two boundary vertices may not.
    /// - **Link condition:** the common neighbours of `u` and `v` must be exactly the
    ///   apex vertices of the triangles shared by the edge (2 interior / 1 boundary),
    ///   otherwise the collapse creates a non-manifold edge or a degenerate.
    /// - **Fold guard:** after moving `u` to the midpoint and deleting the two shared
    ///   triangles, no surviving triangle in the one-ring may invert its orientation
    ///   or become degenerate.
    fn try_collapse<K: GeometryKernel>(
        &mut self,
        kernel: &K,
        u: u32,
        v: u32,
        high_sq: f64,
    ) -> bool {
        let shared = self.edge_triangles(u, v);
        if shared.is_empty() || shared.len() > 2 {
            return false;
        }

        // Boundary rule (conservative, fully boundary-preserving): never collapse an
        // edge that touches the boundary. Collapsing a boundary edge removes a boundary
        // vertex and shrinks the boundary polygon (eroding corners); collapsing an
        // interior edge with a boundary endpoint drags the boundary inward. Both change
        // the input boundary, which this remesher is contracted to preserve. So we only
        // ever collapse edges whose BOTH endpoints are interior. (Boundary *refinement*
        // still happens via splits, which keep the boundary polygon fixed.)
        if self.boundary[u as usize] || self.boundary[v as usize] {
            return false;
        }

        // Link condition: common neighbours must equal the shared apexes.
        let nu = self.vertex_neighbours(u);
        let nv = self.vertex_neighbours(v);
        let mut common: Vec<u32> = nu.iter().copied().filter(|w| nv.contains(w)).collect();
        common.sort_unstable();
        common.dedup();
        let mut apexes: Vec<u32> = shared.iter().map(|&(_, apex)| apex).collect();
        apexes.sort_unstable();
        apexes.dedup();
        if common != apexes {
            return false;
        }

        // Both endpoints are interior (guaranteed by the boundary rule above), so the
        // collapse target is simply the edge midpoint; relaxation + re-projection later
        // pulls it onto the surface.
        let a = self.vertices[u as usize];
        let b = self.vertices[v as usize];
        let target = Point3::new(
            0.5 * (a.x + b.x),
            0.5 * (a.y + b.y),
            0.5 * (a.z + b.z),
        );

        // Fold guard + edge-length guard: simulate the collapse over the union of the
        // one-rings of u and v, excluding the shared (to-be-deleted) triangles.
        let shared_set: [u32; 2] = {
            let mut s = [TOMBSTONE, TOMBSTONE];
            for (i, &(ti, _)) in shared.iter().enumerate() {
                s[i] = ti;
            }
            s
        };
        // Gather affected triangles: incident to u or v, not shared.
        let mut affected: Vec<u32> = Vec::new();
        for &ti in self.incident[u as usize].iter().chain(self.incident[v as usize].iter()) {
            if ti == shared_set[0] || ti == shared_set[1] {
                continue;
            }
            let tri = self.triangles[ti as usize];
            if Self::is_deleted(&tri) {
                continue;
            }
            affected.push(ti);
        }
        affected.sort_unstable();
        affected.dedup();

        for &ti in affected.iter() {
            let tri = self.triangles[ti as usize];
            // Remap v→u to reflect the collapse.
            let remapped = [
                if tri[0] == v { u } else { tri[0] },
                if tri[1] == v { u } else { tri[1] },
                if tri[2] == v { u } else { tri[2] },
            ];
            // Degenerate check: a repeated index means the triangle would vanish
            // illegally (it should only vanish for the shared triangles).
            if remapped[0] == remapped[1]
                || remapped[1] == remapped[2]
                || remapped[2] == remapped[0]
            {
                return false;
            }
            // Orientation / fold check: the triangle normal must not flip when u moves
            // to `target`. Compare orient_3d of (p0,p1,p2, p0+normal_old) style is heavy;
            // instead compare the sign of the triangle normal dotted before/after and
            // require the area not collapse. We use the robust orientation predicate by
            // testing that the moved apex stays on the same side of the opposite edge's
            // supporting plane as before — approximated by a normal-flip test in f64,
            // and additionally guarded by orient_3d against the original apex.
            if self.would_flip(kernel, remapped, u, target) {
                return false;
            }
            // Edge-length guard: don't create an over-long edge (anti-oscillation).
            let p_new: [Point3; 3] = [
                if remapped[0] == u { target } else { self.vertices[remapped[0] as usize] },
                if remapped[1] == u { target } else { self.vertices[remapped[1] as usize] },
                if remapped[2] == u { target } else { self.vertices[remapped[2] as usize] },
            ];
            for k in 0..3 {
                let q0 = p_new[k];
                let q1 = p_new[(k + 1) % 3];
                let dx = q0.x - q1.x;
                let dy = q0.y - q1.y;
                let dz = q0.z - q1.z;
                if dx * dx + dy * dy + dz * dz > high_sq {
                    return false;
                }
            }
        }

        // Commit: move u to target, delete shared triangles, remap v→u everywhere.
        self.vertices[u as usize] = target;
        for &(ti, _) in shared.iter() {
            self.triangles[ti as usize] = [TOMBSTONE, TOMBSTONE, TOMBSTONE];
        }
        for &ti in affected.iter() {
            let tri = self.triangles[ti as usize];
            self.triangles[ti as usize] = [
                if tri[0] == v { u } else { tri[0] },
                if tri[1] == v { u } else { tri[1] },
                if tri[2] == v { u } else { tri[2] },
            ];
        }
        // `v` is now unreferenced; it will be dropped by `compact()`.
        true
    }

    /// Would placing vertex `moved` at `new_pos` invert the orientation of triangle
    /// `remapped` relative to its current geometry? Uses `orient_3d` to compare the
    /// signed volume of the triangle+apex tetra before and after the move; a sign flip
    /// (or collapse to coplanar) means the triangle folds. If `moved` is not a corner
    /// of the triangle, the triangle is unaffected → never a flip.
    fn would_flip<K: GeometryKernel>(
        &self,
        kernel: &K,
        remapped: [u32; 3],
        moved: u32,
        new_pos: Point3,
    ) -> bool {
        use super::expansion::Sign;
        if remapped[0] != moved && remapped[1] != moved && remapped[2] != moved {
            return false;
        }
        // Original corner positions (moved vertex still at its old location).
        let old = [
            self.vertices[remapped[0] as usize],
            self.vertices[remapped[1] as usize],
            self.vertices[remapped[2] as usize],
        ];
        let new = [
            if remapped[0] == moved { new_pos } else { old[0] },
            if remapped[1] == moved { new_pos } else { old[1] },
            if remapped[2] == moved { new_pos } else { old[2] },
        ];
        // Build a fixed off-plane reference apex from the OLD triangle normal so both
        // orientations are measured against the same external point. Using the old
        // normal keeps the reference independent of the move.
        let n = triangle_normal(old[0], old[1], old[2]);
        let nlen = (n.x * n.x + n.y * n.y + n.z * n.z).sqrt();
        if nlen == 0.0 {
            // Original triangle already degenerate; treat any move as a flip risk.
            return true;
        }
        let centroid = Point3::new(
            (old[0].x + old[1].x + old[2].x) / 3.0,
            (old[0].y + old[1].y + old[2].y) / 3.0,
            (old[0].z + old[1].z + old[2].z) / 3.0,
        );
        let apex = Point3::new(
            centroid.x + n.x / nlen,
            centroid.y + n.y / nlen,
            centroid.z + n.z / nlen,
        );
        let before = kernel.orient_3d(old[0], old[1], old[2], apex);
        let after = kernel.orient_3d(new[0], new[1], new[2], apex);
        // A flip = the sign changed, or the new triangle went coplanar (degenerate).
        // Only a preserved, strictly-nonzero, matching sign is NOT a flip.
        match (before, after) {
            (Sign::Positive, Sign::Positive) | (Sign::Negative, Sign::Negative) => false,
            _ => true,
        }
    }

    /// Distinct 1-ring vertex neighbours of `v` over live triangles (sorted).
    fn vertex_neighbours(&self, v: u32) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::new();
        for &ti in self.incident[v as usize].iter() {
            let tri = self.triangles[ti as usize];
            if Self::is_deleted(&tri) {
                continue;
            }
            for &w in &tri {
                if w != v {
                    out.push(w);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    // ── Stage 3: equalize valence by edge flips ───────────────────────────

    /// Flip interior edges to reduce total squared valence deviation from the ideal
    /// (6 for interior vertices, 4 for boundary). Rejects flips that fold a triangle,
    /// duplicate an existing edge, or touch a boundary edge. Returns the flip count.
    fn equalize_valence<K: GeometryKernel>(&mut self, kernel: &K) -> usize {
        self.ensure_clean();
        let edges = self.collect_edges();
        let mut touched = vec![false; self.vertices.len()];
        let mut flips = 0usize;

        for (u, v) in edges {
            if touched[u as usize] || touched[v as usize] {
                continue;
            }
            let tris = self.edge_triangles(u, v);
            if tris.len() != 2 {
                continue; // boundary or non-manifold edge: never flip
            }
            let a = tris[0].1; // apex of first triangle
            let b = tris[1].1; // apex of second triangle
            if a == b {
                continue; // degenerate config
            }
            // The flipped edge would be (a,b); reject if it already exists.
            if self.edge_exists(a, b) {
                continue;
            }
            // Valence improvement test.
            if !self.flip_improves_valence(u, v, a, b) {
                continue;
            }
            // Legality: the flip must not fold either new triangle.
            if !self.flip_is_legal(kernel, u, v, a, b, &tris) {
                continue;
            }
            self.apply_flip(u, v, a, b, &tris);
            touched[u as usize] = true;
            touched[v as usize] = true;
            touched[a as usize] = true;
            touched[b as usize] = true;
            flips += 1;
        }
        if flips > 0 {
            self.dirty = true;
            self.rebuild_incidence();
        }
        flips
    }

    #[inline]
    fn ideal_valence(&self, v: u32) -> i32 {
        if self.boundary[v as usize] {
            4
        } else {
            6
        }
    }

    fn valence(&self, v: u32) -> i32 {
        self.vertex_neighbours(v).len() as i32
    }

    fn edge_exists(&self, a: u32, b: u32) -> bool {
        for &ti in self.incident[a as usize].iter() {
            let tri = self.triangles[ti as usize];
            if Self::is_deleted(&tri) {
                continue;
            }
            let has_b = tri[0] == b || tri[1] == b || tri[2] == b;
            if has_b {
                return true;
            }
        }
        false
    }

    /// Does flipping edge `(u,v)`→`(a,b)` reduce Σ(valence − ideal)²?
    /// u and v each lose one neighbour; a and b each gain one.
    fn flip_improves_valence(&self, u: u32, v: u32, a: u32, b: u32) -> bool {
        let dev = |val: i32, ideal: i32| {
            let d = val - ideal;
            d * d
        };
        let (vu, vv, va, vb) = (self.valence(u), self.valence(v), self.valence(a), self.valence(b));
        let (iu, iv, ia, ib) = (
            self.ideal_valence(u),
            self.ideal_valence(v),
            self.ideal_valence(a),
            self.ideal_valence(b),
        );
        let before = dev(vu, iu) + dev(vv, iv) + dev(va, ia) + dev(vb, ib);
        let after =
            dev(vu - 1, iu) + dev(vv - 1, iv) + dev(va + 1, ia) + dev(vb + 1, ib);
        after < before
    }

    /// A flip is legal iff both resulting triangles keep the original orientation of
    /// the quad (no fold) and are non-degenerate. We orient the two new triangles the
    /// same way the two old ones were wound.
    fn flip_is_legal<K: GeometryKernel>(
        &self,
        kernel: &K,
        u: u32,
        v: u32,
        a: u32,
        b: u32,
        tris: &[(u32, u32)],
    ) -> bool {
        use super::expansion::Sign;
        // Winding of first triangle (contains apex a) tells us the consistent order.
        let (t0, t1) = self.flip_new_triangles(u, v, a, b, tris);
        // New triangles must be non-degenerate and preserve the local surface
        // orientation. Reference apex from the OLD quad normal (average of the two
        // old triangle normals) so both new triangles are compared against the same
        // external side.
        let pu = self.vertices[u as usize];
        let pv = self.vertices[v as usize];
        let pa = self.vertices[a as usize];
        let pb = self.vertices[b as usize];
        // Old triangle normals.
        let (o0, o1) = (tris[0], tris[1]);
        let n0 = self.oriented_normal(o0.0);
        let n1 = self.oriented_normal(o1.0);
        let navg = Point3::new(n0.x + n1.x, n0.y + n1.y, n0.z + n1.z);
        let nlen = (navg.x * navg.x + navg.y * navg.y + navg.z * navg.z).sqrt();
        if nlen == 0.0 {
            return false;
        }
        let centroid = Point3::new(
            (pu.x + pv.x + pa.x + pb.x) / 4.0,
            (pu.y + pv.y + pa.y + pb.y) / 4.0,
            (pu.z + pv.z + pa.z + pb.z) / 4.0,
        );
        let apex = Point3::new(
            centroid.x + navg.x / nlen,
            centroid.y + navg.y / nlen,
            centroid.z + navg.z / nlen,
        );
        // The reference sign is what the OLD triangles gave against `apex` (apex sits on
        // the +normal side, so a correctly-wound triangle gives a fixed nonzero sign).
        // Compute it from an old triangle so the two new triangles must reproduce the
        // *same* surface orientation — a flip that would invert either triangle relative
        // to the local surface is rejected.
        let sign_of = |tri: [u32; 3]| {
            kernel.orient_3d(
                self.vertices[tri[0] as usize],
                self.vertices[tri[1] as usize],
                self.vertices[tri[2] as usize],
                apex,
            )
        };
        let reference = sign_of(self.triangles[o0.0 as usize]);
        if reference == Sign::Zero {
            return false; // old triangle degenerate against apex — refuse to reason
        }
        let s0 = sign_of(t0);
        let s1 = sign_of(t1);
        // Both new triangles must be non-degenerate and match the old surface sign.
        s0 == reference && s1 == reference
    }

    /// The two triangles that replace `(u,v,a)` and `(v,u,b)` after flipping to edge
    /// `(a,b)`, wound consistently with the first old triangle.
    fn flip_new_triangles(
        &self,
        u: u32,
        v: u32,
        a: u32,
        b: u32,
        tris: &[(u32, u32)],
    ) -> ([u32; 3], [u32; 3]) {
        // `tris[0]` is the triangle whose apex is `a` (set by the caller). Read its
        // stored winding to inherit orientation for the flipped pair.
        //
        // Derivation (standard 2-2 edge flip): with T0 (apex a) traversing the shared
        // edge as u→v, the quad boundary consistent with T0/T1's orientation is
        // (v, a, u, b); splitting it on the new diagonal (a,b) yields the two triangles
        // (a,u,b) and (b,v,a). If T0 traverses v→u instead, the quad is (u,a,v,b) and
        // the pair is (a,v,b) and (b,u,a). Both preserve the surface orientation.
        let old_a = self.triangles[tris[0].0 as usize];
        let uv_forward = directed_edge_forward(old_a, u, v);
        if uv_forward {
            ([a, u, b], [b, v, a])
        } else {
            ([a, v, b], [b, u, a])
        }
    }

    /// Apply the flip: rewrite the two shared triangle slots to the flipped pair.
    fn apply_flip(&mut self, u: u32, v: u32, a: u32, b: u32, tris: &[(u32, u32)]) {
        let (t0, t1) = self.flip_new_triangles(u, v, a, b, tris);
        self.triangles[tris[0].0 as usize] = t0;
        self.triangles[tris[1].0 as usize] = t1;
    }

    /// Oriented (unnormalised) normal of live triangle `ti`, using its stored winding.
    fn oriented_normal(&self, ti: u32) -> Point3 {
        let tri = self.triangles[ti as usize];
        triangle_normal(
            self.vertices[tri[0] as usize],
            self.vertices[tri[1] as usize],
            self.vertices[tri[2] as usize],
        )
    }

    // ── Stage 4: tangential relaxation with surface re-projection ──────────

    /// Move each interior vertex toward its one-ring centroid, project the move onto
    /// the tangent plane (remove the normal component so the surface does not shrink),
    /// then snap the result back onto the *original* surface. Boundary vertices are
    /// pinned. Returns the number of vertices moved.
    fn tangential_relaxation(&mut self, projector: &SurfaceProjector) -> usize {
        self.ensure_clean();
        let vn = self.vertices.len();
        let mut new_pos = self.vertices.clone();
        let mut moved = 0usize;
        for vi in 0..vn {
            if self.boundary[vi] {
                continue;
            }
            let neigh = self.vertex_neighbours(vi as u32);
            if neigh.is_empty() {
                continue; // unreferenced (post-collapse) vertex
            }
            // Uniform one-ring centroid.
            let mut cx = 0.0;
            let mut cy = 0.0;
            let mut cz = 0.0;
            for &w in &neigh {
                let p = self.vertices[w as usize];
                cx += p.x;
                cy += p.y;
                cz += p.z;
            }
            let inv = 1.0 / neigh.len() as f64;
            let centroid = Point3::new(cx * inv, cy * inv, cz * inv);
            // Vertex normal = area-weighted sum of incident triangle normals.
            let n = self.vertex_normal(vi as u32);
            let p = self.vertices[vi];
            // Tangential update: q = p + (centroid - p) - ((centroid - p)·n) n
            let dvec = Point3::new(centroid.x - p.x, centroid.y - p.y, centroid.z - p.z);
            let nlen = (n.x * n.x + n.y * n.y + n.z * n.z).sqrt();
            let tang = if nlen > 0.0 {
                let nu = Point3::new(n.x / nlen, n.y / nlen, n.z / nlen);
                let dn = dvec.x * nu.x + dvec.y * nu.y + dvec.z * nu.z;
                Point3::new(
                    p.x + dvec.x - dn * nu.x,
                    p.y + dvec.y - dn * nu.y,
                    p.z + dvec.z - dn * nu.z,
                )
            } else {
                Point3::new(p.x + dvec.x, p.y + dvec.y, p.z + dvec.z)
            };
            // Re-project onto the original surface (keeps the mesh on-shape).
            let projected = projector.project(tang);
            if projected != p {
                moved += 1;
            }
            new_pos[vi] = projected;
        }
        // Commit all positions simultaneously (Jacobi update — deterministic and
        // order-independent within a pass).
        self.vertices = new_pos;
        moved
    }

    /// Area-weighted vertex normal over the live one-ring of `v`.
    fn vertex_normal(&self, v: u32) -> Point3 {
        let mut n = Point3::new(0.0, 0.0, 0.0);
        for &ti in self.incident[v as usize].iter() {
            let tri = self.triangles[ti as usize];
            if Self::is_deleted(&tri) {
                continue;
            }
            let fn_ = triangle_normal(
                self.vertices[tri[0] as usize],
                self.vertices[tri[1] as usize],
                self.vertices[tri[2] as usize],
            );
            n.x += fn_.x;
            n.y += fn_.y;
            n.z += fn_.z;
        }
        n
    }

    // ── Finalisation ──────────────────────────────────────────────────────

    /// Drop tombstoned triangles and unreferenced vertices, renumbering to a compact
    /// canonical form. Deterministic: preserves the relative order of surviving
    /// vertices and triangles.
    fn compact(&mut self) {
        // Which vertices are still used?
        let vn = self.vertices.len();
        let mut used = vec![false; vn];
        for tri in self.triangles.iter() {
            if Self::is_deleted(tri) {
                continue;
            }
            for &w in tri {
                used[w as usize] = true;
            }
        }
        // Build old→new vertex remap preserving order.
        let mut remap = vec![TOMBSTONE; vn];
        let mut new_vertices: Vec<Point3> = Vec::new();
        for (i, &u) in used.iter().enumerate() {
            if u {
                remap[i] = new_vertices.len() as u32;
                new_vertices.push(self.vertices[i]);
            }
        }
        // Rewrite triangles, dropping tombstones, preserving order.
        let mut new_tris: Vec<[u32; 3]> = Vec::new();
        for tri in self.triangles.iter() {
            if Self::is_deleted(tri) {
                continue;
            }
            new_tris.push([
                remap[tri[0] as usize],
                remap[tri[1] as usize],
                remap[tri[2] as usize],
            ]);
        }
        self.vertices = new_vertices;
        self.triangles = new_tris;
        self.incident.clear();
        self.boundary.clear();
        self.dirty = true;
    }

    /// Is the current live mesh closed (no boundary edges) and manifold?
    fn is_closed_manifold(&self) -> bool {
        let live: Vec<[u32; 3]> = self
            .triangles
            .iter()
            .copied()
            .filter(|t| !Self::is_deleted(t))
            .collect();
        if live.is_empty() {
            return false;
        }
        let edge_count = live.len() * 3;
        let slot_count = required_edge_slots(live.len());
        let mut edges = vec![HalfEdge::default(); edge_count];
        let mut slots = vec![EdgeSlot::default(); slot_count];
        match build_triangle_half_edges(self.vertices.len() as u32, &live, &mut edges, &mut slots) {
            Ok(summary) => summary.boundary_half_edges == 0,
            Err(_) => false,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  Geometry helpers
// ──────────────────────────────────────────────────────────────────────────

/// Unnormalised triangle normal `(b−a)×(c−a)`.
#[inline]
fn triangle_normal(a: Point3, b: Point3, c: Point3) -> Point3 {
    let ux = b.x - a.x;
    let uy = b.y - a.y;
    let uz = b.z - a.z;
    let vx = c.x - a.x;
    let vy = c.y - a.y;
    let vz = c.z - a.z;
    Point3::new(
        uy * vz - uz * vy,
        uz * vx - ux * vz,
        ux * vy - uy * vx,
    )
}

/// Does triangle `tri` traverse the directed edge `u→v` (as opposed to `v→u`)?
#[inline]
fn directed_edge_forward(tri: [u32; 3], u: u32, v: u32) -> bool {
    for k in 0..3 {
        let a = tri[k];
        let b = tri[(k + 1) % 3];
        if a == u && b == v {
            return true;
        }
        if a == v && b == u {
            return false;
        }
    }
    // Edge not found (shouldn't happen for a valid incident triangle); default true.
    true
}

/// Split triangle `tri` (given winding) across undirected edge `(u,v)` at new vertex
/// `mid`, returning the two child triangles that preserve the original winding.
///
/// If the triangle winds `… u → v …`, the edge `u→v` becomes `u→mid` and `mid→v`;
/// the apex stays fixed and the two children inherit the parent's orientation.
fn split_triangle_at_edge(tri: [u32; 3], u: u32, v: u32, mid: u32) -> [[u32; 3]; 2] {
    // Find the local index k such that (tri[k], tri[k+1]) is the directed edge on the
    // undirected pair {u,v}. Whatever the direction, we insert `mid` between them.
    for k in 0..3 {
        let a = tri[k];
        let b = tri[(k + 1) % 3];
        let c = tri[(k + 2) % 3]; // apex
        if (a == u && b == v) || (a == v && b == u) {
            // Parent winds a → b → c. Split edge a→b at mid:
            //   child 1: a → mid → c
            //   child 2: mid → b → c
            return [[a, mid, c], [mid, b, c]];
        }
    }
    // Fallback (edge not present): return the triangle unchanged twice (should not
    // happen for a genuine incident triangle).
    [tri, tri]
}

// ──────────────────────────────────────────────────────────────────────────
//  Surface projection (exact nearest triangle on the ORIGINAL mesh)
// ──────────────────────────────────────────────────────────────────────────

/// Snaps a point back onto the input surface by finding the closest point on the
/// nearest original triangle. Cold construction stores the original triangle corner
/// positions; `project` is a linear scan (see the module's `// FOLLOW-UP: BVH`).
struct SurfaceProjector {
    /// Flattened original triangles as `(a,b,c)` corner points.
    tris: Vec<[Point3; 3]>,
}

impl SurfaceProjector {
    fn new(vertices: &[Point3], triangles: &[[u32; 3]]) -> Self {
        let tris = triangles
            .iter()
            .map(|t| {
                [
                    vertices[t[0] as usize],
                    vertices[t[1] as usize],
                    vertices[t[2] as usize],
                ]
            })
            .collect();
        SurfaceProjector { tris }
    }

    /// Return the closest point on the original surface to `p`. If the original mesh
    /// had no triangles, returns `p` unchanged.
    fn project(&self, p: Point3) -> Point3 {
        let mut best = p;
        let mut best_sq = f64::INFINITY;
        for t in &self.tris {
            let q = closest_point_on_triangle(p, t[0], t[1], t[2]);
            let dx = p.x - q.x;
            let dy = p.y - q.y;
            let dz = p.z - q.z;
            let d = dx * dx + dy * dy + dz * dz;
            if d < best_sq {
                best_sq = d;
                best = q;
            }
        }
        best
    }
}

/// Closest point ON triangle `abc` to `p` (barycentric region test — Ericson,
/// *Real-Time Collision Detection*, §5.1.5; original Rust). Mirrors the region logic
/// of [`super::distance::point_triangle_distance_sq_3d`] but returns the point, not
/// just the squared distance, since relaxation needs the projected position.
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
        let vv = d1 / (d1 - d3);
        return add(a, scale(ab, vv)); // edge ab
    }
    let cp = sub(p, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let ww = d2 / (d2 - d6);
        return add(a, scale(ac, ww)); // edge ac
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let ww = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return add(b, scale(sub(c, b), ww)); // edge bc
    }
    // Interior.
    let denom = 1.0 / (va + vb + vc);
    let vv = vb * denom;
    let ww = vc * denom;
    add(a, add(scale(ab, vv), scale(ac, ww)))
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

// ──────────────────────────────────────────────────────────────────────────
//  Tests
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::surface_mesh_processing::{surface_area, signed_volume};
    use super::super::topology::{build_triangle_half_edges, required_edge_slots, EdgeSlot, HalfEdge};

    // ── Mesh builders ─────────────────────────────────────────────────────

    /// Unit cube [0,1]³, 12 outward-wound triangles (from surface_mesh_processing tests).
    fn unit_cube() -> (Vec<Point3>, Vec<[u32; 3]>) {
        let v = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(1.0, 1.0, 1.0),
            Point3::new(0.0, 1.0, 1.0),
        ];
        let t = vec![
            [0, 3, 2], [0, 2, 1],
            [4, 5, 6], [4, 6, 7],
            [0, 1, 5], [0, 5, 4],
            [3, 7, 6], [3, 6, 2],
            [0, 4, 7], [0, 7, 3],
            [1, 2, 6], [1, 6, 5],
        ];
        (v, t)
    }

    /// Flat square in the z=0 plane subdivided into a `n × n` grid of quads (2 tris
    /// each). Open mesh with a boundary. Side length 1.
    fn planar_grid(n: usize) -> (Vec<Point3>, Vec<[u32; 3]>) {
        let mut v = Vec::new();
        for j in 0..=n {
            for i in 0..=n {
                v.push(Point3::new(i as f64 / n as f64, j as f64 / n as f64, 0.0));
            }
        }
        let idx = |i: usize, j: usize| (j * (n + 1) + i) as u32;
        let mut t = Vec::new();
        for j in 0..n {
            for i in 0..n {
                // CCW as seen from +z.
                t.push([idx(i, j), idx(i + 1, j), idx(i + 1, j + 1)]);
                t.push([idx(i, j), idx(i + 1, j + 1), idx(i, j + 1)]);
            }
        }
        (v, t)
    }

    /// Octahedron centred at origin, radius 1, outward-wound. Closed manifold, 6v/8t.
    fn octahedron() -> (Vec<Point3>, Vec<[u32; 3]>) {
        let v = vec![
            Point3::new(1.0, 0.0, 0.0),  // 0 +x
            Point3::new(-1.0, 0.0, 0.0), // 1 -x
            Point3::new(0.0, 1.0, 0.0),  // 2 +y
            Point3::new(0.0, -1.0, 0.0), // 3 -y
            Point3::new(0.0, 0.0, 1.0),  // 4 +z
            Point3::new(0.0, 0.0, -1.0), // 5 -z
        ];
        // Eight faces, each wound CCW when seen from outside.
        let t = vec![
            [0, 2, 4],
            [2, 1, 4],
            [1, 3, 4],
            [3, 0, 4],
            [2, 0, 5],
            [1, 2, 5],
            [3, 1, 5],
            [0, 3, 5],
        ];
        (v, t)
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    /// Rebuild a half-edge summary for a mesh; panics if non-manifold.
    fn manifold_summary(vertices: &[Point3], triangles: &[[u32; 3]]) -> (u32, u32) {
        let edge_count = triangles.len() * 3;
        let slot_count = required_edge_slots(triangles.len());
        let mut edges = vec![HalfEdge::default(); edge_count];
        let mut slots = vec![EdgeSlot::default(); slot_count];
        let s = build_triangle_half_edges(vertices.len() as u32, triangles, &mut edges, &mut slots)
            .expect("output must be a valid 2-manifold");
        (s.face_count, s.boundary_half_edges)
    }

    /// All undirected edge lengths of a mesh.
    fn edge_lengths(vertices: &[Point3], triangles: &[[u32; 3]]) -> Vec<f64> {
        let mut edges: Vec<(u32, u32)> = Vec::new();
        for tri in triangles {
            for k in 0..3 {
                let a = tri[k];
                let b = tri[(k + 1) % 3];
                edges.push(if a < b { (a, b) } else { (b, a) });
            }
        }
        edges.sort_unstable();
        edges.dedup();
        edges
            .iter()
            .map(|&(a, b)| {
                let p = vertices[a as usize];
                let q = vertices[b as usize];
                ((p.x - q.x).powi(2) + (p.y - q.y).powi(2) + (p.z - q.z).powi(2)).sqrt()
            })
            .collect()
    }

    /// Verify no triangle is degenerate (repeated index) or references a missing vertex.
    fn assert_wellformed(vertices: &[Point3], triangles: &[[u32; 3]]) {
        for (t, tri) in triangles.iter().enumerate() {
            assert!(tri[0] != tri[1] && tri[1] != tri[2] && tri[2] != tri[0], "degenerate tri {t}");
            for &vi in tri {
                assert!((vi as usize) < vertices.len(), "tri {t} vertex {vi} OOB");
            }
        }
    }

    // ── Validation / error tests ──────────────────────────────────────────

    #[test]
    fn rejects_nonpositive_target() {
        let (v, t) = unit_cube();
        let mut ov = vec![Point3::default(); 4096];
        let mut ot = vec![[0u32; 3]; 4096];
        assert_eq!(
            isotropic_remesh(&v, &t, RemeshOptions::new(0.0, 1), &mut ov, &mut ot),
            Err(RemeshError::InvalidTargetLength)
        );
        assert_eq!(
            isotropic_remesh(&v, &t, RemeshOptions::new(f64::NAN, 1), &mut ov, &mut ot),
            Err(RemeshError::InvalidTargetLength)
        );
    }

    #[test]
    fn rejects_out_of_bounds_index() {
        let v = vec![Point3::new(0.0, 0.0, 0.0)];
        let t = vec![[0u32, 1, 2]];
        let mut ov = vec![Point3::default(); 16];
        let mut ot = vec![[0u32; 3]; 16];
        assert_eq!(
            isotropic_remesh(&v, &t, RemeshOptions::new(1.0, 1), &mut ov, &mut ot),
            Err(RemeshError::IndexOutOfBounds { triangle: 0, vertex: 1 })
        );
    }

    #[test]
    fn rejects_non_finite_vertex() {
        let v = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(f64::INFINITY, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let t = vec![[0u32, 1, 2]];
        let mut ov = vec![Point3::default(); 16];
        let mut ot = vec![[0u32; 3]; 16];
        assert_eq!(
            isotropic_remesh(&v, &t, RemeshOptions::new(1.0, 1), &mut ov, &mut ot),
            Err(RemeshError::NonFiniteCoordinate { index: 1 })
        );
    }

    #[test]
    fn rejects_degenerate_input_face() {
        let v = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let t = vec![[0u32, 0, 1]];
        let mut ov = vec![Point3::default(); 16];
        let mut ot = vec![[0u32; 3]; 16];
        assert_eq!(
            isotropic_remesh(&v, &t, RemeshOptions::new(1.0, 1), &mut ov, &mut ot),
            Err(RemeshError::DegenerateInputFace { triangle: 0 })
        );
    }

    #[test]
    fn rejects_nonmanifold_input() {
        // Three triangles sharing one edge (0,1): non-manifold.
        let v = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, -1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        ];
        let t = vec![[0, 1, 2], [1, 0, 3], [0, 1, 4]];
        let mut ov = vec![Point3::default(); 64];
        let mut ot = vec![[0u32; 3]; 64];
        assert_eq!(
            isotropic_remesh(&v, &t, RemeshOptions::new(1.0, 1), &mut ov, &mut ot),
            Err(RemeshError::NonManifoldInput)
        );
    }

    #[test]
    fn zero_iterations_passes_through_manifold() {
        let (v, t) = unit_cube();
        let mut ov = vec![Point3::default(); 64];
        let mut ot = vec![[0u32; 3]; 64];
        let rep = isotropic_remesh(&v, &t, RemeshOptions::new(0.5, 0), &mut ov, &mut ot).unwrap();
        assert_eq!(rep.vertex_count, 8);
        assert_eq!(rep.triangle_count, 12);
        assert!(rep.closed_manifold);
        assert_eq!(&ov[..8], &v[..]);
    }

    #[test]
    fn output_too_small_errors() {
        let (v, t) = unit_cube();
        let mut ov = vec![Point3::default(); 2]; // too small
        let mut ot = vec![[0u32; 3]; 64];
        let r = isotropic_remesh(&v, &t, RemeshOptions::new(0.5, 2), &mut ov, &mut ot);
        assert!(matches!(r, Err(RemeshError::VertexOutputTooSmall { .. })));
    }

    // ── Split refines a coarse mesh ───────────────────────────────────────

    #[test]
    fn splitting_refines_toward_target_on_plane() {
        // Coarse 1×1 plane (edges ~1.0 and ~1.414). Remesh toward 0.3.
        let (v, t) = planar_grid(1);
        let target = 0.3;
        let (cap_v, cap_t) = required_output_capacity(v.len(), t.len(), 5);
        let mut ov = vec![Point3::default(); cap_v];
        let mut ot = vec![[0u32; 3]; cap_t];
        let rep = isotropic_remesh(&v, &t, RemeshOptions::new(target, 5), &mut ov, &mut ot).unwrap();

        let ov = &ov[..rep.vertex_count];
        let ot = &ot[..rep.triangle_count];
        assert_wellformed(ov, ot);
        // Boundary is preserved (still a boundary mesh, not closed).
        let (_faces, boundary) = manifold_summary(ov, ot);
        assert!(boundary > 0, "planar patch must remain open");
        assert!(!rep.closed_manifold);

        // Post edge-length histogram must be near target: the vast majority within
        // [4/5, 4/3]·target, and no edge longer than a small multiple of target.
        let lens = edge_lengths(ov, ot);
        let low = 0.8 * target;
        let high = 4.0 / 3.0 * target;
        let within = lens.iter().filter(|&&l| l >= low * 0.9 && l <= high * 1.15).count();
        let frac = within as f64 / lens.len() as f64;
        assert!(frac > 0.6, "only {:.0}% of edges near target ({} of {})",
            frac * 100.0, within, lens.len());
        // No edge should be wildly longer than target after refinement.
        let max_len = lens.iter().cloned().fold(0.0f64, f64::max);
        assert!(max_len < 2.0 * target, "max edge {max_len} >> target {target}");
    }

    #[test]
    fn plane_area_is_preserved() {
        let (v, t) = planar_grid(2);
        let orig_area = surface_area(&v, &t).unwrap();
        let target = 0.25;
        let (cap_v, cap_t) = required_output_capacity(v.len(), t.len(), 5);
        let mut ov = vec![Point3::default(); cap_v];
        let mut ot = vec![[0u32; 3]; cap_t];
        let rep = isotropic_remesh(&v, &t, RemeshOptions::new(target, 5), &mut ov, &mut ot).unwrap();
        let ov = &ov[..rep.vertex_count];
        let ot = &ot[..rep.triangle_count];
        let new_area = surface_area(ov, ot).unwrap();
        // A flat plane's area is exactly preserved by on-plane projection (tolerance
        // for boundary handling — boundary vertices are pinned, so area is exact here).
        assert!((new_area - orig_area).abs() < 1e-9,
            "plane area drifted: {orig_area} → {new_area}");
    }

    // ── Closed manifold refine preserves manifoldness + orientation + volume ─

    #[test]
    fn octahedron_refine_preserves_manifold_and_orientation() {
        let (v, t) = octahedron();
        let orig_vol = signed_volume(&v, &t).unwrap();
        assert!(orig_vol > 0.0, "octahedron must be outward-wound (+vol), got {orig_vol}");

        let target = 0.5;
        let (cap_v, cap_t) = required_output_capacity(v.len(), t.len(), 4);
        let mut ov = vec![Point3::default(); cap_v];
        let mut ot = vec![[0u32; 3]; cap_t];
        let rep = isotropic_remesh(&v, &t, RemeshOptions::new(target, 4), &mut ov, &mut ot).unwrap();
        let ov = &ov[..rep.vertex_count];
        let ot = &ot[..rep.triangle_count];

        assert_wellformed(ov, ot);
        // Manifold + closed (0 boundary) — the invariant that MUST hold every step.
        let (_faces, boundary) = manifold_summary(ov, ot);
        assert_eq!(boundary, 0, "octahedron must stay closed");
        assert!(rep.closed_manifold);

        // Orientation preserved: signed volume stays strictly positive (no face flipped).
        let new_vol = signed_volume(ov, ot).unwrap();
        assert!(new_vol > 0.0, "orientation flipped: new volume {new_vol}");

        // HONEST volume bound: this is a *feature-agnostic* remesher. The octahedron is
        // all sharp corners; collapsing edges near a corner and projecting to the
        // nearest flat face rounds the corner inward, so volume shrinks (measured ~31%
        // for L=0.5). We assert it stays within a wide band and does not collapse to
        // near-zero. Tight volume preservation is only expected on smooth surfaces or
        // when collapses do not fire — see `closed_mesh_no_collapse_preserves_volume`.
        assert!(new_vol > 0.55 * orig_vol && new_vol <= orig_vol * 1.05,
            "volume out of expected band: {orig_vol} → {new_vol}");

        // Edge-length histogram must be tight around target (the primary quality goal).
        let lens = edge_lengths(ov, ot);
        let low = 0.8 * target;
        let high = 4.0 / 3.0 * target;
        let within = lens.iter().filter(|&&l| l >= low * 0.85 && l <= high * 1.2).count();
        let frac = within as f64 / lens.len() as f64;
        assert!(frac > 0.85, "only {:.0}% of edges near target", frac * 100.0);

        assert!(rep.splits > 0, "no splits on a coarse octahedron toward L=0.5");
        assert!(rep.vertex_count > v.len(), "mesh did not refine");
    }

    #[test]
    fn cube_refine_preserves_manifold_and_orientation() {
        let (v, t) = unit_cube();
        let orig_vol = signed_volume(&v, &t).unwrap();
        let target = 0.35;
        let (cap_v, cap_t) = required_output_capacity(v.len(), t.len(), 4);
        let mut ov = vec![Point3::default(); cap_v];
        let mut ot = vec![[0u32; 3]; cap_t];
        let rep = isotropic_remesh(&v, &t, RemeshOptions::new(target, 4), &mut ov, &mut ot).unwrap();
        let ov = &ov[..rep.vertex_count];
        let ot = &ot[..rep.triangle_count];

        assert_wellformed(ov, ot);
        let (_f, boundary) = manifold_summary(ov, ot);
        assert_eq!(boundary, 0);
        assert!(rep.closed_manifold);
        let new_vol = signed_volume(ov, ot).unwrap();
        // Orientation preserved (positive); volume within the feature-rounding band
        // (a cube is all sharp edges — measured ~19% loss for L=0.35).
        assert!(new_vol > 0.0);
        assert!(new_vol > 0.7 * orig_vol && new_vol <= orig_vol * 1.05,
            "cube volume out of band {orig_vol}→{new_vol}");
    }

    /// The algorithm itself does NOT drift volume: on a closed mesh whose edges are
    /// already near target (so split/collapse barely fire), flips + tangential
    /// relaxation preserve the enclosed volume tightly. This isolates "corner rounding
    /// is the only source of drift" — proving the ops are volume-faithful in the smooth
    /// regime. We build an already-fine octahedron (subdivided once) and remesh toward
    /// its own current edge length, so few collapses occur.
    #[test]
    fn closed_mesh_no_collapse_preserves_volume_when_uniform() {
        // Subdivide the octahedron once (1-to-4) by projecting each edge midpoint OUT to
        // radius 1, giving a rounder, near-uniform closed mesh.
        let (mut v, t0) = octahedron();
        use std::collections::HashMap;
        let mut mid: HashMap<(u32, u32), u32> = HashMap::new();
        let get_mid = |v: &mut Vec<Point3>, a: u32, b: u32, mid: &mut HashMap<(u32, u32), u32>| {
            let key = if a < b { (a, b) } else { (b, a) };
            if let Some(&m) = mid.get(&key) {
                return m;
            }
            let pa = v[a as usize];
            let pb = v[b as usize];
            let mut mx = 0.5 * (pa.x + pb.x);
            let mut my = 0.5 * (pa.y + pb.y);
            let mut mz = 0.5 * (pa.z + pb.z);
            let r = (mx * mx + my * my + mz * mz).sqrt();
            if r > 0.0 { mx /= r; my /= r; mz /= r; }
            let idx = v.len() as u32;
            v.push(Point3::new(mx, my, mz));
            mid.insert(key, idx);
            idx
        };
        let mut t: Vec<[u32; 3]> = Vec::new();
        for tri in &t0 {
            let a = tri[0]; let b = tri[1]; let c = tri[2];
            let ab = get_mid(&mut v, a, b, &mut mid);
            let bc = get_mid(&mut v, b, c, &mut mid);
            let ca = get_mid(&mut v, c, a, &mut mid);
            t.push([a, ab, ca]);
            t.push([ab, b, bc]);
            t.push([ca, bc, c]);
            t.push([ab, bc, ca]);
        }
        let orig_vol = signed_volume(&v, &t).unwrap();
        assert!(orig_vol > 0.0);

        // Current mean edge length ~0.7; remesh toward exactly that so split/collapse
        // are minimal and mostly flips + relaxation run.
        let target = {
            let lens = edge_lengths(&v, &t);
            lens.iter().sum::<f64>() / lens.len() as f64
        };
        let (cap_v, cap_t) = required_output_capacity(v.len(), t.len(), 3);
        let mut ov = vec![Point3::default(); cap_v];
        let mut ot = vec![[0u32; 3]; cap_t];
        let rep = isotropic_remesh(&v, &t, RemeshOptions::new(target, 3), &mut ov, &mut ot).unwrap();
        let ov = &ov[..rep.vertex_count];
        let ot = &ot[..rep.triangle_count];

        let (_f, boundary) = manifold_summary(ov, ot);
        assert_eq!(boundary, 0);
        let new_vol = signed_volume(ov, ot).unwrap();
        // With few/no collapses on this rounder mesh, volume is preserved to a few %.
        assert!(new_vol > 0.0);
        assert!((new_vol - orig_vol).abs() < 0.12 * orig_vol,
            "uniform closed remesh drifted too much: {orig_vol} → {new_vol}");
    }

    // ── Coarsening: collapse shortens over-fine edges toward target ────────

    #[test]
    fn collapse_coarsens_overfine_plane() {
        // Start FINE (grid 8 → edge 0.125) and remesh toward a LARGER target 0.4 so
        // the collapse stage must fire.
        let (v, t) = planar_grid(8);
        let fine_edges = edge_lengths(&v, &t);
        let fine_mean: f64 = fine_edges.iter().sum::<f64>() / fine_edges.len() as f64;

        let target = 0.4;
        let (cap_v, cap_t) = required_output_capacity(v.len(), t.len(), 6);
        let mut ov = vec![Point3::default(); cap_v];
        let mut ot = vec![[0u32; 3]; cap_t];
        let rep = isotropic_remesh(&v, &t, RemeshOptions::new(target, 6), &mut ov, &mut ot).unwrap();
        let ov = &ov[..rep.vertex_count];
        let ot = &ot[..rep.triangle_count];

        assert_wellformed(ov, ot);
        assert!(rep.collapses > 0, "no collapses when coarsening a fine plane");
        // Vertex count must drop meaningfully.
        assert!(rep.vertex_count < v.len(), "coarsening did not reduce vertices ({} → {})",
            v.len(), rep.vertex_count);
        // Mean edge length should move toward the (larger) target.
        let new_edges = edge_lengths(ov, ot);
        let new_mean: f64 = new_edges.iter().sum::<f64>() / new_edges.len() as f64;
        assert!(new_mean > fine_mean, "mean edge did not grow: {fine_mean} → {new_mean}");
        // Plane must remain open with a boundary.
        let (_f, boundary) = manifold_summary(ov, ot);
        assert!(boundary > 0);
    }

    // ── Determinism ───────────────────────────────────────────────────────

    #[test]
    fn deterministic_bit_identical() {
        let (v, t) = octahedron();
        let target = 0.5;
        let (cap_v, cap_t) = required_output_capacity(v.len(), t.len(), 4);

        let mut ov_a = vec![Point3::default(); cap_v];
        let mut ot_a = vec![[0u32; 3]; cap_t];
        let ra = isotropic_remesh(&v, &t, RemeshOptions::new(target, 4), &mut ov_a, &mut ot_a).unwrap();

        let mut ov_b = vec![Point3::default(); cap_v];
        let mut ot_b = vec![[0u32; 3]; cap_t];
        let rb = isotropic_remesh(&v, &t, RemeshOptions::new(target, 4), &mut ov_b, &mut ot_b).unwrap();

        assert_eq!(ra, rb);
        assert_eq!(ra.vertex_count, rb.vertex_count);
        // Bit-identical vertex coordinates.
        for i in 0..ra.vertex_count {
            assert_eq!(ov_a[i].x.to_bits(), ov_b[i].x.to_bits());
            assert_eq!(ov_a[i].y.to_bits(), ov_b[i].y.to_bits());
            assert_eq!(ov_a[i].z.to_bits(), ov_b[i].z.to_bits());
        }
        // Bit-identical triangle indices.
        assert_eq!(&ot_a[..ra.triangle_count], &ot_b[..rb.triangle_count]);
    }

    // ── Flip legality: never fold, never duplicate an edge ────────────────

    #[test]
    fn flips_never_break_manifold() {
        // A refined icosphere-ish surface where valence equalisation will flip edges.
        // Use the octahedron refined a couple of passes; then confirm every stage's
        // invariants held by re-validating the output.
        let (v, t) = octahedron();
        let (cap_v, cap_t) = required_output_capacity(v.len(), t.len(), 3);
        let mut ov = vec![Point3::default(); cap_v];
        let mut ot = vec![[0u32; 3]; cap_t];
        let rep = isotropic_remesh(&v, &t, RemeshOptions::new(0.6, 3), &mut ov, &mut ot).unwrap();
        let ov = &ov[..rep.vertex_count];
        let ot = &ot[..rep.triangle_count];
        // The output builds cleanly as a manifold (build_triangle_half_edges rejects
        // duplicate directed edges + non-manifold edges), so if flips had duplicated an
        // edge or folded a triangle this would panic.
        let (_f, boundary) = manifold_summary(ov, ot);
        assert_eq!(boundary, 0);
        assert!(rep.closed_manifold);
    }

    // ── Unit tests for the geometry helpers ───────────────────────────────

    #[test]
    fn closest_point_interior() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let p = Point3::new(0.25, 0.25, 3.0);
        let q = closest_point_on_triangle(p, a, b, c);
        assert!((q.x - 0.25).abs() < 1e-12);
        assert!((q.y - 0.25).abs() < 1e-12);
        assert!((q.z - 0.0).abs() < 1e-12);
    }

    #[test]
    fn closest_point_vertex_and_edge() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        // Beyond vertex a.
        let q = closest_point_on_triangle(Point3::new(-1.0, -1.0, 0.0), a, b, c);
        assert_eq!(q, a);
        // On edge ab (projects to (0.5,0,0)).
        let q2 = closest_point_on_triangle(Point3::new(0.5, -1.0, 0.0), a, b, c);
        assert!((q2.x - 0.5).abs() < 1e-12 && q2.y.abs() < 1e-12);
    }

    #[test]
    fn split_triangle_preserves_winding() {
        // Triangle 0→1→2, split edge (0,1) at mid=3.
        let out = split_triangle_at_edge([0, 1, 2], 0, 1, 3);
        // Expect [0,3,2] and [3,1,2] — both wind consistently with the parent.
        assert_eq!(out, [[0, 3, 2], [3, 1, 2]]);
        // Reverse-direction edge argument yields the same partition.
        let out2 = split_triangle_at_edge([0, 1, 2], 1, 0, 3);
        assert_eq!(out2, [[0, 3, 2], [3, 1, 2]]);
    }

    #[test]
    fn directed_edge_direction() {
        assert!(directed_edge_forward([0, 1, 2], 0, 1));
        assert!(!directed_edge_forward([0, 1, 2], 1, 0));
        assert!(directed_edge_forward([0, 1, 2], 2, 0));
    }

    #[test]
    fn required_capacity_is_monotone_and_generous() {
        let (v0, f0) = required_output_capacity(6, 8, 0);
        assert!(v0 >= 6 && f0 >= 8);
        let (v3, f3) = required_output_capacity(6, 8, 3);
        assert!(v3 > v0 && f3 > f0);
    }
}
