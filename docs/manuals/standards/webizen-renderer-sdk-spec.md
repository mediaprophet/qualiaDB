# Webizen N-Dimensional Renderer — SDK Specification

**Version:** 0.2
**Date:** 2026-06-30
**Status:** Draft Standard
**Target Environment:** QualiaDB / Webizen `0.0.33`
**Repository:** https://github.com/mediaprophet/qualiaDB/tree/0.0.33

> **Why this is a draft, and why it exists before the implementation is "finished."**
> A renderer SDK is defined by the *contract an embedder programs against* — its data
> layouts, its projection model, its draw surface, its conformance oracles — not by how
> complete any one backend is. The position that "nothing can be specified until it is
> fully built" is exactly the tool-lock-in this project rejects: it makes the embedder a
> hostage to one vendor's release schedule. This document defines the renderer's SDK
> surface independently of completeness, and marks every capability with an honest status.
> Implementations are measured *against* this spec; the spec is not watered down to match
> whatever happens to compile today.

---

## Status legend

Every capability below carries one of:

| Mark | Meaning |
|------|---------|
| ✅ **Implemented** | Code exists in the repo and is covered by a passing test/oracle. |
| ◑ **Partial** | A working subset exists; a named part of the contract is not yet wired. |
| ○ **Draft-only** | Contract defined here; implementation is a tracked to-do (§10). |

The honest summary up front: the **manifold→projection→view** core, **zero-heap GPU ABI**,
**neutral scene contract**, **native caller-buffered offscreen rendering**, and the
**depth-buffered volumetric path** are implemented. Browser and native use the same wgpu 29
pipeline for depth, bloom, Tensor10D SOA projection, picking, and mesh surfaces. The legacy
immediate-mode `WgpuRenderer` remains as a compatibility API; default scene-to-image helpers
route through the canonical volumetric engine.

---

## 1. Scope & intent

The Webizen N-Dimensional Renderer projects QualiaDB's 10-dimensional semantic manifold
([`q42-10d-tensor-standard.md`](q42-10d-tensor-standard.md)) onto 2D/3D surfaces using
**Projective Geometric Algebra (PGA)**. It is intended to be **employable as an SDK** in
two deployment profiles from a single engine:

1. **WASM** — a browser/`wasm32` bundle exposing the portal facade for an in-page viewport.
2. **Native / webizen-browser** — desktop and studio applications embedding the renderer
   through the `webizen-render` crate.

**Unified-engine principle.** The renderer is *one* engine with two front doors. The
platform-agnostic projection mathematics, the manifold contract, the PGA oracle, and the
binary ABI and device-facing draw loop live in **`qualia_core_db::render`** (the engine).
**`webizen-render`** is the SDK/serde adapter and image-codec boundary. Native instances use
`gpu_context::shared_gpu()` so inference and rendering share one physical wgpu device.
Embedders never re-implement the projection; they consume it.

```
┌─────────────────────────────────────────────────────────────────┐
│  Embedders                                                        │
│  webizen-desktop · webizen-studio · webizen-web · WASM page       │
└───────────────▲─────────────────────────────▲────────────────────┘
                │ RenderScene (serde IPC)      │ QualiaPortal (wasm)
┌───────────────┴──────────────────┐          │
│  webizen-render (renderer crate)  │          │
│  VolumetricRenderer · RenderScene │          │
│  adapter · PNG/data-URI bridge    │          │
└───────────────▲──────────────────┘          │
                │ depends on                   │ feature = "portal"
┌───────────────┴──────────────────────────────┴───────────────────┐
│  qualia_core_db::render  (the engine — single source of truth)    │
│  Tensor10D · projection (one project, many views) · pga oracle ·  │
│  camera · contract (ABI) · spectral · acoustic · standpoint ·     │
│  physics · navigation · authoring · gpu (native + wasm) · portal  │
└───────────────────────────────────────────────────────────────────┘
```

---

## 2. The projection model — *one projection, many views*

The renderer's foundation is the 10D tensor manifold. A node's *place* is decided **once**,
by the semantic-motor map `10D → 3D world`. Every "view" — the 3D scene, the 2D canvas — is
then a projection of that **same** world point onto a target, never an independent
re-computation. This is the central invariant of the SDK.

**Reference (engine):** `qualia_core_db::render::projection`.

