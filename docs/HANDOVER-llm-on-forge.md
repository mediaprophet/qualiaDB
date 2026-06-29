# HANDOVER — LLM on the DAG-IR forge over the native q42/p64 substrate

**Written:** 2026-06-29, end of a long session, context running out. For the next session (fresh
context) or another instrument to pick up **cold** and execute without re-deriving anything.
**Branch:** `feature/p64-manifold-wal-eigensolver` — everything below is committed + pushed.

---

## 0. The mission (the *why* — non-negotiable, read it once)

Build a **local, edge-viable, rights-respecting** LLM inference path that runs on commodity / off-grid
hardware (Pi cluster, Apple Silicon, RERSS nodes) — so people don't have to route their cognition back
through hyperscale APIs. The bare-metal optimizations **are** the political act: if edge silicon can't run
sophisticated inference at acceptable latency/power, the user capitulates to the centralized "platform-as-god"
model. Memory-residency + ternary quantization + a cached compute graph are the material counter to
asymmetric centralization and the extraction economy it runs on.

**Define "faster" correctly:** *not* a datacenter tok/s drag race (vLLM/TensorRT win that; not the contest).
**Locally the binding constraint is memory bandwidth, not compute** — and on that axis this beats naive
local inference, on **any GPU**, *while* dragging the full q42 provenance + ODRL consent + Phase-8
governance through the same forward pass at no extra architectural cost. You can't out-audit a system whose
substrate **is** the audit trail. Full framing: `docs/plans/llm-on-forge-q42-p64.md` → "Competitive frame".

