//! Typed WGSL geometry kernels with deterministic CPU oracles.
//!
//! These kernels complement WGSL Forge without pretending that branch-heavy
//! exact topology edits belong on the GPU. Parallel broad phases run here;
//! uncertain predicates are explicitly returned to the robust CPU/WASM path.
//!
//! ## P1.9 — orient3d and incircle GPU batches
//!
//! Added `Orient3dF32` and `IncircleF32` kernels, each with:
//! - A WGSL shader that computes the filtered determinant in f32 and flags
//!   `GPU_ORIENTATION_UNCERTAIN` when near the error bound.
//! - A CPU/WASM oracle (`evaluate_orient3d_batch_f32`, `evaluate_incircle_batch_f32`)
//!   that runs the full filtered → compensated → exact ladder.
//!
//! The GPU is the fast broad phase; uncertain lanes fall back to the CPU
//! oracle's exact path. On the no-adapter path, the CPU oracle is the
//! deterministic fallback (identical results, no GPU needed).

use super::box_join::BoxPair;
use super::expansion::Sign;
use super::{incircle, orient_3d, orientation_2, Aabb, Point2, Point3};

pub const GPU_ORIENTATION_UNCERTAIN: i32 = 2;

/// GPU candidate-generation result flag: the pair is a definite overlap.
pub const GPU_OVERLAP_YES: i32 = 1;
/// GPU candidate-generation result flag: the pair is definitely not overlapping.
pub const GPU_OVERLAP_NO: i32 = 0;
/// GPU candidate-generation result flag: uncertain (near-boundary), needs CPU verification.
pub const GPU_OVERLAP_UNCERTAIN: i32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryGpuKernel {
    /// One invocation per packed `(a,b,c)` f32 point triple (6 f32s).
    Orientation2F32,
    /// One invocation per packed `(a,b,c,d)` f32 point quadruple (12 f32s).
    /// P1.9.
    Orient3dF32,
    /// One invocation per packed `(a,b,c,d)` f32 point quadruple (8 f32s).
    /// P1.9.
    IncircleF32,
    /// One invocation per AABB pair (12 f32s: amin[3], amax[3], bmin[3], bmax[3]).
    /// P3.6: GPU broad-phase overlap test with filtered error bound.
    AabbOverlapF32,
    /// One invocation per point-AABB pair (6 f32s: point[3], amin[3]) + 3 f32s (amax[3]).
    /// P3.6: GPU point-to-AABB distance squared for NN candidate filtering.
    PointAabbDistSqF32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeometryGpuSchedule {
    pub workgroup_size: u32,
}

impl Default for GeometryGpuSchedule {
    fn default() -> Self {
        Self {
            workgroup_size: 128,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryGpuError {
    InvalidWorkgroupSize,
    InputLengthNotMultipleOfSix,
    InputLengthNotMultipleOfEight,
    InputLengthNotMultipleOfTwelve,
    InputLengthNotMultipleOfTwentyFour,
    OutputTooSmall {
        required: usize,
    },
    /// Candidate buffer too small for GPU-generated candidates.
    CandidateBufferTooSmall {
        required: usize,
    },
}

/// Deterministically emit a typed computational-geometry shader.
pub fn emit_geometry_wgsl(
    kernel: GeometryGpuKernel,
    schedule: GeometryGpuSchedule,
) -> Result<String, GeometryGpuError> {
    if !(32..=256).contains(&schedule.workgroup_size) || !schedule.workgroup_size.is_power_of_two()
    {
        return Err(GeometryGpuError::InvalidWorkgroupSize);
    }
    match kernel {
        GeometryGpuKernel::Orientation2F32 => Ok(format!(
            r#"// QualiaDB computational geometry: orientation_2_f32 v1
struct Params {{
    triple_count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}}

@group(0) @binding(0)
var<storage, read> points: array<f32>;

@group(0) @binding(1)
var<storage, read_write> orientation: array<i32>;

@group(0) @binding(2)
var<uniform> params: Params;

@compute @workgroup_size({workgroup_size}, 1, 1)
fn orientation_2_f32(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let i = gid.x;
    if (i >= params.triple_count) {{
        return;
    }}
    let base = i * 6u;
    let ax = points[base];
    let ay = points[base + 1u];
    let bx = points[base + 2u];
    let by = points[base + 3u];
    let cx = points[base + 4u];
    let cy = points[base + 5u];
    let left = (ax - cx) * (by - cy);
    let right = (ay - cy) * (bx - cx);
    let det = left - right;
    // Filter only. Near-degenerate triples are resolved by the exact CPU/WASM oracle.
    let error_bound = (abs(left) + abs(right)) * 9.5367431640625e-7;
    if (abs(det) <= error_bound) {{
        orientation[i] = {uncertain};
    }} else if (det > 0.0) {{
        orientation[i] = 1;
    }} else {{
        orientation[i] = -1;
    }}
}}
"#,
            workgroup_size = schedule.workgroup_size,
            uncertain = GPU_ORIENTATION_UNCERTAIN,
        )),

        GeometryGpuKernel::Orient3dF32 => Ok(format!(
            r#"// QualiaDB computational geometry: orient_3d_f32 v1
struct Params {{
    quad_count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}}

@group(0) @binding(0)
var<storage, read> points: array<f32>;

@group(0) @binding(1)
var<storage, read_write> orient: array<i32>;

@group(0) @binding(2)
var<uniform> params: Params;

@compute @workgroup_size({workgroup_size}, 1, 1)
fn orient_3d_f32(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let i = gid.x;
    if (i >= params.quad_count) {{
        return;
    }}
    let base = i * 12u;
    let ax = points[base];
    let ay = points[base + 1u];
    let az = points[base + 2u];
    let bx = points[base + 3u];
    let by = points[base + 4u];
    let bz = points[base + 5u];
    let cx = points[base + 6u];
    let cy = points[base + 7u];
    let cz = points[base + 8u];
    let dx = points[base + 9u];
    let dy = points[base + 10u];
    let dz = points[base + 11u];
    // det = (b-a) · ((c-a) × (d-a))
    let abx = bx - ax;
    let aby = by - ay;
    let abz = bz - az;
    let acx = cx - ax;
    let acy = cy - ay;
    let acz = cz - az;
    let adx = dx - ax;
    let ady = dy - ay;
    let adz = dz - az;
    // cross = (c-a) × (d-a)
    let cx0 = acy * adz - acz * ady;
    let cy0 = acz * adx - acx * adz;
    let cz0 = acx * ady - acy * adx;
    // dot = (b-a) · cross
    let det = abx * cx0 + aby * cy0 + abz * cz0;
    // Filtered error bound: sum of absolute products × f32 epsilon.
    let perm = abs(abx) * (abs(acy) * abs(adz) + abs(acz) * abs(ady))
             + abs(aby) * (abs(acx) * abs(adz) + abs(acz) * abs(adx))
             + abs(abz) * (abs(acx) * abs(ady) + abs(acy) * abs(adx));
    let error_bound = perm * 1.52587890625e-5;
    if (abs(det) <= error_bound) {{
        orient[i] = {uncertain};
    }} else if (det > 0.0) {{
        orient[i] = 1;
    }} else {{
        orient[i] = -1;
    }}
}}
"#,
            workgroup_size = schedule.workgroup_size,
            uncertain = GPU_ORIENTATION_UNCERTAIN,
        )),

        GeometryGpuKernel::IncircleF32 => Ok(format!(
            r#"// QualiaDB computational geometry: incircle_f32 v1
struct Params {{
    quad_count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}}

@group(0) @binding(0)
var<storage, read> points: array<f32>;

@group(0) @binding(1)
var<storage, read_write> incircle_out: array<i32>;

@group(0) @binding(2)
var<uniform> params: Params;

@compute @workgroup_size({workgroup_size}, 1, 1)
fn incircle_f32(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let i = gid.x;
    if (i >= params.quad_count) {{
        return;
    }}
    let base = i * 8u;
    let ax = points[base];
    let ay = points[base + 1u];
    let bx = points[base + 2u];
    let by = points[base + 3u];
    let cx = points[base + 4u];
    let cy = points[base + 5u];
    let dx = points[base + 6u];
    let dy = points[base + 7u];
    // Translate by d
    let adx = ax - dx;
    let ady = ay - dy;
    let bdx = bx - dx;
    let bdy = by - dy;
    let cdx = cx - dx;
    let cdy = cy - dy;
    let ad2 = adx * adx + ady * ady;
    let bd2 = bdx * bdx + bdy * bdy;
    let cd2 = cdx * cdx + cdy * cdy;
    // det = adx*(bdy*cd2 - cdy*bd2) - ady*(bdx*cd2 - cdx*bd2) + ad2*(bdx*cdy - bdy*cdx)
    let m1 = bdy * cd2 - cdy * bd2;
    let m2 = bdx * cd2 - cdx * bd2;
    let m3 = bdx * cdy - bdy * cdx;
    let det = adx * m1 - ady * m2 + ad2 * m3;
    // Filtered error bound
    let perm = abs(adx) * (abs(bdy) * abs(cd2) + abs(cdy) * abs(bd2))
             + abs(ady) * (abs(bdx) * abs(cd2) + abs(cdx) * abs(bd2))
             + abs(ad2) * (abs(bdx) * abs(cdy) + abs(bdy) * abs(cdx));
    let error_bound = perm * 3.0517578125e-5;
    if (abs(det) <= error_bound) {{
        incircle_out[i] = {uncertain};
    }} else if (det > 0.0) {{
        incircle_out[i] = 1;
    }} else {{
        incircle_out[i] = -1;
    }}
}}
"#,
            workgroup_size = schedule.workgroup_size,
            uncertain = GPU_ORIENTATION_UNCERTAIN,
        )),

        GeometryGpuKernel::AabbOverlapF32 => Ok(format!(
            r#"// QualiaDB computational geometry: aabb_overlap_f32 v1
struct Params {{
    pair_count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}}

@group(0) @binding(0)
var<storage, read> pairs: array<f32>;

@group(0) @binding(1)
var<storage, read_write> overlap_out: array<i32>;

@group(0) @binding(2)
var<uniform> params: Params;

@compute @workgroup_size({workgroup_size}, 1, 1)
fn aabb_overlap_f32(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let i = gid.x;
    if (i >= params.pair_count) {{
        return;
    }}
    let base = i * 12u;
    let aminx = pairs[base];      let aminy = pairs[base + 1u];  let aminz = pairs[base + 2u];
    let amaxx = pairs[base + 3u]; let amaxy = pairs[base + 4u];  let amaxz = pairs[base + 5u];
    let bminx = pairs[base + 6u]; let bminy = pairs[base + 7u];  let bminz = pairs[base + 8u];
    let bmaxx = pairs[base + 9u]; let bmaxy = pairs[base + 10u]; let bmaxz = pairs[base + 11u];
    // Overlap test: amin <= bmax && bmin <= amax on all axes.
    let dx1 = bmaxx - aminx;
    let dx2 = amaxx - bminx;
    let dy1 = bmaxy - aminy;
    let dy2 = amaxy - bminy;
    let dz1 = bmaxz - aminz;
    let dz2 = amaxz - bminz;
    // Filtered: if any gap is near zero, flag uncertain.
    let eps = 1.0e-5;
    let min_gap = min(min(min(dx1, dx2), min(dy1, dy2)), min(dz1, dz2));
    if (min_gap < 0.0) {{
        overlap_out[i] = {overlap_no};
    }} else if (min_gap < eps) {{
        overlap_out[i] = {overlap_uncertain};
    }} else {{
        overlap_out[i] = {overlap_yes};
    }}
}}
"#,
            workgroup_size = schedule.workgroup_size,
            overlap_no = GPU_OVERLAP_NO,
            overlap_uncertain = GPU_OVERLAP_UNCERTAIN,
            overlap_yes = GPU_OVERLAP_YES,
        )),

        GeometryGpuKernel::PointAabbDistSqF32 => Ok(format!(
            r#"// QualiaDB computational geometry: point_aabb_dist_sq_f32 v1
struct Params {{
    query_count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}}

@group(0) @binding(0)
var<storage, read> data: array<f32>;

@group(0) @binding(1)
var<storage, read_write> dist_sq_out: array<f32>;

@group(0) @binding(2)
var<uniform> params: Params;

@compute @workgroup_size({workgroup_size}, 1, 1)
fn point_aabb_dist_sq_f32(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let i = gid.x;
    if (i >= params.query_count) {{
        return;
    }}
    let base = i * 6u;
    let px = data[base];     let py = data[base + 1u]; let pz = data[base + 2u];
    let minx = data[base + 3u]; let miny = data[base + 4u]; let minz = data[base + 5u];
    // Clamped distance: max(0, min - p) and max(0, p - max) per axis.
    // We only have min here; for a proper distance we need max too.
    // This kernel is a broad-phase filter: it computes the distance to
    // the AABB's min corner as a lower bound. The CPU merge does the
    // exact point-to-AABB distance.
    let dx = max(0.0, minx - px);
    let dy = max(0.0, miny - py);
    let dz = max(0.0, minz - pz);
    dist_sq_out[i] = dx * dx + dy * dy + dz * dz;
}}
"#,
            workgroup_size = schedule.workgroup_size,
        )),
    }
}

