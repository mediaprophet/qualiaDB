//! WGSL Forge terrain GPU compute capability schemas (GEO-S8 / R8-V).
//!
//! Provides machine schema definitions for all 10 `WgslForge.geo_*` host capability
//! invocations covering hydrodynamic erosion (8-pipe Q8.24/f32), 6-layer stratigraphic
//! column erosion, D8/D-infinity hydrological routing, multifractal procedural noise cascades,
//! non-linear morphological landform filters, 323-biome Sentinel-2 spectral LUT splatting,
//! pure VibeScript cell map algebra compilation, analytical viewshed and hillshading,
//! Horn's 3x3 slope/aspect, TWI, and 10 geomorphons, and Naga/SHA-256 shader certification
//! and GPU workgroup dispatch.

use super::{EffectClass, HonestyLabel, SchemaArg, SchemaEntry};

/// CapabilitySchemaEntry alias for uniform catalog representation.
pub type CapabilitySchemaEntry = SchemaEntry;

/// Effect classification for VibeScript 0.2 capability operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Effect {
    Pure, Compute, Cold, Async, External,
}

impl Effect {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Pure => "pure", Self::Compute => "compute", Self::Cold => "cold",
            Self::Async => "async", Self::External => "external",
        }
    }
}

/// Tag alias for effect classification.
pub type EffectTag = Effect;

/// Execution intent class for WGSL forge GPU/CPU execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionClass { PureCpu, GpuComputeWorkgroup, AsyncHost }

/// Determinism guarantee levels for shader generation and execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeterminismGuarantee { BitExactParity, PlatformAgnostic, DeterministicDigest }

/// Semantic unit classification for spatial shader parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticUnit { Dimensionless, Meters, Radians, Degrees, Seconds, Bytes }

/// Shader value type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueType { F32, F64, U32, I32, Bool, Record, FieldRef }

/// Style profile for WGSL generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleProfile { StandardWgsl, SubgroupMatrix, TiledSharedMem }

/// Style support level for hardware targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleSupportLevel { Core, Optional, Emulated }

/// Parameter type for WGSL forge terrain compute operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamType {
    F64, I64, U64, Bool, Str, Record, List, FieldRef, GeometryRef,
}

impl ParamType {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::F64 => "f64", Self::I64 => "i64", Self::U64 => "u64", Self::Bool => "bool",
            Self::Str => "string", Self::Record => "Record", Self::List => "List",
            Self::FieldRef => "FieldRef", Self::GeometryRef => "GeometryRef",
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

/// Operation schema specification for WGSL forge capability operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeoWgslForgeSpec {
    pub invoke_id: &'static str,
    pub call_path: &'static str,
    pub effect: Effect,
    pub params: &'static [ParamSchema],
    pub returns: &'static str,
    pub description: &'static str,
}

const fn spec(
    invoke_id: &'static str, call_path: &'static str, effect: Effect,
    returns: &'static str, description: &'static str, params: &'static [ParamSchema],
) -> GeoWgslForgeSpec {
    GeoWgslForgeSpec { invoke_id, call_path, effect, params, returns, description }
}

/// Declarative specification for a WGSL Geo Kernel Lifecycle Complex.
#[derive(Debug, Clone, PartialEq)]
pub struct ComplexDecl {
    pub name: String,
    pub complex_id: String,
    pub states: Vec<String>,
    pub transitions: Vec<String>,
    pub invariants: Vec<String>,
    pub initial_state: String,
    pub target_state: String,
}

impl ComplexDecl {
    pub fn new(
        name: impl Into<String>, complex_id: impl Into<String>, states: Vec<String>,
        transitions: Vec<String>, invariants: Vec<String>,
        initial_state: impl Into<String>, target_state: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(), complex_id: complex_id.into(), states, transitions,
            invariants, initial_state: initial_state.into(), target_state: target_state.into(),
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.name.is_empty() { return Err("name must not be empty"); }
        if self.complex_id.is_empty() { return Err("complex_id must not be empty"); }
        if self.states.is_empty() { return Err("states must not be empty"); }
        if self.transitions.is_empty() { return Err("transitions must not be empty"); }
        if !self.states.contains(&self.initial_state) { return Err("initial_state not in states"); }
        if !self.states.contains(&self.target_state) { return Err("target_state not in states"); }
        Ok(())
    }
}

