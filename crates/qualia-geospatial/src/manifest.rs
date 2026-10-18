//! Spatial Asset Provenance & Knowledge Graph Manifest (`eco:SpatialAsset`).
//!
//! Generates .q42-compatible semantic manifest assertions linking compiled .m64 binary
//! chunks to their authoritative open-data source, geodetic datum, license, and provenance DID.

use super::geotiff::CompressionCodec;

/// Semantic Spatial Asset Manifest Descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct SpatialAssetManifest {
    pub asset_id: String,
    pub source_title: String,
    pub authoritative_provider: String,
    pub license_iri: String,
    pub srid: u32,
    pub vertical_datum: u16,
    pub bbox_lat_lon: [f64; 4], // [min_lat, min_lon, max_lat, max_lon]
    pub hilbert_range: (u64, u64),
    pub compression_codec: CompressionCodec,
    pub multiresolution_levels: u8,
    pub dggs_cell_id: String,
    pub wfs_feature_ref: String,
    pub provenance_did_hash: u64,
    pub retrieval_epoch_seconds: u64,
}

impl SpatialAssetManifest {
    /// Snowy Mountains / Talbingo Geoscience Australia 30m DEM Manifest baseline.
    pub fn new_snowy_ga_dem(provenance_did_hash: u64) -> Self {
        Self {
            asset_id: "eco:SpatialAsset/AU-GA-DEM-Talbingo".to_string(),
            source_title: "Geoscience Australia 1-arc-second SRTM DEM-S".to_string(),
            authoritative_provider: "https://www.ga.gov.au/".to_string(),
            license_iri: "https://creativecommons.org/licenses/by/4.0/".to_string(),
            srid: 7844, // GDA2020
            vertical_datum: 5711, // AHD
            bbox_lat_lon: [-35.85, 148.20, -35.70, 148.40],
            hilbert_range: (0, 4096),
            compression_codec: CompressionCodec::Htj2kWavelet,
            multiresolution_levels: 5,
            dggs_cell_id: "R84400123".to_string(),
            wfs_feature_ref: "urn:gsnsw:geology:TalbingoSheet".to_string(),
            provenance_did_hash,
            retrieval_epoch_seconds: 1780000000,
        }
    }

    /// Formats the manifest as W3C N-Triples for .q42 knowledge graph ingestion with GeoSPARQL WKT.
    pub fn to_ntriples(&self) -> String {
        let codec_str = match self.compression_codec {
            CompressionCodec::UncompressedMmap => "UncompressedMmap",
            CompressionCodec::Lz4 => "Lz4",
            CompressionCodec::BaoTree => "BaoTree",
            CompressionCodec::Htj2kWavelet => "Htj2kWavelet",
            CompressionCodec::TerrainRgbPng => "TerrainRgbPng",
            CompressionCodec::TerrariumPng => "TerrariumPng",
            CompressionCodec::Av1IntraDrape => "Av1IntraDrape",
        };

        let min_lat = self.bbox_lat_lon[0];
        let min_lon = self.bbox_lat_lon[1];
        let max_lat = self.bbox_lat_lon[2];
        let max_lon = self.bbox_lat_lon[3];

        format!(
            "<{asset_id}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ecopunk.earth/ontology#SpatialAsset> .\n\
             <{asset_id}> <http://purl.org/dc/terms/title> \"{title}\" .\n\
             <{asset_id}> <http://purl.org/dc/terms/publisher> <{provider}> .\n\
             <{asset_id}> <http://purl.org/dc/terms/license> <{license}> .\n\
             <{asset_id}> <https://ecopunk.earth/ontology#srid> \"{srid}\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n\
             <{asset_id}> <https://ecopunk.earth/ontology#verticalDatum> \"{vdatum}\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n\
             <{asset_id}> <https://ecopunk.earth/ontology#compressionCodec> <https://ecopunk.earth/ontology#{codec_str}> .\n\
             <{asset_id}> <https://ecopunk.earth/ontology#multiresolutionLevels> \"{mips}\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n\
             <{asset_id}> <https://ecopunk.earth/ontology#inDggsCell> \"{dggs}\" .\n\
             <{asset_id}> <https://ecopunk.earth/ontology#wfsGeologyRef> <{wfs}>\n\
             <{asset_id}> <http://www.opengis.net/ont/geosparql#asWKT> \"POLYGON(({min_lon:.4} {min_lat:.4}, {max_lon:.4} {min_lat:.4}, {max_lon:.4} {max_lat:.4}, {min_lon:.4} {max_lat:.4}, {min_lon:.4} {min_lat:.4}))\"^^<http://www.opengis.net/ont/geosparql#wktLiteral> .\n\
             <{asset_id}> <https://ecopunk.earth/ontology#provenanceDidHash> \"{did_hash}\"^^<http://www.w3.org/2001/XMLSchema#unsignedLong> .\n",
            asset_id = self.asset_id,
            title = self.source_title,
            provider = self.authoritative_provider,
            license = self.license_iri,
            srid = self.srid,
            vdatum = self.vertical_datum,
            codec_str = codec_str,
            mips = self.multiresolution_levels,
            dggs = self.dggs_cell_id,
            wfs = self.wfs_feature_ref,
            min_lat = min_lat,
            min_lon = min_lon,
            max_lat = max_lat,
            max_lon = max_lon,
            did_hash = self.provenance_did_hash,
        )
    }
}
