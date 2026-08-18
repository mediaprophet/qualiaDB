# VibeScript Full Implementation — Handover Document

**Last updated:** 2026-08-18
**Session:** Continuation from `history_1024fd011df14a79.md` + `history_f9d49814253f4e3e.md`
**Repository:** `C:\Projects\qualia-27062026` (canonical — never use worktrees or sibling repos)
**Plan file:** `docs/vibescript-full-impl-PLAN.md`

---

## 1. What this workstream is about

Fully implementing VibeScript (the `vibe-0.1` DSL interpreted by the Poet Engine) so that:
- Every engine capability is wrapped as a Vibe `capability.invoke` ID
- The 0.1 binding profile is truthfully "live" wherever the daemon graph supports it
- Domain coverage includes physics (EMF, wave, interference, Doppler, attenuation), spectral/color, geometry/SVG, CSS animation output
- Reactive cells can drive visual output — a cell computes field values from graph-stated parameters, and the output generates CSS/SVG animation properties that truthfully reflect the underlying physics
- The golden corpus covers all domain verticals

The user (Timothy Charles Holborn) specifically wants:
> "full coverage fully supported"
>
> "how it might be used to help script animating CSS output, and perhaps also SVG and/or geometric shapes"
>
> "qualiadb uses EMF, and when presenting various types of frequency, there's sometimes a shift in values due to such things as interference or distance"

This means the EMF physics → visual output pipeline is a priority, not just capability wrapping.

---

## 2. What's done (committed)

### Commits on `0.0.31-dev` branch

| Commit | Description |
|---|---|
| `0c8fdd8b` | Initial doc audit + core work (earlier session) |
| `49e8c8f0` | Pulse transport lift: `pulse.publish` through broadcast channel, `/pulse/events` SSE, `PulseRecord` with payload_summary + seq |
| `f41075eb` | Hook dispatch + user-defined function resolution: `Engine::with_program`, `dispatch_hook` API, `poet_dispatch_hook` desktop command |
| `938ce191` | Docs: update readiness with hook dispatch + pulse transport + test counts |

### Implemented and tested

| Area | Status | Test count |
|---|---|---|
| Language core (lexer, parser, checker, AST interpreter) | Complete | 25 poet-vibe tests |
| 0.1 binding profile (math, rdf, quin, graph, aura, pulse, capability, time) | Complete | All §12/§13 fixtures pass |
| Hook dispatch (`on pulse:message`, `on tick`) | Complete | 4 hook dispatch tests |
| User-defined function resolution in eval_call | Complete | §12.2 CLINIC module works end-to-end |
| Pulse transport (broadcast channel + SSE `/pulse/events`) | Complete | 5 pulse tests |
| 27 `capability.invoke` wrappers (math, crypto, stats, graph, geometry, etc.) | Complete | 89 poet_host tests |
| WASM compilation (`wasm32-unknown-unknown`) | Clean | `cargo check` passes |
| Desktop harness (eval, recompute, cells, gazetteer, capabilities, dispatch_hook) | Complete | `cargo check` passes |
| N3Logic/Prolog supplementary audit | Complete | Written to `C:\Projects\NLP\consult\20260818_n3logic-prolog-supplementary-audit.md` (887 lines) |

### 0.1 conformance bar (§15 of `vibescript-core.md`)

All 6 requirements met:
1. Every §12 example parses, type-checks, and evaluates on native-desktop and wasm32
2. Every §13 fixture is rejected with a stable diagnostic code
3. No public API writes Quin parity
4. Pure cells cannot reach Pulse, graph write, or time
5. `graph.query` without `take` is a type/effect error
6. Tests live in `poet-vibe` and don't require a GPU

---

## 3. What's next — the plan

The full plan is in `docs/vibescript-full-impl-PLAN.md`. Summary:

