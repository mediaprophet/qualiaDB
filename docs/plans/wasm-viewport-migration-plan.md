# Qualia WASM Portal — Migration Plan

**Status:** `IN PROGRESS` — Qualia WASM portal shipped; tensor buffer + telemetry path live (T0/T1)  
**Created:** 2026-06-17  
**Last updated:** 2026-06-17 (Track B3 polish — Sentinel gate + 0.0.17)
**Branch target:** `0.0.17-dev` (qualiaDB) + `webizen-browser` main  
**Companion docs:** `C:\Projects\webizen-browser\BACKGROUND_VISUALISATION.md`, `10D_INTEGRATION_PLAN.md`, `10D_INTEGRATION_SUMMARY.md`

---

## Executive summary

QualiaDB’s GitHub Pages demos (`docs/`) currently use **V8 + Three.js + canvas2d** for spatial visualization and ambient telemetry. That was acceptable scaffolding while `webizen-web` and `webizen-render` matured on desktop — it is **not** the production architecture.

This plan migrates the docs site to a **single WASM artifact — Qualia**:

1. **Qualia WASM** (`qualia.js` / `qualia_bg.wasm`) — the **Semantic Subjectivity Bifurcation Portal**: one package bundling engine + viewport
2. **Qualia engine** (`qualia-core-db`) — graph, logic, spatial encode, tensor queries, telemetry *sources*, Phase-8 inference/sentinel hooks
3. **Webizen render shell** (`webizen-render`, internal) — scene contract, wgpu shaders, `SystemTelemetry`, 10D/spectral projection — **not** a separate user-facing WASM
4. **HTML shell** — thin JS glue only (load Qualia WASM, DOM, file I/O, resize)

JavaScript must not own geometry, particle animation, spectral math, or Quin encoding in the hot path.

---

## Product naming — Qualia WASM

The browser/desktop WASM package is **Qualia**, not “webizen-web” in user-facing docs, filenames, or imports.

| Public (ship) | Internal (repo) | Meaning |
|---------------|-----------------|---------|
| `qualia.js` | `webizen-web/pkg/` build output, renamed on publish | wasm-bindgen glue |
| `qualia_bg.wasm` | `webizen_web_bg.wasm` | Single combined module |
| `QualiaPortal` | `Viewport` / `WebEngine` (rename in Phase 3) | Main JS constructor |
| **Semantic Subjectivity Bifurcation Portal** | — | Full product name (docs, about pages) |
| **Qualia portal** | — | Short name |

**Why “portal”:** The WASM boundary is where *subjective* presentation (viewport, ambient field, epistemic q-states, temporal slices) bifurcates from *objective* semantic truth (`NQuin`, baked `Tensor10D`, zero-heap evaluators). Same engine runs in-process on desktop; the portal is the embeddable edge — browser, QApp, GitHub Pages — without a second semantic stack.

**Why not a separate “webizen WASM”:** Webizen remains the **render manifold host** (wgpu, shaders, Dioxus shell, Tauri). Qualia remains the **epistemic engine**. End users load **one** module named Qualia; Webizen is an implementation layer inside it, not a competing brand at the WASM edge.

**Tagline options (pick one for docs):**

- *Qualia — semantic subjectivity bifurcation portal*
- *Qualia — where meaning meets the manifold*
- *Qualia WASM — one engine, one portal, zero heap in the hot path*

**Phase 3 rename tasks:**

- [x] **N-1** `wasm-pack` out-dir publishes as `docs/pkg/qualia/` (not `webizen_web/`)
- [x] **N-2** Rust `#[wasm_bindgen(js_name = QualiaPortal)]` (crate `qualia-wasm`)
- [x] **N-3** `docs/js/qualia-shell.js` replaces `viewport-shell.js`
- [ ] **N-4** Pages badge: `Qualia WASM` + tier (`T2 · Live`, etc.)
- [ ] **N-5** API catalog entries under `qualia.*` namespace
- [ ] **N-6** Keep `webizen-web` as Cargo crate path in `webizen-browser` until optional rename to `qualia-portal`

**Phenomenal** (definition for this plan) = live 10D tensor volume drives **both** query and viewport on a **shared GPU buffer**, with ambient telemetry tied to real subsystem vitals, σ spectral coloring, epistemic q-state visuals, and zero heap in hot paths. Not a PNG slideshow, not Three.js, not mock Qualia data.

---

## 10D zero-heap thesis (why GPU is in the plan)

The 10D tensor is not a visualization gimmick — it is the **approved escape from heap-backed graph work**.

| Phase | Heap? | Where |
|-------|-------|-------|
| **Bake** (ingest, embed, ontology load) | Yes (cold path) | Build dense `Tensor10D` struct-of-arrays from `NQuin` graph |
| **Query** (evaluators, SPARQL guards, kNN) | **No** | `visit_tensor_search`, `tensor_search_into`, GPU compute mirror |
| **Display** (viewport, ambient) | **No** (CPU) | Same resident buffer; shader animation via uniforms only |

**GPU role (Tier 2+):** mmap/tensor-upload the baked volume to VRAM once. Queries become parallel filter/distance/blend ops. The viewport reads the **same buffer** — no duplicate scene graph in JS or `Vec<SceneNode>` per frame.

**Rule:** If a fallback reintroduces dynamic graph traversal (`HashMap`, `VecDeque`, Three.js mesh rebuild), it is **not** an acceptable production path — only a **degraded display tier** (see Fallback policy below).

---

## Fallback policy — yes, but tiered and honest

**Answer: yes, we need fallbacks** — but only as **explicit hardware tiers**, not silent heap regressions.

### What must always run (no fallback to JS semantics)

These stay **Qualia WASM / native Rust** on every tier:

- `NQuin` logic, modalities, N3/SHACL, GeoSPARQL
- `spatial_encode`, tensor export, zero-heap `visit_*_into` query APIs
- Telemetry *sampling* (may return zeros on edge, but contract stays 48 B)

### What degrades by tier (display + query throughput only)

| Tier | Hardware | Query execution | Viewport | Ambient viz | LLM |
|------|----------|-----------------|----------|-------------|-----|
| **T3** | QPU escrow | Classical first; async GSR patch | N/A (headless) | Off | Paused |
| **T2** | Discrete GPU, VRAM ≥ 1 GB | **GPU tensor compute** + SIMD | **Phenomenal** wgpu: 3D projector, bloom, 50k particles, σ spectral | Live telemetry | Full (ledger budgeted) |
| **T1** | iGPU / 8–16 GB RAM | SIMD `visit_tensor_search_into` | wgpu simplified: 3D nodes, 8k particles, no bloom | Live telemetry | Hybrid/throttled |
| **T0** | Edge / no WebGPU | SIMD scan, smaller buffers | **Display fallback only:** static PNG snapshot or canvas2d ambient stub; badge shown | Sliders / last known | Off or remote |
| **T0−** | WASM, `navigator.gpu` null | Same SIMD WASM | `ambient-viz.js` canvas2d + 2D node overlay; **no Three.js** | Simulated pulses OK | N/A on Pages |

### Fallback rules (non-negotiable)

1. **Never fake phenomenal** — Tier-0 UI shows a badge: `Viewport: CPU fallback` or `WebGPU unavailable`.
2. **Display ≠ engine** — canvas2d may draw particles; it must not re-implement Quin encode or graph traversal.
3. **No Three.js in any tier** — removed entirely after Phase 4; not a fallback option.
4. **Eco / Reserve modes** (desktop) — under VRAM or thermal pressure, drop bloom → particle count → frame rate → diffusion **before** dropping tensor query correctness.
5. **LLM wins scheduling** — render drops frames; inference is never silently run on a JS fallback.
6. **Pages vs desktop** — Pages targets T1–T2 in Chrome/Edge; T0 canvas2d is acceptable for GitHub Pages reach. Desktop targets T2 phenomenal as default.
7. **One WASM on Pages** — fallback does not load a second module; **`qualia_bg.wasm`** (Qualia portal) handles all tiers. Legacy `qualia_core_db.wasm` retired from spatial/playground pages after merge.

### When *not* to fallback

- Do not fall back to heap `tensor_search()` → `Vec` in evaluator hot paths (cold-path `Vec` APIs remain test/migration only).
- Do not fall back to `mock_qualia_projection()` when daemon `:4242` is reachable.
- Do not create a new wgpu device per frame when shared `GpuContext` exists.

---

## Phenomenal acceptance criteria (full checklist)

All boxes required for **PHENOMENAL** status:

### A. Data plane (Qualia / 10D)

