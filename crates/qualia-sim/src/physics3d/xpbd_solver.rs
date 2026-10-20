//! 64 Hz Deterministic XPBD Rigid-Body Solver (`xpbd_solver.rs`).
//!
//! Implements `RigidBody3D` (`Dynamic`, `Static`, `KinematicPosition`, `KinematicVelocity`) with
//! PGA [`Motor`] poses, inverse mass and inertia tensor, restitution, Coulomb friction, sleeping
//! states, and an Extended Position-Based Dynamics (XPBD) + sequential impulse contact solver.

use qualia_core_db::render::pga::{
    motor_mul, motor_translate, rotor_from_axis_angle, Motor,
};

use crate::physics3d::broadphase_bvh::BroadphaseBvh;
use crate::physics3d::gjk_epa::{compute_narrowphase_contact, ColliderShape3D};
use crate::physics3d::joints::{solve_motor_joint_xpbd, MotorJoint3D};
use crate::physics3d::manifold::ContactManifold3D;

/// Kinematic / dynamic simulation class of a 3D rigid body.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RigidBodyType3D {
    /// Simulated under forces, gravity, contacts, and joint constraints.
    #[default]
    Dynamic = 0,
    /// Infinite-mass immovable world collider.
    Static = 1,
    /// Driven by target PGA `Motor` positions (e.g. moving platforms or animated kinematic bones).
    KinematicPosition = 2,
    /// Driven by prescribed linear/angular velocities with infinite mass.
    KinematicVelocity = 3,
}

/// 3D rigid body with PGA [`Motor`] pose and physical properties (`HotZeroHeap`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigidBody3D {
    pub body_type: RigidBodyType3D,
    /// Current world-space PGA [`Motor`] pose.
    pub pose: Motor,
    /// Previous substep world-space PGA [`Motor`] pose (for XPBD velocity update).
    pub prev_pose: Motor,
    /// World-space linear velocity `[vx, vy, vz]` in m/s.
    pub linear_velocity: [f32; 3],
    /// World-space angular velocity `[wx, wy, wz]` in rad/s.
    pub angular_velocity: [f32; 3],
    /// Inverse mass $1 / m$ in $\text{kg}^{-1}$ (`0.0` for `Static` / `Kinematic`).
    pub inv_mass: f32,
    /// Principal local inverse inertia diagonal $[I_{xx}^{-1}, I_{yy}^{-1}, I_{zz}^{-1}]$.
    pub inv_inertia_local: [f32; 3],
    /// Coefficient of restitution $e \in [0.0, 1.0]$.
    pub restitution: f32,
    /// Coefficient of Coulomb friction $\mu \in [0.0, 2.0]$.
    pub friction: f32,
    /// Accumulated external force `[fx, fy, fz]` (Newtons) cleared at the end of each step.
    pub external_force: [f32; 3],
    /// Accumulated external torque `[tx, ty, tz]` (N·m) cleared at the end of each step.
    pub external_torque: [f32; 3],
    /// Associated 3D collision shape.
    pub collider_shape: ColliderShape3D,
    /// True if the body has fallen below the kinetic energy sleep threshold.
    pub is_sleeping: bool,
    /// Seconds spent below the sleep velocity threshold.
    pub sleep_timer: f32,
}

impl Default for RigidBody3D {
    fn default() -> Self {
        Self::new_dynamic(Motor::identity(), 1.0, ColliderShape3D::Sphere { radius: 0.5 })
    }
}

impl RigidBody3D {
    /// Creates a dynamic 3D rigid body with automatic box/sphere inertia estimation.
    pub fn new_dynamic(pose: Motor, mass_kg: f32, collider_shape: ColliderShape3D) -> Self {
        let inv_mass = if mass_kg > 1e-6 { 1.0 / mass_kg } else { 0.0 };
        let aabb = collider_shape.world_aabb(&Motor::identity());
        let he = aabb.half_extents();
        let lx = (2.0 * he[0]).max(0.1);
        let ly = (2.0 * he[1]).max(0.1);
        let lz = (2.0 * he[2]).max(0.1);
        let ix = (mass_kg / 12.0) * (ly * ly + lz * lz);
        let iy = (mass_kg / 12.0) * (lx * lx + lz * lz);
        let iz = (mass_kg / 12.0) * (lx * lx + ly * ly);
        Self {
            body_type: RigidBodyType3D::Dynamic,
            pose,
            prev_pose: pose,
            linear_velocity: [0.0; 3],
            angular_velocity: [0.0; 3],
            inv_mass,
            inv_inertia_local: [1.0 / ix.max(1e-4), 1.0 / iy.max(1e-4), 1.0 / iz.max(1e-4)],
            restitution: 0.25,
            friction: 0.60,
            external_force: [0.0; 3],
            external_torque: [0.0; 3],
            collider_shape,
            is_sleeping: false,
            sleep_timer: 0.0,
        }
    }

