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
/// Physics of artefacts — bbox admission, kinematic joints, material/mass/momentum (Phase 2).
pub mod physics;
/// Place / space / time binding — an artefact NQuin queried by the spatio-temporal AND deontic
/// modalities over one shared identity (Phase 3); delegates to the inherited modality stack.
/// Gated to exactly the configs where `crate::modalities` (the values/logic layer it builds on)
/// is compiled — native always; on wasm only with the logic/scientific/full feature sets, not the
/// minimal `portal` bundle.
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod place_time;
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