- [ ] **P-A1** Ingest pipeline bakes `NQuin` → `Tensor10D` with real q/v/w (not hash placeholders)
- [ ] **P-A2** `Q42TensorVolume` resident as struct-of-arrays (40 B × N, mmap or GPU upload)
- [ ] **P-A3** Hot-path queries use only `visit_*_into` / GPU compute — no `Vec` in evaluator loops
- [ ] **P-A4** High-density spectral sheets (SPD / STFT) linked from σ with mmap sidecars
- [ ] **P-A5** Gravitational/thermodynamic baking stage refines x,y,z (spec §2.4) at ingest
- [x] **P-A6** `Tensor10D::from_nquin()` reads baked metadata, not object-hash xyz proxy *(geo vertex packed coords via `bake_pipeline`)*

### B. GPU back-plane

- [x] **P-B1** Single `GpuContext` per process (LLM + tensor + render + diffusion) *(shared wgpu device; render path pending)*
- [x] **P-B1b** `ComputeUniverse` orchestrator — U0/U1/U2 pinned ledger partitions on one device (Track B2)
- [x] **P-B2** `VramLedger` pre-flight before load/render; `memory_pressure` reflects reality
- [ ] **P-B3** Real `ThermalGovernor` gates render and non-critical inference
- [ ] **P-B4** Persistent `WgpuRenderer` (no per-frame `new_offscreen`)
- [x] **P-B5** `QTensorEngine` reuses shared device; no per-chat GPU init
- [x] **P-B6** `tensor_volume.wgsl` — GPU mirror of `visit_tensor_search` for Tier 2 *(CPU SIMD fallback when GPU unavailable)*

- [x] **P-B8** Graph-guided attention sparsity — U1 kNN → `AttentionRouteMask` constrains `fused_attention.wgsl` KV reads
- [x] **P-B7** Topological speculative decoding — U1 drafts via kNN; U0 `verify_topology_draft_batch` accepts prefix + Sentinel gate (`sentinel_allows_topology_draft`)
- [x] **P-B9** Async continuous context — U1 producer thread injects during U0 matmul; zero stop-and-RAG stalls in decode loop

### C. Viewport (Webizen render)

- [ ] **P-C1** `projector.wgsl` pipeline live — PGA 3D, depth buffer, true projection
- [ ] **P-C2** `epistemic.wgsl` live — q=0 collapsed vs q>0 sandbox visuals
- [ ] **P-C3** σ → CIE XYZ → sRGB in shader (not hardcoded RGB materials)
- [ ] **P-C4** `tensor_buffer.rs` → GPU instanced buffer (upload once, animate in shader)
- [ ] **P-C5** `ambient.wgsl` + bloom; driven by `render_scene_*_and_telemetry`
- [ ] **P-C6** Live Qualia neighborhood (daemon `:4242`), not mock projection
- [ ] **P-C7** Camera IPC from UI; temporal scrubber filters `t` slice
- [ ] **P-C8** Navigation: `select_node_at`, `navigate_to_node`, wavefunction collapse wired
- [ ] **P-C9** Desktop: direct wgpu surface OR 30 FPS PNG with persistent renderer (not both re-initing GPU)

### D. Telemetry / living system

- [x] **P-D1** `gguf_bridge` token loop → `llm_heat`
- [x] **P-D2** Encode/bake → `baking_crystallization`; query resolve → `logic_flashes`
- [ ] **P-D3** VRAM ledger → `memory_pressure`; mesh I/O → `network_ripple`
- [x] **P-D4** UI shows tier badge + operational mode (Full / Eco / Reserve)

### E. WASM / Pages parity

- [x] **P-E1** Single **Qualia WASM** (`qualia_bg.wasm`); no Three.js; thin `qualia-shell.js`
- [x] **P-E2** `spatial.html` encode → tensor buffer → GPU upload end-to-end *(buffer upload + σ projection in portal)*
- [x] **P-E3** Tier-0 canvas2d fallback with honest badge when WebGPU missing
- [ ] **P-E4** Same WGSL shaders as desktop (feature-gated bloom/particle count)

### F. Audio / multi-modal (phenomenal+)

- [ ] **P-F1** AudioWorklet spectral synthesis from STFT sheets (σ band)
- [ ] **P-F2** Visual + auditory share `[α, μ, σ]` truth layer per spec

**PHENOMENAL declared when:** all of **A, B, C, D, E** complete. **F** is phenomenal+ (may trail).

---

## Architectural boundary (non-negotiable)

| Layer | Owns | Zero-heap? |
|-------|------|------------|
| **Qualia** | `NQuin`, SPARQL/GeoSPARQL, modalities, `Tensor10D` semantics, WASM evaluators, telemetry *sampling* | Yes — hot paths |
| **Webizen** | `RenderScene`, `SystemTelemetry`, wgpu instancing, σ→CIE projection, bloom, audio spectral synthesis | GPU buffers; heap OK in UI |
| **JS shell** | `import init, { QualiaPortal }`, canvas mount, sliders → `Float32Array` | N/A — no semantic work |
| **Qualia WASM portal** | Bundles engine + Webizen render; exposes `QualiaPortal::tick`, tensor upload, telemetry | GPU buffers; portal API only |

**Spectral truth** (`α`, `μ`, `σ` + linked SPD/STFT sheets) lives in Qualia/Q42. **RGB/speaker output** is Webizen last-mile projection inside the Qualia WASM module.

---

## Current state audit

### qualiaDB `docs/` (Pages)

| Asset | Role today | Target |
|-------|------------|--------|
| `spatial.html` + `js/spatial-demo.js` | Qualia WASM portal + tensor projection + GeoSPARQL WASM | Phenomenal wgpu path (Track C) |
| `js/ambient-viz.js` | canvas2d telemetry prototype (2400 particles) | wgpu ambient shader; JS kept as Tier-0 fallback only |
| `playground/qualia_core_db.wasm` | Logic, N3, SHACL, optional WebGPU LLM | **Merged into** `pkg/qualia/qualia_bg.wasm` (single portal) |
| `playground/anatomy.js` | Three.js anatomy viewer | Phase 4 — same viewport pattern |
| `modalities-showcase.html` | Canvas2d modality visuals | Phase 5 — optional wgpu overlay |
| CDN `three.js/r128` on `spatial.html` | ~~External dep~~ | **Removed** (2026-06-17) |

### webizen-browser

| Crate | State | Gap |
|-------|-------|-----|
| `webizen-render` | Desktop wgpu + `SystemTelemetry` (48 B) + 10D scene contract ✅ | `#[cfg(not(wasm32))]` gates wasm; **active path is 2D** (`screen.wgsl`), not 3D |
| `webizen-web` (→ **Qualia WASM**) | Single-package portal; canvas2d stub; depends on qualia-core-db | Publish as `qualia.js`; add `webizen-render` inside |
| `webizen-desktop` | PNG preview over `webizen://` protocol; render daemon **implemented but unregistered** | **2D GPU compositor**, not phenomenal 3D viewport |
| `BACKGROUND_VISUALISATION.md` | Design doc | `ambient.wgsl` real; telemetry not wired to live render loop |

### webizen-desktop rendering reality (2026-06-17 audit)

**What is real today:**

- wgpu offscreen render → PNG → `<img src="webizen://localhost/render/preview.png">`
- `screen.wgsl` — 2D clip-space discs, lines, filled polygons (no depth buffer)
- `ambient.wgsl` — ~50k instanced particles with 3D positions, projected to 2D
- `scene_contract.rs` — `Tensor10DProjection`, epistemic state, spectral helpers

**What is scaffolding (written, not on the live path):**

| Asset | Location | Issue |
|-------|----------|-------|
| `projector.wgsl` | PGA motor 3D vertex shader | Pipeline never created |
| `epistemic.wgsl` | LOD / epistemic fragment shader | Not bound to any pipeline |
| `tensor_buffer.rs` | Zero-copy 10D binary views | Not connected to render loop |
| `motor_encoder.rs` | 64-byte PGA `Motor` layout | Unused in active pipeline |
| `toggle_render_loop` | 30 FPS Qualia fetch daemon | **Not registered** in Tauri invoke handler |
| `navigate_to_node` / `select_node_at` | Hit-testing + navigation | Commands exist, UI unwired |
| Live Qualia scene | `fetch_local_neighborhood` | Falls back to `mock_qualia_projection()` |
| 10D tensor on nodes | `scene_to_contract.rs` | Always `Tensor10DProjection::default()` |
| Camera orbit/zoom | `WgpuRenderer` CPU math | UI handlers local-only; render uses `Camera::default()` |

**Verdict:** Desktop has **real wgpu**, but it is a **2D preview compositor** with a particle backdrop — not the phenomenal 10D volumetric viewport the specs describe.

### GPU back-plane / concurrency reality (2026-06-17 audit)

**The problem:** LLM, Qualia compute, and display each create **independent GPU contexts** with no arbitration.