/// Convenience alias for `ComplexDecl`.
pub type WgslGeoKernelComplexDecl = ComplexDecl;

/// Constructs the declarative `WgslGeoKernelComplex` specification.
pub fn wgsl_geo_kernel_complex_decl() -> ComplexDecl {
    ComplexDecl::new(
        "WgslGeoKernelComplex",
        "urn:qualia:topology:wgsl-geo-kernel",
        vec![
            "AstCellProgram".into(),
            "WgslSourceEmitted".into(),
            "NagaAndHashCertified".into(),
            "CpuOracleVerified".into(),
            "GpuDispatched".into(),
        ],
        vec![
            "emit_wgsl_kernel".into(),
            "certify_shader_hash".into(),
            "verify_cpu_oracle_parity".into(),
            "dispatch_gpu_workgroups".into(),
        ],
        vec![
            "DeterministicShaderDigest".into(),
            "ZeroHeapWorkgroupDispatch".into(),
            "CpuGpuOracleParity".into(),
            "NagaAstValidationCertified".into(),
        ],
        "AstCellProgram",
        "GpuDispatched",
    )
}

/// Canonical sequence of all 10 WGSL Forge Terrain GPU invoke IDs.
pub const GEO_WGSL_FORGE_INVOKE_IDS: &[&str] = &[
    "WgslForge.geo_emit_824_pipe_erosion",
    "WgslForge.geo_emit_6layer_strata_erosion",
    "WgslForge.geo_emit_d8_dinfinity_flow",
    "WgslForge.geo_emit_multifractal_noise",
    "WgslForge.geo_emit_landform_filters",
    "WgslForge.geo_emit_sentinel2_biome_splat",
    "WgslForge.geo_compile_cell_map_algebra",
    "WgslForge.geo_emit_viewshed_hillshade",
    "WgslForge.geo_emit_horn_twi_geomorphons",
    "WgslForge.geo_certify_and_dispatch",
];

