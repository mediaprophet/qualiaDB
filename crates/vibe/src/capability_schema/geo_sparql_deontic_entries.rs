//! GeoSPARQL topological predicates and deontic spatial covenant schemas (GEO-S5 / R5-V).
//!
//! Provides machine schema definitions for 19 `GeoSparql.*` and OGC/Datacube `GisIo.*`
//! host capability invocations covering DE-9IM intersection matrices, simple features
//! topological relations (sfEquals, sfDisjoint, sfIntersects, sfTouches, sfWithin,
//! sfContains, sfOverlaps, distance), 48B NQuin spatial minting with deontic opcodes (0x10..0x12)
//! and defeater bit, ODRL defeasible covenant evaluation, OGC API Features CQL2, WFS/ArcGIS/CSW,
//! STAC 1.0 catalog search, OGC SensorThings/SOS, COG range-read planning, WCS/WMS/WMTS,
//! Zarr/NetCDF/GRIB2 unpacker, and 10D Environmental Tensor Datacube compilation.

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

/// Parameter type for GeoSPARQL and GIS/Datacube operations.
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

/// Operation schema descriptor for GeoSPARQL and GIS/Datacube operations.
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

/// Deontic logic norm opcodes (0x10..=0x12).
pub const OP_OBLIGATE: u8 = 0x10;
pub const OP_PERMIT: u8 = 0x11;
pub const OP_FORBID: u8 = 0x12;

/// High-bit mask for Defeater rules in deontic predicate (`^>` vs `~>`).
pub const DEFEATER_BIT: u64 = 1u64 << 63;

/// Declarative specification for a spatial covenant protection zone.
#[derive(Debug, Clone, PartialEq)]
pub struct SpatialCovenantZoneDecl {
    pub zone_id: String,
    pub covenant_type: String,
    pub deontic_opcode: u8,
    pub is_defeater: bool,
    pub buffer_distance_m: f64,
    pub priority: u32,
}

