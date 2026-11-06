//! Semantic & Bifurcation-Aware Quadric Error Metric (QEM) Mesh Simplification (§2.4.1).
//!
//! Extends Garland–Heckbert 3D surface decimation (`decimate_3`) with multi-attribute,
//! anatomical/mechanical subpart boundary, skeletal joint, affordance, and observer-bifurcation
//! sensitivity quadric penalties:
//!
//! $$Q_{\text{total}}(v) = Q_{\text{geom}}(v) + w_{\text{attr}} Q_{\text{attr}}(v)
//!   + w_{\text{sem}} Q_{\text{sem}}(v) + w_{\text{bifurc}} Q_{\text{bifurc}}(v)$$
//!
//! Enforces the `CDAG` (`SectionType::ClusterDag = 23`) hierarchical seam invariant by
//! hard-rejecting any edge collapse involving a `cluster_border_locked` vertex.

use core::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use super::decimate_3::{DecimateError, DecimateOptions, DecimateReport};
use super::expansion::Sign;
use super::kernel::{FilteredF64Kernel, GeometryKernel};
use super::primitives::Point3;

/// Per-vertex semantic, topological, and presentation attributes preserved during QEM decimation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SemanticVertexAttributes {
    pub normal: [f64; 3],
    pub uv: [f64; 2],
    /// Anatomical organ, mechanical link, or botanical branch identifier (`did:q42:` hash).
    pub subpart_tag: u32,
    /// Primary skeletal joint influence index.
    pub skeleton_joint_id: u16,
    /// True if this vertex lies on an interactive affordance trigger boundary.
    pub affordance_boundary: bool,
    /// True if this vertex lies on a locked `CDAG` meshlet cluster boundary seam.
    pub cluster_border_locked: bool,
    /// Sensitivity weight in `[0.0, 1.0]` for medical, personal, or governance bifurcation zones.
    pub bifurcation_sensitivity: f64,
}

impl Default for SemanticVertexAttributes {
    fn default() -> Self {
        Self {
            normal: [0.0, 0.0, 1.0],
            uv: [0.0, 0.0],
            subpart_tag: 0,
            skeleton_joint_id: 0,
            affordance_boundary: false,
            cluster_border_locked: false,
            bifurcation_sensitivity: 0.0,
        }
    }
}

/// Penalty weights governing semantic and bifurcation-aware QEM simplification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SemanticDecimateWeights {
    /// Weight $w_{\text{attr}}$ for normal and UV attribute deviation (default `1.0`).
    pub attribute_weight: f64,
    /// Penalty $w_{\text{sem}}$ for collapsing across distinct `subpart_tag`, `skeleton_joint_id`,
    /// or `affordance_boundary` domains (default `1e6`).
    pub semantic_boundary_penalty: f64,
    /// Weight $w_{\text{bifurc}}$ preserving triangle density in high `bifurcation_sensitivity`
    /// diagnostic/governance regions (default `1e4`).
    pub bifurcation_weight: f64,
}

impl Default for SemanticDecimateWeights {
    fn default() -> Self {
        Self {
            attribute_weight: 1.0,
            semantic_boundary_penalty: 1e6,
            bifurcation_weight: 1e4,
        }
    }
}

