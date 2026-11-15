// Differential 3D Video Codec Modality (`differential_render.rs`)
//
// Implements the zero-heap causal delta stream and N3Logic optical topology compiler:
// - 48-byte DeltaNQuin layout (SIMD-aligned to 16 bytes)
// - Causal Epoch Watermarking for ReSTIR reservoir invalidation
// - Two-tier Long-Term Reference (LTR) promotion and instant O(1) demotion
// - Screen-space tile dirty bitmask accumulation

use bytemuck::{Pod, Zeroable};

pub const OP_DIFF_TRANSFORM_DELTA: u16 = 0x60;
pub const OP_DIFF_RADIANCE_DELTA: u16 = 0x61;
pub const OP_DIFF_ISOLATION_MASK: u16 = 0x62;
pub const OP_DIFF_LTR_PROMOTION: u16 = 0x63;
pub const OP_DIFF_EPOCH_WATERMARK: u16 = 0x64;
pub const OP_DIFF_SALIENT_QP: u16 = 0x65;

pub const SALIENCE_CLASS_A_FOCAL: u8 = 0;
pub const SALIENCE_CLASS_B_DYNAMIC: u8 = 64;
pub const SALIENCE_CLASS_C_AMBIENT: u8 = 192;
pub const SALIENCE_CLASS_D_SUBPERCEPTUAL: u8 = 255;

pub const MEDIUM_HOMOGENEOUS_AIR: u8 = 0;
pub const MEDIUM_CLEAR_WATER: u8 = 1;
pub const MEDIUM_TURBID_BLACKWATER: u8 = 2;
pub const MEDIUM_MIST_FOG: u8 = 3;

/// 48-byte DeltaNQuin Super-Quin representation for GPU/CPU zero-copy delta streams.
///
/// Layout: 12 words of u32 = 48 bytes, aligned to 16 bytes.
#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub struct DeltaNQuin {
    // ── Word 0..1 (8 bytes): Identity & Opcode ──────────────────────────────────
    pub subject_id: u32,
    pub predicate_op: u16,
    pub salience_qp: u8,
    pub medium_branch: u8,

    // ── Word 2..7 (24 bytes): Quantized 10D Delta Payload (Object) ──────────────
    pub delta_pos: [f32; 3],
    pub delta_vel_packed: [u16; 4],
    pub rot_or_sh_delta: u32,

    // ── Word 8..11 (16 bytes): Context, Clock & Parity ──────────────────────────
    pub context_cell: u32,
    pub lamport_epoch: u32,
    pub dirty_tile_mask: u32,
    pub ecc_parity: u32,
}

impl Default for DeltaNQuin {
    fn default() -> Self {
        Self {
            subject_id: 0,
            predicate_op: OP_DIFF_TRANSFORM_DELTA,
            salience_qp: SALIENCE_CLASS_A_FOCAL,
            medium_branch: 0,
            delta_pos: [0.0; 3],
            delta_vel_packed: [0; 4],
            rot_or_sh_delta: 0,
            context_cell: 0,
            lamport_epoch: 0,
            dirty_tile_mask: 0,
            ecc_parity: 0,
        }
    }
}

impl DeltaNQuin {
    /// Constructs a transform delta for an entity.
    pub fn new_transform_delta(
        subject_id: u32,
        context_cell: u32,
        delta_pos: [f32; 3],
        vel_mps: [f32; 3],
        lod_scale: f32,
        epoch: u32,
    ) -> Self {
        let mut quin = Self {
            subject_id,
            predicate_op: OP_DIFF_TRANSFORM_DELTA,
            salience_qp: SALIENCE_CLASS_A_FOCAL,
            medium_branch: 0,
            delta_pos,
            delta_vel_packed: [
                pack_f32_to_f16(vel_mps[0]),
                pack_f32_to_f16(vel_mps[1]),
                pack_f32_to_f16(vel_mps[2]),
                pack_f32_to_f16(lod_scale),
            ],
            rot_or_sh_delta: 0,
            context_cell,
            lamport_epoch: epoch,
            dirty_tile_mask: 0,
            ecc_parity: 0,
        };
        quin.ecc_parity = quin.compute_parity();
        quin
    }

