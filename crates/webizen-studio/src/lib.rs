//! Webizen Studio Library
//!
//! Re-exports render module for use by webizen-render and other crates.
//!
//! `dx build --web` links this rlib into the studio bin. Keep a single `render`
//! module tree here (including spatial_bridge Tauri FFI). The bin must
//! `pub use webizen_studio::render` instead of `mod render`, or
//! `__wbindgen_describe___wbg_invoke` / `listen` double-emit at link.

pub mod canvas_graph;
pub mod canvas_model;
pub mod endpoints;
pub mod keep_volume;
pub mod lexicon_catalog;
pub mod render;
pub mod theme_engine;

pub use render::render_stack_revision;