    /// Creates a static immovable 3D collider body.
    pub fn new_static(pose: Motor, collider_shape: ColliderShape3D) -> Self {
        Self {
            body_type: RigidBodyType3D::Static,
            pose,
            prev_pose: pose,
            linear_velocity: [0.0; 3],
            angular_velocity: [0.0; 3],
            inv_mass: 0.0,
            inv_inertia_local: [0.0; 3],
            restitution: 0.25,
            friction: 0.70,
            external_force: [0.0; 3],
            external_torque: [0.0; 3],
            collider_shape,
            is_sleeping: true,
            sleep_timer: 10.0,
        }
    }

    /// Effective inverse mass (`0.0` if non-dynamic).
    #[inline]
    pub fn effective_inv_mass(&self) -> f32 {
        if self.body_type == RigidBodyType3D::Dynamic {
            self.inv_mass
        } else {
            0.0
        }
    }

    /// Applies an instantaneous world-space linear impulse $\mathbf{J}$ (N·s) at the center of mass.
    #[inline]
    pub fn apply_linear_impulse(&mut self, impulse: [f32; 3]) {
        let im = self.effective_inv_mass();
        if im > 0.0 {
            self.is_sleeping = false;
            self.sleep_timer = 0.0;
            self.linear_velocity[0] += impulse[0] * im;
            self.linear_velocity[1] += impulse[1] * im;
            self.linear_velocity[2] += impulse[2] * im;
        }
    }

    /// Applies an instantaneous world-space impulse $\mathbf{J}$ at `world_point`, generating both
    /// linear and angular velocity changes.
    #[inline]
    pub fn apply_impulse_at_point(&mut self, impulse: [f32; 3], world_point: [f32; 3]) {
        if self.body_type != RigidBodyType3D::Dynamic || self.inv_mass <= 0.0 {
            return;
        }
        self.apply_linear_impulse(impulse);
        let com = self.pose.translation();
        let r = [
            world_point[0] - com[0],
            world_point[1] - com[1],
            world_point[2] - com[2],
        ];
        let tau = cross3(r, impulse);
        self.angular_velocity[0] += tau[0] * self.inv_inertia_local[0];
        self.angular_velocity[1] += tau[1] * self.inv_inertia_local[1];
        self.angular_velocity[2] += tau[2] * self.inv_inertia_local[2];
    }

    /// Integrates translation and angular velocity into the PGA [`Motor`] pose over `dt`.
    #[inline]
    pub fn integrate_pose(&mut self, dt: f32) {
        self.prev_pose = self.pose;
        let dx = [
            self.linear_velocity[0] * dt,
            self.linear_velocity[1] * dt,
            self.linear_velocity[2] * dt,
        ];
        let t_motor = motor_translate(dx);
        let mut next = motor_mul(t_motor, self.pose);

        let w_len = len3(self.angular_velocity);
        if w_len > 1e-6 {
            let axis = [
                self.angular_velocity[0] / w_len,
                self.angular_velocity[1] / w_len,
                self.angular_velocity[2] / w_len,
            ];
            let r_delta = Motor::from_rotor(rotor_from_axis_angle(axis, w_len * dt));
            next = motor_mul(next, r_delta);
        }
        self.pose = next.normalize();
    }

    /// Shifts the rigid body's world position by `delta` using a PGA translation motor.
    #[inline]
    pub fn translate_world(&mut self, delta: [f32; 3]) {
        self.pose = motor_mul(motor_translate(delta), self.pose).normalize();
    }
}

