//! Domain metadata schemas for `.hmc` holistic entity digital twins.
//!
//! Provides CBOR-serializable representations for flora, fauna, and mechanical
//! assembly species metadata, enabling biological growth/phyllotaxis,
//! musculoskeletal biomechanics, and kinematic/fatigue lifespans in `.hmc` bundles.

use serde::{Deserialize, Serialize};

use super::format::BundleError;

pub const SPECIES_META_ENTRY_KEY: &str = "manifest/species_meta.cbor";
pub const SPECIES_META_KIND: &str = "species_meta";

fn default_phyllotaxis() -> f32 {
    137.50776
}

fn default_murray() -> f32 {
    2.49
}

fn default_wbe_radius() -> f32 {
    0.33333334
}

fn default_wbe_length() -> f32 {
    0.6666667
}

fn default_bark_scale() -> f32 {
    1.0
}

fn default_chlorophyll() -> [f32; 2] {
    [430.0, 660.0]
}

/// Botanical growth and morphology parameters for flora entity twins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FloraSpeciesMeta {
    /// Canonical taxonomic identifier (e.g. "urn:taxon:quercus_robur").
    pub taxon_id: String,
    /// Golden phyllotaxis divergence angle in degrees (default 137.50776°).
    #[serde(default = "default_phyllotaxis")]
    pub phyllotaxis_divergence_deg: f32,
    /// Murray's law vascular branching exponent (default 2.49).
    #[serde(default = "default_murray")]
    pub murray_exponent: f32,
    /// West-Brown-Enquist (WBE) metabolic radius scaling exponent (default 1/3).
    #[serde(default = "default_wbe_radius")]
    pub west_brown_enquist_radius_exp: f32,
    /// WBE length scaling exponent (default 2/3).
    #[serde(default = "default_wbe_length")]
    pub wbe_length_exp: f32,
    /// Bark Voronoi surface irregularity scale.
    #[serde(default = "default_bark_scale")]
    pub bark_voronoi_scale: f32,
    /// Chlorophyll a/b peak absorption wavelength range in nanometers [a, b].
    #[serde(default = "default_chlorophyll")]
    pub chlorophyll_band_nm: [f32; 2],
    /// Chronological list of seasonal phenology stages (e.g. Dormant, BudBreak, FullCanopy).
    #[serde(default)]
    pub seasonal_phenology_stages: Vec<String>,
}

impl Default for FloraSpeciesMeta {
    fn default() -> Self {
        Self {
            taxon_id: String::new(),
            phyllotaxis_divergence_deg: default_phyllotaxis(),
            murray_exponent: default_murray(),
            west_brown_enquist_radius_exp: default_wbe_radius(),
            wbe_length_exp: default_wbe_length(),
            bark_voronoi_scale: default_bark_scale(),
            chlorophyll_band_nm: default_chlorophyll(),
            seasonal_phenology_stages: Vec::new(),
        }
    }
}

impl FloraSpeciesMeta {
    pub fn new(taxon_id: impl Into<String>) -> Self {
        Self {
            taxon_id: taxon_id.into(),
            ..Self::default()
        }
    }
}

fn default_pga_motor() -> bool {
    true
}

fn default_isometric_force() -> f32 {
    1000.0
}

fn default_optimal_fiber() -> f32 {
    0.1
}

fn default_contraction_velocity() -> f32 {
    10.0
}

fn default_dimorphism() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

