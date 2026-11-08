//! Hardware GPU PGA Motor Dual-Quaternion Skinning, Multi-Layer ABSM,
//! 2D Delaunay Blend Space & Constrained Motor IK for Australian Fauna.
//!
//! Harnesses:
//! - **Epic 3**: Hardware GPU PGA `Motor` Dual-Quaternion Skinning, `PROC1` Motor Oscillation
//!   (harmonic respiration pitch & caudal undulation) & 8-Channel Pre-Skinning GPU Morph Targets.
//! - **Epic 4**: Zero-Heap Multi-Layer ABSM (`BaseLocomotion`, `UpperBodyVocalizeForage`,
//!   `HeadLookAtPosture`), `EONT1` Ontogenetic/Metabolic Posture Modulation, 2D Bowyer-Watson
//!   Delaunay Blend Spaces, Seam-Preserving Root Motion & Constrained Motor IK (Analytical
//!   Two-Bone + Constrained FABRIK with `ConeLimit` and `HingeLimit`).
//!
//! Covers 164 East Coast Australian fauna species organized into 13 biomechanical archetypes.

use crate::render::skeletal::{
    compute_ontogenetic_morph_weights_zero_heap, evaluate_morph_deltas_zero_heap,
    evaluate_skinning_motor_palette_zero_heap, gpu_shader_skin_vertex_reference,
    sample_blend_space_2d_zero_heap, solve_constrained_fabrik_motor_ik_zero_heap,
    solve_two_bone_motor_ik_zero_heap, AbsmLayer, AbsmMachine, AbsmParameter, AbsmStateNode,
    AbsmStateSource, BlendSample2D, BlendSpace2D, BoneLayerMask, EontPostureConfig,
    GpuInstanceMorphWeights, GpuJointMotorPair, GpuMorphDelta, GpuSkinnedVertex, Joint,
    JointConstraint, LayerBlendMode, Motor, RootMotionDelta, RootMotionExtractor, SignalQueue,
    SignalTrigger, SkeletalError, TransitionCondition, TwoBoneIkResult,
};
use serde::{Deserialize, Serialize};

/// Maximum joints in the Australian fauna skeletal rig (fits in fixed stack buffer).
pub const FAUNA_MAX_JOINTS: usize = 18;

// ── Cardinal Joint Indices (18 Joints) ───────────────────────────────────────
pub const JOINT_PELVIS_ROOT: usize = 0;
pub const JOINT_SPINE: usize = 1;
pub const JOINT_CHEST: usize = 2;
pub const JOINT_NECK: usize = 3;
pub const JOINT_HEAD: usize = 4;
pub const JOINT_JAW: usize = 5;

// Hindlimbs (saltatorial propulsion / bipedal cursorial / quadrupedal hind):
pub const JOINT_LEFT_HIP: usize = 6;
pub const JOINT_LEFT_KNEE: usize = 7;
pub const JOINT_LEFT_ANKLE: usize = 8;
pub const JOINT_RIGHT_HIP: usize = 9;
pub const JOINT_RIGHT_KNEE: usize = 10;
pub const JOINT_RIGHT_ANKLE: usize = 11;

// Forelimbs / Wings / Flippers / Claws:
pub const JOINT_LEFT_SHOULDER: usize = 12;
pub const JOINT_LEFT_ELBOW: usize = 13;
pub const JOINT_LEFT_WRIST_WING: usize = 14;

// Articulated Tail (macropod balance, reptile undulation, avian steering):
pub const JOINT_TAIL_BASE: usize = 15;
pub const JOINT_TAIL_MID: usize = 16;
pub const JOINT_TAIL_TIP: usize = 17;

/// Canonical topological parent array for the 18-joint fauna rig (`parent < 0` for root).
pub const FAUNA_PARENT_INDICES: [i32; FAUNA_MAX_JOINTS] = [
    -1, // 0: PelvisRoot
     0, // 1: Spine
     1, // 2: Chest
     2, // 3: Neck
     3, // 4: Head
     4, // 5: Jaw
     0, // 6: LeftHip
     6, // 7: LeftKnee
     7, // 8: LeftAnkle
     0, // 9: RightHip
     9, // 10: RightKnee
    10, // 11: RightAnkle
     2, // 12: LeftShoulder (Chest)
    12, // 13: LeftElbow
    13, // 14: LeftWristWing
     0, // 15: TailBase (Pelvis)
    15, // 16: TailMid
    16, // 17: TailTip
];

// ── Cardinal Clip Indices (5 Cardinal Clips) ─────────────────────────────────
pub const CLIP_IDLE: usize = 0;
pub const CLIP_WALK_STALK: usize = 1;
pub const CLIP_CRUISE_TROT_GLIDE: usize = 2;
pub const CLIP_SPRINT_HOP_DIVE: usize = 3;
pub const CLIP_TURN: usize = 4;
pub const FAUNA_CLIP_COUNT: usize = 5;

// ── Animation Signal IDs ─────────────────────────────────────────────────────
pub const SIG_FOOTSTEP_LEFT: u32 = 1001;
pub const SIG_FOOTSTEP_RIGHT: u32 = 1002;
pub const SIG_WINGBEAT: u32 = 1003;
pub const SIG_VOCAL_CALL: u32 = 2001;

// ── Biomechanical Archetypes ─────────────────────────────────────────────────

/// 13 Primary Australian Biomechanical Archetypes covering all 164 catalogued species.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FaunaBiomechanicalArchetype {
    /// Eastern Grey Kangaroo, Red Kangaroo, Wallabies, Pademelons, Potoroos, Bettongs.
    MacropodSaltatorial,
    /// Common Wombat, Northern Hairy-Nosed Wombat.
    VombatidFossorial,
    /// Koala, Lumholtz's Tree-Kangaroo.
    PhascolarctidArboreal,
    /// Greater Glider, Sugar Glider, Squirrel Glider, Feathertail Glider, Brushtail Possum.
    PetauridGlider,
    /// Spotted-Tail Quoll, Eastern Quoll, Tasmanian Devil, Antechinus, Bandicoots, Dunnarts.
    DasyuridPredator,
    /// Grey-Headed Flying-Fox, Black Flying-Fox, Little Red Flying-Fox, Gould's Wattled Bat.
    PteropodidChiroptera,
    /// Australian Fur Seal, Indo-Pacific Bottlenose Dolphin.
    MarinePinnipedCetacean,
    /// Emu, Southern Cassowary.
    RatiteBipedal,
    /// Wedge-Tailed Eagle, White-Bellied Sea-Eagle, Eastern Osprey, Powerful Owl, Sooty Owl.
    RaptorSoaring,
    /// Superb Lyrebird, Sulphur-Crested Cockatoo, Gang-Gang, Kookaburra, Fairywren, Magpie.
    PasserinePsittacine,
    /// Lace Monitor, Gould's Goanna, Frill-Necked Lizard, Bearded Dragon, Shingleback, Estuarine Crocodile.
    SquamateSprawling,
    /// Red-Bellied Black Snake, Tiger Snake, Coastal Carpet Python, Green Tree Python.
    SerpentineUndulatory,
    /// Platypus, Short-Beaked Echidna, Murray Cod, Southern Corroboree Frog, Green Tree Frog.
    MonotremeTeleostAmphibian,
}

