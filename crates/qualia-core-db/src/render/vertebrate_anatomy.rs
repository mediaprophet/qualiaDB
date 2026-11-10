//! Comparative Vertebrate Anatomy & 3-Layer Biomechanical Living Containers (`zoological_anatomy.rs`).
//!
//! Implements **Epic 2** (Scale 5 Comparative Vertebrate Anatomy) of the Biological Education Assets
//! and Explorable World master plan (`docs/plans/biological-education-assets-and-explorable-world-plan.md`).
//!
//! Covers all 13 Australian fauna biomechanical archetypes across 3 inspectable anatomical layers:
//! - **Layer 0 — Articulated Osteology** (8 regions including diprotodont rostrum, hypsodont dentition,
//!   keeled carina sternum, furcula, and characteristic marsupial/monotreme **epipubic bones**!).
//! - **Layer 1 — Major Myology & Tendons** (8 functional groups including the elastic
//!   **Achilles tendon spring** energy-recovery store, massive flight pectorals, and fossorial deltoids).
//! - **Layer 2 — External Integument** (high-resolution outer skin, pelage, plumage, or squamation).
//!
//! Generates >= 1,800 vertex high-resolution meshes bound to `FaunaSkeletalRig10d` (18 PGA joints),
//! seals them into `.10d` Living Containers with `MAT4`, `UV01`, `FNRM`, `SRD1`, `PROC1`, `EONT1`,
//! `MRP1` (costal respiration, gular vocalization, Achilles tendon strain), and `PROV`.
//! Also provides zero-heap layer peeling and exploded-view cross-fading.

use bytemuck;
use serde::{Deserialize, Serialize};

use crate::container_10d::entity_ontology_section::{
    AffordanceBinaryRecord, DevelopmentalKeyframeRecord, EntityOntologyMiniHeader,
    AFFORDANCE_RECORD_SIZE, DEVELOPMENTAL_KEYFRAME_RECORD_SIZE, ENTITY_ONTOLOGY_MINI_HEADER_SIZE,
    ENTITY_ONTOLOGY_SECTION_VERSION, MAGIC_EONT,
};
use crate::container_10d::header::{Container10dHeader, MAX_SECTION_COUNT};
use crate::container_10d::integrity::seal_whole_file_crc32c;
use crate::container_10d::material_container::attach_material_section;
use crate::container_10d::material_section::{
    MaterialRecord, OpacityMode, ShadingModel, SubmeshRange,
};
use crate::container_10d::morph_section::{
    encode_morph_section, morph_section_encoded_len, quantize_morph_delta, MorphSectionHeader,
    QuantizedMorphDelta, FLAG_MORPH_HAS_NORMALS, FLAG_MORPH_ONTOGENETIC_BIND, FLAG_MORPH_SPARSE,
    MAGIC_MRP1, MORPH_SECTION_VERSION,
};
use crate::container_10d::procedural_synthesis::{
    encode_procedural_synthesis_section, ProceduralSynthesisHeader, FLAG_ALLOMETRIC_GROWTH,
    FLAG_CELLULAR_VORONOI, FLAG_IMPLICIT_FILLET, FLAG_PGA_MOTOR_DYNAMICS, MAGIC_PROC,
    PROCEDURAL_SYNTHESIS_HEADER_SIZE, PROCEDURAL_SYNTHESIS_SECTION_VERSION,
};
use crate::container_10d::provenance_section::ProvenanceSidecar;
use crate::container_10d::section::{
    encode_container, parse_section_table, AlignmentTier, SectionInput, SectionTableError,
    SectionType,
};
use crate::container_10d::texture_coordinate_container::attach_texture_coordinate_section;
use crate::q_hash;
use crate::render::assets::Mesh;
use crate::render::compile_10d::{compile_mesh_to_10d_with_extras_and_frames, Compile10dExtras};

use crate::render::skeletal::fauna_rig::{
    FaunaBiomechanicalArchetype, FaunaSkeletalRig10d, JOINT_CHEST, JOINT_HEAD, JOINT_JAW,
    JOINT_LEFT_ANKLE, JOINT_LEFT_ELBOW, JOINT_LEFT_HIP, JOINT_LEFT_KNEE, JOINT_LEFT_SHOULDER,
    JOINT_LEFT_WRIST_WING, JOINT_NECK, JOINT_PELVIS_ROOT, JOINT_RIGHT_ANKLE, JOINT_RIGHT_HIP,
    JOINT_RIGHT_KNEE, JOINT_SPINE, JOINT_TAIL_BASE, JOINT_TAIL_MID, JOINT_TAIL_TIP,
};

// ── Anatomical Layers ────────────────────────────────────────────────────────

/// 3 Primary Comparative Anatomical Layers for Scale 5 vertebrate dissection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZoologicalAnatomicalLayer {
    /// Layer 0: Articulated skeleton.
    Osteology,
    /// Layer 1: Functional muscle groups & elastic energy-recovery tendons.
    MyologyAndTendons,
    /// Layer 2: Pelage, plumage, squamation, or amphibian skin.
    ExternalIntegument,
}

// ── Osteology Regions ────────────────────────────────────────────────────────

/// 8 Comparative Vertebrate Osteology Regions (Layer 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VertebrateOsteologyRegion {
    /// Cranium & Rostrum (diprotodont diastema / monotreme electrosensory bill / raptor beak / lizard diapsid).
    CraniumAndRostrum,
    /// Mandible & Dentition (procumbent lower incisors / hypsodont molars / toothless keratinous pads).
    MandibleAndDentition,
    /// Cervical, Thoracic & Lumbar vertebral column (axial spine support).
    CervicalThoracicLumbarVertebrae,
    /// Ribcage & Sternum (including avian keeled carina & furcula wishbone).
    RibcageAndSternum,
    /// Pectoral Girdle & Forelimb (scapula, humerus, radius, ulna, carpals, pentadactyl digits / fossorial claws).
    PectoralGirdleAndForelimb,
    /// Pelvic Girdle & Epipubic Bones (ilium, ischium, pubis + characteristic marsupial/monotreme epipubic bones!).
    PelvicGirdleAndEpipubicBones,
    /// Hindlimb & Syndactyl Pes (femur, elongated tibia/fibula, calcaneum, elongated 4th metatarsal, syndactyl grooming digits II & III).
    HindlimbAndSyndactylPes,
    /// Caudal Vertebral Chain (muscular counterbalance/pentapedal fifth-limb tail in macropods; flattened rudder tail in platypus).
    CaudalVertebralChain,
}

impl VertebrateOsteologyRegion {
    pub const ALL: [Self; 8] = [
        Self::CraniumAndRostrum,
        Self::MandibleAndDentition,
        Self::CervicalThoracicLumbarVertebrae,
        Self::RibcageAndSternum,
        Self::PectoralGirdleAndForelimb,
        Self::PelvicGirdleAndEpipubicBones,
        Self::HindlimbAndSyndactylPes,
        Self::CaudalVertebralChain,
    ];

    pub const fn name(&self) -> &'static str {
        match self {
            Self::CraniumAndRostrum => "Cranium & Rostrum",
            Self::MandibleAndDentition => "Mandible & Dentition",
            Self::CervicalThoracicLumbarVertebrae => "Cervical, Thoracic & Lumbar Vertebrae",
            Self::RibcageAndSternum => "Ribcage & Sternum (Carina / Furcula)",
            Self::PectoralGirdleAndForelimb => "Pectoral Girdle & Forelimb",
            Self::PelvicGirdleAndEpipubicBones => "Pelvic Girdle & Epipubic Bones",
            Self::HindlimbAndSyndactylPes => "Hindlimb & Syndactyl Pes",
            Self::CaudalVertebralChain => "Caudal Vertebral Chain",
        }
    }
}

// ── Myology Groups ───────────────────────────────────────────────────────────

/// 8 Comparative Vertebrate Functional Myology Groups & Tendon Springs (Layer 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VertebrateMyologyGroup {
    /// Gastrocnemius & Plantaris (power calf propulsion).
    GastrocnemiusAndPlantaris,
    /// Elastic Achilles Tendon Spring (stores and returns up to 65% metabolic work per hop/stride).
    ElasticAchillesTendonSpring,
    /// Gluteus & Biceps Femoris (massive thigh hip-extensor mass).
    GluteusAndBicepsFemoris,
    /// Iliopsoas & Caudofemoralis (deep hip flexor & tail-to-femur retractor).
    IliopsoasAndCaudofemoralis,
    /// Pectoralis Major & Supracoracoideus (massive chest flight/forelimb adductor).
    PectoralisMajorAndSupracoracoideus,
    /// Deltoid & Triceps Fossorial (powerful digging/climbing shoulder abductors).
    DeltoidAndTricepsFossorial,
    /// Masseter & Temporalis (jaw mastication muscles powering diprotodont shearing).
    MasseterAndTemporalis,
    /// Intercostals & Diaphragm (thoracic respiratory expansion & costal ventilation).
    IntercostalsAndDiaphragm,
}

impl VertebrateMyologyGroup {
    pub const ALL: [Self; 8] = [
        Self::GastrocnemiusAndPlantaris,
        Self::ElasticAchillesTendonSpring,
        Self::GluteusAndBicepsFemoris,
        Self::IliopsoasAndCaudofemoralis,
        Self::PectoralisMajorAndSupracoracoideus,
        Self::DeltoidAndTricepsFossorial,
        Self::MasseterAndTemporalis,
        Self::IntercostalsAndDiaphragm,
    ];

    pub const fn name(&self) -> &'static str {
        match self {
            Self::GastrocnemiusAndPlantaris => "Gastrocnemius & Plantaris",
            Self::ElasticAchillesTendonSpring => "Elastic Achilles Tendon Spring",
            Self::GluteusAndBicepsFemoris => "Gluteus & Biceps Femoris",
            Self::IliopsoasAndCaudofemoralis => "Iliopsoas & Caudofemoralis",
            Self::PectoralisMajorAndSupracoracoideus => "Pectoralis Major & Supracoracoideus",
            Self::DeltoidAndTricepsFossorial => "Deltoid & Triceps Fossorial",
            Self::MasseterAndTemporalis => "Masseter & Temporalis",
            Self::IntercostalsAndDiaphragm => "Intercostals & Diaphragm",
        }
    }
}

// ── Compiled Vertebrate Anatomy Container ────────────────────────────────────

/// Complete 3-Layer Comparative Anatomical Twin sealed into a `.10d` Living Container.
#[derive(Clone, Debug)]
pub struct CompiledVertebrateAnatomy10d {
    /// Biomechanical archetype.
    pub archetype: FaunaBiomechanicalArchetype,
    /// 18-joint skeletal rig and kinematics definition.
    pub rig: FaunaSkeletalRig10d,
    /// Fully sealed and CRC-32C verified `.10d` living container binary.
    pub container_bytes: Vec<u8>,
    /// Total vertices across all 3 layers (>= 1,800).
    pub vertex_count: usize,
    /// Total triangles across all 3 layers.
    pub triangle_count: usize,
    /// Index range for Osteology (Layer 0).
    pub osteology_vertex_range: std::ops::Range<usize>,
    /// Index range for Myology (Layer 1).
    pub myology_vertex_range: std::ops::Range<usize>,
    /// Index range for External Integument (Layer 2).
    pub integument_vertex_range: std::ops::Range<usize>,
    /// CPU vertex positions [x, y, z] in rest pose.
    pub positions: Vec<[f32; 3]>,
    /// CPU vertex normals [nx, ny, nz].
    pub normals: Vec<[f32; 3]>,
    /// CPU RGBA8 surface readings / colors [r, g, b, a].
    pub colors: Vec<[f32; 4]>,
    /// CPU texture coordinates [u, v].
    pub uvs: Vec<[f32; 2]>,
    /// Contiguous index stream for all submeshes.
    pub indices: Vec<u32>,
    /// 17 PBR material records (8 osteology, 8 myology, 1 integument).
    pub materials: Vec<MaterialRecord>,
    /// 17 submesh index intervals.
    pub submesh_ranges: Vec<SubmeshRange>,
    /// GPU joint indices (up to 4 joints per vertex).
    pub skin_joint_indices: Vec<[u16; 4]>,
    /// GPU joint blending weights (summing to 1.0).
    pub skin_weights: Vec<[f32; 4]>,
    /// Per-vertex anatomical layer classification.
    pub layer_tags: Vec<ZoologicalAnatomicalLayer>,
}

// ── Zero-Heap Layer Peeling & Exploded View ──────────────────────────────────

/// Evaluates interactive layer peeling and exploded-view displacement between
/// `ExternalIntegument`, `MyologyAndTendons`, and `Osteology` in zero heap allocations.
///
/// Modulates alpha opacity and radially displaces muscles and integument outward along
/// surface normals, revealing deeper anatomical layers without allocating memory.
///
/// Returns the number of vertices evaluated.
#[inline]
pub fn evaluate_anatomical_layer_peel_zero_heap(
    in_positions: &[[f32; 3]],
    in_normals: &[[f32; 3]],
    in_layer_tags: &[ZoologicalAnatomicalLayer],
    osteology_vis_01: f32,
    myology_vis_01: f32,
    integument_vis_01: f32,
    explode_displacement_01: f32,
    out_positions: &mut [[f32; 3]],
    out_alphas: &mut [f32],
) -> usize {
    let count = in_positions
        .len()
        .min(in_normals.len())
        .min(in_layer_tags.len())
        .min(out_positions.len())
        .min(out_alphas.len());

    let ost_vis = osteology_vis_01.clamp(0.0, 1.0);
    let myo_vis = myology_vis_01.clamp(0.0, 1.0);
    let int_vis = integument_vis_01.clamp(0.0, 1.0);
    let explode = explode_displacement_01.clamp(0.0, 1.0);

    for i in 0..count {
        let p = in_positions[i];
        let n = in_normals[i];
        let layer = in_layer_tags[i];

        let (alpha, disp_dist) = match layer {
            ZoologicalAnatomicalLayer::Osteology => (ost_vis, 0.0),
            ZoologicalAnatomicalLayer::MyologyAndTendons => (myo_vis, explode * 0.08),
            ZoologicalAnatomicalLayer::ExternalIntegument => (int_vis, explode * 0.18),
        };

        out_positions[i] = [
            p[0] + n[0] * disp_dist,
            p[1] + n[1] * disp_dist,
            p[2] + n[2] * disp_dist,
        ];
        out_alphas[i] = alpha;
    }

    count
}

// ── Mesh Accumulator & Anatomical Loft Generators ────────────────────────────

/// Single cross-section ring station for multi-station anatomical lofting in `AnatomyMeshAccumulator`.
#[derive(Clone, Copy, Debug)]
pub struct AnatomicalLoftRing {
    /// Station 3D center `[x, y, z]`.
    pub center: [f32; 3],
    /// Lateral half-width (`rx`) and dorsoventral half-height (`ry`).
    pub radii: [f32; 2],
    /// Dorsal color (top of cross-section).
    pub dorsal_color: [f32; 4],
    /// Mid-flank color (sides of cross-section).
    pub flank_color: [f32; 4],
    /// Ventral color (underside of cross-section).
    pub ventral_color: [f32; 4],
    /// Primary skeletal joint index.
    pub primary_joint: usize,
    /// Secondary skeletal joint index.
    pub secondary_joint: usize,
    /// Primary skeletal joint weight `[0.0, 1.0]`.
    pub primary_weight: f32,
}

#[inline]
fn lerp_rgba(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let u = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * u,
        a[1] + (b[1] - a[1]) * u,
        a[2] + (b[2] - a[2]) * u,
        a[3] + (b[3] - a[3]) * u,
    ]
}

#[inline]
fn sample_countershade(
    dorsal: [f32; 4],
    flank: [f32; 4],
    ventral: [f32; 4],
    sin_theta: f32,
) -> [f32; 4] {
    if sin_theta >= 0.0 {
        lerp_rgba(flank, dorsal, sin_theta.clamp(0.0, 1.0).powf(0.75))
    } else {
        lerp_rgba(flank, ventral, (-sin_theta).clamp(0.0, 1.0).powf(0.70))
    }
}

/// High-resolution anatomical mesh accumulator supporting multi-station elliptical lofts,
/// bicondylar long bones, curved costal rib arches, fusiform muscle bellies with tendons,
/// internal viscera, and countershaded integument envelopes.
pub struct AnatomyMeshAccumulator {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub layer_tags: Vec<ZoologicalAnatomicalLayer>,
    pub joint_indices: Vec<[u16; 4]>,
    pub joint_weights: Vec<[f32; 4]>,
}

type MeshAccumulator = AnatomyMeshAccumulator;

impl AnatomyMeshAccumulator {
    pub fn new() -> Self {
        Self {
            positions: Vec::with_capacity(6400),
            normals: Vec::with_capacity(6400),
            colors: Vec::with_capacity(6400),
            uvs: Vec::with_capacity(6400),
            indices: Vec::with_capacity(14400),
            layer_tags: Vec::with_capacity(6400),
            joint_indices: Vec::with_capacity(6400),
            joint_weights: Vec::with_capacity(6400),
        }
    }

