//! Webizen Studio Library
//!
//! Re-exports render module for use by webizen-render and other crates.
//!
//! `dx build --web` links this rlib into the studio bin. Shared modules (including `render`) live here once. Tauri `invoke`/`listen`
//! wasm-bindgen imports live only in `tauri_ffi` — never re-declare them in the bin.

pub mod canvas_graph;
pub mod canvas_model;
pub mod endpoints;
pub mod keep_volume;
pub mod lexicon_catalog;
#[cfg(target_arch = "wasm32")]
pub mod tauri_ffi;
pub mod render;
pub mod theme_engine;

pub use render::render_stack_revision;