| Subsystem | GPU init pattern | Risk |
|-----------|------------------|------|
| `QTensorEngine` (`gguf_bridge.rs`) | New `wgpu::Instance` + device per engine; Windows adds separate `DmlDevice` | Competes with render on same adapter |
| `WgpuRenderer` (`wgpu_renderer.rs`) | **New device per** `render_scene_png*` call (`new_offscreen`) | 30 FPS loop = repeated init/teardown |
| `WgpuDiffusionBackend` (`webizen-runtime`) | Third independent wgpu device | Dead 3D fields (`depth_texture`, etc.) |
| LLM inference thread | `QTensorEngine::new()` per chat turn | No engine pool despite resident mmap |

**What does not exist yet:**

- [ ] Global `GpuContext` / shared `wgpu::Device` across inference + render + diffusion
- [ ] VRAM ledger (unify DXGI probe, KV cache 448 MiB cap, render target sizes)
- [ ] Real `ThermalGovernor` (production uses `NullThermalGovernor` → always `Cool`)
- [ ] Render back-pressure when `ModelLifecycle::Active` or VRAM pressure high
- [ ] `telemetry_hooks` fed from `gguf_bridge` token loop → `render_scene_png_with_time_and_telemetry`
- [ ] Persistent `WgpuRenderer` across frames (not per-PNG re-init)
- [ ] `hardware_tier.rs` GPU detection (stubs return `false` → always Tier 0)

**Design docs ahead of code:** operational modes (Full / Eco / Reserve), `q42-10d-volumetric-tensor-spec.md` §3 telemetry-aware dispatch, `BACKGROUND_VISUALISATION.md` `render_scene_ambient()` — mostly unimplemented.

### qualia-core-db WASM exports (today)

Present: `parse_n3logic_wasm`, `validate_shacl_constraint_wasm`, `forward_chain_wasm`, `initialize_webgpu_engine`, …  
**Spatial demo exports (2026-06-17):** `spatial_encode_wasm`, `geosparql_operation_wasm`, `export_tensor_buffer_wasm`, `sample_browser_telemetry_wasm` — in `qualia_bg.wasm`

---

## Target architecture (phenomenal)

```
┌──────────────── BAKE (cold, heap OK) ────────────────────────┐
│  NQuin graph → embedding → Q42TensorVolume (SOA, mmap)     │
│  + spectral sidecars (SPD/STFT) + physics relaxation stage   │
└──────────────────────────┬───────────────────────────────────┘
                           │ upload once
┌──────────────────────────▼───────────────────────────────────┐
│  GpuContext + VramLedger (single device, shared queues)        │
│  ┌────────────────┬─────────────────┬──────────────────────┐ │
│  │ tensor_buffer  │ LLM KV/weights  │ render targets       │ │
│  │ (10D SOA VRAM) │ (budgeted)      │ (viewport+bloom)     │ │
│  └────────────────┴─────────────────┴──────────────────────┘ │
└──────────────────────────┬───────────────────────────────────┘
                           │
        ┌──────────────────┼──────────────────┐
        ▼                  ▼                  ▼
  tensor_volume.wgsl   gguf_bridge       projector.wgsl
  (parallel query)     (LLM, prio 1)     + ambient.wgsl
        │                  │                  │
        └──────────────────┴──────────────────┘
                           │
┌──────────────────────────▼───────────────────────────────────┐
│  Shell: spatial.html / webizen-desktop (DOM only)              │
│  QualiaPortal::tick() — ≤48 B telemetry + camera uniform       │
└────────────────────────────────────────────────────────────────┘
```

**Per-frame CPU work:** copy ≤48 B telemetry + camera uniform. **No** per-vertex JS updates. **No** new `Vec` per query.

**Tier dispatch:** `HardwareTierDispatcher` selects SIMD (T0–T1) vs GPU VRAM (T2) vs throttled (Eco).

---

## Phase 0 — Plan & CI prerequisites

**Goal:** Repos can build and publish the combined WASM artifact.

- [ ] **0.1** Add this plan to PR template / AGENTS.md handoff pointer (`docs/plans/wasm-viewport-migration-plan.md`)
- [ ] **0.2** Document wasm-pack build command in plan appendix (see below)
- [ ] **0.3** Add `docs/pkg/` gitignore exception or CI step to copy `webizen-web/pkg/*` into `docs/pkg/`
- [ ] **0.4** Verify `qualia-core-db` builds for `wasm32-unknown-unknown` with required features
- [ ] **0.5** Smoke-test Pages locally: `npx serve docs` + WASM MIME types

**Exit criteria:** `wasm-pack build` succeeds in `webizen-web`; qualiaDB `cargo check -p qualia-core-db` passes.

---

## Phase 1 — Shared contracts (both repos)

**Goal:** One byte layout for scene + telemetry across desktop and browser.

**Repo: webizen-browser (`webizen-render`)**

- [ ] **1.1** Confirm `SystemTelemetry` is 48 bytes / `Pod` — document field order in plan appendix ✅ (already in `telemetry.rs`)
- [ ] **1.2** Export `Tensor10DProjection`, `RenderScene`, `SceneNode` as `#[repr(C)]` stable layouts for WASM FFI
- [ ] **1.3** Add `scene_contract::tensor_buffer_header()` — magic, version, node_count, stride for binary uploads
- [ ] **1.4** Add unit test: desktop renderer + WASM deserialize same `RenderScene` bytes

**Repo: qualiaDB (`qualia-core-db`)**

- [ ] **1.5** Add `tensor/mod.rs` → `export_tensor_slice_wasm(out: &mut [u8]) -> usize` (bounded, zero-heap)
- [ ] **1.6** Add `spatial_wasm.rs` module with JSON-in/JSON-out wrappers (cold path only) for:
  - `spatial_encode_wasm`
  - `geosparql_operation_wasm`
  - `spatial_bbox_wasm` / `spatial_convex_hull_wasm` (delegate to existing native ops when available)
- [ ] **1.7** Add `sample_browser_telemetry_wasm() -> JsValue` — maps SlgArena pressure + last op timing → 0–1 floats

**Exit criteria:** Contract structs have stable sizes; round-trip test desktop ↔ bytes ↔ WASM parse.

---

## Phase 2 — webizen-render on wasm32

**Goal:** Same WGSL shaders run in browser WebGPU.

**Repo: webizen-browser**

- [ ] **2.1** Add `webizen-render` feature flag `web` (enables wgpu on `wasm32`)
- [ ] **2.2** Un-gate `WgpuRenderer` surface path for `wasm32` (canvas → `wgpu::Surface`)
- [ ] **2.3** Port ambient particle shader from `BACKGROUND_VISUALISATION.md`:
  - static instanced position buffer (upload once)
  - `SystemTelemetry` uniform @ group(0) binding(1)
  - time uniform
  - optional bloom pass (Tier 1+ browsers)
- [ ] **2.4** Port spectral vertex color: `sigma` → CIE XYZ → sRGB in WGSL (reuse `PROJECTOR_WGSL` / epistemic shader)
- [ ] **2.5** Implement tier dispatch in `Viewport::new()`:
  - WebGPU present + `adapter.limits` OK → Tier 1/2 wgpu path
  - WebGPU missing → `ViewportError::WebGpuUnavailable` → shell loads `ambient-viz.js` (display only)
  - Insufficient VRAM → Tier 1 reduced preset (see Appendix E)
- [ ] **2.6** Bench: 30 FPS cap, ≤48 B telemetry upload per frame, zero `Vec` in `tick()` hot path

**Exit criteria:** `webizen-render` example renders instanced particles in browser via `wgpu` on Chrome/Edge; desktop path unchanged.

---

## Phase 3 — Qualia WASM (single portal package)

**Goal:** One published artifact — **`qualia.js` + `qualia_bg.wasm`** — Semantic Subjectivity Bifurcation Portal.

**Repo: webizen-browser (`webizen-web` crate → publish as Qualia)**

- [ ] **3.1** Add dependency: `webizen-render = { path = "../webizen-render", features = ["web"] }`
- [ ] **3.2** Replace canvas2d stub with `QualiaPortal` (wasm-bindgen name; Rust struct may be `QualiaPortal` or `PortalEngine`):
  ```rust
  #[wasm_bindgen(js_name = QualiaPortal)]
  pub struct QualiaPortal { /* qualia engine handle + wgpu viewport */ }

  #[wasm_bindgen]
  impl QualiaPortal {
      pub fn new(canvas: HtmlCanvasElement) -> Result<QualiaPortal, JsValue>;
      pub fn resize(&mut self, width: u32, height: u32);
      pub fn tick(&mut self, dt_ms: f32);
      pub fn set_telemetry(&mut self, floats: &[f32]); // 48 B SystemTelemetry
      pub fn upload_tensor_buffer(&mut self, bytes: &[u8]);
      pub fn spatial_encode(&self, json: &str) -> JsValue;
      pub fn load_q42(&mut self, bytes: &[u8]) -> JsValue;
      pub fn tier(&self) -> u8; // 0–2 for UI badge
  }
  ```
