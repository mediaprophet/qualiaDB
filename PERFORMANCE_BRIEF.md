# QualiaDB — LLM Inference Performance Brief

*A status brief for external review. Goal: invite concrete suggestions on improving **TTFT**
(time-to-first-token) and **TPS** (tokens/sec) for on-device LLM inference. Written to be read cold —
no prior project knowledge assumed. Dated 2026-06-23.*

---

## 0. TL;DR — what we're asking

QualiaDB runs LLMs **in-process** in Rust on **wgpu** (DX12 / Metal / Vulkan natively; WebGPU in the
browser). Models are transcoded to a custom **`.q42`** weight container and executed by hand-written
**WGSL** compute kernels. We are mid-way through a compression-led performance push (BitNet-1.58b
ternary, with KIVI / W4A4 / speculative-decoding planned).

**We want outside eyes on:** (a) whether our bottleneck analysis is right, (b) the highest-leverage
next moves for TTFT/TPS on **consumer hardware** (dev card: **NVIDIA A2000, 12 GB**), and (c) any
kernel/scheduling/quantization techniques we're missing. Specific questions in **§8**.

**Hard constraints (non-negotiable, shape every suggestion):**
- **No external inference libraries.** No llama.cpp / ggml / ONNXRuntime / PyTorch at runtime. The
  stack is our own Rust + WGSL. (Architectural + governance reasons.) Suggestions must be
  implementable in our own kernels/host code, not "call library X".
- **Affordability.** Must run on hardware ordinary people own — a 12 GB card, a laptop, a phone (WASM).
  No "rent an H100" answers.
- **Zero-heap / fixed-budget hot paths.** The engine is built around a 48-byte record and bounded
  arenas; the LLM weight-loading mmap is the one sanctioned large allocation.

---

## 1. The system in one page

- **Language/runtime:** Rust, single process. The graph engine and the LLM share **one wgpu device**.
- **GPU abstraction:** `wgpu` 0.20. Native backends (DX12 on Windows, Metal, Vulkan) + **WebGPU** for
  the `wasm32` browser build. Same WGSL everywhere.
- **No Python, no Ollama, no llama.cpp.** GGUF/safetensor are *transcoded* into our own format and run
  by our own kernels.
- **Targets:** native desktop/mobile, plus `wasm32` (browser, primary web path). `wasm64`/memory64 is
  decided but unbuilt. NPU paths exist (DirectML / CoreML-ANE / NNAPI) but are partial.
- **Governance overhead (by design):** a "Phase-8 bifurcated" path streams logits to a sentinel thread
  that can veto mid-generation. It's a deliberate feature, not removable, but its cost should be
  understood when profiling.

---

## 2. Weight format & memory model

### 2.1 The `.q42` / `Q42W` container
- A model is transcoded ahead-of-time into a `Q42W` weight container: a 144-byte header → a tensor
  **manifest** (per-tensor: role, layer, ggml-type, dims, blob offset/len, CRC) → **page-aligned**
  weight blobs. CRC-32C over header+manifest is checked once at load.
- The broader `.q42` archive is `header → lexicon → block-index (bidx) → block-dir → LZ4 40 KB
  SuperBlocks`, supporting a **two-step range fetch** (fetch small index; one HTTP Range for one 40 KB
  block) — built for streamed/partial loads.
- **Source fidelity policy:** we ingest **high-fidelity only** (Q8 / F16 / BF16); Q4-class inputs are
  rejected. The engine does its *own* down-sampling (quantization) during transcode so quality is
  controlled by us, not inherited from someone else's lossy Q4.

### 2.2 Memory residency (this is central to the TTFT/TPS story)
The source is **memory-mapped** (`memmap2`, zero heap copy) → the OS demand-pages disk→RAM. On the GPU
side there are **two residency modes**:
1. **Resident ("all weights uploaded once")** — every layer's weights live in VRAM; the hot path just
   binds sub-ranges. Fastest. Requires the model to **fit in VRAM**.
2. **Streaming ("one tensor/layer in VRAM at a time")** — a sequential layer-by-layer forward that
   reuses small staging buffers. Runs models **larger than VRAM** (paged from the mmap), at the cost of
   a PCIe upload per layer per token.

A VRAM ledger/budget chooses between them. **Implication for a 12 GB A2000:** small/mid models run
resident (fast); a 7B BF16 (~14 GB) must stream (slow) — *unless* compression shrinks it under 12 GB,
at which point it runs resident. **Compression is therefore also a residency lever, not only a
bandwidth lever.**

---

## 3. The inference pipeline

A forward pass per token (decode) / per prompt (prefill):