| Phase | Description | Effort | Dependencies |
|---|---|---|---|
| A | Physics `capability.invoke` wrappers (wave, heat, advection, oscillator, pendulum, N-body, MD, CFD, quantum, population) | Medium | None — wraps existing solvers |
| B | EMF/spectral `capability.invoke` wrappers (emf_to_spd, spd_to_xyz, emf_to_rgb, blend, gamut_map) | Medium | None — wraps existing spectral kernel |
| C | New EMF physics: interference, Doppler shift, attenuation, 2D field grid | Medium-high | A |
| D | CSS/SVG output bindings (css_animation, svg_path, svg_field, css_color) | Medium | B, C |
| E | Reactive animation loop (poet_tick, poet_pulse_event, pulse transport → hook dispatch) | Medium | D |
| F | Graph honesty lift (graph.read/graph.write from "partial" to "live") | Low | None — independent |
| G | Golden corpus expansion (physics, EMF, geometry, CSS, legal, scientific, financial) | High aggregate | All others |
| H | vibe-bc-0.1 bytecode (v1.0 destination) | High | Post-0.1 |

**Recommended start: Phases A and B in parallel** — they wrap existing tested code, so they're low-risk and high-impact.

---

## 4. Key files

### Core language crate (`poet-vibe`)

| File | Purpose |
|---|---|
| `crates/poet-vibe/src/lib.rs` | Public API: `eval_cell`, `eval_function`, `dispatch_hook`, `load_program` |
| `crates/poet-vibe/src/eval.rs` | AST interpreter. `Engine` holds optional program ref for user-defined function resolution |
| `crates/poet-vibe/src/bind/mod.rs` | `Host` trait + `dispatch` function. All 0.1 bindings routed here |
| `crates/poet-vibe/src/check.rs` | Type/effect checker. Enforces `graph.query` requires `take`, capability gating, etc. |
| `crates/poet-vibe/tests/conformance.rs` | 22 conformance tests (§12/§13 fixtures + hook dispatch) |

### Host crate (`qualia-core-db`)

| File | Purpose |
|---|---|
| `crates/qualia-core-db/src/poet_host/mod.rs` | `PoetSnapshot` — the live host. `pulse_publish`, `graph_query`, `graph_commit`, `dispatch_hook_src`, `aura_validate` |
| `crates/qualia-core-db/src/poet_host/invoke/ids.rs` | All `capability.invoke` ID constants + `ALL_BOUND` + `seam_for` |
| `crates/qualia-core-db/src/poet_host/invoke/mod.rs` | `dispatch()` function — routes invoke IDs to handler functions |
| `crates/qualia-core-db/src/poet_host/invoke/coverage.rs` | WASM suggested invoke coverage tracking |
| `crates/qualia-core-db/src/poet_host/invoke/science/physics.rs` | Currently only `PhysicsAndODE.projectile` — **Phase A expands this to 10 functions** |
| `crates/qualia-core-db/src/poet_host/invoke/render/scene.rs` | `Render.scene` — builds node/edge/face records |
| `crates/qualia-core-db/src/poet_host/catalog.rs` | `VIBE_0_1` binding table with honesty labels |
| `crates/qualia-core-db/src/services/pulse_transport.rs` | Process-wide broadcast channel for pulse events |
| `crates/qualia-core-db/src/services/daemon_graph.rs` | Live graph store with revision tracking + broadcast |

### Engine infrastructure to wrap (Phases A, B, C)

| File | What it has |
|---|---|
| `crates/qualia-core-db/src/render/spectral_kernel.rs` | `emf_to_spd(alpha, mu, sigma)`, `spd_to_xyz`, `emf_to_linear_rgb` |
| `crates/qualia-core-db/src/render/spectral_oracle.rs` | Golden vectors, CPU/GPU differential, determinism harness |
| `crates/qualia-core-db/src/render/spectral_blend.rs` | Spectral blending, metamerism |
| `crates/qualia-core-db/src/render/spectral_operator.rs` | Spectral pipeline operations |
| `crates/qualia-core-db/src/render/gpu_colour_kernel.rs` | EMF → display gamut mapping |
| `crates/qualia-core-db/src/specialized_libs/physics_simulation/fields.rs` | `run_wave_equation_1d`, `run_heat_diffusion_1d`, `run_advection_diffusion_1d` |
| `crates/qualia-core-db/src/specialized_libs/physics_simulation/mechanics.rs` | `run_projectile_motion`, `run_harmonic_oscillator`, `run_pendulum` |
| `crates/qualia-core-db/src/specialized_libs/physics_simulation/nbody.rs` | `run_nbody_gravitation` |
| `crates/qualia-core-db/src/specialized_libs/physics_simulation/molecular_dynamics.rs` | `run_molecular_dynamics` (Lennard-Jones, velocity-Verlet) |
| `crates/qualia-core-db/src/specialized_libs/physics_simulation/cfd.rs` | `run_cfd_simulation`, `solve_cfd_step` |
| `crates/qualia-core-db/src/specialized_libs/physics_simulation/quantum.rs` | `run_quantum_stationary_states_1d` |
| `crates/qualia-core-db/src/specialized_libs/physics_simulation/population.rs` | `run_logistic_growth` |

