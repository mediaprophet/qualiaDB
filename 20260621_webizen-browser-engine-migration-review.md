# webizen-browser → qualiaDB Engine — Migration Review

**Date:** 2026-06-21
**Question reviewed:** *Is there code living in `C:\Projects\webizen-browser` that needs to be brought
across to the qualiaDB "engine" libraries (`qualia-core-db` / `qualia-client-core`)?*
**Method:** structural map of both repos + signature/definition greps + cross-repo diffs (read-only).

---

## 0. Bottom line

**Mostly reassuring, with two genuine engine pieces to extract — and one structural mess to fix first.**

1. **The heavy engine is already single-source in qualiaDB.** The semantic graph, canonical `NQuin`,
   LLM stack, logic/SHACL/N3, crypto, solvers, SPARQL, and the 10D tensor system all live in
   `qualia-core-db`, and webizen-browser consumes them via a path dependency
   (`../../qualiaDB/crates/qualia-core-db`). It is **not** a fork of the engine.
2. **Two genuine engine-domain pieces are mislayered into the browser's shell crates** and should be
   lifted into `qualia-core-db`: the **GLB/glTF asset → 10D/semantic ingest** (`glb_ingest.rs`) and the
   **portable compute-kernel abstraction** (`webizen-runtime/kernel.rs`). See §2.
3. **The real hazard is duplication + divergence:** the `webizen-studio / render / runtime / desktop`
   crates exist in **both** repos and have **drifted apart**. So "bring across" is dangerous until you
   decide which copy is canonical — you could migrate the stale one. See §1.
4. **Render belongs in qualiaDB, not the browser (revised).** The WASM builds need rendering, and
   qualiaDB already has a wasm-capable `portal_*` renderer that *overlaps* `webizen-render`. Consolidate
   into one engine-side render layer (`qualia-render`) + a **minimal dev-bench UI** in qualiaDB; the full
   browser UI/shell stays in webizen-browser and consumes it. See **§2.4** (this refines the old
   "rendering stays browser-side" stance).
5. **No real audio/spectral DSP exists to migrate** — only contract types and telemetry naming. The
   `legacy/` tree is a superseded SvelteKit/Tauri browser. See §3.

---

## 0.1 Dependency boundary & target-environment constraint (clarification, 2026-06-21)

**The browser is an OPTIONAL native shell — the engine must never depend on it.** `webizen-browser` exists
only to overcome consumer-browser (Chrome/Firefox) limits — sandboxed origin-file access, raw WebGPU
buffer control, thread allocation — via a locally-installed native shell. It is **not required for the CLI
or WASM** deployments. Workflow intent: **build the browser, then redirect it to consume the components
migrated into the qualiaDB core.**

**One-way dependency — "Engine down, Browser up":**
- **Engine (`qualia-core-db` / `qualia-client-core`)** is sovereign: pure logic, memory-mapping (48-byte
  Quins), crypto, math, raw byte processing. It must compile **independently** for `cargo build` (native
  CLI) **and** `cargo build --target wasm32-unknown-unknown` **without ever knowing a UI exists.**
- **Browser/CLI (shells)** are consumers: OS windowing, file-picker dialogs, canvas/present contexts.

**Migration must keep the engine platform-agnostic — verified-clean today and must stay that way:**
`qualia-core-db` currently has **no** `tauri`/`winit`/`rfd`/`dioxus` deps (only `wgpu` + `web-sys` +
`wasm-bindgen`, which are the legitimate compute/WASM path). Any code lifted in **must not** add
windowing, OS file-dialog, or UI-shell crates. Concretely:
- **`glb_ingest` (§2.1):** the engine `glb_bridge` must accept a **raw `&[u8]`** and return `NQuin`s — it
  must **not** call `std::fs` (the current `glb_ingest.rs` does `std::fs::File::open(path)`, which doesn't
  exist on wasm32). File-open + OS file-picker stay in the browser/CLI wrapper, which hands bytes down.
