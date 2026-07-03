//! Renderer / engine — the manifold projection + WebGPU viewport.
//!
//! Phase 0.2a (`RENDERER_IMPLEMENTATION_PLAN.md`): the formerly-flat `portal_*` modules are
//! consolidated under one `render` tree. The platform-agnostic pieces (camera, PGA oracle,
//! projection, telemetry, standpoint, spectral, acoustic, control, contract, asset import) compile
//! everywhere. The WebGPU renderer core (`gpu`) compiles wherever `gpu-runtime` is enabled; only
//! the browser facade (`portal`, `portal_wasm`) remains gated to wasm + `portal`.

pub mod acoustic;
/// Asset import: OBJ / STL / GLB → `Mesh` + semantic NQuins (Phase 1.3).
pub mod assets;
/// Native quantized mesh geometry buffer (the geometry half of a Q42 mesh asset; fills the
/// Phase-1.3-vs-Phase-6 gap where GLB→native emitted only semantic metadata).
pub mod mesh_asset;
/// Authoring vocabulary + render planner (Phase 5): a qapp declares 3D + 2D views over one
/// manifold; the planner enforces attestation gates, rights-bounded contexts, and budget-driven
/// 3D→2D degradation before drawing. Gated like `place_time` (needs `crate::modalities`).
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod authoring;
pub mod camera;
pub mod contract;
pub mod control;
/// Model-as-substrate (Phase 6, §F): one buffer holds a renderable manifold AND the transcoded
/// P64 weights; the renderer projects the manifold while the weights are co-resident. Gated to
/// where `crate::p64_weight` (the transcoder) compiles.
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod model_substrate;
pub mod navigation;
pub mod pga;
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
/// Unified manifold projection — one `project()`, many views (Phase 1.4).
pub mod projection;
/// Sense path — the input twin (Phase 4): microphone PCM → forward DSP → the `∫Ψ > τ → Fact`
/// bridge, every capture under the deontic/standpoint consent gate (surveillance-refusal default).
/// Gated like `place_time` (needs `crate::modalities`).
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod sense;
pub mod spectral;
pub mod standpoint;
pub mod telemetry;

/// WebGPU renderer (`PortalGpu`) — depth, bloom, tensor-node projection, mesh surfaces.
#[cfg(feature = "gpu-runtime")]
pub mod gpu;
/// The `#[wasm_bindgen]` portal facade (`QualiaPortal`) driving the browser viewport.
#[cfg(all(target_arch = "wasm32", feature = "portal"))]
pub mod portal;
#[cfg(all(target_arch = "wasm32", feature = "portal"))]
pub mod portal_wasm;
