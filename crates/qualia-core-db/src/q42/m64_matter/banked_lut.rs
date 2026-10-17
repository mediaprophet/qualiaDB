//! Three-Bank 16-KB GPU Constituent Look-Up Table (`BankedConstituentLut`).
//!
//! Provides the universal 16-KB GPU UBO/SSBO buffer:
//! - Bank 0 (`0x00..0x7F`, 128 slots): Pinned global terrestrial baseline.
//! - Bank 1 (`0x80..0xDF`, 96 slots): Dynamically paged per chunk boundary.
//! - Bank 2 (`0xE0..0xFF`, 32 slots): Blend window & reactive phase materials.

use super::chunk_directory::M64Error;
use super::lut_entry::{Q42ConstituentLutEntry, CONSTITUENT_LUT_ENTRY_BYTES};

pub const LUT_TOTAL_ENTRIES: usize = 256;
pub const BANK_0_PINNED_COUNT: usize = 128;
pub const BANK_1_PAGED_COUNT: usize = 96;
pub const BANK_2_BLEND_COUNT: usize = 32;

pub const BANK_0_OFFSET: usize = 0;
pub const BANK_1_OFFSET: usize = 128;
pub const BANK_2_OFFSET: usize = 224;

pub const LUT_BUFFER_BYTES: usize = LUT_TOTAL_ENTRIES * CONSTITUENT_LUT_ENTRY_BYTES; // 16,384 bytes (16 KB)
pub const BANK_1_PAGED_BYTES: usize = BANK_1_PAGED_COUNT * CONSTITUENT_LUT_ENTRY_BYTES; // 6,144 bytes (6 KB)

/// Three-Bank 16-KB Constituent Look-Up Table Manager.
#[repr(C, align(64))]
#[derive(Clone, Debug)]
pub struct BankedConstituentLut {
    /// 16 KB flat buffer of 256 constituent entries.
    pub entries: [Q42ConstituentLutEntry; LUT_TOTAL_ENTRIES],
    /// Epoch identifier of the currently loaded Bank 1 recipe set.
    pub active_bank_1_epoch_id: u16,
    /// Dirty flag indicating a GPU DMA re-upload is required.
    pub is_dirty: bool,
}

impl Default for BankedConstituentLut {
    fn default() -> Self {
        Self::new_with_terrestrial_defaults()
    }
}

