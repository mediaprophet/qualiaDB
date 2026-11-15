//! Mode 0x03: Fixed 64-Byte Phase Juncture & Action Record (`M64PhaseJunctureRecord`).
//!
//! Encodes stationary-action phase junctures, including dynamic tool excavations,
//! structural collapses, and parameterized cartoon kinetics (such as volume deficits
//! for squash-and-stretch reinflation and passive actor provenance).

use super::chunk_directory::M64Error;
use crate::container_10d::crc32c::crc32c;

pub const PHASE_JUNCTURE_RECORD_BYTES: usize = 64;

pub const FLAG_TOOL_EXCAVATION: u32 = 1 << 0;
pub const FLAG_STRATUM_COLLAPSE: u32 = 1 << 1;
pub const FLAG_SURFACE_FLOOD: u32 = 1 << 2;
pub const FLAG_CARTOON_WAFER: u32 = 1 << 3;
pub const FLAG_SPECULATIVE_BRANCH: u32 = 1 << 4;

/// Fixed 64-Byte Phase Juncture Record (Mode 0x03).
///
/// Fields are ordered by descending alignment requirements (8B -> 4B -> 2B -> 1B)
/// to ensure exactly 64 bytes with zero compiler-inserted padding.
#[repr(C, align(64))]
#[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct M64PhaseJunctureRecord {
    // --- 8-byte aligned fields (16 bytes, offsets 0..16) ---
    /// 8B: FNV-1a hash of actor DID responsible for state change (zero-blocker provenance).
    pub provenance_did_hash: u64,
    /// 8B: Monotonic Lamport timestamp of modification.
    pub lamport_clock: u64,

    // --- 4-byte aligned fields (40 bytes, offsets 16..56) ---
    /// 12B: Focal spatial center (x, y, z) in mm relative to geodetic datum.
    pub anchor_pos_mm: [i32; 3],
    /// 4B: Simulation branch q (0 = ground truth, >0 = speculative/counterfactual).
    pub quantum_branch_id: u32,
    /// 4B: Boundary condition flags (tool excavation, collapse, flood, cartoon wafer).
    pub boundary_flags: u32,
    /// 4B: Stationary-action value; holds volume deficit ΔV (m³) for cartoon reinflation.
    pub action_integral_s: f32,
    /// 16B: Relativistic T_μν / continuum Cauchy stress components [σ_xx, σ_yy, σ_zz, τ_xy].
    pub stress_energy_tensor: [f32; 4],

    // --- 2-byte & 1-byte aligned fields (4 bytes, offsets 56..60) ---
    /// 2B: Spherical/CSG radius of tool excavation cavity in mm.
    pub excavation_radius_mm: u16,
    /// 1B: LUT index of material removed or transformed.
    pub excavated_lut_material: u8,
    /// 1B: Simulation ticks remaining before cartoon reinflation dynamics trigger.
    pub re_inflation_delay_ticks: u8,

    // --- Checksum (4 bytes, offsets 60..64) ---
    /// 4B: CRC-32C checksum of bytes 0..59 (Total = 64B).
    pub record_crc32c: u32,
}

impl Default for M64PhaseJunctureRecord {
    fn default() -> Self {
        Self {
            provenance_did_hash: 0,
            lamport_clock: 0,
            anchor_pos_mm: [0, 0, 0],
            quantum_branch_id: 0,
            boundary_flags: 0,
            action_integral_s: 0.0,
            stress_energy_tensor: [0.0; 4],
            excavation_radius_mm: 0,
            excavated_lut_material: 0,
            re_inflation_delay_ticks: 0,
            record_crc32c: 0,
        }
    }
}

impl M64PhaseJunctureRecord {
    /// Compute the CRC-32C over the first 60 bytes of the record.
    #[inline]
    pub fn compute_crc32c(&self) -> u32 {
        let bytes: &[u8; PHASE_JUNCTURE_RECORD_BYTES] = bytemuck::cast_ref(self);
        crc32c(&bytes[..60])
    }

    /// Update `record_crc32c` with the computed CRC-32C.
    #[inline]
    pub fn update_crc(&mut self) {
        self.record_crc32c = self.compute_crc32c();
    }

    /// Validate the CRC-32C of this phase juncture record.
    #[inline]
    pub fn validate(&self) -> Result<(), M64Error> {
        let computed = self.compute_crc32c();
        if self.record_crc32c == computed {
            Ok(())
        } else {
            Err(M64Error::ChecksumMismatch {
                stored: self.record_crc32c,
                computed,
            })
        }
    }

    /// Returns true if this record belongs to the veridical ground-truth branch (q = 0).
    #[inline]
    pub fn is_ground_truth(&self) -> bool {
        self.quantum_branch_id == 0
    }

    /// Returns the volume deficit ΔV in m³ when boundary flags indicate cartoon wafer deformation.
    #[inline]
    pub fn volume_deficit_m3(&self) -> f32 {
        self.action_integral_s
    }

    /// Sets the volume deficit ΔV in m³ and marks the cartoon wafer flag.
    #[inline]
    pub fn set_cartoon_wafer_deficit(&mut self, deficit_m3: f32, delay_ticks: u8) {
        self.boundary_flags |= FLAG_CARTOON_WAFER;
        self.action_integral_s = deficit_m3;
        self.re_inflation_delay_ticks = delay_ticks;
    }
}
