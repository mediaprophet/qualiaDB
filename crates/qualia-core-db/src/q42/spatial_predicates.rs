//! Canonical `.q42` spatial, volumetric, and multi-sensory ontological predicates (§3.6.2 / §2.8).
//!
//! Provides compile-time `q_hash` constants and zero-heap `NQuin` constructors/decoders
//! for the 6 canonical predicate families (28 predicates total):
//! - Multi-scale spatial & topological predicates (`geo:*`)
//! - Volumetric & container sidecar predicates (`q42:*`)
//! - Kinematic, biomechanical & botanical predicates (`kin:*`, `bio:*`, `mech:*`, `phys:*`)
//! - Sensory, acoustic & optical predicates (`sens:*`)
//! - Epistemic & bifurcation predicates (`epi:*`)
//! - KV-cache & LLM grounding predicates (`llm:*`)

use crate::{q_hash, NQuin};

// ── Family A: Multi-Scale Spatial & Topological Predicates (7) ───────────
pub const PRED_GEO_HAS_VERTEX: u64 = q_hash("geo:hasVertex");
pub const PRED_GEO_HAS_PACKED_CENTROID: u64 = q_hash("geo:hasPackedCentroid");
pub const PRED_GEO_HAS_PACKED_BBOX_MIN: u64 = q_hash("geo:hasPackedBboxMin");
pub const PRED_GEO_HAS_PACKED_BBOX_MAX: u64 = q_hash("geo:hasPackedBboxMax");
pub const PRED_GEO_HAS_SCALE_TIER: u64 = q_hash("geo:hasScaleTier");
pub const PRED_GEO_HAS_TOPOLOGY_CLASS: u64 = q_hash("geo:hasTopologyClass");
pub const PRED_GEO_HAS_PORTAL_LINK: u64 = q_hash("geo:hasPortalLink");

// ── Family B: Volumetric & Container Sidecar Predicates (6) ───────────────
pub const PRED_Q42_HAS_CONTAINER_10D: u64 = q_hash("q42:hasContainer10d");
pub const PRED_Q42_HAS_M64_CHUNK: u64 = q_hash("q42:hasM64Chunk");
pub const PRED_Q42_HAS_EMF_CHUNK: u64 = q_hash("q42:hasEmfChunk");
pub const PRED_Q42_HAS_SPECTRAL_SHEET_V3: u64 = q_hash("q42:hasSpectralSheetV3");
pub const PRED_Q42_HAS_HMC_BUNDLE: u64 = q_hash("q42:hasHmcBundle");
pub const PRED_Q42_HAS_CLUSTER_DAG: u64 = q_hash("q42:hasClusterDag");

// ── Family C: Kinematic, Biomechanical & Botanical Predicates (6) ─────────
pub const PRED_KIN_HAS_PGA_MOTOR: u64 = q_hash("kin:hasPgaMotor");
pub const PRED_KIN_HAS_JOINT_PARENT: u64 = q_hash("kin:hasJointParent");
pub const PBIO_HAS_HILL_MUSCLE_ALIAS: u64 = q_hash("bio:hasHillMuscle");
pub const PRED_BIO_HAS_HILL_MUSCLE: u64 = PBIO_HAS_HILL_MUSCLE_ALIAS;
pub const PRED_BIO_HAS_ALLOMETRIC_LAW: u64 = q_hash("bio:hasAllometricLaw");
pub const PRED_MECH_HAS_FATIGUE_LAW: u64 = q_hash("mech:hasFatigueLaw");
pub const PRED_PHYS_HAS_BOND_PORT: u64 = q_hash("phys:hasBondPort");

// ── Family D: Sensory, Acoustic & Optical Predicates (4) ─────────────────
pub const PRED_SENS_HAS_HOA_ORDER: u64 = q_hash("sens:hasHoaOrder");
pub const PRED_SENS_HAS_SOFA_HRTF: u64 = q_hash("sens:hasSofaHrtf");
pub const PRED_SENS_HAS_STOKES_VECTOR: u64 = q_hash("sens:hasStokesVector");
pub const PRED_SENS_HAS_REFRACTIVE_INDEX: u64 = q_hash("sens:hasRefractiveIndex");