/// CPU/WASM oracle for packed `(ax, ay, bx, by, cx, cy)` f32 triples.
pub fn evaluate_orientation_batch_f32(
    packed: &[f32],
    out: &mut [i8],
) -> Result<usize, GeometryGpuError> {
    if packed.len() % 6 != 0 {
        return Err(GeometryGpuError::InputLengthNotMultipleOfSix);
    }
    let count = packed.len() / 6;
    if out.len() < count {
        return Err(GeometryGpuError::OutputTooSmall { required: count });
    }
    for (index, triple) in packed.chunks_exact(6).enumerate() {
        out[index] = orientation_2(
            Point2::new(triple[0] as f64, triple[1] as f64),
            Point2::new(triple[2] as f64, triple[3] as f64),
            Point2::new(triple[4] as f64, triple[5] as f64),
        ) as i8;
    }
    Ok(count)
}

/// CPU/WASM oracle for packed `(ax,ay,az, bx,by,bz, cx,cy,cz, dx,dy,dz)` f32
/// quadruples (12 f32s per quad). Runs the full filtered → compensated → exact
/// ladder via [`orient_3d`].
///
/// P1.9.
pub fn evaluate_orient3d_batch_f32(
    packed: &[f32],
    out: &mut [i8],
) -> Result<usize, GeometryGpuError> {
    if packed.len() % 12 != 0 {
        return Err(GeometryGpuError::InputLengthNotMultipleOfTwelve);
    }
    let count = packed.len() / 12;
    if out.len() < count {
        return Err(GeometryGpuError::OutputTooSmall { required: count });
    }
    for (index, quad) in packed.chunks_exact(12).enumerate() {
        let s = orient_3d(
            Point3::new(quad[0] as f64, quad[1] as f64, quad[2] as f64),
            Point3::new(quad[3] as f64, quad[4] as f64, quad[5] as f64),
            Point3::new(quad[6] as f64, quad[7] as f64, quad[8] as f64),
            Point3::new(quad[9] as f64, quad[10] as f64, quad[11] as f64),
        );
        out[index] = sign_to_i8(s);
    }
    Ok(count)
}

