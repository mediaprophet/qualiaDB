//! Webizen Studio Library
//!
//! Re-exports render module for use by webizen-render and other crates.
//!
//! `dx build --web` links this rlib into the studio bin. Keep a single `render`
//! module tree here. Tauri `invoke` / `listen` wasm-bindgen imports live only in
//! [`tauri_ffi`]. The bin must `pub use webizen_studio::render` instead of
//! `mod render`, and must not declare its own Tauri externs, or
//! `__wbindgen_describe___wbg_invoke` / `listen` double-emit at link.

pub mod canvas_graph;
pub mod canvas_model;
pub mod endpoints;
pub mod keep_volume;
pub mod lexicon_catalog;
pub mod render;
pub mod tauri_ffi;
pub mod theme_engine;

pub use render::render_stack_revision;
