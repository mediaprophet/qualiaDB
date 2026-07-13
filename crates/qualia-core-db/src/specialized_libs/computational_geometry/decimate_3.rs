//! P5.7 — Mesh **decimation** via Garland–Heckbert quadric error metrics (QEM).
//!
//! Greedy edge-collapse simplification of a triangle mesh (`&[Point3]` +
//! `&[[u32; 3]]`) to a target triangle count **or** a target maximum quadric
//! error. Each vertex accumulates a `4×4` error quadric `Q = Σ Kₚ` over its
//! incident face planes (`Kₚ = p pᵀ`, with `p = (a, b, c, d)` the unit-normal
//! plane equation `a x + b y + c z + d = 0`). The cost of collapsing an edge
//! `(i, j)` to a position `v̄` is the quadratic form `v̄ᵀ (Qᵢ + Qⱼ) v̄`; the
//! optimal `v̄` solves `∇(v̄ᵀ Q v̄) = 0` (the top-left `3×3` of `Q` against the
//! right column), with a robust midpoint/endpoint fallback when that system is
//! singular. Edges are collapsed cheapest-first; the accumulated quadric of the
//! surviving vertex becomes `Qᵢ + Qⱼ`, and incident edge costs are recomputed.
//!
//! ## Correctness guards (this is anatomy-critical LOD — wrong output is
//! invalid topology, not a small error)
//!
//! - **Foldover / normal-flip rejection.** Before committing a collapse, every
//!   triangle that survives the collapse (i.e. is incident to the kept vertex
//!   but does not degenerate) is checked with the exact orientation predicate
//!   [`GeometryKernel::orient_3d`]: if the face's orientation relative to its own
//!   pre-collapse apex flips, the collapse is rejected and the next-cheapest edge
//!   is tried. This is the standard QEM safeguard against creating self-folded,
//!   inverted geometry.
//! - **Manifold / link-condition guard.** A collapse is rejected unless the two
//!   endpoints share exactly the faces on the collapsed edge (the *link
//!   condition*); collapsing an edge whose endpoints share a non-edge vertex
//!   would pinch the surface into a non-manifold state. Rejected collapses are
//!   skipped, never faked.
//! - **Boundary quadrics.** Boundary (unpaired) edges add a large perpendicular
//!   "virtual plane" quadric so the open border is preserved, per Garland–
//!   Heckbert §4.
//!
//! ## Zero-heap contract (honest scope)
//!
//! This is a **cold, one-shot construction** (a mesh-build step, not a hot query
//! path). Per the module's caller-buffered contract, the *public output* is
//! written into caller-owned slices (`out_vertices`, `out_triangles`) and the
//! sizes are reported. The greedy simplification itself maintains mutable
//! adjacency and a cost heap, for which an internal `Vec`/`BinaryHeap` working
//! set is used — this is documented and confined to the build, matching the
//! precedent set by the P4 Delaunay/Voronoi builders in this module. No `Vec`
//! escapes into the public surface, and there is no allocation in any predicate.
//!
//! ## Determinism
//!
//! Identical input → **bit-identical** output. Ties in the cost heap are broken
//! by a total order on `(cost_bits, edge_key)` so the collapse sequence — and
//! therefore the vertex remap and the emitted triangle list — is canonical and
//! reproducible. A determinism test asserts bit-identical vertex coordinates and
//! identical triangle indices across two runs.

use core::cmp::Ordering;

use super::kernel::{FilteredF64Kernel, GeometryKernel};
use super::primitives::Point3;

/// Failure modes for QEM decimation. All are input-integrity or buffer-sizing
/// faults surfaced fail-closed; a finite, in-bounds mesh always decimates to a
/// finite result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecimateError {
    /// A triangle referenced a vertex index outside `vertices`.
    IndexOutOfBounds { triangle: usize, vertex: u32 },
    /// A referenced vertex had a non-finite coordinate (NaN / ±∞).
    NonFiniteCoordinate { index: usize },
    /// A face was degenerate on input (two identical corner indices).
    DegenerateInputFace { triangle: usize },
    /// The vertex output buffer is too small; needs at least `required` entries.
    VertexOutputTooSmall { required: usize },
    /// The triangle output buffer is too small; needs at least `required` entries.
    TriangleOutputTooSmall { required: usize },
    /// `target_faces` exceeds the input face count (nothing to decimate up to).
    InvalidTarget { target: usize, input_faces: usize },
    /// The mesh has more vertices than the `u32` index space allows.
    TooManyVertices,
}

/// Options controlling how far decimation proceeds.
///
/// Decimation stops as soon as **either** limit is reached:
/// - `target_faces`: stop once the live triangle count is `<= target_faces`.
/// - `max_error`: stop once the cheapest available collapse would exceed this
///   quadric error. `None` means "no error ceiling" (run to `target_faces`).
///
/// At least one of the two must bound the run; if both are `None`, decimation
/// runs until no further legal collapse exists (full simplification), which is
/// well-defined but rarely what a caller wants — prefer setting a target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecimateOptions {
    /// Target live triangle count. Collapsing stops at or below this.
    pub target_faces: usize,
    /// Optional quadric-error ceiling; collapses costing more are not taken.
    pub max_error: Option<f64>,
}

impl DecimateOptions {
    /// Decimate down to `target_faces` with no explicit error ceiling.
    #[inline]
    pub const fn to_faces(target_faces: usize) -> Self {
        Self {
            target_faces,
            max_error: None,
        }
    }

    /// Decimate greedily until the cheapest collapse would exceed `max_error`
    /// (with a `target_faces` floor of 0 — i.e. error is the only stop).
    #[inline]
    pub const fn to_error(max_error: f64) -> Self {
        Self {
            target_faces: 0,
            max_error: Some(max_error),
        }
    }
}

/// What decimation actually produced.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecimateReport {
    /// Live vertex count written to `out_vertices` (compacted, no orphans).
    pub vertices: usize,
    /// Live triangle count written to `out_triangles`.
    pub faces: usize,
    /// Number of edge collapses actually committed.
    pub collapses: usize,
    /// The largest single-collapse quadric error incurred over the whole run
    /// (0.0 if no collapse happened). This is the QEM cost, i.e. `v̄ᵀ Q v̄` at
    /// the chosen position — an upper bound proxy for squared distance to the
    /// original surface, not an exact Hausdorff distance.
    pub max_error: f64,
    /// Whether the run stopped because the error ceiling was hit (as opposed to
    /// reaching `target_faces` or running out of legal collapses).
    pub stopped_on_error: bool,
}

/// Upper bound on the vertex output slots needed for an input of `vertex_count`
/// vertices. Decimation never adds vertices, so the input count is the bound.
#[inline]
pub fn required_vertices(vertex_count: usize) -> usize {
    vertex_count
}

/// Upper bound on the triangle output slots needed for an input of
/// `triangle_count` triangles. Decimation never adds faces.
#[inline]
pub fn required_triangles(triangle_count: usize) -> usize {
    triangle_count
}

// ──────────────────────────────────────────────────────────────────────────
//  4×4 symmetric quadric (10 unique coefficients)
// ──────────────────────────────────────────────────────────────────────────
//
// Q = [[a2, ab, ac, ad],
//      [ab, b2, bc, bd],
//      [ac, bc, c2, cd],
//      [ad, bd, cd, d2]]
//
// stored as the upper triangle: [a2, ab, ac, ad, b2, bc, bd, c2, cd, d2].

