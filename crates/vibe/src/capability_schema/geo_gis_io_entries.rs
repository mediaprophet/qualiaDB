//! GIS vector, raster, LiDAR, and container capability schemas (GEO-S4 / R4-V).
//!
//! Provides machine schema definitions for all 20 `GisIo.*` host capability invocations
//! covering LiDAR point clouds (LAS/LAZ/COPC, CSF classification, m64 Mode 0x00, DTM/DSM/CHM),
//! vector spatial formats (GeoPackage, FlatGeobuf, FileGDB, Shapefile, GeoParquet, GeoJSON,
//! KML/GPX/GML, DXF CAD, TopoJSON/OSM), and tiled vector publishing (PMTiles, MVT).

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

/// Parameter type for GIS I/O and LiDAR operations.
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

/// Operation schema descriptor for GIS I/O and LiDAR operations.
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

/// Canonical ASPRS LAS 1.4 point classification standard codes (0..=18).
pub const ASPRS_CLASSIFICATION_NAMES: [&str; 19] = [
    "CreatedNeverClassified", "Unclassified", "Ground", "LowVegetation",
    "MediumVegetation", "HighVegetation", "Building", "LowPointNoise",
    "ModelKeyPoint", "Water", "Rail", "RoadSurface", "Reserved12",
    "WireGuard", "WireConductor", "TransmissionTower", "WireConnector",
    "BridgeDeck", "HighNoise",
];

/// Validator and container helper for `LidarPointComplex` (.m64 Mode 0x00).
#[derive(Debug, Clone, PartialEq)]
pub struct LidarPointComplexDecl {
    pub name: String,
    pub m64_submagic: u8,
    pub crs: String,
    pub byte_budget: u64,
    pub classification_codes: Vec<u8>,
    pub point_count: u64,
}

impl LidarPointComplexDecl {
    pub const EXPECTED_SUBMAGIC: u8 = 0x00;
    pub const MAX_ASPRS_CODE: u8 = 18;

    pub fn new(
        name: impl Into<String>, m64_submagic: u8, crs: impl Into<String>,
        byte_budget: u64, classification_codes: Vec<u8>, point_count: u64,
    ) -> Self {
        Self {
            name: name.into(), m64_submagic, crs: crs.into(),
            byte_budget, classification_codes, point_count,
        }
    }

    pub fn is_valid_asprs_code(code: u8) -> bool {
        code <= Self::MAX_ASPRS_CODE
    }

    pub fn asprs_code_name(code: u8) -> Option<&'static str> {
        ASPRS_CLASSIFICATION_NAMES.get(code as usize).copied()
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.m64_submagic != Self::EXPECTED_SUBMAGIC {
            return Err("m64_submagic must equal 0x00 for LidarPointComplex");
        }
        if self.byte_budget == 0 {
            return Err("byte_budget must be strictly positive");
        }
        if self.crs.is_empty() {
            return Err("crs coordinate reference system must not be empty");
        }
        for &code in &self.classification_codes {
            if !Self::is_valid_asprs_code(code) {
                return Err("classification code exceeds ASPRS 1.4 range 0..=18");
            }
        }
        Ok(())
    }
}

/// Canonical sequence of all 20 GIS I/O and LiDAR capability invoke IDs.
pub const GEO_GIS_IO_INVOKE_IDS: &[&str] = &[
    "GisIo.read_lidar_copc", "GisIo.classify_lidar_csf", "GisIo.pack_lidar_m64",
    "GisIo.rasterize_dtm_dsm_chm", "GisIo.read_geopackage", "GisIo.write_geopackage",
    "GisIo.read_flatgeobuf", "GisIo.write_flatgeobuf", "GisIo.read_filegdb",
    "GisIo.read_shapefile", "GisIo.write_shapefile", "GisIo.read_geoparquet",
    "GisIo.write_geoparquet", "GisIo.read_geojson", "GisIo.write_geojson",
    "GisIo.read_kml_gpx_gml", "GisIo.read_dxf_cad", "GisIo.read_topojson_osm",
    "GisIo.read_pmtiles_mvt", "GisIo.encode_mvt_tile",
];

