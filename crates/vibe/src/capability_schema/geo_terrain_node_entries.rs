//! Procedural terrain DAG node capability schemas (GEO-S2 / R2-V).
//!
//! Provides machine schema definitions for all 30 `Terrain.*` host capability
//! operations covering noise generators, 14 landform primitives, modifiers,
//! selectors, combiners, and DAG compilation.

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

/// Parameter type for terrain DAG operations.
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

/// Operation schema descriptor for terrain DAG nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationSchema {
    pub invoke_id: &'static str,
    pub effect: Effect,
    pub params: &'static [ParamSchema],
    pub returns: &'static str,
    pub description: &'static str,
}

const fn pure_field_op(id: &'static str, desc: &'static str, params: &'static [ParamSchema]) -> OperationSchema {
    OperationSchema { invoke_id: id, effect: Effect::Pure, params, returns: "FieldRef", description: desc }
}

/// The 14 primitive landforms supported by `Terrain.primitive_landform`.
pub const PRIMITIVE_LANDFORMS: &[&str] = &[
    "Mountain", "Volcano", "Caldera", "Canyon", "DuneSea", "FaultBlock", "Escarpment",
    "ImpactCrater", "Archipelago", "RidgeLine", "Plateau", "TiltedStrata", "KarstTower",
    "GlacialCirque",
];

/// Returns `true` if `landform` matches any of the 14 registered primitive landforms.
pub fn is_valid_landform(landform: &str) -> bool {
    PRIMITIVE_LANDFORMS.iter().any(|&k| k == landform)
}

/// Canonical sequence of all 30 Terrain capability invoke IDs.
pub const GEO_TERRAIN_INVOKE_IDS: &[&str] = &[
    "Terrain.perlin", "Terrain.simplex", "Terrain.voronoi", "Terrain.gabor", "Terrain.wavelet",
    "Terrain.fbm", "Terrain.ridged", "Terrain.billow", "Terrain.hybrid_multi", "Terrain.swiss_turbulence",
    "Terrain.primitive_landform", "Terrain.terrace", "Terrain.stratify", "Terrain.fold",
    "Terrain.curve", "Terrain.autolevel", "Terrain.histogram_equalize", "Terrain.filter_morphology",
    "Terrain.curvature_map", "Terrain.select_height", "Terrain.select_slope", "Terrain.select_aspect",
    "Terrain.select_convexity", "Terrain.select_flow", "Terrain.select_ao", "Terrain.select_rock_treeline",
    "Terrain.combine", "Terrain.domain_warp", "Terrain.post_process", "Terrain.bake_dag",
];

