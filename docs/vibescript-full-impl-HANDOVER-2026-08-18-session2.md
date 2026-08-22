# VibeScript Full Implementation — Handover (Session 2, 2026-08-18)

**Reason for handover:** Session OOM'd during Phase C design discussion, before any Phase C code was written.
**Previous handover:** `docs/vibescript-full-impl-HANDOVER.md` (covers sessions up to Phase A+B+F)
**Progress log:** `docs/vibescript-full-impl-PROGRESS-LOG.md` (updated through this session)
**Plan:** `docs/vibescript-full-impl-PLAN.md` (phase tracker updated)
**Branch:** `0.0.31-dev`

---

## 1. What's committed (clean state)

| Commit | Description |
|---|---|
| `a6cda55f` | docs: clarify quantum_states_1d is classical simulation, no QPU required |
| `c6cef8a1` | feat(poet-host): physics + spectral invoke wrappers, dynamic graph honesty |
| `28b111c1` | docs: add VibeScript full implementation plan + handover document |

**Working tree is clean** — no uncommitted changes to tracked files. No partial Phase C files exist. The OOM happened before any Phase C code was written.

---

## 2. What's done (Phases A, B, F)

### Phase A — Physics capability.invoke wrappers (10 functions)
All wrap existing tested solvers in `specialized_libs/physics_simulation/`. No physics re-derived.

- `Physics.wave_1d` — 1D scalar wave equation (Dirichlet ends, Dopri5)
- `Physics.heat_diffusion_1d` — heat diffusion (Neumann/insulated ends)
- `Physics.advection_diffusion_1d` — coupled advection-diffusion (periodic grid)
- `Physics.harmonic_oscillator` — spring–mass (symplectic Yoshida4)
- `Physics.pendulum` — nonlinear pendulum (Dopri5)
- `Physics.n_body` — Newtonian N-body gravitation (2D direct sum)
- `Physics.molecular_dynamics` — 2D Lennard-Jones (velocity-Verlet)
- `Physics.cfd_step` — Burgers steady-state residual
- `Physics.quantum_states_1d` — TISE Schrödinger eigenproblem (Jacobi) — **classical simulation, no QPU**
- `Physics.logistic_growth` — population dynamics

### Phase B — EMF/spectral capability.invoke wrappers (5 functions)
All wrap `render::spectral_kernel` and `render::spectral_blend` (deterministic, CIE 1931 CMF tables).

- `Spectral.emf_to_spd` — EMF `[α, μ, σ]` → 41-sample SPD (380–780 nm)
- `Spectral.spd_to_xyz` — SPD → CIE XYZ tristimulus
- `Spectral.emf_to_rgb` — EMF → linear sRGB + 8-bit display sRGB + CSS string
- `Spectral.blend` — spectral-space blend of two EMF payloads → XYZ
- `Spectral.gamut_map` — map XYZ into sRGB display gamut

### Phase F — Dynamic graph honesty labels
- `catalog::resolve_id_with(id, attached)` — `graph.read`, `graph.write`, `aura.validate`, `pulse.publish` flip to "live" when attached to the daemon graph
- `catalog::dynamic_honesty(id, attached)` — public function for the effective label
- `capability.invoke` and `time.unix` stay "partial" regardless

### Verification baseline (as of `a6cda55f`)
- `vibe` lib: 22 passed, 0 failed
- `vibe` conformance: 22 passed, 0 failed
- `poet_host`: **107 passed**, 0 failed (was 89; +18 new tests)
- `webizen-desktop`: clean
- `wasm32`: clean

---

## 3. What's next — Phase C (NOT started, design settled)

Phase C is the EMF physics Timothy specifically cares about — "shift in values due to interference or distance." This is **new physics code** (not wrappers), so it's the highest-effort phase.

### Design decisions settled with Timothy (2026-08-18)

**Field grid dimensionality — 5D sampled:**
- X, Y, Z — position in space
- Depth — distance from camera/observer (independent sampling parameter, drives perspective scaling, LOD, display attenuation — like in 3D games where depth scales the asset)
- Time — as a time-frame with start/end (not just a single instant — needed for Phase E animation)