#[derive(Debug, Clone, Copy, PartialEq)]
struct Quadric {
    a2: f64,
    ab: f64,
    ac: f64,
    ad: f64,
    b2: f64,
    bc: f64,
    bd: f64,
    c2: f64,
    cd: f64,
    d2: f64,
}

impl Quadric {
    #[inline]
    const fn zero() -> Self {
        Self {
            a2: 0.0,
            ab: 0.0,
            ac: 0.0,
            ad: 0.0,
            b2: 0.0,
            bc: 0.0,
            bd: 0.0,
            c2: 0.0,
            cd: 0.0,
            d2: 0.0,
        }
    }

    /// Build `p pᵀ` from a plane `(a, b, c, d)` (`a x + b y + c z + d = 0`),
    /// optionally scaled by `weight` (face area, or a large boundary weight).
    #[inline]
    fn from_plane(a: f64, b: f64, c: f64, d: f64, weight: f64) -> Self {
        Self {
            a2: weight * a * a,
            ab: weight * a * b,
            ac: weight * a * c,
            ad: weight * a * d,
            b2: weight * b * b,
            bc: weight * b * c,
            bd: weight * b * d,
            c2: weight * c * c,
            cd: weight * c * d,
            d2: weight * d * d,
        }
    }

    #[inline]
    fn add(&mut self, o: &Quadric) {
        self.a2 += o.a2;
        self.ab += o.ab;
        self.ac += o.ac;
        self.ad += o.ad;
        self.b2 += o.b2;
        self.bc += o.bc;
        self.bd += o.bd;
        self.c2 += o.c2;
        self.cd += o.cd;
        self.d2 += o.d2;
    }

    #[inline]
    fn sum(a: &Quadric, b: &Quadric) -> Quadric {
        let mut q = *a;
        q.add(b);
        q
    }

    /// Evaluate `vᵀ Q v` for `v = (x, y, z, 1)`. This is the quadratic sum of
    /// squared distances of `v` to every accumulated plane. Clamped to `>= 0`
    /// (the exact form is non-negative; tiny negative values are rounding).
    #[inline]
    fn error_at(&self, x: f64, y: f64, z: f64) -> f64 {
        // vᵀ Q v with the 1 in the homogeneous slot.
        let e = self.a2 * x * x
            + 2.0 * self.ab * x * y
            + 2.0 * self.ac * x * z
            + 2.0 * self.ad * x
            + self.b2 * y * y
            + 2.0 * self.bc * y * z
            + 2.0 * self.bd * y
            + self.c2 * z * z
            + 2.0 * self.cd * z
            + self.d2;
        if e > 0.0 {
            e
        } else {
            0.0
        }
    }