/// CPU/WASM oracle for packed `(ax,ay, bx,by, cx,cy, dx,dy)` f32 quadruples
/// (8 f32s per quad). Runs the full filtered → compensated → exact ladder via
/// [`incircle`].
///
/// P1.9.
pub fn evaluate_incircle_batch_f32(
    packed: &[f32],
    out: &mut [i8],
) -> Result<usize, GeometryGpuError> {
    if packed.len() % 8 != 0 {
        return Err(GeometryGpuError::InputLengthNotMultipleOfEight);
    }
    let count = packed.len() / 8;
    if out.len() < count {
        return Err(GeometryGpuError::OutputTooSmall { required: count });
    }
    for (index, quad) in packed.chunks_exact(8).enumerate() {
        let s = incircle(
            Point2::new(quad[0] as f64, quad[1] as f64),
            Point2::new(quad[2] as f64, quad[3] as f64),
            Point2::new(quad[4] as f64, quad[5] as f64),
            Point2::new(quad[6] as f64, quad[7] as f64),
        );
        out[index] = sign_to_i8(s);
    }
    Ok(count)
}

// ── P3.6: AABB overlap CPU oracle + merge ────────────────────────────────

/// CPU/WASM oracle for packed AABB pairs (12 f32s per pair).
/// Returns exact overlap results: 1 = overlap, 0 = no overlap.
pub fn evaluate_aabb_overlap_batch_f32(
    packed: &[f32],
    out: &mut [i32],
) -> Result<usize, GeometryGpuError> {
    if packed.len() % 12 != 0 {
        return Err(GeometryGpuError::InputLengthNotMultipleOfTwelve);
    }
    let count = packed.len() / 12;
    if out.len() < count {
        return Err(GeometryGpuError::OutputTooSmall { required: count });
    }
    for (index, pair) in packed.chunks_exact(12).enumerate() {
        let a = Aabb::new(
            Point3::new(pair[0] as f64, pair[1] as f64, pair[2] as f64),
            Point3::new(pair[3] as f64, pair[4] as f64, pair[5] as f64),
        );
        let b = Aabb::new(
            Point3::new(pair[6] as f64, pair[7] as f64, pair[8] as f64),
            Point3::new(pair[9] as f64, pair[10] as f64, pair[11] as f64),
        );
        out[index] = if a.overlaps(&b) {
            GPU_OVERLAP_YES
        } else {
            GPU_OVERLAP_NO
        };
    }
    Ok(count)
}

