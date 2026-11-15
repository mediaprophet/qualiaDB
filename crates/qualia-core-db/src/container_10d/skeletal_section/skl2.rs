//! SKL2: versioned biomechanical skeletal joint descriptors (v1 32B / v2 64B),
//! loop-closure constraints, topological landmarks, actuation pathways, SSM1
//! morphable basis descriptors, and bit-packed epistemic provenance.
//!
//! All records are little-endian and `bytemuck::{Pod, Zeroable}` with explicit
//! padding and CRC-32C integrity words. Decoding validates record alignment,
//! bounds, and the 42 MB container budget before returning descriptors.

use bytemuck::{Pod, Zeroable};
use qualia_cooperative_core::contextual_evidence::{
    EpistemicTier, ObservationMethod, ObserverRole, TemporalStabilityClass,
};

use super::SkeletalSectionError;
use crate::container_10d::crc32c::crc32c;

/// 4-byte magic identifier for SKL2 payloads.
pub const SKL2_MAGIC: [u8; 4] = *b"SKL2";
/// 4-byte magic identifier for Statistical Shape Model basis descriptors.
pub const SSM1_MAGIC: [u8; 4] = *b"SSM1";
/// SKL2 format version 1 (32-byte joint descriptor).
pub const SKL2_VERSION_1: u16 = 1;
/// SKL2 format version 2 (64-byte cache-line-aligned joint descriptor).
pub const SKL2_VERSION_2: u16 = 2;
/// SKL2 fixed header length in bytes.
pub const SKL2_HEADER_LEN: usize = 32;
/// Sentinel index denoting no parent / no partner / no linked record.
pub const SKL2_NONE_INDEX: u16 = u16::MAX;
/// Maximum joint descriptors permitted in a single SKL2 payload (Tier-0 limit).
pub const SKL2_MAX_JOINTS: usize = 256;
/// Global QualiaDB 42 MB payload sentinel.
pub const SKL2_MAX_CONTAINER_BYTES: usize = 42 * 1024 * 1024;

/// Joint-level descriptor flags.
pub const SKL2_JOINT_FLAG_SYMMETRIC: u8 = 1 << 0;
pub const SKL2_JOINT_FLAG_VESTIGIAL: u8 = 1 << 1;
pub const SKL2_JOINT_FLAG_HYDROSTATIC: u8 = 1 << 2;
pub const SKL2_JOINT_FLAG_EXOSKELETAL: u8 = 1 << 3;
pub const SKL2_JOINT_FLAG_LOOP_ANCHOR: u8 = 1 << 4;

/// Swing-twist anatomical cone joint limits in radians (16 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, serde::Serialize, serde::Deserialize)]
pub struct SwingTwistConeLimits {
    pub swing_y_rad: f32,
    pub swing_z_rad: f32,
    pub twist_min_rad: f32,
    pub twist_max_rad: f32,
}

impl SwingTwistConeLimits {
    #[inline]
    pub const fn new(
        swing_y_rad: f32,
        swing_z_rad: f32,
        twist_min_rad: f32,
        twist_max_rad: f32,
    ) -> Self {
        Self {
            swing_y_rad,
            swing_z_rad,
            twist_min_rad,
            twist_max_rad,
        }
    }

    #[inline]
    pub const fn symmetric(swing_rad: f32, twist_half_rad: f32) -> Self {
        Self {
            swing_y_rad: swing_rad,
            swing_z_rad: swing_rad,
            twist_min_rad: -twist_half_rad,
            twist_max_rad: twist_half_rad,
        }
    }

    #[inline]
    pub const fn rigid() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    #[inline]
    pub fn validate(&self) -> Result<(), SkeletalSectionError> {
        if !self.swing_y_rad.is_finite()
            || !self.swing_z_rad.is_finite()
            || !self.twist_min_rad.is_finite()
            || !self.twist_max_rad.is_finite()
        {
            return Err(SkeletalSectionError::InvalidData);
        }
        if self.swing_y_rad < 0.0
            || self.swing_z_rad < 0.0
            || self.swing_y_rad > core::f32::consts::PI
            || self.swing_z_rad > core::f32::consts::PI
            || self.twist_min_rad > self.twist_max_rad
            || self.twist_min_rad < -core::f32::consts::TAU
            || self.twist_max_rad > core::f32::consts::TAU
        {
            return Err(SkeletalSectionError::InvalidData);
        }
        Ok(())
    }
}

impl Default for SwingTwistConeLimits {
    fn default() -> Self {
        Self::symmetric(core::f32::consts::FRAC_PI_3, core::f32::consts::FRAC_PI_4)
    }
}

/// Governing mechanical solver regime for a structural element / joint.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum KinematicSolverRegime {
    #[default]
    ArticulatedRigidBodyDca = 0,
    CosseratRodContinuum = 1,
    ExoskeletalCuticleShell = 2,
    HydrostaticMuscularContinuum = 3,
    TurgorAuxinGrowth = 4,
}

impl KinematicSolverRegime {
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    #[inline]
    pub const fn from_u8(raw: u8) -> Result<Self, SkeletalSectionError> {
        match raw {
            0 => Ok(Self::ArticulatedRigidBodyDca),
            1 => Ok(Self::CosseratRodContinuum),
            2 => Ok(Self::ExoskeletalCuticleShell),
            3 => Ok(Self::HydrostaticMuscularContinuum),
            4 => Ok(Self::TurgorAuxinGrowth),
            _ => Err(SkeletalSectionError::InvalidData),
        }
    }
}

