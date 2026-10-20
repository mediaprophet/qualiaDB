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

#[inline]
fn quat_mul_xyzw(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}

#[inline]
fn quat_conjugate_xyzw(q: [f32; 4]) -> [f32; 4] {
    [-q[0], -q[1], -q[2], q[3]]
}

/// 64-bit bone mask (1 bit per bone in [`ZeroHeapSkeleton`]).
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoneMask64(pub u64);

impl Default for BoneMask64 {
    fn default() -> Self {
        Self::ALL
    }
}

impl BoneMask64 {
    pub const ALL: Self = Self(u64::MAX);
    pub const NONE: Self = Self(0);

    /// Builds a subtree mask rooted at `root_bone` from a [`ZeroHeapSkeleton`].
    pub fn from_subtree(skel: &ZeroHeapSkeleton, root_bone: usize) -> Self {
        let n = skel.bone_count.min(MAX_BONES);
        if root_bone >= n {
            return Self::NONE;
        }
        let mut mask = 1u64 << root_bone;
        for _ in 0..n {
            let mut changed = false;
            for i in 0..n {
                if (mask & (1u64 << i)) != 0 {
                    continue;
                }
                let p = skel.parent_indices[i] as usize;
                if p < n && (mask & (1u64 << p)) != 0 {
                    mask |= 1u64 << i;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        Self(mask)
    }

    #[inline]
    pub const fn contains(self, bone_idx: usize) -> bool {
        bone_idx < MAX_BONES && (self.0 & (1u64 << bone_idx)) != 0
    }
}

/// Multi-layer skeletal blend mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerBlendOp {
    Override,
    Additive,
}

/// Blends `layer_pose` into `base_pose` using `mask`, `op`, and `weight` in `[0.0, 1.0]` (`HotZeroHeap`).
pub fn blend_skeleton_layer_zero_heap(
    base_pose: &mut [Transform3D],
    rest_pose: &[Transform3D],
    layer_pose: &[Transform3D],
    mask: BoneMask64,
    op: LayerBlendOp,
    weight: f32,
) {
    let w = weight.clamp(0.0, 1.0);
    if w <= 1e-6 {
        return;
    }
    let count = base_pose
        .len()
        .min(rest_pose.len())
        .min(layer_pose.len())
        .min(MAX_BONES);
    for i in 0..count {
        if !mask.contains(i) {
            continue;
        }
        match op {
            LayerBlendOp::Override => {
                base_pose[i] = base_pose[i].lerp(&layer_pose[i], w);
            }
            LayerBlendOp::Additive => {
                let dt = [
                    (layer_pose[i].translation[0] - rest_pose[i].translation[0]) * w,
                    (layer_pose[i].translation[1] - rest_pose[i].translation[1]) * w,
                    (layer_pose[i].translation[2] - rest_pose[i].translation[2]) * w,
                ];
                let delta_q = quat_mul_xyzw(
                    quat_conjugate_xyzw(rest_pose[i].rotation),
                    layer_pose[i].rotation,
                );
                let weighted_q = quat_slerp([0.0, 0.0, 0.0, 1.0], delta_q, w);
                base_pose[i].translation = [
                    base_pose[i].translation[0] + dt[0],
                    base_pose[i].translation[1] + dt[1],
                    base_pose[i].translation[2] + dt[2],
                ];
                base_pose[i].rotation = quat_mul_xyzw(base_pose[i].rotation, weighted_q);
            }
        }
    }
}

/// 2D blend space sample point for `qualia-sim`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SimBlendPoint2D {
    pub pos: [f32; 2],
    pub clip_index: u16,
}

/// Zero-heap 2D triangle blend space for `Transform3D` skeletal poses.
#[derive(Debug, Clone, Copy)]
pub struct SimBlendSpace2D<const MAX_PTS: usize = 16, const MAX_TRIS: usize = 32> {
    pub points: [SimBlendPoint2D; MAX_PTS],
    pub point_count: usize,
    pub triangles: [[u8; 3]; MAX_TRIS],
    pub tri_count: usize,
}

impl<const MAX_PTS: usize, const MAX_TRIS: usize> Default for SimBlendSpace2D<MAX_PTS, MAX_TRIS> {
    fn default() -> Self {
        Self {
            points: [SimBlendPoint2D {
                pos: [0.0, 0.0],
                clip_index: 0,
            }; MAX_PTS],
            point_count: 0,
            triangles: [[0; 3]; MAX_TRIS],
            tri_count: 0,
        }
    }
}

impl<const MAX_PTS: usize, const MAX_TRIS: usize> SimBlendSpace2D<MAX_PTS, MAX_TRIS> {
    pub fn add_point(&mut self, x: f32, y: f32, clip_index: u16) -> bool {
        if self.point_count >= MAX_PTS {
            return false;
        }
        self.points[self.point_count] = SimBlendPoint2D {
            pos: [x, y],
            clip_index,
        };
        self.point_count += 1;
        true
    }