    /// Constructs an N3Logic optical isolation bitmask delta.
    pub fn new_optical_isolation(
        cell_id: u32,
        affected_tiles_bitmask: u32,
        epoch: u32,
    ) -> Self {
        let mut quin = Self {
            subject_id: cell_id,
            predicate_op: OP_DIFF_ISOLATION_MASK,
            salience_qp: SALIENCE_CLASS_C_AMBIENT,
            medium_branch: 0,
            delta_pos: [0.0; 3],
            delta_vel_packed: [0; 4],
            rot_or_sh_delta: 0,
            context_cell: cell_id,
            lamport_epoch: epoch,
            dirty_tile_mask: affected_tiles_bitmask,
            ecc_parity: 0,
        };
        quin.ecc_parity = quin.compute_parity();
        quin
    }

    /// Computes XOR folding parity across all fields.
    pub fn compute_parity(&self) -> u32 {
        self.subject_id
            ^ (self.predicate_op as u32)
            ^ ((self.salience_qp as u32) << 16)
            ^ ((self.medium_branch as u32) << 24)
            ^ self.delta_pos[0].to_bits()
            ^ self.delta_pos[1].to_bits()
            ^ self.delta_pos[2].to_bits()
            ^ ((self.delta_vel_packed[0] as u32) | ((self.delta_vel_packed[1] as u32) << 16))
            ^ ((self.delta_vel_packed[2] as u32) | ((self.delta_vel_packed[3] as u32) << 16))
            ^ self.rot_or_sh_delta
            ^ self.context_cell
            ^ self.lamport_epoch
            ^ self.dirty_tile_mask
    }

    /// Verifies data integrity via XOR parity fold.
    #[inline]
    pub fn is_parity_valid(&self) -> bool {
        self.ecc_parity == self.compute_parity()
    }
}

/// Helper function to pack an f32 into an IEEE 754 half-precision u16.
pub fn pack_f32_to_f16(val: f32) -> u16 {
    let bits = val.to_bits();
    let sign = (bits >> 31) & 0x1;
    let exp = ((bits >> 23) & 0xFF) as i32;
    let mant = bits & 0x7FFFFF;

    if exp == 255 {
        // Inf / NaN
        return ((sign << 15) | 0x7C00 | (mant >> 13).min(0x3FF)) as u16;
    }
    let new_exp = exp - 127 + 15;
    if new_exp >= 31 {
        // Overflow to Inf
        return ((sign << 15) | 0x7C00) as u16;
    }
    if new_exp <= 0 {
        // Subnormal or underflow
        return (sign << 15) as u16;
    }
    let new_mant = mant >> 13;
    ((sign << 15) | ((new_exp as u32) << 10) | new_mant) as u16
}

/// Helper function to unpack an IEEE 754 half-precision u16 into an f32.
pub fn unpack_f16_to_f32(h: u16) -> f32 {
    let sign = ((h >> 15) & 0x1) as u32;
    let exp = ((h >> 10) & 0x1F) as u32;
    let mant = (h & 0x3FF) as u32;

    if exp == 31 {
        let f_exp = 255u32;
        let f_mant = mant << 13;
        return f32::from_bits((sign << 31) | (f_exp << 23) | f_mant);
    }
    if exp == 0 {
        if mant == 0 {
            return f32::from_bits(sign << 31);
        }
        // Subnormal
        let mut m = mant;
        let mut shift = 0;
        while (m & 0x400) == 0 {
            m <<= 1;
            shift += 1;
        }
        let f_exp = (127 - 14 - shift) as u32;
        let f_mant = (m & 0x3FF) << 13;
        return f32::from_bits((sign << 31) | (f_exp << 23) | f_mant);
    }
    let f_exp = (exp + 127 - 15) as u32;
    let f_mant = mant << 13;
    f32::from_bits((sign << 31) | (f_exp << 23) | f_mant)
}

