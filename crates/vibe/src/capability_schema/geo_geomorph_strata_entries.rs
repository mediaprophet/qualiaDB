//! Geomorphology, hydrology, and stratigraphic column capability schemas (GEO-S3 / R3-V).
//!
//! Provides machine schema definitions for all 17 geomorphological simulation,
//! hydrological routing, 6-layer strata packing (.m64 Mode 0x02), and Sentinel-2
//! spectral LUT host capability invocations.

use super::{EffectClass, HonestyLabel, SchemaArg, SchemaEntry};

/// Effect classification for VibeScript 0.2 capability operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Effect {
    Pure,
    Compute,
    Cold,
    Async,
    External,
}

impl Effect {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Pure => "pure", Self::Compute => "compute", Self::Cold => "cold",
            Self::Async => "async", Self::External => "external",
        }
    }
}

/// Parameter type for terrain geomorphology and strata operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamType {
    F64, I64, U64, Bool, Str, Record, List, FieldRef, GeometryRef,
}

impl ParamType {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::F64 => "f64", Self::I64 => "i64", Self::U64 => "u64",
            Self::Bool => "bool", Self::Str => "string", Self::Record => "Record",
            Self::List => "List", Self::FieldRef => "FieldRef", Self::GeometryRef => "GeometryRef",
        }
    }
}

/// Parameter schema descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParamSchema {
    pub name: &'static str,
    pub param_type: ParamType,
    pub required: bool,
    pub description: &'static str,
}

const fn p(name: &'static str, ty: ParamType, req: bool, desc: &'static str) -> ParamSchema {
    ParamSchema { name, param_type: ty, required: req, description: desc }
}

/// Operation schema descriptor for geomorphology and strata operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationSchema {
    pub invoke_id: &'static str,
    pub effect: Effect,
    pub params: &'static [ParamSchema],
    pub returns: &'static str,
    pub description: &'static str,
}

const fn op(id: &'static str, eff: Effect, ret: &'static str, desc: &'static str, params: &'static [ParamSchema]) -> OperationSchema {
    OperationSchema { invoke_id: id, effect: eff, params, returns: ret, description: desc }
}

/// Canonical 6-layer stratigraphic column layer names in descending depth order (bottom-to-top).
pub const STRATA_6_LAYER_NAMES: [&str; 6] = [
    "bedrock", "saprolite", "regolith", "sand", "silt", "snow_ice",
];

/// Validator and container helper for `StrataColumnComplex` (.m64 Mode 0x02).
#[derive(Debug, Clone, PartialEq)]
pub struct StrataColumnComplexDecl {
    pub name: String,
    pub m64_submagic: u8,
    pub sentinel2_province: u16,
    pub bedrock: f64,
    pub saprolite: f64,
    pub regolith: f64,
    pub sand: f64,
    pub silt: f64,
    pub snow_ice: f64,
}

impl StrataColumnComplexDecl {
    pub const EXPECTED_SUBMAGIC: u8 = 0x02;

    pub fn new(
        name: impl Into<String>, m64_submagic: u8, sentinel2_province: u16,
        bedrock: f64, saprolite: f64, regolith: f64, sand: f64, silt: f64, snow_ice: f64,
    ) -> Self {
        Self {
            name: name.into(), m64_submagic, sentinel2_province,
            bedrock, saprolite, regolith, sand, silt, snow_ice,
        }
    }

    pub fn from_layers(
        name: impl Into<String>, m64_submagic: u8, sentinel2_province: u16, layers: [f64; 6],
    ) -> Self {
        Self::new(
            name, m64_submagic, sentinel2_province,
            layers[0], layers[1], layers[2], layers[3], layers[4], layers[5],
        )
    }

    pub fn layer_thicknesses(&self) -> [f64; 6] {
        [self.bedrock, self.saprolite, self.regolith, self.sand, self.silt, self.snow_ice]
    }

    pub fn total_thickness(&self) -> f64 {
        self.bedrock + self.saprolite + self.regolith + self.sand + self.silt + self.snow_ice
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.m64_submagic != Self::EXPECTED_SUBMAGIC {
            return Err("m64_submagic must equal 0x02 for StrataColumnComplex");
        }
        for &t in &self.layer_thicknesses() {
            if t < 0.0 || t.is_nan() || t.is_infinite() {
                return Err("all 6 layer thicknesses must be finite non-negative values");
            }
        }
        if self.bedrock <= 0.0 {
            return Err("bedrock base layer thickness must be strictly positive");
        }
        if self.sentinel2_province >= 323 {
            return Err("sentinel2_province must be in range 0..322 for ESA Sentinel-2 LUT");
        }
        Ok(())
    }