    /// Sweeps a multi-station anatomical loft with smooth Bishop-like frames, analytical normals,
    /// end caps, and dorsal–flank–ventral countershading.
    pub fn push_countershaded_loft(
        &mut self,
        stations: &[AnatomicalLoftRing],
        radial_segs: usize,
        layer: ZoologicalAnatomicalLayer,
        cap_ends: bool,
    ) {
        if stations.len() < 2 {
            return;
        }
        let r_segs = radial_segs.max(6);
        let n_st = stations.len();
        let stride = (r_segs + 1) as u32;
        let base_idx = self.positions.len() as u32;

        for (si, st) in stations.iter().enumerate() {
            let prev = if si > 0 {
                stations[si - 1].center
            } else {
                stations[0].center
            };
            let next = if si + 1 < n_st {
                stations[si + 1].center
            } else {
                stations[n_st - 1].center
            };
            let mut tx = next[0] - prev[0];
            let mut ty = next[1] - prev[1];
            let mut tz = next[2] - prev[2];
            let tlen = (tx * tx + ty * ty + tz * tz).sqrt().max(1e-5);
            tx /= tlen;
            ty /= tlen;
            tz /= tlen;

            let ref_up = if ty.abs() < 0.92 {
                [0.0_f32, 1.0_f32, 0.0_f32]
            } else {
                [0.0_f32, 0.0_f32, if ty >= 0.0 { -1.0_f32 } else { 1.0_f32 }]
            };
            let mut ux = ref_up[1] * tz - ref_up[2] * ty;
            let mut uy = ref_up[2] * tx - ref_up[0] * tz;
            let mut uz = ref_up[0] * ty - ref_up[1] * tx;
            let ulen = (ux * ux + uy * uy + uz * uz).sqrt().max(1e-5);
            ux /= ulen;
            uy /= ulen;
            uz /= ulen;

            let vx = ty * uz - tz * uy;
            let vy = tz * ux - tx * uz;
            let vz = tx * uy - ty * ux;

            let rx = st.radii[0].max(1e-4);
            let ry = st.radii[1].max(1e-4);
            let w1 = st.primary_weight.clamp(0.0, 1.0);
            let w2 = 1.0 - w1;
            let v_uv = si as f32 / (n_st - 1) as f32;

            for s in 0..=r_segs {
                let u_uv = s as f32 / r_segs as f32;
                let theta = u_uv * core::f32::consts::TAU;
                let ct = theta.cos();
                let st_sin = theta.sin();

                let px = st.center[0] + ux * (rx * ct) + vx * (ry * st_sin);
                let py = st.center[1] + uy * (rx * ct) + vy * (ry * st_sin);
                let pz = st.center[2] + uz * (rx * ct) + vz * (ry * st_sin);

                let nx = ux * (ct / rx) + vx * (st_sin / ry);
                let ny = uy * (ct / rx) + vy * (st_sin / ry);
                let nz = uz * (ct / rx) + vz * (st_sin / ry);
                let nlen = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-5);

                let col =
                    sample_countershade(st.dorsal_color, st.flank_color, st.ventral_color, st_sin);

                self.positions.push([px, py, pz]);
                self.normals.push([nx / nlen, ny / nlen, nz / nlen]);
                self.colors.push(col);
                self.uvs.push([u_uv, v_uv]);
                self.layer_tags.push(layer);
                self.joint_indices
                    .push([st.primary_joint as u16, st.secondary_joint as u16, 0, 0]);
                self.joint_weights.push([w1, w2, 0.0, 0.0]);
            }
        }

        for si in 0..(n_st - 1) as u32 {
            for s in 0..r_segs as u32 {
                let i00 = base_idx + si * stride + s;
                let i10 = i00 + stride;
                let i01 = i00 + 1;
                let i11 = i10 + 1;
                self.indices
                    .extend_from_slice(&[i00, i10, i11, i00, i11, i01]);
            }
        }

        if cap_ends {
            let first = &stations[0];
            let c0_idx = self.positions.len() as u32;
            self.positions.push(first.center);
            self.normals.push([0.0, 0.0, -1.0]);
            self.colors.push(first.flank_color);
            self.uvs.push([0.5, 0.0]);
            self.layer_tags.push(layer);
            self.joint_indices.push([
                first.primary_joint as u16,
                first.secondary_joint as u16,
                0,
                0,
            ]);
            self.joint_weights
                .push([first.primary_weight, 1.0 - first.primary_weight, 0.0, 0.0]);
            for s in 0..r_segs as u32 {
                self.indices
                    .extend_from_slice(&[c0_idx, base_idx + s + 1, base_idx + s]);
            }

            let last = &stations[n_st - 1];
            let last_ring = base_idx + ((n_st - 1) as u32) * stride;
            let c1_idx = self.positions.len() as u32;
            self.positions.push(last.center);
            self.normals.push([0.0, 0.0, 1.0]);
            self.colors.push(last.dorsal_color);
            self.uvs.push([0.5, 1.0]);
            self.layer_tags.push(layer);
            self.joint_indices
                .push([last.primary_joint as u16, last.secondary_joint as u16, 0, 0]);
            self.joint_weights
                .push([last.primary_weight, 1.0 - last.primary_weight, 0.0, 0.0]);
            for s in 0..r_segs as u32 {
                self.indices
                    .extend_from_slice(&[c1_idx, last_ring + s, last_ring + s + 1]);
            }
        }
    }

    /// Builds an anatomical long bone with flared proximal/distal epiphyses (condyles),
    /// curved cortical diaphysis shaft, and periosteal shading.
    #[allow(clippy::too_many_arguments)]
    pub fn push_long_bone_with_epiphyses(
        &mut self,
        prox: [f32; 3],
        dist: [f32; 3],
        shaft_r: f32,
        condyle_r: f32,
        bow_offset: [f32; 3],
        bone_color: [f32; 4],
        layer: ZoologicalAnatomicalLayer,
        primary_joint: usize,
        secondary_joint: usize,
    ) {
        let cartilage_tint = [
            (bone_color[0] * 1.04).min(0.96),
            (bone_color[1] * 1.04).min(0.95),
            (bone_color[2] * 1.06).min(0.94),
            bone_color[3],
        ];
        let shaft_shade = [
            bone_color[0] * 0.94,
            bone_color[1] * 0.93,
            bone_color[2] * 0.91,
            bone_color[3],
        ];
        let mut rings = [AnatomicalLoftRing {
            center: prox,
            radii: [condyle_r, condyle_r],
            dorsal_color: cartilage_tint,
            flank_color: bone_color,
            ventral_color: shaft_shade,
            primary_joint,
            secondary_joint,
            primary_weight: 0.9,
        }; 6];

        let ts = [0.0_f32, 0.14, 0.38, 0.62, 0.86, 1.0];
        let r_scales = [
            (condyle_r * 1.05, condyle_r * 0.92),
            (condyle_r * 0.82, condyle_r * 0.78),
            (shaft_r, shaft_r * 0.94),
            (shaft_r * 0.96, shaft_r * 0.92),
            (condyle_r * 0.80, condyle_r * 0.76),
            (condyle_r * 1.02, condyle_r * 0.88),
        ];

        for i in 0..6 {
            let t = ts[i];
            let arch = (t * core::f32::consts::PI).sin();
            let col = if !(0.2..=0.8).contains(&t) {
                cartilage_tint
            } else {
                bone_color
            };
            rings[i] = AnatomicalLoftRing {
                center: [
                    prox[0] + (dist[0] - prox[0]) * t + bow_offset[0] * arch,
                    prox[1] + (dist[1] - prox[1]) * t + bow_offset[1] * arch,
                    prox[2] + (dist[2] - prox[2]) * t + bow_offset[2] * arch,
                ],
                radii: [r_scales[i].0, r_scales[i].1],
                dorsal_color: col,
                flank_color: bone_color,
                ventral_color: shaft_shade,
                primary_joint,
                secondary_joint,
                primary_weight: 1.0 - 0.75 * t,
            };
        }
        self.push_countershaded_loft(&rings, 8, layer, true);
    }

    /// Sweeps an anatomically curved costal rib arch from a thoracic vertebra around the barrel
    /// to the ventral sternum, transitioning from dorsal bone to ventral costal cartilage.
    #[allow(clippy::too_many_arguments)]
    pub fn push_curved_rib_arch(
        &mut self,
        spine_p: [f32; 3],
        sternum_p: [f32; 3],
        lateral_span: f32,
        rib_r: f32,
        bone_color: [f32; 4],
        cartilage_color: [f32; 4],
        layer: ZoologicalAnatomicalLayer,
        primary_joint: usize,
        secondary_joint: usize,
    ) {
        let mut rings = [AnatomicalLoftRing {
            center: spine_p,
            radii: [rib_r, rib_r],
            dorsal_color: bone_color,
            flank_color: bone_color,
            ventral_color: bone_color,
            primary_joint,
            secondary_joint,
            primary_weight: 0.75,
        }; 6];

        for (i, ring) in rings.iter_mut().enumerate() {
            let t = i as f32 / 5.0;
            let arch_x = (t * core::f32::consts::PI).sin() * lateral_span;
            let arch_z = (t * core::f32::consts::PI).sin() * (-0.015);
            let col = if t > 0.68 {
                lerp_rgba(bone_color, cartilage_color, (t - 0.68) / 0.32)
            } else {
                bone_color
            };
            *ring = AnatomicalLoftRing {
                center: [
                    spine_p[0] + (sternum_p[0] - spine_p[0]) * t + arch_x,
                    spine_p[1] + (sternum_p[1] - spine_p[1]) * t.powf(0.85),
                    spine_p[2] + (sternum_p[2] - spine_p[2]) * t + arch_z,
                ],
                radii: [rib_r * (0.85 + 0.25 * (1.0 - t)), rib_r * 0.68],
                dorsal_color: col,
                flank_color: col,
                ventral_color: col,
                primary_joint,
                secondary_joint,
                primary_weight: 0.75,
            };
        }
        self.push_countershaded_loft(&rings, 6, layer, false);
    }

    /// Builds a fusiform skeletal muscle belly with tapered ivory collagen tendon attachments
    /// at origin and insertion and a swollen striated crimson belly in the middle.
    #[allow(clippy::too_many_arguments)]
    pub fn push_fusiform_muscle(
        &mut self,
        origin: [f32; 3],
        insertion: [f32; 3],
        belly_rx: f32,
        belly_ry: f32,
        belly_bow: [f32; 3],
        muscle_color: [f32; 4],
        tendon_color: [f32; 4],
        layer: ZoologicalAnatomicalLayer,
        primary_joint: usize,
        secondary_joint: usize,
    ) {
        let dark_fascia = [
            muscle_color[0] * 0.78,
            muscle_color[1] * 0.75,
            muscle_color[2] * 0.75,
            muscle_color[3],
        ];
        let mut rings = [AnatomicalLoftRing {
            center: origin,
            radii: [belly_rx * 0.22, belly_ry * 0.22],
            dorsal_color: tendon_color,
            flank_color: tendon_color,
            ventral_color: tendon_color,
            primary_joint,
            secondary_joint,
            primary_weight: 0.85,
        }; 7];

        for (i, ring) in rings.iter_mut().enumerate() {
            let t = i as f32 / 6.0;
            let fusiform = (t * core::f32::consts::PI).sin().max(0.0).powf(0.75);
            let r_scale = 0.18 + 0.82 * fusiform;
            let tendon_blend = if t < 0.20 {
                1.0 - (t / 0.20)
            } else if t > 0.80 {
                (t - 0.80) / 0.20
            } else {
                0.0
            };
            let dorsal_c = lerp_rgba(muscle_color, tendon_color, tendon_blend);
            let ventral_c = lerp_rgba(dark_fascia, tendon_color, tendon_blend);

            *ring = AnatomicalLoftRing {
                center: [
                    origin[0] + (insertion[0] - origin[0]) * t + belly_bow[0] * fusiform,
                    origin[1] + (insertion[1] - origin[1]) * t + belly_bow[1] * fusiform,
                    origin[2] + (insertion[2] - origin[2]) * t + belly_bow[2] * fusiform,
                ],
                radii: [belly_rx * r_scale, belly_ry * r_scale],
                dorsal_color: dorsal_c,
                flank_color: dorsal_c,
                ventral_color: ventral_c,
                primary_joint,
                secondary_joint,
                primary_weight: 1.0 - 0.70 * t,
            };
        }
        self.push_countershaded_loft(&rings, 8, layer, true);
    }

    pub fn push_cylinder(
        &mut self,
        p1: [f32; 3],
        p2: [f32; 3],
        r1: f32,
        r2: f32,
        radial_segs: usize,
        height_segs: usize,
        color: [f32; 4],
        layer: ZoologicalAnatomicalLayer,
        primary_joint: usize,
        secondary_joint: usize,
        primary_weight: f32,
    ) {
        let base_idx = self.positions.len() as u32;
        let axis = [p2[0] - p1[0], p2[1] - p1[1], p2[2] - p1[2]];
        let height = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2])
            .sqrt()
            .max(1e-5);
        let dir = [axis[0] / height, axis[1] / height, axis[2] / height];

        let up = if dir[1].abs() < 0.95 {
            [0.0, 1.0, 0.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let side = [
            dir[1] * up[2] - dir[2] * up[1],
            dir[2] * up[0] - dir[0] * up[2],
            dir[0] * up[1] - dir[1] * up[0],
        ];
        let side_len = (side[0] * side[0] + side[1] * side[1] + side[2] * side[2])
            .sqrt()
            .max(1e-5);
        let u_vec = [side[0] / side_len, side[1] / side_len, side[2] / side_len];
        let v_vec = [
            dir[1] * u_vec[2] - dir[2] * u_vec[1],
            dir[2] * u_vec[0] - dir[0] * u_vec[2],
            dir[0] * u_vec[1] - dir[1] * u_vec[0],
        ];

        let r_segs = radial_segs.max(6);
        let h_segs = height_segs.max(1);

        for h in 0..=h_segs {
            let t = h as f32 / h_segs as f32;
            let center = [
                p1[0] + dir[0] * height * t,
                p1[1] + dir[1] * height * t,
                p1[2] + dir[2] * height * t,
            ];
            let radius = r1 + (r2 - r1) * t;
            let w1 = primary_weight;
            let w2 = 1.0 - w1;

            for s in 0..=r_segs {
                let theta = (s as f32 / r_segs as f32) * core::f32::consts::TAU;
                let cos_t = theta.cos();
                let sin_t = theta.sin();

                let normal = [
                    u_vec[0] * cos_t + v_vec[0] * sin_t,
                    u_vec[1] * cos_t + v_vec[1] * sin_t,
                    u_vec[2] * cos_t + v_vec[2] * sin_t,
                ];
                let pos = [
                    center[0] + normal[0] * radius,
                    center[1] + normal[1] * radius,
                    center[2] + normal[2] * radius,
                ];

                self.positions.push(pos);
                self.normals.push(normal);
                self.colors.push(color);
                self.uvs.push([s as f32 / r_segs as f32, t]);
                self.layer_tags.push(layer);
                self.joint_indices
                    .push([primary_joint as u16, secondary_joint as u16, 0, 0]);
                self.joint_weights.push([w1, w2, 0.0, 0.0]);
            }
        }

        let stride = (r_segs + 1) as u32;
        for h in 0..h_segs as u32 {
            for s in 0..r_segs as u32 {
                let i00 = base_idx + h * stride + s;
                let i10 = i00 + stride;
                let i01 = i00 + 1;
                let i11 = i10 + 1;

                self.indices
                    .extend_from_slice(&[i00, i10, i11, i00, i11, i01]);
            }
        }
    }

    pub fn push_ellipsoid(
        &mut self,
        center: [f32; 3],
        radii: [f32; 3],
        u_segs: usize,
        v_segs: usize,
        color: [f32; 4],
        layer: ZoologicalAnatomicalLayer,
        primary_joint: usize,
        secondary_joint: usize,
        primary_weight: f32,
    ) {
        let base_idx = self.positions.len() as u32;
        let u_n = u_segs.max(8);
        let v_n = v_segs.max(6);

        for v in 0..=v_n {
            let phi = (v as f32 / v_n as f32) * core::f32::consts::PI;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            for u in 0..=u_n {
                let theta = (u as f32 / u_n as f32) * core::f32::consts::TAU;
                let cos_theta = theta.cos();
                let sin_theta = theta.sin();

                let nx = sin_phi * cos_theta;
                let ny = cos_phi;
                let nz = sin_phi * sin_theta;

                let px = center[0] + radii[0] * nx;
                let py = center[1] + radii[1] * ny;
                let pz = center[2] + radii[2] * nz;

                let normal = [
                    nx / radii[0].max(1e-4),
                    ny / radii[1].max(1e-4),
                    nz / radii[2].max(1e-4),
                ];
                let n_len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2])
                    .sqrt()
                    .max(1e-5);
                let unit_normal = [normal[0] / n_len, normal[1] / n_len, normal[2] / n_len];

                self.positions.push([px, py, pz]);
                self.normals.push(unit_normal);
                self.colors.push(color);
                self.uvs
                    .push([u as f32 / u_n as f32, v as f32 / v_n as f32]);
                self.layer_tags.push(layer);
                self.joint_indices
                    .push([primary_joint as u16, secondary_joint as u16, 0, 0]);
                self.joint_weights
                    .push([primary_weight, 1.0 - primary_weight, 0.0, 0.0]);
            }
        }

        let stride = (u_n + 1) as u32;
        for v in 0..v_n as u32 {
            for u in 0..u_n as u32 {
                let i00 = base_idx + v * stride + u;
                let i10 = i00 + stride;
                let i01 = i00 + 1;
                let i11 = i10 + 1;

                self.indices
                    .extend_from_slice(&[i00, i10, i11, i00, i11, i01]);
            }
        }
    }

    pub fn push_box(
        &mut self,
        min: [f32; 3],
        max: [f32; 3],
        color: [f32; 4],
        layer: ZoologicalAnatomicalLayer,
        primary_joint: usize,
    ) {
        let base_idx = self.positions.len() as u32;

        let corners = [
            [min[0], min[1], min[2]],
            [max[0], min[1], min[2]],
            [max[0], max[1], min[2]],
            [min[0], max[1], min[2]],
            [min[0], min[1], max[2]],
            [max[0], min[1], max[2]],
            [max[0], max[1], max[2]],
            [min[0], max[1], max[2]],
        ];

        let center = [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ];

        for c in corners {
            let n = [c[0] - center[0], c[1] - center[1], c[2] - center[2]];
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-5);
            self.positions.push(c);
            self.normals.push([n[0] / len, n[1] / len, n[2] / len]);
            self.colors.push(color);
            self.uvs.push([0.5, 0.5]);
            self.layer_tags.push(layer);
            self.joint_indices.push([primary_joint as u16, 0, 0, 0]);
            self.joint_weights.push([1.0, 0.0, 0.0, 0.0]);
        }

        let face_indices = [
            0, 2, 1, 0, 3, 2, // -Z
            4, 5, 6, 4, 6, 7, // +Z
            0, 1, 5, 0, 5, 4, // -Y
            3, 6, 2, 3, 7, 6, // +Y
            0, 4, 7, 0, 7, 3, // -X
            1, 2, 6, 1, 6, 5, // +X
        ];

        for &idx in &face_indices {
            self.indices.push(base_idx + idx);
        }
    }
}