/// 32-bit bit-packed epistemic provenance word per Section 9.2:
/// `[3b status | 5b method_class | 3b observer_role | 3b stability_class | 10b freshness_q10 | 8b confidence_q8]`
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Pod, Zeroable)]
pub struct EpistemicProvenanceWord(pub u32);

impl EpistemicProvenanceWord {
    #[inline]
    pub const fn pack(
        status: u8,
        method_class: u8,
        observer_role: u8,
        stability_class: u8,
        freshness_q10: u16,
        confidence_q8: u8,
    ) -> Self {
        let word = ((status as u32) & 0x7)
            | (((method_class as u32) & 0x1F) << 3)
            | (((observer_role as u32) & 0x7) << 8)
            | (((stability_class as u32) & 0x7) << 11)
            | (((freshness_q10 as u32) & 0x3FF) << 14)
            | (((confidence_q8 as u32) & 0xFF) << 24);
        Self(word)
    }

    #[inline]
    pub fn from_contextual(
        tier: EpistemicTier,
        method: ObservationMethod,
        role: ObserverRole,
        stability: TemporalStabilityClass,
        freshness_q10: u16,
        confidence_q8: u8,
    ) -> Self {
        let role_code: u8 = match role {
            ObserverRole::SelfMeasured => 0,
            ObserverRole::PeerMeasured => 1,
            ObserverRole::ClinicianMeasured => 2,
            ObserverRole::IsakCertified => 3,
            ObserverRole::AutomatedSensor => 4,
            ObserverRole::CuratorExtracted => 5,
        };
        let stability_code: u8 = match stability {
            TemporalStabilityClass::AcutePhysiological => 0,
            TemporalStabilityClass::SubacuteMetabolicGravidAndHealing => 1,
            TemporalStabilityClass::OntogeneticAndSkeletalRemodeling => 2,
            TemporalStabilityClass::SkeletallyMatureInvariant => 3,
        };
        Self::pack(
            tier.rank(),
            method.category_code(),
            role_code,
            stability_code,
            freshness_q10,
            confidence_q8,
        )
    }

    #[inline]
    pub const fn status(self) -> u8 {
        (self.0 & 0x7) as u8
    }

    #[inline]
    pub const fn method_class(self) -> u8 {
        ((self.0 >> 3) & 0x1F) as u8
    }

    #[inline]
    pub const fn observer_role(self) -> u8 {
        ((self.0 >> 8) & 0x7) as u8
    }

    #[inline]
    pub const fn stability_class(self) -> u8 {
        ((self.0 >> 11) & 0x7) as u8
    }

    #[inline]
    pub const fn freshness_q10(self) -> u16 {
        ((self.0 >> 14) & 0x3FF) as u16
    }

    #[inline]
    pub const fn confidence_q8(self) -> u8 {
        ((self.0 >> 24) & 0xFF) as u8
    }
}

/// Legacy SKL2 v1 joint descriptor (32 bytes, little endian).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Skl2JointDescriptorV1 {
    pub joint_id: u64,
    pub parent_index: u16,
    pub loop_closure_partner: u16,
    pub solver_regime: u8,
    pub flags: u8,
    pub reserved: u16,
    pub swing_twist: SwingTwistConeLimits,
}

const _: () = assert!(core::mem::size_of::<Skl2JointDescriptorV1>() == 32);

/// SKL2 v2 64-byte cache-line-aligned biological joint descriptor.
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Skl2JointDescriptorV2 {
    pub joint_id: u64,
    pub parent_index: u16,
    pub loop_closure_partner: u16,
    pub landmark_index: u16,
    pub actuation_pathway_index: u16,
    pub swing_twist: SwingTwistConeLimits,
    pub cortical_bmd_mg_cm3: f32,
    pub trabecular_bv_tv: f32,
    pub bone_length_m: f32,
    pub segment_mass_kg: f32,
    pub solver_regime: u8,
    pub epistemic_tier: u8,
    pub observation_method: u16,
    pub confidence_q8: u8,
    pub flags: u8,
    pub reserved: u16,
    pub provenance_assertion_idx: u32,
    pub record_crc32c: u32,
}

const _: () = assert!(core::mem::size_of::<Skl2JointDescriptorV2>() == 64);
const _: () = assert!(core::mem::align_of::<Skl2JointDescriptorV2>() == 64);