/// Canonical sequence of all 20 GIS I/O and LiDAR operations.
pub const GEO_GIS_IO_OPERATIONS: &[OperationSchema] = &[
    op("GisIo.read_lidar_copc", Effect::Cold, "Record", "Parse LAS 1.2-1.4 / COPC octree hierarchy and point cloud.", &[
        p("path", ParamType::Str, true, "File path or URL to .copc.laz / .laz / .las"),
        p("bounds", ParamType::Record, false, "Spatial bounding box filter record"),
        p("byte_budget", ParamType::U64, false, "Maximum byte budget for point allocation"),
        p("classification_filter", ParamType::List, false, "ASPRS classification code filter list"),
    ]),
    op("GisIo.classify_lidar_csf", Effect::Cold, "Record", "Cloth Simulation Filter ground/canopy separation and HAG computation.", &[
        p("points", ParamType::Record, true, "Input LiDAR point cloud record"),
        p("cloth_resolution_m", ParamType::F64, true, "Cloth grid resolution in meters"),
        p("rigidness", ParamType::U64, true, "Cloth rigidity parameter (1=flat, 2=relief, 3=steep)"),
        p("time_step", ParamType::F64, true, "Simulation time step"),
        p("class_threshold_m", ParamType::F64, true, "Ground/canopy height threshold in meters"),
    ]),
    op("GisIo.pack_lidar_m64", Effect::Compute, "Record", "Pack LiDAR points into 64B .m64 Mode 0x00 records with CRC-32C.", &[
        p("points", ParamType::Record, true, "Classified LiDAR point cloud record"),
        p("crs", ParamType::Str, true, "Coordinate reference system (e.g. EPSG:7855)"),
        p("quantization_step_m", ParamType::F64, true, "Coordinate quantization precision step in meters"),
    ]),
    op("GisIo.rasterize_dtm_dsm_chm", Effect::Cold, "Record", "Rasterize bare-earth DTM, first-return DSM, and Canopy Height Model CHM.", &[
        p("points", ParamType::Record, true, "Classified LiDAR point cloud record"),
        p("resolution_m", ParamType::F64, true, "Raster grid resolution in meters"),
        p("interpolation", ParamType::Str, true, "Interpolation method (idw, tin_linear, nearest)"),
    ]),
    op("GisIo.read_geopackage", Effect::Cold, "Record", "Read vector layer features and metadata from OGC GeoPackage.", &[
        p("path", ParamType::Str, true, "File path to .gpkg database"),
        p("layer", ParamType::Str, false, "Vector layer name"),
        p("bbox", ParamType::Record, false, "Spatial bounding box filter record"),
    ]),
    op("GisIo.write_geopackage", Effect::External, "Record", "Write vector features and geometries into OGC GeoPackage layer.", &[
        p("path", ParamType::Str, true, "File path to target .gpkg database"),
        p("layer", ParamType::Str, true, "Target layer name"),
        p("features", ParamType::Record, true, "Vector feature collection record"),
        p("crs", ParamType::Str, true, "Layer coordinate reference system"),
    ]),
    op("GisIo.read_flatgeobuf", Effect::Cold, "Record", "Zero-copy Hilbert R-tree spatial bbox filter reader for FlatGeobuf.", &[
        p("path", ParamType::Str, true, "File path or URL to .fgb dataset"),
        p("bbox", ParamType::Record, false, "Spatial bounding box filter [min_x, min_y, max_x, max_y]"),
        p("byte_budget", ParamType::U64, false, "Maximum byte budget"),
    ]),
    op("GisIo.write_flatgeobuf", Effect::External, "Record", "Write vector features to FlatGeobuf with indexed Hilbert R-tree.", &[
        p("path", ParamType::Str, true, "Output file path for .fgb"),
        p("features", ParamType::Record, true, "Vector feature collection record"),
        p("crs", ParamType::Str, true, "Coordinate reference system"),
    ]),
    op("GisIo.read_filegdb", Effect::Cold, "Record", "Read Esri File Geodatabase .gdbtable and .gdbtablx binary tables.", &[
        p("path", ParamType::Str, true, "Path to .gdb directory"),
        p("table_name", ParamType::Str, true, "Table or feature class name"),
        p("bbox", ParamType::Record, false, "Spatial bounding box filter record"),
    ]),
    op("GisIo.read_shapefile", Effect::Cold, "Record", "Read ESRI Shapefile geometries and dBASE III+ attributes.", &[
        p("path", ParamType::Str, true, "File path to .shp file"),
        p("encoding", ParamType::Str, false, "Character encoding for DBF strings"),
    ]),
    op("GisIo.write_shapefile", Effect::External, "Record", "Write ESRI Shapefile (.shp, .shx, .dbf) feature dataset.", &[
        p("path", ParamType::Str, true, "Base file path for shapefile"),
        p("features", ParamType::Record, true, "Vector feature collection record"),
        p("crs", ParamType::Str, true, "Coordinate reference system"),
    ]),
    op("GisIo.read_geoparquet", Effect::Cold, "Record", "Read GeoParquet dataset with PAR1 magic, geo metadata, and WKB batch.", &[
        p("path", ParamType::Str, true, "File path to .parquet file"),
        p("columns", ParamType::List, false, "Column projection list"),
        p("bbox", ParamType::Record, false, "Spatial bounding box filter record"),
    ]),
    op("GisIo.write_geoparquet", Effect::External, "Record", "Write GeoParquet with PAR1 header, geo metadata, and WKB geometries.", &[
        p("path", ParamType::Str, true, "Target .parquet file path"),
        p("features", ParamType::Record, true, "Vector feature collection record"),
        p("crs", ParamType::Str, true, "Coordinate reference system"),
    ]),
    op("GisIo.read_geojson", Effect::Pure, "Record", "Parse RFC 7946 GeoJSON FeatureCollection or Geometry.", &[
        p("input", ParamType::Str, true, "GeoJSON string or file path"),
        p("crs", ParamType::Str, false, "Override coordinate reference system"),
    ]),
    op("GisIo.write_geojson", Effect::Pure, "Record", "Serialize vector features to RFC 7946 GeoJSON string.", &[
        p("features", ParamType::Record, true, "Feature collection record"),
        p("pretty", ParamType::Bool, false, "Format with indentation"),
        p("precision", ParamType::U64, false, "Coordinate decimal precision"),
    ]),
    op("GisIo.read_kml_gpx_gml", Effect::Cold, "Record", "Parse OGC XML vector formats (KML 2.2, GPX 1.1, and GML 3.2).", &[
        p("input", ParamType::Str, true, "XML content string or file path"),
        p("format", ParamType::Str, true, "Format selector (kml, gpx, gml)"),
    ]),
    op("GisIo.read_dxf_cad", Effect::Cold, "Record", "Read AutoCAD DXF CAD drawing vector entities and layers.", &[
        p("path", ParamType::Str, true, "File path to .dxf drawing"),
        p("layers", ParamType::List, false, "Filter list of layer names"),
    ]),
    op("GisIo.read_topojson_osm", Effect::Cold, "Record", "Parse TopoJSON arc topology or OpenStreetMap XML/PBF data.", &[
        p("input", ParamType::Str, true, "Data string or file path"),
        p("format", ParamType::Str, true, "Format selector (topojson, osm_pbf, osm_xml)"),
    ]),
    op("GisIo.read_pmtiles_mvt", Effect::Cold, "Record", "Read PMTiles v3 archive header and extract MVT vector tile.", &[
        p("path", ParamType::Str, true, "File path or URL to .pmtiles archive"),
        p("z", ParamType::U64, true, "Zoom level"),
        p("x", ParamType::U64, true, "Tile column X"),
        p("y", ParamType::U64, true, "Tile row Y"),
    ]),
    op("GisIo.encode_mvt_tile", Effect::Compute, "Record", "Encode vector feature geometries into Mapbox Vector Tile (MVT) protobuf.", &[
        p("layers", ParamType::Record, true, "Tile layer feature records"),
        p("extent", ParamType::U64, false, "Vector tile extent (default 4096)"),
        p("version", ParamType::U64, false, "MVT specification version"),
    ]),
];

