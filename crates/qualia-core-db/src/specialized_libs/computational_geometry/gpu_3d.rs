//! P5.9 — GPU 3-D **point-in-tetrahedron** cull batch, with a deterministic CPU oracle.
//!
//! The 3-D *predicate* GPU batches (`Orient3dF32`, `IncircleF32`) already live in
//! [`super::gpu`] (P1.9) with their WGSL + CPU oracle + differential — this file does
//! **not** duplicate them. It adds the one 3-D broad-phase primitive they don't cover: a
//! batched *point-in-tetrahedron* containment test, used by Delaunay point-location and
//! spatial-containment queries over the P5 3-D family.
//!
//! Same contract as `gpu.rs`: the GPU runs an f32 filtered fast path and returns
//! [`POINT_IN_TETRA_UNCERTAIN`] for any lane near a face (true-zero included); the exact
//! CPU oracle ([`evaluate_point_in_tetra_batch_f32`], full `orient_3d` ladder) is the
//! robust fallback and the no-adapter deterministic path. A conservative f32 error bound
//! means the GPU never returns a *wrong* certain answer — near-boundary lanes are always
//! deferred to the CPU, never guessed.

use super::expansion::Sign;
use super::{orient_3d, Point3};

/// The queried point lies strictly outside the tetrahedron.
pub const POINT_IN_TETRA_OUTSIDE: i8 = 0;
/// The queried point lies strictly inside the tetrahedron.
pub const POINT_IN_TETRA_INSIDE: i8 = 1;
/// The queried point lies exactly on the tetrahedron boundary (a face/edge/vertex), or the
/// tetrahedron is degenerate (coplanar, no interior). Exact-CPU result only.
pub const POINT_IN_TETRA_BOUNDARY: i8 = 2;
/// GPU f32 fast-path could not decide (near a face) — resolve on the CPU exact oracle.
pub const POINT_IN_TETRA_UNCERTAIN: i8 = 3;

/// 15 f32 per query: `p(3), a(3), b(3), c(3), d(3)` — the point then the tetra vertices.
pub const POINT_IN_TETRA_STRIDE: usize = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gpu3dError {
    /// Packed input length was not a multiple of [`POINT_IN_TETRA_STRIDE`].
    InputLengthNotMultipleOfStride,
    /// Output slice smaller than the query count.
    OutputTooSmall { required: usize },
}

#[inline]
fn sign_i8(s: Sign) -> i8 {
    match s {
        Sign::Positive => 1,
        Sign::Zero => 0,
        Sign::Negative => -1,
    }
}

/// Classify one query exactly via the `orient_3d` ladder.
///
/// `p` is inside `abcd` iff each of the four sub-tetrahedra formed by substituting `p` for
/// one vertex has the *same* orientation sign as the full tetrahedron. A `Zero` on any
/// sub-tetra means `p` is coplanar with a face → boundary. A degenerate tetra (`o == 0`)
/// has no interior → boundary.
#[inline]
fn classify_exact(p: Point3, a: Point3, b: Point3, c: Point3, d: Point3) -> i8 {
    let o = sign_i8(orient_3d(a, b, c, d));
    if o == 0 {
        return POINT_IN_TETRA_BOUNDARY; // degenerate tetra, no interior
    }
    let s = [
        sign_i8(orient_3d(p, b, c, d)), // substitute a
        sign_i8(orient_3d(a, p, c, d)), // substitute b
        sign_i8(orient_3d(a, b, p, d)), // substitute c
        sign_i8(orient_3d(a, b, c, p)), // substitute d
    ];
    let mut on_boundary = false;
    let mut outside = false;
    for &si in &s {
        if si == 0 {
            on_boundary = true;
        } else if si != o {
            outside = true;
        }
    }
    if outside {
        POINT_IN_TETRA_OUTSIDE
    } else if on_boundary {
        POINT_IN_TETRA_BOUNDARY
    } else {
        POINT_IN_TETRA_INSIDE
    }
}

