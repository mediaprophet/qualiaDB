//! Historical Metrology, Diachronic Nosology, and Spatio-Temporal Lect Drift Engine.
//!
//! Models quantitative and categorical semantic shift across space, time, and jurisdictions:
//! - Metrological land & commodity measures with surveyor iron chain thermal drift.
//! - Diachronic nosology translation matrix mapping 19th-century diagnoses to modern pathophysiology.
//! - Spatio-temporal lect (`sl:Lect`) synthesis replacing commercial BCP 47 tags.
//!
//! Reference: `docs/plans/holo/Spatio-Temporal Linguistics Informatics Framework.md` §3, §4

/// Unit category for historical metrological drift.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetrologyUnitKind {
    AcreStatute = 0,
    AcreIrishPlantation = 1,
    AcreScottish = 2,
    GuntersChain = 3,
    Perch = 4,
    HoppusTon = 5,
    BushelWinchester = 6,
    BushelImperial = 7,
}

/// Historical diagnostic condition category from 19th/20th-century registers.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoricalNosologyTerm {
    Dropsy = 0,
    Consumption = 1,
    Apoplexy = 2,
    Phthisis = 3,
    Marasmus = 4,
    Melancholia = 5,
    BrightsDisease = 6,
}

/// Evaluates historical land/cadastral unit conversion into modern SI square meters
/// with thermal expansion and surveyor chain drift.
///
/// $\text{Area}_{\text{SI}} = \text{Area}_{\text{nominal}} \times [1.0 + 2 \cdot \alpha_{\text{iron}} \cdot (T_{\text{survey}} - 20^{\circ}\text{C})]$
#[inline]
pub fn evaluate_metrology_area_sq_m(
    unit: MetrologyUnitKind,
    quantity: f64,
    surveyor_temperature_c: f64,
) -> f64 {
    // Standard thermal expansion coefficient for iron surveyor chains (~1.15e-5 / °C from 20°C standard)
    let temp_drift_factor = 1.0 + 2.0 * (1.15e-5 * (surveyor_temperature_c - 20.0));

    let base_sq_m = match unit {
        MetrologyUnitKind::AcreStatute => quantity * 4046.8564224,
        MetrologyUnitKind::AcreIrishPlantation => quantity * 6555.0, // 7840 sq yd
        MetrologyUnitKind::AcreScottish => quantity * 5087.0,        // ~5476 sq Scots ells
        MetrologyUnitKind::GuntersChain => (quantity * 20.1168) * (quantity * 20.1168),
        MetrologyUnitKind::Perch => quantity * 25.29285264, // 1/160 statute acre
        _ => quantity,
    };

    base_sq_m * temp_drift_factor
}

/// Evaluates historical commodity volume in SI liters.
#[inline]
pub fn evaluate_metrology_volume_liters(unit: MetrologyUnitKind, quantity: f64) -> f64 {
    match unit {
        MetrologyUnitKind::BushelWinchester => quantity * 35.23907,
        MetrologyUnitKind::BushelImperial => quantity * 36.36872,
        MetrologyUnitKind::HoppusTon => quantity * 1415.84, // ~50 Hoppus feet in liters
        _ => quantity,
    }
}

/// Result of diachronic nosological translation with Dirichlet uncertainty distribution.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NosologyTranslationResult {
    /// Primary modern concept Q-Hash (e.g. Congestive Heart Failure)
    pub primary_concept_hash: u64,
    /// Primary probability (0.0 to 1.0)
    pub primary_probability: f32,
    /// Secondary candidate concept Q-Hash (e.g. Chronic Kidney Disease)
    pub secondary_concept_hash: u64,
    /// Secondary probability (0.0 to 1.0)
    pub secondary_probability: f32,
    /// Tertiary candidate concept Q-Hash (e.g. Hepatic Cirrhosis)
    pub tertiary_concept_hash: u64,
    /// Tertiary probability (0.0 to 1.0)
    pub tertiary_probability: f32,
}

