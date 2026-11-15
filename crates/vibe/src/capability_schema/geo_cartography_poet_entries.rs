//! Poet Cartography and Interactive Geospatial Authoring Schemas (GEO-S9 / R9-V).
//!
//! Provides machine schema definitions for 12 `PoetGeo.*` and `Cartography.*` host capability invocations
//! covering interactive Copy-on-Write terrain brush sculpting and non-destructive undo/redo journals,
//! shared-vertex planar parcel topology editing, geodesic ruler/protractor measurements,
//! orbital 3D flythrough camera splines, reactive IntentBus canvas sync, Jenks natural breaks and
//! 3x3 bivariate choropleth classification, proportional dot-density synthesis, Brettel CVD color vision
//! deficiency simulation, WCAG 2.1 AA/AAA contrast auditing, and publication SVG cartography composition.

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

/// Execution intent class for Poet interactive and cartographic operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionClass { PureCpu, InteractiveStudio, AsyncHost }

/// Determinism guarantee levels for cartographic publication outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeterminismGuarantee { BitExactParity, PlatformAgnostic, DeterministicDigest }

/// Semantic unit classification for spatial and cartographic measurements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticUnit { Dimensionless, Meters, Radians, Degrees, Seconds, Bytes, Millimeters }

/// Value type classification for schema parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueType { F32, F64, U32, U64, I32, I64, Bool, Str, Record, List, FieldRef, GeometryRef }

/// Style profile for cartographic publication symbology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleProfile { ThematicChoropleth, BivariateMatrix, DotDensity, VectorCadastral }

/// Style support level for cartographic rendering targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleSupportLevel { Core, Optional, Emulated }

#[allow(non_snake_case)]
pub mod complex_Validation { pub use super::ComplexDecl; }
pub mod host { pub use super::EffectTag; }
pub mod intent { pub use super::{DeterminismGuarantee, ExecutionClass}; }
pub mod semantic_types { pub use super::{SemanticUnit, ValueType}; }
pub mod styles { pub use super::{StyleProfile, StyleSupportLevel}; }

/// Parameter type for Poet interactive and cartography operations.
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

/// Operation schema specification for Poet cartography capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeoCartographyPoetSpec {
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
) -> GeoCartographyPoetSpec {
    GeoCartographyPoetSpec { invoke_id, call_path, effect, params, returns, description }
}

/// Declarative specification for a Cartographic Publication Complex.
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
pub type CartographicPublicationComplexDecl = ComplexDecl;

/// Constructs the declarative `CartographicPublicationComplex` specification.
pub fn cartographic_publication_complex_decl() -> ComplexDecl {
    ComplexDecl::new(
        "CartographicPublicationComplex",
        "urn:qualia:topology:cartographic-publication",
        vec!["StudioAuthoring".into(), "TopologySnapped".into(), "ThematicClassified".into(), "CvdAccessibilityVerified".into(), "PublicationSvgExported".into()],
        vec!["sync_terrain_and_topology".into(), "classify_jenks_bivariate".into(), "verify_cvd_wcag_contrast".into(), "export_publication_layout".into()],
        vec!["CoWStrokeReversibility".into(), "ZeroSliverTopology".into(), "WcagAaContrastAdherence".into(), "DeterministicSvgDigest".into()],
        "StudioAuthoring",
        "PublicationSvgExported",
    )
}

/// Canonical sequence of all 12 Poet Cartography and Interactive Geospatial invoke IDs.
pub const GEO_CARTOGRAPHY_POET_INVOKE_IDS: &[&str] = &[
    "PoetGeo.cow_stroke_apply",
    "PoetGeo.cow_undo_redo",
    "PoetGeo.shared_vertex_move_split",
    "PoetGeo.geodesic_ruler_protractor",
    "PoetGeo.orbital_camera_flythrough",
    "PoetGeo.terrain_graph_intent_sync",
    "Cartography.classify_jenks_thematic",
    "Cartography.bivariate_choropleth_3x3",
    "Cartography.proportional_dot_density",
    "Cartography.simulate_cvd_palette",
    "Cartography.audit_wcag_contrast",
    "Cartography.compose_publication_svg",
];

