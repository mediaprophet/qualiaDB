//! Zero-Heap Analytical Two-Bone Motor IK & Constrained Motor FABRIK (`ik_motor.rs`).
//!
//! Operates directly on 3D Projective Geometric Algebra (PGA $\mathcal{G}_{3,0,1}$) `Motor`
//! chains with zero heap allocations (`HotZeroHeap`):
//! - [`solve_two_bone_motor_ik_zero_heap`]: closed-form Law of Cosines + pole-target plane
//!   alignment for biped/quadruped limbs and hand grasping.
//! - [`solve_constrained_fabrik_motor_ik_zero_heap`]: stack-resident (`MAX_IK_CHAIN = 16`)
//!   Forward-And-Backward Reaching Inverse Kinematics with spherical cone and hinge joint limits.

use super::{Motor, SkeletalError};

/// Maximum joints in a single constrained FABRIK chain (`HotZeroHeap` stack bound).
pub const MAX_IK_CHAIN: usize = 16;

/// Angular constraint applied to a joint during [`solve_constrained_fabrik_motor_ik_zero_heap`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum JointConstraint {
    /// Full $SO(3)$ rotational freedom.
    #[default]
    Unconstrained,
    /// Conical swing limit relative to the parent bone direction.
    ConeLimit {
        /// Maximum half-angle of the cone in radians (`(0, \pi]`).
        max_half_angle_rad: f32,
    },
    /// Single-degree-of-freedom hinge rotation around `axis` (in parent/local space).
    HingeLimit {
        /// Unit hinge axis `[ax, ay, az]`.
        axis: [f32; 3],
        /// Minimum hinge angle in radians.
        min_angle_rad: f32,
        /// Maximum hinge angle in radians.
        max_angle_rad: f32,
    },
}

/// Output receipt for [`solve_two_bone_motor_ik_zero_heap`].
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TwoBoneIkResult {
    /// Updated world-space `Motor`s for `[root, mid, end]`.
    pub world_motors: [Motor; 3],
    /// Updated local-space `Motor`s (`[root_world, root^-1 * mid, mid^-1 * end]`).
    pub local_motors: [Motor; 3],
    /// Interior knee/elbow angle in radians (`\pi` = straight limb).
    pub knee_angle_rad: f32,
    /// Euclidean distance from solved end-effector to `target_pos`.
    pub target_error: f32,
}

#[inline]
fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
fn add3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

