// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 QualiaDB & Webizen Contributors (adapted from EarthCraft v0.2.0, Nerthus v2.3.1, and RustGIS v0.4.0)
//! Round 10 (`GEO-S10`) `webizen-render` Planetary-to-Millimeter Geospatial Viewport:
//! Capped-Angle Polyline & Earcut Tessellation, CDLOD Quadtree & Terrain Skirts,
//! MVT Style & Curved River/Road Labels, Screen-Space Collision Solver,
//! Rayleigh/Mie Spherical Atmosphere Limb, Gerstner Water Waves, Hypsometric Contours & GPU Picking.

pub mod geo_tessellation_capped;
pub mod mvt_pmtiles_curved_labels;
pub mod sky_atmosphere_limb;

pub use geo_tessellation_capped::{
    build_terrain_patch_with_skirts, select_cdlod_quadtree_nodes, tessellate_polygon_earcut,
    tessellate_polyline_stroke_capped, CdlodNodeSpec, GeoRenderVertex, GeoTessError,
    StrokeJoinCapStyle,
};
pub use mvt_pmtiles_curved_labels::{
    evaluate_style_zoom_interpolated, layout_glyphs_along_centerline,
    solve_screen_space_label_collisions, CurvedGlyphQuad, GeoLabelError, LabelCandidate,
    PlacedLabel, RTreeNode, RTreeScreenIndex, StyleZoomStop, VectorTileLayerStyle,
};
pub use sky_atmosphere_limb::{
    evaluate_sky_limb_and_aerial_perspective, evaluate_water_wave_and_foam,
    resolve_gpu_picking_id_aperture, shade_hypsometric_and_contours_into, AtmosphereParams,
    GeoAtmosphereError, PickingHitRecord, WaterSurfaceSample,
};
