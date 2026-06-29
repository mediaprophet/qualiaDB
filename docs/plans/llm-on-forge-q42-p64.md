# Plan — Run the LLM on the native q42/p64 substrate via the DAG-IR forge

**Status:** proposed (2026-06-29). Author: forge instrument. Supersedes the hidden plan-mode draft.

## Context

Four things already exist in this repo, built but **not connected**:

1. **A real, working LLM engine** — `inference/inference_agent.rs:997–1277` is a genuine autoregressive
   decode loop: multi-head attention, RoPE, Q4_K/Q8_0/Q6_K dequant, a GPU-resident KV cache
   (`gguf_bridge/init.rs`), greedy sampling, sieve masking, and the **Phase-8 governance**
   (the bifurcated Sentinel `LogitStream`/`ControlStream` + `validate_intent`/`validate_output` in
   `orchestrator.rs:457,550`). But the per-layer compute is **hand-written WGSL**
   (`fused_transformer.wgsl`, `fused_attention.wgsl`, `fused_ffn.wgsl`) — the exact maintenance /
   portability burden the forge was built to remove.
2. **A complete GGUF→p64 transcoder** — `q42/p64_weight.rs::compile_gguf_to_p64` (266–483) already
   produces the native **p64** weight container: role-tagged tensors (`P64_ROLE_ATTN_Q/K/V/O`,
   `FFN_GATE/UP/DOWN`, norms, embd, output), page-aligned blobs, embedded tokenizer, CRC-32C, **no
   re-quantization** — plus an **AWQ ternary-FFN** path (`compile_gguf_to_q42_ffn_quant_awq`, ~1.58-bit).
   The engine already branches on the `p64\0`/`.q42` magic and has `P64TensorIndex::from_p64`.
3. **The q42 provenance substrate** — Merkle-DAG, ODRL, the natural-person vs software-agent DID
   bifurcation (`Q42VolumeHeader`). But the q42 graph **over the model weights** (source hash, transcode
   lineage, consent chain) is *sketched, not integrated*.
4. **The DAG-IR forge** — certified, multi-backend, pipeline-cached. It runs a **faithful** decode block
   today at **19.5 ms (2.89× the CPU)** with a held `ForgeGraphExecutor` + pipeline cache. But it
   **does not consume p64**, **re-uploads weights every call**, **creates its own device**, and lacks
   multi-head / RoPE / QKV / lm_head / embedding builders.

**Goal:** make the LLM forward pass run on the forge, off the native p64 substrate, with q42 provenance
and the existing governance intact. The win is **connecting** what exists and replacing hand-written
WGSL with generated+certified graphs.

## Competitive frame (run the race we can win)

Not a tok/s drag race vs vLLM/TensorRT on NVIDIA — they win that. The category **nobody else enters**,
and whose pieces are mostly already here:

- **Provable correctness** — every decode kernel certified against a CPU oracle (the forge's discipline).
- **Provenance + consent + audit** — q42 Merkle-DAG over p64 weights + the Phase-8 Sentinel:
  *auditable, consent-anchored, governed* local inference.
- **Memory edge with proof** — AWQ ternary FFN (~1.6-bit) consumed by the forge's **certified** ternary
  GatherDequant: BitNet-class footprint *with* correctness + provenance, which nobody combines.
- **Portability** — one DAG-IR → any GPU (WGSL/CUDA/MSL/HLSL), not hand-written per backend.

Speed only has to reach **"competitive enough"** (tensor cores + weight residency + pipeline cache);
the moat is the differentiator.

## Approach — phased, **non-destructive**

The existing hand-written engine is real and works. It stays as the **oracle and fallback**; the forge
path is built **alongside** it (selected by a flag / a new `AgentBackend` sub-mode) and only becomes the
default once it is proven to **match** the hand-written outputs and **beat** its latency. No rip-out of a
working engine before its replacement is certified.

### Phase 1 — Forge consumes p64 (keystone + first measured win)

The technical foundation: the forge must run real layers off GPU-resident p64 weights on the engine's
device. Three changes:

- **Shared device** *(seam confirmed in code).* The engine already centralizes on
  `gpu_context::shared_gpu()` — a `OnceLock<SharedGpuContext>` (HighPerformance adapter, **buffer limits
  raised to the adapter max** so >256 MiB weight tensors are legal, f16/subgroup/coopmat-capable,
  VramLedger-wired); `gguf_bridge/init.rs:13,25,50` shows inference + KV cache run on it on native. The
  **forge** is the outlier: `ForgeGraphExecutor::new()` calls `WgpuComputeContext::new()`, which requests
  its **own** device. Fix = add `WgpuComputeContext::from_device(device, queue, …)` (wgpu `Device`/`Queue`
  are cheap `Arc`-clones) that maps `shared_gpu()`'s `GpuAdapterCaps` → the forge's `AdapterConstraints`
  /`HardwareProfile` and builds the forge slabs **on the shared device**; then
  `ForgeGraphExecutor::with_context` runs on it. Now forge buffers, resident weights, and the KV cache
  are on one device (and one VramLedger). (Not the whole-engine sprawl I first guessed — topk/ternary/lora
  keep their own small devices, secondary to this path.)
- **Weight residency.** Add a persistent weight region + a `load_weights` API to the executor: upload the
  big matrices **once** (referenced by offset across tokens), and have `run()` take **activation-only**
  externals (x, inv_scale, eps) — fixing the per-call re-upload (the forge-seam map's #1 perf gap; the
  slab allocator already supports upload-once residency, only `run()` re-uploads).
- **p64 → forge bridge.** A reader (reuse `P64TensorIndex::from_p64`) that hands the forge role-tagged
  tensors; a builder mapping p64 roles → the decode graph externals. Add the missing **real-layer**
  builders the map flagged: QKV projection, output projection, **multi-head** attention (per-head loop or
  batched, with the axis-aware reduce), **RoPE** (`Stencil::RopePair` is already a forge op).

**Verify (the bake-off):** run **one real transformer layer** of a small real model
(SmolLM2-360M, already referenced by `tests/llm_bench_a0.rs` / `docs/models/`) through the forge decode
graph **and** through the existing hand-written `dispatch_attention_pass`/`dispatch_ffn_pass` on the same
p64 weights; assert they match within f32 tol (the working engine **is** the oracle), and report
ms/layer for both. This is the certified, measured competitive number.

### Phase 2 — Full forward + **auditable** real generation

- Compose `embed → [forge decode_layer]×L → final-norm → lm_head → logits → sample` off p64, held
  executor across tokens, KV cache = the engine's existing GPU cache.
- Wire it into the decode loop at the clean seam: replace `dispatch_transformer_forward`'s per-layer
  hand-written dispatch (`inference_agent.rs:1072–1090`) with the forge graph **behind the flag**,
  **preserving** the Sentinel rings + `validate_intent`/`validate_output` + sampling untouched.
- **q42 provenance graph (the moat, woven in here so the first real generation is already auditable):**
  auto-generate the q42 quins over the transcoded p64 model — model structure, **source-GGUF hash**,
  **transcode algorithm**, weight-tensor pointers (modality flag `0b1001` → p64 offset), and the
  **consent chain** (natural-person vs software-agent DID bifurcation already in the header); write the
  unified q42 volume + Merkle-audit it. Surface it to `validate_intent`/`output` so generation cites the
  model's provenance.

**Verify:** end-to-end greedy generation through the forge path produces the **same tokens** as the
hand-written path on a fixed prompt; measured tok/s; the q42 volume round-trips and its Merkle root is
stable; a generated output carries its model-provenance citation.

### Phase 3 — The differentiators: tensor cores + AWQ ternary FFN

- Route the decode MatMuls through `gemm_f32_tc` (CUDA WMMA now; coopmat on the wgpu release, #57) for
  tensor-core throughput.
- **AWQ ternary FFN:** feed p64's AWQ ternary FFN blocks into the forge's **certified** ternary
  `GatherDequant → MatMul` (`graph_ops/gather_dequant.rs`, `dequant_matmul_graph`) — ~1.6-bit FFN with
  proven correctness. Bake-off vs the f32 FFN for quality + measure the memory/latency win.

**Verify:** tensor-core decode matches the f32 reference within f16 tol; ternary-FFN generation quality
within an acceptable threshold (**a curation call — Timothy sets the threshold**); measured VRAM + ms.

### Phase 4 — Full quant coverage (Q4_K/Q8_0) + portability

- Add **Q4_K** and **Q8_0** `GatherDequant` kernels to the forge (it does Ternary only today;
  `graph_ops/gather_dequant.rs` + executor arm), so the forge consumes **all** real GGUF quant directly
  from p64 — not just ternary/f32. Reuse the `ggml_quants` block layouts as the CPU oracle.
- The multi-backend forge means the decode path then runs on **any** GPU — the portability differentiator
  (MSL/HLSL lowerers already exist for the kit).

**Verify:** Q4_K/Q8_0 forge dequant matches `ggml_quants` CPU dequant; a full forward on a Q4_K model
matches the hand-written path; the decode graph naga/dxc-validates for the portable backends.

## Critical invariants the refactor MUST respect (from the backend map)

- **One shared device, never recreated** — run on the singleton context; do not spawn a second device.
- **Weights upload-once, GPU-resident** — never re-upload weight matrices per token.
- **U0 thread owns the GPU queue** — the forge dispatch happens on the decode-loop thread; the Sentinel
  stays on its SPSC rings; bind via `platform_scheduler::bind_inference_thread`.
- **Two-slab read/read_write model** — weights/KV in the read slab, layer outputs in read_write, one
  `submit_graph` per graph (already how the executor works).
- **Governance is non-negotiable** — `validate_intent` (pre) + `validate_output` (post, ≥1 provenance
  quin) + the Phase-8 `DenyRollback` path must remain intact across the swap.
- **No re-quantization in transcode; CRC + Merkle are fail-closed** — preserve p64's integrity guarantees.

## Files (reuse vs new)

- **Reuse:** `q42/p64_weight.rs` (`P64TensorIndex::from_p64`, roles, `compile_gguf_to_p64`,
  `compile_gguf_to_q42_ffn_quant_awq`); `wgsl_forge/graph_ops/executor.rs` (`ForgeGraphExecutor`,
  `decode_block_graph`, `push_rmsnorm`/`push_softmax`, `dequant_matmul_graph`);
  `wgsl_forge/graph_ops/gather_dequant.rs`; `wgsl_forge/execute/wgpu.rs` (context, slab,
  `compile_pipeline_cached`, `submit_graph`); `gguf_bridge/init.rs` (KV cache); `orchestrator.rs`
  (governance gates); `ggml_quants` (dequant oracles); `q42/q42_volume.rs` (provenance volume write).
- **New (forge side):** a p64→forge bridge module; the multi-head/RoPE/QKV/lm_head/embed builders;
  `ForgeGraphExecutor::with_context` + `load_weights`; Q4_K/Q8_0 GatherDequant kernels; a
  `forge-decode` `AgentBackend` sub-mode behind a flag; the q42-model-provenance generator; the
  layer-bake-off + end-to-end-generation tests.

## Honest scope & risk

This is a large, multi-stage refactor of a real engine — weeks-of-equivalent work, not a single pass.
Each phase is independently valuable and verifiable, and the existing engine remains the oracle/fallback
throughout, so there is no window where a working model is broken. Real risks, stated: (a) multi-head +
RoPE in the forge must match the hand-written kernels exactly (the bake-off is the gate); (b) sharing one
device + weight residency touches the executor's allocation model (covered by the existing GPU certs +
the bake-off); (c) AWQ-ternary FFN quality is a **curation call** (acceptable-quality threshold is
Timothy's to set); (d) it does not, and should not claim to, beat vLLM on raw NVIDIA tok/s — the win is
the certified + provenanced + portable + memory-efficient category.

## Verification (end-to-end)

`cargo test -p qualia-core-db` for the per-phase certs (layer bake-off, generation parity, q42 round-trip,
Q4_K/Q8_0 dequant), the `--ignored` GPU certs on the A2000, the decode bench for ms/layer + tok/s, and a
real SmolLM2-360M generation through the forge path matching the hand-written path token-for-token (greedy)
with its provenance citation attached.
