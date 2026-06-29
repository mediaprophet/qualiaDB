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

---

## 2026-06-29 — Real multi-head (GQA) decode layer on the forge (DONE)

**Step / phase:** Phase 1c (real-layer builders) — a genuine multi-head decode layer composed from
forge ops, the prerequisite for the p64 bake-off. **Status: done, GPU-certified on A2000.**

**What was built:**
- Two real new IR ops (graph.rs, q42_bridge.rs opcodes `0x1B`/`0x1C`, `lower_graph` + `Lowerer` trait
  arms, executor GPU + CPU-oracle arms):
  - **`Slice { offset, len }`** (`graph_ops/slice.rs`) — extract a contiguous sub-range on-device.
    The projected q/K/V are node outputs, so per-head slicing must run on the GPU, not the host.
  - **`Rope { head_dim, pos, mode, base_bits }`** — first-class RoPE op reusing the real
    `stencil::rope_wgsl`/`rope_cpu`. `pos` rides in the params **buffer**, not the kernel source, so
    the pipeline cache stays warm across tokens (only the buffer changes per position).
- **`decode_layer_graph(n_heads, n_kv_heads, head_dim, seq, ffn, pos, rope_mode, theta_base)`**
  (executor.rs): `RMSNorm(x) → Q-proj → RoPE(q) → per-head { slice q_h / Kᵀ_h / V_h ; scaled
  softmax(q_h·Kᵀ_h)·V_h ; o_h·Wo_h } summed → +x → RMSNorm → SwiGLU-FFN → +`. Real **grouped-query
  attention** (each kv-head serves `n_heads/n_kv_heads` query heads); the per-head output projection
  is summed (`Σ_h o_h·Wo_h`), so no concat op is needed. Head-major K/V cache layout makes every
  per-head slice contiguous.

**Measured / verified** (A2000):
- `decode_layer_cpu_oracle_matches_reference` — **PASS**, the strong proof: the graph's composed CPU
  oracle matches an **independent hand-written reference** (real RMSNorm/RoPE/attention/SwiGLU math,
  not the graph code) in **both** RoPE conventions with GQA 2:1. This proves the graph computes a
  *real* decode layer, not merely that the GPU matches its own oracle.
- `decode_layer_gpu_matches_cpu_oracle` — **PASS**: the layer runs on the A2000 and matches the
  composed oracle, both conventions, at a realistic shape (4 heads, 2 kv-heads, head_dim 16, seq 8,
  ffn 32).
- `every_op_class_roundtrips` extended to Slice + Rope (q42 byte round-trip). Non-GPU `wgsl_forge::`
  **141 passed / 0 failed / 47 ignored**; lib clean plain + `--features cuda`.

**Honest boundary:** K and V come from the (head-major) cache **externals** — already RoPE'd, as the
engine stores them. The *current token's* K/V projection + RoPE + append into the mutable cache is the
**decode-loop integration step** (the cache is mutable state living at that seam, not in this
functional graph); this layer is the attention-over-cache compute, which is exactly what the bake-off
compares. Still f32 (not yet the p64/quantized weights), and not yet wired into `inference_agent`.

**⚑ Where I need the human:** none this step.

**Next step:** the **p64 → forge bridge** + the **one-real-layer bake-off**. Read a real small model's
weights via `P64TensorIndex::from_p64` (role-tagged), `load_weights` the projections/FFN resident,
drive `decode_layer_graph` per token, and bake off — same weights — against the hand-written
`dispatch_attention_pass`/`dispatch_ffn_pass`, asserting f32-tol match (the working engine is the
oracle) and reporting ms/layer for both. Then KV-cache update at the decode-loop seam.

---

## 2026-06-29 — Bake-off prep: engine map + faithful RMSNorm weight + native-layout `trans_b`

**Step / phase:** map the real engine (the bake-off oracle) and close the two faithfulness gaps it
revealed before wiring the p64 bridge. **Status: done, GPU-certified on A2000.**

**Engine map** (three parallel Explore scouts over `gguf_bridge/`, `inference/`, `q42/p64_weight.rs`,
model assets). The facts that constrain the forge to match the *working engine*:
- **RoPE = interleaved (GGUF NORM), base = `rope_freq_base` (100,000 for SmolLM2), `scaled_pos =
  pos/rope_scale`**; K is RoPE'd, V is not. My `Rope` op (mode 0, base param) matches.
- **Projection weights are `[in,out]` row-major in GGUF metadata, but the tensor DATA is laid out
  `[out,in]` row-major** (ne[0]=in is the contiguous dim) — i.e. the native weight is `Bᵀ` of what a
  plain `A·B` wants. The engine's GEMM reads it as `[out,in]` and computes `y[out]=Σ W[out,in]·x[in]`.
- **RMSNorm carries a learned weight** (`x·inv_rms·weight[i]`), eps `1e-5`. SwiGLU FFN, GQA via
  `kv_h = q_h/(n_head/n_kv_head)`, scale `1/√head_dim` — all already matched.
- **p64 API**: `P64TensorIndex::from_p64(&[u8]) → .entries` (role_id, dtype, manifold_idx=layer,
  dimensions), `index.blob(&data, entry)`, `ggml_quants::dequantize_row_into(blob, dtype, n, &mut out)`.
  Roles: ATTN_K/V/Q/OUTPUT 0–3, FFN_GATE/UP/DOWN 4–6, ATTN_NORM/FFN_NORM 7–8, embd/output/norm 9–11.