impl FaunaBiomechanicalArchetype {
    /// String identifier matching the fauna catalog representation.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::MacropodSaltatorial => "macropod_saltatorial",
            Self::VombatidFossorial => "vombatid_fossorial",
            Self::PhascolarctidArboreal => "phascolarctid_arboreal",
            Self::PetauridGlider => "petaurid_glider",
            Self::DasyuridPredator => "dasyurid_predator",
            Self::PteropodidChiroptera => "pteropodid_chiroptera",
            Self::MarinePinnipedCetacean => "marine_pinniped_cetacean",
            Self::RatiteBipedal => "ratite_bipedal",
            Self::RaptorSoaring => "raptor_soaring",
            Self::PasserinePsittacine => "passerine_psittacine",
            Self::SquamateSprawling => "squamate_sprawling",
            Self::SerpentineUndulatory => "serpentine_undulatory",
            Self::MonotremeTeleostAmphibian => "monotreme_teleost_amphibian",
        }
    }

    /// Resolves an archetype from its canonical string name.
    pub fn from_archetype_str(s: &str) -> Option<Self> {
        match s {
            "macropod_saltatorial" | "macropod" => Some(Self::MacropodSaltatorial),
            "vombatid_fossorial" | "wombat" => Some(Self::VombatidFossorial),
            "phascolarctid_arboreal" | "koala" => Some(Self::PhascolarctidArboreal),
            "petaurid_glider" | "glider_possum" => Some(Self::PetauridGlider),
            "dasyurid_predator" | "quoll_dasyurid" => Some(Self::DasyuridPredator),
            "pteropodid_chiroptera" | "flying_fox" | "chiroptera" => Some(Self::PteropodidChiroptera),
            "marine_pinniped_cetacean" | "pinniped_cetacean" | "cetacean" | "pinniped" => {
                Some(Self::MarinePinnipedCetacean)
            }
            "ratite_bipedal" | "ratite" => Some(Self::RatiteBipedal),
            "raptor_soaring" | "raptor" => Some(Self::RaptorSoaring),
            "passerine_psittacine" | "songbird" | "lyrebird" | "cockatoo_parrot"
            | "kookaburra_kingfisher" => Some(Self::PasserinePsittacine),
            "squamate_sprawling" | "goanna_skink" | "crocodilian" => Some(Self::SquamateSprawling),
            "serpentine_undulatory" | "snake" => Some(Self::SerpentineUndulatory),
            "monotreme_teleost_amphibian" | "frog" | "fish" | "turtle" | "crayfish"
            | "moth_insect" | "platypus" | "echidna" => Some(Self::MonotremeTeleostAmphibian),
            _ => None,
        }
    }

    /// Resolves species metadata from `(archetype, gait, class_name)` tuple records.
    pub fn from_species_taxonomy(archetype: &str, gait: &str, class_name: &str) -> Self {
        if let Some(arch) = Self::from_archetype_str(archetype) {
            return arch;
        }
        match class_name {
            "Aves" => {
                if gait == "soaring_flight" {
                    Self::RaptorSoaring
                } else if archetype == "ratite" || gait == "bipedal_walk" && archetype.contains("casuarius") {
                    Self::RatiteBipedal
                } else {
                    Self::PasserinePsittacine
                }
            }
            "Reptilia" => {
                if archetype == "snake" || gait == "lateral_undulation" {
                    Self::SerpentineUndulatory
                } else {
                    Self::SquamateSprawling
                }
            }
            "Amphibia" | "Actinopterygii" | "Sarcopterygii" | "Malacostraca" | "Insecta"
            | "Arachnida" => Self::MonotremeTeleostAmphibian,
            _ => {
                if gait == "bipedal_saltatorial" {
                    Self::MacropodSaltatorial
                } else if gait == "quadrupedal_waddle" {
                    Self::VombatidFossorial
                } else if gait == "arboreal_climb" {
                    Self::PhascolarctidArboreal
                } else if gait == "patagial_glide" {
                    Self::PetauridGlider
                } else if gait == "flapping_flight" {
                    Self::PteropodidChiroptera
                } else if gait == "aquatic_swim" {
                    Self::MarinePinnipedCetacean
                } else {
                    Self::DasyuridPredator
                }
            }
        }
    }

    /// Returns the active joint count for this archetype (`<= FAUNA_MAX_JOINTS`).
    #[inline]
    pub const fn joint_count(&self) -> usize {
        match self {
            Self::SerpentineUndulatory => 18,
            Self::SquamateSprawling => 18,
            Self::MacropodSaltatorial => 18,
            Self::DasyuridPredator => 18,
            Self::VombatidFossorial => 18,
            Self::PhascolarctidArboreal => 18,
            Self::PetauridGlider => 18,
            Self::MarinePinnipedCetacean => 16,
            Self::MonotremeTeleostAmphibian => 16,
            Self::RatiteBipedal => 16,
            Self::RaptorSoaring => 16,
            Self::PasserinePsittacine => 16,
            Self::PteropodidChiroptera => 16,
        }
    }

    /// Whether this archetype possesses active wings / aerial patagium.
    #[inline]
    pub const fn has_wings(&self) -> bool {
        matches!(
            self,
            Self::RaptorSoaring
                | Self::PasserinePsittacine
                | Self::PteropodidChiroptera
                | Self::PetauridGlider
        )
    }

    /// Returns authored dimensional scales `(body_length_m, hip_height_m)`.
    pub const fn default_dimensions(&self) -> (f32, f32) {
        match self {
            Self::MacropodSaltatorial => (1.20, 0.85),
            Self::VombatidFossorial => (0.95, 0.35),
            Self::PhascolarctidArboreal => (0.75, 0.40),
            Self::PetauridGlider => (0.45, 0.15),
            Self::DasyuridPredator => (0.65, 0.25),
            Self::PteropodidChiroptera => (0.35, 0.20),
            Self::MarinePinnipedCetacean => (1.80, 0.50),
            Self::RatiteBipedal => (1.60, 1.30),
            Self::RaptorSoaring => (0.80, 0.50),
            Self::PasserinePsittacine => (0.35, 0.20),
            Self::SquamateSprawling => (1.10, 0.20),
            Self::SerpentineUndulatory => (1.60, 0.08),
            Self::MonotremeTeleostAmphibian => (0.45, 0.18),
        }
    }
}

// ── Kinematics Helpers (Zero Heap) ───────────────────────────────────────────

