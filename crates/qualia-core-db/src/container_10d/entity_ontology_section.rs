//! Versioned `.10d` Entity Behavioral & Affordance Ontology Section.
//!
//! EONT1 encodes the digital twin's entity ontology directly inside
//! a `.10d` living container (SectionType 17). It packs:
//! - Mini-header: entity DID hash, taxon IRI hash, lifecycle stage, initial energy, damage, hydration, wear
//! - Affordance Table: fixed-stride POD records (action ID, category, permission tier, preconditions, effects, clip ID)
//! - Zero-heap on hot query and evaluation paths.

use bytemuck::{Pod, Zeroable};

use crate::entity_view::entity_ontologies::EntityOntology;

pub const ENTITY_ONTOLOGY_SECTION_VERSION: u16 = 1;
pub const ENTITY_ONTOLOGY_MINI_HEADER_SIZE: usize = 40;
pub const AFFORDANCE_RECORD_SIZE: usize = 48;
pub const MAX_AFFORDANCES_PER_SECTION: usize = 256;
pub const MAGIC_EONT: [u8; 4] = *b"EONT";

// Legacy aliases
pub const PERSONAL_ONTOLOGY_SECTION_VERSION: u16 = ENTITY_ONTOLOGY_SECTION_VERSION;
pub const PERSONAL_ONTOLOGY_MINI_HEADER_SIZE: usize = ENTITY_ONTOLOGY_MINI_HEADER_SIZE;
pub const MAGIC_PONT: [u8; 4] = MAGIC_EONT;

/// 40-byte header for SectionType::EntityOntology (EONT1).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct EntityOntologyMiniHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub stage: u8,
    pub reserved_u8: u8,
    pub entity_did_hash: u64,
    pub taxon_iri_hash: u64,
    pub energy_permill: u16,
    pub damage_permill: u16,
    pub hydration_permill: u16,
    pub wear_permill: u16,
    pub affordance_count: u32,
    pub reserved_u32: u32,
}
const _: [(); ENTITY_ONTOLOGY_MINI_HEADER_SIZE] =
    [(); core::mem::size_of::<EntityOntologyMiniHeader>()];

pub type PersonalOntologyMiniHeader = EntityOntologyMiniHeader;

/// 48-byte fixed-stride binary record representing one affordance.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct AffordanceBinaryRecord {
    pub action_id: u64,
    pub required_capability_hash: u64,
    pub emitted_topic_hash: u64,
    pub category: u8,
    pub min_observer_status: u8,
    pub min_energy_permill: u16,
    pub max_damage_permill: u16,
    pub min_hydration_permill: u16,
    pub max_wear_permill: u16,
    pub energy_delta: i16,
    pub damage_delta: i16,
    pub hydration_delta: i16,
    pub wear_delta: i16,
    pub reserved_u16: u16,
    pub animation_clip_id: u32,
}
const _: [(); AFFORDANCE_RECORD_SIZE] = [(); core::mem::size_of::<AffordanceBinaryRecord>()];

pub const DEVELOPMENTAL_KEYFRAME_RECORD_SIZE: usize = 32;

/// 32-byte fixed-stride binary record representing an ontogenetic growth milestone.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct DevelopmentalKeyframeRecord {
    /// Age in days since germination/birth.
    pub age_days: f32,
    /// Ontogenetic lifecycle stage (0=SeedOrSpore, 1=Juvenile, 2=Adult, 3=Senescent, 4=Deceased).
    pub stage: u8,
    pub reserved_u8: u8,
    pub reserved_u16: u16,
    /// Allometric crown radius scaling factor (1.0 = base adult size).
    pub allometric_crown_scale: f32,
    /// Allometric trunk diameter scaling factor.
    pub allometric_trunk_scale: f32,
    /// Foliage density / leaf area index multiplier.
    pub lai_multiplier: f32,
    pub reserved_u32: u32,
    /// Associated mesh asset key hash (0 = procedural morph).
    pub mesh_asset_hash: u64,
}
const _: [(); DEVELOPMENTAL_KEYFRAME_RECORD_SIZE] =
    [(); core::mem::size_of::<DevelopmentalKeyframeRecord>()];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityOntologySectionError {
    PayloadTooShort {
        need: usize,
        have: usize,
    },
    BadMagic([u8; 4]),
    UnsupportedVersion(u16),
    TooManyAffordances {
        count: usize,
        max: usize,
    },
    BufferTooSmall {
        need: usize,
        have: usize,
    },
    LengthMismatch {
        header_len: usize,
        payload_len: usize,
    },
}

pub type PersonalOntologySectionError = EntityOntologySectionError;

