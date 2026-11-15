# QualiaDB Geography, Geomorphology, GIS & Viewport Reference (`GEO-SUPERIORITY-v2.1`)

**Status:** Complete (`GEO-S1`–`GEO-S10` Delivered & Verified — `205/205` Unit Tests Passing, `0` Warnings)  
**Crates:** `qualia-core-db`, `vibe`, `webizen-render`, `poet-spatial-authoring`  
**Supersedes / Outclasses:** `EarthCraft-main` (`v0.2.0`), `nerthus-main` (`v2.3.1`), `RustGIS-main` (`v0.4.0`)

---

## 1. Overview & Architectural Invariants

The `GEO-SUPERIORITY-v2.1` implementation across `qualia-core-db::domains::geospatial`, `vibe::capability_schema`, `webizen-render::geo_viewport`, and `poet-spatial-authoring::geo_studio` establishes a single native Rust geospatial, geomorphological, remote-sensing, geoprocessing, cartographic, and planetary viewport stack:

1. **Tier-1 `HotZeroHeap` Enforcement (`AGENTS.md` §0-A)**:
   - Geodesic solvers (`Karney` WGS84, Krueger Transverse Mercator, `MGRS`/`UTM`/`UPS`/`Geohash`/`PlusCode`), `AusGeoid2020`, `A5` DGGS (`u64` cell indexing), `PackedHilbertRTree` spatial queries (`search_bbox`, `knn_search`), `VSOP87`/`ELP2000`/`SGP4` ephemeris, and `GeoSPARQL 1.1` (`DE-9IM`) + `Deontic` (`0x10..0x12`) norm evaluation allocate **zero heap memory** (`0` `Vec`/`String`/`Box` in hot paths).
2. **Tier-2 `ColdBounded` Budgeted Construction (`AGENTS.md` §0-A)**:
   - Terrain DAG evaluation (`terrain_dag_workspace`), Coupled 4D Geomorphology (`erosion_fixed824`, `hydro_stream_power`, `thermal_snow_glacier`), 26-format GIS/LiDAR/DataCube I/O (`gis_io`), Vector/Raster Geoprocessing (`gis_gp`), `Poet` CoW Tile Delta Studio (`geo_studio`), and `webizen-render` CDLOD/MVT/Curved-Label viewport (`geo_viewport`) require an explicit `byte_budget: usize` parameter and fail closed (`ByteBudgetExceeded`) before exceeding budget.
3. **Sub-500-Line Module Hygiene (`AGENTS.md` §0-B)**:
   - All **54 new `.rs` implementation files** are strictly `< 500` lines and organized into focused domain directories (`geodesy/`, `terrain/`, `gis_io/`, `gis_gp/`, `wgsl_kernels/`, `geo_studio/`, `geo_viewport/`, and `vibe/src/capability_schema/`).

---

## 2. Module & API Map

### 2.1 `qualia-core-db::domains::geospatial` (`crates/qualia-core-db/src/domains/geospatial/`)