/// Zero-heap forward kinematics computing world-space `Motor`s from local transforms.
#[inline]
pub fn forward_kinematics_zero_heap(
    parent_indices: &[i32],
    local_pose: &[Motor],
    out_world: &mut [Motor],
) {
    let count = parent_indices
        .len()
        .min(local_pose.len())
        .min(out_world.len());
    for j in 0..count {
        let p = parent_indices[j];
        out_world[j] = if p >= 0 && (p as usize) < count {
            out_world[p as usize].mul(&local_pose[j]).normalize()
        } else {
            local_pose[j].normalize()
        };
    }
}

/// Computes the relative local `Motor` between a parent and child world transform.
#[inline]
pub fn world_to_local_motor(parent_world: &Motor, child_world: &Motor) -> Motor {
    parent_world.conjugate().mul(child_world).normalize()
}

// ── 10D Fauna Skeletal Rig ───────────────────────────────────────────────────

/// Complete 10D Articulated Fauna Skeletal Rig.
///
/// Encapsulates the PGA Motor joint hierarchy, 2D Bowyer–Watson Delaunay blend space,
/// multi-layer ABSM, procedural motor oscillators, two-bone + FABRIK IK solvers,
/// and GPU skinning palette / morph target evaluation buffers.
#[derive(Clone, Copy, Debug)]
pub struct FaunaSkeletalRig10d {
    /// Biomechanical archetype.
    pub archetype: FaunaBiomechanicalArchetype,
    /// Active joint count (`<= FAUNA_MAX_JOINTS`).
    pub joint_count: usize,
    /// Up to 18 skeletal joints with authored rest transforms and inverse-bind motors.
    pub joints: [Joint; FAUNA_MAX_JOINTS],
    /// Parent index lookup table (`parent < 0` for root).
    pub parent_indices: [i32; FAUNA_MAX_JOINTS],
    /// Authored rest pose in local space.
    pub rest_pose: [Motor; FAUNA_MAX_JOINTS],
    /// 5 cardinal clips for 2D blend space locomotion and state machine generation.
    pub clip_poses: [[Motor; FAUNA_MAX_JOINTS]; FAUNA_CLIP_COUNT],
    /// 2D Bowyer-Watson Delaunay blend space parameterized by `(forward_speed, turn_rate)`.
    pub blend_space_2d: BlendSpace2D<16, 32>,
    /// Multi-layer zero-heap Animation Blending State Machine.
    pub absm: AbsmMachine<4, 16, 32>,
    /// Previous-frame skinning motors for exact TAA velocity vector evaluation.
    pub prev_skin_motors: [Motor; FAUNA_MAX_JOINTS],
    /// Current-frame skinning motors.
    pub current_skin_motors: [Motor; FAUNA_MAX_JOINTS],
    /// 64-byte GPU storage records (`GpuJointMotorPair`) ready for direct SSBO upload.
    pub gpu_palette: [GpuJointMotorPair; FAUNA_MAX_JOINTS],
    /// Event ring buffer for timeline animation signals.
    pub signal_queue: SignalQueue<16>,
}

