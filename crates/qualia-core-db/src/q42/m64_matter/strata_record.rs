//! Mode 0x02: Fixed 64-Byte Strata Column & Fluid Record (`M64StrataColumnRecord`).
//!
//! Encodes surface fluid depth and velocity, along with a 6-layer subsurface
//! stratigraphic stack (topsoil, clay, alluvial gravel, weathered clasts,
//! fractured basalt, and bedrock) and bulk homogenized sensor metrics.

use super::chunk_directory::M64Error;
use crate::container_10d::crc32c::crc32c;

pub const STRATA_RECORD_BYTES: usize = 64;
pub const STRATA_LAYER_COUNT: usize = 6;

/// Fixed 64-Byte Strata Column Record (Mode 0x02).
#[repr(C, align(64))]
#[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct M64StrataColumnRecord {
    /// 4B: Surface elevation Z in mm relative to geodetic datum.
    pub surface_elevation_mm: i32,
    /// 2B: Depth from surface to water table in mm.
    pub water_table_depth_mm: u16,
    /// 2B: Depth of flowing surface water (river/flood) in mm.
    pub surface_fluid_depth_mm: u16,
    /// 4B: Surface fluid flow velocity vector in mm/s (vx, vy).
    pub fluid_velocity_xy: [i16; 2],
    /// 2B: Suspended sediment concentration in parts-per-million.
    pub fluid_sediment_load_ppm: u16,
    /// 2B: Hydrodynamic Mannings n / surface roughness in mm.
    pub surface_roughness_mm: u16,

    // 6-Layer Subsurface Stratigraphic Stack (Total = 36B)
    /// 12B: Thickness of layers 0..5 in mm (0: Topsoil, 1: Clay, 2: Gravel, 3: Clasts, 4: Basalt, 5: Bedrock).
    pub layer_thickness_mm: [u16; 6],
    /// 6B: Index into 16-KB ConstituentLUT (Bank 0 or 1) per layer.
    pub layer_lut_index: [u8; 6],
    /// 6B: Volumetric water saturation Sw (0..100 maps to 0..255) per layer.
    pub layer_moisture_pct: [u8; 6],
    /// 6B: Overburden compaction ratio (0..100 maps to 0..255) per layer.
    pub layer_compaction_pct: [u8; 6],
    /// 6B: Bitmask of embedded artefacts (gold flakes, quartz clasts, charcoal) per layer.
    pub layer_artefact_mask: [u8; 6],

    // Macro Mechanical / Sensor Summary (Total = 8B)
    /// 2B: Homogenized Mohr-Coulomb cohesion c in kPa.
    pub bulk_shear_strength_kpa: u16,
    /// 2B: Bulk angle of repose θ in 0.1 degree units (e.g. 340 = 34.0°).
    pub angle_of_repose_deg10: u16,
    /// 2B: CRIM complex relative permittivity ε_eff * 100.
    pub gpr_dielectric_perm_x100: u16,
    /// 2B: Biot poroelastic P-wave velocity in m/s.
    pub acoustic_p_wave_m_s: u16,

    /// 4B: CRC-32C checksum of bytes 0..59 (Total = 64B).
    pub record_crc32c: u32,
}

impl Default for M64StrataColumnRecord {
    fn default() -> Self {
        Self {
            surface_elevation_mm: 0,
            water_table_depth_mm: 1200,
            surface_fluid_depth_mm: 0,
            fluid_velocity_xy: [0, 0],
            fluid_sediment_load_ppm: 0,
            surface_roughness_mm: 25,
            layer_thickness_mm: [400, 800, 1600, 1200, 2000, 10000],
            layer_lut_index: [1, 2, 3, 4, 5, 6],
            layer_moisture_pct: [64, 128, 192, 128, 64, 10],
            layer_compaction_pct: [30, 80, 150, 180, 220, 255],
            layer_artefact_mask: [0; 6],
            bulk_shear_strength_kpa: 45,
            angle_of_repose_deg10: 340,
            gpr_dielectric_perm_x100: 650,
            acoustic_p_wave_m_s: 1850,
            record_crc32c: 0,
        }
    }
}

impl M64StrataColumnRecord {
    /// Compute the CRC-32C over the first 60 bytes of the record.
    #[inline]
    pub fn compute_crc32c(&self) -> u32 {
        let bytes: &[u8; STRATA_RECORD_BYTES] = bytemuck::cast_ref(self);
        crc32c(&bytes[..60])
    }

    /// Update `record_crc32c` with the computed CRC-32C.
    #[inline]
    pub fn update_crc(&mut self) {
        self.record_crc32c = self.compute_crc32c();
    }

    /// Validate the CRC-32C of this strata record.
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

    /// Total depth of the upper 5 soil/weathered layers in millimeters (excluding basement bedrock layer 5).
    #[inline]
    pub fn total_soil_depth_mm(&self) -> u32 {
        let mut sum = 0u32;
        for i in 0..5 {
            sum += self.layer_thickness_mm[i] as u32;
        }
        sum
    }

    /// Absolute elevation of the solid bedrock interface in millimeters relative to datum.
    #[inline]
    pub fn bedrock_elevation_mm(&self) -> i32 {
        self.surface_elevation_mm - (self.total_soil_depth_mm() as i32)
    }

    /// Returns the stratum layer index (0..5) present at the specified depth in mm beneath the surface.
    #[inline]
    pub fn layer_at_depth_mm(&self, depth_mm: u32) -> usize {
        let mut accumulated_mm = 0u32;
        for i in 0..5 {
            accumulated_mm += self.layer_thickness_mm[i] as u32;
            if depth_mm < accumulated_mm {
                return i;
            }
        }
        5 // Bedrock basement
    }
}