    pub fn add_triangle(&mut self, i0: u8, i1: u8, i2: u8) -> bool {
        if self.tri_count >= MAX_TRIS {
            return false;
        }
        self.triangles[self.tri_count] = [i0, i1, i2];
        self.tri_count += 1;
        true
    }

    /// Evaluates barycentric weights `([clip0, clip1, clip2], [w0, w1, w2])` for `query` (`HotZeroHeap`).
    pub fn sample_weights(&self, query: [f32; 2]) -> ([u16; 3], [f32; 3]) {
        if self.point_count == 0 {
            return ([0, 0, 0], [1.0, 0.0, 0.0]);
        }
        for t in 0..self.tri_count {
            let [i0, i1, i2] = self.triangles[t];
            let p0 = self.points[i0 as usize];
            let p1 = self.points[i1 as usize];
            let p2 = self.points[i2 as usize];
            let area = (p1.pos[0] - p0.pos[0]) * (p2.pos[1] - p0.pos[1])
                - (p1.pos[1] - p0.pos[1]) * (p2.pos[0] - p0.pos[0]);
            if area.abs() <= 1e-7 {
                continue;
            }
            let inv = 1.0 / area;
            let w0 = ((p1.pos[0] - query[0]) * (p2.pos[1] - query[1])
                - (p1.pos[1] - query[1]) * (p2.pos[0] - query[0]))
                * inv;
            let w1 = ((query[0] - p0.pos[0]) * (p2.pos[1] - p0.pos[1])
                - (query[1] - p0.pos[1]) * (p2.pos[0] - p0.pos[0]))
                * inv;
            let w2 = 1.0 - w0 - w1;
            if w0 >= -1e-4 && w1 >= -1e-4 && w2 >= -1e-4 {
                let cw0 = w0.max(0.0);
                let cw1 = w1.max(0.0);
                let cw2 = w2.max(0.0);
                let s = (cw0 + cw1 + cw2).max(1e-6);
                return (
                    [p0.clip_index, p1.clip_index, p2.clip_index],
                    [cw0 / s, cw1 / s, cw2 / s],
                );
            }
        }
        let c = self.points[0].clip_index;
        ([c, c, c], [1.0, 0.0, 0.0])
    }
}

/// Samples a [`SimBlendSpace2D`] at `query` and blends the 3 enclosing clip `Transform3D` poses (`HotZeroHeap`).
pub fn sample_pose_2d_zero_heap<const MAX_PTS: usize, const MAX_TRIS: usize>(
    space: &SimBlendSpace2D<MAX_PTS, MAX_TRIS>,
    query: [f32; 2],
    clip_poses: &[&[Transform3D]],
    out_pose: &mut [Transform3D],
) -> usize {
    let (clips, [w0, w1, w2]) = space.sample_weights(query);
    let p0 = clip_poses[clips[0] as usize];
    let p1 = clip_poses[clips[1] as usize];
    let p2 = clip_poses[clips[2] as usize];
    let n = p0.len().min(p1.len()).min(p2.len()).min(out_pose.len());
    let w01 = w0 + w1;
    let t01 = if w01 > 1e-6 { w1 / w01 } else { 0.0 };
    for i in 0..n {
        let a = p0[i].lerp(&p1[i], t01);
        out_pose[i] = a.lerp(&p2[i], w2);
    }
    n
}

/// Root motion displacement extracted from bone 0.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SimRootMotionDelta {
    pub delta_translation: [f32; 3],
    pub delta_rotation: [f32; 4],
}

/// Loop-safe root motion extractor for `Transform3D` skeletons.
#[derive(Debug, Clone, Copy, Default)]
pub struct SimRootMotionExtractor {
    pub prev_translation: [f32; 3],
    pub loop_end_translation: [f32; 3],
    pub initialized: bool,
}

