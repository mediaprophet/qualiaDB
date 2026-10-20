//! Zero-Heap Multi-Layer Animation Blending State Machine (`absm.rs`).
//!
//! Combines multi-layer bone-masked animation graphs (`Override` and `Additive` layers),
//! boolean/threshold parameter transition trees, 1D/2D Delaunay blend spaces,
//! timeline `AnimationSignal` ring buffers (for `HolosonicWavefield` `SonicToken`s),
//! loop-safe `RootMotionExtractor`, and `.10d` `MAGIC_EONT` (`EONT1`) ontogenetic/metabolic
//! posture modulation.
//!
//! Honors `AGENTS.md` §0 & §0-A: all hot-path state machine stepping and pose blending
//! execute with zero heap allocations (`HotZeroHeap`).

use super::blend_space::{
    sample_blend_space_1d_zero_heap, sample_blend_space_2d_zero_heap, BlendSpace1D, BlendSpace2D,
};
use super::{Joint, Motor, SkeletalError, MAX_JOINTS};
use crate::container_10d::entity_ontology_section::EntityOntologyMiniHeader;

/// Maximum parameter slots in an [`AbsmMachine`].
pub const MAX_ABSM_PARAMS: usize = 16;

/// Maximum signal triggers per animation state.
pub const MAX_SIGNALS_PER_STATE: usize = 4;

/// 128-bit bone mask (1 bit per joint up to `MAX_JOINTS = 128`).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BoneLayerMask(pub u128);

impl Default for BoneLayerMask {
    fn default() -> Self {
        Self::ALL
    }
}

impl BoneLayerMask {
    /// All 128 joints enabled.
    pub const ALL: Self = Self(u128::MAX);
    /// No joints enabled.
    pub const NONE: Self = Self(0);

    /// Builds a `BoneLayerMask` covering `root_joint` and all of its descendants
    /// from a parent index slice (`parent < 0` indicates a root bone).
    pub fn from_joint_subtree(parent_indices: &[i32], root_joint: usize) -> Self {
        let count = parent_indices.len().min(MAX_JOINTS);
        if root_joint >= count {
            return Self::NONE;
        }
        let mut mask = 1u128 << root_joint;
        // Fixed bounded convergence loop handles both topological and arbitrary bone orders
        for _ in 0..count {
            let mut changed = false;
            for j in 0..count {
                if (mask & (1u128 << j)) != 0 {
                    continue;
                }
                let p = parent_indices[j];
                if p >= 0 && (p as usize) < count && (mask & (1u128 << (p as usize))) != 0 {
                    mask |= 1u128 << j;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        Self(mask)
    }

    /// Builds a `BoneLayerMask` covering `root_joint` and all of its descendants from `&[Joint]`.
    pub fn from_joints(joints: &[Joint], root_joint: usize) -> Self {
        let count = joints.len().min(MAX_JOINTS);
        if root_joint >= count {
            return Self::NONE;
        }
        let mut mask = 1u128 << root_joint;
        for j in (root_joint + 1)..count {
            if let Some(p) = joints[j].parent {
                if (p as usize) < count && (mask & (1u128 << (p as usize))) != 0 {
                    mask |= 1u128 << j;
                }
            }
        }
        Self(mask)
    }

    /// Returns `true` if `joint_idx` (`< 128`) is enabled in this mask.
    #[inline]
    pub const fn contains(self, joint_idx: usize) -> bool {
        if joint_idx >= MAX_JOINTS {
            return false;
        }
        (self.0 & (1u128 << joint_idx)) != 0
    }

    /// Enables `joint_idx` in this mask.
    #[inline]
    pub fn insert(&mut self, joint_idx: usize) {
        if joint_idx < MAX_JOINTS {
            self.0 |= 1u128 << joint_idx;
        }
    }

    /// Disables `joint_idx` in this mask.
    #[inline]
    pub fn remove(&mut self, joint_idx: usize) {
        if joint_idx < MAX_JOINTS {
            self.0 &= !(1u128 << joint_idx);
        }
    }
}

/// How an ABSM layer combines its output pose with lower layers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayerBlendMode {
    /// Geodesic `Motor::sclerp` override of masked joints by `layer.weight`.
    #[default]
    Override,
    /// Multiplies relative delta motor $M_{\text{base}} \circ (M_{\text{rest}}^{-1} \circ M_{\text{layer}})^w$
    /// onto masked joints.
    Additive,
}

/// Typed parameter value driving ABSM transitions and blend spaces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AbsmParameter {
    Float(f32),
    Bool(bool),
    Int(i32),
    Vec2([f32; 2]),
}

impl Default for AbsmParameter {
    fn default() -> Self {
        Self::Float(0.0)
    }
}

/// Atomic predicate for transition evaluation (stack-resident, `Copy`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LeafCondition {
    Always,
    ParamGt { param_idx: u8, threshold: f32 },
    ParamLt { param_idx: u8, threshold: f32 },
    ParamEqBool { param_idx: u8, expected: bool },
    ParamEqInt { param_idx: u8, expected: i32 },
    StateTimeFinished,
}

