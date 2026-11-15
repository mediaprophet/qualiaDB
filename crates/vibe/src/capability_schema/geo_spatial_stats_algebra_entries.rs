//! Spatial statistics, geomorphometry, map algebra, and network capability schemas (GEO-S7 / R7-V).
//!
//! Provides machine schema definitions for all 16 `GisGp.*` host capability invocations
//! covering geostatistical kriging and spatial autocorrelation (empirical variogram, ordinary
//! kriging, IDW / natural neighbor, Moran's I / Geary's C, LISA / Getis-Ord Gi*, GWR / Ripley's K),
//! viewshed and terrain geomorphometry (radial/cumulative viewshed, TPI/TRI/VRM/geomorphons),
//! map algebra, zonal statistics, marching squares contours, and Saaty AHP overlay,
//! and hydrological cost distance, least cost paths, and network topology analysis (A*, isochrones,
//! TSP, MST, max-flow, and centrality).

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

/// Parameter type for spatial statistics, map algebra, and network operations.
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

/// Operation schema specification for spatial statistics and map algebra capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeoSpatialStatsSpec {
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
) -> GeoSpatialStatsSpec {
    GeoSpatialStatsSpec { invoke_id, call_path, effect, params, returns, description }
}

/// Declarative specification for a Map Algebra Suitability and Routing Complex.
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
pub type MapAlgebraSuitabilityComplexDecl = ComplexDecl;

/// Constructs the declarative `MapAlgebraSuitabilityComplex` specification.
pub fn map_algebra_suitability_complex_decl() -> ComplexDecl {
    ComplexDecl::new(
        "MapAlgebraSuitabilityComplex",
        "urn:qualia:topology:map-algebra-suitability",
        vec!["RawBands".into(), "VariogramKriged".into(), "HydroIndicesEvaluated".into(), "AhpWeightedOverlay".into(), "LeastCostRouted".into()],
        vec!["krige_covariates".into(), "compute_twi_rusle".into(), "compile_cell_algebra".into(), "route_least_cost_network".into()],
        vec!["ContinuousFieldPreservation".into(), "ZeroNegativeCost".into(), "AcyclicOptimalPath".into(), "DeterministicCellCompilation".into()],
        "RawBands",
        "LeastCostRouted",
    )
}

/// Canonical sequence of all 16 Spatial Statistics, Map Algebra, and Network invoke IDs.
pub const GEO_SPATIAL_STATS_ALGEBRA_INVOKE_IDS: &[&str] = &[
    "GisGp.empirical_variogram", "GisGp.ordinary_kriging", "GisGp.idw_natural_neighbor",
    "GisGp.morans_i_gearys_c", "GisGp.local_lisa_getis_ord_gi_star", "GisGp.gwr_ripleys_k",
    "GisGp.viewshed_radial_cumulative", "GisGp.tpi_tri_vrm_geomorphons", "GisGp.map_algebra_cell_compile",
    "GisGp.zonal_statistics_reclassify", "GisGp.marching_squares_contours", "GisGp.ahp_multi_criteria_overlay",
    "GisGp.twi_spi_sti_rusle", "GisGp.watershed_cost_distance_lcp", "GisGp.network_astar_isochrone_tsp",
    "GisGp.network_mst_maxflow_centrality",
];

