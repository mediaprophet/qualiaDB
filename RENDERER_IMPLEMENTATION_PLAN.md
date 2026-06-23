# Renderer / Engine — Implementation Plan

> **Spec:** [`RENDERER_DEFINITION.md`](RENDERER_DEFINITION.md) (the *what/why*). **This file:** the
> *how/when/in-what-order*, with verifiable acceptance gates. **Companion math:**
> [`INGEST_PIPELINE_SPEC.md`](INGEST_PIPELINE_SPEC.md). **Source specs:** STELLAR §D–§G, the 10D tensor
> spec, the migration review. Authorship: the architecture is Timothy's; this plan operationalises it and
> claims no authorship.
>
> **Honesty contract (non-negotiable, the style of `DEONTIC_LOGIC_PLAN.md`):** nothing is marked "done"
> without its **acceptance check** passing — a green test, a visibly running artifact, or a screenshot. No
> overclaiming. Current-state is stated honestly. Work that is *designed-now / built-later* is labelled as
> such, not hidden.

---

## 0. Governing constraints carried into EVERY phase

These are not phase work; they are gates on all of it (RENDERER_DEFINITION §8; the memories):

- **Affordability & honest-scope.** Heavy passes run **once, AOT, off-device** (capable node / guild) and are
  **distributed**; the user's device pays only the **cheap zero-heap fold**. Runs on hardware people own; we
  do **not** claim to replace datacenter compute. (`feedback-affordability-honest-scope`.)
- **Inherited governance substrate.** The deontic gate, standpoint/frame-relative resolution, wisdom-out-of-
  band, store-physics-map-percept, Curation Directive (propose/attest). The engine **inherits** these — it
  does not get its own (RENDERER_DEFINITION §8).
- **Zero-heap / 48-byte / one device.** Fixed 10D stride for any modality; out-buffer hydration; one `wgpu`
  device shared by compute + render.