Be precise, not sycophantic. Honest measurement (never claim done when it isn't). See the memory note
`feedback-plans-and-ownership-holborn` for how Timothy wants you to work (repo-markdown plans not hidden
files; prose not survey forms; full ownership — he is the sole allocator, make the calls and own them).

---

## 1. Read first (orientation)

1. `CLAUDE.md` + `AGENTS.md` — standing orientation. **§1: the LLM is NOT Ollama** — native in-process
   GGUF/wgpu stack. Don't add HTTP/Ollama anything.
2. `docs/plans/llm-on-forge-q42-p64.md` — **THE plan** (4 phases). Authoritative.
3. `docs/plans/dag-ir-forge.md` — the forge (P1–P7, all ✅ DONE this session). The substrate you build on.
4. `ACCELERATION_ROLLOUT_PROGRESS_LOG.md` — the running honest engineering record (per-step, measured).
5. `docs/WGPU_UPSTREAM_TRACKING.md` — coopmat/#9741 soft-fork findings (NO-GO for now; see §2).

---

## 2. What is DONE this session (the foundation — don't rebuild it)

**DAG-IR forge P1–P7 — complete, certified on the A2000.** One typed compute-graph IR (`wgsl_forge/ir/graph.rs`)
lowers to **4 backends** (WGSL / CUDA-C / MSL / HLSL) with no per-kernel branches, plus q42 persistence and
the physics/spatial op kit. Key commits: P4c tensor-core `3addc32b`/`1397d142`; **P5 CudaCLowerer** `606c32ab`;
**P4b attention+GatherDequant** `407b2c2f`; **P6 q42 binding** `9d889162`; **P7** `dfb95b43`/`595dbfb4`/`7698d863`.

**Executor throughput + correctness:**
- Pipeline cache + context reuse + single-encoder fusion (`c733869f`, `e01a39ce`): a held
  `ForgeGraphExecutor` runs a faithful decode block at **~19.5 ms, 2.89× the plain-Rust CPU** on the A2000
  (`decode_block_kernel_uplift_bench`). Arc of the metric: 869 → 238 → 16.7 ms over the session.
- **Faithfulness fix** (`bd5f30a0`): added `1/√d` attention scaling + RMSNorm `eps`. The decode block is now
  a *real* transformer block (was unfaithful before — an adversarial review caught it). `decode_block_graph`
  externals are now `[x, kt, v, Wg, Wu, Wd, inv_scale, eps]`.

**Tensor cores:** CUDA WMMA tiled GEMM is **certified + active** (`gemm_f32_tc`). The portable **wgpu coopmat**
path is built + naga-validated + `coopmat_usable()`-probe-gated but **dormant** (returns zeros on wgpu 29.0.3,
upstream bug #9741). **Soft-fork attempted + reverted** (`0492f3b1`): the fix commit `56535d7d` compiles but the
unreleased wgpu `main` has crate-wide API drift (`get_mapped_range → Result`, `RequestAdapterOptions` new field)
across many files — NO-GO until a wgpu release ships it (task #57). Don't re-attempt the soft-fork; watch #57.

**Plan + framing** committed: `59a5136c` (the plan), `93d50446` (corrected "faster" framing).

Full `wgsl_forge::` test sweep: **134 passed / 0 failed**; GPU `--ignored` certs green on the A2000.

---

## 3. The architecture (distilled from 4 deep maps — load-bearing facts, so you DON'T re-run them)

### 3a. Current LLM inference path — it is REAL, not a mock
- Genuine autoregressive loop: `inference/inference_agent.rs:997–1277`. Per token: embed → all layers →
  output RMSNorm → sample → governance check.
- **Per-layer compute is HAND-WRITTEN WGSL** (`shaders/fused_transformer.wgsl`, `fused_attention.wgsl`,
  `fused_ffn.wgsl`), dispatched by `gguf_bridge/forward.rs::dispatch_transformer_forward` (the seam to
  replace). This is what the forge migration retires.
- Already has: **multi-head attention, RoPE, Q4_K/Q8_0/Q6_K dequant, a GPU-resident KV cache**
  (`gguf_bridge/init.rs`), greedy argmax sampling, sieve masking. **The forge's decode block is currently
  *less* capable** (single-head, f32) — Phase 1/Phase 4 close that gap.
- **Governance (mandatory, must survive the swap):** `orchestrator.rs:457` `validate_intent` (pre),
  `:550` `validate_output` (post, ≥1 provenance quin); the **Phase-8 bifurcated Sentinel**
  (`inference_agent.rs:1231–1316` LogitStream/ControlStream + `DenyRollback`, SPSC rings in
  `inference/compute_universe.rs`). Anomaly = top-byte `0x99`.
- Already branches on the **`p64\0` / `.q42` magic** and has `P64TensorIndex::from_p64`.

### 3b. Compute backend — the device model (CRITICAL for Phase 1)
- **`gpu_context::shared_gpu()`** (`gpu_context.rs:672`, `OnceLock<SharedGpuContext>`) is the process-wide
  device: HighPerformance adapter, **buffer limits raised to the adapter max** (so >256 MiB weight tensors
  are legal), f16/subgroup/coopmat-capable, wired to the `VramLedger` + `UniverseOrchestrator`
  (U0 LlmInference / U1 Tensor10D / U2 Viewport / U3 Acoustic).
- **Inference + KV cache run on `shared_gpu()`** on native (`gguf_bridge/init.rs:13,25,50`).
- **The forge does NOT** — `ForgeGraphExecutor::new()` → `WgpuComputeContext::new()` requests its **own**
  device. **This is the Phase-1 keystone gap.** (topk_gpu/ternary_gpu/lora make their own small devices too,
  but they're secondary; the one that matters is forge ↔ shared_gpu.)
- **Invariants the swap MUST respect:** one shared device never recreated; **weights upload-once,
  GPU-resident**; the **U0 thread owns the GPU queue** (forge dispatch happens on the decode-loop thread;
  Sentinel stays on SPSC rings; `platform_scheduler::bind_inference_thread`); two-slab read/read_write model;
  one `submit_graph` per graph.

### 3c. q42 ↔ p64 substrate (the transcode is mostly DONE; the moat is the q42 graph)
- **p64 = the quantized-weight container** (`q42/p64_weight.rs`): `p64\0` magic, **role-tagged** tensors
  (`P64_ROLE_ATTN_Q/K/V/O`, `FFN_GATE/UP/DOWN`, norms, `TOKEN_EMBD`, `OUTPUT`), **page-aligned** blobs,
  per-layer 10-D manifold coords, **CRC-32C** (fail-closed), **embedded tokenizer**. Reader:
  **`P64TensorIndex::from_p64` (p64_weight.rs:1442)** — reuse it.
- **GGUF → p64 is DONE:** `compile_gguf_to_p64` (p64_weight.rs:266–483) — no re-quantization (Q4_K/Q8_0/F16
  byte-for-byte). Plus an **AWQ ternary-FFN** path `compile_gguf_to_q42_ffn_quant_awq` (~1.58-bit) — the
  memory-bandwidth differentiator.
- **q42 = the provenance/consent graph** (`q42/q42_volume.rs`, `lib.rs` NQuin): Merkle-DAG, ODRL sensitivity,
  the **natural-person vs software-agent DID bifurcation** in `Q42VolumeHeader`. A quin's `object` upper-4-bits
  `0b1001` = **p64 tensor pointer** (lower 60 bits = byte offset). **GAP: the q42 graph OVER the model weights
  (source-GGUF hash, transcode lineage, consent chain) is sketched, not integrated** — that's Phase 2's moat.
- The forge does **not consume p64 yet** (`evaluate_p64` is a placeholder).

### 3d. Forge-side gaps to run a real model (Phase 1 + 4)
`ForgeGraphExecutor::run()` re-uploads all externals per call (must become upload-once weight residency).
Missing builders: QKV projection, output projection, **multi-head** attention (needs axis-aware reduce; the
current Reduce is whole-tensor), **RoPE** (`Stencil::RopePair` op exists), embedding lookup, lm_head. Dequant:
only **Ternary** GatherDequant is wired; **Q4_K/Q8_0 missing** (`graph_ops/gather_dequant.rs` + executor arm;
`ggml_quants` has the CPU dequant to use as oracle).

---

## 4. The plan (4 phases) + the IMMEDIATE next step

Full detail in `docs/plans/llm-on-forge-q42-p64.md`. **Non-destructive**: build the forge path *alongside*
the working hand-written engine, use the engine as the **oracle + fallback**, flip the default only once the
forge path matches its outputs and beats its latency.

- **Phase 1 (task #60) — START HERE.** Device unification → weight residency → p64→forge bridge →
  multi-head/RoPE/QKV/lm_head builders → **layer bake-off** (one real SmolLM2-360M layer: forge vs
  hand-written, same p64 weights, certified within f32 tol, ms/layer reported).
- **Phase 2** — full forward + real generation wired into the decode loop behind a flag (preserve the
  Sentinel + validate_intent/output); **q42 provenance graph woven in** so the first real generation is
  auditable.
- **Phase 3** — tensor-core MatMul (`gemm_f32_tc`) + **AWQ ternary FFN** through the certified GatherDequant
  (the memory-bandwidth win).
- **Phase 4** — Q4_K/Q8_0 forge dequant (full GGUF coverage) + any-GPU portability.

**THE very next concrete action (Phase 1a):**
> Add `WgpuComputeContext::from_device(device: wgpu::Device, queue: wgpu::Queue, …)` (wgpu Device/Queue are
> cheap `Arc`-clones) that maps `gpu_context::shared_gpu()`'s `GpuAdapterCaps` → the forge's
> `AdapterConstraints`/`HardwareProfile`/`AdapterIdentity` and builds the forge slabs **on the shared
> device**; then add `ForgeGraphExecutor::with_context(ctx)`. Verify the existing GPU certs
> (`execute_graph_gpu_matches_cpu_oracle`, `p4b_graphs_gpu_match_cpu_oracle`) still pass when the executor is
> built via `with_context(shared_gpu()…)`. That is the unblock for everything else.
> Seams: `wgsl_forge/execute/wgpu.rs::WgpuComputeContext::new` (~45–158, replicate its slab/constraint tail);
> `wgsl_forge/graph_ops/executor.rs::ForgeGraphExecutor` (145–159); `gpu_context.rs::shared_gpu` (672) +
> `SharedGpuContext` struct + `GpuAdapterCaps`.

---

## 5. Open / tracked tasks

- **#60** LLM-on-forge Phase 1 (above) — the active workstream. *(Add #61–#63 for Phases 2–4 as you start them.)*
- **#57** wgpu: watch crates.io for a release carrying coopmat #9741 → drop the dormant gate, bump the pin.
- **#59** RT-core acceleration of 3-D Neighbor (optional speedup; needs wgpu procedural-AABB ray_query feasibility check).
- **#45** fluid_dynamics + quantum_bio formalization — **needs Timothy's physical-model direction** (curation).
- Curation/quality calls reserved for Timothy: the **AWQ-ternary FFN acceptable-quality threshold** (Phase 3);
  `Stencil::{Divergence,Advection}` velocity-field/model (same class as #45).

---

## 6. How to verify / run (A2000 present; CUDA available)

- Build: `cargo build -p qualia-core-db --lib` (add `--features cuda` for the WMMA path).
- Forge sweep (fast, non-GPU + probes): `cargo test -p qualia-core-db --lib wgsl_forge::`
- GPU certs (A2000): `cargo test -p qualia-core-db --lib wgsl_forge::graph_ops::executor -- --ignored --nocapture`
- Decode bench: the `decode_block_kernel_uplift_bench` (ignored) prints ms/call + ratio.
- The **bake-off** (Phase 1) is the new gate: forge layer == hand-written layer (the existing engine is the oracle).
- `cargo test` takes ONE positional filter; don't pass two. Windows: use the Bash tool (Git Bash) for POSIX,
  PowerShell for cmdlets. LF→CRLF git warnings are benign.

---

## 7. Working norms (how to engage Timothy — from `~/.claude/.../memory/`)

Plans go in **repo `docs/plans/*.md`**, never the hidden plan-mode file. **No `AskUserQuestion` survey forms** —
discuss options in prose and recommend. **Full ownership**: when he directs, it's in your hands, "pass, excel or
fail" — make the architectural calls yourself, don't hedge with lane-coordination caveats (he is the sole
allocator). Precision over sycophancy and over empty hedging. The human-rights mission is real; don't flatten it,
don't perform it back at him. Announce work in `coordination/NOTICES.md` (canonical
`C:\Projects\qualiaDB\coordination\NOTICES.md`) per AGENTS §10.

---

## 8. State

Branch `feature/p64-manifold-wal-eigensolver`, all pushed (latest: `93d50446`). Working tree clean except this
handover. The DAG-IR forge is done + certified; the LLM-on-forge work is **not started** — Phase 1a
(`from_device`) is the first edit. Go.
