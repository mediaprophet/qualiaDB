// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) RustGIS contributors (adapted under MIT License)
//! Rounds 6 & 7 (`GEO-S6` & `GEO-S7`) Complete Vector Geoprocessing, Spatial Statistics,
//! Kriging, Viewshed, Geomorphons, Map Algebra, Hydrology & Network Topology Suite.

pub mod gp_buffer_dissolve;
pub mod gp_hydro_cost_watershed;
pub mod gp_kriging_spatial_stats;
pub mod gp_map_algebra_zonal;
pub mod gp_network_topology;
pub mod gp_overlay_clip;
pub mod gp_simplify_hull_tess;
pub mod gp_spatial_join_kde;
pub mod gp_viewshed_solar_morph;

pub use gp_buffer_dissolve::{
    gp_buffer_polygon, gp_buffer_polyline, gp_dissolve_by_key, gp_multi_ring_buffer,
    gp_polygons_to_unique_lines, gp_snap_and_eliminate_slivers, ring_perimeter, signed_ring_area,
    BufferJoinStyle, GpBufferError,
};
pub use gp_hydro_cost_watershed::{
    gp_anisotropic_cost_distance_and_backlink, gp_d8_flow_dir_and_watershed_basins,
    gp_rusle_soil_loss_into, gp_time_of_concentration_isochrones, gp_trace_least_cost_path,
    gp_twi_spi_sti_into, GpHydroCostError,
};
pub use gp_kriging_spatial_stats::{
    gp_empirical_variogram, gp_fit_variogram_wls, gp_global_morans_i_and_gearys_c,
    gp_gwr_local_regression, gp_idw_interpolate_into, gp_local_lisa_and_getis_ord_gi_star,
    gp_natural_neighbor_sibson, gp_ordinary_kriging_grid, gp_ripleys_k_and_l_into,
    EmpiricalVariogramBin, GpStatsError, MoranGlobalResult, VariogramModel,
};
pub use gp_map_algebra_zonal::{
    gp_ahp_weights_and_consistency, gp_eval_local_map_algebra_into, gp_focal_filter_into,
    gp_marching_squares_contours, gp_rasterize_polygon_scanline_into, gp_reclassify_ranges_into,
    gp_resample_grid_into, gp_zonal_statistics, AhpResult, FocalStatKind, GpAlgebraError,
    MapAlgebraOp, ResampleKernel, ZonalRecord,
};
pub use gp_network_topology::{
    gp_centrality_closeness_betweenness, gp_max_flow_edmonds_karp,
    gp_minimum_spanning_tree_kruskal, gp_service_area_isochrone_nodes,
    gp_shortest_path_astar_turn_grade, gp_tsp_tour_2opt, GpNetworkError, NetworkEdge, NetworkNode,
    TurnRestriction,
};
pub use gp_overlay_clip::{
    gp_clip_polygon, gp_clip_polyline, gp_difference, gp_identity_and_update_overlay,
    gp_intersection, gp_sym_difference, gp_union, point_in_ring, ring_area, ring_signed_area,
    GpOverlayError, OverlayFeature, Ring2D,
};
pub use gp_simplify_hull_tess::{
    gp_centroid_and_polylabel, gp_concave_hull_alpha, gp_convex_hull, gp_delaunay_triangulate,
    gp_generate_tessellation_grid, gp_simplify_douglas_peucker, gp_simplify_visvalingam_whyatt,
    gp_smooth_chaikin, gp_voronoi_diagram_bounded, polygon_area, GpGeomError, TessellationShape,
};
pub use gp_spatial_join_kde::{
    gp_cluster_dbscan, gp_cluster_kmeans_pp, gp_discrete_frechet_distance, gp_hausdorff_distance,
    gp_kernel_density_quartic_into, gp_spatial_join_points_to_polygons, GpSpatialError,
    SpatialJoinPredicate, SpatialJoinSummary,
};
pub use gp_viewshed_solar_morph::{
    classify_geomorphon_pair, gp_classify_geomorphons_into, gp_cumulative_viewshed,
    gp_horn_slope_aspect_curvature_into, gp_line_of_sight_profile, gp_tpi_tri_vrm_roughness_into,
    gp_viewshed_radial_sweep, gp_wind_exposition_index_into, GeomorphonClass, GpMorphError,
    ObserverSpec, EARTH_RADIUS_M,
};