impl SpatialCovenantZoneDecl {
    pub fn new(
        zone_id: impl Into<String>, covenant_type: impl Into<String>,
        deontic_opcode: u8, is_defeater: bool, buffer_distance_m: f64, priority: u32,
    ) -> Self {
        Self {
            zone_id: zone_id.into(), covenant_type: covenant_type.into(),
            deontic_opcode, is_defeater, buffer_distance_m, priority,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.zone_id.is_empty() { return Err("zone_id must not be empty"); }
        if self.covenant_type.is_empty() { return Err("covenant_type must not be empty"); }
        if !matches!(self.deontic_opcode, OP_OBLIGATE | OP_PERMIT | OP_FORBID) {
            return Err("deontic_opcode must be 0x10, 0x11, or 0x12");
        }
        if self.buffer_distance_m < 0.0 { return Err("buffer_distance_m must not be negative"); }
        Ok(())
    }
}

/// Canonical sequence of all 19 GeoSPARQL and OGC / Datacube capability invoke IDs.
pub const GEO_SPARQL_DEONTIC_INVOKE_IDS: &[&str] = &[
    "GeoSparql.de9im_matrix", "GeoSparql.sf_equals", "GeoSparql.sf_disjoint",
    "GeoSparql.sf_intersects", "GeoSparql.sf_touches", "GeoSparql.sf_within",
    "GeoSparql.sf_contains", "GeoSparql.sf_overlaps", "GeoSparql.distance",
    "GeoSparql.mint_spatial_quin", "GeoSparql.evaluate_deontic_covenant",
    "GisIo.query_ogc_features_cql2", "GisIo.query_wfs_arcgis_csw", "GisIo.query_stac_catalog",
    "GisIo.query_sensorthings_sos", "GisIo.plan_cog_range_read", "GisIo.query_wcs_wms_wmts",
    "GisIo.read_zarr_netcdf_grib", "GisIo.compile_datacube_10d",
];

/// Canonical sequence of all 19 GeoSPARQL and OGC / Datacube operations.
pub const GEO_SPARQL_DEONTIC_OPERATIONS: &[OperationSchema] = &[
    op("GeoSparql.de9im_matrix", Effect::Pure, "Str", "Compute 9-character DE-9IM intersection matrix between two geometries.", &[
        p("geom_a", ParamType::GeometryRef, true, "First geometry operand"),
        p("geom_b", ParamType::GeometryRef, true, "Second geometry operand"),
    ]),
    op("GeoSparql.sf_equals", Effect::Pure, "Bool", "Evaluate topological equality predicate sfEquals (DE-9IM T*F**FFF*).", &[
        p("geom_a", ParamType::GeometryRef, true, "First geometry operand"),
        p("geom_b", ParamType::GeometryRef, true, "Second geometry operand"),
    ]),
    op("GeoSparql.sf_disjoint", Effect::Pure, "Bool", "Evaluate topological disjointness predicate sfDisjoint (DE-9IM FF*FF****).", &[
        p("geom_a", ParamType::GeometryRef, true, "First geometry operand"),
        p("geom_b", ParamType::GeometryRef, true, "Second geometry operand"),
    ]),
    op("GeoSparql.sf_intersects", Effect::Pure, "Bool", "Evaluate topological intersection predicate sfIntersects (DE-9IM !FF*FF****).", &[
        p("geom_a", ParamType::GeometryRef, true, "First geometry operand"),
        p("geom_b", ParamType::GeometryRef, true, "Second geometry operand"),
    ]),
    op("GeoSparql.sf_touches", Effect::Pure, "Bool", "Evaluate topological boundary touch predicate sfTouches.", &[
        p("geom_a", ParamType::GeometryRef, true, "First geometry operand"),
        p("geom_b", ParamType::GeometryRef, true, "Second geometry operand"),
    ]),
    op("GeoSparql.sf_within", Effect::Pure, "Bool", "Evaluate topological within predicate sfWithin (DE-9IM T*F**F***).", &[
        p("geom_a", ParamType::GeometryRef, true, "Candidate interior geometry"),
        p("geom_b", ParamType::GeometryRef, true, "Target enclosing geometry"),
    ]),
    op("GeoSparql.sf_contains", Effect::Pure, "Bool", "Evaluate topological spatial containment predicate sfContains (DE-9IM T*****FF*).", &[
        p("geom_a", ParamType::GeometryRef, true, "Target enclosing geometry"),
        p("geom_b", ParamType::GeometryRef, true, "Candidate interior geometry"),
    ]),
    op("GeoSparql.sf_overlaps", Effect::Pure, "Bool", "Evaluate topological spatial overlap predicate sfOverlaps (DE-9IM T*T***T**).", &[
        p("geom_a", ParamType::GeometryRef, true, "First geometry operand"),
        p("geom_b", ParamType::GeometryRef, true, "Second geometry operand"),
    ]),
    op("GeoSparql.distance", Effect::Pure, "F64", "Compute minimum distance between geometries in meters.", &[
        p("geom_a", ParamType::GeometryRef, true, "First geometry operand"),
        p("geom_b", ParamType::GeometryRef, true, "Second geometry operand"),
        p("crs", ParamType::Str, false, "Coordinate reference system (default EPSG:7855)"),
    ]),
    op("GeoSparql.mint_spatial_quin", Effect::Pure, "Record", "Mint 48B NQuin with deontic opcode (0x10..0x12) and defeater bit.", &[
        p("subject_did", ParamType::Str, true, "Subject entity or place DID"),
        p("predicate_iri", ParamType::Str, true, "Predicate norm IRI"),
        p("object_ref", ParamType::Record, true, "Target geometry or resource payload reference"),
        p("deontic_opcode", ParamType::U64, true, "Deontic opcode: 0x10 (Obligate), 0x11 (Permit), 0x12 (Forbid)"),
        p("is_defeater", ParamType::Bool, true, "Defeater rule flag (^> vs ~>) encoded into predicate MSB"),
        p("context_zone", ParamType::Str, true, "Covenant context zone DID or graph IRI"),
    ]),
    op("GeoSparql.evaluate_deontic_covenant", Effect::Pure, "Record", "Evaluate DE-9IM intersection against covenant zones and resolve defeasible rules.", &[
        p("proposed_geom", ParamType::GeometryRef, true, "Proposed activity or infrastructure footprint geometry"),
        p("covenant_zones", ParamType::List, true, "List of active SpatialCovenantZone protection records"),
        p("context", ParamType::Record, false, "Evaluation context with requester credentials and epoch"),
    ]),
    op("GisIo.query_ogc_features_cql2", Effect::External, "Record", "Query OGC API Features endpoint with CQL2 filter.", &[
        p("url", ParamType::Str, true, "OGC API Features service endpoint URL"),
        p("collection_id", ParamType::Str, true, "Feature collection identifier"),
        p("cql2_filter", ParamType::Str, true, "CQL2-JSON or CQL2-Text query filter expression"),
        p("bbox", ParamType::Record, false, "Spatial bounding box filter record"),
        p("limit", ParamType::U64, false, "Maximum feature limit per page"),
    ]),
    op("GisIo.query_wfs_arcgis_csw", Effect::External, "Record", "Query WFS 2.0, ArcGIS REST FeatureServer, or CSW 3.0 catalog.", &[
        p("service_url", ParamType::Str, true, "Base service endpoint URL"),
        p("service_type", ParamType::Str, true, "Service type selector (wfs, arcgis, csw)"),
        p("type_names", ParamType::Str, true, "Feature type names or catalog layer name"),
        p("filter_xml_json", ParamType::Str, false, "OGC XML filter or ArcGIS JSON where clause"),
        p("bbox", ParamType::Record, false, "Spatial bounding box filter record"),
    ]),
    op("GisIo.query_stac_catalog", Effect::External, "Record", "Query STAC 1.0 catalog API search and select cloud-free COG assets.", &[
        p("stac_url", ParamType::Str, true, "STAC API search endpoint URL"),
        p("collections", ParamType::List, true, "Target collection IDs list"),
        p("bbox", ParamType::Record, true, "Spatial bounding box filter record"),
        p("datetime", ParamType::Str, false, "RFC 3339 datetime or interval string"),
        p("max_cloud_cover", ParamType::F64, false, "Maximum cloud cover percentage threshold"),
    ]),
    op("GisIo.query_sensorthings_sos", Effect::External, "Record", "Query OGC SensorThings v1.1 API or SOS 2.0 observation stream.", &[
        p("service_url", ParamType::Str, true, "SensorThings or SOS service root URL"),
        p("observed_property", ParamType::Str, true, "Target observed property name or URN"),
        p("temporal_range", ParamType::Str, false, "ISO 8601 temporal range filter"),
        p("bbox", ParamType::Record, false, "Spatial bounding box filter record"),
    ]),
    op("GisIo.plan_cog_range_read", Effect::Pure, "Record", "Parse COG IFD headers and plan HTTP byte-range GET requests.", &[
        p("cog_metadata", ParamType::Record, true, "Cloud-Optimized GeoTIFF IFD metadata record"),
        p("target_bbox", ParamType::Record, true, "Target sub-window bounding box record"),
        p("resolution_level", ParamType::U64, false, "Overview level (0 = full resolution)"),
    ]),
    op("GisIo.query_wcs_wms_wmts", Effect::External, "Record", "Query OGC WCS 2.0, axis-order-aware WMS 1.3.0, or WMTS 1.0 tile matrix.", &[
        p("service_url", ParamType::Str, true, "OGC service endpoint URL"),
        p("service_type", ParamType::Str, true, "Service type selector (wcs, wms, wmts)"),
        p("layer", ParamType::Str, true, "Layer or coverage identifier"),
        p("bbox", ParamType::Record, true, "Spatial bounding box filter record"),
        p("crs", ParamType::Str, true, "Coordinate reference system"),
        p("width", ParamType::U64, false, "Raster image pixel width"),
        p("height", ParamType::U64, false, "Raster image pixel height"),
    ]),
    op("GisIo.read_zarr_netcdf_grib", Effect::Cold, "Record", "Unpack gridded environmental array data from Zarr v2/v3, NetCDF, or GRIB2.", &[
        p("path", ParamType::Str, true, "File path or URL to .zarr, .nc, or .grib2 dataset"),
        p("format", ParamType::Str, true, "Format selector (zarr, netcdf, grib2)"),
        p("variable_names", ParamType::List, true, "Variable names to extract"),
        p("temporal_slice", ParamType::Str, false, "Temporal index or timestamp range"),
        p("byte_budget", ParamType::U64, false, "Maximum byte budget for buffer allocation"),
    ]),
    op("GisIo.compile_datacube_10d", Effect::Compute, "Record", "Compile multi-variable slices into 10D Environmental Tensor Cube with QUDT units.", &[
        p("slices", ParamType::Record, true, "Input spatial and multi-variable slice records"),
        p("spatial_extent", ParamType::Record, true, "Bounding spatial extent record"),
        p("temporal_extent", ParamType::Record, true, "Temporal range and interval record"),
        p("qudt_units", ParamType::List, true, "QUDT unit tokens list for tensor dimensions"),
    ]),
];

/// Looks up a GeoSPARQL or GIS / Datacube operation schema by its canonical invoke ID.
pub fn lookup_geo_sparql_deontic_operation(invoke_id: &str) -> Option<&'static OperationSchema> {
    GEO_SPARQL_DEONTIC_OPERATIONS.iter().find(|op| op.invoke_id == invoke_id)
}

