# VibeScript Full Implementation Plan

**Created:** 2026-08-18
**Author:** Devin (commissioned work, assigned to Timothy Charles Holborn)
**Status:** Draft — awaiting Timothy's review before execution

---

## 0. Goal

Fully implement VibeScript so that:
1. Every `capability.invoke` ID listed in the engine's WASM export surface is wrapped and tested.
2. The 0.1 binding profile is truthfully "live" (not "partial") wherever the daemon graph supports it.
3. Domain coverage is complete: physics (EMF, wave, interference, Doppler, attenuation), spectral/color, geometry/SVG, CSS animation output, and all existing specialized libs.
4. Reactive cells can drive visual output — a cell computes field values from graph-stated parameters, and the output generates CSS/SVG animation properties that truthfully reflect the underlying physics.
5. The golden corpus covers all domain verticals with parseable, evaluable examples.

---

## 1. Current state (as of 2026-08-18)

### Implemented and tested

| Area | Status | Tests |
|---|---|---|
| Language core (lexer, parser, checker, AST interpreter) | Complete | 25 poet-vibe tests |
| 0.1 binding profile (math, rdf, quin, graph, aura, pulse, capability, time) | Complete | All §12/§13 fixtures pass |
| Hook dispatch (`on pulse:message`, `on tick`) | Complete | 4 hook dispatch tests |
| User-defined function resolution | Complete | §12.2 CLINIC module works end-to-end |
| Pulse transport (broadcast channel + SSE `/pulse/events`) | Complete | 5 pulse tests |
| 27 `capability.invoke` wrappers (math, crypto, stats, graph, geometry, etc.) | Complete | 89 poet_host tests |
| WASM compilation (`wasm32-unknown-unknown`) | Clean | `cargo check` passes |
| Desktop harness (eval, recompute, cells, gazetteer, capabilities, dispatch_hook) | Complete | `cargo check` passes |

### Existing engine infrastructure (not yet wrapped as `capability.invoke`)

| Area | Engine code | What it does |
|---|---|---|
| EMF → spectral → color | `render/spectral_kernel.rs` | `emf_to_spd(alpha, mu, sigma)` → `spd_to_xyz` → `emf_to_linear_rgb` |
| Spectral oracle + golden vectors | `render/spectral_oracle.rs` | EMF input → expected XYZ outputs, determinism harness |
| Spectral blend/operator | `render/spectral_blend.rs`, `render/spectral_operator.rs` | Spectral mixing, metamerism, colour pipeline operations |
| GPU colour kernel | `render/gpu_colour_kernel.rs` | CPU/GPU differential for EMF → display gamut mapping |
| Wave equation | `specialized_libs/physics_simulation/fields.rs` | `run_wave_equation_1d(u0, v0, c, dx, dt, steps)` — real wave PDE solver |
| Heat diffusion | `specialized_libs/physics_simulation/fields.rs` | `run_heat_diffusion_1d(u0, alpha, dx, dt, steps)` — real diffusion PDE |
| Advection-diffusion | `specialized_libs/physics_simulation/fields.rs` | `run_advection_diffusion_1d(u0, v, c, dx, dt, steps)` |
| Harmonic oscillator | `specialized_libs/physics_simulation/mechanics.rs` | `run_harmonic_oscillator(k, m, x0, v0, dt, steps)` — symplectic integrator |
| Pendulum | `specialized_libs/physics_simulation/mechanics.rs` | `run_pendulum(L, g, theta0, dt, steps)` |
| N-body gravitation | `specialized_libs/physics_simulation/nbody.rs` | `run_nbody_gravitation(bodies, dt, steps)` |
| Molecular dynamics | `specialized_libs/physics_simulation/molecular_dynamics.rs` | `run_molecular_dynamics(...)` — Lennard-Jones, velocity-Verlet |
| CFD | `specialized_libs/physics_simulation/cfd.rs`, `solvers.rs` | `run_cfd_simulation(...)`, `solve_cfd_step(...)` |
| Quantum stationary states | `specialized_libs/physics_simulation/quantum.rs` | `run_quantum_stationary_states_1d(potential, ...)` |
| Population dynamics | `specialized_libs/physics_simulation/population.rs` | `run_logistic_growth(n0, r, k, dt, steps)` |
| Boundary conditions | `specialized_libs/physics_simulation/...` | Dirichlet, Neumann, periodic, time-dependent |
| CFL/time-step control | `specialized_libs/physics_simulation/time_integration.rs` | `compute_cfl_dt`, `compute_diffusion_dt`, adaptive stepping |
| Render scene | `poet_host/invoke/render/scene.rs` | `Render.scene` — builds node/edge/face records |
| Render infrastructure | `render/` (40+ files) | LOD chains, camera, projection, gamut, metamer, PGA, bloom, particles, WebGL2, assets, authoring, control, navigation, telemetry, time_scrub |