```rust
/// Which view of the shared manifold world point to produce.
pub enum ProjectionTarget {
    Plane2D,   // orthographic shadow on z = 0
    Volume3D,  // the world point itself (orbit camera applied downstream)
}

/// Shared step: a 10D tensor node → its 3D world position via the semantic-motor map.
/// This is the parity-tested CPU oracle of `projector.wgsl`.
pub fn manifold_world(t: &Tensor10D, time: f32) -> [f32; 3];

/// One projection, many views. Volume3D = world point; Plane2D = its (x,y) shadow.
/// The two agree on (x,y) by construction — the same manifold point seen two ways.
pub fn project(t: &Tensor10D, time: f32, target: ProjectionTarget) -> [f32; 3];
```

Status: ✅ Implemented. Tests assert `Plane2D` is the exact `(x,y)` shadow of `Volume3D`,
that a Euclidean (`v=0,w=0,q=0`) node projects to itself, and that projection is
deterministic for fixed `(node, time)`.

### 2.1 PGA semantic motors

The `10D → 3D` map is a **Motor** (a PGA element unifying rotation and translation) derived
from the tensor's structural dimensions. **Reference (engine):** `qualia_core_db::render::pga`.

| Function | Role |
|----------|------|
| `semantic_motor_intrinsic(v,w,q,σ,time,α,local,standpoint,aperture)` | The full per-node motor: topology (`v`) bands, domain (`w`), epistemic (`q`) animation, spectral cluster (`σ`). |
| `sandwich_point(m, p)` | Apply a motor to a point (PGA sandwich product). |
| `motor_mul` / `motor_reverse` / `motor_translate` | Motor algebra. |
| `motor_rq_gated(q,σ,time,α,standpoint_class,aperture)` | **Standpoint-gated** epistemic rotor — see §7.2. |
| `cluster_id_from_sigma(σ)` / `cluster_centroid_lattice(id)` | Spectral clustering of related concepts in world space. |
| `motor_to_mat4_col(m)` | Motor → column-major mat4 for GPU upload. |

Status: ✅ Implemented. The CPU `pga` module is the **normative oracle**: `projector.wgsl`
on the GPU MUST produce the identical world point (parity is a conformance requirement, §9).

---

## 3. Binary ABI — the zero-heap GPU layouts

All hot-path data crosses the CPU↔GPU boundary as **fixed-size, bit-packed, `bytemuck`-castable**
structs. No `Vec`/`String`/`Box` on the render hot path (project-wide invariant). These sizes
are **normative** and asserted by tests.

| Type | Size | Crate / module | Purpose |
|------|------|----------------|---------|
| `Tensor10D` | **40 B** | `qualia_core_db::tensor` | `[q,v,w,x,y,z,t,α,μ,σ]` f32 manifold node. |
| `Motor` (render) | **64 B** | `webizen_render::math::motor_encoder` | rotor(scalar,e12,e31,e23) + translator(scalar,e1,e2,e3) + pad. |
| `RenderQuin` | **64 B** | `webizen_render::math::buffer_alignment` | `motor[8]` + `semantic_id u64` + `intensity/confidence/timestamp f32` + pad; `#[repr(C, align(64))]`. |
| `AlignedBufferF32` | **64 B** | `webizen_render::math::buffer_alignment` | `[f32; 16]` aligned scratch. |
| `TensorBufferHeader` | **32 B** | `qualia_core_db::tensor::buffer_export` | SOA upload header; `TENSOR_STRIDE = 40`. |
| `CameraUniform` | **128 B** | `qualia_core_db::render::telemetry` | GPU camera uniform. |
| `ObserverStandpoint` | **128 B** | `qualia_core_db::render::telemetry` | Standpoint/aperture uniform (binding 4). |
| `ParticleInstance` | **16 B** | `qualia_core_db::render::telemetry` | Ambient particle instance. |
| `SystemTelemetry` | **48 B** | `qualia_core_db::render::telemetry` | Per-frame telemetry uniform. |
| `PortalControlCommand` | **8 B** | `qualia_core_db::render::control` | Input-control-protocol (ICP) ring command. |
| `SonicToken` | **8 B** | `qualia_core_db::sonic_token` | Acoustic-plane token. |
| `AcousticUniform` | **328 B** | `qualia_core_db::audio::acoustic_plane` | 18 scalars + 64-bin σ preview. |

Status: ✅ Implemented. `render::contract::phenomenal_uniform_struct_sizes_match_wgsl` and the
crate-level `test_motor_size`/`test_render_quin_size` lock these. The **Motor SOA upload** path:
`TensorBufferHeader` (32 B) is skipped when binding the SOA storage; nodes follow at
`TENSOR_STRIDE = 40 B` each.

---

## 4. The Scene Contract — the neutral SDK surface

