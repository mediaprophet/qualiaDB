//! Resident animation controls admitted from the mounted HMC's semantic model.

mod femc_surface;

use bytemuck::Zeroable;
use wasm_bindgen::prelude::*;

use super::QualiaPortal;
use femc_surface::FemcSurface;
use crate::{
    bundle::{BundleReader, semantic_model::{SEMANTIC_MODEL_KEY, hmc_entry_key_iri, open_semantic_model}},
    container_10d::{
        AffordanceBinaryRecord, FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE, HOLOGRAPHIC_PHASE_HEADER_SIZE,
        HolographicPhaseHeader, MANIFOLD_IDENTITY_V2_MAX_FIELDS, MAX_AFFORDANCES_PER_SECTION,
        SectionType, TypedFieldRecord, TypedPayloadResolver, decode_entity_ontology_section_into,
    },
    q42::m64_matter::{M64PayloadInterval, M64ReadLimits, M64Reader},
    q42_volume::{MAX_COMPRESSED_SUPERBLOCK_SIZE, SUPERBLOCK_SIZE},
    q_hash,
    render::{
        hmc_playback::{HmcMeshResidencyReceipt, resolve_hmc_mesh_in_bundle},
        instance_culling::GpuInstanceRecord,
        portal_load_10d::{admit_leaf_assembly, assembly_residency_digest, decode_hmc_leaf_assembly, decode_load_10d_in_frame},
        skeletal::{MAX_JOINTS, Motor, SkeletalPlayer, valid_motor, semantic_motion::{
            ControllerFeedback, DcaChainState, FemcFrame, FemcState, HydraulicEdgeStep, MAX_FEMC_STEP_S, MAX_HYDRAULIC_EDGES,
            MacroChannel, MacroKeyframe, MarineVehicleControl, MarineVehicleState, MarineVehicleStep,
            MotionGraph, PreparedAeroelasticRotation, PreparedClipBlend, PreparedDcaChain, PreparedFeedbackRotation, PreparedFemcMotion,
            PreparedHydraulicMotion, PreparedMarineVehicle, PreparedMorphTargets,
            SemanticAction, SemanticMotionError, compose_leaf_delta,
            prepare_aeroelastic_rotation, prepare_dca_motor_chain, prepare_feedback_rotation_for_resource, prepare_femc_asset_motion,
            prepare_hydraulic_motion, prepare_marine_vehicle, prepare_q42_rule_macro, prepare_q42_semantic_macro, rest_poses_agree,
            sample_prepared_macro,
        }},
    },
};

const MAX_MODEL_BLOCKS: u64 = 256;
const MAX_SETUP_QUERIES: usize = 4096;
const MAX_KEYS: usize = 512;
const MAX_CHANNELS: usize = 128;
const MAX_TIME: f64 = 86_400.0;
const MAX_MORPH_DELTA_BYTES: usize = 32 * 1024 * 1024;

#[derive(Default)]
pub(crate) struct SceneSemanticMotion {
    model_digest: Option<([u8; 32], [u8; 32])>,
    command: Option<BoundCommand>,
}

struct BoundCommand {
    source: HmcMeshResidencyReceipt,
    target: ResidentMotionTarget,
    target_ids: Box<[u64]>,
    playback: Playback,
}

enum Playback {
    Macro {
        keys: Box<[MacroKeyframe]>,
        time: f64,
        morph: Option<MorphPlayback>,
        blend: Option<Box<ClipBlendPlayback>>,
        channel_count: usize,
    },
    Feedback { controller: PreparedFeedbackRotation, angle: f64, velocity: f64, goal: f64 },
    Aeroelastic(PreparedAeroelasticRotation),
    Dca {
        controller: PreparedDcaChain,
        state: Box<[DcaChainState]>,
        target: Option<DcaTarget>,
        pair_residuals_m: Option<[f64; 4]>,
    },
    Hydraulic {
        controller: PreparedHydraulicMotion,
        pressure_pa: Box<[f64]>,
        source_m3_s: Box<[f64]>,
        flow: Box<[HydraulicEdgeStep]>,
        has_sample: bool,
    },
    Marine {
        controller: PreparedMarineVehicle,
        state: MarineVehicleState,
        control: MarineVehicleControl,
        latest: Option<MarineVehicleStep>,
    },
    Femc {
        controller: PreparedFemcMotion,
        state: Box<FemcState>,
        activation: f64,
        latest: Option<FemcFrame>,
        recruitment: Option<FemcRecruitment>,
    },
}

#[derive(Clone, Copy)]
enum DcaTarget {
    Point([f64; 3]),
    Pair([[f64; 3]; 2]),
}

struct FemcRecruitment {
    keys: Box<[MacroKeyframe]>,
    time: f64,
}

struct MorphPlayback {
    targets: PreparedMorphTargets,
    positions: Box<[[f32; 3]]>,
    normals: Box<[[f32; 3]]>,
}

struct ClipBlendPlayback {
    space: PreparedClipBlend,
    poses: [[Motor; MAX_JOINTS]; 3],
}

enum ResidentMotionTarget {
    Skin(SkeletalPlayer),
    Femc(FemcSurface),
    Assembly {
        rest: Box<[GpuInstanceRecord]>,
        posed: Box<[GpuInstanceRecord]>,
        slots: Box<[(u64, usize)]>,
    },
}

impl SceneSemanticMotion {
    pub(crate) fn record_model(&mut self, resident_digest: [u8; 32], bundle: &BundleReader<'_>) {
        self.command = None;
        self.model_digest = semantic_digest(bundle).map(|digest| (resident_digest, digest));
    }
}

fn semantic_digest(bundle: &BundleReader<'_>) -> Option<[u8; 32]> {
    let entry = match (bundle.entry(SEMANTIC_MODEL_KEY), bundle.entry("manifest.q42")) {
        (Some(entry), None) | (None, Some(entry)) if entry.kind == "q42" => entry,
        _ => return None,
    };
    if !bundle.verify_entry(&entry.key) { return None; }
    entry.sha256.as_slice().try_into().ok()
}

fn motion_error(error: SemanticMotionError) -> JsValue {
    JsValue::from_str(&format!("HMC semantic motion: {error:?}"))
}