/// Steps a slice of [`RigidBody3D`] bodies, contact manifolds, and [`MotorJoint3D`] constraints
/// by one fixed timestep `dt` (typically $1/64 = 0.015625\text{ s}$) using `substeps` XPBD iterations.
pub fn step_xpbd_world<const MAX_LEAVES: usize>(
    bodies: &mut [RigidBody3D],
    joints: &mut [MotorJoint3D],
    manifolds: &mut [ContactManifold3D],
    manifold_count: &mut usize,
    bvh: &mut BroadphaseBvh<MAX_LEAVES>,
    gravity: [f32; 3],
    dt: f32,
    substeps: usize,
) {
    let n_bodies = bodies.len().min(MAX_LEAVES);
    if n_bodies == 0 || dt <= 0.0 {
        return;
    }
    let steps = substeps.max(1);
    let sub_dt = dt / (steps as f32);

    // 1. Rebuild / update BVH tight AABBs.
    *bvh = BroadphaseBvh::new(0.10);
    let mut i = 0usize;
    while i < n_bodies {
        let aabb = bodies[i].collider_shape.world_aabb(&bodies[i].pose);
        let _ = bvh.insert(i as u32, aabb);
        i += 1;
    }

    // 2. Substep XPBD loop.
    let mut s = 0usize;
    while s < steps {
        s += 1;

        // 2a. Integrate external forces + gravity into velocities and predict poses.
        let mut bi = 0usize;
        while bi < n_bodies {
            let b = &mut bodies[bi];
            if b.body_type == RigidBodyType3D::Dynamic && !b.is_sleeping {
                let im = b.inv_mass;
                b.linear_velocity[0] += (gravity[0] + b.external_force[0] * im) * sub_dt;
                b.linear_velocity[1] += (gravity[1] + b.external_force[1] * im) * sub_dt;
                b.linear_velocity[2] += (gravity[2] + b.external_force[2] * im) * sub_dt;

                b.angular_velocity[0] += b.external_torque[0] * b.inv_inertia_local[0] * sub_dt;
                b.angular_velocity[1] += b.external_torque[1] * b.inv_inertia_local[1] * sub_dt;
                b.angular_velocity[2] += b.external_torque[2] * b.inv_inertia_local[2] * sub_dt;

                b.integrate_pose(sub_dt);
                let _ = bvh.update(bi, b.collider_shape.world_aabb(&b.pose));
            } else if b.body_type == RigidBodyType3D::KinematicVelocity {
                b.integrate_pose(sub_dt);
                let _ = bvh.update(bi, b.collider_shape.world_aabb(&b.pose));
            }
            bi += 1;
        }

        // 2b. Broadphase + Narrowphase contact detection & XPBD positional + impulse resolution.
        let mut pairs = [(0u32, 0u32); 256];
        let pair_count = bvh.collect_overlapping_pairs(&mut pairs);
        *manifold_count = 0;

        let mut p_idx = 0usize;
        while p_idx < pair_count {
            let (ia, ib) = pairs[p_idx];
            p_idx += 1;
            let a = ia as usize;
            let b = ib as usize;
            if a >= n_bodies || b >= n_bodies {
                continue;
            }
            let w_a = bodies[a].effective_inv_mass();
            let w_b = bodies[b].effective_inv_mass();
            if w_a + w_b <= 0.0 {
                continue;
            }

            if let Some(contact) = compute_narrowphase_contact(
                &bodies[a].collider_shape,
                &bodies[a].pose,
                &bodies[b].collider_shape,
                &bodies[b].pose,
            ) {
                let w_sum = w_a + w_b;
                let n = contact.normal;
                let pen = contact.penetration;

                // XPBD positional depenetration
                if pen > 1e-5 {
                    let corr = pen / w_sum;
                    if w_a > 0.0 {
                        bodies[a].translate_world([
                            -n[0] * corr * w_a,
                            -n[1] * corr * w_a,
                            -n[2] * corr * w_a,
                        ]);
                    }
                    if w_b > 0.0 {
                        bodies[b].translate_world([
                            n[0] * corr * w_b,
                            n[1] * corr * w_b,
                            n[2] * corr * w_b,
                        ]);
                    }
                }

                // Velocity restitution & Coulomb friction impulse
                let rv = [
                    bodies[b].linear_velocity[0] - bodies[a].linear_velocity[0],
                    bodies[b].linear_velocity[1] - bodies[a].linear_velocity[1],
                    bodies[b].linear_velocity[2] - bodies[a].linear_velocity[2],
                ];
                let vn = dot3(rv, n);
                let mut j_n = 0.0f32;
                if vn < 0.0 {
                    let e = if vn < -0.5 {
                        0.5 * (bodies[a].restitution + bodies[b].restitution)
                    } else {
                        0.0
                    };
                    j_n = -(1.0 + e) * vn / w_sum;
                    let imp = [n[0] * j_n, n[1] * j_n, n[2] * j_n];
                    bodies[a].apply_linear_impulse([-imp[0], -imp[1], -imp[2]]);
                    bodies[b].apply_linear_impulse(imp);

                    // Tangential friction impulse
                    let vt = [rv[0] - n[0] * vn, rv[1] - n[1] * vn, rv[2] - n[2] * vn];
                    let vt_len = len3(vt);
                    if vt_len > 1e-6 {
                        let t_dir = [vt[0] / vt_len, vt[1] / vt_len, vt[2] / vt_len];
                        let mu = 0.5 * (bodies[a].friction + bodies[b].friction);
                        let j_t = (vt_len / w_sum).min(mu * j_n);
                        let f_imp = [t_dir[0] * j_t, t_dir[1] * j_t, t_dir[2] * j_t];
                        bodies[a].apply_linear_impulse(f_imp);
                        bodies[b].apply_linear_impulse([-f_imp[0], -f_imp[1], -f_imp[2]]);
                    }
                }

                if *manifold_count < manifolds.len() {
                    let mut m = ContactManifold3D::new(ia, ib, n);
                    let la = bodies[a].pose.reverse().transform_point(contact.point_a);
                    let lb = bodies[b].pose.reverse().transform_point(contact.point_b);
                    m.push_or_warm_start(&contact, la, lb, 1);
                    m.points[0].normal_impulse = j_n;
                    manifolds[*manifold_count] = m;
                    *manifold_count += 1;
                }
            }
        }

        // 2c. Solve 6-DOF PGA Motor Joint constraints.
        let mut j_idx = 0usize;
        while j_idx < joints.len() {
            solve_motor_joint_xpbd(&joints[j_idx], bodies, sub_dt);
            j_idx += 1;
        }
    }

    // 3. Clear per-step forces and update sleep timers.
    let mut bi = 0usize;
    while bi < n_bodies {
        let b = &mut bodies[bi];
        b.external_force = [0.0; 3];
        b.external_torque = [0.0; 3];
        if b.body_type == RigidBodyType3D::Dynamic {
            let speed_sq = dot3(b.linear_velocity, b.linear_velocity)
                + dot3(b.angular_velocity, b.angular_velocity);
            if speed_sq < 0.0025 {
                b.sleep_timer += dt;
                if b.sleep_timer >= 0.5 {
                    b.is_sleeping = true;
                    b.linear_velocity = [0.0; 3];
                    b.angular_velocity = [0.0; 3];
                }
            } else {
                b.sleep_timer = 0.0;
                b.is_sleeping = false;
            }
        }
        bi += 1;
    }
}