impl Skl2JointDescriptorV2 {
    /// Construct a new sealed v2 descriptor with computed CRC-32C.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        joint_id: u64,
        parent_index: u16,
        swing_twist: SwingTwistConeLimits,
        solver_regime: KinematicSolverRegime,
        cortical_bmd_mg_cm3: f32,
        trabecular_bv_tv: f32,
        bone_length_m: f32,
        segment_mass_kg: f32,
        epistemic_tier: u8,
        observation_method: u16,
        confidence_q8: u8,
    ) -> Self {
        let desc = Self {
            joint_id,
            parent_index,
            loop_closure_partner: SKL2_NONE_INDEX,
            landmark_index: SKL2_NONE_INDEX,
            actuation_pathway_index: SKL2_NONE_INDEX,
            swing_twist,
            cortical_bmd_mg_cm3,
            trabecular_bv_tv,
            bone_length_m,
            segment_mass_kg,
            solver_regime: solver_regime.as_u8(),
            epistemic_tier,
            observation_method,
            confidence_q8,
            flags: 0,
            reserved: 0,
            provenance_assertion_idx: 0,
            record_crc32c: 0,
        };
        desc.with_sealed_crc()
    }

    #[inline]
    pub fn compute_crc32c(&self) -> u32 {
        let bytes = bytemuck::bytes_of(self);
        crc32c(&bytes[..60])
    }

    #[inline]
    pub fn seal_crc(&mut self) {
        self.reserved = 0;
        self.record_crc32c = self.compute_crc32c();
    }

    #[inline]
    pub fn with_sealed_crc(mut self) -> Self {
        self.seal_crc();
        self
    }

    #[inline]
    pub fn verify_crc(&self) -> bool {
        self.record_crc32c == self.compute_crc32c()
    }

    #[inline]
    pub const fn solver_regime(&self) -> Result<KinematicSolverRegime, SkeletalSectionError> {
        KinematicSolverRegime::from_u8(self.solver_regime)
    }

    #[inline]
    pub const fn epistemic_tier_enum(&self) -> Result<EpistemicTier, SkeletalSectionError> {
        match self.epistemic_tier {
            0 => Ok(EpistemicTier::DirectlyObserved),
            1 => Ok(EpistemicTier::InferredFromObservation),
            2 => Ok(EpistemicTier::ExtractedFromRecord),
            3 => Ok(EpistemicTier::LongitudinallyProjected),
            4 => Ok(EpistemicTier::StatisticallyCalibrated),
            5 => Ok(EpistemicTier::PhylogeneticallyBracketed),
            6 => Ok(EpistemicTier::AuthorialIntent),
            _ => Err(SkeletalSectionError::InvalidData),
        }
    }

    /// Losslessly upgrade a v1 32-byte joint descriptor into a sealed v2 64-byte descriptor.
    pub fn from_v1(v1: Skl2JointDescriptorV1) -> Result<Self, SkeletalSectionError> {
        let _ = KinematicSolverRegime::from_u8(v1.solver_regime)?;
        v1.swing_twist.validate()?;
        let out = Self {
            joint_id: v1.joint_id,
            parent_index: v1.parent_index,
            loop_closure_partner: v1.loop_closure_partner,
            landmark_index: SKL2_NONE_INDEX,
            actuation_pathway_index: SKL2_NONE_INDEX,
            swing_twist: v1.swing_twist,
            cortical_bmd_mg_cm3: 0.0,
            trabecular_bv_tv: 0.0,
            bone_length_m: 0.0,
            segment_mass_kg: 0.0,
            solver_regime: v1.solver_regime,
            epistemic_tier: EpistemicTier::AuthorialIntent.rank(),
            observation_method: 0,
            confidence_q8: 0,
            flags: v1.flags,
            reserved: 0,
            provenance_assertion_idx: 0,
            record_crc32c: 0,
        };
        Ok(out.with_sealed_crc())
    }

    /// Down-project to a v1 32-byte joint descriptor (retaining core kinematic identity and limits).
    #[inline]
    pub const fn to_v1(&self) -> Skl2JointDescriptorV1 {
        Skl2JointDescriptorV1 {
            joint_id: self.joint_id,
            parent_index: self.parent_index,
            loop_closure_partner: self.loop_closure_partner,
            solver_regime: self.solver_regime,
            flags: self.flags,
            reserved: 0,
            swing_twist: self.swing_twist,
        }
    }

    pub fn validate(&self, joint_index: usize, joint_count: usize) -> Result<(), SkeletalSectionError> {
        if self.reserved != 0 || !self.verify_crc() {
            return Err(SkeletalSectionError::InvalidData);
        }
        let _ = self.solver_regime()?;
        let _ = self.epistemic_tier_enum()?;
        self.swing_twist.validate()?;
        if !self.cortical_bmd_mg_cm3.is_finite()
            || self.cortical_bmd_mg_cm3 < 0.0
            || !self.trabecular_bv_tv.is_finite()
            || !(0.0..=1.0).contains(&self.trabecular_bv_tv)
            || !self.bone_length_m.is_finite()
            || self.bone_length_m < 0.0
            || !self.segment_mass_kg.is_finite()
            || self.segment_mass_kg < 0.0
        {
            return Err(SkeletalSectionError::InvalidData);
        }
        if self.parent_index != SKL2_NONE_INDEX && usize::from(self.parent_index) >= joint_index {
            return Err(SkeletalSectionError::InvalidData);
        }
        if self.loop_closure_partner != SKL2_NONE_INDEX
            && usize::from(self.loop_closure_partner) >= joint_count
        {
            return Err(SkeletalSectionError::InvalidData);
        }
        Ok(())
    }
}

/// Closed-chain loop closure constraint (e.g. radius-ulna, pelvis-sacrum, arthropod linkage) (64B).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct LoopClosureConstraintRecord {
    pub constraint_id: u64,
    pub joint_a: u16,
    pub joint_b: u16,
    pub closure_type: u8,
    pub flags: u8,
    pub reserved: u16,
    pub anchor_a_local: [f32; 3],
    pub anchor_b_local: [f32; 3],
    pub stiffness_n_m: f32,
    pub damping_n_s_m: f32,
    pub rest_angle_rad: f32,
    pub tolerance_m: f32,
    pub provenance: EpistemicProvenanceWord,
    pub record_crc32c: u32,
}