/// Returns the 19 GeoSPARQL and OGC / Datacube capability schema entries for the catalog.
pub fn entries() -> Vec<SchemaEntry> {
    GEO_SPARQL_DEONTIC_OPERATIONS.iter().map(|op| {
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
    fn test_geo_sparql_deontic_operations_count_and_uniqueness() {
        assert_eq!(GEO_SPARQL_DEONTIC_OPERATIONS.len(), 19);
        assert_eq!(GEO_SPARQL_DEONTIC_INVOKE_IDS.len(), 19);
        for (i, op) in GEO_SPARQL_DEONTIC_OPERATIONS.iter().enumerate() {
            assert_eq!(op.invoke_id, GEO_SPARQL_DEONTIC_INVOKE_IDS[i]);
            assert!(op.invoke_id.starts_with("GeoSparql.") || op.invoke_id.starts_with("GisIo."));
            assert!(!op.returns.is_empty());
            assert!(!op.description.is_empty());
        }
        let mut ids: Vec<&str> = GEO_SPARQL_DEONTIC_INVOKE_IDS.to_vec();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 19, "All 19 invoke IDs must be unique");
    }

    #[test]
    fn test_entries_conversion_and_lookup() {
        assert!(lookup_geo_sparql_deontic_operation("GeoSparql.de9im_matrix").is_some());
        assert!(lookup_geo_sparql_deontic_operation("GeoSparql.mint_spatial_quin").is_some());
        assert!(lookup_geo_sparql_deontic_operation("GeoSparql.evaluate_deontic_covenant").is_some());
        assert!(lookup_geo_sparql_deontic_operation("GisIo.query_stac_catalog").is_some());
        assert!(lookup_geo_sparql_deontic_operation("GisIo.plan_cog_range_read").is_some());
        assert!(lookup_geo_sparql_deontic_operation("GisIo.compile_datacube_10d").is_some());
        assert!(lookup_geo_sparql_deontic_operation("GeoSparql.unknown_operation").is_none());

        let converted = entries();
        assert_eq!(converted.len(), 19);
        for (i, entry) in converted.iter().enumerate() {
            assert_eq!(entry.id, GEO_SPARQL_DEONTIC_INVOKE_IDS[i]);
            assert_eq!(entry.honesty, HonestyLabel::Always);
        }
    }

    #[test]
    fn test_spatial_covenant_zone_decl_validation() {
        let valid_forbid = SpatialCovenantZoneDecl::new(
            "urn:qualia:zone:sphagnum-bog-kosciuszko",
            "IndigenousCulturalHeritageAndPeatlandSanctuary",
            OP_FORBID, true, 150.0, 1,
        );
        assert!(valid_forbid.validate().is_ok());
        assert_eq!(valid_forbid.deontic_opcode, 0x12);
        assert!(valid_forbid.is_defeater);
        assert_eq!(valid_forbid.buffer_distance_m, 150.0);

        let valid_permit = SpatialCovenantZoneDecl::new(
            "urn:qualia:zone:perisher-resort-corridor",
            "PermittedSkiCorridor",
            OP_PERMIT, false, 0.0, 2,
        );
        assert!(valid_permit.validate().is_ok());

        let invalid_opcode = SpatialCovenantZoneDecl::new("urn:qualia:zone:invalid", "Type", 0x00, false, 10.0, 1);
        assert!(invalid_opcode.validate().is_err());

        let empty_id = SpatialCovenantZoneDecl::new("", "ValidType", OP_FORBID, false, 10.0, 1);
        assert!(empty_id.validate().is_err());

        let empty_type = SpatialCovenantZoneDecl::new("urn:qualia:zone:test", "", OP_FORBID, false, 10.0, 1);
        assert!(empty_type.validate().is_err());

        let negative_buffer = SpatialCovenantZoneDecl::new("urn:qualia:zone:test", "ValidType", OP_FORBID, false, -5.0, 1);
        assert!(negative_buffer.validate().is_err());
    }

    #[test]
    fn test_geosparql_deontic_covenant_fixture_exists() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path = std::path::Path::new(manifest_dir)
            .join("fixtures")
            .join("geography")
            .join("geosparql_deontic_covenant.vibe");
        assert!(
            fixture_path.exists(),
            "Fixture geosparql_deontic_covenant.vibe must exist at {:?}",
            fixture_path
        );
    }
}
