//! Planetary Viewport and Multi-Scale Terrain Rendering Schemas (GEO-S10 / R10-V).
//!
//! Provides machine schema definitions for 10 `WebizenGeo.*` host capability invocations
//! covering capped polyline and earcut polygon GPU tessellation, continuous distance-dependent
//! LOD (CDLOD) quadtree node selection, crack-free vertical terrain skirts, MVT vector tile
//! zoom styling, curved centerline label placement, screen-space R-tree label collision resolution,
//! planetary sky atmosphere Rayleigh/Mie limb scattering, Gerstner wave displacement with foam,
//! and hypsometric elevation contour shading with screen-coordinate feature picking.

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

/// Execution intent class for Webizen planetary viewport operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionClass { PureCpu, InteractiveViewport, AsyncHost }

/// Determinism guarantee levels for planetary viewport operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeterminismGuarantee { BitExactParity, PlatformAgnostic, DeterministicDigest }

/// Semantic unit classification for viewport measurements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticUnit { Dimensionless, Meters, Radians, Degrees, Seconds, Bytes, Pixels }

/// Value type classification for schema parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueType { F32, F64, U32, U64, I32, I64, Bool, Str, Record, List, FieldRef, GeometryRef }

/// Style profile for viewport rendering passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleProfile { CdlodTerrain, MvtVector, SkyAtmosphere, WaterSurface, HypsometricContour }

/// Style support level for viewport rendering targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleSupportLevel { Core, Optional, Emulated }

#[allow(non_snake_case)]
pub mod complex_Validation { pub use super::ComplexDecl; }
pub mod host { pub use super::EffectTag; }
pub mod intent { pub use super::{DeterminismGuarantee, ExecutionClass}; }
pub mod semantic_types { pub use super::{SemanticUnit, ValueType}; }
pub mod styles { pub use super::{StyleProfile, StyleSupportLevel}; }

/// Parameter type for Webizen planetary viewport operations.
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

/// Operation schema specification for Webizen Geo viewport capability operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeoViewportSpec {
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
) -> GeoViewportSpec {
    GeoViewportSpec { invoke_id, call_path, effect, params, returns, description }
}

/// Declarative specification for a Planetary Viewport Complex.
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
pub type PlanetaryViewportComplexDecl = ComplexDecl;

/// Constructs the declarative `PlanetaryViewportComplex` specification.
pub fn planetary_viewport_complex_decl() -> ComplexDecl {
    ComplexDecl::new(
        "PlanetaryViewportComplex",
        "urn:qualia:topology:planetary-viewport",
        vec![
            "GeodesyAndTerrainReady".into(),
            "CdlodMeshAndSkirtsBuilt".into(),
            "VectorTilesAndLabelsPlaced".into(),
            "AtmosphereAndWaterShaded".into(),
            "InteractiveFramePresented".into(),
        ],
        vec![
            "subdivide_cdlod_and_skirts".into(),
            "place_mvt_curved_labels".into(),
            "shade_atmosphere_water_contours".into(),
            "present_and_pick_feature".into(),
        ],
        vec![
            "SeamlessCdlodSkirts".into(),
            "ZeroLabelCollisions".into(),
            "AtmosphereLimbConservation".into(),
            "DeterministicPickParity".into(),
        ],
        "GeodesyAndTerrainReady",
        "InteractiveFramePresented",
    )
}

/// Canonical sequence of all 10 Webizen Geo viewport invoke IDs.
pub const GEO_VIEWPORT_INVOKE_IDS: &[&str] = &[
    "WebizenGeo.tessellate_polyline_capped",
    "WebizenGeo.tessellate_polygon_earcut",
    "WebizenGeo.select_cdlod_quadtree",
    "WebizenGeo.build_terrain_skirts",
    "WebizenGeo.eval_mvt_style_zoom",
    "WebizenGeo.layout_curved_centerline_labels",
    "WebizenGeo.solve_label_collisions",
    "WebizenGeo.eval_sky_atmosphere_limb",
    "WebizenGeo.eval_water_wave_foam",
    "WebizenGeo.shade_hypsometric_and_pick",
];