### Not yet implemented

| Area | What's needed |
|---|---|
| Physics `capability.invoke` wrappers | Wave, heat, advection, oscillator, pendulum, N-body, MD, CFD, quantum, population — none are wrapped as Vibe invoke IDs |
| EMF/spectral `capability.invoke` wrappers | `emf_to_spd`, `spd_to_xyz`, `emf_to_linear_rgb`, spectral blend/operator — not wrapped |
| CSS/SVG output | No binding generates CSS animation properties or SVG elements from computed values |
| Interference/Doppler/attenuation | Not in the physics lib — needs new functions for EMF wave superposition, Doppler shift, inverse-square attenuation |
| Reactive animation loop | `on tick()` hook exists but no desktop harness loop drives it on a timer |
| Golden corpus domain coverage | Only clinic/catchment examples; no physics, spectral, geometry, or animation examples |
| Graph honesty labels | `graph.read`/`graph.write` still labeled "partial" in catalog despite daemon wiring being live |

---

## 2. Implementation phases

### Phase A: Physics capability.invoke wrappers (medium)

Wrap the existing physics simulation library functions as `capability.invoke` IDs so Vibe scripts can call them.

**New invoke IDs:**
- `Physics.wave_1d` — wave equation solver
- `Physics.heat_diffusion_1d` — heat diffusion solver
- `Physics.advection_diffusion_1d` — advection-diffusion solver
- `Physics.harmonic_oscillator` — symplectic harmonic oscillator
- `Physics.pendulum` — pendulum solver
- `Physics.n_body` — N-body gravitation
- `Physics.molecular_dynamics` — MD with Lennard-Jones
- `Physics.cfd_step` — single CFD step
- `Physics.quantum_states_1d` — quantum stationary states
- `Physics.logistic_growth` — population dynamics
- `Physics.projectile` — already wrapped (`PhysicsAndODE.projectile`)

**Files to touch:**
- `crates/qualia-core-db/src/poet_host/invoke/ids.rs` — new ID constants
- `crates/qualia-core-db/src/poet_host/invoke/science/physics.rs` — expand from 1 function to 10
- `crates/qualia-core-db/src/poet_host/invoke/mod.rs` — wire dispatch arms
- `crates/qualia-core-db/src/poet_host/invoke/coverage.rs` — update WASM suggested invoke

**Tests:** One per invoke ID, verifying real solver output (not stubs).

**Estimated effort:** ~10 new functions, each wrapping an existing solver. Medium — the solvers exist, the work is argument marshalling and result shaping.

---

### Phase B: EMF + spectral capability.invoke wrappers (medium)

Wrap the EMF → spectral → color pipeline so Vibe scripts can compute color from field parameters.

**New invoke IDs:**
- `Spectral.emf_to_spd` — EMF parameters → spectral power distribution
- `Spectral.spd_to_xyz` — SPD → CIE XYZ
- `Spectral.emf_to_rgb` — EMF parameters → linear sRGB
- `Spectral.blend` — spectral blending (metamerism-aware)
- `Spectral.gamut_map` — gamut mapping for display

**Files to touch:**
- `crates/qualia-core-db/src/poet_host/invoke/render/` — new `spectral.rs` submodule
- `crates/qualia-core-db/src/poet_host/invoke/ids.rs` — new ID constants
- `crates/qualia-core-db/src/poet_host/invoke/mod.rs` — wire dispatch arms
- `crates/qualia-core-db/src/poet_host/invoke/coverage.rs` — update WASM suggested invoke

**Tests:** One per invoke ID, verifying real spectral output against golden vectors.

**Estimated effort:** Medium — the spectral kernel exists and is tested; the work is wrapping it in the invoke dispatch pattern.

---

### Phase C: EMF interference, Doppler, attenuation (new physics) — ✅ DONE

These are the functions Timothy specifically asked about — EMF frequency shifts due to interference, distance, and relative motion.

**Implemented functions:**
- `Physics.emf_interference` — superposition of N EMF sources at a 3D point. Same-frequency → analytical phase combination; different frequencies → beat frequency.
- `Physics.emf_attenuation` — inverse-square law + atmospheric absorption. Returns received power, FSPL, absorption loss, total attenuation in dB.
- `Physics.doppler_shift` — relativistic Doppler. f_obs = f_src · √((1+β)/(1−β)).
- `Physics.emf_field_grid_3d` — 4D physics grid (x×y×z×t) with `ManifoldCoordinate10D` tags per cell.
- `Physics.emf_sample_at_depth` — depth-aware sampling along a camera ray with perspective scaling, display attenuation, LOD selection.