// ── Family E: Epistemic & Bifurcation Predicates (3) ──────────────────────
pub const PRED_EPI_HAS_QUANTUM_RANGE: u64 = q_hash("epi:hasQuantumRange");
pub const PRED_EPI_HAS_BIFURCATION_WING: u64 = q_hash("epi:hasBifurcationWing");
pub const PRED_EPI_HAS_CONSENT_RECEIPT: u64 = q_hash("epi:hasConsentReceipt");

// ── Family F: KV-Cache & LLM Grounding Predicates (2) ────────────────────
pub const PRED_LLM_HAS_KV_SLOT: u64 = q_hash("llm:hasKvSlot");
pub const PRED_LLM_HAS_SPARSE_ATOM_CODE: u64 = q_hash("llm:hasSparseAtomCode");

/// All 28 canonical predicates defined by §3.6.2.
pub const ALL_SPATIAL_SENSORY_PREDICATES: [u64; 28] = [
    PRED_GEO_HAS_VERTEX,
    PRED_GEO_HAS_PACKED_CENTROID,
    PRED_GEO_HAS_PACKED_BBOX_MIN,
    PRED_GEO_HAS_PACKED_BBOX_MAX,
    PRED_GEO_HAS_SCALE_TIER,
    PRED_GEO_HAS_TOPOLOGY_CLASS,
    PRED_GEO_HAS_PORTAL_LINK,
    PRED_Q42_HAS_CONTAINER_10D,
    PRED_Q42_HAS_M64_CHUNK,
    PRED_Q42_HAS_EMF_CHUNK,
    PRED_Q42_HAS_SPECTRAL_SHEET_V3,
    PRED_Q42_HAS_HMC_BUNDLE,
    PRED_Q42_HAS_CLUSTER_DAG,
    PRED_KIN_HAS_PGA_MOTOR,
    PRED_KIN_HAS_JOINT_PARENT,
    PRED_BIO_HAS_HILL_MUSCLE,
    PRED_BIO_HAS_ALLOMETRIC_LAW,
    PRED_MECH_HAS_FATIGUE_LAW,
    PRED_PHYS_HAS_BOND_PORT,
    PRED_SENS_HAS_HOA_ORDER,
    PRED_SENS_HAS_SOFA_HRTF,
    PRED_SENS_HAS_STOKES_VECTOR,
    PRED_SENS_HAS_REFRACTIVE_INDEX,
    PRED_EPI_HAS_QUANTUM_RANGE,
    PRED_EPI_HAS_BIFURCATION_WING,
    PRED_EPI_HAS_CONSENT_RECEIPT,
    PRED_LLM_HAS_KV_SLOT,
    PRED_LLM_HAS_SPARSE_ATOM_CODE,
];

// ── Coordinate Packing & Scale Helpers ────────────────────────────────────

/// Step in meters for scale tier `0..=7`.
#[inline]
pub fn scale_tier_step_meters(tier: u8) -> f32 {
    match tier {
        0 => 0.001,      // mm default (legacy compatible)
        1 => 1.0e-9,     // nm
        2 => 1.0e-6,     // um
        3 => 1.0,        // m
        4 => 1.0e3,      // km
        5 => 1.0e6,      // 100 km / Mm
        6 => 1.496e11,   // Astronomical (AU)
        7 => 3.0857e16,  // Interstellar (Parsec)
        _ => 0.001,
    }
}

#[inline]
fn sign_extend_20(v: u32) -> i32 {
    let v = v & 0xF_FFFF;
    if (v & 0x8_0000) != 0 {
        (v | !0xF_FFFF) as i32
    } else {
        v as i32
    }
}

/// Pack 3D floating-point coordinates `(x, y, z)` into a 60-bit payload + 3-bit scale tier.
#[inline]
pub fn pack_coord_with_scale(x: f32, y: f32, z: f32, scale_tier: u8) -> u64 {
    let step = scale_tier_step_meters(scale_tier);
    let xi = ((x / step).round() as i64) & 0xF_FFFF;
    let yi = ((y / step).round() as i64) & 0xF_FFFF;
    let zi = ((z / step).round() as i64) & 0xF_FFFF;
    let tier_bits = (scale_tier as u64 & 0x7) << 60;
    tier_bits | ((xi as u64) << 40) | ((yi as u64) << 20) | (zi as u64)
}

