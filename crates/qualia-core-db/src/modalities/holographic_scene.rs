//! Holographic Scene State-Graph, Scoped Manifold Slicing, and Predicate Broadcast Triggers.
//!
//! Implements the zero-heap neuro-symbolic scene loop:
//! 1. Scoped State-Graph Slicing (`WholeScene`, `Topic`, `SubjectClass`, `ManifoldNeighborhood`)
//! 2. Compact CBOR-LD binary frame encoding/decoding into caller-supplied buffers
//! 3. Multi-second Animation Preset & Sub-Part Command `NQuin` packing/unpacking
//! 4. Zero-heap Predicate Broadcast Trigger evaluation across asset arrays with radial
//!    propagation delay, Allen Interval sequencing, and Defeater (`DEFEATER_BIT`) overrides
//! 5. Multi-vendor `.p64` neural extension slot selection with Tier-0 universal fallback

use crate::frame_layout::DEFEATER_BIT;
use crate::modalities::manifold::ManifoldCoordinate10D;
use crate::modalities::spatio_temporal::TemporalOp;
use crate::{q_hash, NQuin};

pub const OP_SCENE_PLAY_PRESET: u8 = 0x70;
pub const OP_SCENE_BLEND_SUBPART: u8 = 0x71;
pub const OP_SCENE_PREDICATE_TRIGGER: u8 = 0x72;
pub const OP_SCENE_DEFEATER_CANCEL: u8 = 0x73;

pub const CBOR_LD_SCENE_MAGIC: [u8; 4] = [0xD9, 0x05, 0x01, 0xA4];

/// Scope filter for querying a holographic slice of the scene state graph.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SceneSliceScope {
    /// Macro summary across the entire scene.
    WholeScene,
    /// Only assets and Quins participating in a specific topic/context (`q_hash(topic_iri)`).
    Topic(u64),
    /// Only assets belonging to a specific ontological class (`q_hash(class_iri)`).
    SubjectClass(u64),
    /// Assets within a 3D spatial radius and 10D epistemic weight threshold.
    ManifoldNeighborhood {
        center: [f32; 3],
        radius: f32,
        min_epistemic_weight: f32,
    },
}

/// Live state-graph node for one scene asset (`.10d` flora/fauna or CAD mechanical assembly).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssetStateNode {
    pub entity_did: u64,
    pub class_hash: u64,
    pub topic_hash: u64,
    pub position: [f32; 3],
    pub manifold: ManifoldCoordinate10D,
    pub active_preset_hash: u64,
    pub active_subpart_hash: u64,
    pub remaining_duration_ms: u32,
}

/// Declarative predicate broadcast rule emitted by the Scene AI or loaded from a `.p64` cartridge.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PredicateTriggerRule {
    pub trigger_event_hash: u64,
    pub target_class_hash: u64,
    pub target_topic_hash: u64,
    pub target_subpart_hash: u64,
    pub preset_hash: u64,
    pub follow_up_preset_hash: u64,
    pub temporal_op: TemporalOp,
    pub max_radius_m: f32,
    pub propagation_speed_mps: f32,
    pub duration_ms: u16,
    pub blend_in_ms: u8,
    pub amplitude_u8: u8,
}

/// Concrete multi-second animation preset command scheduled onto a matching asset.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScheduledPresetCommand {
    pub entity_did: u64,
    pub subpart_hash: u64,
    pub preset_hash: u64,
    pub follow_up_preset_hash: u64,
    pub temporal_op: TemporalOp,
    pub delay_ms: u32,
    pub duration_ms: u16,
    pub blend_in_ms: u8,
    pub amplitude_u8: u8,
    pub command_quin: NQuin,
}

/// Vendor/checkpoint slot descriptor inside a `.p64` neural extension container.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct P64ExtensionSlot {
    pub vendor_model_hash: u64,
    pub architecture_id: u32,
    pub has_recurrent_state_seed: bool,
    pub has_kv_prefix: bool,
    pub has_ple_table: bool,
    pub has_lora_weights: bool,
    pub byte_offset: u64,
    pub byte_len: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneGraphError {
    OutputBufferTooSmall,
    InvalidCborLdHeader,
    TruncatedCborLdPayload,
}

