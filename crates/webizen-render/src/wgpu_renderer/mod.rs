//! WGPU Renderer Module
//!
//! Provides WebGPU/wgpu-based rendering for both WASM and native targets.
//! Decomposed into modular capability probe, renderer state, pipeline builders,
//! readback, drawing, and headless scene preview submodules per `AGENTS.md §0-B`,
//! while re-exporting pure camera and pixel-receipt types from
//! `etalon_render::webizen_core`.

/// Deterministic ambient vegetation effects shared with the native renderer.
pub mod vegetation_effects {
    pub use etalon_render::geo_viewport::vegetation_effects::*;
}

mod draw;
mod pipelines;
mod probe;
mod readback;
mod renderer;
#[cfg(not(target_arch = "wasm32"))]
mod scene_preview;
#[cfg(test)]
mod tests;

pub use etalon_render::webizen_core::camera::{Camera, ScreenPoint, Vec3};
pub use etalon_render::webizen_core::pixel_receipt::{
    bounded_capability_text, bounded_offscreen_extent, fnv1a64, offscreen_rgba8_bytes,
    pixel_content_hash, pixel_receipt_from_rgba8, raw_readback_bytes, readback_layout,
    truncate_text, PixelFormat, PixelReadback, PixelReceipt, PixelReceiptStatus,
    WebGpuAdapterReport, WebGpuCapabilityReport, WebGpuFeatureReport, WebGpuLimitReport,
    WebGpuRuntimeReceipt, WebGpuStatus, COPY_BYTES_PER_ROW_ALIGNMENT, MAX_CAPABILITY_TEXT_BYTES,
    MAX_OFFSCREEN_DIMENSION, MAX_PIXEL_READBACK_BYTES,
};

pub use self::pipelines::AmbientConfig;
pub use self::probe::probe_webgpu_capabilities;
pub use self::renderer::WgpuRenderer;
#[cfg(not(target_arch = "wasm32"))]
pub use self::scene_preview::{
    render_preview_data_uri, render_preview_png, render_scene_data_uri, render_scene_png,
    render_scene_png_with_time, render_scene_png_with_time_and_telemetry,
};