const _: () = assert!(core::mem::size_of::<LoopClosureConstraintRecord>() == 64);

impl LoopClosureConstraintRecord {
    #[inline]
    pub fn seal(&mut self) {
        self.reserved = 0;
        let bytes = bytemuck::bytes_of(self);
        self.record_crc32c = crc32c(&bytes[..60]);
    }

    #[inline]
    pub fn with_sealed_crc(mut self) -> Self {
        self.seal();
        self
    }

    #[inline]
    pub fn verify_crc(&self) -> bool {
        let bytes = bytemuck::bytes_of(self);
        self.reserved == 0 && self.record_crc32c == crc32c(&bytes[..60])
    }
}

/// Topological surface landmark pinned to mesh face barycentric coordinates (32B).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct TopologicalLandmarkRecord {
    pub landmark_id: u64,
    pub face_index: u32,
    pub bary_u_q16: u16,
    pub bary_v_q16: u16,
    pub attached_joint: u16,
    pub landmark_kind: u8,
    pub flags: u8,
    pub normal_offset_m: f32,
    pub provenance: EpistemicProvenanceWord,
    pub record_crc32c: u32,
}

const _: () = assert!(core::mem::size_of::<TopologicalLandmarkRecord>() == 32);

impl TopologicalLandmarkRecord {
    #[inline]
    pub fn seal(&mut self) {
        let bytes = bytemuck::bytes_of(self);
        self.record_crc32c = crc32c(&bytes[..28]);
    }

    #[inline]
    pub fn with_sealed_crc(mut self) -> Self {
        self.seal();
        self
    }

    #[inline]
    pub fn verify_crc(&self) -> bool {
        let bytes = bytemuck::bytes_of(self);
        self.record_crc32c == crc32c(&bytes[..28])
    }
}

/// Musculotendon / hydraulic / cuticle actuation pathway descriptor (48B).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct ActuationPathwayRecord {
    pub pathway_id: u64,
    pub origin_joint: u16,
    pub insertion_joint: u16,
    pub pathway_kind: u8,
    pub solver_regime: u8,
    pub reserved: u16,
    pub max_force_n: f32,
    pub optimal_fiber_length_m: f32,
    pub tendon_slack_length_m: f32,
    pub pennation_angle_rad: f32,
    pub moment_arm_m: f32,
    pub activation_tau_s: f32,
    pub provenance: EpistemicProvenanceWord,
    pub record_crc32c: u32,
}

const _: () = assert!(core::mem::size_of::<ActuationPathwayRecord>() == 48);

impl ActuationPathwayRecord {
    #[inline]
    pub fn seal(&mut self) {
        self.reserved = 0;
        let bytes = bytemuck::bytes_of(self);
        self.record_crc32c = crc32c(&bytes[..44]);
    }

    #[inline]
    pub fn with_sealed_crc(mut self) -> Self {
        self.seal();
        self
    }

    #[inline]
    pub fn verify_crc(&self) -> bool {
        let bytes = bytemuck::bytes_of(self);
        self.reserved == 0 && self.record_crc32c == crc32c(&bytes[..44])
    }
}

/// Statistical Shape Model (`SSM1`) basis header (32B).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Ssm1BasisHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub mode_count: u16,
    pub target_vertex_count: u32,
    pub target_joint_count: u16,
    pub flags: u16,
    pub species_taxon_hash: u64,
    pub provenance: EpistemicProvenanceWord,
    pub record_crc32c: u32,
}

const _: () = assert!(core::mem::size_of::<Ssm1BasisHeader>() == 32);

impl Ssm1BasisHeader {
    #[inline]
    pub fn seal(&mut self) {
        self.magic = SSM1_MAGIC;
        let bytes = bytemuck::bytes_of(self);
        self.record_crc32c = crc32c(&bytes[..28]);
    }

    #[inline]
    pub fn with_sealed_crc(mut self) -> Self {
        self.seal();
        self
    }

    #[inline]
    pub fn verify_crc(&self) -> bool {
        let bytes = bytemuck::bytes_of(self);
        self.magic == SSM1_MAGIC && self.record_crc32c == crc32c(&bytes[..28])
    }
}

/// Single Statistical Shape Model principal component mode descriptor (16B).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Ssm1ModeWeight {
    pub mode_id: u32,
    pub eigenvalue: f32,
    pub min_sigma: f32,
    pub max_sigma: f32,
}

const _: () = assert!(core::mem::size_of::<Ssm1ModeWeight>() == 16);

/// Decoded SKL2 biological skeletal payload supporting lossless v1 (32B) and v2 (64B) joints.
#[derive(Debug, Clone, PartialEq)]
pub struct SkeletalSectionPayload {
    pub source_version: u16,
    pub flags: u16,
    pub joints: Vec<Skl2JointDescriptorV2>,
    pub loop_closures: Vec<LoopClosureConstraintRecord>,
    pub landmarks: Vec<TopologicalLandmarkRecord>,
    pub actuation_pathways: Vec<ActuationPathwayRecord>,
    pub ssm_headers: Vec<Ssm1BasisHeader>,
    pub ssm_modes: Vec<Ssm1ModeWeight>,
}