- [ ] **3.3** Re-export all former `qualia_core_db` WASM symbols from same module (portal = single import)
- [ ] **3.4** `wasm-pack build --target web --out-dir pkg-qualia`; post-step rename to `qualia.js` / `qualia_bg.wasm`
- [ ] **3.5** Copy → `qualiaDB/docs/pkg/qualia/` in CI
- [ ] **3.6** Naming tasks **N-1** through **N-6** (see Product naming)

**Repo: qualiaDB**

- [ ] **3.7** Add `scripts/package-qualia-wasm.ps1` (replaces `package-webizen-wasm.ps1`)
- [ ] **3.8** Add `docs/js/qualia-shell.js` — `import init, { QualiaPortal } from '../pkg/qualia/qualia.js'`
- [ ] **3.9** Retire duplicate `playground/qualia_core_db.js` load on pages that mount the portal

**Exit criteria:** `docs/spatial.html` loads **only** `pkg/qualia/`; badge reads `Qualia WASM · T1`.

---

## Phase 4 — spatial.html migration (flagship demo)

**Goal:** Remove Three.js; spatial demo is WASM-native end-to-end.

**Repo: qualiaDB `docs/`**

- [x] **4.1** Remove `<script three.js>` from `spatial.html`
- [x] **4.2** Replace `spatial-demo.js` Three.js path with `qualia-shell.js` + `QualiaPortal` API
- [ ] **4.3** Wire geometry controls → `spatial_encode_wasm` → `upload_scene` (GPU buffer, not JS mesh)
- [ ] **4.4** Wire GeoSPARQL tab → `geosparql_operation_wasm` (remove JS polygon fallback; WASM required on all tiers)
- [ ] **4.5** Wire Spatial Ops tab → WASM bbox/hull/triangulate
- [ ] **4.6** Telemetry sliders → `set_telemetry(Float32Array)`; encode/spatial ops → `pulse` via WASM
- [ ] **4.7** Mark `ambient-viz.js` deprecated; load only when `QualiaPortal::new` returns WebGpuUnavailable
- [ ] **4.8** Q42 10D tab: live readout of tensor fields from uploaded scene buffer
- [ ] **4.9** Update in-page architecture callout to reflect implemented state (not “future”)

**Exit criteria:** `spatial.html` works on GitHub Pages with no Three.js; hard-refresh shows wgpu particles + encoded geometry.

---

## Phase 5 — Broader docs site updates

**Goal:** Pages consistently describe and use the WASM viewport pattern.

### 5A — Pages to update

| Page | Change | Priority |
|------|--------|----------|
| `spatial.html` | Full migration (Phase 4) | P0 |
| `index.html` | Feature card: **Qualia WASM Portal** (semantic subjectivity bifurcation) | P0 |
| `advanced-features.html` | Add viewport + spectral payload section | P1 |
| `network_webizen.html` | Link Webizen shell vs Qualia engine; ambient viz | P1 |
| `scientific-computing.html` | EM spectrum → `[α,μ,σ]` truth layer | P1 |
| `science-playground.html` | Optional shared `Viewport` embed | P2 |
| `modalities-showcase.html` | Telemetry pulse hooks → WASM when loaded | P2 |
| `playground/index.html` | Note single-package WASM; link spatial demo | P1 |
| `playground/anatomy.js` | Migrate off Three.js (OrbitControls → WASM camera) | P2 |
| `api.html` / `api-explorer/catalog.js` | Document `Viewport` + `spatial_*_wasm` exports | P1 |
| `zero-heap-compliance.html` | “JS shell only” policy for demos | P1 |
| `menu.json` | Optional “Spatial Viewport” nav highlight | P2 |

### 5B — New docs (optional)

- [ ] **5.10** `docs/manuals/qualia-wasm-portal.md` — portal concept, build, deploy, fallback tiers
- [ ] **5.11** `docs/plans/wasm-viewport-migration-plan.md` — this file; update checkboxes per PR

### 5C — Cross-repo doc sync

- [ ] **5.12** Update `webizen-browser/BACKGROUND_VISUALISATION.md` status: “implemented in webizen-render Phase 2”
- [ ] **5.13** Add pointer in `AGENTS.md` §7 session notes when phases complete
- [ ] **5.14** Update `Claude.md` §8 known inaccuracies: spatial demo no longer Three.js

**Exit criteria:** No docs page claims Three.js is the spatial engine; API catalog lists viewport exports.

---

## Phase 6 — Telemetry bridge (live vitals)

**Goal:** Demo telemetry reflects real WASM work, not sliders only.

- [ ] **6.1** Map encode timing → `baking_crystallization`
- [ ] **6.2** Map GeoSPARQL/op queue → `logic_flashes`
- [ ] **6.3** Map WASM memory / mount progress → `memory_pressure`
- [ ] **6.4** (Desktop) Port `telemetry_hooks.rs` mapping to `sample_browser_telemetry_wasm` subset
- [ ] **6.5** (Future) SSE/WS from daemon `:4242` → Pages telemetry uniform (behind opt-in dev mode)

**Exit criteria:** Clicking “Encode to Quins” visibly drives shader without manual slider movement.

---

## Phase 8 — GPU back-plane (LLM + Qualia + display coexistence)

**Goal:** One adapter, one policy — inference, graph compute, and viewport share VRAM and queues without stomping each other.

**Priority:** **P0 for desktop phenomenal path** — should precede or run parallel with Phase 2–3, not after Pages migration.

**Repo: qualiaDB (`qualia-core-db` + `qualia-client-core`)**

- [ ] **8.1** Add `gpu_context.rs` — process-wide `GpuContext { device, queue, adapter_info, vram_ledger }`
- [ ] **8.2** Refactor `QTensorEngine::try_new()` to accept `&GpuContext` (or `Arc<GpuContext>`) instead of creating a new instance
- [ ] **8.3** Implement `VramLedger` — track: GGUF staging, KV cache (448 MiB cap), render targets, diffusion buffers; expose `pressure: f32` for `SystemTelemetry.memory_pressure`
- [ ] **8.4** Wire real `ThermalGovernor` (Windows DXGI / sysinfo thermal stub → trait impl); replace `NullThermalGovernor` in `model_lifecycle.rs`
- [ ] **8.5** `orchestrate_inference()` + render daemon: on `ThermalStatus::Critical`, pause render loop and block non-critical intents
- [ ] **8.6** Persist one `QTensorEngine` per process (or small pool) — stop `QTensorEngine::new()` per chat thread when resident mmap exists
- [ ] **8.7** Export `sample_gpu_telemetry()` for Webizen `telemetry_hooks`

**Repo: webizen-browser**

- [ ] **8.8** `WgpuRenderer` accepts shared `Arc<wgpu::Device>` + `Arc<wgpu::Queue>` from `GpuContext`
- [ ] **8.9** Store persistent offscreen `WgpuRenderer` in `PreviewState` — reuse across 30 FPS loop
- [ ] **8.10** Register `toggle_render_loop` in Tauri invoke handler; gate on `VramLedger` + `ModelLifecycle`
- [ ] **8.11** Render daemon calls `render_scene_png_with_time_and_telemetry()` (not the without-telemetry variant)
- [ ] **8.12** Wire `telemetry_hooks::increment_inference_counter()` from qualia token generation
- [ ] **8.13** `webizen-runtime` diffusion backend shares same `GpuContext` (or separate queue on same device)

**Scheduling policy (to implement in `gpu_scheduler.rs` or `GpuContext`):**

```
Priority queue (same physical device):
  1. LLM token generation (UserInteractive QoS, pre-empts render)
  2. Qualia graph/query bursts (short, bounded)
  3. Viewport present (30 FPS cap, drops frames under pressure)
  4. Ambient particles (always GPU, cheapest — runs in viewport pass)
  5. Diffusion compute (background, Eco mode disables)
```

**Exit criteria:** Single `adapter.request_device()` per process; LLM + 30 FPS preview run concurrently without OOM; telemetry `llm_heat` moves ambient shader during inference.

---

## Phase 9 — Phenomenal viewport (desktop + shared shaders)

**Goal:** True 3D volumetric viewport — same shader core as WASM Pages.

**Depends on:** Phase 8, Phase 10, Phase 1