#[inline]
fn scale3(v: [f64; 3], s: f64) -> [f64; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

#[inline]
fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[inline]
fn len3(v: [f64; 3]) -> f64 {
    dot3(v, v).sqrt()
}

#[inline]
fn normalize3_or(v: [f64; 3], fallback: [f64; 3]) -> [f64; 3] {
    let l = len3(v);
    if l > 1e-9 {
        [v[0] / l, v[1] / l, v[2] / l]
    } else {
        fallback
    }
}

#[inline]
fn orthogonal_unit(u: [f64; 3]) -> [f64; 3] {
    let ref_axis = if u[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    normalize3_or(cross3(u, ref_axis), [0.0, 0.0, 1.0])
}

/// Shortest-arc unit quaternion `[w, x, y, z]` rotating unit vector `from` to unit vector `to`.
#[inline]
fn quat_from_unit_vecs(from: [f64; 3], to: [f64; 3]) -> [f64; 4] {
    let d = dot3(from, to).clamp(-1.0, 1.0);
    if d > 1.0 - 1e-9 {
        return [1.0, 0.0, 0.0, 0.0];
    }
    if d < -1.0 + 1e-9 {
        let axis = orthogonal_unit(from);
        return [0.0, axis[0], axis[1], axis[2]];
    }
    let c = cross3(from, to);
    let w = ((1.0 + d) * 0.5).sqrt();
    let inv = 1.0 / (2.0 * w);
    let q = [w, c[0] * inv, c[1] * inv, c[2] * inv];
    let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    [q[0] / n, q[1] / n, q[2] / n, q[3] / n]
}

#[inline]
fn quat_mul(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    [
        a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
        a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
        a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
        a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0],
    ]
}

#[inline]
fn rotate_vec_by_axis_angle(v: [f64; 3], axis: [f64; 3], angle: f64) -> [f64; 3] {
    let (s, c) = angle.sin_cos();
    let k_dot_v = dot3(axis, v);
    let k_cross_v = cross3(axis, v);
    [
        v[0] * c + k_cross_v[0] * s + axis[0] * k_dot_v * (1.0 - c),
        v[1] * c + k_cross_v[1] * s + axis[1] * k_dot_v * (1.0 - c),
        v[2] * c + k_cross_v[2] * s + axis[2] * k_dot_v * (1.0 - c),
    ]
}

/// Analytically solves two-bone inverse kinematics (e.g., hip–knee–ankle or shoulder–elbow–wrist)
/// directly on PGA `Motor`s using the Law of Cosines and a pole-vector bend plane (`HotZeroHeap`).
///
/// - `root_world`, `mid_world`, `end_world`: initial world-space `Motor`s of the 3 joints.
/// - `l1`, `l2`: upper and lower bone lengths (if `<= 0.0`, measured from initial joint positions).
/// - `target_pos`: desired world-space `[x, y, z]` position for the end joint.
/// - `pole_target`: world-space `[x, y, z]` point guiding the knee/elbow bend plane.
/// - `min_knee_angle_rad`, `max_knee_angle_rad`: clamp bounds `[min, max]` in `(0, \pi]` for the
///   interior angle between the upper and lower bones (`\pi` = fully extended straight limb).
pub fn solve_two_bone_motor_ik_zero_heap(
    root_world: Motor,
    mid_world: Motor,
    end_world: Motor,
    l1: f32,
    l2: f32,
    target_pos: [f32; 3],
    pole_target: [f32; 3],
    min_knee_angle_rad: f32,
    max_knee_angle_rad: f32,
) -> Result<TwoBoneIkResult, SkeletalError> {
    if target_pos.iter().chain(pole_target.iter()).any(|v| !v.is_finite()) {
        return Err(SkeletalError::InvalidInput);
    }

    let (root_r0, p_root) = root_world.to_rotation_translation();
    let (mid_r0, p_mid0) = mid_world.to_rotation_translation();
    let (end_r0, p_end0) = end_world.to_rotation_translation();

    let len1 = if l1 > 1e-5 && l1.is_finite() {
        l1 as f64
    } else {
        len3(sub3(p_mid0, p_root)).max(1e-4)
    };
    let len2 = if l2 > 1e-5 && l2.is_finite() {
        l2 as f64
    } else {
        len3(sub3(p_end0, p_mid0)).max(1e-4)
    };

    let p_target = [
        target_pos[0] as f64,
        target_pos[1] as f64,
        target_pos[2] as f64,
    ];
    let p_pole = [
        pole_target[0] as f64,
        pole_target[1] as f64,
        pole_target[2] as f64,
    ];

    let min_angle = (min_knee_angle_rad as f64).clamp(0.01, std::f64::consts::PI - 1e-4);
    let max_angle = (max_knee_angle_rad as f64).clamp(min_angle, std::f64::consts::PI);

    // Distance bounds implied by interior knee angle [min_angle, max_angle]:
    // D^2 = L1^2 + L2^2 - 2 L1 L2 cos(theta_interior)
    let min_reach = (len1 * len1 + len2 * len2 - 2.0 * len1 * len2 * min_angle.cos())
        .max(1e-8)
        .sqrt();
    let max_reach = (len1 * len1 + len2 * len2 - 2.0 * len1 * len2 * max_angle.cos())
        .max(min_reach * min_reach)
        .sqrt();

    let to_target = sub3(p_target, p_root);
    let raw_dist = len3(to_target);
    let u_dir = normalize3_or(to_target, [0.0, -1.0, 0.0]);
    let clamped_dist = raw_dist.clamp(min_reach, max_reach);

    // Law of cosines for root elevation angle alpha
    let cos_alpha = ((len1 * len1 + clamped_dist * clamped_dist - len2 * len2)
        / (2.0 * len1 * clamped_dist))
        .clamp(-1.0, 1.0);
    let sin_alpha = (1.0 - cos_alpha * cos_alpha).max(0.0).sqrt();

    // Interior knee angle theta
    let cos_theta = ((len1 * len1 + len2 * len2 - clamped_dist * clamped_dist)
        / (2.0 * len1 * len2))
        .clamp(-1.0, 1.0);
    let knee_angle = cos_theta.acos() as f32;

    // Pole plane direction perpendicular to u_dir
    let to_pole = sub3(p_pole, p_root);
    let pole_perp = sub3(to_pole, scale3(u_dir, dot3(to_pole, u_dir)));
    let n_pole = normalize3_or(pole_perp, orthogonal_unit(u_dir));

    // Solved mid (knee/elbow) and end (ankle/wrist) positions
    let p_mid_new = add3(
        p_root,
        add3(
            scale3(u_dir, len1 * cos_alpha),
            scale3(n_pole, len1 * sin_alpha),
        ),
    );
    let lower_dir = normalize3_or(sub3(p_target, p_mid_new), u_dir);
    let p_end_new = add3(p_mid_new, scale3(lower_dir, len2));

    // Update world orientations using shortest-arc rotors from initial bone directions
    let init_upper_dir = normalize3_or(sub3(p_mid0, p_root), [0.0, -1.0, 0.0]);
    let new_upper_dir = normalize3_or(sub3(p_mid_new, p_root), init_upper_dir);
    let q_upper = quat_from_unit_vecs(init_upper_dir, new_upper_dir);
    let new_root_r = quat_mul(q_upper, root_r0);

    let init_lower_dir = normalize3_or(sub3(p_end0, p_mid0), init_upper_dir);
    let q_lower = quat_from_unit_vecs(init_lower_dir, lower_dir);
    let new_mid_r = quat_mul(q_lower, mid_r0);

    let m_root_new = Motor::from_rotation_translation(new_root_r, p_root);
    let m_mid_new = Motor::from_rotation_translation(new_mid_r, p_mid_new);
    let m_end_new = Motor::from_rotation_translation(end_r0, p_end_new);

    let local_root = m_root_new;
    let local_mid = m_root_new.conjugate().mul(&m_mid_new).normalize();
    let local_end = m_mid_new.conjugate().mul(&m_end_new).normalize();

    let target_error = len3(sub3(p_end_new, p_target)) as f32;

    Ok(TwoBoneIkResult {
        world_motors: [m_root_new, m_mid_new, m_end_new],
        local_motors: [local_root, local_mid, local_end],
        knee_angle_rad: knee_angle,
        target_error,
    })
}

/// Applies `constraint` to a child bone unit direction `dir` relative to `parent_dir`.
#[inline]
fn apply_joint_constraint_dir(
    dir: [f64; 3],
    parent_dir: [f64; 3],
    constraint: JointConstraint,
) -> [f64; 3] {
    match constraint {
        JointConstraint::Unconstrained => dir,
        JointConstraint::ConeLimit { max_half_angle_rad } => {
            let max_angle = (max_half_angle_rad as f64).clamp(1e-4, std::f64::consts::PI);
            let cos_angle = dot3(parent_dir, dir).clamp(-1.0, 1.0);
            let angle = cos_angle.acos();
            if angle <= max_angle {
                return dir;
            }
            let axis = normalize3_or(cross3(parent_dir, dir), orthogonal_unit(parent_dir));
            normalize3_or(rotate_vec_by_axis_angle(parent_dir, axis, max_angle), parent_dir)
        }
        JointConstraint::HingeLimit {
            axis,
            min_angle_rad,
            max_angle_rad,
        } => {
            let h_axis = normalize3_or(
                [axis[0] as f64, axis[1] as f64, axis[2] as f64],
                [1.0, 0.0, 0.0],
            );
            // Reference direction in the plane perpendicular to h_axis
            let ref_proj = sub3(parent_dir, scale3(h_axis, dot3(parent_dir, h_axis)));
            let ref_dir = normalize3_or(ref_proj, orthogonal_unit(h_axis));

            // Project candidate direction onto the hinge plane
            let cand_proj = sub3(dir, scale3(h_axis, dot3(dir, h_axis)));
            let cand_dir = normalize3_or(cand_proj, ref_dir);

            let sin_a = dot3(cross3(ref_dir, cand_dir), h_axis);
            let cos_a = dot3(ref_dir, cand_dir);
            let angle = sin_a.atan2(cos_a);
            let lo = (min_angle_rad as f64).min(max_angle_rad as f64);
            let hi = (min_angle_rad as f64).max(max_angle_rad as f64);
            let clamped = angle.clamp(lo, hi);
            normalize3_or(rotate_vec_by_axis_angle(ref_dir, h_axis, clamped), ref_dir)
        }
    }
}

/// Solves multi-joint Constrained Forward-And-Backward Reaching Inverse Kinematics (FABRIK)
/// directly on a chain of up to `MAX_IK_CHAIN = 16` PGA `Motor`s with zero heap allocations (`HotZeroHeap`).
///
/// Outputs both `out_world_motors` and `out_local_motors` (`out_local_motors[0] = out_world_motors[0]`,
/// and `out_local_motors[i] = out_world_motors[i-1]^-1 * out_world_motors[i]`), returning the
/// final Euclidean distance from the end-effector to `target_pos`.
pub fn solve_constrained_fabrik_motor_ik_zero_heap(
    chain_world_motors: &[Motor],
    constraints: &[JointConstraint],
    target_pos: [f32; 3],
    tolerance: f32,
    max_iterations: usize,
    out_world_motors: &mut [Motor],
    out_local_motors: &mut [Motor],
) -> Result<f32, SkeletalError> {
    let n = chain_world_motors.len();
    if n < 2 || n > MAX_IK_CHAIN || target_pos.iter().any(|v| !v.is_finite()) {
        return Err(SkeletalError::InvalidInput);
    }
    if out_world_motors.len() < n || out_local_motors.len() < n {
        return Err(SkeletalError::OutputTooSmall);
    }

    let mut pos = [[0.0f64; 3]; MAX_IK_CHAIN];
    let mut init_pos = [[0.0f64; 3]; MAX_IK_CHAIN];
    let mut init_rot = [[1.0f64, 0.0, 0.0, 0.0]; MAX_IK_CHAIN];
    let mut lengths = [0.0f64; MAX_IK_CHAIN];

    for i in 0..n {
        let (r, t) = chain_world_motors[i].to_rotation_translation();
        pos[i] = t;
        init_pos[i] = t;
        init_rot[i] = r;
    }
    for i in 0..(n - 1) {
        lengths[i] = len3(sub3(pos[i + 1], pos[i])).max(1e-5);
    }

    let root_anchor = pos[0];
    let root_parent_dir = normalize3_or(sub3(init_pos[1], init_pos[0]), [0.0, 1.0, 0.0]);
    let target = [
        target_pos[0] as f64,
        target_pos[1] as f64,
        target_pos[2] as f64,
    ];
    let tol = (tolerance.max(1e-5)) as f64;
    let iters = max_iterations.clamp(1, 64);

    for _ in 0..iters {
        let err = len3(sub3(pos[n - 1], target));
        if err <= tol {
            break;
        }

        // 1. Backward reaching pass: pull from end-effector to root
        pos[n - 1] = target;
        for i in (0..(n - 1)).rev() {
            let dir = normalize3_or(sub3(pos[i], pos[i + 1]), [0.0, -1.0, 0.0]);
            pos[i] = add3(pos[i + 1], scale3(dir, lengths[i]));
        }

        // 2. Forward reaching pass: anchor root and enforce JointConstraint per segment
        pos[0] = root_anchor;
        let mut parent_dir = root_parent_dir;
        for i in 0..(n - 1) {
            let raw_dir = normalize3_or(sub3(pos[i + 1], pos[i]), parent_dir);
            let constraint = constraints
                .get(i)
                .copied()
                .unwrap_or(JointConstraint::Unconstrained);
            let constrained_dir = apply_joint_constraint_dir(raw_dir, parent_dir, constraint);
            pos[i + 1] = add3(pos[i], scale3(constrained_dir, lengths[i]));
            parent_dir = constrained_dir;
        }
    }

    // Reconstruct world and local Motors from solved joint positions
    for i in 0..(n - 1) {
        let old_dir = normalize3_or(sub3(init_pos[i + 1], init_pos[i]), [0.0, 1.0, 0.0]);
        let new_dir = normalize3_or(sub3(pos[i + 1], pos[i]), old_dir);
        let dq = quat_from_unit_vecs(old_dir, new_dir);
        let new_r = quat_mul(dq, init_rot[i]);
        out_world_motors[i] = Motor::from_rotation_translation(new_r, pos[i]);
    }
    out_world_motors[n - 1] = Motor::from_rotation_translation(init_rot[n - 1], pos[n - 1]);

    out_local_motors[0] = out_world_motors[0];
    for i in 1..n {
        out_local_motors[i] = out_world_motors[i - 1]
            .conjugate()
            .mul(&out_world_motors[i])
            .normalize();
    }

    Ok(len3(sub3(pos[n - 1], target)) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

    #[test]
    fn analytical_two_bone_motor_ik_reaches_target_and_aligns_pole_zero_heap() {
        let root = Motor::from_translation(0.0, 2.0, 0.0);
        let mid = Motor::from_translation(0.0, 1.0, 0.0);
        let end = Motor::from_translation(0.0, 0.0, 0.0);

        let target = [0.0f32, 0.8, 0.6]; // Distance from (0,2,0) is sqrt(1.44 + 0.36) = sqrt(1.8) < 2.0
        let pole = [0.0f32, 1.0, 2.0];   // Forward +Z knee pole

        let mut res = None;
        assert_zero_alloc("two_bone_motor_ik", || {
            res = Some(
                solve_two_bone_motor_ik_zero_heap(
                    root,
                    mid,
                    end,
                    1.0,
                    1.0,
                    target,
                    pole,
                    0.1,
                    std::f32::consts::PI,
                )
                .unwrap(),
            );
        });
        let res = res.unwrap();
        assert!(res.target_error < 1e-4, "target_error={}", res.target_error);

        // Verify knee bent toward +Z pole
        let (_, mid_t) = res.world_motors[1].to_rotation_translation();
        assert!(mid_t[2] > 0.4, "knee must bend toward +Z pole: {mid_t:?}");

        // Verify forward composition of local_motors matches world_motors
        let composed_mid = res.local_motors[0].mul(&res.local_motors[1]).normalize();
        let composed_end = composed_mid.mul(&res.local_motors[2]).normalize();
        let (_, end_t) = composed_end.to_rotation_translation();
        for axis in 0..3 {
            assert!((end_t[axis] - target[axis] as f64).abs() < 1e-4);
        }
    }

    #[test]
    fn constrained_fabrik_motor_ik_respects_cone_and_hinge_limits_zero_heap() {
        let chain = [
            Motor::from_translation(0.0, 0.0, 0.0),
            Motor::from_translation(0.0, 1.0, 0.0),
            Motor::from_translation(0.0, 2.0, 0.0),
            Motor::from_translation(0.0, 3.0, 0.0),
        ];
        let constraints = [
            JointConstraint::ConeLimit {
                max_half_angle_rad: 0.5,
            },
            JointConstraint::HingeLimit {
                axis: [1.0, 0.0, 0.0],
                min_angle_rad: -0.8,
                max_angle_rad: 0.8,
            },
            JointConstraint::Unconstrained,
            JointConstraint::Unconstrained,
        ];
        let mut world_out = [Motor::identity(); 4];
        let mut local_out = [Motor::identity(); 4];

        assert_zero_alloc("constrained_fabrik_motor_ik", || {
            let err = solve_constrained_fabrik_motor_ik_zero_heap(
                &chain,
                &constraints,
                [0.0, 2.2, 1.2],
                1e-3,
                16,
                &mut world_out,
                &mut local_out,
            )
            .unwrap();
            assert!(err.is_finite());
        });

        // Root must remain anchored at (0, 0, 0)
        let (_, p0) = world_out[0].to_rotation_translation();
        assert!(len3(p0) < 1e-6);

        // Segment lengths must be preserved at 1.0
        for i in 0..3 {
            let (_, pa) = world_out[i].to_rotation_translation();
            let (_, pb) = world_out[i + 1].to_rotation_translation();
            let seg_len = len3(sub3(pb, pa));
            assert!((seg_len - 1.0).abs() < 1e-4, "seg {i} len={seg_len}");
        }

        // First segment angle relative to +Y must respect ConeLimit <= 0.5 rad
        let (_, p1) = world_out[1].to_rotation_translation();
        let dir0 = normalize3_or(p1, [0.0, 1.0, 0.0]);
        let angle0 = dot3(dir0, [0.0, 1.0, 0.0]).clamp(-1.0, 1.0).acos();
        assert!(angle0 <= 0.5 + 1e-3, "cone limit violated: {angle0}");
    }
}
