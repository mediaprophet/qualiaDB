# qualia-extensions — Production-Excellence Pass

**Directed by Timothy 2026-06-25.** The `crates/qualia-extensions` crate is the heap/heavy-compute counterpart
to the zero-alloc core (it explicitly may use `std`, GPU, external APIs). It was **out of the original
`qualia-core-db` audit scope** (boundary doc §G). This is its own production-excellence pass: de-mock to real,
tested implementations, module by module, to the same completeness bar (no mock-as-dodge).

Heap is EXPECTED here (this crate is the heavy-compute lane) — the zero-heap invariant applies to the core, not
to extensions. The bar is **correctness + real implementation + tests**, not zero-allocation.

## Initial assessment (2026-06-25)

| Module | LOC | State found | Plan |
|--------|-----|-------------|------|
| `lib.rs` | 252 | `todo!("Implement FFI bridge")` (core↔ext FFI unimplemented) | ✅ **DONE** — real C-ABI marshalling + free fn + tests. |
| `snn_extension.rs` | 955 | mock synaptic input / mock weights / mock spike extraction / mock CRDT sync | ✅ **DONE** — real LIF + STDP + CRDT merge. |
| `pinn_extension.rs` | 1135 | `mock_neural_forward` (fake Navier-Stokes/heat/Lorenz), "placeholder" | ✅ **DONE** — real ternary-MLP forward + real PDE residual. |
| `webgpu_extension.rs` | 710 | "Mock WebGPU execution"; mock velocity fields | ✅ **DONE** — 6 real solvers (split into a module dir), analytic-validated. |
| `qpu_extension.rs` | 472 | mock IBM/Google/Braket calls | **DEFERRED-TO-LAST** per §0.11 + boundary §H (QPU design directive). NOT touched in this pass. |

## Progress

### `snn_extension.rs` — DE-MOCKED (2026-06-25)

The SNN simulation was a mock that produced **empty output** (synaptic input was a fake constant 0.1,
`extract_output_spikes` returned `vec![]`, refractory used wall-clock `Instant` against sim-time — a bug).
Replaced with real event-driven dynamics + real CRDT merge:

- **Real LIF integrate-and-fire** (`step_neurons`): synapse-weighted input from the pre-neurons that fired the
  previous step, signed by excitatory/inhibitory neuron type; `exp(−dt/τ_m)` membrane leak; threshold spike +
  reset; **simulation-time** refractory (fixes the Instant-vs-Duration bug).
- **Real synaptic current** from `network.synapses` (weights × pre-fired), replacing the fake 0.1 constant.
- **Real STDP** plasticity: pre-before-post potentiation, post-without-pre depression (clamped).
- **Real spike output** (`group_into_spike_trains`): the simulation now produces actual per-neuron spike trains.
- **Real CRDT merge** (`perform_sync`): version-vector advance + genuine value-conflict detection across source
  nodes, resolving by higher-confidence (noisy) value; metrics derived from real data, not hard-coded.
  `resolve_conflict_with_noise`: highest-confidence pick with a noise tie-break.
- Removed the two dead mock methods (`calculate_synaptic_input`, `extract_output_spikes`).
- Tests: real LIF fires on synaptic drive + STDP potentiates; leak/grouping helpers. (+ existing tests.)

### `pinn_extension.rs` — DE-MOCKED (2026-06-25)

`mock_neural_forward` was hardcoded formulas that **never touched the model's real `ternary_weights`**, and
`calculate_residual` computed arbitrary algebra unrelated to any PDE. Replaced with a genuine PINN:

- **Real ternary-MLP forward** (`ternary_forward`): each `TernaryTensor` is a layer (W = ternary_data ×
  scaling_factor over `shape=[out,in]`), `tanh` hidden + linear final. `pinn_forward` runs it when the model
  has trained weights.
- **Real analytic references** (`physics_reference`) for untrained models — exact/standard solutions, NOT
  mocks: heat → fundamental (Gaussian) solution of `u_t=u_xx`; chaos → Lorenz state by RK4 (`lorenz_state_at`);
  fluid → Taylor–Green vortex (an exact incompressible Navier–Stokes solution).
