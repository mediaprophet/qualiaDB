//! Geological Feature & Vector Attribute Parser (WFS / GeoSPARQL).
//!
//! Dual-path "truth" ingestion: parses geological formations, lithology attributes,
//! and stratigraphic ages from vector data to populate BankedConstituentLut mineral recipes.

use qualia_core_db::q42::m64_matter::{Q42ConstituentLutEntry, BANK_1_PAGED_COUNT};

/// A parsed geological stratigraphic formation record.
#[derive(Debug, Clone, PartialEq)]
pub struct GeologicUnitRecord {
    /// Formation unit code (e.g. "Jmb" = Tertiary Monaro Basalt, "Gda" = Adamaby Granite).
    pub unit_code: String,
    /// Dominant rock classification (e.g. "Basalt", "Granite", "Silcrete", "Kaolinite Clay").
    pub lithology: String,
    /// Absolute geological age in millions of years (Ma).
    pub stratigraphic_age_ma: f64,
    /// Assigned LUT index in Bank 1 (128..223).
    pub lut_index: u8,
}

/// Geological Vector Layer manager.
#[derive(Debug, Clone)]
pub struct GeologicVectorLayer {
    pub units: Vec<GeologicUnitRecord>,
}

impl Default for GeologicVectorLayer {
    fn default() -> Self {
        Self::new_snowy_monaro_baseline()
    }
}

impl GeologicVectorLayer {
    /// Baseline regional geological formations for the Snowy Monaro / Talbingo region (GSNSW).
    pub fn new_snowy_monaro_baseline() -> Self {
        let units = vec![
            GeologicUnitRecord {
                unit_code: "Jmb".to_string(), // Monaro Basalt
                lithology: "Basalt".to_string(),
                stratigraphic_age_ma: 45.0,
                lut_index: 128,
            },
            GeologicUnitRecord {
                unit_code: "Gda".to_string(), // Adamaby Granite
                lithology: "Granite".to_string(),
                stratigraphic_age_ma: 410.0,
                lut_index: 129,
            },
            GeologicUnitRecord {
                unit_code: "Skt".to_string(), // Silurian Siltstone / Shale
                lithology: "Siltstone".to_string(),
                stratigraphic_age_ma: 425.0,
                lut_index: 130,
            },
            GeologicUnitRecord {
                unit_code: "Qa".to_string(), // Quaternary Alluvium / River Gravel
                lithology: "Gravel".to_string(),
                stratigraphic_age_ma: 0.1,
                lut_index: 131,
            },
        ];
        Self { units }
    }

    /// Resolves a geological unit code into its assigned ConstituentLUT index (0..255).
    pub fn resolve_unit_to_lut_index(&self, unit_code: &str) -> Option<u8> {
        self.units
            .iter()
            .find(|u| u.unit_code.eq_ignore_ascii_case(unit_code))
            .map(|u| u.lut_index)
    }

    /// Compiles regional Bank 1 recipes (96 entries) from known geological units.
    pub fn compile_bank_1_recipes(&self) -> [Q42ConstituentLutEntry; BANK_1_PAGED_COUNT] {
        let mut recipes = [Q42ConstituentLutEntry::default(); BANK_1_PAGED_COUNT];

        for unit in &self.units {
            let offset = (unit.lut_index as usize).saturating_sub(128);
            if offset < BANK_1_PAGED_COUNT {
                match unit.lithology.as_str() {
                    "Basalt" => {
                        recipes[offset].bulk_density_kg_m3 = 2850;
                        recipes[offset].friction_angle_deg = 44;
                        recipes[offset].cohesion_kpa = 1100;
                        recipes[offset].refractive_index_x100 = 164;
                    }
                    "Granite" => {
                        recipes[offset].bulk_density_kg_m3 = 2680;
                        recipes[offset].friction_angle_deg = 50;
                        recipes[offset].cohesion_kpa = 4500;
                        recipes[offset].refractive_index_x100 = 155;
                    }
                    "Siltstone" => {
                        recipes[offset].bulk_density_kg_m3 = 2400;
                        recipes[offset].friction_angle_deg = 30;
                        recipes[offset].cohesion_kpa = 180;
                        recipes[offset].refractive_index_x100 = 158;
                    }
                    _ => {}
                }
            }
        }

        for recipe in recipes.iter_mut() {
            recipe.update_crc();
        }

        recipes
    }
}