/// Musculoskeletal and anatomical parameters for fauna entity twins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FaunaSpeciesMeta {
    /// Canonical taxonomic identifier (e.g. "urn:taxon:canis_lupus").
    pub taxon_id: String,
    /// Number of skeletal articulation joints in the base rig.
    #[serde(default)]
    pub skeleton_joint_count: u32,
    /// Whether Projective Geometric Algebra (PGA) motor skinning is enabled.
    #[serde(default = "default_pga_motor")]
    pub pga_motor_skinning: bool,
    /// Number of simulated Hill-type muscle-tendon units.
    #[serde(default)]
    pub hill_muscle_count: u32,
    /// Maximum isometric force in Newtons (F0).
    #[serde(default = "default_isometric_force")]
    pub max_isometric_force_n: f32,
    /// Optimal muscle fiber resting length in meters (l0_m).
    #[serde(default = "default_optimal_fiber")]
    pub optimal_fiber_length_m: f32,
    /// Maximum contraction velocity in optimal fiber lengths per second (v_max / l0_m).
    #[serde(default = "default_contraction_velocity")]
    pub max_contraction_velocity_l0_s: f32,
    /// Sexual dimorphism volumetric scale multipliers [x, y, z].
    #[serde(default = "default_dimorphism")]
    pub sexual_dimorphism_scale: [f32; 3],
    /// Bundle entry key for the species vocalization audio asset (.q4au), if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vocalization_q4au_key: Option<String>,
}

impl Default for FaunaSpeciesMeta {
    fn default() -> Self {
        Self {
            taxon_id: String::new(),
            skeleton_joint_count: 0,
            pga_motor_skinning: default_pga_motor(),
            hill_muscle_count: 0,
            max_isometric_force_n: default_isometric_force(),
            optimal_fiber_length_m: default_optimal_fiber(),
            max_contraction_velocity_l0_s: default_contraction_velocity(),
            sexual_dimorphism_scale: default_dimorphism(),
            vocalization_q4au_key: None,
        }
    }
}

impl FaunaSpeciesMeta {
    pub fn new(taxon_id: impl Into<String>) -> Self {
        Self {
            taxon_id: taxon_id.into(),
            ..Self::default()
        }
    }
}

fn default_basquin_coeff() -> f32 {
    500.0
}

fn default_basquin_exp() -> f32 {
    -0.1
}

fn default_paris_c() -> f32 {
    1e-11
}

fn default_paris_m() -> f32 {
    3.0
}

fn default_temp_range() -> [f32; 2] {
    [233.15, 353.15]
}

/// Kinematic and structural fatigue parameters for mechanical assemblies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MechanicalAssemblyMeta {
    /// Unique assembly or part catalog identifier (e.g. "urn:part:turbopump_mk4").
    pub assembly_id: String,
    /// Number of kinematic pairs (revolute, prismatic, spherical joints).
    #[serde(default)]
    pub kinematic_pair_count: u32,
    /// Basquin high-cycle fatigue strength coefficient in MPa (sigma_f').
    #[serde(default = "default_basquin_coeff")]
    pub basquin_fatigue_strength_coeff_mpa: f32,
    /// Basquin fatigue strength exponent (b).
    #[serde(default = "default_basquin_exp")]
    pub basquin_fatigue_exponent_b: f32,
    /// Paris-Erdogan crack propagation coefficient (C).
    #[serde(default = "default_paris_c")]
    pub paris_erdogan_c: f32,
    /// Paris-Erdogan crack propagation exponent (m).
    #[serde(default = "default_paris_m")]
    pub paris_erdogan_m: f32,
    /// Operational temperature range in Kelvin [min_k, max_k].
    #[serde(default = "default_temp_range")]
    pub operating_temp_kelvin_range: [f32; 2],
    /// Bundle entry key for the precise CAD B-Rep STEP model, if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cad_brep_step_key: Option<String>,
}

impl Default for MechanicalAssemblyMeta {
    fn default() -> Self {
        Self {
            assembly_id: String::new(),
            kinematic_pair_count: 0,
            basquin_fatigue_strength_coeff_mpa: default_basquin_coeff(),
            basquin_fatigue_exponent_b: default_basquin_exp(),
            paris_erdogan_c: default_paris_c(),
            paris_erdogan_m: default_paris_m(),
            operating_temp_kelvin_range: default_temp_range(),
            cad_brep_step_key: None,
        }
    }
}

impl MechanicalAssemblyMeta {
    pub fn new(assembly_id: impl Into<String>) -> Self {
        Self {
            assembly_id: assembly_id.into(),
            ..Self::default()
        }
    }
}

