//! Webizen Studio Library
//!
//! Re-exports render module for use by webizen-render and other crates.
//!
//! On wasm32, `dx build` links this rlib into the studio bin. Compiling the
//! full `render` tree (`spatial_bridge` Tauri invoke/listen) here AND again
//! under `main.rs` double-emits `__wbindgen_describe___wbg_invoke` / `listen`.
//! Native desktop keeps the full `render` module in the library. The wasm lib
//! only exposes `render::motion` / `render::motion_loop` so `theme_engine`
//! helpers compile; Tauri FFI stays bin-only.

pub mod canvas_graph;
pub mod canvas_model;
pub mod endpoints;
pub mod keep_volume;
pub mod lexicon_catalog;
pub mod theme_engine;

#[cfg(not(target_arch = "wasm32"))]
pub mod render;

/// Wasm rlib slice of `render`: spring physics only, no `spatial_bridge`.
#[cfg(target_arch = "wasm32")]
pub mod render {
    pub mod motion;
    pub mod motion_loop;
}

#[cfg(not(target_arch = "wasm32"))]
pub use render::render_stack_revision;
