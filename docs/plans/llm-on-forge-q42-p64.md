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

## Competitive frame — name the real competitors; target genuinely faster, then measure

**Name the actual competitors.** The local-LLM field is **Ollama / LM Studio** (UX wrappers around
**llama.cpp / MLX**) and llama.cpp itself — hand-written kernel engines with **no compute-graph IR, no
auto-tune+certify, no q42/p64 substrate, no in-queue solvers**. Datacenter vLLM/TensorRT are a different
deployment and not the target. **Do not import "can't beat the datacenter SOTA" onto the local field** —
that is a defeatist anti-pattern; from a position of *structural advantage* it manufactures insecurity.

Qualia has a **computational engine the local competitors structurally lack**, and that is a *speed asset*,
not only a moat. Local decode is **memory-bandwidth-bound, not compute-bound**, and the engine gives
specific levers to be **genuinely faster** (not "competitive enough") — to be **built and measured**, never
pre-conceded and never pre-claimed:

- **Sub-4-bit weights.** AWQ ternary FFN (~1.58-bit) through the certified `GatherDequant` moves **~2.5×
  fewer bytes per token** than llama.cpp's Q4_K on the FFN bulk (the dominant param mass). Fewer bytes off
  VRAM / unified memory ⇒ faster decode. They don't ship BitNet-1.58 for arbitrary models; we transcode to it.
- **Per-hardware auto-tuned, certified schedules.** The forge's tune/certify + topology cache fits the
  dispatch (workgroup/tiling/warp-align) to the *actual* adapter and caches a certified-optimal; llama.cpp
  ships fixed kernels. Across a heterogeneous fleet (Pi / Apple / AMD / off-grid) per-machine tuning beats one-size.
- **Composite workload in one GPU queue.** Inference + N3 logic + solvers + verification on the *same*
  device/queue — no CPU/process round-trips. Ollama/LM Studio are inference-only; Qualia's governed/grounded/
  reasoned generation would force them off-GPU. They can't match this because they lack the engine.
- **Cached-graph orchestration** (one submitted graph/token to the U0 queue) beats naive per-layer WGSL
  dispatch on the portable any-GPU path.

These rest on the same memory-residency + bandwidth efficiencies that beat **naive local inference**:

- **Transfer tax eliminated.** p64 weights resident once (`load_weights`); only activations cross the bus
  per token — no per-token weight re-upload. Bypasses the PCIe/bus bottleneck on every decode step.
- **Bandwidth wall broken.** AWQ ternary FFN (~1.58-bit) through the **certified** `GatherDequant`: the
  dominant FFN payload is ~5–10× denser than f16, so the memory-bound step stops stalling. **This win is
  real on any GPU, including unified-memory edge — it is bandwidth, not a tensor-core feature.**
- **Orchestration collapsed.** The pipeline-cached decode graph hands **one** submitted graph to the
  U0-owned queue per token, instead of per-layer host-driven dispatch/binding. CPU-bound orchestration →
  continuous GPU saturation.
- **Portable.** One DAG-IR → any GPU via WGSL, not hand-written per backend.

**Gated, kept separate (no conflation):** *continuous* tensor-core feeding needs the coopmat path
(dormant, #57) or CUDA WMMA (NVIDIA-only). That is an additional layer; the memory-bandwidth + residency +
orchestration win stands **today**, on any GPU.

**What is genuinely unavailable by other means:** the optimizations and the
provenance/consent/governance are the **same substrate** — p64 GPU-resident weights + the q42 Merkle-DAG
over them + the Phase-8 gates — so a fully *auditable, consent-anchored* forward pass runs at edge-viable
latency at **no extra architectural cost**. Extraction-architecture engines can't bolt this on without
abandoning their model; here it is native. Optimization at the bare metal *is* the political act: edge
viability is the precondition for breaking dependence on hyperscale APIs and the "platform-as-god" model —
this is the infrastructure for localized cognitive autonomy, not just an inference engine.

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

**The competition proof (do not skip):** a **head-to-head vs Ollama / LM Studio** on the *same* model and
the *same* hardware — tok/s, time-to-first-token, peak VRAM, and watts — reported honestly (the ternary +
auto-tune + residency levers are the thesis; measure whether they net to *faster*, not just different). Run
it on at least one memory-bandwidth-bound target (a unified-memory / edge box), since that is where the
structural advantage is largest. Pre-conceding is forbidden; pre-claiming a win before the numbers is equally forbidden.