### Desktop crate (`webizen-desktop`)

| File | Purpose |
|---|---|
| `crates/webizen-desktop/src/commands/poet.rs` | Tauri commands: `poet_eval`, `poet_reset`, `poet_recompute`, `poet_cells`, `poet_dispatch_hook`, `poet_gazetteer`, `poet_capabilities` |
| `crates/webizen-desktop/src/commands/mod.rs` | `get_invoke_handler()` — registers all Tauri commands |

### Documentation

| File | Purpose |
|---|---|
| `docs/manuals/standards/vibescript-core.md` | Normative 0.1 spec (§1-§16) |
| `docs/manuals/standards/vibescript-specification.md` | Architectural spec (marked stale/aspirational) |
| `docs/manuals/ai-agent-vibescript-readiness.md` | Agent readiness overlay (pillars, binding table, completeness verdict) |
| `docs/vibescript-full-impl-PLAN.md` | This workstream's implementation plan + phase tracker |

---

## 5. Open questions for Timothy (need answers before proceeding)

These are in section 4 of the plan file. Timothy has not yet answered them:

1. **CSS/SVG output namespace:** `capability.invoke("Render.css_animation", …)` (0.1-compliant) vs a new `render.*` first-class namespace (post-0.1 grammar extension)?

2. **Graph honesty labels:** Dynamic (changes based on attached/detached state) or static with documentation?

3. **Golden corpus priorities:** Which domain verticals first? (Physics/EMF is implied by his interest in CSS/SVG animation from EMF fields, but he said "full coverage fully supported")

4. **EMF interference scope:** 2D only (for CSS/SVG) or also 3D (for the existing render scene infrastructure)?

5. **Animation loop timing:** `requestAnimationFrame` (60fps), `setInterval` (configurable), or both? Pausable?

6. **Phase ordering:** Does he agree with A→B→C→D→E→F→G, or different priority?

**Timothy's stated preferences so far:**
- "full coverage fully supported" — implies all phases, not cherry-picking
- EMF → CSS/SVG animation is a specific interest, not just generic capability wrapping
- He understands the physics (interference, distance, frequency shift) — so the implementation should be physically truthful, not decorative

---

## 6. Verification commands

Run these after each phase:

```
cargo test -p poet-vibe
cargo test -p qualia-core-db --lib poet_host
cargo check -p webizen-desktop
cargo check -p poet-vibe --target wasm32-unknown-unknown
```

Current baseline (as of commit `938ce191`):
- `poet-vibe`: 25 passed, 0 failed
- `poet_host`: 89 passed, 0 failed
- `webizen-desktop`: clean
- `wasm32`: clean

---

## 7. Project rules to follow

From `CLAUDE.md` and `AGENTS.md`:

1. **Canonical repo only:** `C:\Projects\qualia-27062026`. No worktrees, no sibling repos.
2. **No Ollama/llama.cpp/Python:** Qualia has its own native in-process LLM inference stack (`gguf_bridge.rs` + `wgpu`).
3. **No `Vec`/`String`/`Box` in hot paths:** Zero-copy ABI for WASM/desktop/edge.
4. **48-byte `NQuin`** for all semantic data. 42 MB `SlgArena` ceiling.
5. **Big files → library with sub-directory:** If a file is heading past ~400-500 lines, split it into `foo/mod.rs` + `foo/<concern>.rs`.
6. **Fully implement:** No `// TODO`, no `◑ partial` left in place of real work. The acceptance test is: an independent reviewer asks "is this complete?" and answers yes.
7. **Modernize dependency APIs:** If touching stale dependency code, update to current API + capabilities.
8. **Per-step progress logging:** Append dated entries to a progress-log `.md` at the end of every step. (Note: `docs/plans/` is gitignored — use `docs/` root or another non-ignored path.)
9. **Multi-agent coordination:** Check `coordination/NOTICES.md` before writing code. Append CLAIM/RELEASE notices.
10. **Privacy:** Personal circumstances stay local, never committed.
11. **Authorship:** All work assigned to Timothy Charles Holborn. No third-party/fictional/self authorship.