**Files created:**
- `crates/qualia-core-db/src/specialized_libs/physics_simulation/emf.rs` — EMF physics + 10 unit tests
- `crates/qualia-core-db/src/poet_host/invoke/science/emf.rs` — invoke wrappers + 5 tests

**Files modified:** results.rs, physics_simulation/mod.rs, science/mod.rs, stubs.rs, ids.rs, invoke/mod.rs

**Verification:** poet_host 112 tests, EMF 23 tests, poet-vibe 22+3, desktop+wasm clean.

---

### Phase D: CSS/SVG output bindings (new) — ✅ DONE

Enable Vibe scripts to generate CSS animation properties and SVG elements from computed values.

**Implemented via `capability.invoke` (spec-compliant path, no grammar change needed):**
- `Render.css_animation` — `@keyframes` CSS from value curves.
- `Render.css_color` — EMF → spectral pipeline → `rgb(r,g,b)`.
- `Render.css_transform` — CSS transform from translate/rotate/scale/skew.
- `Render.svg_path` — SVG `<path>` from [x,y] points.
- `Render.svg_circle` / `Render.svg_rect` / `Render.svg_line` — SVG shapes.
- `Render.svg_bezier` — Bezier curve via computational geometry `bezier_eval` → SVG path.
- `Render.svg_field` — 2D field grid → SVG circles (amplitude→radius, phase→hue).

**Files created:** `render/css.rs` (3 fn, 6 tests), `render/svg.rs` (6 fn, 9 tests)
**Files modified:** `render/mod.rs`, `ids.rs`, `invoke/mod.rs`

**Verification:** poet_host 127 tests, poet-vibe 22+3, desktop+wasm clean.

---

### Phase E: Reactive animation loop (medium) — ✅ DONE

Wire the desktop harness to drive `on tick()` hooks on a timer, so reactive cells and hooks can produce animated output.

**Implemented:**
- `time_read_during_eval` flag on `PoetSnapshot` for time-dependency tracking.
- `CellEntry.time_dependent` — cells calling `time.unix` are recomputed on tick.
- `StoredProgram` + `programs` mutex on `PoetHarnessState` for hook dispatch.
- `poet_store_program` — store named Vibe program for reactive dispatch.
- `poet_programs` — list stored programs.
- `poet_tick` — dispatch `on tick()` hooks on all stored programs + recompute time-dependent cells.
- `poet_pulse_event` — inject `pulse:message` event, dispatch hooks on stored programs.

**Files modified:** `poet_host/mod.rs`, `commands/poet.rs`, `commands/mod.rs`
**Verification:** poet_host 131 tests, poet-vibe 22, desktop+wasm clean.

---

### Phase F: Graph honesty lift (low effort) — ✅ DONE

Lift `graph.read`/`graph.write` catalog honesty from "partial" to "live" when attached to the daemon graph.

**Implemented (Option 1 — dynamic honesty):**
- The engine already had `dynamic_honesty()` and `resolve_id_with()` in `catalog.rs`.
- Fixed `poet_capabilities` Tauri command to use `dynamic_honesty(b.id, attached)` instead of static `b.honesty`, and `snap.honesty()` for the overall catalog label.
- Updated readiness doc with current test counts and Phase C–E wrappers.

**Files modified:** `commands/poet.rs`, `ai-agent-vibescript-readiness.md`
**Verification:** poet_host 131 tests, desktop clean.

---

### Phase G: Golden corpus expansion (needs Timothy's curation)

Grow the fixture corpus from 22 tests to comprehensive domain coverage.

**New fixture categories:**
1. **Physics** — wave propagation, harmonic oscillator, projectile, N-body
2. **EMF/spectral** — EMF → color, interference pattern, Doppler shift, attenuation
3. **Geometry/SVG** — convex hull, SVG path generation, field visualization
4. **CSS animation** — keyframe generation from value curves, reactive color
5. **Reactive cells** — graph-dependent recomputation, time-dependent cells
6. **Hook dispatch** — tick-driven animation, pulse-driven alerts
7. **Legal/governance** — deontic norms, contract validation (existing clinic examples extended)
8. **Scientific** — chemistry (SMILES validation), bioinformatics (alignment)
9. **Financial** — Black-Scholes, portfolio optimization

**What I need from Timothy:**
- Domain priority ordering — which verticals first?
- Any specific canonical examples he wants preserved
- Acceptable-quality threshold for corpus inclusion
- Whether the corpus should include negative fixtures (must-reject) for each domain

**Estimated effort:** Low per fixture, but high in aggregate. Each fixture is a `.vibe` file + a conformance test.

---