// ── PBR Material Builder Helpers ─────────────────────────────────────────────

pub fn make_pbr_material(
    id_str: &str,
    base_color: [f32; 4],
    roughness: f32,
    metallic: f32,
) -> MaterialRecord {
    let mut mat = MaterialRecord::legacy_default();
    mat.id = q_hash(id_str);
    mat.shading_model = ShadingModel::MetallicRoughness;
    mat.opacity_mode = OpacityMode::Opaque;
    mat.base_color = base_color;
    mat.roughness = roughness;
    mat.metallic = metallic;
    mat
}

// ── Main Anatomical 10D Builder ──────────────────────────────────────────────

/// Silhouette-level comparative profile for the broad fauna rigs. Ratios are
/// authored presentation defaults, not validated species measurements.
#[derive(Clone, Copy)]
struct ArchetypeMorphology {
    trunk: f32,
    body_width: f32,
    neck: f32,
    head: f32,
    tail: f32,
    forelimb: f32,
    hindlimb: f32,
    shoulder_span: f32,
    patagium: f32,
}

impl ArchetypeMorphology {
    fn for_archetype(archetype: FaunaBiomechanicalArchetype) -> Self {
        use FaunaBiomechanicalArchetype as A;
        match archetype {
            A::MacropodSaltatorial => Self {
                trunk: 1.05,
                body_width: 1.05,
                neck: 0.9,
                head: 0.95,
                tail: 1.55,
                forelimb: 0.72,
                hindlimb: 1.32,
                shoulder_span: 0.9,
                patagium: 0.0,
            },
            A::VombatidFossorial => Self {
                trunk: 1.05,
                body_width: 1.42,
                neck: 0.72,
                head: 1.12,
                tail: 0.35,
                forelimb: 0.82,
                hindlimb: 0.82,
                shoulder_span: 0.95,
                patagium: 0.0,
            },
            A::PhascolarctidArboreal => Self {
                trunk: 0.95,
                body_width: 1.08,
                neck: 0.88,
                head: 1.05,
                tail: 0.62,
                forelimb: 1.12,
                hindlimb: 0.96,
                shoulder_span: 1.02,
                patagium: 0.0,
            },
            A::PetauridGlider => Self {
                trunk: 0.92,
                body_width: 0.82,
                neck: 0.88,
                head: 0.92,
                tail: 1.3,
                forelimb: 1.15,
                hindlimb: 0.98,
                shoulder_span: 1.12,
                patagium: 0.72,
            },
            A::DasyuridPredator => Self {
                trunk: 0.96,
                body_width: 0.82,
                neck: 1.05,
                head: 0.9,
                tail: 1.12,
                forelimb: 0.98,
                hindlimb: 1.12,
                shoulder_span: 0.92,
                patagium: 0.0,
            },
            A::PteropodidChiroptera => Self {
                trunk: 0.82,
                body_width: 0.9,
                neck: 0.76,
                head: 0.84,
                tail: 0.42,
                forelimb: 1.62,
                hindlimb: 0.72,
                shoulder_span: 1.55,
                patagium: 1.0,
            },
            A::MarinePinnipedCetacean => Self {
                trunk: 1.48,
                body_width: 0.86,
                neck: 0.64,
                head: 1.08,
                tail: 1.0,
                forelimb: 1.16,
                hindlimb: 0.28,
                shoulder_span: 1.1,
                patagium: 0.0,
            },
            A::RatiteBipedal => Self {
                trunk: 1.12,
                body_width: 0.86,
                neck: 1.42,
                head: 0.78,
                tail: 0.42,
                forelimb: 0.58,
                hindlimb: 1.34,
                shoulder_span: 0.82,
                patagium: 0.0,
            },
            A::RaptorSoaring => Self {
                trunk: 0.98,
                body_width: 0.88,
                neck: 1.08,
                head: 0.92,
                tail: 0.82,
                forelimb: 1.38,
                hindlimb: 0.82,
                shoulder_span: 1.42,
                patagium: 0.0,
            },
            A::PasserinePsittacine => Self {
                trunk: 0.82,
                body_width: 0.82,
                neck: 0.9,
                head: 0.88,
                tail: 0.88,
                forelimb: 1.18,
                hindlimb: 0.9,
                shoulder_span: 1.16,
                patagium: 0.0,
            },
            A::SquamateSprawling => Self {
                trunk: 1.28,
                body_width: 0.92,
                neck: 0.78,
                head: 0.92,
                tail: 1.34,
                forelimb: 0.72,
                hindlimb: 0.72,
                shoulder_span: 1.0,
                patagium: 0.0,
            },
            A::SerpentineUndulatory => Self {
                trunk: 1.62,
                body_width: 0.62,
                neck: 0.68,
                head: 0.72,
                tail: 1.45,
                forelimb: 0.18,
                hindlimb: 0.18,
                shoulder_span: 0.8,
                patagium: 0.0,
            },
            A::MonotremeTeleostAmphibian => Self {
                trunk: 1.08,
                body_width: 1.08,
                neck: 0.72,
                head: 1.0,
                tail: 0.9,
                forelimb: 0.88,
                hindlimb: 0.88,
                shoulder_span: 1.0,
                patagium: 0.0,
            },
        }
    }
}