The primary embedding surface is a **serde-serializable, backend-agnostic scene graph** that
any host (webizen-studio, a Tauri command, a wasm caller) passes to the renderer for headless
or on-surface drawing. It uses CSS color strings to match the codebase's visual semantics.

**Reference (renderer):** `webizen_render::scene_contract`.

```rust
pub struct RenderScene {
    pub nodes: Vec<SceneNode>,         // vertices
    pub edges: Vec<SceneEdge>,         // lines
    pub faces: Vec<SceneFace>,         // filled polygons
    pub camera: SceneCamera,
    pub background: String,            // CSS color
    pub selected_node_index: Option<usize>,  // zero-heap binary picking
    pub hovered_node_index: Option<usize>,
    pub transition_state: Option<TransitionState>, // smooth interpolation
    pub temporal_slice: f64,           // t for time-travel navigation
    pub epistemic_filter: EpistemicState,
}

pub struct SceneNode {
    pub id: String,
    pub position: ScenePoint,          // x,y normalized 0..1; z depth
    pub color: String, pub radius: f64, pub alpha: f64,
    pub is_inferencing: bool, pub pulse_rate: f64,
    pub tensor: Tensor10DProjection,   // the 10D node behind the vertex
    pub epistemic_state: EpistemicState,
    pub version: f64,                  // temporal (t)
}
```

`SceneCamera { position:[f64;3], target:[f64;3], fov:f64 }` (default eye `[0,0,500]`, fov 60°).
`ScenePoint { x, y, z: f64 }`. `SceneEdge { from, to, color, width, alpha }`.
`SceneFace { vertices: Vec<ScenePoint>, color, alpha }`.

### 4.1 `Tensor10DProjection` and `EpistemicState`

Each scene node carries the 10D tensor it projects from, plus its epistemic state:

```rust
pub struct Tensor10DProjection { pub q,v,w,x,y,z,t,alpha,mu,sigma: f64 } // default α=1
pub enum EpistemicState { Collapsed /*q=0*/, Pending /*q>0 escrow*/, Sandbox /*q>0 parallel*/ }
```

Helper surface on `Tensor10DProjection`: `spectral_to_color() -> String` (σ → CIE XYZ → sRGB),
`amplitude_to_opacity()`, `has_hidden_metadata()` (μ > 0.5), `get_epistemic_state()`.

Status: ✅ Implemented (serde round-trip + builder tests). With the default `qualia` feature,
`spectral_to_color` delegates to the engine's normative
`qualia_core_db::render::spectral::sigma_to_display_rgb` oracle.

---

## 5. The device renderer — `PortalGpu` / `VolumetricRenderer`

The engine owns the canonical wgpu 29 draw graph. `webizen-render::VolumetricRenderer` adapts the
neutral scene contract to it. `WgpuRenderer` remains available for compatibility primitives, but
the default scene-to-image entry points route through `VolumetricRenderer`.

```rust
// Canonical native construction (uses qualia_core_db::gpu_context::shared_gpu)
VolumetricRenderer::new_offscreen(width, height, particle_cap) -> Result<Self>

// Zero-copy/caller-buffered inputs and output
upload_tensor_buffer(&[u8]) -> Result<u32>
upload_mesh(&[[f32;3]], &[u32]) -> u32
render(time, &SystemTelemetry) -> Result<()>
required_rgba8_bytes() -> usize
read_rgba8_into(&mut [u8]) -> Result<usize>

// Neutral scene adapter
render_scene_rgba8_into(&RenderScene, w, h, time, telemetry, out) -> Result<usize>

// Explicit cold image-codec boundary
render_scene_png(&RenderScene, w, h) -> Option<Vec<u8>>
render_scene_png_with_time(&RenderScene, w, h, t) -> …
render_scene_png_with_time_and_telemetry(&RenderScene, w, h, t, &SystemTelemetry) -> …
render_scene_data_uri(&RenderScene, w, h) -> Option<String>
```

Status: ✅ Implemented. Nodes upload as Tensor10D SOA projector instances; faces and edges are
triangulated into the depth-tested mesh path; ambient particles, bloom, picking, orbit camera,
linear RGBA8 readback, PNG, and data-URI bridges are live. The native hardware gate renders and
reads back a tensor-plus-mesh frame on the shared GPU.

---

## 6. The semantic rendering layer

What distinguishes this from a generic graph renderer is that **rights, epistemics, and
provenance are first-class in the projection** — they gate what is drawn, not merely how.

### 6.1 Epistemic state (q)
`q = 0` ⇒ `Collapsed` (ground truth); `q > 0` ⇒ `Pending`/`Sandbox` (escrow / parallel
context). The renderer animates and filters by epistemic state (`RenderScene::epistemic_filter`).
Status: ✅.

