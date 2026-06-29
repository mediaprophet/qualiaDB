# LLM-on-Forge — Progress Log

Honest engineering record for wiring the LLM forward pass onto the DAG-IR forge, off the native
q42/p64 substrate, with q42 provenance + the existing Phase-8 governance intact.

- **Plan:** [`docs/plans/llm-on-forge-q42-p64.md`](docs/plans/llm-on-forge-q42-p64.md)
- **Handover:** [`docs/HANDOVER-llm-on-forge.md`](docs/HANDOVER-llm-on-forge.md)
- **Branch:** `feature/p64-manifold-wal-eigensolver` · **Hardware:** NVIDIA RTX A2000 (8 GB)

Measurement-honesty rule applies: real numbers or "not measured"; never extrapolate a kernel
figure to end-to-end tok/s.

---

## 2026-06-29 — Phase 1a: device unification (DONE)

**Step / phase:** Phase 1a (device unification) — the keystone prerequisite for weight residency
and the p64 bake-off. **Status: done, GPU-certified on A2000.**

**What was built** (files: `crates/qualia-core-db/src/wgsl_forge/execute/wgpu.rs`,
`crates/qualia-core-db/src/wgsl_forge/graph_ops/executor.rs`):

- `WgpuComputeContext::from_device(device, queue, caps, capacity_bytes)` — builds a forge context
  on an **already-existing** `wgpu::Device` + `Queue` (cheap `Arc` clones) instead of requesting a
  *second* adapter/device the way `new()` does. Adapter identity / constraints / hardware profile
  are reconstructed from the **live** `device.limits()` + `device.features()` plus the caller's
  `GpuAdapterCaps` snapshot (the original `wgpu::Adapter` is consumed at shared-gpu init and not
  retained). The existing `new()` path (own device, used by standalone forge tests) is untouched.
- `ForgeGraphExecutor::with_context(ctx)` + `ForgeGraphExecutor::on_shared_gpu()` /
  `on_shared_gpu_with_capacity()` — construct an executor around a caller-supplied context, and a
  convenience that builds one on the process-wide `gpu_context::shared_gpu()` device. This is the
  entry point by which the forge runs on the **same** device that owns the resident LLM weights +
  KV cache, rather than a separate device. `with_capacity()` now routes through `with_context()`.
- A GPU cert `shared_device_executor_matches_cpu_oracle` (`#[ignore]`, native) that builds the
  executor via `on_shared_gpu()`, asserts its reported adapter `vendor`/`device` equal the shared
  device's (i.e. **no second adapter was created**), and runs softmax (1024-wide) + a full faithful
  decode block (RMSNorm·eps → scaled attention → residual → SwiGLU-FFN → residual) against the
  composed CPU oracle within f32 tolerance.

**Mechanism in one sentence:** the forge can now adopt the LLM's own GPU device instead of spinning
up its own, so subsequent phases (resident weights, p64 bake-off, decode-loop swap) can share one
device + one set of weight buffers.

**Measured results** (A2000, release `--ignored`, `--test-threads=1`):

