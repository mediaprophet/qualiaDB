//! Caller-buffered skeletal animation using the rigid motor algebra (`etalon_render::skeletal`).
//! Joint transforms are rigid; authoring scale is baked into mesh/bind positions.
//! Sampling, hierarchy composition and skinning allocate no heap memory.

pub mod absm;
pub mod activity;
pub mod authoring;
pub mod blend_space;
pub mod fauna_rig;
pub mod gpu_morph;
pub mod gpu_skin;
pub mod ik_motor;
pub mod player;

pub use fauna_rig::{
    FaunaBiomechanicalArchetype, FaunaSkeletalRig10d, FAUNA_MAX_JOINTS, FAUNA_PARENT_INDICES,
};

pub use etalon_render::skeletal::{
    compose_palette_into, compute_ontogenetic_morph_weights_from_header_zero_heap,
    compute_ontogenetic_morph_weights_zero_heap, evaluate_morph_deltas_zero_heap,
    evaluate_skinning_motor_palette_zero_heap, gpu_shader_skin_vertex_reference,
    hemisphere_align_motor, ontogenetic_stage_growth_factor, sample_blend_space_1d_zero_heap,
    sample_blend_space_2d_zero_heap, sample_pose_into, skin_vertices_gpu_reference_into,
    skin_vertices_into, solve_constrained_fabrik_motor_ik_zero_heap,
    solve_two_bone_motor_ik_zero_heap, valid_motor, validate_joints, AbsmLayer, AbsmMachine,
    AbsmParameter, AbsmStateNode, AbsmStateSource, AbsmTransition, AnimationClip, AnimationEvent,
    AnimationSignal, AnimationStateMachine, BlendPoint1D, BlendPoint2D, BlendSample2D,
    BlendSpace1D, BlendSpace2D, BlendTriangle, BoneLayerMask, BoneTrack, Clip, EontPostureConfig,
    GpuInstanceMorphWeights, GpuJointMotorPair, GpuMorphDelta, GpuSkinnedVertex, Joint,
    JointConstraint, Key, Keyframe3D, LayerBlendMode, LeafCondition, Motor, MotorBivector,
    OntogeneticStateHeader, PoseTimePolicy, RootMotionDelta, RootMotionExtractor, SignalQueue,
    SignalTrigger, SkeletalError, SkinWeight, Track, Transform3D, TransitionCondition,
    TransitionMode, TwoBoneIkResult, ZeroHeapSkeleton, MAX_BONES, MAX_GPU_MORPH_TARGETS,
    MAX_GPU_RIG_JOINTS, MAX_GPU_SKELETON_INSTANCES, MAX_IK_CHAIN, MAX_JOINTS, MAX_KEYS,
    MAX_VERTICES,
};
pub use player::{
    BodyFitSource, BodyFitSpec, PlaybackReceipt, PlayerError, SkeletalCounts, SkeletalPlayer,
    SkeletalRigDescriptor, SkeletalSectionSource,
};

#[cfg(test)]
mod tests;