/// Builds a high-resolution, 3-layer Comparative Vertebrate Anatomical Digital Twin
/// bound to `FaunaSkeletalRig10d` (18 PGA joints) and sealed into a `.10d` Living Container.
///
/// Features:
/// - >= 1,800 vertices across 8 Osteology regions, 8 Myology groups, and External Integument.
/// - Distinct submesh ranges for all 17 anatomical structures.
/// - Valid `.10d` container with `MAT4`, `UV01`, `FNRM`, `SRD1`, `PROC1`, `EONT1`, `MRP1`, and `PROV`.
/// - 3 Morph Targets: Costal Respiration, Gular Vocalization, and Achilles Tendon Strain.
pub fn build_scale5_vertebrate_anatomy_10d(
    archetype: FaunaBiomechanicalArchetype,
    osteology_vis_01: f32,
    myology_vis_01: f32,
    integument_vis_01: f32,
    explode_layer_01: f32,
) -> CompiledVertebrateAnatomy10d {
    let (base_body_len, hip_height) = archetype.default_dimensions();
    let morphology = ArchetypeMorphology::for_archetype(archetype);
    let body_len = base_body_len * morphology.trunk;

    let mut acc = MeshAccumulator::new();
    let mut submesh_ranges = Vec::with_capacity(17);
    let mut materials = Vec::with_capacity(17);

    // Anatomical proportions
    let spine_len = (body_len * 0.32).max(0.10);
    let neck_len = (body_len * 0.18 * morphology.neck).max(0.06);
    let head_len = (body_len * 0.22 * morphology.head).max(0.08);
    let tail_len = (body_len * 0.40 * morphology.tail).max(0.12);
    let half_width = (body_len * 0.14 * morphology.body_width).max(0.04);
    let femur_len = (hip_height * 0.48 * morphology.hindlimb).max(0.025);
    let tibia_len = (hip_height * 0.54 * morphology.hindlimb).max(0.025);

    // ── LAYER 0: OSTEOLOGY (8 Regions) ───────────────────────────────────────
    let ost_start_v = acc.positions.len();
    let bone_ivory = [0.89, 0.87, 0.81, 1.0];
    let bone_dense = [0.85, 0.83, 0.77, 1.0];
    let cartilage_blue = [0.82, 0.85, 0.86, 1.0];
    let enamel_white = [0.95, 0.94, 0.91, 1.0];
    let tendon_ivory = [0.92, 0.90, 0.85, 1.0];

    let is_monotreme = archetype == FaunaBiomechanicalArchetype::MonotremeTeleostAmphibian;
    let is_avian = matches!(
        archetype,
        FaunaBiomechanicalArchetype::RaptorSoaring
            | FaunaBiomechanicalArchetype::PasserinePsittacine
            | FaunaBiomechanicalArchetype::RatiteBipedal
    );

    // 1. Cranium & Rostrum (Multi-Station Sculpted Neurocranium, Nasal Bridge, Rostrum & Zygomatic Arches)
    let cranium_start = acc.indices.len() as u32;
    let cranium_head_pos = [0.0, hip_height + neck_len * 0.6, spine_len * 0.7 + neck_len];
    let rostrum_tip = [
        0.0,
        cranium_head_pos[1] - 0.025,
        cranium_head_pos[2] + head_len,
    ];

    let rostrum_rx = if is_monotreme {
        half_width * 0.44
    } else if is_avian {
        half_width * 0.08
    } else {
        half_width * 0.20
    };
    let rostrum_ry = if is_monotreme {
        half_width * 0.10
    } else if is_avian {
        half_width * 0.08
    } else {
        half_width * 0.18
    };

    let cranium_loft = [
        AnatomicalLoftRing {
            center: [
                0.0,
                cranium_head_pos[1] - 0.01,
                cranium_head_pos[2] - head_len * 0.34,
            ],
            radii: [half_width * 0.38, half_width * 0.36],
            dorsal_color: bone_ivory,
            flank_color: bone_dense,
            ventral_color: bone_dense,
            primary_joint: JOINT_HEAD,
            secondary_joint: JOINT_NECK,
            primary_weight: 0.75,
        },
        AnatomicalLoftRing {
            center: cranium_head_pos,
            radii: [half_width * 0.56, half_width * 0.50],
            dorsal_color: bone_ivory,
            flank_color: bone_ivory,
            ventral_color: bone_dense,
            primary_joint: JOINT_HEAD,
            secondary_joint: JOINT_NECK,
            primary_weight: 0.90,
        },
        AnatomicalLoftRing {
            center: [
                0.0,
                cranium_head_pos[1] + 0.005,
                cranium_head_pos[2] + head_len * 0.32,
            ],
            radii: [half_width * 0.44, half_width * 0.38],
            dorsal_color: bone_ivory,
            flank_color: bone_dense,
            ventral_color: bone_dense,
            primary_joint: JOINT_HEAD,
            secondary_joint: JOINT_JAW,
            primary_weight: 0.92,
        },
        AnatomicalLoftRing {
            center: [
                0.0,
                cranium_head_pos[1] - 0.012,
                cranium_head_pos[2] + head_len * 0.68,
            ],
            radii: [
                (half_width * 0.32).max(rostrum_rx),
                (half_width * 0.25).max(rostrum_ry),
            ],
            dorsal_color: bone_ivory,
            flank_color: bone_dense,
            ventral_color: bone_dense,
            primary_joint: JOINT_HEAD,
            secondary_joint: JOINT_JAW,
            primary_weight: 0.95,
        },
        AnatomicalLoftRing {
            center: rostrum_tip,
            radii: [rostrum_rx, rostrum_ry],
            dorsal_color: bone_ivory,
            flank_color: bone_dense,
            ventral_color: bone_dense,
            primary_joint: JOINT_HEAD,
            secondary_joint: JOINT_JAW,
            primary_weight: 0.98,
        },
    ];
    acc.push_countershaded_loft(
        &cranium_loft,
        12,
        ZoologicalAnatomicalLayer::Osteology,
        true,
    );

    // Sculpted Zygomatic Arches & Supraorbital Ridges (L+R)
    for side in [-1.0_f32, 1.0_f32] {
        acc.push_long_bone_with_epiphyses(
            [
                side * half_width * 0.44,
                cranium_head_pos[1] - 0.02,
                cranium_head_pos[2] - 0.03,
            ],
            [
                side * half_width * 0.38,
                cranium_head_pos[1] - 0.01,
                cranium_head_pos[2] + head_len * 0.42,
            ],
            0.008,
            0.012,
            [side * half_width * 0.14, -0.008, 0.0],
            bone_ivory,
            ZoologicalAnatomicalLayer::Osteology,
            JOINT_HEAD,
            JOINT_NECK,
        );
    }

    let cranium_mat = make_pbr_material("mat_ost_cranium", bone_ivory, 0.45, 0.05);
    materials.push(cranium_mat);
    submesh_ranges.push(SubmeshRange {
        first_index: cranium_start,
        index_count: acc.indices.len() as u32 - cranium_start,
        material_id: cranium_mat.id,
        semantic_id: q_hash("urn:qualia:bio:submesh:cranium_rostrum"),
    });

    // 2. Mandible & Dentition (Curved Mandibular Rami, Coronoid Process, Procumbent Incisors & Molars)
    let mandible_start = acc.indices.len() as u32;
    let jaw_joint_l = [
        -half_width * 0.40,
        cranium_head_pos[1] - 0.04,
        cranium_head_pos[2],
    ];
    let jaw_joint_r = [
        half_width * 0.40,
        cranium_head_pos[1] - 0.04,
        cranium_head_pos[2],
    ];
    let chin_pos = [0.0, cranium_head_pos[1] - 0.07, rostrum_tip[2] - 0.02];

    for (side, jaw_j) in [(-1.0_f32, jaw_joint_l), (1.0_f32, jaw_joint_r)] {
        // Mandibular ramus & body with masseteric curvature
        acc.push_long_bone_with_epiphyses(
            jaw_j,
            chin_pos,
            half_width * 0.11,
            half_width * 0.16,
            [side * half_width * 0.08, -0.018, 0.0],
            bone_ivory,
            ZoologicalAnatomicalLayer::Osteology,
            JOINT_JAW,
            JOINT_HEAD,
        );
        // Ascending coronoid process
        acc.push_cylinder(
            [jaw_j[0] * 0.88, jaw_j[1] - 0.01, jaw_j[2] + head_len * 0.14],
            [
                jaw_j[0] * 0.84,
                jaw_j[1] + 0.032,
                jaw_j[2] + head_len * 0.10,
            ],
            0.009,
            0.004,
            6,
            2,
            bone_dense,
            ZoologicalAnatomicalLayer::Osteology,
            JOINT_JAW,
            JOINT_HEAD,
            0.90,
        );
        // Row of 4 bilophodont/hypsodont cheek molars along each dentary
        for m in 0..4 {
            let mt = 0.35 + (m as f32) * 0.14;
            let mx = jaw_j[0] * (1.0 - mt) + chin_pos[0] * mt;
            let my = jaw_j[1] * (1.0 - mt) + chin_pos[1] * mt + 0.014;
            let mz = jaw_j[2] * (1.0 - mt) + chin_pos[2] * mt;
            acc.push_ellipsoid(
                [mx, my, mz],
                [0.007, 0.009, 0.008],
                6,
                4,
                enamel_white,
                ZoologicalAnatomicalLayer::Osteology,
                JOINT_JAW,
                JOINT_HEAD,
                0.95,
            );
        }
    }

    // Procumbent lower incisors (characteristic of Diprotodontia)
    let incisor_tip = [0.0, chin_pos[1] + 0.016, chin_pos[2] + 0.038];
    acc.push_cylinder(
        chin_pos,
        incisor_tip,
        0.008,
        0.004,
        8,
        2,
        enamel_white,
        ZoologicalAnatomicalLayer::Osteology,
        JOINT_JAW,
        JOINT_HEAD,
        0.95,
    );

    let mandible_mat = make_pbr_material("mat_ost_mandible", [0.90, 0.88, 0.82, 1.0], 0.32, 0.02);
    materials.push(mandible_mat);
    submesh_ranges.push(SubmeshRange {
        first_index: mandible_start,
        index_count: acc.indices.len() as u32 - mandible_start,
        material_id: mandible_mat.id,
        semantic_id: q_hash("urn:qualia:bio:submesh:mandible_dentition"),
    });

    // 3. Cervical, Thoracic & Lumbar Vertebrae (Segmented Centra, Neural Spines & Transverse Processes)
    let vert_start = acc.indices.len() as u32;
    let pelvis_pos = [0.0, hip_height, 0.0];
    let chest_pos = [0.0, hip_height + 0.05, spine_len * 0.7];

    // Cervical vertebral loft (S-curved neck column)
    let cervical_loft = [
        AnatomicalLoftRing {
            center: chest_pos,
            radii: [half_width * 0.21, half_width * 0.19],
            dorsal_color: bone_dense,
            flank_color: bone_ivory,
            ventral_color: cartilage_blue,
            primary_joint: JOINT_CHEST,
            secondary_joint: JOINT_NECK,
            primary_weight: 0.65,
        },
        AnatomicalLoftRing {
            center: [
                0.0,
                (chest_pos[1] + cranium_head_pos[1]) * 0.46,
                (chest_pos[2] + cranium_head_pos[2]) * 0.52,
            ],
            radii: [half_width * 0.19, half_width * 0.17],
            dorsal_color: bone_ivory,
            flank_color: bone_dense,
            ventral_color: cartilage_blue,
            primary_joint: JOINT_NECK,
            secondary_joint: JOINT_CHEST,
            primary_weight: 0.80,
        },
        AnatomicalLoftRing {
            center: cranium_head_pos,
            radii: [half_width * 0.17, half_width * 0.16],
            dorsal_color: bone_ivory,
            flank_color: bone_dense,
            ventral_color: bone_dense,
            primary_joint: JOINT_NECK,
            secondary_joint: JOINT_HEAD,
            primary_weight: 0.75,
        },
    ];
    acc.push_countershaded_loft(
        &cervical_loft,
        8,
        ZoologicalAnatomicalLayer::Osteology,
        false,
    );

    // Thoracic & Lumbar vertebral column with curved dorsal neural spines and lateral transverse processes
    acc.push_cylinder(
        pelvis_pos,
        chest_pos,
        half_width * 0.24,
        half_width * 0.20,
        8,
        7,
        bone_dense,
        ZoologicalAnatomicalLayer::Osteology,
        JOINT_SPINE,
        JOINT_CHEST,
        0.55,
    );

    for s in 1..=7 {
        let t = s as f32 / 8.0;
        let c = [
            pelvis_pos[0] + (chest_pos[0] - pelvis_pos[0]) * t,
            pelvis_pos[1] + (chest_pos[1] - pelvis_pos[1]) * t,
            pelvis_pos[2] + (chest_pos[2] - pelvis_pos[2]) * t,
        ];
        // Dorsal spinous process (taller over withers/thorax, angling caudally)
        let spine_h = half_width * (0.34 + 0.22 * (t * core::f32::consts::PI).sin());
        let tilt_z = if t > 0.45 { -0.018 } else { 0.008 };
        acc.push_cylinder(
            [c[0], c[1] + half_width * 0.14, c[2]],
            [c[0], c[1] + half_width * 0.14 + spine_h, c[2] + tilt_z],
            0.008,
            0.004,
            6,
            2,
            bone_ivory,
            ZoologicalAnatomicalLayer::Osteology,
            JOINT_SPINE,
            JOINT_CHEST,
            1.0 - t * 0.6,
        );
        // Paired lumbar/thoracic transverse processes
        let tp_span = half_width * (0.42 - 0.14 * t);
        acc.push_cylinder(
            [-tp_span, c[1] - 0.004, c[2]],
            [tp_span, c[1] - 0.004, c[2]],
            0.006,
            0.006,
            6,
            2,
            bone_dense,
            ZoologicalAnatomicalLayer::Osteology,
            JOINT_SPINE,
            JOINT_CHEST,
            1.0 - t * 0.6,
        );
    }

    let vert_mat = make_pbr_material("mat_ost_vertebrae", bone_dense, 0.50, 0.02);
    materials.push(vert_mat);
    submesh_ranges.push(SubmeshRange {
        first_index: vert_start,
        index_count: acc.indices.len() as u32 - vert_start,
        material_id: vert_mat.id,
        semantic_id: q_hash("urn:qualia:bio:submesh:vertebrae"),
    });

    // 4. Ribcage & Sternum (8 Pairs of Anatomically Curved Costal Rib Arches, Sternebrae / Carina & Furcula)
    let rib_start = acc.indices.len() as u32;
    let sternum_y = chest_pos[1] - half_width * 0.78;
    let sternum_pos = [0.0, sternum_y, chest_pos[2] * 0.80];

    for r in 1..=8 {
        let t = r as f32 / 9.0;
        let spine_p = [
            pelvis_pos[0] + (chest_pos[0] - pelvis_pos[0]) * (0.24 + 0.70 * t),
            pelvis_pos[1] + (chest_pos[1] - pelvis_pos[1]) * (0.24 + 0.70 * t),
            pelvis_pos[2] + (chest_pos[2] - pelvis_pos[2]) * (0.24 + 0.70 * t),
        ];
        let sternum_p = [
            sternum_pos[0],
            sternum_pos[1] + 0.01 * (1.0 - (t * core::f32::consts::PI).sin()),
            sternum_pos[2] + (t - 0.5) * spine_len * 0.48,
        ];
        let barrel_bulge = half_width * (0.58 + 0.34 * (t * core::f32::consts::PI).sin());
        acc.push_curved_rib_arch(
            spine_p,
            sternum_p,
            -barrel_bulge,
            0.007,
            bone_ivory,
            cartilage_blue,
            ZoologicalAnatomicalLayer::Osteology,
            JOINT_CHEST,
            JOINT_SPINE,
        );
        acc.push_curved_rib_arch(
            spine_p,
            sternum_p,
            barrel_bulge,
            0.007,
            bone_ivory,
            cartilage_blue,
            ZoologicalAnatomicalLayer::Osteology,
            JOINT_CHEST,
            JOINT_SPINE,
        );
    }

    // Sternum / Keeled Carina loft
    let carina_depth = if is_avian {
        half_width * 0.58
    } else {
        half_width * 0.14
    };
    let sternum_loft = [
        AnatomicalLoftRing {
            center: [
                0.0,
                sternum_pos[1] - carina_depth * 0.3,
                sternum_pos[2] - spine_len * 0.25,
            ],
            radii: [0.012, carina_depth * 0.45],
            dorsal_color: bone_dense,
            flank_color: bone_ivory,
            ventral_color: cartilage_blue,
            primary_joint: JOINT_CHEST,
            secondary_joint: JOINT_SPINE,
            primary_weight: 0.85,
        },
        AnatomicalLoftRing {
            center: [0.0, sternum_pos[1] - carina_depth * 0.5, sternum_pos[2]],
            radii: [0.014, carina_depth * 0.65],
            dorsal_color: bone_ivory,
            flank_color: bone_ivory,
            ventral_color: bone_dense,
            primary_joint: JOINT_CHEST,
            secondary_joint: JOINT_SPINE,
            primary_weight: 0.90,
        },
        AnatomicalLoftRing {
            center: [
                0.0,
                sternum_pos[1] - carina_depth * 0.2,
                sternum_pos[2] + spine_len * 0.26,
            ],
            radii: [0.016, carina_depth * 0.35],
            dorsal_color: bone_ivory,
            flank_color: bone_dense,
            ventral_color: bone_dense,
            primary_joint: JOINT_CHEST,
            secondary_joint: JOINT_NECK,
            primary_weight: 0.92,
        },
    ];
    acc.push_countershaded_loft(&sternum_loft, 8, ZoologicalAnatomicalLayer::Osteology, true);

    let rib_mat = make_pbr_material("mat_ost_ribcage", [0.83, 0.81, 0.75, 1.0], 0.42, 0.02);
    materials.push(rib_mat);
    submesh_ranges.push(SubmeshRange {
        first_index: rib_start,
        index_count: acc.indices.len() as u32 - rib_start,
        material_id: rib_mat.id,
        semantic_id: q_hash("urn:qualia:bio:submesh:ribcage_sternum"),
    });

    // 5. Pectoral Girdle & Forelimb (Sculpted Scapular Blades, Humerus, Radius/Ulna & Digits)
    let pect_start = acc.indices.len() as u32;
    let shoulder_l = [
        -half_width * 0.70 * morphology.shoulder_span,
        chest_pos[1] - 0.02,
        chest_pos[2] * 0.9,
    ];
    let shoulder_r = [
        half_width * 0.70 * morphology.shoulder_span,
        chest_pos[1] - 0.02,
        chest_pos[2] * 0.9,
    ];
    let elbow_l = [
        -half_width * 0.80 * morphology.shoulder_span,
        chest_pos[1] - femur_len * 0.55 * morphology.forelimb,
        shoulder_l[2] - 0.04,
    ];
    let elbow_r = [
        half_width * 0.80 * morphology.shoulder_span,
        chest_pos[1] - femur_len * 0.55 * morphology.forelimb,
        shoulder_r[2] - 0.04,
    ];
    let wrist_l = [
        -half_width * 0.75 * morphology.shoulder_span,
        chest_pos[1] - femur_len * 0.95 * morphology.forelimb,
        elbow_l[2] + 0.08,
    ];
    let wrist_r = [
        half_width * 0.75 * morphology.shoulder_span,
        chest_pos[1] - femur_len * 0.95 * morphology.forelimb,
        elbow_r[2] + 0.08,
    ];

    for (side, sh, el, wr) in [
        (-1.0_f32, shoulder_l, elbow_l, wrist_l),
        (1.0_f32, shoulder_r, elbow_r, wrist_r),
    ] {
        // Flared triangular scapular blade with scapular spine
        let scap_loft = [
            AnatomicalLoftRing {
                center: sh,
                radii: [0.014, 0.020],
                dorsal_color: cartilage_blue,
                flank_color: bone_ivory,
                ventral_color: bone_dense,
                primary_joint: JOINT_LEFT_SHOULDER,
                secondary_joint: JOINT_CHEST,
                primary_weight: 0.85,
            },
            AnatomicalLoftRing {
                center: [
                    sh[0] * 0.88,
                    sh[1] + half_width * 0.34,
                    sh[2] - spine_len * 0.12,
                ],
                radii: [0.010, 0.042],
                dorsal_color: bone_ivory,
                flank_color: bone_ivory,
                ventral_color: bone_dense,
                primary_joint: JOINT_LEFT_SHOULDER,
                secondary_joint: JOINT_CHEST,
                primary_weight: 0.70,
            },
            AnatomicalLoftRing {
                center: [
                    sh[0] * 0.72,
                    sh[1] + half_width * 0.56,
                    sh[2] - spine_len * 0.22,
                ],
                radii: [0.008, 0.056],
                dorsal_color: cartilage_blue,
                flank_color: bone_ivory,
                ventral_color: bone_dense,
                primary_joint: JOINT_CHEST,
                secondary_joint: JOINT_LEFT_SHOULDER,
                primary_weight: 0.65,
            },
        ];
        acc.push_countershaded_loft(&scap_loft, 8, ZoologicalAnatomicalLayer::Osteology, true);

        // Humerus with proximal head and distal trochlear condyles
        acc.push_long_bone_with_epiphyses(
            sh,
            el,
            half_width * 0.11,
            half_width * 0.16,
            [side * 0.008, 0.0, 0.008],
            bone_ivory,
            ZoologicalAnatomicalLayer::Osteology,
            JOINT_LEFT_SHOULDER,
            JOINT_LEFT_ELBOW,
        );
        // Radius & Ulna with olecranon process
        acc.push_long_bone_with_epiphyses(
            [el[0], el[1] + 0.012, el[2] - 0.014],
            wr,
            half_width * 0.085,
            half_width * 0.12,
            [0.0, -0.006, 0.006],
            bone_ivory,
            ZoologicalAnatomicalLayer::Osteology,
            JOINT_LEFT_ELBOW,
            JOINT_LEFT_WRIST_WING,
        );
        // Metacarpals & phalanges
        for d in -1..=1 {
            let dx = (d as f32) * 0.010;
            acc.push_cylinder(
                [wr[0] + dx, wr[1], wr[2]],
                [wr[0] + dx * 1.3, wr[1] - 0.025, wr[2] + 0.035],
                0.006,
                0.003,
                6,
                2,
                bone_dense,
                ZoologicalAnatomicalLayer::Osteology,
                JOINT_LEFT_WRIST_WING,
                JOINT_LEFT_ELBOW,
                0.95,
            );
        }
    }

    let pect_mat = make_pbr_material("mat_ost_pectoral", bone_ivory, 0.40, 0.02);
    materials.push(pect_mat);
    submesh_ranges.push(SubmeshRange {
        first_index: pect_start,
        index_count: acc.indices.len() as u32 - pect_start,
        material_id: pect_mat.id,
        semantic_id: q_hash("urn:qualia:bio:submesh:pectoral_forelimb"),
    });

    // 6. Pelvic Girdle & EPIPUBIC BONES (Sculpted Ilium, Ischium, Pubis & Paired Ossa Epipubica)
    let pelvis_start = acc.indices.len() as u32;
    let ilium_l = [
        -half_width * 0.62,
        pelvis_pos[1] + 0.045,
        pelvis_pos[2] + 0.07,
    ];
    let ilium_r = [
        half_width * 0.62,
        pelvis_pos[1] + 0.045,
        pelvis_pos[2] + 0.07,
    ];
    let ischium_l = [
        -half_width * 0.52,
        pelvis_pos[1] - 0.035,
        pelvis_pos[2] - 0.075,
    ];
    let ischium_r = [
        half_width * 0.52,
        pelvis_pos[1] - 0.035,
        pelvis_pos[2] - 0.075,
    ];
    let pubis_pos = [0.0, pelvis_pos[1] - half_width * 0.45, pelvis_pos[2] + 0.04];

    for (il, isch, hip_j) in [
        (ilium_l, ischium_l, JOINT_LEFT_HIP),
        (ilium_r, ischium_r, JOINT_RIGHT_HIP),
    ] {
        // Ilium -> Acetabulum -> Ischium sculpted pelvic wing
        acc.push_long_bone_with_epiphyses(
            il,
            isch,
            half_width * 0.14,
            half_width * 0.21,
            [0.0, 0.012, 0.0],
            bone_ivory,
            ZoologicalAnatomicalLayer::Osteology,
            JOINT_PELVIS_ROOT,
            hip_j,
        );
        // Pubic ramus arch
        acc.push_long_bone_with_epiphyses(
            pubis_pos,
            il,
            half_width * 0.09,
            half_width * 0.13,
            [0.0, -0.010, 0.0],
            bone_dense,
            ZoologicalAnatomicalLayer::Osteology,
            JOINT_PELVIS_ROOT,
            hip_j,
        );
    }

    // ── EPIPUBIC BONES (Ossa Epipubica) ──
    let epipubic_len = half_width * 0.85;
    let epipubic_tip_l = [
        -half_width * 0.24,
        pubis_pos[1] + epipubic_len * 0.40,
        pubis_pos[2] + epipubic_len * 0.90,
    ];
    let epipubic_tip_r = [
        half_width * 0.24,
        pubis_pos[1] + epipubic_len * 0.40,
        pubis_pos[2] + epipubic_len * 0.90,
    ];

    acc.push_long_bone_with_epiphyses(
        [-0.02, pubis_pos[1], pubis_pos[2]],
        epipubic_tip_l,
        0.008,
        0.012,
        [-0.006, 0.005, 0.0],
        bone_ivory,
        ZoologicalAnatomicalLayer::Osteology,
        JOINT_PELVIS_ROOT,
        JOINT_SPINE,
    );
    acc.push_long_bone_with_epiphyses(
        [0.02, pubis_pos[1], pubis_pos[2]],
        epipubic_tip_r,
        0.008,
        0.012,
        [0.006, 0.005, 0.0],
        bone_ivory,
        ZoologicalAnatomicalLayer::Osteology,
        JOINT_PELVIS_ROOT,
        JOINT_SPINE,
    );

    let pelvis_mat = make_pbr_material(
        "mat_ost_pelvis_epipubic",
        [0.87, 0.85, 0.79, 1.0],
        0.44,
        0.02,
    );
    materials.push(pelvis_mat);
    submesh_ranges.push(SubmeshRange {
        first_index: pelvis_start,
        index_count: acc.indices.len() as u32 - pelvis_start,
        material_id: pelvis_mat.id,
        semantic_id: q_hash("urn:qualia:bio:submesh:pelvic_epipubic_bones"),
    });

    // 7. Hindlimb & Syndactyl Pes (Bicondylar Femur, Tibia/Fibula, Calcaneum & Syndactyl Digits)
    let hind_start = acc.indices.len() as u32;
    let hip_l = [-half_width * 0.65, pelvis_pos[1] - 0.02, pelvis_pos[2]];
    let hip_r = [half_width * 0.65, pelvis_pos[1] - 0.02, pelvis_pos[2]];
    let knee_l = [
        -half_width * 0.72,
        pelvis_pos[1] - femur_len,
        pelvis_pos[2] + femur_len * 0.45,
    ];
    let knee_r = [
        half_width * 0.72,
        pelvis_pos[1] - femur_len,
        pelvis_pos[2] + femur_len * 0.45,
    ];
    let ankle_l = [
        -half_width * 0.58,
        knee_l[1] - tibia_len,
        pelvis_pos[2] - 0.04,
    ];
    let ankle_r = [
        half_width * 0.58,
        knee_r[1] - tibia_len,
        pelvis_pos[2] - 0.04,
    ];
    let pes_tip_l = [ankle_l[0], ankle_l[1], ankle_l[2] + half_width * 1.4];
    let pes_tip_r = [ankle_r[0], ankle_r[1], ankle_r[2] + half_width * 1.4];

    for (side, hip_p, knee_p, ankle_p, pes_p, j_hip, j_knee, j_ankle) in [
        (
            -1.0_f32,
            hip_l,
            knee_l,
            ankle_l,
            pes_tip_l,
            JOINT_LEFT_HIP,
            JOINT_LEFT_KNEE,
            JOINT_LEFT_ANKLE,
        ),
        (
            1.0_f32,
            hip_r,
            knee_r,
            ankle_r,
            pes_tip_r,
            JOINT_RIGHT_HIP,
            JOINT_RIGHT_KNEE,
            JOINT_RIGHT_ANKLE,
        ),
    ] {
        // Femur with greater trochanter & distal condyles
        acc.push_long_bone_with_epiphyses(
            hip_p,
            knee_p,
            half_width * 0.14,
            half_width * 0.22,
            [side * 0.010, 0.012, 0.010],
            bone_ivory,
            ZoologicalAnatomicalLayer::Osteology,
            j_hip,
            j_knee,
        );
        // Tibia & Fibula with cnemial crest
        acc.push_long_bone_with_epiphyses(
            knee_p,
            ankle_p,
            half_width * 0.11,
            half_width * 0.17,
            [0.0, 0.0, -0.012],
            bone_ivory,
            ZoologicalAnatomicalLayer::Osteology,
            j_knee,
            j_ankle,
        );
        // Calcaneum heel lever
        acc.push_long_bone_with_epiphyses(
            ankle_p,
            [
                ankle_p[0],
                ankle_p[1] + 0.018,
                ankle_p[2] - half_width * 0.36,
            ],
            0.012,
            0.018,
            [0.0, 0.004, 0.0],
            bone_dense,
            ZoologicalAnatomicalLayer::Osteology,
            j_ankle,
            j_knee,
        );
        // Elongated 4th Metatarsal & Hopping Digit IV
        acc.push_long_bone_with_epiphyses(
            ankle_p,
            pes_p,
            half_width * 0.09,
            half_width * 0.13,
            [0.0, 0.006, 0.0],
            bone_ivory,
            ZoologicalAnatomicalLayer::Osteology,
            j_ankle,
            j_knee,
        );
        // Syndactyl grooming digits II & III (paired medial digits)
        acc.push_cylinder(
            [ankle_p[0] - side * 0.015, ankle_p[1], ankle_p[2] + 0.02],
            [pes_p[0] - side * 0.015, pes_p[1], pes_p[2] * 0.75],
            0.005,
            0.003,
            6,
            2,
            bone_ivory,
            ZoologicalAnatomicalLayer::Osteology,
            j_ankle,
            j_knee,
            0.95,
        );
    }

    let hind_mat = make_pbr_material("mat_ost_hindlimb_pes", bone_ivory, 0.42, 0.02);
    materials.push(hind_mat);
    submesh_ranges.push(SubmeshRange {
        first_index: hind_start,
        index_count: acc.indices.len() as u32 - hind_start,
        material_id: hind_mat.id,
        semantic_id: q_hash("urn:qualia:bio:submesh:hindlimb_syndactyl_pes"),
    });

    // 8. Caudal Vertebral Chain (Articulated Caudal Centra, Transverse Processes & Haemal Chevrons)
    let tail_start = acc.indices.len() as u32;
    let tail_base = [0.0, pelvis_pos[1] - 0.02, -spine_len * 0.1];
    let tail_mid = [0.0, pelvis_pos[1] - femur_len * 0.45, -tail_len * 0.55];
    let tail_tip = [0.0, ankle_l[1] + 0.02, -tail_len];

    let caudal_loft = [
        AnatomicalLoftRing {
            center: tail_base,
            radii: [half_width * 0.24, half_width * 0.22],
            dorsal_color: bone_ivory,
            flank_color: bone_dense,
            ventral_color: bone_dense,
            primary_joint: JOINT_TAIL_BASE,
            secondary_joint: JOINT_PELVIS_ROOT,
            primary_weight: 0.80,
        },
        AnatomicalLoftRing {
            center: tail_mid,
            radii: [half_width * 0.16, half_width * 0.15],
            dorsal_color: bone_ivory,
            flank_color: bone_dense,
            ventral_color: bone_dense,
            primary_joint: JOINT_TAIL_MID,
            secondary_joint: JOINT_TAIL_BASE,
            primary_weight: 0.80,
        },
        AnatomicalLoftRing {
            center: tail_tip,
            radii: [half_width * 0.05, half_width * 0.05],
            dorsal_color: bone_dense,
            flank_color: bone_dense,
            ventral_color: bone_dense,
            primary_joint: JOINT_TAIL_TIP,
            secondary_joint: JOINT_TAIL_MID,
            primary_weight: 0.90,
        },
    ];
    acc.push_countershaded_loft(&caudal_loft, 8, ZoologicalAnatomicalLayer::Osteology, true);

    for c in 1..=5 {
        let t = c as f32 / 6.0;
        let pos = [
            tail_base[0] + (tail_mid[0] - tail_base[0]) * t,
            tail_base[1] + (tail_mid[1] - tail_base[1]) * t - 0.014,
            tail_base[2] + (tail_mid[2] - tail_base[2]) * t,
        ];
        acc.push_cylinder(
            [pos[0], pos[1], pos[2]],
            [pos[0], pos[1] - 0.022 * (1.0 - t * 0.4), pos[2] - 0.008],
            0.006,
            0.003,
            6,
            2,
            bone_dense,
            ZoologicalAnatomicalLayer::Osteology,
            JOINT_TAIL_BASE,
            JOINT_TAIL_MID,
            1.0 - t * 0.5,
        );
    }

    if archetype == FaunaBiomechanicalArchetype::MarinePinnipedCetacean {
        acc.push_ellipsoid(
            tail_tip,
            [half_width * 0.72, 0.035, half_width * 0.24],
            12,
            6,
            bone_dense,
            ZoologicalAnatomicalLayer::Osteology,
            JOINT_TAIL_TIP,
            JOINT_TAIL_MID,
            0.8,
        );
    }

    let tail_mat = make_pbr_material("mat_ost_caudal", bone_dense, 0.48, 0.02);
    materials.push(tail_mat);
    submesh_ranges.push(SubmeshRange {
        first_index: tail_start,
        index_count: acc.indices.len() as u32 - tail_start,
        material_id: tail_mat.id,
        semantic_id: q_hash("urn:qualia:bio:submesh:caudal_vertebrae"),
    });

    let ost_end_v = acc.positions.len();

    // ── LAYER 1: MYOLOGY, TENDONS & VISCERA (8 Functional Submeshes) ─────────
    let myo_start_v = acc.positions.len();

    // 9. Gastrocnemius & Plantaris (Lateral & Medial Fusiform Calf Bellies)
    let gast_start = acc.indices.len() as u32;
    let gastro_center_l = [
        knee_l[0] * 1.02,
        (knee_l[1] + ankle_l[1]) * 0.58,
        knee_l[2] - half_width * 0.15,
    ];
    let gastro_center_r = [
        knee_r[0] * 1.02,
        (knee_r[1] + ankle_r[1]) * 0.58,
        knee_r[2] - half_width * 0.15,
    ];
    let myotendinous_l = [
        ankle_l[0],
        (knee_l[1] + ankle_l[1]) * 0.36,
        ankle_l[2] - half_width * 0.22,
    ];
    let myotendinous_r = [
        ankle_r[0],
        (knee_r[1] + ankle_r[1]) * 0.36,
        ankle_r[2] - half_width * 0.22,
    ];

    acc.push_fusiform_muscle(
        knee_l,
        myotendinous_l,
        half_width * 0.24,
        half_width * 0.22,
        [-0.012, 0.0, -half_width * 0.16],
        [0.58, 0.17, 0.17, 1.0],
        tendon_ivory,
        ZoologicalAnatomicalLayer::MyologyAndTendons,
        JOINT_LEFT_KNEE,
        JOINT_LEFT_ANKLE,
    );
    acc.push_fusiform_muscle(
        knee_r,
        myotendinous_r,
        half_width * 0.24,
        half_width * 0.22,
        [0.012, 0.0, -half_width * 0.16],
        [0.58, 0.17, 0.17, 1.0],
        tendon_ivory,
        ZoologicalAnatomicalLayer::MyologyAndTendons,
        JOINT_RIGHT_KNEE,
        JOINT_RIGHT_ANKLE,
    );

    let gast_mat = make_pbr_material("mat_myo_gastrocnemius", [0.55, 0.18, 0.18, 1.0], 0.65, 0.0);
    materials.push(gast_mat);
    submesh_ranges.push(SubmeshRange {
        first_index: gast_start,
        index_count: acc.indices.len() as u32 - gast_start,
        material_id: gast_mat.id,
        semantic_id: q_hash("urn:qualia:bio:submesh:gastrocnemius_plantaris"),
    });

    // 10. ELASTIC ACHILLES TENDON SPRING (Stores & Returns up to 65% Metabolic Work)
    let ach_start = acc.indices.len() as u32;
    let achilles_start_l = [
        gastro_center_l[0],
        gastro_center_l[1] - tibia_len * 0.22,
        gastro_center_l[2],
    ];
    let achilles_start_r = [
        gastro_center_r[0],
        gastro_center_r[1] - tibia_len * 0.22,
        gastro_center_r[2],
    ];
    let calcaneus_l = [
        ankle_l[0],
        ankle_l[1] + 0.012,
        ankle_l[2] - half_width * 0.30,
    ];
    let calcaneus_r = [
        ankle_r[0],
        ankle_r[1] + 0.012,
        ankle_r[2] - half_width * 0.30,
    ];
    let plantar_l = [
        ankle_l[0],
        ankle_l[1] - 0.008,
        ankle_l[2] + half_width * 0.90,
    ];
    let plantar_r = [
        ankle_r[0],
        ankle_r[1] - 0.008,
        ankle_r[2] + half_width * 0.90,
    ];

    for (a_start, calc_p, plant_p, j_ank, j_kn) in [
        (
            achilles_start_l,
            calcaneus_l,
            plantar_l,
            JOINT_LEFT_ANKLE,
            JOINT_LEFT_KNEE,
        ),
        (
            achilles_start_r,
            calcaneus_r,
            plantar_r,
            JOINT_RIGHT_ANKLE,
            JOINT_RIGHT_KNEE,
        ),
    ] {
        let tendon_rings = [
            AnatomicalLoftRing {
                center: a_start,
                radii: [half_width * 0.068, half_width * 0.055],
                dorsal_color: tendon_ivory,
                flank_color: tendon_ivory,
                ventral_color: tendon_ivory,
                primary_joint: j_kn,
                secondary_joint: j_ank,
                primary_weight: 0.65,
            },
            AnatomicalLoftRing {
                center: calc_p,
                radii: [half_width * 0.056, half_width * 0.048],
                dorsal_color: tendon_ivory,
                flank_color: tendon_ivory,
                ventral_color: tendon_ivory,
                primary_joint: j_ank,
                secondary_joint: j_kn,
                primary_weight: 0.90,
            },
            AnatomicalLoftRing {
                center: plant_p,
                radii: [half_width * 0.048, half_width * 0.028],
                dorsal_color: tendon_ivory,
                flank_color: tendon_ivory,
                ventral_color: tendon_ivory,
                primary_joint: j_ank,
                secondary_joint: j_kn,
                primary_weight: 0.95,
            },
        ];
        acc.push_countershaded_loft(
            &tendon_rings,
            8,
            ZoologicalAnatomicalLayer::MyologyAndTendons,
            true,
        );
    }

    let ach_mat = make_pbr_material("mat_myo_achilles_tendon", tendon_ivory, 0.22, 0.08);
    materials.push(ach_mat);
    submesh_ranges.push(SubmeshRange {
        first_index: ach_start,
        index_count: acc.indices.len() as u32 - ach_start,
        material_id: ach_mat.id,
        semantic_id: q_hash("urn:qualia:bio:submesh:achilles_tendon_spring"),
    });

    // 11. Gluteus, Biceps Femoris & Quadriceps Vastus Lateralis (Fusiform Thigh Extensors)
    let glut_start = acc.indices.len() as u32;
    for (side, il_p, isch_p, hip_p, kn_p, j_hip, j_kn) in [
        (
            -1.0_f32,
            ilium_l,
            ischium_l,
            hip_l,
            knee_l,
            JOINT_LEFT_HIP,
            JOINT_LEFT_KNEE,
        ),
        (
            1.0_f32,
            ilium_r,
            ischium_r,
            hip_r,
            knee_r,
            JOINT_RIGHT_HIP,
            JOINT_RIGHT_KNEE,
        ),
    ] {
        // Biceps femoris & gluteus superficialis
        acc.push_fusiform_muscle(
            isch_p,
            kn_p,
            half_width * 0.28,
            half_width * 0.32,
            [side * half_width * 0.14, 0.02, -0.02],
            [0.54, 0.16, 0.16, 1.0],
            tendon_ivory,
            ZoologicalAnatomicalLayer::MyologyAndTendons,
            j_hip,
            j_kn,
        );
        // Quadriceps vastus lateralis & rectus femoris
        acc.push_fusiform_muscle(
            il_p,
            kn_p,
            half_width * 0.24,
            half_width * 0.26,
            [side * half_width * 0.10, 0.03, 0.03],
            [0.58, 0.19, 0.18, 1.0],
            tendon_ivory,
            ZoologicalAnatomicalLayer::MyologyAndTendons,
            j_hip,
            j_kn,
        );
        let _ = hip_p;
    }

    let glut_mat = make_pbr_material("mat_myo_gluteus", [0.52, 0.16, 0.16, 1.0], 0.62, 0.0);
    materials.push(glut_mat);
    submesh_ranges.push(SubmeshRange {
        first_index: glut_start,
        index_count: acc.indices.len() as u32 - glut_start,
        material_id: glut_mat.id,
        semantic_id: q_hash("urn:qualia:bio:submesh:gluteus_biceps_femoris"),
    });

    // 12. Iliopsoas, Caudofemoralis & Epaxial/Hypaxial Spinal/Caudal Bundles
    let ilio_start = acc.indices.len() as u32;
    for (side, hip_p, j_hip) in [
        (-1.0_f32, hip_l, JOINT_LEFT_HIP),
        (1.0_f32, hip_r, JOINT_RIGHT_HIP),
    ] {
        // Caudofemoralis (tail base to femur retractor)
        acc.push_fusiform_muscle(
            tail_mid,
            hip_p,
            half_width * 0.20,
            half_width * 0.22,
            [side * half_width * 0.12, -0.01, 0.0],
            [0.50, 0.15, 0.15, 1.0],
            tendon_ivory,
            ZoologicalAnatomicalLayer::MyologyAndTendons,
            JOINT_TAIL_BASE,
            j_hip,
        );
        // Dorsal epaxial (longissimus dorsi) muscle bundle spanning pelvis to chest
        acc.push_fusiform_muscle(
            [
                side * half_width * 0.22,
                pelvis_pos[1] + 0.03,
                pelvis_pos[2],
            ],
            [side * half_width * 0.20, chest_pos[1] + 0.03, chest_pos[2]],
            half_width * 0.18,
            half_width * 0.16,
            [side * 0.01, 0.02, 0.0],
            [0.53, 0.16, 0.16, 1.0],
            tendon_ivory,
            ZoologicalAnatomicalLayer::MyologyAndTendons,
            JOINT_SPINE,
            JOINT_CHEST,
        );
    }

    let ilio_mat = make_pbr_material("mat_myo_iliopsoas", [0.50, 0.15, 0.15, 1.0], 0.60, 0.0);
    materials.push(ilio_mat);
    submesh_ranges.push(SubmeshRange {
        first_index: ilio_start,
        index_count: acc.indices.len() as u32 - ilio_start,
        material_id: ilio_mat.id,
        semantic_id: q_hash("urn:qualia:bio:submesh:iliopsoas_caudofemoralis"),
    });

    // 13. Pectoralis Major, Supracoracoideus & Latissimus Dorsi (Chest & Back Fan Muscles)
    let pect_myo_start = acc.indices.len() as u32;
    for (side, sh_p) in [(-1.0_f32, shoulder_l), (1.0_f32, shoulder_r)] {
        // Pectoralis major fan from sternum/carina to humerus deltopectoral crest
        acc.push_fusiform_muscle(
            [0.0, sternum_pos[1], sternum_pos[2]],
            sh_p,
            half_width * 0.30,
            half_width * 0.24,
            [side * half_width * 0.12, -0.02, 0.03],
            [0.58, 0.20, 0.20, 1.0],
            tendon_ivory,
            ZoologicalAnatomicalLayer::MyologyAndTendons,
            JOINT_CHEST,
            JOINT_LEFT_SHOULDER,
        );
        // Latissimus dorsi from mid-thoracic fascia to proximal humerus
        acc.push_fusiform_muscle(
            [
                0.0,
                (pelvis_pos[1] + chest_pos[1]) * 0.52,
                (pelvis_pos[2] + chest_pos[2]) * 0.45,
            ],
            sh_p,
            half_width * 0.20,
            half_width * 0.24,
            [side * half_width * 0.26, 0.0, 0.0],
            [0.54, 0.18, 0.18, 1.0],
            tendon_ivory,
            ZoologicalAnatomicalLayer::MyologyAndTendons,
            JOINT_CHEST,
            JOINT_SPINE,
        );
    }

    let pect_myo_mat = make_pbr_material("mat_myo_pectoralis", [0.58, 0.20, 0.20, 1.0], 0.64, 0.0);
    materials.push(pect_myo_mat);
    submesh_ranges.push(SubmeshRange {
        first_index: pect_myo_start,
        index_count: acc.indices.len() as u32 - pect_myo_start,
        material_id: pect_myo_mat.id,
        semantic_id: q_hash("urn:qualia:bio:submesh:pectoralis_supracoracoideus"),
    });

    // 14. Deltoid, Triceps Brachii & Biceps Brachii (Fossorial/Brachial Musculature)
    let delt_start = acc.indices.len() as u32;
    for (side, sh_p, el_p, wr_p) in [
        (-1.0_f32, shoulder_l, elbow_l, wrist_l),
        (1.0_f32, shoulder_r, elbow_r, wrist_r),
    ] {
        // Deltoid & Triceps brachii
        acc.push_fusiform_muscle(
            sh_p,
            el_p,
            half_width * 0.19,
            half_width * 0.21,
            [side * 0.015, 0.0, -0.018],
            [0.54, 0.17, 0.17, 1.0],
            tendon_ivory,
            ZoologicalAnatomicalLayer::MyologyAndTendons,
            JOINT_LEFT_SHOULDER,
            JOINT_LEFT_ELBOW,
        );
        // Antebrachial flexors/extensors
        acc.push_fusiform_muscle(
            el_p,
            wr_p,
            half_width * 0.13,
            half_width * 0.14,
            [side * 0.008, 0.0, 0.010],
            [0.52, 0.16, 0.16, 1.0],
            tendon_ivory,
            ZoologicalAnatomicalLayer::MyologyAndTendons,
            JOINT_LEFT_ELBOW,
            JOINT_LEFT_WRIST_WING,
        );
    }

    let delt_mat = make_pbr_material("mat_myo_deltoid", [0.54, 0.17, 0.17, 1.0], 0.63, 0.0);
    materials.push(delt_mat);
    submesh_ranges.push(SubmeshRange {
        first_index: delt_start,
        index_count: acc.indices.len() as u32 - delt_start,
        material_id: delt_mat.id,
        semantic_id: q_hash("urn:qualia:bio:submesh:deltoid_triceps"),
    });

    // 15. Masseter, Temporalis & Cervical Splenius/Sternocleidomastoid
    let mass_start = acc.indices.len() as u32;
    for (side, jaw_j) in [(-1.0_f32, jaw_joint_l), (1.0_f32, jaw_joint_r)] {
        acc.push_fusiform_muscle(
            [
                side * half_width * 0.42,
                cranium_head_pos[1] + 0.01,
                cranium_head_pos[2] + head_len * 0.12,
            ],
            [
                jaw_j[0] * 0.75,
                chin_pos[1] + 0.01,
                (jaw_j[2] + chin_pos[2]) * 0.5,
            ],
            half_width * 0.15,
            half_width * 0.20,
            [side * 0.012, 0.0, 0.0],
            [0.48, 0.16, 0.16, 1.0],
            tendon_ivory,
            ZoologicalAnatomicalLayer::MyologyAndTendons,
            JOINT_HEAD,
            JOINT_JAW,
        );
        // Splenius / sternocleidomastoid neck strap
        acc.push_fusiform_muscle(
            [side * half_width * 0.24, chest_pos[1], chest_pos[2]],
            [
                side * half_width * 0.30,
                cranium_head_pos[1],
                cranium_head_pos[2] - 0.02,
            ],
            half_width * 0.15,
            half_width * 0.16,
            [side * 0.01, 0.01, 0.0],
            [0.52, 0.17, 0.17, 1.0],
            tendon_ivory,
            ZoologicalAnatomicalLayer::MyologyAndTendons,
            JOINT_NECK,
            JOINT_HEAD,
        );
    }

    let mass_mat = make_pbr_material("mat_myo_masseter", [0.48, 0.16, 0.16, 1.0], 0.62, 0.0);
    materials.push(mass_mat);
    submesh_ranges.push(SubmeshRange {
        first_index: mass_start,
        index_count: acc.indices.len() as u32 - mass_start,
        material_id: mass_mat.id,
        semantic_id: q_hash("urn:qualia:bio:submesh:masseter_temporalis"),
    });

    // 16. Intercostals, Diaphragm & Internal VISCERA (Bilobed Lungs, Bronchial Tree,
    //     4-Chambered Heart with Aortic Arch, Lobulated Liver, Stomach & Coiled Intestines)
    let inter_start = acc.indices.len() as u32;
    let thorax_y = (chest_pos[1] + sternum_pos[1]) * 0.52;
    let thorax_z = chest_pos[2] * 0.78;

    // Cartilaginous Trachea & Bifurcating Primary Bronchi
    acc.push_cylinder(
        [0.0, cranium_head_pos[1] - 0.05, cranium_head_pos[2] - 0.02],
        [0.0, thorax_y + 0.03, thorax_z + spine_len * 0.14],
        0.012,
        0.012,
        8,
        4,
        cartilage_blue,
        ZoologicalAnatomicalLayer::MyologyAndTendons,
        JOINT_NECK,
        JOINT_CHEST,
        0.65,
    );

    // Bilobed Left & Right Lungs with Primary Bronchial Branches
    let lung_coral = [0.82, 0.44, 0.46, 1.0];
    for side in [-1.0_f32, 1.0_f32] {
        let lung_c = [side * half_width * 0.34, thorax_y + 0.01, thorax_z];
        acc.push_cylinder(
            [0.0, thorax_y + 0.03, thorax_z + spine_len * 0.14],
            lung_c,
            0.009,
            0.007,
            6,
            2,
            cartilage_blue,
            ZoologicalAnatomicalLayer::MyologyAndTendons,
            JOINT_CHEST,
            JOINT_SPINE,
            0.85,
        );
        // Cranial & caudal pulmonary lobes
        acc.push_ellipsoid(
            [lung_c[0], lung_c[1] + 0.01, lung_c[2] + spine_len * 0.08],
            [half_width * 0.24, half_width * 0.26, spine_len * 0.16],
            10,
            6,
            lung_coral,
            ZoologicalAnatomicalLayer::MyologyAndTendons,
            JOINT_CHEST,
            JOINT_SPINE,
            0.85,
        );
        acc.push_ellipsoid(
            [
                lung_c[0] * 1.05,
                lung_c[1] - 0.01,
                lung_c[2] - spine_len * 0.09,
            ],
            [half_width * 0.27, half_width * 0.30, spine_len * 0.20],
            10,
            6,
            lung_coral,
            ZoologicalAnatomicalLayer::MyologyAndTendons,
            JOINT_CHEST,
            JOINT_SPINE,
            0.80,
        );
    }

    // 4-Chambered Heart & Systemic Aortic Arch
    let heart_pos = [
        -half_width * 0.06,
        sternum_pos[1] + half_width * 0.26,
        thorax_z + 0.02,
    ];
    acc.push_ellipsoid(
        heart_pos,
        [half_width * 0.18, half_width * 0.22, half_width * 0.19],
        10,
        6,
        [0.68, 0.12, 0.16, 1.0],
        ZoologicalAnatomicalLayer::MyologyAndTendons,
        JOINT_CHEST,
        JOINT_SPINE,
        0.90,
    );
    // Curving Aortic Arch
    acc.push_curved_rib_arch(
        [heart_pos[0], heart_pos[1] + half_width * 0.16, heart_pos[2]],
        [0.0, chest_pos[1] - 0.02, thorax_z - spine_len * 0.20],
        0.022,
        0.009,
        [0.78, 0.16, 0.18, 1.0],
        [0.72, 0.14, 0.16, 1.0],
        ZoologicalAnatomicalLayer::MyologyAndTendons,
        JOINT_CHEST,
        JOINT_SPINE,
    );

    // Lobulated Liver (caudal to diaphragm)
    let liver_pos = [half_width * 0.10, thorax_y - 0.02, spine_len * 0.42];
    acc.push_ellipsoid(
        liver_pos,
        [half_width * 0.36, half_width * 0.26, spine_len * 0.16],
        10,
        6,
        [0.44, 0.15, 0.14, 1.0],
        ZoologicalAnatomicalLayer::MyologyAndTendons,
        JOINT_SPINE,
        JOINT_CHEST,
        0.75,
    );

    // Sacciform Stomach
    let stomach_pos = [-half_width * 0.18, thorax_y - 0.04, spine_len * 0.30];
    acc.push_ellipsoid(
        stomach_pos,
        [half_width * 0.26, half_width * 0.22, spine_len * 0.18],
        10,
        6,
        [0.74, 0.50, 0.42, 1.0],
        ZoologicalAnatomicalLayer::MyologyAndTendons,
        JOINT_SPINE,
        JOINT_PELVIS_ROOT,
        0.75,
    );

    // Coiled Small & Large Intestinal Tract
    let gut_pink = [0.78, 0.56, 0.48, 1.0];
    let mut gut_rings = [AnatomicalLoftRing {
        center: stomach_pos,
        radii: [0.018, 0.018],
        dorsal_color: gut_pink,
        flank_color: gut_pink,
        ventral_color: gut_pink,
        primary_joint: JOINT_SPINE,
        secondary_joint: JOINT_PELVIS_ROOT,
        primary_weight: 0.70,
    }; 8];
    for (gi, ring) in gut_rings.iter_mut().enumerate() {
        let gt = gi as f32 / 7.0;
        let coil_angle = gt * core::f32::consts::TAU * 2.0;
        *ring = AnatomicalLoftRing {
            center: [
                coil_angle.cos() * half_width * 0.24,
                pelvis_pos[1] - half_width * 0.18 + coil_angle.sin() * half_width * 0.12,
                spine_len * (0.28 - gt * 0.24),
            ],
            radii: [half_width * 0.13, half_width * 0.13],
            dorsal_color: gut_pink,
            flank_color: gut_pink,
            ventral_color: [0.70, 0.48, 0.42, 1.0],
            primary_joint: JOINT_SPINE,
            secondary_joint: JOINT_PELVIS_ROOT,
            primary_weight: 0.65,
        };
    }
    acc.push_countershaded_loft(
        &gut_rings,
        8,
        ZoologicalAnatomicalLayer::MyologyAndTendons,
        true,
    );

    let inter_mat = make_pbr_material("mat_myo_intercostals", [0.46, 0.15, 0.15, 1.0], 0.68, 0.0);
    materials.push(inter_mat);
    submesh_ranges.push(SubmeshRange {
        first_index: inter_start,
        index_count: acc.indices.len() as u32 - inter_start,
        material_id: inter_mat.id,
        semantic_id: q_hash("urn:qualia:bio:submesh:intercostals_diaphragm"),
    });

    let myo_end_v = acc.positions.len();

    // ── LAYER 2: EXTERNAL INTEGUMENT (Multi-Station Countershaded Pelage / Plumage / Skin) ──
    let int_start_v = acc.positions.len();
    let int_start_idx = acc.indices.len() as u32;

    let integument_color = match archetype {
        FaunaBiomechanicalArchetype::MacropodSaltatorial => [0.48, 0.40, 0.32, 1.0],
        FaunaBiomechanicalArchetype::VombatidFossorial => [0.34, 0.28, 0.22, 1.0],
        FaunaBiomechanicalArchetype::PhascolarctidArboreal => [0.55, 0.52, 0.48, 1.0],
        FaunaBiomechanicalArchetype::PetauridGlider => [0.60, 0.58, 0.56, 1.0],
        FaunaBiomechanicalArchetype::DasyuridPredator => [0.42, 0.30, 0.22, 1.0],
        FaunaBiomechanicalArchetype::PteropodidChiroptera => [0.22, 0.18, 0.16, 1.0],
        FaunaBiomechanicalArchetype::MarinePinnipedCetacean => [0.32, 0.36, 0.42, 1.0],
        FaunaBiomechanicalArchetype::RatiteBipedal => [0.18, 0.17, 0.18, 1.0],
        FaunaBiomechanicalArchetype::RaptorSoaring => [0.28, 0.22, 0.16, 1.0],
        FaunaBiomechanicalArchetype::PasserinePsittacine => [0.38, 0.52, 0.28, 1.0],
        FaunaBiomechanicalArchetype::SquamateSprawling => [0.44, 0.42, 0.34, 1.0],
        FaunaBiomechanicalArchetype::SerpentineUndulatory => [0.20, 0.18, 0.20, 1.0],
        FaunaBiomechanicalArchetype::MonotremeTeleostAmphibian => [0.36, 0.26, 0.20, 1.0],
    };
    let flank_tint: [f32; 4] = [
        (integument_color[0] * 1.22_f32).min(0.92),
        (integument_color[1] * 1.20_f32).min(0.90),
        (integument_color[2] * 1.18_f32).min(0.86),
        1.0,
    ];
    let ventral_cream: [f32; 4] = [
        (integument_color[0] * 0.55_f32 + 0.42).min(0.95),
        (integument_color[1] * 0.55_f32 + 0.40).min(0.93),
        (integument_color[2] * 0.52_f32 + 0.36).min(0.89),
        1.0,
    ];
    let charcoal_dark = [0.14, 0.12, 0.12, 1.0];

    // Multi-Station Continuous Head -> Neck -> Thorax -> Abdomen -> Pelvis -> Tail Integument Loft
    let axial_integument_loft = [
        AnatomicalLoftRing {
            center: tail_tip,
            radii: [half_width * 0.10, half_width * 0.10],
            dorsal_color: charcoal_dark,
            flank_color: integument_color,
            ventral_color: integument_color,
            primary_joint: JOINT_TAIL_TIP,
            secondary_joint: JOINT_TAIL_MID,
            primary_weight: 0.90,
        },
        AnatomicalLoftRing {
            center: tail_mid,
            radii: [half_width * 0.24, half_width * 0.22],
            dorsal_color: integument_color,
            flank_color: flank_tint,
            ventral_color: ventral_cream,
            primary_joint: JOINT_TAIL_MID,
            secondary_joint: JOINT_TAIL_BASE,
            primary_weight: 0.80,
        },
        AnatomicalLoftRing {
            center: tail_base,
            radii: [half_width * 0.38, half_width * 0.36],
            dorsal_color: integument_color,
            flank_color: flank_tint,
            ventral_color: ventral_cream,
            primary_joint: JOINT_TAIL_BASE,
            secondary_joint: JOINT_PELVIS_ROOT,
            primary_weight: 0.75,
        },
        AnatomicalLoftRing {
            center: [0.0, pelvis_pos[1] + 0.01, pelvis_pos[2] + spine_len * 0.08],
            radii: [half_width * 0.96, half_width * 0.90],
            dorsal_color: integument_color,
            flank_color: flank_tint,
            ventral_color: ventral_cream,
            primary_joint: JOINT_PELVIS_ROOT,
            secondary_joint: JOINT_SPINE,
            primary_weight: 0.75,
        },
        AnatomicalLoftRing {
            center: [0.0, hip_height + 0.03, spine_len * 0.42],
            radii: [half_width * 0.94, half_width * 0.88],
            dorsal_color: integument_color,
            flank_color: flank_tint,
            ventral_color: ventral_cream,
            primary_joint: JOINT_SPINE,
            secondary_joint: JOINT_CHEST,
            primary_weight: 0.65,
        },
        AnatomicalLoftRing {
            center: [0.0, chest_pos[1], chest_pos[2]],
            radii: [half_width * 0.82, half_width * 0.78],
            dorsal_color: integument_color,
            flank_color: flank_tint,
            ventral_color: ventral_cream,
            primary_joint: JOINT_CHEST,
            secondary_joint: JOINT_NECK,
            primary_weight: 0.75,
        },
        AnatomicalLoftRing {
            center: [
                0.0,
                (chest_pos[1] + cranium_head_pos[1]) * 0.52,
                (chest_pos[2] + cranium_head_pos[2]) * 0.52,
            ],
            radii: [half_width * 0.46, half_width * 0.44],
            dorsal_color: integument_color,
            flank_color: flank_tint,
            ventral_color: ventral_cream,
            primary_joint: JOINT_NECK,
            secondary_joint: JOINT_HEAD,
            primary_weight: 0.75,
        },
        AnatomicalLoftRing {
            center: cranium_head_pos,
            radii: [half_width * 0.62, half_width * 0.56],
            dorsal_color: integument_color,
            flank_color: flank_tint,
            ventral_color: ventral_cream,
            primary_joint: JOINT_HEAD,
            secondary_joint: JOINT_NECK,
            primary_weight: 0.88,
        },
        AnatomicalLoftRing {
            center: [
                0.0,
                cranium_head_pos[1] - 0.02,
                cranium_head_pos[2] + head_len * 0.55,
            ],
            radii: [half_width * 0.38, half_width * 0.32],
            dorsal_color: integument_color,
            flank_color: flank_tint,
            ventral_color: ventral_cream,
            primary_joint: JOINT_HEAD,
            secondary_joint: JOINT_JAW,
            primary_weight: 0.90,
        },
        AnatomicalLoftRing {
            center: [rostrum_tip[0], rostrum_tip[1], rostrum_tip[2] + 0.015],
            radii: [rostrum_rx * 1.15, rostrum_ry * 1.15],
            dorsal_color: charcoal_dark,
            flank_color: charcoal_dark,
            ventral_color: charcoal_dark,
            primary_joint: JOINT_HEAD,
            secondary_joint: JOINT_JAW,
            primary_weight: 0.95,
        },
    ];
    acc.push_countershaded_loft(
        &axial_integument_loft,
        16,
        ZoologicalAnatomicalLayer::ExternalIntegument,
        true,
    );

    // Multi-Station Countershaded Hindlimbs & Forelimbs Integument
    for (hip_p, knee_p, ankle_p, pes_p, j_hip, j_knee, j_ankle) in [
        (
            hip_l,
            knee_l,
            ankle_l,
            pes_tip_l,
            JOINT_LEFT_HIP,
            JOINT_LEFT_KNEE,
            JOINT_LEFT_ANKLE,
        ),
        (
            hip_r,
            knee_r,
            ankle_r,
            pes_tip_r,
            JOINT_RIGHT_HIP,
            JOINT_RIGHT_KNEE,
            JOINT_RIGHT_ANKLE,
        ),
    ] {
        let hind_loft = [
            AnatomicalLoftRing {
                center: hip_p,
                radii: [half_width * 0.42, half_width * 0.44],
                dorsal_color: integument_color,
                flank_color: flank_tint,
                ventral_color: ventral_cream,
                primary_joint: j_hip,
                secondary_joint: JOINT_PELVIS_ROOT,
                primary_weight: 0.80,
            },
            AnatomicalLoftRing {
                center: knee_p,
                radii: [half_width * 0.28, half_width * 0.30],
                dorsal_color: integument_color,
                flank_color: flank_tint,
                ventral_color: ventral_cream,
                primary_joint: j_knee,
                secondary_joint: j_hip,
                primary_weight: 0.80,
            },
            AnatomicalLoftRing {
                center: ankle_p,
                radii: [half_width * 0.16, half_width * 0.18],
                dorsal_color: integument_color,
                flank_color: flank_tint,
                ventral_color: charcoal_dark,
                primary_joint: j_ankle,
                secondary_joint: j_knee,
                primary_weight: 0.85,
            },
            AnatomicalLoftRing {
                center: pes_p,
                radii: [half_width * 0.11, half_width * 0.08],
                dorsal_color: charcoal_dark,
                flank_color: charcoal_dark,
                ventral_color: charcoal_dark,
                primary_joint: j_ankle,
                secondary_joint: j_knee,
                primary_weight: 0.95,
            },
        ];
        acc.push_countershaded_loft(
            &hind_loft,
            10,
            ZoologicalAnatomicalLayer::ExternalIntegument,
            true,
        );
    }

    for (sh_p, el_p, wr_p) in [
        (shoulder_l, elbow_l, wrist_l),
        (shoulder_r, elbow_r, wrist_r),
    ] {
        let fore_loft = [
            AnatomicalLoftRing {
                center: sh_p,
                radii: [half_width * 0.26, half_width * 0.28],
                dorsal_color: integument_color,
                flank_color: flank_tint,
                ventral_color: ventral_cream,
                primary_joint: JOINT_LEFT_SHOULDER,
                secondary_joint: JOINT_CHEST,
                primary_weight: 0.80,
            },
            AnatomicalLoftRing {
                center: el_p,
                radii: [half_width * 0.19, half_width * 0.20],
                dorsal_color: integument_color,
                flank_color: flank_tint,
                ventral_color: ventral_cream,
                primary_joint: JOINT_LEFT_ELBOW,
                secondary_joint: JOINT_LEFT_SHOULDER,
                primary_weight: 0.80,
            },
            AnatomicalLoftRing {
                center: wr_p,
                radii: [half_width * 0.12, half_width * 0.10],
                dorsal_color: charcoal_dark,
                flank_color: charcoal_dark,
                ventral_color: charcoal_dark,
                primary_joint: JOINT_LEFT_WRIST_WING,
                secondary_joint: JOINT_LEFT_ELBOW,
                primary_weight: 0.90,
            },
        ];
        acc.push_countershaded_loft(
            &fore_loft,
            8,
            ZoologicalAnatomicalLayer::ExternalIntegument,
            true,
        );
    }

    // Patagial sheet for gliders and bats
    if morphology.patagium > 0.0 {
        for side in [-1.0f32, 1.0] {
            acc.push_ellipsoid(
                [
                    side * half_width * 0.72,
                    hip_height + femur_len * 0.15,
                    spine_len * 0.30,
                ],
                [half_width * morphology.patagium, 0.012, spine_len * 0.42],
                12,
                8,
                integument_color,
                ZoologicalAnatomicalLayer::ExternalIntegument,
                JOINT_LEFT_SHOULDER,
                JOINT_PELVIS_ROOT,
                0.55,
            );
        }
    }

    // Flight feathers for avian flight archetypes
    if matches!(
        archetype,
        FaunaBiomechanicalArchetype::RaptorSoaring
            | FaunaBiomechanicalArchetype::PasserinePsittacine
    ) {
        for side in [-1.0f32, 1.0] {
            for feather in 0..6 {
                let t = feather as f32 / 5.0;
                let wrist = if side < 0.0 { wrist_l } else { wrist_r };
                let elbow = if side < 0.0 { elbow_l } else { elbow_r };
                let base = [
                    wrist[0] * (1.0 - t * 0.35) + elbow[0] * t * 0.35,
                    wrist[1] - 0.015,
                    wrist[2] - body_len * t * 0.05,
                ];
                let tip = [
                    side * half_width * morphology.shoulder_span * (1.35 + t * 0.48),
                    wrist[1] - 0.025,
                    wrist[2] - body_len * (0.10 + t * 0.16),
                ];
                acc.push_cylinder(
                    base,
                    tip,
                    0.012,
                    0.0015,
                    6,
                    2,
                    integument_color,
                    ZoologicalAnatomicalLayer::ExternalIntegument,
                    JOINT_LEFT_WRIST_WING,
                    JOINT_LEFT_ELBOW,
                    0.8,
                );
            }
        }
    }

    let int_mat = make_pbr_material("mat_integument", integument_color, 0.82, 0.0);
    materials.push(int_mat);
    submesh_ranges.push(SubmeshRange {
        first_index: int_start_idx,
        index_count: acc.indices.len() as u32 - int_start_idx,
        material_id: int_mat.id,
        semantic_id: q_hash("urn:qualia:bio:submesh:external_integument"),
    });

    let int_end_v = acc.positions.len();

    finalize_and_seal_vertebrate_anatomy_10d(
        archetype,
        acc,
        materials,
        submesh_ranges,
        ost_start_v..ost_end_v,
        myo_start_v..myo_end_v,
        int_start_v..int_end_v,
        osteology_vis_01,
        myology_vis_01,
        integument_vis_01,
        explode_layer_01,
    )
}