```
embed lookup
 └─ per layer ×N:
     RMSNorm → QKV projection (GEMM) → attention (scores·softmax·∑V)
            → output projection (GEMM) → +residual
     RMSNorm → FFN: SwiGLU = silu(gate·x) * (up·x)  → down·x  (GEMMs) → +residual
 └─ final RMSNorm → output/logits projection (GEMM) → sample
```

- **Kernels (WGSL, hand-written):** `fused_tensor_contraction.wgsl` (general GEMM),
  `fused_ffn.wgsl` (the SwiGLU expansion fused into one dispatch — gate GEMM + up GEMM + SiLU×mul in a
  single pass, saving 2 dispatches + 2 VRAM round-trips per layer), `fused_attention.wgsl`,
  `fused_transformer.wgsl`, `dequant_template.wgsl` (per-weight-role dequant math injected into the
  GEMM at pipeline build).
- **On-GPU weight dequant:** the GEMM dequantizes each weight inline. Supported ggml types in the
  kernels: `Q4_0 / Q5_0 / Q8_0 / Q4_K / Q6_K` (+ F16). There is a **block-amortized** path for Q5_0
  (decode the block scale `d` + `qh` once per 32-elem block, not per element).
- **KV cache:** allocated per hyperparams, bounded; bound in per-layer slices (the full arena exceeds
  the 128 MiB wgpu binding cap).
- **Workgroups:** GEMMs run `@workgroup_size(64)`, `global_id.x` = output feature, `.y` = batch row.

---

## 4. Current performance — measured vs. unmeasured (honest)

**Measured:**
- **Decode:** ~**5.9 tok/s** on **SmolLM2-360M**, **in-browser (WASM/WebGPU)**. This came from fixing a
  kernel bug (see §5) that took it from ~0.6 → 5.9. *Caveats: it's a 360M model, in the browser, and
  the figure is several weeks old.*
- **Ternary compression (new, measured today on the real model):** SmolLM2-360M BF16 **723.7 MB →
  302.5 MB (2.39×)** with the FFN-only ternary policy. The **FFN tensors alone compress ~10×** (472 MB
  → 47 MB, i.e. ~1.6 bits/weight); the 2.39× overall is because attention + embeddings + norms (~252
  MB) are deliberately kept verbatim BF16. The ratio improves with model size (FFN is a larger
  fraction of a 7B) and tightens further once attention is quantized too.
- **Ternary GEMM kernel:** verified **on-device on the A2000** (the WGSL kernel's output matches a
  byte-exact CPU reference).

**NOT yet measured (important gaps):**
- **No clean current *native* tok/s baseline.** The 5.9 figure is browser/WASM. We need a fresh native
  (DX12, A2000) baseline for F16/Q8.
- **Ternary inference TPS is unmeasured** — the ternary kernel is proven on-device in isolation but
  **not yet spliced into the live FFN dispatch loop**, so we cannot yet report a ternary tok/s.
- **No TTFT numbers** (load time + prefill) captured systematically.

---

## 5. Bottleneck analysis (please challenge this)

- **Decode is memory-bandwidth-bound, not compute-bound.** Each generated token reads ~all weights
  once; arithmetic intensity is low. ⇒ the dominant TPS lever is **fewer bytes per weight** (and per
  KV entry). This is why the optimization plan is compression-led (ternary/KIVI/W4A4).
- **Prefill is compute-bound** (a big batched GEMM over all prompt tokens). ⇒ benefits from kernel
  occupancy, fusion, and lower-precision math.
- **A concrete past win shows occupancy dominates:** the historic decode killer was
  `fused_attention.wgsl` running the Q/K/V projection at **`@workgroup_size(1)`** (≈15 threads doing
  projection serially). Routing Q/K/V projection through the parallel GEMM gave ~**10×**. We suspect
  similar occupancy/dispatch-shape issues may remain elsewhere.
- **The WASM path has a hard ceiling:** the browser build currently falls back to a **CPU GEMM** in
  places, which caps WASM TPS regardless of algorithm. Native gets the real wgpu backend + NPU. (We
  prefer native where available; WASM is the zero-install fallback.)
- **PCIe upload** dominates the *streaming* residency mode (models > VRAM) — one weight upload per
  layer per token.

---

## 6. Optimization roadmap (STELLAR §A) — status