impl SkeletalSectionPayload {
    pub fn new(joints: Vec<Skl2JointDescriptorV2>) -> Self {
        Self {
            source_version: SKL2_VERSION_2,
            flags: 0,
            joints,
            loop_closures: Vec::new(),
            landmarks: Vec::new(),
            actuation_pathways: Vec::new(),
            ssm_headers: Vec::new(),
            ssm_modes: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), SkeletalSectionError> {
        if self.joints.len() > SKL2_MAX_JOINTS {
            return Err(SkeletalSectionError::CountLimit);
        }
        let jc = self.joints.len();
        for (idx, j) in self.joints.iter().enumerate() {
            j.validate(idx, jc)?;
            if j.landmark_index != SKL2_NONE_INDEX
                && usize::from(j.landmark_index) >= self.landmarks.len()
            {
                return Err(SkeletalSectionError::InvalidData);
            }
            if j.actuation_pathway_index != SKL2_NONE_INDEX
                && usize::from(j.actuation_pathway_index) >= self.actuation_pathways.len()
            {
                return Err(SkeletalSectionError::InvalidData);
            }
        }
        for lc in &self.loop_closures {
            if !lc.verify_crc()
                || usize::from(lc.joint_a) >= jc
                || usize::from(lc.joint_b) >= jc
                || lc.joint_a == lc.joint_b
            {
                return Err(SkeletalSectionError::InvalidData);
            }
        }
        for lm in &self.landmarks {
            if !lm.verify_crc()
                || (lm.attached_joint != SKL2_NONE_INDEX && usize::from(lm.attached_joint) >= jc)
            {
                return Err(SkeletalSectionError::InvalidData);
            }
        }
        for ap in &self.actuation_pathways {
            if !ap.verify_crc()
                || usize::from(ap.origin_joint) >= jc
                || usize::from(ap.insertion_joint) >= jc
                || KinematicSolverRegime::from_u8(ap.solver_regime).is_err()
            {
                return Err(SkeletalSectionError::InvalidData);
            }
        }
        for sh in &self.ssm_headers {
            if !sh.verify_crc() {
                return Err(SkeletalSectionError::InvalidData);
            }
        }
        if self.encoded_v2_size() > SKL2_MAX_CONTAINER_BYTES {
            return Err(SkeletalSectionError::CountLimit);
        }
        Ok(())
    }

    #[inline]
    pub fn encoded_v2_size(&self) -> usize {
        self.encoded_body_size(SKL2_VERSION_2)
    }

    #[inline]
    pub fn encoded_v1_size(&self) -> usize {
        self.encoded_body_size(SKL2_VERSION_1)
    }

    fn encoded_body_size(&self, version: u16) -> usize {
        let joint_stride = if version == SKL2_VERSION_1 { 32 } else { 64 };
        SKL2_HEADER_LEN
            + self.joints.len() * joint_stride
            + self.loop_closures.len() * core::mem::size_of::<LoopClosureConstraintRecord>()
            + self.landmarks.len() * core::mem::size_of::<TopologicalLandmarkRecord>()
            + self.actuation_pathways.len() * core::mem::size_of::<ActuationPathwayRecord>()
            + self.ssm_headers.len() * core::mem::size_of::<Ssm1BasisHeader>()
            + self.ssm_modes.len() * core::mem::size_of::<Ssm1ModeWeight>()
    }

    /// Encode this payload into a caller-provided slice in SKL2 v2 (64-byte joint descriptors).
    pub fn encode_v2_into(&self, out: &mut [u8]) -> Result<usize, SkeletalSectionError> {
        self.encode_into_version(SKL2_VERSION_2, out)
    }

    /// Encode this payload into a caller-provided slice in SKL2 v1 (32-byte joint descriptors).
    pub fn encode_v1_into(&self, out: &mut [u8]) -> Result<usize, SkeletalSectionError> {
        self.encode_into_version(SKL2_VERSION_1, out)
    }

    /// Encode this payload to an owned `Vec<u8>` in SKL2 v2 format.
    pub fn to_bytes_v2(&self) -> Result<Vec<u8>, SkeletalSectionError> {
        let len = self.encoded_v2_size();
        let mut buf = vec![0u8; len];
        let written = self.encode_v2_into(&mut buf)?;
        buf.truncate(written);
        Ok(buf)
    }

    /// Encode this payload to an owned `Vec<u8>` in SKL2 v1 format.
    pub fn to_bytes_v1(&self) -> Result<Vec<u8>, SkeletalSectionError> {
        let len = self.encoded_v1_size();
        let mut buf = vec![0u8; len];
        let written = self.encode_v1_into(&mut buf)?;
        buf.truncate(written);
        Ok(buf)
    }

    fn encode_into_version(
        &self,
        version: u16,
        out: &mut [u8],
    ) -> Result<usize, SkeletalSectionError> {
        self.validate()?;
        let total_len = self.encoded_body_size(version);
        if total_len > SKL2_MAX_CONTAINER_BYTES {
            return Err(SkeletalSectionError::CountLimit);
        }
        if out.len() < total_len {
            return Err(SkeletalSectionError::OutputTooSmall);
        }

        let mut cursor = SKL2_HEADER_LEN;
        if version == SKL2_VERSION_1 {
            for j in &self.joints {
                let v1 = j.to_v1();
                out[cursor..cursor + 32].copy_from_slice(bytemuck::bytes_of(&v1));
                cursor += 32;
            }
        } else {
            for j in &self.joints {
                let mut sealed = *j;
                sealed.seal_crc();
                out[cursor..cursor + 64].copy_from_slice(bytemuck::bytes_of(&sealed));
                cursor += 64;
            }
        }

        for lc in &self.loop_closures {
            let mut sealed = *lc;
            sealed.seal();
            out[cursor..cursor + 64].copy_from_slice(bytemuck::bytes_of(&sealed));
            cursor += 64;
        }
        for lm in &self.landmarks {
            let mut sealed = *lm;
            sealed.seal();
            out[cursor..cursor + 32].copy_from_slice(bytemuck::bytes_of(&sealed));
            cursor += 32;
        }
        for ap in &self.actuation_pathways {
            let mut sealed = *ap;
            sealed.seal();
            out[cursor..cursor + 48].copy_from_slice(bytemuck::bytes_of(&sealed));
            cursor += 48;
        }
        for sh in &self.ssm_headers {
            let mut sealed = *sh;
            sealed.seal();
            out[cursor..cursor + 32].copy_from_slice(bytemuck::bytes_of(&sealed));
            cursor += 32;
        }
        for sm in &self.ssm_modes {
            out[cursor..cursor + 16].copy_from_slice(bytemuck::bytes_of(sm));
            cursor += 16;
        }

        let payload_crc = crc32c(&out[SKL2_HEADER_LEN..cursor]);

        out[0..4].copy_from_slice(&SKL2_MAGIC);
        out[4..6].copy_from_slice(&version.to_le_bytes());
        out[6..8].copy_from_slice(&self.flags.to_le_bytes());
        out[8..10].copy_from_slice(&(self.joints.len() as u16).to_le_bytes());
        out[10..12].copy_from_slice(&(self.loop_closures.len() as u16).to_le_bytes());
        out[12..14].copy_from_slice(&(self.landmarks.len() as u16).to_le_bytes());
        out[14..16].copy_from_slice(&(self.actuation_pathways.len() as u16).to_le_bytes());
        out[16..18].copy_from_slice(&(self.ssm_headers.len() as u16).to_le_bytes());
        out[18..20].copy_from_slice(&(self.ssm_modes.len() as u16).to_le_bytes());
        out[20..24].copy_from_slice(&0u32.to_le_bytes());
        out[24..28].copy_from_slice(&payload_crc.to_le_bytes());
        let header_crc = crc32c(&out[0..28]);
        out[28..32].copy_from_slice(&header_crc.to_le_bytes());

        Ok(cursor)
    }

    /// Losslessly decode SKL2 v1 (32B joint descriptors) or SKL2 v2 (64B joint descriptors).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SkeletalSectionError> {
        if bytes.len() > SKL2_MAX_CONTAINER_BYTES {
            return Err(SkeletalSectionError::CountLimit);
        }
        if bytes.len() < SKL2_HEADER_LEN || bytes[0..4] != SKL2_MAGIC {
            return Err(SkeletalSectionError::Header);
        }
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version != SKL2_VERSION_1 && version != SKL2_VERSION_2 {
            return Err(SkeletalSectionError::Header);
        }
        let expected_header_crc = u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]);
        if crc32c(&bytes[0..28]) != expected_header_crc {
            return Err(SkeletalSectionError::Header);
        }
        let reserved = u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        if reserved != 0 {
            return Err(SkeletalSectionError::Header);
        }

        let flags = u16::from_le_bytes([bytes[6], bytes[7]]);
        let joint_count = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
        let loop_closure_count = u16::from_le_bytes([bytes[10], bytes[11]]) as usize;
        let landmark_count = u16::from_le_bytes([bytes[12], bytes[13]]) as usize;
        let actuation_pathway_count = u16::from_le_bytes([bytes[14], bytes[15]]) as usize;
        let ssm_header_count = u16::from_le_bytes([bytes[16], bytes[17]]) as usize;
        let ssm_mode_count = u16::from_le_bytes([bytes[18], bytes[19]]) as usize;
        let expected_payload_crc = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);

        if joint_count > SKL2_MAX_JOINTS {
            return Err(SkeletalSectionError::CountLimit);
        }

        let joint_stride = if version == SKL2_VERSION_1 { 32 } else { 64 };
        let expected_len = SKL2_HEADER_LEN
            .checked_add(joint_count * joint_stride)
            .and_then(|v| v.checked_add(loop_closure_count * 64))
            .and_then(|v| v.checked_add(landmark_count * 32))
            .and_then(|v| v.checked_add(actuation_pathway_count * 48))
            .and_then(|v| v.checked_add(ssm_header_count * 32))
            .and_then(|v| v.checked_add(ssm_mode_count * 16))
            .ok_or(SkeletalSectionError::Length)?;

        if bytes.len() != expected_len {
            return Err(SkeletalSectionError::Length);
        }
        if crc32c(&bytes[SKL2_HEADER_LEN..]) != expected_payload_crc {
            return Err(SkeletalSectionError::InvalidData);
        }

        let mut cursor = SKL2_HEADER_LEN;
        let mut joints = Vec::with_capacity(joint_count);
        if version == SKL2_VERSION_1 {
            for _ in 0..joint_count {
                let mut v1 = Skl2JointDescriptorV1::zeroed();
                bytemuck::bytes_of_mut(&mut v1).copy_from_slice(&bytes[cursor..cursor + 32]);
                cursor += 32;
                if v1.reserved != 0 {
                    return Err(SkeletalSectionError::InvalidData);
                }
                joints.push(Skl2JointDescriptorV2::from_v1(v1)?);
            }
        } else {
            for _ in 0..joint_count {
                let mut v2 = Skl2JointDescriptorV2::zeroed();
                bytemuck::bytes_of_mut(&mut v2).copy_from_slice(&bytes[cursor..cursor + 64]);
                cursor += 64;
                joints.push(v2);
            }
        }

        let mut loop_closures = Vec::with_capacity(loop_closure_count);
        for _ in 0..loop_closure_count {
            let mut lc = LoopClosureConstraintRecord::zeroed();
            bytemuck::bytes_of_mut(&mut lc).copy_from_slice(&bytes[cursor..cursor + 64]);
            cursor += 64;
            loop_closures.push(lc);
        }

        let mut landmarks = Vec::with_capacity(landmark_count);
        for _ in 0..landmark_count {
            let mut lm = TopologicalLandmarkRecord::zeroed();
            bytemuck::bytes_of_mut(&mut lm).copy_from_slice(&bytes[cursor..cursor + 32]);
            cursor += 32;
            landmarks.push(lm);
        }

        let mut actuation_pathways = Vec::with_capacity(actuation_pathway_count);
        for _ in 0..actuation_pathway_count {
            let mut ap = ActuationPathwayRecord::zeroed();
            bytemuck::bytes_of_mut(&mut ap).copy_from_slice(&bytes[cursor..cursor + 48]);
            cursor += 48;
            actuation_pathways.push(ap);
        }

        let mut ssm_headers = Vec::with_capacity(ssm_header_count);
        for _ in 0..ssm_header_count {
            let mut sh = Ssm1BasisHeader::zeroed();
            bytemuck::bytes_of_mut(&mut sh).copy_from_slice(&bytes[cursor..cursor + 32]);
            cursor += 32;
            ssm_headers.push(sh);
        }

        let mut ssm_modes = Vec::with_capacity(ssm_mode_count);
        for _ in 0..ssm_mode_count {
            let mut sm = Ssm1ModeWeight::zeroed();
            bytemuck::bytes_of_mut(&mut sm).copy_from_slice(&bytes[cursor..cursor + 16]);
            cursor += 16;
            ssm_modes.push(sm);
        }

        let payload = Self {
            source_version: version,
            flags,
            joints,
            loop_closures,
            landmarks,
            actuation_pathways,
            ssm_headers,
            ssm_modes,
        };
        payload.validate()?;
        Ok(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_v2_payload() -> SkeletalSectionPayload {
        let j0 = Skl2JointDescriptorV2::new(
            0x1000,
            SKL2_NONE_INDEX,
            SwingTwistConeLimits::symmetric(0.35, 0.2),
            KinematicSolverRegime::ArticulatedRigidBodyDca,
            1150.0,
            0.32,
            0.25,
            12.5,
            EpistemicTier::DirectlyObserved.rank(),
            ObservationMethod::MicroCtSynchrotron as u16,
            245,
        );
        let mut j1 = Skl2JointDescriptorV2::new(
            0x1001,
            0,
            SwingTwistConeLimits::new(0.8, 0.6, -0.4, 0.4),
            KinematicSolverRegime::CosseratRodContinuum,
            980.0,
            0.28,
            0.40,
            4.2,
            EpistemicTier::InferredFromObservation.rank(),
            ObservationMethod::OpticalMarkerMoCap as u16,
            210,
        );
        j1.landmark_index = 0;
        j1.actuation_pathway_index = 0;
        j1.seal_crc();

        let lc = LoopClosureConstraintRecord {
            constraint_id: 0x2000,
            joint_a: 0,
            joint_b: 1,
            closure_type: 1,
            flags: 0,
            reserved: 0,
            anchor_a_local: [0.0, 0.1, 0.0],
            anchor_b_local: [0.0, -0.1, 0.0],
            stiffness_n_m: 1500.0,
            damping_n_s_m: 45.0,
            rest_angle_rad: 0.0,
            tolerance_m: 0.002,
            provenance: EpistemicProvenanceWord::pack(0, 1, 0, 0, 1000, 240),
            record_crc32c: 0,
        }
        .with_sealed_crc();

        let lm = TopologicalLandmarkRecord {
            landmark_id: 0x3000,
            face_index: 42,
            bary_u_q16: 21845,
            bary_v_q16: 21845,
            attached_joint: 1,
            landmark_kind: 2,
            flags: 0,
            normal_offset_m: 0.004,
            provenance: EpistemicProvenanceWord::pack(1, 2, 1, 0, 900, 220),
            record_crc32c: 0,
        }
        .with_sealed_crc();

        let ap = ActuationPathwayRecord {
            pathway_id: 0x4000,
            origin_joint: 0,
            insertion_joint: 1,
            pathway_kind: 1,
            solver_regime: KinematicSolverRegime::ArticulatedRigidBodyDca.as_u8(),
            reserved: 0,
            max_force_n: 850.0,
            optimal_fiber_length_m: 0.18,
            tendon_slack_length_m: 0.12,
            pennation_angle_rad: 0.22,
            moment_arm_m: 0.035,
            activation_tau_s: 0.015,
            provenance: EpistemicProvenanceWord::pack(2, 1, 1, 2, 750, 190),
            record_crc32c: 0,
        }
        .with_sealed_crc();

        let sh = Ssm1BasisHeader {
            magic: SSM1_MAGIC,
            version: 1,
            mode_count: 1,
            target_vertex_count: 4096,
            target_joint_count: 24,
            flags: 0,
            species_taxon_hash: 0xCAFE_BABE_1234_5678,
            provenance: EpistemicProvenanceWord::pack(4, 3, 4, 0, 1023, 230),
            record_crc32c: 0,
        }
        .with_sealed_crc();

        let sm = Ssm1ModeWeight {
            mode_id: 0,
            eigenvalue: 0.42,
            min_sigma: -3.0,
            max_sigma: 3.0,
        };

        SkeletalSectionPayload {
            source_version: SKL2_VERSION_2,
            flags: 0,
            joints: vec![j0, j1],
            loop_closures: vec![lc],
            landmarks: vec![lm],
            actuation_pathways: vec![ap],
            ssm_headers: vec![sh],
            ssm_modes: vec![sm],
        }
    }

    #[test]
    fn skl2_v2_descriptor_layout_and_roundtrip() {
        assert_eq!(core::mem::size_of::<Skl2JointDescriptorV1>(), 32);
        assert_eq!(core::mem::size_of::<Skl2JointDescriptorV2>(), 64);
        assert_eq!(core::mem::align_of::<Skl2JointDescriptorV2>(), 64);

        let payload = sample_v2_payload();
        let bytes = payload.to_bytes_v2().expect("v2 encode");
        assert_eq!(&bytes[0..4], b"SKL2");

        let decoded = SkeletalSectionPayload::from_bytes(&bytes).expect("v2 decode");
        assert_eq!(decoded.source_version, SKL2_VERSION_2);
        assert_eq!(decoded.joints, payload.joints);
        assert_eq!(decoded.loop_closures, payload.loop_closures);
        assert_eq!(decoded.landmarks, payload.landmarks);
        assert_eq!(decoded.actuation_pathways, payload.actuation_pathways);
        assert_eq!(decoded.ssm_headers, payload.ssm_headers);
        assert_eq!(decoded.ssm_modes, payload.ssm_modes);
    }

    #[test]
    fn skl2_v1_lossless_upgrade_decode() {
        let mut payload = sample_v2_payload();
        payload.joints[1].landmark_index = SKL2_NONE_INDEX;
        payload.joints[1].actuation_pathway_index = SKL2_NONE_INDEX;
        payload.loop_closures.clear();
        payload.landmarks.clear();
        payload.actuation_pathways.clear();
        payload.ssm_headers.clear();
        payload.ssm_modes.clear();

        let v1_bytes = payload.to_bytes_v1().expect("v1 encode");
        assert_eq!(
            v1_bytes.len(),
            SKL2_HEADER_LEN + payload.joints.len() * 32
        );

        let decoded = SkeletalSectionPayload::from_bytes(&v1_bytes).expect("v1 decode");
        assert_eq!(decoded.source_version, SKL2_VERSION_1);
        assert_eq!(decoded.joints.len(), 2);
        assert_eq!(decoded.joints[0].joint_id, 0x1000);
        assert_eq!(decoded.joints[1].joint_id, 0x1001);
        assert_eq!(
            decoded.joints[1].solver_regime().unwrap(),
            KinematicSolverRegime::CosseratRodContinuum
        );
        assert_eq!(decoded.joints[1].swing_twist, payload.joints[1].swing_twist);
        assert!(decoded.joints[0].verify_crc());
        assert!(decoded.joints[1].verify_crc());
    }

    #[test]
    fn skl2_corrupted_crc_fails_closed() {
        let payload = sample_v2_payload();
        let mut bytes = payload.to_bytes_v2().expect("v2 encode");
        // Corrupt a byte in the first 64B joint descriptor.
        bytes[SKL2_HEADER_LEN + 8] ^= 0xFF;
        assert_eq!(
            SkeletalSectionPayload::from_bytes(&bytes),
            Err(SkeletalSectionError::InvalidData)
        );
    }

    #[test]
    fn epistemic_provenance_word_bit_pack_roundtrip() {
        let word = EpistemicProvenanceWord::from_contextual(
            EpistemicTier::PhylogeneticallyBracketed,
            ObservationMethod::PublishedScientificLiterature,
            ObserverRole::PeerMeasured,
            TemporalStabilityClass::OntogeneticAndSkeletalRemodeling,
            777,
            213,
        );
        assert_eq!(word.status(), EpistemicTier::PhylogeneticallyBracketed.rank());
        assert_eq!(
            word.method_class(),
            ObservationMethod::PublishedScientificLiterature.category_code()
        );
        assert_eq!(word.observer_role(), 1);
        assert_eq!(word.stability_class(), 2);
        assert_eq!(word.freshness_q10(), 777);
        assert_eq!(word.confidence_q8(), 213);
    }

    #[test]
    fn zero_heap_encode_into_caller_buffer() {
        let payload = sample_v2_payload();
        let needed = payload.encoded_v2_size();
        let mut short = vec![0u8; needed - 1];
        assert_eq!(
            payload.encode_v2_into(&mut short),
            Err(SkeletalSectionError::OutputTooSmall)
        );
        let mut exact = vec![0u8; needed];
        let written = payload.encode_v2_into(&mut exact).expect("encode into exact");
        assert_eq!(written, needed);
        let decoded = SkeletalSectionPayload::from_bytes(&exact[..written]).expect("decode");
        assert_eq!(decoded.joints, payload.joints);
    }
}