impl std::fmt::Display for EntityOntologySectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PayloadTooShort { need, have } => {
                write!(
                    f,
                    "entity ontology section too short: have {have}, need {need}"
                )
            }
            Self::BadMagic(m) => write!(f, "bad entity ontology magic: {m:?}"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported entity ontology version {v}"),
            Self::TooManyAffordances { count, max } => {
                write!(f, "too many affordances: {count} > {max}")
            }
            Self::BufferTooSmall { need, have } => {
                write!(f, "destination buffer too small: have {have}, need {need}")
            }
            Self::LengthMismatch {
                header_len,
                payload_len,
            } => {
                write!(
                    f,
                    "length mismatch: expected {header_len}, payload {payload_len}"
                )
            }
        }
    }
}

impl std::error::Error for EntityOntologySectionError {}

/// Calculate total encoded byte length for a given affordance count.
#[inline]
pub fn entity_ontology_encoded_len(affordance_count: usize) -> usize {
    ENTITY_ONTOLOGY_MINI_HEADER_SIZE + (affordance_count * AFFORDANCE_RECORD_SIZE)
}

pub use entity_ontology_encoded_len as personal_ontology_encoded_len;

/// Encode an EntityOntology into a caller-supplied binary buffer.
pub fn encode_entity_ontology_section(
    onto: &EntityOntology,
    taxon_hash: u64,
    out: &mut [u8],
) -> Result<usize, EntityOntologySectionError> {
    let count = onto.affordances.len();
    if count > MAX_AFFORDANCES_PER_SECTION {
        return Err(EntityOntologySectionError::TooManyAffordances {
            count,
            max: MAX_AFFORDANCES_PER_SECTION,
        });
    }

    let needed = entity_ontology_encoded_len(count);
    if out.len() < needed {
        return Err(EntityOntologySectionError::BufferTooSmall {
            need: needed,
            have: out.len(),
        });
    }

    let header = EntityOntologyMiniHeader {
        magic: MAGIC_EONT,
        version: ENTITY_ONTOLOGY_SECTION_VERSION,
        stage: onto.stage as u8,
        reserved_u8: 0,
        entity_did_hash: onto.entity_id.0,
        taxon_iri_hash: taxon_hash,
        energy_permill: onto.energy_permill,
        damage_permill: onto.damage_permill,
        hydration_permill: onto.hydration_permill,
        wear_permill: onto.wear_permill,
        affordance_count: count as u32,
        reserved_u32: 0,
    };

    let header_bytes = bytemuck::bytes_of(&header);
    out[..ENTITY_ONTOLOGY_MINI_HEADER_SIZE].copy_from_slice(header_bytes);

    let mut cursor = ENTITY_ONTOLOGY_MINI_HEADER_SIZE;
    for aff in &onto.affordances {
        let rec = AffordanceBinaryRecord {
            action_id: aff.action_id,
            required_capability_hash: aff.preconditions.required_capability_hash,
            emitted_topic_hash: aff.effects.emitted_topic_hash,
            category: aff.category as u8,
            min_observer_status: aff.preconditions.min_observer_status as u8,
            min_energy_permill: aff.preconditions.min_energy_permill,
            max_damage_permill: aff.preconditions.max_damage_permill,
            min_hydration_permill: aff.preconditions.min_hydration_permill,
            max_wear_permill: aff.preconditions.max_wear_permill,
            energy_delta: aff.effects.energy_delta,
            damage_delta: aff.effects.damage_delta,
            hydration_delta: aff.effects.hydration_delta,
            wear_delta: aff.effects.wear_delta,
            reserved_u16: 0,
            animation_clip_id: aff.effects.animation_clip_id,
        };
        let rec_bytes = bytemuck::bytes_of(&rec);
        out[cursor..cursor + AFFORDANCE_RECORD_SIZE].copy_from_slice(rec_bytes);
        cursor += AFFORDANCE_RECORD_SIZE;
    }

    Ok(needed)
}

pub use encode_entity_ontology_section as encode_personal_ontology_section;

