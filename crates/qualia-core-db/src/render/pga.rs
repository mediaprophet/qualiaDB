//! 3D PGA motors as dual quaternions — CPU oracle for `projector.wgsl` Phase 2b+.
//!
//! `Motor { r, d }` packs the even subalgebra: rotation (`r`) + translation (`d`).
//! When `d == 0`, `sandwich_point` reduces to the Phase 1 quaternion path.
//! Phase 2c: bilateral `T_pull` via `motor_translate` composed after intrinsic `R_w · R_q`.
//! Phase 3: `v`-band topology (`R_toroidal`, `T_radial`, `T_anchor`) inside intrinsic stack.

use crate::render::telemetry::{
    DEONTIC_LANE_BILATERAL, STANDPOINT_DID, STANDPOINT_SPECTATOR, STANDPOINT_VAULT,
};

/// PGA motor — matches WGSL `Motor { r: vec4, d: vec4 }`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motor {
    pub r: [f32; 4],
    pub d: [f32; 4],
}

impl Motor {
    #[inline]
    pub const fn identity() -> Self {
        Self {
            r: [1.0, 0.0, 0.0, 0.0],
            d: [0.0; 4],
        }
    }

    #[inline]
    pub fn from_rotor(r: [f32; 4]) -> Self {
        Self { r, d: [0.0; 4] }
    }
}

/// Quaternion `(w, x, y, z)` stored as `[w, x, y, z]`.
type Quat = [f32; 4];

#[inline]
fn quat_mul(a: Quat, b: Quat) -> Quat {
    [
        a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
        a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
        a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
        a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0],
    ]
}

#[inline]
fn quat_conj(q: Quat) -> Quat {
    [q[0], -q[1], -q[2], -q[3]]
}

#[inline]
fn quat_add(a: Quat, b: Quat) -> Quat {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3] + b[3]]
}

/// Map motor `r` or `d` vec4 `(s, e12, e13, e23)` → quaternion `(w, x, y, z)`.
#[inline]
pub fn blade4_to_quat(v: [f32; 4]) -> Quat {
    [v[0], -v[3], -v[2], -v[1]]
}

/// Inverse of [`blade4_to_quat`].
#[inline]
pub fn quat_to_blade4(q: Quat) -> [f32; 4] {
    [q[0], -q[3], -q[2], -q[1]]
}

/// PGA reversion: flip bivector signs; scalar + pseudoscalar stay positive.
#[inline]
pub fn motor_reverse(m: Motor) -> Motor {
    Motor {
        r: [m.r[0], -m.r[1], -m.r[2], -m.r[3]],
        d: [m.d[0], -m.d[1], -m.d[2], -m.d[3]],
    }
}

/// Dual-quaternion product `(qr1, qd1) ⊗ (qr2, qd2)`.
#[inline]
pub fn motor_mul(a: Motor, b: Motor) -> Motor {
    let qr1 = blade4_to_quat(a.r);
    let qd1 = blade4_to_quat(a.d);
    let qr2 = blade4_to_quat(b.r);
    let qd2 = blade4_to_quat(b.d);
    let qr3 = quat_mul(qr1, qr2);
    let qd3 = quat_add(quat_mul(qr1, qd2), quat_mul(qd1, qr2));
    Motor {
        r: quat_to_blade4(qr3),
        d: quat_to_blade4(qd3),
    }
}

/// Pure translation motor — dual part encodes `v/2` through [`quat_to_blade4`].
#[inline]
pub fn motor_translate(v: [f32; 3]) -> Motor {
    Motor {
        r: [1.0, 0.0, 0.0, 0.0],
        d: quat_to_blade4([0.0, v[0] * 0.5, v[1] * 0.5, v[2] * 0.5]),
    }
}

/// Per-node deontic lane encoded in `Tensor10D::mu` (metadata carrier).
#[inline]
pub fn tensor_deontic_lane(mu: f32) -> u32 {
    mu.round() as u32
}

/// Bilateral `T_pull` magnitude gate — node lane + authenticated Human-Centric standpoint.
#[inline]
pub fn bilateral_pull_active(tensor_mu: f32, standpoint_class: u32) -> bool {
    tensor_deontic_lane(tensor_mu) == DEONTIC_LANE_BILATERAL && standpoint_class >= STANDPOINT_DID
}