/// Interpolate vertex attributes at parameter `t` in `[0.0, 1.0]` along edge `(attr_lo -> attr_hi)`.
#[inline]
pub fn interpolate_semantic_attributes(
    attr_lo: &SemanticVertexAttributes,
    attr_hi: &SemanticVertexAttributes,
    t: f64,
) -> SemanticVertexAttributes {
    let t = t.clamp(0.0, 1.0);
    let omt = 1.0 - t;
    let nx = omt * attr_lo.normal[0] + t * attr_hi.normal[0];
    let ny = omt * attr_lo.normal[1] + t * attr_hi.normal[1];
    let nz = omt * attr_lo.normal[2] + t * attr_hi.normal[2];
    let nlen = (nx * nx + ny * ny + nz * nz).sqrt();
    let normal = if nlen > 1e-12 && nlen.is_finite() {
        [nx / nlen, ny / nlen, nz / nlen]
    } else {
        attr_hi.normal
    };
    let uv = [
        omt * attr_lo.uv[0] + t * attr_hi.uv[0],
        omt * attr_lo.uv[1] + t * attr_hi.uv[1],
    ];
    let pick_lo = t <= 0.5;
    SemanticVertexAttributes {
        normal,
        uv,
        subpart_tag: if pick_lo {
            attr_lo.subpart_tag
        } else {
            attr_hi.subpart_tag
        },
        skeleton_joint_id: if pick_lo {
            attr_lo.skeleton_joint_id
        } else {
            attr_hi.skeleton_joint_id
        },
        affordance_boundary: attr_lo.affordance_boundary || attr_hi.affordance_boundary,
        cluster_border_locked: attr_lo.cluster_border_locked || attr_hi.cluster_border_locked,
        bifurcation_sensitivity: attr_lo
            .bifurcation_sensitivity
            .max(attr_hi.bifurcation_sensitivity),
    }
}

/// Evaluate the extended semantic + bifurcation QEM collapse cost (Zero-Heap).
///
/// Returns `None` if either endpoint is `cluster_border_locked` or if a locked semantic
/// transition is forbidden.
#[inline]
pub fn compute_extended_collapse_cost(
    pos_lo: Point3,
    pos_hi: Point3,
    target: Point3,
    geom_cost: f64,
    attr_lo: &SemanticVertexAttributes,
    attr_hi: &SemanticVertexAttributes,
    weights: &SemanticDecimateWeights,
) -> Option<f64> {
    if attr_lo.cluster_border_locked || attr_hi.cluster_border_locked {
        return None;
    }
    let dx = pos_hi.x - pos_lo.x;
    let dy = pos_hi.y - pos_lo.y;
    let dz = pos_hi.z - pos_lo.z;
    let edge_len_sq = dx * dx + dy * dy + dz * dz;

    let d_lo_sq = (target.x - pos_lo.x).powi(2)
        + (target.y - pos_lo.y).powi(2)
        + (target.z - pos_lo.z).powi(2);
    let d_hi_sq = (target.x - pos_hi.x).powi(2)
        + (target.y - pos_hi.y).powi(2)
        + (target.z - pos_hi.z).powi(2);
    let disp_scale = (d_lo_sq + d_hi_sq).max(1e-6);

    // Attribute quadric proxy: normal + UV gradient squared across the collapsed span.
    let dn_sq = (attr_lo.normal[0] - attr_hi.normal[0]).powi(2)
        + (attr_lo.normal[1] - attr_hi.normal[1]).powi(2)
        + (attr_lo.normal[2] - attr_hi.normal[2]).powi(2);
    let duv_sq =
        (attr_lo.uv[0] - attr_hi.uv[0]).powi(2) + (attr_lo.uv[1] - attr_hi.uv[1]).powi(2);
    let q_attr = weights.attribute_weight.max(0.0) * (dn_sq + duv_sq) * disp_scale;

    // Semantic boundary penalty across distinct organ/subpart, skeletal joint, or affordance tags.
    let mut sem_transitions = 0.0f64;
    if attr_lo.subpart_tag != attr_hi.subpart_tag {
        sem_transitions += 1.0;
    }
    if attr_lo.skeleton_joint_id != attr_hi.skeleton_joint_id {
        sem_transitions += 1.0;
    }
    if attr_lo.affordance_boundary != attr_hi.affordance_boundary {
        sem_transitions += 1.0;
    }
    if sem_transitions > 0.0 && !weights.semantic_boundary_penalty.is_finite() {
        return None;
    }
    let q_sem = weights.semantic_boundary_penalty.max(0.0) * sem_transitions;

    // Bifurcation sensitivity quadric: penalizes loss of sampling density in high-sensitivity zones.
    let sens_max = attr_lo
        .bifurcation_sensitivity
        .max(attr_hi.bifurcation_sensitivity)
        .clamp(0.0, 1.0);
    let sens_diff = (attr_lo.bifurcation_sensitivity - attr_hi.bifurcation_sensitivity).abs();
    let q_bifurc = weights.bifurcation_weight.max(0.0)
        * (sens_max * (edge_len_sq + d_lo_sq + d_hi_sq) + sens_diff * sens_diff);

    let total = geom_cost.max(0.0) + q_attr + q_sem + q_bifurc;
    if total.is_finite() {
        Some(total)
    } else {
        None
    }
}