/// Finalizes an [`AnatomyMeshAccumulator`] built from either a portable model-family scaffold
/// or a baseline archetype, applies zero-heap layer peeling & exploded-view displacement,
/// and seals the 9-section `.10d` Living Container (`MAT4`, `UV01`, `FNRM`, `SRD1`, `PROC1`,
/// `EONT1`, `MRP1`, `PROV`) with whole-file CRC-32C verification.
#[allow(clippy::too_many_arguments)]
pub fn finalize_and_seal_vertebrate_anatomy_10d(
    archetype: FaunaBiomechanicalArchetype,
    mut acc: AnatomyMeshAccumulator,
    materials: Vec<MaterialRecord>,
    submesh_ranges: Vec<SubmeshRange>,
    osteology_range: std::ops::Range<usize>,
    myology_range: std::ops::Range<usize>,
    integument_range: std::ops::Range<usize>,
    osteology_vis_01: f32,
    myology_vis_01: f32,
    integument_vis_01: f32,
    explode_layer_01: f32,
) -> CompiledVertebrateAnatomy10d {
    let rig = FaunaSkeletalRig10d::new(archetype);
    let (base_body_len, hip_height) = archetype.default_dimensions();
    let morphology = ArchetypeMorphology::for_archetype(archetype);
    let body_len = base_body_len * morphology.trunk;
    let spine_len = (body_len * 0.32).max(0.10);
    let femur_len = (hip_height * 0.48 * morphology.hindlimb).max(0.025);
    let (ost_start_v, ost_end_v) = (osteology_range.start, osteology_range.end);
    let (myo_start_v, myo_end_v) = (myology_range.start, myology_range.end);
    let (int_start_v, int_end_v) = (integument_range.start, integument_range.end);

    // Ensure all positions and normals are strictly finite and unit-length for .10d FNRM validation
    for p in &mut acc.positions {
        for k in 0..3 {
            if !p[k].is_finite() {
                p[k] = 0.0;
            }
        }
    }
    for n in &mut acc.normals {
        let len_sq = n[0] * n[0] + n[1] * n[1] + n[2] * n[2];
        if len_sq.is_finite() && len_sq > 1e-8 {
            let inv = 1.0 / len_sq.sqrt();
            n[0] *= inv;
            n[1] *= inv;
            n[2] *= inv;
        } else {
            *n = [0.0, 1.0, 0.0];
        }
    }

    // ── Apply Layer Visibility & Exploded View Displacements ─────────────────
    let total_v = acc.positions.len();
    let mut eval_positions = acc.positions.clone();
    let mut eval_alphas = vec![1.0f32; total_v];

    evaluate_anatomical_layer_peel_zero_heap(
        &acc.positions,
        &acc.normals,
        &acc.layer_tags,
        osteology_vis_01,
        myology_vis_01,
        integument_vis_01,
        explode_layer_01,
        &mut eval_positions,
        &mut eval_alphas,
    );

    // Merge computed alpha into RGBA colors
    for i in 0..total_v {
        acc.colors[i][3] = eval_alphas[i];
    }

    // Compute Mesh AABB
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in &eval_positions {
        for k in 0..3 {
            min[k] = min[k].min(p[k]);
            max[k] = max[k].max(p[k]);
        }
    }

    let mut triangles = Vec::with_capacity(acc.indices.len() / 3);
    for tri in acc.indices.chunks_exact(3) {
        triangles.push([tri[0], tri[1], tri[2]]);
    }

    let mesh = Mesh {
        positions: eval_positions.clone(),
        triangles,
        min,
        max,
    };

    // ── Assemble & Seal .10d Container ───────────────────────────────────────
    let provenance = ProvenanceSidecar::new(
        format!("urn:qualia:fauna:scale5:{}", archetype.as_str()).into_bytes(),
        "application/vnd.ecopunk.fauna-anatomy;version=1",
        "Apache-2.0",
    );

    let mut bytes = compile_mesh_to_10d_with_extras_and_frames(
        &mesh,
        &[],
        Some(&provenance),
        Compile10dExtras::default(),
        Some(&acc.colors),
        Some(&acc.normals),
        None,
    )
    .expect("compile_mesh_to_10d_with_extras_and_frames failed for vertebrate anatomy");

    // MAT4 Material Section (materials must be sorted by id in ascending order)
    let mut sorted_materials = materials.clone();
    sorted_materials.sort_by_key(|m| m.id);
    sorted_materials.dedup_by_key(|m| m.id);
    bytes = attach_material_section(
        &bytes,
        &sorted_materials,
        &submesh_ranges,
        acc.indices.len() as u32,
    )
    .expect("attach_material_section failed");

    // UV01 Texture Coordinate Section
    bytes = attach_texture_coordinate_section(&bytes, &acc.uvs)
        .expect("attach_texture_coordinate_section failed");

    // PROC1 Procedural Synthesis Section
    let proc_header = ProceduralSynthesisHeader {
        magic: MAGIC_PROC,
        version: PROCEDURAL_SYNTHESIS_SECTION_VERSION,
        flags: (FLAG_CELLULAR_VORONOI
            | FLAG_IMPLICIT_FILLET
            | FLAG_ALLOMETRIC_GROWTH
            | FLAG_PGA_MOTOR_DYNAMICS) as u16,
        seed: q_hash(archetype.as_str()),
        voronoi_frequency: 14.5,
        voronoi_jitter: 0.80,
        fillet_blend_radius: 0.10,
        fillet_threshold: 0.04,
        allometry_exponent_k: 0.67,
        allometry_base_b: 1.12,
        spiral_growth_rate_k: 0.20,
        spiral_initial_radius_a: 0.05,
        motor_frequency_hz: 2.10,
        motor_damping_ratio: 0.06,
        critical_stress_threshold: 32.0,
        reserved_f32: 0.0,
    };
    bytes = attach_procedural_synthesis_section(&bytes, &proc_header)
        .expect("attach_procedural_synthesis_section failed");

    // EONT1 Entity Ontology Section
    let onto_header = EntityOntologyMiniHeader {
        magic: MAGIC_EONT,
        version: ENTITY_ONTOLOGY_SECTION_VERSION,
        stage: 2, // Adult
        reserved_u8: 0,
        entity_did_hash: q_hash(&format!("did:qualia:fauna:{}", archetype.as_str())),
        taxon_iri_hash: q_hash(&format!("urn:qualia:taxon:{}", archetype.as_str())),
        energy_permill: 960,
        damage_permill: 20,
        hydration_permill: 910,
        wear_permill: 40,
        affordance_count: 3,
        reserved_u32: 4,
    };

    let affordances = [
        AffordanceBinaryRecord {
            action_id: 0x0011,
            required_capability_hash: 0xCA00_1011,
            emitted_topic_hash: 0x7000_1011,
            category: 1, // Locomotion
            min_observer_status: 0,
            min_energy_permill: 100,
            max_damage_permill: 900,
            min_hydration_permill: 100,
            max_wear_permill: 950,
            energy_delta: -5,
            damage_delta: 0,
            hydration_delta: -2,
            wear_delta: 1,
            reserved_u16: 0,
            animation_clip_id: 1,
        },
        AffordanceBinaryRecord {
            action_id: 0x0012,
            required_capability_hash: 0xCA00_1012,
            emitted_topic_hash: 0x7000_1012,
            category: 2, // Foraging
            min_observer_status: 0,
            min_energy_permill: 50,
            max_damage_permill: 950,
            min_hydration_permill: 50,
            max_wear_permill: 980,
            energy_delta: 25,
            damage_delta: 0,
            hydration_delta: 10,
            wear_delta: 2,
            reserved_u16: 0,
            animation_clip_id: 2,
        },
        AffordanceBinaryRecord {
            action_id: 0x0013,
            required_capability_hash: 0xCA00_1013,
            emitted_topic_hash: 0x7000_1013,
            category: 3, // Vocalize / Social Display
            min_observer_status: 0,
            min_energy_permill: 80,
            max_damage_permill: 900,
            min_hydration_permill: 80,
            max_wear_permill: 900,
            energy_delta: -8,
            damage_delta: 0,
            hydration_delta: -4,
            wear_delta: 1,
            reserved_u16: 0,
            animation_clip_id: 3,
        },
    ];

    let keyframes = [
        DevelopmentalKeyframeRecord {
            age_days: 60.0,
            stage: 0, // Neonate Pouch Young
            reserved_u8: 0,
            reserved_u16: 0,
            allometric_crown_scale: 0.20,
            allometric_trunk_scale: 0.15,
            lai_multiplier: 0.10,
            reserved_u32: 0,
            mesh_asset_hash: 0,
        },
        DevelopmentalKeyframeRecord {
            age_days: 240.0,
            stage: 1, // Sub-adult Joey
            reserved_u8: 0,
            reserved_u16: 0,
            allometric_crown_scale: 0.65,
            allometric_trunk_scale: 0.58,
            lai_multiplier: 0.50,
            reserved_u32: 0,
            mesh_asset_hash: 0,
        },
        DevelopmentalKeyframeRecord {
            age_days: 1200.0,
            stage: 2, // Mature Adult
            reserved_u8: 0,
            reserved_u16: 0,
            allometric_crown_scale: 1.0,
            allometric_trunk_scale: 1.0,
            lai_multiplier: 1.0,
            reserved_u32: 0,
            mesh_asset_hash: 0,
        },
        DevelopmentalKeyframeRecord {
            age_days: 4500.0,
            stage: 3, // Dominant Elder
            reserved_u8: 0,
            reserved_u16: 0,
            allometric_crown_scale: 1.15,
            allometric_trunk_scale: 1.25,
            lai_multiplier: 0.95,
            reserved_u32: 0,
            mesh_asset_hash: 0,
        },
    ];

    bytes = attach_entity_ontology_section(&bytes, &onto_header, &affordances, &keyframes)
        .expect("attach_entity_ontology_section failed");

    // MRP1 Morph Target Section (Costal Respiration, Gular Vocalization, Achilles Tendon Strain)
    let mut morph_deltas = Vec::with_capacity(total_v * 3 / 8);
    let step = (total_v / 64).max(1);

    for vi in (0..total_v).step_by(step) {
        let p = eval_positions[vi];
        let n = acc.normals[vi];
        let layer = acc.layer_tags[vi];

        // Target 0: Costal respiration (intercostal and ribcage expansion)
        if layer == ZoologicalAnatomicalLayer::Osteology
            || layer == ZoologicalAnatomicalLayer::MyologyAndTendons
        {
            let resp_amp = if p[1] > hip_height - 0.1 && p[1] < hip_height + 0.2 {
                0.035
            } else {
                0.008
            };
            let d0 = quantize_morph_delta(
                [n[0] * resp_amp, 0.015, n[2] * resp_amp],
                [n[0] * 0.1, 0.05, n[2] * 0.1],
                [0.0; 3],
                0.25,
                0,
                vi as u32,
            );
            morph_deltas.push(d0);
        }

        // Target 1: Gular vocalization (neck / throat / jaw distension)
        if p[2] > spine_len * 0.5 {
            let gular_amp = 0.045;
            let d1 = quantize_morph_delta(
                [n[0] * gular_amp, -0.040, n[2] * gular_amp],
                [0.0, -0.1, 0.0],
                [0.0; 3],
                0.25,
                1,
                vi as u32,
            );
            morph_deltas.push(d1);
        }

        // Target 2: Achilles tendon strain (spring elongation during locomotion)
        if p[1] < hip_height - femur_len * 0.5 && p[2] < 0.05 {
            let strain_amp = 0.055;
            let d2 = quantize_morph_delta(
                [0.0, -strain_amp, 0.012],
                [0.0, -0.05, 0.02],
                [0.0; 3],
                0.25,
                2,
                vi as u32,
            );
            morph_deltas.push(d2);
        }
    }

    let morph_header = MorphSectionHeader {
        magic: MAGIC_MRP1,
        version: MORPH_SECTION_VERSION,
        target_count: 3,
        vertex_count: total_v as u32,
        flags: FLAG_MORPH_SPARSE | FLAG_MORPH_HAS_NORMALS | FLAG_MORPH_ONTOGENETIC_BIND,
        max_displacement: 0.25,
        ontogenetic_bind_mask: 0b0111,
        reserved: [0u8; 8],
    };

    bytes = attach_morph_section(&bytes, &morph_header, &morph_deltas)
        .expect("attach_morph_section failed");

    CompiledVertebrateAnatomy10d {
        archetype,
        rig,
        container_bytes: bytes,
        vertex_count: total_v,
        triangle_count: acc.indices.len() / 3,
        osteology_vertex_range: ost_start_v..ost_end_v,
        myology_vertex_range: myo_start_v..myo_end_v,
        integument_vertex_range: int_start_v..int_end_v,
        positions: eval_positions,
        normals: acc.normals,
        colors: acc.colors,
        uvs: acc.uvs,
        indices: acc.indices,
        materials,
        submesh_ranges,
        skin_joint_indices: acc.joint_indices,
        skin_weights: acc.joint_weights,
        layer_tags: acc.layer_tags,
    }
}