- [ ] **9.1** Wire `projector.wgsl` — PGA motors, depth buffer, 3D clip from tensor xyz
- [ ] **9.2** Wire `epistemic.wgsl` — q-state LOD, collapsed vs sandbox differentiation
- [ ] **9.3** `tensor_buffer.rs` → `wgpu::Buffer` instanced upload (SOA stride = 40 B `Tensor10D`)
- [ ] **9.4** `scene_to_contract.rs` reads baked volume, not `Tensor10DProjection::default()`
- [ ] **9.5** Live `fetch_local_neighborhood` via daemon `:4242`; mock only when daemon offline (badge)
- [ ] **9.6** IPC camera + temporal slice from Dioxus → renderer uniforms each frame
- [ ] **9.7** Register `toggle_render_loop`, `navigate_to_node`, `select_node_at`; wire `RenderPreview`
- [ ] **9.8** Bloom post-pass (Tier 2 only; auto-disable Eco mode)
- [ ] **9.9** Tauri wgpu child surface (Tier 2 default); PNG protocol Tier 1 fallback with persistent renderer
- [ ] **9.10** Operational modes in UI: Full / Eco / Reserve (`gpu_context::OperationalMode`)
- [ ] **9.11** Hit-testing against GPU depth or CPU projected bounds (zero-heap `&mut [u32]` pick buffer)

**Exit criteria:** P-C1 through P-C9 satisfied on desktop T2 hardware.

---

## Phase 10 — 10D bake pipeline + VRAM residency (zero-heap core)

**Goal:** Make the 10D tensor the primary query surface; GPU holds the resident volume.

**Depends on:** Phase 1 contracts; parallel with Phase 8

**Repo: qualiaDB (`qualia-core-db`)**

- [ ] **10.1** `tensor/bake_pipeline.rs` — ingest stages: NQuin scan → embed xyz → assign q,v,w → spectral link → physics relaxation (spec §2.4)
- [ ] **10.2** Replace `from_nquin` hash-proxy xyz with baked coordinates from pipeline output
- [ ] **10.3** Emit mmap-ready SOA: `tensors.bin` + `index.bin` + optional `spectral/` sidecars
- [ ] **10.4** Harden hot path: audit call sites; ban `tensor_search()` → `Vec` from evaluators (grep CI rule)
- [ ] **10.5** `hardware_tier.rs` — real `has_gpu()` (DXGI / wgpu adapter probe); wire `ExecutionStrategy::GPUVRAM`
- [ ] **10.6** `tensor_volume.wgsl` + `tensor_query_dispatch()` — GPU kNN / radius filter / w-manifold mask
- [ ] **10.7** `GpuContext` maps tensor SOA to `wgpu::Buffer` with persist flag; ledger accounts bytes
- [ ] **10.8** Daemon `:4242` endpoint `GET /tensor/slice` or binary WS for viewport upload (zero JSON in hot path)

**Repo: webizen-browser**

- [ ] **10.9** Viewport consumes binary tensor header from Phase 1.3 — no JSON scene round-trip
- [ ] **10.10** Render path reads tensor buffer directly; `RenderScene` becomes a view, not a duplicate copy

**Exit criteria:** P-A1 through P-A6 and P-B6 satisfied; graph kNN query runs on GPU without Rust heap allocation.

---

## Phase 11 — Phenomenal+ (audio, science, docs polish)

**Goal:** Multi-modal spectral fidelity and public docs match implemented reality.

- [ ] **11.1** AudioWorklet + `audio_contract.rs` spectral synthesis (P-F1, P-F2)
- [ ] **11.2** `scientific-computing.html` — live blackbody / σ band demo from tensor sheet
- [ ] **11.3** `docs/manuals/webizen-wasm-viewport.md` — tiers, fallback badges, build guide
- [ ] **11.4** `zero-heap-compliance.html` — bake vs query vs display table
- [ ] **11.5** CI: `phenomenal-checklist.mjs` asserts P-* flags from smoke tests
- [ ] **11.6** Remove `ambient-viz.js` from default bundle (Tier-0 lazy-load only)
- [ ] **11.7** Delete `scene.rs` duplicate contract in webizen-render

**Exit criteria:** PHENOMENAL+; all P-* including F-*; docs never overclaim.

---

## Phase 7 — CI, size budget, and release

- [ ] **7.1** CI job: `wasm-pack build` + assert `webizen_web_bg.wasm` < **8 MB** gzip budget (adjust after first build)
- [ ] **7.2** CI job: `docs/tests/` suite includes viewport boot smoke (headless Playwright or wasm bind test)
- [ ] **7.3** `scripts/package-flutter-windows.ps1` unaffected; separate `scripts/package-docs-wasm.ps1`
- [ ] **7.4** GitHub Pages deploy: ensure `application/wasm` MIME + COOP/COEP headers if needed for threads
- [ ] **7.5** Release note in `docs/RELEASE_NOTES_*.md`

---

## WASM API contract (Qualia portal — Pages shell)

### JavaScript surface (thin)

```javascript
import init, { QualiaPortal, init_panic_hook } from './pkg/qualia/qualia.js';

await init();
init_panic_hook?.();

const canvas = document.getElementById('qualia-portal');
const portal = new QualiaPortal(canvas);

// Resize
new ResizeObserver(() => portal.resize(canvas.clientWidth, canvas.clientHeight)).observe(canvas.parentElement);

// Telemetry (48 bytes = 12 f32)
const telem = new Float32Array(12);
telem[0] = 0.5; // memory_pressure
portal.set_telemetry(telem);

// Tier badge
document.getElementById('wasm-text').textContent = `Qualia WASM · T${portal.tier()}`;

// Loop
function frame() { portal.tick(16.67); requestAnimationFrame(frame); }
requestAnimationFrame(frame);
```

### Rust exports (qualia-core-db, re-exported through Qualia portal module)

| Export | Purpose |
|--------|---------|
| `spatial_encode_wasm(json)` | Geometry → Quin + tensor buffer bytes |
| `geosparql_operation_wasm(json)` | WKT + op → result |
| `spatial_native_op_wasm(json)` | bbox / hull / triangulate |
| `sample_browser_telemetry_wasm()` | Normalized vitals for shader |
| `export_tensor_slice_wasm(max_nodes)` | Binary tensor buffer for GPU upload |

---

## Risks and mitigations

| Risk | Mitigation |
|------|------------|
| WebGPU unavailable on older Safari | Tier-0 **display** fallback only (`ambient-viz.js` lazy); WASM semantics unchanged; badge |
| Fallback creep (Three.js, heap graphs) | CI ban: no `three.js` in `docs/`; no `Vec` in tensor hot-path call graph |
| WASM size bloat (Qualia + render) | Feature-gate modalities in `web` build; `wasm-opt -Oz`; split `webizen_web_lite.wasm` if > 8 MB gzip |
| Cross-repo path dependency | `package-webizen-wasm.ps1` |
| `wgpu` on wasm32 immature | Native shader tests first; desktop T2 is reference |
| COOP/COEP on GitHub Pages | Single-threaded wgpu default; threads optional Tier 2 |
| Double WASM load | Only `qualia_bg.wasm` on spatial page |
| LLM + render OOM | `VramLedger` pre-flight; Eco degrades viewport before dropping inference |
| Mock data ships as default | Daemon probe + UI badge `Live` / `Offline` / `CPU fallback` |

---

## Verification checklist (acceptance)

### Minimum ship (Track A — not yet phenomenal)

- [ ] `spatial.html` — zero Three.js CDN requests
- [ ] WebGPU T1: ≥24 FPS ambient on mid-range laptop
- [ ] Encode → WASM → GPU buffer without JS vertex loops
- [ ] GeoSPARQL `"backend": "wasm"` on all tiers
- [ ] Tier-0 badge visible when WebGPU missing

### Phenomenal ship (Tracks B + C — required for PHENOMENAL status)

- [ ] All **P-A** through **P-E** boxes checked (see Phenomenal acceptance criteria)
- [ ] LLM + 30 FPS viewport concurrent without OOM (T2, 6 GB+ VRAM test rig)
- [ ] `llm_heat` animates ambient field during live inference without slider
- [ ] Live daemon neighborhood — zero mock nodes when `:4242` healthy
- [ ] `tensor_volume.wgsl` kNN matches `visit_tensor_search_into` on test fixture (±ε)
- [ ] Eco mode: bloom off, particles reduced, queries still correct
- [ ] `cargo test -p qualia-core-db --lib` + `cargo test -p webizen-render` pass

---

## Implementation order (recommended PR stack)

### Track A — Pages WASM viewport (can start now)

```
PR-A1  qualiaDB:  spatial_wasm.rs + tensor export + tests
PR-A2  webizen:   webizen-render `web` feature + wasm32 wgpu surface
PR-A3  webizen:   ambient WGSL + SystemTelemetry uniform (shared with desktop)
PR-A4  webizen:   QualiaPortal API + wasm-pack → docs/pkg/qualia/
PR-A5  qualiaDB:  qualia-shell.js + spatial.html migration + naming N-1..N-5
PR-A6  qualiaDB:  docs site sweep (index, api, features, zero-heap page)
PR-A7  both:      telemetry bridge + CI wasm build
```

### Track B — GPU back-plane + 10D VRAM (blocks phenomenal; lead Track A)