impl FaunaSkeletalRig10d {
    /// Constructs a fully authored 10D fauna rig for the specified biomechanical archetype.
    pub fn new(archetype: FaunaBiomechanicalArchetype) -> Self {
        let joint_count = archetype.joint_count();
        let parent_indices = FAUNA_PARENT_INDICES;
        let (body_len, hip_height) = archetype.default_dimensions();

        // 1. Author archetype rest local motors based on physical proportions
        let mut rest_pose = [Motor::identity(); FAUNA_MAX_JOINTS];
        let half_width = (body_len * 0.16).max(0.04) as f64;
        let upper_leg = (hip_height * 0.48).max(0.05) as f64;
        let lower_leg = (hip_height * 0.52).max(0.05) as f64;
        let spine_len = (body_len * 0.28).max(0.06) as f64;
        let neck_len = (body_len * 0.18).max(0.04) as f64;
        let tail_seg = (body_len * 0.32).max(0.06) as f64;

        // Pelvis / Root
        rest_pose[JOINT_PELVIS_ROOT] = Motor::from_translation(0.0, hip_height as f64, 0.0);
        // Spine & Chest (forward along +Z)
        rest_pose[JOINT_SPINE] = Motor::from_translation(0.0, 0.04, spine_len * 0.5);
        rest_pose[JOINT_CHEST] = Motor::from_translation(0.0, 0.03, spine_len * 0.5);
        // Neck, Head, Jaw (forward + upward)
        rest_pose[JOINT_NECK] = Motor::from_translation(0.0, neck_len * 0.6, neck_len * 0.6);
        rest_pose[JOINT_HEAD] = Motor::from_translation(0.0, neck_len * 0.3, neck_len * 0.5);
        rest_pose[JOINT_JAW] = Motor::from_translation(0.0, -0.04, neck_len * 0.2);

        // Hindlimbs (Left & Right Hips, Knees, Ankles)
        rest_pose[JOINT_LEFT_HIP] = Motor::from_translation(-half_width, -0.02, -0.02);
        rest_pose[JOINT_LEFT_KNEE] = Motor::from_translation(0.0, -upper_leg, 0.04);
        rest_pose[JOINT_LEFT_ANKLE] = Motor::from_translation(0.0, -lower_leg, -0.04);

        rest_pose[JOINT_RIGHT_HIP] = Motor::from_translation(half_width, -0.02, -0.02);
        rest_pose[JOINT_RIGHT_KNEE] = Motor::from_translation(0.0, -upper_leg, 0.04);
        rest_pose[JOINT_RIGHT_ANKLE] = Motor::from_translation(0.0, -lower_leg, -0.04);

        // Forelimbs / Wings
        let fore_upper = upper_leg * 0.8;
        let fore_lower = lower_leg * 0.8;
        rest_pose[JOINT_LEFT_SHOULDER] = Motor::from_translation(-half_width * 0.9, -0.04, 0.04);
        rest_pose[JOINT_LEFT_ELBOW] = Motor::from_translation(0.0, -fore_upper, -0.02);
        rest_pose[JOINT_LEFT_WRIST_WING] = Motor::from_translation(0.0, -fore_lower, 0.02);

        // Articulated Tail (backward along -Z)
        rest_pose[JOINT_TAIL_BASE] = Motor::from_translation(0.0, -0.02, -tail_seg * 0.6);
        rest_pose[JOINT_TAIL_MID] = Motor::from_translation(0.0, -0.04, -tail_seg * 0.8);
        rest_pose[JOINT_TAIL_TIP] = Motor::from_translation(0.0, -0.04, -tail_seg * 0.8);

        // 2. Compute World Rest and Inverse Bind Motors
        let mut world_rest = [Motor::identity(); FAUNA_MAX_JOINTS];
        forward_kinematics_zero_heap(&parent_indices, &rest_pose, &mut world_rest);

        let mut joints = [Joint {
            parent: None,
            rest_local: Motor::identity(),
            inverse_bind: Motor::identity(),
        }; FAUNA_MAX_JOINTS];

        for j in 0..joint_count {
            let p = parent_indices[j];
            joints[j] = Joint {
                parent: if p >= 0 { Some(p as u16) } else { None },
                rest_local: rest_pose[j],
                inverse_bind: world_rest[j].conjugate().normalize(),
            };
        }

        // 3. Synthesize 5 Cardinal Locomotion Clips
        let mut clip_poses = [rest_pose; FAUNA_CLIP_COUNT];

        // CLIP 0: Idle (rest pose with baseline postural tension)
        clip_poses[CLIP_IDLE] = rest_pose;

        // CLIP 1: Walk / Stalk (subtle alternating stride, spine yaw, tail counterbalance)
        let mut walk_pose = rest_pose;
        let walk_hip_rot = Motor::from_rotation_translation([0.98, 0.18, 0.0, 0.0], [0.0, 0.0, 0.0]);
        let walk_hip_rev = Motor::from_rotation_translation([0.98, -0.18, 0.0, 0.0], [0.0, 0.0, 0.0]);
        let walk_spine_yaw = Motor::from_rotation_translation([0.99, 0.0, 0.08, 0.0], [0.0, 0.0, 0.0]);
        let walk_tail_sway = Motor::from_rotation_translation([0.99, 0.0, -0.12, 0.0], [0.0, 0.0, 0.0]);

        walk_pose[JOINT_LEFT_HIP] = walk_pose[JOINT_LEFT_HIP].mul(&walk_hip_rot).normalize();
        walk_pose[JOINT_RIGHT_HIP] = walk_pose[JOINT_RIGHT_HIP].mul(&walk_hip_rev).normalize();
        walk_pose[JOINT_SPINE] = walk_pose[JOINT_SPINE].mul(&walk_spine_yaw).normalize();
        walk_pose[JOINT_TAIL_MID] = walk_pose[JOINT_TAIL_MID].mul(&walk_tail_sway).normalize();
        clip_poses[CLIP_WALK_STALK] = walk_pose;

        // CLIP 2: Cruise / Trot / Glide (deeper stride or wing expansion)
        let mut cruise_pose = rest_pose;
        let cruise_hip_rot = Motor::from_rotation_translation([0.94, 0.34, 0.0, 0.0], [0.0, 0.0, 0.0]);
        let cruise_hip_rev = Motor::from_rotation_translation([0.94, -0.34, 0.0, 0.0], [0.0, 0.0, 0.0]);
        let cruise_chest_pitch = Motor::from_rotation_translation([0.99, -0.10, 0.0, 0.0], [0.0, 0.0, 0.0]);

        cruise_pose[JOINT_LEFT_HIP] = cruise_pose[JOINT_LEFT_HIP].mul(&cruise_hip_rot).normalize();
        cruise_pose[JOINT_RIGHT_HIP] = cruise_pose[JOINT_RIGHT_HIP].mul(&cruise_hip_rev).normalize();
        cruise_pose[JOINT_CHEST] = cruise_pose[JOINT_CHEST].mul(&cruise_chest_pitch).normalize();
        clip_poses[CLIP_CRUISE_TROT_GLIDE] = cruise_pose;

        // CLIP 3: Sprint / Saltatorial Hop / Dive (synchronous limb flex or rapid propulsion)
        let mut sprint_pose = rest_pose;
        let sprint_hip_rot = Motor::from_rotation_translation([0.88, 0.48, 0.0, 0.0], [0.0, 0.0, 0.0]);
        let sprint_knee_rot = Motor::from_rotation_translation([0.92, -0.38, 0.0, 0.0], [0.0, 0.0, 0.0]);
        let sprint_spine_pitch = Motor::from_rotation_translation([0.97, 0.22, 0.0, 0.0], [0.0, 0.0, 0.0]);

        sprint_pose[JOINT_LEFT_HIP] = sprint_pose[JOINT_LEFT_HIP].mul(&sprint_hip_rot).normalize();
        sprint_pose[JOINT_LEFT_KNEE] = sprint_pose[JOINT_LEFT_KNEE].mul(&sprint_knee_rot).normalize();
        sprint_pose[JOINT_RIGHT_HIP] = sprint_pose[JOINT_RIGHT_HIP].mul(&sprint_hip_rot).normalize();
        sprint_pose[JOINT_RIGHT_KNEE] = sprint_pose[JOINT_RIGHT_KNEE].mul(&sprint_knee_rot).normalize();
        sprint_pose[JOINT_SPINE] = sprint_pose[JOINT_SPINE].mul(&sprint_spine_pitch).normalize();
        clip_poses[CLIP_SPRINT_HOP_DIVE] = sprint_pose;

        // CLIP 4: Turn (asymmetric yaw on spine, neck and tail)
        let mut turn_pose = rest_pose;
        let turn_spine_yaw = Motor::from_rotation_translation([0.95, 0.0, 0.31, 0.0], [0.0, 0.0, 0.0]);
        let turn_neck_yaw = Motor::from_rotation_translation([0.93, 0.0, 0.36, 0.0], [0.0, 0.0, 0.0]);
        let turn_tail_counter = Motor::from_rotation_translation([0.94, 0.0, -0.34, 0.0], [0.0, 0.0, 0.0]);

        turn_pose[JOINT_SPINE] = turn_pose[JOINT_SPINE].mul(&turn_spine_yaw).normalize();
        turn_pose[JOINT_NECK] = turn_pose[JOINT_NECK].mul(&turn_neck_yaw).normalize();
        turn_pose[JOINT_TAIL_MID] = turn_pose[JOINT_TAIL_MID].mul(&turn_tail_counter).normalize();
        clip_poses[CLIP_TURN] = turn_pose;

        // 4. Build 2D Delaunay Blend Space
        let mut blend_space_2d = BlendSpace2D::<16, 32>::new();
        // 5 cardinal points forming non-collinear Delaunay triangles
        blend_space_2d.add_point(0.0, 0.0, CLIP_IDLE as u16);
        blend_space_2d.add_point(0.35, -0.7, CLIP_WALK_STALK as u16);
        blend_space_2d.add_point(0.70, 0.0, CLIP_CRUISE_TROT_GLIDE as u16);
        blend_space_2d.add_point(1.00, 0.0, CLIP_SPRINT_HOP_DIVE as u16);
        blend_space_2d.add_point(0.35, 0.7, CLIP_TURN as u16);
        blend_space_2d.build_triangulation();

        // 5. Build Multi-Layer ABSM
        let mut absm = AbsmMachine::<4, 16, 32>::new();
        absm.blend_spaces_2d[0] = blend_space_2d;

        // Layer 0: BaseLocomotion (Full body locomotion driven by 2D blend space)
        let mut base_layer = AbsmLayer::new(BoneLayerMask::ALL, LayerBlendMode::Override, 1.0);
        let mut loco_node = AbsmStateNode::default();
        loco_node.source = AbsmStateSource::Blend2D {
            space_index: 0,
            param_idx: 0, // Vec2: [forward_speed_norm, turn_rate_rad]
        };
        loco_node.duration = 1.0;
        loco_node.speed = 1.0;
        loco_node.looping = true;
        loco_node.triggers[0] = SignalTrigger {
            active: true,
            signal_id: SIG_FOOTSTEP_LEFT,
            normalized_time: 0.25,
            sonic_archetype: 1,
            payload_f32: 0.5,
        };
        loco_node.triggers[1] = SignalTrigger {
            active: true,
            signal_id: SIG_FOOTSTEP_RIGHT,
            normalized_time: 0.75,
            sonic_archetype: 1,
            payload_f32: 0.5,
        };
        base_layer.add_state(loco_node);

        let mut idle_node = AbsmStateNode::default();
        idle_node.source = AbsmStateSource::SingleClip {
            clip_index: CLIP_IDLE as u16,
        };
        idle_node.duration = 2.0;
        idle_node.looping = true;
        base_layer.add_state(idle_node);

        // Transitions between Idle (state 1) and Locomotion (state 0) driven by scalar speed param (idx 1)
        base_layer.add_transition(
            1,
            0,
            0.15,
            TransitionCondition::ParamGt {
                param_idx: 1,
                threshold: 0.05,
            },
        );
        base_layer.add_transition(
            0,
            1,
            0.25,
            TransitionCondition::ParamLt {
                param_idx: 1,
                threshold: 0.05,
            },
        );
        absm.add_layer(base_layer);

        // Layer 1: UpperBodyVocalizeForage (Masked layer covering Chest, Neck, Head, Jaw, Forelimbs)
        let upper_body_mask = BoneLayerMask::from_joint_subtree(&parent_indices, JOINT_CHEST);
        let mut upper_layer = AbsmLayer::new(upper_body_mask, LayerBlendMode::Override, 0.0);

        let mut forage_node = AbsmStateNode::default();
        forage_node.source = AbsmStateSource::SingleClip {
            clip_index: CLIP_WALK_STALK as u16,
        };
        forage_node.duration = 1.5;
        forage_node.looping = true;
        upper_layer.add_state(forage_node);

        let mut vocal_node = AbsmStateNode::default();
        vocal_node.source = AbsmStateSource::SingleClip {
            clip_index: CLIP_TURN as u16,
        };
        vocal_node.duration = 1.2;
        vocal_node.looping = false;
        vocal_node.triggers[0] = SignalTrigger {
            active: true,
            signal_id: SIG_VOCAL_CALL,
            normalized_time: 0.50,
            sonic_archetype: 10,
            payload_f32: 1.0,
        };
        upper_layer.add_state(vocal_node);

        // Transition on bool parameter 2 (is_vocalizing)
        upper_layer.add_transition(
            0,
            1,
            0.10,
            TransitionCondition::ParamEqBool {
                param_idx: 2,
                expected: true,
            },
        );
        upper_layer.add_transition(
            1,
            0,
            0.20,
            TransitionCondition::ParamEqBool {
                param_idx: 2,
                expected: false,
            },
        );
        absm.add_layer(upper_layer);

        // Layer 2: HeadLookAtPosture (Additive neck/head modulation)
        let neck_mask = BoneLayerMask::from_joint_subtree(&parent_indices, JOINT_NECK);
        let mut head_layer = AbsmLayer::new(neck_mask, LayerBlendMode::Additive, 0.5);
        let mut head_node = AbsmStateNode::default();
        head_node.source = AbsmStateSource::SingleClip {
            clip_index: CLIP_IDLE as u16,
        };
        head_node.duration = 1.0;
        head_layer.add_state(head_node);
        absm.add_layer(head_layer);

        // Configure EONT1 Posture Fatigue / Age Droop
        absm.eont_posture = EontPostureConfig {
            enabled: true,
            spine_mask: BoneLayerMask::from_joint_subtree(&parent_indices, JOINT_SPINE),
            stage: 2, // Adult baseline
            energy_permill: 1000,
            damage_permill: 0,
            hydration_permill: 1000,
            max_droop_rad: 0.35,
        };

        // Configure Root Motion Extractor on root pelvis joint
        absm.root_motion = Some(RootMotionExtractor::new(
            JOINT_PELVIS_ROOT,
            rest_pose[JOINT_PELVIS_ROOT],
            rest_pose[JOINT_PELVIS_ROOT],
        ));

        // 6. Initialize GPU Palette & Tracking
        let prev_skin_motors = [Motor::identity(); FAUNA_MAX_JOINTS];
        let current_skin_motors = [Motor::identity(); FAUNA_MAX_JOINTS];
        let gpu_palette = [GpuJointMotorPair::identity(); FAUNA_MAX_JOINTS];
        let signal_queue = SignalQueue::new();

        Self {
            archetype,
            joint_count,
            joints,
            parent_indices,
            rest_pose,
            clip_poses,
            blend_space_2d,
            absm,
            prev_skin_motors,
            current_skin_motors,
            gpu_palette,
            signal_queue,
        }
    }

