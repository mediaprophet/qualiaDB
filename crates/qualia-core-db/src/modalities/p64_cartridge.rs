//! `.p64` Neural Extension Cartridge binary packer, unpacker, and recurrent-state/KV seed bridge.
//!
//! A `.p64` neural extension cartridge attaches to any non-`.p64` mechanical CAD assembly or `.10d`
//! flora/fauna asset and carries two tiers in a single deterministic binary container:
//! - **Tier 0 (Vendor-Agnostic Universal Payload):** compact CBOR-LD `PredicateTriggerRule` table
//!   and `NQuin` guardrails that any model (`Qwen`, `Gemma`, `Gemini`, etc.) can consume.
//! - **Tier 1 (Vendor/Model-Keyed Neural Slots):** zero-copy aligned byte ranges containing
//!   pre-warmed `Qwen3.5` `GatedDeltaNet` recurrent state seeds (`f32`), KV prefix snapshots,
//!   `Gemma 4` PLE tables, or LoRA deltas.

use crate::modalities::holographic_scene::{
    decode_trigger_rules_cbor_ld_into, select_p64_vendor_slot, P64ExtensionSlot,
    PredicateTriggerRule, SceneGraphError, CBOR_LD_SCENE_MAGIC,
};
use crate::NQuin;

pub const P64_EXT_CARTRIDGE_MAGIC: [u8; 8] = *b"P64EXT01";
const HEADER_BYTES: usize = 32;
const SLOT_RECORD_BYTES: usize = 32;
const RULE_BYTES: usize = 60;
const QUIN_BYTES: usize = 48;

const FLAG_RECURRENT_SEED: u8 = 1 << 0;
const FLAG_KV_PREFIX: u8 = 1 << 1;
const FLAG_PLE_TABLE: u8 = 1 << 2;
const FLAG_LORA_WEIGHTS: u8 = 1 << 3;

/// Parsed header of a `.p64` neural extension cartridge.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct P64ExtensionHeader {
    pub asset_did_hash: u64,
    pub tier0_rule_count: u16,
    pub tier0_quin_count: u16,
    pub tier1_slot_count: u16,
    pub flags: u16,
    pub tier0_offset: u32,
    pub tier1_dir_offset: u32,
}

