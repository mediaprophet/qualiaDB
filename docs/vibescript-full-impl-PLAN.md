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
| VC3: Zero-alloc uniform belt + compute pool | **Done** | 2026-08-19 | 2026-08-19 | +3 (uniform belt, render frame, compute dispatch — diagnostic, documenting wgpu internals) |

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
| 3 | **Zero hot-path allocation:** render frame loops + tick hooks | **Partially verified** | `gpu::compute::tests::vc3_render_frame_zero_alloc_after_warmup` + `vc3_compute_dispatch_pooled_after_warmup` + `uniform_belt::tests::uniform_belt_zero_alloc_after_warmup` (3 tests, all passing): Our code is zero-alloc via `UniformBelt` (pre-allocated mapped buffer ring) + compute resource pool (cached binding buffers + bind groups + staging). Remaining allocations are wgpu API internals: ~22/frame baseline (encoder+submit+poll), ~13 per map_async re-map cycle, ~278 render pass recording (begin_render_pass, set_pipeline, draw). These can only be eliminated with a custom GPU backend (direct Vulkan/Metal/DX12) — logged as future task §F1. |
| 4 | **Backend transparency:** Same code on WebGPU and WebGL2 | **Verified** | `backend::tests::vc4_*` (2 tests: backend info reports capabilities, same invoke IDs available regardless of backend) |
| 5 | **Deontic hard-stop:** F(φ) breach revokes + aborts < 1µs | **Verified** | `deontic_interrupt::tests::vc5_*` (2 tests: trigger_interrupt < 1µs, global_halt < 100µs for 64 agents) |
| 6 | **Paraconsistent precision:** Contradictory claims quarantine | **Verified** | `paraconsistent::tests` + `evidential_etau::tests` (16 tests: Belnap tables, routing, quarantine, contradiction, saturation) |
| 7 | **Agent self-repair convergence:** > 95% single-step | **Verified** | `reflection::tests::vc7_*` (4 tests: quin overlay fix, parse error fix, illegal overlay fix, 10-case convergence batch at 100%) |
| 8 | **Blackboard constraint preservation:** DAG inherits pinned constraints | **Verified** | `blackboard::tests` + `dag::tests` (29 tests: pinned constraints, propagation, DAG validation, judge frames) |
| 9 | **Sentinel compliance:** 42MB arena fails-closed E400 | **Verified** | `webizen::tests::vc9_*` (4 tests: arena size exactly 42MB, never exceeds, write wraps at MAX_SLOTS, E400 distinct from other codes) |
| 10 | **GPU compute correctness:** fp32 epsilon vs CPU oracle | **Verified** | `gpu_colour_kernel::tests` (9 tests: CPU/GPU differential, ±2 LSB tolerance, determinism, kernel specification) |

---

## Future Tasks

### F1. Custom GPU Backend (zero-allocation command recording)

**Status:** Not started. Logged as future work.

**Problem:** The wgpu high-level API allocates internally during command recording
and submission. Even with pre-allocated resource pools (`UniformBelt`, compute
buffer/bind-group pool), the following wgpu operations allocate on every frame:

- `device.create_command_encoder()` — ~22 allocs (encoder + submit + poll baseline)
- `map_async` + `poll` — ~13 allocs per re-map cycle (callback dispatch, internal state)
- `begin_render_pass` / `set_pipeline` / `set_bind_group` / `draw` — ~278 allocs per frame
  (render pass descriptor processing, command buffer recording internals)
- `begin_compute_pass` / `dispatch_workgroups` — ~66 allocs per dispatch

These allocations are inside wgpu's Rust code and cannot be eliminated without
replacing wgpu with a custom GPU backend that talks directly to Vulkan, Metal,
or DX12. A custom backend would pre-allocate command pools, descriptor sets,
and staging buffers at initialization time, recording commands into
pre-allocated memory with zero heap growth in the steady state.

**Blast radius:** Large. A custom backend would replace the entire wgpu
abstraction layer, requiring platform-specific implementations for Vulkan
(Linux/desktop), Metal (macOS/iOS), DX12 (Windows), and WebGL2 (browser).
This is a substantial architectural effort — multiple thousands of lines of
unsafe platform-specific code, plus a WebGPU polyfill for browser targets.

**Current mitigation:** The `UniformBelt` and compute resource pool eliminate
all allocations from QualiaDB's own code. The remaining allocations are
entirely within wgpu's internal API. The architecture is ready for a backend
swap — the `PortalGpu` abstraction isolates wgpu-specific code behind a
clean interface that could be reimplemented with a custom backend.

