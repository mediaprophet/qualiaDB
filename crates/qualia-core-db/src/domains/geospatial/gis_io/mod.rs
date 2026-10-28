// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) RustGIS contributors (adapted under MIT License)
//! Round 4 (`GEO-S4`) & Round 5 (`GEO-S5`) Geospatial Vector, CAD, LiDAR `.m64` `0x00`,
//! `PMTiles`/`MVT`, 10 OGC/STAC/ArcGIS Protocols, Zarr/NetCDF/GRIB2 `.10d` Datacubes,
//! and Zero-Heap OGC GeoSPARQL 1.1 + 48B `NQuin` Deontic Spatial Norm Bridge.

pub mod cog_wcs_wms_wmts;
pub mod datacube_zarr_netcdf_grib;
pub mod filegdb_gpkg_fgb;
pub mod geosparql_deontic_bridge;
pub mod kml_gpx_gml_dxf_osm;
pub mod lidar_copc_m64;
pub mod ogc_feature_services;
pub mod ogc_sensor_stac;
pub mod pmtiles_mvt_io;
pub mod shp_parquet_geojson;

pub use cog_wcs_wms_wmts::{
    build_wcs20_get_coverage_url, build_wms130_get_map_url, build_wmts10_get_tile_url,
    lonlat_to_xyz_tile, parse_cog_ifd, plan_cog_http_ranges, xyz_tile_to_lonlat_bbox,
    xyz_y_to_tms_y, CogIfdLayout, HttpByteRange, RasterServiceError,
};
pub use datacube_zarr_netcdf_grib::{
    compile_datacube_slices_to_tensor10d, decode_zarr_f32_chunk_into, format_zarr_chunk_key,
    parse_grib2_sections, parse_netcdf_classic_header, parse_zarr_array_meta,
    read_netcdf_f32_slab_into, unpack_grib2_simple_packing_into, DatacubeError,
    Grib2RecordSummary, NetCdfHeaderSummary, ZarrArrayMeta,
};
pub use filegdb_gpkg_fgb::{
    decode_fgb_feature_at, decode_gpkg_geometry, encode_fgb_container,
    encode_filegdb_point_or_polygon_row, encode_gpkg_geometry, parse_filegdb_point_or_polygon_row,
    parse_filegdb_tablx_offset, parse_gpkg_geom_header, query_fgb_bbox_zero_copy,
    validate_sqlite_gpkg_file_header, BBox2D, FgbIndexNode, GisBinaryError, GpkgGeomHeader,
    SimpleGeom2D, FGB_MAGIC, GPKG_GEOM_MAGIC, SQLITE3_HEADER_MAGIC,
};
pub use geosparql_deontic_bridge::{
    compute_polygon_de9im, evaluate_spatial_deontic_covenants, geof_distance_polygons,
    geof_sf_contains, geof_sf_disjoint, geof_sf_equals, geof_sf_intersects, geof_sf_overlaps,
    geof_sf_touches, geof_sf_within, mint_spatial_deontic_quin, De9imMatrix,
    SpatialCovenantZone, SpatialNormVerdict, SpatialQuin48, DEFEATER_BIT, OP_FORBID, OP_OBLIGATE,
    OP_PERMIT,
};
pub use kml_gpx_gml_dxf_osm::{
    decode_topojson_polygon_arcs, export_dxf_entities, export_gml_feature_collection,
    export_gpx_track, export_kml_document, parse_dxf_entities, parse_gml_poslists,
    parse_gpx_points, parse_kml_placemarks, parse_osm_xml_ways, ExchangeFeature, ExchangeGeomKind,
    ExchangeIoError,
};
pub use lidar_copc_m64::{
    classify_ground_and_canopy_csf, decode_las_point_format0, ingest_las_point_batch,
    parse_copc_hierarchy_entry, parse_las_header, rasterize_dtm_dsm_chm, AsprsClass,
    CopcOctreeVoxelKey, LasHeaderSummary, LidarError, M64LidarPoint, M64_LIDAR_SUBMAGIC,
};
pub use ogc_feature_services::{
    build_arcgis_feature_query_url, build_ogc_features_items_url, build_wfs20_get_feature_url,
    format_cql2_text, parse_arcgis_json_features, parse_csw30_records_xml,
    parse_ogc_features_response, parse_wfs20_member_response, Cql2Expr, CswMetadataRecord,
    OgcServiceError, ServiceFeature,
};
pub use ogc_sensor_stac::{
    build_ogc_process_execute_json, build_sensorthings_observations_url,
    build_stac_search_payload, parse_ogc_process_job_status,
    parse_sensorthings_observations_json, parse_sos20_om_observations_xml, parse_stac_item_json,
    select_best_cog_asset, OgcProcessStatus, SensorObservation, SensorStacError, StacAsset,
    StacItem,
};
pub use pmtiles_mvt_io::{
    decode_mvt_geometry_commands, decode_mvt_layer_pbf, decode_pmtiles_header_v3,
    encode_mvt_geometry_commands, encode_mvt_layer_pbf, encode_pmtiles_header_v3,
    lookup_pmtiles_entry, pmtiles_hilbert_id_to_zxy, zigzag_decode, zigzag_encode,
    zxy_to_pmtiles_hilbert_id, MvtLayer, PmTilesDirEntry, PmTilesHeaderV3, TileIoError,
    MVT_DEFAULT_EXTENT, PMTILES_V3_MAGIC,
};
pub use shp_parquet_geojson::{
    decode_dbf_attributes, decode_ewkb, decode_geoparquet_wkb_batch, decode_geojson_geometries,
    decode_shapefile_shp, encode_dbf_attributes, encode_ewkb, encode_geoparquet_wkb_batch,
    encode_geojson_feature_collection, encode_shapefile_shp_shx, parse_csv_lonlat_or_wkt, GisGeom,
    VectorIoError, EWKB_SRID_FLAG, EWKB_Z_FLAG, SHP_FILE_CODE,
};