impl LeafCondition {
    #[inline]
    pub fn evaluate(self, params: &[AbsmParameter], normalized_state_time: f32) -> bool {
        match self {
            Self::Always => true,
            Self::ParamGt {
                param_idx,
                threshold,
            } => matches!(
                params.get(param_idx as usize),
                Some(AbsmParameter::Float(v)) if *v > threshold
            ),
            Self::ParamLt {
                param_idx,
                threshold,
            } => matches!(
                params.get(param_idx as usize),
                Some(AbsmParameter::Float(v)) if *v < threshold
            ),
            Self::ParamEqBool {
                param_idx,
                expected,
            } => matches!(
                params.get(param_idx as usize),
                Some(AbsmParameter::Bool(v)) if *v == expected
            ),
            Self::ParamEqInt {
                param_idx,
                expected,
            } => matches!(
                params.get(param_idx as usize),
                Some(AbsmParameter::Int(v)) if *v == expected
            ),
            Self::StateTimeFinished => normalized_state_time >= 1.0 - 1e-5,
        }
    }
}

/// Zero-heap transition firing condition supporting single predicates and `And`/`Or` combinators.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransitionCondition {
    ParamGt { param_idx: u8, threshold: f32 },
    ParamLt { param_idx: u8, threshold: f32 },
    ParamEqBool { param_idx: u8, expected: bool },
    StateTimeFinished,
    And(LeafCondition, LeafCondition),
    Or(LeafCondition, LeafCondition),
}

impl TransitionCondition {
    /// Evaluates this transition condition against `params` and `normalized_state_time`.
    #[inline]
    pub fn evaluate(self, params: &[AbsmParameter], normalized_state_time: f32) -> bool {
        match self {
            Self::ParamGt {
                param_idx,
                threshold,
            } => LeafCondition::ParamGt {
                param_idx,
                threshold,
            }
            .evaluate(params, normalized_state_time),
            Self::ParamLt {
                param_idx,
                threshold,
            } => LeafCondition::ParamLt {
                param_idx,
                threshold,
            }
            .evaluate(params, normalized_state_time),
            Self::ParamEqBool {
                param_idx,
                expected,
            } => LeafCondition::ParamEqBool {
                param_idx,
                expected,
            }
            .evaluate(params, normalized_state_time),
            Self::StateTimeFinished => {
                LeafCondition::StateTimeFinished.evaluate(params, normalized_state_time)
            }
            Self::And(a, b) => {
                a.evaluate(params, normalized_state_time)
                    && b.evaluate(params, normalized_state_time)
            }
            Self::Or(a, b) => {
                a.evaluate(params, normalized_state_time)
                    || b.evaluate(params, normalized_state_time)
            }
        }
    }
}

/// Timeline event emitted when an animation playhead crosses a normalized trigger timestamp.
/// Coupled to `HolosonicWavefield` via `sonic_archetype` and `payload_f32`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AnimationSignal {
    pub signal_id: u32,
    pub layer_index: u8,
    pub state_index: u8,
    pub normalized_time: f32,
    pub sonic_archetype: u8,
    pub payload_f32: f32,
}

/// Authored signal trigger attached to an animation state.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SignalTrigger {
    pub active: bool,
    pub signal_id: u32,
    pub normalized_time: f32,
    pub sonic_archetype: u8,
    pub payload_f32: f32,
}

/// Fixed-capacity FIFO ring buffer for [`AnimationSignal`] events (`HotZeroHeap`).
#[derive(Clone, Copy, Debug)]
pub struct SignalQueue<const CAP: usize = 16> {
    buffer: [AnimationSignal; CAP],
    head: usize,
    tail: usize,
    len: usize,
}

impl<const CAP: usize> Default for SignalQueue<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> SignalQueue<CAP> {
    /// Creates an empty signal queue.
    pub const fn new() -> Self {
        Self {
            buffer: [AnimationSignal {
                signal_id: 0,
                layer_index: 0,
                state_index: 0,
                normalized_time: 0.0,
                sonic_archetype: 0,
                payload_f32: 0.0,
            }; CAP],
            head: 0,
            tail: 0,
            len: 0,
        }
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.len = 0;
    }

    /// Pushes a signal into the ring buffer. Returns `false` if full.
    pub fn push(&mut self, sig: AnimationSignal) -> bool {
        if self.len >= CAP || CAP == 0 {
            return false;
        }
        self.buffer[self.tail] = sig;
        self.tail = (self.tail + 1) % CAP;
        self.len += 1;
        true
    }

    /// Pops the oldest signal from the ring buffer.
    pub fn pop(&mut self) -> Option<AnimationSignal> {
        if self.len == 0 || CAP == 0 {
            return None;
        }
        let item = self.buffer[self.head];
        self.head = (self.head + 1) % CAP;
        self.len -= 1;
        Some(item)
    }

