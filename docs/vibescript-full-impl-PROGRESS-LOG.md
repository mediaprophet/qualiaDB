# VibeScript Full Implementation — Progress Log

**Workstream:** Full VibeScript implementation (Phases A–H)
**Plan:** `docs/vibescript-full-impl-PLAN.md`
**Handover:** `docs/vibescript-full-impl-HANDOVER.md`
**Branch:** `0.0.31-dev`

---

## 2026-08-18 — Phases A + B + F (done)

**Status:** Done. All three phases implemented, tested, and verified.

### What was built

**Phase A — Physics `capability.invoke` wrappers (10 functions):**
- `Physics.wave_1d` — 1D scalar wave equation (Dirichlet ends, Dopri5 integrator)
- `Physics.heat_diffusion_1d` — heat diffusion (Neumann/insulated ends)
- `Physics.advection_diffusion_1d` — coupled advection-diffusion (periodic grid)
- `Physics.harmonic_oscillator` — spring–mass (symplectic Yoshida4)
- `Physics.pendulum` — nonlinear pendulum (Dopri5)
- `Physics.n_body` — Newtonian N-body gravitation (2D direct sum)
- `Physics.molecular_dynamics` — 2D Lennard-Jones (velocity-Verlet)
- `Physics.cfd_step` — Burgers steady-state residual
- `Physics.quantum_states_1d` — TISE Schrödinger eigenproblem (Jacobi)
- `Physics.logistic_growth` — population dynamics

All wrap existing tested solvers in `specialized_libs/physics_simulation/`. No physics re-derived — numerics delegate to `integrate_dopri5`, `integrate_symplectic`, `symmetric_eigen`.

Files touched:
- `crates/qualia-core-db/src/poet_host/invoke/science/physics.rs` — expanded from 1 to 11 functions (563 lines — flagged for library-ization pass)
- `crates/qualia-core-db/src/poet_host/invoke/science/stubs.rs` — new WASM-ontology fallbacks
- `crates/qualia-core-db/src/poet_host/invoke/science/mod.rs` — re-exports + stub wiring
- `crates/qualia-core-db/src/poet_host/invoke/args.rs` — added `rec_f64_list` helper
- `crates/qualia-core-db/src/poet_host/invoke/ids.rs` — 15 new ID constants + `ALL_BOUND` + `seam_for`
- `crates/qualia-core-db/src/poet_host/invoke/mod.rs` — 15 new dispatch arms

**Phase B — EMF/spectral `capability.invoke` wrappers (5 functions):**
- `Spectral.emf_to_spd` — EMF `[α, μ, σ]` → 41-sample SPD (380–780 nm)
- `Spectral.spd_to_xyz` — SPD → CIE XYZ tristimulus
- `Spectral.emf_to_rgb` — EMF → linear sRGB + 8-bit display sRGB + CSS string
- `Spectral.blend` — spectral-space blend of two EMF payloads → XYZ
- `Spectral.gamut_map` — map XYZ into sRGB display gamut

All wrap `render::spectral_kernel` and `render::spectral_blend` (deterministic, compile-time CIE 1931 CMF tables).

Files touched:
- `crates/qualia-core-db/src/poet_host/invoke/render/spectral.rs` — new (204 lines)
- `crates/qualia-core-db/src/poet_host/invoke/render/mod.rs` — added `pub mod spectral`

**Phase F — Dynamic graph honesty labels:**
- `catalog::resolve_id_with(id, attached)` — `graph.read`, `graph.write`, `aura.validate`, `pulse.publish` flip to "live" when attached to the daemon graph; stay "partial" on WASM/detached.
- `catalog::dynamic_honesty(id, attached)` — public function for the effective label.
- `capability.invoke` and `time.unix` stay "partial" regardless (not all invoke IDs are bound; WASM time is limited).
- `poet_host/mod.rs` `capability_resolve` now passes `self.attached`.

Files touched:
- `crates/qualia-core-db/src/poet_host/catalog.rs` — `resolve_id_with`, `dynamic_honesty`, updated tests
- `crates/qualia-core-db/src/poet_host/mod.rs` — caller updated

### Measured results

| Verification | Result |
|---|---|
| `cargo test -p poet-vibe` (lib) | 22 passed, 0 failed |
| `cargo test -p poet-vibe --test conformance` | 22 passed, 0 failed |
| `cargo test -p qualia-core-db --lib poet_host` | **107 passed**, 0 failed (was 89; +18 new tests) |
| `cargo check -p webizen-desktop` | clean |
| `cargo check -p poet-vibe --target wasm32-unknown-unknown` | clean |

New test breakdown: 10 physics + 5 spectral + 3 catalog (dynamic honesty) = 18.

### Caveats