/// Canonical sequence of all 10 WGSL Forge Terrain GPU raw specs.
pub const GEO_WGSL_FORGE_RAW: &[GeoWgslForgeSpec] = &[
    // 1. Hydrodynamic pipe erosion
    spec("WgslForge.geo_emit_824_pipe_erosion", "wgsl_forge.geo_emit_824_pipe_erosion", Effect::Cold, "Record",
        "Emits WGSL compute shader for 8-pipe shallow-water hydrodynamic erosion and sediment transport (Q8.24 fixed-point / f32).", &[
            p("grid_resolution", ParamType::Record, true, "Grid width, height, and cell size dx in meters"),
            p("workgroup_size", ParamType::Record, true, "Workgroup dimensions (x, y)"),
            p("pipe_mode", ParamType::Str, true, "Pipe discretization mode: q8_24_fixed or f32_float"),
        ]),
    // 2. 6-layer stratigraphic column erosion
    spec("WgslForge.geo_emit_6layer_strata_erosion", "wgsl_forge.geo_emit_6layer_strata_erosion", Effect::Cold, "Record",
        "Emits WGSL compute shader for 6-layer stratigraphic regolith column erosion and bedrock differential weathering.", &[
            p("grid_resolution", ParamType::Record, true, "Raster domain dimensions (width, height)"),
            p("workgroup_size", ParamType::Record, true, "Workgroup dispatch size (x, y)"),
            p("layer_count", ParamType::U64, true, "Number of stratigraphic geological layers (default 6)"),
        ]),
    // 3. D8 / D-infinity hydrological flow
    spec("WgslForge.geo_emit_d8_dinfinity_flow", "wgsl_forge.geo_emit_d8_dinfinity_flow", Effect::Cold, "Record",
        "Emits WGSL compute shader for D8 and Tarboton D-infinity flow direction, slope steepest descent, and upslope contributing area.", &[
            p("grid_resolution", ParamType::Record, true, "Elevation grid extent and resolution"),
            p("workgroup_size", ParamType::Record, true, "Workgroup dimensions (x, y)"),
            p("flow_model", ParamType::Str, true, "Hydrological routing algorithm: d8 or d_infinity"),
        ]),
    // 4. Multifractal procedural noise cascades
    spec("WgslForge.geo_emit_multifractal_noise", "wgsl_forge.geo_emit_multifractal_noise", Effect::Cold, "Record",
        "Emits WGSL compute shader for procedural multifractal terrain cascades (RidgedMulti, FBM, Jordan, Simplex3D domain warp).", &[
            p("workgroup_size", ParamType::Record, true, "Compute workgroup tiling (x, y)"),
            p("octaves", ParamType::U64, true, "Number of harmonic fractal noise octaves"),
            p("warp_enabled", ParamType::Bool, true, "Enable non-linear Simplex domain warping"),
        ]),
    // 5. Landform filters
    spec("WgslForge.geo_emit_landform_filters", "wgsl_forge.geo_emit_landform_filters", Effect::Cold, "Record",
        "Emits WGSL compute shader for non-linear morphological landform filters (hydraulic talus deposition, thermal terracing, plateau clamping).", &[
            p("workgroup_size", ParamType::Record, true, "Compute workgroup size (x, y)"),
            p("filter_type", ParamType::Str, true, "Morphological operator: hydraulic_talus, thermal_terrace, or plateau_clamp"),
            p("terrace_steps", ParamType::F64, false, "Number of quantized elevation terrace steps"),
        ]),
    // 6. Sentinel-2 biome splatting
    spec("WgslForge.geo_emit_sentinel2_biome_splat", "wgsl_forge.geo_emit_sentinel2_biome_splat", Effect::Cold, "Record",
        "Emits WGSL compute shader for 323-biome Sentinel-2 spectral surface reflectance splatting and Whittaker biome classification.", &[
            p("workgroup_size", ParamType::Record, true, "Workgroup tiling dimensions (x, y)"),
            p("biome_lut_count", ParamType::U64, true, "Size of spectral reflectance LUT table (default 323)"),
            p("elevation_lapse_rate", ParamType::F64, false, "Temperature lapse rate per 1000m elevation"),
        ]),
    // 7. Cell map algebra compilation
    spec("WgslForge.geo_compile_cell_map_algebra", "wgsl_forge.geo_compile_cell_map_algebra", Effect::Pure, "Record",
        "Compiles a pure VibeScript cell mathematical formula or AST into an optimized, zero-heap WGSL compute kernel.", &[
            p("formula", ParamType::Str, true, "Pure mathematical formula or cell AST expression"),
            p("raster_inputs", ParamType::Record, true, "Named dictionary of input raster bind groups"),
            p("target_format", ParamType::Str, true, "Target output backend: wgsl or naga_ir"),
        ]),
    // 8. Viewshed and hillshade
    spec("WgslForge.geo_emit_viewshed_hillshade", "wgsl_forge.geo_emit_viewshed_hillshade", Effect::Cold, "Record",
        "Emits WGSL compute shader for multi-directional analytical hillshading, slope/aspect, and radial line-of-sight viewshed visibility.", &[
            p("workgroup_size", ParamType::Record, true, "Workgroup tiling layout (x, y)"),
            p("azimuth_deg", ParamType::F64, true, "Sun illumination azimuth angle in degrees"),
            p("altitude_deg", ParamType::F64, true, "Sun illumination solar altitude elevation angle in degrees"),
            p("shading_mode", ParamType::Str, true, "Shading algorithm: hillshade, viewshed, or combined"),
        ]),
    // 9. Horn's slope, TWI, geomorphons
    spec("WgslForge.geo_emit_horn_twi_geomorphons", "wgsl_forge.geo_emit_horn_twi_geomorphons", Effect::Cold, "Record",
        "Emits WGSL compute shader for Horn's 3x3 finite-difference slope/aspect, Topographic Wetness Index (TWI), and 10 Geomorphon landforms.", &[
            p("workgroup_size", ParamType::Record, true, "Workgroup dispatch size (x, y)"),
            p("cell_size_m", ParamType::F64, true, "DEM horizontal spatial resolution in meters"),
            p("flatness_threshold_deg", ParamType::F64, true, "Geomorphon zenith/nadir angle threshold in degrees"),
        ]),
    // 10. Certify and dispatch
    spec("WgslForge.geo_certify_and_dispatch", "wgsl_forge.geo_certify_and_dispatch", Effect::Compute, "Record",
        "Certifies WGSL shader via Naga AST validation and SHA-256 digest, verifies against CPU oracle, and dispatches GPU workgroups.", &[
            p("wgsl_source", ParamType::Str, true, "WGSL shader source text to validate and execute"),
            p("entry_point", ParamType::Str, true, "Compute shader entry point function name"),
            p("workgroup_dispatch", ParamType::Record, true, "Grid dispatch counts (x, y, z)"),
            p("expected_hash", ParamType::U64, true, "Expected FNV-1a / SHA-256 shader source hash digest"),
            p("verify_oracle", ParamType::Bool, false, "Verify execution parity against pure CPU oracle if true"),
        ]),
];