/// Canonical sequence of all 12 Poet Cartography and Interactive Geospatial raw specs.
pub const GEO_CARTOGRAPHY_POET_RAW: &[GeoCartographyPoetSpec] = &[
    // 1. Interactive CoW brush sculpt stroke
    spec("PoetGeo.cow_stroke_apply", "poet_geo.cow_stroke_apply", Effect::Compute, "Record",
        "Applies interactive Copy-on-Write brush sculpt stroke on raster/vector layers with localized dirty tile tracking.", &[
            p("stroke_id", ParamType::Str, true, "Unique identifier for interactive brush stroke"),
            p("tool_type", ParamType::Str, true, "Brush operator: raise, lower, smooth, terrace, or stamp"),
            p("stroke_path", ParamType::List, true, "List of 3D/geodetic cursor waypoint coordinates [x, y, z]"),
            p("brush_radius_m", ParamType::F64, true, "Brush spatial influence radius in meters"),
            p("intensity", ParamType::F64, false, "Brush application intensity factor (0.0 to 1.0)"),
        ]),
    // 2. Non-destructive undo/redo revision tree
    spec("PoetGeo.cow_undo_redo", "poet_geo.cow_undo_redo", Effect::Compute, "Record",
        "Navigates non-destructive CoW undo/redo stroke journal with immutable tile-hash revision tree.", &[
            p("session_id", ParamType::Str, true, "Active Poet interactive editing session identifier"),
            p("action", ParamType::Str, true, "History traversal operation: undo, redo, or checkout_revision"),
            p("steps", ParamType::U64, false, "Number of historical transaction steps to step backward or forward"),
        ]),
    // 3. Shared vertex move and edge split
    spec("PoetGeo.shared_vertex_move_split", "poet_geo.shared_vertex_move_split", Effect::Compute, "Record",
        "Moves or splits a shared topological vertex across adjacent parcel/polygon boundaries preserving zero-gap topology.", &[
            p("target_vertex", ParamType::Record, true, "Source vertex coordinates and tolerance radius {x, y, crs}"),
            p("destination", ParamType::Record, true, "New vertex coordinates {x, y}"),
            p("split_edge", ParamType::Bool, false, "Split intersected polygon edges if new vertex lies on boundary"),
            p("snap_tolerance_m", ParamType::F64, false, "Snapping tolerance distance in meters"),
        ]),
    // 4. Geodesic ruler and protractor
    spec("PoetGeo.geodesic_ruler_protractor", "poet_geo.geodesic_ruler_protractor", Effect::Pure, "Record",
        "Computes interactive Karney geodesic distance, forward/reverse azimuths, and angular vertex protractor bearings.", &[
            p("waypoints", ParamType::List, true, "Sequence of geodetic points [{lat, lon, height_m}]"),
            p("ellipsoid", ParamType::Str, false, "Reference ellipsoid model (default GRS80/WGS84)"),
            p("include_profile", ParamType::Bool, false, "Compute continuous terrain elevation profile along geodesic"),
        ]),
    // 5. Cinematic orbital flythrough camera
    spec("PoetGeo.orbital_camera_flythrough", "poet_geo.orbital_camera_flythrough", Effect::Pure, "Record",
        "Generates cinematic Catmull-Rom or B-spline orbital 3D camera keyframes with terrain collision avoidance.", &[
            p("keyframe_waypoints", ParamType::List, true, "Camera target path keypoints [{pos, target, fov, roll, duration_s}]"),
            p("spline_mode", ParamType::Str, false, "Interpolation curve type: catmull_rom, b_spline, or centripetal"),
            p("min_ground_clearance_m", ParamType::F64, false, "Minimum camera clearance height above elevation terrain"),
        ]),
    // 6. Reactive IntentBus canvas synchronization
    spec("PoetGeo.terrain_graph_intent_sync", "poet_geo.terrain_graph_intent_sync", Effect::Compute, "Record",
        "Synchronizes live interactive Poet canvas edits with QualiaDB reactive IntentBus and topological graph ledger.", &[
            p("intent_id", ParamType::Str, true, "Intent transaction identifier"),
            p("layer_manifest", ParamType::Record, true, "Manifest describing affected terrain raster and vector layers"),
            p("author_did", ParamType::Str, true, "Decentralized identifier DID of authoring agent or user"),
        ]),
    // 7. Jenks natural breaks thematic classification
    spec("Cartography.classify_jenks_thematic", "cartography.classify_jenks_thematic", Effect::Pure, "Record",
        "Computes Jenks Natural Breaks optimization, quantiles, equal-interval, or standard-deviation thematic class limits.", &[
            p("values", ParamType::List, true, "Numeric attribute sample series to classify"),
            p("class_count", ParamType::U64, true, "Number of thematic output intervals/bins (e.g. 5 to 9)"),
            p("method", ParamType::Str, false, "Classification algorithm: jenks_natural_breaks, quantile, equal_interval, or std_dev"),
        ]),
    // 8. 3x3 Bivariate choropleth color matrix
    spec("Cartography.bivariate_choropleth_3x3", "cartography.bivariate_choropleth_3x3", Effect::Pure, "Record",
        "Generates a 2D 3x3 bivariate choropleth color matrix encoding two intersecting statistical variables simultaneously.", &[
            p("var1_values", ParamType::List, true, "First numeric attribute distribution series (X-axis)"),
            p("var2_values", ParamType::List, true, "Second numeric attribute distribution series (Y-axis)"),
            p("color_scheme", ParamType::Str, true, "Bivariate palette scheme: pink_blue, purple_green, or brown_teal"),
            p("var1_breaks", ParamType::List, false, "Explicit class cutoffs for variable 1"),
            p("var2_breaks", ParamType::List, false, "Explicit class cutoffs for variable 2"),
        ]),
    // 9. Proportional dot density distribution
    spec("Cartography.proportional_dot_density", "cartography.proportional_dot_density", Effect::Compute, "Record",
        "Synthesizes pseudo-random dot-density distributions within polygon boundaries with spatial exclusion masks.", &[
            p("polygon_geometry", ParamType::GeometryRef, true, "Bounding polygon geometry reference or GeoJSON feature"),
            p("attribute_total", ParamType::F64, true, "Total statistical quantity to distribute"),
            p("unit_value_per_dot", ParamType::F64, true, "Value represented by each discrete dot symbol"),
            p("exclusion_mask", ParamType::Record, false, "Waterbody or steep-slope exclusion mask preventing dot placement"),
        ]),
    // 10. Color vision deficiency simulation
    spec("Cartography.simulate_cvd_palette", "cartography.simulate_cvd_palette", Effect::Pure, "Record",
        "Simulates color vision deficiency (deuteranopia, protanopia, tritanopia, achromatopsia) on thematic palettes.", &[
            p("palette_hex", ParamType::List, true, "List of sRGB hex color codes ['#1a9850', '#ffffbf', ...]"),
            p("deficiency_type", ParamType::Str, true, "CVD model: deuteranopia, protanopia, tritanopia, or achromatopsia"),
            p("severity", ParamType::F64, false, "Deficiency severity factor from 0.0 (normal) to 1.0 (complete)"),
        ]),
    // 11. WCAG 2.1 AA/AAA contrast ratio audit
    spec("Cartography.audit_wcag_contrast", "cartography.audit_wcag_contrast", Effect::Pure, "Record",
        "Audits WCAG 2.1 AA/AAA relative luminance contrast ratios for text, symbols, and adjacent map polygon boundaries.", &[
            p("foreground_colors", ParamType::List, true, "List of foreground text/symbol hex colors"),
            p("background_colors", ParamType::List, true, "List of background canvas/polygon hex colors"),
            p("conformance_target", ParamType::Str, false, "WCAG conformance level: aa_normal, aa_large, or aaa"),
        ]),
    // 12. Publication-ready SVG cartographic layout
    spec("Cartography.compose_publication_svg", "cartography.compose_publication_svg", Effect::Pure, "Record",
        "Composes publication-ready cartographic SVG layout with neatline, graticule ticks, dynamic scale bar, north arrow, legend, and metadata.", &[
            p("map_layers", ParamType::List, true, "Ordered list of styled vector/raster layer render directives"),
            p("layout_dimensions", ParamType::Record, true, "Output canvas dimensions {width_mm, height_mm, dpi, margin_mm}"),
            p("scale_bar", ParamType::Record, true, "Scale bar configuration {units, max_width_mm, divisions}"),
            p("north_arrow", ParamType::Record, false, "North arrow graphic style and placement anchor"),
            p("legend_config", ParamType::Record, false, "Legend swatch symbols, titles, and layout orientation"),
        ]),
];