- **Target matrix (every phase's acceptance is verified against this).** The engine ships across *all* of:

  | Class | Target | wgpu backend | CPU / NPU | Status |
  |---|---|---|---|---|
  | Native desktop/laptop | **Windows** (x86-64) | DX12 | AVX2 SIMD · DirectML (NPU) | |
  | | **macOS** (Apple Silicon + Intel) | **Metal** | NEON/AMX · Accelerate · ANE (CoreML) | best-case — unified memory = zero-copy |
  | | **Linux** (x86-64 / arm64) | Vulkan | AVX2 / NEON | |
  | Native mobile | **iOS / iPadOS** (arm64) | Metal | NEON · ANE (CoreML) | via PWA-edge shell / Flutter FRB bridge |
  | | **Android** (arm64) | Vulkan | NEON · NNAPI | via PWA-edge shell / Flutter FRB bridge |
  | WASM (browser) | **`wasm32`-unknown-unknown** | WebGPU | WASM-SIMD · WebNN | primary web path; **LIVE in Chrome 2026-06-23** (3D `PortalGpu` viewport) |
  | | **`wasm64` / memory64** | WebGPU | WASM-SIMD · WebNN | **decided, currently unbuilt** — stand up + verify tooling; for >4 GB manifolds/models |

  **Verification discipline:** per-phase acceptance is verified on at least the **primary targets** (one native
  backend + `wasm32`); **full-matrix parity** (incl. mobile + `wasm64`) is a **release gate**, tracked per
  target — never silently assumed. The *same* 10D structure + VM bytecode runs on all; only the math backend
  swaps (spec §3.1). This matrix is the **canonical** target list (the spec's §3.3 hardware story references it).

  **Selection rule — prefer native over WASM.** If **both** a native runtime and the WASM path are available
  on a device, **use native.** Native gets the real wgpu backend (Metal / DX12 / Vulkan), **direct hardware +
  NPU** (DirectML / CoreML-ANE / NNAPI), the **full address space** (no 4 GB `wasm32` cap), **real threads**,
  and **no browser sandbox or WebGPU device-limit fragility**. **WASM is the portable fallback** — for
  zero-install reach and devices with no native app installed (the browser is an *optional shell over the
  engine*, per the migration review). This is the **default**; the user/standpoint may override (e.g. choosing
  the browser for ephemerality/privacy) — agency, not a forced override.

Each phase below ends with a **Rail-check** confirming it honours these.

---

## 1. Current state — the honest baseline we build FROM

Per RENDERER_DEFINITION §9 + [`RENDERER_SURVEY.md`](RENDERER_SURVEY.md) + STELLAR §E (point-in-time; **verify
against code before executing each phase** — these reads may have moved):

- **Real:** data is genuinely 10D with real 3D (`Tensor10D{q,v,w,x,y,z,t,α,μ,σ}`, `SpacetimeCoord`);
  `webizen-render` has 3D scaffolding (4×4 `view_projection`, look-at `SceneCamera`, PGA math, z-depth); the
  10D metric is GPU-resident (`tensor_volume.wgsl`).
- **Not yet real (the gap):** implemented output is the **~2.5D ambient particle field** (50k points) — **no
  depth-stencil, no mesh vertex/index, no `.obj`/`.stl`/OpenUSD import** → 3D *assets* are not rendered.
- **Duplication hazard:** `webizen-render` (browser, builds) vs engine `portal_*` (builds) vs orphaned
  `crates/webizen-*` (committed, **not** in workspace).
- **Dark demo:** `docs/spatial.html` shows "WASM Engine Required" — the WASM bundle isn't loading.
- **Blocker:** `wgpu 0.19.4` sends `maxInterStageShaderComponents` → `requestDevice` fails on current Chrome.
- **Sense path:** acoustic/spectral is contract types only — **no real DSP**.

---

## 2. The phases

### Phase 0 — Unblock & consolidate *(foundation; gates everything)*
The precondition for "fast on every device" being non-fiction (migration review §4).

- **0.1 `wgpu` upgrade** (0.19.4 → 0.20+): remove the `maxInterStageShaderComponents` device-limit failure;
  one GPU regression pass for **both** compute and render on the shared device.
  - *Acceptance:* `requestDevice` succeeds across the **WebGPU targets** — current Chrome on **`wasm32` and
    `wasm64`** — **and** the native backends (**Metal / DX12 / Vulkan**, which are less affected by this bug);
    existing GPU tests green; the browser LLM demo initialises again.
- **0.2 Render consolidation → one `qualia-render` crate** (workspace member), base = `portal_*` (already
  in-engine, already has the wasm canvas path); **absorb** `webizen-render`'s unique pieces (scene-graph /
  `scene_contract`, native offscreen delivery, audio/spectral contract, glTF intent); **delete** the orphaned
  `crates/webizen-*` after salvage. Engine stays platform-agnostic (no `winit`/`tauri`/`rfd`/`dioxus`).
  - *Acceptance:* `qualia-render` builds standalone across the **target matrix** — native (**Windows/DX12,
    macOS/Metal, Linux/Vulkan**), **`wasm32-unknown-unknown`**, and **`wasm64`/memory64** (stand it up — decided
    but currently unbuilt; verify tooling) — with **no UI-shell deps**; orphan crates removed; **one** renderer
    source of truth. (Mobile native — iOS/Metal, Android/Vulkan — builds via the shell/bridge, matrix-tracked.)
- **0.3 Re-light the dev-bench:** fix the WASM bundle so `spatial.html` runs the existing ~2.5D field.
  **✅ DONE 2026-06-23** — exceeded: `spatial.html` runs the *3D* `PortalGpu` path, not just the 2.5D field.
  - *Acceptance:* `spatial.html` renders the particle field in-browser (no "WASM Engine Required") — screenshot. **met**
- **Rail-check:** consolidation removes drift (one source); no shell deps in the engine; affordability path
  (one shared device) preserved.

### Phase 1 — World-space 3D scene *(closes the ~2.5D → 3D gap; STELLAR §E step 1)*

> **Discovered 2026-06-23:** `portal_gpu.rs::PortalGpu` is *already* a real WebGPU 3D renderer —
> surface-from-canvas, **depth texture + depth-stencil**, render passes, Kawase **bloom**. So much of
> 1.1 and the render scaffolding already exist. The blocker is that it is **hard-disabled in WASM**
> (`portal.rs`: `let wasm_sync_gpu_ok = false`) because `PortalGpu::try_new` uses `pollster::block_on`,
> which traps on the browser main thread. Hence **1.0** below — the real near-term unlock.

- **1.0 — Unlock the existing WebGPU 3D path (async init).** Replace the synchronous
  `block_on(request_adapter / request_device)` with **async WebGPU init** for wasm32 (await via
  `wasm-bindgen-futures`; keep `block_on` for native); plumb a one-time async init through the portal's
  wasm-bindgen API + `loadQualiaPortal`, stash the initialised GPU, and flip the gate so the GPU path
  engages. The spectral σ data still drives colour; depth + bloom come for free from `PortalGpu`.
  **✅ DONE 2026-06-23** (commit `13f9a3346`) — async init + the three Dawn-strict fixes (see Progress log).
  - *Acceptance:* `spatial.html` renders via `PortalGpu` (depth-tested 3D + bloom), **not** canvas2d, in
    Chrome — screenshot proof; canvas2d remains the fallback when WebGPU is unavailable. **met** (orbit verified)
- **1.1** depth-stencil buffer (occlusion). **1.2** mesh vertex/index buffers (geometry).
- **1.3** asset import: **`glb_bridge` as `fn(&[u8]) -> …NQuin…`** (no `std::fs`; the shell hands bytes down —
  migration §2.1); then `.obj`/`.stl`; **OpenUSD deferred** (heavy — built-later).
- **1.4** `project: 10D → target` via the volume metric — the **same manifold** yields a 2D canvas and a 3D
  scene (one projection, many views).
  - *Acceptance:* a loaded `.obj`/`.stl` renders as an **occluded world-space mesh** on a native backend
    **and** `wasm32` (mobile + `wasm64` matrix-tracked per §0); a 2D view and a 3D view of the **same** manifold
    are produced from one `project()` call (test + screenshot).
- **Rail-check:** per-vertex path zero-heap; asset import is `&[u8]` (wasm-safe); percept mapped at render
  (store-physics).

### Phase 2 — Physics of artefacts *(STELLAR §E step 2)*
- mass / material / momentum (`P` in the Manifold-Coordinate); `specialized_libs/physics_simulation`; PGA
  geometry that **refuses to contract** on a bounding-box violation; joints as kinematic multivectors.
  - *Acceptance:* an artefact deterministically **refuses** an action that violates its bounding box (test); a
    kinematic multivector animates a joint over `t` (screenshot/test). **✅ MET 2026-06-23** (`f60df3969`):
    `render/physics/{aabb,admission,joint,material}.rs` — 16/16 tests; both acceptance items test-verified.
- **Rail-check:** deterministic prevention (no probabilistic guess); zero-heap operators. **met** (deterministic
  admission; fixed-array operators). Optional next: live viewer integration (animate a loaded mesh via a joint).

### Phase 3 — Place / space / time *(spatio-temporal binding; STELLAR §E step 3)*
- `x,y,z` (space) + `t` (temporal evolution / animation) + **GeoSPARQL** place/jurisdiction; native **RCC-8**
  (`spatio_temporal.rs`) + **Allen / LTL** (`temporal_ltl.rs`).
  - *Acceptance:* an artefact situated at a place **and** time is queryable by the **same modalities the values
    layer uses** — demonstrate a spatio-temporal query and a deontic query over the **same** NQuin (test).
- **Rail-check:** spatio-temporal logic uses the inherited modality stack (not a bespoke engine).

### Phase 4 — Sense path *(the input twin; STELLAR §D)* — parallelisable after Phase 0
- SOSA/SSN sensor ingest → **wave coordinates** → the **percept→fact bridge** (`∫Ψ > τ → Fact`, §20
  `manifold_logic`). **Microphone STFT/CQT first** (the readily-available band — real DSP); **RF / Wi-Fi CSI
  deferred** (needs SDR / hardware + permission — honest note, may never be in-browser).
  - *Acceptance:* mic input → STFT → a discrete **Fact** NQuin via the bridge, **under a consent/standpoint
    gate** (test); RF path documented as deferred with the hardware/permission caveat.
- **Rail-check:** every sense under the deontic/standpoint gate — own-environment + consent; surveillance-
  refusal; biometrics never leave device.

### Phase 5 — Authoring vocabulary *(the enhanced `ns/ui`; the qapps upgrade; §6/§7)* — depends on Phase 1
- Design the enhanced `ns/ui` vocabulary: **HTML-like document + 3D scene + SVG vector + spectral/percept
  mapping + sensing bindings + governance primitives** — **mobile/edge-first**, **budget-aware by
  construction** (declarative bounds + graceful **3D→2D degradation**), **governance-native** (**attestation
  gates** = DID-signature triggers invoking "human attests"; **defeasible views** = render `q>0` escrow +
  confidence + quorum; **rights-bounded contexts** = container refuses sensitive render in a shared civic view).
- Bridge: **RDF → CBOR-LD → NQuin** (`@context` expansion = task #8). **ShEx (ADR 0009) *describes*** the
  vocabulary's contract + recursive structures; **SHACL *enforces***; one source.
- Upgrade [`qapps_specification.md`](docs/manuals/qapps_specification.md): 2D panes → manifold worlds.
  - *Acceptance:* a sample qapp authored in the vocabulary renders a **3D scene + a 2D pane from one manifold**;
    an attestation-gate and a rights-bounded context are **demonstrably enforced**; on a low-tier device
    profile, budget-degradation **collapses 3D→2D** (test + screenshots).
- **Rail-check:** affordability enforced *at authoring time* (budget-aware syntax); governance primitives are
  the §8 substrate surfaced; wisdom-out-of-band (attestation gates).

### Phase 6 — Model-as-substrate & ingest/convergence integration *(largely the §A/§12 workstream)*
- Heterogeneous dispatch (NPU/GPU/CPU, §3.1) + q42 perf (ternary/KIVI/W4A4/spec-decode, §3.2) — **task #12 /
  STELLAR §A**; the renderer **consumes** their output. Model-as-substrate: the renderer projects the **same**
  manifold the transcoded model lives on (graph–tensor duality, §F). Ingest/convergence (birth-record,
  commutative folds) per `INGEST_PIPELINE_SPEC` — **AOT-elsewhere + distributed**, on-device = cheap fold.
  - *Acceptance:* the renderer projects a view of a manifold that **also** holds transcoded model weights (one
    substrate, one device) — demonstrated end-to-end (test).
- **Transcoder (GGUF/safetensor/MLX → Q42) — versioned + streaming** (`INGEST_PIPELINE_SPEC` §7): **leave the
  legacy GGUF→Q42W path in place** (it works for some cases); add a **new versioned** pipeline that ingests
  **high-fidelity sources only (Q8 / F16 / BF16)**, adds **safetensor + MLX** to `detect.rs`, and uses a
  **streaming encoder** (mmap source + per-tensor / per-block processing + incremental page-aligned flush) so
  the **whole file is never loaded into RAM**.
  - *Acceptance:* a multi-GB F16/Q8 model transcodes to Q42 with **peak memory ≈ the largest single tensor**
    (measured, not the whole file); the **legacy GGUF→Q42W path still works** (regression test); **Q4 input is
    rejected/warned**.
- **Honest dependency:** this is mostly the separate §A/§12 effort. **Basic 3D rendering (Phases 1–3) does NOT
  block on it.** Sequenced in parallel / after; the renderer depends on it only for the *unification* claim.

---

## 3. Sequencing & critical path
1. **Phase 0 gates everything** (0.1 `wgpu` first — it gates both the WASM LLM and the WASM render path).
2. **Phases 1 → 2 → 3** = the renderer core (roughly sequential).
3. **Phase 4 (sense)** parallelisable after Phase 0 (largely independent of 1–3).
4. **Phase 5 (authoring)** depends on Phase 1 (needs a 3D scene to author into) + the CBOR-LD bridge (task #8)
   + ShEx (ADR 0009).
5. **Phase 6** is largely §A/§12 (task #12); renderer does not block on it for basic 3D.

**Capacity honesty:** this is a multi-month effort for one person + AI under resource constraint. Sequence by
**highest-leverage, foundation-preserving** steps (Phase 0 → 1). The advanced layers (full ingest unification,
RF sensing, OpenUSD) are **designed-now / built-later** — designed so the foundation isn't gutted, built when
capacity allows.

---

## 4. What runs where (the affordability split)

| Phase work | On-device (cheap) | AOT-elsewhere + distributed (heavy) |
|---|---|---|
| 0.1 wgpu / 0.2 consolidate / 0.3 bundle | build-time | — |
| 1 mesh/asset render | render (zero-heap) | asset *baking* (mesh→NQuin) if large |
| 2 physics | runtime operators (light) | heavy sim (offline) |
| 3 spatio-temporal queries | runtime (modalities) | embedding/bake |
| 4 mic STFT | runtime (light) | — (RF needs hardware) |
| 5 authoring render | render the compiled qapp | CBOR-LD compile of the RDF source |
| 6 model merge/fuse/transcode | **the cheap fold only** | **all of it** (dequant/align/fuse/quantize) |

---

## 5. Risks & honest gaps
- **wgpu upgrade may cascade** (GPU regression across compute + render) — do it once, one regression pass.
- **ShEx tooling maturity** (ADR 0009) — verify the implementation before relying on it.
- **RF / Wi-Fi CSI** needs hardware + permissions — may never be in-browser; keep it honestly deferred.
- **OpenUSD import** is heavy — deferred to built-later.
- **§A perf numbers are targets, not measured** — the proven baseline is ~5.9 tok/s SmolLM2-360M.
- **One-person capacity** — the binding constraint; favour foundation-preserving order.

---

## 6. Acceptance philosophy (restated)
A phase is "done" only when its **acceptance check** passes — green test, visibly running artifact, or
screenshot proof. No phase is reported complete on "code written." Current-state claims are verified against
code, not assumed. Designed-now/built-later items are labelled, never presented as built.

---

## Remaining work & optimizations (renderer) — captured 2026-06-23

**Correctness / latent bugs**
- **Bloom composite bind group** (`render/gpu/resources.rs` `make_bloom_bind_group`): binding 4
  (`composite_params`) uses `as_entire_binding()` (offset 0), so the composite shader reads the
  *bloom* params, not the composite params — `exposure` becomes `BLOOM_THRESHOLD` (1.0) instead of
  `BLOOM_EXPOSURE` (1.05) and `bloom_strength` becomes `BLOOM_INTENSITY` (1.15) instead of
  `BLOOM_STRENGTH` (0.85). It *looks* fine (exposure≈1), but the tuning constants are silently
  ignored. Fix: bind `composite_params` at offset 16 (its own `BufferBinding`) or a separate buffer.
- **Mesh orbit-drag not re-verified** with a real pointer (synthetic drag didn't visibly rotate; the
  camera transform itself is correct — perspective is right). Confirm real-user orbit on a mesh.

**Performance**
- **`sync_bloom_targets()` runs every frame** (called from `paint_frame`) and rebuilds the entire
  bloom chain — textures + 3 pipelines + uniform buffer — per frame. Should rebuild only on
  size/mode change. Likely the biggest per-frame waste.
- **DPR-ignoring resize**: the `ResizeObserver` calls `portal.resize(canvas, clientWidth,
  clientHeight)` in CSS pixels, so the GPU renders below device resolution (soft). Resize at
  `×devicePixelRatio` (clamped for the affordability budget) for crisp output.

**Aesthetics (“remarkable”)**
- Tensor node points read as **hard squares** — bloom blows out the soft-dot alpha falloff. Tune
  bloom threshold/intensity vs the projector `hdr_gain`, or sharpen the fragment falloff, for soft
  glowing orbs.

**Structure**
- `render/portal/mod.rs` (~940) could split into acoustic-API / data-ingestion-API submodules.
- `render/portal/paint.rs` also holds 3 non-painter helpers (`acoustic_uniform_to_floats`,
  `html_escape`, `append_parsed_dom`) — relocate for cohesion.
- Relabel the init validation error-scope comment in `render/gpu/mod.rs` (says “DIAG”, is the
  permanent safety net).
- **0.2b** — lift `render/` into a standalone `qualia-render` crate (break the core↔render cycles:
  `daemon_tensor` / `webizen_server` / `acoustic_plane` / `buffer_export`).

**Phase 2 physics extensions**
- ~~Live viewer integration — animate a loaded mesh via a joint (the visible half).~~ **DONE
  2026-06-23 (`c84b4d5ae`)**: per-mesh model-transform uniform + `motor_to_mat4_col`; an "Animate
  artefact" toggle spins a loaded mesh via a revolute joint — verified in Chrome (two poses).
- Bind momentum `P` to the NQuin Manifold-Coordinate (STELLAR §C file-format work).
- Inter-artefact collision/contact (current physics is per-artefact admission + kinematics only).
- ~~Surface the **deterministic refusal** visibly too.~~ **DONE 2026-06-23**: a "Demo refusal"
  button slides a loaded mesh (+X prismatic joint) into a world bound; admission clamps it at the
  wall and `artefact_refused()` flips the on-screen verdict — verified in Chrome (clamped flush at
  the bound, `refused=true`, held).

## Progress log

### 2026-06-23 — Phase 2 ROUND-OUT: visible deterministic refusal + elapsed-time joint fix
- **Visible refusal** — `demo_artefact_refusal()` (portal) drives the loaded mesh along +X into a
  world `Aabb`; `PortalGpu::update_model` gates each frame's proposed pose through `Admission` and
  holds the last admitted pose on refusal. `artefact_refused()` surfaces the verdict; the viewer's
  "Demo refusal" button + `#refusal-status` line show admit→REFUSED live. Deterministic tick probe:
  `admit` while in-bounds, `REFUSED` once past the bound, then stable.
- **Bug fixed — joints now driven by *elapsed* time, not absolute sim-time.** `motor_at` was fed the
  renderer's monotonic `self.time` (accumulating since page load), so arming a prismatic slide late
  proposed a huge translation on frame 1 → instant refuse, mesh stuck at the origin (never slid).
  Added `artefact_t0` (latched on the first post-arm frame; reset on `set_artefact_joint`) and drive
  the joint by `time − t0`. A slide/spin now always starts from rest when armed, regardless of how
  long the page has been open. Verified: after pushing sim-time to ~60 s, a freshly-armed slide still
  glides from rest (~8 admitted frames) before clamping.

### 2026-06-23 — Phase 2 DONE (acceptance): physics of artefacts (`f60df3969`)
`render/physics/` — deterministic, zero-alloc, on the `render::pga` motor oracle:
- **aabb** (artefact extent + rigid/scale transform), **admission** (deterministic refuse of
  contraction-below-floor / out-of-world — rotation is correctly *not* contraction), **joint**
  (revolute/prismatic PGA motors over `t`, chainable), **material** (mass = density·volume,
  momentum `P = m·v`, kinetic energy). **16/16 tests**; native + wasm(portal) green.
- Acceptance (test-based) met: deterministic bbox refusal + kinematic joint over `t`. Optional
  follow-up: wire a joint to animate a loaded mesh in the viewport (the visible "screenshot" half).

### 2026-06-23 — Phase 0.2a DONE: renderer consolidated into a `render/` module tree (no monoliths)
In-crate restructure (precursor to the 0.2b standalone crate), honouring "libraries with
subdirectories, no monolithic files". Verified: native `cargo test --lib` **1173 passed / 0 failed**;
wasm(`portal`) build green; viewport unchanged in Chrome (3D scene + 2D shadow). JS / wasm-bindgen API
unchanged; crate-root re-exports (`QualiaPortal`, `WebEngine`) preserved.
- **Stage 1 (`0f8e9e543`)** — flat `portal_*` / `asset_bridge` / `manifold_project` → one `render/`
  tree (`telemetry, standpoint, camera, navigation, pga, projection, contract, spectral, acoustic,
  control, assets, gpu, portal`); `lib.rs` exposes one `pub mod render;`. qualia-cli re-pathed.
- **Stage 2 (`68a0d5316`)** — `render/gpu.rs` (1764) → `gpu/{mod,bloom,resources,particles}.rs`
  (Kawase bloom / resource builders / particle field); no file over ~840 lines.
- **Stage 3 (`2b2c7e95d`)** — `render/portal.rs` (1214) → `portal/{mod,paint}.rs` (canvas2d painters
  out of the `#[wasm_bindgen]` facade).
- **Remaining:** **0.2b** — lift `render/` into a standalone `qualia-render` workspace crate (requires
  breaking the core↔render cycles: `daemon_tensor` / `webizen_server` / `acoustic_plane` /
  `buffer_export` reach into the renderer). Optional: finer split of `portal/mod.rs` (~940) into
  acoustic / data-ingestion API submodules.

### 2026-06-23 (Phase 1.1–1.3 DONE) — imported meshes render as solid 3D surfaces
Commits `65a14dd74` (import), `c9b0736b3` (surface render), `11b55a178` (picker UI), branch `0.0.19`,
not pushed.
- **1.3 asset import** — `asset_bridge.rs`: pure `&[u8]` → `Mesh` + semantic NQuins for OBJ / STL
  (binary+ASCII) / GLB (no `std::fs`, wasm-safe). 10/10 unit tests; CLI `Mesh` ingest format. `mesh_to_nquins`
  records the asset as *known* geometry (counts, bbox, centroid, format) in one identity space.
- **1.1 + 1.2 surface render** — `PortalGpu` mesh pipeline (surface + HDR), f32x3 vertex + u32 index
  buffers, **depth-tested**, flat-shaded via screen-space derivatives (`mesh.wgsl`); reuses the orbit
  camera. `QualiaPortal.upload_mesh_asset(bytes, hint)` centres+scales to the orbit frame and uploads.
  **Verified in Chrome:** a 12-triangle cube OBJ renders as a solid, depth-tested, perspective surface.
- **UI** — spatial.html "Load 3D Asset (OBJ/STL/GLB)" picker → `loadMeshAsset` → `upload_mesh_asset`.
- **1.4 unified projection — DONE** (`1c79a8e11` core, `fc5b341d0` live view): `manifold_project.rs`
  exposes one `project(tensor, time, target)` over the parity-tested `portal_pga` oracle — the same
  manifold world point as a 3D volume position OR its 2D planar shadow (property unit-tested 3/3).
  `QualiaPortal.project_resident_plane2d` + the spatial.html 2D companion canvas render that 2D view
  live beside the GPU 3D scene — **verified in Chrome** (screenshot: 3D node scene + 2D shadow inset).
  Acceptance (test + screenshot) met. Nuance: the 2D view shows all nodes; the 3D view additionally
  applies the temporal scrub. **➡ Phase 1 COMPLETE** (1.0–1.4).
- Note: orbit-drag on a loaded mesh not re-verified this run (the camera transform applies — perspective
  is correct — but the drag→orbit input binding wasn't confirmed).

### 2026-06-23 (later) — Phase 0.1 browser-accepted + Phase 0.3 + Phase 1.0 DONE: WebGPU 3D viewport live in Chrome
Committed `13f9a3346` (branch `0.0.19`, not pushed).
- **Phase 0.1 browser acceptance: PASSED.** `requestDevice` succeeds on current Chrome (`wasm32`) — the
  removed-limit rejection is stripped by `docs/js/webgpu-limits-shim.js` before `requestDevice`. The
  previously-"pending your hardware" items in the entry below are now resolved (Chrome accepts the device;
  demo published + browser-tested).
- **Phase 1.0 (async WebGPU unlock): DONE.** `PortalGpu::try_new_async` replaces `block_on` on the browser
  main thread; `portal_init_webgpu()` is awaited by `loadQualiaPortal` **before** `new QualiaPortal()` (so the
  first paint adopts the stashed GPU instead of grabbing a 2d context). The GPU path now engages (tier T2).
- **The black-viewport saga — three Dawn-strict WebGPU bugs** (native backends tolerate them; only exposed
  once the path actually ran in a browser). A failed-creation pipeline/bindgroup becomes *deferred-invalid*;
  binding it voids the whole command buffer so the frame (and its clear) never presents → black, with **no JS
  error**. Found via `push_error_scope(Validation)` / `pop_error_scope().await`:
  1. **Device limits** — `downlevel_webgl2_defaults()` sets `max_storage_buffers_per_shader_stage = 0`,
     invalidating both pipelines (vertex shaders read the tensor SOA / particle SSBOs). → `Limits::default()`.
  2. **WGSL** — projector `pick_id: u32` inter-stage output needs `@interpolate(flat)` (Dawn rejects non-flat
     integers; naga is lenient).
  3. **Storage offset** — tensor SOA bound at offset 32 violates the 256-byte `minStorageBufferOffsetAlignment`;
     strip the 32-byte header at upload, bind at offset 0.
- **Canvas-resize hygiene** — `render()` reconciles depth/picking/bloom to the *actual acquired frame texture*;
  `surface_size()` getter + `paint_frame` self-heal resize. **Safety net** — init validation error scope now
  surfaces otherwise-silent deferred pipeline-creation errors.
- **Phase 0.3 (re-light dev-bench): DONE** — `spatial.html` renders via `PortalGpu`, not canvas2d.
- **Verified in Chrome:** live interactive 3D viewport — bloom + ambient particle field + tensor-node
  projection + **orbit camera** (drag reframes the cluster); console clean. Rebuilt `docs/pkg/qualia` bundle.
- **Cosmetic follow-up (not broken):** nodes read as hard squares (bloom blows out the soft-dot alpha
  falloff — tunable). ~9/52 nodes visible at once = temporal scrub (t = 0.50 ± 0.08) working as designed.
- **NOT committed this session:** asset-import feature (`asset_bridge.rs` + CLI `Mesh` ingest — complete but
  not re-verified this session); planning `.md`; personal/staging files. Phase 0.2 still not started.

### 2026-06-23 — Phase 0.1 (wgpu upgrade): code-complete; all builds + tests green; browser acceptance pending
- **wgpu 0.19 → 0.20.1** in `qualia-core-db` (+ `qualia-extensions`); naga dev-dep 0.19 → 0.20. Minimal-risk
  target (drops the removed `maxInterStageShaderComponents` limit current Chrome rejects); chosen over a leap
  to 25.x so it's finishable + cheap, with your Chrome as the test oracle.
- **35 `compilation_options` inserts** (the only 0.20 API break) across compute pipelines + render
  vertex/fragment states in 7 files (`gguf_bridge`, `portal_gpu`, `lora/webgpu_lora`, `npu_ffi`,
  `modalities/calculus/gpu`, `modalities/diffusion`, `tensor/volume_gpu`).
- **Verified GREEN here:** native `cargo check`; native `cargo test --lib` (**1160 passed / 0 failed**);
  wasm32 `--features portal`; wasm32 `--features wasm-full`; native test-harness compile; **wasm-pack release +
  wasm-bindgen** portal bundle (`qualia_core_db_bg.wasm`, **492 KB**).
- **Pre-existing wasm-bundle breakages fixed (NOT caused by wgpu)** — the wasm `portal`/`full` builds were
  already broken on `modalities` / `daemon_graph` / `specialized_libs` feature-gating. Gated correctly:
  `mcp_cooperation` (lib.rs → modalities' cfg), `graph_index::with_graph_index` (native-only), `legal_compose`
  §26 proportionality + its re-export (native-only). This unblocks the demo bundle.
- **PENDING — your hardware (cannot verify here):** (1) the actual fix — Chrome accepts `requestDevice` on
  `wasm32`/`wasm64`; (2) native GPU runtime tests; (3) publish + browser-test the demo. **To publish + test:**
  run `scripts/package-qualia-wasm.ps1`, then open `docs/spatial.html`. If 2026 Chrome still rejects, bump
  wgpu past 0.20 (now a small, known increment).
- **NOT started:** Phase 0.2 (renderer consolidation into `qualia-render`) — large refactor, flagged for your
  review before autonomous execution. Phase 0.3 full re-light (the CSS-overlay / asset-stamp fix + browser
  verify).
- Note: uncommitted non-wgpu changes pre-existed in the tree (`docs/benchmark.html`, `docs/js/benchmark-live.js`,
  `webizen-desktop/.../menu.json`) — untouched by me. Nothing committed (your review first).

## Appendix — mapping
- Spec sections: RENDERER_DEFINITION §1 (projection), §2 (sense), §3 (compute), §3.1–3.4 (heterogeneous +
  perf), §4 (capabilities), §5 (pilot), §6–§7 (authoring), §8 (rails), §9 (state), §10 (path), §12 (ingest).
- STELLAR §E steps 1–4 → Phases 1–3 + §1.4. STELLAR §D → Phase 4. STELLAR §A/§F → Phase 6.
- Migration review §4 → Phase 0. ADR 0009 (ShEx) → Phase 5. Tasks: #11 (renderer), #12 (transcode), #13
  (compute-universe), #8 (CBOR-LD), #15 (frame-relative resolution).