#[derive(Clone, Copy)]
enum Program<'a> {
    Macro { rule_source: Option<&'a str> },
    Feedback { material_key: &'a str },
    Aeroelastic { material_key: &'a str },
    Dca { material_key: &'a str, joints: &'a [&'a str] },
    Hydraulic { material_key: &'a str, nodes: &'a [&'a str], edges: &'a [&'a str] },
    Marine { material_key: &'a str },
    Femc { material_key: &'a str, recruitment: Option<RecruitmentSource<'a>> },
}

#[derive(Clone, Copy)]
enum RecruitmentSource<'a> {
    Materialized,
    Rule(&'a str),
}

impl QualiaPortal {
    fn install_semantic_macro(&mut self, command: BoundCommand) -> Result<u32, JsValue> {
        let Playback::Macro { channel_count, .. } = &command.playback else {
            return Err(motion_error(SemanticMotionError::InvalidCommand));
        };
        let count = *channel_count as u32;
        self.clear_hmc_semantic_motion()?;
        self.semantic_motion.command = Some(command);
        Ok(count)
    }

    fn prepare_hmc_semantic_motion(
        &self, bytes: &[u8], asset_key: &str, action_iri: &str, context_iri: &str, group_iri: &str,
        program: Program<'_>,
    ) -> Result<BoundCommand, JsValue> {
        if [asset_key, action_iri, context_iri, group_iri].iter().any(|value| value.is_empty() || value.len() > 4096) {
            return Err(motion_error(SemanticMotionError::InvalidCommand));
        }
        let bundle = BundleReader::parse(bytes).map_err(|error| JsValue::from_str(&format!("Motion HMC: {error}")))?;
        let asset = resolve_hmc_mesh_in_bundle(&bundle, asset_key).map_err(|error| JsValue::from_str(&error))?;
        if !bundle.verify_entry(asset_key) { return Err(JsValue::from_str("Motion HMC asset SHA-256 mismatch")); }
        let assembly = assembly_residency_digest(&bundle, asset.bytes, asset.digest).map_err(|error| JsValue::from_str(&error))?;
        let digest = assembly.unwrap_or(asset.digest);
        let state = self.hmc_mesh_residency.resident(digest).ok_or_else(|| JsValue::from_str("Motion requires the matching resident HMC asset"))?;
        let motion_model_digest = semantic_digest(&bundle).ok_or_else(|| JsValue::from_str("Missing authoritative HMC semantic model"))?;
        if self.semantic_motion.model_digest != Some((digest, motion_model_digest)) {
            return Err(JsValue::from_str("Motion semantic model differs from the mounted HMC model"));
        }
        if self.scene_animation.external_controlled || !self.scene_animation.actors.is_empty() || self.gpu.is_none() {
            return Err(JsValue::from_str("Semantic motion requires an exclusive resident HMC WebGPU mesh"));
        }
        let model = open_semantic_model(&bundle).map_err(|error| JsValue::from_str(&error.to_string()))?;
        let mut field_scratch = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
        let mut fields = field_scratch;
        let view = TypedPayloadResolver::new(asset.bytes, &[], &mut field_scratch, &mut fields)
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        let eont = view.section_payload(SectionType::EntityOntology).ok_or_else(|| JsValue::from_str("Motion action requires an EONT table"))?;
        let mut records = [AffordanceBinaryRecord::zeroed(); MAX_AFFORDANCES_PER_SECTION];
        let (physiology, count) = decode_entity_ontology_section_into(eont, &mut records)
            .map_err(|error| JsValue::from_str(&format!("Motion EONT: {error:?}")))?;
        let action = SemanticAction {
            iri: action_iri, affordances: &records[..count], capability_ids: &[], observer_status: 0,
            energy_permill: physiology.energy_permill, damage_permill: physiology.damage_permill,
            hydration_permill: physiology.hydration_permill, wear_permill: physiology.wear_permill,
        };
        let mut compressed = vec![0; MAX_COMPRESSED_SUPERBLOCK_SIZE];
        let mut decoded = vec![0; SUPERBLOCK_SIZE];
        let mut lex_page = vec![0; 1024 * 1024];
        let mut lexical = [0; 4096];
        let group = q_hash(group_iri);
        let group_len = model.lookup_lexicon_hash_into(group, &mut lex_page, &mut lexical)
            .map_err(|error| JsValue::from_str(&error.to_string()))?
            .ok_or_else(|| motion_error(SemanticMotionError::UnknownIdentity))?;
        if lexical.get(..group_len) != Some(group_iri.as_bytes()) {
            return Err(motion_error(SemanticMotionError::UnknownIdentity));
        }
        let mut graph = MotionGraph {
            model: &model, context: q_hash(context_iri), max_model_blocks: MAX_MODEL_BLOCKS,
            queries_left: MAX_SETUP_QUERIES, compressed: &mut compressed, decoded: &mut decoded,
            lex_page: &mut lex_page, lex_value: &mut lexical,
        };
        graph.verify_context_iri(context_iri).map_err(motion_error)?;
        let mut femc_visual = None;
        let (playback, ids) = match program {
            Program::Macro { rule_source } => {
                let mut keys = vec![MacroKeyframe::default(); MAX_KEYS];
                let count = match rule_source {
                    Some(source) => prepare_q42_rule_macro(&mut graph, &action, source, &mut keys),
                    None => prepare_q42_semantic_macro(&mut graph, &action, &mut keys),
                }.map_err(motion_error)?;
                keys.truncate(count);
                let mut staged = [None; MAX_CHANNELS];
                let mut out = [None; MAX_CHANNELS];
                let channels = sample_prepared_macro(&keys, 0.0, &mut staged, &mut out).map_err(motion_error)?;
                let mut ids = Vec::new();
                let mut morph_ids = Vec::new();
                let mut blend_ids = [None; 2];
                for candidate in &out[..channels] {
                    let candidate = candidate.as_ref().ok_or_else(|| motion_error(SemanticMotionError::InvalidCommand))?;
                    match candidate.channel {
                        MacroChannel::Rotation => ids.push(candidate.semantic_id),
                        MacroChannel::MorphWeight => morph_ids.push(candidate.semantic_id),
                        MacroChannel::BlendX | MacroChannel::BlendY => {
                            let axis = (candidate.channel == MacroChannel::BlendY) as usize;
                            if blend_ids[axis].replace(candidate.semantic_id).is_some() {
                                return Err(motion_error(SemanticMotionError::AmbiguousTarget));
                            }
                        }
                        MacroChannel::MuscleRecruitment => return Err(motion_error(SemanticMotionError::UnboundTarget)),
                    }
                }
                let blend = match blend_ids {
                    [None, None] => None,
                    [Some(x), Some(y)] if x == y && assembly.is_none() => {
                        let space = PreparedClipBlend::bind(asset.bytes, &mut graph, group, x, &ids).map_err(motion_error)?;
                        space.sample(&out[..channels]).map_err(motion_error)?;
                        Some(Box::new(ClipBlendPlayback { space, poses: [[Motor::identity(); MAX_JOINTS]; 3] }))
                    }
                    _ => return Err(motion_error(SemanticMotionError::UnboundTarget)),
                };
                let morph = if morph_ids.is_empty() {
                    None
                } else {
                    if assembly.is_some() { return Err(motion_error(SemanticMotionError::UnboundTarget)); }
                    let vertex_count = state.vertex_count as usize;
                    let targets = PreparedMorphTargets::bind(
                        asset.bytes, vertex_count, &mut graph, group, &morph_ids, MAX_MORPH_DELTA_BYTES,
                    ).map_err(motion_error)?;
                    Some(MorphPlayback {
                        targets,
                        positions: vec![[0.0; 3]; vertex_count].into_boxed_slice(),
                        normals: vec![[0.0; 3]; vertex_count].into_boxed_slice(),
                    })
                };
                (Playback::Macro { keys: keys.into_boxed_slice(), time: 0.0, morph, blend, channel_count: channels }, ids)
            }
            Program::Feedback { material_key } | Program::Aeroelastic { material_key }
            | Program::Dca { material_key, .. } | Program::Hydraulic { material_key, .. }
            | Program::Marine { material_key } | Program::Femc { material_key, .. } => {
                if material_key.is_empty() || material_key.len() > 4096 {
                    return Err(motion_error(SemanticMotionError::InvalidMaterial));
                }
                let entry = bundle.entry(material_key).filter(|entry| entry.kind == "m64")
                    .ok_or_else(|| motion_error(SemanticMotionError::InvalidMaterial))?;
                if entry.length > 128 * 1024 * 1024 || !bundle.verify_entry(material_key) {
                    return Err(motion_error(SemanticMotionError::InvalidMaterial));
                }
                let mut intervals = vec![M64PayloadInterval::default(); 4096];
                let material = M64Reader::parse(
                    bundle.get(&entry.key).ok_or_else(|| motion_error(SemanticMotionError::InvalidMaterial))?,
                    M64ReadLimits { max_file_bytes: 128 * 1024 * 1024, max_chunks: 4096, max_records: 1_048_576 },
                    &mut intervals,
                ).map_err(|error| JsValue::from_str(&format!("Motion material: {error:?}")))?;
                let material_iri = hmc_entry_key_iri(material_key);
                match program {
                    Program::Aeroelastic { .. } => {
                        let controller = prepare_aeroelastic_rotation(&mut graph, &action, &material_iri, &material)
                            .map_err(motion_error)?;
                        let id = controller.semantic_id();
                        (Playback::Aeroelastic(controller), vec![id])
                    }
                    Program::Feedback { .. } => {
                        let controller = prepare_feedback_rotation_for_resource(&mut graph, &action, &material_iri, &material)
                            .map_err(motion_error)?;
                        let angle = controller.resting_angle().map_err(motion_error)?;
                        let id = controller.semantic_id();
                        (Playback::Feedback { controller, angle, velocity: 0.0, goal: angle }, vec![id])
                    }
                    Program::Dca { joints, .. } => {
                        if assembly.is_some() { return Err(motion_error(SemanticMotionError::UnboundTarget)); }
                        let controller = prepare_dca_motor_chain(&mut graph, &action, joints, &material_iri, &material)
                            .map_err(motion_error)?;
                        let ids = joints.iter().map(|iri| q_hash(iri)).collect::<Vec<_>>();
                        verify_dca_bind_chain(asset.bytes, group, &ids, &controller)?;
                        let mut state = vec![DcaChainState::default(); controller.len()].into_boxed_slice();
                        controller.initialize_rest_state(&mut state).map_err(motion_error)?;
                        if state.iter().any(|joint| joint.angle_radians != 0.0) {
                            return Err(motion_error(SemanticMotionError::InvalidFit));
                        }
                        (Playback::Dca { controller, state, target: None, pair_residuals_m: None }, ids)
                    }
                    Program::Hydraulic { nodes, edges, .. } => {
                        let controller = prepare_hydraulic_motion(&mut graph, &action, nodes, edges, &material_iri, &material)
                            .map_err(motion_error)?;
                        let ids = controller.motion_target_ids().collect::<Vec<_>>();
                        if ids.is_empty() { return Err(motion_error(SemanticMotionError::UnboundTarget)); }
                        let pressure_pa = controller.reference_pressures().collect::<Box<_>>();
                        let source_m3_s = vec![0.0; controller.node_count()].into_boxed_slice();
                        let flow = vec![HydraulicEdgeStep::default(); controller.edge_count()].into_boxed_slice();
                        (Playback::Hydraulic { controller, pressure_pa, source_m3_s, flow, has_sample: false }, ids)
                    }
                    Program::Marine { .. } => {
                        if assembly.is_none() { return Err(motion_error(SemanticMotionError::UnboundTarget)); }
                        let controller = prepare_marine_vehicle(&mut graph, &action, &material_iri, &material)
                            .map_err(motion_error)?;
                        let id = controller.semantic_id();
                        let state = controller.initial_state();
                        (Playback::Marine { controller, state, control: MarineVehicleControl::default(), latest: None }, vec![id])
                    }
                    Program::Femc { recruitment, .. } => {
                        if assembly.is_some() { return Err(motion_error(SemanticMotionError::UnboundTarget)); }
                        let digest = entry.sha256.as_slice().try_into()
                            .map_err(|_| motion_error(SemanticMotionError::InvalidMaterial))?;
                        let (controller, visual) = prepare_femc_asset_motion(
                            &mut graph, &action, group_iri, &material_iri, digest, &material, asset.bytes,
                        ).map_err(motion_error)?;
                        femc_visual = Some(visual);
                        let state = Box::new(controller.initial_state());
                        let recruitment = if let Some(source) = recruitment {
                            let mut keys = vec![MacroKeyframe::default(); MAX_KEYS];
                            let count = match source {
                                RecruitmentSource::Materialized => prepare_q42_semantic_macro(&mut graph, &action, &mut keys),
                                RecruitmentSource::Rule(source) => prepare_q42_rule_macro(&mut graph, &action, source, &mut keys),
                            }.map_err(motion_error)?;
                            keys.truncate(count);
                            let recruitment = FemcRecruitment { keys: keys.into_boxed_slice(), time: 0.0 };
                            recruitment.sample(0.0, Some(group))?;
                            Some(recruitment)
                        } else {
                            None
                        };
                        (Playback::Femc { controller, state, activation: 0.0, latest: None, recruitment }, Vec::new())
                    }
                    Program::Macro { .. } => return Err(motion_error(SemanticMotionError::InvalidCommand)),
                }
            }
        };
        let target = if assembly.is_some() {
            verify_holo_actor(&view, group)?;
            let marine = match &playback {
                Playback::Marine { controller, .. } => Some(controller),
                _ => None,
            };
            source_assembly_target(&bundle, asset_key, &ids, marine)?
        } else if let Playback::Femc { controller, .. } = &playback {
            let visual = femc_visual.ok_or_else(|| motion_error(SemanticMotionError::UnboundTarget))?;
            ResidentMotionTarget::Femc(FemcSurface::bind(asset.bytes, visual, state.vertex_count, controller)?)
        } else {
            let requires_morph_topology = matches!(&playback, Playback::Macro { morph: Some(_), .. });
            let player = source_skin_player(asset.bytes, state.vertex_count, requires_morph_topology)?;
            if !player.has_joint(group) || ids.iter().any(|id| !player.has_joint(*id)) {
                return Err(motion_error(SemanticMotionError::UnboundTarget));
            }
            ResidentMotionTarget::Skin(player)
        };
        Ok(BoundCommand { source: state, target, target_ids: ids.into_boxed_slice(), playback })
    }

    pub(crate) fn tick_scene_semantic_motion(&mut self, dt: f64, reduced_motion: bool) -> Result<bool, JsValue> {
        let Some(mut command) = self.semantic_motion.command.take() else { return Ok(false); };
        if self.hmc_mesh_residency.current() != Some(command.source) || self.scene_animation.external_controlled
            || !self.scene_animation.actors.is_empty()
        {
            return Ok(false);
        }
        let result = advance_bound_command(&mut command, dt, reduced_motion)
            .and_then(|()| self.upload_semantic_pose(&command.target));
        match result {
            Ok(()) => {
                self.semantic_motion.command = Some(command);
                Ok(true)
            }
            Err(error) => {
                // Quarantine the failed command. The best-effort rest upload
                // never reactivates it or advances a partial pose next frame.
                if let Err(rollback) = command.target.reset(&command.target_ids)
                    .and_then(|()| self.upload_semantic_pose(&command.target))
                {
                    let reason = error.as_string().unwrap_or_else(|| "frame failed".into());
                    let recovery = rollback.as_string().unwrap_or_else(|| "rest upload failed".into());
                    return Err(JsValue::from_str(&format!("Semantic motion stopped ({reason}); reset failed ({recovery})")));
                }
                Err(error)
            }
        }
    }

    fn upload_semantic_pose(&mut self, target: &ResidentMotionTarget) -> Result<(), JsValue> {
        let gpu = self.gpu.as_mut().ok_or_else(|| JsValue::from_str("Semantic motion requires WebGPU"))?;
        match target {
            ResidentMotionTarget::Skin(player) => {
                if !gpu.write_mesh_animated_frames(0, player.positions(), player.normals(), player.tangents()) {
                    return Err(JsValue::from_str("Semantic motion frame upload failed"));
                }
            }
            ResidentMotionTarget::Femc(surface) => {
                let frame = &surface.posed;
                if !gpu.write_mesh_animated_frames(0, &frame.positions, &frame.normals, &frame.tangents) {
                    return Err(JsValue::from_str("FEMC frame upload failed"));
                }
            }
            ResidentMotionTarget::Assembly { posed, .. } => {
                gpu.update_semantic_assembly_instances(posed).map_err(|error| JsValue::from_str(&error))?;
            }
        }
        Ok(())
    }
}

fn verify_holo_actor(view: &TypedPayloadResolver<'_, '_>, group: u64) -> Result<(), JsValue> {
    let unbound = || motion_error(SemanticMotionError::UnboundTarget);
    let mut payloads = view.payloads_of_kind(FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE);
    let first = payloads.next().ok_or_else(unbound)?.map_err(|error| JsValue::from_str(&error.to_string()))?;
    let header_bytes = first.bytes.get(..HOLOGRAPHIC_PHASE_HEADER_SIZE).ok_or_else(unbound)?;
    let header: HolographicPhaseHeader = bytemuck::pod_read_unaligned(header_bytes);
    if payloads.next().is_some() || header.nquin_subject_hash != group { return Err(unbound()); }
    Ok(())
}

fn verify_dca_bind_chain(
    asset: &[u8], group: u64, ids: &[u64], controller: &PreparedDcaChain,
) -> Result<(), JsValue> {
    let unbound = || motion_error(SemanticMotionError::UnboundTarget);
    let rig = crate::container_10d::skeletal_container::read_skeletal_section(asset)
        .map_err(|_| unbound())?;
    if ids.first() != Some(&group) { return Err(unbound()); }
    let clip = rig.clip(0).ok_or_else(unbound)?;
    let tracks = (clip.first_track..clip.first_track + clip.track_count).filter_map(|i| rig.track(i as usize));
    let mut parent = None;
    for (slot, id) in ids.iter().enumerate() {
        let index = (0..rig.counts().joints).find(|&i| rig.joint_id(i) == Some(*id)).ok_or_else(unbound)?;
        let joint = rig.joint(index).ok_or_else(unbound)?;
        let expected = controller.rest_motor(slot).ok_or_else(unbound)?;
        if joint.parent != parent || !rest_poses_agree(expected, joint.rest_local) {
            return Err(unbound());
        }
        if let Some(track) = tracks.clone().find(|track| usize::from(track.joint) == index) {
            let key = rig.key(track.first_key as usize).ok_or_else(unbound)?;
            if key.time != 0.0 || !rest_poses_agree(expected, key.local) { return Err(unbound()); }
        }
        parent = Some(u16::try_from(index).map_err(|_| unbound())?);
    }
    Ok(())
}

fn parse_ordered_iris(input: &str, max: usize) -> Result<Vec<String>, JsValue> {
    if input.len() > 64 * 1024 { return Err(motion_error(SemanticMotionError::InvalidCommand)); }
    let iris: Vec<String> = serde_json::from_str(input).map_err(|_| motion_error(SemanticMotionError::InvalidCommand))?;
    if iris.is_empty() || iris.len() > max || iris.iter().any(|iri| iri.is_empty() || iri.len() > 4096) {
        return Err(motion_error(SemanticMotionError::InvalidCommand));
    }
    Ok(iris)
}

fn source_assembly_target(
    bundle: &BundleReader<'_>, key: &str, targets: &[u64], marine: Option<&PreparedMarineVehicle>,
) -> Result<ResidentMotionTarget, JsValue> {
    let leaves = admit_leaf_assembly(bundle, key).map_err(|error| JsValue::from_str(&error))?;
    if let Some(controller) = marine {
        let [leaf] = leaves.as_slice() else { return Err(motion_error(SemanticMotionError::UnboundTarget)); };
        controller.validate_instance_root(&leaf.overrides).map_err(motion_error)?;
    }
    let slots = targets.iter().map(|&id| {
        let mut matches = leaves.iter().filter(|leaf| leaf.semantic_id == id);
        let leaf = matches.next().ok_or_else(|| motion_error(SemanticMotionError::UnboundTarget))?;
        if matches.next().is_some() { return Err(motion_error(SemanticMotionError::AmbiguousTarget)); }
        Ok((id, leaf.slot))
    }).collect::<Result<Vec<_>, _>>()?;
    let decoded = decode_hmc_leaf_assembly(bundle, key).map_err(|error| JsValue::from_str(&error))?;
    if decoded.instances.len() != leaves.len() {
        return Err(motion_error(SemanticMotionError::UnboundTarget));
    }
    let rest = decoded.instances.into_boxed_slice();
    let posed = rest.clone();
    Ok(ResidentMotionTarget::Assembly { rest, posed, slots: slots.into_boxed_slice() })
}

impl ResidentMotionTarget {
    fn apply_motor(&mut self, id: u64, motor: Motor) -> Result<(), JsValue> {
        if !valid_motor(&motor) { return Err(motion_error(SemanticMotionError::InvalidCommand)); }
        match self {
            Self::Skin(player) => player.set_joint_delta(id, motor).map_err(|_| motion_error(SemanticMotionError::InvalidCommand)),
            Self::Femc(_) => Err(motion_error(SemanticMotionError::UnboundTarget)),
            Self::Assembly { rest, posed, slots } => {
                let (_, slot) = *slots.iter().find(|(candidate, _)| *candidate == id)
                    .ok_or_else(|| motion_error(SemanticMotionError::UnboundTarget))?;
                posed[slot] = compose_leaf_delta(&rest[slot], motor).map_err(motion_error)?;
                Ok(())
            }
        }
    }

    fn reset(&mut self, joints: &[u64]) -> Result<(), JsValue> {
        match self {
            Self::Skin(player) => {
                for &id in joints {
                    player.set_joint_delta(id, Motor::identity()).map_err(|_| motion_error(SemanticMotionError::InvalidCommand))?;
                }
                player.seek(0.0).map_err(|_| motion_error(SemanticMotionError::InvalidCommand))?;
            }
            Self::Femc(surface) => surface.reset(),
            Self::Assembly { posed, rest, .. } => posed.copy_from_slice(rest),
        }
        self.advance_skin(0.0, true)
    }

    fn advance_skin(&mut self, dt: f64, reduced: bool) -> Result<(), JsValue> {
        if let Self::Skin(player) = self {
            player.advance(dt, reduced).map_err(|error| JsValue::from_str(&format!("Semantic motion skinning: {error:?}")))?;
        }
        Ok(())
    }
}

fn source_skin_player(
    asset: &[u8], expected_vertices: u32, requires_morph_topology: bool,
) -> Result<SkeletalPlayer, JsValue> {
    let decoded = decode_load_10d_in_frame(asset, false).map_err(|error| JsValue::from_str(&error))?;
    if decoded.governance_refused || decoded.vertex_count != expected_vertices {
        return Err(JsValue::from_str("The admitted mesh cannot receive semantic skinning"));
    }
    if requires_morph_topology && decoded.field_representation != "QuantizedMesh" {
        return Err(motion_error(SemanticMotionError::UnboundTarget));
    }
    let section = crate::container_10d::skeletal_container::read_skeletal_section(asset)
        .map_err(|error| JsValue::from_str(&format!("Semantic motion requires a resident skin and bind rig: {error:?}")))?;
    let count = decoded.positions.len();
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for point in &decoded.positions {
        for axis in 0..3 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    }
    let origin = std::array::from_fn(|axis| (min[axis] + max[axis]) * 0.5);
    let span = (0..3).map(|axis| max[axis] - min[axis]).fold(1e-6, f32::max);
    let normals = match decoded.normals {
        Some(normals) => normals,
        None => {
            let mut normals = vec![[0.0; 3]; count];
            crate::render::mesh_surface_frames::vertex_normals_from_indices_into(
                &decoded.positions, &decoded.indices, &mut normals,
            ).map_err(|error| JsValue::from_str(&format!("Semantic motion source normals: {error:?}")))?;
            normals
        }
    };
    let uv = decoded.uv0.unwrap_or_else(|| vec![[0.0; 2]; count]);
    let mut player = SkeletalPlayer::from_section_with_budget(
        &section, &decoded.positions, &normals, &uv, &decoded.indices, 42 * 1024 * 1024,
    ).map_err(|error| JsValue::from_str(&format!("Semantic motion skin admission: {error:?}")))?;
    player.set_output_frame(origin, 1.6 / span)
        .map_err(|error| JsValue::from_str(&format!("Semantic motion orbit frame: {error:?}")))?;
    Ok(player)
}

impl FemcRecruitment {
    fn sample(&self, time: f64, required_id: Option<u64>) -> Result<f64, JsValue> {
        let mut staged = [None];
        let mut out = [None];
        let count = sample_prepared_macro(&self.keys, time, &mut staged, &mut out).map_err(motion_error)?;
        let sample = out[0].ok_or_else(|| motion_error(SemanticMotionError::UnboundTarget))?;
        if count != 1 || sample.channel != MacroChannel::MuscleRecruitment
            || required_id.is_some_and(|id| id != sample.semantic_id)
        {
            return Err(motion_error(SemanticMotionError::UnboundTarget));
        }
        Ok(sample.value)
    }
}

fn advance_bound_command(command: &mut BoundCommand, dt: f64, reduced_motion: bool) -> Result<(), JsValue> {
    match &mut command.playback {
            Playback::Macro { keys, time, morph, blend, .. } => {
                let next = if reduced_motion { 0.0 } else { (*time + dt).min(MAX_TIME) };
                let mut staged = [None; MAX_CHANNELS];
                let mut out = [None; MAX_CHANNELS];
                let count = sample_prepared_macro(keys, next, &mut staged, &mut out).map_err(motion_error)?;
                let sample = &out[..count];
                for candidate in sample.iter().flatten().filter(|row| row.channel == MacroChannel::Rotation) {
                    let rotation = candidate.rotation.ok_or_else(|| motion_error(SemanticMotionError::InvalidCommand))?;
                    command.target.apply_motor(rotation.semantic_id, rotation.motor)?;
                }
                if !reduced_motion { *time = next; }
                if blend.is_some() || morph.is_some() {
                    let ResidentMotionTarget::Skin(player) = &mut command.target else {
                        return Err(motion_error(SemanticMotionError::UnboundTarget));
                    };
                    let geometry = if let Some(workspace) = morph {
                        let (base_positions, base_normals) = player.bind_geometry();
                        workspace.targets.sample_from_weights_into(
                            sample, base_positions, base_normals, &mut workspace.positions, &mut workspace.normals,
                        ).map_err(motion_error)?;
                        Some((workspace.positions.as_ref(), workspace.normals.as_ref()))
                    } else {
                        None
                    };
                    let result = if let Some(blend) = blend {
                        let weights = blend.space.sample(sample).map_err(motion_error)?;
                        player.advance_with_clip_blend(weights, next, &mut blend.poses, geometry)
                    } else if let Some((positions, normals)) = geometry {
                        player.advance_with_bind_geometry(dt, reduced_motion, positions, normals).map(|_| ())
                    } else {
                        return Err(motion_error(SemanticMotionError::UnboundTarget));
                    };
                    return result.map_err(|error| JsValue::from_str(&format!("Semantic motion skinning: {error:?}")));
                }
            }
            Playback::Feedback { controller, angle, velocity, goal } => {
                if dt == 0.0 || reduced_motion {
                    return command.target.advance_skin(0.0, reduced_motion);
                }
                let steps = (dt / 0.02).ceil() as u32;
                let mut state = (*angle, *velocity);
                let mut latest = None;
                for _ in 0..steps {
                    let sample = controller.step(ControllerFeedback {
                        angle_rad: state.0, angular_velocity_rad_s: state.1,
                        target_rad: *goal, dt_s: dt / f64::from(steps),
                    }).map_err(motion_error)?;
                    state = (sample.rotation.angle_radians, sample.angular_velocity_rad_s);
                    latest = Some(sample.rotation);
                }
                if let Some(rotation) = latest {
                    command.target.apply_motor(rotation.semantic_id, rotation.motor)?;
                    (*angle, *velocity) = state;
                }
            }
            Playback::Aeroelastic(controller) => {
                let rotation = controller.advance(if reduced_motion { 0.0 } else { dt }).map_err(motion_error)?;
                command.target.apply_motor(rotation.semantic_id, rotation.motor)?;
            }
            Playback::Dca { controller, state, target, pair_residuals_m } => {
                if let Some(target) = (*target).filter(|_| dt > 0.0 && !reduced_motion) {
                    let substeps = (dt / 0.05).ceil() as u32;
                    let substep = dt / f64::from(substeps);
                    let mut staged = [DcaChainState::default(); MAX_JOINTS];
                    let next = &mut staged[..state.len()];
                    next.copy_from_slice(state);
                    let mut motors = [None; MAX_CHANNELS];
                    let mut torques = [0.0; MAX_CHANNELS];
                    let mut pair = None;
                    for _ in 0..substeps {
                        match target {
                            DcaTarget::Point(point) => {
                                controller.advance(point, substep, next, &mut motors, &mut torques)
                                    .map_err(motion_error)?;
                            }
                            DcaTarget::Pair(points) => {
                                let [primary, secondary] = controller.advance_two_points(
                                    points, substep, next, &mut motors, &mut torques,
                                ).map_err(motion_error)?;
                                pair = Some([
                                    primary.ik_residual_m, primary.tracking_residual_m,
                                    secondary.ik_residual_m, secondary.tracking_residual_m,
                                ]);
                            }
                        }
                    }
                    for rotation in motors[..controller.len()].iter().flatten() {
                        command.target.apply_motor(rotation.semantic_id, rotation.motor)?;
                    }
                    command.target.advance_skin(0.0, true)?;
                    state.copy_from_slice(next);
                    *pair_residuals_m = pair;
                    return Ok(());
                }
                return command.target.advance_skin(0.0, true);
            }
            Playback::Hydraulic { controller, pressure_pa, source_m3_s, flow, has_sample } => {
                if dt > 0.0 && !reduced_motion {
                    let mut motors = [None; MAX_CHANNELS];
                    controller.advance(dt, pressure_pa, source_m3_s, flow, &mut motors).map_err(motion_error)?;
                    for rotation in motors[..controller.node_count()].iter().flatten() {
                        command.target.apply_motor(rotation.semantic_id, rotation.motor)?;
                    }
                    *has_sample = true;
                }
            }
            Playback::Marine { controller, state, control, latest } => {
                if dt > 0.0 && !reduced_motion {
                    let substeps = (dt / 0.25).ceil() as u32;
                    let substep = dt / f64::from(substeps);
                    let mut next = *state;
                    let mut sample = None;
                    for _ in 0..substeps {
                        sample = Some(controller.advance(&mut next, *control, substep).map_err(motion_error)?);
                    }
                    if let Some(step) = sample {
                        command.target.apply_motor(step.semantic_id, step.local_delta)?;
                        *state = next;
                        *latest = Some(step);
                    }
                }
            }
            Playback::Femc { controller, state, activation, latest, recruitment } => {
                if dt > 0.0 && !reduced_motion {
                    let ResidentMotionTarget::Femc(surface) = &mut command.target else {
                        return Err(motion_error(SemanticMotionError::UnboundTarget));
                    };
                    let steps = (dt / MAX_FEMC_STEP_S).ceil() as u32;
                    let substep = dt / f64::from(steps);
                    let mut next = **state;
                    let mut frame = None;
                    for step in 1..=steps {
                        let excitation = if let Some(track) = recruitment.as_ref() {
                            track.sample((track.time + f64::from(step) * substep).min(MAX_TIME), None)?
                        } else {
                            *activation
                        };
                        frame = Some(controller.advance(&mut next, substep, excitation).map_err(motion_error)?);
                    }
                    surface.deform(controller, &next)?;
                    **state = next;
                    *latest = frame;
                    if let Some(track) = recruitment {
                        track.time = (track.time + dt).min(MAX_TIME);
                    }
                }
            }
        }
    command.target.advance_skin(dt, reduced_motion)
}

#[wasm_bindgen]
impl QualiaPortal {
    /// Start an EONT-authorized public kinematic macro on the mounted actor.
    /// Returns the number of bound rotation, morph-weight and blend-axis tracks.
    pub fn bind_hmc_semantic_macro(
        &mut self, hmc: &[u8], asset_key: &str, action_iri: &str, context_iri: &str, actor_group_iri: &str,
    ) -> Result<u32, JsValue> {
        let command = self.prepare_hmc_semantic_motion(
            hmc, asset_key, action_iri, context_iri, actor_group_iri, Program::Macro { rule_source: None },
        )?;
        self.install_semantic_macro(command)
    }

    /// Execute one strict N3 keyframe rule already present in the mounted Q42
    /// semantic model's rule sources for `context_iri`.
    pub fn bind_hmc_semantic_rule_macro(
        &mut self, hmc: &[u8], asset_key: &str, action_iri: &str, context_iri: &str, actor_group_iri: &str,
        rule_source: &str,
    ) -> Result<u32, JsValue> {
        let command = self.prepare_hmc_semantic_motion(
            hmc, asset_key, action_iri, context_iri, actor_group_iri, Program::Macro { rule_source: Some(rule_source) },
        )?;
        self.install_semantic_macro(command)
    }

    /// Start one Q42 and M64 torsional controller on the mounted actor.
    pub fn bind_hmc_semantic_feedback(
        &mut self, hmc: &[u8], asset_key: &str, material_key: &str, action_iri: &str,
        context_iri: &str, actor_group_iri: &str,
    ) -> Result<(), JsValue> {
        let command = self.prepare_hmc_semantic_motion(hmc, asset_key, action_iri, context_iri, actor_group_iri, Program::Feedback { material_key })?;
        self.clear_hmc_semantic_motion()?;
        self.semantic_motion.command = Some(command);
        Ok(())
    }

    /// Bind an admitted branch to its Etalon aeroelastic wind oscillator.
    pub fn bind_hmc_aeroelastic_wind(
        &mut self, hmc: &[u8], asset_key: &str, material_key: &str, action_iri: &str,
        context_iri: &str, actor_group_iri: &str,
    ) -> Result<(), JsValue> {
        let command = self.prepare_hmc_semantic_motion(hmc, asset_key, action_iri, context_iri, actor_group_iri, Program::Aeroelastic { material_key })?;
        self.clear_hmc_semantic_motion()?;
        self.semantic_motion.command = Some(command);
        Ok(())
    }

    /// Bind a Q42/M64 serial hinge chain rooted at `actor_group_iri` on the
    /// resident skin's first frame, whose chain pose must agree with bind.
    /// Joint IRIs are JSON strings in Q42 slot order.
    /// The solver waits for a base-frame metre target before its first step.
    pub fn bind_hmc_dca_chain(
        &mut self, hmc: &[u8], asset_key: &str, material_key: &str, action_iri: &str,
        context_iri: &str, actor_group_iri: &str, joint_iris_json: &str,
    ) -> Result<u32, JsValue> {
        let iris = parse_ordered_iris(joint_iris_json, 16)?;
        let joints = iris.iter().map(String::as_str).collect::<Vec<_>>();
        let program = Program::Dca { material_key, joints: &joints };
        let command = self.prepare_hmc_semantic_motion(hmc, asset_key, action_iri, context_iri, actor_group_iri, program)?;
        let count = command.target_ids.len() as u32;
        self.clear_hmc_semantic_motion()?;
        self.semantic_motion.command = Some(command);
        Ok(count)
    }

    /// Select a single base-frame target in metres on the next playback frame.
    pub fn set_hmc_dca_target(&mut self, x_m: f64, y_m: f64, z_m: f64) -> Result<(), JsValue> {
        let point = [x_m, y_m, z_m];
        if point.iter().any(|v| !v.is_finite()) { return Err(motion_error(SemanticMotionError::InvalidCommand)); }
        let Some(BoundCommand { playback: Playback::Dca { target, pair_residuals_m, .. }, source, .. }) = self.semantic_motion.command.as_mut() else {
            return Err(motion_error(SemanticMotionError::InvalidCommand));
        };
        if self.hmc_mesh_residency.current() != Some(*source) {
            return Err(motion_error(SemanticMotionError::UnboundTarget));
        }
        *target = Some(DcaTarget::Point(point));
        *pair_residuals_m = None;
        Ok(())
    }

    /// Select two base-frame target points in metres, with the three primary
    /// coordinates followed by the three secondary coordinates. The second
    /// point must match the Q42-authored marker separation.
    pub fn set_hmc_dca_target_pair(&mut self, targets_m: &[f64]) -> Result<(), JsValue> {
        let &[x, y, z, second_x, second_y, second_z] = targets_m else {
            return Err(motion_error(SemanticMotionError::InvalidCommand));
        };
        let points = [[x, y, z], [second_x, second_y, second_z]];
        let Some(BoundCommand { playback: Playback::Dca { controller, target, pair_residuals_m, .. }, source, .. }) = self.semantic_motion.command.as_mut() else {
            return Err(motion_error(SemanticMotionError::InvalidCommand));
        };
        if self.hmc_mesh_residency.current() != Some(*source) {
            return Err(motion_error(SemanticMotionError::UnboundTarget));
        }
        controller.validate_target_pair(points).map_err(motion_error)?;
        *target = Some(DcaTarget::Pair(points));
        *pair_residuals_m = None;
        Ok(())
    }

    /// Metre residuals from the last solved frame for the current pair,
    /// ordered as primary IK, primary tracking, secondary IK, secondary tracking.
    /// Returns an empty array until a paired frame is solved.
    pub fn hmc_dca_pair_residuals(&self) -> Result<Vec<f64>, JsValue> {
        let Some(BoundCommand { playback: Playback::Dca { pair_residuals_m, .. }, source, .. }) = self.semantic_motion.command.as_ref() else {
            return Err(motion_error(SemanticMotionError::InvalidCommand));
        };
        if self.hmc_mesh_residency.current() != Some(*source) {
            return Err(motion_error(SemanticMotionError::UnboundTarget));
        }
        Ok(pair_residuals_m.map(|residuals| residuals.to_vec()).unwrap_or_default())
    }

    /// Packed `[angle radians, angular velocity rad/s]` pairs in joint slot order.
    pub fn hmc_dca_state(&self) -> Result<Vec<f64>, JsValue> {
        let Some(BoundCommand { playback: Playback::Dca { state, .. }, source, .. }) = self.semantic_motion.command.as_ref() else {
            return Err(motion_error(SemanticMotionError::InvalidCommand));
        };
        if self.hmc_mesh_residency.current() != Some(*source) {
            return Err(motion_error(SemanticMotionError::UnboundTarget));
        }
        Ok(state.iter().flat_map(|row| [row.angle_radians, row.velocity_radians_s]).collect())
    }

    /// Bind fitted hydraulic nodes to resident skin joints or assembly leaves.
    /// The node and edge arguments are JSON string arrays in Q42 slot order.
    pub fn bind_hmc_hydraulic_motion(
        &mut self, hmc: &[u8], asset_key: &str, material_key: &str, action_iri: &str,
        context_iri: &str, actor_group_iri: &str, node_iris_json: &str, edge_iris_json: &str,
    ) -> Result<u32, JsValue> {
        let nodes = parse_ordered_iris(node_iris_json, 32)?;
        let edges = parse_ordered_iris(edge_iris_json, MAX_HYDRAULIC_EDGES)?;
        let nodes = nodes.iter().map(String::as_str).collect::<Vec<_>>();
        let edges = edges.iter().map(String::as_str).collect::<Vec<_>>();
        let program = Program::Hydraulic { material_key, nodes: &nodes, edges: &edges };
        let command = self.prepare_hmc_semantic_motion(hmc, asset_key, action_iri, context_iri, actor_group_iri, program)?;
        let count = command.target_ids.len() as u32;
        self.clear_hmc_semantic_motion()?;
        self.semantic_motion.command = Some(command);
        Ok(count)
    }

    /// Set persistent signed source flows in m³/s, one per admitted node.
    pub fn set_hmc_hydraulic_sources(&mut self, sources: &[f64]) -> Result<(), JsValue> {
        let Some(BoundCommand { playback: Playback::Hydraulic { source_m3_s, .. }, source, .. }) = self.semantic_motion.command.as_mut() else {
            return Err(motion_error(SemanticMotionError::InvalidCommand));
        };
        if self.hmc_mesh_residency.current() != Some(*source) {
            return Err(motion_error(SemanticMotionError::UnboundTarget));
        }
        if sources.len() != source_m3_s.len() || sources.iter().any(|v| !v.is_finite()) {
            return Err(motion_error(SemanticMotionError::InvalidCommand));
        }
        source_m3_s.copy_from_slice(sources);
        Ok(())
    }

    /// Pressure readings in Pa, in the supplied Q42 node slot order.
    pub fn hmc_hydraulic_pressures(&self) -> Result<Vec<f64>, JsValue> {
        let Some(BoundCommand { playback: Playback::Hydraulic { pressure_pa, .. }, source, .. }) = self.semantic_motion.command.as_ref() else {
            return Err(motion_error(SemanticMotionError::InvalidCommand));
        };
        if self.hmc_mesh_residency.current() != Some(*source) {
            return Err(motion_error(SemanticMotionError::UnboundTarget));
        }
        Ok(pressure_pa.to_vec())
    }

    /// Packed `[flow m³/s, wall shear Pa, conductance fraction]` triples in
    /// Q42 edge slot order. Returns an empty array until the first solved frame.
    pub fn hmc_hydraulic_edges(&self) -> Result<Vec<f64>, JsValue> {
        let Some(BoundCommand { playback: Playback::Hydraulic { flow, has_sample, .. }, source, .. }) = self.semantic_motion.command.as_ref() else {
            return Err(motion_error(SemanticMotionError::InvalidCommand));
        };
        if self.hmc_mesh_residency.current() != Some(*source) {
            return Err(motion_error(SemanticMotionError::UnboundTarget));
        }
        if !*has_sample { return Ok(Vec::new()); }
        Ok(flow.iter().flat_map(|edge| [edge.flow_m3_s, edge.wall_shear_pa, edge.conductance_fraction]).collect())
    }

    /// Bind an admitted six-DOF hull to a single unit-scale IOVR leaf whose
    /// authored rest pose agrees with the Q42 world pose in metres.
    pub fn bind_hmc_marine_vehicle(
        &mut self, hmc: &[u8], asset_key: &str, material_key: &str, action_iri: &str,
        context_iri: &str, actor_group_iri: &str,
    ) -> Result<(), JsValue> {
        let command = self.prepare_hmc_semantic_motion(
            hmc, asset_key, action_iri, context_iri, actor_group_iri, Program::Marine { material_key },
        )?;
        self.clear_hmc_semantic_motion()?;
        self.semantic_motion.command = Some(command);
        Ok(())
    }

    /// Set signed throttle and rudder fractions, each in `[-1,1]`.
    pub fn set_hmc_marine_control(&mut self, throttle: f32, rudder: f32) -> Result<(), JsValue> {
        if !(-1.0..=1.0).contains(&throttle) || !(-1.0..=1.0).contains(&rudder) {
            return Err(motion_error(SemanticMotionError::InvalidCommand));
        }
        let Some(BoundCommand { playback: Playback::Marine { control, .. }, source, .. }) = self.semantic_motion.command.as_mut() else {
            return Err(motion_error(SemanticMotionError::InvalidCommand));
        };
        if self.hmc_mesh_residency.current() != Some(*source) {
            return Err(motion_error(SemanticMotionError::UnboundTarget));
        }
        *control = MarineVehicleControl { throttle, rudder };
        Ok(())
    }

    /// Last solved sample in order: time, wxyz rotor, xyz metres, xyz m/s,
    /// body-local xyz rad/s, submerged m³, buoyancy N, propulsive W.
    /// Returns an empty array until the first solved frame.
    pub fn hmc_marine_state(&self) -> Result<Vec<f64>, JsValue> {
        let Some(BoundCommand { playback: Playback::Marine { latest, .. }, source, .. }) = self.semantic_motion.command.as_ref() else {
            return Err(motion_error(SemanticMotionError::InvalidCommand));
        };
        if self.hmc_mesh_residency.current() != Some(*source) {
            return Err(motion_error(SemanticMotionError::UnboundTarget));
        }
        let Some(step) = latest else { return Ok(Vec::new()); };
        let (rotation, translation) = step.world_pose.to_rotation_translation();
        Ok(std::iter::once(step.time_s)
            .chain(rotation).chain(translation)
            .chain(step.linear_velocity_m_s.map(f64::from)).chain(step.angular_velocity_rad_s.map(f64::from))
            .chain([step.submerged_volume_m3, step.buoyancy_n, step.propulsive_power_w].map(f64::from))
            .collect())
    }

    /// Bind a standalone quantized mesh's FEMC surface to an EONT-authorized
    /// Q42 cage and its verified M64 rest vertices.
    pub fn bind_hmc_femc_motion(
        &mut self, hmc: &[u8], asset_key: &str, material_key: &str, action_iri: &str,
        context_iri: &str, actor_group_iri: &str,
    ) -> Result<(), JsValue> {
        let command = self.prepare_hmc_semantic_motion(
            hmc, asset_key, action_iri, context_iri, actor_group_iri, Program::Femc { material_key, recruitment: None },
        )?;
        self.clear_hmc_semantic_motion()?;
        self.semantic_motion.command = Some(command);
        Ok(())
    }

    /// Run one Q42 `MuscleRecruitment` track targeting the admitted FEMC cage.
    /// An optional verified N3 source derives the track's keyframes at setup.
    pub fn bind_hmc_femc_macro(
        &mut self, hmc: &[u8], asset_key: &str, material_key: &str, action_iri: &str,
        context_iri: &str, actor_group_iri: &str, rule_source: Option<String>,
    ) -> Result<(), JsValue> {
        let recruitment = rule_source.as_deref().map_or(RecruitmentSource::Materialized, RecruitmentSource::Rule);
        let command = self.prepare_hmc_semantic_motion(
            hmc, asset_key, action_iri, context_iri, actor_group_iri,
            Program::Femc { material_key, recruitment: Some(recruitment) },
        )?;
        self.clear_hmc_semantic_motion()?;
        self.semantic_motion.command = Some(command);
        Ok(())
    }

    /// Set the persistent muscle activation fraction for the resident cage.
    pub fn set_hmc_femc_activation(&mut self, fraction: f64) -> Result<(), JsValue> {
        if !(0.0..=1.0).contains(&fraction) { return Err(motion_error(SemanticMotionError::InvalidCommand)); }
        let Some(BoundCommand { playback: Playback::Femc { activation, recruitment: None, .. }, source, .. }) = self.semantic_motion.command.as_mut() else {
            return Err(motion_error(SemanticMotionError::InvalidCommand));
        };
        if self.hmc_mesh_residency.current() != Some(*source) {
            return Err(motion_error(SemanticMotionError::UnboundTarget));
        }
        *activation = fraction;
        Ok(())
    }

    /// Last solved `[minimum determinant, maximum determinant, enthesis drift in m]`.
    /// Returns an empty array before the first solved frame.
    pub fn hmc_femc_frame(&self) -> Result<Vec<f64>, JsValue> {
        let Some(BoundCommand { playback: Playback::Femc { latest, .. }, source, .. }) = self.semantic_motion.command.as_ref() else {
            return Err(motion_error(SemanticMotionError::InvalidCommand));
        };
        if self.hmc_mesh_residency.current() != Some(*source) {
            return Err(motion_error(SemanticMotionError::UnboundTarget));
        }
        Ok(latest.map(|frame| vec![frame.determinant_min, frame.determinant_max, frame.enthesis_drift_m]).unwrap_or_default())
    }

    pub fn set_hmc_semantic_feedback_goal(&mut self, radians: f64) -> Result<(), JsValue> {
        if !radians.is_finite() { return Err(motion_error(SemanticMotionError::InvalidCommand)); }
        let Some(BoundCommand { playback: Playback::Feedback { goal, .. }, source, .. }) = self.semantic_motion.command.as_mut() else {
            return Err(motion_error(SemanticMotionError::InvalidCommand));
        };
        if self.hmc_mesh_residency.current() != Some(*source) {
            return Err(motion_error(SemanticMotionError::UnboundTarget));
        }
        *goal = radians;
        Ok(())
    }

    pub fn clear_hmc_semantic_motion(&mut self) -> Result<(), JsValue> {
        let Some(mut command) = self.semantic_motion.command.take() else { return Ok(()); };
        if self.hmc_mesh_residency.current() == Some(command.source) {
            command.target.reset(&command.target_ids)?;
            self.upload_semantic_pose(&command.target)?;
        }
        Ok(())
    }
}