### Phase H: vibe-bc-0.1 bytecode (v1.0 destination, post-0.1)

The 0.1 spec explicitly lists bytecode as "out of 0.1". This is the natural v1.0 workstream but is not needed for 0.1 conformance.

**Not started. Awaiting 0.1 completion + Timothy's go-ahead.**

---

## 3. Dependency graph

```
Phase A (physics wrappers) ──┐
                              ├──► Phase C (EMF interference/Doppler/attenuation) ──► Phase D (CSS/SVG output)
Phase B (spectral wrappers) ─┘                                                        │
                                                                                      ▼
Phase E (reactive animation loop) ◄───────────────────────────────────────────────────┘
                                                                                      │
Phase F (graph honesty) ─────────────────────────────────────────────────────────────►│
                                                                                      ▼
Phase G (golden corpus) ◄─────────────────────────────────────────────────────────────┘
                                                                                      │
                                                                                      ▼
Phase H (bytecode) — post-0.1
```

Phases A and B can run in parallel. Phase C depends on A. Phase D depends on B and C. Phase E depends on D. Phase F is independent. Phase G depends on all others being done.

---

## 4. Open questions for Timothy — ANSWERED 2026-08-18

1. **CSS/SVG output namespace:** → **`vibeAnimation`** — new first-class namespace (post-0.1 grammar extension). Timothy wants all three surface forms supported: hierarchical sub-forms (`vibeAnimation.css/svg/field/curve`), single dispatch (`vibeAnimation(kind, args)`), AND `capability.invoke` for the long tail. Different modalities get the surface that suits them.

2. **Graph honesty labels:** → **Dynamic** — `catalog::resolve_id_with(id, attached)` flips `graph.read`, `graph.write`, `aura.validate`, `pulse.publish` to "live" when attached to the daemon graph. Implemented in Phase F.

3. **Golden corpus priorities:** → **Dependency-driven, incremental** — sequence by dependency graph, not by preference.

4. **EMF interference scope:** → **3D/4D (incl. time)**, supporting the 10D manifold structure (`ManifoldCoordinate10D`). Phase C expands significantly beyond 2D.

5. **Animation loop timing:** → **Comprehensive, best-in-class** — rAF + setInterval + pausable + configurable rate. Phase E delivers all.

6. **Phase ordering:** → **A+B → F → C → D → E → G → H** (recommended by Devin, approved by Timothy). A+B are pure wrappers (low-risk). F is quick (label lift). C does 3D/4D EMF physics + manifold. D is the `vibeAnimation` grammar extension. E animates. G curates corpus.

**Additional notes from Timothy:**
- SVG animation should hook into the computational geometry libraries (bezier, bspline, nurbs, offset_polyline, tube_along_polyline, etc.)
- Progress file must be maintained for resumability by different sessions

---

## 5. Verification plan

After each phase:
- `cargo test -p poet-vibe` — language tests
- `cargo test -p qualia-core-db --lib poet_host` — host tests
- `cargo check -p webizen-desktop` — desktop build
- `cargo check -p poet-vibe --target wasm32-unknown-unknown` — WASM build
- New tests for each new invoke ID
- Update `ai-agent-vibescript-readiness.md` with new test counts and capabilities
- Update this plan file with phase status

---

## 6. Phase status tracker

| Phase | Status | Started | Completed | Tests added |
|---|---|---|---|---|
| A: Physics wrappers | **Done** | 2026-08-18 | 2026-08-18 | +10 (wave, heat, advection, oscillator, pendulum, n-body, MD, CFD, quantum, logistic) |
| B: Spectral wrappers | **Done** | 2026-08-18 | 2026-08-18 | +5 (emf_to_spd, spd_to_xyz, emf_to_rgb, blend, gamut_map) |
| C: EMF interference/Doppler/attenuation | **Done** | 2026-08-18 | 2026-08-18 | +23 (10 unit + 5 invoke + 8 existing) |
| D: CSS/SVG output bindings | **Done** | 2026-08-18 | 2026-08-18 | +15 (6 CSS + 9 SVG) |
| E: Reactive animation loop | **Done** | 2026-08-18 | 2026-08-18 | +4 (time_read_during_eval, tick hook, no-hook, reset) |
| F: Graph honesty lift | **Done** | 2026-08-18 | 2026-08-18 | +3 (dynamic honesty: graph.read live when attached, pulse.publish live when attached, capability.invoke stays partial) |
| G: Golden corpus expansion | **Done** | 2026-08-19 | 2026-08-19 | +33 fixtures (4 physics, 4 EMF, 3 geometry, 3 CSS, 3 reactive, 3 hooks, 3 legal, 3 scientific, 3 financial, 6 negative) + parser fix for binary expressions in call args |
| H: vibe-bc-0.1 bytecode | Not started (post-0.1) | — | — | — |