    /// Checks `triggers` against the playhead advancement `(prev_norm_t, curr_norm_t]`
    /// (handling loop wrap-around cleanly) and pushes crossed events.
    pub fn emit_crossed_signals(
        &mut self,
        triggers: &[SignalTrigger],
        layer_index: u8,
        state_index: u8,
        prev_norm_t: f32,
        curr_norm_t: f32,
        looped: bool,
    ) -> usize {
        let mut fired = 0usize;
        for trig in triggers {
            if !trig.active {
                continue;
            }
            let t = trig.normalized_time;
            let crossed = if !looped {
                t > prev_norm_t && t <= curr_norm_t
            } else {
                t > prev_norm_t || t <= curr_norm_t
            };
            if crossed {
                if self.push(AnimationSignal {
                    signal_id: trig.signal_id,
                    layer_index,
                    state_index,
                    normalized_time: t,
                    sonic_archetype: trig.sonic_archetype,
                    payload_f32: trig.payload_f32,
                }) {
                    fired += 1;
                }
            }
        }
        fired
    }
}

/// Frame-to-frame root motion displacement extracted from the skeleton's root joint.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RootMotionDelta {
    /// World/character-local horizontal and vertical translation delta `[dx, dy, dz]`.
    pub delta_translation: [f32; 3],
    /// Relative root rotation quaternion `[r_w, r_x, r_y, r_z]`.
    pub delta_rotor: [f32; 4],
}

impl Default for RootMotionDelta {
    fn default() -> Self {
        Self {
            delta_translation: [0.0, 0.0, 0.0],
            delta_rotor: [1.0, 0.0, 0.0, 0.0],
        }
    }
}

/// Extracts continuous root motion from the root bone (eliminating loop-wrap teleport spikes)
/// and locks horizontal `(x, z)` drift on the visual skeleton.
#[derive(Clone, Copy, Debug)]
pub struct RootMotionExtractor {
    pub root_joint_index: usize,
    pub lock_horizontal: bool,
    pub lock_vertical: bool,
    pub prev_root_motor: Motor,
    pub loop_start_motor: Motor,
    pub loop_end_motor: Motor,
    pub initialized: bool,
}

impl Default for RootMotionExtractor {
    fn default() -> Self {
        Self::new(0, Motor::identity(), Motor::identity())
    }
}

impl RootMotionExtractor {
    /// Creates a root motion extractor for `root_joint_index` with authored clip seam poses.
    pub const fn new(
        root_joint_index: usize,
        loop_start_motor: Motor,
        loop_end_motor: Motor,
    ) -> Self {
        Self {
            root_joint_index,
            lock_horizontal: true,
            lock_vertical: false,
            prev_root_motor: loop_start_motor,
            loop_start_motor,
            loop_end_motor,
            initialized: false,
        }
    }

    /// Resets tracking to `current_root`.
    pub fn reset(&mut self, current_root: Motor) {
        self.prev_root_motor = current_root;
        self.initialized = true;
    }

    /// Extracts the frame's `RootMotionDelta` from `local_pose[self.root_joint_index]`
    /// and zeroes out horizontal `(x, z)` root bone drift in-place (`HotZeroHeap`).
    pub fn extract_and_lock(
        &mut self,
        local_pose: &mut [Motor],
        looped_this_frame: bool,
    ) -> RootMotionDelta {
        if self.root_joint_index >= local_pose.len() {
            return RootMotionDelta::default();
        }
        let curr = local_pose[self.root_joint_index];
        if !self.initialized {
            self.prev_root_motor = curr;
            self.initialized = true;
            if self.lock_horizontal {
                let (rot, trans) = curr.to_rotation_translation();
                let (_, start_t) = self.loop_start_motor.to_rotation_translation();
                let locked_y = if self.lock_vertical {
                    start_t[1]
                } else {
                    trans[1]
                };
                local_pose[self.root_joint_index] =
                    Motor::from_rotation_translation(rot, [start_t[0], locked_y, start_t[2]]);
            }
            return RootMotionDelta::default();
        }

        let (_, prev_t) = self.prev_root_motor.to_rotation_translation();
        let (curr_r, curr_t) = curr.to_rotation_translation();

        let delta_t = if !looped_this_frame {
            [
                (curr_t[0] - prev_t[0]) as f32,
                (curr_t[1] - prev_t[1]) as f32,
                (curr_t[2] - prev_t[2]) as f32,
            ]
        } else {
            let (_, start_t) = self.loop_start_motor.to_rotation_translation();
            let (_, end_t) = self.loop_end_motor.to_rotation_translation();
            [
                ((end_t[0] - prev_t[0]) + (curr_t[0] - start_t[0])) as f32,
                ((end_t[1] - prev_t[1]) + (curr_t[1] - start_t[1])) as f32,
                ((end_t[2] - prev_t[2]) + (curr_t[2] - start_t[2])) as f32,
            ]
        };

        let delta_motor = self.prev_root_motor.conjugate().mul(&curr).normalize();
        self.prev_root_motor = curr;

        if self.lock_horizontal {
            let (_, start_t) = self.loop_start_motor.to_rotation_translation();
            let locked_y = if self.lock_vertical {
                start_t[1]
            } else {
                curr_t[1]
            };
            local_pose[self.root_joint_index] =
                Motor::from_rotation_translation(curr_r, [start_t[0], locked_y, start_t[2]]);
        }

        RootMotionDelta {
            delta_translation: delta_t,
            delta_rotor: [
                delta_motor.r_w as f32,
                delta_motor.r_x as f32,
                delta_motor.r_y as f32,
                delta_motor.r_z as f32,
            ],
        }
    }
}

