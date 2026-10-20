//! Articulated Ragdoll Controller (`ragdoll.rs`).
//!
//! Bridges skeletal animation poses (PGA [`Motor`]s) and dynamic XPBD [`RigidBody3D`] bodies,
//! allowing seamless blending or instant switching between:
//! - [`RagdollMode::AnimatedKinematic`]: bones strictly follow animated target [`Motor`]s,
//! - [`RagdollMode::BlendedPowered`]: dynamic bodies are driven toward animated target [`Motor`]s
//!   via PGA `sclerp` proportional-derivative compliance (`physics_blend_weight`),
//! - [`RagdollMode::DynamicRagdoll`]: full passive XPBD ragdoll under gravity, contacts, and joints.

use qualia_core_db::render::pga::Motor;

use crate::physics3d::xpbd_solver::{RigidBody3D, RigidBodyType3D};

/// Operational mode of an [`ArticulatedRagdoll`].
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RagdollMode {
    /// Purely kinematic animation playback (`RigidBodyType3D::KinematicPosition`).
    #[default]
    AnimatedKinematic = 0,
    /// Powered active ragdoll blending animated [`Motor`]s with XPBD physics.
    BlendedPowered = 1,
    /// Unpowered passive physics ragdoll (`RigidBodyType3D::Dynamic`).
    DynamicRagdoll = 2,
}

/// Bone-to-rigid-body mapping entry inside an [`ArticulatedRagdoll`].
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RagdollLimbBinding {
    /// Index of the skeletal bone in the character rig.
    pub bone_index: u16,
    /// Index of the corresponding [`RigidBody3D`] in the physics world.
    pub body_index: u32,
}

impl Default for RagdollLimbBinding {
    #[inline]
    fn default() -> Self {
        Self {
            bone_index: 0,
            body_index: u32::MAX,
        }
    }
}

/// Zero-heap articulated ragdoll controller for up to `MAX_LIMBS` bones.
#[derive(Clone, Copy, Debug)]
pub struct ArticulatedRagdoll<const MAX_LIMBS: usize = 16> {
    pub limbs: [RagdollLimbBinding; MAX_LIMBS],
    pub limb_count: usize,
    pub mode: RagdollMode,
    /// Blend weight in `[0.0, 1.0]` (`0.0 = 100% animation`, `1.0 = 100% dynamic physics`).
    pub physics_blend_weight: f32,
}

impl<const MAX_LIMBS: usize> Default for ArticulatedRagdoll<MAX_LIMBS> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const MAX_LIMBS: usize> ArticulatedRagdoll<MAX_LIMBS> {
    /// Creates an empty ragdoll controller in `AnimatedKinematic` mode.
    pub const fn new() -> Self {
        Self {
            limbs: [RagdollLimbBinding {
                bone_index: 0,
                body_index: u32::MAX,
            }; MAX_LIMBS],
            limb_count: 0,
            mode: RagdollMode::AnimatedKinematic,
            physics_blend_weight: 0.0,
        }
    }

    /// Registers a limb binding between `bone_index` and `body_index`.
    pub fn add_limb(&mut self, bone_index: u16, body_index: u32) -> bool {
        if self.limb_count >= MAX_LIMBS {
            return false;
        }
        self.limbs[self.limb_count] = RagdollLimbBinding {
            bone_index,
            body_index,
        };
        self.limb_count += 1;
        true
    }

    /// Transitions the ragdoll into `new_mode` and updates bound `RigidBody3D::body_type` flags.
    pub fn set_mode(&mut self, new_mode: RagdollMode, blend_weight: f32, bodies: &mut [RigidBody3D]) {
        self.mode = new_mode;
        self.physics_blend_weight = match new_mode {
            RagdollMode::AnimatedKinematic => 0.0,
            RagdollMode::BlendedPowered => blend_weight.clamp(0.0, 1.0),
            RagdollMode::DynamicRagdoll => 1.0,
        };

        let target_body_type = match new_mode {
            RagdollMode::AnimatedKinematic => RigidBodyType3D::KinematicPosition,
            RagdollMode::BlendedPowered | RagdollMode::DynamicRagdoll => RigidBodyType3D::Dynamic,
        };

        let mut i = 0usize;
        while i < self.limb_count {
            let b_idx = self.limbs[i].body_index as usize;
            if b_idx < bodies.len() {
                bodies[b_idx].body_type = target_body_type;
                bodies[b_idx].is_sleeping = false;
                bodies[b_idx].sleep_timer = 0.0;
            }
            i += 1;
        }
    }

    /// Synchronizes `animated_motors` and `bodies` after an XPBD physics step, writing the final
    /// blended world-space bone [`Motor`]s into `out_bone_motors`.
    pub fn sync_poses(
        &self,
        animated_motors: &[Motor],
        bodies: &mut [RigidBody3D],
        out_bone_motors: &mut [Motor],
        dt: f32,
    ) {
        let inv_dt = if dt > 1e-6 { 1.0 / dt } else { 0.0 };
        let mut i = 0usize;
        while i < self.limb_count {
            let bone_idx = self.limbs[i].bone_index as usize;
            let body_idx = self.limbs[i].body_index as usize;
            i += 1;
            if bone_idx >= animated_motors.len()
                || bone_idx >= out_bone_motors.len()
                || body_idx >= bodies.len()
            {
                continue;
            }

            let anim_motor = animated_motors[bone_idx];
            match self.mode {
                RagdollMode::AnimatedKinematic => {
                    let prev_pos = bodies[body_idx].pose.translation();
                    let next_pos = anim_motor.translation();
                    bodies[body_idx].linear_velocity = [
                        (next_pos[0] - prev_pos[0]) * inv_dt,
                        (next_pos[1] - prev_pos[1]) * inv_dt,
                        (next_pos[2] - prev_pos[2]) * inv_dt,
                    ];
                    bodies[body_idx].pose = anim_motor;
                    out_bone_motors[bone_idx] = anim_motor;
                }
                RagdollMode::BlendedPowered => {
                    let phys_motor = bodies[body_idx].pose;
                    let blended = anim_motor.sclerp(&phys_motor, self.physics_blend_weight);
                    bodies[body_idx].pose = blended;
                    out_bone_motors[bone_idx] = blended;
                }
                RagdollMode::DynamicRagdoll => {
                    out_bone_motors[bone_idx] = bodies[body_idx].pose;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics3d::gjk_epa::ColliderShape3D;
    use qualia_core_db::render::pga::motor_translate;

    #[test]
    fn articulated_ragdoll_blends_between_animation_and_xpbd_physics() {
        let mut bodies = [RigidBody3D::new_dynamic(
            motor_translate([0.0, 2.0, 0.0]),
            5.0,
            ColliderShape3D::Capsule {
                radius: 0.1,
                half_height: 0.3,
            },
        )];
        let anim_motors = [motor_translate([0.0, 4.0, 0.0])];
        let mut out_motors = [Motor::identity(); 1];

        let mut ragdoll = ArticulatedRagdoll::<4>::new();
        assert!(ragdoll.add_limb(0, 0));

        // 50% powered blend between anim (y=4.0) and physics (y=2.0) -> y = 3.0
        ragdoll.set_mode(RagdollMode::BlendedPowered, 0.5, &mut bodies);
        ragdoll.sync_poses(&anim_motors, &mut bodies, &mut out_motors, 1.0 / 64.0);
        let blended_y = out_motors[0].translation()[1];
        assert!((blended_y - 3.0).abs() < 1e-4, "expected y=3.0, got {blended_y}");
    }
}