#[derive(Clone, Copy)]
struct Quadric4([f64; 10]);

impl Quadric4 {
    const fn zero() -> Self {
        Self([0.0; 10])
    }
    fn from_plane(a: f64, b: f64, c: f64, d: f64, w: f64) -> Self {
        Self([
            w * a * a,
            w * a * b,
            w * a * c,
            w * a * d,
            w * b * b,
            w * b * c,
            w * b * d,
            w * c * c,
            w * c * d,
            w * d * d,
        ])
    }
    fn add(&mut self, o: &Self) {
        for i in 0..10 {
            self.0[i] += o.0[i];
        }
    }
    fn error_at(&self, p: Point3) -> f64 {
        let q = &self.0;
        let (x, y, z) = (p.x, p.y, p.z);
        (q[0] * x * x
            + 2.0 * q[1] * x * y
            + 2.0 * q[2] * x * z
            + 2.0 * q[3] * x
            + q[4] * y * y
            + 2.0 * q[5] * y * z
            + 2.0 * q[6] * y
            + q[7] * z * z
            + 2.0 * q[8] * z
            + q[9])
            .max(0.0)
    }
}

#[derive(Clone, Copy)]
struct Candidate {
    cost: f64,
    target: Point3,
    t_param: f64,
    lo: u32,
    hi: u32,
    ver_lo: u32,
    ver_hi: u32,
}

