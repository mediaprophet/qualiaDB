# Inference decode: native GPU-resident fast path (one fence per token)

**Owner:** Claude (Fable 5), 2026-07-05. **Lane:** `crates/qualia-core-db/src/{inference,gguf_bridge}` +
decode shaders (claimed in `coordination/NOTICES.md` 2026-07-05).
**Goal:** raise engine decode tok/s and extend decode capability (real sampler), starting from the
settled 0.0.23 baseline (~18.8 tok/s, SmolLM2-360M Q8, Vulkan, RTX A2000; ~63% attention / ~37% FFN
of forward; ~19% fence overhead; DX12 decode deadlocks — Vulkan is the only working GPU path).

## 1. Measured structure of the hot path (read from code, 2026-07-05)

Per decode token, native, all current fast-path toggles ON (resident weights, KV preproject,
attn-O fuse, FFN fusion, coop GEMV, GPU top-1):

| Step (per layer ×32) | Submits | Blocking fences | CPU work between |
|---|---|---|---|
| attn RMSNorm | — | — | CPU (`prepare_pre_norm_input`) + re-upload hidden |
| K/V preproject fused (`attention/preproject.rs`) | 1 | 0 | — |
| Q-SDPA + O-proj fused (`attention/fused_tail.rs`) | 1 | **1** (readback ~3.8 KB) | residual add on CPU |
| FFN RMSNorm | — | — | CPU + re-upload |
| FFN fused gate/up/SiLU·mul/down (`ffn.rs`) | 1 | **1** (readback) | residual add on CPU |
| **Total forward** | **96** | **64** | 64 CPU norms/residuals + 96 `write_buffer` of activations |
| output norm + GPU top-1 (`output.rs`) | ~6 | 1 | — |

So ~65 blocking `submit→poll(wait)` round-trips per token. At the profiled empty-round-trip cost this
is the measured ~19% fence overhead — and the *hidden* cost is larger: the GPU idles during every
CPU norm/residual/turnaround between the 96 sequential submits, which is booked as "attention/FFN
time" in the profile, not as fence time.

The **wasm MC8 path already implements the fix** (`gguf_bridge/mc8_wasm/`, `encode_attn_ffn_tail_gpu`,
`prefill_async.rs` ELEM_OP chains): whole token encoded into ONE `wgpu::CommandEncoder`, hidden state
resident in a GPU buffer, RMSNorm / SiLU·mul / residual-add as `ELEM_OP_*` compute passes (those three
pipelines are already built natively in `init.rs`), uniform params staged in 256-byte arena slots so
one encoder can carry many passes. `WasmGpuPipeline` is a plain wrapper over `wgpu::CommandEncoder` —
nothing browser-specific. Native never adopted it.

## 2. The plan

### A. Native resident-token decode (the tok/s lever)
New module `gguf_bridge/resident_decode.rs` (new file per §11 — no monolith growth):

1. **Param/bind-group plan built once per model** — per-layer uniform slots (256-B aligned arena) and
   pre-created bind groups (legal: resident weight buffers + fixed activation buffers are stable),
   removing per-token `create_bind_group` + `write_buffer(params)` CPU cost.
2. **`dispatch_token_forward_resident()`** — upload the embedded token once, then encode per layer:
   RMSNorm(elem) → KV preproject → Q-SDPA → O-proj → residual(elem) → RMSNorm(elem) → gate/up GEMV →
   SiLU·mul(elem) → down GEMV → residual(elem); after layer 32: output RMSNorm(elem) → logits GEMV
   chunks → top-1 reduction → **single staging copy, single submit set, ONE `poll_wait` per token**,
   readback = top-1 candidates only.
3. **Toggle + honest fallback:** `QUALIA_LLM_RESIDENT_DECODE` (default ON once verified); any
   ineligibility (unsupported quant, missing buffers, sieve mask active) falls back to the existing
   per-layer path unchanged.
4. **Correctness gates:** differential test — resident vs legacy decode must emit identical text on
   the bench prompts (same kernels, same order; the one numeric change is RMSNorm CPU→GPU elem op,
   already the proven wasm-decode convention). Existing a1a/a1c coherence + parity tests stay green.

Expected effect (honest): removes ~64 of ~65 fences (~10 ms/tok at baseline ≈ 19%) **plus** the
CPU-turnaround GPU idle between 96 sequential submits. Real number comes from re-running
`a0_decode_profile`; no projection is claimed beyond "fence share → ~0 and turnaround idle removed".

### B. Real sampler (capability/functionality)
Decode is pure greedy argmax today (repetition collapse is documented in the tree). Add a proper
sampling chain, exact (not approximated over top-K): when sampling is requested, read back full
logits for the token (196 KB — acceptable at one fence/token) and run temperature → repetition
penalty → top-k → top-p → seeded RNG on CPU. Greedy stays on the GPU top-1 fast path. Surfaced
through the existing agent config; deterministic under a fixed seed.

