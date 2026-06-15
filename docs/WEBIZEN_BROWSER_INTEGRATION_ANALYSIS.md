# Webizen-Browser → QualiaDB Integration Analysis

**Author:** Claude (Opus 4.8) · **Date:** 2026-06-15
**Question answered:** Are there libraries in `C:\Projects\webizen-browser` that should be
incorporated into QualiaDB — specifically the `webizen-runtime` wgpu compute backend and the
3D render engine?
**Method:** Full source audit of `webizen-browser` (`webizen-runtime`, `webizen-studio`,
`webizen-desktop`, `legacy/`), cross-referenced against existing QualiaDB capability in
`crates/qualia-core-db/src/`. Cross-checked the browser repo's own design docs
(`WEBIZEN_3D_ENGINE.md`, `WEBIZEN_QUALIADB_GAPS.md`, `WEBIZEN_3D_NATIVE_VISION.md`).

---

## TL;DR (recommendations)

| Candidate | Verdict | Why |
|---|---|---|
| `webizen-runtime/src/wgpu_backend.rs` (diffusion compute) | **Do NOT import wholesale** | Duplicates existing `modalities/diffusion.rs` + `shaders/diffusion.wgsl` |
| `webizen-runtime` **kernel abstraction** (`kernel.rs`, `clock.rs`, `snapshot.rs`) | **Import / lift into QualiaDB** | Genuinely novel: a deterministic, provenance-emitting stepped simulation runtime QualiaDB lacks |
| `webizen-studio/src/render/` (3D engine) | **Leave in the browser** | It is *presentation*; per the project's own no-external-engine rule, rendering is the principled browser-side exception |

**Architectural principle (from `WEBIZEN_QUALIADB_GAPS.md`, restated):** engine capability
(compute, math, logic, inference) lives in QualiaDB; *presentation* (rendering, draw calls,
scene graph) lives in the browser and **delegates math to QualiaDB**. The dependency
direction is browser → QualiaDB, never the reverse.

---

## 1. `webizen-runtime` — a standalone GPU simulation kernel

`webizen-runtime` is a self-contained crate (deps: `wgpu`, `bytemuck`, `crossbeam-channel`,
`sha2`) with **no dependency on QualiaDB**. Public surface (`lib.rs`):

- `SimulationKernel` + `RuntimeCommand` — a stepped real-time simulation loop with a command channel
- `ComputeBackend` trait — `step(epoch) -> SimulationSnapshot` (+ `reconfigure`, `shared_frames`)
- `FixedStepClock` — deterministic fixed-timestep driver
- `SimulationSnapshot` + `StateHash` (sha2) — reproducible, hashable frames
- `LedgerSink` / `ChannelLedgerSink` / `LedgerRecord` — per-frame (epoch, dims, state-hash) recording
- `WgpuDiffusionBackend` — the one concrete `ComputeBackend`: a WGSL diffusion compute shader
  (toroidal wrap, double-buffered storage), `WORKGROUP_SIZE = 8`

### 1a. Why NOT to import `wgpu_backend.rs`

QualiaDB **already has** this capability:

- `crates/qualia-core-db/src/modalities/diffusion.rs`
- `crates/qualia-core-db/src/shaders/diffusion.wgsl`
- …plus a whole shader library: `fluid_dynamics`, `molecular_dynamics`, `kinematics`,
  `calculus`, `quantum_bio`, `fused_attention`, `fused_transformer`, `lora_apply`,
  `quantized_embedding`, `sieve`
- …and real wgpu plumbing in `gguf_bridge.rs`, `lora/webgpu_lora.rs`, `modalities/calculus/gpu.rs`

Copying the diffusion backend would duplicate engine code that already exists. That file is the
*least* valuable part of `webizen-runtime`.

### 1b. What IS worth taking — the kernel abstraction

QualiaDB's GPU shaders are each wired ad-hoc; there is **no general stepped-simulation runtime**
and **no reproducible frame/ledger model** for simulations. The `webizen-runtime` kernel provides
exactly that, and its design maps cleanly onto QualiaDB invariants:

| `webizen-runtime` piece | Fit in QualiaDB |
|---|---|
| `ComputeBackend` trait | A uniform `step(epoch) -> Snapshot` seam so *all* existing shaders (diffusion, fluid, MD, kinematics) plug into one driver instead of bespoke call sites |
| `SimulationKernel` + command channel | The missing general stepped-sim runtime |
| `FixedStepClock` | Determinism / reproducibility |
| `SimulationSnapshot` + `StateHash` (sha2) | Hashable, reproducible frames → maps directly onto the **WAL / provenance / DagStore** model |
| `LedgerSink` | Per-frame epoch+hash recording → provenance-native |

**Proposed shape:** a new `qualia-core-db` module (e.g. `simulation_kernel/`) that lifts the
`ComputeBackend` trait + `SimulationKernel` + clock + snapshot + ledger, and drives QualiaDB's
**existing** shaders. The diffusion backend serves only as the reference `ComputeBackend` impl.
Net effect: a pile of one-off shaders becomes a uniform, deterministic, ledgered simulation
runtime that emits provenance.

**Caveats before doing this:**
- Respect QualiaDB hot-path invariants (no `Vec`/`String`/`Box` in hot paths; 48-byte NQuin;
  42 MB SlgArena ceiling). `webizen-runtime` uses `Vec<f32>` frames and `crossbeam-channel` —
  these are fine for a host-side simulation driver but must not leak into the zero-copy ABI.