/// GPU filter simulation for AABB overlap (f32). Returns YES/NO/UNCERTAIN.
fn gpu_filter_aabb_overlap_f32(pair: &[f32]) -> i32 {
    let aminx = pair[0] as f64;
    let aminy = pair[1] as f64;
    let aminz = pair[2] as f64;
    let amaxx = pair[3] as f64;
    let amaxy = pair[4] as f64;
    let amaxz = pair[5] as f64;
    let bminx = pair[6] as f64;
    let bminy = pair[7] as f64;
    let bminz = pair[8] as f64;
    let bmaxx = pair[9] as f64;
    let bmaxy = pair[10] as f64;
    let bmaxz = pair[11] as f64;

    let dx1 = bmaxx - aminx;
    let dx2 = amaxx - bminx;
    let dy1 = bmaxy - aminy;
    let dy2 = amaxy - bminy;
    let dz1 = bmaxz - aminz;
    let dz2 = amaxz - bminz;

    let min_gap = dx1.min(dx2).min(dy1).min(dy2).min(dz1).min(dz2);
    let eps = 1.0e-5;

    if min_gap < 0.0 {
        GPU_OVERLAP_NO
    } else if min_gap < eps {
        GPU_OVERLAP_UNCERTAIN
    } else {
        GPU_OVERLAP_YES
    }
}

/// Deterministic merge: combine GPU candidate results with CPU verification.
///
/// GPU-certain YES/NO lanes are trusted. GPU-uncertain lanes are resolved
/// by the CPU exact oracle. The output is deterministic: identical for
/// GPU-on vs GPU-off (the merge guarantee).
///
/// `gpu_results` is the output of the GPU kernel (or the CPU simulation).
/// `packed` is the original packed AABB pair data.
/// `out` receives the final verified overlap results.
pub fn merge_aabb_overlap_results(
    packed: &[f32],
    gpu_results: &[i32],
    out: &mut [i32],
) -> Result<usize, GeometryGpuError> {
    let count = gpu_results.len();
    if packed.len() < count * 12 {
        return Err(GeometryGpuError::InputLengthNotMultipleOfTwelve);
    }
    if out.len() < count {
        return Err(GeometryGpuError::OutputTooSmall { required: count });
    }
    for i in 0..count {
        if gpu_results[i] == GPU_OVERLAP_UNCERTAIN {
            // CPU exact resolution for uncertain lanes.
            let pair = &packed[i * 12..(i + 1) * 12];
            let a = Aabb::new(
                Point3::new(pair[0] as f64, pair[1] as f64, pair[2] as f64),
                Point3::new(pair[3] as f64, pair[4] as f64, pair[5] as f64),
            );
            let b = Aabb::new(
                Point3::new(pair[6] as f64, pair[7] as f64, pair[8] as f64),
                Point3::new(pair[9] as f64, pair[10] as f64, pair[11] as f64),
            );
            out[i] = if a.overlaps(&b) {
                GPU_OVERLAP_YES
            } else {
                GPU_OVERLAP_NO
            };
        } else {
            // Trust GPU-certain lanes.
            out[i] = gpu_results[i];
        }
    }
    Ok(count)
}

// ── P3.6: BVH overlap candidate generation + CPU merge ───────────────────

/// Generate candidate pairs from a BVH overlap query using GPU-style
/// broad-phase filtering, then merge with CPU exact verification.
///
/// This is the deterministic CPU path that produces results identical
/// to the GPU path. When a GPU adapter is available, the GPU kernel
/// generates the candidate set; this merge function verifies uncertain
/// lanes. Without a GPU, this function does the full computation.
///
/// `boxes_a` / `boxes_b`: the two AABB sets.
/// `out_pairs`: receives verified overlapping pairs.
/// Returns the number of pairs written.
pub fn gpu_candidate_box_join(
    boxes_a: &[Aabb],
    boxes_b: &[Aabb],
    out_pairs: &mut [BoxPair],
) -> Result<usize, GeometryGpuError> {
    let max_pairs = boxes_a.len() * boxes_b.len();
    if out_pairs.len() < max_pairs {
        return Err(GeometryGpuError::CandidateBufferTooSmall {
            required: max_pairs,
        });
    }

    // Pack all pairs into f32 buffer (12 f32s per pair).
    let mut packed: Vec<f32> = Vec::with_capacity(max_pairs * 12);
    let mut pair_indices: Vec<(u32, u32)> = Vec::with_capacity(max_pairs);
    for (i, a) in boxes_a.iter().enumerate() {
        for (j, b) in boxes_b.iter().enumerate() {
            packed.push(a.min.x as f32);
            packed.push(a.min.y as f32);
            packed.push(a.min.z as f32);
            packed.push(a.max.x as f32);
            packed.push(a.max.y as f32);
            packed.push(a.max.z as f32);
            packed.push(b.min.x as f32);
            packed.push(b.min.y as f32);
            packed.push(b.min.z as f32);
            packed.push(b.max.x as f32);
            packed.push(b.max.y as f32);
            packed.push(b.max.z as f32);
            pair_indices.push((i as u32, j as u32));
        }
    }

    // GPU filter (CPU simulation).
    let pair_count = pair_indices.len();
    let mut gpu_results: Vec<i32> = vec![0; pair_count];
    for i in 0..pair_count {
        gpu_results[i] = gpu_filter_aabb_overlap_f32(&packed[i * 12..(i + 1) * 12]);
    }

    // Merge: verify uncertain lanes with CPU exact oracle.
    let mut merged: Vec<i32> = vec![0; pair_count];
    merge_aabb_overlap_results(&packed, &gpu_results, &mut merged)?;

    // Collect verified overlapping pairs in deterministic (a, b) order.
    let mut count = 0usize;
    for i in 0..pair_count {
        if merged[i] == GPU_OVERLAP_YES {
            out_pairs[count] = BoxPair {
                a: pair_indices[i].0,
                b: pair_indices[i].1,
            };
            count += 1;
        }
    }

    // Sort for deterministic output.
    out_pairs[..count].sort_unstable();
    Ok(count)
}