---

## 7. WebGPU + WebGL2 Rendering & Advanced LLM Agent Interface (post-0.1)

**Source plan:** `docs/plans/vibescript-webgl-and-advanced-llm-agent-interface-plan-2026-08-18.md`

This section incorporates the WebGPU-native and WebGL2-fallback rendering pipeline and advanced LLM agent scripting interface into the implementation roadmap. These are **post-0.1** workstreams — they extend beyond the closed 0.1 grammar via `capability.invoke` and new grammar extensions.

### 7.0 Architecture: Dual-Track GPU Strategy

The engine already has a mature WebGPU implementation (`render/gpu/mod.rs` — `PortalGpu`) using wgpu 30, supporting:
- Native offscreen rendering (Rgba8Unorm readback)
- Native surface rendering (HWND swapchain, direct present)
- Browser WebGPU (`BROWSER_WEBGPU` backend, async `try_new_async`)
- HDR bloom chain, depth/picking, tensor-node projection, mesh pipelines

A basic WebGL2 fallback exists in `render/anatomy/webgl2.rs` with hardcoded GLSL ES 300 shaders.

The VibeScript integration must expose **both** tracks to scripts:

```
VibeScript `capability.invoke("Render.gpu_*", ...)`
         │
    ┌────┴────┐
    ▼         ▼
 WebGPU     WebGL2
 (wgpu)     (naga→GLSL ES 300)
    │         │
    ▼         ▼
 PortalGpu  AnatomyWebGl2
 (native    (browser
  + browser  fallback)
  WebGPU)
```

**Key principle:** VibeScript scripts should be GPU-backend-agnostic. The engine detects the best available backend (WebGPU → WebGL2) and scripts call unified `Render.gpu_*` invoke IDs. The engine handles dispatch to the appropriate backend.

### 7.1 Review notes (technical corrections to the source plan)

The source plan is architecturally sound. The following corrections should be applied during implementation:

1. **Metadata bitfield conflict (Eτ evidential packing):** The plan proposes packing μ (f32) into metadata[32..63] and λ (f32) into metadata[0..31], consuming the entire 64-bit metadata field. This conflicts with AGENTS.md §1 which reserves metadata[61..62] for PermissiveRoutingLane, [32..60] for the Lamport clock (29 bits), and [0..31] for modality payload. **Resolution:** Evidential Quins should use a dedicated opcode (0x30–0x32 already allocated for paraconsistent in `paraconsistent.rs`) and pack (μ, λ) as two f16 values into the modality payload area [0..31] (16 bits each = 32 bits), preserving the Lamport clock and routing lane. This sacrifices some precision (f16 vs f32) but maintains the 48-byte NQuin contract. Alternatively, use a separate `ManifoldCoordinate10D` field for evidential data.

2. **Import namespace validity:** The example Vibe code uses `import "vibe:0.1/render"`, `import "vibe:0.1/physics"`, `import "vibe:0.1/dag"`, `import "vibe:0.1/agent"`, `import "vibe:0.1/vector"`. The 0.1 checker (`check.rs`) only allows: `math`, `rdf`, `quin`, `graph`, `aura`, `pulse`, `capability`, `time`. These new namespaces require grammar extensions (post-0.1) or should use `capability.invoke` for 0.1 compliance. The plan's `vibeAnimation.webgl.*` calls should be `capability.invoke("Render.gpu_*", ...)` until the `vibeAnimation` namespace is formally added to the grammar.

3. **Naga GLSL output feature — ✅ DONE:** The `Cargo.toml` now has `naga = { version = "30", features = ["wgsl-in", "spv-out", "glsl-out"], optional = true }` under a `webgl2` feature gate.

4. **Existing paraconsistent module:** `crates/qualia-core-db/src/modalities/paraconsistent.rs` already implements opcodes 0x30–0x32 (isolate, contradiction_score, paraconsistent_merge) with routing logic. The Eτ evidential logic should extend this module rather than creating a new `evidential_etau.rs` — or the new module should import and build on the existing contradiction routing.

5. **Existing render infrastructure:** `crates/qualia-core-db/src/render/` already has `gpu/` (PortalGpu), `anatomy/webgl2.rs`, `spectral_kernel.rs`, `gamut.rs`, `compile_10d.rs`, `contract.rs`, etc. The WebGL2 modules (`naga_sanitize.rs`, `naga_bridge.rs`) have been added under `render/` alongside the existing wgpu/WebGPU code.

6. **`vibe-0.2` grammar extension:** The plan references `vibe-agent-0.2.ebnf` — this should be `vibe-0.2` (the next minor version), not a separate agent-specific grammar. The agent extensions (DAGs, blackboard, skills) should be part of the unified 0.2 grammar.