- `shared_device_executor_matches_cpu_oracle` — **PASS** (forge on `shared_gpu()` matches CPU oracle
  for softmax + faithful decode block; adapter vendor/device equal the shared device's).
- Existing certs still green: `execute_graph_gpu_matches_cpu_oracle`, `p4b_graphs_gpu_match_cpu_oracle`,
  `pipeline_cache_amortizes_across_runs` — all PASS.
- `decode_block_kernel_uplift_bench` (own-device path, unchanged by 1a): d=576/kv=128/ffn=1536,
  30 nodes, 12 cached pipelines → GPU reused **12.573 ms/call** (~0.419 ms/node) vs CPU oracle
  56.918 ms/call = **4.53× CPU**. One block, NOT end-to-end tok/s, NOT ×L layers.
- Non-GPU `wgsl_forge::` sweep: **134 passed / 0 failed / 45 ignored**. Lib builds clean plain and
  `--features cuda` (both EXIT 0).

**Honest boundary:** `from_device` inherits the host device's negotiated features + limits verbatim.
`shared_gpu()` today does **not** raise the ray-tracing acceleration-structure limits, so RT-core
Neighbor cannot create BLAS/TLAS on a shared-device forge context even though `supports_rt_cores`
reports true. `from_device` deliberately does **not** silently widen the host device. The decode
path (matmul / elementwise / reduce / softmax) needs none of that, so 1a is unaffected; RT-on-shared
is a later coordination point with the LLM lane (it would mean `shared_gpu` raising those limits).

**⚑ Where I need the human:** none this step. (The bake-off acceptable-quality threshold for the
AWQ-ternary FFN — Phase 3 — will be a curation call, flagged when we reach it.)

**Next step:** Phase 1b — weight residency. Add a persistent weight region + `load_weights` to the
executor so the big matrices upload **once** (referenced by offset across tokens) and `run()` takes
activation-only externals; then the p64→forge bridge (`P64TensorIndex::from_p64` → role-tagged
graph externals) and the real-layer builders (QKV, output proj, multi-head, RoPE) for the
one-real-layer bake-off vs the hand-written `dispatch_attention_pass`/`dispatch_ffn_pass`.

---

## 2026-06-29 — Phase 1b: weight residency (DONE)

**Step / phase:** Phase 1b (weight residency) — eliminate the per-call weight re-upload (the #1
named perf gap in the forge-seam map: `run()` re-uploaded **every** external each call, so a decode
block re-shipped its constant projection/FFN matrices over PCIe per token). **Status: done,
GPU-certified on A2000.**

**What was built** (files: `crates/qualia-core-db/src/wgsl_forge/execute/memory.rs`,
`crates/qualia-core-db/src/wgsl_forge/execute/wgpu.rs`,
`crates/qualia-core-db/src/wgsl_forge/graph_ops/executor.rs`):

- A persistent weight region distinct from the recycling transient ring. New
  `BindingUsage::StorageReadResident`; a third device buffer `weight_slab` + a write-once,
  256-aligned bump cursor on `WgpuComputeContext`; `allocate_weight` (upload-once into `weight_slab`,
  returns a resident `BufferView`), `clear_weights`, `resident_weight_bytes`. A resident view is
  bound exactly like a read-only storage input — `slab_for` just routes it to `weight_slab` — so it
  flows through the existing per-node `at(view, slot)` re-bind machinery unchanged, and it **survives**
  `clear_transient_allocations` (which only resets the ring).
- `ForgeGraphExecutor::load_weights(&[(ext_index, data)]) -> ResidentWeights` (upload a graph's
  weight externals once), and `run_resident(graph, externals, &ResidentWeights)` — identical to
  `run` except external indices present in the handle bind their **resident** device buffer and are
  **not** re-uploaded; only the activation externals are written that call. `run` and `run_resident`
  now share one `run_prepared` core, so residency changes nothing about node scheduling.

**Mechanism in one sentence:** the big constant matrices are uploaded to a separate on-device buffer
once via `load_weights`, and each `run_resident` call references them by offset while uploading only
the per-token activations — so per-token PCIe traffic drops by the weight size.

**Measured results** (A2000, release `--ignored`, `--test-threads=1`):

- `resident_weights_decode_block` — **PASS**. Decode block d=576/kv=128/ffn=1536, FFN matrices
  (Wg,Wu,Wd = **10,616,832 B** ≈ 10.6 MB) resident: **resident 9.605 ms/call vs all-upload 11.977
  ms/call** (~1.25×), with 10.6 MB/call no longer re-uploaded. **Correctness: `run_resident` ==
  `run` (all-upload) bit-for-bit, AND matches the composed CPU oracle, on each of 3 successive
  calls** — proving the resident weights persist across the per-call ring reset and change nothing
  numerically.
- All existing executor GPU certs still green: `execute_graph_gpu_matches_cpu_oracle`,
  `p4b_graphs_gpu_match_cpu_oracle`, `pipeline_cache_amortizes_across_runs`,
  `shared_device_executor_matches_cpu_oracle`, `decode_block_kernel_uplift_bench` (4.56× CPU).
- Non-GPU `wgsl_forge::` sweep: **134 passed / 0 failed / 46 ignored**. Lib builds clean plain and
  `--features cuda` (both EXIT 0).

**Honest boundary:** the ~2.4 ms/call saving here is the PCIe write of 10.6 MB on this discrete
A2000; the win **scales with weight size** (a real layer's Q/K/V/O + FFN ≈ 40 MB f32 → ~4× the saved
upload) and would differ on unified memory. This is not yet end-to-end tok/s, and the decode block
still uploads Kt/V each call (those are the KV cache, which legitimately changes per token) — in the
full forward the attention projection weights become resident too, so residency matters more there.
`weight_slab` capacity currently equals the ring capacity (64 MiB); holding **all** layers resident
(Phase 2, multi-layer) will need a larger region sized to the model, not per-layer reuse.

**⚑ Where I need the human:** none this step.

**Next step:** the **p64 → forge bridge** + real-layer builders, then the **one-real-layer bake-off**.
Read a real model's weights via `P64TensorIndex::from_p64` (role-tagged: `P64_ROLE_ATTN_Q/K/V/O`,
`FFN_GATE/UP/DOWN`, norms), `load_weights` them into the resident region, build the real decode-layer
graph (QKV projection, output projection, multi-head attention, RoPE), and bake it off — same p64
weights — against the hand-written `dispatch_attention_pass`/`dispatch_ffn_pass`, asserting f32-tol
match (the working engine is the oracle) and reporting ms/layer for both. "No lanes" (Timothy,
2026-06-29): the decode-loop seam in `inference_agent.rs` + `gguf_bridge` is in scope for that wiring.