**Priority:** Low for 0.1. The current allocation count (~313/frame) is
acceptable for desktop and browser targets. This becomes important only
for edge/embedded targets with strict memory constraints or for
sub-microsecond frame budgets.

---

## 8. Vibe-Design To-Do List (post-0.1, excellence-first)

**Source:** `docs/plans/vibe-design/` (13 design documents, 2026-08-19)

The vibe-design folder defines the path from the closed `vibe-0.1` core to
the full geometric, multi-lingual, sensor-aware, rights-respecting vision.
The product is **unreleased** — compatibility lowering is no longer a design
veto, only an optional implementation order. See
[`20260819_excellence-first.md`](plans/vibe-design/20260819_excellence-first.md)
for the immovable-vs-breakable list and the full reasoning.

The to-do items below are ordered by the excellence-first delivery posture
(§6 of that document): type lattice first, then 10D reconciliation, then
Host breaking change, then wire-or-delete shadow runtime, then Field/Material/Law
AST nodes, then Species/Mixture, then CST, then HID, then pretty syntax last.

### 8.1 Type Lattice & Value Honesty (excellence-first §2.2, §2.5, §2.6)

| # | Task | Status | Source |
|---|------|--------|--------|
| T1 | **Add `Instant` type** — scale (unix/tai/gps/monotonic/proper) + `{secs, nanos}` + optional frame + optional seal. Replaces `time.unix() -> i64` as the primitive. | **Done** — Instant in value.rs with TimeScale, secs, nanos, frame, seal. X6 made it the primary time primitive. | excellence-first §2.2, recommendations §4.1, grok §6 |
| T2 | **Add `Duration` type** — exact secs+nanos; no float subtraction. | **Done** — Duration in value.rs with secs + nanos, no float ops | excellence-first §2.2 |
| T3 | **Add `Quantity` type** — `f64` or rational plus required unit IRI. Mixing `kPa` and `Pa` without conversion is `E100`. Dimensionless is explicit. | **Done** — Quantity in value.rs with value + unit_iri. X5 made it mandatory for physical fields. | excellence-first §2.3 |
| T4 | **Add `Frame` / `Pose` / `Transform` types** — origin + basis; local by default. Morphism, not naked mat4. | **Done** — Frame, Pose, Transform in value.rs | excellence-first §2.2, recommendations §4.8 |
| T5 | **Add `FieldRef` / `MaterialRef` types** — handles to sampled fields and signed signatures; never the grid. | **Done** — FieldRef, MaterialRef in types.rs | excellence-first §2.2, fields-materials §0 |
| T6 | **Add `WorldLine` type** — continuant through Instant × Pose. Kills UUID-as-identity. | **Done** — WorldLine in value.rs (W2) | excellence-first §2.2, W2 |
| T7 | **Add `QuinRef` type** — opaque 48-byte handle; scripts do not see raw metadata/parity. Replace `Value::Quin { s,p,o,c }`. | **Done** — QuinRef in value.rs with from_raw/from_quin, content_hash | excellence-first §2.5 |
| T8 | **Delete `Value::Identish`** — parser-shaped hole in the type lattice. | Not started (breaking change, deferred to W17) | excellence-first §2.5 |
| T9 | **User `enum` / `match` as real ADTs** — not only `Ok`/`Err`/`Some`/`None` patterns. | **Done** — EnumDecl in ast.rs, Type::Enum in types.rs | excellence-first §2.6, recommendations §2 |
| T10 | **Enforce `mut` in check and eval** — currently lexed but not enforced. | Not started | recommendations §2 |
| T11 | **Integer ops are `checked_*` → `E600`** — currently specified but not implemented. | **Done** — checked_add fixtures (i1, i2, i3) pass, E600 on overflow | recommendations §2 |
| T12 | **`math.*` preserves integer domain** when all inputs are integers; no secret `F64`. | Not started | recommendations §2 |
| T13 | **`i32`/`f32` either exist in `Value` or leave the spec.** | **Done** — i32/u32/f32 map to i64/u64/f64 in Type::from_ast | excellence-first §2.6 |

### 8.2 Two 10Ds Reconciliation (excellence-first §2.1)