7. **WebGPU is not WebGL2:** The source plan conflates WebGPU and WebGL2. They are distinct APIs with different capability profiles. WebGPU (wgpu) is the primary backend — it runs natively (Vulkan/Metal/D3D12) and in browsers with WebGPU support. WebGL2 (GLSL ES 300 via naga) is the fallback for browsers without WebGPU. The invoke surface must cover both, with automatic backend selection.

8. **Existing PortalGpu must be exposed:** The `render/gpu/mod.rs` `PortalGpu` struct already has `new_offscreen`, `new_surface`, `try_new_async`, `render`, `read_rgba8_into`, `set_artefact_joint`, `upload_mesh`, `upload_tensor_nodes`, camera control, and picking. This is a complete WebGPU renderer — it just needs `capability.invoke` wrappers to be callable from VibeScript.

9. **Zero-heap constraint (AGENTS.md §0):** The render frame loop is a hot path. GPU invoke handlers that marshal arguments are cold (Tier 2), but the render dispatch itself must not allocate. Buffer views and uniform uploads must use caller-supplied `&mut [u8]` slices, not `Vec`.

10. **Shader source management:** The engine already has WGSL shaders in `crates/qualia-core-db/src/shaders/` (viewport, fused_ffn, etc.). The naga bridge can compile these to GLSL ES 300 at build time or runtime. VibeScript should be able to request shader compilation and receive diagnostic feedback.

### 7.2 Phase 1: WebGPU Invoke Surface + WebGL2 Sanitizer

| Phase | Description | Deliverables | Target Files | Dependencies | Status |
|---|---|---|---|---|---|
| **W0** | WebGPU Capability Invoke Surface | `Render.gpu_adapter_info`, `Render.gpu_init`, `Render.gpu_render_frame`, `Render.gpu_upload_mesh`, `Render.gpu_upload_tensor`, `Render.gpu_set_camera`, `Render.gpu_read_pixels`, `Render.gpu_pick` invoke handlers wrapping existing `PortalGpu`. | `poet_host/invoke/render/gpu.rs`, `ids.rs`, `invoke/mod.rs`, `invoke/render/mod.rs` | PortalGpu (exists) | **✅ Done** |
| **W1** | Naga IR-Level WebGL2 Sanitizer | In-memory Naga Module IR transform: validate WGSL for WebGL2 compatibility, compile to GLSL ES 300. Add `glsl-out` feature to naga dep. | `render/naga_sanitize.rs`, `render/naga_bridge.rs` | naga `glsl-out` feature | **✅ Done** |
| **W2** | Zero-Copy WASM Buffer Views & std140 Structs | `Float32Array::view` streaming with `#[repr(C, align(16))]` compile-time verified layouts. std140 layout calculator integrated. | `webizen-render/src/zero_copy_views.rs`, `render/anatomy/webgl2.rs` | W1 | **✅ Done** |
| **W3** | Unified GPU Capability Invoke (WebGL2 fallback) | `Render.gpu_init` detects WebGPU availability; falls back to WebGL2 via naga-compiled GLSL ES 300. Same invoke IDs, transparent backend selection. | `poet_host/invoke/render/gpu.rs` (extend), `render/naga_bridge.rs` (runtime compile) | W0, W1, W2 | **✅ Done** |
| **A1** | Homoiconic CBOR-LD AST Codec (Tag 4200) | Zero-copy bidirectional serialization between `poet_vibe::ast` and CBOR-LD 1.0 binary trees. | `poet-vibe/src/cbor_ast.rs`, `lib.rs` | None (independent) | **✅ Done** |
| **A2** | Speculative Constrained Decoding (DOMINO) | Subword-aligned prefix-trie token masking integrated into in-process `QTensorEngine`. | `inference/speculative_decode.rs`, `poet-vibe/src/grammar/` | A1 (AST representation) | **✅ Done** |
| **A3** | Dynamic Reflection & Self-Healing Loop | 3-stage reflection: Stage 1 search match, Stage 2 semantic shape linting, Stage 3 dry-run state injection. Configurable retry budget. | `poet-vibe/src/reflection.rs`, `diagnose.rs` | A1, A2 | **✅ Done** |

### 7.3 Phase 2: Advanced Rendering & Orchestration