**Two-layer separation:**
1. **Physics layer** (`Physics.emf_field_grid_3d`): 4D grid (x×y×z×t) of EMF values (amplitude, phase, frequency). Pure physics — interference superposition, inverse-square attenuation from sources, Doppler shift. No rendering concerns.
2. **Render-depth layer** (`Physics.emf_sample_at_depth`): samples the 4D physics field at specified depths from camera/observer, applying perspective scaling, display attenuation, LOD selection. Bridges physics to `vibeAnimation` output (Phase D).

**10D manifold integration:** Each field sample is tagged with a `ManifoldCoordinate10D` derived from the physics:
- amplitude → `scale`
- frequency → `recurrence_frequency`
- phase → `spatial_phase`
- depth → `attention_depth`
- time → `temporal_decay`
- (others mapped as appropriate)

The manifold captures the *semantic* state of the field at that point, not just raw numbers. See `crates/qualia-core-db/src/modalities/manifold.rs` for the `ManifoldCoordinate10D` struct (10 f32 dims, encoded as 2× 48-byte NQuins).

**All classical — no QPU required.** EM superposition, inverse-square, Doppler are all classical physics.

### Functions to implement

| Invoke ID | Description |
|---|---|
| `Physics.emf_interference` | Superposition of N EMF sources at a 3D observation point. Each source contributes `A_i * sin(2π * f_i * t - k_i * r_i + φ_i)`. Resultant = Σ. Same frequency → standing interference pattern; different frequencies → beat frequencies. |
| `Physics.emf_attenuation` | Inverse-square law + atmospheric absorption. Given source power, frequency, distance, medium properties → received signal strength. |
| `Physics.doppler_shift` | Relativistic Doppler. Given source frequency, relative velocity, geometry → observed frequency. |
| `Physics.emf_field_grid_3d` | 4D physics grid (x×y×z×t) with `ManifoldCoordinate10D` tags. The core visualization feed. |
| `Physics.emf_sample_at_depth` | Depth-aware sampling for render integration. Samples the 4D field at specified depths, applies perspective scaling + display attenuation + LOD selection. |

### Files to create/modify

| File | Action |
|---|---|
| `crates/qualia-core-db/src/specialized_libs/physics_simulation/emf.rs` | **New** — the actual EMF physics (interference, attenuation, Doppler, field grid) |
| `crates/qualia-core-db/src/specialized_libs/physics_simulation/mod.rs` | Add `mod emf;` + re-exports |
| `crates/qualia-core-db/src/poet_host/invoke/science/emf.rs` | **New** — the invoke wrappers (marshal Value → call physics → shape result) |
| `crates/qualia-core-db/src/poet_host/invoke/science/mod.rs` | Add `mod emf;` + re-exports + WASM stubs |
| `crates/qualia-core-db/src/poet_host/invoke/science/stubs.rs` | Add EMF stubs |
| `crates/qualia-core-db/src/poet_host/invoke/ids.rs` | Add 5 new ID constants + `ALL_BOUND` + `seam_for` |
| `crates/qualia-core-db/src/poet_host/invoke/mod.rs` | Add 5 dispatch arms |

### Tests (analytical verification)
- Two-source interference: known constructive/destructive points
- Inverse-square: amplitude at 2× distance = ¼ amplitude
- Doppler: known ratios (e.g., source approaching at 0.1c → frequency × √(1.1/0.9))
- Field grid: finite values, monotonic attenuation with distance

### Key files to read before starting Phase C

| File | Why |
|---|---|
| `crates/qualia-core-db/src/modalities/manifold.rs` | `ManifoldCoordinate10D` struct (10 f32 dims), `ManifoldState10D`, encode/decode to NQuins |
| `crates/qualia-core-db/src/specialized_libs/physics_simulation/quantum.rs` | Pattern for a physics submodule (impl block on `PhysicsSimulationLibrary`) |
| `crates/qualia-core-db/src/specialized_libs/physics_simulation/results.rs` | Result struct patterns (fields, Debug+Clone derives) |
| `crates/qualia-core-db/src/poet_host/invoke/science/physics.rs` | Pattern for invoke wrappers (marshal args → call solver → shape Value) |
| `crates/qualia-core-db/src/poet_host/invoke/args.rs` | `rec_f64`, `rec_f64_list`, `rec_u64`, `args::record`, `args::f64_list_value` helpers |
| `crates/qualia-core-db/src/render/spectral_kernel.rs` | `emf_to_spd`, `emf_to_linear_rgb` — the spectral pipeline Phase C feeds into |
| `crates/qualia-core-db/src/specialized_libs/computational_geometry/mod.rs` | Geometry lib exports (bezier, bspline, nurbs, etc.) — needed for Phase D SVG integration |

