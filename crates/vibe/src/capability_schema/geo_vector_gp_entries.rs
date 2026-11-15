//! Vector geoprocessing and planar parcel topology schemas (GEO-S6 / R6-V).
//!
//! Provides machine schema definitions for all 24 `GisGp.*` host capability invocations
//! covering overlay operations (clip, intersection, union, difference, sym_difference,
//! identity, update), proximity buffers, dissolve, boundary extraction, sliver elimination,
//! line simplification (Douglas-Peucker, Visvalingam-Whyatt), Chaikin smoothing, convex/concave hulls,
//! pole of inaccessibility (polylabel), Delaunay/Voronoi, grid tessellations (HexagonFlatTop, etc.),
//! spatial joins, Hausdorff/Fréchet metrics, KDE raster surface, and DBSCAN/KMeans clustering.

use super::{EffectClass, HonestyLabel, SchemaArg, SchemaEntry};

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

/// Parameter type for vector geoprocessing operations.
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

/// Operation schema descriptor for vector geoprocessing operations.
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

/// Declarative specification for a cadastral planar parcel topology complex.
#[derive(Debug, Clone, PartialEq)]
pub struct CadastralTopologyComplexDecl {
    pub complex_id: String,
    pub must_not_overlap: bool,
    pub must_not_have_gaps: bool,
    pub snap_tolerance_m: f64,
    pub min_sliver_area_m2: f64,
    pub crs_epsg: u32,
}