### C. Measure + report
`a0_decode_profile` + `a0_native_llm_baseline` before/after on the same machine/model/backend;
results to `STELLAR-style` progress log + NOTICES release line. Kernel-level claims never
extrapolated to end-to-end.

## 3. seeAlso — disposition of the draft research notes

**Source:** [`QualiaDB Inference Pipeline Improvement Report.md`](QualiaDB%20Inference%20Pipeline%20Improvement%20Report.md)
(Timothy's seeAlso, 2026-07-05). Engaged in full; per-hypothesis disposition against this tree:

**Grounding correction first (so the good ideas survive it):** the report's §2 "qualia-core-db
primitives" (`Store`/`Checkpoint`/`Collection`/`CachedMapping`, experiment-tracking SQLite,
`RawDataChunks`) describe the **crates.io `qualia` document store and the LEAT-EDGE/qualia Python
edge-ML framework — different projects that share the name**, not this repo (works-cited 1–3 point
at those). The hypotheses below are re-grounded onto the actual stack (NQuin/q42, `gguf_bridge`,
`wgsl_forge`, the 80 MiB GPU KV arena) — the *directions* survive; the named primitives don't exist
here.

| Hypothesis | Disposition here |
|---|---|
| **H1 — KV compression via Top-K SAE / Lexico sparse dictionaries** (real: arXiv 2412.08890) | **Registered as the long-context lever, not this pass.** Today's KV arena is dense f32, 80 MiB at ctx 1024 — fine for SmolLM2-class; the wall arrives with long context / bigger models. Lexico needs trained per-layer dictionaries + OMP per token — a quality-gated (D20-style) eval task with a real PPL gate, same discipline as the ternary/AWQ work (where naive PTQ already taught us the honest lesson). The Key/Value asymmetry insight (sparse routing keys vs dense payload values) is worth keeping when this is picked up. |
| **H2 — thermal routing + q4_2 storage** | **Principle adopted, format superseded.** Fine-grained scale groups are exactly why the settled shippable compression is **Q4_K_M** (superblock scales — finer-grained than q4_2's 16-elem blocks; q4_2 itself was removed from llama.cpp years ago as a dead end). Thermal: `ThermalGovernor` exists in `orchestrator.rs`; wiring it to real telemetry + efficiency-curve power capping is a real, separate workstream (edge targets), noted in §4. |
| **H3 — 10D manifold / projective-manifold-gradient embeddings** (real: CVPR 2022) | **Homonym warning + geometry-lane pointer.** The report's "10D" is the symmetric-matrix over-parameterization of *rotations* — NOT the `.10d` container's ten semantic axes. Do not graft one onto the other on the strength of the name. Rotation-representation choices belong to the computational-geometry / spatial lanes (`container_10d/`, Tensor10D), not the LLM decode path; flagged there rather than acted on here. |
| **H4 — direct-AMX kernels (Apple)** (real: the M1 AMX prefill paper, ~1.44× GEMM prefill) | **Future Apple-host prefill lever.** This stack is wgpu-first (Metal on macOS), and AMX is a CPU-side coprocessor path — a platform-specific kernel lane analogous to the existing CUDA WMMA work, worth registering for when Apple hosts become a shipping target. No effect on the current Vulkan/A2000 decode work. |
| **H5 — CML/FML symbolic routing** | **Direction already native.** "Query conversational context without a forward pass" is what the graph engine + `neuro_symbolic_sieve` + Q42/CBOR-LD term codes already are; the Rights Ontology gates are the deontic layer. Adopting the *CML/FML-APML vocabularies specifically* is an ontology-curation decision (Timothy's), not an engine change. |
| **§8 — FHE + matrix-chain-ordered routing** | Privacy engine (BFV, feature-gated) exists; MCM-optimal encrypted layout is a distributed-inference item for when remote/multi-node inference is real. Not this pass. |

The report's core physics claim — **inference is memory-movement-bound, not FLOP-bound** — is
exactly what this pass acts on: resident weights killed the per-token weight re-upload (done,
Phase 2), and the resident-token path (§2A) kills the per-layer activation round-trips + fences,
which are the last CPU↔GPU movement in the decode loop.

## 4. Out of scope (flagged, not silently dropped)
- **DX12 decode deadlock** — likely fence/poll related; the 1-fence/token path may change its
  surface. Re-test DX12 after A; if still hung, file the finding (separate bug hunt).
- **Prefill** batching already exists; prefill gets the param-arena benefit for free only if later
  ported — not this pass.
- **Tensor-core/coopmat matmuls** — blocked upstream (wgpu #9741 unreleased), tracked in
  `docs/WGPU_UPSTREAM_TRACKING.md`.
- **KV-cache quantization / compression, speculative decode** — future levers, not started here
  (H1 above is the researched candidate for the compression half).
- **Thermal governor → real telemetry + efficiency-curve power capping** (H2) — edge-target
  workstream; `ThermalGovernor` exists but reads no real diode yet.
