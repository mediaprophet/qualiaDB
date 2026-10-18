//! Botanical & Flora Biophysics Modality (`botany.rs`)
//!
//! Provides deterministic 10D ontological graph ledger bindings for procedural flora:
//! - Metamer organ state, sapwood pipe count, and heartwood core
//! - Continuous cellular turgor pressure and hydraulic potential
//! - Thermal time phenology (Growing Degree Days $GDD$, vernalization chilling)
//! - Ground cover ecotope patch (LAI, culm height, trample deformation)
//! - Murray's Law vascular radius and Hagen-Poiseuille conductance
//! - Raunkiær life-form classification and harsh-season dieback state
//!
//! Strictly zero heap in hot loops (`AGENTS.md` §0), 48-byte Super-Quin layout.

use crate::NQuin;

// ── Modality Opcodes (0x57–0x5C) ──────────────────────────────────────────

/// Metamer organ state: Organ type, active sapwood pipes, retired heartwood pipes.
pub const OP_BOTANY_METAMER: u8 = 0x57;
/// Cellular turgor pressure in kPa + hydraulic water potential.
pub const OP_BOTANY_TURGOR: u8 = 0x58;
/// Thermal phenology: Accumulated GDD + chilling hours.
pub const OP_BOTANY_PHENOLOGY: u8 = 0x59;
/// Ground cover ecotope: LAI + mean culm height mm + trample matted ratio.
pub const OP_BOTANY_ECOTOPE: u8 = 0x5A;
/// Vascular venation: Murray's Law radius um + Poiseuille conductance.
pub const OP_BOTANY_VASCULAR: u8 = 0x5B;
/// Raunkiær life-form classification + seasonal dieback pruning flag.
pub const OP_BOTANY_RAUNKIAER: u8 = 0x5C;

/// Evaluated botanical assessment for a plant, branch, metamer, or ecotope patch.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BotanyVerdict {
    pub claim: NQuin,
    pub organ_type: u8,
    pub active_pipes: u16,
    pub turgor_kpa: u16,
    pub accumulated_gdd: f32,
    pub is_dieback_active: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BotanyError {
    BufferOverflow,
    InvalidQuinOpcode,
}