- Confirm overlap vs. enhancement with `modalities/diffusion.rs` before writing code — this may
  be a *merge* (adopt the kernel, keep QualiaDB's shader) rather than a fresh import.
- WASM target: `crossbeam-channel` threading model differs in the browser; gate accordingly.

---

## 2. The 3D engine (`webizen-studio/src/render/`) — thorough audit

**User belief checked:** "the 3D engine is fully built." **Verdict: partially correct.** The CPU
scene/mesh/graph **dev-kit foundation is real, compiles, and is unit-tested** — but the GPU
backend, asset loading, lighting, camera controls, and SPARQL scene source are **NOT built**;
they are documented placeholders. This matches the browser repo's own
`WEBIZEN_3D_ENGINE.md`, whose status line reads *"Foundation implemented (compiles, tested),
GPU backend + QualiaDB-native math pending."*

### 2a. What IS built (verified by reading every file)

| File | Built & tested |
|---|---|
| `scene.rs` | `Vec3` (dot/cross/normalize), `ScreenPoint`, `Camera` look-at **perspective projection** (`Camera::project`, near-plane cull) |
| `mesh.rs` | `Transform` (pos/euler-XYZ/scale, `apply`), `Mesh` + primitive builders: `line`, `cube`, `quad`, `grid`, `uv_sphere`. Tests: cube topology, transform, grid count |
| `graph.rs` | `Scene`/`Node`/`Style` scene graph, recursive `render_node` with nested transform composition, draws faces→edges→points. Test: nesting |
| `qualia.rs` | **Semantic binding:** `SemanticScene`/`SceneItem` (state, intensity, **provenance**, reasons), `item_color` heatmap, `build_scene(sem, camera, layout)`. Tests: color monotonic, layout skip-unknown, contract-JSON deserialize |
| `mod.rs` | `Renderer` trait + `prelude` |
| `canvas2d.rs` | **CPU reference backend** (HTML Canvas 2D), wasm-only (`cfg(target_arch = "wasm32")`) — strokes lines, fills polygons, draws points |

Wired into exactly one consumer: `components/physics_simulator.rs`.

### 2b. What is NOT built (genuine gaps — confirmed by repo-wide grep, 0 implementations)

A grep across the **entire** `webizen-browser` repo for `WgpuRenderer`, `RenderPipeline`,
`create_render_pipeline`, `gltf`, `.glb`, `VertexState`, `FragmentState`, `rasteriz*` returned
**only doc-comment references to a "future WgpuRenderer"** — no implementations.

- ❌ **GPU renderer** (`WgpuRenderer`) — does not exist; only a trait seam + doc promises
- ❌ **glTF / `.glb` asset loading** — does not exist
- ❌ **Lighting / materials / shading / textures** — flat CSS colors + alpha depth-fade only
- ❌ **Depth buffer / z-sorting** — faces drawn in mesh order (no painter's sort, no z-buffer)
- ❌ **Camera controls** (orbit / arc-ball / pick-hover) — not built
- ❌ **`SceneSource` over SPARQL** — trait exists; the query-backed impl does not
- ⚠️ CPU backend is **wasm-only**; there is no native CPU backend

`WEBIZEN_3D_ENGINE.md §5` lists all of the above as "Next (incremental)" work, and
`WEBIZEN_QUALIADB_GAPS.md §2` lists "3D rendering (render pipeline / vertex+fragment shaders)"
and "glTF/.glb loading" as **0-match genuine gaps to build**.

### 2c. Recommendation — leave it in the browser, delegate math

The 3D engine is **presentation** and belongs on the browser side. The project's own architecture
(`WEBIZEN_3D_ENGINE.md §6`) is to promote `render/` to a `webizen-render` crate *after* the
`WgpuRenderer` lands, depending on `wgpu` and (native) `qualia-core-db::geometric_algebra`.

What QualiaDB should do to *support* it (no engine code moves into the browser):
1. Ensure `geometric_algebra` exposes the vector/matrix/projection ops the renderer needs, so
   `scene.rs`/`mesh.rs` f64 fallbacks can delegate on native. (Note: `geometric_algebra` has
   open failing tests — see `KNOWN_ISSUES.md`; fix those first so the renderer can rely on it.)
2. When the simulation kernel (§1b) lands, **share one wgpu device** between QualiaDB compute and
   the future `WgpuRenderer` so compute output renders without a CPU round-trip.
3. Provide a `SceneSource`-friendly query path (SPARQL/RDF-star → `SemanticScene` JSON) — the
   contract is already defined by `qualia.rs::SemanticScene` (serde).

---

## 3. Everything else in `webizen-browser` (not engine candidates)

- `webizen-studio` — hundreds of Dioxus `*_qapp.rs` UI components (presentation thin-clients). Stays.
- `webizen-desktop` — app shell (`main.rs`, `runtime.rs`, command glue). Stays.
- `legacy/src-tauri` — old Tauri command handlers (`nquin_parser`, `query_router`, `qlinks`,
  `attention_tracker`, `wellfare_commands`, …); mostly glue that *calls* the engine. Candidate
  for deletion, not absorption.
- **HCAI / `webai` negotiation endpoint** — the GAPS doc's highest-leverage true gap. Only UI
  references exist in this repo (no implementation). If built, it belongs next to
  `deontic_logic` in QualiaDB, not copied from here.

---

## 4. Suggested action order

1. **Fix `geometric_algebra` + other `KNOWN_ISSUES` test failures** (in progress) — the renderer
   will depend on `geometric_algebra` math.
2. **Lift the `webizen-runtime` kernel abstraction** into `qualia-core-db::simulation_kernel`,
   driving existing shaders, emitting provenance via the WAL/Dag model. (Plan first; then port.)
3. **Browser side (separate workstream):** build `WgpuRenderer`, then promote `render/` to a
   `webizen-render` crate that delegates math to `geometric_algebra` and shares the kernel's wgpu device.