impl Candidate {
    fn key(&self) -> (u64, u32, u32) {
        let b = self.cost.to_bits();
        let ord = if b & 0x8000_0000_0000_0000 != 0 {
            !b
        } else {
            b | 0x8000_0000_0000_0000
        };
        (ord, self.lo, self.hi)
    }
}
impl PartialEq for Candidate {
    fn eq(&self, o: &Self) -> bool {
        self.key() == o.key()
    }
}
impl Eq for Candidate {}
impl PartialOrd for Candidate {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Candidate {
    fn cmp(&self, o: &Self) -> Ordering {
        o.key().cmp(&self.key())
    }
}

const REMOVED: u32 = u32::MAX;

fn face_plane(a: Point3, b: Point3, c: Point3) -> Option<(f64, f64, f64, f64, f64)> {
    let (ux, uy, uz) = (b.x - a.x, b.y - a.y, b.z - a.z);
    let (vx, vy, vz) = (c.x - a.x, c.y - a.y, c.z - a.z);
    let (nx, ny, nz) = (uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx);
    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    if !(len > 0.0) || !len.is_finite() {
        return None;
    }
    let (pa, pb, pc) = (nx / len, ny / len, nz / len);
    Some((pa, pb, pc, -(pa * a.x + pb * a.y + pc * a.z), 0.5 * len))
}

/// Decimate a triangle mesh with semantic, cluster-seam, and bifurcation-aware QEM.
pub fn decimate_semantic_qem_3(
    vertices: &[Point3],
    attributes: &[SemanticVertexAttributes],
    triangles: &[[u32; 3]],
    options: DecimateOptions,
    weights: SemanticDecimateWeights,
    out_vertices: &mut [Point3],
    out_attributes: &mut [SemanticVertexAttributes],
    out_triangles: &mut [[u32; 3]],
) -> Result<DecimateReport, DecimateError> {
    decimate_semantic_qem_3_with_kernel(
        &FilteredF64Kernel::default(),
        vertices,
        attributes,
        triangles,
        options,
        weights,
        out_vertices,
        out_attributes,
        out_triangles,
    )
}

/// Kernel-generic semantic and bifurcation-aware QEM mesh simplification.
#[allow(clippy::too_many_arguments)]
pub fn decimate_semantic_qem_3_with_kernel<K: GeometryKernel>(
    kernel: &K,
    vertices: &[Point3],
    attributes: &[SemanticVertexAttributes],
    triangles: &[[u32; 3]],
    options: DecimateOptions,
    weights: SemanticDecimateWeights,
    out_vertices: &mut [Point3],
    out_attributes: &mut [SemanticVertexAttributes],
    out_triangles: &mut [[u32; 3]],
) -> Result<DecimateReport, DecimateError> {
    let n = vertices.len();
    if n > u32::MAX as usize {
        return Err(DecimateError::TooManyVertices);
    }
    if attributes.len() < n || out_vertices.len() < n || out_attributes.len() < n {
        return Err(DecimateError::VertexOutputTooSmall { required: n });
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
    for (i, v) in vertices.iter().enumerate() {
        if !v.x.is_finite() || !v.y.is_finite() || !v.z.is_finite() {
            return Err(DecimateError::NonFiniteCoordinate { index: i });
        }
    }
    for (t, tri) in triangles.iter().enumerate() {
        for &vi in tri {
            if vi as usize >= n {
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

    let mut pos = vertices.to_vec();
    let mut attr = attributes[..n].to_vec();
    let mut quad = vec![Quadric4::zero(); n];
    let mut ver = vec![0u32; n];
    let mut alive = vec![true; n];
    let mut tris = triangles.to_vec();
    let mut faces_of = vec![Vec::<u32>::new(); n];
    let mut live_faces = triangles.len();

    let mut dir_edges: HashMap<(u32, u32), u32> = HashMap::new();
    for (f, tri) in triangles.iter().enumerate() {
        for &vi in tri {
            faces_of[vi as usize].push(f as u32);
        }
        for &(u, v) in &[(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            *dir_edges.entry((u, v)).or_insert(0) += 1;
        }
        if let Some((pa, pb, pc, pd, area)) =
            face_plane(pos[tri[0] as usize], pos[tri[1] as usize], pos[tri[2] as usize])
        {
            let q = Quadric4::from_plane(pa, pb, pc, pd, area);
            for &vi in tri {
                quad[vi as usize].add(&q);
            }
        }
    }

    // Add boundary virtual planes for open mesh edges.
    for tri in triangles {
        let Some((fa, fb, fc, _, area)) =
            face_plane(pos[tri[0] as usize], pos[tri[1] as usize], pos[tri[2] as usize])
        else {
            continue;
        };
        for &(i, j) in &[(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            if dir_edges.contains_key(&(j, i)) {
                continue;
            }
            let (pi, pj) = (pos[i as usize], pos[j as usize]);
            let (ex, ey, ez) = (pj.x - pi.x, pj.y - pi.y, pj.z - pi.z);
            let (nx, ny, nz) = (ey * fc - ez * fb, ez * fa - ex * fc, ex * fb - ey * fa);
            let len = (nx * nx + ny * ny + nz * nz).sqrt();
            if len > 0.0 && len.is_finite() {
                let (bnx, bny, bnz) = (nx / len, ny / len, nz / len);
                let bd = -(bnx * pi.x + bny * pi.y + bnz * pi.z);
                let kb = Quadric4::from_plane(bnx, bny, bnz, bd, 1000.0 * area.max(1e-6));
                quad[i as usize].add(&kb);
                quad[j as usize].add(&kb);
            }
        }
    }

    let eval_pair = |lo: u32,
                     hi: u32,
                     pos: &[Point3],
                     attr: &[SemanticVertexAttributes],
                     quad: &[Quadric4],
                     ver: &[u32],
                     alive: &[bool]|
     -> Option<Candidate> {
        if !alive[lo as usize] || !alive[hi as usize] {
            return None;
        }
        let (pl, ph) = (pos[lo as usize], pos[hi as usize]);
        let (al, ah) = (&attr[lo as usize], &attr[hi as usize]);
        let mut qsum = quad[lo as usize];
        qsum.add(&quad[hi as usize]);
        let mid = Point3::new(0.5 * (pl.x + ph.x), 0.5 * (pl.y + ph.y), 0.5 * (pl.z + ph.z));
        let mut best: Option<(f64, Point3, f64)> = None;
        for (cand_p, t_p) in [(mid, 0.5), (pl, 0.0), (ph, 1.0)] {
            let g_err = qsum.error_at(cand_p);
            if let Some(total) =
                compute_extended_collapse_cost(pl, ph, cand_p, g_err, al, ah, &weights)
            {
                if best.map_or(true, |(b_cost, ..)| total < b_cost) {
                    best = Some((total, cand_p, t_p));
                }
            }
        }
        let (cost, target, t_param) = best?;
        Some(Candidate {
            cost,
            target,
            t_param,
            lo,
            hi,
            ver_lo: ver[lo as usize],
            ver_hi: ver[hi as usize],
        })
    };

    let mut heap = BinaryHeap::new();
    let mut seen = HashSet::new();
    for tri in triangles {
        for &(i, j) in &[(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            let (lo, hi) = if i < j { (i, j) } else { (j, i) };
            if seen.insert((lo, hi)) {
                if let Some(c) = eval_pair(lo, hi, &pos, &attr, &quad, &ver, &alive) {
                    heap.push(c);
                }
            }
        }
    }

    let mut collapses = 0usize;
    let mut max_error = 0.0f64;
    let mut stopped_on_error = false;

    while live_faces > options.target_faces {
        let Some(c) = heap.pop() else { break };
        let (lo, hi) = (c.lo, c.hi);
        if !alive[lo as usize] || !alive[hi as usize] {
            continue;
        }
        if ver[lo as usize] != c.ver_lo || ver[hi as usize] != c.ver_hi {
            if let Some(fresh) = eval_pair(lo, hi, &pos, &attr, &quad, &ver, &alive) {
                heap.push(fresh);
            }
            continue;
        }
        if let Some(limit) = options.max_error {
            if c.cost > limit {
                stopped_on_error = true;
                break;
            }
        }
        if !can_collapse(kernel, lo, hi, c.target, &pos, &tris, &faces_of) {
            continue;
        }

        pos[hi as usize] = c.target;
        attr[hi as usize] =
            interpolate_semantic_attributes(&attr[lo as usize], &attr[hi as usize], c.t_param);
        let ql = quad[lo as usize];
        quad[hi as usize].add(&ql);

        let lo_faces = faces_of[lo as usize].clone();
        for f in lo_faces {
            let fi = f as usize;
            let t = tris[fi];
            if t[0] == REMOVED {
                continue;
            }
            if t.contains(&hi) {
                tris[fi] = [REMOVED; 3];
                live_faces -= 1;
                if let Some(idx) = faces_of[hi as usize].iter().position(|&x| x == f) {
                    faces_of[hi as usize].swap_remove(idx);
                }
            } else {
                for s in tris[fi].iter_mut() {
                    if *s == lo {
                        *s = hi;
                    }
                }
                faces_of[hi as usize].push(f);
            }
        }
        faces_of[lo as usize].clear();
        alive[lo as usize] = false;
        ver[lo as usize] = ver[lo as usize].wrapping_add(1);
        ver[hi as usize] = ver[hi as usize].wrapping_add(1);

        let ring = one_ring(hi, REMOVED, &tris, &faces_of);
        for &w in &ring {
            ver[w as usize] = ver[w as usize].wrapping_add(1);
        }
        for &w in &ring {
            let (a, b) = if w < hi { (w, hi) } else { (hi, w) };
            if let Some(next_c) = eval_pair(a, b, &pos, &attr, &quad, &ver, &alive) {
                heap.push(next_c);
            }
        }
        collapses += 1;
        if c.cost > max_error {
            max_error = c.cost;
        }
    }

    let mut used = vec![false; n];
    for t in &tris {
        if t[0] != REMOVED {
            for &v in t {
                used[v as usize] = true;
            }
        }
    }
    let mut remap = vec![REMOVED; n];
    let mut vc = 0usize;
    for v in 0..n {
        if used[v] && alive[v] {
            out_vertices[vc] = pos[v];
            out_attributes[vc] = attr[v];
            remap[v] = vc as u32;
            vc += 1;
        }
    }
    let mut fc = 0usize;
    for t in &tris {
        if t[0] != REMOVED {
            let nt = [remap[t[0] as usize], remap[t[1] as usize], remap[t[2] as usize]];
            if nt[0] != nt[1] && nt[1] != nt[2] && nt[2] != nt[0] {
                out_triangles[fc] = nt;
                fc += 1;
            }
        }
    }
    Ok(DecimateReport {
        vertices: vc,
        faces: fc,
        collapses,
        max_error,
        stopped_on_error,
    })
}

fn one_ring(v: u32, exclude: u32, tris: &[[u32; 3]], faces_of: &[Vec<u32>]) -> Vec<u32> {
    let mut ring = Vec::new();
    for &f in &faces_of[v as usize] {
        let t = tris[f as usize];
        if t[0] == REMOVED {
            continue;
        }
        for &w in &t {
            if w != v && w != exclude && w != REMOVED {
                ring.push(w);
            }
        }
    }
    ring.sort_unstable();
    ring.dedup();
    ring
}

fn can_collapse<K: GeometryKernel>(
    kernel: &K,
    lo: u32,
    hi: u32,
    newp: Point3,
    pos: &[Point3],
    tris: &[[u32; 3]],
    faces_of: &[Vec<u32>],
) -> bool {
    let mut shared_apex = Vec::new();
    let mut connected = false;
    for &f in &faces_of[lo as usize] {
        let t = tris[f as usize];
        if t[0] != REMOVED && t.contains(&hi) {
            connected = true;
            for &v in &t {
                if v != lo && v != hi {
                    shared_apex.push(v);
                }
            }
        }
    }
    if !connected {
        return false;
    }
    shared_apex.sort_unstable();
    shared_apex.dedup();
    let r_lo = one_ring(lo, hi, tris, faces_of);
    let r_hi = one_ring(hi, lo, tris, faces_of);
    for v in &r_lo {
        if r_hi.binary_search(v).is_ok() && shared_apex.binary_search(v).is_err() {
            return false;
        }
    }
    for &f in faces_of[lo as usize].iter().chain(faces_of[hi as usize].iter()) {
        let t = tris[f as usize];
        if t[0] == REMOVED || (t.contains(&lo) && t.contains(&hi)) {
            continue;
        }
        let orig = [pos[t[0] as usize], pos[t[1] as usize], pos[t[2] as usize]];
        let mut newc = orig;
        for k in 0..3 {
            if t[k] == lo || t[k] == hi {
                newc[k] = newp;
            }
        }
        let Some((nx, ny, nz, _, area0)) = face_plane(orig[0], orig[1], orig[2]) else {
            continue;
        };
        let Some((mx, my, mz, _, _)) = face_plane(newc[0], newc[1], newc[2]) else {
            return false;
        };
        if nx * mx + ny * my + nz * mz <= 0.0 {
            return false;
        }
        let apex = Point3::new(
            (orig[0].x + orig[1].x + orig[2].x) / 3.0 + nx * area0.sqrt().max(1e-6),
            (orig[0].y + orig[1].y + orig[2].y) / 3.0 + ny * area0.sqrt().max(1e-6),
            (orig[0].z + orig[1].z + orig[2].z) / 3.0 + nz * area0.sqrt().max(1e-6),
        );
        let s0 = kernel.orient_3d(orig[0], orig[1], orig[2], apex);
        let s1 = kernel.orient_3d(newc[0], newc[1], newc[2], apex);
        if matches!(
            (s0, s1),
            (Sign::Positive, Sign::Negative) | (Sign::Negative, Sign::Positive)
        ) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn planar_grid(nx: usize, ny: usize) -> (Vec<Point3>, Vec<[u32; 3]>) {
        let mut v = Vec::new();
        for j in 0..=ny {
            for i in 0..=nx {
                v.push(Point3::new(i as f64, j as f64, 0.0));
            }
        }
        let idx = |i: usize, j: usize| (j * (nx + 1) + i) as u32;
        let mut t = Vec::new();
        for j in 0..ny {
            for i in 0..nx {
                let (a, b, c, d) = (idx(i, j), idx(i + 1, j), idx(i + 1, j + 1), idx(i, j + 1));
                t.push([a, b, c]);
                t.push([a, c, d]);
            }
        }
        (v, t)
    }

    #[test]
    fn cluster_border_locked_vertices_are_never_collapsed() {
        let (v, t) = planar_grid(4, 4);
        let attrs: Vec<SemanticVertexAttributes> = v
            .iter()
            .map(|p| SemanticVertexAttributes {
                cluster_border_locked: p.x == 0.0 || p.x == 4.0 || p.y == 0.0 || p.y == 4.0,
                ..Default::default()
            })
            .collect();
        let locked_count = attrs.iter().filter(|a| a.cluster_border_locked).count();
        let mut ov = vec![Point3::default(); v.len()];
        let mut oa = vec![SemanticVertexAttributes::default(); v.len()];
        let mut ot = vec![[0u32; 3]; t.len()];
        let rep = decimate_semantic_qem_3(
            &v,
            &attrs,
            &t,
            DecimateOptions::to_faces(16),
            SemanticDecimateWeights::default(),
            &mut ov,
            &mut oa,
            &mut ot,
        )
        .unwrap();
        assert!(rep.collapses > 0);
        let surviving_locked = oa[..rep.vertices]
            .iter()
            .filter(|a| a.cluster_border_locked)
            .count();
        assert_eq!(surviving_locked, locked_count);
    }

    #[test]
    fn cross_organ_subpart_mismatch_preserves_anatomical_boundary() {
        let (v, t) = planar_grid(4, 4);
        let attrs: Vec<SemanticVertexAttributes> = v
            .iter()
            .map(|p| SemanticVertexAttributes {
                subpart_tag: if p.x <= 2.0 { 10 } else { 20 },
                ..Default::default()
            })
            .collect();
        let mut ov = vec![Point3::default(); v.len()];
        let mut oa = vec![SemanticVertexAttributes::default(); v.len()];
        let mut ot = vec![[0u32; 3]; t.len()];
        let rep = decimate_semantic_qem_3(
            &v,
            &attrs,
            &t,
            DecimateOptions::to_error(100.0),
            SemanticDecimateWeights::default(),
            &mut ov,
            &mut oa,
            &mut ot,
        )
        .unwrap();
        assert!(rep.collapses > 0);
        assert!(rep.stopped_on_error);
        let has_organ_10 = oa[..rep.vertices].iter().any(|a| a.subpart_tag == 10);
        let has_organ_20 = oa[..rep.vertices].iter().any(|a| a.subpart_tag == 20);
        assert!(has_organ_10 && has_organ_20);
    }

    #[test]
    fn high_bifurcation_sensitivity_retains_higher_triangle_density() {
        let (v, t) = planar_grid(6, 4);
        let attrs: Vec<SemanticVertexAttributes> = v
            .iter()
            .map(|p| SemanticVertexAttributes {
                bifurcation_sensitivity: if p.x >= 3.0 { 1.0 } else { 0.0 },
                ..Default::default()
            })
            .collect();
        let mut ov = vec![Point3::default(); v.len()];
        let mut oa = vec![SemanticVertexAttributes::default(); v.len()];
        let mut ot = vec![[0u32; 3]; t.len()];
        let rep = decimate_semantic_qem_3(
            &v,
            &attrs,
            &t,
            DecimateOptions::to_error(100.0),
            SemanticDecimateWeights::default(),
            &mut ov,
            &mut oa,
            &mut ot,
        )
        .unwrap();
        let (mut low_tris, mut high_tris) = (0usize, 0usize);
        for tri in &ot[..rep.faces] {
            let cx = (ov[tri[0] as usize].x + ov[tri[1] as usize].x + ov[tri[2] as usize].x) / 3.0;
            if cx >= 3.0 {
                high_tris += 1;
            } else {
                low_tris += 1;
            }
        }
        assert!(high_tris > low_tris, "high={high_tris} <= low={low_tris}");
    }
}