    pub fn is_expected_layer(index: usize, name: &str) -> bool {
        STRATA_6_LAYER_NAMES.get(index).map_or(false, |&expected| expected == name)
    }
}

/// Canonical sequence of all 17 Geomorphology and Strata capability invoke IDs.
pub const GEO_GEOMORPH_STRATA_INVOKE_IDS: &[&str] = &[
    "Terrain.erode_fixed824", "Terrain.erode_coupled_6species", "Terrain.erode_droplet",
    "Terrain.carve_meander_channel", "Terrain.hydro_priority_flood", "Terrain.hydro_flow_dinf",
    "Terrain.hydro_fastscape_spl", "Terrain.coastal_wave_cliff", "Terrain.thermal_talus_creep",
    "Terrain.landslide_debris_runout", "Terrain.glacier_sia_carve", "Terrain.snow_wind_avalanche",
    "Terrain.aeolian_sand_saltation", "Terrain.strata_pack_m64", "Terrain.strata_constituents8",
    "Terrain.sentinel2_biome_albedo", "Terrain.geomorph_qa_audit",
];

/// Canonical sequence of all 17 Geomorphology and Strata operations.
pub const GEO_GEOMORPH_STRATA_OPERATIONS: &[OperationSchema] = &[
    op("Terrain.erode_fixed824", Effect::Cold, "FieldRef", "Bit-exact 8.24 fixed-point hydraulic pipe step.", &[
        p("input", ParamType::FieldRef, true, "Input elevation field"), p("dt_secs", ParamType::F64, true, "Delta time in seconds"), p("iterations", ParamType::U64, true, "Iteration count"), p("rain_rate", ParamType::F64, true, "Rainfall in mm/hr"), p("pipe_len_m", ParamType::F64, true, "Pipe length in meters"),
    ]),
    op("Terrain.erode_coupled_6species", Effect::Cold, "FieldRef", "Coupled 6-species bedrock/saprolite/regolith/sand/silt/snow erosion.", &[
        p("input", ParamType::FieldRef, true, "Input elevation field"), p("strata", ParamType::Record, true, "6-layer strata thickness record"), p("dt_years", ParamType::F64, true, "Geological time step in years"), p("shear_stress", ParamType::F64, true, "Critical shear threshold"), p("carrying_capacity", ParamType::F64, true, "Sediment capacity factor"),
    ]),
    op("Terrain.erode_droplet", Effect::Cold, "FieldRef", "Lagrangian particle droplet erosion.", &[
        p("input", ParamType::FieldRef, true, "Input elevation field"), p("droplet_count", ParamType::U64, true, "Droplet count"), p("inertia", ParamType::F64, true, "Momentum inertia factor"), p("capacity", ParamType::F64, true, "Sediment capacity coefficient"), p("evaporation", ParamType::F64, true, "Evaporation rate"),
    ]),
    op("Terrain.carve_meander_channel", Effect::Cold, "FieldRef", "Asymmetric cutbank/point-bar river meander carve.", &[
        p("input", ParamType::FieldRef, true, "Input elevation field"), p("curvature_weight", ParamType::F64, true, "Meander curvature weight"), p("cutbank_steepness", ParamType::F64, true, "Outer cutbank steepness"), p("point_bar_deposit", ParamType::F64, true, "Inner bar deposition factor"),
    ]),
    op("Terrain.hydro_priority_flood", Effect::Cold, "FieldRef", "Barnes 2014 pit filling & breach carving.", &[
        p("input", ParamType::FieldRef, true, "Input elevation field"), p("min_slope_deg", ParamType::F64, true, "Minimum gradient slope in degrees"), p("carve_breaches", ParamType::Bool, true, "Carve depression breaches"),
    ]),
    op("Terrain.hydro_flow_dinf", Effect::Cold, "FieldRef", "Tarboton 1997 D-Infinity & D8 flow accumulation.", &[
        p("input", ParamType::FieldRef, true, "Conditioned elevation field"), p("use_dinf", ParamType::Bool, true, "D-Infinity continuous routing"), p("weight_field", ParamType::FieldRef, false, "Runoff weight field"),
    ]),
    op("Terrain.hydro_fastscape_spl", Effect::Cold, "FieldRef", "Implicit O(N) FastScape Stream Power Law bedrock incision.", &[
        p("input", ParamType::FieldRef, true, "Input elevation field"), p("k_coeff", ParamType::F64, true, "Erodibility K"), p("m_exp", ParamType::F64, true, "Area exponent m"), p("n_exp", ParamType::F64, true, "Slope exponent n"), p("uplift_rate", ParamType::F64, true, "Uplift rate m/yr"), p("dt_years", ParamType::F64, true, "Time step years"),
    ]),
    op("Terrain.coastal_wave_cliff", Effect::Cold, "FieldRef", "Coastal wave-cut notch, terrace & cliff collapse.", &[
        p("input", ParamType::FieldRef, true, "Coastal elevation field"), p("sea_level_m", ParamType::F64, true, "Sea level in meters"), p("wave_energy", ParamType::F64, true, "Wave energy coefficient"), p("cliff_collapse_angle_deg", ParamType::F64, true, "Collapse angle degrees"), p("notch_depth_m", ParamType::F64, true, "Notch depth meters"),
    ]),
    op("Terrain.thermal_talus_creep", Effect::Cold, "FieldRef", "8-neighbor anisotropic talus creep & freeze-thaw weathering.", &[
        p("input", ParamType::FieldRef, true, "Input elevation field"), p("repose_angle_deg", ParamType::F64, true, "Repose angle in degrees"), p("talus_rate", ParamType::F64, true, "Diffusion creep rate"), p("freeze_thaw_cycles", ParamType::U64, true, "Freeze-thaw cycles"),
    ]),
    op("Terrain.landslide_debris_runout", Effect::Cold, "Record", "Mohr-Coulomb infinite-slope failure & debris runout.", &[
        p("input", ParamType::FieldRef, true, "Input elevation field"), p("friction_angle_deg", ParamType::F64, true, "Internal friction angle phi"), p("cohesion_kpa", ParamType::F64, true, "Cohesion c in kPa"), p("pore_pressure_ratio", ParamType::F64, true, "Ru pore pressure factor"),
    ]),
    op("Terrain.glacier_sia_carve", Effect::Cold, "FieldRef", "Shallow Ice Approximation alpine glacier flow & basal quarrying.", &[
        p("input", ParamType::FieldRef, true, "Topography field"), p("ela_alt_m", ParamType::F64, true, "ELA altitude in meters"), p("sliding_coeff", ParamType::F64, true, "Basal sliding coefficient"), p("quarrying_rate", ParamType::F64, true, "Plucking abrasion rate"), p("dt_years", ParamType::F64, true, "Epoch years"),
    ]),
    op("Terrain.snow_wind_avalanche", Effect::Cold, "FieldRef", "Wind snow drift transport & critical slope avalanche.", &[
        p("input", ParamType::FieldRef, true, "Input elevation field"), p("wind_direction_deg", ParamType::F64, true, "Wind azimuth degrees"), p("wind_speed_m_s", ParamType::F64, true, "Wind speed m/s"), p("critical_slope_deg", ParamType::F64, true, "Avalanche slope degrees"), p("deposition_rate", ParamType::F64, true, "Deposition fraction"),
    ]),
    op("Terrain.aeolian_sand_saltation", Effect::Cold, "FieldRef", "Werner saltation & barchan dune slipface avalanche.", &[
        p("input", ParamType::FieldRef, true, "Terrain field"), p("wind_bearing_deg", ParamType::F64, true, "Wind direction degrees"), p("shear_velocity", ParamType::F64, true, "Friction velocity u*"), p("saltation_length_m", ParamType::F64, true, "Hop length in meters"), p("slipface_angle_deg", ParamType::F64, true, "Slipface angle degrees"),
    ]),
    op("Terrain.strata_pack_m64", Effect::Compute, "Record", "Pack 6-layer stratigraphic column into 64B .m64 0x02 cells with CRC-32C.", &[
        p("elevation", ParamType::FieldRef, true, "Elevation field"), p("strata_layers", ParamType::Record, true, "6-layer thicknesses"), p("cell_resolution_m", ParamType::F64, true, "Grid spacing in meters"), p("sentinel2_province", ParamType::U64, true, "Sentinel-2 province ID [0..322]"),
    ]),
    op("Terrain.strata_constituents8", Effect::Pure, "Record", "Extract 8 normalized surface constituent splat weights.", &[
        p("strata_column", ParamType::Record, true, ".m64 strata container record"), p("surface_depth_m", ParamType::F64, true, "Sampling depth in meters"),
    ]),
    op("Terrain.sentinel2_biome_albedo", Effect::Pure, "Record", "Evaluate 323-province ESA Sentinel-2 L2A biome spectral LUT.", &[
        p("strata_column", ParamType::Record, true, ".m64 strata container record"), p("province_id", ParamType::U64, true, "ESA Sentinel-2 biome province ID [0..322]"),
    ]),
    op("Terrain.geomorph_qa_audit", Effect::Compute, "Record", "Compute Hypsometric Integral, Strahler Rb, Hack's h, concavity theta, and mass conservation gate.", &[
        p("initial_terrain", ParamType::FieldRef, true, "Pre-erosion topography field"), p("eroded_terrain", ParamType::FieldRef, true, "Post-erosion topography field"), p("tolerance_mass_kg", ParamType::F64, true, "Mass discrepancy tolerance"),
    ]),
];