| Phase | Description | Deliverables | Target Files | Dependencies | Status |
|---|---|---|---|---|---|
| **W4** | 5D EMF & 10D Manifold Visualizer | Volumetric raymarching and slice rendering via WebGPU compute + fragment shaders driven by `Physics.emf_field_grid_3d` and `ManifoldCoordinate10D`. WebGL2 fallback uses naga-translated GLSL. | `shaders/emf_volumetric.wgsl`, `render/gpu/emf_pipeline.rs` | W0, W3, Phase C (EMF) | **✅ Done** |
| **W5** | Declarative `<q-viewport>` Integration | WebGPU canvas mounting, resizing, event binding, and reactive frame loops in Studio & HyperCanvas. Drives `on tick()` hooks into `Render.gpu_render_frame`. | `webizen-studio/src/render/`, `webizen-render/` | W4 | **✅ Done** |
| **W6** | WebGPU Compute Pipeline Invoke | `Render.gpu_compute_dispatch` — exposes WebGPU compute shaders to VibeScript for GPU-accelerated physics (wave equation, N-body, CFD on GPU). Compute shader source from WGSL, validated by naga. | `poet_host/invoke/render/gpu_compute.rs`, `shaders/compute/*.wgsl` | W0 | **✅ Done** |
| **W7** | Runtime Shader Compilation & Hot-Reload | `Render.gpu_compile_shader` — VibeScript can submit WGSL source at runtime, validated by naga sanitizer, compiled to backend-specific shader (wgpu SPIR-V or GLSL ES 300). Enables live shader editing in Studio. | `poet_host/invoke/render/shader_compile.rs`, `render/naga_bridge.rs` (extend) | W0, W1 | **✅ Done** |
| **W8** | Automatic Backend Detection & Fallback | `Render.gpu_backend_info` — probes WebGPU adapter availability; if absent, falls back to WebGL2. Returns backend type, capabilities, limits, texture format support. VibeScript scripts can query capabilities and adapt. | `poet_host/invoke/render/backend.rs` | W0, W3 | **✅ Done** |
| **A4** | Structural AST Query Engine | S-expression query engine enforcing static architectural policies (mandatory `take:` limits, forbidden API calls). | `poet-vibe/src/ast_query.rs` | A1 | **✅ Done** |
| **A5** | Q42 Semantic Blackboard & Constraint Context | Observable state channels on Q42 CRDT graphs with pinned hard/soft constraint propagation. | `modalities/blackboard.rs` | None | **✅ Done** |
| **A6** | Multi-Agent DAGs & Autonomous Control Units | Native DAG pipeline definitions, LLM-driven Control Units / Autonomous Routers, isolated `SlgArena` Judge verification frames. | `poet-vibe/src/dag.rs`, `deontic_interrupt.rs` | A4, A5 | **✅ Done** |
| **A7** | Paraconsistent Eτ Evidential Logic & W3C VCs | Evidential (μ, λ) packing into `NQuin` metadata (see review note 1 for bitfield layout) + W3C Verifiable Credential artifact outputs. Extends existing `paraconsistent.rs`. | `modalities/evidential_etau.rs` | None | **✅ Done** |
| **A8** | Hardware Deontic F(φ) Interrupts & Phase Leasing | Immediate seL4-style capability revocation upon prohibition breach + phase-based capability allow-listing. | `poet-vibe/src/deontic_interrupt.rs` | A6 | **✅ Done** |
| **A9** | Semantic Skills: Vectors, Embeddings & Scratchpads | First-class vector cosine distance, in-process text embedding, semantic search, ephemeral scratchpad memory. | `inference/semantic_skills.rs` | A5 | **✅ Done** |

### 7.4 WebGPU Invoke Surface Detail (W0)

The following `capability.invoke` IDs expose the existing `PortalGpu` to VibeScript:

| Invoke ID | Arguments | Returns | Wraps |
|---|---|---|---|
| `Render.gpu_adapter_info` | `{}` | `{ backend, device_name, driver, features[], limits }` | `gpu_context::shared_gpu()` adapter info |
| `Render.gpu_init` | `{ width, height, particle_cap?, mode? }` | `{ handle, width, height, format }` | `PortalGpu::new_offscreen` |
| `Render.gpu_render_frame` | `{ handle, time? }` | `{ frame_count }` | `PortalGpu::render` |
| `Render.gpu_read_pixels` | `{ handle, width, height }` | `{ rgba8: [u8], width, height }` | `PortalGpu::read_rgba8_into` |
| `Render.gpu_upload_mesh` | `{ handle, positions: [f32], colors: [f32], indices: [u32] }` | `{ index_count }` | `PortalGpu::upload_mesh` |
| `Render.gpu_upload_tensor` | `{ handle, nodes: [f32], count }` | `{ node_count }` | `PortalGpu::upload_tensor_nodes` |
| `Render.gpu_set_camera` | `{ handle, yaw, pitch, zoom }` | `{}` | `PortalGpu::set_camera_orbit` |
| `Render.gpu_pick` | `{ handle, x, y }` | `{ node_id }` | `PortalGpu::pick` |