---

## 4. Architecture constraints (from Timothy)

### QPU constraint (2026-08-18)
**Physics functions must work without QPU access.** Only a few expressly-declared exceptions may require QPU access, and QPU access is expected to be extremely specialised and extremely limited in the vast majority of circumstances.

All VibeScript physics wrappers are **classical simulations** — they run on CPU/GPU, no QPU required. If a future function genuinely requires QPU access, it must:
1. Be expressly declared as QPU-required in its doc comment
2. Fail-closed when no QPU is available (return a diagnostic, not a panic)
3. Follow the existing `qpu_enabled` gating pattern

### Timothy's answers to the 6 open questions (2026-08-18)
1. **CSS/SVG output namespace:** `vibeAnimation` — new first-class namespace (post-0.1 grammar extension). Support ALL three surface forms: hierarchical sub-forms (`vibeAnimation.css/svg/field/curve`), single dispatch (`vibeAnimation(kind, args)`), AND `capability.invoke` for the long tail.
2. **Graph honesty labels:** Dynamic (implemented in Phase F).
3. **Golden corpus priorities:** Dependency-driven, incremental.
4. **EMF interference scope:** 5D — XYZ + depth + time. Depth = distance from camera/observer (render-aware). See §3 above.
5. **Animation loop timing:** Comprehensive, best-in-class — rAF + setInterval + pausable + configurable rate.
6. **Phase ordering:** A+B → F → C → D → E → G → H (approved).

### Additional notes from Timothy
- SVG animation should hook into the computational geometry libraries (bezier, bspline, nurbs, offset_polyline, tube_along_polyline, etc.)
- Progress file must be maintained for resumability by different sessions

---

## 5. Phase order (remaining)

```
C (EMF interference/Doppler/attenuation — 5D field grid + 10D manifold)  ← NEXT
    │
    ▼
D (vibeAnimation namespace — grammar extension + CSS/SVG + computational geometry integration)
    │
    ▼
E (reactive animation loop — comprehensive timing: rAF + setInterval + pausable + configurable)
    │
    ▼
G (golden corpus expansion — all domain verticals)
    │
    ▼
H (vibe-bc-0.1 bytecode — post-0.1)
```

---

## 6. How to start the next session

1. Read this file: `docs/vibescript-full-impl-HANDOVER-2026-08-18-session2.md`
2. Read the progress log: `docs/vibescript-full-impl-PROGRESS-LOG.md`
3. Read the plan: `docs/vibescript-full-impl-PLAN.md`
4. Check `coordination/NOTICES.md` for collisions (Phase C claim was released)
5. Read the key files in §3 above before implementing Phase C
6. Claim in NOTICES, implement Phase C, test, verify, commit, append to progress log
7. Verification commands:
   ```
   cargo test -p vibe
   cargo test -p qualia-core-db --lib poet_host
   cargo check -p webizen-desktop
   cargo check -p vibe --target wasm32-unknown-unknown
   ```

---

## 7. Project rules to follow

From `CLAUDE.md` and `AGENTS.md`:
1. **Canonical repo only:** `C:\Projects\qualia-27062026`. No worktrees.
2. **No Ollama/llama.cpp/Python:** Qualia has its own native in-process LLM inference stack.
3. **No `Vec`/`String`/`Box` in hot paths:** Zero-copy ABI for WASM/desktop/edge.
4. **48-byte `NQuin`** for all semantic data. 42 MB `SlgArena` ceiling.
5. **Big files → library with sub-directory:** If a file heads past ~400-500 lines, split it. (But don't block implementation on a split — flag and move on.)
6. **Fully implement:** No `// TODO`, no `◑ partial` left in place of real work.
7. **Modernize dependency APIs:** If touching stale dependency code, update to current API.
8. **Per-step progress logging:** Append dated entries to `docs/vibescript-full-impl-PROGRESS-LOG.md` at the end of every step.
9. **Multi-agent coordination:** Check `coordination/NOTICES.md` before writing code. Append CLAIM/RELEASE notices.
10. **No QPU by default:** Physics functions must work without QPU access (see §4).
11. **Authorship:** All work assigned to Timothy Charles Holborn.