/// Pull vector toward camera eye: `direction · (0.12 · α · epistemic_q)`.
#[inline]
pub fn pull_vector(
    node: [f32; 3],
    camera_eye: [f32; 3],
    alpha: f32,
    epistemic_q: f32,
) -> [f32; 3] {
    let dx = camera_eye[0] - node[0];
    let dy = camera_eye[1] - node[1];
    let dz = camera_eye[2] - node[2];
    let len = (dx * dx + dy * dy + dz * dz).sqrt();
    if len < 1e-6 {
        return [0.0; 3];
    }
    let gain = alpha.clamp(0.2, 1.0);
    let delta = 0.12 * gain * epistemic_q.clamp(0.0, 1.0);
    [dx / len * delta, dy / len * delta, dz / len * delta]
}

/// Null-vector sandwich on euclidean `(x, y, z)` — `P' = Ω P Ω̃`.
///
/// `P = e₀ + x·e₁ + y·e₂ + z·e₃`. When `m.d == 0`, ε terms vanish → pure rotation.
#[inline]
pub fn sandwich_point(m: Motor, p: [f32; 3]) -> [f32; 3] {
    let qr = blade4_to_quat(m.r);
    let qd = blade4_to_quat(m.d);
    let qr_conj = quat_conj(qr);

    let p_q: Quat = [0.0, p[0], p[1], p[2]];
    let p_rot = quat_mul(quat_mul(qr, p_q), qr_conj);

    // Translation from dual part: t = 2 · qd · qr*
    let t_q = quat_mul(qd, qr_conj);
    const T_SCALE: f32 = 2.0;
    [
        p_rot[1] + T_SCALE * t_q[1],
        p_rot[2] + T_SCALE * t_q[2],
        p_rot[3] + T_SCALE * t_q[3],
    ]
}

// ── Phase 1 legacy path (regression oracle) ─────────────────────────────────

#[inline]
pub fn rotor_from_axis_angle(axis: [f32; 3], angle: f32) -> [f32; 4] {
    let half = angle * 0.5;
    let c = half.cos();
    let s = half.sin();
    [
        c,
        s * (-axis[2]),
        s * axis[1],
        s * (-axis[0]),
    ]
}

#[inline]
pub fn rotor_mul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
        a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
        a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
        a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0],
    ]
}

/// Phase 1 quaternion sandwich — regression baseline when `d = 0`.
#[inline]
pub fn legacy_rotor_apply_vector(r: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let q = blade4_to_quat(r);
    let q_conj = quat_conj(q);
    let p_q: Quat = [0.0, v[0], v[1], v[2]];
    let out = quat_mul(quat_mul(q, p_q), q_conj);
    [out[1], out[2], out[3]]
}

// ── Semantic motor builders (mirror projector.wgsl) ─────────────────────────

const TWO_PI: f32 = std::f32::consts::TAU;
const MANIFOLD_COUNT: f32 = 5.0;
const CLUSTER_COUNT: u32 = 8;
const T_RADIAL_GAIN: f32 = 0.06;
const ANCHOR_RING_RADIUS: f32 = 0.35;

/// Boundary-clique cluster slot derived from `tensor.sigma` (spectral class index).
#[inline]
pub fn cluster_id_from_sigma(sigma: f32) -> u32 {
    let frac = sigma.fract();
    let idx = (frac * CLUSTER_COUNT as f32).floor() as u32;
    idx % CLUSTER_COUNT
}

/// Deterministic centroid lattice for boundary cliques — matches WGSL (no extra SSBO in Phase 3).
#[inline]
pub fn cluster_centroid_lattice(cluster_id: u32) -> [f32; 3] {
    let k = cluster_id % CLUSTER_COUNT;
    let angle = (k as f32) * TWO_PI / CLUSTER_COUNT as f32;
    [
        ANCHOR_RING_RADIUS * angle.cos(),
        0.0,
        ANCHOR_RING_RADIUS * angle.sin(),
    ]
}

/// Phase 3 `v`-band intrinsic motor: Euclidean / cyclic / hyperbolic / boundary clique.
#[inline]
pub fn motor_v_band(v: f32, node: [f32; 3], sigma: f32, time: f32, alpha: f32) -> Motor {
    let gain = alpha.clamp(0.2, 1.0);
    if v < 1.0 {
        Motor::identity()
    } else if v < 2.0 {
        let band = v - 1.0;
        let theta = band * TWO_PI * (time * 0.5 + sigma).sin() * gain;
        Motor::from_rotor(rotor_from_axis_angle([0.0, 1.0, 0.0], theta))
    } else if v < 3.0 {
        let band = v - 2.0;
        let len = (node[0] * node[0] + node[1] * node[1] + node[2] * node[2])
            .sqrt()
            .max(1e-4);
        let dir = [node[0] / len, node[1] / len, node[2] / len];
        let delta = T_RADIAL_GAIN * band * gain;
        motor_translate([dir[0] * delta, dir[1] * delta, dir[2] * delta])
    } else {
        let centroid = cluster_centroid_lattice(cluster_id_from_sigma(sigma));
        let blend = (v - 3.0).min(1.0) * gain;
        motor_translate([
            (centroid[0] - node[0]) * blend,
            (centroid[1] - node[1]) * blend,
            (centroid[2] - node[2]) * blend,
        ])
    }
}