- **No model file on disk** (gitignored; tests skip when absent). SmolLM2-360M: 32 layers, n_embd 960,
  15 heads, 5 kv-heads, head_dim 64, ffn 2560, rope θ 100k.

**Two gaps closed** (commits `acc6ed66`, this commit), both certified:
1. **Faithful RMSNorm learned weight** — `decode_layer_graph` now multiplies each RMSNorm by its
   `attn_norm`/`ffn_norm` weight (externals [8]/[9]). Cert: composed oracle vs independent reference
   (with norm weights, both RoPE conventions) + A2000 GPU.
2. **`MatMul.trans_b` was a silently-dropped flag** — the IR carried `trans_b`, the executor pattern
   `MatMul { m,n,k,tc,.. }` dropped it AND the CPU oracle ignored it (a latent fake). Now real: a
   `gemm_trans_b` WGSL kernel + CPU oracle branch compute `C[m,n]=A[m,k]·Bᵀ` with **B bound `[n,k]`
   row-major** — exactly the native `[out,in]` GGUF/p64 weight layout, so the forge can consume p64
   projection weights with **no transpose copy**. Cert: CPU oracle vs independent `A·Bᵀ` reference,
   and A2000 GPU vs **plain GEMM on an explicitly-transposed B** (a different kernel path — the gold
   cross-check).

**Verified A2000:** 8/8 executor GPU certs pass (incl. `matmul_trans_b_gpu_matches_plain_on_transposed`,
`decode_layer_*`); non-GPU `wgsl_forge::` **142 passed / 0 failed / 48 ignored**; lib clean plain +
`--features cuda`.

**⚑ Where I need the human:** to run the **real head-to-head** (forge layer vs the hand-written engine
on actual SmolLM2 weights, + tok/s), drop a SmolLM2-360M GGUF into `docs/models/` (e.g.
`smollm2-360m-instruct-q8_0.gguf` or `SmolLM2-360M-Instruct-Q4_K_M.gguf`). With no model on disk the
bake-off can build the bridge + a hermetic synthetic-p64 cert, but the engine-parity + tok/s numbers
need real weights. One file unlocks the competition proof.

**Next step:** the p64→forge bridge (read role tensors via `from_p64`, map to `decode_layer_graph`
externals using `trans_b` for the native layout, `load_weights` them resident) + a hermetic
synthetic-full-layer-p64 cert; then the model-gated engine bake-off.

---

## 2026-06-29 — p64 → forge bridge: forge decode layer on REAL SmolLM2-360M weights (DONE)

**Step / phase:** the p64→forge bridge + the first **real-weights** proof (Timothy supplied the model:
`C:\LLM_Models\GGUF\smollm2-360m-instruct-q8_0.gguf`). **Status: done, run on the A2000.**

**What was built** (`crates/qualia-core-db/src/wgsl_forge/graph_ops/p64_bridge.rs`, new):
- `read_role(index, data, role, layer)` — locate a role-tagged tensor in a `P64TensorIndex` and
  dequantize it to f32 via `ggml_quants::dequantize_row_into` (handles the model's Q8_0).
- `read_forge_layer_weights(index, data, layer)` — read a decode layer's `Wq/Wo/Wg/Wu/Wd/attn_norm/
  ffn_norm`, transposing the 2-D projections from the native `[out,in]` p64 layout to the `[in,out]`
  the forge graph consumes (one-time load cost; `trans_b` is the no-copy upgrade for a follow-on).

**Measured / verified** (A2000, `#[ignore]` real-model cert):
- `forge_decode_layer_on_real_p64_weights_matches_oracle` — **PASS**. Loads the 386 MB SmolLM2-360M
  Q8_0 GGUF, `compile_gguf_to_p64`, `from_p64`, reads layer-0 weights through the bridge, and runs the
  forge `decode_layer_graph` at the model's **real** dims (n_embd 960, 15 heads, 5 kv-heads GQA,
  head_dim 64, ffn 2560, RoPE base 100k). **forge == composed CPU oracle at max rel 3.28e-6** — the
  forge runs SmolLM2's actual layer-0 weights correctly. (Skips cleanly when no model is on disk.)
- Non-GPU `wgsl_forge::` 142 passed / 0 failed / 49 ignored; lib builds clean.

**Honest boundary:** the **weights, dims, and RoPE base are the model's real values**; `x` and the KV
cache are synthetic (this certifies the layer *compute* on real weights, not generated text). The
remaining piece for the full competition number is the **engine head-to-head**: run the *hand-written*
`dispatch_transformer_layer` on the same p64-derived index and assert forge==engine token-for-token,
plus ms/layer + end-to-end tok/s. That needs constructing the engine (KV cache + `GgufTensorIndex` via
`to_gguf_index()`) — the next unit, now unblocked by the model on disk.

**⚑ Where I need the human:** none this step (model supplied — thank you).

**Next step:** engine head-to-head — forge `decode_layer_graph` vs the hand-written
`dispatch_attention_pass`/`dispatch_ffn_pass` on the same SmolLM2 weights (parity + ms/layer), then
wire the forge path into the decode loop behind a flag (`inference_agent.rs` seam ~1077) with KV-cache
update, for end-to-end tok/s.