impl CadastralTopologyComplexDecl {
    pub fn new(
        complex_id: impl Into<String>,
        must_not_overlap: bool,
        must_not_have_gaps: bool,
        snap_tolerance_m: f64,
        min_sliver_area_m2: f64,
        crs_epsg: u32,
    ) -> Self {
        Self {
            complex_id: complex_id.into(),
            must_not_overlap,
            must_not_have_gaps,
            snap_tolerance_m,
            min_sliver_area_m2,
            crs_epsg,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.complex_id.is_empty() { return Err("complex_id must not be empty"); }
        if self.snap_tolerance_m <= 0.0 { return Err("snap_tolerance_m must be greater than zero"); }
        if self.min_sliver_area_m2 <= 0.0 { return Err("min_sliver_area_m2 must be greater than zero"); }
        if self.crs_epsg == 0 { return Err("crs_epsg must be greater than zero"); }
        Ok(())
    }
}

/// Canonical sequence of all 24 Vector Geoprocessing capability invoke IDs.
pub const GEO_VECTOR_GP_INVOKE_IDS: &[&str] = &[
    "GisGp.clip", "GisGp.intersection", "GisGp.union", "GisGp.difference",
    "GisGp.sym_difference", "GisGp.identity_overlay", "GisGp.update_overlay", "GisGp.buffer",
    "GisGp.multi_ring_buffer", "GisGp.dissolve", "GisGp.polygon_to_lines",
    "GisGp.snap_eliminate_slivers", "GisGp.simplify_dp", "GisGp.simplify_vw",
    "GisGp.smooth_chaikin", "GisGp.convex_hull", "GisGp.concave_hull",
    "GisGp.centroid_polylabel", "GisGp.delaunay_voronoi", "GisGp.tessellate_grid",
    "GisGp.spatial_join", "GisGp.hausdorff_frechet", "GisGp.kernel_density",
    "GisGp.cluster_dbscan_kmeans",
];

/// Canonical sequence of all 24 Vector Geoprocessing operation schemas.
pub const GEO_VECTOR_GP_OPERATIONS: &[OperationSchema] = &[
    op("GisGp.clip", Effect::Cold, "GeometryRef", "Clip input geometry layer to the extent of a clipping mask polygon.", &[
        p("input", ParamType::GeometryRef, true, "Input geometry or layer"),
        p("clip_mask", ParamType::GeometryRef, true, "Clipping mask polygon geometry"),
    ]),
    op("GisGp.intersection", Effect::Cold, "GeometryRef", "Compute topological intersection of two geometries preserving attributes.", &[
        p("geom_a", ParamType::GeometryRef, true, "First input geometry"),
        p("geom_b", ParamType::GeometryRef, true, "Second input geometry"),
    ]),
    op("GisGp.union", Effect::Cold, "GeometryRef", "Compute topological union of two geometries or planar polygonal partitions.", &[
        p("geom_a", ParamType::GeometryRef, true, "First input geometry"),
        p("geom_b", ParamType::GeometryRef, true, "Second input geometry"),
    ]),
    op("GisGp.difference", Effect::Cold, "GeometryRef", "Compute topological difference (erase) removing mask from input geometry.", &[
        p("input", ParamType::GeometryRef, true, "Base geometry"),
        p("erase_mask", ParamType::GeometryRef, true, "Mask geometry to erase from input"),
    ]),
    op("GisGp.sym_difference", Effect::Cold, "GeometryRef", "Compute symmetrical difference (exclusive OR) between two geometries.", &[
        p("geom_a", ParamType::GeometryRef, true, "First input geometry"),
        p("geom_b", ParamType::GeometryRef, true, "Second input geometry"),
    ]),
    op("GisGp.identity_overlay", Effect::Cold, "GeometryRef", "Compute geometric intersection with identity retaining all input features.", &[
        p("input", ParamType::GeometryRef, true, "Input primary geometry"),
        p("identity_layer", ParamType::GeometryRef, true, "Identity overlay geometry"),
    ]),
    op("GisGp.update_overlay", Effect::Cold, "GeometryRef", "Update input geometry by replacing overlapping regions with update geometry.", &[
        p("input", ParamType::GeometryRef, true, "Base input geometry"),
        p("update_geom", ParamType::GeometryRef, true, "Replacement geometry stamp"),
    ]),
    op("GisGp.buffer", Effect::Cold, "GeometryRef", "Generate fixed-distance buffer zone polygon around input geometry.", &[
        p("geom", ParamType::GeometryRef, true, "Input geometry"),
        p("distance_m", ParamType::F64, true, "Buffer distance in meters"),
        p("quad_segs", ParamType::U64, false, "Segments per quadrant (default 8)"),
    ]),
    op("GisGp.multi_ring_buffer", Effect::Cold, "Record", "Generate concentric multi-ring buffer zones at specified radial intervals.", &[
        p("geom", ParamType::GeometryRef, true, "Input geometry"),
        p("distances", ParamType::List, true, "List of increasing buffer distances in meters"),
        p("dissolve", ParamType::Bool, false, "Whether to dissolve rings into non-overlapping bands"),
    ]),
    op("GisGp.dissolve", Effect::Cold, "GeometryRef", "Dissolve boundaries between adjacent polygons sharing identical attributes.", &[
        p("geoms", ParamType::List, true, "List of input polygon geometries"),
        p("by_field", ParamType::Str, false, "Optional attribute field name to group and dissolve by"),
    ]),
    op("GisGp.polygon_to_lines", Effect::Cold, "GeometryRef", "Extract planar boundary linestrings and shared cadastral boundary edges from polygons.", &[
        p("geom", ParamType::GeometryRef, true, "Input polygon or multipolygon"),
        p("identify_shared", ParamType::Bool, false, "Label shared inter-parcel edges"),
    ]),
    op("GisGp.snap_eliminate_slivers", Effect::Cold, "Record", "Snap adjacent vertices within tolerance and merge sliver polygons into neighbors.", &[
        p("geoms", ParamType::List, true, "Cadastral polygon parcel collection"),
        p("snap_tolerance_m", ParamType::F64, true, "Vertex snapping tolerance in meters"),
        p("min_sliver_area_m2", ParamType::F64, true, "Minimum area threshold for sliver polygon elimination"),
    ]),
    op("GisGp.simplify_dp", Effect::Pure, "GeometryRef", "Simplify geometry vertices using Douglas-Peucker polyline decimation.", &[
        p("geom", ParamType::GeometryRef, true, "Input geometry"),
        p("tolerance_m", ParamType::F64, true, "Perpendicular distance error tolerance in meters"),
    ]),
    op("GisGp.simplify_vw", Effect::Cold, "GeometryRef", "Simplify geometry vertices using Visvalingam-Whyatt area-based effective triangles.", &[
        p("geom", ParamType::GeometryRef, true, "Input geometry"),
        p("min_area_m2", ParamType::F64, true, "Minimum triangle area threshold in square meters"),
    ]),
    op("GisGp.smooth_chaikin", Effect::Cold, "GeometryRef", "Smooth polyline or polygon rings using Chaikin corner-cutting subdivision.", &[
        p("geom", ParamType::GeometryRef, true, "Input polyline or polygon geometry"),
        p("iterations", ParamType::U64, false, "Subdivision refinement passes (1..5, default 2)"),
    ]),
    op("GisGp.convex_hull", Effect::Cold, "GeometryRef", "Compute smallest convex bounding polygon enclosing input points or geometry vertices.", &[
        p("geom", ParamType::GeometryRef, true, "Input geometry or point collection"),
    ]),
    op("GisGp.concave_hull", Effect::Cold, "GeometryRef", "Compute alpha-shape concave hull boundary hugging the geometry perimeter.", &[
        p("geom", ParamType::GeometryRef, true, "Input geometry or point collection"),
        p("concavity_alpha", ParamType::F64, true, "Concavity factor or alpha parameter (0.0..1.0)"),
    ]),
    op("GisGp.centroid_polylabel", Effect::Pure, "Record", "Compute visual centroid or pole of inaccessibility (polylabel) inside polygon interior.", &[
        p("geom", ParamType::GeometryRef, true, "Input polygon geometry"),
        p("precision", ParamType::F64, false, "Convergence precision tolerance (default 1.0)"),
    ]),
    op("GisGp.delaunay_voronoi", Effect::Cold, "Record", "Construct 2D Delaunay triangulation and dual Voronoi Thiessen polygon partition.", &[
        p("points", ParamType::List, true, "Seed point locations or coordinate pairs"),
        p("clip_extent", ParamType::Record, false, "Bounding envelope for Voronoi diagram boundary"),
    ]),
    op("GisGp.tessellate_grid", Effect::Cold, "Record", "Generate regular spatial tessellation covering bounding envelope.", &[
        p("bbox", ParamType::Record, true, "Bounding box extent record (min_x, min_y, max_x, max_y)"),
        p("cell_shape", ParamType::Str, true, "Tessellation shape: HexagonFlatTop, HexagonPointyTop, Square, Triangle"),
        p("cell_size_m", ParamType::F64, true, "Hexagon diameter or cell edge size in meters"),
    ]),
    op("GisGp.spatial_join", Effect::Cold, "Record", "Perform spatial predicate join transferring target attributes to source features.", &[
        p("target_layer", ParamType::List, true, "Target feature geometries to receive attributes"),
        p("join_layer", ParamType::List, true, "Join feature geometries providing attributes"),
        p("match_predicate", ParamType::Str, true, "Topological predicate: Intersects, Within, Contains, Closest"),
        p("join_type", ParamType::Str, false, "Join cardinality: OneToOne or OneToMany"),
    ]),
    op("GisGp.hausdorff_frechet", Effect::Pure, "Record", "Compute Hausdorff and discrete Fréchet trajectory similarity distance metrics.", &[
        p("geom_a", ParamType::GeometryRef, true, "First polyline or polygon boundary"),
        p("geom_b", ParamType::GeometryRef, true, "Second polyline or polygon boundary"),
    ]),
    op("GisGp.kernel_density", Effect::Compute, "FieldRef", "Compute 2D kernel density estimation (KDE) surface over point events with bandwidth.", &[
        p("points", ParamType::List, true, "Input point locations with optional weights"),
        p("bandwidth_m", ParamType::F64, true, "Kernel search radius bandwidth in meters"),
        p("cell_size_m", ParamType::F64, true, "Output raster surface grid cell resolution in meters"),
    ]),
    op("GisGp.cluster_dbscan_kmeans", Effect::Cold, "Record", "Cluster spatial points using density-based DBSCAN or centroid-based K-Means.", &[
        p("points", ParamType::List, true, "Input point coordinates list"),
        p("algorithm", ParamType::Str, true, "Clustering algorithm: dbscan or kmeans"),
        p("eps_or_k", ParamType::F64, true, "Epsilon neighborhood radius (DBSCAN) or cluster count k (KMeans)"),
        p("min_points", ParamType::U64, false, "Minimum core points for DBSCAN (unused for KMeans)"),
    ]),
];

/// Looks up a Vector Geoprocessing operation schema by its canonical invoke ID.
pub fn lookup_geo_vector_gp_operation(invoke_id: &str) -> Option<&'static OperationSchema> {
    GEO_VECTOR_GP_OPERATIONS.iter().find(|op| op.invoke_id == invoke_id)
}