/// Canonical sequence of all 16 Spatial Statistics, Map Algebra, and Network raw specs.
pub const GEO_SPATIAL_STATS_ALGEBRA_RAW: &[GeoSpatialStatsSpec] = &[
    // 1. Kriging & Spatial Autocorrelation
    spec("GisGp.empirical_variogram", "gis_gp.empirical_variogram", Effect::Cold, "Record",
        "Computes experimental empirical variogram from irregular point samples.", &[
            p("points", ParamType::List, true, "Sample point observations with values"),
            p("lag_distance", ParamType::F64, true, "Lag bin distance in meters"),
            p("max_lags", ParamType::U64, true, "Maximum number of lag bins"),
        ]),
    spec("GisGp.ordinary_kriging", "gis_gp.ordinary_kriging", Effect::Compute, "Record",
        "Performs Best Linear Unbiased Prediction (BLUP) ordinary kriging interpolation with variance surface.", &[
            p("points", ParamType::List, true, "Sample observations with values"),
            p("variogram_model", ParamType::Str, true, "Fitted model: Spherical, Exponential, or Gaussian"),
            p("nugget", ParamType::F64, true, "Nugget variance c0"),
            p("sill", ParamType::F64, true, "Partial sill c"),
            p("range", ParamType::F64, true, "Spatial correlation range a in meters"),
            p("target_grid", ParamType::Record, true, "Prediction raster grid specification (bbox, cell_size)"),
        ]),
    spec("GisGp.idw_natural_neighbor", "gis_gp.idw_natural_neighbor", Effect::Compute, "FieldRef",
        "Interpolates scattered points into a continuous raster using IDW or Natural Neighbor (Sibson).", &[
            p("points", ParamType::List, true, "Scatter observation points with values"),
            p("method", ParamType::Str, true, "Interpolation method: idw or natural_neighbor"),
            p("power", ParamType::F64, true, "IDW distance weighting exponent (default 2.0)"),
            p("radius_or_neighbors", ParamType::F64, true, "Search radius or maximum neighbor count"),
            p("target_grid", ParamType::Record, true, "Output raster grid extent and cell size"),
        ]),
    spec("GisGp.morans_i_gearys_c", "gis_gp.morans_i_gearys_c", Effect::Cold, "Record",
        "Evaluates global spatial autocorrelation indices (Moran's I and Geary's C) with permutation significance.", &[
            p("features", ParamType::List, true, "Spatial features with target attribute values"),
            p("weights_matrix", ParamType::Str, true, "Spatial weights: Queen, Rook, kNN, or InverseDistance"),
            p("permutations", ParamType::U64, true, "Monte Carlo pseudo p-value permutation count"),
        ]),
    spec("GisGp.local_lisa_getis_ord_gi_star", "gis_gp.local_lisa_getis_ord_gi_star", Effect::Cold, "Record",
        "Computes Local Indicators of Spatial Association (Anselin LISA) and Getis-Ord Gi* hotspot statistics.", &[
            p("features", ParamType::List, true, "Feature points or polygons with attribute values"),
            p("distance_band", ParamType::F64, true, "Critical distance threshold or bandwidth in meters"),
            p("metric", ParamType::Str, true, "Metric to evaluate: lisa or getis_ord_gi_star"),
        ]),
    spec("GisGp.gwr_ripleys_k", "gis_gp.gwr_ripleys_k", Effect::Cold, "Record",
        "Performs Geographically Weighted Regression (GWR) or multi-scale Ripley's K / Besag's L point pattern analysis.", &[
            p("points", ParamType::List, true, "Point features with dependent and explanatory covariates"),
            p("analysis", ParamType::Str, true, "Analysis mode: gwr or ripleys_k"),
            p("bandwidth_or_radii", ParamType::List, true, "Kernel bandwidth or list of evaluation radii"),
        ]),

    // 2. Viewshed & Terrain Geomorphometry
    spec("GisGp.viewshed_radial_cumulative", "gis_gp.viewshed_radial_cumulative", Effect::Compute, "FieldRef",
        "Computes radial or cumulative line-of-sight viewshed visibility over a digital elevation model.", &[
            p("dem", ParamType::FieldRef, true, "Elevation surface raster field"),
            p("observer_points", ParamType::List, true, "Observer coordinates and viewer heights"),
            p("max_distance_m", ParamType::F64, true, "Maximum line-of-sight sight distance in meters"),
            p("target_offset_m", ParamType::F64, false, "Target above-ground height in meters"),
            p("cumulative", ParamType::Bool, false, "Cumulative observer count visibility if true"),
        ]),
    spec("GisGp.tpi_tri_vrm_geomorphons", "gis_gp.tpi_tri_vrm_geomorphons", Effect::Pure, "Record",
        "Derives Topographic Position Index (TPI), Terrain Ruggedness (TRI), Vector Ruggedness (VRM), and Geomorphons.", &[
            p("dem", ParamType::FieldRef, true, "Input digital elevation model surface"),
            p("scale_radius_cells", ParamType::U64, true, "Neighborhood radius in cells for TPI/TRI/VRM"),
            p("flatness_threshold_deg", ParamType::F64, true, "Geomorphon zenith/nadir angle threshold in degrees"),
        ]),

    // 3. Map Algebra, Zonal, Contours & AHP
    spec("GisGp.map_algebra_cell_compile", "gis_gp.map_algebra_cell_compile", Effect::Pure, "Record",
        "Compiles a pure VibeScript cell expression into zero-heap map algebra and WGSL compute shader kernel.", &[
            p("expression_ast", ParamType::Str, true, "Pure VibeScript cell mathematical formula or expression AST"),
            p("raster_inputs", ParamType::Record, true, "Named map of input raster fields"),
            p("target_format", ParamType::Str, true, "Target backend: wgsl or zero_heap_simd"),
        ]),
    spec("GisGp.zonal_statistics_reclassify", "gis_gp.zonal_statistics_reclassify", Effect::Cold, "Record",
        "Calculates zonal statistical aggregates over defined spatial zones and performs raster cell value reclassification.", &[
            p("raster", ParamType::FieldRef, true, "Value raster surface"),
            p("zones", ParamType::GeometryRef, true, "Zone definitions: polygon feature layer or categorical raster"),
            p("reclass_table", ParamType::List, false, "Optional slice reclassification mapping rules"),
        ]),
    spec("GisGp.marching_squares_contours", "gis_gp.marching_squares_contours", Effect::Cold, "GeometryRef",
        "Extracts vector isoline contours from a continuous raster grid using the Marching Squares algorithm.", &[
            p("raster", ParamType::FieldRef, true, "Continuous scalar raster surface"),
            p("contour_interval", ParamType::F64, true, "Elevation or scalar contour step interval"),
            p("base_contour", ParamType::F64, false, "Base datum elevation from which intervals are reckoned"),
            p("smooth_lines", ParamType::Bool, false, "Apply sub-pixel linear interpolation smoothing"),
        ]),
    spec("GisGp.ahp_multi_criteria_overlay", "gis_gp.ahp_multi_criteria_overlay", Effect::Cold, "Record",
        "Executes Saaty Analytic Hierarchy Process (AHP) multi-criteria weighted linear combination suitability overlay.", &[
            p("criteria_layers", ParamType::List, true, "Normalized criterion raster layers in [0.0, 1.0]"),
            p("pairwise_matrix", ParamType::List, true, "Saaty 1-9 pairwise comparison matrix rows"),
            p("suitability_scale", ParamType::Str, false, "Suitability scale: standard_0_100 or unit_interval"),
        ]),

    // 4. Hydrology, Cost-Distance & Network Topology
    spec("GisGp.twi_spi_sti_rusle", "gis_gp.twi_spi_sti_rusle", Effect::Compute, "Record",
        "Derives Topographic Wetness (TWI), Stream Power (SPI), Sediment Transport (STI), and RUSLE soil loss surfaces.", &[
            p("dem", ParamType::FieldRef, true, "Hydrologically conditioned elevation model"),
            p("slope_rad", ParamType::FieldRef, true, "Surface slope in radians"),
            p("flow_accumulation", ParamType::FieldRef, true, "Upslope contributing catchment area"),
            p("rusle_factors", ParamType::Record, true, "RUSLE parameters (rainfall R, soil K, cover C, practice P)"),
        ]),
    spec("GisGp.watershed_cost_distance_lcp", "gis_gp.watershed_cost_distance_lcp", Effect::Cold, "Record",
        "Calculates cumulative cost distance surface, hydrological watersheds, and Least Cost Path (LCP) corridors.", &[
            p("friction_surface", ParamType::FieldRef, true, "Traversal resistance / cost raster surface"),
            p("source_points", ParamType::List, true, "Source origin coordinates"),
            p("destination_points", ParamType::List, true, "Destination target coordinates for LCP"),
            p("flow_direction", ParamType::FieldRef, false, "Optional D8 flow direction grid for watershed delineation"),
        ]),
    spec("GisGp.network_astar_isochrone_tsp", "gis_gp.network_astar_isochrone_tsp", Effect::Cold, "Record",
        "Computes turn-restricted A* shortest path, service area isochrones, and Traveling Salesperson (TSP) route.", &[
            p("network_graph", ParamType::Record, true, "Topological line network with edge weights and turn restrictions"),
            p("origin", ParamType::Record, true, "Origin coordinate pair"),
            p("destinations", ParamType::List, true, "Destination coordinate pairs or waypoints"),
            p("cutoff_cost", ParamType::F64, true, "Maximum impedance cutoff for drive-time / walk-time isochrone polygons"),
        ]),
    spec("GisGp.network_mst_maxflow_centrality", "gis_gp.network_mst_maxflow_centrality", Effect::Cold, "Record",
        "Computes Minimum Spanning Tree (MST), maximum network flow (Edmonds-Karp), and topological centrality indices.", &[
            p("network_graph", ParamType::Record, true, "Edge capacity, length, and directional connectivity graph"),
            p("source_node", ParamType::U64, true, "Source terminal node ID"),
            p("sink_node", ParamType::U64, true, "Sink terminal node ID"),
            p("centrality_metric", ParamType::Str, true, "Centrality metric: betweenness, closeness, or eigenvector"),
        ]),
];