---

## 2026-06-29 — Real RoPE (placeholder removed), directed by Timothy ("no placeholders")

**Step / phase:** remove the RoPE placeholder before building the real decode layer. **Status: done,
GPU-certified on A2000.**

**What was the placeholder:** `wgsl_forge/graph_ops/stencil.rs::RopePair` was a **fixed-angle**
rotation (`c = cos(1), s = sin(1)`), self-described as a "content-free structural rotation" — it
applied the *same* unit rotation to every pair regardless of position or dimension. Not RoPE.

**What was built** (file: `crates/qualia-core-db/src/wgsl_forge/graph_ops/stencil.rs`): a **real**
rotary position embedding. `RopeConfig { head_dim, pos, mode, theta_base }`, `RopeMode::{Interleaved,
Neox}` (GGUF `NORM` = adjacent pairs `(2j,2j+1)`; GGUF `NEOX`/HF `rotate_half` = split pairs
`(j, j+head_dim/2)`); each pair rotated by the true angle `θ = pos · base^(−2j/head_dim)`. Real WGSL
kernel (`rope_wgsl`, reads `[n, head_dim, pos, mode, theta_bits]` params, default base 10000), exact
f32 CPU oracle (`rope_cpu`), standalone GPU runner (`rope_gpu`). The generic `stencil_{cpu,gpu}` now
take a `RopeConfig` and route `RopePair` through the real RoPE; `Laplacian` unchanged.

**Measured / verified** (A2000):
- 7 non-GPU stencil/RoPE tests pass, incl. **relative-position invariance** — RoPE's *defining*
  property, that `dot(RoPE(q,m), RoPE(k,n))` depends only on `m−n`, verified in **both** conventions
  (the score is unchanged when both positions shift by the same Δ); plus single-pair hand-check,
  pos-0-is-identity, per-pair norm preservation, bad-head_dim rejection, naga validation.
- GPU cert `stencil_gpu_matches_oracle` — **PASS**: real RoPE (both conventions, head_dim 64, pos 5,
  multi-head 512-wide) + Laplacian match the CPU oracle on hardware.
- Full non-GPU `wgsl_forge::` 138 passed / 0 failed / 46 ignored; lib clean plain + `--features cuda`.

**Other placeholders scanned (Timothy: "any others you come across"):**
- `emit/{hlsl,msl}.rs` "Simplified for now" vectorized-affine path — **examined: not a fake.** It is a
  correct unrolled fast-path + bounds-checked tail, and `affine-f32` (gated by `kernel.id`) is the only
  kernel that sets `vector_width>1`, whose sole op is `out = in·scale + bias`. Native float4 SIMD loads
  would be a throughput optimization needing DXC/Metal to validate (absent on this Windows host), not a
  correctness gap. Corrected the misleading comments; left the (correct) codegen.
- `fluid_dynamics` (`velocity*=0.99`) + `Divergence`/`Advection` stencils + `quantum_bio` — these
  return honest `Err`s / are flagged as **curation item #45**: they need Timothy's physical-model
  direction (which discretization / scheme), the one allowed defer per §12. Not faked, not silently
  no-op. Left for his call.
- `execute/memory.rs` unified zero-copy "not yet implemented" — honest scope note; the uniform copy
  path is correct on both topologies, and the unified-only optimization cannot be verified on this
  discrete-only A2000 (shipping it unverified would over-claim). Left as documented.