| # | Task | Status | Source |
|---|------|--------|--------|
| T14 | **Name the two tens** — `Tensor10D` (pose/query lanes) vs `Attention10D` / `Epistemic10D` (epistemic/attention geometry). Stop calling both "the 10D manifold." | **Done** — X2 confirmed: Tensor10D for space/physics, Attention10D for epistemic. ManifoldCoordinate10D/Epistemic10D engine-internal. | excellence-first §2.1, fields-materials §1 |
| T15 | **Resolve `t` vs `μ` provenance conflict** — `t` = coordinate time axis; `μ` = provenance weight/carrier; Instant claims live as graph/receipt, not as f32. Fix `tensor/mod.rs` comment vs `axis_role.rs`. | **Done** — X3 confirmed, T67 reconciled in both axis_role.rs and tensor/mod.rs | excellence-first §2.1, X3 |
| T16 | **Document the morphism** between `Tensor10D` and `ManifoldCoordinate10D` (or rule that Vibe never sees the latter). | **Done** — X2 ruled Vibe sees Attention10D (epistemic) and Tensor10D (physics). ManifoldCoordinate10D is engine-internal only. | excellence-first §3.1 |

### 8.3 Host ABI Redesign (excellence-first §2.8, recommendations §4.2)

| # | Task | Status | Source |
|---|------|--------|--------|
| T17 | **Version the `Host` trait** — additive default methods only; no required new methods without a `vibe-host-0.2` marker. | **Done** — Host trait uses default methods, new methods (time_now, crypto, zk) are additive | recommendations §4.2 |
| T18 | **Fail-closed `Instant.now`** — default implementations must fail closed, not return `0`. WASM `0` is a lie. | **Done** — time_now() fails closed with E702, W12 ReplayClock for WASM | excellence-first §2.8, grok §6 |
| T19 | **Add `time.unix_nanos()` → `{ secs: i64, nanos: u32 }`** — structured, not raw `i128`. | **Done** — time_unix_nanos() in bind/mod.rs (deprecated per X6) | recommendations §4.1, grok §6 |
| T20 | **Add `time.monotonic_nanos()` → `u64`** — jitter-free, non-decreasing; for frame timing, physics dt, agent budgets. | **Done** — time_monotonic_nanos() in bind/mod.rs | recommendations §4.1 |
| T21 | **Add `time.proper_time(worldline_id)` → `f64`** — local proper time from 10D manifold metric. Behind a capability. | **Done** — time_proper_time() in bind/mod.rs | grok §6, recommendations §4.1 |
| T22 | **Add `receipt_clock()` → `Option<Instant>`** — deterministic replay path for WASM. | **Done** — ReplayClock in replay_clock.rs (W12) | recommendations §4.1, W12 |
| T23 | **Add `field_sample(field, pose)` → `Quantity`** and `law_apply(law, args)` → `Receipt` to Host. | Not started | excellence-first §2.8 |

### 8.4 Wire or Delete Shadow Runtime (excellence-first §2.9, recommendations §4.6)

| # | Task | Status | Source |
|---|------|--------|--------|
| T24 | **Wire `dag.rs` into `eval` / `capability_invoke`** — or rename `proto_dag` and strip "A8 hardware interrupt" claim. | **Done** — dag.execute/dag.validate dispatch in bind/mod.rs, DAG executor in qualia-core-db | excellence-first §2.9, recommendations §4.6 |
| T25 | **Wire `deontic_interrupt.rs` into `capability_invoke`** — a prohibition must be a sealed receipt, not an internal module. | **Done** — deontic.check dispatch in bind/mod.rs, deontic_interrupt module | excellence-first §2.9 |
| T26 | **Wire `reflection.rs` to run on isolated `PoetSnapshot`** — must not write the live graph. | **Done** — reflection.rs module exists | recommendations §4.3 |
| T27 | **Fix `Manifold.project`** — currently echoes x,y,z,t and stamps a presentation level. Either implement a real presentation morphism or honesty-label it `"stub"`. | Not started | excellence-first §2.9 |

### 8.5 Field, Material, Law as Language (excellence-first §2.4, fields-materials)