/// Returns the 10 capability schema entries for the catalog.
pub fn geo_wgsl_forge_entries() -> Vec<CapabilitySchemaEntry> {
    GEO_WGSL_FORGE_RAW.iter().map(|s| {
        let effect_class = match s.effect {
            Effect::Pure => EffectClass::Pure,
            Effect::Compute | Effect::Cold => EffectClass::Read,
            Effect::Async | Effect::External => EffectClass::External,
        };
        let mut entry = SchemaEntry::new(s.invoke_id, effect_class, HonestyLabel::Always, s.returns)
            .with_description(s.description);
        for p in s.params {
            entry = entry.with_arg(SchemaArg::new(p.name, p.param_type.as_str(), p.required).with_description(p.description));
        }
        entry
    }).collect()
}

/// Standard entrypoint matching the `entries()` convention across all capability modules.
pub fn entries() -> Vec<SchemaEntry> {
    geo_wgsl_forge_entries()
}

/// Looks up a capability schema entry by either invoke ID or lowercase call path.
pub fn lookup_geo_wgsl_forge_entry(call_path: &str) -> Option<CapabilitySchemaEntry> {
    let s = GEO_WGSL_FORGE_RAW.iter().find(|s| s.call_path == call_path || s.invoke_id == call_path)?;
    let effect_class = match s.effect {
        Effect::Pure => EffectClass::Pure,
        Effect::Compute | Effect::Cold => EffectClass::Read,
        Effect::Async | Effect::External => EffectClass::External,
    };
    let mut entry = SchemaEntry::new(s.invoke_id, effect_class, HonestyLabel::Always, s.returns)
        .with_description(s.description);
    for p in s.params {
        entry = entry.with_arg(SchemaArg::new(p.name, p.param_type.as_str(), p.required).with_description(p.description));
    }
    Some(entry)
}

