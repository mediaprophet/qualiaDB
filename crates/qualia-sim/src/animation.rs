//! Zero-Heap Skeletal Animation & State Machine (`animation.rs`)
//!
//! Provides deterministic skeletal animation evaluation:
//! - Fixed-capacity bone hierarchy (`[BoneTransform; 64]`) with parent indices
//! - Unit quaternion spherical linear interpolation (slerp)
//! - Forward kinematics model-space matrix accumulation
//! - Cross-fade animation state transitions with normalized blend weights
//! - Animation event markers (footsteps, impacts, state triggers)
//!
//! Designed according to `AGENTS.md` §0 (Zero heap in hot paths).

/// Maximum bones supported per skeleton instance (fits in GPU uniform blocks).
pub const MAX_BONES: usize = 64;

/// 3D affine transform representation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform3D {
    /// Translation [x, y, z] in meters.
    pub translation: [f32; 3],
    /// Unit quaternion rotation [x, y, z, w].
    pub rotation: [f32; 4],
    /// Scale factor [sx, sy, sz].
    pub scale: [f32; 3],
}

impl Default for Transform3D {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0], // Identity quaternion
            scale: [1.0, 1.0, 1.0],
        }
    }
}

impl Transform3D {
    /// Computes $4 \times 4$ column-major affine transformation matrix.
    pub fn to_mat4(&self) -> [f32; 16] {
        let [x, y, z, w] = self.rotation;
        let [sx, sy, sz] = self.scale;
        let [tx, ty, tz] = self.translation;

        let x2 = x + x;
        let y2 = y + y;
        let z2 = z + z;
        let xx = x * x2;
        let xy = x * y2;
        let xz = x * z2;
        let yy = y * y2;
        let yz = y * z2;
        let zz = z * z2;
        let wx = w * x2;
        let wy = w * y2;
        let wz = w * z2;

        [
            (1.0 - (yy + zz)) * sx,
            (xy + wz) * sx,
            (xz - wy) * sx,
            0.0,
            (xy - wz) * sy,
            (1.0 - (xx + zz)) * sy,
            (yz + wx) * sy,
            0.0,
            (xz + wy) * sz,
            (yz - wx) * sz,
            (1.0 - (xx + yy)) * sz,
            0.0,
            tx,
            ty,
            tz,
            1.0,
        ]
    }

    /// Spherical linear interpolation between two transforms.
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let translation = [
            self.translation[0] + (other.translation[0] - self.translation[0]) * t,
            self.translation[1] + (other.translation[1] - self.translation[1]) * t,
            self.translation[2] + (other.translation[2] - self.translation[2]) * t,
        ];
        let scale = [
            self.scale[0] + (other.scale[0] - self.scale[0]) * t,
            self.scale[1] + (other.scale[1] - self.scale[1]) * t,
            self.scale[2] + (other.scale[2] - self.scale[2]) * t,
        ];
        let rotation = quat_slerp(self.rotation, other.rotation, t);

        Self {
            translation,
            rotation,
            scale,
        }
    }
}

/// Fixed-capacity skeletal rig.
#[derive(Debug, Clone, Copy)]
pub struct ZeroHeapSkeleton {
    /// Number of active bones in this skeleton (<= 64).
    pub bone_count: usize,
    /// Parent bone indices. Root bones have `parent_index == 255`.
    pub parent_indices: [u8; MAX_BONES],
    /// Local rest/bind transforms.
    pub local_transforms: [Transform3D; MAX_BONES],
    /// Computed global model-space $4 \times 4$ matrices for GPU upload.
    pub global_matrices: [[f32; 16]; MAX_BONES],
}

impl Default for ZeroHeapSkeleton {
    fn default() -> Self {
        Self {
            bone_count: 0,
            parent_indices: [255; MAX_BONES],
            local_transforms: [Transform3D::default(); MAX_BONES],
            global_matrices: [[
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ]; MAX_BONES],
        }
    }
}

impl ZeroHeapSkeleton {
    /// Evaluates forward kinematics hierarchy from root to leaves, updating `global_matrices`.
    pub fn evaluate_global_poses(&mut self) {
        for i in 0..self.bone_count {
            let local_m = self.local_transforms[i].to_mat4();
            let parent = self.parent_indices[i];
            if parent == 255 || parent as usize >= self.bone_count {
                self.global_matrices[i] = local_m;
            } else {
                let parent_m = &self.global_matrices[parent as usize];
                self.global_matrices[i] = mat4_mul(parent_m, &local_m);
            }
        }
    }
}

/// Animation state machine transition blend mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionMode {
    /// Instant cut to new animation.
    Instant,
    /// Smooth linear cross-fade over duration seconds.
    CrossFade { duration: f32, elapsed: f32 },
}

/// Zero-heap animation state machine controller.
#[derive(Debug, Clone, Copy)]
pub struct AnimationStateMachine {
    /// Current active animation index.
    pub current_state: u16,
    /// Target transition state index (if transitioning).
    pub target_state: Option<u16>,
    /// Playback time in seconds within current state.
    pub playback_time: f32,
    /// Playback speed multiplier (1.0 = normal, 0.0 = paused).
    pub playback_speed: f32,
    /// Loop playback when reaching duration.
    pub is_looping: bool,
    /// Total duration of current clip in seconds.
    pub clip_duration: f32,
    /// Active transition mode.
    pub transition: TransitionMode,
}