    /// Sets an ABSM control parameter (e.g. idx 0: Vec2 `[speed, turn]`, idx 1: Float `speed`, idx 2: Bool `is_vocalizing`).
    #[inline]
    pub fn set_parameter(&mut self, idx: usize, param: AbsmParameter) {
        self.absm.set_parameter(idx, param);
    }

    /// Samples the 2D Bowyer–Watson Delaunay blend space at `(forward_speed_norm, turn_rate_rad)`
    /// blending cardinal clip poses directly with `Motor::sclerp` (zero heap allocations).
    pub fn sample_blend_space(
        &self,
        forward_speed_norm: f32,
        turn_rate_rad: f32,
        out_pose: &mut [Motor],
    ) -> Result<BlendSample2D, SkeletalError> {
        let clip_refs: [&[Motor]; FAUNA_CLIP_COUNT] = [
            &self.clip_poses[0][..self.joint_count],
            &self.clip_poses[1][..self.joint_count],
            &self.clip_poses[2][..self.joint_count],
            &self.clip_poses[3][..self.joint_count],
            &self.clip_poses[4][..self.joint_count],
        ];
        let (sample, _) = sample_blend_space_2d_zero_heap(
            &self.blend_space_2d,
            [forward_speed_norm, turn_rate_rad],
            &clip_refs,
            out_pose,
        )?;
        Ok(sample)
    }

    /// Advances the multi-layer ABSM by `dt` seconds, evaluating layer blending, EONT1
    /// posture modulation, and root motion extraction while pushing crossed timeline signals
    /// into `self.signal_queue` (`HotZeroHeap`).
    pub fn step_absm(
        &mut self,
        dt: f32,
        out_local_pose: &mut [Motor],
    ) -> Result<RootMotionDelta, SkeletalError> {
        let clip_refs: [&[Motor]; FAUNA_CLIP_COUNT] = [
            &self.clip_poses[0][..self.joint_count],
            &self.clip_poses[1][..self.joint_count],
            &self.clip_poses[2][..self.joint_count],
            &self.clip_poses[3][..self.joint_count],
            &self.clip_poses[4][..self.joint_count],
        ];

        let delta = self.absm.step_and_evaluate_zero_heap(
            dt,
            &self.rest_pose[..self.joint_count],
            &clip_refs,
            &mut out_local_pose[..self.joint_count],
            &mut self.signal_queue,
        )?;

        Ok(delta)
    }