/// Map a `Sign` to `i8` matching the GPU's encoding: +1 / 0 / -1.
#[inline]
fn sign_to_i8(s: Sign) -> i8 {
    match s {
        Sign::Positive => 1,
        Sign::Zero => 0,
        Sign::Negative => -1,
    }
}

/// GPU filter result for a single orient3d quadruple (f32).
/// Returns +1 / -1 / `GPU_ORIENTATION_UNCERTAIN`.
///
/// This is the CPU-side simulation of the GPU filtered stage — used by the
/// differential test to verify that GPU-certain lanes match the CPU exact
/// ladder and GPU-uncertain lanes are flagged.
pub fn gpu_filter_orient3d_f32(quad: &[f32]) -> i32 {
    let ax = quad[0] as f64;
    let ay = quad[1] as f64;
    let az = quad[2] as f64;
    let bx = quad[3] as f64;
    let by = quad[4] as f64;
    let bz = quad[5] as f64;
    let cx = quad[6] as f64;
    let cy = quad[7] as f64;
    let cz = quad[8] as f64;
    let dx = quad[9] as f64;
    let dy = quad[10] as f64;
    let dz = quad[11] as f64;

    let abx = bx - ax;
    let aby = by - ay;
    let abz = bz - az;
    let acx = cx - ax;
    let acy = cy - ay;
    let acz = cz - az;
    let adx = dx - ax;
    let ady = dy - ay;
    let adz = dz - az;

    let cx0 = acy * adz - acz * ady;
    let cy0 = acz * adx - acx * adz;
    let cz0 = acx * ady - acy * adx;
    let det = abx * cx0 + aby * cy0 + abz * cz0;

    let perm = abx.abs() * (acy.abs() * adz.abs() + acz.abs() * ady.abs())
        + aby.abs() * (acx.abs() * adz.abs() + acz.abs() * adx.abs())
        + abz.abs() * (acx.abs() * ady.abs() + acy.abs() * adx.abs());
    // f32 epsilon ≈ 1.19e-7, but we use a slightly larger bound for safety
    let error_bound = perm * 1.5e-5;

    if det.abs() <= error_bound {
        GPU_ORIENTATION_UNCERTAIN
    } else if det > 0.0 {
        1
    } else {
        -1
    }
}