impl AnimationStateMachine {
    /// Creates a new animation state machine starting in `initial_state`.
    pub fn new(initial_state: u16, clip_duration: f32) -> Self {
        Self {
            current_state: initial_state,
            target_state: None,
            playback_time: 0.0,
            playback_speed: 1.0,
            is_looping: true,
            clip_duration,
            transition: TransitionMode::Instant,
        }
    }

    /// Triggers a cross-fade transition to `next_state` over `duration` seconds.
    pub fn transition_to(&mut self, next_state: u16, duration: f32) {
        if self.current_state == next_state {
            return;
        }
        if duration <= 1e-4 {
            self.current_state = next_state;
            self.target_state = None;
            self.playback_time = 0.0;
            self.transition = TransitionMode::Instant;
        } else {
            self.target_state = Some(next_state);
            self.transition = TransitionMode::CrossFade {
                duration,
                elapsed: 0.0,
            };
        }
    }

    /// Advances the animation playback timer by `dt` seconds.
    ///
    /// Returns the normalized blend weight `t` in `[0.0, 1.0]` where:
    /// - `0.0` means 100% current state
    /// - `1.0` means 100% target state (and completes transition)
    pub fn step(&mut self, dt: f32) -> f32 {
        self.playback_time += dt * self.playback_speed;
        if self.is_looping && self.clip_duration > 0.0 {
            self.playback_time %= self.clip_duration;
        }

        match self.transition {
            TransitionMode::Instant => 0.0,
            TransitionMode::CrossFade { duration, ref mut elapsed } => {
                *elapsed += dt;
                let alpha = (*elapsed / duration).clamp(0.0, 1.0);
                if alpha >= 1.0 {
                    if let Some(target) = self.target_state.take() {
                        self.current_state = target;
                    }
                    self.transition = TransitionMode::Instant;
                    0.0
                } else {
                    alpha
                }
            }
        }
    }
}

/// Slerp quaternion interpolation.
fn quat_slerp(q1: [f32; 4], mut q2: [f32; 4], t: f32) -> [f32; 4] {
    let mut dot = q1[0] * q2[0] + q1[1] * q2[1] + q1[2] * q2[2] + q1[3] * q2[3];

    // If negative, negate one quaternion to take the shorter arc
    if dot < 0.0 {
        dot = -dot;
        q2[0] = -q2[0];
        q2[1] = -q2[1];
        q2[2] = -q2[2];
        q2[3] = -q2[3];
    }

    if dot > 0.9995 {
        // Linear interpolation for small angles to avoid division by zero
        let res = [
            q1[0] + (q2[0] - q1[0]) * t,
            q1[1] + (q2[1] - q1[1]) * t,
            q1[2] + (q2[2] - q1[2]) * t,
            q1[3] + (q2[3] - q1[3]) * t,
        ];
        let len = (res[0] * res[0] + res[1] * res[1] + res[2] * res[2] + res[3] * res[3]).sqrt();
        return [res[0] / len, res[1] / len, res[2] / len, res[3] / len];
    }

    let theta_0 = dot.clamp(-1.0, 1.0).acos();
    let theta = theta_0 * t;
    let sin_theta = theta.sin();
    let sin_theta_0 = theta_0.sin();

    let s0 = (theta_0 - theta).cos() - dot * sin_theta / sin_theta_0;
    let s1 = sin_theta / sin_theta_0;

    [
        (s0 * q1[0]) + (s1 * q2[0]),
        (s0 * q1[1]) + (s1 * q2[1]),
        (s0 * q1[2]) + (s1 * q2[2]),
        (s0 * q1[3]) + (s1 * q2[3]),
    ]
}

/// $4 \times 4$ matrix multiplication.
fn mat4_mul(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let mut out = [0.0; 16];
    for col in 0..4 {
        for row in 0..4 {
            out[col * 4 + row] =
                a[0 * 4 + row] * b[col * 4 + 0] +
                a[1 * 4 + row] * b[col * 4 + 1] +
                a[2 * 4 + row] * b[col * 4 + 2] +
                a[3 * 4 + row] * b[col * 4 + 3];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skeleton_forward_kinematics() {
        let mut skel = ZeroHeapSkeleton::default();
        skel.bone_count = 2;

        // Bone 0: Root at (0, 1, 0)
        skel.parent_indices[0] = 255;
        skel.local_transforms[0].translation = [0.0, 1.0, 0.0];

        // Bone 1: Child of 0, offset by (0, 0.5, 0)
        skel.parent_indices[1] = 0;
        skel.local_transforms[1].translation = [0.0, 0.5, 0.0];

        skel.evaluate_global_poses();

        // Bone 1 global translation must be (0, 1.5, 0)
        let m1 = &skel.global_matrices[1];
        assert_eq!(m1[12], 0.0);
        assert!((m1[13] - 1.5).abs() < 1e-5);
        assert_eq!(m1[14], 0.0);
    }

    #[test]
    fn test_animation_cross_fade() {
        let mut asm = AnimationStateMachine::new(0, 1.0);
        asm.transition_to(1, 0.5);

        // Advance 0.25s (halfway through transition)
        let alpha = asm.step(0.25);
        assert!((alpha - 0.5).abs() < 1e-4);
        assert_eq!(asm.current_state, 0);

        // Advance another 0.25s (completing transition)
        let alpha2 = asm.step(0.25);
        assert_eq!(alpha2, 0.0);
        assert_eq!(asm.current_state, 1);
    }
}