#[inline]
fn dist_3d(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

#[inline]
fn temporal_op_code(op: TemporalOp) -> u8 {
    match op {
        TemporalOp::Before => 1,
        TemporalOp::Meets => 2,
        TemporalOp::Overlaps => 3,
        TemporalOp::Starts => 4,
        TemporalOp::During => 5,
        TemporalOp::Finishes => 6,
        TemporalOp::Equals => 7,
    }
}

#[inline]
fn temporal_op_from_code(code: u8) -> TemporalOp {
    match code {
        1 => TemporalOp::Before,
        2 => TemporalOp::Meets,
        3 => TemporalOp::Overlaps,
        4 => TemporalOp::Starts,
        5 => TemporalOp::During,
        6 => TemporalOp::Finishes,
        _ => TemporalOp::Equals,
    }
}

/// Pack a multi-second animation preset command into a parity-valid 48-byte `NQuin`.
#[must_use]
pub fn pack_preset_command_quin(
    entity_did: u64,
    subpart_hash: u64,
    preset_hash: u64,
    temporal_op: TemporalOp,
    duration_ms: u16,
    blend_in_ms: u8,
    amplitude_u8: u8,
    scene_context_hash: u64,
    lamport_clock: u32,
) -> NQuin {
    let subject = entity_did & 0x7FFF_FFFF_FFFF_FFFF;
    let pred_hash = (subpart_hash & 0x007F_FFFF_FFFF_FFFF) << 16;
    let t_code = (temporal_op_code(temporal_op) as u64) << 8;
    let predicate = pred_hash | t_code | (OP_SCENE_PLAY_PRESET as u64);
    let object = preset_hash & 0x0FFF_FFFF_FFFF_FFFF;
    let context = scene_context_hash & 0x00FF_FFFF_FFFF_FFFF;
    let payload = ((duration_ms as u64) << 16)
        | ((blend_in_ms as u64) << 8)
        | (amplitude_u8 as u64);
    let clock = ((lamport_clock & 0x1FFF_FFFF) as u64) << 32;
    let metadata = clock | payload;
    let parity = subject ^ predicate ^ object ^ context;
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Pack a Defeater (`DEFEATER_BIT`) Quin that cancels or overrides predicate triggers for `entity_did`.
#[must_use]
pub fn pack_preset_defeater_quin(
    entity_did: u64,
    preset_hash: u64,
    scene_context_hash: u64,
    lamport_clock: u32,
) -> NQuin {
    let subject = entity_did & 0x7FFF_FFFF_FFFF_FFFF;
    let predicate = DEFEATER_BIT | (OP_SCENE_DEFEATER_CANCEL as u64);
    let object = preset_hash & 0x0FFF_FFFF_FFFF_FFFF;
    let context = scene_context_hash & 0x00FF_FFFF_FFFF_FFFF;
    let metadata = ((lamport_clock & 0x1FFF_FFFF) as u64) << 32;
    let parity = subject ^ predicate ^ object ^ context;
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Filter `assets` by `scope` into caller-supplied `out_indices` (zero-heap hot path).
pub fn slice_scene_state_graph(
    assets: &[AssetStateNode],
    scope: SceneSliceScope,
    out_indices: &mut [usize],
) -> Result<usize, SceneGraphError> {
    let mut written = 0usize;
    for (idx, asset) in assets.iter().enumerate() {
        let matches = match scope {
            SceneSliceScope::WholeScene => true,
            SceneSliceScope::Topic(t) => t == 0 || asset.topic_hash == t,
            SceneSliceScope::SubjectClass(c) => c == 0 || asset.class_hash == c,
            SceneSliceScope::ManifoldNeighborhood {
                center,
                radius,
                min_epistemic_weight,
            } => {
                dist_3d(asset.position, center) <= radius
                    && asset.manifold.epistemic_weight >= min_epistemic_weight
            }
        };
        if matches {
            if written >= out_indices.len() {
                return Err(SceneGraphError::OutputBufferTooSmall);
            }
            out_indices[written] = idx;
            written += 1;
        }
    }
    Ok(written)
}

/// Evaluate `rules` triggered by `(trigger_event_hash, trigger_origin)` across `assets`,
/// honoring `defeater_quins` (`DEFEATER_BIT`) and radial wave propagation delays (zero-heap).
#[allow(clippy::too_many_arguments)]
pub fn evaluate_predicate_triggers_zero_heap(
    assets: &[AssetStateNode],
    rules: &[PredicateTriggerRule],
    defeater_quins: &[NQuin],
    trigger_event_hash: u64,
    trigger_origin: [f32; 3],
    scene_context_hash: u64,
    lamport_clock: u32,
    out_commands: &mut [ScheduledPresetCommand],
) -> Result<usize, SceneGraphError> {
    let mut count = 0usize;
    for rule in rules {
        if rule.trigger_event_hash != 0 && rule.trigger_event_hash != trigger_event_hash {
            continue;
        }
        for asset in assets {
            if rule.target_class_hash != 0 && asset.class_hash != rule.target_class_hash {
                continue;
            }
            if rule.target_topic_hash != 0 && asset.topic_hash != rule.target_topic_hash {
                continue;
            }
            let d = dist_3d(asset.position, trigger_origin);
            if rule.max_radius_m > 0.0 && d > rule.max_radius_m {
                continue;
            }
            let masked_entity = asset.entity_did & 0x7FFF_FFFF_FFFF_FFFF;
            let masked_preset = rule.preset_hash & 0x0FFF_FFFF_FFFF_FFFF;
            let is_defeated = defeater_quins.iter().any(|q| {
                (q.predicate & DEFEATER_BIT) != 0
                    && (q.subject & 0x7FFF_FFFF_FFFF_FFFF) == masked_entity
                    && (q.object == 0 || (q.object & 0x0FFF_FFFF_FFFF_FFFF) == masked_preset)
            });
            if is_defeated {
                continue;
            }
            if count >= out_commands.len() {
                return Err(SceneGraphError::OutputBufferTooSmall);
            }
            let delay_ms = if rule.propagation_speed_mps > 0.0 {
                ((d / rule.propagation_speed_mps) * 1000.0).round() as u32
            } else {
                0
            };
            let command_quin = pack_preset_command_quin(
                asset.entity_did,
                rule.target_subpart_hash,
                rule.preset_hash,
                rule.temporal_op,
                rule.duration_ms,
                rule.blend_in_ms,
                rule.amplitude_u8,
                scene_context_hash,
                lamport_clock,
            );
            out_commands[count] = ScheduledPresetCommand {
                entity_did: asset.entity_did,
                subpart_hash: rule.target_subpart_hash,
                preset_hash: rule.preset_hash,
                follow_up_preset_hash: rule.follow_up_preset_hash,
                temporal_op: rule.temporal_op,
                delay_ms,
                duration_ms: rule.duration_ms,
                blend_in_ms: rule.blend_in_ms,
                amplitude_u8: rule.amplitude_u8,
                command_quin,
            };
            count += 1;
        }
    }
    Ok(count)
}

/// Encode a scoped slice of `AssetStateNode`s into a compact CBOR-LD binary frame (zero-heap).
/// Each record is 44 bytes after the 8-byte header (`CBOR_LD_SCENE_MAGIC` + `u32` count).
pub fn encode_scene_slice_cbor_ld_into(
    assets: &[AssetStateNode],
    slice_indices: &[usize],
    out_bytes: &mut [u8],
) -> Result<usize, SceneGraphError> {
    const RECORD_BYTES: usize = 44;
    let needed = 8 + slice_indices.len() * RECORD_BYTES;
    if out_bytes.len() < needed {
        return Err(SceneGraphError::OutputBufferTooSmall);
    }
    out_bytes[0..4].copy_from_slice(&CBOR_LD_SCENE_MAGIC);
    out_bytes[4..8].copy_from_slice(&(slice_indices.len() as u32).to_le_bytes());
    let mut off = 8usize;
    for &idx in slice_indices {
        let a = &assets[idx];
        out_bytes[off..off + 8].copy_from_slice(&a.entity_did.to_le_bytes());
        out_bytes[off + 8..off + 16].copy_from_slice(&a.class_hash.to_le_bytes());
        out_bytes[off + 16..off + 24].copy_from_slice(&a.topic_hash.to_le_bytes());
        out_bytes[off + 24..off + 28].copy_from_slice(&a.position[0].to_le_bytes());
        out_bytes[off + 28..off + 32].copy_from_slice(&a.position[1].to_le_bytes());
        out_bytes[off + 32..off + 36].copy_from_slice(&a.position[2].to_le_bytes());
        out_bytes[off + 36..off + 44].copy_from_slice(&a.active_preset_hash.to_le_bytes());
        off += RECORD_BYTES;
    }
    Ok(off)
}

/// Decode compact CBOR-LD binary trigger rules emitted by the Scene AI into `out_rules` (zero-heap).
pub fn decode_trigger_rules_cbor_ld_into(
    src: &[u8],
    out_rules: &mut [PredicateTriggerRule],
) -> Result<usize, SceneGraphError> {
    const RULE_BYTES: usize = 60;
    if src.len() < 8 || src[0..4] != CBOR_LD_SCENE_MAGIC {
        return Err(SceneGraphError::InvalidCborLdHeader);
    }
    let count = u32::from_le_bytes(src[4..8].try_into().unwrap_or([0; 4])) as usize;
    if src.len() < 8 + count * RULE_BYTES {
        return Err(SceneGraphError::TruncatedCborLdPayload);
    }
    if count > out_rules.len() {
        return Err(SceneGraphError::OutputBufferTooSmall);
    }
    for i in 0..count {
        let base = 8 + i * RULE_BYTES;
        let u64_at = |o: usize| u64::from_le_bytes(src[base + o..base + o + 8].try_into().unwrap());
        let f32_at = |o: usize| f32::from_le_bytes(src[base + o..base + o + 4].try_into().unwrap());
        out_rules[i] = PredicateTriggerRule {
            trigger_event_hash: u64_at(0),
            target_class_hash: u64_at(8),
            target_topic_hash: u64_at(16),
            target_subpart_hash: u64_at(24),
            preset_hash: u64_at(32),
            follow_up_preset_hash: u64_at(40),
            max_radius_m: f32_at(48),
            propagation_speed_mps: f32_at(52),
            duration_ms: u16::from_le_bytes(src[base + 56..base + 58].try_into().unwrap()),
            blend_in_ms: src[base + 58],
            amplitude_u8: temporal_op_code(temporal_op_from_code(src[base + 59])),
            temporal_op: temporal_op_from_code(src[base + 59]),
        };
    }
    Ok(count)
}

/// Select the best matching vendor/model neural extension slot from a `.p64` cartridge,
/// returning `None` when the caller should use the Tier-0 universal CBOR-LD + `NQuin` fallback.
#[must_use]
pub fn select_p64_vendor_slot<'a>(
    slots: &'a [P64ExtensionSlot],
    preferred_model_id: &str,
    architecture_id: u32,
) -> Option<&'a P64ExtensionSlot> {
    let target_hash = q_hash(preferred_model_id);
    if let Some(exact) = slots.iter().find(|s| s.vendor_model_hash == target_hash) {
        return Some(exact);
    }
    slots
        .iter()
        .find(|s| s.architecture_id != 0 && s.architecture_id == architecture_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifold_slice_and_predicate_broadcast_trigger_across_assets() {
        let class_macropod = q_hash("Fauna:Macropod");
        let class_tree = q_hash("Flora:Corymbia");
        let topic_alert = q_hash("topic:acoustic_alert");
        let event_thump = q_hash("event:hindfoot_alarm_thump");
        let subpart_ears = q_hash("subpart:pinnae_both");
        let preset_ear_swivel = q_hash("preset:ear_radar_swivel");
        let preset_bound = q_hash("preset:saltatorial_bound");

        let mut m_high = ManifoldCoordinate10D::default();
        m_high.epistemic_weight = 0.9;

        let assets = [
            AssetStateNode {
                entity_did: q_hash("did:q42:wallaby_01"),
                class_hash: class_macropod,
                topic_hash: topic_alert,
                position: [10.0, 0.0, 0.0],
                manifold: m_high,
                active_preset_hash: 0,
                active_subpart_hash: 0,
                remaining_duration_ms: 0,
            },
            AssetStateNode {
                entity_did: q_hash("did:q42:wallaby_02"),
                class_hash: class_macropod,
                topic_hash: topic_alert,
                position: [34.3, 0.0, 0.0],
                manifold: m_high,
                active_preset_hash: 0,
                active_subpart_hash: 0,
                remaining_duration_ms: 0,
            },
            AssetStateNode {
                entity_did: q_hash("did:q42:corymbia_01"),
                class_hash: class_tree,
                topic_hash: topic_alert,
                position: [5.0, 0.0, 0.0],
                manifold: m_high,
                active_preset_hash: 0,
                active_subpart_hash: 0,
                remaining_duration_ms: 0,
            },
        ];

        let mut slice_idx = [0usize; 8];
        let n_sliced = slice_scene_state_graph(
            &assets,
            SceneSliceScope::ManifoldNeighborhood {
                center: [0.0, 0.0, 0.0],
                radius: 40.0,
                min_epistemic_weight: 0.5,
            },
            &mut slice_idx,
        )
        .unwrap();
        assert_eq!(n_sliced, 3);

        let mut cbor_buf = [0u8; 256];
        let bytes_written =
            encode_scene_slice_cbor_ld_into(&assets, &slice_idx[..n_sliced], &mut cbor_buf)
                .unwrap();
        assert_eq!(bytes_written, 8 + 3 * 44);

        let rules = [PredicateTriggerRule {
            trigger_event_hash: event_thump,
            target_class_hash: class_macropod,
            target_topic_hash: topic_alert,
            target_subpart_hash: subpart_ears,
            preset_hash: preset_ear_swivel,
            follow_up_preset_hash: preset_bound,
            temporal_op: TemporalOp::Meets,
            max_radius_m: 50.0,
            propagation_speed_mps: 343.0,
            duration_ms: 4000,
            blend_in_ms: 120,
            amplitude_u8: 255,
        }];

        let mut cmds = [ScheduledPresetCommand {
            entity_did: 0,
            subpart_hash: 0,
            preset_hash: 0,
            follow_up_preset_hash: 0,
            temporal_op: TemporalOp::Equals,
            delay_ms: 0,
            duration_ms: 0,
            blend_in_ms: 0,
            amplitude_u8: 0,
            command_quin: NQuin::default(),
        }; 8];

        let fired = evaluate_predicate_triggers_zero_heap(
            &assets,
            &rules,
            &[],
            event_thump,
            [0.0, 0.0, 0.0],
            q_hash("scene:bushland_01"),
            1,
            &mut cmds,
        )
        .unwrap();
        assert_eq!(fired, 2);
        assert_eq!(cmds[0].delay_ms, 29);
        assert_eq!(cmds[1].delay_ms, 100);
        assert_eq!(
            cmds[0].command_quin.parity,
            cmds[0].command_quin.subject
                ^ cmds[0].command_quin.predicate
                ^ cmds[0].command_quin.object
                ^ cmds[0].command_quin.context
        );

        // Now test Defeater override on wallaby_02
        let defeater = [pack_preset_defeater_quin(
            assets[1].entity_did,
            preset_ear_swivel,
            q_hash("scene:bushland_01"),
            2,
        )];
        let fired_after_defeat = evaluate_predicate_triggers_zero_heap(
            &assets,
            &rules,
            &defeater,
            event_thump,
            [0.0, 0.0, 0.0],
            q_hash("scene:bushland_01"),
            2,
            &mut cmds,
        )
        .unwrap();
        assert_eq!(fired_after_defeat, 1);
        assert_eq!(cmds[0].entity_did, assets[0].entity_did);
    }

    #[test]
    fn p64_vendor_slot_selection_and_tier0_fallback() {
        let slots = [
            P64ExtensionSlot {
                vendor_model_hash: q_hash("qwen3.5:0.8b"),
                architecture_id: 10,
                has_recurrent_state_seed: true,
                has_kv_prefix: true,
                has_ple_table: false,
                has_lora_weights: true,
                byte_offset: 4096,
                byte_len: 1048576,
            },
            P64ExtensionSlot {
                vendor_model_hash: q_hash("gemma4:e4b"),
                architecture_id: 20,
                has_recurrent_state_seed: false,
                has_kv_prefix: true,
                has_ple_table: true,
                has_lora_weights: false,
                byte_offset: 2097152,
                byte_len: 524288,
            },
        ];
        let matched = select_p64_vendor_slot(&slots, "qwen3.5:0.8b", 10).unwrap();
        assert!(matched.has_recurrent_state_seed);
        assert!(select_p64_vendor_slot(&slots, "gemini:2.5-flash", 99).is_none());
    }
}