// ── Private Section Attachment Helpers ───────────────────────────────────────

fn attach_raw_section(
    container: &[u8],
    section_type: SectionType,
    alignment_tier: AlignmentTier,
    stride: u32,
    element_count: u32,
    payload: &[u8],
) -> Result<Vec<u8>, String> {
    let header =
        Container10dHeader::parse(container).map_err(|e| format!("10d header parse: {e:?}"))?;
    let descriptors = parse_section_table(container, &header)
        .map_err(|e| format!("parse_section_table: {e:?}"))?;
    if descriptors.len() >= MAX_SECTION_COUNT as usize {
        return Err("MAX_SECTION_COUNT reached".into());
    }

    let mut inputs = Vec::with_capacity(descriptors.len() + 1);
    for descriptor in descriptors {
        let start = descriptor.byte_offset as usize;
        let end = start
            .checked_add(descriptor.byte_length as usize)
            .filter(|&end| end <= container.len())
            .ok_or_else(|| {
                format!(
                    "SectionTableError::OutOfBounds offset={} length={}",
                    descriptor.byte_offset, descriptor.byte_length
                )
            })?;
        inputs.push(SectionInput {
            section_type: descriptor
                .typ()
                .ok_or_else(|| format!("UndefinedSectionType: {}", descriptor.section_type))?,
            alignment_tier: descriptor
                .tier()
                .ok_or_else(|| format!("UndefinedAlignmentTier: {}", descriptor.alignment_tier))?,
            stride: descriptor.stride,
            element_count: descriptor.element_count,
            payload: &container[start..end],
        });
    }

    inputs.push(SectionInput {
        section_type,
        alignment_tier,
        stride,
        element_count,
        payload,
    });

    let needed = match encode_container(&header, &inputs, &mut []) {
        Err(SectionTableError::OutputBufferTooSmall { needed, .. }) => needed,
        Ok(size) => size,
        Err(error) => return Err(format!("encode_container dry-run error: {error:?}")),
    };
    let mut output = vec![0u8; needed];
    let written = encode_container(&header, &inputs, &mut output)
        .map_err(|e| format!("encode_container error: {e:?}"))?;
    output.truncate(written);
    seal_whole_file_crc32c(&mut output);
    Ok(output)
}