- `physics.rs` is 563 lines — over the §11 400–500 line threshold. Flagged for a library-ization pass. All functions are the same cohesive pattern (physics wrapper marshalling), so the split would be by solver category (fields, mechanics, nbody, etc.). Deferred per §11 refinement: "Do not block a full implementation on a split."
- `cfd_step` computes the Burgers residual directly rather than constructing the unused `CfdSolver`/`Mesh` params that `PhysicsSolver::solve_cfd_step` ignores. The computation is identical (L2 norm of `ν·u_xx − u·u_x`). Noted in the function doc comment.
- The 10D manifold integration for EMF fields is Phase C (not yet started). The physics wrappers currently return flat `Vec<f64>` snapshots; Phase C will add manifold-coordinate-aware field grids.

### ⚑ Where I need the human

None this step. Phase C (3D/4D EMF + 10D manifold) and Phase D (`vibeAnimation` grammar extension) are next — both are well-defined engineering tasks that don't require out-of-band decisions.

### Architecture constraint (Timothy, 2026-08-18)

**Physics functions must work without QPU access.** Only a few expressly-declared exceptions may require QPU access, and QPU access is expected to be extremely specialised and extremely limited in the vast majority of circumstances.

The existing codebase already follows this pattern:
- `qpu_bridge` / `qpu_oracle` — remote quantum hardware, fail-closed, opt-in, requires Human Rights commitment
- MCP `qpu_optimize` / `qpu_dft` / `qpu_status` — gated by `qpu_enabled` flag, fail-closed when not enabled
- `fallback_to_classical: true` by default in the QPU Oracle

All VibeScript physics wrappers are **classical simulations** — they run on CPU/GPU, no QPU required:
- `Physics.quantum_states_1d` — solves the Schrödinger equation via finite differences + classical Jacobi eigensolver. This is classical simulation of quantum mechanics, NOT quantum hardware.
- All Phase A wrappers (wave, heat, oscillator, pendulum, N-body, MD, CFD, logistic) — classical ODE/PDE solvers.
- All Phase B wrappers (EMF → spectral → color) — classical spectral projection.
- All Phase C wrappers (EMF interference, Doppler, attenuation) — classical EM superposition.

If a future function genuinely requires QPU access (e.g., quantum annealing for NP-hard optimization, real quantum DFT), it must:
1. Be expressly declared as QPU-required in its doc comment
2. Fail-closed when no QPU is available (return a diagnostic, not a panic)
3. Follow the existing `qpu_enabled` gating pattern

### Next step

**Phase C — EMF interference, Doppler shift, attenuation (5D: XYZ + depth + time):**

**Design decisions settled with Timothy (2026-08-18, before OOM):**
- The field grid is **5D sampled**: X, Y, Z (position), Depth (distance from camera/observer — independent sampling parameter, drives perspective scaling, LOD, display attenuation), Time (as a time-frame with start/end, not just a single instant — needed for Phase E animation).
- **Two-layer separation:**
  - **Physics layer** (`Physics.emf_field_grid_3d`): 4D grid (x×y×z×t) of EMF values (amplitude, phase, frequency). Pure physics — interference superposition, inverse-square attenuation from sources, Doppler shift. No rendering concerns.
  - **Render-depth layer** (`Physics.emf_sample_at_depth` or similar): samples the 4D physics field at specified depths from camera/observer, applying perspective scaling, display attenuation, LOD selection. Bridges physics to `vibeAnimation` output (Phase D).
- **10D manifold integration:** Each field sample is tagged with a `ManifoldCoordinate10D` derived from the physics (amplitude→scale, frequency→recurrence_frequency, phase→spatial_phase, depth→attention_depth, time→temporal_decay, etc.). The manifold captures the *semantic* state of the field at that point.
- **All classical — no QPU required.** EM superposition, inverse-square, Doppler are all classical physics.

**Functions to implement:**
- `Physics.emf_interference` — superposition of N EMF sources at a 3D observation point
- `Physics.emf_attenuation` — inverse-square law + atmospheric absorption
- `Physics.doppler_shift` — relativistic Doppler
- `Physics.emf_field_grid_3d` — 4D physics grid (x×y×z×t) with `ManifoldCoordinate10D` tags
- `Physics.emf_sample_at_depth` — depth-aware sampling for render integration
- New `specialized_libs/physics_simulation/emf.rs` submodule (the actual physics)
- New `poet_host/invoke/science/emf.rs` (the invoke wrappers)

Then **Phase D — `vibeAnimation` namespace** (grammar extension with all three surface forms: hierarchical sub-forms, single dispatch, and `capability.invoke` for the long tail; SVG wired through computational geometry libs).

Then **Phase E — reactive animation loop** (comprehensive timing: rAF + setInterval + pausable + configurable).

### OOM note (2026-08-18)

Phase C was claimed in NOTICES but the session OOM'd before any code was written. No files were touched. Claim released. Handover written to `docs/vibescript-full-impl-HANDOVER-2026-08-18-session2.md`.