/// Motion generator source for a state inside an [`AbsmLayer`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AbsmStateSource {
    /// Plays a single clip from the clip pose table.
    SingleClip { clip_index: u16 },
    /// Samples a [`BlendSpace1D`] driven by `Float` parameter `param_idx`.
    Blend1D { space_index: u8, param_idx: u8 },
    /// Samples a [`BlendSpace2D`] driven by `Vec2` parameter `param_idx`.
    Blend2D { space_index: u8, param_idx: u8 },
}

impl Default for AbsmStateSource {
    fn default() -> Self {
        Self::SingleClip { clip_index: 0 }
    }
}

/// A state node inside an [`AbsmLayer`].
#[derive(Clone, Copy, Debug)]
pub struct AbsmStateNode {
    pub active: bool,
    pub source: AbsmStateSource,
    pub duration: f32,
    pub speed: f32,
    pub looping: bool,
    pub triggers: [SignalTrigger; MAX_SIGNALS_PER_STATE],
}

impl Default for AbsmStateNode {
    fn default() -> Self {
        Self {
            active: false,
            source: AbsmStateSource::default(),
            duration: 1.0,
            speed: 1.0,
            looping: true,
            triggers: [SignalTrigger::default(); MAX_SIGNALS_PER_STATE],
        }
    }
}

/// Directed transition edge between two states in an [`AbsmLayer`].
#[derive(Clone, Copy, Debug)]
pub struct AbsmTransition {
    pub active: bool,
    pub from_state: u8,
    pub to_state: u8,
    pub duration: f32,
    pub condition: TransitionCondition,
}

impl Default for AbsmTransition {
    fn default() -> Self {
        Self {
            active: false,
            from_state: 0,
            to_state: 0,
            duration: 0.2,
            condition: TransitionCondition::StateTimeFinished,
        }
    }
}

/// Single animation layer within an [`AbsmMachine`].
#[derive(Clone, Copy, Debug)]
pub struct AbsmLayer<const MAX_STATES: usize = 16, const MAX_TRANSITIONS: usize = 32> {
    pub enabled: bool,
    pub mask: BoneLayerMask,
    pub blend_mode: LayerBlendMode,
    pub weight: f32,
    pub states: [AbsmStateNode; MAX_STATES],
    pub state_count: usize,
    pub transitions: [AbsmTransition; MAX_TRANSITIONS],
    pub transition_count: usize,
    pub current_state: u8,
    pub target_state: Option<u8>,
    pub state_time: f32,
    pub target_state_time: f32,
    pub transition_elapsed: f32,
    pub transition_duration: f32,
    pub looped_last_step: bool,
}

impl<const MAX_STATES: usize, const MAX_TRANSITIONS: usize> Default
    for AbsmLayer<MAX_STATES, MAX_TRANSITIONS>
{
    fn default() -> Self {
        Self::new(BoneLayerMask::ALL, LayerBlendMode::Override, 1.0)
    }
}

impl<const MAX_STATES: usize, const MAX_TRANSITIONS: usize> AbsmLayer<MAX_STATES, MAX_TRANSITIONS> {
    pub const fn new(mask: BoneLayerMask, blend_mode: LayerBlendMode, weight: f32) -> Self {
        Self {
            enabled: false,
            mask,
            blend_mode,
            weight,
            states: [AbsmStateNode {
                active: false,
                source: AbsmStateSource::SingleClip { clip_index: 0 },
                duration: 1.0,
                speed: 1.0,
                looping: true,
                triggers: [SignalTrigger {
                    active: false,
                    signal_id: 0,
                    normalized_time: 0.0,
                    sonic_archetype: 0,
                    payload_f32: 0.0,
                }; MAX_SIGNALS_PER_STATE],
            }; MAX_STATES],
            state_count: 0,
            transitions: [AbsmTransition {
                active: false,
                from_state: 0,
                to_state: 0,
                duration: 0.2,
                condition: TransitionCondition::StateTimeFinished,
            }; MAX_TRANSITIONS],
            transition_count: 0,
            current_state: 0,
            target_state: None,
            state_time: 0.0,
            target_state_time: 0.0,
            transition_elapsed: 0.0,
            transition_duration: 0.0,
            looped_last_step: false,
        }
    }

    pub fn add_state(&mut self, mut node: AbsmStateNode) -> Option<u8> {
        if self.state_count >= MAX_STATES {
            return None;
        }
        node.active = true;
        let idx = self.state_count;
        self.states[idx] = node;
        self.state_count += 1;
        self.enabled = true;
        Some(idx as u8)
    }

    pub fn add_transition(
        &mut self,
        from_state: u8,
        to_state: u8,
        duration: f32,
        condition: TransitionCondition,
    ) -> bool {
        if self.transition_count >= MAX_TRANSITIONS
            || (from_state as usize) >= self.state_count
            || (to_state as usize) >= self.state_count
        {
            return false;
        }
        self.transitions[self.transition_count] = AbsmTransition {
            active: true,
            from_state,
            to_state,
            duration: duration.max(0.0),
            condition,
        };
        self.transition_count += 1;
        true
    }
}

