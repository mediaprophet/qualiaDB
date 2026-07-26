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

pub use adapters::{AdapterRegistry, LayerFetchReport, LayerFetchStatus};
pub use steward::{validate_steward_unlock, StewardContract, StewardVerdict};
pub use triggers::{LocationTriggerEngine, LocationTriggerRegistry};