/// Looks up raw operation specification by invoke ID or call path.
pub fn lookup_geo_wgsl_forge_spec(call_path: &str) -> Option<&'static GeoWgslForgeSpec> {
    GEO_WGSL_FORGE_RAW.iter().find(|s| s.call_path == call_path || s.invoke_id == call_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geo_wgsl_forge_count_and_uniqueness() {
        assert_eq!(GEO_WGSL_FORGE_RAW.len(), 10);
        assert_eq!(GEO_WGSL_FORGE_INVOKE_IDS.len(), 10);
        for (i, spec) in GEO_WGSL_FORGE_RAW.iter().enumerate() {
            assert_eq!(spec.invoke_id, GEO_WGSL_FORGE_INVOKE_IDS[i]);
            assert!(spec.invoke_id.starts_with("WgslForge."));
            assert!(spec.call_path.starts_with("wgsl_forge."));
            assert!(!spec.returns.is_empty());
            assert!(!spec.description.is_empty());
        }
        let mut ids: Vec<&str> = GEO_WGSL_FORGE_INVOKE_IDS.to_vec();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 10, "All 10 invoke IDs must be unique");

        let mut paths: Vec<&str> = GEO_WGSL_FORGE_RAW.iter().map(|s| s.call_path).collect();
        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), 10, "All 10 call paths must be unique");
    }

    #[test]
    fn test_entries_conversion_and_lookup() {
        assert!(lookup_geo_wgsl_forge_entry("WgslForge.geo_emit_824_pipe_erosion").is_some());
        assert!(lookup_geo_wgsl_forge_entry("wgsl_forge.geo_emit_824_pipe_erosion").is_some());
        assert!(lookup_geo_wgsl_forge_entry("WgslForge.geo_emit_6layer_strata_erosion").is_some());
        assert!(lookup_geo_wgsl_forge_entry("wgsl_forge.geo_emit_6layer_strata_erosion").is_some());
        assert!(lookup_geo_wgsl_forge_entry("WgslForge.geo_emit_d8_dinfinity_flow").is_some());
        assert!(lookup_geo_wgsl_forge_entry("WgslForge.geo_emit_multifractal_noise").is_some());
        assert!(lookup_geo_wgsl_forge_entry("WgslForge.geo_emit_landform_filters").is_some());
        assert!(lookup_geo_wgsl_forge_entry("WgslForge.geo_emit_sentinel2_biome_splat").is_some());
        assert!(lookup_geo_wgsl_forge_entry("WgslForge.geo_compile_cell_map_algebra").is_some());
        assert!(lookup_geo_wgsl_forge_entry("WgslForge.geo_emit_viewshed_hillshade").is_some());
        assert!(lookup_geo_wgsl_forge_entry("WgslForge.geo_emit_horn_twi_geomorphons").is_some());
        assert!(lookup_geo_wgsl_forge_entry("WgslForge.geo_certify_and_dispatch").is_some());
        assert!(lookup_geo_wgsl_forge_entry("WgslForge.unknown_call").is_none());

        let converted = geo_wgsl_forge_entries();
        assert_eq!(converted.len(), 10);
        for (i, entry) in converted.iter().enumerate() {
            assert_eq!(entry.id, GEO_WGSL_FORGE_INVOKE_IDS[i]);
            assert_eq!(entry.honesty, HonestyLabel::Always);
        }

        let alias_entries = entries();
        assert_eq!(alias_entries.len(), 10);
    }

    #[test]
    fn test_wgsl_geo_kernel_complex_decl_validation() {
        let decl = wgsl_geo_kernel_complex_decl();
        assert!(decl.validate().is_ok());
        assert_eq!(decl.name, "WgslGeoKernelComplex");
        assert_eq!(decl.complex_id, "urn:qualia:topology:wgsl-geo-kernel");
        assert_eq!(decl.states.len(), 5);
        assert_eq!(decl.transitions.len(), 4);
        assert_eq!(decl.invariants.len(), 4);
        assert_eq!(decl.initial_state, "AstCellProgram");
        assert_eq!(decl.target_state, "GpuDispatched");

        let bad_initial = ComplexDecl::new("C", "id", vec!["A".into()], vec!["t".into()], vec![], "NonExistent", "A");
        assert!(bad_initial.validate().is_err());

        let bad_target = ComplexDecl::new("C", "id", vec!["A".into()], vec!["t".into()], vec![], "A", "NonExistent");
        assert!(bad_target.validate().is_err());

        let empty_name = ComplexDecl::new("", "id", vec!["A".into()], vec!["t".into()], vec![], "A", "A");
        assert!(empty_name.validate().is_err());
    }

    #[test]
    fn test_wgsl_forge_terrain_gpu_fixture_exists() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path = std::path::Path::new(manifest_dir)
            .join("fixtures")
            .join("geography")
            .join("wgsl_forge_terrain_gpu.vibe");
        assert!(
            fixture_path.exists(),
            "Fixture wgsl_forge_terrain_gpu.vibe must exist at {:?}",
            fixture_path
        );
    }
}