| Sub-Domain | Module | Key Types & Entrypoints | Tier |
| :--- | :--- | :--- | :---: |
| **`geodesy`** | [`wgs84_karney.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/geodesy/wgs84_karney.rs) | `karney_inverse_wgs84`, `karney_direct_wgs84`, `geodesic_polygon_area_perimeter`, `geodetic_to_ecef`, `ecef_to_geodetic`, `ecef_to_enu` | Tier-1 |
| **`geodesy`** | [`coordinates_mgrs.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/geodesy/coordinates_mgrs.rs) | `latlon_to_utm`, `utm_to_latlon`, `latlon_to_ups`, `encode_mgrs`, `decode_mgrs`, `encode_geohash`, `encode_plus_code`, `encode_gars`, `encode_maidenhead` | Tier-1 |
| **`geodesy`** | [`crs_proj_wkt.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/geodesy/crs_proj_wkt.rs) | `lookup_epsg_def`, `transform_crs_point`, `helmert_7param`, `helmert_14param_epoch`, `ausgeoid2020_n_value`, `ellipsoidal_to_ahd`, `parse_wkt2_or_proj` | Tier-1 |
| **`geodesy`** | [`dggs_rtree_hot.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/geodesy/dggs_rtree_hot.rs) | `A5CellId`, `latlon_to_a5_cell`, `a5_cell_to_boundary`, `a5_k_ring`, `PackedHilbertRTree`, `search_bbox`, `knn_search`, `STRTreeIndex` | Tier-1 / Tier-2 |
| **`geodesy`** | [`ephemeris_orbit.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/geodesy/ephemeris_orbit.rs) | `solar_ephemeris_vsop87`, `lunar_ephemeris_elp2000`, `sgp4_propagate_tle`, `predict_satellite_swath_passes` | Tier-1 |
| **`terrain`** | [`noise_simd_lut.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/terrain/noise_simd_lut.rs) | `Simplex2D/3D/4D`, `Perlin3D`, `WorleyCellular`, `GaborAnisotropic`, `WaveletTileable`, `Fbm`, `RidgedMulti`, `Billow`, `HybridMulti`, `SwissTurbulence`, `JordanTurbulence` | Tier-1 |
| **`terrain`** | [`terrain_primitives.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/terrain/terrain_primitives.rs) | 17 Landform generators (`VoronoiPeak`, `MountainRange`, `CanyonNetwork`, `VolcanicCaldera`, `KarstTowerField`, `AlluvialFan`, `BadlandsGully`, `FjordTrough`, `GlacialCirque`, etc.) | Tier-2 |
| **`terrain`** | [`terrain_adjust_filter.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/terrain/terrain_adjust_filter.rs) | 11 Adjustment/Filter/Warp nodes (`AnisotropicDomainWarp`, `VortexSwirl`, `GeologicalFold`, `ApexPeakSharpen`, `EqualizeHistogram`, `Terrace`, `Plateau`, etc.) | Tier-2 |
| **`terrain`** | [`terrain_selectors_combine.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/terrain/terrain_selectors_combine.rs) | 10 Selector & Combine nodes (`SelectHeight`, `SelectSlope`, `SelectCurvature`, `SelectFlowAccum`, `SelectConvexity`, `CombineBlend`, `CombineMaxSmooth`, etc.) | Tier-2 |
| **`terrain`** | [`terrain_dag_workspace.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/terrain/terrain_dag_workspace.rs) | `TerrainDagGraph`, `TileHaloConfig`, `evaluate_terrain_dag_tile` (`BLAKE3` node digest, seamless tile-halo guttering) | Tier-2 |
| **`terrain`** | [`erosion_fixed824.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/terrain/erosion_fixed824.rs) | `Fixed824`, `step_hydraulic_pipe_fixed824`, `step_wind_aeolian_saltation` (bit-identical `Q8.24` signed fixed-point flux) | Tier-2 |
| **`terrain`** | [`hydro_stream_power.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/terrain/hydro_stream_power.rs) | `priority_flood_fill_depressions`, `compute_d8_dinf_flow_accumulation`, `solve_implicit_stream_power_spl`, `extract_strahler_shreve_network` | Tier-2 |
| **`terrain`** | [`thermal_snow_glacier.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/terrain/thermal_snow_glacier.rs) | `step_nonlinear_hillslope_creep`, `step_debris_flow_runout`, `compute_orographic_snow_avalanche`, `step_shallow_ice_glacier_sia` | Tier-2 |
| **`terrain`** | [`m64_strata_constituents.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/terrain/m64_strata_constituents.rs) | `M64StrataColumn` (64B `.m64` `0x02`, 6 layers), `SoilConstituentFractions` (`Clay`/`Silt`/`Sand`/`Humus`), `SENTINEL2_BIOME_LUT_323` (`B02/B03/B04/B08`) | Tier-1 / Tier-2 |
| **`terrain`** | [`geomorph_qa_report.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/terrain/geomorph_qa_report.rs) | `compute_geomorph_qa_metrics` (Hypsometric integral $HI$, Strahler $R_b$, Hack's Law $h$, slope-area $\theta$), `encode_geomorph_qa_quins` | Tier-1 / Tier-2 |
| **`gis_io`** | [`lidar_copc_m64.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/gis_io/lidar_copc_m64.rs) | `LasLazCopcReader`, `M64OctreeCell` (`.m64` `0x00`), `csf_ground_classify`, `lidar_to_dsm_dtm_chm` | Tier-2 |
| **`gis_io`** | [`filegdb_gpkg_fgb.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/gis_io/filegdb_gpkg_fgb.rs) | `OpenFileGdbReader`, `GeoPackageCodec`, `FlatGeobufCodec` | Tier-2 |
| **`gis_io`** | [`shp_parquet_geojson.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/gis_io/shp_parquet_geojson.rs) | `ShapefileCodec`, `GeoParquetCodec`, `GeoJsonCodec`, `TopoJsonCodec` | Tier-2 |
| **`gis_io`** | [`kml_gpx_gml_dxf_osm.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/gis_io/kml_gpx_gml_dxf_osm.rs) | `KmlKmzCodec`, `GpxCodec`, `Gml32Codec`, `DxfCadCodec`, `OsmPbfReader` | Tier-2 |
| **`gis_io`** | [`pmtiles_mvt_io.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/gis_io/pmtiles_mvt_io.rs) | `PmTilesV3Archive`, `MvtTileCodec` (`encode_mvt_layer`, `decode_mvt_layer`) | Tier-1 / Tier-2 |
| **`gis_io`** | [`ogc_feature_services.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/gis_io/ogc_feature_services.rs) | `OgcApiFeaturesCodec`, `OgcApiTilesCodec`, `OgcApiProcessesCodec`, `OgcApiRecordsCodec`, `Wfs20Codec`, `ArcGisRestFeatureServiceCodec` | Tier-2 |
| **`gis_io`** | [`ogc_sensor_stac.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/gis_io/ogc_sensor_stac.rs) | `OgcSos20Codec`, `SensorThingsCodec`, `Stac10CatalogCodec`, `observation_to_quin` | Tier-1 / Tier-2 |
| **`gis_io`** | [`cog_wcs_wms_wmts.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/gis_io/cog_wcs_wms_wmts.rs) | `CloudOptimizedGeoTiffCodec`, `Wms130RequestBuilder`, `WmtsTileMatrixCodec`, `Wcs201CoverageCodec` | Tier-2 |
| **`gis_io`** | [`datacube_zarr_netcdf_grib.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/gis_io/datacube_zarr_netcdf_grib.rs) | `ZarrV3CubeCodec`, `NetCdf4CfCodec`, `Grib2SectionDecoder`, `datacube_slice_to_10d_tensor` | Tier-2 |
| **`gis_io`** | [`geosparql_deontic_bridge.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/gis_io/geosparql_deontic_bridge.rs) | `De9imMatrix`, `evaluate_geosparql_predicate`, `evaluate_spatial_deontic_norms` (`0x10..0x12`) | Tier-1 |
| **`gis_gp`** | [`gp_overlay_clip.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/gis_gp/gp_overlay_clip.rs) | `PolygonOverlayOp` (`Intersection`, `Union`, `Difference`, `SymmetricDifference`, `Clip`, `Erase`, `Identity`, `Update`, `Split`), `polygon_boolean_overlay` | Tier-2 |
| **`gis_gp`** | [`gp_buffer_dissolve.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/gis_gp/gp_buffer_dissolve.rs) | `buffer_geometry`, `dissolve_polygons_by_key`, `polygon_to_lines`, `lines_to_polygons`, `eliminate_sliver_polygons` | Tier-2 |
| **`gis_gp`** | [`gp_simplify_hull_tess.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/gis_gp/gp_simplify_hull_tess.rs) | `simplify_douglas_peucker`, `simplify_visvalingam_whyatt`, `smooth_chaikin`, `convex_hull_monotone`, `concave_hull_knn`, `polylabel_inaccessibility`, `constrained_delaunay_voronoi` | Tier-2 |
| **`gis_gp`** | [`gp_spatial_join_kde.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/gis_gp/gp_spatial_join_kde.rs) | `spatial_join_features`, `point_in_polygon_batch`, `nearest_neighbor_distance_table`, `kde_density_raster`, `cluster_dbscan_spatial`, `cluster_kmeans_spatial` | Tier-2 |
| **`gis_gp`** | [`gp_kriging_spatial_stats.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/gis_gp/gp_kriging_spatial_stats.rs) | `fit_empirical_variogram`, `kriging_interpolate_grid`, `idw_interpolate_grid`, `sibson_natural_neighbor_grid`, `rbf_thin_plate_spline_grid`, `global_morans_i`, `anselin_local_moran_lisa`, `getis_ord_gi_star`, `ripleys_k_l_function` | Tier-2 |
| **`gis_gp`** | [`gp_viewshed_solar_morph.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/gis_gp/gp_viewshed_solar_morph.rs) | `compute_horizon_viewshed`, `compute_cumulative_viewshed`, `compute_surface_derivatives_horn`, `compute_tpi_tri_vrm_roughness`, `compute_hemispherical_insolation`, `classify_geomorphons_10class` | Tier-2 |
| **`gis_gp`** | [`gp_map_algebra_zonal.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/gis_gp/gp_map_algebra_zonal.rs) | `evaluate_map_algebra_bytecode`, `compute_spectral_index`, `compute_zonal_statistics`, `apply_focal_filter`, `compute_haralick_glcm_textures`, `raster_to_contour_isobands`, `rasterize_polygon_scanline` | Tier-2 |
| **`gis_gp`** | [`gp_hydro_cost_watershed.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/gis_gp/gp_hydro_cost_watershed.rs) | `compute_twi_spi_sti_indices`, `compute_rusle_soil_loss`, `compute_anisotropic_cost_distance`, `trace_least_cost_path`, `delineate_watershed_basins` | Tier-2 |
| **`gis_gp`** | [`gp_network_topology.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/gis_gp/gp_network_topology.rs) | `shortest_path_astar_turn_restricted`, `compute_service_area_isochrones`, `solve_tsp_2opt`, `minimum_spanning_tree_kruskal`, `max_flow_min_cut_dinic`, `validate_planar_topology` | Tier-2 |
| **`wgsl_kernels`** | [`geo_erosion_hydro_kernels.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/wgsl_kernels/geo_erosion_hydro_kernels.rs) | Certified WGSL + CPU oracles: `WGSL_PIPE_EROSION_Q824`, `WGSL_THERMAL_TALUS_SLIP`, `WGSL_JFA_FLOW_ROUTING` | Tier-1 / Tier-2 |
| **`wgsl_kernels`** | [`geo_noise_filter_kernels.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/wgsl_kernels/geo_noise_filter_kernels.rs) | Certified WGSL + CPU oracles: `WGSL_NOISE_FBM_RIDGED`, `WGSL_DOMAIN_WARP_BILATERAL`, `WGSL_BIOME_SPLATTING_323` | Tier-1 / Tier-2 |
| **`wgsl_kernels`** | [`geo_viewshed_algebra_kernels.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/geospatial/wgsl_kernels/geo_viewshed_algebra_kernels.rs) | Certified WGSL + CPU oracles: `WGSL_RADIAL_HORIZON_VIEWSHED`, `WGSL_GEOMORPHON_8RAY`, `compile_vibe_cell_expr_to_wgsl` | Tier-1 / Tier-2 |

---

### 2.2 `webizen-render::geo_viewport` (`crates/webizen-render/src/geo_viewport/`)

- [`geo_tessellation_capped.rs`](file:///C:/github/qualiaDB/crates/webizen-render/src/geo_viewport/geo_tessellation_capped.rs) — `tessellate_polyline_capped_miter`, `tessellate_polygon_earclip`, `select_cdlod_quadtree_nodes`, `build_cdlod_patch_with_skirts`.
- [`mvt_pmtiles_curved_labels.rs`](file:///C:/github/qualiaDB/crates/webizen-render/src/geo_viewport/mvt_pmtiles_curved_labels.rs) — `evaluate_mvt_feature_style`, `layout_curved_bezier_glyphs`, `place_point_labels_spiral`, `LabelCollisionGrid`.
- [`sky_atmosphere_limb.rs`](file:///C:/github/qualiaDB/crates/webizen-render/src/geo_viewport/sky_atmosphere_limb.rs) — `evaluate_sky_atmosphere_limb`, `evaluate_gerstner_water_surface`, `extract_realtime_contour_segments`, `raycast_pick_terrain_and_features`.
