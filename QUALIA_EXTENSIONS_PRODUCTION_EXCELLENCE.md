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
| `lib.rs` | 252 | `todo!("Implement FFI bridge")` (core↔ext FFI unimplemented) | Implement the FFI marshalling. |
| `snn_extension.rs` | 955 | mock synaptic input / mock weights / mock spike extraction / mock CRDT sync | Real LIF neuron dynamics + STDP + real sync semantics. |
| `pinn_extension.rs` | 1135 | `mock_neural_forward` (fake Navier-Stokes/heat/Lorenz), "placeholder" | Real numerical physics forward + honest PINN residual; honest scope on training. |
| `webgpu_extension.rs` | 710 | "Mock WebGPU execution"; mock velocity fields | Real CPU physics (and/or real wgpu where verifiable); honest GPU boundary. |
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