/// Polymorphic container for domain-specific species and assembly metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HolisticSpeciesMeta {
    Flora(FloraSpeciesMeta),
    Fauna(FaunaSpeciesMeta),
    Mechanical(MechanicalAssemblyMeta),
}

impl HolisticSpeciesMeta {
    /// Serialise the metadata record to CBOR bytes.
    pub fn encode_to_cbor(&self) -> Result<Vec<u8>, BundleError> {
        let mut buf = Vec::new();
        ciborium::into_writer(self, &mut buf).map_err(|e| BundleError::Cbor(e.to_string()))?;
        Ok(buf)
    }

    /// Deserialise a metadata record from CBOR bytes.
    pub fn decode_from_cbor(bytes: &[u8]) -> Result<Self, BundleError> {
        ciborium::from_reader(bytes).map_err(|e| BundleError::Cbor(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flora_meta_roundtrip() {
        let mut flora = FloraSpeciesMeta::new("urn:taxon:sequoia_sempervirens");
        flora.seasonal_phenology_stages = vec!["SpringFlush".into(), "Dormant".into()];
        let meta = HolisticSpeciesMeta::Flora(flora.clone());

        let bytes = meta.encode_to_cbor().expect("cbor encode");
        let decoded = HolisticSpeciesMeta::decode_from_cbor(&bytes).expect("cbor decode");
        match decoded {
            HolisticSpeciesMeta::Flora(f) => {
                assert_eq!(f.taxon_id, "urn:taxon:sequoia_sempervirens");
                assert_eq!(f.phyllotaxis_divergence_deg, 137.50776);
                assert_eq!(f.murray_exponent, 2.49);
                assert_eq!(f.seasonal_phenology_stages.len(), 2);
            }
            _ => panic!("expected Flora variant"),
        }
    }

    #[test]
    fn fauna_meta_roundtrip() {
        let mut fauna = FaunaSpeciesMeta::new("urn:taxon:panthera_leo");
        fauna.skeleton_joint_count = 64;
        fauna.hill_muscle_count = 120;
        fauna.vocalization_q4au_key = Some("audio/lion_roar.q4au".into());
        let meta = HolisticSpeciesMeta::Fauna(fauna);

        let bytes = meta.encode_to_cbor().expect("cbor encode");
        let decoded = HolisticSpeciesMeta::decode_from_cbor(&bytes).expect("cbor decode");
        match decoded {
            HolisticSpeciesMeta::Fauna(f) => {
                assert_eq!(f.taxon_id, "urn:taxon:panthera_leo");
                assert_eq!(f.skeleton_joint_count, 64);
                assert_eq!(f.hill_muscle_count, 120);
                assert!(f.pga_motor_skinning);
                assert_eq!(f.vocalization_q4au_key.as_deref(), Some("audio/lion_roar.q4au"));
            }
            _ => panic!("expected Fauna variant"),
        }
    }

    #[test]
    fn mechanical_meta_roundtrip() {
        let mut mech = MechanicalAssemblyMeta::new("urn:part:centrifugal_pump_100");
        mech.kinematic_pair_count = 4;
        mech.cad_brep_step_key = Some("cad/pump.step".into());
        let meta = HolisticSpeciesMeta::Mechanical(mech);

        let bytes = meta.encode_to_cbor().expect("cbor encode");
        let decoded = HolisticSpeciesMeta::decode_from_cbor(&bytes).expect("cbor decode");
        match decoded {
            HolisticSpeciesMeta::Mechanical(m) => {
                assert_eq!(m.assembly_id, "urn:part:centrifugal_pump_100");
                assert_eq!(m.kinematic_pair_count, 4);
                assert_eq!(m.basquin_fatigue_strength_coeff_mpa, 500.0);
                assert_eq!(m.cad_brep_step_key.as_deref(), Some("cad/pump.step"));
            }
            _ => panic!("expected Mechanical variant"),
        }
    }
}