/// GPU filter result for a single incircle quadruple (f32).
pub fn gpu_filter_incircle_f32(quad: &[f32]) -> i32 {
    let ax = quad[0] as f64;
    let ay = quad[1] as f64;
    let bx = quad[2] as f64;
    let by = quad[3] as f64;
    let cx = quad[4] as f64;
    let cy = quad[5] as f64;
    let dx = quad[6] as f64;
    let dy = quad[7] as f64;

    let adx = ax - dx;
    let ady = ay - dy;
    let bdx = bx - dx;
    let bdy = by - dy;
    let cdx = cx - dx;
    let cdy = cy - dy;
    let ad2 = adx * adx + ady * ady;
    let bd2 = bdx * bdx + bdy * bdy;
    let cd2 = cdx * cdx + cdy * cdy;

    let m1 = bdy * cd2 - cdy * bd2;
    let m2 = bdx * cd2 - cdx * bd2;
    let m3 = bdx * cdy - bdy * cdx;
    let det = adx * m1 - ady * m2 + ad2 * m3;

    let perm = adx.abs() * (bdy.abs() * cd2.abs() + cdy.abs() * bd2.abs())
        + ady.abs() * (bdx.abs() * cd2.abs() + cdx.abs() * bd2.abs())
        + ad2.abs() * (bdx.abs() * cdy.abs() + bdy.abs() * cdx.abs());
    let error_bound = perm * 3.0e-5;

    if det.abs() <= error_bound {
        GPU_ORIENTATION_UNCERTAIN
    } else if det > 0.0 {
        1
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_oracle_matches_scalar_predicates() {
        let packed = [
            0.0, 0.0, 1.0, 0.0, 1.0, 1.0, // CCW
            0.0, 0.0, 1.0, 0.0, 1.0, -1.0, // CW
            0.0, 0.0, 1.0, 0.0, 2.0, 0.0, // collinear
        ];
        let mut out = [9i8; 3];
        assert_eq!(
            evaluate_orientation_batch_f32(&packed, &mut out).unwrap(),
            3
        );
        assert_eq!(
            out,
            [
                super::super::Orientation::CounterClockwise as i8,
                super::super::Orientation::Clockwise as i8,
                super::super::Orientation::Collinear as i8,
            ]
        );
    }

    #[test]
    fn shader_generation_is_typed_and_deterministic() {
        let schedule = GeometryGpuSchedule::default();
        let a = emit_geometry_wgsl(GeometryGpuKernel::Orientation2F32, schedule).unwrap();
        let b = emit_geometry_wgsl(GeometryGpuKernel::Orientation2F32, schedule).unwrap();
        assert_eq!(a, b);
        assert!(a.contains("@workgroup_size(128, 1, 1)"));
        assert!(a.contains("orientation[i] = 2"));
    }

    #[cfg(feature = "wgsl-forge")]
    #[test]
    fn shader_passes_naga_validation() {
        let source = emit_geometry_wgsl(
            GeometryGpuKernel::Orientation2F32,
            GeometryGpuSchedule::default(),
        )
        .unwrap();
        let report = crate::wgsl_forge::validate_wgsl(&source).unwrap();
        assert_eq!(report.entry_points, vec!["orientation_2_f32"]);
    }

    // ── P1.9: orient3d GPU kernel + oracle ────────────────────────────────

    #[test]
    fn orient3d_batch_oracle_matches_scalar() {
        // Positive, negative, coplanar tetrahedra
        let packed = [
            // Positive: (0,0,0),(1,0,0),(0,1,0),(0,0,1)
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0,
            // Negative: swap b and c
            0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            // Coplanar: all z=0
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let mut out = [9i8; 3];
        assert_eq!(evaluate_orient3d_batch_f32(&packed, &mut out).unwrap(), 3);
        assert_eq!(out, [1, -1, 0]);
    }

    #[test]
    fn orient3d_shader_generation_is_deterministic() {
        let schedule = GeometryGpuSchedule::default();
        let a = emit_geometry_wgsl(GeometryGpuKernel::Orient3dF32, schedule).unwrap();
        let b = emit_geometry_wgsl(GeometryGpuKernel::Orient3dF32, schedule).unwrap();
        assert_eq!(a, b);
        assert!(a.contains("orient_3d_f32"));
        assert!(a.contains("@workgroup_size(128, 1, 1)"));
    }

    #[cfg(feature = "wgsl-forge")]
    #[test]
    fn orient3d_shader_passes_naga_validation() {
        let source = emit_geometry_wgsl(
            GeometryGpuKernel::Orient3dF32,
            GeometryGpuSchedule::default(),
        )
        .unwrap();
        let report = crate::wgsl_forge::validate_wgsl(&source).unwrap();
        assert_eq!(report.entry_points, vec!["orient_3d_f32"]);
    }

    #[test]
    fn orient3d_gpu_certain_lanes_match_cpu_exact() {
        // Clear cases where the GPU filter should be certain
        let packed = [
            // Positive tetrahedron (clear)
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0,
            // Negative tetrahedron (clear)
            0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let mut cpu_out = [9i8; 2];
        evaluate_orient3d_batch_f32(&packed, &mut cpu_out).unwrap();

        for (i, quad) in packed.chunks_exact(12).enumerate() {
            let gpu = gpu_filter_orient3d_f32(quad);
            // GPU-certain lanes must match CPU exact
            assert_ne!(gpu, GPU_ORIENTATION_UNCERTAIN, "lane {i} should be certain");
            assert_eq!(
                gpu as i8, cpu_out[i],
                "lane {i}: GPU={gpu}, CPU={}",
                cpu_out[i]
            );
        }
    }

    #[test]
    fn orient3d_gpu_uncertain_lanes_flagged_near_degeneracy() {
        // Coplanar case — exact zero determinant. GPU filter flags uncertain
        // (|det| = 0 <= error_bound = 0).
        let packed = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0];
        let gpu = gpu_filter_orient3d_f32(&packed);
        assert_eq!(
            gpu, GPU_ORIENTATION_UNCERTAIN,
            "coplanar should be uncertain"
        );

        // CPU oracle resolves it exactly (Zero)
        let mut cpu_out = [9i8; 1];
        evaluate_orient3d_batch_f32(&packed, &mut cpu_out).unwrap();
        assert_eq!(cpu_out[0], 0);
    }

    // ── P1.9: incircle GPU kernel + oracle ────────────────────────────────

    #[test]
    fn incircle_batch_oracle_matches_scalar() {
        // Inside, outside, on (unit circle, CCW)
        let packed = [
            // a=(1,0), b=(0,1), c=(-1,0), d=(0,0) → inside
            1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0, 0.0,
            // a=(1,0), b=(0,1), c=(-1,0), d=(2,0) → outside
            1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 2.0, 0.0,
            // a=(1,0), b=(0,1), c=(-1,0), d=(0,-1) → on
            1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0, -1.0,
        ];
        let mut out = [9i8; 3];
        assert_eq!(evaluate_incircle_batch_f32(&packed, &mut out).unwrap(), 3);
        assert_eq!(out, [1, -1, 0]);
    }

    #[test]
    fn incircle_shader_generation_is_deterministic() {
        let schedule = GeometryGpuSchedule::default();
        let a = emit_geometry_wgsl(GeometryGpuKernel::IncircleF32, schedule).unwrap();
        let b = emit_geometry_wgsl(GeometryGpuKernel::IncircleF32, schedule).unwrap();
        assert_eq!(a, b);
        assert!(a.contains("incircle_f32"));
        assert!(a.contains("@workgroup_size(128, 1, 1)"));
    }

    #[cfg(feature = "wgsl-forge")]
    #[test]
    fn incircle_shader_passes_naga_validation() {
        let source = emit_geometry_wgsl(
            GeometryGpuKernel::IncircleF32,
            GeometryGpuSchedule::default(),
        )
        .unwrap();
        let report = crate::wgsl_forge::validate_wgsl(&source).unwrap();
        assert_eq!(report.entry_points, vec!["incircle_f32"]);
    }

    #[test]
    fn incircle_gpu_certain_lanes_match_cpu_exact() {
        let packed = [
            // Inside (clear)
            1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0, 0.0, // Outside (clear)
            1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 2.0, 0.0,
        ];
        let mut cpu_out = [9i8; 2];
        evaluate_incircle_batch_f32(&packed, &mut cpu_out).unwrap();

        for (i, quad) in packed.chunks_exact(8).enumerate() {
            let gpu = gpu_filter_incircle_f32(quad);
            assert_ne!(gpu, GPU_ORIENTATION_UNCERTAIN, "lane {i} should be certain");
            assert_eq!(
                gpu as i8, cpu_out[i],
                "lane {i}: GPU={gpu}, CPU={}",
                cpu_out[i]
            );
        }
    }

    #[test]
    fn incircle_gpu_uncertain_lanes_flagged_near_degeneracy() {
        // Cocircular case — exact zero determinant. GPU filter flags uncertain.
        let packed = [1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0, -1.0];
        let gpu = gpu_filter_incircle_f32(&packed);
        assert_eq!(
            gpu, GPU_ORIENTATION_UNCERTAIN,
            "cocircular should be uncertain"
        );

        let mut cpu_out = [9i8; 1];
        evaluate_incircle_batch_f32(&packed, &mut cpu_out).unwrap();
        // CPU resolves exactly (Zero — on circle)
        assert_eq!(cpu_out[0], 0);
    }

    // ── P1.9: CPU/GPU differential over the determinism corpus ────────────

    #[test]
    fn cpu_gpu_differential_orient3d_over_corpus() {
        // A set of orient3d cases covering clear, degenerate, and near-degenerate
        let packed: [f32; 36] = [
            // Clear positive
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, // Clear negative
            0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            // Coplanar (exact zero)
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        ];

        let mut cpu_out = [9i8; 3];
        evaluate_orient3d_batch_f32(&packed, &mut cpu_out).unwrap();

        for (i, quad) in packed.chunks_exact(12).enumerate() {
            let gpu = gpu_filter_orient3d_f32(quad);
            if gpu != GPU_ORIENTATION_UNCERTAIN {
                // GPU-certain lane must match CPU exact
                assert_eq!(
                    gpu as i8, cpu_out[i],
                    "GPU-certain lane {i} disagrees with CPU exact: GPU={gpu}, CPU={}",
                    cpu_out[i]
                );
            }
            // GPU-uncertain lanes are correctly flagged — CPU resolves them
        }
    }

    #[test]
    fn cpu_gpu_differential_incircle_over_corpus() {
        let packed: [f32; 24] = [
            // Inside (clear)
            1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0, 0.0, // Outside (clear)
            1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 2.0, 0.0, // On circle (exact zero)
            1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0, -1.0,
        ];

        let mut cpu_out = [9i8; 3];
        evaluate_incircle_batch_f32(&packed, &mut cpu_out).unwrap();

        for (i, quad) in packed.chunks_exact(8).enumerate() {
            let gpu = gpu_filter_incircle_f32(quad);
            if gpu != GPU_ORIENTATION_UNCERTAIN {
                assert_eq!(
                    gpu as i8, cpu_out[i],
                    "GPU-certain lane {i} disagrees with CPU exact: GPU={gpu}, CPU={}",
                    cpu_out[i]
                );
            }
        }
    }

    #[test]
    fn orient3d_batch_rejects_wrong_input_length() {
        let packed = [0.0f32; 11]; // not multiple of 12
        let mut out = [0i8; 1];
        assert_eq!(
            evaluate_orient3d_batch_f32(&packed, &mut out),
            Err(GeometryGpuError::InputLengthNotMultipleOfTwelve)
        );
    }

    #[test]
    fn incircle_batch_rejects_wrong_input_length() {
        let packed = [0.0f32; 7]; // not multiple of 8
        let mut out = [0i8; 1];
        assert_eq!(
            evaluate_incircle_batch_f32(&packed, &mut out),
            Err(GeometryGpuError::InputLengthNotMultipleOfEight)
        );
    }

    #[test]
    fn orient3d_batch_rejects_small_output() {
        let packed = [0.0f32; 12];
        let mut out = [0i8; 0]; // too small
        assert_eq!(
            evaluate_orient3d_batch_f32(&packed, &mut out),
            Err(GeometryGpuError::OutputTooSmall { required: 1 })
        );
    }

    // ── P3.6: AABB overlap GPU kernel + merge ─────────────────────────────

    #[test]
    fn aabb_overlap_shader_generation_is_deterministic() {
        let schedule = GeometryGpuSchedule::default();
        let a = emit_geometry_wgsl(GeometryGpuKernel::AabbOverlapF32, schedule).unwrap();
        let b = emit_geometry_wgsl(GeometryGpuKernel::AabbOverlapF32, schedule).unwrap();
        assert_eq!(a, b);
        assert!(a.contains("aabb_overlap_f32"));
        assert!(a.contains("@workgroup_size(128, 1, 1)"));
    }

    #[test]
    fn aabb_overlap_cpu_oracle_matches_exact() {
        // Clear overlap, clear non-overlap, boundary-touching.
        let packed: [f32; 36] = [
            // Overlapping: [0,0,0]-[2,2,2] vs [1,1,1]-[3,3,3]
            0.0, 0.0, 0.0, 2.0, 2.0, 2.0, 1.0, 1.0, 1.0, 3.0, 3.0, 3.0,
            // Non-overlapping: [0,0,0]-[1,1,1] vs [5,5,5]-[6,6,6]
            0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 5.0, 5.0, 5.0, 6.0, 6.0, 6.0,
            // Boundary-touching: [0,0,0]-[1,1,1] vs [1,0,0]-[2,1,1]
            0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 2.0, 1.0, 1.0,
        ];
        let mut out = [0i32; 3];
        evaluate_aabb_overlap_batch_f32(&packed, &mut out).unwrap();
        assert_eq!(out, [GPU_OVERLAP_YES, GPU_OVERLAP_NO, GPU_OVERLAP_YES]);
    }

    #[test]
    fn aabb_overlap_gpu_certain_lanes_match_cpu() {
        let packed: [f32; 24] = [
            // Clear overlap
            0.0, 0.0, 0.0, 2.0, 2.0, 2.0, 1.0, 1.0, 1.0, 3.0, 3.0, 3.0,
            // Clear non-overlap
            0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 5.0, 5.0, 5.0, 6.0, 6.0, 6.0,
        ];
        let mut cpu_out = [0i32; 2];
        evaluate_aabb_overlap_batch_f32(&packed, &mut cpu_out).unwrap();
        for (i, pair) in packed.chunks_exact(12).enumerate() {
            let gpu = gpu_filter_aabb_overlap_f32(pair);
            assert_ne!(gpu, GPU_OVERLAP_UNCERTAIN, "lane {i} should be certain");
            assert_eq!(gpu, cpu_out[i], "lane {i}: GPU={gpu}, CPU={}", cpu_out[i]);
        }
    }

    #[test]
    fn aabb_overlap_merge_produces_identical_results() {
        // The merge guarantee: GPU-on vs GPU-off produce identical results.
        let packed: [f32; 36] = [
            // Clear overlap
            0.0, 0.0, 0.0, 2.0, 2.0, 2.0, 1.0, 1.0, 1.0, 3.0, 3.0, 3.0,
            // Clear non-overlap
            0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 5.0, 5.0, 5.0, 6.0, 6.0, 6.0,
            // Boundary-touching (may be uncertain)
            0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 2.0, 1.0, 1.0,
        ];

        // Path 1: GPU filter + merge.
        let mut gpu_results = [0i32; 3];
        for i in 0..3 {
            gpu_results[i] = gpu_filter_aabb_overlap_f32(&packed[i * 12..(i + 1) * 12]);
        }
        let mut merged = [0i32; 3];
        merge_aabb_overlap_results(&packed, &gpu_results, &mut merged).unwrap();

        // Path 2: CPU exact only (no GPU).
        let mut cpu_only = [0i32; 3];
        evaluate_aabb_overlap_batch_f32(&packed, &mut cpu_only).unwrap();

        // Merge guarantee: identical results.
        assert_eq!(merged, cpu_only);
    }

    #[test]
    fn gpu_candidate_box_join_matches_brute_force() {
        let boxes_a: Vec<Aabb> = vec![
            Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 2.0, 2.0)),
            Aabb::new(Point3::new(3.0, 3.0, 3.0), Point3::new(5.0, 5.0, 5.0)),
            Aabb::new(Point3::new(1.0, 1.0, 1.0), Point3::new(4.0, 4.0, 4.0)),
        ];
        let boxes_b: Vec<Aabb> = vec![
            Aabb::new(Point3::new(1.5, 1.5, 1.5), Point3::new(3.5, 3.5, 3.5)),
            Aabb::new(Point3::new(10.0, 10.0, 10.0), Point3::new(11.0, 11.0, 11.0)),
        ];

        let max_pairs = boxes_a.len() * boxes_b.len();
        let mut gpu_out = vec![BoxPair { a: 0, b: 0 }; max_pairs];
        let gpu_count = gpu_candidate_box_join(&boxes_a, &boxes_b, &mut gpu_out).unwrap();

        let mut brute_out = vec![BoxPair { a: 0, b: 0 }; max_pairs];
        let brute_count =
            super::super::box_join::box_join_brute_force(&boxes_a, &boxes_b, &mut brute_out)
                .unwrap();

        assert_eq!(gpu_count, brute_count);
        let mut gpu_sorted = gpu_out[..gpu_count].to_vec();
        gpu_sorted.sort_unstable();
        let mut brute_sorted = brute_out[..brute_count].to_vec();
        brute_sorted.sort_unstable();
        assert_eq!(gpu_sorted, brute_sorted);
    }

    #[test]
    fn gpu_candidate_box_join_deterministic() {
        let boxes_a: Vec<Aabb> = vec![
            Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 2.0, 2.0)),
            Aabb::new(Point3::new(1.0, 1.0, 1.0), Point3::new(3.0, 3.0, 3.0)),
        ];
        let boxes_b: Vec<Aabb> = vec![Aabb::new(
            Point3::new(1.5, 0.0, 0.0),
            Point3::new(2.5, 1.0, 1.0),
        )];

        let run = || {
            let mut out = vec![BoxPair { a: 0, b: 0 }; boxes_a.len() * boxes_b.len()];
            let count = gpu_candidate_box_join(&boxes_a, &boxes_b, &mut out).unwrap();
            (count, out)
        };

        let (c1, o1) = run();
        let (c2, o2) = run();
        assert_eq!(c1, c2);
        assert_eq!(o1[..c1], o2[..c2]);
    }

    #[test]
    fn point_aabb_dist_sq_shader_generation_is_deterministic() {
        let schedule = GeometryGpuSchedule::default();
        let a = emit_geometry_wgsl(GeometryGpuKernel::PointAabbDistSqF32, schedule).unwrap();
        let b = emit_geometry_wgsl(GeometryGpuKernel::PointAabbDistSqF32, schedule).unwrap();
        assert_eq!(a, b);
        assert!(a.contains("point_aabb_dist_sq_f32"));
    }
}