/// Decode a `.10d` EntityOntology section into caller-supplied `AffordanceBinaryRecord` buffer.
/// Zero heap allocations in hot path.
pub fn decode_entity_ontology_section_into<'a>(
    payload: &'a [u8],
    out_records: &mut [AffordanceBinaryRecord],
) -> Result<(EntityOntologyMiniHeader, usize), EntityOntologySectionError> {
    if payload.len() < ENTITY_ONTOLOGY_MINI_HEADER_SIZE {
        return Err(EntityOntologySectionError::PayloadTooShort {
            need: ENTITY_ONTOLOGY_MINI_HEADER_SIZE,
            have: payload.len(),
        });
    }

    let header: EntityOntologyMiniHeader =
        bytemuck::pod_read_unaligned(&payload[..ENTITY_ONTOLOGY_MINI_HEADER_SIZE]);

    if header.magic != MAGIC_EONT && header.magic != *b"PONT" {
        return Err(EntityOntologySectionError::BadMagic(header.magic));
    }
    if header.version != ENTITY_ONTOLOGY_SECTION_VERSION {
        return Err(EntityOntologySectionError::UnsupportedVersion(
            header.version,
        ));
    }

    let count = header.affordance_count as usize;
    if count > MAX_AFFORDANCES_PER_SECTION {
        return Err(EntityOntologySectionError::TooManyAffordances {
            count,
            max: MAX_AFFORDANCES_PER_SECTION,
        });
    }

    let expected_len = entity_ontology_encoded_len(count);
    if payload.len() != expected_len {
        return Err(EntityOntologySectionError::LengthMismatch {
            header_len: expected_len,
            payload_len: payload.len(),
        });
    }

    if out_records.len() < count {
        return Err(EntityOntologySectionError::BufferTooSmall {
            need: count,
            have: out_records.len(),
        });
    }

    for (index, record) in out_records[..count].iter_mut().enumerate() {
        let start = ENTITY_ONTOLOGY_MINI_HEADER_SIZE + index * AFFORDANCE_RECORD_SIZE;
        *record = bytemuck::pod_read_unaligned(&payload[start..start + AFFORDANCE_RECORD_SIZE]);
    }

    Ok((header, count))
}

pub use decode_entity_ontology_section_into as decode_personal_ontology_section_into;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_view::entity_ontologies::{
        ActionAffordance, AffordanceCategory, LifecycleStage,
    };
    use crate::entity_view::{EntityId, ObserverStatus};

    #[test]
    fn entity_ontology_section_roundtrips() {
        let entity_id = EntityId::from_u64(0xFEED_FACE_CAFE_BEEF);
        let mut onto = EntityOntology::new(entity_id, "qualia:TestEntity");
        onto.stage = LifecycleStage::Juvenile;
        onto.energy_permill = 950;
        onto.damage_permill = 15;
        onto.hydration_permill = 880;
        onto.wear_permill = 40;

        let mut a1 = ActionAffordance::new(
            "graze",
            "Consume pasture grass",
            AffordanceCategory::Biological,
        );
        a1.preconditions.min_observer_status = ObserverStatus::Public;
        a1.preconditions.min_energy_permill = 100;
        a1.effects.energy_delta = 250;
        a1.effects.animation_clip_id = 7;
        onto.add_affordance(a1);

        let mut a2 = ActionAffordance::new(
            "flee",
            "Emergency evasive sprint",
            AffordanceCategory::Physical,
        );
        a2.preconditions.min_energy_permill = 300;
        a2.effects.energy_delta = -200;
        a2.effects.animation_clip_id = 12;
        onto.add_affordance(a2);

        let mut buffer = [0u8; 512];
        let bytes_written =
            encode_entity_ontology_section(&onto, 0x9988_7766, &mut buffer).unwrap();
        assert_eq!(
            bytes_written,
            ENTITY_ONTOLOGY_MINI_HEADER_SIZE + 2 * AFFORDANCE_RECORD_SIZE
        );

        let mut decoded_records = [AffordanceBinaryRecord::zeroed(); 4];
        let (header, count) =
            decode_entity_ontology_section_into(&buffer[..bytes_written], &mut decoded_records)
                .unwrap();
        let mut unaligned = [0u8; 513];
        unaligned[1..1 + bytes_written].copy_from_slice(&buffer[..bytes_written]);
        assert_eq!(
            decode_entity_ontology_section_into(
                &unaligned[1..1 + bytes_written],
                &mut decoded_records
            )
            .unwrap()
            .1,
            count
        );

        assert_eq!(header.magic, MAGIC_EONT);
        assert_eq!(header.version, 1);
        assert_eq!(header.stage, LifecycleStage::Juvenile as u8);
        assert_eq!(header.entity_did_hash, 0xFEED_FACE_CAFE_BEEF);
        assert_eq!(header.energy_permill, 950);
        assert_eq!(header.damage_permill, 15);
        assert_eq!(count, 2);

        assert_eq!(decoded_records[0].action_id, onto.affordances[0].action_id);
        assert_eq!(
            decoded_records[0].category,
            AffordanceCategory::Biological as u8
        );
        assert_eq!(decoded_records[0].energy_delta, 250);
        assert_eq!(decoded_records[0].animation_clip_id, 7);

        assert_eq!(decoded_records[1].action_id, onto.affordances[1].action_id);
        assert_eq!(
            decoded_records[1].category,
            AffordanceCategory::Physical as u8
        );
        assert_eq!(decoded_records[1].energy_delta, -200);
        assert_eq!(decoded_records[1].animation_clip_id, 12);
    }
}