/// Returns the 16 capability schema entries for the catalog.
pub fn geo_spatial_stats_algebra_entries() -> Vec<CapabilitySchemaEntry> {
    GEO_SPATIAL_STATS_ALGEBRA_RAW.iter().map(|s| {
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
    geo_spatial_stats_algebra_entries()
}

/// Looks up a capability schema entry by either invoke ID or lowercase call path.
pub fn lookup_geo_spatial_stats_algebra_entry(call_path: &str) -> Option<CapabilitySchemaEntry> {
    let s = GEO_SPATIAL_STATS_ALGEBRA_RAW.iter().find(|s| s.call_path == call_path || s.invoke_id == call_path)?;
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
pub fn lookup_geo_spatial_stats_algebra_spec(call_path: &str) -> Option<&'static GeoSpatialStatsSpec> {
    GEO_SPATIAL_STATS_ALGEBRA_RAW.iter().find(|s| s.call_path == call_path || s.invoke_id == call_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geo_spatial_stats_algebra_count_and_uniqueness() {
        assert_eq!(GEO_SPATIAL_STATS_ALGEBRA_RAW.len(), 16);
        assert_eq!(GEO_SPATIAL_STATS_ALGEBRA_INVOKE_IDS.len(), 16);
        for (i, spec) in GEO_SPATIAL_STATS_ALGEBRA_RAW.iter().enumerate() {
            assert_eq!(spec.invoke_id, GEO_SPATIAL_STATS_ALGEBRA_INVOKE_IDS[i]);
            assert!(spec.invoke_id.starts_with("GisGp."));
            assert!(spec.call_path.starts_with("gis_gp."));
            assert!(!spec.returns.is_empty());
            assert!(!spec.description.is_empty());
        }
        let mut ids: Vec<&str> = GEO_SPATIAL_STATS_ALGEBRA_INVOKE_IDS.to_vec();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 16, "All 16 invoke IDs must be unique");

        let mut paths: Vec<&str> = GEO_SPATIAL_STATS_ALGEBRA_RAW.iter().map(|s| s.call_path).collect();
        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), 16, "All 16 call paths must be unique");
    }

    #[test]
    fn test_entries_conversion_and_lookup() {
        assert!(lookup_geo_spatial_stats_algebra_entry("GisGp.empirical_variogram").is_some());
        assert!(lookup_geo_spatial_stats_algebra_entry("gis_gp.empirical_variogram").is_some());
        assert!(lookup_geo_spatial_stats_algebra_entry("GisGp.ordinary_kriging").is_some());
        assert!(lookup_geo_spatial_stats_algebra_entry("gis_gp.ordinary_kriging").is_some());
        assert!(lookup_geo_spatial_stats_algebra_entry("GisGp.map_algebra_cell_compile").is_some());
        assert!(lookup_geo_spatial_stats_algebra_entry("gis_gp.map_algebra_cell_compile").is_some());
        assert!(lookup_geo_spatial_stats_algebra_entry("GisGp.twi_spi_sti_rusle").is_some());
        assert!(lookup_geo_spatial_stats_algebra_entry("GisGp.network_astar_isochrone_tsp").is_some());
        assert!(lookup_geo_spatial_stats_algebra_entry("GisGp.unknown_call").is_none());

        let converted = geo_spatial_stats_algebra_entries();
        assert_eq!(converted.len(), 16);
        for (i, entry) in converted.iter().enumerate() {
            assert_eq!(entry.id, GEO_SPATIAL_STATS_ALGEBRA_INVOKE_IDS[i]);
            assert_eq!(entry.honesty, HonestyLabel::Always);
        }

        let alias_entries = entries();
        assert_eq!(alias_entries.len(), 16);
    }

    #[test]
    fn test_map_algebra_suitability_complex_decl_validation() {
        let decl = map_algebra_suitability_complex_decl();
        assert!(decl.validate().is_ok());
        assert_eq!(decl.name, "MapAlgebraSuitabilityComplex");
        assert_eq!(decl.complex_id, "urn:qualia:topology:map-algebra-suitability");
        assert_eq!(decl.states.len(), 5);
        assert_eq!(decl.transitions.len(), 4);
        assert_eq!(decl.invariants.len(), 4);
        assert_eq!(decl.initial_state, "RawBands");
        assert_eq!(decl.target_state, "LeastCostRouted");

        let bad_initial = ComplexDecl::new("C", "id", vec!["A".into()], vec!["t".into()], vec![], "NonExistent", "A");
        assert!(bad_initial.validate().is_err());

        let bad_target = ComplexDecl::new("C", "id", vec!["A".into()], vec!["t".into()], vec![], "A", "NonExistent");
        assert!(bad_target.validate().is_err());

        let empty_name = ComplexDecl::new("", "id", vec!["A".into()], vec!["t".into()], vec![], "A", "A");
        assert!(empty_name.validate().is_err());
    }

    #[test]
    fn test_map_algebra_kriging_network_fixture_exists() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path = std::path::Path::new(manifest_dir)
            .join("fixtures")
            .join("geography")
            .join("map_algebra_kriging_network.vibe");
        assert!(
            fixture_path.exists(),
            "Fixture map_algebra_kriging_network.vibe must exist at {:?}",
            fixture_path
        );
    }
}