- **Anything that opens an OS window or paints pixels stays in the shell.** The engine does bytes + math only.

---

## 1. Repo relationship & the duplication hazard (read this first)

webizen-browser is a 5-crate Cargo workspace that **consumes** qualiaDB:

| Crate | Files / lines | Role | qualia dep |
|------|---------------|------|-----------|
| `webizen-runtime` | 7 / ~914 | WGPU/compute HAL + kernel abstraction + diffusion field | none (wgpu only) |
| `webizen-render` | 12 / ~3,682 | WGPU renderer, projection math, 6 `.wgsl` shaders, audio contract | `qualia-core-db` |
| `webizen-studio` | **357 / ~54,506** | Dioxus UI: ~340 qapp/components, panes, canvas, theme | `qualia-core-db`, `webizen-render` |
| `webizen-desktop` | 7 / ~4,091 | Tauri shell, IPC commands, runtime wiring | `qualia-core-db`, `qualia-client-core`, runtime, render, studio |
| `webizen-web` | 3 / ~671 | wasm package `qualia-wasm` (wraps `qualia-core-db`) | `qualia-core-db` |

**The problem:** qualiaDB *also* contains `crates/webizen-studio`, `webizen-render`, `webizen-runtime`,
`webizen-desktop` — **near-identical copies** (studio 358 vs 357 files, render 12/12, runtime 7/7).
They are **diverged**, not just duplicated:

| File (same path in both repos) | Divergence |
|---|---|
| `webizen-studio/src/studio_canvas.rs` | **DIVERGED — 2,790 diff lines** |
| `webizen-desktop/src/commands/glb_ingest.rs` | **DIVERGED — 932 diff lines** |
| `webizen-runtime/src/kernel.rs` | **DIVERGED — 404 diff lines** |
| `webizen-render/src/math/motor_encoder.rs` | DIVERGED — 228 diff lines |
| `webizen-runtime/src/diffusion.rs` | identical |

**Confusing state to resolve:**
- qualiaDB's copies are **newer** (2026-06-19, commit `9a4c8e83` "feat(0.0.18): Webizen crates…") than
  the browser's (2026-06-16, `09f59fb9` "release: 0.0.4").
- …**but qualiaDB does NOT build them** — `webizen-*` are **not** in qualiaDB's workspace `members`
  (only `qualia-*`, `wellfare-core`, `webizen-component-harvester` are). So in qualiaDB they are
  **committed-but-orphaned, unbuilt copies**; in webizen-browser they are the live, building crates.

**Recommendation (decide before any migration):** pick **one** home for the `webizen-*` shell/UI crates.
The natural split for "browser as a separate product consuming the engine" is:
- **Engine** (`qualia-*`) → lives in `qualiaDB`, single source.
- **Browser shell/UI** (`webizen-studio/render/runtime/desktop/web`) → lives **only** in `webizen-browser`,
  consuming the engine via the path (or a git) dependency.
- → **Delete the orphaned `crates/webizen-*` copies from qualiaDB** (they don't build there and only create
  drift), **after** salvaging anything newer in them (qualiaDB's are the Jun-19 copies, so diff each
  before deleting). Or, if you want them in qualiaDB's workspace, add them as members and retire the
  browser's — but do **not** keep two building/diverging copies.

---

## 2. Genuine engine-domain code to bring across to `qualia-core-db`

These are the real answers to your question — engine logic currently sitting in the browser shell crates:

### 2.1 GLB / glTF asset ingest — `webizen-desktop/src/commands/glb_ingest.rs` (466 L)
- Contains `GLBView` (parses GLB magic/JSON-chunk/binary-chunk), `GLBMetadata`,
  `GLBIngestionManager`, `SemanticMapping`, and `Tensor10DMapping` (GLB → 10D tensor mapping).