#[inline]
pub fn motor_rw(w: f32, alpha: f32) -> [f32; 4] {
    let theta_w = w * (TWO_PI / MANIFOLD_COUNT);
    let gain = alpha.clamp(0.2, 1.0);
    rotor_from_axis_angle([0.0, 1.0, 0.0], theta_w * gain)
}

#[inline]
pub fn motor_rq(q: f32, sigma: f32, time: f32, alpha: f32) -> [f32; 4] {
    motor_rq_gated(q, sigma, time, alpha, STANDPOINT_SPECTATOR, 1.0)
}

#[inline]
pub fn motor_rq_gated(
    q: f32,
    sigma: f32,
    time: f32,
    alpha: f32,
    standpoint_class: u32,
    epistemic_q: f32,
) -> [f32; 4] {
    if standpoint_class == STANDPOINT_VAULT {
        return [1.0, 0.0, 0.0, 0.0];
    }
    if q <= 0.001 {
        return [1.0, 0.0, 0.0, 0.0];
    }
    let gain = alpha.clamp(0.2, 1.0);
    let mut theta_q = q * (time * 2.0 + sigma * TWO_PI).sin() * gain;
    if standpoint_class == STANDPOINT_DID {
        theta_q *= epistemic_q.clamp(0.0, 1.0);
    }
    let ax = (sigma * TWO_PI).cos();
    let az = (sigma * TWO_PI).sin();
    let len = (ax * ax + az * az).sqrt().max(1e-4);
    rotor_from_axis_angle([ax / len, 0.0, az / len], theta_q)
}

#[inline]
pub fn semantic_motor(w: f32, q: f32, sigma: f32, time: f32, alpha: f32) -> Motor {
    semantic_motor_intrinsic(0.0, w, q, sigma, time, alpha, [0.0; 3], STANDPOINT_SPECTATOR, 1.0)
}

#[inline]
pub fn semantic_motor_intrinsic(
    tensor_v: f32,
    w: f32,
    q: f32,
    sigma: f32,
    time: f32,
    alpha: f32,
    node: [f32; 3],
    standpoint_class: u32,
    epistemic_q: f32,
) -> Motor {
    let r_v = motor_v_band(tensor_v, node, sigma, time, alpha);
    let r_w = motor_rw(w, alpha);
    let r_q = motor_rq_gated(q, sigma, time, alpha, standpoint_class, epistemic_q);
    // R_w · R_q · R_v — v-band local topology, then epistemic spin, then manifold fan-out.
    motor_mul(
        Motor::from_rotor(r_w),
        motor_mul(Motor::from_rotor(r_q), r_v),
    )
}

/// Phase 2c semantic motor: `Ω = T_pull · (R_w · R_q)`.
#[inline]
pub fn semantic_motor_phase2c(
    tensor_v: f32,
    w: f32,
    q: f32,
    sigma: f32,
    time: f32,
    alpha: f32,
    tensor_mu: f32,
    node: [f32; 3],
    camera_eye: [f32; 3],
    standpoint_class: u32,
    epistemic_q: f32,
) -> Motor {
    let r_intrinsic =
        semantic_motor_intrinsic(tensor_v, w, q, sigma, time, alpha, node, standpoint_class, epistemic_q);
    let t_motor = if bilateral_pull_active(tensor_mu, standpoint_class) {
        motor_translate(pull_vector(node, camera_eye, alpha, epistemic_q))
    } else {
        Motor::identity()
    };
    motor_mul(t_motor, r_intrinsic)
}