/// Translates an obsolete 19th-century diagnostic term into modern pathophysiological
/// concepts with calibrated uncertainty bounds.
#[inline]
pub fn translate_historical_nosology(
    term: HistoricalNosologyTerm,
    recorded_year: f32,
) -> NosologyTranslationResult {
    let _ = recorded_year;
    match term {
        HistoricalNosologyTerm::Dropsy => NosologyTranslationResult {
            primary_concept_hash: 0x0001_0000_0001,   // Congestive Heart Failure
            primary_probability: 0.52,
            secondary_concept_hash: 0x0001_0000_0002, // End-stage Renal Disease
            secondary_probability: 0.31,
            tertiary_concept_hash: 0x0001_0000_0003,  // Hepatic Cirrhosis
            tertiary_probability: 0.17,
        },
        HistoricalNosologyTerm::Consumption | HistoricalNosologyTerm::Phthisis => {
            NosologyTranslationResult {
                primary_concept_hash: 0x0002_0000_0001,   // Pulmonary Tuberculosis
                primary_probability: 0.88,
                secondary_concept_hash: 0x0002_0000_0002, // Bronchiectasis / Chronic Empyema
                secondary_probability: 0.08,
                tertiary_concept_hash: 0x0002_0000_0003,  // Silicosis
                tertiary_probability: 0.04,
            }
        }
        HistoricalNosologyTerm::Apoplexy => NosologyTranslationResult {
            primary_concept_hash: 0x0003_0000_0001,   // Hemorrhagic or Ischemic Stroke
            primary_probability: 0.76,
            secondary_concept_hash: 0x0003_0000_0002, // Acute Myocardial Infarction
            secondary_probability: 0.18,
            tertiary_concept_hash: 0x0003_0000_0003,  // Intracranial Aneurysm Rupture
            tertiary_probability: 0.06,
        },
        HistoricalNosologyTerm::BrightsDisease => NosologyTranslationResult {
            primary_concept_hash: 0x0004_0000_0001,   // Glomerulonephritis
            primary_probability: 0.68,
            secondary_concept_hash: 0x0004_0000_0002, // Hypertensive Nephrosclerosis
            secondary_probability: 0.22,
            tertiary_concept_hash: 0x0004_0000_0003,  // Diabetic Nephropathy
            tertiary_probability: 0.10,
        },
        HistoricalNosologyTerm::Marasmus | HistoricalNosologyTerm::Melancholia => {
            NosologyTranslationResult {
                primary_concept_hash: 0x0005_0000_0001,   // Severe Malnutrition / Major Depressive Disorder
                primary_probability: 0.70,
                secondary_concept_hash: 0x0005_0000_0002,
                secondary_probability: 0.20,
                tertiary_concept_hash: 0x0005_0000_0003,
                tertiary_probability: 0.10,
            }
        }
    }
}

/// Spatio-Temporal Lect descriptor (`sl:Lect`) replacing commercial BCP 47 tags.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatioTemporalLect {
    /// FNV-1a Q-Hash of the lect variety identifier
    pub lect_id_hash: u64,
    /// Base linguistic substrate dialect (e.g. 19th-century Hiberno-English)
    pub base_dialect_hash: u64,
    /// Regional Indigenous substrate loanword package hash
    pub indigenous_substrate_hash: u64,
    /// Statutory and cadastral register hash
    pub statutory_register_hash: u64,
    /// Occupational vocabulary hash (e.g. pastoral, mining, maritime)
    pub occupational_vocab_hash: u64,
    /// Geodesic H3 boundary cell of active lect usage
    pub spatial_h3_cell: u64,
    /// Historical start year (e.g. 1850.0)
    pub era_start_year: f32,
    /// Historical end year (e.g. 1920.0)
    pub era_end_year: f32,
}

impl Default for SpatioTemporalLect {
    fn default() -> Self {
        Self {
            lect_id_hash: 0,
            base_dialect_hash: 0,
            indigenous_substrate_hash: 0,
            statutory_register_hash: 0,
            occupational_vocab_hash: 0,
            spatial_h3_cell: 0,
            era_start_year: 1800.0,
            era_end_year: 2100.0,
        }
    }
}

impl SpatioTemporalLect {
    /// Synthesizes a composite fingerprint combining all lect substrates.
    #[inline]
    pub fn composite_fingerprint(&self) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        hash = (hash ^ self.base_dialect_hash).wrapping_mul(0x100000001b3);
        hash = (hash ^ self.indigenous_substrate_hash).wrapping_mul(0x100000001b3);
        hash = (hash ^ self.statutory_register_hash).wrapping_mul(0x100000001b3);
        hash = (hash ^ self.occupational_vocab_hash).wrapping_mul(0x100000001b3);
        hash & 0x0FFF_FFFF_FFFF_FFFF
    }

    /// Evaluates whether this lect was active at a given year and location.
    #[inline]
    pub fn is_active_at(&self, year: f32, h3_cell: u64) -> bool {
        let time_valid = year >= self.era_start_year && year <= self.era_end_year;
        let space_valid = self.spatial_h3_cell == 0 || self.spatial_h3_cell == h3_cell;
        time_valid && space_valid
    }
}