- This is **asset-ingestion engine logic** — the same family as `qualia-core-db/src/kml_bridge.rs` and
  the STELLAR §E "direct-load `.obj`/`.stl`/OpenUSD → quins" item. It even builds its *own*
  `SemanticMapping`/`Tensor10DMapping` types instead of emitting canonical engine `NQuin`s — which is
  exactly why it should be unified into the engine.
- **Action:** extract the parser + mapping into `qualia-core-db` (e.g. `glb_bridge.rs` beside
  `kml_bridge.rs`), emitting real `NQuin`/10D-tensor output; leave only the thin Tauri command wrapper in
  `webizen-desktop`. **Signature must be `fn(&[u8]) -> …NQuin…`, NOT a path** — the current
  `glb_ingest.rs` calls `std::fs::File::open(path)` (no `std::fs` on wasm32); the shell reads the file /
  runs the OS picker and passes bytes down (§0.1). Keep the hot parse zero-alloc (no `String` on the
  per-vertex path). **Caveat:** the two repo copies differ by 932 lines — reconcile/own first (§1).

### 2.2 Portable compute-kernel abstraction — `webizen-runtime/src/kernel.rs` (202 L)
- `ComputeBackend` trait, `LedgerRecord`, `LedgerSink` trait (+ `ChannelLedgerSink`/`NullLedgerSink`),
  and `SimulationKernel<B, L>` — a clean, backend-agnostic compute + provenance-ledger kernel,
  **decoupled from wgpu** (the wgpu impl is the separate `wgpu_backend.rs`).
- This matches the existing project note to "lift the webizen-runtime kernel abstraction (not
  `wgpu_backend`) into qualiaDB." It is reusable engine/runtime infrastructure, not browser-specific.
- **Action:** lift the trait layer into `qualia-core-db` (or a small `qualia-runtime` engine crate);
  keep the concrete `wgpu_backend.rs` in the browser. Already portable (no `wgpu`/`tauri`/`web-sys`) — but
  it pulls `crate::diffusion`/`crate::snapshot`/`crate::clock`, so lift those (or abstract them) alongside.
  **Caveat:** copies differ by 404 lines — reconcile/own first.

### 2.4 Render engine — CONSOLIDATE into qualiaDB (revised — the WASM builds need it)

**This supersedes the earlier "rendering stays browser-side" line.** Per the correct observation that the
render engine is required for the WASM builds: rendering is an **engine dependency**, not browser
presentation. And critically — **qualiaDB already has a WASM-capable renderer**, so this is *unification of
two overlapping renderers*, not a relocation.

**The two renderers that overlap:**
| | qualiaDB engine `portal_*` (in `qualia-core-db`) | webizen-browser `webizen-render` |
|---|---|---|
| Size | ~3,885 L (`portal.rs` 1121, `portal_gpu.rs` 1542, `portal_pga.rs` 511, `portal_camera.rs` 183, `portal_wasm.rs` 62, `spatial_wasm.rs` 466) | ~3,682 L (12 files) |
| WASM | **Yes** — `portal_wasm.rs` creates `HtmlCanvasElement`; `portal_gpu.rs` has `RenderPipeline` + wgsl | mostly **native-only** (`wgpu_renderer.rs` is `cfg(not(wasm32))`; "wasm renders to `<canvas>`" is a stub future) |
| Shared concepts | Camera (45), Motor/PGA (30), RenderPipeline (16), wgsl (25), Tensor10D (14) | Scene (99), Motor (35), Camera (24), Spectral (22), Projection, RenderPipeline, Tensor10D |
| Unique strengths | already in-engine, already wasm canvas path, PGA motors (`portal_pga`), spectral (`portal_spectral`) | richer **scene-graph** (`scene.rs`/`scene_contract.rs`), native **offscreen frame delivery**, **audio/spectral contract**, glTF intent |
| Dev-bench today | `docs/spatial.html` + `docs/js/spatial-demo.js` (a minimal WASM portal page) | the full Dioxus studio |