    // ── Epic 3: PROC1 Motor Oscillation ──────────────────────────────────────

    /// Applies procedural respiration breathing pitch oscillation onto the Chest and Spine joints.
    pub fn apply_proc1_respiration_oscillation(
        &self,
        pose: &mut [Motor],
        time_sec: f32,
        amplitude_rad: f32,
    ) {
        const RESPIRATION_HZ: f32 = 0.35;
        let angle = (amplitude_rad
            * (std::f32::consts::TAU * RESPIRATION_HZ * time_sec).sin()) as f64;
        let half = angle * 0.5;
        let pitch_rotor = Motor::from_rotation_translation(
            [half.cos(), half.sin(), 0.0, 0.0],
            [0.0, 0.0, 0.0],
        );

        if JOINT_CHEST < pose.len() {
            pose[JOINT_CHEST] = pose[JOINT_CHEST].mul(&pitch_rotor).normalize();
        }
        if JOINT_SPINE < pose.len() {
            let half_spine = half * 0.5;
            let spine_rotor = Motor::from_rotation_translation(
                [half_spine.cos(), half_spine.sin(), 0.0, 0.0],
                [0.0, 0.0, 0.0],
            );
            pose[JOINT_SPINE] = pose[JOINT_SPINE].mul(&spine_rotor).normalize();
        }
    }

    /// Applies continuous procedural caudal (tail) undulation along the Tail Base, Mid, and Tip joints.
    pub fn apply_proc1_tail_undulation(
        &self,
        pose: &mut [Motor],
        time_sec: f32,
        frequency_hz: f32,
        amplitude_rad: f32,
    ) {
        let tail_joints = [JOINT_TAIL_BASE, JOINT_TAIL_MID, JOINT_TAIL_TIP];
        for (i, &j_idx) in tail_joints.iter().enumerate() {
            if j_idx < pose.len() {
                let phase = (i as f32) * 0.85;
                let yaw_angle = (amplitude_rad
                    * (std::f32::consts::TAU * frequency_hz * time_sec - phase).sin())
                    as f64;
                let half = yaw_angle * 0.5;
                let yaw_rotor = Motor::from_rotation_translation(
                    [half.cos(), 0.0, half.sin(), 0.0],
                    [0.0, 0.0, 0.0],
                );
                pose[j_idx] = pose[j_idx].mul(&yaw_rotor).normalize();
            }
        }
    }

    // ── Epic 4: Inverse Kinematics ───────────────────────────────────────────

    /// Solves analytical Two-Bone Motor IK for both hindlimbs (Left and Right Hip-Knee-Ankle)
    /// to adapt foot placement to sloped East Coast terrain elevations (`HotZeroHeap`).
    pub fn solve_hindlimb_terrain_ik(
        &self,
        local_pose: &mut [Motor],
        left_foot_target: [f32; 3],
        right_foot_target: [f32; 3],
    ) -> Result<(TwoBoneIkResult, TwoBoneIkResult), SkeletalError> {
        let mut world_motors = [Motor::identity(); FAUNA_MAX_JOINTS];
        forward_kinematics_zero_heap(
            &self.parent_indices,
            local_pose,
            &mut world_motors[..self.joint_count],
        );

        // Left Hindlimb
        let (_, _p_l_hip) = world_motors[JOINT_LEFT_HIP].to_rotation_translation();
        let (_, p_l_knee) = world_motors[JOINT_LEFT_KNEE].to_rotation_translation();
        let left_pole = [p_l_knee[0] as f32, p_l_knee[1] as f32, (p_l_knee[2] + 0.6) as f32];

        let left_res = solve_two_bone_motor_ik_zero_heap(
            world_motors[JOINT_LEFT_HIP],
            world_motors[JOINT_LEFT_KNEE],
            world_motors[JOINT_LEFT_ANKLE],
            0.0,
            0.0,
            left_foot_target,
            left_pole,
            0.1,
            std::f32::consts::PI - 0.05,
        )?;

        world_motors[JOINT_LEFT_HIP] = left_res.world_motors[0];
        world_motors[JOINT_LEFT_KNEE] = left_res.world_motors[1];
        world_motors[JOINT_LEFT_ANKLE] = left_res.world_motors[2];

        let p_pelvis = world_motors[JOINT_PELVIS_ROOT];
        local_pose[JOINT_LEFT_HIP] = world_to_local_motor(&p_pelvis, &world_motors[JOINT_LEFT_HIP]);
        local_pose[JOINT_LEFT_KNEE] = left_res.local_motors[1];
        local_pose[JOINT_LEFT_ANKLE] = left_res.local_motors[2];

        // Right Hindlimb
        let (_, _p_r_hip) = world_motors[JOINT_RIGHT_HIP].to_rotation_translation();
        let (_, p_r_knee) = world_motors[JOINT_RIGHT_KNEE].to_rotation_translation();
        let right_pole = [p_r_knee[0] as f32, p_r_knee[1] as f32, (p_r_knee[2] + 0.6) as f32];

        let right_res = solve_two_bone_motor_ik_zero_heap(
            world_motors[JOINT_RIGHT_HIP],
            world_motors[JOINT_RIGHT_KNEE],
            world_motors[JOINT_RIGHT_ANKLE],
            0.0,
            0.0,
            right_foot_target,
            right_pole,
            0.1,
            std::f32::consts::PI - 0.05,
        )?;

        world_motors[JOINT_RIGHT_HIP] = right_res.world_motors[0];
        world_motors[JOINT_RIGHT_KNEE] = right_res.world_motors[1];
        world_motors[JOINT_RIGHT_ANKLE] = right_res.world_motors[2];

        local_pose[JOINT_RIGHT_HIP] = world_to_local_motor(&p_pelvis, &world_motors[JOINT_RIGHT_HIP]);
        local_pose[JOINT_RIGHT_KNEE] = right_res.local_motors[1];
        local_pose[JOINT_RIGHT_ANKLE] = right_res.local_motors[2];

        Ok((left_res, right_res))
    }