- **Real physics-informed residual** (`pde_residual`): the actual PDE operator (heat `u_t−αu_xx`, Lorenz
  `‖dX/dt−f(X)‖`, NS continuity `u_x+v_y`) applied to `pinn_forward` by central finite differences.
- Rewired both the default and the native (cfg `pinn`) paths; removed the dead mocks. The native LLM/GGUF path
  honestly falls back to the analytic reference (not a mock) until it is wired.
- Tests: real MLP forward (W·x), heat reference is unit at origin, Lorenz advances by RK4, and the headline —
  the heat fundamental solution yields a **near-zero PDE residual** (proving the residual is real PDE math).

### `webgpu_extension.rs` — DE-MOCKED + LIBRARY-IZED (2026-06-25)

The extension was a mock: `execute_webgpu_computation` returned analytic fake fields that ignored the inputs
and the time-stepping; the perf metrics were invented constants (`tflops 1.5`, `gpu_utilization 85%`); the
3D Maxwell shader's `compute_curl_e` returned `vec3(0)`; and four of the six advertised operations
(`compute_heat_transfer`, `propagate_waves`, `simulate_particles`, `tensor_operations`) were unwired
(`OperationNotSupported`). Replaced with **six real solvers**, each validated against an exact analytic
solution, and split into a module directory (`webgpu_extension/{mod,shaders,fluid,electromagnetics,heat,wave,particles,tensor}.rs`)
per the big-file rule (CLAUDE.md §10):

- **fluid** — incompressible 2D Navier–Stokes by **Chorin projection** (advect+diffuse → pressure-Poisson
  Jacobi → project). Reproduces the **Taylor–Green vortex** `exp(-2νt)` decay to ~0.1%; the projected field is
  divergence-free; the inviscid vortex is conserved.
- **electromagnetics** — **1D Yee FDTD** (`Ey`/`Hz` staggered leapfrog, fixed Maxwell sign convention). An
  impedance-matched pulse propagates at exactly `c`; lossless energy stays bounded; a conductive medium
  dissipates energy.
- **heat** — 2D explicit diffusion. The `sin·sin` Laplacian eigenmode decays at `exp(-2αt)`.
- **wave** — 2D leapfrog wave equation. The standing wave inverts at `T/2` (strong anti-correlation with IC)
  and is CFL-stable over long runs.
- **particles** — gravitational **N-body, velocity-Verlet** (symplectic). Total energy conserved <1%; a
  two-body orbit stays bounded.
- **tensor** — exact dense GEMM. Verified products incl. rectangular shapes.

Perf metrics are now **measured** (time / FLOP-rate / bandwidth); `gpu_utilization` is honestly `0` on the CPU
reference path. The WGSL is kept as the **GPU kernel spec** (corrected: true `dx`/`h²`, completed `curl`); the
CPU solvers are the verifiable reference (matching the core engine's CPU-reference pattern), and GPU dispatch
routes through the core's shared `wgpu` device rather than a second device here. 18 solver tests + the full
suite green.

### `lib.rs` FFI bridge — IMPLEMENTED (2026-06-25)

`extension_manager_execute_job` was `todo!("Implement FFI bridge")`. Implemented the full C-ABI marshalling:
UTF-8 job fields + JSON `parameters` → `ExtensionJob`, run on a process-wide Tokio runtime, return the
JSON-serialised `ExtensionResult` (success) or an error message. Added `extension_result_free` to release the
heap buffers and made the `CExtensionJob`/`CExtensionResult` fields `pub`. Null pointers, bad UTF-8, unknown
extensions and malformed JSON all **fail closed** with an error message instead of panicking. 4 FFI tests
(real round-trip via webgpu, unknown extension, null manager, bad JSON).

## Status

**qualia-extensions de-mock COMPLETE** except `qpu_extension.rs` (deferred-to-last per §0.11 + QPU design
directive, boundary §H). Whole-crate suite: **39/39 lib tests green**; bins build. Commits: snn `575046aa9`,
pinn `f839a7ba9`, webgpu `5558e5dd3`, FFI `3851c163b`.


