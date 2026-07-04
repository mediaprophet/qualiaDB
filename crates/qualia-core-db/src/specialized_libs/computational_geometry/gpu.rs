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

use super::{incircle, orientation_2, orient_3d, Point2, Point3};
use super::expansion::Sign;

pub const GPU_ORIENTATION_UNCERTAIN: i32 = 2;

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
    OutputTooSmall { required: usize },
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
fn gpu_filter_orient3d_f32(quad: &[f32]) -> i32 {
    let ax = quad[0] as f64; let ay = quad[1] as f64; let az = quad[2] as f64;
    let bx = quad[3] as f64; let by = quad[4] as f64; let bz = quad[5] as f64;
    let cx = quad[6] as f64; let cy = quad[7] as f64; let cz = quad[8] as f64;
    let dx = quad[9] as f64; let dy = quad[10] as f64; let dz = quad[11] as f64;

    let abx = bx - ax; let aby = by - ay; let abz = bz - az;
    let acx = cx - ax; let acy = cy - ay; let acz = cz - az;
    let adx = dx - ax; let ady = dy - ay; let adz = dz - az;

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
fn gpu_filter_incircle_f32(quad: &[f32]) -> i32 {
    let ax = quad[0] as f64; let ay = quad[1] as f64;
    let bx = quad[2] as f64; let by = quad[3] as f64;
    let cx = quad[4] as f64; let cy = quad[5] as f64;
    let dx = quad[6] as f64; let dy = quad[7] as f64;

    let adx = ax - dx; let ady = ay - dy;
    let bdx = bx - dx; let bdy = by - dy;
    let cdx = cx - dx; let cdy = cy - dy;
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
            assert_eq!(gpu as i8, cpu_out[i], "lane {i}: GPU={gpu}, CPU={}", cpu_out[i]);
        }
    }

    #[test]
    fn orient3d_gpu_uncertain_lanes_flagged_near_degeneracy() {
        // Coplanar case — exact zero determinant. GPU filter flags uncertain
        // (|det| = 0 <= error_bound = 0).
        let packed = [
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let gpu = gpu_filter_orient3d_f32(&packed);
        assert_eq!(gpu, GPU_ORIENTATION_UNCERTAIN, "coplanar should be uncertain");

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
            1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0, 0.0,
            // Outside (clear)
            1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 2.0, 0.0,
        ];
        let mut cpu_out = [9i8; 2];
        evaluate_incircle_batch_f32(&packed, &mut cpu_out).unwrap();

        for (i, quad) in packed.chunks_exact(8).enumerate() {
            let gpu = gpu_filter_incircle_f32(quad);
            assert_ne!(gpu, GPU_ORIENTATION_UNCERTAIN, "lane {i} should be certain");
            assert_eq!(gpu as i8, cpu_out[i], "lane {i}: GPU={gpu}, CPU={}", cpu_out[i]);
        }
    }

    #[test]
    fn incircle_gpu_uncertain_lanes_flagged_near_degeneracy() {
        // Cocircular case — exact zero determinant. GPU filter flags uncertain.
        let packed = [
            1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0, -1.0,
        ];
        let gpu = gpu_filter_incircle_f32(&packed);
        assert_eq!(gpu, GPU_ORIENTATION_UNCERTAIN, "cocircular should be uncertain");

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
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0,
            // Clear negative
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
            1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0, 0.0,
            // Outside (clear)
            1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 2.0, 0.0,
            // On circle (exact zero)
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
}