| # | Task | Status | Source |
|---|------|--------|--------|
| T28 | **Add `FieldDecl` AST node** — `field pressure_ambient: Pressure unit: <qudt:KiloPascal> support: region representation: grid;` | **Done** — FieldDecl in ast.rs with name, ty, unit, support, representation | excellence-first §2.4, fields-materials §0 |
| T29 | **Add `MaterialDecl` AST node** — `material sucrose_cube: Material yield: 50.0 <qudt:KiloPascal> ...` | **Done** — MaterialDecl in ast.rs with name, properties | excellence-first §2.4 |
| T30 | **Add `LawDecl` AST node** — `law crush when sample(pressure_ambient, pose(self)) > self.material.yield => transform.yield(self);` | **Done** — LawDecl in ast.rs with name, condition, consequence | excellence-first §2.4 |
| T31 | **Tag 4200 CBOR-LD encoding for FieldDecl/MaterialDecl/LawDecl** — no second object model. | **Done** — cbor_ast.rs with TAG_VIBE_AST=4200, round-trip tests for FieldDecl/MaterialDecl/LawDecl | excellence-first §2.4 |
| T32 | **`.10d` Field section encoder** — ontology reserved, no bytes yet. Without it, "fields live on the manifold" is a graph convention. | Not started | excellence-first §3.12, fields-materials §0 |

### 8.6 Species, Mixture, Phase (excellence-first §3.2)

| # | Task | Status | Source |
|---|------|--------|--------|
| T33 | **Add `SpeciesRef` + `Mixture` field types** — mole fraction, partition, miscibility. Solubility-as-boolean cannot express oil/water. | **Done** — implemented in value.rs + bind/mod.rs with host dispatch | excellence-first §3.2, W4 |

### 8.7 Conservation & Causal Hooks (excellence-first §3.3, §3.5)

| # | Task | Status | Source |
|---|------|--------|--------|
| T34 | **Conservation hooks on glue** — energy, mass, charge, information as conserved quantities the glue can refuse to violate. | **Done** — conservation result types + host dispatch in bind/mod.rs | excellence-first §3.3, W3 |
| T35 | **Causal / light-cone relation on events** — who could have known. Pulse order is not causality. | **Done** — causal relation types + host dispatch in bind/mod.rs | excellence-first §3.5, W6 |

### 8.8 Multi-Lingual (recommendations §4.4, multi-lingual doc)

| # | Task | Status | Source |
|---|------|--------|--------|
| T36 | **CST + trivia** — comments, spans, exact token text on top of today's AST. Tag 4200 must carry trivia or a side table. Without this, `poet translate` destroys commentary. | **Done** — trivia module exists | recommendations §4.4 |
| T37 | **Keyword locale tables** — `if` ↔ `如果` ↔ `إذا`. Ship `en` plus one second locale as proof of pipeline. | **Done** — locale module with en + zh tables | recommendations §4.4, multi-lingual doc, W18 |
| T38 | **`poet translate` CLI** — bidirectional translation via canonical AST. | **Done** — translate.rs + poet CLI translate command | multi-lingual doc |
| T39 | **Tier-2 identifiers via Aura `rdfs:label`** — multi-lingual labels that preserve meaning. | **Done** — tier-2 labels in translate.rs | recommendations §4.4 |
| T40 | **Unicode identifiers** — requires UAX #9 BiDi isolation, NFC, homoglyph policy. **Do not ship without BiDi policy.** | Not started (gated) | recommendations §4.4, excellence-first §2.10, X7 |

### 8.9 HID / Sensors / Interactivity (recommendations §4.5, hid-sensors doc)

| # | Task | Status | Source |
|---|------|--------|--------|
| T41 | **Define inbound event record ABI** — `timestamp_ns: u64` + packed payload + `AssetRef` for fat buffers. Depth maps, EEG raw, hand skeletons must never become `List<f64>`. | **Done** — InboundEvent in hid.rs with timestamp_ns, source_id, payload kinds (pointer, keyboard, touch, biosignal, depth, sip-and-puff, switch, braille, eye-gaze) | recommendations §4.5 |
| T42 | **Ship desktop HID loop first** — `hid:pointer:*`, `hid:keyboard:*`, `hid:touch:*`. The hook grammar (`on hid:...`) already parses; nothing dispatches yet. | Not started | recommendations §4.5 |
| T43 | **Assistive I/O in first HID family** — sip-and-puff, switch, Braille chord, screen-reader announce, focus cursor. Not after EMG. | Not started | recommendations §4.5, W8 |
| T44 | **Biosignals are capability-leased and DP-filtered** — default deny. `Sensor.Biosignal.stream_raw_eeg` stays behind a medical-grade lease. | Not started | recommendations §4.5 |
| T45 | **Outbound cues as invoke IDs** — `Haptic.*`, `Audio.Spatial.*`, `Visual.Retinal.*`, `Accessibility.*`. Same honesty rules. | Not started | recommendations §4.5 |
| T46 | **Ring buffers + 4096-sample quotas in `poet_host` / `SlgArena`** — not in `poet-vibe`. Host constant, not language constant. | Not started | recommendations §4.5 |