```
PR-B1  qualiaDB:  GpuContext + VramLedger
PR-B2  qualiaDB:  QTensorEngine adopts shared device
PR-B3  qualiaDB:  tensor/bake_pipeline.rs + real from_nquin coords
PR-B4  qualiaDB:  hardware_tier real GPU detect + tensor_volume.wgsl
PR-B5  webizen:   persistent WgpuRenderer + register toggle_render_loop
PR-B6  both:      telemetry_hooks ↔ gguf_bridge ↔ ambient shader
```

### Track B2 — Compute universes (Qualia-native, not generic GPU)

**Objective:** Eliminate VRAM thrashing and pipeline stalls by transitioning the single `wgpu::Device` from temporal time-slicing to persistent, logically partitioned **Compute Universes**. U0 (LLM), U1 (QualiaDB/Tensor10D), and U2 (Viewport) operate as concurrent, isolated execution planes on one adapter.

**Architecture constraints:**

- **Single physical adapter** — all universes share `shared_gpu().device`; no multiple `GPUDevice` instantiation.
- **Pinned residency** — universes bind to pre-allocated `VramLedger` slots; SOA tensor blob stays resident and is never evicted to serve another universe.
- **Queue-level concurrency** — dispatch to separate logical queue lanes (`LlmCompute` / `TensorCompute` / `ViewportRender`) for driver scheduling overlap.
- **Zero-copy state** — cross-universe communication via shared buffer visibility + lock-free SPSC rings (Phase-8 bifurcation).

**Milestones:**

| ID | Deliverable | Status |
|----|-------------|--------|
| **B2.1** | `ComputeUniverse` enums tag queue submissions and memory requests (`gpu_context.rs`) | ✅ |
| **B2.2** | `VramLedger` + `UniverseOrchestrator::from_total_budget(total, mode)` — consecutive `VramByteRange` pins per Full / Eco / Reserve | ✅ |
| **B2.3** | Dispatch routes LLM / `tensor_volume.wgsl` / viewport render through `queue_for_lane()` | 🟡 logical lanes wired; multi-queue when adapter exposes |
| **B2.4** | U1 `visit_tensor_search_into` → `ContextInjectRing` → U0 decode (`push_tensor_search_hits`) | ✅ |

**Concept:** One physical `wgpu::Device` (`shared_gpu()`), three **parallel universes** as pinned VRAM domains + queue lanes — not multiple `requestDevice()` calls. This is **not** a portable GPU abstraction; it maps directly to QualiaDB primitives:

| Qualia primitive | Universe | Mechanism |
|------------------|----------|-----------|
| **Graph–tensor duality** (`NQuin` ↔ `Tensor10D` SOA) | U1 (+ U2 read, U0 read) | `GraphTensorSubstrate` — one resident blob; kNN updates index into same VRAM pin |
| **Phase-8 bifurcation** (SPSC rings) | U0 ↔ Sentinel ↔ U1 | `Phase8Channel`: LogitUpstream, ControlDownstream, **ContextInject** (U1→U0) |
| **Sentinel governance** | U0 + U1 boundary | `DenyRollback` on control ring before next matmul; deontic match vs U1 topology |
| **VramLedger pins** | U0 / U1 / U2 | `can_allocate_in_universe()`; Reserve caps U2, U0 KV stays Full |

**Code:** `compute_universe.rs` (`UniverseFabric`, `ContextInjectRing`, `AttentionRouteMask`) + `gpu_context.rs` (ledger slots).

| Universe | Role | Ledger slots | Queue lane |
|----------|------|--------------|------------|
| **U0** `LlmInference` | KV + matmul + weight staging | `LlmKvCache`, `LlmWeightStaging` | `LlmCompute` |
| **U1** `Tensor10D` | Baked SOA volume, kNN, attention bitmask | `Tensor10D` | `TensorCompute` |
| **U2** `Viewport` | Projector, ambient, bloom | `Viewport` | `ViewportRender` |

**Implemented (2026-06-17):** `gpu_context.rs` — `ComputeUniverse`, `VramLedgerSlot`, `VramByteRange`, `QueueLane`, `UniversePartition`, `UniverseOrchestrator` (mode-aware), `can_allocate_in_universe()`, `SharedGpuContext::queue_for_universe()`, per-universe `effective_mode()` (U0 wins under Reserve). `gguf_bridge::ensure_kv_cache` pre-flights U0 pin.

**Budget splits (mode-aware):**

| Mode | U0 | U1 | U2 |
|------|----|----|-----|
| **Full** | 55% | 25% | 15% |
| **Eco** | 60% of remainder after U2=25% | remainder | 25% |
| **Reserve** | 50% of remainder after U2=10% | remainder | 10% (viewport off) |

```
PR-B7  qualiaDB:  wire gguf_bridge pre-flight → can_allocate_in_universe(U0)
PR-B8  qualiaDB:  tensor_volume.wgsl dispatch on QueueLane::TensorCompute
PR-B9  webizen:   WgpuRenderer on QueueLane::ViewportRender (same device)
PR-B10 qualiaDB:  Phase-8 continuous RAG — U1 kNN → NQuin ring → Sentinel (no stop-and-fetch)
PR-B11 qualiaDB:  10D attention bitmask export for fused attention kernel (U1 → U0)
PR-B12 qualiaDB:  TopologyDraftRing + TopologyDraftBatch (B3.1a)
PR-B13 qualiaDB:  U1 extrapolate_topology_draft — 10D trajectory → concept hashes (B3.1b)
PR-B14 qualiaDB:  gguf_bridge verify_topology_draft_batch + Sentinel accept (B3.1d)
PR-B15 qualiaDB:  build_attention_route_mask — kNN → mask per decode step (B3.2a) ✅
PR-B16 qualiaDB:  fused_attention.wgsl masked KV + AttentionGpuParams (B3.2c) ✅
PR-B12 qualiaDB:  TopologyDraftRing + TopologyDraftBatch (B3.1a) ✅
PR-B13 qualiaDB:  TopologyDraftMapper + extrapolate_topology_draft_mapped (B3.1b/c) ✅
PR-B14 qualiaDB:  verify_topology_draft_batch + llm_agent accept loop (B3.1d) ✅
PR-B8  qualiaDB:  tensor_volume.wgsl + volume_gpu.rs GPU kNN (B8) ✅
PR-B17 qualiaDB:  start_tensor_search_producer background thread (B3.3a) ✅
PR-B18 qualiaDB:  KvInjectRing for pre-baked daemon slices (B3.3c, optional)
```

**Tier degradation (same logical orchestration):**

| Tier | U0 | U1 | U2 |
|------|----|----|-----|
| T2 | GPU queues + pinned VRAM | GPU tensor compute | wgpu phenomenal |
| T1 | Shared device, throttled | SIMD `visit_*_into` | simplified wgpu |
| T0 | WASM CPU / remote | SIMD WASM | canvas2d portal |

**Exit criteria:** Viewport rotation never exceeds U2 cap; KV cache (U0) never evicted by U2; continuous RAG injects via SPSC ring without blocking decode.

### Track B3 — U1→U0 inference acceleration (research-aligned, Qualia-native)

**Thesis:** Standard 2025/2026 LLM acceleration (speculative decoding, structured sparsity, continuous batching) assumes a **second draft model**, **dense attention**, or **blocking RAG fetches**. QualiaDB already has the substrate to bypass all three — **Compute Universes + Graph–Tensor duality + Phase-8 SPSC rings** — without loading extra weights or heap-backed retrieval.

| SOTA pattern (2025/2026) | Qualia-native replacement | Existing hook |
|--------------------------|---------------------------|---------------|
| Draft model (Cassandra, PicoSpec) | **10D topology as drafter** — U1 predicts `NQuin` trajectory, maps to token ids | `ContextInjectRing`, `GgufTokenizer` |
| Hessian / graph pruning (June 2026) | **kNN bitmask** — attention only over topological neighborhood | `AttentionRouteMask`, `visit_tensor_search_into` |
| Continuous batching + RAG bubbles | **Phase-8 async inject** — U1 fills ring while U0 matmuls | `Phase8Channel::ContextInject`, `pop_tensor_context` |

**Hard constraints (same as Track B2):** zero heap in decode hot path; fixed-capacity rings; Sentinel `DenyRollback` before accepting draft tokens or sparse KV slots; all URIs via `q_hash` / lexicon at cold-path mapping only.

---

#### B3.1 — Topological speculative decoding (draft model = 10D space)

**Research gap addressed:** Draft-model speculative decoding pays a severe VRAM penalty (second weight set) and suffers mutual-waiting between draft and target on edge devices.

**Mechanism:**

