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
| C: EMF interference/Doppler/attenuation | Not started | — | — | — |
| D: vibeAnimation namespace + CSS/SVG | Not started | — | — | — |
| E: Reactive animation loop | Not started | — | — | — |
| F: Graph honesty lift | **Done** | 2026-08-18 | 2026-08-18 | +3 (dynamic honesty: graph.read live when attached, pulse.publish live when attached, capability.invoke stays partial) |
| G: Golden corpus expansion | Not started | — | — | — |
| H: vibe-bc-0.1 bytecode | Not started (post-0.1) | — | — | — |