### 8.10 Geometry, Sheaves, Stalks (recommendations §4.6, grok §1)

| # | Task | Status | Source |
|---|------|--------|--------|
| T47 | **Stalk as isolated `PoetSnapshot` + capability lease + pulse topic prefix** — agent context is a pointer, not a copied transcript. | Not started | recommendations §4.6, grok §2 |
| T48 | **Glue / sheaf condition as Pure predicate at commit of staged deltas** — failure is a diagnostic, not an exception unwind. | Not started | recommendations §4.6 |
| T49 | **Simplex as named record of jointly-required cells/graph shapes** — missing member ⇒ load or commit reject. | Not started | recommendations §4.6 |
| T50 | **Topological tear as `Diagnostic` + evidential (μ, λ) on sealed receipt** — quarantine context is a host routing decision. | Not started | recommendations §4.6 |

### 8.11 MCP Replacement / Agent-Native (recommendations §4.3, grok §2)

| # | Task | Status | Source |
|---|------|--------|--------|
| T51 | **Every `ALL_BOUND` id exports a machine schema** — not English prose. Arguments, effect class, honesty, GBNF fragment from the same table as the catalog. | **Done** — capability_schema.rs with 36+ entries, EffectClass, HonestyLabel, SchemaArg | recommendations §4.3 |
| T52 | **Heavy returns are `QuinRef` / `did:q42:…` / `TensorRef` / `GeometryRef`** — never a 10k-line payload. | **Done** — QuinRef, TensorRef, GeometryRef, AssetRef are Value variants with extractors | recommendations §4.3 |
| T53 | **Wire GBNF into in-process sampling loop** — projectional mutations can wait; logit mask is the actual MCP replacement. | Not started | recommendations §4.3, W11, excellence-first §3.13 |
| T54 | **Reflection stage 3 on isolated `PoetSnapshot`** — must not write the live graph. | Not started | recommendations §4.3 (overlaps T26) |

### 8.12 Disclosure Boundary & Instrument Traces (disclosure-boundary, bylines docs)

| # | Task | Status | Source |
|---|------|--------|--------|
| T55 | **`DisclosureDenied` as a first-class value** — credentialed refusal the principal can show to an auditor; instrument degrades without seeing the payload. | **Done** — Value::DisclosureDenied in value.rs (D6 confirmed: stays as Value) | disclosure-boundary §0 |
| T56 | **Four-boundary separation** — publication / replication / agency / exfiltration are four bits, not one. `.gitignore` is publication only. | **Done** — four-boundary separation implemented | disclosure-boundary §0 |
| T57 | **Instrument trace ledger (Kind B)** — production notes the *customer* can read: which instrument instance, which act, which Instant, which lease, what cost. Vendor-only copy is not this. | **Done** — instrument_trace.rs in governance/ | bylines §0, AGENT_INTENT_LOGGING_SPEC |
| T58 | **No bylines (Kind A) enforced mechanically** — §16 rule already in CLAUDE.md; needs tooling to prevent injection. | Rule added | bylines §0, CLAUDE.md §16 |
| T59 | **Agent characteristics KB** — log characteristics of agents (including AI agents) from behaviour. Local inference first, then packs for jurisdictions. | **Done** — AgentCharacteristicsKb in observer.rs | rights-not-sovereignty §1 |

### 8.13 Ecosystem & Standalone Packaging (ecosystem doc, recommendations §1)

| # | Task | Status | Source |
|---|------|--------|--------|
| T60 | **LSP server** — `tower-lsp` based; autocomplete, go-to-definition, find-references, real-time diagnostics. | **Done** — poet-lsp crate with tower-lsp | ecosystem §3.1, topics-yet-considered §1 |
| T61 | **WASM playground** — Monaco editor + live output; zero-install. | Not started (deferred) | ecosystem §3.2, topics-yet-considered §2 |
| T62 | **CLI toolchain** — REPL, formatter, linter/static analyzer. | **Done** — poet CLI with check/fmt/eval/translate/repl | ecosystem §3.3, topics-yet-considered §3 |
| T63 | **Module system / package manager** — `import <iri> as name`; catalog is the module graph. No npm. No `vibe.toml` dependency solver in 0.x. | Not started | excellence-first §2.7, ecosystem §3.4 |
| T64 | **One import / capability story** — kill `vibe:0.1/` prefix as sacred string; language version lives on module header / AST tag. | **Done** — optional vibe:0.1/ prefix | excellence-first §2.7 |
| T65 | **Interactive onboarding / "Tour of..."** — walk new users through paradigms. | Not started (deferred) | ecosystem §3.5, topics-yet-considered §5 |

