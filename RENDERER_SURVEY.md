# Manifold / N-D Renderer — Survey of what already exists (2026-06-22)

Recon before defining the renderer + authoring docs (Track C of `MULTI_AGENT_MCP_PLAN.md`).
Timothy's read was right: it's **substantially ported into qualiaDB and running to some degree**
— but it's split across two parallel codebases and the GH-Pages surface is gated on a WASM
bundle that isn't reliably loading. This is the map; nothing here is changed yet.

---

## 1. Where the renderer lives in qualiaDB (verified)

### A. Standalone native renderer — `crates/webizen-render/` (ported, OUTSIDE the main workspace)
"**Webizen N-Dimensional Renderer** — a zero-heap, N-dimensional semantic renderer that projects
QualiaDB's multi-modal logic graph onto 2D/3D using **Projective Geometric Algebra (PGA)**."
Modules: `math` (Motor / MotorEncoder / RenderQuin / AlignedBufferF32), `pipeline`
(BindGroupManager), `scene_contract` (RenderScene/Camera/Node/Edge/Face/Point), `shaders`
(PROJECTOR_WGSL, EPISTEMIC_WGSL), `telemetry`, `wgpu_renderer` (native `render_scene_png` /
`render_scene_data_uri` / `render_preview_*`). Semantic culling (deontic/temporal), epistemic LoD.
- **Build state:** ⚠️ **NOT in the root `Cargo.toml` `members`** (which lists only
  `qualia-core-db`, `qualia-cli`, `qualia-solid-bridge`, `qualia-client-core`, `qualia-extensions`,
  `webizen-component-harvester`, `qualia-mobile-harness`, `wellfare-core`). Consumed via path by
  `crates/webizen-desktop` + `crates/webizen-studio` (also present, also outside the workspace).
  So the main `cargo build` does **not** build it — it's a staged port, not yet wired in.
- Sibling ported crates also outside the workspace: `webizen-desktop`, `webizen-studio`,
  `webizen-web`, `webizen-runtime`.

### B. In-engine browser portal surface — `qualia-core-db/src/portal_*` (IN the workspace, builds)
The **Semantic Subjectivity Bifurcation Portal** (browser/WASM surface), 12 modules:
`portal.rs`, `portal_gpu`, `portal_camera`, `portal_navigation`, `portal_pga`, `portal_acoustic`,
`portal_spectral`, `portal_standpoint`, `portal_control`, `portal_telemetry`,
`portal_phenomenal_contract`, `portal_wasm`. This **is** part of `qualia-core-db` (a workspace
member) → compiled, WASM-targetable. It is the live browser renderer path.

### C. The 10D substrate (IN the workspace)
- `compute_universe.rs` — the 10D compute universe (the §20 substrate).
- `tensor/manifold.rs` — the manifold tensor.
- `shaders/viewport/*.wgsl` — `projector`, `epistemic`, `spectral`, `ambient`, `bloom`, `screen`.
- `modalities/manifold_logic.rs` — the §20 continuous→discrete *logic* bridge (built this session).

### D. GH-Pages surface (`docs/`)
- `spatial.html` (36 KB, "Spatial Mathematics & GeoSPARQL", **2 canvases**, WebGPU present) —
  wired, but currently renders **"WASM Engine Required"**: it's gated on a WASM engine bundle
  that isn't loading.
- `playground/anatomy.html` + `anatomy.js` — a 3D anatomy demo.
- `js/spatial-demo.js`, `js/ambient-viz.js`, `design-studio.html` — reference the render path.

---

## 2. Where the original lives — `C:\Projects\webizen-browser`
The renderer's birthplace: `webizen-render`, `webizen-web`, `webizen-desktop`, `webizen-studio`,
`webizen-runtime` + design/status docs that are the spec source: `10D_INTEGRATION_PLAN.md`,
`10D_INTEGRATION_SUMMARY.md`, `ANATOMY_PROJECT_STATUS.md`, `AUDIO_PROJECT_STATUS.md`,
`NAVIGATION_ENGINE_API.md`, `BACKGROUND_VISUALISATION.md`, and `QUALIADB_INVESTIGATION_REPORT.md`
(2026-06-16 — the record of porting `qualia-core-db` into webizen-browser and the module mismatches
hit). These docs are the **authoritative definition source** for the "define it fully" task.

---

## 3. Honest state — "running to some degree", and the two problems

1. **Two parallel renderer codebases.** The native PGA renderer (`webizen-render` + desktop/studio/
   web, *outside* the workspace) and the in-engine WASM portal (`portal_*`, *inside* core-db). They
   overlap (both PGA, both project the logic graph). The split is the main source of "needs
   significant work" — it's unclear which is canonical, and only one is in the build.
2. **The GH-Pages renderer is gated on a WASM bundle that isn't loading** ("WASM Engine Required").
   This is the "disabled by bad code / bad luck" symptom Timothy described — same family as the
   divergent/stale WASM bundles affecting the LLM demos.

---

## 4. What "significant work" actually is (the path to Track C)

1. **Decide the canonical renderer** and its home: either bring `webizen-render` (+ desktop/studio/
   web) into the root workspace and consolidate, OR make the in-engine `portal_*` WASM surface the
   one true browser path and treat `webizen-render` as the native/PNG sibling. (Timothy's call —
   this is an architecture decision, not a mechanical one.)
2. **Fix the WASM bundle** so `spatial.html` / `anatomy.html` actually run on GH Pages (resolve the
   "WASM Engine Required" gate — which bundle, built how, served where).
3. **Fully define the renderer** from the webizen-browser spec docs (§2): the input contract (10D
   tensor → scene), the PGA/Motor projection, the CPU fallback, the shader set.
4. **Write the authoring docs** — "how to define / create apps / pages" on the substrate — the
   thing Timothy needs *before* updating the GH-Pages demos.

**Next decision for Timothy:** which leg first — (1) the canonical-renderer/workspace decision,
(2) diagnosing+fixing the WASM-bundle gate so a demo visibly runs, or (3) reading the
webizen-browser 10D/anatomy/navigation spec docs and writing the unified definition + authoring docs?