/// Canonical sequence of all 30 Terrain DAG operations.
pub const GEO_TERRAIN_NODE_OPERATIONS: &[OperationSchema] = &[
    pure_field_op("Terrain.perlin", "Generates coherent 2D/3D Perlin gradient noise field.", &[
        p("seed", ParamType::U64, true, "RNG seed"), p("frequency", ParamType::F64, true, "Frequency"), p("octaves", ParamType::U64, true, "Octave count"), p("persistence", ParamType::F64, true, "Persistence"), p("lacunarity", ParamType::F64, true, "Lacunarity"),
    ]),
    pure_field_op("Terrain.simplex", "Generates Simplex continuous gradient noise field with reduced directional artifacts.", &[
        p("seed", ParamType::U64, true, "RNG seed"), p("frequency", ParamType::F64, true, "Frequency"), p("octaves", ParamType::U64, true, "Octave count"), p("persistence", ParamType::F64, true, "Persistence"),
    ]),
    pure_field_op("Terrain.voronoi", "Generates Voronoi/cellular cellular feature field with selectable metric.", &[
        p("seed", ParamType::U64, true, "RNG seed"), p("frequency", ParamType::F64, true, "Frequency"), p("jitter", ParamType::F64, true, "Cell jitter"), p("distance_metric", ParamType::Str, true, "Distance metric: Euclidean, Manhattan, Chebyshev"),
    ]),
    pure_field_op("Terrain.gabor", "Generates anisotropic Gabor noise kernel for oriented geological strata patterns.", &[
        p("seed", ParamType::U64, true, "RNG seed"), p("frequency", ParamType::F64, true, "Frequency"), p("orientation_deg", ParamType::F64, true, "Strike angle"), p("bandwidth", ParamType::F64, true, "Bandwidth"),
    ]),
    pure_field_op("Terrain.wavelet", "Generates band-limited wavelet noise without aliasing across spatial scales.", &[
        p("seed", ParamType::U64, true, "RNG seed"), p("scale", ParamType::F64, true, "Wavelet scale"), p("band_count", ParamType::U64, true, "Sub-band count"),
    ]),
    pure_field_op("Terrain.fbm", "Evaluates standard Fractional Brownian Motion multi-octave spectral synthesis.", &[
        p("noise_type", ParamType::Str, true, "Basis noise"), p("octaves", ParamType::U64, true, "Octave count"), p("lacunarity", ParamType::F64, true, "Lacunarity"), p("gain", ParamType::F64, true, "Gain / persistence"),
    ]),
    pure_field_op("Terrain.ridged", "Evaluates ridged multifractal noise producing sharp mountain crests and ridges.", &[
        p("noise_type", ParamType::Str, true, "Basis noise"), p("octaves", ParamType::U64, true, "Octave count"), p("lacunarity", ParamType::F64, true, "Lacunarity"), p("gain", ParamType::F64, true, "Gain"), p("offset", ParamType::F64, true, "Ridge offset"),
    ]),
    pure_field_op("Terrain.billow", "Evaluates billow noise producing puffy, rounded boulder or cloud-like forms.", &[
        p("noise_type", ParamType::Str, true, "Basis noise"), p("octaves", ParamType::U64, true, "Octave count"), p("lacunarity", ParamType::F64, true, "Lacunarity"), p("gain", ParamType::F64, true, "Gain"),
    ]),
    pure_field_op("Terrain.hybrid_multi", "Generates hybrid multifractal terrain varying roughness across elevation bands.", &[
        p("octaves", ParamType::U64, true, "Octaves"), p("roughness", ParamType::F64, true, "Roughness H"), p("lacunarity", ParamType::F64, true, "Lacunarity"), p("offset", ParamType::F64, true, "Zero-offset"),
    ]),
    pure_field_op("Terrain.swiss_turbulence", "Swiss turbulence procedural generator with derivative warping for alpine peaks.", &[
        p("octaves", ParamType::U64, true, "Octaves"), p("lacunarity", ParamType::F64, true, "Lacunarity"), p("gain", ParamType::F64, true, "Gain"), p("warp_factor", ParamType::F64, true, "Turbulence warp factor"),
    ]),
    pure_field_op("Terrain.primitive_landform", "Evaluates one of 14 procedural landform primitives (Mountain, Volcano, Caldera, Canyon, DuneSea, FaultBlock, Escarpment, ImpactCrater, Archipelago, RidgeLine, Plateau, TiltedStrata, KarstTower, GlacialCirque).", &[
        p("landform", ParamType::Str, true, "Landform identifier"), p("params", ParamType::Record, true, "Landform morphological parameters"),
    ]),
    pure_field_op("Terrain.terrace", "Applies geological terracing with step quantization and slope blending.", &[
        p("input", ParamType::FieldRef, true, "Input elevation field"), p("levels", ParamType::U64, true, "Terrace count"), p("smoothness", ParamType::F64, true, "Edge smoothing factor"), p("variability", ParamType::F64, true, "Step height variation"),
    ]),
    pure_field_op("Terrain.stratify", "Modulates elevation into alternating soft and hard sedimentary strata layers.", &[
        p("input", ParamType::FieldRef, true, "Input field"), p("strata_count", ParamType::U64, true, "Strata layer count"), p("sharpness", ParamType::F64, true, "Differential erosion sharpness"),
    ]),
    pure_field_op("Terrain.fold", "Applies geological tectonic folding (anticline/syncline) along a structural axis.", &[
        p("input", ParamType::FieldRef, true, "Input field"), p("wavelength", ParamType::F64, true, "Fold wavelength in meters"), p("axial_strike_deg", ParamType::F64, true, "Axial plane strike angle"), p("vergence", ParamType::F64, true, "Fold asymmetry / vergence"),
    ]),
    pure_field_op("Terrain.curve", "Remaps elevation profile through a piecewise monotonic spline curve.", &[
        p("input", ParamType::FieldRef, true, "Input field"), p("control_points", ParamType::List, true, "Spline knots [x, y]"),
    ]),
    pure_field_op("Terrain.autolevel", "Normalizes field elevation to target minimum and maximum altitude range.", &[
        p("input", ParamType::FieldRef, true, "Input field"), p("target_min", ParamType::F64, true, "Target min altitude"), p("target_max", ParamType::F64, true, "Target max altitude"),
    ]),
    pure_field_op("Terrain.histogram_equalize", "Performs cumulative histogram equalization to redistribute elevation contrast.", &[
        p("input", ParamType::FieldRef, true, "Input field"), p("bins", ParamType::U64, true, "Histogram bin count"), p("blend", ParamType::F64, true, "Equalization blend fraction"),
    ]),
    pure_field_op("Terrain.filter_morphology", "Performs mathematical morphology operations (erosion, dilation, opening, closing).", &[
        p("input", ParamType::FieldRef, true, "Input field"), p("op", ParamType::Str, true, "Operation: erode, dilate, open, close"), p("radius_m", ParamType::F64, true, "Structuring element radius in meters"),
    ]),
    pure_field_op("Terrain.curvature_map", "Computes differential terrain surface curvature (mean, Gaussian, profile, plan).", &[
        p("input", ParamType::FieldRef, true, "Input field"), p("scale_m", ParamType::F64, true, "Finite difference sampling radius in meters"),
    ]),
    pure_field_op("Terrain.select_height", "Generates an elevation threshold selection mask with smooth boundary falloff.", &[
        p("input", ParamType::FieldRef, true, "Input field"), p("min_alt_m", ParamType::F64, true, "Minimum altitude in meters"), p("max_alt_m", ParamType::F64, true, "Maximum altitude in meters"), p("falloff_m", ParamType::F64, true, "Boundary falloff width in meters"),
    ]),
    pure_field_op("Terrain.select_slope", "Generates a terrain surface slope selection mask in degrees with smooth margins.", &[
        p("input", ParamType::FieldRef, true, "Input field"), p("min_deg", ParamType::F64, true, "Minimum slope in degrees"), p("max_deg", ParamType::F64, true, "Maximum slope in degrees"), p("falloff_deg", ParamType::F64, true, "Margin falloff in degrees"),
    ]),
    pure_field_op("Terrain.select_aspect", "Generates a directional slope aspect selection mask (azimuth) with tolerance.", &[
        p("input", ParamType::FieldRef, true, "Input field"), p("azimuth_deg", ParamType::F64, true, "Target azimuth in degrees (0=N, 90=E)"), p("tolerance_deg", ParamType::F64, true, "Angular tolerance width in degrees"),
    ]),
    pure_field_op("Terrain.select_convexity", "Selects ridges (positive convexity) or valleys (negative convexity) across terrain.", &[
        p("input", ParamType::FieldRef, true, "Input field"), p("radius_m", ParamType::F64, true, "Neighborhood radius in meters"), p("threshold", ParamType::F64, true, "Convexity threshold"),
    ]),
    pure_field_op("Terrain.select_flow", "Generates hydrological upslope contributing accumulation mask from D8 flow routing.", &[
        p("input", ParamType::FieldRef, true, "Input field"), p("threshold_area_m2", ParamType::F64, true, "Flow accumulation threshold area in m2"),
    ]),
    pure_field_op("Terrain.select_ao", "Computes horizon-based ambient occlusion mask for crevice sediment accumulation.", &[
        p("input", ParamType::FieldRef, true, "Input field"), p("samples", ParamType::U64, true, "Ray sample count"), p("radius_m", ParamType::F64, true, "Occlusion search radius in meters"),
    ]),
    pure_field_op("Terrain.select_rock_treeline", "Selects exposed alpine rocky terrain based on altitude treeline and slope threshold.", &[
        p("input", ParamType::FieldRef, true, "Input field"), p("treeline_alt_m", ParamType::F64, true, "Ecological treeline altitude in meters"), p("slope_limit_deg", ParamType::F64, true, "Subalpine rocky slope threshold in degrees"),
    ]),
    pure_field_op("Terrain.combine", "Combines two terrain fields via mathematical blending (add, sub, mul, min, max, lerp).", &[
        p("a", ParamType::FieldRef, true, "First input field"), p("b", ParamType::FieldRef, true, "Second input field"), p("blend_mode", ParamType::Str, true, "Blend mode: add, sub, mul, min, max, lerp"), p("factor", ParamType::F64, true, "Blend weight or lerp alpha"),
    ]),
    pure_field_op("Terrain.domain_warp", "Warps terrain domain coordinates by a 2D vector offset displacement field.", &[
        p("input", ParamType::FieldRef, true, "Input field to displace"), p("warp_field", ParamType::FieldRef, true, "Vector offset or scalar warp field"), p("strength", ParamType::F64, true, "Displacement magnitude in meters"),
    ]),
    pure_field_op("Terrain.post_process", "Chains an ordered list of post-processing filter stages across the terrain field.", &[
        p("input", ParamType::FieldRef, true, "Input field"), p("steps", ParamType::List, true, "List of filter stage descriptor records"),
    ]),
    OperationSchema {
        invoke_id: "Terrain.bake_dag", effect: Effect::Compute, returns: "Record",
        description: "Executes and bakes a procedural terrain DAG into an evaluated elevation container.",
        params: &[
            p("root_node", ParamType::FieldRef, true, "Root output field of the terrain DAG"),
            p("bounds", ParamType::Record, true, "Bounding box record: min_x, min_y, max_x, max_y"),
            p("resolution_m", ParamType::F64, true, "Sampling grid cell resolution in meters"),
            p("format", ParamType::Str, true, "Target container format (m64_georaster, f32_grid, heightmap_raw)"),
        ],
    },
];