/// `.10d` `MAGIC_EONT` (`EONT1`) ontogenetic and metabolic posture/cadence modulator.
#[derive(Clone, Copy, Debug)]
pub struct EontPostureConfig {
    pub enabled: bool,
    /// Mask of spine/neck/posture joints affected by fatigue/age pitch droop.
    pub spine_mask: BoneLayerMask,
    /// Lifecycle stage (`0=SeedOrSpore, 1=Juvenile, 2=Adult, 3=Senescent, 4=Deceased`).
    pub stage: u8,
    pub energy_permill: u16,
    pub damage_permill: u16,
    pub hydration_permill: u16,
    /// Maximum forward pitch droop angle in radians at full exhaustion/senescence.
    pub max_droop_rad: f32,
}

impl Default for EontPostureConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            spine_mask: BoneLayerMask::NONE,
            stage: 2,
            energy_permill: 1000,
            damage_permill: 0,
            hydration_permill: 1000,
            max_droop_rad: 0.35,
        }
    }
}

impl EontPostureConfig {
    /// Updates metabolic/ontogenetic fields directly from an `.10d` `EntityOntologyMiniHeader`.
    #[inline]
    pub fn update_from_header(&mut self, header: &EntityOntologyMiniHeader) {
        self.stage = header.stage;
        self.energy_permill = header.energy_permill;
        self.damage_permill = header.damage_permill;
        self.hydration_permill = header.hydration_permill;
    }

    /// Computes a cadence speed multiplier in `[0.25, 1.15]` driven by age and metabolic energy.
    #[inline]
    pub fn cadence_multiplier(&self) -> f32 {
        if !self.enabled {
            return 1.0;
        }
        let stage_factor = match self.stage {
            0 => 0.50,
            1 => 1.10, // Juvenile brisk cadence
            2 => 1.00, // Adult baseline
            3 => 0.75, // Senescent slower gait
            _ => 0.25,
        };
        let energy_norm = (self.energy_permill.min(1000) as f32) * 0.001;
        let damage_norm = (self.damage_permill.min(1000) as f32) * 0.001;
        let vitality = (0.5 + 0.5 * energy_norm) * (1.0 - 0.4 * damage_norm);
        (stage_factor * vitality).clamp(0.25, 1.15)
    }

    /// Applies ontogenetic/metabolic posture pitch droop onto joints in `self.spine_mask`.
    pub fn apply_posture_droop(&self, pose: &mut [Motor]) {
        if !self.enabled || self.spine_mask == BoneLayerMask::NONE {
            return;
        }
        let fatigue = 1.0 - (self.energy_permill.min(1000) as f32) * 0.001;
        let damage = (self.damage_permill.min(1000) as f32) * 0.001;
        let dehydration = 1.0 - (self.hydration_permill.min(1000) as f32) * 0.001;
        let age_stoop = if self.stage >= 3 { 0.35 } else { 0.0 };

        let droop_intensity =
            (0.45 * fatigue + 0.35 * damage + 0.20 * dehydration + age_stoop).clamp(0.0, 1.0);
        if droop_intensity <= 1e-4 {
            return;
        }
        let half_angle = (0.5 * droop_intensity * self.max_droop_rad) as f64;
        let droop_rotor = Motor::from_rotation_translation(
            [half_angle.cos(), half_angle.sin(), 0.0, 0.0],
            [0.0, 0.0, 0.0],
        );

        let limit = pose.len().min(MAX_JOINTS);
        for j in 0..limit {
            if self.spine_mask.contains(j) {
                pose[j] = pose[j].mul(&droop_rotor).normalize();
            }
        }
    }
}

/// Multi-layer zero-heap Animation Blending State Machine (`AbsmMachine`).
#[derive(Clone, Copy, Debug)]
pub struct AbsmMachine<
    const MAX_LAYERS: usize = 4,
    const MAX_STATES: usize = 16,
    const MAX_TRANSITIONS: usize = 32,
> {
    pub layers: [AbsmLayer<MAX_STATES, MAX_TRANSITIONS>; MAX_LAYERS],
    pub layer_count: usize,
    pub params: [AbsmParameter; MAX_ABSM_PARAMS],
    pub blend_spaces_1d: [BlendSpace1D<8>; 4],
    pub blend_spaces_2d: [BlendSpace2D<16, 32>; 4],
    pub eont_posture: EontPostureConfig,
    pub root_motion: Option<RootMotionExtractor>,
}

