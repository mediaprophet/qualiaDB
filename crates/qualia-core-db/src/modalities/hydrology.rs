//! Hydrological & Atmospheric Modality (`hydrology.rs`)
//!
//! Provides deterministic 10D ontological graph ledger bindings for multi-scale fluid dynamics:
//! - Submersion state transitions: Dry (0), Wading (1), Swimming (2), Diving (3)
//! - Dynamic clothing / surface wetness ratio encoding
//! - Barometric pressure, moist air density, and wind vectors
//! - Downsampled hysteresis evaluation for agents in the `.q42` graph
//!
//! Adheres strictly to `AGENTS.md` §0 (Zero heap in hot paths, 48-byte Super-Quin layout).

use crate::NQuin;

// ── Modality Opcodes (0x51–0x56) ──────────────────────────────────────────

/// Submersion state predicate (Dry, Wading, Swimming, Diving) + depth in mm.
pub const OP_HYDRO_SUBMERSION: u8 = 0x51;
/// Surface / clothing wetness ratio [0.0, 1.0] mapped to u16.
pub const OP_HYDRO_WETNESS: u8 = 0x52;
/// Fluid current velocity vector [vx, vz] in mm/s.
pub const OP_HYDRO_CURRENT: u8 = 0x53;
/// Thermal conductivity drain rate (hypothermia risk metric).
pub const OP_HYDRO_THERMAL: u8 = 0x54;
/// Precipitation intensity in mm/hr + Lifting Condensation Level (LCL).
pub const OP_HYDRO_PRECIPITATION: u8 = 0x55;
/// Synoptic barometric state: MSLP in hPa + wind vector.
pub const OP_HYDRO_BAROMETRIC: u8 = 0x56;

// ── Submersion Status Values ──────────────────────────────────────────────

pub const SUBMERSION_DRY: u8 = 0;
pub const SUBMERSION_WADING: u8 = 1;
pub const SUBMERSION_SWIMMING: u8 = 2;
pub const SUBMERSION_DIVING: u8 = 3;

/// Evaluated hydrological assessment for an entity or spatial node.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HydroVerdict {
    pub claim: NQuin,
    pub submersion_state: u8,
    pub depth_mm: u16,
    pub wetness_ratio: f32,
    pub flow_speed_m_s: f32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum HydroError {
    BufferOverflow,
    InvalidQuinOpcode,
}