1. **U1 drafter** — While U0 decodes token *t*, U1 extrapolates a semantic trajectory in `Tensor10D` (current `manifold_w`, velocity from last kNN hits, optional temporal `t` slice). Trajectory yields γ concept hashes (`subject_hash` per step), not logits.
2. **Cold-path vocab bridge** — `TopologyDraftMapper` (load-time only): `subject_hash` → nearest tokenizer id via resident `GgufTokenizer` + lexicon label lookup. Output: fixed `[u32; MAX_DRAFT_LEN]` stack buffer.
3. **U0 verifier** — Single batched forward pass over γ draft positions (extends existing prefill chunk path in `gguf_bridge`). Compare draft ids to target argmax chain; accept longest matching prefix (standard speculative acceptance).
4. **SPSC transport** — New ring `TopologyDraftRing` (producer U1, consumer U0) alongside `ContextInjectRing`; carries `{ draft_ids: [u32; γ], concept_hashes: [u64; γ], draft_len: u8 }`.
5. **Sentinel gate** — Before accepting drafts, Sentinel checks deontic/epistemic constraints on the proposed `NQuin` path (same Phase-8 control ring as anomaly rollback).

**Expected speedup model** (acceptance rate α, draft length γ, verify cost ratio *c* ≈ cost(verify γ) / cost(1 token)):

$$\text{Speedup} \approx \frac{1 - \alpha^{\gamma+1}}{(1-\alpha)\,(1 + c\,\gamma)}$$

When α → 1 (10D topology aligns with LLM semantics), speedup → γ. Measure α empirically per ontology domain; tune γ ∈ {2, 4, 8} under U0 VRAM pin.

| Milestone | Deliverable | Depends on |
|-----------|-------------|------------|
| **B3.1a** | `TopologyDraftRing` + `TopologyDraftBatch` (`compute_universe.rs`, fixed γ≤8) | B2.4 |
| **B3.1b** | `extrapolate_topology_draft()` — U1 SIMD/GPU kNN trajectory → concept hashes | B8, bake pipeline |
| **B3.1c** | `TopologyDraftMapper` — hash→token id (cold path, `GgufTokenizer`) | resident GGUF |
| **B3.1d** | `verify_topology_draft_batch()` in `gguf_bridge` — parallel γ verify + prefix accept | fused prefill |
| **B3.1e** | Telemetry: `draft_accept_rate`, `draft_len`, `speculative_speedup` atomics | P-D |

**PR stack:** `PR-B12` rings + types → `PR-B13` U1 trajectory → `PR-B14` U0 verify loop + Sentinel hook.

**Exit criteria:** On Gemma-class fixture, measured tokens/sec ≥ 1.5× autoregressive baseline at α ≥ 0.6, γ = 4, with **zero** additional weight staging bytes in `VramLedger`.

---

#### B3.2 — Graph-guided attention sparsity (dynamic KV mask)

**Research gap addressed:** Dense attention is memory-bandwidth bound at long context; graph-pruning literature shows LLMs waste compute on overlapping, irrelevant context.

**Mechanism:**

1. **U1 mask builder** — Each decode step: kNN on current query tensor (from last accepted `ContextInjectToken` or decode position embedding projected into 10D). `tensor_search_into` → indices → `AttentionRouteMask::set_index` for each hit (cap `ATTENTION_MASK_WORDS × 64` bits).
2. **KV slot mapping** — Cold-path table maps `tensor_index` → KV cache slot (prompt token positions with provenance `NQuin`). Hot path passes only `u64` mask words to GPU.
3. **Kernel contract** — Extend `AttentionGpuParams` + `fused_attention.wgsl` with `mask_words` buffer binding; softmax skips masked-out KV positions (structured block-sparse, not unstructured).
4. **Fallback** — If mask empty or U1 behind, dense attention (current path); never block decode.

| Milestone | Deliverable | Depends on |
|-----------|-------------|------------|
| **B3.2a** | `build_attention_route_mask()` — kNN → `AttentionRouteMask` (zero-heap) | B2.4 |
| **B3.2b** | `KvProvenanceMap` — tensor index ↔ prompt KV slot (cold bake) | ingest |
| **B3.2c** | `fused_attention.wgsl` masked softmax + WGSL tests | B11 |
| **B3.2d** | Benchmark: 1024 ctx, mask density 5–20%, bandwidth ↓ ≥ 40% | T2 rig |

**PR stack:** `PR-B11` (already planned) → `PR-B15` mask builder → `PR-B16` shader + params.

**Exit criteria:** Identical token stream vs dense baseline on masked fixture test; wall-clock attention pass ↓ proportional to active_bits / context_len.

---

#### B3.3 — Async continuous context via SPSC (no stop-and-RAG)

**Research gap addressed:** Continuous batching still stalls when retrieval blocks the forward pass.

**Mechanism:**

1. **U1 producer thread** — Bound to `platform_scheduler::bind_background_thread()`; loop: read decode position hint (atomic from U0), run `visit_tensor_search_into`, push `ContextInjectToken` + refresh `AttentionRouteMask` snapshot.
2. **U0 consumer** — Existing decode loop drains ring non-blocking (`pop_tensor_context`); never `await`s graph/daemon.
3. **KV inject (phase 2)** — Optional `KvInjectRing` for pre-baked prompt segments (daemon `:4242` slice uploaded once to U0 pin); distinct from per-step context tokens.
4. **Universe isolation** — Producer only writes U1 ledger + rings; U0 reads shared SOA substrate by reference (`GraphTensorSubstrate`); no cross-universe buffer realloc.

| Milestone | Deliverable | Depends on |
|-----------|-------------|------------|
| **B3.3a** | `start_tensor_search_producer()` — background U1 loop | ✅ (native); B8 GPU mirror pending |
| **B3.3b** | Decode position atomic + producer query tensor refresh | llm_agent |
| **B3.3c** | Daemon slice → resident SOA mmap (no per-token fetch) | C2 |
| **B3.3d** | SM utilization / stall counters in telemetry | P-D |

**PR stack:** `PR-B10` (continuous RAG) → `PR-B17` producer thread → `PR-B18` KV inject ring (optional).

**Exit criteria:** Decode loop contains **no** `block_on` graph/SPARQL calls; p99 inter-token latency variance ↓ vs synchronous kNN; ring overflow drops tokens (lossy) rather than stalling U0.

---

#### Track B3 dependency graph

```mermaid
flowchart LR
  B24[B2.4 ContextInject ring]
  B11[B11 AttentionRouteMask export]
  B81[B8 tensor_volume.wgsl]
  B24 --> B33[B3.3 Async continuous]
  B81 --> B33
  B24 --> B32[B3.2 Graph sparsity]
  B11 --> B32
  B24 --> B31[B3.1 Topo spec decode]
  B32 --> B31
```

**Recommended implementation order:** Complete **B2.4** → **B3.3** (unblocks latency) → **B3.2** (unblocks bandwidth) → **B3.1** (multiplier on top). B3.1 has highest upside but needs accurate α from real ontology trajectories.

**Research citations (planning only, not legal deps):** Cassandra / PicoSpec (speculative decoding tradeoffs); *Beyond FLOPs: Benchmarking Real Inference Acceleration of LLM Pruning* (June 2026); AutoPrunedRetriever / minimal reasoning graphs (graph-guided context pruning).

---

### Track C — Phenomenal viewport (after B4 + Phase 1)

```
PR-C1  webizen:   projector.wgsl + epistemic.wgsl + depth
PR-C2  both:      tensor_buffer GPU upload + binary daemon slice
PR-C3  webizen:   live Qualia neighborhood + camera/temporal IPC
PR-C4  webizen:   bloom + Eco/Reserve modes + navigation wired
PR-C5  webizen:   Tauri wgpu surface (PNG fallback Tier 1 only)
PR-C6  qualiaDB:  Pages spatial.html on shared shaders (Track A merge)
PR-C7  both:      phenomenal-checklist CI + docs sweep
```

**Recommended order:** B1 → B3 (bake) → B4 → B2.4 → B3.3 → B3.2 → B3.1 → (A2/A3 parallel) → C1 → C2 → C6 → C7.

**Throughput priority fork:** If inference tokens/sec is the bottleneck before phenomenal viewport, interleave **B2.4 → B10 → B17 → B15 → B16** ahead of Track C.

---

## Progress log