| Lever | Mechanism | Status |
|---|---|---|
| **Ternary FFN (BitNet 1.58b)** | FFN weights → {−1,0,+1} + per-tensor absmean scale, 5 trits/byte (~1.6 bit). GEMM = add/subtract, no per-weight multiply. | **codec ✓, transcode ✓ (real model 2.39×), WGSL kernel ✓ on-device (A2000). PENDING: splice into the live FFN loop + measure tok/s.** |
| **Name→role mapping + policy** | Map GGUF/HF tensor names → engine roles; ternary the FFN only, keep attention/norms/embeddings high-fidelity. | ✓ (validated on real SmolLM2-360M names). |
| **KIVI KV-cache** | Key cache 2-bit/channel, Value 4-bit/token → long context in consumer VRAM, less KV bandwidth. | planned. |
| **W4A4 + AWQ** | Activation-aware 4-bit weights/activations (Q8-equivalent quality at 4-bit speed), scales baked into the header. | planned (the hard one). |
| **Speculative decoding** | ~100 M draft guesses 4–5 tokens via zero-copy mmap; target verifies in one pass. | planned. |
| **Demand-paged mmap** | Page layers into VRAM at first use; run > VRAM models. | streaming mode exists; demand-paging refinement planned. |
| **Pre-compiled `.q42` distribution** | Host pre-built `.q42` on HF/WebTorrent → end-user TTFT ≈ 0 (no runtime transcode). | planned. |
| **NPU offload** | DirectML / CoreML-ANE / NNAPI for the GEMMs. | partial / bridges exist. |

---

## 7. Hardware & targets

- **Dev card:** NVIDIA **A2000, 12 GB** (Ampere, GA106). Windows, DX12 backend.
- **Target matrix:** Windows/macOS/Linux native (DX12/Metal/Vulkan; AVX2/NEON; DirectML/CoreML/NNAPI
  NPU), iOS/Android (via PWA/Flutter), **`wasm32` WebGPU** (primary web), `wasm64` deferred.
- **Design intent:** the same model and kernels run everywhere; only the math backend swaps. Heavy
  *transcode/compression* passes run once, ahead-of-time, on a capable node and are distributed; the
  end device pays only the cheap fold.

---

## 8. Questions for reviewers (the asks)

1. **Is the "decode = memory-bound ⇒ compress weights" framing correct** for these model sizes on a
   12 GB Ampere card, or are we leaving compute/occupancy on the table that matters more?
2. **Ternary GEMM kernel design:** our inner loop does branch-y add/subtract per weight
   (`if trit>0 acc+=x; else if trit<0 acc-=x`) with the per-tensor scale applied once at the end,
   reading trits packed 5/byte base-3. **How would you make this fast on Ampere?** (Avoiding branches?
   packing for coalesced loads? `dp4a`/integer SIMD? shared-memory staging of activations? subgroup
   reductions? Is base-3 packing a mistake vs. 2-bit-per-trit for load efficiency?)
3. **Mixed precision in one model:** we keep attention BF16 and ternary the FFN. Is that the right
   split, or should attention go W4A4 first? Any guidance on where ternary *hurts* quality.
4. **KV-cache:** for long context on 12 GB, is KIVI (2-bit K / 4-bit V) the best bang-for-buck, or
   would you prioritize something else (paged attention, etc.)?
5. **Occupancy/scheduling:** given `@workgroup_size(64)`, one output-feature per thread — what
   workgroup shapes / tiling / persistent-kernel strategies would you try first for both the GEMV
   (decode, batch=1) and GEMM (prefill) regimes? Decode is batch-1 GEMV — notoriously
   bandwidth-bound; any tricks specific to that.
6. **Streaming (model > VRAM):** for the layer-by-layer streaming mode, how would you hide PCIe upload
   latency (double-buffering / async copy queues / compute-copy overlap in wgpu)?
7. **TTFT:** beyond demand-paged mmap + pre-compiled distribution, what most reduces first-token
   latency for a cold start on consumer disk?
8. **WASM ceiling:** anything that meaningfully lifts WebGPU-in-browser decode TPS short of leaving the
   browser (subgroups where available, F16 storage, etc.)?

---

## 9. Pointers (for the curious / code review)

| Concern | Files |
|---|---|
| Ternary codec + GEMM kernel + CPU oracle | `crates/qualia-core-db/src/ternary.rs`, `src/shaders/ternary_gemm.wgsl` |
| On-device dispatch + A2000 parity test | `src/ternary_gpu.rs` |
| Transcode (verbatim / ternary / FFN-policy) + container | `src/q42_weight.rs`, `src/safetensor.rs`, `src/tensor_roles.rs` |
| Live inference dispatch (resident + streaming) | `src/gguf_bridge.rs` (`dispatch_transformer_forward`, `encode_fused_ffn_expansion`) |
| Existing GEMM/FFN/attention kernels | `src/shaders/{fused_tensor_contraction,fused_ffn,fused_attention,dequant_template}.wgsl` |
| GPU device + VRAM budget | `src/gpu_context.rs` |
| Roadmap | `STELLAR_MISSION.md` §A |

---

*Status brief for the QualiaDB / WebCivics project (Timothy C. Holborn). Figures are as-measured on the
dates noted; "planned/pending" items are honestly marked as not-yet-built. Drafted with AI tooling used
as an instrument (not an author).*