/// Packs an agent's fluid submersion transition into a 48-byte NQuin.
///
/// Layout:
/// - `subject`: Agent DID hash
/// - `predicate`: `(property_hash << 8) | OP_HYDRO_SUBMERSION`
/// - `object`: `(submersion_level as u64) << 32 | (depth_mm as u64)`
/// - `context`: World / Basin DID hash
/// - `metadata`: Lamport logical clock in bits [32..60]
/// - `parity`: XOR fold
pub fn pack_hydro_submersion_quin(
    agent_did: u64,
    world_did: u64,
    submersion: u8,
    depth_mm: u16,
    lamport: u32,
) -> NQuin {
    let subject = agent_did;
    let predicate = (0x4859_4452_4F53_5542u64 << 8) | (OP_HYDRO_SUBMERSION as u64); // "HYDROSUB"
    let object = ((submersion as u64) << 32) | (depth_mm as u64);
    let context = world_did;
    let metadata = ((lamport as u64 & 0x1FFF_FFFF) << 32) | (depth_mm as u64 & 0xFFFF);
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

/// Packs dynamic surface wetness into an NQuin.
pub fn pack_hydro_wetness_quin(
    agent_did: u64,
    world_did: u64,
    wetness_ratio: f32,
    lamport: u32,
) -> NQuin {
    let subject = agent_did;
    let predicate = (0x4859_4452_4F57_4554u64 << 8) | (OP_HYDRO_WETNESS as u64); // "HYDROWET"
    let wetness_u16 = ((wetness_ratio.clamp(0.0, 1.0) * 65535.0) as u64) & 0xFFFF;
    let object = wetness_u16;
    let context = world_did;
    let metadata = ((lamport as u64 & 0x1FFF_FFFF) << 32) | wetness_u16;
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

/// Packs synoptic barometric pressure and wind state into an NQuin.
pub fn pack_hydro_barometric_quin(
    world_did: u64,
    mslp_hpa: f32,
    wind_u_mps: f32,
    wind_v_mps: f32,
    lamport: u32,
) -> NQuin {
    let subject = world_did;
    let predicate = (0x4859_4452_4F42_4152u64 << 8) | (OP_HYDRO_BAROMETRIC as u64); // "HYDROBAR"
    let mslp_scaled = ((mslp_hpa.clamp(800.0, 1100.0) - 800.0) * 100.0) as u64; // 0.01 hPa precision
    let u_packed = ((wind_u_mps.clamp(-100.0, 100.0) + 100.0) * 10.0) as u64;
    let v_packed = ((wind_v_mps.clamp(-100.0, 100.0) + 100.0) * 10.0) as u64;
    let object = (mslp_scaled << 32) | (u_packed << 16) | v_packed;
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

/// Unpacks a submersion Quin returning `(submersion_level, depth_mm)`.
#[inline]
pub fn unpack_hydro_submersion_quin(quin: &NQuin) -> Option<(u8, u16)> {
    let opcode = (quin.predicate & 0xFF) as u8;
    if opcode == OP_HYDRO_SUBMERSION {
        let submersion = ((quin.object >> 32) & 0xFF) as u8;
        let depth_mm = (quin.object & 0xFFFF) as u16;
        Some((submersion, depth_mm))
    } else {
        None
    }
}

/// Unpacks a wetness Quin returning the continuous wetness ratio in [0.0, 1.0].
#[inline]
pub fn unpack_hydro_wetness_quin(quin: &NQuin) -> Option<f32> {
    let opcode = (quin.predicate & 0xFF) as u8;
    if opcode == OP_HYDRO_WETNESS {
        let raw = (quin.object & 0xFFFF) as f32;
        Some(raw / 65535.0)
    } else {
        None
    }
}

/// Evaluates a frame of Quins, filtering by optional agent DID hash and producing zero-alloc verdicts.
pub fn evaluate_hydrological_frame(
    quins: &[NQuin],
    agent_did: u64,
    out: &mut [HydroVerdict],
) -> Result<usize, HydroError> {
    let mut count = 0;

    for q in quins {
        if agent_did != 0 && q.subject != agent_did {
            continue;
        }

        let opcode = (q.predicate & 0xFF) as u8;
        if opcode < OP_HYDRO_SUBMERSION || opcode > OP_HYDRO_BAROMETRIC {
            continue;
        }

        if count >= out.len() {
            return Err(HydroError::BufferOverflow);
        }

        let mut verdict = HydroVerdict {
            claim: *q,
            submersion_state: SUBMERSION_DRY,
            depth_mm: 0,
            wetness_ratio: 0.0,
            flow_speed_m_s: 0.0,
        };

        match opcode {
            OP_HYDRO_SUBMERSION => {
                if let Some((sub, depth)) = unpack_hydro_submersion_quin(q) {
                    verdict.submersion_state = sub;
                    verdict.depth_mm = depth;
                }
            }
            OP_HYDRO_WETNESS => {
                if let Some(wet) = unpack_hydro_wetness_quin(q) {
                    verdict.wetness_ratio = wet;
                }
            }
            OP_HYDRO_CURRENT => {
                let speed_mm = (q.object & 0xFFFF) as f32;
                verdict.flow_speed_m_s = speed_mm / 1000.0;
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
    fn test_hydro_submersion_quin_roundtrip() {
        let q = pack_hydro_submersion_quin(12345, 67890, SUBMERSION_WADING, 450, 42);
        assert_eq!((q.predicate & 0xFF) as u8, OP_HYDRO_SUBMERSION);
        assert_eq!(q.subject, 12345);
        assert_eq!(q.context, 67890);

        let unpacked = unpack_hydro_submersion_quin(&q);
        assert_eq!(unpacked, Some((SUBMERSION_WADING, 450)));
    }

    #[test]
    fn test_hydro_wetness_quin_roundtrip() {
        let q = pack_hydro_wetness_quin(555, 777, 0.75, 100);
        let wetness = unpack_hydro_wetness_quin(&q).expect("wetness quin");
        assert!((wetness - 0.75).abs() < 1e-4);
    }

    #[test]
    fn test_evaluate_hydrological_frame() {
        let q1 = pack_hydro_submersion_quin(101, 999, SUBMERSION_SWIMMING, 1200, 1);
        let q2 = pack_hydro_wetness_quin(101, 999, 1.0, 2);
        let q3 = pack_hydro_submersion_quin(202, 999, SUBMERSION_DRY, 0, 3);

        let quins = [q1, q2, q3];
        let mut verdicts = [HydroVerdict {
            claim: q1,
            submersion_state: 0,
            depth_mm: 0,
            wetness_ratio: 0.0,
            flow_speed_m_s: 0.0,
        }; 4];

        // Filter for agent 101 only
        let count = evaluate_hydrological_frame(&quins, 101, &mut verdicts).expect("eval");
        assert_eq!(count, 2);
        assert_eq!(verdicts[0].submersion_state, SUBMERSION_SWIMMING);
        assert_eq!(verdicts[0].depth_mm, 1200);
        assert!((verdicts[1].wetness_ratio - 1.0).abs() < 1e-4);
    }
}