    /// Solves Constrained Motor FABRIK on the articulated tail chain (Tail Base, Mid, Tip)
    /// enforcing conical swing limits and hinge constraints (`HotZeroHeap`).
    pub fn solve_tail_fabrik_ik(
        &self,
        local_pose: &mut [Motor],
        target_tip: [f32; 3],
    ) -> Result<f32, SkeletalError> {
        let mut world_motors = [Motor::identity(); FAUNA_MAX_JOINTS];
        forward_kinematics_zero_heap(
            &self.parent_indices,
            local_pose,
            &mut world_motors[..self.joint_count],
        );

        let tail_world = [
            world_motors[JOINT_TAIL_BASE],
            world_motors[JOINT_TAIL_MID],
            world_motors[JOINT_TAIL_TIP],
        ];

        let constraints = [
            JointConstraint::ConeLimit {
                max_half_angle_rad: 0.52, // 30 deg cone limit
            },
            JointConstraint::HingeLimit {
                axis: [0.0, 1.0, 0.0],
                min_angle_rad: -0.78, // 45 deg lateral hinge
                max_angle_rad: 0.78,
            },
            JointConstraint::Unconstrained,
        ];

        let mut out_world = [Motor::identity(); 3];
        let mut out_local = [Motor::identity(); 3];

        let err = solve_constrained_fabrik_motor_ik_zero_heap(
            &tail_world,
            &constraints,
            target_tip,
            1e-3,
            16,
            &mut out_world,
            &mut out_local,
        )?;

        let pelvis_world = world_motors[JOINT_PELVIS_ROOT];
        local_pose[JOINT_TAIL_BASE] = world_to_local_motor(&pelvis_world, &out_world[0]);
        local_pose[JOINT_TAIL_MID] = out_local[1];
        local_pose[JOINT_TAIL_TIP] = out_local[2];

        Ok(err)
    }

    // ── Epic 3: GPU Skinning Palette & Morph Target Evaluation ────────────────

    /// Evaluates the GPU dual-quaternion skinning palette into `self.gpu_palette` (`GpuJointMotorPair`),
    /// computing $M_{\text{skin}} = M_{\text{world}} \circ M_{\text{inv\_bind}}$ and updating previous-frame
    /// motors for exact TAA velocity vector evaluation (`HotZeroHeap`).
    pub fn evaluate_gpu_palette(&mut self, local_pose: &[Motor]) -> usize {
        let mut world_motors = [Motor::identity(); FAUNA_MAX_JOINTS];
        forward_kinematics_zero_heap(
            &self.parent_indices,
            local_pose,
            &mut world_motors[..self.joint_count],
        );

        let mut bind_inv = [Motor::identity(); FAUNA_MAX_JOINTS];
        for j in 0..self.joint_count {
            bind_inv[j] = self.joints[j].inverse_bind;
        }

        let count = evaluate_skinning_motor_palette_zero_heap(
            &bind_inv[..self.joint_count],
            &world_motors[..self.joint_count],
            &self.prev_skin_motors[..self.joint_count],
            &mut self.gpu_palette[..self.joint_count],
            &mut self.current_skin_motors[..self.joint_count],
        );

        for j in 0..count {
            self.prev_skin_motors[j] = self.current_skin_motors[j];
        }

        count
    }

    /// Computes 48-byte `GpuInstanceMorphWeights` combining explicit facial/vocal blend shape
    /// weights with `.10d` `EONT1` metabolic damage, hydration, and lifecycle growth factors (`HotZeroHeap`).
    pub fn compute_morph_weights(
        &self,
        explicit_weights: &[f32],
        stage: u8,
        damage_permill: u16,
        hydration_permill: u16,
    ) -> GpuInstanceMorphWeights {
        compute_ontogenetic_morph_weights_zero_heap(
            explicit_weights,
            4, // 4 active morph targets (respiration, vocal_gular, ontogenetic_growth, hydration_condition)
            1, // vertex_stride
            stage,
            damage_permill,
            hydration_permill,
        )
    }

    /// Displaces authored bind-pose vertex positions and normals by pre-skinning GPU morph deltas (`HotZeroHeap`).
    pub fn evaluate_morph_deltas(
        &self,
        base_positions: &[[f32; 3]],
        base_normals: &[[f32; 3]],
        morph_deltas: &[GpuMorphDelta],
        weights: &GpuInstanceMorphWeights,
        out_positions: &mut [[f32; 3]],
        out_normals: &mut [[f32; 3]],
    ) -> Result<usize, SkeletalError> {
        evaluate_morph_deltas_zero_heap(
            base_positions,
            base_normals,
            morph_deltas,
            weights,
            out_positions,
            out_normals,
        )
    }

    /// Software reference execution of the GPU WGSL dual-quaternion skinning vertex shader,
    /// returning `(skinned_pos, skinned_normal, skinned_tangent, taa_motion_vector)`.
    pub fn verify_vertex_skinning(
        &self,
        pos: [f32; 3],
        normal: [f32; 3],
        tangent: [f32; 4],
        skin_attr: &GpuSkinnedVertex,
    ) -> ([f32; 3], [f32; 3], [f32; 4], [f32; 3]) {
        gpu_shader_skin_vertex_reference(
            pos,
            normal,
            tangent,
            skin_attr,
            &self.gpu_palette[..self.joint_count],
        )
    }
}