/// Unpack 60-bit payload + 3-bit scale tier into `(x, y, z, scale_tier)`.
#[inline]
pub fn unpack_coord_with_scale(object: u64) -> (f32, f32, f32, u8) {
    let tier = ((object >> 60) & 0x7) as u8;
    let xi = sign_extend_20(((object >> 40) & 0xF_FFFF) as u32);
    let yi = sign_extend_20(((object >> 20) & 0xF_FFFF) as u32);
    let zi = sign_extend_20((object & 0xF_FFFF) as u32);
    let step = scale_tier_step_meters(tier);
    (xi as f32 * step, yi as f32 * step, zi as f32 * step, tier)
}

// ── Zero-Heap Constructors & Decoders ─────────────────────────────────────

/// Construct an `NQuin` for a 3D spatial coordinate or bounding box bound.
#[inline]
pub fn make_spatial_coord_quin(
    subject: u64,
    predicate: u64,
    x: f32,
    y: f32,
    z: f32,
    scale_tier: u8,
    context: u64,
    metadata: u64,
) -> NQuin {
    let object = pack_coord_with_scale(x, y, z, scale_tier);
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Decode coordinates and scale tier from a spatial coordinate `NQuin`.
#[inline]
pub fn decode_spatial_coord_quin(quin: &NQuin) -> (f32, f32, f32, u8) {
    unpack_coord_with_scale(quin.object)
}

/// Construct an `NQuin` linking a subject to a binary container section (.10d, .m64, .emf, etc.).
#[inline]
pub fn make_container_link_quin(
    subject: u64,
    predicate: u64,
    container_hash: u64,
    section_offset: u32,
    chunk_records: u16,
    context: u64,
) -> NQuin {
    let object = container_hash & 0x0FFF_FFFF_FFFF_FFFF;
    let metadata = ((section_offset as u64) << 32) | (chunk_records as u64);
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Decode container link payload: `(container_hash, section_offset, chunk_records)`.
#[inline]
pub fn decode_container_link_quin(quin: &NQuin) -> (u64, u32, u16) {
    let container_hash = quin.object & 0x0FFF_FFFF_FFFF_FFFF;
    let section_offset = (quin.metadata >> 32) as u32;
    let chunk_records = (quin.metadata & 0xFFFF) as u16;
    (container_hash, section_offset, chunk_records)
}

/// Construct an `NQuin` for a kinematic or physical property (joint parent, PGA motor slot).
#[inline]
pub fn make_kinematic_quin(
    subject: u64,
    predicate: u64,
    parent_or_type_hash: u64,
    motor_slot: u32,
    context: u64,
) -> NQuin {
    let object = parent_or_type_hash & 0x0FFF_FFFF_FFFF_FFFF;
    let metadata = motor_slot as u64;
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Decode kinematic quin: `(parent_or_type_hash, motor_slot)`.
#[inline]
pub fn decode_kinematic_quin(quin: &NQuin) -> (u64, u32) {
    (quin.object & 0x0FFF_FFFF_FFFF_FFFF, quin.metadata as u32)
}

/// Construct an `NQuin` for sensory or acoustic attachments (Ambisonics HOA order, SOFA HRTF).
#[inline]
pub fn make_sensory_quin(
    subject: u64,
    predicate: u64,
    asset_hash: u64,
    hoa_order_or_band: u32,
    context: u64,
) -> NQuin {
    let object = asset_hash & 0x0FFF_FFFF_FFFF_FFFF;
    let metadata = hoa_order_or_band as u64;
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Decode sensory quin: `(asset_hash, hoa_order_or_band)`.
#[inline]
pub fn decode_sensory_quin(quin: &NQuin) -> (u64, u32) {
    (quin.object & 0x0FFF_FFFF_FFFF_FFFF, quin.metadata as u32)
}

/// Construct an `NQuin` linking a subject to an epistemic quantum range or consent receipt.
#[inline]
pub fn make_epistemic_quin(
    subject: u64,
    predicate: u64,
    value: u64,
    context: u64,
    lamport_clock: u32,
) -> NQuin {
    let metadata = lamport_clock as u64;
    let parity = NQuin::calculate_parity(subject, predicate, value, context, metadata);
    NQuin {
        subject,
        predicate,
        object: value,
        context,
        metadata,
        parity,
    }
}

/// Decode epistemic quin: returns `quin.object`.
#[inline]
pub fn decode_epistemic_quin(quin: &NQuin) -> u64 {
    quin.object
}

/// Construct an `NQuin` linking a volumetric node to an LLM KV slot and prompt page.
#[inline]
pub fn make_llm_grounding_quin(
    subject: u64,
    predicate: u64,
    kv_slot: u32,
    parent_page_id: u64,
    context: u64,
) -> NQuin {
    let object = kv_slot as u64;
    let metadata = parent_page_id;
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Decode LLM grounding quin: `(kv_slot, parent_page_id)`.
#[inline]
pub fn decode_llm_grounding_quin(quin: &NQuin) -> (u32, u64) {
    (quin.object as u32, quin.metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_28_predicates_are_unique_with_zero_collisions() {
        let preds = ALL_SPATIAL_SENSORY_PREDICATES;
        assert_eq!(preds.len(), 28);
        for i in 0..preds.len() {
            for j in (i + 1)..preds.len() {
                assert_ne!(
                    preds[i], preds[j],
                    "Collision detected between predicate index {i} and {j}"
                );
            }
        }
    }

    #[test]
    fn spatial_coord_round_trip_millimeter() {
        let quin = make_spatial_coord_quin(100, PRED_GEO_HAS_VERTEX, 1.25, -2.5, 3.75, 0, 0, 0);
        assert!(quin.verify_ecc_parity());
        let (x, y, z, tier) = decode_spatial_coord_quin(&quin);
        assert_eq!(tier, 0);
        assert!((x - 1.25).abs() < 0.002);
        assert!((y - -2.5).abs() < 0.002);
        assert!((z - 3.75).abs() < 0.002);
    }

    #[test]
    fn spatial_coord_round_trip_meter_tier() {
        let quin = make_spatial_coord_quin(101, PRED_GEO_HAS_PACKED_CENTROID, 120.0, -45.0, 300.0, 3, 0, 0);
        assert!(quin.verify_ecc_parity());
        let (x, y, z, tier) = decode_spatial_coord_quin(&quin);
        assert_eq!(tier, 3);
        assert!((x - 120.0).abs() < 1.0);
        assert!((y - -45.0).abs() < 1.0);
        assert!((z - 300.0).abs() < 1.0);
    }

    #[test]
    fn container_link_round_trip() {
        let quin = make_container_link_quin(200, PRED_Q42_HAS_M64_CHUNK, 0x1234_5678_9ABC_DEF0, 4096, 64, 1);
        assert!(quin.verify_ecc_parity());
        let (hash, offset, chunks) = decode_container_link_quin(&quin);
        assert_eq!(hash, 0x1234_5678_9ABC_DEF0 & 0x0FFF_FFFF_FFFF_FFFF);
        assert_eq!(offset, 4096);
        assert_eq!(chunks, 64);
    }

    #[test]
    fn kinematic_round_trip() {
        let quin = make_kinematic_quin(300, PRED_KIN_HAS_PGA_MOTOR, 0x9999_8888, 12, 0);
        assert!(quin.verify_ecc_parity());
        let (hash, slot) = decode_kinematic_quin(&quin);
        assert_eq!(hash, 0x9999_8888);
        assert_eq!(slot, 12);
    }

    #[test]
    fn sensory_round_trip() {
        let quin = make_sensory_quin(400, PRED_SENS_HAS_HOA_ORDER, 0x7777, 3, 0);
        assert!(quin.verify_ecc_parity());
        let (hash, hoa) = decode_sensory_quin(&quin);
        assert_eq!(hash, 0x7777);
        assert_eq!(hoa, 3);
    }

    #[test]
    fn epistemic_round_trip() {
        let quin = make_epistemic_quin(500, PRED_EPI_HAS_QUANTUM_RANGE, 998, 0, 42);
        assert!(quin.verify_ecc_parity());
        assert_eq!(decode_epistemic_quin(&quin), 998);
        assert_eq!(quin.extract_lamport_clock(), 42);
    }

    #[test]
    fn llm_grounding_round_trip() {
        let quin = make_llm_grounding_quin(600, PRED_LLM_HAS_KV_SLOT, 128, 0xABCDE, 0);
        assert!(quin.verify_ecc_parity());
        let (slot, page) = decode_llm_grounding_quin(&quin);
        assert_eq!(slot, 128);
        assert_eq!(page, 0xABCDE);
    }
}