impl BankedConstituentLut {
    /// Initialize with certified Australian & terrestrial mineral baseline in Bank 0.
    pub fn new_with_terrestrial_defaults() -> Self {
        let mut entries = [Q42ConstituentLutEntry::default(); LUT_TOTAL_ENTRIES];

        // Slot 0: Air / Vacuum
        entries[0] = Q42ConstituentLutEntry {
            albedo_rgb_f16: [0, 0, 0],
            roughness_f16: 0,
            kubelka_munk_k_s: [0, 0],
            refractive_index_x100: 100, // 1.00
            bulk_density_kg_m3: 1,      // 1.2 kg/m³
            porosity_pct: 100,
            friction_angle_deg: 0,
            cohesion_kpa: 0,
            youngs_modulus_mpa: 0,
            poissons_ratio_x100: 0,
            _pad0: 0,
            permeability_darcy_x100: 65535,
            suction_head_mm: 0,
            van_genuchten_n_x100: 0,
            dielectric_permittivity_x100: 100, // 1.0
            electrical_conductivity_ms_m: 0,
            acoustic_p_wave_m_s: 343,
            ceremonial_permission_mask: 0,
            canonical_q42_material_hash: 0,
            provenance_did_hash: 0,
            _reserved: [0; 4],
            entry_crc32c: 0,
        };
        entries[0].update_crc();

        // Slot 1: Topsoil / Organic Humus
        entries[1] = Q42ConstituentLutEntry {
            albedo_rgb_f16: [0x3100, 0x2C00, 0x2800], // Dark rich brown
            roughness_f16: 0x3C00,                     // 0.8
            kubelka_munk_k_s: [0x3800, 0x3000],
            refractive_index_x100: 145,
            bulk_density_kg_m3: 1300,
            porosity_pct: 55,
            friction_angle_deg: 28,
            cohesion_kpa: 12,
            youngs_modulus_mpa: 20,
            poissons_ratio_x100: 35,
            _pad0: 0,
            permeability_darcy_x100: 350,
            suction_head_mm: 250,
            van_genuchten_n_x100: 130,
            dielectric_permittivity_x100: 380,
            electrical_conductivity_ms_m: 25,
            acoustic_p_wave_m_s: 450,
            ceremonial_permission_mask: 0,
            canonical_q42_material_hash: 1,
            provenance_did_hash: 0,
            _reserved: [0; 4],
            entry_crc32c: 0,
        };
        entries[1].update_crc();

        // Slot 2: Kaolinite / Silty Clay
        entries[2] = Q42ConstituentLutEntry {
            albedo_rgb_f16: [0x3900, 0x3700, 0x3400], // Buff pale tan
            roughness_f16: 0x3800,
            kubelka_munk_k_s: [0x3000, 0x3C00],
            refractive_index_x100: 156,
            bulk_density_kg_m3: 1750,
            porosity_pct: 42,
            friction_angle_deg: 22,
            cohesion_kpa: 45,
            youngs_modulus_mpa: 80,
            poissons_ratio_x100: 40,
            _pad0: 0,
            permeability_darcy_x100: 2,
            suction_head_mm: 800,
            van_genuchten_n_x100: 115,
            dielectric_permittivity_x100: 650,
            electrical_conductivity_ms_m: 120,
            acoustic_p_wave_m_s: 1450,
            ceremonial_permission_mask: 0,
            canonical_q42_material_hash: 2,
            provenance_did_hash: 0,
            _reserved: [0; 4],
            entry_crc32c: 0,
        };
        entries[2].update_crc();

        // Slot 3: Quartz Sand / Alluvial Gravel
        entries[3] = Q42ConstituentLutEntry {
            albedo_rgb_f16: [0x3A00, 0x3800, 0x3200], // Golden sand
            roughness_f16: 0x3400,
            kubelka_munk_k_s: [0x2C00, 0x3E00],
            refractive_index_x100: 154,
            bulk_density_kg_m3: 2050,
            porosity_pct: 32,
            friction_angle_deg: 36,
            cohesion_kpa: 0,
            youngs_modulus_mpa: 150,
            poissons_ratio_x100: 25,
            _pad0: 0,
            permeability_darcy_x100: 850,
            suction_head_mm: 80,
            van_genuchten_n_x100: 210,
            dielectric_permittivity_x100: 450,
            electrical_conductivity_ms_m: 4,
            acoustic_p_wave_m_s: 1850,
            ceremonial_permission_mask: 0,
            canonical_q42_material_hash: 3,
            provenance_did_hash: 0,
            _reserved: [0; 4],
            entry_crc32c: 0,
        };
        entries[3].update_crc();

        // Slot 4: Weathered Clasts / Silcrete
        entries[4] = Q42ConstituentLutEntry {
            albedo_rgb_f16: [0x3600, 0x3300, 0x3000],
            roughness_f16: 0x3A00,
            kubelka_munk_k_s: [0x3200, 0x3600],
            refractive_index_x100: 158,
            bulk_density_kg_m3: 2300,
            porosity_pct: 18,
            friction_angle_deg: 40,
            cohesion_kpa: 150,
            youngs_modulus_mpa: 12000,
            poissons_ratio_x100: 22,
            _pad0: 0,
            permeability_darcy_x100: 15,
            suction_head_mm: 350,
            van_genuchten_n_x100: 160,
            dielectric_permittivity_x100: 520,
            electrical_conductivity_ms_m: 10,
            acoustic_p_wave_m_s: 2900,
            ceremonial_permission_mask: 0,
            canonical_q42_material_hash: 4,
            provenance_did_hash: 0,
            _reserved: [0; 4],
            entry_crc32c: 0,
        };
        entries[4].update_crc();

        // Slot 5: Fractured Basalt / Monaro Volcanics
        entries[5] = Q42ConstituentLutEntry {
            albedo_rgb_f16: [0x2900, 0x2900, 0x2A00], // Dark charcoal/slate
            roughness_f16: 0x3200,
            kubelka_munk_k_s: [0x3C00, 0x2800],
            refractive_index_x100: 162,
            bulk_density_kg_m3: 2800,
            porosity_pct: 8,
            friction_angle_deg: 45,
            cohesion_kpa: 1200,
            youngs_modulus_mpa: 45000,
            poissons_ratio_x100: 20,
            _pad0: 0,
            permeability_darcy_x100: 1,
            suction_head_mm: 50,
            van_genuchten_n_x100: 140,
            dielectric_permittivity_x100: 750,
            electrical_conductivity_ms_m: 2,
            acoustic_p_wave_m_s: 4800,
            ceremonial_permission_mask: 0,
            canonical_q42_material_hash: 5,
            provenance_did_hash: 0,
            _reserved: [0; 4],
            entry_crc32c: 0,
        };
        entries[5].update_crc();

        // Slot 6: Solid Granitic Bedrock
        entries[6] = Q42ConstituentLutEntry {
            albedo_rgb_f16: [0x3800, 0x3700, 0x3600], // Mottled pale granite
            roughness_f16: 0x2E00,
            kubelka_munk_k_s: [0x2E00, 0x3A00],
            refractive_index_x100: 155,
            bulk_density_kg_m3: 2700,
            porosity_pct: 1,
            friction_angle_deg: 52,
            cohesion_kpa: 5000,
            youngs_modulus_mpa: 60000,
            poissons_ratio_x100: 18,
            _pad0: 0,
            permeability_darcy_x100: 0,
            suction_head_mm: 10,
            van_genuchten_n_x100: 100,
            dielectric_permittivity_x100: 600,
            electrical_conductivity_ms_m: 1,
            acoustic_p_wave_m_s: 5800,
            ceremonial_permission_mask: 0,
            canonical_q42_material_hash: 6,
            provenance_did_hash: 0,
            _reserved: [0; 4],
            entry_crc32c: 0,
        };
        entries[6].update_crc();

        // Slot 7: Fresh Water (Surface flow / riverbed)
        entries[7] = Q42ConstituentLutEntry {
            albedo_rgb_f16: [0x2800, 0x3400, 0x3800], // Blue-cyan tint
            roughness_f16: 0x1400,                     // 0.05 mirror smooth
            kubelka_munk_k_s: [0x1000, 0x1000],
            refractive_index_x100: 133, // 1.333
            bulk_density_kg_m3: 1000,
            porosity_pct: 100,
            friction_angle_deg: 0,
            cohesion_kpa: 0,
            youngs_modulus_mpa: 2200, // Bulk modulus ~2.2 GPa
            poissons_ratio_x100: 50,
            _pad0: 0,
            permeability_darcy_x100: 65535,
            suction_head_mm: 0,
            van_genuchten_n_x100: 0,
            dielectric_permittivity_x100: 8000, // ε_r = 80.0
            electrical_conductivity_ms_m: 50,
            acoustic_p_wave_m_s: 1482,
            ceremonial_permission_mask: 0,
            canonical_q42_material_hash: 7,
            provenance_did_hash: 0,
            _reserved: [0; 4],
            entry_crc32c: 0,
        };
        entries[7].update_crc();

        // Update remaining slots in Bank 0
        for i in 8..BANK_0_PINNED_COUNT {
            entries[i].update_crc();
        }

        Self {
            entries,
            active_bank_1_epoch_id: 0,
            is_dirty: true,
        }
    }

    /// Dynamically page Bank 1 (slots 128..223, 6 KB) without modifying Bank 0 or Bank 2.
    pub fn page_bank_1(
        &mut self,
        epoch_id: u16,
        paged_recipes: &[Q42ConstituentLutEntry; BANK_1_PAGED_COUNT],
    ) -> Result<(), M64Error> {
        self.entries[BANK_1_OFFSET..BANK_1_OFFSET + BANK_1_PAGED_COUNT]
            .copy_from_slice(paged_recipes);
        self.active_bank_1_epoch_id = epoch_id;
        self.is_dirty = true;
        Ok(())
    }

    /// Zero-copy cast into uniform/storage buffer upload byte slice (16,384 bytes).
    #[inline]
    pub fn as_gpu_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.entries)
    }

    /// Read entry by index (0..255).
    #[inline]
    pub fn entry(&self, index: u8) -> &Q42ConstituentLutEntry {
        &self.entries[index as usize]
    }

    /// Mutable access to entry by index.
    #[inline]
    pub fn entry_mut(&mut self, index: u8) -> &mut Q42ConstituentLutEntry {
        self.is_dirty = true;
        &mut self.entries[index as usize]
    }
}