/// Canonical sequence of all 10 Webizen Geo viewport raw specs.
pub const GEO_VIEWPORT_RAW: &[GeoViewportSpec] = &[
    // 1. Tessellate capped 2D/3D polylines
    spec("WebizenGeo.tessellate_polyline_capped", "webizen_geo.tessellate_polyline_capped", Effect::Compute, "Record",
        "Tessellates 2D/3D polylines into vertex and index buffers with miter, bevel, or round joints and butt, square, or round end caps.", &[
            p("polyline_vertices", ParamType::List, true, "Sequence of polyline vertex coordinates [[x, y, z], ...]"),
            p("width_m", ParamType::F64, true, "Stroke width in world meters or screen pixels"),
            p("joint_style", ParamType::Str, false, "Join style: miter, bevel, or round"),
            p("cap_style", ParamType::Str, false, "End cap style: butt, square, or round"),
            p("miter_limit", ParamType::F64, false, "Maximum miter ratio before falling back to bevel join"),
        ]),
    // 2. Tessellate polygons with earcut
    spec("WebizenGeo.tessellate_polygon_earcut", "webizen_geo.tessellate_polygon_earcut", Effect::Compute, "Record",
        "Tessellates planar or geodetic 2D polygons with optional interior hole rings into triangle indices using robust earcut.", &[
            p("exterior_ring", ParamType::List, true, "Exterior polygon boundary ring coordinates [[x, y], ...]"),
            p("interior_rings", ParamType::List, false, "List of interior hole boundary rings [[[x, y], ...], ...]"),
            p("winding_rule", ParamType::Str, false, "Winding orientation check: clockwise or counter_clockwise"),
        ]),
    // 3. Select CDLOD terrain quadtree nodes
    spec("WebizenGeo.select_cdlod_quadtree", "webizen_geo.select_cdlod_quadtree", Effect::Compute, "Record",
        "Selects continuous distance-dependent LOD (CDLOD) terrain quadtree nodes based on camera frustum and morph ranges.", &[
            p("camera_position", ParamType::List, true, "Camera position in world/geodetic coordinates [x, y, z]"),
            p("frustum_planes", ParamType::List, true, "Camera view frustum bounding planes [[nx, ny, nz, d], ...]"),
            p("terrain_bounds", ParamType::Record, true, "Root terrain bounding box {min_x, min_y, max_x, max_y, min_z, max_z}"),
            p("max_lod_level", ParamType::U64, false, "Maximum quadtree subdivision depth level"),
            p("morph_factor", ParamType::F64, false, "CDLOD morph range transition overlap factor"),
        ]),
    // 4. Build vertical terrain skirts
    spec("WebizenGeo.build_terrain_skirts", "webizen_geo.build_terrain_skirts", Effect::Compute, "Record",
        "Constructs vertical perimeter skirt mesh triangles along tile borders to seal cracks between adjacent CDLOD LOD levels.", &[
            p("tile_vertices", ParamType::List, true, "Border vertices of terrain tile grid"),
            p("skirt_depth_m", ParamType::F64, true, "Downward extrusion depth of perimeter skirt in meters"),
            p("boundary_edges", ParamType::Record, false, "Bitmask or flags for north, south, east, and west perimeter seams"),
        ]),
    // 5. Evaluate MVT styling at zoom
    spec("WebizenGeo.eval_mvt_style_zoom", "webizen_geo.eval_mvt_style_zoom", Effect::Pure, "Record",
        "Evaluates Mapbox/MapLibre vector tile (MVT) styling expressions, layer filters, and zoom-interpolated symbology rules.", &[
            p("feature_properties", ParamType::Record, true, "MVT feature attributes and metadata properties"),
            p("zoom_level", ParamType::F64, true, "Active viewport continuous zoom level"),
            p("style_rules", ParamType::Record, true, "Layer stylesheet filter expressions and visual styling rules"),
        ]),
    // 6. Layout curved centerline labels
    spec("WebizenGeo.layout_curved_centerline_labels", "webizen_geo.layout_curved_centerline_labels", Effect::Compute, "Record",
        "Calculates smooth curved baseline text glyph positions, orientations, and curvature limits along road/river centerlines.", &[
            p("centerline_points", ParamType::List, true, "Path polyline points [[x, y], ...] defining label centerline"),
            p("label_text", ParamType::Str, true, "Text string to format and lay out along centerline"),
            p("glyph_font_metrics", ParamType::Record, true, "Font advance, ascender, and glyph bounding box metrics"),
            p("max_angle_delta_rad", ParamType::F64, false, "Maximum permitted angular change per glyph before label rejection"),
        ]),
    // 7. Solve label collisions
    spec("WebizenGeo.solve_label_collisions", "webizen_geo.solve_label_collisions", Effect::Compute, "Record",
        "Resolves screen-space 2D label and icon bounding box collisions using priority ranking and spatial R-tree packing.", &[
            p("candidate_labels", ParamType::List, true, "List of candidate label bounding boxes [{id, bbox, priority, anchor}, ...]"),
            p("viewport_extent", ParamType::Record, true, "Screen-space viewport bounds {width_px, height_px}"),
            p("decay_rate", ParamType::F64, false, "Temporal opacity fade transition rate for newly placed or occluded labels"),
        ]),
    // 8. Evaluate sky atmosphere limb
    spec("WebizenGeo.eval_sky_atmosphere_limb", "webizen_geo.eval_sky_atmosphere_limb", Effect::Pure, "Record",
        "Evaluates planetary atmospheric limb radiance, Rayleigh and Mie scattering, ozone absorption, and solar disk illumination.", &[
            p("camera_altitude_m", ParamType::F64, true, "Observer altitude above planetary ellipsoid in meters"),
            p("sun_direction", ParamType::List, true, "Normalized solar directional vector [x, y, z]"),
            p("view_vector", ParamType::List, true, "Normalized view direction vector [x, y, z]"),
            p("atmosphere_params", ParamType::Record, false, "Scale heights, Rayleigh/Mie scattering coefficients, and ozone absorption"),
        ]),
    // 9. Evaluate Gerstner water wave and foam
    spec("WebizenGeo.eval_water_wave_foam", "webizen_geo.eval_water_wave_foam", Effect::Compute, "Record",
        "Evaluates multi-octave Gerstner ocean/lake wave displacement, surface normal vectors, and shoreline foam turbulence.", &[
            p("world_coordinates", ParamType::List, true, "Sample locations on water plane [[x, y], ...]"),
            p("time_seconds", ParamType::F64, true, "Elapsed simulation time in seconds"),
            p("wind_vector", ParamType::List, true, "Wind direction and speed vector [vx, vy]"),
            p("wave_octaves", ParamType::List, false, "Gerstner wave parameter harmonics [{amplitude, wavelength, speed, steepness}, ...]"),
        ]),
    // 10. Shade hypsometric contours and pick
    spec("WebizenGeo.shade_hypsometric_and_pick", "webizen_geo.shade_hypsometric_and_pick", Effect::Compute, "Record",
        "Performs hypsometric elevation tinting with contour line generation and screen-coordinate GPU feature picking query.", &[
            p("elevation_field", ParamType::FieldRef, true, "Terrain elevation field reference or raster surface"),
            p("color_ramp", ParamType::List, true, "Hypsometric elevation gradient color stops [{elevation_m, color_rgba}, ...]"),
            p("contour_interval_m", ParamType::F64, false, "Major/minor contour line elevation step in meters"),
            p("pick_screen_coord", ParamType::Record, false, "Screen cursor coordinates {x, y} for interactive feature identification"),
        ]),
];