/// Column-major affine `mat4` (WGSL `mat4x4<f32>`) for a rigid motor (rotation + translation).
/// Built by sending the origin (→ translation) and the basis vectors (→ rotation columns) through
/// the sandwich product, so the matrix reproduces `sandwich_point` exactly. Used as the per-artefact
/// model transform in the mesh shader (Phase 2 kinematic joints).
pub fn motor_to_mat4_col(m: Motor) -> [[f32; 4]; 4] {
    let t = sandwich_point(m, [0.0, 0.0, 0.0]);
    let cx = sandwich_point(m, [1.0, 0.0, 0.0]);
    let cy = sandwich_point(m, [0.0, 1.0, 0.0]);
    let cz = sandwich_point(m, [0.0, 0.0, 1.0]);
    [
        [cx[0] - t[0], cx[1] - t[1], cx[2] - t[2], 0.0],
        [cy[0] - t[0], cy[1] - t[1], cy[2] - t[2], 0.0],
        [cz[0] - t[0], cz[1] - t[1], cz[2] - t[2], 0.0],
        [t[0], t[1], t[2], 1.0],
    ]
}

#[inline]
fn approx_eq3(a: [f32; 3], b: [f32; 3], eps: f32) -> bool {
    (a[0] - b[0]).abs() <= eps
        && (a[1] - b[1]).abs() <= eps
        && (a[2] - b[2]).abs() <= eps
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-5;

    #[test]
    fn identity_sandwich_is_noop() {
        let p = [0.3, -0.7, 0.15];
        assert!(approx_eq3(sandwich_point(Motor::identity(), p), p, EPS));
    }

    #[test]
    fn mat4_identity_and_translation() {
        let id = motor_to_mat4_col(Motor::identity());
        assert_eq!(
            id,
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ]
        );
        let tm = motor_to_mat4_col(motor_translate([2.0, -3.0, 4.0]));
        assert!((tm[3][0] - 2.0).abs() < EPS);
        assert!((tm[3][1] + 3.0).abs() < EPS);
        assert!((tm[3][2] - 4.0).abs() < EPS);
    }

    #[test]
    fn d_zero_matches_legacy_quaternion_path() {
        let fixtures: [([f32; 4], [f32; 3]); 6] = [
            (rotor_from_axis_angle([0.0, 1.0, 0.0], 0.8), [1.0, 0.0, 0.0]),
            (rotor_from_axis_angle([1.0, 0.0, 0.0], 1.2), [0.0, 1.0, 0.2]),
            (rotor_mul(
                motor_rw(2.0, 1.0),
                motor_rq(0.35, 0.25, 1.7, 0.9),
            ), [-0.4, 0.6, 0.1]),
            (semantic_motor(3.0, 0.2, 0.5, 2.3, 1.0).r, [0.2, -0.3, 0.8]),
            ([0.70710677, 0.0, 0.70710677, 0.0], [1.0, 0.0, 0.0]),
            (rotor_from_axis_angle([0.0, 0.0, 1.0], -0.5), [-0.2, 0.5, 0.0]),
        ];
        for (r, p) in fixtures {
            let m = Motor::from_rotor(r);
            let legacy = legacy_rotor_apply_vector(r, p);
            let pga = sandwich_point(m, p);
            assert!(
                approx_eq3(legacy, pga, EPS),
                "legacy {:?} != pga {:?} for r {:?}",
                legacy,
                pga,
                r
            );
        }
    }

    #[test]
    fn motor_mul_r_matches_rotor_mul_when_d_zero() {
        let a = motor_rw(1.0, 0.8);
        let b = motor_rq(0.4, 0.33, 0.5, 1.0);
        let m = motor_mul(Motor::from_rotor(a), Motor::from_rotor(b));
        let r_legacy = rotor_mul(a, b);
        for i in 0..4 {
            assert!((m.r[i] - r_legacy[i]).abs() < EPS, "i={i}");
        }
        assert!(m.d.iter().all(|&x| x.abs() < EPS));
    }

    #[test]
    fn semantic_fixtures_stable_against_legacy() {
        let cases = [
            (0.0, 0.0, 0.0, 0.0, 1.0, [0.5, 0.5, 0.0]),
            (2.0, 0.25, 0.4, 1.0, 0.8, [0.1, -0.2, 0.3]),
            (4.0, 0.0, 0.9, 3.14, 1.0, [-0.6, 0.0, 0.4]),
        ];
        for (w, q, sigma, time, alpha, p) in cases {
            let m = semantic_motor(w, q, sigma, time, alpha);
            let r_composed = m.r;
            let legacy = legacy_rotor_apply_vector(r_composed, p);
            let pga = sandwich_point(m, p);
            assert!(
                approx_eq3(legacy, pga, EPS),
                "w={w} q={q} sigma={sigma}: legacy {legacy:?} pga {pga:?}"
            );
        }
    }

    #[test]
    fn motor_reverse_roundtrip_rotation() {
        let r = motor_rq(0.5, 0.2, 2.0, 1.0);
        let m = Motor::from_rotor(r);
        let p = [0.7, -0.1, 0.4];
        let forward = sandwich_point(m, p);
        let round = sandwich_point(motor_reverse(m), forward);
        assert!(approx_eq3(round, p, EPS));
    }

    #[test]
    fn motor_translate_shifts_point() {
        let v = [0.08, -0.03, 0.02];
        let p = [0.2, 0.1, -0.4];
        let out = sandwich_point(motor_translate(v), p);
        assert!(approx_eq3(out, [p[0] + v[0], p[1] + v[1], p[2] + v[2]], EPS));
    }

    #[test]
    fn bilateral_gate_off_matches_intrinsic_only() {
        let node = [0.3, 0.1, -0.2];
        let eye = [3.0, 1.0, 2.0];
        let m_pull = semantic_motor_phase2c(
            0.0, 2.0, 0.3, 0.5, 1.0, 0.9, 0.0, node, eye, STANDPOINT_SPECTATOR, 1.0,
        );
        let m_base =
            semantic_motor_intrinsic(0.0, 2.0, 0.3, 0.5, 1.0, 0.9, node, STANDPOINT_SPECTATOR, 1.0);
        for i in 0..4 {
            assert!((m_pull.r[i] - m_base.r[i]).abs() < EPS);
            assert!((m_pull.d[i] - m_base.d[i]).abs() < EPS);
        }
    }

    #[test]
    fn bilateral_pull_shifts_toward_eye() {
        let node = [0.0, 0.0, 0.0];
        let eye = [3.0, 0.0, 0.0];
        let p = [0.5, 0.0, 0.0];
        let without = sandwich_point(
            semantic_motor_intrinsic(0.0, 0.0, 0.0, 0.0, 0.0, 1.0, node, STANDPOINT_DID, 0.8),
            p,
        );
        let with = sandwich_point(
            semantic_motor_phase2c(
                0.0, 0.0, 0.0, 0.0, 1.0, 2.0, 2.0, node, eye, STANDPOINT_DID, 0.8,
            ),
            p,
        );
        assert!(with[0] > without[0]);
        assert!((with[1] - without[1]).abs() < EPS);
        assert!((with[2] - without[2]).abs() < EPS);
    }

    #[test]
    fn v_zero_regresses_to_pre_phase3_intrinsic() {
        let node = [0.2, -0.1, 0.3];
        let m_v0 = semantic_motor_intrinsic(0.0, 1.0, 0.2, 0.4, 1.5, 0.9, node, STANDPOINT_SPECTATOR, 1.0);
        let r_w = motor_rw(1.0, 0.9);
        let r_q = motor_rq(0.2, 0.4, 1.5, 0.9);
        let m_legacy = motor_mul(Motor::from_rotor(r_w), Motor::from_rotor(r_q));
        for i in 0..4 {
            assert!((m_v0.r[i] - m_legacy.r[i]).abs() < EPS, "r[{i}]");
            assert!((m_v0.d[i] - m_legacy.d[i]).abs() < EPS, "d[{i}]");
        }
    }

    #[test]
    fn cyclic_v_band_rotates_around_y() {
        let p = [0.5, 0.0, 0.0];
        let m = motor_v_band(1.5, p, 0.0, 0.25, 1.0);
        let out = sandwich_point(m, p);
        assert!(out[2].abs() > 0.01);
    }

    #[test]
    fn hyperbolic_v_band_spreads_radially() {
        let p = [0.4, 0.0, 0.0];
        let m = motor_v_band(2.5, p, 0.0, 0.0, 1.0);
        let out = sandwich_point(m, p);
        assert!(out[0] > p[0]);
    }

    #[test]
    fn boundary_v_band_snaps_toward_centroid() {
        let p = [0.8, 0.1, 0.0];
        let sigma = 0.125; // cluster 1
        let centroid = cluster_centroid_lattice(cluster_id_from_sigma(sigma));
        let m = motor_v_band(3.2, p, sigma, 0.0, 1.0);
        let out = sandwich_point(m, p);
        let blend = (3.2_f32 - 3.0).min(1.0);
        let expected = [
            p[0] + (centroid[0] - p[0]) * blend,
            p[1] + (centroid[1] - p[1]) * blend,
            p[2] + (centroid[2] - p[2]) * blend,
        ];
        assert!(approx_eq3(out, expected, EPS));
    }
}