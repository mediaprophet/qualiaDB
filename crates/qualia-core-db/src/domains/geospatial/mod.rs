//! Geospatial domain engines
//!
//! This module provides geospatial computation capabilities for QualiaDB,
//! including quadtree indexing, H3 context embedding, and spatial analysis
//! operations for location-based reasoning.

pub mod adapters;
pub mod ar_anchor;
pub mod canvas_query;
pub mod canvas_rights;
pub mod dem;
pub mod environmental_raster;
pub mod geodesy;
pub mod geodetic;
pub mod gis_gp;
pub mod gis_io;
pub mod reference_frame;
pub mod render_surface;
pub mod scale_continuum;
pub mod spatial;
pub mod spatial_sync;
pub mod steward;
pub mod streaming;
pub mod terrain;
pub mod terrain_pipeline;
pub mod triggers;
pub mod vector_scene;
pub mod wgsl_kernels;

pub use adapters::{AdapterRegistry, LayerFetchReport, LayerFetchStatus};
pub use environmental_raster::{
    compile_environmental_rasters_to_10d, decode_geotiff_raster_into, derive_canopy_cover_grid,
    derive_slope_horn, sample_bilinear, EnvironmentalRasterLayer, GeoTiffError, GeoTiffRasterInfo,
};
pub use steward::{validate_steward_unlock, StewardContract, StewardVerdict};
pub use triggers::{LocationTriggerEngine, LocationTriggerRegistry};