/// Looks up a terrain DAG node operation schema by its canonical invoke ID.
pub fn lookup_geo_terrain_node_operation(invoke_id: &str) -> Option<&'static OperationSchema> {
    GEO_TERRAIN_NODE_OPERATIONS.iter().find(|op| op.invoke_id == invoke_id)
}

/// Returns the 30 `Terrain.*` host capability schema entries for the catalog.
pub fn entries() -> Vec<SchemaEntry> {
    GEO_TERRAIN_NODE_OPERATIONS
        .iter()
        .map(|op| {
            let effect_class = match op.effect {
                Effect::Pure => EffectClass::Pure,
                Effect::Compute => EffectClass::Read,
                Effect::Cold => EffectClass::Read,
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
    fn test_terrain_operations_count_and_uniqueness() {
        assert_eq!(GEO_TERRAIN_NODE_OPERATIONS.len(), 30);
        assert_eq!(GEO_TERRAIN_INVOKE_IDS.len(), 30);
        for (i, op) in GEO_TERRAIN_NODE_OPERATIONS.iter().enumerate() {
            assert_eq!(op.invoke_id, GEO_TERRAIN_INVOKE_IDS[i]);
            assert!(op.invoke_id.starts_with("Terrain."));
            assert!(!op.returns.is_empty());
            assert!(!op.description.is_empty());
        }
        let mut ids: Vec<&str> = GEO_TERRAIN_INVOKE_IDS.to_vec();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 30, "All 30 Terrain invoke IDs must be unique");
    }

    #[test]
    fn test_primitive_landforms_complete() {
        assert_eq!(PRIMITIVE_LANDFORMS.len(), 14);
        for name in PRIMITIVE_LANDFORMS {
            assert!(is_valid_landform(name), "Primitive landform '{name}' must be valid");
        }
        assert!(!is_valid_landform("FlatPlains"));
    }

    #[test]
    fn test_bake_dag_is_compute_and_constructors_are_pure() {
        let bake = lookup_geo_terrain_node_operation("Terrain.bake_dag")
            .expect("Terrain.bake_dag must be registered");
        assert_eq!(bake.effect, Effect::Compute);
        assert_eq!(bake.returns, "Record");

        for op in GEO_TERRAIN_NODE_OPERATIONS {
            if op.invoke_id != "Terrain.bake_dag" {
                assert_eq!(op.effect, Effect::Pure, "Op {} must have Effect::Pure", op.invoke_id);
            }
        }
    }

    #[test]
    fn test_lookup_and_entries_conversion() {
        assert!(lookup_geo_terrain_node_operation("Terrain.swiss_turbulence").is_some());
        assert!(lookup_geo_terrain_node_operation("Terrain.unknown").is_none());

        let converted = entries();
        assert_eq!(converted.len(), 30);
        for (i, entry) in converted.iter().enumerate() {
            assert_eq!(entry.id, GEO_TERRAIN_INVOKE_IDS[i]);
            assert_eq!(entry.honesty, HonestyLabel::Always);
        }
    }

    #[test]
    fn test_terrain_dag_fixture_exists() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path = std::path::Path::new(manifest_dir)
            .join("fixtures")
            .join("geography")
            .join("terrain_dag_pipeline.vibe");
        assert!(
            fixture_path.exists(),
            "Fixture terrain_dag_pipeline.vibe must exist at {:?}",
            fixture_path
        );
    }
}