---

## 8. How to start the next session

1. Read this file: `docs/vibescript-full-impl-HANDOVER.md`
2. Read the plan: `docs/vibescript-full-impl-PLAN.md`
3. Check `coordination/NOTICES.md` for collisions
4. If Timothy has answered the open questions in §5, proceed with the indicated phases
5. If not, ask him the open questions before starting
6. Start with Phases A and B (physics + spectral wrappers) — they're independent and can run in parallel
7. After each phase: run verification (§6), update the plan's phase tracker, update `ai-agent-vibescript-readiness.md`, commit, append to `coordination/NOTICES.md`

---

## 9. Architecture notes for the implementer

### How `capability.invoke` wrappers work

The pattern (followed by all 27 existing wrappers):

1. Add an ID constant to `crates/qualia-core-db/src/poet_host/invoke/ids.rs`:
   ```rust
   pub const PHYS_WAVE_1D: &str = "Physics.wave_1d";
   ```

2. Add it to `ALL_BOUND` and `seam_for` in the same file.

3. Implement the wrapper in the appropriate submodule (e.g. `invoke/science/physics.rs`):
   ```rust
   pub fn wave_1d(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
       let u0 = args::rec_f64_list(args_v, "u0").unwrap_or_default();
       // ... marshal args, call the existing solver, shape the result ...
       Ok(args::record([("energy_initial", Value::F64(r.energy_initial)), ...]))
   }
   ```

4. Wire the dispatch arm in `invoke/mod.rs`:
   ```rust
   ids::PHYS_WAVE_1D => science::physics::wave_1d(&args, span),
   ```

5. Update `coverage.rs` `wasm_suggested_invoke` if the function should be WASM-accessible.

6. Add a test in the same submodule.

### How the EMF → color pipeline works

The existing pipeline in `render/spectral_kernel.rs`:
1. `emf_to_spd(alpha, mu, sigma)` — EMF parameters (amplitude α, spectral center μ, bandwidth σ) → spectral power distribution (SPD) over 380-780nm
2. `spd_to_xyz(spd)` — SPD → CIE XYZ tristimulus values (using CIE 1931 color matching functions)
3. `emf_to_linear_rgb(alpha, mu, sigma)` — full pipeline → linear sRGB

The spectral oracle has golden vectors with known EMF → XYZ mappings for determinism testing.

### What "interference" means physically

When N EMF sources emit at the same point in space:
- Each source contributes a wave: `A_i * sin(2π * f_i * t - k_i * r_i + φ_i)`
- Where `r_i` is the distance from source i to the observation point
- The resultant field is the superposition: `Σ A_i * sin(2π * f_i * t - k_i * r_i + φ_i)`
- If all sources have the same frequency, this produces a standing interference pattern (constructive/destructive)
- If sources have different frequencies, the resultant has beat frequencies
- Distance affects both amplitude (inverse-square attenuation) and phase (propagation delay)

This is what Timothy means by "shift in values due to interference or distance" — the observed frequency/amplitude at a point is not the source frequency/amplitude, but the superposition result.

### How reactive cells work

- `poet_eval` with `as_cell=true` registers a cell in `PoetHarnessState::cells`
- The cell tracks `graph_read_during_eval` — if true, the cell is graph-dependent
- `poet_recompute` re-evaluates cells whose `graph_revision_at_eval` < current revision
- Phase E extends this to time-dependent cells (recompute on tick)

### How hook dispatch works

- `dispatch_hook(program, path, args, host, env)` finds the first `on <path>(…)` hook in the program
- The `Engine` holds an optional program reference (`*const Program`) so `eval_call` can resolve user-defined functions
- `call_function` and `call_hook` set `self.program` before evaluating the body
- The desktop `poet_dispatch_hook` command takes `source`, `path` (Vec<String>), and `args_json` (JSON array)