/// Evaluates a batch of differential quins, checking epoch watermarks and accumulating dirty tiles.
///
/// Respects QualiaDB zero-heap contract by writing into caller-supplied output buffers.
pub fn evaluate_differential_frame(
    delta_quins: &[DeltaNQuin],
    active_epoch: u32,
    out_dirty_tiles: &mut [u32],
) -> Result<usize, &'static str> {
    if delta_quins.is_empty() {
        return Ok(0);
    }

    let mut dirty_count = 0usize;

    for quin in delta_quins {
        if !quin.is_parity_valid() {
            return Err("ECC parity failure in DeltaNQuin packet");
        }

        // Causal Epoch Watermarking: Ignore state updates from outdated epochs
        if quin.lamport_epoch < active_epoch {
            continue;
        }

        match quin.predicate_op {
            OP_DIFF_TRANSFORM_DELTA | OP_DIFF_RADIANCE_DELTA => {
                // If tile mask provided, accumulate into caller's dirty tiles
                if quin.dirty_tile_mask != 0 && dirty_count < out_dirty_tiles.len() {
                    out_dirty_tiles[dirty_count] = quin.dirty_tile_mask;
                    dirty_count += 1;
                }
            }
            OP_DIFF_ISOLATION_MASK => {
                // N3Logic optical topology update
                if dirty_count < out_dirty_tiles.len() {
                    out_dirty_tiles[dirty_count] = quin.dirty_tile_mask;
                    dirty_count += 1;
                }
            }
            OP_DIFF_LTR_PROMOTION => {
                // LTR proxy activation: flags tile dirty on transition
                if dirty_count < out_dirty_tiles.len() {
                    out_dirty_tiles[dirty_count] = quin.dirty_tile_mask;
                    dirty_count += 1;
                }
            }
            _ => {}
        }
    }

    Ok(dirty_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, size_of};

    #[test]
    fn test_delta_nquin_size_and_alignment() {
        assert_eq!(
            size_of::<DeltaNQuin>(),
            48,
            "DeltaNQuin must be exactly 48 bytes"
        );
        assert_eq!(
            align_of::<DeltaNQuin>(),
            16,
            "DeltaNQuin must be 16-byte aligned for GPU SIMD loading"
        );
    }

    #[test]
    fn test_f16_round_trip() {
        let test_values = [0.0f32, 1.0f32, -1.0f32, 24.5f32, 0.0035f32, -120.75f32];
        for &val in &test_values {
            let packed = pack_f32_to_f16(val);
            let unpacked = unpack_f16_to_f32(packed);
            let error = (val - unpacked).abs();
            assert!(
                error < 0.1,
                "f16 pack/unpack error {} too large for val {}",
                error,
                val
            );
        }
    }

    #[test]
    fn test_causal_epoch_watermark_invalidation() {
        let current_epoch = 120u32;
        let stale_quin = DeltaNQuin {
            subject_id: 101,
            predicate_op: OP_DIFF_TRANSFORM_DELTA,
            lamport_epoch: 110, // Stale!
            dirty_tile_mask: 0x000000FF,
            ..Default::default()
        };
        let mut stale_quin_valid = stale_quin;
        stale_quin_valid.ecc_parity = stale_quin_valid.compute_parity();

        let fresh_quin = DeltaNQuin {
            subject_id: 102,
            predicate_op: OP_DIFF_TRANSFORM_DELTA,
            lamport_epoch: 121, // Fresh
            dirty_tile_mask: 0x0000FF00,
            ..Default::default()
        };
        let mut fresh_quin_valid = fresh_quin;
        fresh_quin_valid.ecc_parity = fresh_quin_valid.compute_parity();

        let mut output_tiles = [0u32; 16];
        let count = evaluate_differential_frame(
            &[stale_quin_valid, fresh_quin_valid],
            current_epoch,
            &mut output_tiles,
        )
        .expect("Evaluation failed");

        assert_eq!(count, 1);
        assert_eq!(output_tiles[0], 0x0000FF00);
    }

    #[test]
    fn test_optical_isolation_delta() {
        let quin = DeltaNQuin::new_optical_isolation(42, 0x0F0F0F0F, 10);
        assert!(quin.is_parity_valid());
        assert_eq!(quin.predicate_op, OP_DIFF_ISOLATION_MASK);
        assert_eq!(quin.dirty_tile_mask, 0x0F0F0F0F);
    }
}