fn attach_procedural_synthesis_section(
    container: &[u8],
    header: &ProceduralSynthesisHeader,
) -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; PROCEDURAL_SYNTHESIS_HEADER_SIZE];
    encode_procedural_synthesis_section(header, &mut buf)
        .map_err(|e| format!("encode_procedural_synthesis_section: {e:?}"))?;
    attach_raw_section(
        container,
        SectionType::ProceduralSynthesis,
        AlignmentTier::CacheLine,
        0,
        0,
        &buf,
    )
}

fn attach_entity_ontology_section(
    container: &[u8],
    header: &EntityOntologyMiniHeader,
    affordances: &[AffordanceBinaryRecord],
    keyframes: &[DevelopmentalKeyframeRecord],
) -> Result<Vec<u8>, String> {
    let mut hdr = *header;
    hdr.affordance_count = affordances.len() as u32;
    hdr.reserved_u32 = keyframes.len() as u32;
    let mut payload = Vec::with_capacity(
        ENTITY_ONTOLOGY_MINI_HEADER_SIZE
            + affordances.len() * AFFORDANCE_RECORD_SIZE
            + keyframes.len() * DEVELOPMENTAL_KEYFRAME_RECORD_SIZE,
    );
    payload.extend_from_slice(bytemuck::bytes_of(&hdr));
    for a in affordances {
        payload.extend_from_slice(bytemuck::bytes_of(a));
    }
    for k in keyframes {
        payload.extend_from_slice(bytemuck::bytes_of(k));
    }
    attach_raw_section(
        container,
        SectionType::EntityOntology,
        AlignmentTier::CacheLine,
        0,
        0,
        &payload,
    )
}