**They are two implementations of the same 10D-manifold WebGPU renderer** — the same duplication problem
as the studio/desktop crates (§1), but for rendering.

**Recommended target architecture:**
1. **One canonical render engine inside the qualiaDB workspace.** The natural base is the existing
   `portal_*` (it's already in the engine and already has the wasm canvas path). Extract it into a proper
   **`qualia-render` crate** (workspace member) so it's reusable, testable, and wasm-buildable on its own —
   sharing the **single `wgpu` device** with the compute path (the 0.19 pin already enforces one wgpu
   version; keep that until the upgrade in §below).
2. **Absorb webizen-render's genuinely-unique pieces** into it: the scene-graph/`scene_contract`, native
   offscreen frame delivery, the audio/spectral contract, and the glTF/`glb_ingest` path (§2.1). Drop
   whatever merely duplicates `portal_gpu`/`portal_pga`/`portal_camera`.
3. **Add a *minimal* dev-bench UI in qualiaDB — not a full browser.** Grow the existing `spatial.html` /
   benchmark portal pages (or a small `qualia-devbench` crate) into a thin harness that exercises
   engine + render in WASM standalone. This makes qualiaDB self-sufficient for its own demos/benchmarks
   without pulling in the Dioxus studio or Tauri shell.
4. **webizen-browser keeps only the *full product*** — `webizen-studio` (Dioxus UI) + `webizen-desktop`
   (Tauri shell) — consuming the consolidated qualiaDB engine + render. The `webizen-runtime` **kernel
   abstraction** folds into the engine too (§2.2); only `wgpu_backend.rs` + browser-specific glue stay.

**Two caveats that gate this:**
- **The `wgpu` upgrade becomes more urgent.** Defect #1 (wgpu 0.19.4 sends `maxInterStageShaderComponents`
  → `requestDevice` fails on current Chrome — see `LLM_Q42_STRATEGIC_PLAN.md`) already blocks the WASM LLM;
  consolidating render into the WASM build means the **same** device-init bug now gates rendering too.
  Fix the wgpu upgrade once, for both compute and render, on the shared device.
- **Sequence it after §1.** Reconcile/own the diverged copies first; unifying two renderers while two
  diverged copies of each exist would be chaos.

### 2.3 Canonicalise the 10D / Quin types (de-duplicate, don't fork)
The browser defines several **local mirrors** of engine types — fine for GPU `#[repr(C)]` layout, but
they must derive from / stay pinned to qualiaDB's canonical definitions, not drift:
- `webizen-studio/src/studio_canvas.rs:7` — a **local `struct NQuin`** (a straight duplicate of
  `qualia_core_db::NQuin`). **Should use the engine's `NQuin`** (or the already-existing
  `webizen-render` `RenderQuin`), not redefine it.
- `webizen-render` `RenderQuin`, `Tensor10DProjection`; `webizen-studio` `Tensor10DView`;
  `webizen-desktop` `Tensor10DMapping` — render/GPU-side projections of the canonical 10D tensor
  (`qualia-core-db` `q42_volume` / `tensor-10d`). **Action:** keep one canonical 10D definition in the
  engine; make the render mirrors thin, clearly-named views over it.

---

## 3. What should NOT move (legitimately browser-side) and what isn't real