impl SimRootMotionExtractor {
    pub fn extract_and_lock(
        &mut self,
        root_transform: &mut Transform3D,
        looped: bool,
    ) -> SimRootMotionDelta {
        let curr = root_transform.translation;
        if !self.initialized {
            self.prev_translation = curr;
            self.initialized = true;
            root_transform.translation[0] = 0.0;
            root_transform.translation[2] = 0.0;
            return SimRootMotionDelta {
                delta_translation: [0.0; 3],
                delta_rotation: [0.0, 0.0, 0.0, 1.0],
            };
        }
        let dt = if !looped {
            [
                curr[0] - self.prev_translation[0],
                curr[1] - self.prev_translation[1],
                curr[2] - self.prev_translation[2],
            ]
        } else {
            [
                (self.loop_end_translation[0] - self.prev_translation[0]) + curr[0],
                (self.loop_end_translation[1] - self.prev_translation[1]) + curr[1],
                (self.loop_end_translation[2] - self.prev_translation[2]) + curr[2],
            ]
        };
        self.prev_translation = curr;
        root_transform.translation[0] = 0.0;
        root_transform.translation[2] = 0.0;
        SimRootMotionDelta {
            delta_translation: dt,
            delta_rotation: root_transform.rotation,
        }
    }
}

/// Fixed-capacity animation signal event and ring buffer for `qualia-sim`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SimAnimationSignal {
    pub signal_id: u32,
    pub normalized_time: f32,
    pub payload: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct SimSignalQueue<const CAP: usize = 16> {
    buf: [SimAnimationSignal; CAP],
    head: usize,
    tail: usize,
    len: usize,
}

impl<const CAP: usize> Default for SimSignalQueue<CAP> {
    fn default() -> Self {
        Self {
            buf: [SimAnimationSignal::default(); CAP],
            head: 0,
            tail: 0,
            len: 0,
        }
    }
}

impl<const CAP: usize> SimSignalQueue<CAP> {
    pub fn push(&mut self, sig: SimAnimationSignal) -> bool {
        if self.len >= CAP || CAP == 0 {
            return false;
        }
        self.buf[self.tail] = sig;
        self.tail = (self.tail + 1) % CAP;
        self.len += 1;
        true
    }

    pub fn pop(&mut self) -> Option<SimAnimationSignal> {
        if self.len == 0 || CAP == 0 {
            return None;
        }
        let s = self.buf[self.head];
        self.head = (self.head + 1) % CAP;
        self.len -= 1;
        Some(s)
    }
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

    #[test]
    fn test_multi_layer_mask_blend_space_2d_root_motion_and_signals() {
        let mut skel = ZeroHeapSkeleton::default();
        skel.bone_count = 3;
        skel.parent_indices[0] = 255;
        skel.parent_indices[1] = 0;
        skel.parent_indices[2] = 1;

        let upper_mask = BoneMask64::from_subtree(&skel, 1);
        assert!(!upper_mask.contains(0));
        assert!(upper_mask.contains(1));
        assert!(upper_mask.contains(2));

        let rest = [Transform3D::default(); 3];
        let mut base = rest;
        let mut upper = rest;
        upper[2].translation = [0.0, 2.0, 0.0];
        blend_skeleton_layer_zero_heap(
            &mut base,
            &rest,
            &upper,
            upper_mask,
            LayerBlendOp::Additive,
            0.5,
        );
        assert!((base[2].translation[1] - 1.0).abs() < 1e-5);
        assert_eq!(base[0].translation[1], 0.0);

        // 2D Blend space
        let mut bs = SimBlendSpace2D::<8, 8>::default();
        bs.add_point(0.0, 0.0, 0);
        bs.add_point(1.0, 0.0, 1);
        bs.add_point(0.0, 1.0, 2);
        bs.add_triangle(0, 1, 2);
        let mut c1 = rest;
        c1[0].translation = [4.0, 0.0, 0.0];
        let mut c2 = rest;
        c2[0].translation = [0.0, 0.0, 4.0];
        let clips: [&[Transform3D]; 3] = [&rest, &c1, &c2];
        let mut sampled = rest;
        sample_pose_2d_zero_heap(&bs, [0.25, 0.25], &clips, &mut sampled);
        assert!((sampled[0].translation[0] - 1.0).abs() < 1e-4);
        assert!((sampled[0].translation[2] - 1.0).abs() < 1e-4);

        // Root motion extraction & signal queue
        let mut rm = SimRootMotionExtractor {
            loop_end_translation: [0.0, 0.0, 2.0],
            ..Default::default()
        };
        let _ = rm.extract_and_lock(&mut sampled[0], false);
        sampled[0].translation = [0.0, 0.0, 1.5];
        let delta = rm.extract_and_lock(&mut sampled[0], false);
        assert!((delta.delta_translation[2] - 0.5).abs() < 1e-5);
        assert_eq!(sampled[0].translation[2], 0.0);

        let mut q = SimSignalQueue::<4>::default();
        assert!(q.push(SimAnimationSignal {
            signal_id: 7,
            normalized_time: 0.5,
            payload: 1.0,
        }));
        assert_eq!(q.pop().unwrap().signal_id, 7);
    }
}
