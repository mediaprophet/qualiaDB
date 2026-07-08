//! Chora — the spatio-temporal commons canvas inside **WellFair** (Peace Infrastructure).
//!
//! WellFair is the rights-grounded personal vault and host shell; Chora is one area within it
//! (not a replacement for WellFair). Canvas state is reached via [`crate::wellfair::api::WebizenHostApi`].

pub mod api;
pub mod flagship_worlds;
pub mod layers;
pub mod asset_pipeline;

pub use crate::canvas_state;
pub use crate::canvas_store;
pub use crate::canvas_world;

pub use flagship_worlds::{
    all_flagship_worlds, biosphere_world, council_world, glam_world, history_world, sdg_world,
    seed_all_flagships,
};