/// CPU/WASM exact oracle for a packed batch of point-in-tetra queries (15 f32 each,
/// [`POINT_IN_TETRA_STRIDE`]). Runs the full filtered → compensated → exact `orient_3d`
/// ladder, so its output is the ground truth the GPU fast path is verified against.
/// Deterministic; never returns [`POINT_IN_TETRA_UNCERTAIN`] (that is a GPU-only code).
pub fn evaluate_point_in_tetra_batch_f32(
    packed: &[f32],
    out: &mut [i8],
) -> Result<usize, Gpu3dError> {
    if packed.len() % POINT_IN_TETRA_STRIDE != 0 {
        return Err(Gpu3dError::InputLengthNotMultipleOfStride);
    }
    let count = packed.len() / POINT_IN_TETRA_STRIDE;
    if out.len() < count {
        return Err(Gpu3dError::OutputTooSmall { required: count });
    }
    for (i, q) in packed.chunks_exact(POINT_IN_TETRA_STRIDE).enumerate() {
        let pt = |o: usize| Point3::new(q[o] as f64, q[o + 1] as f64, q[o + 2] as f64);
        out[i] = classify_exact(pt(0), pt(3), pt(6), pt(9), pt(12));
    }
    Ok(count)
}

// ── f32 filtered fast path (mirrors the WGSL below, for the no-adapter differential) ──

/// Conservative f32 orient3d filter. Returns `1` / `-1` for a confident sign, or `0` when
/// `|det|` is within the (deliberately generous) error bound — "uncertain, defer to CPU".
/// Generous ⇒ the fast path is never *wrong*, only more often deferred.
#[inline]
fn orient3d_f32_filtered(a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3]) -> i32 {
    let ba = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ca = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let da = [d[0] - a[0], d[1] - a[1], d[2] - a[2]];
    let m0 = ca[1] * da[2] - ca[2] * da[1];
    let m1 = ca[0] * da[2] - ca[2] * da[0];
    let m2 = ca[0] * da[1] - ca[1] * da[0];
    let det = ba[0] * m0 - ba[1] * m1 + ba[2] * m2;
    let perm = ba[0].abs() * (ca[1].abs() * da[2].abs() + ca[2].abs() * da[1].abs())
        + ba[1].abs() * (ca[0].abs() * da[2].abs() + ca[2].abs() * da[0].abs())
        + ba[2].abs() * (ca[0].abs() * da[1].abs() + ca[1].abs() * da[0].abs());
    let bound = perm * 1.0e-4; // conservative f32 orient3d bound
    if det.abs() <= bound {
        0
    } else if det > 0.0 {
        1
    } else {
        -1
    }
}

/// CPU simulation of the GPU point-in-tetra fast path (f32), for a GPU/CPU differential test
/// that needs no adapter. Returns `INSIDE` / `OUTSIDE` / `UNCERTAIN` (never `BOUNDARY` — the
/// fast path maps every near-face case to `UNCERTAIN` for the CPU exact oracle to resolve).
pub fn gpu_filter_point_in_tetra_f32(
    packed: &[f32],
    out: &mut [i8],
) -> Result<usize, Gpu3dError> {
    if packed.len() % POINT_IN_TETRA_STRIDE != 0 {
        return Err(Gpu3dError::InputLengthNotMultipleOfStride);
    }
    let count = packed.len() / POINT_IN_TETRA_STRIDE;
    if out.len() < count {
        return Err(Gpu3dError::OutputTooSmall { required: count });
    }
    for (i, q) in packed.chunks_exact(POINT_IN_TETRA_STRIDE).enumerate() {
        let v = |o: usize| [q[o], q[o + 1], q[o + 2]];
        let (p, a, b, c, d) = (v(0), v(3), v(6), v(9), v(12));
        let o = orient3d_f32_filtered(a, b, c, d);
        if o == 0 {
            out[i] = POINT_IN_TETRA_UNCERTAIN; // degenerate/near-degenerate tetra
            continue;
        }
        let s = [
            orient3d_f32_filtered(p, b, c, d),
            orient3d_f32_filtered(a, p, c, d),
            orient3d_f32_filtered(a, b, p, d),
            orient3d_f32_filtered(a, b, c, p),
        ];
        if s.iter().any(|&si| si == 0) {
            out[i] = POINT_IN_TETRA_UNCERTAIN;
        } else if s.iter().any(|&si| si != o) {
            out[i] = POINT_IN_TETRA_OUTSIDE;
        } else {
            out[i] = POINT_IN_TETRA_INSIDE;
        }
    }
    Ok(count)
}