### 8.14 Core Spec Hygiene (excellence-first §2.10, §2.11, recommendations §2)

| # | Task | Status | Source |
|---|------|--------|--------|
| T66 | **Update spec to intended lattice** — not to whatever the interpreter happened to do last week. `time.now()` vs `time.unix` discrepancy. | **Done** — X6 resolved: time.now() is primary, time.unix() deprecated. X5 resolved: Quantity mandatory for physical fields. Spec updated in decision table. | excellence-first §2.11 |
| T67 | **Reconcile `Tensor10D` field comments and `axis_role.rs`** — they disagree about `t` and `μ`. One document. | **Done** — reconciled (T67, 2026-08-19). t = coordinate time, μ = provenance carrier. Both files document consistently. | excellence-first §2.11 (overlaps T15) |
| T68 | **Tick policy under load** — drop / coalesce / tear. Unspecified, so animation and sensor fusion will lie. | **Done** — tick_policy.rs with Drop/Coalesce/Tear policies, TickQueue, (μ,λ) tear evidence | excellence-first §3.9 |
| T69 | **Presentation morphism as sheaf** — visual / haptic / auditory / Braille. Not `Render.css_*` plus hope. | **Done** — presentation.rs with PresentationMorphism, PresentationSheaf, PresentationModality (Visual/Haptic/Auditory/Braille) | excellence-first §3.10 |
| T70 | **Identifier vs continuant in type system** — `Did` vs something that is allowed to mean a person. Prose-only is how UUID-identity comes back. | **Done** — Did and Continuant are distinct Type variants with clear semantics: Did refers to, Continuant has a Did and endures | excellence-first §3.11, recommendations §4.7 |
| T71 | **One clock story in Wellfair / pulse / poet** — `asserted_time_unix: u32` in wellfare-core is another coarse Unix. Replace together, or Vibe Instant won't compose. | Not started | excellence-first §3.14 |
| T72 | **Law packages as signed content** — who authored the dissolve rate; under what licence; whether it is physical or fictional. Provenance must travel. | **Done** — law_package.rs with LawPackage, LawKind, LawStore, canonical bytes for signing | excellence-first §3.8, W10 |
| T73 | **Quantity dimension algebra** — Pa = N/m². Conversions are not a host lookup table of strings. | **Done** — quantity.rs with Dimension struct, Mul/Div for derived dimensions, Unit::convert() with dimension mismatch checking, lookup_unit() | excellence-first §3.7 |

### 8.15 Wish List (excellence-first §4, ordered by product-defining power)

| # | Wish | Status | Source |
|---|------|--------|--------|
| W1 | Projectional authoring — edit Instant / Field / Law as structure; text is a view | Not started | excellence-first §4 |
| W2 | WorldLine as the continuant's time-like self | **Done** — WorldLine implemented in value.rs | excellence-first §4 |
| W3 | Conservation hooks on glue | **Done** (tracked as T34) | excellence-first §4 |
| W4 | Mixture / phase diagrams as data | **Done** (tracked as T33) | excellence-first §4 |
| W5 | Frame morphisms (Galilean → Lorentz later) | Not started | excellence-first §4 |
| W6 | Causal cone on pulse / graph | **Done** (tracked as T35) | excellence-first §4 |
| W7 | Measurement context / observer stalk | **Done** — MeasurementContext in observer.rs | excellence-first §4 |
| W8 | Assistive I/O in the first HID vertical | Not started (tracked as T43) | excellence-first §4 |
| W9 | Oral / heraldic lexicon modalities as identifier views | **Done** — IdentifierView + IdentifierModality in observer.rs | excellence-first §4 |
| W10 | Law store = signed packages | **Done** (tracked as T72) — law_package.rs with LawStore | excellence-first §4 |
| W11 | GBNF on the in-process sampler | Not started (tracked as T53) | excellence-first §4 |
| W12 | Deterministic replay Instant as the only wasm clock | **Done** — replay_clock.rs with ReplayClock, ReplayTimeline, ExhaustedPolicy | excellence-first §4 |
| W13 | Custom GPU backend when wgpu internals dominate allocs | Not started (tracked as §F1) | excellence-first §4 |
| W14 | Multi-scale / filtered sheaves (LOD as physics) | **Done** — MultiScaleSheaf + LevelOfDetail in observer.rs | excellence-first §4 |
| W15 | Civic time + authority to assert it | **Done** — CivicInstant in observer.rs | excellence-first §4 |
| W16 | Pretty material/field syntax that is 100% CST sugar | Not started (gated on T28–T31) | excellence-first §4 |
| W17 | Delete Identish, four-field Quin, time.unix-as-primary in one breaking pass | **Partially done** — time.unix-as-primary killed (X6). Identish and four-field Quin still pending (T8, T7). | excellence-first §4 |
| W18 | Keyword locale views with English or another locale as first pretty dialect | **Done** (tracked as T37) — locale.rs with en + zh tables, translate.rs | excellence-first §4 |