#[inline]
fn temporal_op_code(op: crate::modalities::spatio_temporal::TemporalOp) -> u8 {
    use crate::modalities::spatio_temporal::TemporalOp;
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

/// Encode `PredicateTriggerRule`s into the compact 60-byte-per-rule CBOR-LD binary frame (zero-heap).
pub fn encode_trigger_rules_cbor_ld_into(
    rules: &[PredicateTriggerRule],
    out_bytes: &mut [u8],
) -> Result<usize, SceneGraphError> {
    let needed = 8 + rules.len() * RULE_BYTES;
    if out_bytes.len() < needed {
        return Err(SceneGraphError::OutputBufferTooSmall);
    }
    out_bytes[0..4].copy_from_slice(&CBOR_LD_SCENE_MAGIC);
    out_bytes[4..8].copy_from_slice(&(rules.len() as u32).to_le_bytes());
    let mut off = 8usize;
    for r in rules {
        out_bytes[off..off + 8].copy_from_slice(&r.trigger_event_hash.to_le_bytes());
        out_bytes[off + 8..off + 16].copy_from_slice(&r.target_class_hash.to_le_bytes());
        out_bytes[off + 16..off + 24].copy_from_slice(&r.target_topic_hash.to_le_bytes());
        out_bytes[off + 24..off + 32].copy_from_slice(&r.target_subpart_hash.to_le_bytes());
        out_bytes[off + 32..off + 40].copy_from_slice(&r.preset_hash.to_le_bytes());
        out_bytes[off + 40..off + 48].copy_from_slice(&r.follow_up_preset_hash.to_le_bytes());
        out_bytes[off + 48..off + 52].copy_from_slice(&r.max_radius_m.to_le_bytes());
        out_bytes[off + 52..off + 56].copy_from_slice(&r.propagation_speed_mps.to_le_bytes());
        out_bytes[off + 56..off + 58].copy_from_slice(&r.duration_ms.to_le_bytes());
        out_bytes[off + 58] = r.blend_in_ms;
        out_bytes[off + 59] = temporal_op_code(r.temporal_op);
        off += RULE_BYTES;
    }
    Ok(off)
}

/// Pack a `.p64` neural extension cartridge (Tier-0 CBOR-LD rules + `NQuin` guardrails + Tier-1
/// vendor/model slot directory and f32 recurrent-state/KV payloads) into caller-supplied `out_bytes`.
pub fn pack_p64_extension_cartridge_into(
    asset_did_hash: u64,
    tier0_rules: &[PredicateTriggerRule],
    tier0_quins: &[NQuin],
    tier1_slots: &[(P64ExtensionSlot, &[f32])],
    out_bytes: &mut [u8],
) -> Result<usize, SceneGraphError> {
    let tier0_rules_bytes = 8 + tier0_rules.len() * RULE_BYTES;
    let tier0_quins_bytes = tier0_quins.len() * QUIN_BYTES;
    let tier1_dir_offset = (HEADER_BYTES + tier0_rules_bytes + tier0_quins_bytes + 7) & !7;
    let payload_start = tier1_dir_offset + tier1_slots.len() * SLOT_RECORD_BYTES;
    let total_payload_bytes: usize = tier1_slots.iter().map(|(_, s)| s.len() * 4).sum();
    let total_needed = payload_start + total_payload_bytes;
    if out_bytes.len() < total_needed {
        return Err(SceneGraphError::OutputBufferTooSmall);
    }
    out_bytes[..total_needed].fill(0);
    out_bytes[0..8].copy_from_slice(&P64_EXT_CARTRIDGE_MAGIC);
    out_bytes[8..16].copy_from_slice(&asset_did_hash.to_le_bytes());
    out_bytes[16..18].copy_from_slice(&(tier0_rules.len() as u16).to_le_bytes());
    out_bytes[18..20].copy_from_slice(&(tier0_quins.len() as u16).to_le_bytes());
    out_bytes[20..22].copy_from_slice(&(tier1_slots.len() as u16).to_le_bytes());
    out_bytes[22..24].copy_from_slice(&0u16.to_le_bytes());
    out_bytes[24..28].copy_from_slice(&(HEADER_BYTES as u32).to_le_bytes());
    out_bytes[28..32].copy_from_slice(&(tier1_dir_offset as u32).to_le_bytes());

    let written_rules = encode_trigger_rules_cbor_ld_into(
        tier0_rules,
        &mut out_bytes[HEADER_BYTES..HEADER_BYTES + tier0_rules_bytes],
    )?;
    let mut q_off = HEADER_BYTES + written_rules;
    for q in tier0_quins {
        out_bytes[q_off..q_off + 8].copy_from_slice(&q.subject.to_le_bytes());
        out_bytes[q_off + 8..q_off + 16].copy_from_slice(&q.predicate.to_le_bytes());
        out_bytes[q_off + 16..q_off + 24].copy_from_slice(&q.object.to_le_bytes());
        out_bytes[q_off + 24..q_off + 32].copy_from_slice(&q.context.to_le_bytes());
        out_bytes[q_off + 32..q_off + 40].copy_from_slice(&q.metadata.to_le_bytes());
        out_bytes[q_off + 40..q_off + 48].copy_from_slice(&q.parity.to_le_bytes());
        q_off += QUIN_BYTES;
    }

    let mut cur_payload_off = payload_start;
    for (idx, (slot, floats)) in tier1_slots.iter().enumerate() {
        let dir_off = tier1_dir_offset + idx * SLOT_RECORD_BYTES;
        let byte_len = floats.len() * 4;
        let mut flags = 0u8;
        if slot.has_recurrent_state_seed {
            flags |= FLAG_RECURRENT_SEED;
        }
        if slot.has_kv_prefix {
            flags |= FLAG_KV_PREFIX;
        }
        if slot.has_ple_table {
            flags |= FLAG_PLE_TABLE;
        }
        if slot.has_lora_weights {
            flags |= FLAG_LORA_WEIGHTS;
        }
        out_bytes[dir_off..dir_off + 8].copy_from_slice(&slot.vendor_model_hash.to_le_bytes());
        out_bytes[dir_off + 8..dir_off + 12].copy_from_slice(&slot.architecture_id.to_le_bytes());
        out_bytes[dir_off + 12] = flags;
        out_bytes[dir_off + 16..dir_off + 24]
            .copy_from_slice(&(cur_payload_off as u64).to_le_bytes());
        out_bytes[dir_off + 24..dir_off + 32].copy_from_slice(&(byte_len as u64).to_le_bytes());

        for (i, &val) in floats.iter().enumerate() {
            let fo = cur_payload_off + i * 4;
            out_bytes[fo..fo + 4].copy_from_slice(&val.to_le_bytes());
        }
        cur_payload_off += byte_len;
    }
    Ok(cur_payload_off)
}

/// Unpack a `.p64` neural extension cartridge into caller-supplied Tier-0 and Tier-1 buffers (zero-heap).
pub fn unpack_p64_extension_cartridge_into(
    cartridge_bytes: &[u8],
    out_rules: &mut [PredicateTriggerRule],
    out_quins: &mut [NQuin],
    out_slots: &mut [P64ExtensionSlot],
) -> Result<P64ExtensionHeader, SceneGraphError> {
    if cartridge_bytes.len() < HEADER_BYTES || cartridge_bytes[0..8] != P64_EXT_CARTRIDGE_MAGIC {
        return Err(SceneGraphError::InvalidCborLdHeader);
    }
    let asset_did_hash = u64::from_le_bytes(cartridge_bytes[8..16].try_into().unwrap());
    let tier0_rule_count = u16::from_le_bytes(cartridge_bytes[16..18].try_into().unwrap());
    let tier0_quin_count = u16::from_le_bytes(cartridge_bytes[18..20].try_into().unwrap());
    let tier1_slot_count = u16::from_le_bytes(cartridge_bytes[20..22].try_into().unwrap());
    let flags = u16::from_le_bytes(cartridge_bytes[22..24].try_into().unwrap());
    let tier0_offset = u32::from_le_bytes(cartridge_bytes[24..28].try_into().unwrap());
    let tier1_dir_offset = u32::from_le_bytes(cartridge_bytes[28..32].try_into().unwrap());

    if (tier0_rule_count as usize) > out_rules.len()
        || (tier0_quin_count as usize) > out_quins.len()
        || (tier1_slot_count as usize) > out_slots.len()
    {
        return Err(SceneGraphError::OutputBufferTooSmall);
    }

    let t0_off = tier0_offset as usize;
    let rules_len_bytes = 8 + (tier0_rule_count as usize) * RULE_BYTES;
    let quins_len_bytes = (tier0_quin_count as usize) * QUIN_BYTES;
    if t0_off + rules_len_bytes + quins_len_bytes > cartridge_bytes.len() {
        return Err(SceneGraphError::TruncatedCborLdPayload);
    }
    decode_trigger_rules_cbor_ld_into(
        &cartridge_bytes[t0_off..t0_off + rules_len_bytes],
        &mut out_rules[..tier0_rule_count as usize],
    )?;

    let q_start = t0_off + rules_len_bytes;
    for i in 0..tier0_quin_count as usize {
        let bo = q_start + i * QUIN_BYTES;
        let u64_at = |o: usize| {
            u64::from_le_bytes(cartridge_bytes[bo + o..bo + o + 8].try_into().unwrap())
        };
        out_quins[i] = NQuin {
            subject: u64_at(0),
            predicate: u64_at(8),
            object: u64_at(16),
            context: u64_at(24),
            metadata: u64_at(32),
            parity: u64_at(40),
        };
    }

    let t1_off = tier1_dir_offset as usize;
    if t1_off + (tier1_slot_count as usize) * SLOT_RECORD_BYTES > cartridge_bytes.len() {
        return Err(SceneGraphError::TruncatedCborLdPayload);
    }
    for i in 0..tier1_slot_count as usize {
        let so = t1_off + i * SLOT_RECORD_BYTES;
        let vendor_model_hash =
            u64::from_le_bytes(cartridge_bytes[so..so + 8].try_into().unwrap());
        let architecture_id =
            u32::from_le_bytes(cartridge_bytes[so + 8..so + 12].try_into().unwrap());
        let sflags = cartridge_bytes[so + 12];
        let byte_offset =
            u64::from_le_bytes(cartridge_bytes[so + 16..so + 24].try_into().unwrap());
        let byte_len = u64::from_le_bytes(cartridge_bytes[so + 24..so + 32].try_into().unwrap());
        out_slots[i] = P64ExtensionSlot {
            vendor_model_hash,
            architecture_id,
            has_recurrent_state_seed: (sflags & FLAG_RECURRENT_SEED) != 0,
            has_kv_prefix: (sflags & FLAG_KV_PREFIX) != 0,
            has_ple_table: (sflags & FLAG_PLE_TABLE) != 0,
            has_lora_weights: (sflags & FLAG_LORA_WEIGHTS) != 0,
            byte_offset,
            byte_len,
        };
    }

    Ok(P64ExtensionHeader {
        asset_did_hash,
        tier0_rule_count,
        tier0_quin_count,
        tier1_slot_count,
        flags,
        tier0_offset,
        tier1_dir_offset,
    })
}

/// Extract the `f32` recurrent-state or KV seed payload for a matched `P64ExtensionSlot` into `out_floats` (zero-heap).
pub fn load_p64_slot_f32_seed_into(
    cartridge_bytes: &[u8],
    slot: &P64ExtensionSlot,
    out_floats: &mut [f32],
) -> Result<usize, SceneGraphError> {
    let off = slot.byte_offset as usize;
    let len = slot.byte_len as usize;
    if off + len > cartridge_bytes.len() || len % 4 != 0 {
        return Err(SceneGraphError::TruncatedCborLdPayload);
    }
    let count = len / 4;
    if out_floats.len() < count {
        return Err(SceneGraphError::OutputBufferTooSmall);
    }
    for i in 0..count {
        let bo = off + i * 4;
        out_floats[i] = f32::from_le_bytes(cartridge_bytes[bo..bo + 4].try_into().unwrap());
    }
    Ok(count)
}

/// Mount a `.p64` neural extension cartridge onto a `QTensorEngine` when a matching Tier-1
/// recurrent-state seed slot exists for `(preferred_model_id, architecture_id)`.
/// Even when no Tier-1 vendor slot matches (e.g., running on a cloud or different vendor LLM),
/// the Tier-0 `PredicateTriggerRule`s and `NQuin` guardrails are unpacked and returned.
pub fn mount_p64_cartridge_onto_engine(
    cartridge_bytes: &[u8],
    preferred_model_id: &str,
    architecture_id: u32,
    engine: &mut crate::gguf_bridge::QTensorEngine,
    out_rules: &mut [PredicateTriggerRule],
    out_quins: &mut [NQuin],
    out_slots: &mut [P64ExtensionSlot],
    scratch_seed: &mut [f32],
) -> Result<(P64ExtensionHeader, bool), SceneGraphError> {
    let header = unpack_p64_extension_cartridge_into(
        cartridge_bytes,
        out_rules,
        out_quins,
        out_slots,
    )?;
    let valid_slots = &out_slots[..header.tier1_slot_count as usize];
    if let Some(matched) = select_p64_vendor_slot(valid_slots, preferred_model_id, architecture_id)
    {
        if matched.has_recurrent_state_seed {
            let n = load_p64_slot_f32_seed_into(cartridge_bytes, matched, scratch_seed)?;
            let restored = engine.restore_recurrent_state_from_slice(&scratch_seed[..n]);
            return Ok((header, restored));
        }
    }
    Ok((header, false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modalities::holographic_scene::pack_preset_defeater_quin;
    use crate::modalities::spatio_temporal::TemporalOp;
    use crate::q_hash;

    #[test]
    fn pack_unpack_and_mount_p64_extension_cartridge_zero_heap() {
        let asset_did = q_hash("did:q42:mechanical_windmill_01");
        let rule = PredicateTriggerRule {
            trigger_event_hash: q_hash("event:gust_front_arrival"),
            target_class_hash: q_hash("CAD:WindTurbineAssembly"),
            target_topic_hash: q_hash("topic:aerodynamic_load"),
            target_subpart_hash: q_hash("subpart:rotor_hub_pitch"),
            preset_hash: q_hash("preset:feather_blades_30deg"),
            follow_up_preset_hash: q_hash("preset:engage_yaw_brake"),
            temporal_op: TemporalOp::Meets,
            max_radius_m: 120.0,
            propagation_speed_mps: 25.0,
            duration_ms: 6000,
            blend_in_ms: 200,
            amplitude_u8: 220,
        };
        let guardrail = pack_preset_defeater_quin(
            asset_did,
            q_hash("preset:overspeed_spin"),
            q_hash("scene:outback_station"),
            1,
        );
        let qwen_seed = [0.25f32, -0.5, 0.75, 1.25, -0.125, 0.5, 0.875, -0.25];
        let qwen_slot = P64ExtensionSlot {
            vendor_model_hash: q_hash("qwen3.5:0.8b"),
            architecture_id: 10,
            has_recurrent_state_seed: true,
            has_kv_prefix: false,
            has_ple_table: false,
            has_lora_weights: false,
            byte_offset: 0,
            byte_len: 0,
        };

        let mut buf = [0u8; 512];
        let total_bytes = pack_p64_extension_cartridge_into(
            asset_did,
            &[rule],
            &[guardrail],
            &[(qwen_slot, &qwen_seed)],
            &mut buf,
        )
        .unwrap();
        assert!(total_bytes <= buf.len());

        let mut engine = crate::gguf_bridge::QTensorEngine::new();
        // Simulate an allocated 8-float recurrent state on the engine
        let mut out_rules = [rule; 2];
        let mut out_quins = [NQuin::default(); 2];
        let mut out_slots = [qwen_slot; 2];
        let mut scratch = [0.0f32; 16];

        let (hdr, tier1_mounted) = mount_p64_cartridge_onto_engine(
            &buf[..total_bytes],
            "gemini:2.5-flash",
            99,
            &mut engine,
            &mut out_rules,
            &mut out_quins,
            &mut out_slots,
            &mut scratch,
        )
        .unwrap();
        assert_eq!(hdr.tier0_rule_count, 1);
        assert_eq!(hdr.tier0_quin_count, 1);
        assert!(!tier1_mounted, "Unmatched vendor should fall back cleanly to Tier-0");
        assert_eq!(out_rules[0].preset_hash, rule.preset_hash);
        assert_eq!(out_quins[0].parity, guardrail.parity);
    }
}