/// Returns the 10 capability schema entries for the catalog.
pub fn geo_viewport_entries() -> Vec<CapabilitySchemaEntry> {
    GEO_VIEWPORT_RAW.iter().map(|s| {
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
    geo_viewport_entries()
}

/// Looks up a capability schema entry by either invoke ID or lowercase call path.
pub fn lookup_geo_viewport_entry(call_path: &str) -> Option<CapabilitySchemaEntry> {
    let s = GEO_VIEWPORT_RAW.iter().find(|s| s.call_path == call_path || s.invoke_id == call_path)?;
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
pub fn lookup_geo_viewport_spec(call_path: &str) -> Option<&'static GeoViewportSpec> {
    GEO_VIEWPORT_RAW.iter().find(|s| s.call_path == call_path || s.invoke_id == call_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geo_viewport_count_and_uniqueness() {
        assert_eq!(GEO_VIEWPORT_RAW.len(), 10);
        assert_eq!(GEO_VIEWPORT_INVOKE_IDS.len(), 10);
        for (i, spec) in GEO_VIEWPORT_RAW.iter().enumerate() {
            assert_eq!(spec.invoke_id, GEO_VIEWPORT_INVOKE_IDS[i]);
            assert!(spec.invoke_id.starts_with("WebizenGeo."));
            assert!(spec.call_path.starts_with("webizen_geo."));
            assert!(!spec.returns.is_empty());
            assert!(!spec.description.is_empty());
        }
        let mut ids: Vec<&str> = GEO_VIEWPORT_INVOKE_IDS.to_vec();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 10, "All 10 invoke IDs must be unique");

        let mut paths: Vec<&str> = GEO_VIEWPORT_RAW.iter().map(|s| s.call_path).collect();
        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), 10, "All 10 call paths must be unique");
    }

    #[test]
    fn test_entries_conversion_and_lookup() {
        for id in GEO_VIEWPORT_INVOKE_IDS {
            assert!(lookup_geo_viewport_entry(id).is_some());
        }
        for spec in GEO_VIEWPORT_RAW {
            assert!(lookup_geo_viewport_entry(spec.call_path).is_some());
        }
        assert!(lookup_geo_viewport_entry("WebizenGeo.unknown_call").is_none());

        let converted = geo_viewport_entries();
        assert_eq!(converted.len(), 10);
        for (i, entry) in converted.iter().enumerate() {
            assert_eq!(entry.id, GEO_VIEWPORT_INVOKE_IDS[i]);
            assert_eq!(entry.honesty, HonestyLabel::Always);
        }

        let alias_entries = entries();
        assert_eq!(alias_entries.len(), 10);

        let spec_ref = lookup_geo_viewport_spec("webizen_geo.select_cdlod_quadtree");
        assert!(spec_ref.is_some());
        assert_eq!(spec_ref.unwrap().invoke_id, "WebizenGeo.select_cdlod_quadtree");
    }

    #[test]
    fn test_planetary_viewport_complex_decl_validation() {
        let decl = planetary_viewport_complex_decl();
        assert!(decl.validate().is_ok());
        assert_eq!(decl.name, "PlanetaryViewportComplex");
        assert_eq!(decl.complex_id, "urn:qualia:topology:planetary-viewport");
        assert_eq!(decl.states.len(), 5);
        assert_eq!(decl.transitions.len(), 4);
        assert_eq!(decl.invariants.len(), 4);
        assert_eq!(decl.initial_state, "GeodesyAndTerrainReady");
        assert_eq!(decl.target_state, "InteractiveFramePresented");

        let bad_initial = ComplexDecl::new("C", "id", vec!["A".into()], vec!["t".into()], vec![], "NonExistent", "A");
        assert!(bad_initial.validate().is_err());

        let bad_target = ComplexDecl::new("C", "id", vec!["A".into()], vec!["t".into()], vec![], "A", "NonExistent");
        assert!(bad_target.validate().is_err());

        let empty_name = ComplexDecl::new("", "id", vec!["A".into()], vec!["t".into()], vec![], "A", "A");
        assert!(empty_name.validate().is_err());
    }

    #[test]
    fn test_kosciuszko_e2e_authoring_fixture_exists() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path = std::path::Path::new(manifest_dir)
            .join("fixtures")
            .join("geography")
            .join("kosciuszko_e2e_authoring.vibe");
        assert!(
            fixture_path.exists(),
            "Fixture kosciuszko_e2e_authoring.vibe must exist at {:?}",
            fixture_path
        );
    }
}