// ── Unit Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_13_ARCHETYPES: [FaunaBiomechanicalArchetype; 13] = [
        FaunaBiomechanicalArchetype::MacropodSaltatorial,
        FaunaBiomechanicalArchetype::VombatidFossorial,
        FaunaBiomechanicalArchetype::PhascolarctidArboreal,
        FaunaBiomechanicalArchetype::PetauridGlider,
        FaunaBiomechanicalArchetype::DasyuridPredator,
        FaunaBiomechanicalArchetype::PteropodidChiroptera,
        FaunaBiomechanicalArchetype::MarinePinnipedCetacean,
        FaunaBiomechanicalArchetype::RatiteBipedal,
        FaunaBiomechanicalArchetype::RaptorSoaring,
        FaunaBiomechanicalArchetype::PasserinePsittacine,
        FaunaBiomechanicalArchetype::SquamateSprawling,
        FaunaBiomechanicalArchetype::SerpentineUndulatory,
        FaunaBiomechanicalArchetype::MonotremeTeleostAmphibian,
    ];

    #[test]
    fn test_rig_construction_all_13_archetypes() {
        for archetype in ALL_13_ARCHETYPES {
            let mut rig = FaunaSkeletalRig10d::new(archetype);
            assert!(
                rig.joint_count >= 15 && rig.joint_count <= FAUNA_MAX_JOINTS,
                "archetype {:?} has joint count {}",
                archetype,
                rig.joint_count
            );
            assert_eq!(rig.blend_space_2d.point_count, 5);
            assert!(rig.blend_space_2d.tri_count > 0);
            assert_eq!(rig.absm.layer_count, 3);

            // Verify bind inverse cancels rest pose: skinning in bind pose produces identity motors
            let local_pose = rig.rest_pose;
            let count = rig.evaluate_gpu_palette(&local_pose);
            assert_eq!(count, rig.joint_count);

            for j in 0..count {
                let pair = rig.gpu_palette[j];
                // In bind pose, M_skin = M_world * M_inv_bind = Identity
                assert!(
                    (pair.current_r[0].abs() - 1.0).abs() < 1e-3,
                    "archetype {:?} joint {} scalar rotor = {}",
                    archetype,
                    j,
                    pair.current_r[0]
                );
                assert!(
                    pair.current_d[0].abs() < 1e-3
                        && pair.current_d[1].abs() < 1e-3
                        && pair.current_d[2].abs() < 1e-3
                        && pair.current_d[3].abs() < 1e-3,
                    "archetype {:?} joint {} dual translation must be near 0 in bind pose",
                    archetype,
                    j
                );
            }
        }
    }

    #[test]
    fn test_blend_space_2d_delaunay_sampling() {
        let rig = FaunaSkeletalRig10d::new(FaunaBiomechanicalArchetype::MacropodSaltatorial);
        let mut out_pose = [Motor::identity(); FAUNA_MAX_JOINTS];

        // Sample at origin (Idle)
        let sample_idle = rig.sample_blend_space(0.0, 0.0, &mut out_pose).unwrap();
        assert_eq!(sample_idle.clips[0], CLIP_IDLE as u16);
        assert!((sample_idle.weights[0] - 1.0).abs() < 1e-4);

        // Sample at max sprint
        let sample_sprint = rig.sample_blend_space(1.0, 0.0, &mut out_pose).unwrap();
        let sprint_idx = sample_sprint
            .clips
            .iter()
            .position(|&c| c == CLIP_SPRINT_HOP_DIVE as u16);
        assert!(sprint_idx.is_some());
        let w = sample_sprint.weights[sprint_idx.unwrap()];
        assert!(w > 0.8, "Sprint weight must dominate at speed=1.0: w={w}");

        // Sample at intermediate coordinate (0.5, 0.1)
        let sample_mid = rig.sample_blend_space(0.5, 0.1, &mut out_pose).unwrap();
        let weight_sum = sample_mid.weights[0] + sample_mid.weights[1] + sample_mid.weights[2];
        assert!((weight_sum - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_absm_stepping_transitions_and_signal_emission() {
        let mut rig = FaunaSkeletalRig10d::new(FaunaBiomechanicalArchetype::MacropodSaltatorial);
        let mut local_pose = rig.rest_pose;

        // Start stationary: speed = 0.0
        rig.set_parameter(0, AbsmParameter::Vec2([0.0, 0.0]));
        rig.set_parameter(1, AbsmParameter::Float(0.0));
        let _ = rig.step_absm(0.05, &mut local_pose).unwrap();

        // Accelerate into locomotion: speed = 0.6
        rig.set_parameter(0, AbsmParameter::Vec2([0.6, 0.0]));
        rig.set_parameter(1, AbsmParameter::Float(0.6));

        // Step through full cycle (1.0 sec in steps of 0.1)
        let mut saw_signals = false;
        for _ in 0..15 {
            let delta = rig.step_absm(0.1, &mut local_pose).unwrap();
            assert!(delta.delta_rotor[0].is_finite());
            while let Some(sig) = rig.signal_queue.pop() {
                if sig.signal_id == SIG_FOOTSTEP_LEFT || sig.signal_id == SIG_FOOTSTEP_RIGHT {
                    saw_signals = true;
                }
            }
        }
        assert!(saw_signals, "Locomotion ABSM layer must emit footstep signals");
    }

    #[test]
    fn test_two_bone_and_constrained_fabrik_ik() {
        let rig = FaunaSkeletalRig10d::new(FaunaBiomechanicalArchetype::MacropodSaltatorial);
        let mut pose = rig.rest_pose;

        // Sloped East Coast terrain targets
        let left_target = [-0.15f32, 0.05, 0.0];
        let right_target = [0.15f32, -0.05, 0.0];

        let (left_res, right_res) = rig
            .solve_hindlimb_terrain_ik(&mut pose, left_target, right_target)
            .unwrap();
        assert!(left_res.target_error < 0.05);
        assert!(right_res.target_error < 0.05);

        // Constrained FABRIK tail adaptation (within 30-degree cone limit)
        let tail_target = [0.0f32, 0.72, -0.82];
        let tail_err = rig.solve_tail_fabrik_ik(&mut pose, tail_target).unwrap();
        assert!(tail_err < 0.15, "Tail FABRIK error must converge: {tail_err}");
    }

    #[test]
    fn test_gpu_palette_evaluation_and_taa_motion_vectors() {
        let mut rig = FaunaSkeletalRig10d::new(FaunaBiomechanicalArchetype::DasyuridPredator);
        let mut pose = rig.rest_pose;

        // Initial frame evaluation
        rig.evaluate_gpu_palette(&pose);

        // Perturb root translation by +0.10 m in Z for frame 2
        pose[JOINT_PELVIS_ROOT] =
            Motor::from_translation(0.0, 0.25, 0.10);
        rig.evaluate_gpu_palette(&pose);

        let test_vertex = [0.0f32, 0.25, 0.0];
        let test_normal = [0.0f32, 1.0, 0.0];
        let test_tangent = [1.0f32, 0.0, 0.0, 1.0];
        let skin_attr = GpuSkinnedVertex {
            joint_indices: [JOINT_PELVIS_ROOT as u16, 0, 0, 0],
            joint_weights: [1.0, 0.0, 0.0, 0.0],
        };

        let (curr_p, _, _, motion_vec) =
            rig.verify_vertex_skinning(test_vertex, test_normal, test_tangent, &skin_attr);

        // Motion vector = curr_p - prev_p. Since root moved +0.10 in Z, motion_vec[2] should be ~0.10
        assert!(
            (motion_vec[2] - 0.10).abs() < 1e-2,
            "TAA velocity vector Z must capture +0.10m displacement: actual={}",
            motion_vec[2]
        );
        assert!(curr_p[2].is_finite());
    }

    #[test]
    fn test_ontogenetic_morph_weights_and_deltas() {
        let rig = FaunaSkeletalRig10d::new(FaunaBiomechanicalArchetype::SquamateSprawling);
        let explicit = [0.3f32, 0.7, 0.0, 0.0];

        // Stage 1 (Juvenile), 200 damage permill, 800 hydration permill
        let weights = rig.compute_morph_weights(&explicit, 1, 200, 800);
        assert_eq!(weights.weights[0], 0.3);
        assert_eq!(weights.weights[1], 0.7);
        assert_eq!(weights.weights[5], 0.50); // Juvenile growth factor
        assert_eq!(weights.weights[6], 0.20); // 200 permill damage
        assert!((weights.weights[7] - 0.20).abs() < 1e-4); // 1.0 - 0.8 hydration

        // Displace base vertex via morph deltas
        let base_pos = [[0.0f32, 1.0, 0.0]];
        let base_norm = [[0.0f32, 1.0, 0.0]];
        let mut deltas = [GpuMorphDelta::default(); 8];
        deltas[0] = GpuMorphDelta::new([0.0, 0.1, 0.0], [0.0, 0.0, 0.0]); // Respiration expansion

        let mut out_pos = [[0.0f32; 3]];
        let mut out_norm = [[0.0f32; 3]];
        let n = rig
            .evaluate_morph_deltas(&base_pos, &base_norm, &deltas, &weights, &mut out_pos, &mut out_norm)
            .unwrap();
        assert_eq!(n, 1);
        // Displaced by 0.3 * 0.1 = 0.03 in Y
        assert!((out_pos[0][1] - 1.03).abs() < 1e-4);
    }
}
