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
  | WASM (browser) | **`wasm32`-unknown-unknown** | WebGPU | WASM-SIMD · WebNN | primary web path; gated by the wgpu fix (0.1) |
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
  - *Acceptance:* `spatial.html` renders the particle field in-browser (no "WASM Engine Required") — screenshot.
- **Rail-check:** consolidation removes drift (one source); no shell deps in the engine; affordability path
  (one shared device) preserved.

### Phase 1 — World-space 3D scene *(closes the ~2.5D → 3D gap; STELLAR §E step 1)*
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
    kinematic multivector animates a joint over `t` (screenshot/test).
- **Rail-check:** deterministic prevention (no probabilistic guess); zero-heap operators.

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

## Progress log

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