#[inline]
fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[inline]
fn len3(v: [f32; 3]) -> f32 {
    dot3(v, v).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xpbd_dynamic_sphere_lands_on_static_floor() {
        let floor = RigidBody3D::new_static(
            motor_translate([0.0, -0.5, 0.0]),
            ColliderShape3D::Cuboid {
                half_extents: [10.0, 0.5, 10.0],
            },
        );
        let ball = RigidBody3D::new_dynamic(
            motor_translate([0.0, 1.5, 0.0]),
            2.0,
            ColliderShape3D::Sphere { radius: 0.5 },
        );

        let mut bodies = [floor, ball];
        let mut joints = [];
        let mut manifolds = [ContactManifold3D::default(); 8];
        let mut manifold_count = 0;
        let mut bvh = BroadphaseBvh::<16>::new(0.1);

        // Simulate 1 second at 64 Hz
        for _ in 0..64 {
            step_xpbd_world(
                &mut bodies,
                &mut joints,
                &mut manifolds,
                &mut manifold_count,
                &mut bvh,
                [0.0, -9.81, 0.0],
                1.0 / 64.0,
                4,
            );
        }

        // Ball of radius 0.5 rests on floor whose top surface is at y = 0.0 -> ball center ~ 0.5
        let ball_y = bodies[1].pose.translation()[1];
        assert!(
            (ball_y - 0.5).abs() < 0.08,
            "ball should rest at y ~ 0.5, got {ball_y}"
        );
    }
}
