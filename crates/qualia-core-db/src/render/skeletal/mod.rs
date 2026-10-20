//! Caller-buffered skeletal animation using the VibeScript rigid motor algebra.
//! Joint transforms are rigid; authoring scale is baked into mesh/bind positions.
//! Sampling, hierarchy composition and skinning allocate no heap memory.

pub mod absm;
pub mod activity;
pub mod authoring;
pub mod blend_space;
pub mod gpu_morph;
pub mod gpu_skin;
pub mod ik_motor;
mod player;
mod pose;
mod skin;
pub use absm::{
    AbsmLayer, AbsmMachine, AbsmParameter, AbsmStateNode, AbsmStateSource, AbsmTransition,
    AnimationSignal, BoneLayerMask, EontPostureConfig, LayerBlendMode, LeafCondition,
    RootMotionDelta, RootMotionExtractor, SignalQueue, SignalTrigger, TransitionCondition,
};
pub use blend_space::{
    sample_blend_space_1d_zero_heap, sample_blend_space_2d_zero_heap, BlendPoint1D, BlendPoint2D,
    BlendSample2D, BlendSpace1D, BlendSpace2D, BlendTriangle,
};
pub use gpu_morph::{
    compute_ontogenetic_morph_weights_from_header_zero_heap,
    compute_ontogenetic_morph_weights_zero_heap, evaluate_morph_deltas_zero_heap,
    GpuInstanceMorphWeights, GpuMorphDelta, MAX_GPU_MORPH_TARGETS,
};
pub use gpu_skin::{
    evaluate_skinning_motor_palette_zero_heap, gpu_shader_skin_vertex_reference,
    hemisphere_align_motor, skin_vertices_gpu_reference_into, GpuJointMotorPair, GpuSkinnedVertex,
    MAX_GPU_RIG_JOINTS, MAX_GPU_SKELETON_INSTANCES,
};
pub use ik_motor::{
    solve_constrained_fabrik_motor_ik_zero_heap, solve_two_bone_motor_ik_zero_heap,
    JointConstraint, TwoBoneIkResult, MAX_IK_CHAIN,
};
pub use player::{PlaybackReceipt, PlayerError, SkeletalPlayer};
pub use pose::{compose_palette_into, sample_pose_into};
pub use skin::{skin_vertices_into, SkinWeight};
pub use vibe::animation::Motor;

pub const MAX_JOINTS: usize = 128;
pub const MAX_KEYS: usize = 4096;
pub const MAX_VERTICES: usize = 262_144;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkeletalError {
    InvalidInput,
    OutputTooSmall,
    BudgetExceeded,
}

#[derive(Debug, Clone, Copy)]
pub struct Joint {
    /// None for a root; otherwise strictly less than this joint's index.
    pub parent: Option<u16>,
    pub rest_local: Motor,
    pub inverse_bind: Motor,
}

#[derive(Debug, Clone, Copy)]
pub struct Key {
    pub time: f64,
    pub local: Motor,
}

#[derive(Debug, Clone, Copy)]
pub struct Track {
    pub joint: u16,
    pub first_key: u32,
    pub key_count: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct Clip<'a> {
    pub duration: f64,
    pub looping: bool,
    pub tracks: &'a [Track],
    pub keys: &'a [Key],
}

/// `true` when the motor is valid.
pub(crate) fn valid_motor(m: &Motor) -> bool {
    let real = [m.r_w, m.r_x, m.r_y, m.r_z];
    let dual = [m.d_w, m.d_x, m.d_y, m.d_z];
    real.iter()
        .chain(dual.iter())
        .all(|v| v.is_finite() && v.abs() < 1e8)
        && (real.iter().map(|v| v * v).sum::<f64>() - 1.0).abs() < 1e-5
        && real.iter().zip(dual).map(|(r, d)| r * d).sum::<f64>().abs() < 1e-5
}

/// Validates `joints`.
pub(crate) fn validate_joints(joints: &[Joint]) -> Result<(), SkeletalError> {
    if joints.len() > MAX_JOINTS {
        return Err(SkeletalError::BudgetExceeded);
    }
    if joints.is_empty()
        || joints.iter().enumerate().any(|(i, j)| {
            j.parent.is_some_and(|p| p as usize >= i)
                || !valid_motor(&j.rest_local)
                || !valid_motor(&j.inverse_bind)
        })
    {
        return Err(SkeletalError::InvalidInput);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