impl<const MAX_LAYERS: usize, const MAX_STATES: usize, const MAX_TRANSITIONS: usize> Default
    for AbsmMachine<MAX_LAYERS, MAX_STATES, MAX_TRANSITIONS>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const MAX_LAYERS: usize, const MAX_STATES: usize, const MAX_TRANSITIONS: usize>
    AbsmMachine<MAX_LAYERS, MAX_STATES, MAX_TRANSITIONS>
{
    /// Creates an empty multi-layer ABSM.
    pub fn new() -> Self {
        Self {
            layers: [AbsmLayer::default(); MAX_LAYERS],
            layer_count: 0,
            params: [AbsmParameter::Float(0.0); MAX_ABSM_PARAMS],
            blend_spaces_1d: [BlendSpace1D::new(); 4],
            blend_spaces_2d: [BlendSpace2D::new(); 4],
            eont_posture: EontPostureConfig::default(),
            root_motion: None,
        }
    }

    #[inline]
    pub fn set_parameter(&mut self, idx: usize, value: AbsmParameter) {
        if idx < MAX_ABSM_PARAMS {
            self.params[idx] = value;
        }
    }

    #[inline]
    pub fn add_layer(&mut self, layer: AbsmLayer<MAX_STATES, MAX_TRANSITIONS>) -> Option<usize> {
        if self.layer_count >= MAX_LAYERS {
            return None;
        }
        let idx = self.layer_count;
        self.layers[idx] = layer;
        self.layers[idx].enabled = true;
        self.layer_count += 1;
        Some(idx)
    }

    fn evaluate_state_source(
        &self,
        source: AbsmStateSource,
        clip_poses: &[&[Motor]],
        rest_pose: &[Motor],
        out_pose: &mut [Motor],
    ) {
        let joint_count = rest_pose.len().min(out_pose.len()).min(MAX_JOINTS);
        match source {
            AbsmStateSource::SingleClip { clip_index } => {
                if let Some(clip) = clip_poses.get(clip_index as usize) {
                    let n = joint_count.min(clip.len());
                    out_pose[..n].copy_from_slice(&clip[..n]);
                } else {
                    out_pose[..joint_count].copy_from_slice(&rest_pose[..joint_count]);
                }
            }
            AbsmStateSource::Blend1D {
                space_index,
                param_idx,
            } => {
                let p = match self.params.get(param_idx as usize) {
                    Some(AbsmParameter::Float(v)) => *v,
                    _ => 0.0,
                };
                if let Some(space) = self.blend_spaces_1d.get(space_index as usize) {
                    if sample_blend_space_1d_zero_heap(
                        space,
                        p,
                        clip_poses,
                        &mut out_pose[..joint_count],
                    )
                    .is_err()
                    {
                        out_pose[..joint_count].copy_from_slice(&rest_pose[..joint_count]);
                    }
                }
            }
            AbsmStateSource::Blend2D {
                space_index,
                param_idx,
            } => {
                let q = match self.params.get(param_idx as usize) {
                    Some(AbsmParameter::Vec2(v)) => *v,
                    _ => [0.0, 0.0],
                };
                if let Some(space) = self.blend_spaces_2d.get(space_index as usize) {
                    if sample_blend_space_2d_zero_heap(
                        space,
                        q,
                        clip_poses,
                        &mut out_pose[..joint_count],
                    )
                    .is_err()
                    {
                        out_pose[..joint_count].copy_from_slice(&rest_pose[..joint_count]);
                    }
                }
            }
        }
    }

    /// Advances all active layers by `dt` seconds, evaluates state transitions, fires timeline
    /// [`AnimationSignal`]s into `signals`, blends `Override` and `Additive` bone-masked layers,
    /// applies `.10d` `EONT1` ontogenetic/metabolic posture modulation, and extracts
    /// [`RootMotionDelta`] — all with zero heap allocations (`HotZeroHeap`).
    pub fn step_and_evaluate_zero_heap<const SIG_CAP: usize>(
        &mut self,
        dt: f32,
        rest_pose: &[Motor],
        clip_poses: &[&[Motor]],
        out_pose: &mut [Motor],
        signals: &mut SignalQueue<SIG_CAP>,
    ) -> Result<RootMotionDelta, SkeletalError> {
        let joint_count = rest_pose.len().min(MAX_JOINTS);
        if joint_count == 0 || !dt.is_finite() || dt < 0.0 {
            return Err(SkeletalError::InvalidInput);
        }
        if out_pose.len() < joint_count {
            return Err(SkeletalError::OutputTooSmall);
        }

        out_pose[..joint_count].copy_from_slice(&rest_pose[..joint_count]);
        let cadence = self.eont_posture.cadence_multiplier();
        let eff_dt = dt * cadence;

        let mut layer_pose = [Motor::identity(); MAX_JOINTS];
        let mut target_pose = [Motor::identity(); MAX_JOINTS];

        for l_idx in 0..self.layer_count.min(MAX_LAYERS) {
            if !self.layers[l_idx].enabled
                || self.layers[l_idx].state_count == 0
                || self.layers[l_idx].weight <= 1e-6
            {
                continue;
            }

            let mut curr_idx = self.layers[l_idx].current_state as usize;
            if curr_idx >= self.layers[l_idx].state_count {
                continue;
            }

            // Evaluate parameter-driven transitions before advancing state_time
            if self.layers[l_idx].target_state.is_none() {
                let init_dur = self.layers[l_idx].states[curr_idx].duration.max(1e-4);
                let init_norm = (self.layers[l_idx].state_time / init_dur).clamp(0.0, 1.0);
                let t_count = self.layers[l_idx].transition_count.min(MAX_TRANSITIONS);
                for tr_idx in 0..t_count {
                    let tr = self.layers[l_idx].transitions[tr_idx];
                    if tr.active
                        && tr.from_state == self.layers[l_idx].current_state
                        && tr.condition.evaluate(&self.params, init_norm)
                    {
                        if tr.duration <= 1e-4 {
                            self.layers[l_idx].current_state = tr.to_state;
                            self.layers[l_idx].state_time = 0.0;
                            curr_idx = tr.to_state as usize;
                        } else {
                            self.layers[l_idx].target_state = Some(tr.to_state);
                            self.layers[l_idx].target_state_time = 0.0;
                            self.layers[l_idx].transition_elapsed = 0.0;
                            self.layers[l_idx].transition_duration = tr.duration;
                        }
                        break;
                    }
                }
            }

            let state_node = self.layers[l_idx].states[curr_idx];
            let dur = state_node.duration.max(1e-4);
            let prev_norm = (self.layers[l_idx].state_time / dur).clamp(0.0, 1.0);

            self.layers[l_idx].state_time += eff_dt * state_node.speed;
            let mut looped = false;
            if state_node.looping {
                if self.layers[l_idx].state_time >= dur {
                    self.layers[l_idx].state_time = self.layers[l_idx].state_time.rem_euclid(dur);
                    looped = true;
                }
            } else if self.layers[l_idx].state_time > dur {
                self.layers[l_idx].state_time = dur;
            }
            self.layers[l_idx].looped_last_step = looped;
            let curr_norm = (self.layers[l_idx].state_time / dur).clamp(0.0, 1.0);

            signals.emit_crossed_signals(
                &state_node.triggers,
                l_idx as u8,
                curr_idx as u8,
                prev_norm,
                curr_norm,
                looped,
            );

            // Evaluate end-of-clip transitions (e.g. StateTimeFinished) after advancing time
            if self.layers[l_idx].target_state.is_none() {
                let t_count = self.layers[l_idx].transition_count.min(MAX_TRANSITIONS);
                for tr_idx in 0..t_count {
                    let tr = self.layers[l_idx].transitions[tr_idx];
                    if tr.active
                        && tr.from_state == self.layers[l_idx].current_state
                        && tr.condition.evaluate(&self.params, curr_norm)
                    {
                        if tr.duration <= 1e-4 {
                            self.layers[l_idx].current_state = tr.to_state;
                            self.layers[l_idx].state_time = 0.0;
                        } else {
                            self.layers[l_idx].target_state = Some(tr.to_state);
                            self.layers[l_idx].target_state_time = 0.0;
                            self.layers[l_idx].transition_elapsed = 0.0;
                            self.layers[l_idx].transition_duration = tr.duration;
                        }
                        break;
                    }
                }
            }

            // Sample active state pose
            let active_source =
                self.layers[l_idx].states[self.layers[l_idx].current_state as usize].source;
            self.evaluate_state_source(active_source, clip_poses, rest_pose, &mut layer_pose);

            // Handle active cross-fade transition within this layer
            if let Some(target_idx) = self.layers[l_idx].target_state {
                self.layers[l_idx].transition_elapsed += eff_dt;
                let fade_dur = self.layers[l_idx].transition_duration.max(1e-4);
                let alpha = (self.layers[l_idx].transition_elapsed / fade_dur).clamp(0.0, 1.0);
                let t_source = self.layers[l_idx].states[target_idx as usize].source;
                self.evaluate_state_source(t_source, clip_poses, rest_pose, &mut target_pose);

                if alpha >= 1.0 {
                    self.layers[l_idx].current_state = target_idx;
                    self.layers[l_idx].state_time = self.layers[l_idx].target_state_time;
                    self.layers[l_idx].target_state = None;
                    layer_pose[..joint_count].copy_from_slice(&target_pose[..joint_count]);
                } else {
                    let alpha_f64 = alpha as f64;
                    for j in 0..joint_count {
                        layer_pose[j] = Motor::sclerp(&layer_pose[j], &target_pose[j], alpha_f64);
                    }
                }
            }

            // Blend layer_pose into out_pose according to mask, blend_mode, and layer weight
            let layer_w = self.layers[l_idx].weight.clamp(0.0, 1.0) as f64;
            let mask = self.layers[l_idx].mask;
            let mode = self.layers[l_idx].blend_mode;

            for j in 0..joint_count {
                if !mask.contains(j) {
                    continue;
                }
                match mode {
                    LayerBlendMode::Override => {
                        out_pose[j] = if layer_w >= 1.0 - 1e-6 {
                            layer_pose[j]
                        } else {
                            Motor::sclerp(&out_pose[j], &layer_pose[j], layer_w)
                        };
                    }
                    LayerBlendMode::Additive => {
                        // Delta from rest pose: D = rest^(-1) * layer_pose
                        let delta = rest_pose[j].conjugate().mul(&layer_pose[j]).normalize();
                        let weighted_delta = if layer_w >= 1.0 - 1e-6 {
                            delta
                        } else {
                            Motor::sclerp(&Motor::identity(), &delta, layer_w)
                        };
                        out_pose[j] = out_pose[j].mul(&weighted_delta).normalize();
                    }
                }
            }
        }

        // Apply EONT1 ontogenetic & metabolic posture modulation
        self.eont_posture
            .apply_posture_droop(&mut out_pose[..joint_count]);

        // Extract root motion and lock horizontal drift if configured
        let base_looped = self.layer_count > 0 && self.layers[0].looped_last_step;
        let root_delta = if let Some(ref mut rm) = self.root_motion {
            rm.extract_and_lock(&mut out_pose[..joint_count], base_looped)
        } else {
            RootMotionDelta::default()
        };

        Ok(root_delta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

    #[test]
    fn bone_layer_mask_from_subtree_and_multi_layer_absm_zero_heap() {
        // 3 joints: 0 = root (pelvis), 1 = spine (child of 0), 2 = right_arm (child of 1)
        let parents = [-1, 0, 1];
        let upper_mask = BoneLayerMask::from_joint_subtree(&parents, 1);
        assert!(!upper_mask.contains(0));
        assert!(upper_mask.contains(1));
        assert!(upper_mask.contains(2));

        let rest = [Motor::identity(); 3];
        // Clip 0: Idle
        let clip_idle = [
            Motor::from_translation(0.0, 0.0, 0.0),
            Motor::identity(),
            Motor::identity(),
        ];
        // Clip 1: Walk (moves root Z to 2.0)
        let clip_walk = [
            Motor::from_translation(0.0, 0.0, 2.0),
            Motor::identity(),
            Motor::identity(),
        ];
        // Clip 2: Wave upper body (translates arm Y by 1.0)
        let clip_wave = [
            Motor::identity(),
            Motor::identity(),
            Motor::from_translation(0.0, 1.0, 0.0),
        ];
        let clips: [&[Motor]; 3] = [&clip_idle, &clip_walk, &clip_wave];

        let mut machine = AbsmMachine::<4, 8, 8>::new();
        machine.root_motion = Some(RootMotionExtractor::new(
            0,
            Motor::identity(),
            Motor::from_translation(0.0, 0.0, 2.0),
        ));
        machine.eont_posture = EontPostureConfig {
            enabled: true,
            spine_mask: BoneLayerMask(1u128 << 1),
            stage: 3, // Senescent
            energy_permill: 400,
            damage_permill: 300,
            hydration_permill: 700,
            max_droop_rad: 0.3,
        };

        // Layer 0: Base locomotion (Full body)
        let mut base_layer = AbsmLayer::new(BoneLayerMask::ALL, LayerBlendMode::Override, 1.0);
        let s_idle = base_layer
            .add_state(AbsmStateNode {
                source: AbsmStateSource::SingleClip { clip_index: 0 },
                duration: 1.0,
                speed: 1.0,
                looping: true,
                ..Default::default()
            })
            .unwrap();
        let mut walk_node = AbsmStateNode {
            source: AbsmStateSource::SingleClip { clip_index: 1 },
            duration: 1.0,
            speed: 1.0,
            looping: true,
            ..Default::default()
        };
        walk_node.triggers[0] = SignalTrigger {
            active: true,
            signal_id: 42,
            normalized_time: 0.5,
            sonic_archetype: 2,
            payload_f32: 0.8,
        };
        let s_walk = base_layer.add_state(walk_node).unwrap();
        base_layer.add_transition(
            s_idle,
            s_walk,
            0.0, // Instant switch when speed > 0.5
            TransitionCondition::ParamGt {
                param_idx: 0,
                threshold: 0.5,
            },
        );
        machine.add_layer(base_layer).unwrap();

        // Layer 1: Upper-body masked additive action
        let mut upper_layer = AbsmLayer::new(upper_mask, LayerBlendMode::Additive, 1.0);
        upper_layer
            .add_state(AbsmStateNode {
                source: AbsmStateSource::SingleClip { clip_index: 2 },
                duration: 1.0,
                speed: 1.0,
                looping: true,
                ..Default::default()
            })
            .unwrap();
        machine.add_layer(upper_layer).unwrap();

        machine.set_parameter(0, AbsmParameter::Float(1.0));

        let mut out_pose = [Motor::identity(); 3];
        let mut signals = SignalQueue::<16>::new();

        assert_zero_alloc("absm_step_and_evaluate", || {
            let rm1 = machine
                .step_and_evaluate_zero_heap(0.1, &rest, &clips, &mut out_pose, &mut signals)
                .unwrap();
            // Root motion extracted and visual root Z locked to 0
            assert!(rm1.delta_translation[2] >= 0.0);
            let (_, root_t) = out_pose[0].to_rotation_translation();
            assert!(root_t[2].abs() < 1e-5);

            // Step across 0.5 normalized time (accounting for senescent EONT cadence multiplier) to trigger footstep signal 42
            let _ = machine
                .step_and_evaluate_zero_heap(1.1, &rest, &clips, &mut out_pose, &mut signals)
                .unwrap();
        });

        let sig = signals.pop().expect("footstep signal must fire");
        assert_eq!(sig.signal_id, 42);
        assert_eq!(sig.sonic_archetype, 2);

        // Upper-body additive layer translated joint 2 along Y by 1.0
        let (_, arm_t) = out_pose[2].to_rotation_translation();
        assert!((arm_t[1] - 1.0).abs() < 1e-5);
        // Spine joint 1 received EONT1 posture droop rotation
        let (spine_r, _) = out_pose[1].to_rotation_translation();
        assert!(spine_r[1].abs() > 0.01);
    }
}