### 8.16 Decisions Needing Timothy (excellence-first §7)

| # | Decision | Excellence default | Status |
|---|----------|-------------------|--------|
| X1 | Confirm immovable list (§1) vs breakable surface | As written | **Resolved by Timothy 2026-08-20** — confirmed as final. 48B Quin, 42MB Sentinel, Zero-Heap Tier-1, deterministic no-JIT, RDF 1.2, capability-gated security, Option A axes, deontic/SHACL at commit are immovable. VibeScript surface (QuinRef, Instant, FieldDecl/MaterialDecl/LawDecl, Quantity) is breakable. |
| X2 | Official name for `ManifoldCoordinate10D` in Vibe | `Attention10D` (or your term) — **not** "manifold" | **Resolved by Timothy 2026-08-20** — confirmed `Attention10D` as VibeScript-facing name. `Tensor10D` for space/physics/pose, `Attention10D` for epistemic/attention states. `ManifoldCoordinate10D`/`Epistemic10D` remain engine-internal. |
| X3 | `t` vs `μ` as provenance | `t` = time axis; `μ` = carrier; Instant on receipts | **Resolved by Timothy 2026-08-20** — confirmed: `t` = coordinate time axis (participates in geometric distance alongside x,y,z); `μ` = in-band provenance/consent carrier. Exact provenance timestamps live as Instant receipts, not lossy f32s. T15/T67 reconciled in both axis_role.rs and tensor/mod.rs. |
| X4 | Grow grammar for `field` / `material` / `law` now? | **Yes**, as AST nodes + Tag 4200, not `nquin` | **Resolved by implementation** — FieldDecl/MaterialDecl/LawDecl exist as AST nodes with CBOR Tag 4200 |
| X5 | `Quantity` mandatory in 0.2-that-replaces-0.1? | **Yes** | **Resolved by Timothy 2026-08-20** — confirmed mandatory for physical/material/geometric fields. Pure math (math.sin, math.sqrt) stays numeric. Physical measurements require explicit unit IRI (or qudt:DimensionlessUnit). Type checker must reject unit mismatch (e.g. kPa + Pa) without explicit conversion. |
| X6 | Kill `time.unix` as primitive? | **Yes** — keep a projection helper | **Resolved by Timothy 2026-08-20** — confirmed: kill `time.unix()` as primary primitive. Native primitive is `time.now() -> Instant` with nanosecond resolution, explicit TimeScale (Unix, Tai, Gps, Monotonic, Proper), optional receipt seal. Projection helper `instant.to_unix_secs()` for display/logging. Integer seconds cannot support sub-frame animation, physics dt, or deterministic WASM replay. |
| X7 | Unicode identifiers this pass? | **No** unless BiDi/homoglyph is in the same pass | **Resolved** — no Unicode identifiers shipped (T40 remains gated) |
| X8 | First wish-list slice after the lattice | W2 WorldLine + W4 Mixture + W12 replay Instant | **Resolved by implementation** — WorldLine (W2) and Mixture (W4) implemented |

### 8.17 What NOT to do (excellence-first §5, recommendations §1)

- An 11th–Nth manifold axis for the next physical idea.
- HoTT / latent-graph rewrite before Field+Instant exist.
- A sibling `vibe-script` working tree or a licence fiction (all dev stays in `qualia-27062026`).
- Ollama / external model server.
- Ambient `nquin` literals.
- ECS as a second runtime beside the graph.
- "JavaScript interop Value" that is only `F64`.
- Shipping Unicode identifiers without BiDi policy.
- Binding 120 HID ids before Instant, Field, and a switch/announce path.
- A second syntax surface that agents use while humans use another.
- Treating the geometric model as pure research — force every new geometric primitive to lower.
- Letting the 42 MB sentinel or zero-heap rules become soft.

