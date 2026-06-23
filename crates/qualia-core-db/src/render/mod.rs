//! Renderer / engine — the manifold projection + WebGPU viewport.
//!
//! Phase 0.2a (`RENDERER_IMPLEMENTATION_PLAN.md`): the formerly-flat `portal_*` modules are
//! consolidated under one `render` tree. The platform-agnostic pieces (camera, PGA oracle,
//! projection, telemetry, standpoint, spectral, acoustic, control, contract, asset import) compile
//! everywhere; the WebGPU surface (`gpu`, `portal`, `portal_wasm`) is gated to the wasm `portal`
//! build, as before. Phase 0.2b will lift this tree into a standalone `qualia-render` crate.

pub mod telemetry;
pub mod standpoint;
pub mod camera;
pub mod navigation;
pub mod pga;
/// Unified manifold projection — one `project()`, many views (Phase 1.4).
pub mod projection;
pub mod contract;
pub mod spectral;
pub mod acoustic;
pub mod control;
/// Asset import: OBJ / STL / GLB → `Mesh` + semantic NQuins (Phase 1.3).
pub mod assets;

/// WebGPU renderer (`PortalGpu`) — depth, bloom, tensor-node projection, mesh surfaces.
#[cfg(all(target_arch = "wasm32", feature = "portal"))]
pub mod gpu;
/// The `#[wasm_bindgen]` portal facade (`QualiaPortal`) driving the browser viewport.
#[cfg(all(target_arch = "wasm32", feature = "portal"))]
pub mod portal;
#[cfg(all(target_arch = "wasm32", feature = "portal"))]
pub mod portal_wasm;