/// Emit the Naga-valid WGSL compute shader for the point-in-tetra fast path. One invocation
/// per 15-f32 query; writes `0/1/3` (outside/inside/uncertain) into `result_out`.
pub fn point_in_tetra_wgsl(workgroup_size: u32) -> String {
    format!(
        r#"// QualiaDB computational geometry: point_in_tetra_f32 v1
struct Params {{
    query_count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}}

@group(0) @binding(0)
var<storage, read> data: array<f32>;

@group(0) @binding(1)
var<storage, read_write> result_out: array<i32>;

@group(0) @binding(2)
var<uniform> params: Params;

// Conservative f32 orient3d: +1 / -1 confident, 0 = uncertain (|det| within bound).
fn o3d(a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, d: vec3<f32>) -> i32 {{
    let ba = b - a;
    let ca = c - a;
    let da = d - a;
    let m0 = ca.y * da.z - ca.z * da.y;
    let m1 = ca.x * da.z - ca.z * da.x;
    let m2 = ca.x * da.y - ca.y * da.x;
    let det = ba.x * m0 - ba.y * m1 + ba.z * m2;
    let perm = abs(ba.x) * (abs(ca.y) * abs(da.z) + abs(ca.z) * abs(da.y))
             + abs(ba.y) * (abs(ca.x) * abs(da.z) + abs(ca.z) * abs(da.x))
             + abs(ba.z) * (abs(ca.x) * abs(da.y) + abs(ca.y) * abs(da.x));
    let bound = perm * 1.0e-4;
    if (abs(det) <= bound) {{ return 0; }}
    if (det > 0.0) {{ return 1; }}
    return -1;
}}

@compute @workgroup_size({workgroup_size}, 1, 1)
fn point_in_tetra_f32(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let i = gid.x;
    if (i >= params.query_count) {{ return; }}
    let base = i * 15u;
    let p = vec3<f32>(data[base],       data[base + 1u],  data[base + 2u]);
    let a = vec3<f32>(data[base + 3u],  data[base + 4u],  data[base + 5u]);
    let b = vec3<f32>(data[base + 6u],  data[base + 7u],  data[base + 8u]);
    let c = vec3<f32>(data[base + 9u],  data[base + 10u], data[base + 11u]);
    let d = vec3<f32>(data[base + 12u], data[base + 13u], data[base + 14u]);
    let o = o3d(a, b, c, d);
    if (o == 0) {{ result_out[i] = {uncertain}; return; }}
    let s0 = o3d(p, b, c, d);
    let s1 = o3d(a, p, c, d);
    let s2 = o3d(a, b, p, d);
    let s3 = o3d(a, b, c, p);
    if (s0 == 0 || s1 == 0 || s2 == 0 || s3 == 0) {{ result_out[i] = {uncertain}; return; }}
    if (s0 != o || s1 != o || s2 != o || s3 != o) {{ result_out[i] = {outside}; return; }}
    result_out[i] = {inside};
}}
"#,
        workgroup_size = workgroup_size,
        uncertain = POINT_IN_TETRA_UNCERTAIN as i32,
        outside = POINT_IN_TETRA_OUTSIDE as i32,
        inside = POINT_IN_TETRA_INSIDE as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reference tetra: origin + unit axes.
    fn tetra() -> [f32; 12] {
        [
            0.0, 0.0, 0.0, // a
            1.0, 0.0, 0.0, // b
            0.0, 1.0, 0.0, // c
            0.0, 0.0, 1.0, // d
        ]
    }

    fn query(p: [f32; 3]) -> Vec<f32> {
        let mut v = vec![p[0], p[1], p[2]];
        v.extend_from_slice(&tetra());
        v
    }

    #[test]
    fn interior_point_is_inside() {
        let q = query([0.2, 0.2, 0.2]); // centroid-ish, strictly inside
        let mut out = [0i8; 1];
        evaluate_point_in_tetra_batch_f32(&q, &mut out).unwrap();
        assert_eq!(out[0], POINT_IN_TETRA_INSIDE);
    }

    #[test]
    fn exterior_point_is_outside() {
        let mut out = [0i8; 1];
        evaluate_point_in_tetra_batch_f32(&query([1.0, 1.0, 1.0]), &mut out).unwrap();
        assert_eq!(out[0], POINT_IN_TETRA_OUTSIDE);
        evaluate_point_in_tetra_batch_f32(&query([-0.1, 0.2, 0.2]), &mut out).unwrap();
        assert_eq!(out[0], POINT_IN_TETRA_OUTSIDE);
    }

    #[test]
    fn point_on_face_is_boundary() {
        // On the z=0 face (a,b,c).
        let mut out = [0i8; 1];
        evaluate_point_in_tetra_batch_f32(&query([0.25, 0.25, 0.0]), &mut out).unwrap();
        assert_eq!(out[0], POINT_IN_TETRA_BOUNDARY);
        // A tetra vertex is on the boundary.
        evaluate_point_in_tetra_batch_f32(&query([0.0, 0.0, 0.0]), &mut out).unwrap();
        assert_eq!(out[0], POINT_IN_TETRA_BOUNDARY);
    }

    #[test]
    fn degenerate_tetra_has_no_interior() {
        // Four coplanar points (all z=0): no interior → boundary for any query.
        let flat = [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0];
        let mut v = vec![0.3f32, 0.3, 0.0];
        v.extend_from_slice(&flat);
        let mut out = [0i8; 1];
        evaluate_point_in_tetra_batch_f32(&v, &mut out).unwrap();
        assert_eq!(out[0], POINT_IN_TETRA_BOUNDARY);
    }

    #[test]
    fn batch_and_length_errors() {
        let mut out = [0i8; 3];
        let batch: Vec<f32> = [
            query([0.2, 0.2, 0.2]),
            query([2.0, 2.0, 2.0]),
            query([0.1, 0.1, 0.05]),
        ]
        .concat();
        assert_eq!(evaluate_point_in_tetra_batch_f32(&batch, &mut out).unwrap(), 3);
        assert_eq!(out, [POINT_IN_TETRA_INSIDE, POINT_IN_TETRA_OUTSIDE, POINT_IN_TETRA_INSIDE]);
        assert_eq!(
            evaluate_point_in_tetra_batch_f32(&[0.0; 14], &mut out),
            Err(Gpu3dError::InputLengthNotMultipleOfStride)
        );
        let mut tiny = [0i8; 1];
        assert_eq!(
            evaluate_point_in_tetra_batch_f32(&batch, &mut tiny),
            Err(Gpu3dError::OutputTooSmall { required: 3 })
        );
    }

    #[test]
    fn gpu_filter_agrees_with_exact_on_confident_lanes() {
        // Clearly-inside / clearly-outside points: the f32 fast path must MATCH the exact
        // oracle (never a wrong certain answer). On-boundary points must flag UNCERTAIN.
        let cases: [([f32; 3], i8); 4] = [
            ([0.2, 0.2, 0.2], POINT_IN_TETRA_INSIDE),
            ([2.0, 2.0, 2.0], POINT_IN_TETRA_OUTSIDE),
            ([-0.5, 0.3, 0.3], POINT_IN_TETRA_OUTSIDE),
            ([0.1, 0.1, 0.05], POINT_IN_TETRA_INSIDE),
        ];
        for (p, expect) in cases {
            let q = query(p);
            let mut ex = [0i8; 1];
            let mut gp = [0i8; 1];
            evaluate_point_in_tetra_batch_f32(&q, &mut ex).unwrap();
            gpu_filter_point_in_tetra_f32(&q, &mut gp).unwrap();
            assert_eq!(ex[0], expect);
            // GPU either matches exactly or defers (uncertain) — never a wrong certain sign.
            assert!(
                gp[0] == ex[0] || gp[0] == POINT_IN_TETRA_UNCERTAIN,
                "gpu {} vs exact {} for {:?}",
                gp[0], ex[0], p
            );
        }
        // On-face point: exact says boundary, GPU fast path must defer (uncertain).
        let qf = query([0.25, 0.25, 0.0]);
        let mut gp = [0i8; 1];
        gpu_filter_point_in_tetra_f32(&qf, &mut gp).unwrap();
        assert_eq!(gp[0], POINT_IN_TETRA_UNCERTAIN);
    }

    #[test]
    fn wgsl_is_naga_valid() {
        let src = point_in_tetra_wgsl(128);
        crate::wgsl_forge::validate_wgsl(&src)
            .expect("point_in_tetra WGSL should pass Naga validation");
    }

    #[test]
    fn deterministic() {
        let q = query([0.2, 0.2, 0.2]);
        let mut a = [0i8; 1];
        let mut b = [0i8; 1];
        evaluate_point_in_tetra_batch_f32(&q, &mut a).unwrap();
        evaluate_point_in_tetra_batch_f32(&q, &mut b).unwrap();
        assert_eq!(a, b);
        assert_eq!(point_in_tetra_wgsl(128), point_in_tetra_wgsl(128));
    }
}