fn attach_morph_section(
    container: &[u8],
    header: &MorphSectionHeader,
    deltas: &[QuantizedMorphDelta],
) -> Result<Vec<u8>, String> {
    let needed = morph_section_encoded_len(deltas.len()).ok_or_else(|| {
        format!(
            "morph_section_encoded_len overflow: deltas={}",
            deltas.len()
        )
    })?;
    let mut buf = vec![0u8; needed];
    encode_morph_section(header, deltas, &mut buf)
        .map_err(|e| format!("encode_morph_section: {e:?}"))?;
    attach_raw_section(
        container,
        SectionType::Reconstruction,
        AlignmentTier::CacheLine,
        0,
        0,
        &buf,
    )
}

// ── Unit Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_10d::integrity::verify_whole_file_crc32c;

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
    fn test_all_13_archetypes_build_valid_scale5_anatomy() {
        for arch in ALL_13_ARCHETYPES {
            let compiled = build_scale5_vertebrate_anatomy_10d(arch, 1.0, 1.0, 1.0, 0.0);

            // 1. Verify high-resolution vertex count (>= 1,800)
            assert!(
                compiled.vertex_count >= 1800,
                "Archetype {:?} vertex count {} is below minimum 1,800",
                arch,
                compiled.vertex_count
            );

            // 2. Verify submesh count & material count (17 distinct anatomical parts)
            assert_eq!(
                compiled.submesh_ranges.len(),
                17,
                "Archetype {:?} must have 17 submesh ranges (8 osteo + 8 myo + 1 integument)",
                arch
            );
            assert_eq!(compiled.materials.len(), 17);

            // 3. Verify contiguous index stream without gaps
            let mut expected_first = 0u32;
            for range in &compiled.submesh_ranges {
                assert_eq!(range.first_index, expected_first);
                assert!(range.index_count > 0);
                expected_first += range.index_count;
            }
            assert_eq!(expected_first as usize, compiled.indices.len());

            // 4. Verify 3-layer vertex intervals
            assert!(!compiled.osteology_vertex_range.is_empty());
            assert!(!compiled.myology_vertex_range.is_empty());
            assert!(!compiled.integument_vertex_range.is_empty());
            assert_eq!(
                compiled.osteology_vertex_range.end,
                compiled.myology_vertex_range.start
            );
            assert_eq!(
                compiled.myology_vertex_range.end,
                compiled.integument_vertex_range.start
            );
            assert_eq!(compiled.integument_vertex_range.end, compiled.vertex_count);

            // 5. Verify non-empty .10d container bytes
            assert!(!compiled.container_bytes.is_empty());
        }
    }

    #[test]
    fn test_epipubic_bones_and_achilles_tendon_submeshes() {
        let compiled = build_scale5_vertebrate_anatomy_10d(
            FaunaBiomechanicalArchetype::MacropodSaltatorial,
            1.0,
            1.0,
            1.0,
            0.0,
        );

        // Submesh 5: PelvicGirdleAndEpipubicBones
        let pelvis_range = &compiled.submesh_ranges[5];
        assert!(pelvis_range.index_count >= 36);
        assert_eq!(
            pelvis_range.semantic_id,
            q_hash("urn:qualia:bio:submesh:pelvic_epipubic_bones")
        );

        // Submesh 9: ElasticAchillesTendonSpring
        let achilles_range = &compiled.submesh_ranges[9];
        assert!(achilles_range.index_count >= 36);
        assert_eq!(
            achilles_range.semantic_id,
            q_hash("urn:qualia:bio:submesh:achilles_tendon_spring")
        );

        // Verify tendon material has collagen PBR properties
        let tendon_mat = &compiled.materials[9];
        assert!(
            tendon_mat.roughness < 0.35,
            "Tendon should have smooth glistening collagen sheen"
        );
        assert!(
            tendon_mat.base_color[0] > 0.85,
            "Tendon should be ivory/white collagen"
        );
    }

    #[test]
    fn test_zero_heap_layer_peeling() {
        let compiled = build_scale5_vertebrate_anatomy_10d(
            FaunaBiomechanicalArchetype::VombatidFossorial,
            1.0,
            1.0,
            1.0,
            0.0,
        );

        let n = compiled.vertex_count;
        let mut out_positions = vec![[0.0f32; 3]; n];
        let mut out_alphas = vec![0.0f32; n];

        // 1. Fully visible without explosion
        let count = evaluate_anatomical_layer_peel_zero_heap(
            &compiled.positions,
            &compiled.normals,
            &compiled.layer_tags,
            1.0,
            1.0,
            1.0,
            0.0,
            &mut out_positions,
            &mut out_alphas,
        );
        assert_eq!(count, n);
        for a in &out_alphas {
            assert_eq!(*a, 1.0);
        }

        // 2. Hide integument (0.0), half myology (0.5), full osteology (1.0), exploded (1.0)
        evaluate_anatomical_layer_peel_zero_heap(
            &compiled.positions,
            &compiled.normals,
            &compiled.layer_tags,
            1.0,
            0.5,
            0.0,
            1.0,
            &mut out_positions,
            &mut out_alphas,
        );

        // Check osteology: alpha = 1.0, displacement = 0
        for vi in compiled.osteology_vertex_range.clone() {
            assert_eq!(out_alphas[vi], 1.0);
            assert_eq!(out_positions[vi], compiled.positions[vi]);
        }

        // Check myology: alpha = 0.5, displaced outward
        for vi in compiled.myology_vertex_range.clone() {
            assert_eq!(out_alphas[vi], 0.5);
            let disp = ((out_positions[vi][0] - compiled.positions[vi][0]).powi(2)
                + (out_positions[vi][1] - compiled.positions[vi][1]).powi(2)
                + (out_positions[vi][2] - compiled.positions[vi][2]).powi(2))
            .sqrt();
            assert!(disp > 0.05, "Myology layer should be exploded outward");
        }

        // Check integument: alpha = 0.0, displaced even further outward
        for vi in compiled.integument_vertex_range.clone() {
            assert_eq!(out_alphas[vi], 0.0);
            let disp = ((out_positions[vi][0] - compiled.positions[vi][0]).powi(2)
                + (out_positions[vi][1] - compiled.positions[vi][1]).powi(2)
                + (out_positions[vi][2] - compiled.positions[vi][2]).powi(2))
            .sqrt();
            assert!(
                disp > 0.12,
                "Integument layer should be exploded furthest outward"
            );
        }
    }

    #[test]
    fn test_10d_container_verification() {
        let compiled = build_scale5_vertebrate_anatomy_10d(
            FaunaBiomechanicalArchetype::MacropodSaltatorial,
            1.0,
            1.0,
            1.0,
            0.0,
        );

        let mut bytes = compiled.container_bytes.clone();

        // Verify valid 10d header
        let header = Container10dHeader::parse(&bytes).expect("Header parse must succeed");
        assert_eq!(header.magic, crate::container_10d::header::MAGIC_10D);

        // Verify whole-file CRC32C seal
        verify_whole_file_crc32c(&mut bytes).expect("Whole-file CRC-32C verification must pass");

        // Verify section table contents
        let descriptors =
            parse_section_table(&bytes, &header).expect("Section table parse must succeed");
        assert!(
            descriptors.len() >= 8,
            "Expected at least 8 sections, got {}",
            descriptors.len()
        );

        let section_types: Vec<Option<SectionType>> = descriptors.iter().map(|d| d.typ()).collect();
        assert!(section_types.contains(&Some(SectionType::QuantizedMesh)));
        assert!(section_types.contains(&Some(SectionType::ProvenanceSidecar)));
        assert!(section_types.contains(&Some(SectionType::FieldSidecar)));
        assert!(section_types.contains(&Some(SectionType::SurfaceReading)));
        assert!(section_types.contains(&Some(SectionType::Materials)));
        assert!(section_types.contains(&Some(SectionType::TextureCoordinates)));
        assert!(section_types.contains(&Some(SectionType::Reconstruction))); // Morph targets
        assert!(section_types.contains(&Some(SectionType::EntityOntology)));
        assert!(section_types.contains(&Some(SectionType::ProceduralSynthesis)));
    }
}