### 6.2 Standpoint-gated visibility
A node's epistemic rotor is gated by the **observer standpoint class** —
`STANDPOINT_{SPECTATOR, EPHEMERAL, DID, VAULT}` (`render::telemetry`). `motor_rq_gated`
collapses to the identity rotor (node renders inert/hidden) when the standpoint lacks aperture:
a **VAULT** node is always identity (never revealed to the viewport), and a **DID** node with
epistemic aperture 0 collapses. This is surveillance-refusal by construction. Status: ✅
(`phenomenal_standpoint_rq_motor_identity_gate`).

### 6.3 Deontic / temporal culling
`tensor_deontic_lane(μ)` and `bilateral_pull_active(μ, standpoint_class)` route nodes into
deontic lanes; `pull_vector(node, eye, α, q)` applies rights-bounded attraction. Temporal
culling uses `t` against `RenderScene::temporal_slice`. Status: ◑ (lane + pull math
implemented; full deontic/temporal cull pass as a documented pipeline stage is partial).

### 6.4 σ — one signature, two modalities (vision + hearing)
The spectral signature `σ` projects into **both** last-mile modalities without duplicating
storage, with a **determinism requirement**: the GPU path MUST compute the identical mapping
as the CPU reference.

| Modality | Mapping | Reference |
|----------|---------|-----------|
| **Visual** | `λ_nm = 400 + fract(σ)×300` → CIE 1931 XYZ → linear sRGB | `render::spectral::sigma_to_cie_xyz` |
| **Auditory** | same `λ_nm` → `f_hz` (HRTF-spatialized) | `render::acoustic::sigma_to_center_frequency_hz` |

Status: ✅ (`phenomenal_sigma_visual_audio_parity`: `λ ∈ [400,700] nm`, `f ∈ [55, 8000] Hz`,
integer wraps on σ do not change either projection). See
[`q42-acoustic-plane-draft.md`](q42-acoustic-plane-draft.md) §1.3.

### 6.5 Budget governance — VRAM ledger & operational modes
`OperationalMode { Full, Eco, Reserve }` step down under VRAM pressure
(`render::contract` + `gpu_context::VramLedger`); `ambient_draw_instances_for_mode` caps
draw counts (Eco caps at 8k instances). Status: ✅.

---

## 7. Deployment profiles

### 7.1 WASM portal — `feature = "portal"`
The `#[wasm_bindgen]` facade `QualiaPortal` (`render::portal`, with `render::gpu` +
`render::portal_wasm`) drives an in-page WebGPU viewport. Gated to `wasm32 + portal`.
Status: ✅ for the portal facade + volumetric `PortalGpu` pipeline (depth, bloom, tensor SOA).

### 7.2 Native / webizen-browser
`webizen-render` + `qualia_core_db::render` embedded by `webizen-desktop` (Tauri commands,
telemetry bridge) and `webizen-studio` (scene → contract). Studio's `render/` is a *frontend
scene graph* that bridges into the engine via `qualia_core_db::render::contract`
(`Tensor10DProjection`, `EpistemicState`), not a duplicate engine. Status: ✅ — renderer and
studio are workspace members and verified; the desktop host's renderer entry points route to the
volumetric adapter. A full desktop dependency check still requires fetching uncached Tauri
dependencies.

### 7.3 Authoring planner & model-as-substrate
- `render::authoring` (Phase 5): a qapp declares 3D + 2D views over one manifold; the planner
  enforces attestation gates, rights-bounded contexts, and **budget-driven 3D→2D degradation**
  before drawing. Status: ◑.
- `render::model_substrate` (Phase 6, wasm-`llm`-gated): one buffer holds a renderable manifold
  **and** the transcoded P64 weights co-resident; the renderer projects the manifold while the
  weights stay resident. Status: ◑ (early).

---

## 8. Conformance

An implementation conforms to this spec if:

1. **Projection parity.** For every `Tensor10D` and `time`, the GPU `projector.wgsl` world
   point equals the CPU `render::pga` oracle within tolerance (the `pga` module is normative).
   ✅ enforced by parity tests.
2. **Binding coverage.** Every `@group/@binding` declared in WGSL exists in the Rust
   bind-group layout manifest (`render::contract::assert_wgsl_bindings_covered` for projector,
   ambient, bloom). ✅.
3. **ABI sizes.** All §3 struct sizes hold exactly. ✅.
4. **σ determinism.** The visual and auditory σ mappings are identical on GPU and CPU and
   stable under integer wraps. ✅.
