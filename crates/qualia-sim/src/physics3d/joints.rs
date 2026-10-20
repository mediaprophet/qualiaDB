//! 6-DOF PGA [`Motor`] Joint Constraints (`joints.rs`).
//!
//! Implements zero-heap Extended Position-Based Dynamics (XPBD) joint constraints operating
//! directly on 3D PGA [`Motor`] poses:
//! - [`BallMotorJoint`]: 3-DOF spherical socket constraint locking local anchor points.
//! - [`RevoluteMotorJoint`]: 1-DOF hinge constraint with `[min_angle, max_angle]` limits.
//! - [`FixedMotorJoint`]: 0-DOF rigid weld constraint locking relative PGA [`Motor`] frame.
//! - [`SpringSuspensionJoint`]: Compliant 1-DOF linear spring-damper strut constraint.

use qualia_core_db::render::pga::Motor;

use crate::physics3d::xpbd_solver::RigidBody3D;

/// Spherical ball-and-socket joint locking two local anchor points in world space.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BallMotorJoint {
    pub body_a: u32,
    pub body_b: u32,
    pub local_anchor_a: [f32; 3],
    pub local_anchor_b: [f32; 3],
    /// XPBD compliance $\alpha \ge 0$ ($0.0$ = infinitely stiff).
    pub compliance: f32,
}

/// 1-DOF revolute hinge joint with angular limits and optional motor drive.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RevoluteMotorJoint {
    pub body_a: u32,
    pub body_b: u32,
    pub local_anchor_a: [f32; 3],
    pub local_anchor_b: [f32; 3],
    /// Local hinge axis on both bodies (unit vector).
    pub local_axis: [f32; 3],
    /// Minimum hinge angle in radians.
    pub min_angle_rad: f32,
    /// Maximum hinge angle in radians.
    pub max_angle_rad: f32,
    /// Target angular velocity in rad/s.
    pub motor_target_vel: f32,
    /// Maximum motor torque in N·m.
    pub max_motor_torque: f32,
    pub compliance: f32,
}

/// Rigid weld joint preserving a fixed relative PGA [`Motor`] offset from `body_a` to `body_b`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixedMotorJoint {
    pub body_a: u32,
    pub body_b: u32,
    /// Desired local pose of `body_b` in `body_a`'s frame: $M_B = M_A \otimes M_{\text{rel}}$.
    pub relative_motor: Motor,
    pub compliance: f32,
}

/// Compliant linear suspension strut connecting `body_a` (chassis) and `body_b` (wheel/bogie).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringSuspensionJoint {
    pub body_a: u32,
    pub body_b: u32,
    pub local_anchor_a: [f32; 3],
    pub local_anchor_b: [f32; 3],
    pub rest_length: f32,
    pub stiffness: f32,
    pub damping: f32,
}

/// Unified 6-DOF PGA [`Motor`] joint enum.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MotorJoint3D {
    Ball(BallMotorJoint),
    Revolute(RevoluteMotorJoint),
    Fixed(FixedMotorJoint),
    SpringSuspension(SpringSuspensionJoint),
}