| Date | Phase | Update | Author |
|------|-------|--------|--------|
| 2026-06-17 | — | Plan created; scaffolding audit complete (Three.js on spatial, canvas2d ambient) | Agent |
| 2026-06-17 | 8–9 | Desktop audit: 2D wgpu PNG path; projector/epistemic shaders unwired; 3 independent GPU contexts; Phase 8–9 added | Agent |
| 2026-06-17 | 10–11 | Full phenomenal criteria (P-A..P-F); 10D bake+VRAM phase; tiered fallback policy; Track C PR stack | Agent |
| 2026-06-17 | 3, N | Product name: **Qualia WASM** — Semantic Subjectivity Bifurcation Portal; `QualiaPortal` API | Agent |
| 2026-06-17 | B2 | `ComputeUniverse` + `UniverseOrchestrator` in `gpu_context.rs`; Track B2 PR stack B7–B11 | Agent |
| 2026-06-17 | B2 | `compute_universe.rs` — Qualia fabric: SOA substrate, ContextInject ring, AttentionRouteMask | Agent |
| 2026-06-17 | B3 | Track B3 plan: topological spec decode, graph-guided attention sparsity, async SPSC continuous context; PR-B12–B18 | Agent |
| 2026-06-17 | B3.3 | `resident_substrate.rs`, U1 producer thread, decode hints, `build_attention_route_mask`, llm_agent + spatial_wasm wired | Agent |
| 2026-06-17 | B3.2/B3.1a | `attention_kv_mask_u32` → `fused_attention.wgsl` binding 5; `TopologyDraftRing` + `extrapolate_topology_draft` scaffold | Agent |
| 2026-06-17 | B3.1/B8 | `topology_draft.rs`, `verify_topology_draft_batch`, `kv_provenance.rs`, `tensor_volume.wgsl` | Agent |
| 2026-06-17 | B3 polish | `sentinel_allows_topology_draft`, shared `try_accept_topology_draft`, WASM decode parity, branch `0.0.17-dev` | Agent |
| | | | |

*Update this table and checkboxes when each item completes.*

---

## Appendix A — Build commands

```powershell
# qualiaDB engine tests
cargo test -p qualia-core-db --lib

# Qualia WASM portal (build from webizen-web crate, publish as qualia)
cd C:\Projects\webizen-browser\webizen-web
wasm-pack build --target web --out-dir pkg-qualia --features web
# Rename webizen_web.js → qualia.js, webizen_web_bg.wasm → qualia_bg.wasm

# Copy into qualiaDB docs
.\scripts\package-qualia-wasm.ps1

# Local Pages preview
npx --yes serve C:\Projects\qualiaDB\docs -p 4173
```

---

## Appendix B — SystemTelemetry field map (48 bytes)

| Index | Field | Visual behavior |
|-------|-------|-----------------|
| 0 | `memory_pressure` | Nebula compression toward core |
| 1 | `network_ripple` | Holographic X/Z ripples |
| 2 | `baking_crystallization` | Chaos → lattice morph |
| 3 | `logic_flashes` | Arc strikes / collapse flashes |
| 4 | `llm_heat` | Localized high-freq jitter + warm hue |
| 5 | `quantum_activity` | Phase tunneling flicker |
| 6 | `spectral_shift` | Global hue drift (σ band) |
| 7 | `temporal_pulse` | Radial provenance waves |
| 8 | `epistemic_density` | Clustering strength |
| 9 | `manifold_pressure` | Radial breathing |
| 10–11 | `_padding` | WGSL alignment |

Source: `webizen-browser/webizen-render/src/telemetry.rs`

---

## Appendix C — Ready-to-implement gates

### Gate A — Pages WASM viewport (minimum ship)

**Status:** Ready for **PR-A1** after **PR-B1** scaffold recommended.

- [x] Problem, boundary, fallback policy documented
- [ ] `GpuContext` not required for Pages-only preview, but shaders should target shared contracts

**First session:** PR-B1 (GpuContext skeleton) then PR-A1 + PR-A2.

### Gate B — GPU back-plane + 10D VRAM

**Status:** **Required before PHENOMENAL** — start here.

- [ ] `GpuContext` + `VramLedger`
- [ ] `tensor/bake_pipeline.rs`
- [ ] `tensor_volume.wgsl`
- [ ] Real `hardware_tier` GPU detection

**First session:** PR-B1 → PR-B3 → PR-B4.

### Gate C — PHENOMENAL declared

**Status:** Not ready — requires all P-A..P-E checkboxes.

**First session after B+C tracks:** PR-C1 + PR-C2.

### Strategic recommendation

1. **Fallback: yes** — tiered display degradation only; semantics always WASM.
2. **Lead with Track B** — without VRAM residency, Pages migration just moves Three.js to a smaller JS lie.
3. **One shader codebase** — `ambient.wgsl`, `projector.wgsl`, `tensor_volume.wgsl` shared native + wasm32.
4. **PHENOMENAL is not Phase 4** — Phase 4 is minimum Pages ship; phenomenal is Phase 8+9+10+Gate C.

---

## Appendix D — Tier fallback matrix (implementation spec)

| Capability | T2 Full | T1 Eco | T0 Edge | T0− no WebGPU |
|------------|---------|--------|---------|----------------|
| Tensor query | GPU `tensor_volume.wgsl` | SIMD `visit_*_into` | SIMD, smaller buffers | Same WASM SIMD |
| Node render | `projector.wgsl` 3D + depth | 3D, no bloom | 2D snapshot / static PNG | canvas2d nodes |
| Particles | 50 000 | 8 000 | off | 2 400 canvas2d |
| Bloom | on | off | off | off |
| LLM local | on (ledger) | throttled | off | N/A |
| Telemetry | live | live | partial | simulated OK |
| UI badge | `Tier 2 · Live` | `Tier 1 · Eco` | `Tier 0 · SIMD` | `Display fallback` |
| Mock Qualia | forbidden if daemon up | forbidden | allowed + badge | allowed + badge |

Implement `Viewport::tier()` returning enum; shell reads it for badge + control visibility.

---

## Appendix E — File ownership map (phenomenal)

| Concern | qualiaDB | webizen-browser |
|---------|----------|-----------------|
| `Tensor10D`, bake, query | `crates/qualia-core-db/src/tensor/` | — |
| `GpuContext`, `VramLedger`, compute universes | `gpu_context.rs`, `compute_universe.rs` | consume via FFI/Tauri |
| U1→U0 rings, attention mask, topo drafts | `compute_universe.rs` | — |
| `tensor_volume.wgsl` | embed in qualia or shared crate | `webizen-render/src/shaders/` |
| Viewport shaders | — | `ambient.wgsl`, `projector.wgsl`, `epistemic.wgsl` |
| Qualia WASM portal | re-export + `QualiaPortal` | `webizen-web/` → `docs/pkg/qualia/` |
| Desktop shell | daemon `:4242` tensor slice | `webizen-desktop/`, `webizen-studio/` |
| Tier fallback UI badge | `docs/js/qualia-shell.js` | `render_preview.rsx` |
| Fallback canvas2d | `docs/js/ambient-viz.js` (lazy) | — |

---

## Appendix F — Track B3 benchmarks & speculative speedup calculator

Use this to validate **B3.1** before shipping topological speculative decoding. Variables are measured at runtime via atomics (no heap); the calculator is for planning and CI regression targets.

### Variables

| Symbol | Meaning | How to measure |
|--------|---------|----------------|
| **α** | Draft acceptance rate (per-token match probability) | `accepted_draft_tokens / total_draft_tokens` over 1k decode steps |
| **γ** | Draft length (concepts pushed per U1 cycle) | `TopologyDraftBatch.draft_len` (2–8, capped by ring) |
| **c** | Verify cost ratio | `wall_time(verify γ)` / `wall_time(1 autoregressive step)` on T2 rig |

### Speedup formula (prefix acceptance, parallel verify)

$$\text{Speedup}(\alpha, \gamma, c) \approx \frac{1 - \alpha^{\gamma+1}}{(1-\alpha)\,(1 + c\,\gamma)}$$

**Planning examples** (assume *c* = 0.15, i.e. verifying 4 drafts costs ~60% of one serial step):

| α | γ | Predicted speedup |
|---|---|-------------------|
| 0.4 | 4 | ~1.3× |
| 0.6 | 4 | ~2.0× |
| 0.8 | 4 | ~3.2× |
| 0.6 | 8 | ~2.8× |

α is ontology-dependent: guardianship/legal graphs with tight 10D manifolds should score higher than open-domain chat.

### CI / bench harness (add to `benchmarks/qualia/`)

1. **Baseline** — autoregressive decode only, fixed prompt fixture, report tokens/sec.
2. **B3.3** — same fixture with U1 producer on; report p50/p99 inter-token latency (stall regression guard).
3. **B3.2** — 1024-token context, 10% mask density; report attention pass time vs dense.
4. **B3.1** — enable `TopologyDraftRing`; sweep γ ∈ {2,4,8}; report measured speedup vs formula residual &lt; 15%.

### Telemetry atomics (P-D extension, `gpu_context.rs`)

| Atomic | Maps to |
|--------|---------|
| `DRAFT_ACCEPT_MILLI` | α × 1000 (EMA) |
| `DRAFT_LEN_CUR` | current γ |
| `SPEC_SPEEDUP_MILLI` | measured speedup × 1000 |

Portal HUD can expose these alongside `llm_heat` once B3.1e lands.

---

*Maintainers: check boxes in this file as PRs land; bump **Last updated** and **Progress log** on each merge.*