/// Looks up a geomorphology/strata operation schema by its canonical invoke ID.
pub fn lookup_geo_geomorph_strata_operation(invoke_id: &str) -> Option<&'static OperationSchema> {
    GEO_GEOMORPH_STRATA_OPERATIONS.iter().find(|op| op.invoke_id == invoke_id)
}

/// Returns the 17 `Terrain.*` geomorphology and strata capability schema entries for the catalog.
pub fn entries() -> Vec<SchemaEntry> {
    GEO_GEOMORPH_STRATA_OPERATIONS
        .iter()
        .map(|op| {
            let effect_class = match op.effect {
                Effect::Pure => EffectClass::Pure,
                Effect::Compute | Effect::Cold => EffectClass::Read,
                Effect::Async | Effect::External => EffectClass::External,
            };
            let mut entry = SchemaEntry::new(
                op.invoke_id,
                effect_class,
                HonestyLabel::Always,
                op.returns,
            )
            .with_description(op.description);
            for p in op.params {
                entry = entry.with_arg(
                    SchemaArg::new(p.name, p.param_type.as_str(), p.required)
                        .with_description(p.description),
                );
            }
            entry
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geomorph_strata_operations_count_and_uniqueness() {
        assert_eq!(GEO_GEOMORPH_STRATA_OPERATIONS.len(), 17);
        assert_eq!(GEO_GEOMORPH_STRATA_INVOKE_IDS.len(), 17);
        for (i, op) in GEO_GEOMORPH_STRATA_OPERATIONS.iter().enumerate() {
            assert_eq!(op.invoke_id, GEO_GEOMORPH_STRATA_INVOKE_IDS[i]);
            assert!(op.invoke_id.starts_with("Terrain."));
            assert!(!op.returns.is_empty());
            assert!(!op.description.is_empty());
        }
        let mut ids: Vec<&str> = GEO_GEOMORPH_STRATA_INVOKE_IDS.to_vec();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 17, "All 17 Geomorph Strata invoke IDs must be unique");
    }

    #[test]
    fn test_entries_conversion_and_lookup() {
        assert!(lookup_geo_geomorph_strata_operation("Terrain.erode_fixed824").is_some());
        assert!(lookup_geo_geomorph_strata_operation("Terrain.strata_pack_m64").is_some());
        assert!(lookup_geo_geomorph_strata_operation("Terrain.geomorph_qa_audit").is_some());
        assert!(lookup_geo_geomorph_strata_operation("Terrain.unknown_op").is_none());

        let converted = entries();
        assert_eq!(converted.len(), 17);
        for (i, entry) in converted.iter().enumerate() {
            assert_eq!(entry.id, GEO_GEOMORPH_STRATA_INVOKE_IDS[i]);
            assert_eq!(entry.honesty, HonestyLabel::Always);
        }
    }

    #[test]
    fn test_strata_column_complex_decl_validation() {
        let valid = StrataColumnComplexDecl::new(
            "kosciuszko_glacial_strata",
            0x02,
            0,
            850.0,
            18.0,
            6.0,
            3.5,
            1.5,
            2.0,
        );
        assert!(valid.validate().is_ok());
        assert_eq!(valid.total_thickness(), 881.0);
        assert_eq!(valid.layer_thicknesses().len(), 6);

        for (idx, name) in STRATA_6_LAYER_NAMES.iter().enumerate() {
            assert!(StrataColumnComplexDecl::is_expected_layer(idx, name));
        }

        let bad_magic = StrataColumnComplexDecl::new("test", 0x01, 0, 800.0, 10.0, 5.0, 3.0, 1.0, 1.0);
        assert!(bad_magic.validate().is_err());

        let neg_thickness = StrataColumnComplexDecl::new("test", 0x02, 0, 800.0, -10.0, 5.0, 3.0, 1.0, 1.0);
        assert!(neg_thickness.validate().is_err());

        let zero_bedrock = StrataColumnComplexDecl::new("test", 0x02, 0, 0.0, 10.0, 5.0, 3.0, 1.0, 1.0);
        assert!(zero_bedrock.validate().is_err());

        let bad_province = StrataColumnComplexDecl::new("test", 0x02, 323, 800.0, 10.0, 5.0, 3.0, 1.0, 1.0);
        assert!(bad_province.validate().is_err());
    }

    #[test]
    fn test_strata_column_fixture_exists() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path = std::path::Path::new(manifest_dir)
            .join("fixtures")
            .join("geography")
            .join("strata_column_complex.vibe");
        assert!(
            fixture_path.exists(),
            "Fixture strata_column_complex.vibe must exist at {:?}",
            fixture_path
        );
    }
}