/// Returns the 12 capability schema entries for the catalog.
pub fn geo_cartography_poet_entries() -> Vec<CapabilitySchemaEntry> {
    GEO_CARTOGRAPHY_POET_RAW.iter().map(|s| {
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
    geo_cartography_poet_entries()
}

/// Looks up a capability schema entry by either invoke ID or lowercase call path.
pub fn lookup_geo_cartography_poet_entry(call_path: &str) -> Option<CapabilitySchemaEntry> {
    let s = GEO_CARTOGRAPHY_POET_RAW.iter().find(|s| s.call_path == call_path || s.invoke_id == call_path)?;
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
pub fn lookup_geo_cartography_poet_spec(call_path: &str) -> Option<&'static GeoCartographyPoetSpec> {
    GEO_CARTOGRAPHY_POET_RAW.iter().find(|s| s.call_path == call_path || s.invoke_id == call_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geo_cartography_poet_count_and_uniqueness() {
        assert_eq!(GEO_CARTOGRAPHY_POET_RAW.len(), 12);
        assert_eq!(GEO_CARTOGRAPHY_POET_INVOKE_IDS.len(), 12);
        for (i, spec) in GEO_CARTOGRAPHY_POET_RAW.iter().enumerate() {
            assert_eq!(spec.invoke_id, GEO_CARTOGRAPHY_POET_INVOKE_IDS[i]);
            assert!(spec.invoke_id.starts_with("PoetGeo.") || spec.invoke_id.starts_with("Cartography."));
            assert!(spec.call_path.starts_with("poet_geo.") || spec.call_path.starts_with("cartography."));
            assert!(!spec.returns.is_empty());
            assert!(!spec.description.is_empty());
        }
        let mut ids: Vec<&str> = GEO_CARTOGRAPHY_POET_INVOKE_IDS.to_vec();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 12, "All 12 invoke IDs must be unique");

        let mut paths: Vec<&str> = GEO_CARTOGRAPHY_POET_RAW.iter().map(|s| s.call_path).collect();
        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), 12, "All 12 call paths must be unique");
    }

    #[test]
    fn test_entries_conversion_and_lookup() {
        for id in GEO_CARTOGRAPHY_POET_INVOKE_IDS {
            assert!(lookup_geo_cartography_poet_entry(id).is_some());
        }
        for spec in GEO_CARTOGRAPHY_POET_RAW {
            assert!(lookup_geo_cartography_poet_entry(spec.call_path).is_some());
        }
        assert!(lookup_geo_cartography_poet_entry("Cartography.unknown_call").is_none());

        let converted = geo_cartography_poet_entries();
        assert_eq!(converted.len(), 12);
        for (i, entry) in converted.iter().enumerate() {
            assert_eq!(entry.id, GEO_CARTOGRAPHY_POET_INVOKE_IDS[i]);
            assert_eq!(entry.honesty, HonestyLabel::Always);
        }

        let alias_entries = entries();
        assert_eq!(alias_entries.len(), 12);

        let spec_ref = lookup_geo_cartography_poet_spec("poet_geo.cow_stroke_apply");
        assert!(spec_ref.is_some());
        assert_eq!(spec_ref.unwrap().invoke_id, "PoetGeo.cow_stroke_apply");
    }

    #[test]
    fn test_cartographic_publication_complex_decl_validation() {
        let decl = cartographic_publication_complex_decl();
        assert!(decl.validate().is_ok());
        assert_eq!(decl.name, "CartographicPublicationComplex");
        assert_eq!(decl.complex_id, "urn:qualia:topology:cartographic-publication");
        assert_eq!(decl.states.len(), 5);
        assert_eq!(decl.transitions.len(), 4);
        assert_eq!(decl.invariants.len(), 4);
        assert_eq!(decl.initial_state, "StudioAuthoring");
        assert_eq!(decl.target_state, "PublicationSvgExported");

        let bad_initial = ComplexDecl::new("C", "id", vec!["A".into()], vec!["t".into()], vec![], "NonExistent", "A");
        assert!(bad_initial.validate().is_err());

        let bad_target = ComplexDecl::new("C", "id", vec!["A".into()], vec!["t".into()], vec![], "A", "NonExistent");
        assert!(bad_target.validate().is_err());

        let empty_name = ComplexDecl::new("", "id", vec!["A".into()], vec!["t".into()], vec![], "A", "A");
        assert!(empty_name.validate().is_err());
    }

    #[test]
    fn test_poet_cartography_live_edit_fixture_exists() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path = std::path::Path::new(manifest_dir)
            .join("fixtures")
            .join("geography")
            .join("poet_cartography_live_edit.vibe");
        assert!(
            fixture_path.exists(),
            "Fixture poet_cartography_live_edit.vibe must exist at {:?}",
            fixture_path
        );
    }
}