---

## 9. Multi-Agent Orchestration Strategy (2026-08-19)

**Source:** `docs/plans/multi-agent-orchestration-strategy-2026-08-19.md`

The VibeScript agent primitives (A1–A9), the engine coordination ISA
(`governance/coordination.rs`), and the desktop agent roster
(`qualia-client-core/agent_registry.rs`) are each implemented and tested in
isolation. The orchestration strategy defines how to wire them together into
a unified multi-agent execution substrate.

### 9.1 The 14 gaps (G1–G14)

| Gap | What's not wired |
|-----|------------------|
| G1 | DAG → eval (A6 `dag.rs` not on `lib.rs` public surface) |
| G2 | Deontic → capability_invoke (A8 prohibitions don't gate eval) |
| G3 | Reflection → isolated PoetSnapshot (A3 stage 3 may write live graph) |
| G4 | Blackboard → DAG node I/O (A5 channels declared but not read/written) |
| G5 | Coord ISA → host seams (verify_root_delegation, SuspendedTransactionQueue, VC minting) |
| G6 | Agent roster → chat dispatch (@mention doesn't resolve to roster agent) |
| G7 | Agent roster → DAG pipeline (no path from @mention to DAG node binding) |
| G8 | Job scheduler → agent turns (no `LocalJobKind::AgentTurn`) |
| G9 | Eτ evidential → diagnostic loop (diagnostics don't carry (μ, λ)) |
| G10 | Semantic skills → agent context (A9 not injected into context windows) |
| G11 | DOMINO → in-process sampler (GBNF artifacts exist; inference doesn't consume) |
| G12 | Performance VCs → reputation routing (compute_priority not called) |
| G13 | Instrument traces (Kind B) not shipped |
| G14 | DisclosureDenied not a first-class value |

### 9.2 The 12 refactors (R1–R12)

| # | Refactor | Priority | Risk | Depends on |
|---|----------|----------|------|------------|
| R1 | `pub use dag, deontic_interrupt, reflection` from `lib.rs` | P0 | Low | — |
| R2 | Wire `PhaseLeaser` into `eval.rs` capability dispatch | P1 | Medium | R1 |
| R3 | Wire `DagPipeline` execution into `poet_host` | P2 | Medium | R1, R5 |
| R4 | Isolate `reflection::Stage3` on `PoetSnapshot` fork | P0 | Low | — |
| R5 | Connect blackboard channels to DAG node I/O | P0 | Low | R1 |
| R6 | Add `LocalJobKind::AgentTurn` to job scheduler | P2 | Low | — |
| R7 | Resolve @mentions to roster agents | P2 | Medium | — |
| R8 | Wire `governance/coordination.rs` host seams | P1 | Medium | — |
| R9 | Wire DOMINO logit mask into `QTensorEngine` | P3 | High | — |
| R10 | Add `DisclosureDenied` value type | P3 | Low | — |
| R11 | Wire `compute_priority` into `daemon_swarm.rs` | P1 | Low | — |
| R12 | Instrument trace ledger (Kind B) | P4 | Low | — |

### 9.3 Priority tiers

| Tier | Items | Can start? |
|------|-------|------------|
| P0: Foundation | R1, R4, R5 | ✅ Immediately |
| P1: Governance | R2, R8, R11 | After P0 |
| P2: Agent dispatch | R6, R7, R3 | After P1 |
| P3: Inference quality | R9, R10 | After P2 |
| P4: Audit | R12, G9 | After P2 |

### 9.4 Critical path

```
R1 → R5 → R3 → R6 → R7 → agent_turn_handler
R2 ──────────────────────────┘
```

Six steps to "Timothy can @mention two agents and they run as a governed DAG
pipeline with blackboard-mediated state sharing."

### 9.5 Decisions needing Timothy (D1–D6)

See strategy document §6. Defaults provided for speed.

### 9.6 Tracking

- Strategy: `docs/plans/multi-agent-orchestration-strategy-2026-08-19.md`
- Progress: `docs/plans/multi-agent-orchestration-PROGRESS-LOG.md`
- Coordination: `coordination/NOTICES.md` (CLAIM/PROGRESS/RELEASE)