5. **Offscreen image contract.** `read_pixels` yields tightly-packed **linear** RGBA8 (a
   mid-tone `#808080` reads back ~128, *not* sRGB-re-encoded ~188); `read_png` emits a valid
   PNG (8-byte magic `89 50 4E 47 0D 0A 1A 0A`); `read_data_uri` starts with
   `data:image/png;base64,`. ✅.
6. **Headless graceful degradation.** With no GPU adapter, offscreen constructors return `Err`
   / the one-call helpers return `None` (never panic). ✅.

Run: `cargo test -p qualia-core-db phenomenal_contract --lib`,
`cargo test -p webizen-render`, and `node docs/tests/phenomenal-verify.mjs`.

---

## 9. Implementation status (honest summary)

| Capability | Status |
|------------|--------|
| Manifold projection model (`project`, `manifold_world`, one-projection-many-views) | ✅ |
| PGA semantic-motor oracle (`pga`) + GPU parity requirement | ✅ |
| Zero-heap GPU ABI (§3 sizes) | ✅ |
| Neutral serde scene contract (`RenderScene` & friends) | ✅ |
| Offscreen render → RGBA8 / PNG / data-URI | ✅ |
| Scene draw of nodes/edges/faces + ambient particles + picking + orbit camera | ✅ |
| Epistemic state, standpoint-gated visibility, σ vision/audio parity, VRAM ledger | ✅ |
| WASM portal facade + **volumetric `PortalGpu`** (depth, bloom, tensor SOA, mesh) | ✅ (wasm-`portal`) |
| **Cross-platform volumetric 3D draw in `webizen-render`** (depth buffer + projector SOA) | ✅ |
| `scene_contract` CIE color unified with engine `spectral` path | ✅ |
| Deontic/temporal cull as an explicit pipeline stage | ◑ (to-do §10.1) |
| Authoring planner (3D+2D over one manifold, budget degradation) | ◑ |
| Model-as-substrate co-residency | ◑ |
| Renderer crates in the default workspace build | ◑ (members added; render + studio verified) |
| Lift `render` tree to a standalone `qualia-render` crate (Phase 0.2b) | ○ (to-do §10.2) |
| SDK packaging (npm/wasm bundle entry + published crate) | ○ (to-do §10.3) |

---

## 10. To-do items (enhancements & unbuilt contract)

Completed in v0.2: cross-platform volumetric rendering, native shared-device offscreen readback,
scene-to-Tensor10D/mesh adaptation, engine spectral-color parity, wgpu 29 alignment, and workspace
membership.

- **§10.1 — Deontic culling as a named pipeline stage** with its own conformance test
  (currently lane + pull math exist but the cull pass is implicit).
- **§10.2 — Phase 0.2b: lift `qualia_core_db::render` into a standalone `qualia-render` crate**
  (the engine `mod.rs` already names this). Resolves the missing `RENDERER_IMPLEMENTATION_PLAN.md`
  reference by folding its content here + an internal plan doc.
- **§10.3 — SDK packaging.** A published crate (`webizen-render`) + a wasm bundle entry
  (`QualiaPortal`) with a documented embedding example, so the renderer is consumable as an SDK
  without building the whole monorepo.
- **§10.4 — Decommission/clearly-mark the legacy `C:\Projects\webizen-browser` copies** so there
  is one source of truth for the renderer (the renderer was pulled into qualiaDB to unify the
  engine; the external checkout still holds parallel copies). *Out-of-band: Timothy's call.*

---

## 11. References

- [`q42-10d-tensor-standard.md`](q42-10d-tensor-standard.md) — the 10D manifold coordinate system.
- [`q42-acoustic-plane-draft.md`](q42-acoustic-plane-draft.md) — σ auditory projection, HRTF, SAB layout.
- [`MULTI_AGENT_PROTOCOL.md`](MULTI_AGENT_PROTOCOL.md) — governance ISA (shares the standpoint/rights model).
- Engine: `crates/qualia-core-db/src/render/` (`projection`, `pga`, `camera`, `contract`,
  `spectral`, `acoustic`, `standpoint`, `telemetry`, `control`, `physics`, `navigation`,
  `authoring`, `model_substrate`, `gpu`/`portal`/`portal_wasm`).
- Renderer: `crates/webizen-render/src/` (`volumetric`, `wgpu_renderer`, `scene_contract`, `math`,
  `pipeline`, `shaders`, `telemetry`, `audio_contract`).

---

*This is a draft. It is authored to be built toward, not to describe a finished artefact.
Where the implementation is incomplete (§9, §10), the contract here is the target; the
implementation is brought up to it, not the spec brought down to the implementation.*