- **UI qapps / panels** — the 41 "solver" and 38 "sparql" hits are almost entirely `*_qapp.rs` Dioxus
  components (`OdeSolver()`, `SparqlExplorer()`, `astronomy_qapp`, `chemistry_qapp`, …). These are the
  **UI layer** calling the engine; they stay in the browser. (Note: these same qapps are *also* in
  qualiaDB's orphaned `webizen-studio` copy — part of the §1 duplication, not engine migration.)
- **Rendering** — ~~browser-side by design~~ **REVISED: consolidate into qualiaDB — see §2.4.** Because
  the WASM builds need rendering and qualiaDB already has a wasm-capable `portal_*` renderer, the
  `wgpu_renderer.rs` / `.wgsl` shaders / `motor_encoder.rs` / scene-graph should be **unified into the
  engine**, not left as a browser-only concern. Only the full Dioxus/Tauri *product UI* stays in the browser.
- **Diffusion field** (`webizen-runtime/diffusion.rs` + `wgpu_backend.rs`) — a reaction-diffusion GPU
  sim for the **ambient background visualization**. Borderline (it's physics compute) but tied to the
  renderer; recommend it **stays** unless you deliberately want it as a reusable engine compute primitive.
- **Audio / spectral** — despite ~36 "spectral/STFT/CQT" hits, there is **no real DSP implementation**:
  only contract types (`SpectralParams`, `AudioSpectralSheet` in `audio_contract.rs`) and desktop
  telemetry counters (`get_spectral_shift`, `get_manifold_pressure`). **Nothing to migrate** — it's
  scaffolding/plan (`AUDIO_PROJECT_STATUS.md`), consistent with the unbuilt "multimodal-as-physics" horizon.
- **`legacy/` + `browser-legacy-files/`** — a **superseded** SvelteKit + Tauri (`src-tauri/`) browser
  (node_modules, package.json, `.svelte-kit/`). Contains old `nquin_parser.rs`, `query_router.rs`,
  `mcp_bridge.rs`, `qlinks.rs`, `wellfare_commands.rs`. **Archive it.** Only worth a glance if you want to
  rescue an idea from `nquin_parser.rs`/`query_router.rs`; otherwise it's noise (and bloats greps).

---

## 4. Recommended sequence

1. **Resolve §1 first** — decide the canonical home for `webizen-*`; reconcile the diverged copies (the
   qualiaDB Jun-19 copies vs the browser Jun-16 copies); delete the orphaned non-member copies from
   qualiaDB. *Do this before migrating anything, or you risk migrating a stale file.*
2. **Fix the `wgpu` upgrade (defect #1) once** — it gates both the WASM LLM *and* the consolidated WASM
   render path on the shared device. Doing it before the render consolidation avoids redoing GPU regression twice.
3. **Consolidate render (§2.4)** — make qualiaDB's `portal_*` the canonical renderer (extract to a
   `qualia-render` workspace crate), absorb webizen-render's unique pieces (scene-graph, offscreen
   delivery, spectral contract, glTF), add a **minimal dev-bench UI** (grow `spatial.html`/benchmark pages).
4. **Extract GLB ingest (§2.1)** → `qualia-core-db/glb_bridge.rs` (emit canonical NQuins) — folds in with §3’s glTF path; thin Tauri wrapper stays.
5. **Lift the kernel abstraction (§2.2)** → engine; keep `wgpu_backend.rs` in the browser.
6. **De-dup the Quin/10D types (§2.3)** — remove the local `NQuin` in `studio_canvas.rs`; pin render mirrors to canonical engine types.
7. **Archive `legacy/`.** Leave the diffusion ambient-viz where it is (or fold into `qualia-render` if you want it as a reusable primitive).

---

## 5. Caveats / coverage

- Read-only review. I mapped both workspaces, grepped engine signatures + **definitions** (not just
  `use`s), and diffed the key cross-repo files — I did **not** read all 357 `webizen-studio` files line by
  line. If a single qapp hides real engine computation (rather than calling the engine), it would not show
  in the structural pass; name any you suspect and I'll inspect it.
- "Engine" here = `qualia-core-db` / `qualia-client-core` domain (semantic graph, NQuin, tensor, logic,
  crypto, LLM, solvers, ingest). UI/render/shell/HAL are treated as browser-side.
- Divergence line-counts are `diff` hunks at 2026-06-21; they will change as either repo moves.