/// Packs a metamer's vascular pipe state into a 48-byte NQuin.
pub fn pack_botany_metamer_quin(
    plant_did: u64,
    world_did: u64,
    organ_type: u8,
    active_pipes: u16,
    retired_pipes: u16,
    lamport: u32,
) -> NQuin {
    let subject = plant_did;
    let predicate = (0x424F_5441_4E59_4D45u64 << 8) | (OP_BOTANY_METAMER as u64); // "BOTANYME"
    let object = ((organ_type as u64) << 32) | ((active_pipes as u64) << 16) | (retired_pipes as u64);
    let context = world_did;
    let metadata = ((lamport as u64 & 0x1FFF_FFFF) << 32) | (active_pipes as u64 & 0xFFFF);
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

/// Packs cellular turgor pressure into an NQuin.
pub fn pack_botany_turgor_quin(
    plant_did: u64,
    world_did: u64,
    turgor_kpa: u16,
    water_potential_kpa: i16,
    lamport: u32,
) -> NQuin {
    let subject = plant_did;
    let predicate = (0x424F_5441_4E59_5455u64 << 8) | (OP_BOTANY_TURGOR as u64); // "BOTANYTU"
    let object = ((turgor_kpa as u64) << 16) | ((water_potential_kpa as u16) as u64);
    let context = world_did;
    let metadata = ((lamport as u64 & 0x1FFF_FFFF) << 32) | (turgor_kpa as u64);
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

/// Packs thermal phenology GDD into an NQuin.
pub fn pack_botany_phenology_quin(
    plant_did: u64,
    world_did: u64,
    accumulated_gdd: f32,
    chilling_hours: f32,
    lamport: u32,
) -> NQuin {
    let subject = plant_did;
    let predicate = (0x424F_5441_4E59_5048u64 << 8) | (OP_BOTANY_PHENOLOGY as u64); // "BOTANYPH"
    let gdd_u32 = (accumulated_gdd.max(0.0) * 10.0) as u32; // 0.1 degree-day precision
    let chill_u32 = (chilling_hours.max(0.0) * 10.0) as u32;
    let object = ((gdd_u32 as u64) << 32) | (chill_u32 as u64);
    let context = world_did;
    let metadata = (lamport as u64 & 0x1FFF_FFFF) << 32;
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

/// Unpacks a metamer Quin returning `(organ_type, active_pipes, retired_pipes)`.
#[inline]
pub fn unpack_botany_metamer_quin(quin: &NQuin) -> Option<(u8, u16, u16)> {
    let opcode = (quin.predicate & 0xFF) as u8;
    if opcode == OP_BOTANY_METAMER {
        let organ = ((quin.object >> 32) & 0xFF) as u8;
        let active = ((quin.object >> 16) & 0xFFFF) as u16;
        let retired = (quin.object & 0xFFFF) as u16;
        Some((organ, active, retired))
    } else {
        None
    }
}

/// Unpacks a turgor Quin returning `turgor_kpa`.
#[inline]
pub fn unpack_botany_turgor_quin(quin: &NQuin) -> Option<u16> {
    let opcode = (quin.predicate & 0xFF) as u8;
    if opcode == OP_BOTANY_TURGOR {
        let turgor = ((quin.object >> 16) & 0xFFFF) as u16;
        Some(turgor)
    } else {
        None
    }
}

/// Evaluates a frame of botanical Quins, outputting zero-alloc verdicts.
pub fn evaluate_botanical_frame(
    quins: &[NQuin],
    plant_did: u64,
    out: &mut [BotanyVerdict],
) -> Result<usize, BotanyError> {
    let mut count = 0;

    for q in quins {
        if plant_did != 0 && q.subject != plant_did {
            continue;
        }

        let opcode = (q.predicate & 0xFF) as u8;
        if opcode < OP_BOTANY_METAMER || opcode > OP_BOTANY_RAUNKIAER {
            continue;
        }

        if count >= out.len() {
            return Err(BotanyError::BufferOverflow);
        }

        let mut verdict = BotanyVerdict {
            claim: *q,
            organ_type: 0,
            active_pipes: 0,
            turgor_kpa: 0,
            accumulated_gdd: 0.0,
            is_dieback_active: false,
        };

        match opcode {
            OP_BOTANY_METAMER => {
                if let Some((organ, active, _retired)) = unpack_botany_metamer_quin(q) {
                    verdict.organ_type = organ;
                    verdict.active_pipes = active;
                }
            }
            OP_BOTANY_TURGOR => {
                if let Some(turgor) = unpack_botany_turgor_quin(q) {
                    verdict.turgor_kpa = turgor;
                }
            }
            OP_BOTANY_PHENOLOGY => {
                let gdd_raw = (q.object >> 32) as u32;
                verdict.accumulated_gdd = (gdd_raw as f32) / 10.0;
            }
            OP_BOTANY_RAUNKIAER => {
                let dieback = (q.object & 0x1) != 0;
                verdict.is_dieback_active = dieback;
            }
            _ => {}
        }

        out[count] = verdict;
        count += 1;
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metamer_quin_roundtrip() {
        let plant_did = 0x1234_5678_9ABC_DEF0;
        let world_did = 0x9999_8888_7777_6666;
        let quin = pack_botany_metamer_quin(plant_did, world_did, 2, 42, 108, 12);

        assert_eq!((quin.predicate & 0xFF) as u8, OP_BOTANY_METAMER);
        let unpacked = unpack_botany_metamer_quin(&quin).unwrap();
        assert_eq!(unpacked, (2, 42, 108));

        // Parity ECC check
        let parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context;
        assert_eq!(quin.parity, parity);
    }

    #[test]
    fn test_botanical_frame_evaluator() {
        let plant_did = 0xAAAA_BBBB_CCCC_DDDD;
        let world_did = 0x1111_2222_3333_4444;

        let q1 = pack_botany_metamer_quin(plant_did, world_did, 4, 15, 30, 1);
        let q2 = pack_botany_turgor_quin(plant_did, world_did, 1200, -800, 2);
        let q3 = pack_botany_phenology_quin(plant_did, world_did, 850.5, 420.0, 3);

        let quins = [q1, q2, q3];
        let mut verdicts = [BotanyVerdict {
            claim: q1,
            organ_type: 0,
            active_pipes: 0,
            turgor_kpa: 0,
            accumulated_gdd: 0.0,
            is_dieback_active: false,
        }; 8];

        let count = evaluate_botanical_frame(&quins, plant_did, &mut verdicts).unwrap();
        assert_eq!(count, 3);
        assert_eq!(verdicts[0].organ_type, 4);
        assert_eq!(verdicts[0].active_pipes, 15);
        assert_eq!(verdicts[1].turgor_kpa, 1200);
        assert!((verdicts[2].accumulated_gdd - 850.5).abs() < 0.2);
    }
}