/// Returns the 24 Vector Geoprocessing capability schema entries for the catalog.
pub fn entries() -> Vec<SchemaEntry> {
    GEO_VECTOR_GP_OPERATIONS.iter().map(|op| {
        let effect_class = match op.effect {
            Effect::Pure => EffectClass::Pure,
            Effect::Compute | Effect::Cold => EffectClass::Read,
            Effect::Async | Effect::External => EffectClass::External,
        };
        let mut entry = SchemaEntry::new(op.invoke_id, effect_class, HonestyLabel::Always, op.returns)
            .with_description(op.description);
        for p in op.params {
            entry = entry.with_arg(SchemaArg::new(p.name, p.param_type.as_str(), p.required).with_description(p.description));
        }
        entry
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geo_vector_gp_operations_count_and_uniqueness() {
        assert_eq!(GEO_VECTOR_GP_OPERATIONS.len(), 24);
        assert_eq!(GEO_VECTOR_GP_INVOKE_IDS.len(), 24);
        for (i, op) in GEO_VECTOR_GP_OPERATIONS.iter().enumerate() {
            assert_eq!(op.invoke_id, GEO_VECTOR_GP_INVOKE_IDS[i]);
            assert!(op.invoke_id.starts_with("GisGp."));
            assert!(!op.returns.is_empty());
            assert!(!op.description.is_empty());
        }
        let mut ids: Vec<&str> = GEO_VECTOR_GP_INVOKE_IDS.to_vec();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 24, "All 24 invoke IDs must be unique");
    }

    #[test]
    fn test_entries_conversion_and_lookup() {
        assert!(lookup_geo_vector_gp_operation("GisGp.clip").is_some());
        assert!(lookup_geo_vector_gp_operation("GisGp.buffer").is_some());
        assert!(lookup_geo_vector_gp_operation("GisGp.snap_eliminate_slivers").is_some());
        assert!(lookup_geo_vector_gp_operation("GisGp.delaunay_voronoi").is_some());
        assert!(lookup_geo_vector_gp_operation("GisGp.kernel_density").is_some());
        assert!(lookup_geo_vector_gp_operation("GisGp.cluster_dbscan_kmeans").is_some());
        assert!(lookup_geo_vector_gp_operation("GisGp.unknown_op").is_none());

        let converted = entries();
        assert_eq!(converted.len(), 24);
        for (i, entry) in converted.iter().enumerate() {
            assert_eq!(entry.id, GEO_VECTOR_GP_INVOKE_IDS[i]);
            assert_eq!(entry.honesty, HonestyLabel::Always);
        }
    }

    #[test]
    fn test_cadastral_topology_complex_decl_validation() {
        let valid = CadastralTopologyComplexDecl::new(
            "snowy_cadastral_fabric", true, true, 0.05, 1.0, 7855,
        );
        assert!(valid.validate().is_ok());
        assert!(valid.must_not_overlap);
        assert!(valid.must_not_have_gaps);
        assert_eq!(valid.snap_tolerance_m, 0.05);
        assert_eq!(valid.min_sliver_area_m2, 1.0);
        assert_eq!(valid.crs_epsg, 7855);

        let empty_id = CadastralTopologyComplexDecl::new("", true, true, 0.05, 1.0, 7855);
        assert!(empty_id.validate().is_err());

        let zero_snap = CadastralTopologyComplexDecl::new("test", true, true, 0.0, 1.0, 7855);
        assert!(zero_snap.validate().is_err());

        let neg_sliver = CadastralTopologyComplexDecl::new("test", true, true, 0.05, -1.0, 7855);
        assert!(neg_sliver.validate().is_err());

        let zero_crs = CadastralTopologyComplexDecl::new("test", true, true, 0.05, 1.0, 0);
        assert!(zero_crs.validate().is_err());
    }

    #[test]
    fn test_vector_geoprocessing_fixture_exists() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path = std::path::Path::new(manifest_dir)
            .join("fixtures")
            .join("geography")
            .join("vector_geoprocessing.vibe");
        assert!(
            fixture_path.exists(),
            "Fixture vector_geoprocessing.vibe must exist at {:?}",
            fixture_path
        );
    }
}