    /// Solve for the error-minimizing position `v̄`. The gradient of `vᵀ Q v`
    /// vanishes where the top-left `3×3` block `A` times `(x, y, z)` equals
    /// `-(ad, bd, cd)`. Returns `None` if `A` is (near-)singular, in which case
    /// the caller falls back to endpoints / midpoint.
    fn optimal_position(&self) -> Option<(f64, f64, f64)> {
        // A = [[a2, ab, ac], [ab, b2, bc], [ac, bc, c2]]
        // b = -(ad, bd, cd)
        let a = self.a2;
        let b = self.ab;
        let c = self.ac;
        let d = self.b2;
        let e = self.bc;
        let f = self.c2;
        // Determinant of the symmetric 3×3.
        let det = a * (d * f - e * e) - b * (b * f - e * c) + c * (b * e - d * c);
        // Scale-aware singularity guard: compare against the magnitude of the
        // matrix so we do not "solve" an ill-conditioned system.
        let scale = a.abs() + b.abs() + c.abs() + d.abs() + e.abs() + f.abs();
        if !det.is_finite() || det.abs() <= 1e-12 * (scale * scale * scale).max(1e-300) {
            return None;
        }
        let rx = -self.ad;
        let ry = -self.bd;
        let rz = -self.cd;
        // Inverse of the symmetric 3×3 applied to r (Cramer / adjugate).
        let inv_det = 1.0 / det;
        // Cofactors (symmetric matrix ⇒ adjugate is symmetric).
        let c00 = d * f - e * e;
        let c01 = c * e - b * f;
        let c02 = b * e - c * d;
        let c11 = a * f - c * c;
        let c12 = b * c - a * e;
        let c22 = a * d - b * b;
        let x = inv_det * (c00 * rx + c01 * ry + c02 * rz);
        let y = inv_det * (c01 * rx + c11 * ry + c12 * rz);
        let z = inv_det * (c02 * rx + c12 * ry + c22 * rz);
        if x.is_finite() && y.is_finite() && z.is_finite() {
            Some((x, y, z))
        } else {
            None
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  Pending-collapse heap entry
// ──────────────────────────────────────────────────────────────────────────

/// A candidate edge collapse in the priority queue. Ordered so the **smallest
/// cost** pops first, with a deterministic tie-break on the canonical edge key.
#[derive(Debug, Clone, Copy)]
struct Pending {
    cost: f64,
    /// The chosen collapse target position.
    px: f64,
    py: f64,
    pz: f64,
    /// Canonical edge endpoints, `lo < hi`, in the live-vertex index space.
    lo: u32,
    hi: u32,
    /// Version stamps of both endpoints when this entry was enqueued; a stale
    /// entry (endpoint mutated since) is discarded on pop.
    ver_lo: u32,
    ver_hi: u32,
}

impl Pending {
    /// Total order used by the heap. We want a **min-heap on cost**, then a
    /// deterministic tie-break, so `BinaryHeap` (a max-heap) is fed the reverse.
    #[inline]
    fn cmp_key(&self) -> (u64, u32, u32) {
        // total_cmp-compatible bit key: map f64 to a monotonic u64 ordering.
        (f64_sort_bits(self.cost), self.lo, self.hi)
    }
}

impl PartialEq for Pending {
    fn eq(&self, other: &Self) -> bool {
        self.cmp_key() == other.cmp_key()
    }
}
impl Eq for Pending {}
impl PartialOrd for Pending {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Pending {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse so that BinaryHeap (max-heap) yields the *smallest* cost_key
        // first, with deterministic tie-break on (lo, hi).
        other.cmp_key().cmp(&self.cmp_key())
    }
}

/// Map an `f64` to a `u64` whose unsigned ordering matches IEEE total order
/// (for non-NaN finite inputs, which is all we heap on). Positive costs only in
/// practice, but this is correct for negatives too.
#[inline]
fn f64_sort_bits(x: f64) -> u64 {
    let bits = x.to_bits();
    // Flip sign bit for positives; flip all bits for negatives.
    if bits & 0x8000_0000_0000_0000 != 0 {
        !bits
    } else {
        bits | 0x8000_0000_0000_0000
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  Live mesh working state (internal, cold construction)
// ──────────────────────────────────────────────────────────────────────────

const REMOVED: u32 = u32::MAX;

struct Mesh {
    /// Vertex positions (indexed by original id; collapsed ids are abandoned).
    pos: Vec<Point3>,
    /// Accumulated quadric per vertex.
    quad: Vec<Quadric>,
    /// Version stamp per vertex, bumped on every collapse touching it.
    ver: Vec<u32>,
    /// alive[v] = false once v has been collapsed away.
    alive: Vec<bool>,
    /// Triangles as vertex-id triples; a collapsed/degenerate face has any
    /// slot set to `REMOVED`.
    tris: Vec<[u32; 3]>,
    /// For each vertex, the set of incident live-face indices (into `tris`).
    /// This is the adjacency we mutate on collapse.
    faces_of: Vec<Vec<u32>>,
    live_faces: usize,
}

impl Mesh {
    #[inline]
    fn face_is_live(&self, f: usize) -> bool {
        let t = self.tris[f];
        t[0] != REMOVED && t[1] != REMOVED && t[2] != REMOVED
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  Plane extraction
// ──────────────────────────────────────────────────────────────────────────

/// Unit-normal plane of a triangle plus its area. Returns `None` for a
/// degenerate (zero-area) triangle — such a face contributes no quadric.
#[inline]
fn face_plane(a: Point3, b: Point3, c: Point3) -> Option<(f64, f64, f64, f64, f64)> {
    let ux = b.x - a.x;
    let uy = b.y - a.y;
    let uz = b.z - a.z;
    let vx = c.x - a.x;
    let vy = c.y - a.y;
    let vz = c.z - a.z;
    let nx = uy * vz - uz * vy;
    let ny = uz * vx - ux * vz;
    let nz = ux * vy - uy * vx;
    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    if !(len > 0.0) || !len.is_finite() {
        return None;
    }
    let inv = 1.0 / len;
    let a_ = nx * inv;
    let b_ = ny * inv;
    let c_ = nz * inv;
    // Plane through a: a_*x + b_*y + c_*z + d = 0 ⇒ d = -(n·a).
    let d_ = -(a_ * a.x + b_ * a.y + c_ * a.z);
    let area = 0.5 * len;
    Some((a_, b_, c_, d_, area))
}

// ──────────────────────────────────────────────────────────────────────────
//  Public entry points
// ──────────────────────────────────────────────────────────────────────────

/// Decimate a triangle mesh with QEM edge collapses using the default
/// [`FilteredF64Kernel`] for the exact foldover-orientation guard.
///
/// See [`decimate_qem_with_kernel`] for the full contract.
pub fn decimate_qem(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    options: DecimateOptions,
    out_vertices: &mut [Point3],
    out_triangles: &mut [[u32; 3]],
) -> Result<DecimateReport, DecimateError> {
    decimate_qem_with_kernel(
        &FilteredF64Kernel::default(),
        vertices,
        triangles,
        options,
        out_vertices,
        out_triangles,
    )
}

/// Kernel-generic QEM decimation. The algorithm runs unchanged over any
/// [`GeometryKernel`]; the kernel supplies the **exact** `orient_3d` sign used
/// to reject foldover (normal-flipping) collapses.
///
/// Writes the decimated, compacted mesh into `out_vertices` / `out_triangles`
/// and returns a [`DecimateReport`]. Output vertices are the surviving vertices
/// compacted to a dense `0..vertices` range in ascending original-id order (a
/// canonical, deterministic remap); triangles reference the compacted ids.
///
/// Buffer sizing: `out_vertices` needs [`required_vertices(vertices.len())`]
/// entries and `out_triangles` needs [`required_triangles(triangles.len())`]
/// entries (decimation never grows the mesh).
pub fn decimate_qem_with_kernel<K: GeometryKernel>(
    kernel: &K,
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    options: DecimateOptions,
    out_vertices: &mut [Point3],
    out_triangles: &mut [[u32; 3]],
) -> Result<DecimateReport, DecimateError> {
    if vertices.len() > (u32::MAX as usize) {
        return Err(DecimateError::TooManyVertices);
    }
    if out_vertices.len() < vertices.len() {
        return Err(DecimateError::VertexOutputTooSmall {
            required: vertices.len(),
        });
    }
    if out_triangles.len() < triangles.len() {
        return Err(DecimateError::TriangleOutputTooSmall {
            required: triangles.len(),
        });
    }
    if options.target_faces > triangles.len() {
        return Err(DecimateError::InvalidTarget {
            target: options.target_faces,
            input_faces: triangles.len(),
        });
    }

    // ---- validate input ---------------------------------------------------
    for (i, v) in vertices.iter().enumerate() {
        if !v.x.is_finite() || !v.y.is_finite() || !v.z.is_finite() {
            return Err(DecimateError::NonFiniteCoordinate { index: i });
        }
    }
    for (t, tri) in triangles.iter().enumerate() {
        for &vi in tri {
            if vi as usize >= vertices.len() {
                return Err(DecimateError::IndexOutOfBounds {
                    triangle: t,
                    vertex: vi,
                });
            }
        }
        if tri[0] == tri[1] || tri[1] == tri[2] || tri[2] == tri[0] {
            return Err(DecimateError::DegenerateInputFace { triangle: t });
        }
    }

    // ---- build working mesh ----------------------------------------------
    let n = vertices.len();
    let mut mesh = Mesh {
        pos: vertices.to_vec(),
        quad: vec![Quadric::zero(); n],
        ver: vec![0u32; n],
        alive: vec![true; n],
        tris: triangles.to_vec(),
        faces_of: vec![Vec::new(); n],
        live_faces: triangles.len(),
    };

    // Face-adjacency + per-vertex quadric accumulation (area-weighted planes).
    for (f, tri) in triangles.iter().enumerate() {
        let a = vertices[tri[0] as usize];
        let b = vertices[tri[1] as usize];
        let c = vertices[tri[2] as usize];
        for &vi in tri {
            mesh.faces_of[vi as usize].push(f as u32);
        }
        if let Some((pa, pb, pc, pd, area)) = face_plane(a, b, c) {
            let kp = Quadric::from_plane(pa, pb, pc, pd, area);
            mesh.quad[tri[0] as usize].add(&kp);
            mesh.quad[tri[1] as usize].add(&kp);
            mesh.quad[tri[2] as usize].add(&kp);
        }
    }

    // Boundary-edge quadrics: for every unpaired directed edge, add a virtual
    // plane perpendicular to the face through the edge, weighted large so the
    // open boundary is preserved (Garland–Heckbert §4).
    add_boundary_quadrics(&mut mesh, triangles);

    // ---- seed the collapse heap ------------------------------------------
    // Deduplicate undirected edges via a set.
    use std::collections::BinaryHeap;
    let mut heap: BinaryHeap<Pending> = BinaryHeap::new();
    {
        let mut seen: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
        for tri in triangles.iter() {
            let e = [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])];
            for &(i, j) in &e {
                let (lo, hi) = if i < j { (i, j) } else { (j, i) };
                if seen.insert((lo, hi)) {
                    if let Some(p) = eval_edge(&mesh, lo, hi) {
                        heap.push(p);
                    }
                }
            }
        }
    }

    // ---- greedy collapse loop --------------------------------------------
    let mut collapses = 0usize;
    let mut max_error = 0.0f64;
    let mut stopped_on_error = false;

    while mesh.live_faces > options.target_faces {
        let Some(p) = heap.pop() else {
            break; // no candidates left
        };
        // Skip stale entries (endpoint mutated since enqueue).
        if !mesh.alive[p.lo as usize] || !mesh.alive[p.hi as usize] {
            continue;
        }
        if mesh.ver[p.lo as usize] != p.ver_lo || mesh.ver[p.hi as usize] != p.ver_hi {
            // Recompute fresh; the position/cost may have changed.
            if let Some(fresh) = eval_edge(&mesh, p.lo, p.hi) {
                heap.push(fresh);
            }
            continue;
        }
        // Error ceiling check.
        if let Some(limit) = options.max_error {
            if p.cost > limit {
                stopped_on_error = true;
                break;
            }
        }
        // Attempt the collapse (foldover + manifold guards inside).
        if try_collapse(kernel, &mut mesh, &p) {
            collapses += 1;
            if p.cost > max_error {
                max_error = p.cost;
            }
            // Re-evaluate every edge incident to the surviving vertex (hi).
            reenqueue_incident(&mesh, p.hi, &mut heap);
        }
        // If the collapse was rejected, we simply drop this entry and move on;
        // a fresh entry for this edge will not be re-created, which is correct:
        // a foldover-blocked edge stays blocked until its neighbourhood changes,
        // at which point reenqueue_incident re-adds it.
    }

    // ---- compact & emit ---------------------------------------------------
    emit(&mesh, out_vertices, out_triangles).map(|(vc, fc)| DecimateReport {
        vertices: vc,
        faces: fc,
        collapses,
        max_error,
        stopped_on_error,
    })
}

// ──────────────────────────────────────────────────────────────────────────
//  Boundary quadrics
// ──────────────────────────────────────────────────────────────────────────

fn add_boundary_quadrics(mesh: &mut Mesh, triangles: &[[u32; 3]]) {
    use std::collections::HashMap;
    // Count directed edges; an undirected edge with only one direction present
    // is a boundary edge.
    let mut dir: HashMap<(u32, u32), u32> = HashMap::new();
    for tri in triangles {
        for &(i, j) in &[(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            *dir.entry((i, j)).or_insert(0) += 1;
        }
    }
    // For each boundary edge, build a plane perpendicular to the incident face,
    // passing through the edge, weighted large.
    for tri in triangles {
        let a = mesh.pos[tri[0] as usize];
        let b = mesh.pos[tri[1] as usize];
        let c = mesh.pos[tri[2] as usize];
        let Some((fa, fb, fc, _fd, area)) = face_plane(a, b, c) else {
            continue;
        };
        let fn_ = (fa, fb, fc);
        for &(i, j) in &[(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            let is_boundary = !dir.contains_key(&(j, i));
            if !is_boundary {
                continue;
            }
            let pi = mesh.pos[i as usize];
            let pj = mesh.pos[j as usize];
            // Edge direction.
            let ex = pj.x - pi.x;
            let ey = pj.y - pi.y;
            let ez = pj.z - pi.z;
            // Plane normal = edge × face_normal (perpendicular to both).
            let nx = ey * fn_.2 - ez * fn_.1;
            let ny = ez * fn_.0 - ex * fn_.2;
            let nz = ex * fn_.1 - ey * fn_.0;
            let len = (nx * nx + ny * ny + nz * nz).sqrt();
            if !(len > 0.0) || !len.is_finite() {
                continue;
            }
            let inv = 1.0 / len;
            let (bnx, bny, bnz) = (nx * inv, ny * inv, nz * inv);
            let bd = -(bnx * pi.x + bny * pi.y + bnz * pi.z);
            // Large weight so the boundary edge is heavily penalized against
            // moving off its line (area scale keeps it comparable to faces).
            let weight = 1000.0 * area.max(1e-6);
            let kb = Quadric::from_plane(bnx, bny, bnz, bd, weight);
            mesh.quad[i as usize].add(&kb);
            mesh.quad[j as usize].add(&kb);
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  Edge evaluation
// ──────────────────────────────────────────────────────────────────────────

/// Compute the collapse candidate for undirected edge `(lo, hi)` (`lo < hi`).
/// Returns `None` if either endpoint is dead.
fn eval_edge(mesh: &Mesh, lo: u32, hi: u32) -> Option<Pending> {
    if !mesh.alive[lo as usize] || !mesh.alive[hi as usize] {
        return None;
    }
    let q = Quadric::sum(&mesh.quad[lo as usize], &mesh.quad[hi as usize]);
    let pl = mesh.pos[lo as usize];
    let ph = mesh.pos[hi as usize];
    // Candidate positions, in a fixed order so ties resolve deterministically
    // (first candidate of equal cost wins): the optimal solve (if the 3×3 is
    // non-singular), then midpoint, then each endpoint. This is the standard
    // robust QEM fallback ladder.
    let mid = (
        0.5 * (pl.x + ph.x),
        0.5 * (pl.y + ph.y),
        0.5 * (pl.z + ph.z),
    );
    let mut best: Option<(f64, f64, f64, f64)> = None;
    let consider = |best: &mut Option<(f64, f64, f64, f64)>, x: f64, y: f64, z: f64| {
        let e = q.error_at(x, y, z);
        let take = match *best {
            Some((be, ..)) => e < be,
            None => true,
        };
        if take {
            *best = Some((e, x, y, z));
        }
    };
    if let Some((x, y, z)) = q.optimal_position() {
        consider(&mut best, x, y, z);
    }
    consider(&mut best, mid.0, mid.1, mid.2);
    consider(&mut best, pl.x, pl.y, pl.z);
    consider(&mut best, ph.x, ph.y, ph.z);
    let (cost, px, py, pz) = best?;
    Some(Pending {
        cost,
        px,
        py,
        pz,
        lo,
        hi,
        ver_lo: mesh.ver[lo as usize],
        ver_hi: mesh.ver[hi as usize],
    })
}

/// Are `lo` and `hi` connected by at least one live face? Only connected edges
/// are collapsible.
fn edge_is_connected(mesh: &Mesh, lo: u32, hi: u32) -> bool {
    for &f in &mesh.faces_of[lo as usize] {
        if !mesh.face_is_live(f as usize) {
            continue;
        }
        let t = mesh.tris[f as usize];
        if t.contains(&hi) {
            return true;
        }
    }
    false
}

// ──────────────────────────────────────────────────────────────────────────
//  Collapse (with foldover + manifold guards)
// ──────────────────────────────────────────────────────────────────────────

/// Attempt to collapse edge `(lo → hi)`: move `hi` to the candidate position,
/// merge `lo` into `hi`, delete the (one or two) shared faces. Returns `true`
/// on success. Rejects (returns `false`, leaving the mesh untouched) if:
///   - the edge is no longer connected by a live face,
///   - the collapse violates the link condition (would create a non-manifold),
///   - any surviving face incident to `lo`/`hi` would flip normal (foldover).
fn try_collapse<K: GeometryKernel>(kernel: &K, mesh: &mut Mesh, p: &Pending) -> bool {
    let lo = p.lo;
    let hi = p.hi;
    if !edge_is_connected(mesh, lo, hi) {
        return false;
    }

    // Faces on the collapsed edge (shared by both endpoints) — these are removed.
    // Faces incident to lo only (or hi only) — these are rewritten/kept.
    // First, the link condition: the set of vertices adjacent to BOTH lo and hi
    // (excluding lo, hi themselves) must equal exactly the apex vertices of the
    // shared faces. Otherwise the collapse pinches the surface.
    if !link_condition_ok(mesh, lo, hi) {
        return false;
    }

    // Candidate new position for the merged vertex.
    let newp = Point3::new(p.px, p.py, p.pz);

    // Foldover test: for every live face incident to lo or hi that does NOT
    // contain the *other* endpoint (i.e. survives the collapse), verify its
    // orientation does not flip when lo→hi and hi moves to newp.
    if !foldover_ok(kernel, mesh, lo, hi, newp) {
        return false;
    }

    // ---- commit ----------------------------------------------------------
    // 1. Move hi to the new position, sum quadrics into hi.
    mesh.pos[hi as usize] = newp;
    let ql = mesh.quad[lo as usize];
    mesh.quad[hi as usize].add(&ql);

    // 2. Remove faces containing both lo and hi; rewrite lo→hi elsewhere.
    // Collect lo's faces (clone the small adjacency to iterate while mutating).
    let lo_faces: Vec<u32> = mesh.faces_of[lo as usize].clone();
    for f in lo_faces {
        let fi = f as usize;
        if !mesh.face_is_live(fi) {
            continue;
        }
        let t = mesh.tris[fi];
        if t.contains(&hi) {
            // Shared face → remove.
            mesh.tris[fi] = [REMOVED, REMOVED, REMOVED];
            mesh.live_faces -= 1;
            // Remove this face from hi's adjacency too.
            remove_face(&mut mesh.faces_of[hi as usize], f);
        } else {
            // Rewrite lo → hi in this face and reparent to hi.
            let mut nt = t;
            for s in nt.iter_mut() {
                if *s == lo {
                    *s = hi;
                }
            }
            mesh.tris[fi] = nt;
            mesh.faces_of[hi as usize].push(f);
        }
    }

    // 3. Kill lo, bump versions.
    mesh.faces_of[lo as usize].clear();
    mesh.alive[lo as usize] = false;
    mesh.ver[lo as usize] = mesh.ver[lo as usize].wrapping_add(1);
    mesh.ver[hi as usize] = mesh.ver[hi as usize].wrapping_add(1);

    // 4. Bump versions of every neighbour whose incident edge cost changed.
    // (The one-ring of hi.)
    let mut ring: Vec<u32> = Vec::new();
    for &f in &mesh.faces_of[hi as usize] {
        if !mesh.face_is_live(f as usize) {
            continue;
        }
        for &v in &mesh.tris[f as usize] {
            if v != hi && v != REMOVED {
                ring.push(v);
            }
        }
    }
    ring.sort_unstable();
    ring.dedup();
    for v in ring {
        mesh.ver[v as usize] = mesh.ver[v as usize].wrapping_add(1);
    }

    true
}

/// Remove one occurrence of `f` from an adjacency vector.
#[inline]
fn remove_face(v: &mut Vec<u32>, f: u32) {
    if let Some(pos) = v.iter().position(|&x| x == f) {
        v.swap_remove(pos);
    }
}

/// The **link condition** for a manifold edge collapse: the intersection of the
/// (live) one-ring neighbourhoods of `lo` and `hi` must be exactly the apex
/// vertices of the faces shared by the edge (one apex for a boundary edge, two
/// for an interior edge). If any *other* common neighbour exists, collapsing
/// would create a non-manifold (pinched) vertex, so the collapse is rejected.
fn link_condition_ok(mesh: &Mesh, lo: u32, hi: u32) -> bool {
    // Apex vertices of shared faces.
    let mut shared_apex: Vec<u32> = Vec::new();
    for &f in &mesh.faces_of[lo as usize] {
        if !mesh.face_is_live(f as usize) {
            continue;
        }
        let t = mesh.tris[f as usize];
        if t.contains(&hi) {
            for &v in &t {
                if v != lo && v != hi {
                    shared_apex.push(v);
                }
            }
        }
    }
    shared_apex.sort_unstable();
    shared_apex.dedup();

    // One-ring of lo (neighbours via live faces), excluding hi.
    let ring_lo = one_ring(mesh, lo, hi);
    // One-ring of hi, excluding lo.
    let ring_hi = one_ring(mesh, hi, lo);

    // Common neighbours = intersection.
    for v in &ring_lo {
        if ring_hi.binary_search(v).is_ok() {
            // v is a common neighbour; it must be a shared apex.
            if shared_apex.binary_search(v).is_err() {
                return false;
            }
        }
    }
    true
}

/// Sorted, deduped one-ring neighbour ids of `v` reachable through live faces,
/// excluding `exclude`.
fn one_ring(mesh: &Mesh, v: u32, exclude: u32) -> Vec<u32> {
    let mut ring: Vec<u32> = Vec::new();
    for &f in &mesh.faces_of[v as usize] {
        if !mesh.face_is_live(f as usize) {
            continue;
        }
        for &w in &mesh.tris[f as usize] {
            if w != v && w != exclude && w != REMOVED {
                ring.push(w);
            }
        }
    }
    ring.sort_unstable();
    ring.dedup();
    ring
}

/// Foldover guard: no face surviving the collapse may flip its orientation.
///
/// For each live face incident to `lo` or `hi` that does NOT contain the other
/// endpoint (so it survives), we compare the sign of its area-normal before and
/// after the move (lo→hi, hi→newp). A sign flip means the triangle inverted —
/// the collapse is rejected. We use the exact [`GeometryKernel::orient_3d`]
/// against the face's own apex projected out of plane to get a robust sign.
fn foldover_ok<K: GeometryKernel>(kernel: &K, mesh: &Mesh, lo: u32, hi: u32, newp: Point3) -> bool {
    // Gather candidate faces from both endpoints (dedup by face index).
    let mut faces: Vec<u32> = Vec::new();
    for &f in &mesh.faces_of[lo as usize] {
        faces.push(f);
    }
    for &f in &mesh.faces_of[hi as usize] {
        faces.push(f);
    }
    faces.sort_unstable();
    faces.dedup();

    for f in faces {
        let fi = f as usize;
        if !mesh.face_is_live(fi) {
            continue;
        }
        let t = mesh.tris[fi];
        // A face containing BOTH lo and hi is removed by the collapse → skip.
        let has_lo = t.contains(&lo);
        let has_hi = t.contains(&hi);
        if has_lo && has_hi {
            continue;
        }
        // Original corner positions.
        let orig = [
            mesh.pos[t[0] as usize],
            mesh.pos[t[1] as usize],
            mesh.pos[t[2] as usize],
        ];
        // Post-collapse positions: lo→newp and hi→newp (only one is present).
        let mut newc = orig;
        for k in 0..3 {
            if t[k] == lo || t[k] == hi {
                newc[k] = newp;
            }
        }
        // Compare the triangle-normal sign before/after by orienting each
        // configuration against a common off-plane reference apex. We use the
        // pre-collapse normal direction as the reference: build a 4th point by
        // lifting the original centroid along the original normal, and test that
        // the new triangle keeps `d` on the same side.
        if flips_normal(kernel, &orig, &newc) {
            return false;
        }
    }
    true
}

/// True if triangle `newc` is oriented oppositely to `orig` (a normal flip),
/// robustly. We lift a reference point off the *original* triangle's plane and
/// require the new triangle to keep it on the same side.
fn flips_normal<K: GeometryKernel>(kernel: &K, orig: &[Point3; 3], newc: &[Point3; 3]) -> bool {
    // Original geometric normal (direction only).
    let (nx, ny, nz) = tri_normal(orig);
    let len2 = nx * nx + ny * ny + nz * nz;
    if !(len2 > 0.0) || !len2.is_finite() {
        // Original was degenerate; nothing to preserve. Treat any new
        // non-degenerate face as acceptable (it can only improve).
        return false;
    }
    // Reference apex: centroid of the ORIGINAL triangle pushed along +normal by
    // a scale tied to the triangle size, so orient_3d has a well-separated sign.
    let cx = (orig[0].x + orig[1].x + orig[2].x) / 3.0;
    let cy = (orig[0].y + orig[1].y + orig[2].y) / 3.0;
    let cz = (orig[0].z + orig[1].z + orig[2].z) / 3.0;
    let scale = len2.sqrt().sqrt(); // ~ sqrt of edge length scale
    let apex = Point3::new(
        cx + nx * 0.5 / scale.max(1e-12),
        cy + ny * 0.5 / scale.max(1e-12),
        cz + nz * 0.5 / scale.max(1e-12),
    );

    let s_orig = kernel.orient_3d(orig[0], orig[1], orig[2], apex);
    // If the new triangle is degenerate, reject (collapsing must not create a
    // sliver we then treat as valid).
    let (mx, my, mz) = tri_normal(newc);
    if !(mx * mx + my * my + mz * mz > 0.0) {
        return true;
    }
    let s_new = kernel.orient_3d(newc[0], newc[1], newc[2], apex);
    // A flip: the same apex went from one side of the face to the other side,
    // i.e. the signs are strictly opposite. Zero (apex became coplanar) is not
    // treated as a flip on its own, but combined with the dot-product check
    // below we require a genuine directional reversal.
    use super::expansion::Sign;
    let opposite = matches!(
        (s_orig, s_new),
        (Sign::Positive, Sign::Negative) | (Sign::Negative, Sign::Positive)
    );
    if opposite {
        return true;
    }
    // Secondary robust check: the dot product of the old and new geometric
    // normals must be positive (angle < 90°). This catches large rotations that
    // the single-apex test can miss for skinny triangles.
    let dot = nx * mx + ny * my + nz * mz;
    dot < 0.0
}

#[inline]
fn tri_normal(t: &[Point3; 3]) -> (f64, f64, f64) {
    let ux = t[1].x - t[0].x;
    let uy = t[1].y - t[0].y;
    let uz = t[1].z - t[0].z;
    let vx = t[2].x - t[0].x;
    let vy = t[2].y - t[0].y;
    let vz = t[2].z - t[0].z;
    (uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx)
}

/// Re-evaluate and enqueue every edge incident to the surviving vertex `hi`.
fn reenqueue_incident(mesh: &Mesh, hi: u32, heap: &mut std::collections::BinaryHeap<Pending>) {
    let ring = one_ring(mesh, hi, REMOVED);
    for w in ring {
        let (lo, hipair) = if w < hi { (w, hi) } else { (hi, w) };
        if edge_is_connected(mesh, lo, hipair) {
            if let Some(p) = eval_edge(mesh, lo, hipair) {
                heap.push(p);
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  Emit / compaction
// ──────────────────────────────────────────────────────────────────────────

/// Compact surviving vertices to a dense range (ascending original id → new id)
/// and write live triangles remapped. Deterministic: the remap follows original
/// id order, independent of collapse order.
fn emit(
    mesh: &Mesh,
    out_vertices: &mut [Point3],
    out_triangles: &mut [[u32; 3]],
) -> Result<(usize, usize), DecimateError> {
    // Only vertices referenced by a live face survive in the output (orphans,
    // if any, are dropped). Build a remap.
    let n = mesh.pos.len();
    let mut used = vec![false; n];
    for f in 0..mesh.tris.len() {
        if !mesh.face_is_live(f) {
            continue;
        }
        for &v in &mesh.tris[f] {
            used[v as usize] = true;
        }
    }
    let mut remap = vec![REMOVED; n];
    let mut vc = 0usize;
    for v in 0..n {
        if used[v] && mesh.alive[v] {
            if vc >= out_vertices.len() {
                return Err(DecimateError::VertexOutputTooSmall { required: vc + 1 });
            }
            out_vertices[vc] = mesh.pos[v];
            remap[v] = vc as u32;
            vc += 1;
        }
    }
    let mut fc = 0usize;
    for f in 0..mesh.tris.len() {
        if !mesh.face_is_live(f) {
            continue;
        }
        let t = mesh.tris[f];
        let nt = [
            remap[t[0] as usize],
            remap[t[1] as usize],
            remap[t[2] as usize],
        ];
        // A live face's vertices are always used, so remap is valid; but guard
        // against a degenerate remap (should not happen).
        if nt[0] == REMOVED || nt[1] == REMOVED || nt[2] == REMOVED {
            continue;
        }
        if nt[0] == nt[1] || nt[1] == nt[2] || nt[2] == nt[0] {
            continue; // drop any degenerate that slipped through
        }
        if fc >= out_triangles.len() {
            return Err(DecimateError::TriangleOutputTooSmall { required: fc + 1 });
        }
        out_triangles[fc] = nt;
        fc += 1;
    }
    Ok((vc, fc))
}

// ══════════════════════════════════════════════════════════════════════════
//  Tests
// ══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::surface_mesh_processing::{
        signed_volume, surface_area,
    };
    use crate::specialized_libs::computational_geometry::topology::{
        build_triangle_half_edges, required_edge_slots, EdgeSlot, HalfEdge,
    };

    // ── mesh generators ──────────────────────────────────────────────────

    /// A closed, consistently-outward-wound unit cube (8 v, 12 t).
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
            [0, 3, 2],
            [0, 2, 1],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [3, 7, 6],
            [3, 6, 2],
            [0, 4, 7],
            [0, 7, 3],
            [1, 2, 6],
            [1, 6, 5],
        ];
        (v, t)
    }

    /// Subdivide every triangle into 4 by edge midpoints, `levels` times.
    /// Keeps a shared-vertex (welded) index list so the mesh stays closed.
    fn subdivide(
        verts: &[Point3],
        tris: &[[u32; 3]],
        levels: usize,
    ) -> (Vec<Point3>, Vec<[u32; 3]>) {
        let mut v = verts.to_vec();
        let mut t = tris.to_vec();
        for _ in 0..levels {
            let mut nt: Vec<[u32; 3]> = Vec::new();
            // midpoint cache keyed by sorted endpoint pair
            use std::collections::HashMap;
            let mut mid: HashMap<(u32, u32), u32> = HashMap::new();
            let mut midpoint = |a: u32, b: u32, v: &mut Vec<Point3>| -> u32 {
                let key = if a < b { (a, b) } else { (b, a) };
                if let Some(&m) = mid.get(&key) {
                    return m;
                }
                let pa = v[a as usize];
                let pb = v[b as usize];
                let m = v.len() as u32;
                v.push(Point3::new(
                    0.5 * (pa.x + pb.x),
                    0.5 * (pa.y + pb.y),
                    0.5 * (pa.z + pb.z),
                ));
                mid.insert(key, m);
                m
            };
            for tr in &t {
                let a = tr[0];
                let b = tr[1];
                let c = tr[2];
                let ab = midpoint(a, b, &mut v);
                let bc = midpoint(b, c, &mut v);
                let ca = midpoint(c, a, &mut v);
                nt.push([a, ab, ca]);
                nt.push([ab, b, bc]);
                nt.push([ca, bc, c]);
                nt.push([ab, bc, ca]);
            }
            t = nt;
        }
        (v, t)
    }

    /// A flat grid in the z=0 plane, `n×n` cells → `2 n²` triangles (open mesh).
    fn flat_grid(n: usize) -> (Vec<Point3>, Vec<[u32; 3]>) {
        let mut v = Vec::new();
        for j in 0..=n {
            for i in 0..=n {
                v.push(Point3::new(i as f64, j as f64, 0.0));
            }
        }
        let idx = |i: usize, j: usize| (j * (n + 1) + i) as u32;
        let mut t = Vec::new();
        for j in 0..n {
            for i in 0..n {
                let a = idx(i, j);
                let b = idx(i + 1, j);
                let c = idx(i + 1, j + 1);
                let d = idx(i, j + 1);
                t.push([a, b, c]);
                t.push([a, c, d]);
            }
        }
        (v, t)
    }

    fn is_manifold_closed(v: &[Point3], t: &[[u32; 3]]) -> (bool, u32) {
        let mut edges = vec![HalfEdge::default(); t.len() * 3];
        let slot_n = required_edge_slots(t.len());
        let mut slots = vec![EdgeSlot::default(); slot_n];
        match build_triangle_half_edges(v.len() as u32, t, &mut edges, &mut slots) {
            Ok(s) => (true, s.boundary_half_edges),
            Err(_) => (false, u32::MAX),
        }
    }

    // ── tests ─────────────────────────────────────────────────────────────

    #[test]
    fn subdivided_cube_decimates_to_target_and_stays_manifold() {
        let (v0, t0) = unit_cube();
        let (v, t) = subdivide(&v0, &t0, 2); // 12 * 16 = 192 triangles
        assert_eq!(t.len(), 192);

        let area0 = surface_area(&v, &t).unwrap();
        let vol0 = signed_volume(&v, &t).unwrap();
        assert!((area0 - 6.0).abs() < 1e-9);
        assert!((vol0 - 1.0).abs() < 1e-9);

        let target = 24usize;
        let mut ov = vec![Point3::default(); required_vertices(v.len())];
        let mut ot = vec![[0u32; 3]; required_triangles(t.len())];
        let report =
            decimate_qem(&v, &t, DecimateOptions::to_faces(target), &mut ov, &mut ot).unwrap();

        // Hit (or beat) the target.
        assert!(
            report.faces <= target,
            "faces {} should be <= target {}",
            report.faces,
            target
        );
        assert!(report.faces > 0);
        assert!(report.collapses > 0);

        let dv = &ov[..report.vertices];
        let dt = &ot[..report.faces];

        // Output is still a valid manifold, and — being a decimated closed
        // solid — still closed (no boundary).
        let (manifold, boundary) = is_manifold_closed(dv, dt);
        assert!(manifold, "decimated cube must be manifold");
        assert_eq!(boundary, 0, "decimated closed cube must stay closed");

        // Surface area is preserved to within a declared, generous bound: a
        // convex cube decimates back toward its 8-corner form, area ≈ 6.
        let area1 = surface_area(dv, dt).unwrap();
        assert!(
            (area1 - 6.0).abs() < 0.75,
            "decimated area {} should be near original 6.0",
            area1
        );

        // The measured max quadric error is finite and small for a mesh whose
        // ideal decimation is exact (the cube's corners lie on their planes).
        assert!(report.max_error.is_finite());
        assert!(
            report.max_error < 1e-6,
            "planar-consistent cube decimation should be near-zero error, got {}",
            report.max_error
        );
    }

    #[test]
    fn flat_plane_decimates_with_zero_error() {
        // A planar grid: every vertex lies in z=0, all quadrics share the plane,
        // so interior collapses cost ~0. Boundary quadrics keep the border.
        let (v, t) = flat_grid(4); // 32 triangles
        let mut ov = vec![Point3::default(); v.len()];
        let mut ot = vec![[0u32; 3]; t.len()];
        let report = decimate_qem(&v, &t, DecimateOptions::to_faces(2), &mut ov, &mut ot).unwrap();

        // Interior of a plane decimates to (near) the 2-triangle quad; error is
        // essentially zero since all removed points were coplanar.
        assert!(
            report.max_error < 1e-9,
            "planar error should vanish, got {}",
            report.max_error
        );
        // Area is exactly preserved for a coplanar decimation (the quad area).
        let area0 = surface_area(&v, &t).unwrap();
        let area1 = surface_area(&ov[..report.vertices], &ot[..report.faces]).unwrap();
        assert!(
            (area1 - area0).abs() < 1e-6,
            "coplanar decimation must preserve area exactly: {} vs {}",
            area1,
            area0
        );
    }

    #[test]
    fn stop_at_error_mode_respects_ceiling() {
        // Subdivided cube; run in error mode with a tiny ceiling. Only the
        // (near-zero-cost) coplanar collapses on each flat face should proceed;
        // collapses that round a corner (higher cost) must be refused.
        let (v0, t0) = unit_cube();
        let (v, t) = subdivide(&v0, &t0, 2);
        let mut ov = vec![Point3::default(); v.len()];
        let mut ot = vec![[0u32; 3]; t.len()];

        let ceiling = 1e-9;
        let report =
            decimate_qem(&v, &t, DecimateOptions::to_error(ceiling), &mut ov, &mut ot).unwrap();

        // Every committed collapse cost <= ceiling.
        assert!(
            report.max_error <= ceiling,
            "max_error {} must be within ceiling {}",
            report.max_error,
            ceiling
        );
        // It should have stopped because the next collapse exceeded the ceiling
        // (a cube has non-coplanar edges to round), OR run out — but with a
        // subdivided cube there are always corner edges left, so it stops on error.
        assert!(report.stopped_on_error, "should stop on the error ceiling");
        // It still made progress on the coplanar interior collapses.
        assert!(
            report.collapses > 0,
            "coplanar interior collapses should proceed"
        );
        // Output remains manifold + closed.
        let (manifold, boundary) = is_manifold_closed(&ov[..report.vertices], &ot[..report.faces]);
        assert!(manifold);
        assert_eq!(boundary, 0);
    }

    #[test]
    fn foldover_guard_preserves_orientation() {
        // After any decimation, the mesh must remain consistently oriented:
        // signed volume keeps its sign (positive, outward) — a foldover would
        // invert faces and corrupt this.
        let (v0, t0) = unit_cube();
        let (v, t) = subdivide(&v0, &t0, 2);
        let mut ov = vec![Point3::default(); v.len()];
        let mut ot = vec![[0u32; 3]; t.len()];
        let report = decimate_qem(&v, &t, DecimateOptions::to_faces(30), &mut ov, &mut ot).unwrap();
        let vol = signed_volume(&ov[..report.vertices], &ot[..report.faces]).unwrap();
        assert!(
            vol > 0.0,
            "orientation must be preserved (vol > 0), got {}",
            vol
        );
        // For a convex solid decimated toward its hull, the enclosed volume is
        // close to 1 (the cube), bounded below by the inscribed decimation.
        assert!(
            vol > 0.5 && vol <= 1.0 + 1e-9,
            "volume {} out of expected band",
            vol
        );
    }

    #[test]
    fn determinism_bit_identical() {
        let (v0, t0) = unit_cube();
        let (v, t) = subdivide(&v0, &t0, 2);

        let run = || {
            let mut ov = vec![Point3::default(); v.len()];
            let mut ot = vec![[0u32; 3]; t.len()];
            let r = decimate_qem(&v, &t, DecimateOptions::to_faces(40), &mut ov, &mut ot).unwrap();
            (r, ov, ot)
        };
        let (r1, ov1, ot1) = run();
        let (r2, ov2, ot2) = run();

        assert_eq!(r1.faces, r2.faces);
        assert_eq!(r1.vertices, r2.vertices);
        assert_eq!(r1.collapses, r2.collapses);
        assert_eq!(r1.max_error.to_bits(), r2.max_error.to_bits());
        // Bit-identical vertex coordinates.
        for i in 0..r1.vertices {
            assert_eq!(ov1[i].x.to_bits(), ov2[i].x.to_bits());
            assert_eq!(ov1[i].y.to_bits(), ov2[i].y.to_bits());
            assert_eq!(ov1[i].z.to_bits(), ov2[i].z.to_bits());
        }
        // Identical triangle indices.
        assert_eq!(&ot1[..r1.faces], &ot2[..r2.faces]);
    }

    #[test]
    fn quadric_error_is_nonnegative_and_zero_on_plane() {
        // A single face plane through the origin: points on the plane cost 0,
        // points off it cost the squared distance.
        let q = Quadric::from_plane(0.0, 0.0, 1.0, 0.0, 1.0); // z = 0 plane
        assert_eq!(q.error_at(5.0, -3.0, 0.0), 0.0);
        // Distance 2 off the plane ⇒ error = 2² = 4.
        assert!((q.error_at(0.0, 0.0, 2.0) - 4.0).abs() < 1e-12);
        // Sum of two parallel planes z=0 and z=2 at the midpoint z=1: each
        // contributes 1 ⇒ total 2.
        let q2 = Quadric::sum(&q, &Quadric::from_plane(0.0, 0.0, 1.0, -2.0, 1.0));
        assert!((q2.error_at(0.0, 0.0, 1.0) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn optimal_position_finds_plane_intersection() {
        // Three orthogonal planes x=1, y=2, z=3 ⇒ optimal minimizer is (1,2,3).
        let mut q = Quadric::from_plane(1.0, 0.0, 0.0, -1.0, 1.0);
        q.add(&Quadric::from_plane(0.0, 1.0, 0.0, -2.0, 1.0));
        q.add(&Quadric::from_plane(0.0, 0.0, 1.0, -3.0, 1.0));
        let (x, y, z) = q.optimal_position().expect("well-conditioned system");
        assert!((x - 1.0).abs() < 1e-9);
        assert!((y - 2.0).abs() < 1e-9);
        assert!((z - 3.0).abs() < 1e-9);
        // And the error at that point is ~0 (the point lies on all three planes).
        assert!(q.error_at(x, y, z) < 1e-12);
    }

    #[test]
    fn optimal_position_singular_returns_none() {
        // Two identical planes ⇒ rank-1 system ⇒ singular ⇒ None (fallback).
        let mut q = Quadric::from_plane(0.0, 0.0, 1.0, 0.0, 1.0);
        q.add(&Quadric::from_plane(0.0, 0.0, 1.0, -1.0, 1.0));
        assert!(q.optimal_position().is_none());
    }

    #[test]
    fn rejects_out_of_bounds_index() {
        let v = vec![Point3::new(0.0, 0.0, 0.0)];
        let t = vec![[0u32, 1, 2]];
        let mut ov = vec![Point3::default(); 1];
        let mut ot = vec![[0u32; 3]; 1];
        assert_eq!(
            decimate_qem(&v, &t, DecimateOptions::to_faces(0), &mut ov, &mut ot),
            Err(DecimateError::IndexOutOfBounds {
                triangle: 0,
                vertex: 1
            })
        );
    }

    #[test]
    fn rejects_non_finite_and_degenerate_and_small_buffers() {
        // Non-finite coordinate.
        let v = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(f64::NAN, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let t = vec![[0u32, 1, 2]];
        let mut ov = vec![Point3::default(); 3];
        let mut ot = vec![[0u32; 3]; 1];
        assert_eq!(
            decimate_qem(&v, &t, DecimateOptions::to_faces(1), &mut ov, &mut ot),
            Err(DecimateError::NonFiniteCoordinate { index: 1 })
        );

        // Degenerate input face.
        let v2 = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let t2 = vec![[0u32, 0, 1]];
        assert_eq!(
            decimate_qem(&v2, &t2, DecimateOptions::to_faces(1), &mut ov, &mut ot),
            Err(DecimateError::DegenerateInputFace { triangle: 0 })
        );

        // Undersized vertex buffer.
        let (vc, tc) = unit_cube();
        let mut small_v = vec![Point3::default(); vc.len() - 1];
        let mut ot2 = vec![[0u32; 3]; tc.len()];
        assert_eq!(
            decimate_qem(
                &vc,
                &tc,
                DecimateOptions::to_faces(4),
                &mut small_v,
                &mut ot2
            ),
            Err(DecimateError::VertexOutputTooSmall { required: vc.len() })
        );

        // Target exceeding input face count.
        let mut full_v = vec![Point3::default(); vc.len()];
        assert_eq!(
            decimate_qem(
                &vc,
                &tc,
                DecimateOptions::to_faces(tc.len() + 1),
                &mut full_v,
                &mut ot2
            ),
            Err(DecimateError::InvalidTarget {
                target: tc.len() + 1,
                input_faces: tc.len()
            })
        );
    }

    #[test]
    fn no_op_when_target_equals_input() {
        let (v, t) = unit_cube();
        let mut ov = vec![Point3::default(); v.len()];
        let mut ot = vec![[0u32; 3]; t.len()];
        let report =
            decimate_qem(&v, &t, DecimateOptions::to_faces(t.len()), &mut ov, &mut ot).unwrap();
        assert_eq!(report.faces, t.len());
        assert_eq!(report.collapses, 0);
        assert_eq!(report.max_error, 0.0);
        // Identity output preserves area & volume exactly.
        assert!(
            (surface_area(&ov[..report.vertices], &ot[..report.faces]).unwrap() - 6.0).abs()
                < 1e-12
        );
    }

    #[test]
    fn empty_mesh_is_ok() {
        let mut ov: [Point3; 0] = [];
        let mut ot: [[u32; 3]; 0] = [];
        let report =
            decimate_qem(&[], &[], DecimateOptions::to_faces(0), &mut ov, &mut ot).unwrap();
        assert_eq!(report.faces, 0);
        assert_eq!(report.vertices, 0);
        assert_eq!(report.collapses, 0);
    }
}
