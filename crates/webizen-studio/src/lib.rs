//! Webizen Studio Library
//!
//! Re-exports render module for use by webizen-render and other crates.
//!
//! On wasm32, `dx build` links this rlib into the studio bin. Compiling `render`
//! (spatial_bridge Tauri invoke/listen) here AND again under `main.rs` double-emits
//! `__wbindgen_describe___wbg_invoke` / `listen`. Native desktop keeps `render` in
//! the library; wasm FFI lives only in the bin module tree.

pub mod canvas_graph;
pub mod canvas_model;
pub mod endpoints;
pub mod keep_volume;
pub mod lexicon_catalog;
pub mod theme_engine;

#[cfg(not(target_arch = "wasm32"))]
pub mod render;

#[cfg(not(target_arch = "wasm32"))]
pub use render::render_stack_revision;
