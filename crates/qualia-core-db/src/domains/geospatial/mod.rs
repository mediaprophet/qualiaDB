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
pub mod geodetic;
pub mod reference_frame;
pub mod render_surface;
pub mod scale_continuum;
pub mod spatial;
pub mod spatial_sync;
pub mod steward;
pub mod streaming;
pub mod terrain_pipeline;
pub mod triggers;
pub mod vector_scene;

pub use adapters::{AdapterRegistry, LayerFetchReport, LayerFetchStatus};
pub use environmental_raster::{
    compile_environmental_rasters_to_10d, derive_canopy_cover_grid, derive_slope_horn,
    sample_bilinear, EnvironmentalRasterLayer,
};
pub use steward::{validate_steward_unlock, StewardContract, StewardVerdict};
pub use triggers::{LocationTriggerEngine, LocationTriggerRegistry};

