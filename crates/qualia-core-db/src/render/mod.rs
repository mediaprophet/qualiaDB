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
/// Validate-before-render barrier needs full `modalities::logic::geometry_asset_shacl`
/// (not modalities_lite / wasm-ontology-only).
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod barrier;
pub mod camera;
pub mod derivation;
/// Compile a `Mesh` into a sealed `.10d` container (the dense compiled-geometry sidecar)
/// and read it back — the "mesh → `.10d`" step of the 3-D-anatomy asset pipeline.
pub mod compile_10d;
/// Shared metadata schema for a `.qualia` anatomy asset pack (per-organ system /
/// position / neutral colour). Platform-agnostic (native + WASM consumers).
pub mod anatomy_pack;
pub mod contract;
pub mod control;
/// P7.2 — Gamut / object-colour solid + closest-point gamut mapping.
pub mod gamut;
/// P7.4 — GPU colour-projection / gamut batch kernel + CPU oracle.
pub mod gpu_colour_kernel;
/// P7.1 — Metamers as the affine fibre of the colour-matching projection.
pub mod metamer;
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
/// P7.3 — σ spectral blend as interpolation on the spectral manifold.
pub mod spectral_blend;
#[cfg(test)]
pub mod spectral_harness;
/// P7.0 — Spectral-space kernel: SPD/CMF POD types + CIE linear-projection.
pub mod spectral_kernel;
/// P7.7 — Unified spectral-operator API surface.
pub mod spectral_operator;
/// P7.8 — golden-oracle + CPU/GPU differential + determinism harness.
pub mod spectral_oracle;
pub mod standpoint;
pub mod telemetry;

/// LOD chain (P5.8): author mesh → decimate N LODs → serialize to `.10d` →
/// renderer parses each level → `plan_view` selects the expected LOD. Gated
/// like the scientific geometry stack (needs `crate::specialized_libs`).
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod lod_chain;

/// WebGPU renderer (`PortalGpu`) — depth, bloom, tensor-node projection, mesh surfaces.
#[cfg(feature = "gpu-runtime")]
pub mod gpu;
/// The `#[wasm_bindgen]` portal facade (`QualiaPortal`) driving the browser viewport.
#[cfg(all(target_arch = "wasm32", feature = "portal"))]
pub mod portal;
#[cfg(all(target_arch = "wasm32", feature = "portal"))]
pub mod portal_wasm;