/// Looks up a GIS I/O or LiDAR operation schema by its canonical invoke ID.
pub fn lookup_geo_gis_io_operation(invoke_id: &str) -> Option<&'static OperationSchema> {
    GEO_GIS_IO_OPERATIONS.iter().find(|op| op.invoke_id == invoke_id)
}

/// Returns the 20 `GisIo.*` GIS I/O and LiDAR capability schema entries for the catalog.
pub fn entries() -> Vec<SchemaEntry> {
    GEO_GIS_IO_OPERATIONS.iter().map(|op| {
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
    fn test_gis_io_operations_count_and_uniqueness() {
        assert_eq!(GEO_GIS_IO_OPERATIONS.len(), 20);
        assert_eq!(GEO_GIS_IO_INVOKE_IDS.len(), 20);
        for (i, op) in GEO_GIS_IO_OPERATIONS.iter().enumerate() {
            assert_eq!(op.invoke_id, GEO_GIS_IO_INVOKE_IDS[i]);
            assert!(op.invoke_id.starts_with("GisIo."));
            assert!(!op.returns.is_empty());
            assert!(!op.description.is_empty());
        }
        let mut ids: Vec<&str> = GEO_GIS_IO_INVOKE_IDS.to_vec();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 20, "All 20 GisIo invoke IDs must be unique");
    }

    #[test]
    fn test_entries_conversion_and_lookup() {
        assert!(lookup_geo_gis_io_operation("GisIo.read_lidar_copc").is_some());
        assert!(lookup_geo_gis_io_operation("GisIo.classify_lidar_csf").is_some());
        assert!(lookup_geo_gis_io_operation("GisIo.pack_lidar_m64").is_some());
        assert!(lookup_geo_gis_io_operation("GisIo.rasterize_dtm_dsm_chm").is_some());
        assert!(lookup_geo_gis_io_operation("GisIo.read_flatgeobuf").is_some());
        assert!(lookup_geo_gis_io_operation("GisIo.encode_mvt_tile").is_some());
        assert!(lookup_geo_gis_io_operation("GisIo.unknown_op").is_none());

        let converted = entries();
        assert_eq!(converted.len(), 20);
        for (i, entry) in converted.iter().enumerate() {
            assert_eq!(entry.id, GEO_GIS_IO_INVOKE_IDS[i]);
            assert_eq!(entry.honesty, HonestyLabel::Always);
        }
    }

    #[test]
    fn test_lidar_point_complex_decl_validation() {
        let valid = LidarPointComplexDecl::new(
            "snowy_lidar_cloud", 0x00, "EPSG:7855", 67_108_864, vec![2, 3, 4, 5], 1_250_000,
        );
        assert!(valid.validate().is_ok());
        assert_eq!(valid.m64_submagic, 0x00);
        assert_eq!(valid.byte_budget, 67_108_864);
        assert_eq!(valid.classification_codes.len(), 4);

        for code in 0..=18 {
            assert!(LidarPointComplexDecl::is_valid_asprs_code(code));
            assert!(LidarPointComplexDecl::asprs_code_name(code).is_some());
        }
        assert!(!LidarPointComplexDecl::is_valid_asprs_code(19));
        assert!(LidarPointComplexDecl::asprs_code_name(19).is_none());

        let bad_magic = LidarPointComplexDecl::new("test", 0x01, "EPSG:7855", 1024, vec![2], 100);
        assert!(bad_magic.validate().is_err());

        let zero_budget = LidarPointComplexDecl::new("test", 0x00, "EPSG:7855", 0, vec![2], 100);
        assert!(zero_budget.validate().is_err());

        let bad_code = LidarPointComplexDecl::new("test", 0x00, "EPSG:7855", 1024, vec![2, 19], 100);
        assert!(bad_code.validate().is_err());

        let empty_crs = LidarPointComplexDecl::new("test", 0x00, "", 1024, vec![2], 100);
        assert!(empty_crs.validate().is_err());
    }

    #[test]
    fn test_lidar_vector_ingest_fixture_exists() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path = std::path::Path::new(manifest_dir)
            .join("fixtures")
            .join("geography")
            .join("lidar_vector_ingest.vibe");
        assert!(
            fixture_path.exists(),
            "Fixture lidar_vector_ingest.vibe must exist at {:?}",
            fixture_path
        );
    }
}