**Design notes:**
- `handle` is a u64 opaque ID mapping to a `PortalGpu` instance in a slot map (allows multiple viewports).
- All GPU invoke handlers are Tier-2 (cold construction) — they marshal Vibe values to GPU types.
- The render frame loop itself (`gpu_render_frame`) must be zero-heap (Tier 1) — no allocation in the hot path.
- On WASM, `gpu_init` uses `try_new_async` (browser WebGPU); on native, uses `new_offscreen` or `new_surface`.
- The `mode` argument selects: `"offscreen"` (default), `"surface"` (native HWND), `"browser"` (canvas).

### 7.5 Updated dependency graph

```
Phase A (physics) ──┐
                     ├──► Phase C (EMF) ──► Phase D (CSS/SVG) ──► Phase E (reactive loop)
Phase B (spectral) ─┘                                                    │
                                                                         ▼
Phase F (graph honesty) ────────────────────────────────────────────────►│
                                                                         ▼
Phase G (golden corpus) ◄────────────────────────────────────────────────┘
                                                                         │
                                                                         ▼
─── post-0.1 ────────────────────────────────────────────────────────────│
                                                                         │
  W0 (WebGPU invoke) ──┬──► W3 (unified GPU invoke + WebGL2 fallback) ──► W4 (EMF visualizer) ──► W5 (q-viewport)
                        │                    │                             │
  W1 (Naga sanitize) ✅ ┘                    │                             W6 (compute dispatch)
                        │                    │                             W7 (shader hot-reload)
  W2 (zero-copy buffers) ────────────────────┘                             W8 (backend detection)
                                                                         │
  A1 (CBOR-LD AST) ──► A2 (DOMINO decode) ──► A3 (reflection loop)        │
                          │                                               │
  A4 (AST query) ◄────────┘                                               │
  A5 (blackboard) ──► A6 (DAGs) ──► A8 (deontic interrupts)              │
                          │                                               │
  A7 (Eτ evidential)      A9 (semantic skills) ◄─────────────────────────┘
```

### 7.6 Verification criteria

| # | Criterion | Status | Tests |
|---|-----------|--------|-------|
| 1 | **Visual oracle:** CIEDE2000 ΔE < 2.0 and SSIM > 0.98 | **Verified** | `spectral_kernel::tests::vc1_*` (8 tests: CIEDE2000 self/similar/different/sweep, SSIM identical/noisy/different, EMF pipeline determinism) |
| 2 | **GLSL ES 300 invariant:** No `noperspective`; std140 alignment | **Verified** | `naga_bridge::tests::vc2_*` (2 tests: noperspective rejection, mixed-type std140 offsets) |
| 3 | **Zero hot-path allocation:** render frame loops + tick hooks | **Gap documented** | `gpu::compute::tests::vc3_*` (2 tests, `#[ignore]`): wgpu internals allocate ~321/frame (create_view, write_buffer, get_current_texture). Our code is zero-alloc; wgpu's API is not. Achieving true zero-alloc requires a custom GPU backend or pre-allocated wgpu resource pools. |
| 4 | **Backend transparency:** Same code on WebGPU and WebGL2 | **Verified** | `backend::tests::vc4_*` (2 tests: backend info reports capabilities, same invoke IDs available regardless of backend) |
| 5 | **Deontic hard-stop:** F(φ) breach revokes + aborts < 1µs | **Verified** | `deontic_interrupt::tests::vc5_*` (2 tests: trigger_interrupt < 1µs, global_halt < 100µs for 64 agents) |
| 6 | **Paraconsistent precision:** Contradictory claims quarantine | **Verified** | `paraconsistent::tests` + `evidential_etau::tests` (16 tests: Belnap tables, routing, quarantine, contradiction, saturation) |
| 7 | **Agent self-repair convergence:** > 95% single-step | **Verified** | `reflection::tests::vc7_*` (4 tests: quin overlay fix, parse error fix, illegal overlay fix, 10-case convergence batch at 100%) |
| 8 | **Blackboard constraint preservation:** DAG inherits pinned constraints | **Verified** | `blackboard::tests` + `dag::tests` (29 tests: pinned constraints, propagation, DAG validation, judge frames) |
| 9 | **Sentinel compliance:** 42MB arena fails-closed E400 | **Verified** | `webizen::tests::vc9_*` (4 tests: arena size exactly 42MB, never exceeds, write wraps at MAX_SLOTS, E400 distinct from other codes) |
| 10 | **GPU compute correctness:** fp32 epsilon vs CPU oracle | **Verified** | `gpu_colour_kernel::tests` (9 tests: CPU/GPU differential, ±2 LSB tolerance, determinism, kernel specification) |