/// Solves a single [`MotorJoint3D`] constraint over `bodies` for substep `dt`.
pub fn solve_motor_joint_xpbd(joint: &MotorJoint3D, bodies: &mut [RigidBody3D], dt: f32) {
    match *joint {
        MotorJoint3D::Ball(ref j) => {
            solve_anchor_distance(
                bodies,
                j.body_a as usize,
                j.body_b as usize,
                j.local_anchor_a,
                j.local_anchor_b,
                0.0,
                j.compliance,
                dt,
            );
        }
        MotorJoint3D::Revolute(ref j) => {
            solve_anchor_distance(
                bodies,
                j.body_a as usize,
                j.body_b as usize,
                j.local_anchor_a,
                j.local_anchor_b,
                0.0,
                j.compliance,
                dt,
            );
            let a = j.body_a as usize;
            let b = j.body_b as usize;
            if a < bodies.len() && b < bodies.len() && a != b {
                // Align hinge axes and clamp relative angular velocity to limits.
                let axis_a = bodies[a].pose.transform_vector(j.local_axis);
                let axis_b = bodies[b].pose.transform_vector(j.local_axis);
                let rel_angle = angle_around_axis(&bodies[a].pose, &bodies[b].pose, axis_a);

                if rel_angle < j.min_angle_rad || rel_angle > j.max_angle_rad {
                    let clamped = rel_angle.clamp(j.min_angle_rad, j.max_angle_rad);
                    let err = rel_angle - clamped;
                    if bodies[b].effective_inv_mass() > 0.0 {
                        bodies[b].angular_velocity[0] -= axis_b[0] * err / dt.max(1e-4) * 0.5;
                        bodies[b].angular_velocity[1] -= axis_b[1] * err / dt.max(1e-4) * 0.5;
                        bodies[b].angular_velocity[2] -= axis_b[2] * err / dt.max(1e-4) * 0.5;
                    }
                }

                if j.max_motor_torque > 0.0 && bodies[b].effective_inv_mass() > 0.0 {
                    let cur_w = dot3(bodies[b].angular_velocity, axis_a);
                    let dw = (j.motor_target_vel - cur_w).clamp(
                        -j.max_motor_torque * dt,
                        j.max_motor_torque * dt,
                    );
                    bodies[b].angular_velocity[0] += axis_a[0] * dw;
                    bodies[b].angular_velocity[1] += axis_a[1] * dw;
                    bodies[b].angular_velocity[2] += axis_a[2] * dw;
                }
            }
        }
        MotorJoint3D::Fixed(ref j) => {
            let a = j.body_a as usize;
            let b = j.body_b as usize;
            if a >= bodies.len() || b >= bodies.len() || a == b {
                return;
            }
            let target_pose = bodies[a].pose.motor_mul(&j.relative_motor).normalize();
            let w_b = bodies[b].effective_inv_mass();
            if w_b > 0.0 {
                let stiffness = (1.0 / (1.0 + j.compliance / (dt * dt).max(1e-6))).clamp(0.0, 1.0);
                bodies[b].pose = bodies[b].pose.sclerp(&target_pose, stiffness);
                bodies[b].linear_velocity = bodies[a].linear_velocity;
                bodies[b].angular_velocity = bodies[a].angular_velocity;
            }
        }
        MotorJoint3D::SpringSuspension(ref j) => {
            let a = j.body_a as usize;
            let b = j.body_b as usize;
            if a >= bodies.len() || b >= bodies.len() || a == b {
                return;
            }
            let pa = bodies[a].pose.transform_point(j.local_anchor_a);
            let pb = bodies[b].pose.transform_point(j.local_anchor_b);
            let diff = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
            let dist = (diff[0] * diff[0] + diff[1] * diff[1] + diff[2] * diff[2]).sqrt();
            if dist < 1e-6 {
                return;
            }
            let dir = [diff[0] / dist, diff[1] / dist, diff[2] / dist];
            let stretch = dist - j.rest_length;
            let rv = [
                bodies[b].linear_velocity[0] - bodies[a].linear_velocity[0],
                bodies[b].linear_velocity[1] - bodies[a].linear_velocity[1],
                bodies[b].linear_velocity[2] - bodies[a].linear_velocity[2],
            ];
            let v_rel = dot3(rv, dir);
            let force_mag = -j.stiffness * stretch - j.damping * v_rel;
            let impulse = [
                dir[0] * force_mag * dt,
                dir[1] * force_mag * dt,
                dir[2] * force_mag * dt,
            ];
            bodies[a].apply_linear_impulse([-impulse[0], -impulse[1], -impulse[2]]);
            bodies[b].apply_linear_impulse(impulse);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn solve_anchor_distance(
    bodies: &mut [RigidBody3D],
    a: usize,
    b: usize,
    local_a: [f32; 3],
    local_b: [f32; 3],
    target_dist: f32,
    compliance: f32,
    dt: f32,
) {
    if a >= bodies.len() || b >= bodies.len() || a == b {
        return;
    }
    let w_a = bodies[a].effective_inv_mass();
    let w_b = bodies[b].effective_inv_mass();
    let w_sum = w_a + w_b;
    if w_sum <= 0.0 {
        return;
    }

    let pa = bodies[a].pose.transform_point(local_a);
    let pb = bodies[b].pose.transform_point(local_b);
    let diff = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
    let dist = (diff[0] * diff[0] + diff[1] * diff[1] + diff[2] * diff[2]).sqrt();
    let c = dist - target_dist;
    if c.abs() < 1e-6 {
        return;
    }
    let dir = if dist > 1e-6 {
        [diff[0] / dist, diff[1] / dist, diff[2] / dist]
    } else {
        [0.0, 1.0, 0.0]
    };

    let alpha_tilde = compliance / (dt * dt).max(1e-8);
    let delta_lambda = -c / (w_sum + alpha_tilde);

    if w_a > 0.0 {
        let corr_a = -w_a * delta_lambda;
        bodies[a].translate_world([dir[0] * corr_a, dir[1] * corr_a, dir[2] * corr_a]);
    }
    if w_b > 0.0 {
        let corr_b = w_b * delta_lambda;
        bodies[b].translate_world([dir[0] * corr_b, dir[1] * corr_b, dir[2] * corr_b]);
    }
}

#[inline]
fn angle_around_axis(pose_a: &Motor, pose_b: &Motor, axis_world: [f32; 3]) -> f32 {
    let ref_perp = if axis_world[0].abs() < 0.8 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let va = pose_a.transform_vector(ref_perp);
    let vb = pose_b.transform_vector(ref_perp);
    let cross = [
        va[1] * vb[2] - va[2] * vb[1],
        va[2] * vb[0] - va[0] * vb[2],
        va[0] * vb[1] - va[1] * vb[0],
    ];
    let sin_a = dot3(cross, axis_world);
    let cos_a = dot3(va, vb);
    sin_a.atan2(cos_a)
}

#[inline]
fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics3d::gjk_epa::ColliderShape3D;
    use qualia_core_db::render::pga::motor_translate;

    #[test]
    fn ball_motor_joint_pulls_separated_anchors_together() {
        let b0 = RigidBody3D::new_static(
            Motor::identity(),
            ColliderShape3D::Sphere { radius: 0.2 },
        );
        let b1 = RigidBody3D::new_dynamic(
            motor_translate([0.0, -2.0, 0.0]),
            1.0,
            ColliderShape3D::Sphere { radius: 0.2 },
        );
        let mut bodies = [b0, b1];
        let joint = MotorJoint3D::Ball(BallMotorJoint {
            body_a: 0,
            body_b: 1,
            local_anchor_a: [0.0, 0.0, 0.0],
            local_anchor_b: [0.0, 1.0, 0.0],
            compliance: 0.0,
        });

        solve_motor_joint_xpbd(&joint, &mut bodies, 1.0 / 64.0);
        // Anchor B is [0, 1, 0] in body 1, so body 1's center should snap to [0, -1, 0].
        let pos1 = bodies[1].pose.translation();
        assert!((pos1[1] + 1.0).abs() < 1e-4, "expected y = -1.0, got {}", pos1[1]);
    }
}
