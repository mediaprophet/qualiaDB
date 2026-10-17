//! 64-Byte Universal Constituent LUT Entry (`Q42ConstituentLutEntry`).
//!
//! Encodes multi-physics material parameters (optical BRDF, Kubelka-Munk pigment
//! diffusion, geotechnical mechanics, Darcy hydrology, electromagnetics, and
//! passive biocultural/open-commons provenance DID hashes).

use super::chunk_directory::M64Error;
use crate::container_10d::crc32c::crc32c;

pub const CONSTITUENT_LUT_ENTRY_BYTES: usize = 64;

/// Fixed 64-Byte Constituent Look-Up Table Entry.
#[repr(C, align(64))]
#[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Q42ConstituentLutEntry {
    // --- 1. Optical & Radiometric Properties (14B) ---
    /// 6B: Diffuse albedo in linear RGB (f16 half-floats).
    pub albedo_rgb_f16: [u16; 3],
    /// 2B: Surface microfacet roughness (f16 half-float).
    pub roughness_f16: u16,
    /// 4B: Kubelka-Munk absorption (K) and scattering (S) coefficients (f16).
    pub kubelka_munk_k_s: [u16; 2],
    /// 2B: Real refractive index n * 100 (e.g. 154 = 1.54 for Quartz).
    pub refractive_index_x100: u16,

    // --- 2. Physical & Geotechnical Mechanics (18B) ---
    /// 2B: Solid grain density in kg/m³ (e.g. 2650 for Quartz, 1000 for Water).
    pub bulk_density_kg_m3: u16,
    /// 1B: Material porosity ratio φ (0..100%).
    pub porosity_pct: u8,
    /// 1B: Internal friction angle φ_friction in degrees (e.g. 34° for sand).
    pub friction_angle_deg: u8,
    /// 2B: Mohr-Coulomb cohesion c in kPa (e.g. 0 for sand, 25 for stiff clay).
    pub cohesion_kpa: u16,
    /// 2B: Young's Modulus E in MPa (e.g. 35000 MPa for granite).
    pub youngs_modulus_mpa: u16,
    /// 1B: Poisson's ratio ν * 100 (e.g. 25 = 0.25).
    pub poissons_ratio_x100: u8,
    /// 1B: Padding for alignment.
    pub _pad0: u8,

    // --- 3. Hydrodynamic & Electromagnetic Properties (16B) ---
    /// 2B: Hydraulic permeability κ in 0.01 Darcy units (e.g. 150 = 1.50 Darcy).
    pub permeability_darcy_x100: u16,
    /// 2B: Capillary suction head in mm.
    pub suction_head_mm: u16,
    /// 2B: van Genuchten empirical exponent n * 100.
    pub van_genuchten_n_x100: u16,
    /// 2B: Real relative dielectric permittivity ε_r * 100 (e.g. 450 = 4.5 for quartz).
    pub dielectric_permittivity_x100: u16,
    /// 2B: Electrical conductivity in mS/m.
    pub electrical_conductivity_ms_m: u16,
    /// 2B: Biot poroelastic P-wave acoustic velocity in m/s.
    pub acoustic_p_wave_m_s: u16,
    /// 4B: Biocultural/ceremonial permissions & customary access mask.
    pub ceremonial_permission_mask: u32,

    // --- 4. Semantic Identity & Provenance (16B) ---
    /// 8B: Canonical FNV-1a hash of the `did:q42` mineral/material IRI.
    pub canonical_q42_material_hash: u64,
    /// 8B: FNV-1a hash of the provenance DID / author custodian.
    pub provenance_did_hash: u64,

    // --- 5. Integrity & Alignment (8B) ---
    /// 4B: Explicit zero reserved.
    pub _reserved: [u8; 4],
    /// 4B: CRC-32C checksum of bytes 0..59 (Total = 64B).
    pub entry_crc32c: u32,
}

impl Default for Q42ConstituentLutEntry {
    fn default() -> Self {
        Self {
            albedo_rgb_f16: [0x3800, 0x3800, 0x3800], // 0.5 in f16
            roughness_f16: 0x3800,                     // 0.5 in f16
            kubelka_munk_k_s: [0x3000, 0x3800],
            refractive_index_x100: 150,
            bulk_density_kg_m3: 2650,
            porosity_pct: 35,
            friction_angle_deg: 32,
            cohesion_kpa: 10,
            youngs_modulus_mpa: 25000,
            poissons_ratio_x100: 25,
            _pad0: 0,
            permeability_darcy_x100: 100,
            suction_head_mm: 100,
            van_genuchten_n_x100: 180,
            dielectric_permittivity_x100: 450,
            electrical_conductivity_ms_m: 5,
            acoustic_p_wave_m_s: 2200,
            ceremonial_permission_mask: 0,
            canonical_q42_material_hash: 0,
            provenance_did_hash: 0,
            _reserved: [0; 4],
            entry_crc32c: 0,
        }
    }
}

impl Q42ConstituentLutEntry {
    /// Compute the CRC-32C over the first 60 bytes of the record.
    #[inline]
    pub fn compute_crc32c(&self) -> u32 {
        let bytes: &[u8; CONSTITUENT_LUT_ENTRY_BYTES] = bytemuck::cast_ref(self);
        crc32c(&bytes[..60])
    }

    /// Update `entry_crc32c` with the computed CRC-32C.
    #[inline]
    pub fn update_crc(&mut self) {
        self.entry_crc32c = self.compute_crc32c();
    }

    /// Validate the CRC-32C of this LUT entry.
    #[inline]
    pub fn validate(&self) -> Result<(), M64Error> {
        let computed = self.compute_crc32c();
        if self.entry_crc32c == computed {
            Ok(())
        } else {
            Err(M64Error::ChecksumMismatch {
                stored: self.entry_crc32c,
                computed,
            })
        }
    }
}
