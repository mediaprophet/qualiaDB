# Qualia WASM LLM Inference — Planning & Agent Task Specification

**Date:** 2026-06-19 · **Owner:** Qualia · **Companion doc:** [`WASM_LLM_INFERENCE_DIAGNOSIS.md`](WASM_LLM_INFERENCE_DIAGNOSIS.md)

This is the authoritative plan for getting Qualia's **own** WASM GGUF + WebGPU LLM pipeline
running in the browser (no wllama / no external LLM libs). It merges the user's task
specification with verified engineering findings from the code. **Read §0 (Critical
Findings) and §6 (Open Questions) before starting — they change the work.**

---

## 🛑 0. PRIME DIRECTIVES (DO NOT VIOLATE)

1. **Target gating:** all wasm fixes strictly behind `#[cfg(target_arch = "wasm32")]`.
2. **Native integrity:** do not modify native GPU frameworks (`TensorVolumeGpu`, the
   `#[cfg(not(wasm32))]` synchronous GPU readbacks, `gpu_context::shared_gpu`). Native must
   stay pristine and byte-for-byte unchanged.
3. **Zero-heap adherence:** no `Vec`/`String`/`Box` in the hot loop. LLM loading is the
   **only** permitted heap exception and must stay quarantined to the wasm LLM path. Use
   pre-allocated stack arrays or fixed-capacity resident buffers (e.g. `kv_cache_cpu:
   Box<[f32]>` allocated once at load — acceptable; per-call `vec!` in the decode loop — not).
4. **No external LLM libraries** (wllama / tvm / JS wrappers). Fix the native
   `gguf_bridge.rs` pipeline.

---

## 🔴 0b. CRITICAL FINDINGS (verified in code — these correct the spec)

**F1 — Attention had NO CPU fallback on wasm; only GEMM did.** *(Partially resolved MC1–MC2.)*
`dispatch_gemm_raw_into` (gguf_bridge.rs ~1230) falls through to `stack_gemm_quant`
(gguf_bridge.rs:294) on wasm — a real CPU GEMM. Historically `dispatch_attention_pass`
(gguf_bridge.rs:1508) used the `map_async`/`poll(Wait)`/`#[cfg(not(wasm32))]` readback
pattern with no CPU equivalent (F1 symptom → `"firehose"` garbage).
➡️ **MC1–MC2 fix:** `dispatch_attention_pass` now early-returns into `cpu_attention_pass`
(`#[cfg(wasm32)]`) before any GPU encode; GPU body is `#[cfg(not(wasm32))]` only.
`stack_gemm_quant` handles Q/K/V/output projections; `cpu_attention_pass` handles RoPE, KV
writes, and SDPA. **Caveat:** coherence is still blocked until prefill populates the KV
arena (see F5, F6).

**F2 — The raw OOB is probably NOT in `stack_gemm_quant`.**
`stack_gemm_quant` already guards `n_in > input.len() || n_out > out.len()`. Its only
unguarded access is `row[..n_in]` with `row = [0f32; MAX_STACK_GEMM_IN(=10240)]` — but
`n_in > 10240` would be a **Rust bounds panic** (visible via `init_panic_hook`), whereas
we observe a **raw `memory access out of bounds` trap** (no panic message). So the trap is
more likely a `bytemuck::cast_slice` over a too-short slice, a wgpu-wasm `write_buffer` /
`copy_buffer_to_buffer` with a size from a mis-parsed dimension, or an unchecked/SIMD
access — most plausibly inside `dispatch_attention_pass` / `dispatch_prefill_layer_batch`,
which run first (~10 ms). Guards are still worth adding, but instrument **attention first**.

**F3 — On wasm, GPU encode work runs then is thrown away.**
Even where a CPU fallback exists, the wasm path still creates buffers, `write_buffer`s, and
dispatches GPU work *before* the `#[cfg(not(wasm32))]` readback, then `unmap()`s a
never-resolved map. This is wasted work and a candidate OOB site. The wasm path should
short-circuit to CPU **before** touching GPU buffers (see Phase 2).

**F4 — Async dispatch variants.** *(Resolved MC5–MC8.)* `dispatch_gemm_into_async`,
`dispatch_attention_pass_async`, `dispatch_transformer_layer_async`,
`dispatch_transformer_forward_async`, `dispatch_prefill_chunk_async`, and
`dispatch_output_argmax_chunked_async` all exist and are wired to `inferWasmAsync`
(`wasm_llm.rs`). **MC8 pt3:** GPU prefill is the sole path
(`dispatch_prefill_chunk_async_mc8_gpu`; CPU fallback blocked). Argmax remains sync
(architect-gated).

**F5 — WASM init skipped KV/GEMM arena setup.** *(Discovered MC2 session; fix landed,
validation pending.)* `initialize_webgpu_engine` (wasm, `gguf_bridge.rs:3242`) previously
set only `engine.gguf_mmap = Some(data)` and **did not** call `adopt_resident_mmap`, which
is the sole path that invokes `ensure_kv_cache()` + `ensure_gemm_buffers()`. Without that,
`dispatch_attention_pass` guard-trips on `kv_cache_gpu.is_none()` / missing GEMM buffers →
prefill K/V never run → decode proceeds with an empty KV mirror → F1 garbage output even
after SDPA lands. **Fix:** init now calls `engine.adopt_resident_mmap(gguf_data)?`.

**F6 — Prefill failure was silently ignored.** *(Resolved MC8 pt3.)* `llm_agent.rs` used
`let _ = dispatch_prefill_chunk(...)` (return value discarded). **Fix landed:** log
`[llm] PREFILL chunk FAILED pos=… n=…` on failure + F5 `adopt_resident_mmap`. **MC8 pt3:**
GPU prefill succeeds (`[MC8] GPU prefill OK layers=32 n_tokens=4 start=0` in harness).
Remaining coherence gap is **depth accumulation / KV correctness**, not prefill dispatch
failure.

---

## ✅ STATUS: Already fixed & verified (do not redo)

| Fix | Detail |
|-----|--------|
| Init hang | `docs/js/webgpu-limits-shim.js` strips removed WebGPU limits; `gguf_bridge.rs::initialize_webgpu_engine` (wasm) uses `try_new().await?` not `new_async()`. Engine init verified (~440 ms / 258 MB). |
| Init OOB | `Cargo.toml` `wasm-opt = false` (was `-Oz --enable-bulk-memory`, miscompiled the model `to_vec` copy). |
| Memory layout | `scripts/package-qualia-wasm.ps1` RUSTFLAGS: `-zstack-size=8388608` + `--max-memory=4294967296` → binary `min=131p(8MB)`, `max=65536p(4GB)`. (Did not fix the inference trap → not a stack overflow.) |
| LoRA | `context_detector` + `adapter_manager` compile for wasm; `webgpu_lora` is native-only; CPU apply wired at `llm_agent.rs:1379`. |
| Harness | `docs/wasm-llm-test.html` (model dropdown, cache toggle, panic hook, on-page log). |
| Phase 1 OOB | Trap resolved; defensive guards in `dispatch_gemm_into` + `stack_gemm_quant` kept permanently. |
| Phase 2A MC1 | `cpu_attention_pass` K/V + RoPE + KV writes; Q stubbed; KV index math OOB-safe. |
| Phase 2A MC2 (code) | SDPA + softmax + V-sum in `cpu_attention_pass` proj_kind=0; not yet validated end-to-end. |
| WASM init arenas | `initialize_webgpu_engine` → `adopt_resident_mmap` (F5 fix). |
| GPU prefill (MC8 pt3) | `dispatch_prefill_chunk_async_mc8_gpu` — sole path; logs `GPU prefill OK`. |
| K/V weight-buffer flush (MC8 pt3c) | `mc8_flush` between K→V and gate→up; L31 272→21. |
| Prefill attn_input handoff (MC8 pt3d) | `prefill_scratch[t]` → `aux_buf` for Q; Q `abs_pos` explicit. |

---

## 🐞 PHASE 1 — Pinpoint & squash the OOB trap

**STATUS: ✅ COMPLETE** (2026-06-18 entry f). Inference runs end-to-end without trap; guards
are permanent. Diagnostic `wlog` scaffolding remains until Phase 2A validates coherent output.

**Goal:** get an exact faulting site for the `memory access out of bounds` trap.

1. **Instrument `dispatch_attention_pass` (PRIMARY, gguf_bridge.rs:1460)** — add
   `#[cfg(target_arch="wasm32")] web_sys::console::log_1` before each buffer
   `write_buffer`/`copy`/`slice` and each `bytemuck::cast_slice`, printing the computed
   byte length vs the target buffer capacity, and the parsed `n_head/n_kv/head_dim/n_embd`.
   Add `if off+len > buf_len { return false; }` guards around every slice/cast.
2. **Instrument `dispatch_prefill_layer_batch`** similarly (it runs first in prefill).
3. **`stack_gemm_quant` (gguf_bridge.rs:294):** add `if n_in > MAX_STACK_GEMM_IN || n_out >
   MAX_STACK_GEMM_OUT { log; return false; }` and log `n_in`/`n_out` (cheap; rules it in/out).
4. **`matmul_dims()`:** log parsed `(n_in, n_out)` per tensor vs the known SmolLM2-360M dims
   (hidden 960, FFN 2560, heads 15, kv-heads 5, head_dim 64) to catch a GGUF shape misparse.
5. Rebuild (wasm-opt off), run harness with `SmolLM2-360M-Instruct-Q4_K_M.gguf`, read the
   last log line before the trap → that's the faulting access. Fix the dimension/index math.

**Constraint:** all guards/logs `#[cfg(target_arch="wasm32")]`; native unchanged.

---

## 🌉 PHASE 2 — Resolve the sync/async WebGPU chasm (wasm compute path)

**Accepted reality:** wasm streaming cannot use synchronous WebGPU readback.

### Phase 2A — CPU attention stent (Option A, §6 Q1) — ✅ CLOSED

| Micro-commit | Scope | Status |
|--------------|-------|--------|
| **MC1** | K/V `stack_gemm_quant` projection, RoPE, `kv_cache_cpu` writes via `k_index`/`v_index`; Q readback stubbed zero | ✅ Verified (no OOB, no `[cpu_attn]` guard trips) |
| **MC2** | SDPA: GQA head map (`q_h / q_heads_per_kv`), scaled dot-product, stable softmax, V-weighted sum → `readback_out` → existing `attn_output` wo via `stack_gemm_quant` | 🟡 Code landed; **output still `"firehose"`** — prefill not populating KV (F5/F6) |
| **MC2b** | Fix prefill layer-0 failure (Q5_0); confirm `[MC2] SDPA L1` non-zero; capital-of-France coherent | ✅ Closed — prefill/SDPA/TTFT validated; EOS spam → MC3 |
| **MC3** | **Logit & alignment resolution:** NaN probes at argmax + SDPA + hidden; trace poison layer | ✅ Closed — NaN→argmax-0 confirmed; fix = RMSNorm (MC3b) |
| **MC3b** | **Pre-Norm RMSNorm (CPU):** `attn_norm` / `ffn_norm` / `output_norm`; stack scratch; prefill K/V norm | ✅ Closed — `contains_nan=false`; real token IDs; garbled text → MC3c |
| **MC3c** | **SwiGLU SiLU:** `silu(gate) ⊗ up` with stack `gate_buf`/`up_buf`; wasm32-only | ✅ Closed — L0 variance fixed; logits finite; English still garbled → MC3d |
| **MC3d** | **Coherence tail:** `rope_theta` from GGUF; BPE `Ġ` decode; KV `token_idx` probe | ✅ Closed — rope=100k; no `Ġ` leak; fragmentary English → MC3e |
| **MC3e** | **Coherence polish:** BOS prepend, NEOX RoPE audit, full-vocab argmax probe | ✅ Closed — BOS/RoPE/argmax verified; output still fragmentary → MC3f |
| **MC3f** | **Tokenizer/BPE alignment:** smollm pretoken + BPE merges + special tokens; naked baseline | ✅ Closed — HF parity (22/5 IDs); output still garbled → MC3g |
| **MC3g** | **Inference math:** output.weight tie, GQA audit, Q8_0 vs Q4_K_M dequant isolation | ✅ Closed — tie OK; Q6_K dequant bug isolated → MC3h |
| **MC3h** | **K-Quant dequant fix:** `dequant_q6_k` signed `int8` scales (`blk.*.ffn_down` Q6_K) | ✅ Closed — naked ` Paris.`; ChatML `The capital of France` |
| **MC4** | Trim temporary `wlog` instrumentation; commit `0.0.18` CPU fallback checkpoint | ✅ Closed — `v0.0.18-wasm-cpu-fallback-stable` |

### Phase 2B — Async WebGPU compute (Option B) — ✅ GATE CLOSED (Part 3y, 2026-06-19: TTFT ~3957 ms, `Paris.`)

| Micro-commit | Scope | Status |
|--------------|-------|--------|
| **MC5** | `dispatch_attention_pass_async` + `dispatch_transformer_forward_async` plumbing | ✅ Closed — wired via `inferWasmAsync` (`wasm_llm.rs`) |
| **MC6** | `inferWasmAsync` + JS `Promise` bridge; `_async` dispatch loop | ✅ Closed — naked ` Paris.` TTFT ~9s (vs ~47s CPU) |
| **MC7** | WGSL `Q5_0`/`Q8_0` dequant; gate removal; full GPU offload | ✅ Closed — `Paris is the capital of France`; TTFT ~11s |
| **MC8** | Pipeline fusing + decode super-arena (3w) + resident weights (3x) + eager upload (3y) | ✅ **Closed (Part 3y)** — `Paris.` ✅; **TTFT ~3957 ms (gate &lt;4500 ms ✅)**; throughput ~0.6 tok/s — bottleneck since traced to **per-token logits re-upload**, NOT layer compute (Phase 5, log ar–au) |

### Phase 5 — Decode throughput (post-gate) — 🔎 ROOT CAUSE FOUND, fix pending

| Step | Scope | Status |
|------|-------|--------|
| **5.0 Dispatch fusion** | gate+up+SiLU → 1 pass; modular Rust-composed WGSL; Phase-6 const seam | ✅ Landed (ar) — coherent, **throughput-neutral** ⇒ not dispatch-bound. Not committed |
| **5.2 Block-amortized dequant** | decode Q5_0 `d`+`qh` once/32-block in fused FFN | ✅ Landed (as) — coherent, **throughput-neutral** ⇒ not dequant-bound |
| **5.x Bisect** | neuter gate/up to 1/30 work | ✅ (at) — **0 change** ⇒ FFN GEMM compute ≈ 0% of per-token time |
| **5.3 Resident output projection** | upload ~50 MB tied `token_embd` (Q8_0) **once**; resident chunked argmax, no per-token re-upload | ⏳ **NEXT** — the actual lever (au); proven Phase 2B/3x pattern |

#### MC8 — split delivery

**Part 1 (✅ infrastructure — committed `chore(wasm-llm): MC8 pt1`)**

WebGPU validation plumbing is complete; fused compute is **gated off** until Part 2 numerical audit passes.

| Component | Detail |
|-----------|--------|
| `wasm_elementwise.wgsl` | GPU RMSNorm (`rms_norm_batch`), SiLU×mul (`silu_mul_main`), residual add (`add_residual_main`) |
| `add_residual_main` bind fix | Uses `buf_a + buf_b` (not `buf_out + buf_b`) so binding 0 is not stripped by WGSL dead-code elimination |
| `gemm_ffn_buf` | Dedicated SwiGLU up-projection scratch — eliminates in-place gate/up GEMM on same buffer |
| `prefill_scratch_buf` | Batch-sized RMS output — avoids in-place `batch_buf` read+write in prefill encoder |
| `mc8_flush()` | Submits encoder between GEMM writes and elementwise reads (WebGPU sync-scope rule) |
| `encode_residual_add_gpu` | Disjoint storage bindings: add into scratch, `copy_buffer_to_buffer` into dst |
| `encode_transformer_layer_gpu` | Full fused layer encoder (present, not wired to hot path) |
| `dispatch_prefill_chunk_async_mc8_gpu` | GPU batched prefill — **hot path** (pt3; CPU fallback blocked) |
| `dispatch_output_argmax_chunked_async_mc8_fused` | Per-chunk async vocab GEMM + CPU argmax — **hot path** (Part 3l; no WGSL argmax shader) |
| `prefill_work_buf` | Strided per-token Q/FFN scratch (`PREFILL_CHUNK_SIZE` rows; disjoint GEMM in/out) |
| `encode_prefill_q_ffn_tail_fused` | Batched Q+FFN tail — one `mc8_flush` per stage (**landed, not hot-path**) |
| `encode_*_offset` helpers | `mc8_buf_slice`, `encode_gemm_bufs_offset`, `encode_elem_offset`, `encode_residual_add_offset` |
| `mc8_flush()` between K→V / gate→up | Prevents `gemm_weight_buf` queue race (pt3c) |

**Current hot-path routing (post Part 3 — GPU manifold unified):**

- Prefill → `dispatch_prefill_chunk_async_mc8_gpu` (GPU batched K/V + per-token Q/FFN; **no CPU fallback**)
- Decode forward → `dispatch_transformer_forward_async` → `encode_transformer_layer_gpu` (fused GPU: attn + `wasm_elementwise` RMSNorm/SiLU/residual)
- Argmax → `dispatch_output_argmax_chunked_async` → `_mc8_fused` (per-chunk `dispatch_gemm_raw_into_async` + CPU argmax)

**Part 2 (✅ RoPE alignment + fused decode — landed `feat(wasm-llm): MC8 pt2`)**

Architect-approved WGSL manifold alignment executed:

| Fix | Detail | Status |
|-----|--------|--------|
| **P0 RoPE pair layout** | Replaced `rotate_rope_pair` (consecutive `(p*2,p*2+1)`) with `apply_rope_neox` (split-half `(i, i+half)`) in `fused_attention.wgsl` | ✅ |
| **P0 RoPE freq base** | `AttentionGpuParams.rope_theta_base` ← `h.effective_rope_freq_base()` (100k SmolLM2); was hardcoded 10k | ✅ |
| **P1 RoPE scale** | Added `rope_scale: f32` uniform; WGSL uses `pos / rope_scale` | ✅ |
| **Fused decode** | `dispatch_transformer_forward_async` wired to `encode_transformer_layer_gpu` + per-layer `mc8_flush` + single readback | ✅ |
| **P2 RMSNorm / SiLU** | Formulas unchanged; L0 probe **inconclusive** — see below | 🟡 |

**L0 variance probe (SmolLM2-360M naked, fused GPU decode, 32 steps):**

Probe read `h[0]` after layer-0 post-FFN via `pipeline_read_hidden`. Target from MC3c CPU path: **~1.09**.

| Decode step | `h[0]` | Notes |
|-------------|--------|-------|
| 0 | -1.337 | Miss — likely first-token / prefill boundary |
| 1 | **0.930** | Within ~15% of target |
| 2–10 | 2.88 → 3.05 | Drift high — elementwise or residual routing suspect |
| 11+ | mixed (-1.5 … 1.6) | Unstable across context growth |

**Harness post-Part 2 (naked `The capital of France is`, Q4_K_M, `inferWasmAsync`):**

| Metric | MC7 (CPU elem) | MC8 pt2 (fused GPU) |
|--------|----------------|---------------------|
| TTFT | ~11 s | **~9 s** ✅ (8961 ms) |
| WebGPU errors | 0 | **0** ✅ |
| Output | `Paris is the capital of France.` | **garbled repetition** (`KeyNotKeyNot…`) ❌ |
| L0 `h[0]` @ step 1 | ~1.09 (CPU, MC3c) | **0.930** (close, not locked) |

**Diagnosis for advisors:** RoPE geometric divergence is **closed** (P0/P1). Remaining incoherence points to **(a)** CPU-prefill K/V vs GPU-decode Q manifold split, **(b)** GPU elementwise / `base_save` residual routing (P2), or **(c)** partial layer fusion without full prefill GPU parity. MC8 Part 3 should run **CPU vs GPU L0 diff on decode step 1 only** (after prefill settles) before widening scope.

**Build trap (discovered Part 2):** wasm-pack **without** `-zstack-size=8388608` causes immediate `memory access out of bounds` at inference start (1 ms trap). Always build via `scripts/package-qualia-wasm.ps1` or explicit `RUSTFLAGS` in §BUILD/DEPLOY/TEST. Rebuilt binaries without 8 MB stack are **not** comparable to committed artifacts.

**Part 3–3h (✅ KV+embed locked; 🔴 L1 prefill hidden divergence)**

**Current harness snapshot (naked SmolLM2-360M Q4_K_M, `inferWasmAsync`, decode step 1):**

| Metric | MC7 reference | MC8 current (pt3d) |
|--------|---------------|-------------------|
| TTFT | ~11 s | **~7.7–8.2 s** |
| Output | `Paris is the capital of France.` | partial English, repetitive (`starlings soar…`) |
| L0 `h[0]` | ~1.09 (CPU) | **0.423** (true GPU; pt3 `1.011` was false positive) |
| L1 `h[0]` | — | 0.098 |
| L31 `h[0]` | ~1.09 band | **20.87** (↓ from 271.7 after pt3c flush fix) |
| WebGPU errors | 0 | 0 |

#### Architect answers (§8 feedback — 2026-06-18)

| Q | Decision |
|---|----------|
| **A** Prefill split | **Mandate full GPU prefill** — no CPU/GPU KV boundary |
| **B** L0 gate | **Decode step 1 only** (not step 0 prefill/decode boundary) |
| **C** Priority | **GPU prefill K/V first**, then elementwise audit |
| **D** Argmax fuse | **Gated** — keep sync `dispatch_output_argmax_chunked` |

#### Part 3 results (naked SmolLM2-360M, `inferWasmAsync`)

| Checkpoint | `h[0]` | Status |
|------------|--------|--------|
| GPU prefill | `[MC8] GPU prefill OK layers=32 n_tokens=4` | ✅ |
| L0 @ layer 0 (step 1) | **1.011** | ⚠️ false positive (wrong-K residual mask; true L0=**0.423** post-pt3c) |
| L0 @ layer 31 (step 1, post-full forward) | **271.7** | ❌ depth blow-up (→ **20.87** post-pt3c) |
| TTFT | **7942 ms** | ✅ (↓ from ~9s pt2) |
| Output | `pries underm` repetition | ❌ |

**Diagnosis:** Manifold unification fixed the step-1 layer-0 variance gate. Incoherence is now a **depth accumulation fault** — layers 1–31 amplify hidden state (~1.01 → ~272). Suspects: per-layer GPU attention over unified KV, `wasm_elementwise.wgsl` residual routing, or missing `mc8_flush` between GEMM/elem at depth > 0.

#### Part 3b — depth bisect + FFN/residual routing audit (2026-06-19)

**Architect answer E:** audit **FFN elementwise chain first** (attention softmax bounds KV drift; unnormalized FFN + residual compounds).

**Code changes (`gguf_bridge.rs`):**
- `encode_residual_add_gpu` — explicit `scratch` buffer (no longer aliases `gemm_ffn_buf`).
- Attn residual scratch → `prefill_scratch_buf` / `gemm_aux_buf`; FFN `base_save` → `prefill_scratch_buf` only.
- Extra `mc8_flush()` after `base_save` copy, FFN RMSNorm, final FFN residual add.
- FFN residual scratch → `aux_buf` (post-`ffn_down`, post-flush).
- Depth bisect probe logs `h[0]` @ layers 0–3 + post-L31 on decode step 1.

**Elementwise audit:**
- `wasm_elementwise.wgsl::add_residual_main` confirmed: `buf_out[i] = buf_a[i] + buf_b[i]` (assignment, not `+=`).

**Harness (`inferWasmAsync`, naked SmolLM2-360M, step 1):**

| Layer | `h[0]` | vs ~1.09 target |
|-------|--------|-----------------|
| L0 | **1.011** | ✅ (~7%) |
| L1 | **-1.662** | ❌ **~2.5× magnitude flip** |
| L2 | **-7.337** | ❌ amplifying |
| L3 | **-0.595** | ❌ oscillating (not monotonic creep) |
| L31 | **271.7** | ❌ unchanged from pt3 |

TTFT **7533 ms**; output still `pries underm` repetition.

**Diagnosis:** L0 lock proves RoPE, `AttentionGpuParams`, and single-layer elemwise are sound. The **L1 jump** (1.01 → -1.66 in one layer) indicates **cross-layer buffer pollution or layer-1 block fault**, not slow FFN bias creep. FFN scratch isolation + flush barriers **did not move** post-L31 variance — leak is elsewhere.

#### Part 3c — KV layer indexing audit (2026-06-19)

**Architect answer F:** audit **KV cache layout & layer indexing first** (not full CPU/GPU tensor diff).

**Audit findings:**
- **Rust uniform loop:** `encode_attention_pass_gpu` rebuilds `AttentionGpuParams` with `layer_idx: layer` and `layer_offset = layer * layer_stride` per dispatch — ✅ correct.
- **WGSL index math:** `k_cache_idx` / `v_cache_idx` are layer-local (bind group maps one layer slice); matches CPU `k_index`/`v_index` relative to layer base — ✅ structurally sound.
- **Hidden ping-pong:** L0 probe @ 1.011 proves `gemm_input_buf` carries L0 output into L1 — ✅ not reading raw embedding.
- **Root cause (weight-buffer race):** K and V passes (and gate/up FFN) shared `gemm_weight_buf` in one command encoder **without** `mc8_flush` between them. WebGPU applies all `write_buffer` uploads before the encoder runs, so the **K dispatch executed with V weights** — corrupting KV cache for every layer. L0 `h[0]` stayed ~1.09 via residual dominance; L1+ attention over multi-token KV exploded.

**Fix (pt3c):** `mc8_flush()` between K→V attention passes (`encode_transformer_layer_gpu`, `dispatch_prefill_chunk_async_mc8_gpu`) and between gate→up FFN GEMMs (`encode_attn_ffn_tail_gpu`).

**Harness verify (post K/V + gate/up flush fix):**

| Layer | pt3b `h[0]` | pt3c `h[0]` |
|-------|-------------|-------------|
| L0 | 1.011 | 0.423 |
| L1 | -1.662 | 0.098 |
| L2 | -7.337 | -0.736 |
| L3 | -0.595 | -1.080 |
| L31 | **271.7** | **20.87** |

TTFT 8212 ms. Output shifted from `pries underm` repetition → partial English (`starlings soar…`) but still incoherent vs MC7 `Paris.`

**Interpretation:** L31 variance dropped **13×** (272→21) — confirms K/V `gemm_weight_buf` + uniform race was the dominant depth amplifier. L0 no longer accidentally matches ~1.09 (residual-dominated wrong-K regime); geometry not yet at CPU parity.

#### Part 3d — batched prefill audit (2026-06-19)

**Architect answer (pt3c verify):** **prefill batched-attn audit before Q/K diff** — corrupted KV geometry makes tensor diff useless.

**WGSL audit answers:**

| Shader / path | Per-token `pos`? | Verdict |
|---------------|------------------|---------|
| `fused_attention.wgsl` K/V prefill (`proj_kind` 1\|2) | `abs_pos = batch_start_token_idx + token_in_batch` via `wg_id.x / n_kv_head` | ✅ already per-row |
| `fused_attention.wgsl` Q decode (`proj_kind` 0) | Was `params.token_idx`; updated to `batch_start_token_idx + token_in_batch` | ✅ explicit (decode: batch=1) |
| `wasm_elementwise.wgsl` RMSNorm | `wg_id.x` = row; `ss` summed over `base..base+n` only | ✅ per-row variance |

**Not the architect's hypothesized flat-RoPE bug** for batched K/V — position already derives from workgroup index, not the singular `token_idx` uniform.

**Real prefill parity gap:** `encode_attn_ffn_tail_gpu(..., attn_input: None)` re-RMSNorm'd per token while K/V consumed batch `prefill_scratch` — redundant and flush-sensitive.

**Fixes (pt3d):**
1. `mc8_flush` after batch RMSNorm before K/V prefill; after decode-layer RMSNorm before K.
2. Prefill Q handoff: copy `prefill_scratch[t]` → `aux_buf`, pass `Some(aux_buf)` into tail (K/V/Q share identical normed input).
3. `online_softmax_attention`: RoPE + causal window keyed on `batch_start_token_idx + token_in_batch`.

**Harness verify (pt3d, same as pt3c):** L0=0.423, L1=0.098, L2=-0.736, L3=-1.080, L31=20.87 — **unchanged** (confirms batched K/V RoPE was already correct; `attn_input` handoff is parity hygiene, not the L31 gap).

#### Part 3e — L0 targeted diff: Q/K/Attn_Out @ layer 0 (2026-06-19)

**Architect answer H:** **Neither L1 nor prefill causal window first** — prioritize **Q/K/V/SDPA diff @ layer 0**. L0 `h[0]` drops to **0.423** (vs CPU ~1.09) before L1; the L31 explosion is downstream amplification of that collapse.

**Probe (`mc8_log_l0_attention_diff`):** decode step 1, `token_idx=5`, layer 0, after Q SDPA (before `o_proj`).

| Tensor | CPU `h[0]` | GPU `h[0]` | max_abs_err | Verdict |
|--------|------------|------------|-------------|---------|
| `attn_rmsnorm` | 0.000452 | 0.000452 | 0.000000 | ✅ match — `attn_norm.weight` routing correct (not `ffn_norm`) |
| `K_rope` (slot cur) | -0.712626 | -0.712626 | 0.000000 | ✅ match — K GEMM + RoPE + KV write correct |
| `Q_rope` h0 | 0.751404 | — | — | CPU only (Q lives inside shader pre-SDPA) |
| `Attn_Out` h0 (SDPA) | -0.002785 | -0.002785 | 0.000000 | ✅ match — `online_softmax_attention` + causal window OK |
| `mask_active` | — | 0 | — | dense path (6 tokens); **not** a causal-mask failure |

**`first_divergence(0.01)=none`** for attention tensors.

**Probe artefact (fixed):** initial run reported `K_rope` divergence (`gpu[0]=0.000452` = stale staging buffer) because `StaticKvCacheArena` lacked `BufferUsage::COPY_SRC`. WebGPU validation failed on `pipeline_read_kv_head`; readback returned prior `attn_rmsnorm` bytes. Fix: add `COPY_SRC` to KV arena creation.

**Interpretation:** L0 attention stack (RMSNorm → K/V write → Q GEMM/RoPE → online softmax) is **bit-exact vs CPU** on decode step 1. The **0.423 vs ~1.09** depth-bisect gap is **not** inside SDPA or causal masking — it accumulates **after** `Attn_Out`, in **`o_proj`**, **attn residual add**, or **FFN tail** (post-L0 full layer still reads `h[0]=0.423`).

**Harness verify (pt3e, same build):** L0=0.423, L1=0.098, L31=20.87, TTFT ~7.6s, partial English output — depth bisect unchanged (attention was already correct; post-attn path still leaks variance).

#### Part 3f — L0 mid-layer diff: o_proj + attn-residual + ffn_norm (2026-06-19)

**Architect answer (pt3f):** **`o_proj` + attn-residual first** — never audit FFN until attention block cleared.

**Probe (`mc8_log_l0_midlayer_diff`):** decode step 1, layer 0, three phases hooked into `encode_attn_ffn_tail_gpu`.

| Phase | CPU `h[0]` | GPU `h[0]` | max_abs_err | Verdict |
|-------|------------|------------|-------------|---------|
| `pristine_hidden_pre_residual` | 0.005753 | 0.005753 | 0.000000 | ✅ raw embedding intact (not normalized residual) |
| `o_proj` | -0.102919 | -0.102919 | 0.000001 | ✅ bit-exact |
| `post_attn_residual` | -0.097166 | -0.097166 | 0.000001 | ✅ `hidden + o_proj` correct |
| `ffn_norm` | -0.057536 | -0.057536 | 0.000000 | ✅ `ffn_norm.weight` routing correct (not `attn_norm`) |

**`first_divergence(0.01)=none`** for all three phases.

**Interpretation:** The **0.423 vs ~1.09** depth-bisect gap is **not** in o_proj, pristine-residual routing, or ffn_norm. Variance collapse happens **after `ffn_norm`**, inside the **gate/up/SiLU/down chain** or the **FFN residual add** (`base_save + down → token_hidden`). Depth bisect L0 `h[0]=0.423` unchanged; L1 `h[0]=0.098` ≈ magnitude of post-attn residual (sign differs — layer-1 geometry).

**Harness verify (pt3f):** L0=0.423, L1=0.098, L31=20.87, TTFT ~8s — geometry unchanged; fault localized to FFN tail.

#### Part 3g — FFN chain diff: gate / up / SwiGLU / down / residual (2026-06-19)

**Architect answer (pt3g):** **Flush/alias audit first, then tensor diff.**

**60-second audit (`encode_attn_ffn_tail_gpu`):**

| Check | Verdict |
|-------|---------|
| `mc8_flush` after gate GEMM, before up | ✅ present |
| `mc8_flush` after up GEMM, before `silu_mul_main` | ✅ present |
| gate output → `work_buf`, up output → `ffn_buf` (disjoint) | ✅ no alias |
| `silu_mul`: `buf_a=work_buf` (gate), `buf_b=ffn_buf` (up), out → `aux_buf` | ✅ correct |
| `base_save` copied **before** `ffn_norm` (captures post-attn, not normed) | ✅ timing correct |

**Probe (`mc8_log_l0_ffn_chain_diff`):** decode step 1, layer 0.

| Stage | CPU `h[0]` | GPU `h[0]` | max_abs_err | Verdict |
|-------|------------|------------|-------------|---------|
| `base_save_post_attn` | -0.097166 | -0.097166 | 0.000000 | ✅ pristine FFN skip |
| `gate_out` | -3.880136 | -3.880135 | 0.000004 | ✅ bit-exact |
| `up_out` | -0.014292 | -0.014292 | 0.000004 | ✅ bit-exact |
| `swiglu_out` | 0.001122 | 0.001122 | 0.000001 | ✅ SiLU×up correct |
| `down_out` | 0.520334 | 0.520334 | 0.000015 | ✅ bit-exact |
| `ffn_residual` | **0.423168** | **0.423168** | 0.000015 | ✅ matches depth bisect L0 |

**`first_divergence(0.01)=none`** for all FFN stages.

**Watershed finding:** There is **no FFN tensor leak at L0**. The GPU FFN chain is bit-exact vs CPU replay (using GPU KV for SDPA). **`h[0]=0.423` is the correct unified-manifold L0 exit** — it equals `post_attn(-0.097) + down(0.520) = 0.423`. The historical **~1.09 MC7 CPU reference** is a **cross-manifold baseline mismatch**, not a within-layer GPU math fault. L31 `h[0]=20.87` remains the coherence blocker (depth amplification from L0=0.423, not L0=1.09).

**Harness verify (pt3g):** L0=0.423, L1=0.098, L31=20.87, TTFT ~8.2s — geometry unchanged; L0 pipeline internally consistent.

#### Part 3h — prefill / embedding input reconciliation (2026-06-19)

**Architect answer H:** **Prefill/embedding reconciliation first** — not L1–L31 bisect. Break the replay trap: compare pure CPU prefill KV vs GPU batched prefill KV (not CPU replay on GPU KV).

**Probes (`mc8_log_prefill_reconciliation`, `mc8_log_decode_embedding_probe`):**

| Input | CPU `h[0]` | GPU `h[0]` | max_abs_err | Verdict |
|-------|------------|------------|-------------|---------|
| `token_embd` @ decode step 1 | 0.005753 | 0.005753 | 0.000000 | ✅ embedding lookup correct |
| `token_embd_gpu_upload` | 0.005753 | 0.005753 | 0.000000 | ✅ upload path correct |
| `K_L0` @ prefill pos 3 (pure CPU vs GPU) | -0.208442 | -0.208442 | 0.000000 | ✅ **replay trap broken** — KV not poisoned |
| `V_L0` @ prefill pos 3 | 0.002047 | 0.002047 | 0.000000 | ✅ bit-exact |
| `L1_input_hidden` token 1 after L0 prefill | **0.179094** | **-0.529114** | **15.363** | ❌ **FIRST DIVERGENCE** |

**`first_divergence(0.01)=L1_input_hidden`** (KV and embedding: none).

**Interpretation:** Batched K/V writes and token embeddings are **correct**. The poison is in the **GPU prefill per-token Q+FFN tail** after batched K/V — the hidden state entering **layer 1** for token 1 is wrong on GPU while CPU sequential prefill produces `0.179`. This corrupts layers 1–31 for all prefill tokens during the batched pass, explaining L31=20.87 despite bit-exact decode-step L0 kernels. Suspects: batched prefill `encode_attn_ffn_tail_gpu` sequencing, `attn_input` row handoff, or per-token `batch_buf`/`token_buf` copy offsets in `dispatch_prefill_chunk_async_mc8_gpu`.

**Harness verify (pt3h):** L0=0.423, L31=20.87 unchanged; TTFT ~12.8s (CPU prefill replay adds ~4s probe cost).

#### Part 3i — Buffer slicing & loop flush audit ✅

**Architect answer (pt3i):** Prioritize **`batch_buf`/`token_buf` copy-offset and flush audit** — not per-token tensor math diff (pt3e–3g proved kernels bit-exact).

**Audit findings (`dispatch_prefill_chunk_async_mc8_gpu`):**

| Check | Result |
|-------|--------|
| `batch_buf`→`token_buf` offset `t * n_embd * 4` | ✅ correct |
| `prefill_scratch[t]`→`aux_buf` offset | ✅ correct (`t * n_embd * 4`) |
| `token_buf`→`batch_buf` writeback offset | ✅ correct |
| `mc8_flush` after `encode_attn_ffn_tail_gpu` | ✅ present |
| `mc8_flush` after per-token writeback | ❌ **missing** — patched |
| **`token_hidden` / `work_buf` alias in prefill tail** | ❌ **root cause** — both `gemm_output_buf`; o_proj clobbered pristine residual base |

**Root cause:** Decode path keeps `hidden_buf` (`gemm_input_buf`) separate from `work_buf` (`gemm_output_buf`), so o_proj preserves the embedding for post-attn residual. Prefill per-token loop routes both through `token_buf` (`gemm_output_buf`), so o_proj overwrote the residual base before `encode_residual_add_gpu` — pt3e–3g diffs missed this because probes run on the **decode** path (non-aliased buffers).

**Fix (pt3i):**
1. Pass `work_aliases_hidden: true` from prefill loop (`encode_attn_ffn_tail_gpu`): snapshot embedding to `aux_buf` before o_proj; use `aux_buf` as post-attn residual base (decode path passes `false` — `hidden_buf` ≠ `work_buf`).
2. `mc8_flush()` after each `token_buf`→`batch_buf` writeback in the `t` loop.

**Harness verify (pt3i):** `L1_input_hidden` **bit-exact** (cpu/gpu `0.179094`, err `0.000017`); `first_divergence(0.01)=none`; L31 `h[0]=0.841` (was `20.87`); naked output **coherent** (`Paris. The capital of France…`); TTFT ~13.3s on **`inferWasmAsync`** at time of pt3i session. *(Endgame 2026-06-19: same fix re-landed; see Part 3j — `WASM_ASYNC=1` regressed due to fused argmax; Part 3k CPU argmax gate restores `Paris.`.)*

#### Part 3j — Endgame: TTFT + Argmax fusion (2026-06-19) 🟡

**Architect decision:** Prioritize TTFT (~13s → ~4s) before argmax; then fuse argmax. **Outcome:** TTFT improved; coherence gate **not met** on `inferWasmAsync`.

**Landings (`gguf_bridge.rs` / `wasm_llm.rs`):**

| Change | Status |
|--------|--------|
| `work_aliases_hidden: true` in prefill `encode_attn_ffn_tail_gpu` | ✅ snapshot `token_hidden` → `aux_buf` before `o_proj` |
| Attn-residual scratch **must not** alias `prefill_scratch_buf` | ✅ uses `ffn_buf` (batched RMSNorm rows live in `prefill_scratch`) |
| `mc8_log_*` diagnostic probes removed | ✅ ~26k lines stripped |
| `dispatch_transformer_forward_async` probe params removed | ✅ |
| GPU argmax fusion | ✅ `dispatch_output_argmax_chunked_async` → `_mc8_fused` |
| Strided TTFT infrastructure | ✅ `prefill_work_buf`, offset encoders, `encode_prefill_q_ffn_tail_fused` |
| Fused prefill hot-path | ❌ disabled — WebGPU validation error when in-place GEMM on `PrefillBatchWork` |

**Harness matrix (naked `The capital of France is`, Q4_K_M, `agent-tools/wasm-mc2-test.mjs`):**

| Path | Env | TTFT | Output |
|------|-----|------|--------|
| `inferWasmStreaming` | `WASM_ASYNC=0` | **~8.8s** | **`Paris. The capital of France…`** ✅ |
| `inferWasmAsync` | `WASM_ASYNC=1` | **~7.6s** (was ~13.3s) | `prolesİİ…nownow…` ❌ |

**Bisect findings:**

- CPU prefill + GPU decode (`inferWasmAsync` decode only) → still garbled → **likely still used fused GPU argmax** (Part 3k: decode manifold innocent).
- Full CPU stack (`dispatch_prefill_chunk` + `dispatch_transformer_layer`) → **Paris** ✅.
- Removing per-token `mc8_flush` in prefill `t` loop without strided scratch → races on shared `token_buf` → garbled.

**Active blocker (pre-3k):** Assumed GPU async manifold parity — **resolved in Part 3k** (fused argmax was the sole regression).

**Next (Part 3j → superseded by 3k):**

1. ~~GPU async coherence~~ → **Part 3k:** CPU argmax gate restores `Paris.` on `WASM_ASYNC=1`.
2. Enable `encode_prefill_q_ffn_tail_fused` with **disjoint** GEMM in/out buffers (ping-pong `prefill_work_buf_A`/`_B`).
3. Target TTFT ~4s once fused argmax also coherent.
4. MC7 ChatML regression after Phase 2B gate.

**Phase 2B status:** **NOT CLOSED** — requires `WASM_ASYNC=1` naked `Paris.` + TTFT &lt;4s (Part 3o: TTFT ~6s ✅; coherence bisect open).

#### Part 3k — Isolation: Argmax gate (2026-06-19) ✅ coherence restored

**Architect directive:** Gate fused GPU argmax to CPU fallback; run `WASM_ASYNC=1` harness; do **not** attempt TTFT ping-pong until `Paris.` returns.

**Action:** `dispatch_output_argmax_chunked_async` reverted to synchronous CPU `dispatch_output_argmax_chunked` (not `_mc8_fused`).

**Harness verify (`agent-tools/wasm-mc2-test.mjs`, naked `The capital of France is`, Q4_K_M):**

| Path | Env | TTFT | Output |
|------|-----|------|--------|
| `inferWasmAsync` | `WASM_ASYNC=1` (pre-3k) | ~7.6s | `prolesİİ…nownow…` ❌ |
| `inferWasmAsync` | `WASM_ASYNC=1` (post-3k, CPU argmax) | **~7.9s** | **`Paris. The capital of France…`** ✅ |

**Interpretation:**

- GPU decode manifold (`encode_transformer_layer_gpu` + GPU prefill) is **coherent** — not the regression source.
- Regression was in batched `_mc8_fused` vocab GEMM (not decode); Part 3l root-caused as `gemm_weight_buf` queue-write race (no WGSL argmax shader exists).
- **`ffn_buf` residual routing:** not implicated — no audit needed while CPU argmax restores coherence.
- Endgame bisect ("CPU prefill + GPU decode → still garbled") likely still used fused GPU argmax on decode steps.

**Next (Part 3k → superseded by 3l):** see Part 3l below.

#### Part 3l — Argmax audit (2026-06-19) ✅ fixed

**Architect directive:** Audit WGSL argmax for OOB guards, `-INFINITY` padding, `workgroupBarrier()`, 256-byte readback alignment, chunk offset math; re-enable `_mc8_fused`.

**Critical finding: there is no `fused_argmax.wgsl`.** `_mc8_fused` never ran a GPU parallel reduction. It batched multiple vocab-chunk GEMMs into one `CommandEncoder`, copied logits into `gemm_output_staging`, mapped once, then ran **CPU** `update_streaming_argmax_sieved`.

| Audit item | Result |
|------------|--------|
| WGSL argmax OOB / barriers | **N/A** — no argmax shader in tree |
| `-INFINITY` padding | CPU argmax path already uses `update_streaming_argmax_sieved` (sieve → `-∞`) |
| 256-byte readback alignment | `copy_buffer_to_buffer` linear staging — no row-pitch padding; not the failure mode |
| Chunk offset math | `(chunk_idx * VOCAB_CHUNK_ROWS) + local` — correct in CPU argmax helper |
| **Root cause** | Batched `encode_gemm_bufs` loop: `queue.write_buffer` on shared `gemm_weight_buf` races across chunks in one submit scope (same class as pt3c K/V race). `mc8_flush` between chunks **insufficient** — batched single-readback still garbled. |
| **Part 3k CPU gate nuance** | `dispatch_output_argmax_chunked` on wasm32 falls through to **`stack_gemm_quant` (CPU)** — not GPU GEMM. |

**Fix:** Rewrite `_mc8_fused` to call `dispatch_gemm_raw_into_async` per chunk (submit + readback each chunk), then streaming CPU argmax — same semantics as sync path, keeps GPU vocab projection.

**Harness verify (`WASM_ASYNC=1`, naked, Q4_K_M):**

| Path | TTFT | Output |
|------|------|--------|
| Batched `_mc8_fused` (pre-3l) | ~8.0s | `prolesİİ…nownow…` ❌ |
| Per-chunk `_mc8_fused` (post-3l) | **~7.5s** | **`Paris. The capital of France…`** ✅ |

**Deferred:** Batched single-readback vocab GEMM (performance opt) — needs in-encoder weight upload via `copy_buffer_to_buffer`, not interleaved `queue.write_buffer`.

**Next (Part 3l → superseded by 3m):** see Part 3m below.

#### Part 3m — TTFT ping-pong (2026-06-19) 🟡 infrastructure landed

**Architect directive:** Allocate `prefill_work_buf_A`/`_B`, re-enable `encode_prefill_q_ffn_tail_fused` with ping-pong bindings, purge internal flushes, target ~4s TTFT + `Paris.`

**Landings (`gguf_bridge.rs`):**

| Item | Status |
|------|--------|
| `prefill_work_buf_a` + `prefill_work_buf_b` | ✅ allocated in `ensure_gemm_buffers` |
| Ping-pong routing in `encode_prefill_q_ffn_tail_fused` | ✅ A/B alternation for GEMM + elem |
| Pristine snapshot (`work_aliases_hidden`) | ✅ `batch_buf` → `work_a[slot_save]` before o_proj |
| Residual scratch | ✅ `prefill_scratch_buf` (third buffer — avoids A/B read+write alias) |
| WebGPU validation | ✅ no `PrefillBatchWork*` aliasing errors after scratch fix |
| Hot-path enable | ✅ `MC8_FUSED_PREFILL_TAIL = true` (Part 3o: zero-flush batched Q) |

**Harness (`WASM_ASYNC=1`, naked, Q4_K_M):**

| Path | TTFT | Output | WebGPU validation |
|------|------|--------|-------------------|
| Per-token `encode_attn_ffn_tail_gpu` (hot path) | **~7.9s** | **`Paris. The capital of France…`** ✅ | clean |
| Fused ping-pong (pre-3n) | ~8.3s | garbled (`()()The function…`) ❌ | clean |
| Fused ping-pong (post-3n) | **~7.9s** | **`Paris. The capital of France…`** ✅ | clean |

**Findings:**

1. **Ping-pong alone does not eliminate `mc8_flush`.** Chrome validates buffer usage across the **entire `CommandEncoder` sync scope**, not per compute pass. Required flushes: (a) between weight uploads (gate/up/down/o_proj/Q), (b) between write-then-read on same buffer across stages, (c) residual scratch must be a **third** buffer (`prefill_scratch_buf`), not `work_a`/`work_b`.
2. **~4s TTFT not achieved** — per-token prefill still ~7.9s; fused ~8.3s (no win while incoherent).
3. **Fused batched math** still wrong despite validation clean — likely batched Q/FFN slot layout or batched attention offset; per-token path remains authoritative.

**Next (Part 3m → superseded by 3n):** see Part 3n below.

#### Part 3n — Batched numerics isolation (2026-06-19) ✅ coherence restored

**Architect directive:** Split Q-SDPA (sequential) from batched FFN; audit elementwise dispatch grids; verify GEMM $M$ dimension.

**Root cause:** Q SDPA loop called `encode_attention_pass_gpu_offset` for each token but shared one `CommandEncoder` submit. `attention_params_buf` + `attention_mask_buf` are uploaded via `queue.write_buffer` per token — **only the last token's `token_idx`/causal mask survived** (pt3c uniform race analogue). Tokens 1–3 attended to the wrong causal horizon → `()()The function…` geometric shear.

**Fix:** `mc8_flush(pipeline)` after **each** per-token Q SDPA dispatch (sequential Q unchanged; batched FFN chain unchanged).

**Audits:**

| Item | Finding |
|------|---------|
| Q-SDPA batching | Already sequential (`num_tokens_in_batch=1` per token) — correct "split the baby" |
| Elementwise `batch` param | Per-token `for t` loops with `batch=1` + row offsets — **not** the bug (not a single `batch=1` over full buffer) |
| WGSL GEMM $M$ | `encode_gemm_bufs_offset` is vector×matrix ($M=1$); fused path loops per token per stage — correct for current shader |
| True $M=37$ batched GEMM | Not yet implemented — future TTFT win requires GEMM shader $M$ support |

**Harness (`WASM_ASYNC=1`, naked, Q4_K_M, `MC8_FUSED_PREFILL_TAIL=true`):**

| Metric | Result |
|--------|--------|
| TTFT | **7925 ms** (~7.9s; similar to per-token ~7.9s) |
| Output | **`Paris. The capital of France…`** ✅ |
| WebGPU validation | clean |

**Note:** Prefill chunk in harness logs `n_tokens=4` per layer pass — TTFT floor unchanged until larger batch fusion or $M&gt;1$ GEMM lands.

**Next (Part 3n):** superseded by Part 3o below.

#### Part 3o — TTFT Collapse: zero-flush batched Q-SDPA (2026-06-19) 🟡 TTFT win; coherence gate open

**Architect directive:** Eliminate per-token `mc8_flush()` in the Q-SDPA loop via parameter-array upgrade + 2D dispatch grid; expand chunk horizon (`PREFILL_CHUNK_SIZE` already **64**); prepare batched GEMM $M>1$ (deferred).

**Problem (Part 3n floor):** Per-token Q SDPA required `mc8_flush` after each dispatch because `queue.write_buffer` on shared `attention_params_buf` / `attention_mask_buf` races inside one encoder submit. With `n_tokens=4` and 32 layers, that is **128 extra queue submits** per prefill chunk — API throttle dominates TTFT (~7.9s).

**Implementation (landed):**

| Item | Detail |
|------|--------|
| Rust | `encode_attention_batched_q_prefill()` in `gguf_bridge.rs` — uploads **one** uniform `AttentionGpuParams` + **one** batched mask buffer (`n_tokens × KV_ATTENTION_MASK_WORDS` words); single `dispatch(n_head, n_tokens, 1)` per layer |
| WGSL | `fused_attention.wgsl`: `out_stride_elems` for strided Q rows into `work_a`; `q_mask_token = wg_id.y` when `num_tokens_in_batch > 1`; decode uses `token_ix = 0` via `select` |
| Mask buffer | `attention_mask_buf` resized to `PREFILL_CHUNK_SIZE × KV_ATTENTION_MASK_WORDS` (2048 `u32`s) |
| Per-Q flush | **Removed** from Q loop inside `encode_prefill_q_ffn_tail_fused` |
| Weight-stage flush | **Retained** between gate/up/down/o_proj (encoder-scope `gemm_weight_buf` rule unchanged) |
| Batched GEMM $M>1$ | **Deferred** — `fused_transformer.wgsl` / `encode_gemm_bufs_offset` still vector×matrix ($M=1$) per token |

**Rejected approach (documented):** `array<AttentionParams>` **storage** buffer for per-token params. WGSL storage-array element stride = `roundUp(sizeof(struct), 16)` → **96 bytes** per slot while Rust `#[repr(C)]` struct is **84 bytes**. Misaligned reads corrupt params for `token_ix > 0`. **Fix if revisiting:** pad struct to 96 bytes on both sides, or use flat `array<u32>` params slab.

**Harness (`WASM_ASYNC=1`, naked, SmolLM2-360M-Q4_K_M, `MC8_FUSED_PREFILL_TAIL=true`):**

| Run | TTFT | Output | Validation |
|-----|------|--------|------------|
| Part 3n (per-Q flush) | **7925 ms** | **`Paris. The capital of France…`** ✅ | clean |
| Part 3o (batched Q) | **5638–6534 ms** | ` a country that is a member of the European Union…` ❌ | clean |
| Part 3o bisect (revert to 3n sequential Q+flush, same WGSL) | **~6700 ms** | same EU garble ❌ | clean |

**Interpretation:** TTFT dropped **~1.4–2.3s** (−17–27%) — queue-submit amortization works. Coherence regression is **not fully explained** by batched Q alone (3n sequential path also failed `Paris.` on the same rebuilt wasm in-session). Treat as **branch/build bisect** before blaming batched grid numerics.

**Files:** `gguf_bridge.rs` (`encode_attention_batched_q_prefill`, expanded `attention_mask_buf`, `out_stride_elems` on `AttentionGpuParams`); `fused_attention.wgsl` (2D Q grid + strided `attn_out`).

**Next (Part 3o → 3p):** superseded by Part 3p below.

#### Part 3p — Coherence Recovery & Batched GEMM (2026-06-19) 🟡 `Paris.` ✅; batched Q held

**Architect directive:** Hard environmental bisect to recover `Paris.`; implement $M>1$ matrix-matrix GEMM in `fused_transformer.wgsl`.

**Phantom regression root cause (resolved):** Not environmental contamination. The “3n revert” in-session passed **`batch_start_token_idx` (chunk start)** instead of **`abs` (per-token position)** to `encode_attention_pass_gpu_offset` when `num_tokens_in_batch=1`. Shader computes `abs_pos = batch_start_token_idx + 0` → every Q SDPA pass used **RoPE position 0**, producing EU-coherent but wrong logits. Separately, **`encode_prefill_q_ffn_tail_fused`** (not `encode_attn_ffn_tail_gpu`) was the failing path; disabling `MC8_FUSED_PREFILL_TAIL` immediately restored `Paris.` on clean rebuild.

**Environmental purge (verified):** `cargo clean`, fresh `package-qualia-wasm.ps1` (8 MB stack `RUSTFLAGS`), harness `WASM_NAKED_PROMPT=1` → `prompt mode: naked` ✅.

**Batched GEMM $M>1$ (landed):**

| Item | Detail |
|------|--------|
| WGSL | `fused_transformer.wgsl`: `n_batch`, `in_row_stride`, `out_row_stride`; `global_id.y` = token index $m$; dispatch `(⌈N/64⌉, M, 1)` |
| Rust | `GemmGpuParams` extended; `encode_gemm_bufs_offset(..., n_batch, in_row_stride, out_row_stride)` |
| FFN collapse | `encode_prefill_q_ffn_tail_fused`: o_proj + gate + up + down → **one batched dispatch each** (strided `row_stride` rows) |

**Harness (`WASM_ASYNC=1`, naked, SmolLM2-360M-Q4_K_M, `MC8_FUSED_PREFILL_TAIL=true`):**

| Run | TTFT | Output | Notes |
|-----|------|--------|-------|
| Bisect: fused off (per-token tail) | **7656 ms** | **`Paris. The capital of France…`** ✅ | confirms fused-tail bug |
| Part 3p: fused + per-token Q (`abs` fix) + batched GEMM | **7602 ms** | **`Paris. The capital of France…`** ✅ | coherence + strided $M>1$ GEMM |
| Part 3p: fused + batched Q (3o) + batched GEMM | **5982 ms** | EU garble ❌ | batched Q numerics still open |

**Batched Q status:** Zero-flush batched Q still fails coherence even with `abs` fix on per-token path. Held at per-token Q+flush until batched grid/mask audit completes. TTFT ~7.6s (not ~4s target).

**Files:** `gguf_bridge.rs` (`abs` RoPE fix, batched `encode_gemm_bufs_offset`); `fused_transformer.wgsl` ($M>1$ GEMM).

**Next (Part 3p → 3q):** superseded by Part 3q below.

#### Part 3q — Batched Q Coherence (2026-06-19) ✅ `Paris.` on ~6s path

**Architect directive:** Isolate M>1 batched Q-SDPA EU garble via batch-of-1 fallback + mask/grid audit.

**Batch-of-1 isolation:**

| Test | TTFT | Output | Conclusion |
|------|------|--------|------------|
| Batched pipeline, M=1 loop, **no flush** | ~7722 ms | EU garble ❌ | `attention_params_buf` race (last token wins) |
| Batched pipeline, M=1 loop, **with flush** | ~7834 ms | **`Paris.`** ✅ | Batched WGSL math innocent at M=1 |
| M>1 single dispatch (pre-fix) | ~5947 ms | EU garble ❌ | Mask slab + grid class |

**Root cause (confirmed):** **U1 `mask_active` OR across batch rows.** `encode_attention_batched_q_prefill` OR-ed `mask_active` if *any* token's U1 routing mask was active, then uploaded per-token sparse mask slabs. Each slab row only marks the token's own slot (+ route bits), **not** the full causal $0 \dots P$ bitmap. With global `mask_active=1`, all $M$ query rows used slab indexing (`q_mask_token * mask_word_count`) and **skipped valid KV slots** — fluent EU-context smear. Sequential per-token Q masked each dispatch independently (no OR), so only the affected token was poisoned; batched OR poisoned the whole chunk.

**Secondary requirement:** **`mc8_flush` after M>1 batched Q** before o_proj — `gemm_weight_buf` encoder-scope rule (same as pt3c K/V).

**Fix (landed):**

| Item | Detail |
|------|--------|
| Dense prefill Q | `mask_active = 0`; skip mask slab upload (causal loop `logical <= abs_pos` is authoritative) |
| Post-Q flush | `mc8_flush` after `encode_attention_batched_q_prefill` |
| Uniform padding | `AttentionGpuParams` + WGSL `_pad` → **96 bytes** (16-byte uniform alignment) |
| Production path | M>1 batched Q + M>1 batched GEMM (o_proj/gate/up/down) |

**Harness (`WASM_ASYNC=1`, naked, SmolLM2-360M-Q4_K_M):**

| Run | TTFT | Output |
|-----|------|--------|
| Part 3p baseline (per-token Q + batched GEMM) | ~7602 ms | **`Paris.`** ✅ |
| **Part 3q (M>1 batched Q + batched GEMM)** | **5403–6084 ms** | **`Paris. The capital of France…`** ✅ |

**TTFT delta:** ~1.4–2.2s below 3p floor (−18–29%); ~2.2s above ~4s Phase 2B target.

**Files:** `gguf_bridge.rs` (`encode_attention_batched_q_prefill` mask policy, post-Q flush); `fused_attention.wgsl` (96-byte uniform).

**Next (Part 3q → 3r):** superseded by Part 3r below.

#### Part 3r — Batched Elementwise & Submit Coalescing (2026-06-19) 🟡 `Paris.` ✅; Phase 2B gate ❌

**Architect directive:** $M>1$ 2D elementwise grids + crush prefill API overhead toward TTFT &lt;4500 ms.

**WGSL (`wasm_elementwise.wgsl`):**
- `silu_mul_main` / `add_residual_main`: `gid.y` = batch row; strided `a_idx`/`b_idx`/`out_idx` via `ElemParams` row_stride + slot fields.
- `rms_norm_batch`: `wg_id.y` = token row; variance reduction strictly row-local (no cross-token `workgroupBarrier`).

**Rust (`gguf_bridge.rs`):**
- Removed per-token `for t` loops in `encode_prefill_q_ffn_tail_fused` for attn/FFN residual, FFN RMSNorm, SiLU×mul.
- Dispatch grids: `(n/64, batch, 1)` elementwise; RMSNorm `(1, batch, 1)`.
- **Ping-pong staging (Part 3r):** `gemm_weight_buf_b`, `gemm_params_buf_b`, `attention_params_buf_b`, `elem_params_buf_b` — K/V and gate/up in one encoder submit; elem slot 0/1 for silu+FFN residual pair.
- **Safe coalescing rules (learned):** shared `elem_params_buf` / `gemm_params_buf` poison all dispatches in the same submit unless ping-ponged; `o_proj`→gate requires flush (`gemm_params` slot 0 reuse).
- **Submit hygiene:** removed duplicate empty `finish()` per layer; attn RMSNorm + K share encoder.
- **U1 segregation:** prefill `mask_active = 0` (dense causal); sparse slabs decode-only via `encode_attention_pass_gpu`.

**Harness (`WASM_ASYNC=1`, `WASM_NAKED_PROMPT=1`, SmolLM2-360M-Q4_K_M):**

| Run | TTFT | Output |
|-----|------|--------|
| Part 3q baseline | 5403–6084 ms | **`Paris. The capital of France…`** ✅ |
| **Part 3r (batched elem + coalescing)** | **5759–6228 ms** (best ~5835 ms) | **`Paris. The capital of France…`** ✅ |

**Phase 2B gate:** TTFT &lt;4500 ms ❌ (~1.3–1.7 s remaining). WebGPU validation clean. Output coherent.

**Files:** `gguf_bridge.rs`, `wasm_elementwise.wgsl`, `fused_attention.wgsl` (96-byte uniform, unchanged).

**Next (Part 3r → 3s):** superseded by Part 3s below.

#### Part 3s — Dynamic Uniform Offsets & Flush Purge (2026-06-19) 🟡 `Paris.` ✅; Phase 2B gate ❌

**Architect directive:** Replace ping-pong uniform buffers with WebGPU `has_dynamic_offset: true`; batch all layer params in one `write_buffer`; purge flush budget toward ≤3/layer; wire U1 decode masks ($M=1$).

**Rust (`gguf_bridge.rs`):**
- `MC8_UNIFORM_ALIGN = 256`; `Mc8UniformArena` (fixed 8-slot stack buffer).
- Explicit `BindGroupLayout` with `has_dynamic_offset: true` on gemm binding 2, elem binding 3, attn binding 2.
- Pipelines recreated with explicit `PipelineLayout` (wasm only).
- `gemm_params_buf` / `elem_params_buf` / `attention_params_buf` sized to `256 × 8` bytes; removed `*_buf_b` ping-pong uniform buffers.
- `encode_prefill_q_ffn_tail_fused`: pre-build all `GemmGpuParams` + `ElemGpuParams` in arenas → single upload → dispatch with `set_bind_group(..., &[dyn_offset])`.
- Prefill flush budget (fused tail): after Q → after o_proj+attn → after gate/up → end-of-layer (4 submits in tail + 1 after K/V = **5/layer**).
- Decode: `encode_attn_ffn_tail_gpu` wires U1 sparse mask via `attention_kv_mask_for_dispatch` + `mc8_upload_attn_param` when `mask_active != 0` ($M=1$ safe).

**Compile fixes (wasm-pack features):**
- `BufferBindingType::Storage { read_only: false }` for writable storage bindings.
- Non-generic `Mc8UniformArena` (const-generic array sizes illegal on stable).
- Missing `out_slot` arg in FFN norm `encode_elem_offset`; `&layout` in decode mask call.

**Flush experiments (reverted):**
- o_proj on weight slot 1 (merge Q+o_proj submit) → **coherence break** (`is is is…`).
- Remove gate/up→SiLU flush → **coherence break**. Storage/weight ordering stricter than encoder-order alone.

**Harness (`WASM_ASYNC=1`, `WASM_NAKED_PROMPT=1`, SmolLM2-360M-Q4_K_M):**

| Run | TTFT | Output |
|-----|------|--------|
| Part 3r baseline | 5759–6228 ms | **`Paris. The capital of France…`** ✅ |
| **Part 3s (dynamic offsets)** | **6013–6062 ms** | **`Paris. The capital of France…`** ✅ |

**Phase 2B gate:** TTFT &lt;4500 ms ❌ (~1.5 s remaining). WebGPU validation clean. Output coherent.

**Files:** `gguf_bridge.rs` (dynamic offsets, arena upload, decode U1 mask).

**Next (Part 3s → 3t):** superseded by Part 3t below.

#### Part 3t — Weight Arena & Single-Submit Layer (2026-06-19) 🟡 `Paris.` ✅; Phase 2B gate ❌

**Architect directive:** Disjoint weight buffers (`qkv`/`oproj`/`gate`/`up`/`down`); purge mid-layer flushes; target 1 submit/layer.

**Rust (`gguf_bridge.rs`):**
- `Mc8WeightRole` + `Mc8WeightArenaBufs` (7 disjoint `STORAGE` buffers, one per GEMM role).
- `write_weight_role` / `mc8_weight_role_buf` — each `queue.write_buffer` targets a unique buffer.
- Prefill GEMM/attn binds role-specific buffer (`AttnK`, `AttnV`, `AttnQ`, `OProj`, `Gate`, `Up`, `Down`).
- **Tail flush purge (coherent):** removed post-Q, post-o_proj, post-gate/up flushes inside `encode_prefill_q_ffn_tail_fused`.
- **Submit budget:** **2/layer** — flush after K/V block, flush at layer end (down from ~5/layer).

**Single-submit attempt (reverted):**
- Merging K/V + Q + FFN into one encoder without K/V flush broke coherence (`is is…` / `1000000…`).
- Root cause: `attention_params_buf` / `elem_params_buf` queue races — Q upload overwrote K/V uniform slots; attn RMSNorm elem upload clobbered tail elem arena when merged.
- Fix path for Part 3u: pre-stage **all** attn (K/V/Q) + elem (attn norm + tail) uniforms in one arena upload **before** any dispatches in the layer encoder.

**Harness (`WASM_ASYNC=1`, `WASM_NAKED_PROMPT=1`, SmolLM2-360M-Q4_K_M):**

| Run | TTFT | Output | Submits/layer |
|-----|------|--------|---------------|
| Part 3s baseline | 6013–6062 ms | **`Paris. The capital of France…`** ✅ | ~5 |
| **Part 3t (weight arena + tail flush purge)** | **5822–5948 ms** | **`Paris. The capital of France…`** ✅ | **2** |

**Phase 2B gate:** TTFT &lt;4500 ms ❌ (~1.3–1.4 s remaining). WebGPU validation clean.

**Files:** `gguf_bridge.rs` (`Mc8WeightArenaBufs`, `Mc8WeightRole`, `ensure_gemm_buffers`).

**Next (Part 3t → 3u):** superseded by Part 3u below.

#### Part 3u — Unified Super-Arena (2026-06-19) 🟡 `Paris.` ✅; Phase 2B gate ❌

**Architect directive:** Pre-encoder super-arena (one `write_buffer` per uniform buffer); purge K/V flush; **1 submit/layer**.

**Rust (`gguf_bridge.rs`):**
- `Mc8PrefillLayerUniforms` + `Mc8PrefillLayerGeom` + `mc8_stage_prefill_layer_super_arena`.
- `Mc8AttnUniformArena` (K/V/Q), `Mc8ElemUniformArena` (attn norm + tail elem), `Mc8UniformArena` (tail gemm).
- **Three uploads/layer** (attn + elem + gemm) executed **before** `WasmGpuPipeline::begin`.
- Attn RMSNorm uses pre-staged `encode_elem_offset` (dense `prefill_scratch` layout).
- Tail dispatches consume pre-staged dynamic offsets only (no mid-layer uniform uploads).

**Single-submit attempt:**
- Removing K/V↔tail `mc8_flush` with super-arena still breaks coherence (`is is…`).
- **Root cause (empirical):** KV cache **storage** write visibility — Q-SDPA reads stale KV without queue submit between V and Q on Chrome/WebGPU, despite encoder-ordered dispatches.
- **Retained:** 1 flush between K/V block and Q/FFN tail → **2 submits/layer** (uniform races eliminated; KV flush still required).

**Harness (`WASM_ASYNC=1`, `WASM_NAKED_PROMPT=1`, SmolLM2-360M-Q4_K_M):**

| Run | TTFT | Output | Uniform uploads/layer |
|-----|------|--------|------------------------|
| Part 3t baseline | 5822–5948 ms | **`Paris. The capital of France…`** ✅ | staggered (~5+) |
| **Part 3u (super-arena)** | **5646–6127 ms** | **`Paris. The capital of France…`** ✅ | **3** (pre-encoder) |

**Phase 2B gate:** TTFT &lt;4500 ms ❌ (~1.1–1.5 s remaining). WebGPU validation clean.

**Next (Part 3u → 3v):** see Part 3v below.

#### Part 3v — Cross-Layer Encoder Merge (2026-06-19) ❌ blocked on shared-buffer races

**Architect directive:** If 1-submit/layer TTFT still &gt;4500 ms, merge multiple layers per encoder (e.g. 4 layers × 8 chunk submits).

**Attempted (`gguf_bridge.rs`):**
- `MC8_LAYERS_PER_ENCODER = 4` + persistent encoder across layers; cumulative uniform slot offsets (`Mc8ChunkUniformCursors`, `upload_at`).
- Chunk-sized uniform VRAM (16 attn / 24 elem / 20 gemm slots).

**Harness result:** coherence broken (`is is…` / `to to…`) until fixes applied; after cumulative-uniform fix still broken.

**Root causes (empirical):**
1. **Uniform clobber** — per-layer super-arena `write_buffer` at offset 0 overwrote uniforms still referenced by pending Q+FFN in the open encoder. Fixed by cumulative `upload_at` slots.
2. **`norm_weight_buf` clobber** — next layer `upload_norm_weights` runs on the queue timeline before the prior layer's encoder (containing FFN RMSNorm) is submitted. Same class of race as pt3c weight arena.
3. **KV storage visibility** (Part 3u) — K/V↔Q flush still required per layer.

**Reverted:** per-layer encoder + layer-end flush restored. `Paris.` ✅ @ **6127 ms** TTFT.

**Part 3v prerequisite:** disjoint **norm-weight arena** (like `Mc8WeightArenaBufs`) before cross-layer merge can be retried.

**Next:**
1. `Mc8NormWeightArena` (per-layer norm slots) → retry 3v encoder merge.
2. KV visibility without K/V flush (storage barrier) → true 1-submit/layer.
3. MC7 ChatML regression after Phase 2B TTFT gate.

**Ruled out (pt3b–3h):**
- FFN `add_residual_main` routing as *dominant L31 amplifier* — pt3b ruled dominant leak; pt3c K/V race was L31 fix
- KV layer stride / uniform `layer_idx` — structurally correct
- Batched K/V flat-RoPE — already per `wg_id`
- Batch RMSNorm cross-row variance — per-row via `wg_id.x`
- Prefill `attn_input` handoff — parity hygiene only
- **L0 causal mask / softmax flattening** — pt3e Attn_Out bit-exact
- **L0 `attn_norm` weight swap** — pt3e RMSNorm bit-exact
- **L0 K write / RoPE failure** — pt3e K_rope bit-exact (after COPY_SRC fix)
- **L0 o_proj GEMM** — pt3f bit-exact
- **L0 pristine-residual trap** — pt3f `pristine_hidden` bit-exact; not normalized residual
- **L0 post-attn residual** — pt3f bit-exact
- **L0 `ffn_norm` weight swap** — pt3f bit-exact
- **L0 FFN gate/up/SwiGLU/down/residual** — pt3g all bit-exact; `ffn_residual=0.423` locked
- **L0 `base_save` timing trap** — pt3g captures post-attn, not ffn_norm output
- **gate→up buffer alias / missing flush before SiLU** — pt3g audit clean
- **token_embd lookup / upload** — pt3h bit-exact
- **pure CPU vs GPU prefill KV (replay trap)** — pt3h K/V bit-exact @ L0
- **decode-step L0 kernels** — pt3e–3g all bit-exact (replay on GPU KV was red herring for KV itself)

**Confirmed root cause (pt3c):** `gemm_weight_buf` queue race — K dispatch ran with V weights
without `mc8_flush` between passes. Fix dropped L31 **271→21**.

**Confirmed root cause (pt3h):** **batched prefill hidden handoff** — `L1_input_hidden` diverges after GPU L0 prefill; KV writes remain correct.

**Confirmed root cause (pt3i):** **`token_hidden`/`work_buf` alias** in prefill per-token tail (`gemm_output_buf`) — o_proj clobbered residual base; pt3e–3g decode-path diffs did not exercise this routing.

**Next actions (post-pt3u):**
1. **TTFT** — cross-layer encoder merge (Part 3v); super-arena done; KV flush blocks true 1-submit/layer.
2. **U1 decode masks** — wired in decode Q path ($M=1$); prefill remains dense `mask_active=0`.
3. MC7 ChatML regression after Phase 2B TTFT gate.
4. Phase 2B **not closed** until `WASM_ASYNC=1` naked `Paris.` + TTFT &lt;4500 ms.

**Implementation notes (MC2 session):**
- `cpu_attention_pass` uses raw-pointer KV mutation (sound on single-threaded wasm).
- `att_scores` stack buffer (`[f32; MAX_CONTEXT_WINDOW]`) scoped to proj_kind=0 only (keeps K/V stack lean).
- `rope_inplace` and `fused_attention.wgsl::apply_rope_neox` both use **NEOX split-half** + dynamic `rope_theta_base`/`rope_scale` (MC8 pt2).
- `initialize_webgpu_engine` must call **`adopt_resident_mmap`** (F5) — do not set `gguf_mmap` alone.

1. **Short-circuit GEMM to CPU on wasm:** in `dispatch_gemm_raw_into` /`dispatch_gemm_into`,
   gate the entire GPU encode+map block under `#[cfg(not(target_arch="wasm32"))]` and on
   wasm call `stack_gemm_quant` directly — **do not create/write GPU buffers or `map_async`
   on wasm at all** (prevents wgpu state churn and the wasted-work OOB surface, F3).
   *(Attention path already short-circuits via `cpu_attention_pass`; GEMM path unchanged.)*
2. **Attention (F1, Option A):** `cpu_attention_pass` in `dispatch_attention_pass` — see MC table above.
   - **Option B (async WebGPU, Phase 2B):** make the wasm decode path async — add
     `dispatch_transformer_forward_async` + async attention using `JsFuture` on
     `map_async`, and make `infer_wasm_streaming` await per-step. Keeps GPU compute.
     **Not started** — mandated for real demo per §6 Q3; Option A is debug-only.
3. **Success gate:** `inferWasmStreaming` returns coherent tokens for capital-of-France on
   `SmolLM2-360M`. **TTFT heuristic:** real CPU SDPA prefill should be **seconds**, not
   <600 ms. Fast TTFT + `"firehose"` = attention/KV path still bypassed.

---

## 💾 PHASE 3 — OPFS robust model caching

**Problem:** Cache Storage `put` fails on >250 MB GGUF (`Unexpected internal error`).

1. **Write path:** `navigator.storage.getDirectory()` → stream the `fetch` response body
   directly into a `FileSystemWritableFileStream` (do **not** buffer the whole blob in one
   ArrayBuffer first). Key by model filename + a version/etag.
2. **Read path:** `FileSystemFileHandle.getFile()` → `arrayBuffer()` → `Uint8Array` →
   `initialize_webgpu_engine`. Add a "Cache model in browser" toggle / first-run prompt.
3. **Future-proofing:** structure the OPFS layout to align with `.q42.bidx` demand-paging so
   a later version can map chunks from OPFS rather than loading the whole file. (Note: the
   current `initialize_webgpu_engine` still does one `to_vec` copy into wasm memory — fine
   for now; chunked/`mmap`-from-OPFS is a Phase 4 concern.)
4. Implement in the harness first (`docs/wasm-llm-test.html`), then port to
   `docs/online-llm-demo.html`.

---

## 🚀 PHASE 4 — GGUF → `.q42` AOT ingestion (architectural horizon)

**Goal:** compile GGUF → Qualia-native `.q42` *ahead of time* so inference skips runtime
GGUF parsing and maps weights directly into WebGPU.

1. New wasm fn `compile_gguf_to_q42(input_gguf: Uint8Array) -> Uint8Array`.
2. Parse GGUF header; extract weight tensors; emit a `.q42` container with: pre-computed
   tensor byte offsets, **pre-baked WGPU bind-group layouts** for `fused_transformer.wgsl` /
   `fused_attention.wgsl`, and a Quin index/manifest linking tensors to lexical graphs.
3. Save `.q42` to OPFS; future runs map it directly (zero CPU parse).

⚠️ **Design tension to resolve first (see §6 Q2):** GEMM needs **contiguous** weight
matrices; a 48-byte `NQuin` is a semantic record. Weights almost certainly should be stored
as **opaque contiguous tensor blobs with a Quin/NQuin manifest pointing at them**, not
weights re-encoded as Quins. Confirm the intended `.q42` weight representation before building.

---

## 📍 KEY CODE REFERENCES

| Item | Location |
|------|----------|
| wasm exports | `crates/qualia-core-db/src/wasm_llm.rs` |
| wasm init | `gguf_bridge.rs:3242` (`initialize_webgpu_engine` → `adopt_resident_mmap`), `:401` (`try_new`) |
| wasm CPU attention | `cpu_attention_pass` `gguf_bridge.rs:1705` (MC1–MC2); entered from `dispatch_attention_pass` `:1552` |
| decode (wasm branch) | `llm_agent.rs:654` / wasm region from ~`:1126` |
| prefill | `dispatch_prefill_chunk` `gguf_bridge.rs:1965` → `dispatch_prefill_layer_batch` |
| transformer layer | `dispatch_transformer_layer` `gguf_bridge.rs:2003`; forward `:2095` |
| attention | `dispatch_attention_layer` `:1617` → `dispatch_attention_pass` `:1460` (**no CPU fallback**) |
| GEMM + CPU fallback | `dispatch_gemm_into` `:1308`, readback `~:1284`, `stack_gemm_quant` `:294` |
| argmax | `dispatch_output_argmax_chunked` `:1331` |
| async variants | `dispatch_gemm_into_async`, `dispatch_transformer_forward_async`, `dispatch_prefill_chunk_async`, `dispatch_output_argmax_chunked_async` |
| MC8 fused layer | `encode_transformer_layer_gpu`, `encode_attn_ffn_tail_gpu`, `wasm_elementwise.wgsl` |
| MC8 GPU prefill | `dispatch_prefill_chunk_async_mc8_gpu` `gguf_bridge.rs` (~4608) |
| MC8 depth bisect | `dispatch_transformer_forward_async` `l0_probe_step1` → layers 0–3 + post-L31 |
| MC8 RoPE (WGSL) | `fused_attention.wgsl::apply_rope_neox`; K/V `abs_pos=batch_start+token_in_batch` |
| MC8 flush rule | `mc8_flush()` between K→V, gate→up, and before elem reads of GEMM output |
| constants | `MAX_STACK_GEMM_DIM=10240` `:188`; `PREFILL_CHUNK_STACK_FLOATS=2560*64` `:196`; `MAX_PREFILL_BATCH_FLOATS=10240*64` `:194` |
| KV cache | struct fields `gguf_bridge.rs:354-358` (`kv_layout`, `kv_cache_gpu`, `kv_cache_cpu: Box<[f32]>`) |

---

## 🔧 BUILD / DEPLOY / TEST (canonical)

```bash
# build (Git Bash — PS wrapper aborts on wasm-pack stderr)
cd crates/qualia-core-db
RUSTFLAGS="-C target-feature=+simd128 -C link-arg=-zstack-size=8388608 -C link-arg=--max-memory=4294967296" \
  wasm-pack build --target web --out-dir pkg-qualia --release -- \
  --no-default-features --features portal,wasm-llm,wasm-logic,wasm-scientific
# deploy
SRC=crates/qualia-core-db/pkg-qualia DOCS=docs/pkg/qualia
cp -f $SRC/qualia_core_db.js $DOCS/qualia.js; cp -f $SRC/qualia_core_db_bg.wasm $DOCS/qualia_bg.wasm
cp -f $SRC/qualia_core_db.d.ts $DOCS/qualia.d.ts; cp -f $SRC/qualia_core_db_bg.wasm.d.ts $DOCS/qualia_bg.wasm.d.ts
sed -i 's/qualia_core_db_bg\.wasm/qualia_bg.wasm/g' $DOCS/qualia.js
```
- Test page: `docs/wasm-llm-test.html` (served by `.claude/serve_docs.py` on :8788).
- Models (gitignored, `docs/models/`): `SmolLM2-360M-Instruct-Q4_K_M.gguf`,
  `smollm2-360m-instruct-q8_0.gguf`, `gemma-3-1b-it-q4_0.gguf`.
- `wasm-opt` must stay **off** until a safe opt set is validated against the model copy.

---

## ✅ 6. DECISIONS (answered by Qualia Architect, 2026-06-18)

- **Q1 — Attention:** **Option A now** (strictly bounded, fixed-capacity CPU attention
  fallback, `#[cfg(target_arch="wasm32")]`) to close the OOB and prove token coherence;
  **then Option B** (true async-WebGPU decode through `fused_attention.wgsl`) as the
  mandated real path. Option A is a temporary diagnostic "stent," not the destination.
- **Q2 — `.q42`:** **Contiguous strided tensor blocks + a 48-byte Quin manifest.** Weights
  are opaque, OS-page-aligned, contiguous binary blocks strided for WGPU bind groups. The
  48-byte Quin is the *epistemic/topological scaffold* (header manifest binding lexical
  graphs / WordNet to tensor data). **Do NOT encode weights as Quins** (kills contiguous
  matmul + cache).
- **Q3 — Perf / COOP-COEP:** GPU compute (Option B) is **mandatory for the real demo**; CPU
  (Option A) is **debug-only** to prove the data structures don't trap. Enable
  `crossOriginIsolated` (COOP/COEP) on the dev servers (`serve_docs.py`, `serve_models.py`)
  to unlock `SharedArrayBuffer` for future multithreading; primary compute vector stays WebGPU.
- **Q4 — Sequence:** Phase 1 → Phase 2 (Option A) → Option B; **Phase 3 (OPFS) in parallel**
  (independent of the inference fix).
- **Q5 — Commit:** **Bank the verified fixes now** as a clean **0.0.18** baseline before
  touching the attention kernels.

---

## 📓 7. PROGRESS LOG (newest first — keep this updated every step)

- **2026-06-19 (aw)** — **Phase 5 TRUE ROOT CAUSE (direct profiling): WebGPU queue-submit IPC overhead.**
  - Added CPU-side `js_sys::Date::now()` phase timing (temporary). Decode = **98.7% forward**
    (`forward=52441ms argmax=667ms / 32 tok` → argmax only ~21 ms/tok).
  - Forward split: **23 ms CPU encode + 1555 ms GPU drain** → GPU-bound, not CPU-encoding. Linear in
    layers: **1-layer run = 11.5 tok/s**; 32-layer = 1639 ms/tok → **~49 ms/layer**.
  - 49 ms/layer for ~15M MACs = 0.3 GMAC/s (absurd for Ampere) ⇒ NOT compute. The forward issues
    **64 `queue.submit()`/token** (2 `mc8_flush`/layer); 64 × ~24 ms ≈ 1555 ms ≈ the GPU drain ⇒
    **WebGPU submit IPC overhead** (WASM→Chrome-GPU-process boundary + Dawn validation per submit).
  - This **overturns the (au) "logits re-upload" finding** — the logits projection is only ~21 ms/tok;
    making it resident (5.3) changed nothing. au was wrong; corrected here + in memory.
  - **Next (5.4): single-submit forward** — one encoder for all 32 layers, non-overlapping super-arena
    uniforms (no mid-loop write_buffer race), delete per-layer `mc8_flush`, one submit before readback.
    Existing `MC8_LAYERS_PER_ENCODER`(=4) chunk machinery already does this for prefill. OPEN: prefill's
    KV flush is "(backend empirical)" — must confirm Dawn auto-barriers intra-encoder KV writes→reads.

- **2026-06-19 (av)** — **Phase 5.3: output/logits projection made GPU-resident — correct, throughput-neutral.**
  - `mc8_upload_resident_logits` uploads the tied `token_embd` (Q8_0 [960,49152], 47.8 MB) to a
    dedicated STORAGE buffer **once** at init (both `adopt_resident_mmap` + `adopt_resident_q42`).
    `dispatch_gemm_resident_chunk_async` binds per-chunk 256-aligned sub-ranges (VOCAB_CHUNK_ROWS=8192
    is a 256-multiple → aligned) — zero per-token weight upload.
  - **Bug found+fixed:** `self.pipeline` uses `MC8GemmBGL` (dynamic uniform at binding 2); the resident
    bind needed `mc8_dynamic_uniform_binding(params)` + `set_bind_group(…, &[0])`. Also revealed the
    *original* logits path CPU-fell-back (`stack_gemm_quant`) because a chunk (~8.3 MB) > `max_tensor_bytes`
    staging — so "GPU re-upload" was never even happening; it was a CPU GEMM (~21 ms/tok, not the bottleneck).
  - **`Paris.` preserved**, resident upload confirmed (`[MC8] resident logits projection uploaded once`).
    Throughput unchanged (0.6 tok/s) — argmax was never the bottleneck (see aw).

- **2026-06-19 (au)** — **Phase 5 ROOT CAUSE: decode bottleneck = per-token re-upload of the non-resident output/logits projection.** ⚠️ **SUPERSEDED by (aw)** — argmax/logits is only ~21 ms/tok; the real bottleneck is the forward's submit IPC. Kept for history.
  - Decisive bisection ruled out everything layer-side: dispatch fusion (ar), block-amortized dequant
    (as), and neutering gate/up to 1/30 work (at) each gave **0 tok/s change** (still ~0.6 tok/s,
    ~1747 ms/token). Total per-token MAC work (~500M incl. the 49152-vocab projection) ≈ tens of ms of
    arithmetic ⇒ the cost is **data movement, not math**.
  - **Found (code):** `dispatch_output_argmax_chunked_async_mc8_fused` (gguf_bridge.rs ~7838) loops 6
    vocab chunks (49152/8192) and `write_buffer`s the output-projection weights to the GPU **per chunk,
    per token**. Output is **tied to `token_embd` (Q8_0 [960,49152] ≈ 50 MB)** → the engine
    **re-uploads ~50 MB to VRAM every token** + 6 dispatch/readback round-trips. Phase 3x made the
    *layer* weights resident; the **logits projection never was** — the Phase 2B reupload bug surviving
    in the decode logits stage. Explains why every layer-side optimization moved nothing.
  - **Fix (next, high-confidence — proven Phase 2B/3x pattern):** make the output/logits projection
    GPU-**resident** (upload ~50 MB once, mirror `mc8_upload_all_resident_weights`); run the chunked
    argmax GEMM against the resident buffer with zero per-token re-upload. Keep per-chunk
    submit/readback (gguf_bridge.rs:7808 warns batched chunks garble tokens via write_buffer races).
    Mem: ~50 MB on top of 219 MB resident (browser WebGPU, not the 128 MB Local cap).

- **2026-06-19 (at)** — **Phase 5 bisect: FFN GEMM compute is ~0% of per-token time.**
  - Temporarily neutered the fused gate/up loop to 1 of 30 Q5_0 blocks (≈1/30 the dot-product). tok/s
    **unchanged (0.6; 55742 vs 55907 ms)**; output went to garbage (no `Paris`) — which *confirms the
    fused path is live and the edit took effect*. ⇒ gate/up GEMM compute is not the bottleneck. Reverted.

- **2026-06-19 (as)** — **Phase 5.2: block-amortized Q5_0 dequant in the fused FFN — coherent, throughput-neutral.**
  - Rewrote the fused FFN inner loop to decode each 32-elem Q5_0 block's `d`+`qh` **once** (gate AND
    up) into registers, then 32 nibble-extract+MAC — vs the per-element path that re-decoded them 32×.
    Math identical to `dequant_q5_0_weight`; **`Paris.` preserved**. **tok/s unchanged (0.6)** ⇒ decode
    is not dequant-ALU-bound.
  - **Quant reality (via new `agent-tools/gguf-types.mjs`):** n_embd=960 ∤ 256 → k-quants fell back:
    ffn_gate/up = **Q5_0**, ffn_down = Q6_K, attn q/k/o = Q5_0, attn_v + token_embd = Q8_0. (The
    architect's Q4_K assumption was wrong for this model; a Q4_K loop would have garbled output.)

- **2026-06-19 (ar)** — **Phase 5: dispatch fusion (gate+up+SiLU → 1 pass) via modular Rust-composed WGSL — coherent, throughput-neutral.**
  - **Modular WGSL packaging (architect-approved):** the `math_core` dequant is authored once in
    `shaders/dequant_template.wgsl`, instantiated per weight role (`$W`/`$S` substitution) and stitched
    into `shaders/fused_ffn.wgsl` at runtime in `gguf_bridge.rs::try_new` (`format!`); the proven
    `fused_transformer.wgsl` GEMM is untouched. Phase-6 neuro-symbolic seam = `const
    ENABLE_DEONTIC_TAINT` (const-folded → zero hot-path cost; wgpu 0.19.0 has no `compilation_options`
    to set `@id` overrides at pipeline creation). New engine fields `mc8_ffn_fused_bind_layout` +
    `mc8_ffn_fused_pipeline`; method `encode_fused_ffn_expansion`; runtime fallback to the 3-dispatch
    path when gate/up quant types differ.
  - **Bug found+fixed:** WebGPU forbids one buffer bound read-only AND writable in a single pass
    (buffer-granularity, not by offset). The fused pass read `work_b@0` and wrote
    `work_b@slot_scratch_half` → invalid encoder every layer. Fixed by staging the normalized hidden
    into `work_a@0` (small per-token copy; `down` overwrites it after) so input/output use distinct
    buffers. **`Paris.` restored**, zero validation errors.
  - **Result:** removed 64 compute dispatches/forward + the SiLU + the gate/up VRAM round-trips →
    **tok/s unchanged (0.6)** ⇒ decode is NOT dispatch-bound (overturns the Phase 5 premise). The
    modular fused kernel is correct + coherent but currently throughput-neutral; kept as substrate.
    Not committed.

- **2026-06-19 (aq)** — **Wired Qualia into the comparative LLM benchmark page (`docs/benchmarks.html`).**
  - `docs/js/wasm-llm-benchmarks.js`: flipped the `qualia` engine def to live + added `QualiaAdapter`
    (lazy-imports `../pkg/qualia/qualia.js` + `./opfs-model-cache.js`; loads via `loadOrCompileQ42`
    [.q42 AOT] or `loadGgufCached` [GGUF], `initialize_webgpu_engine`, streams `inferWasmAsync` →
    TTFT/output, reports load/ttft/gen/tok-s/heapΔ via the page's `BrowserLlmAdapter` contract).
    Container toggle: `.q42` (AOT, OPFS-cached) | GGUF (direct). Added the WebGPU limits shim to the
    page. **Did NOT touch the external comparison adapters** (webllm/transformers.js are Timothy's
    lazy-loaded comparison targets, not ours).
  - **Verified (headless Chrome):** Qualia engine runs end-to-end — `load 5.39s · ttft 6.70s ·
    gen 10.16s`, results row populated, no page errors. (Output fragmentary = known greedy-ChatML
    early-EOS on SmolLM2-360M; perf metrics valid.)
  - **Note:** default model URL is same-origin `models/…gguf` (works in the local harness; on GitHub
    Pages it 404s — point the control at a reachable GGUF URL).

- **2026-06-19 (ap)** — **Phase 4: JS AOT OPFS ingest pipeline — compile-once, warm-boot from `.q42`.**
  - **`opfs-model-cache.js::loadOrCompileQ42`:** compile GGUF→`.q42` once → stream `.q42` to OPFS
    (chunked writable, no `Cache.put`; store only the `.q42`, not the GGUF) → warm-boot from it.
    **Version-keyed cache** via new `q42FormatVersion()` wasm export (single source of truth → format
    bump auto-recompiles). Hot loop zero-heap; one-time GGUF+compile = cold-path ingest tier, freed
    immediately (conformant with the RDF-ingest no-heap posture Timothy asked for).
  - **Verified (headless Chrome, `?q42=1`):** cold = download(1 net) → compile+cache (260 MB, ~3.0 s) →
    boot; warm reload = **OPFS `.q42` hit, 0 network, 0 compile, ~290 ms read** → boot → infer `Paris`.
  - **Ported:** `wasm-llm-test.html` + `online-llm-demo.html` (remote→AOT; local file still GGUF via
    the dual gate). Load-checked clean.
  - **`llmdemo` AOT BLOCKED:** imports shared `docs/playground/qualia_core_db.js` (older build, no q42
    exports, used by 6 pages) — needs a deliberate playground-wasm refresh first; keeps OPFS-GGUF cache
    for now. (Future-doc `qualia-llm-future-updates.md`: V1 format is forward-compatible for
    spectral/acoustic/PGA modalities via `role` + `NQuin.metadata`.)
  - **Next:** playground-wasm refresh → llmdemo AOT; cold CBOR-LD + metadata flags; single-buffer
    zero-copy bind; decode-fusion throughput.

- **2026-06-19 (ao)** — **Phase 4 v3: `.q42` tokenizer section → self-contained inference; "Paris" from q42 ✅.**
  - **Tokenizer section:** `GgufTokenizer::to_q42_section` / `from_q42_section` (gguf_sharder) —
    vocab/merges/bos/eos/add_bos/pre packed contiguous (no page align), derived maps rebuilt on read,
    fully bounds-checked. Header v3 (144 B) + `tokenizer_offset`/`tokenizer_len`; compiler appends the
    section after the blobs. `Q42TensorIndex::tokenizer_bytes()` accessor.
  - **Boot wiring:** `run_inference_async` builds BOTH the synthetic tensor index and the tokenizer
    from the `.q42` when `q42_resident` (no GGUF parse). Harness `?q42=1` mode + `WASM_Q42` env compile
    GGUF→q42 in-browser and boot from `Q42W`.
  - **Proven natively:** `q42_tokenizer_roundtrip` — encode/decode identical to GGUF (49152 vocab,
    1.29 MB section). With weight byte-parity (an), q42 inference ≡ GGUF inference.
  - **End-to-end (headless Chrome):** compile GGUF→`.q42` in-browser (260 MB, ~1.16 s, off TTFT clock),
    boot purely from `Q42W` (`[Q42] boot OK: 290 tensors, 32 layers`), output **`Paris. The capital of
    France…`**, **TTFT 3891 ms** (gate held). Self-contained `.q42` execution container milestone met.
  - **Next:** JS ingest pipeline (compile→OPFS→boot from q42); cold CBOR-LD + metadata flags; deploy
    rebuilt wasm to production demos.

- **2026-06-19 (an)** — **Phase 4: weight hot-path decoupling from `.q42` (synthetic GGUF index) + proof.**
  - **Approach (lower-risk than per-`encode_*` branching):** `Q42TensorIndex::to_gguf_index()` builds a
    **synthetic `GgufTensorIndex`** from the manifest via new `GgufTensorIndex::from_components`
    (`tensor_data_start=0`, absolute blob offsets, names rebuilt from role/layer → same `gguf_name_hash`
    so `get_layer_tensors` resolves). `adopt_resident_q42` now points `gguf_mmap` at the `.q42` bytes
    and uploads via the **standard** `mc8_upload_all_resident_weights` — the whole GGUF hot path runs
    unchanged, format-agnostic. Removed the interim `mc8_upload_resident_from_q42` + `q42_role_to_mc8`.
  - **Proven natively (no browser, no external libs):** `q42_synthetic_index_matches_gguf` PASSES —
    synthetic index == GGUF index, **290 tensors byte-identical** + dims/ggml_type match. ⇒ identical
    weights → identical logits → identical output. wasm compiles clean.
  - **⚠️ Tokenizer gap surfaced:** `.q42` carries weights + hyperparams but **not the tokenizer**
    (vocab/merges/specials). q42-only inference is blocked until a tokenizer section (v3) lands — same
    class of gap as hyperparams. So literal "Paris from q42 alone" is the next step, not this commit.
  - **Next:** tokenizer section + `GgufTokenizer::from_q42`; wire `run_inference_async` index branch to
    `to_gguf_index()`; then JS ingest pipeline (compile→OPFS→boot from q42).

- **2026-06-19 (am)** — **Phase 4 v2: `.q42` integrity layer + runtime reader + dual-format boot gate.**
  - **Format v2 (`q42_weight.rs`):** 128B header adds hyperparams (n_embd/n_head/n_kv_head/vocab/rope —
    self-contained boot) + `header_crc:u32` + `format_flags:u32` (replacing dead padding). Per-entry
    integrity reuses the **existing `NQuin` fields** (architect-approved, no struct churn): `parity` =
    CRC-32C of the entry's 32 functional bytes; `metadata` = reserved bitfield (sparsity / quant /
    deontic-ODRL taint). Table-less CRC-32C inline (no new dep). Decision driven by research
    (ZFS/btrfs per-block checksums; safetensors/GGUF have none; NVIDIA UST decouple-layout precedent)
    — see [[feedback_no_external_llm_libs]] re: research-not-install.
  - **Reader:** `Q42TensorIndex::from_q42` — validates magic/version, **verifies header + every entry
    CRC** (rejects corruption pre-bind → no WebGPU OOB), reconstructs `GgufHyperparams`, zero-copy
    `blob()`.
  - **Boot gate (`gguf_bridge.rs`):** `initialize_webgpu_engine` peeks 4 magic bytes → `GGUF`
    (`adopt_resident_mmap`) | `Q42W` (`adopt_resident_q42`). q42 path validates, reserves GEMM/KV from
    header hyperparams, maps GEMM-role blobs into `Mc8WeightArenaBufs` (`mc8_upload_resident_from_q42`,
    `q42_role_to_mc8`). New engine field `q42_resident`.
  - **Verified:** native test PASS (`290 tensors, 32 layers, blob@32768`, reader round-trip +
    hyperparams + **CRC tamper rejection**); wasm build compiles clean.
  - **Blob bit-rot integrity DEFERRED** (lazy/sampled — hashing the 250MB blob at index defeats
    zero-copy). **Next:** inference-from-`.q42` hot path (source per-tensor params from manifest, not
    `GgufTensorIndex`) so a q42 boot can decode; then JS ingest pipeline (compile→OPFS→boot from q42).

- **2026-06-19 (al)** — **Phase 4: AOT GGUF → `.q42` weight-container compiler v1 — done + verified.**
  - **New:** `src/q42_weight.rs` — `compile_gguf_to_q42(input, page_log2) -> Vec<u8>`; structs
    `Q42WeightHeader` (96B) / `Q42TensorEntry` (80B) `#[repr(C, align(16))]`, size-asserted; explicit
    little-endian serialization; `b"Q42W"` magic (sibling of the semantic `.q42`, never collides).
    Reuses `GgufTensorIndex::from_gguf` + the per-role/per-layer page layout from
    `mc8_upload_all_resident_weights`. wasm export `compileGgufToQ42` in `wasm_llm.rs`; module
    registered in `lib.rs` (`not(wasm32) || wasm-llm`).
  - **Architect decisions baked:** page_log2 (default 16K) · raw NQuin hot manifest + reserved cold
    CBOR-LD section · ALL tensors (`layer=0xFFFF` global sentinel) · little-endian + version gate.
  - **Verified (native unit test, no browser/no external libs):**
    `q42_weight::tests::compile_smollm2_to_q42_layout` PASS — `290 tensors, 32 layers, blob@32768`
    (16K-aligned), every tensor blob 16K-aligned + in-bounds, 258 MB. (`output.weight` tied → not
    double-counted; runtime projects via the included `token_embd`.)
  - **Next:** runtime `.q42` reader (bind-by-offset / mmap → resident arena, skip GGUF parse); JS
    ingest (compile-once → OPFS via Phase 3 writer → load `.q42`); cold CBOR-LD ontology; optional
    Web-Worker compiler farm. wasm export not yet rebuilt/deployed (no consumer yet).

- **2026-06-19 (ak)** — **Phase 3: OPFS model caching (JS layer) — done + verified in harness.**
  - **New:** `docs/js/opfs-model-cache.js` (`loadGgufCached`, `clearOpfsModel`, `clearAllOpfsModels`).
    Streaming `fetch.body.pipeThrough(progressCounter).pipeTo(FileSystemWritableFileStream)` → no
    >250MB JS-heap blob; atomic `.part`→`move()` promotion gated on bytes==Content-Length; falls back
    to buffered fetch on any OPFS error. Engine contract unchanged (`Uint8Array`).
  - **Wired:** `docs/wasm-llm-test.html` `getModelBytes()` (replaces the failing Cache-Storage
    `cache.put`), progress → `#loadstat`, "Clear cache" purges OPFS too. **JS-only, no wasm rebuild.**
  - **Verified (headless Chrome):** miss → streamed 258.1 MB → boot (1 `.gguf` net req); reload →
    **OPFS HIT 246 ms, 0 net req** → boot. Old `cache.put` large-entry failure gone.
  - **Pending:** port to `online-llm-demo.html` + `llmdemo/index.html` (was architect-gated on harness
    proof — now proven). Chunked OPFS→wasm mmap = Phase 4.

- **2026-06-19 (aj)** — **MC7 ChatML investigation: root cause = greedy early-EOS, NOT an engine bug.**
  - Phase 2B banked first: instrumentation stripped, clean rebuild holds gate (TTFT 3849/4001 ms,
    `Paris.` ✅), committed `850ac3b1`, tagged **`v0.0.18-wasm-gpu-phase2b-closed`**.
  - **ChatML re-test (current build):** `What is the capital of France?` → **`The capital of
    France<|im_end|>`** — fragmentary (parrots the subject, stops before "Paris").
  - **Bisect (decisive):**
    - **CPU path == GPU path** — byte-identical fragment ⇒ NOT the GPU manifold (3w/3x/3y innocent);
      architect vector 3 (22- vs 5-token attn mask) ruled out.
    - Tokenization HF-parity (test `smollm_tokenizer_audit_vs_hf_reference`, 22 IDs) + BOS correct
      (no double-BOS; `<|im_start|>`=BOS=1) ⇒ vectors 1 & 2 ruled out.
    - System message added → **no change** (not a missing-system-prompt issue).
    - **Primed assistant turn** (`…assistant\nThe capital of France is`) → **` Paris.<|im_end|>`** ✅
      ⇒ engine math correct; model knows the fact in-context.
  - **Root cause:** from a bare `assistant\n` turn, greedy decode emits `<|im_end|>` one token early
    (after "…France", instead of "is") — a tiny-model greedy artifact, not a tokenizer/mask/manifold
    bug. The whole inference stack is coherent.
  - **Resolution (architect, 2026-06-19): MC7 CLOSED — "Expected Model Behavior".** No engine change.
    Decode hacks rejected (no `min_new_tokens`/EOS-suppression, no sampling in the core — the tensor
    core must emit the highest-logit token deterministically for CI gates). External ground-truth
    runner (llama.cpp/HF) **rejected** per Prime Directive #4 — internal proof (CPU≡GPU + tokenizer
    parity + primed completion) is sufficient. Proper sampler (Temp/Top-K/Top-P) deferred to the
    **agent layer, outside the engine**, for chat demos on small models. Endgame doc §3 updated.

- **2026-06-19 (ai)** — **MC8 Part 3y: eager resident upload → ✅ PHASE 2B GATE CLOSED.**
  - **Eager upload:** moved `mc8_upload_all_resident_weights` from the lazy first-prefill path into
    init (`adopt_resident_mmap`), so the one-time 219 MB upload is paid at model-load, before the
    TTFT clock. (In 3x it ran lazily *inside* the timed prefill and was uncounted — it bypasses the
    `weight_upload` accumulator.)
  - **3-run gate (naked SmolLM2-360M-Q4_K_M, `WASM_ASYNC=1`, headless Chrome / NVIDIA Ampere):**
    TTFT **3905 / 3968 / 3997 ms (avg 3957)** — **< 4500 ms** ✅; output **`Paris.`** ✅ (3/3).
    `prefill 2409 → 35 ms`; TTFT **6118 → 3957 ms** (−2160).
  - **Gate conditions:** TTFT < 4500 ✅ · coherence locked ✅ → **Phase 2B CLOSED** (per architect
    gate definition). Endgame doc §1 updated.
  - **Post-gate (not blockers):** throughput ~0.6 tok/s (~1700 ms/token) — decode forward is
    GPU-execution-bound (~400 `M=1` dispatches/forward, invariant to submits/upload). Part 3y
    Phase 2/3 (timestamp profiling → dispatch fusion) **deferred** — gate met without it. Then:
    strip `ttft_profile` instrumentation; MC7 ChatML regression.

- **2026-06-19 (ah)** — **MC8 Part 3x landed: GPU-resident weights — bottleneck is now GPU execution.**
  - **Resident weights (DONE):** `mc8_upload_all_resident_weights` uploads all 32 layers' 7 role
    tensors **once** into per-role buffers sized `stride×n_layer` (256-aligned); hot-path encoders
    bind per-layer sub-ranges (`mc8_weight_binding`). `[MC8] resident weights uploaded once:
    219.7 MB`; `[PROFILE] weight_upload total_MB=0.0` (per-forward upload eliminated ✅).
    Coherence `Paris.` ✅. Approach A (architect-approved).
  - **3-run gate (naked SmolLM2-360M-Q4_K_M, WASM_ASYNC=1):** TTFT **5942 / 6046 / 6365 ms**
    (avg ~6118). Prefill **3276 → ~2409 ms** (−870, the upload removal). **Gate NOT met.**
  - **Why TTFT didn't move:** removing decode's 1542 ms upload just exposed the GPU-drain wait —
    decode readback **904 → ~3200 ms** (first token also absorbs the one-time 219 MB resident-upload
    drain + deferred prefill GPU compute, since prefill never reads back).
  - **Decisive finding:** adapter is a **real NVIDIA Ampere GPU** (not software; maxBuffer 2 GB).
    Yet a single-token decode forward = **~1700 ms** (steady-state, 0.5 tok/s) and is **invariant**
    to submit count (416→64, Part 3w) and weight upload (resident, Part 3x). CPU encode per decode
    ≈ 35 ms. ⇒ decode is **GPU command-execution-bound** (~400 small M=1 dispatches/forward), not
    data-/submit-/upload-bound. Arithmetic is trivial (~720 MFLOP/token) so this is dispatch/round-
    trip overhead in the Dawn-wasm path, not FLOPs.
  - **Open (needs architect):** next vector — (a) GPU **timestamp-query profiling** to split
    dispatch-launch vs readback vs shader; (b) **dispatch fusion** (fewer/bigger compute passes per
    layer); (c) **eager resident upload at load** (removes ~219 MB drain from first-token TTFT only);
    (d) reconsider whether <4500 ms is reachable for M=1 autoregressive decode without shader rework
    / batched (speculative) decode. Parts 3w+3x are correct and banked regardless.

- **2026-06-19 (ag)** — **MC8 Part 3w landed + Part 3x root cause: per-forward weight re-upload.**
  - **Decode super-arena port (DONE, coherent):** `dispatch_transformer_forward_async` rewritten to
    reuse `mc8_stage_prefill_layer_super_arena` + `encode_prefill_q_ffn_tail_fused` at `n_tokens=1`
    (dynamic-offset uniforms + 7 disjoint weight buffers). Decode flushes **416 → 64 (2/layer)**;
    output **`Paris.`** ✅. `encode_transformer_layer_gpu` retired (`#[allow(dead_code)]`).
  - **TTFT UNCHANGED (~6.2–6.6 s).** Submit savings (~750 ms CPU) moved into the final readback
    (210 → 904 ms): decode was **never submit-bound** — submits overlapped GPU work. `dec0_fwd` wall
    conserved (~2523 ms); throughput still 0.5 tok/s.
  - **Root cause (proven, new instrumentation):** `[PROFILE] weight_upload total_MB=416.6
    total_ms=2337.8 prefill=795.3 dec0_fwd=1542.5`. The engine **re-uploads ~208 MB of model
    weights per forward pass** (`write_weight_role`/`write_weight_words` → `queue.write_buffer` of
    each layer's K/V/Q/O/gate/up/down, 7 buffers overwritten per layer). **38% of TTFT**; **61% of
    decode forward**; ~100% of the 0.5 tok/s ceiling.
  - **Recommended Part 3x (pending architect):** **GPU-resident weights** — upload each layer/role
    weight once (at load / first use), reuse across forwards; eliminate per-forward `write_buffer`.
    Projected TTFT ≈ setup 371 + prefill ~2407 + decode ~981 + argmax 98 ≈ **~3.9 s** (under gate) +
    large throughput gain. **Risk:** ~208 MB resident GPU (browser `maxBufferSize` /
    `maxStorageBufferBindingSize`) → needs per-layer buffers or one big buffer + offsets.

- **2026-06-19 (af)** — **MC8 Part 3w: TTFT profile attribution (architect-approved profile-first).**
  - **Instrumentation (wasm-only, reverts after gate):** `ttft_profile` module in `gguf_bridge.rs`
    (thread-local readback-ms / readback-count / submit-count accumulators); `await_wgpu_map`
    times each `map_async` round-trip; `mc8_flush` counts compute submits; phase wall-timers +
    `[PROFILE]` log in `wasm_llm.rs::run_inference_async`. Harness surfaces `[PROFILE]` lines.
  - **Build:** `web-sys` += `Performance`; canonical Git-Bash wasm-pack (8 MB stack); deployed.
  - **3-run profile (naked SmolLM2-360M-Q4_K_M, `WASM_ASYNC=1`, `MC8_FUSED_PREFILL_TAIL=true`):**
    TTFT avg **6346 ms** (6256/6507/6275); output **`Paris.`** ✅ all runs.

    | Phase | avg ms | % | compute flushes | readbacks |
    |-------|--------|---|-----------------|-----------|
    | setup (tokenize + `GgufTensorIndex::from_gguf`) | 381 | 6% | 0 | 0 |
    | prefill (4 tok × 32 layers, batched) | 3276 | 52% | **64** (2/layer) | 0 |
    | decode step-0 forward (1 tok × 32 layers) | 2590 | 41% | **416** (13/layer) | 1 (210 ms) |
    | decode step-0 argmax (CPU `stack_gemm_quant`) | 99 | 2% | 0 | 0 |

  - **Findings (overturns prior assumptions):**
    1. **Readback is negligible** — 210 ms / **1** readback on the whole first-token path. Q1.2
       REFUTED: argmax is CPU (Part 3k gate), 0 GPU readbacks. Consolidating vocab-GEMM readback
       is NOT a TTFT lever.
    2. **Decode path is un-optimized and submit-bound.** First-token decode forward = **416
       submits (13 flushes/layer)** vs prefill's 64 (2/layer). Parts 3o–3u optimized
       `encode_prefill_q_ffn_tail_fused` (prefill) only; the decode path
       (`encode_attn_ffn_tail_gpu` = 11 flushes + `encode_transformer_layer_gpu` = +2) was never
       given the super-arena/dynamic-offset/weight-arena treatment. Confirmed by grep + 416/32=13.
    3. **Decode per-token (2590 ms) ≈ 3× prefill per-token (~820 ms)** at equal depth — pure
       submit overhead. Also explains **0.5 tok/s** throughput (every token pays the 13-flush tax).
    4. **Prefill (52%) is compute-bound** — already 2/layer; documented `Mc8NormWeightArena` +
       cross-layer merge would shave little (~32 submits ≈ ~0.2 s).
  - **Recommended redirect (pending architect):** port the MC8 super-arena to the **decode path**
    (`encode_attn_ffn_tail_gpu` → ~2 flushes/layer) — projected ~1.5–2 s off TTFT (toward/under
    4500 ms) **and** ~3–5× throughput. **Hold** `Mc8NormWeightArena`/prefill-merge. KV-flush probe
    (Q2) folds into the decode port (same K/V→Q visibility floor).

- **2026-06-19 (ae)** — **MC8 Part 3o: TTFT Collapse (zero-flush batched Q-SDPA).**
  - **`encode_attention_batched_q_prefill`:** single uniform params + batched per-token KV masks; `dispatch(n_head, n_tokens, 1)`; per-Q `mc8_flush` removed from fused tail.
  - **WGSL:** `out_stride_elems`, `q_mask_token`, 2D Q grid (`wg_id.y` = token index when `num_tokens_in_batch > 1`).
  - **Storage-array params rejected:** WGSL 96-byte struct stride vs Rust 84-byte pack → corrupt multi-token params.
  - **Harness:** TTFT **5638–6534 ms** (↓ from 3n **7925 ms**); validation clean; output **not `Paris.`** (EU garble). 3n sequential Q+flush **also** failed `Paris.` on same rebuild — coherence bisect required.
  - **Deferred:** batched GEMM $M=n_tokens$; Phase 2B still open.

- **2026-06-19 (ad)** — **MC8 Part 3h: prefill/embedding reconciliation (replay trap).**
  Architect H: prefill first. Pure CPU vs GPU KV @ L0 **bit-exact** (replay trap broken).
  token_embd ✅. **L1_input_hidden** GPU -0.529 vs CPU 0.179 — **first divergence** in batched
  prefill Q+FFN tail, not KV or embedding.

- **2026-06-19 (ac)** — **MC8 Part 3g: FFN chain diff (gate/up/SwiGLU/down/residual).**
  Architect J: flush audit then diff. Audit: gate→work_buf/up→ffn_buf disjoint, flush before
  silu_mul ✅; base_save pre-ffn_norm ✅. All FFN tensors bit-exact; `ffn_residual=0.423168`
  matches depth bisect. **No FFN leak** — MC7 ~1.09 is cross-manifold reference mismatch.

- **2026-06-19 (ab)** — **MC8 Part 3f: L0 mid-layer diff (o_proj + attn-residual + ffn_norm).**
  Architect I: o_proj+residual before FFN. Probes @ decode step 1 L0: pristine_hidden,
  o_proj, post_attn_residual, ffn_norm all **bit-exact**. `first_divergence=none` for all three
  phases. L0 post-FFN `h[0]=0.423` unchanged — fault is **gate/up/SiLU/down or FFN residual**
  after `ffn_norm`.

- **2026-06-19 (aa)** — **MC8 Part 3e: L0 Q/K/Attn_Out targeted diff.**
  Architect H: L0 not L1. Probe @ decode step 1 layer 0: attn_rmsnorm/K_rope/Attn_Out
  **bit-exact** vs CPU; `mask_active=0`; causal softmax ruled out. False K divergence was
  probe artefact (`StaticKvCacheArena` missing `COPY_SRC`). L0 post-FFN `h[0]=0.423` unchanged —
  gap is **post-SDPA** (o_proj / residual / FFN). Harness: L31=20.87, TTFT ~7.6s.

- **2026-06-19 (z)** — **MC8 Part 3d: batched prefill audit.**
  Architect G: prefill audit before Q/K diff. WGSL K/V already uses
  `abs_pos = batch_start_token_idx + token_in_batch`; RMSNorm per-row ✅; flat-RoPE hypothesis
  **not confirmed**. Patched prefill `attn_input` handoff (`prefill_scratch[t]`→`aux_buf`),
  Q `abs_pos` via `batch_start+offset`, flush after batch RMSNorm. Harness unchanged:
  L0=0.423, L31=20.87. Commit `3613b2e4`.

- **2026-06-19 (ad)** — **MC8 Part 3n: Batched numerics isolation.**
  - **Root cause:** Q SDPA `attention_params`/`mask` queue-write race — all tokens used last token's causal horizon.
  - **Fix:** `mc8_flush` after each per-token Q in `encode_prefill_q_ffn_tail_fused`.
  - **Audits:** elementwise per-token loops OK; GEMM $M=1$ per dispatch OK for current WGSL.
  - **Harness:** **`Paris.`** @ **7925 ms** TTFT; `MC8_FUSED_PREFILL_TAIL=true`; validation clean.

- **2026-06-19 (ac)** — **MC8 Part 3m: TTFT ping-pong.**
  - **`prefill_work_buf_a`/`_b`** allocated; `encode_prefill_q_ffn_tail_fused` rewritten with A/B ping-pong + pristine snapshot.
  - **WebGPU validation:** residual scratch moved to `prefill_scratch_buf`; weight-stage `mc8_flush` retained (encoder-scope aliasing rule).
  - **Harness:** per-token hot path **`Paris.`** @ **7906 ms** TTFT; fused path validation clean but output garbled @ ~8.3s.
  - **`MC8_FUSED_PREFILL_TAIL = false`** until batched math audit passes.

- **2026-06-19 (ab)** — **MC8 Part 3l: Argmax audit.**
  - **No `fused_argmax.wgsl`** — `_mc8_fused` was batched vocab GEMM + single readback + CPU argmax.
  - **Root cause:** `queue.write_buffer` weight uploads race on shared `gemm_weight_buf` across chunks in one submit scope (pt3c analogue). `mc8_flush` between chunks still garbled.
  - **Part 3k nuance:** sync `dispatch_output_argmax_chunked` uses **`stack_gemm_quant` (CPU)** on wasm32, not GPU GEMM.
  - **Fix:** per-chunk `dispatch_gemm_raw_into_async` + streaming CPU argmax.
  - **Harness (`WASM_ASYNC=1`):** TTFT **7537 ms**; **`Paris. The capital of France…`** ✅.
  - **Next:** TTFT ping-pong.

- **2026-06-19 (aa)** — **MC8 Part 3k: Argmax isolation.**
  - **Gate:** `dispatch_output_argmax_chunked_async` → CPU `dispatch_output_argmax_chunked` (not `_mc8_fused`).
  - **Harness (`WASM_ASYNC=1`, naked):** TTFT **7856 ms**; output **`Paris. The capital of France…`** ✅.
  - **Conclusion:** GPU decode manifold (`encode_transformer_layer_gpu` + GPU prefill) coherent; regression was **fused GPU argmax only**.
  - **Deferred:** TTFT ping-pong (`prefill_work_buf_A`/`_B`) until `_mc8_fused` debugged.
  - **Phase 2B NOT CLOSED** (fused argmax + TTFT &lt;4s).

- **2026-06-19 (z)** — **MC8 Endgame: TTFT push + argmax fusion + probe cleanup.**
  - **`work_aliases_hidden`** re-landed in `encode_attn_ffn_tail_gpu`; prefill passes `true`.
  - **Attn-residual scratch fix:** never use `prefill_scratch_buf` as residual scratch during prefill tail (aliases batched RMSNorm rows) — use `ffn_buf`.
  - **`mc8_log_*` probes removed**; `dispatch_transformer_forward_async` simplified (no `l0_probe_step1` / `decode_emb_probe`).
  - **GPU argmax fusion wired:** `dispatch_output_argmax_chunked_async` → `_mc8_fused`.
  - **TTFT infrastructure:** `prefill_work_buf`, offset encoders, `encode_prefill_q_ffn_tail_fused` (not hot-path — WebGPU buffer aliasing on in-place strided GEMM).
  - **Harness:**
    | Path | TTFT | Output |
    |------|------|--------|
    | `WASM_ASYNC=0` (`inferWasmStreaming`) | ~8.8s | **`Paris. The capital of France…`** ✅ |
    | `WASM_ASYNC=1` (`inferWasmAsync`) | ~7.6s | `prolesİİ…nownow…` ❌ |
  - **Bisect:** CPU prefill + GPU decode still garbled → GPU decode also suspect.
  - **Phase 2B NOT CLOSED.**

- **2026-06-19 (y)** — **MC8 Part 3c: KV indexing + weight-buffer race.**
  Architect F: KV layout audit first. Layer stride/uniforms ✅. Root cause: K and V shared
  `gemm_weight_buf` without `mc8_flush` — K ran with V weights. Fix: flush K→V, gate→up.
  L31 **271.7→20.87**; L0 **1.011→0.423** (false positive gone). Output → partial English.
  Commit `64194425`.

- **2026-06-19 (x)** — **MC8 Part 3b: depth bisect + FFN residual audit.**
  Architect E: FFN chain first. `add_residual_main` OK; scratch isolation + extra flushes.
  Bisect: L0=1.011, L1=-1.662, L31=271.7 (unchanged). L1 jump → cross-layer fault, not FFN creep.
  Commit `ba357389`.

- **2026-06-18 (w)** — **MC8 Part 3: GPU prefill manifold unification + L0 step-1 lock.**
  - Wired `dispatch_prefill_chunk_async` → `dispatch_prefill_chunk_async_mc8_gpu` (CPU fallback blocked).
  - L0 probe gated to **decode step 1 only** (`dispatch_transformer_forward_async(..., l0_probe_step1)`).
  - **Harness (naked Q4_K_M):** GPU prefill OK; L0@L0 step1 `h[0]=**1.011**` (target ~1.09); L0@L31 `h[0]=**271.7**`; TTFT **7942 ms**; output still garbled.
  - **Finding:** layer-0 variance gate met; depth accumulation (layers 1–31) is the remaining fault.
  **Next:** Part 3b — layer bisect + elementwise residual audit.

- **2026-06-18 (v)** — **MC8 Part 2 LANDED: NEOX RoPE WGSL alignment + fused decode path.**
  - **`AttentionGpuParams`:** added `rope_scale`; `rope_theta_base` ← `effective_rope_freq_base()` (100k).
  - **`fused_attention.wgsl`:** `apply_rope_neox` split-half `(i, i+half_dim)`; removed consecutive-pair `rotate_rope_pair`.
  - **Hot path:** `dispatch_transformer_forward_async` → `encode_transformer_layer_gpu` (upload, per-layer encode, `mc8_flush`, readback).
  - **L0 probe:** decode step 1 `h[0]=0.930` (target ~1.09); step 0 miss; steps 2+ drift to 2–3.
  - **Harness:** TTFT **8961 ms** (↓ from ~11s MC7); zero WebGPU errors; output **garbled** (`KeyNotKeyNot…`) vs MC7 `Paris is the capital of France`.
  - **Build trap:** wasm without `-zstack-size=8388608` → instant OOB; use `package-qualia-wasm.ps1` or §BUILD `RUSTFLAGS`.
  - Commits: pt1 `60ba2451`, pt2 `4f506932` on `0.0.18`.
  **Next:** MC8 Part 3 — L0 CPU/GPU parity @ decode step 1; GPU prefill K/V; elementwise audit.

- **2026-06-18 (u)** — **MC4 CLOSED: CPU stent checkpoint + Phase 2B async plumbing.**
  - Trimmed MC2/MC3 diagnostic `wlog` (`[prefill_layer] ENTER`, `[MC3f]` IDs, `[MC3e]` argmax,
    `[MC2] SDPA L1`, etc.); retained structural GUARD/OOB/FAILED logs.
  - **Phase 2B landed (not wired to inference yet):**
    `dispatch_attention_pass_async`, `dispatch_attention_layer_async`,
    `dispatch_prefill_layer_batch_async`, `dispatch_ffn_block_pre_norm_async` (SwiGLU),
    `dispatch_transformer_layer_async`, `dispatch_transformer_forward_async`.
  - Tag: **`v0.0.18-wasm-cpu-fallback-stable`** — mathematically proven CPU fallback.
  **Next:** MC6 — wire async forward into `infer_wasm_streaming` + JS `await` per step.

- **2026-06-18 (t)** — **MC3h CLOSED: Q6_K signed-scale dequant fix.**
  - **Root cause (not Q4_K):** SmolLM2 Q4_K_M uses Q8_0/Q5_0/Q6_K mix; `blk.*.ffn_down.weight` is
    **Q6_K** (type 14). CPU `dequant_q6_k_block` read scales as **unsigned** `u8`; llama.cpp uses
    **signed** `int8_t` — 314/2560 weights per row flipped on negative scales.
  - **Fix:** `dequant_q6_k_block` now casts via `BlockQ6K.scales: [i8; 16]` (WGSL already used
    `i8_from_u8`). `dequant_q4_k` + `ggml_row_bytes` audited — already correct (256 elems / 144 B).
  - Unit test `q6_k_dequant_matches_gguf_smollm2_ffn_down_row0` — row-0 parity vs llama.cpp.
  **Harness post-fix (Q4_K_M):**
  | Mode | Prompt IDs | Output |
  |------|------------|--------|
  | Naked | `[504, 3575, 282, 4649, 314]` | **` Paris.`** + coherent tail ✅ |
  | ChatML | 22 IDs (HF parity) | **`The capital of France`** + EOS ✅ |
  - `contains_nan=false`; no guard trips; TTFT naked ~54 s / ChatML ~35 s (22-tok prefill).
  **Next:** MC4 trim wlog; then Phase 2B async WebGPU compute.

- **2026-06-18 (s)** — **MC3g CLOSED: Weight tie validated; Q4_K_M dequant isolated.**
  - `logits_projection_info()` already ties `output.weight` → `token_embd.weight` when absent.
  - Unit test `smollm_gguf_output_weight_tie_probe`: both Q4_K_M + Q8_0 report
    `tied=true emb_off=0x0 dims=[960, 49152]` (offsets match).
  - `[MC3g]` wlog: `output_tied=true`; GQA `n_head=15 n_kv=5 q_heads_per_kv=3`.
  **Naked prompt harness (`The capital of France is`, 5 tok):**
  | Quant | TTFT | Output |
  |-------|------|--------|
  | **Q8_0** | ~7.6 s | **` Paris.`** + coherent repetitions ✅ |
  | **Q4_K_M** | ~8.4 s | fragmentary soup ❌ |
  **Conclusion:** weight tie + GQA + tokenizer are sound; **`dequant_q4_k` in
  `ggml_quants.rs` corrupts matmul rows** for attention/FFN/output projection.
  **Next:** MC3h — fix Q4_K block dequant (reference: llama.cpp `ggml-quants.c`).

- **2026-06-18 (r)** — **MC3f: SmolLM BPE tokenizer aligned to HuggingFace.**
  - Root cause confirmed: greedy longest-match **shredded** ChatML (38 IDs) vs HF **22**.
  - Fix: parse `tokenizer.ggml.merges` + `pre=smollm`; special-token atomicity; GPT-2
    byte-encode + BPE merge ranks; smollm pretoken regex.
  - Unit test `smollm_tokenizer_audit_vs_hf_reference` — ChatML + naked **byte-for-byte
    parity** with HF `tokenizers` crate.
  - `[MC3f] Prompt IDs` wlog; `rope_scale` parsed (SmolLM2 = 1.0); naked harness checkbox.
  **Harness post-fix:**
  - ChatML `Prompt IDs: [1, 4093, 198, 1780, 314, 260, 3575, 282, 4649, 47, 19842, 281, 582, 1890, 6330, 30, 2, 198, 1, 520, 9531, 198]` ✅
  - Naked `Prompt IDs: [504, 3575, 282, 4649, 314]` ✅
  - TTFT dropped (22 vs 38 tok prefill; naked 5 tok → ~9 s TTFT)
  - Output **still fragmentary** on both naked + ChatML ❌ → inference/math (MC3g?)
    not tokenizer shred.

- **2026-06-18 (q)** — **MC3e CLOSED: BOS injection + NEOX RoPE audit + full-vocab argmax.**
  - `GgufTokenizer::encode_prompt` — reads `tokenizer.ggml.add_bos_token`; prepends
    `bos_token_id` when enabled and absent; `llm_agent` uses `encode_prompt` on wasm path.
  - `[MC3e]` probe: `prompt tokens=38 bos_id=1 add_bos=false first_id=1` (SmolLM2 GGUF
    sets `add_bos_token=false`; ChatML encode already begins with BOS id 1 at slot 0).
  - `rope_inplace` audited: NEOX split-half `(i, i + head_dim/2)`; theta =
    `pos * base^(-2i/head_dim)` — matches architect spec.
  - `dispatch_output_argmax_chunked`: release wasm `TEST_VOCAB_CHUNK_CAP=0` → full sweep;
    `[MC3e] Argmax sweep: chunks=6/6 vocab=49152` on every decode step.
  - Unit tests: `encode_prompt_prepends_bos_when_enabled`,
    `encode_prompt_skips_duplicate_bos` pass.
  **Harness (32-token, SmolLM2-360M Q4_K_M):**
  - TTFT **~54 s**; 32 tok in **~94 s** (0.3 tok/s) ✅
  - `contains_nan=false` ✅; full-vocab argmax ✅; no guard trips ✅
  - Output **still fragmentary** (not capital-of-France):
    `Ĩ posthumwrightsaced low … Answer … little … next … get …` ❌
  **Diagnosis:** physical manifolds (RMSNorm/SwiGLU/RoPE/argmax) appear sound; remaining
  gap is likely prompt/tokenizer alignment (`tokenizer.ggml.pre=smollm`,
  `llama.rope.scaling`) or ChatML tokenization vs HF reference. **Next:** MC3f.

- **2026-06-18 (p)** — **MC3d CLOSED: RoPE theta + BPE decode + KV position probes.**
  - `GgufHyperparams.rope_freq_base` parsed from `llama.rope.freq_base` (f32); default **100_000**.
  - `cpu_attention_pass` uses `h.effective_rope_freq_base()` (was hardcoded **10_000** — 10× error).
  - `GgufTokenizer::decode`: `Ġ` (U+0120) → space; unit test + SmolLM GGUF parse test pass.
  - `[MC3d]` probes: rope at init; `token_idx`/`ctx_len` on decode steps 0/1/last.
  **Harness (32-token):** `rope_freq_base=100000` ✅; `token_idx` 37→38→… ✅;
  `contains_nan=false` ✅; output has **fragmentary English** (no raw `Ġ`) but not
  capital-of-France (`…low…Answer…little…next…get…`). **Next:** MC3e (BOS, longer budget).

- **2026-06-18 (o)** — **MC3c CLOSED: SwiGLU / SiLU on WASM CPU FFN (architect directive).**
  Replaced ReLU-gated FFN in `dispatch_ffn_block_pre_norm` (`#[cfg(wasm32)]` only):
  - `silu_inplace` — SiLU(x) = x / (1 + e^{-x})
  - Stack `gate_buf` / `up_buf` (`[f32; MAX_STACK_GEMM_DIM]`, n_ffn=2560 for SmolLM2)
  - Sequence: RMSNorm → gate GEMM → up GEMM → SiLU(gate) → gate⊗up → down GEMM → residual
  - Native path unchanged (legacy ReLU-gated FFN)
  **Harness (32-token, SmolLM2-360M Q4_K_M):**
  - `contains_nan=false` all decode steps ✅; diverse argmax IDs (not token 0) ✅
  - **L0 post-FFN variance normalized:** `h[0]=47.7` (MC3b ReLU) → `h[0]≈1.09` (MC3c SwiGLU) ✅
  - TTFT **~47 s**; 32 tok in **~85 s** (0.4 tok/s) ✅
  - Output still **garbled** (`eerymourĠacceleroleum…`) — not capital-of-France coherent ❌
  **Deploy fix:** `package-qualia-wasm.ps1` now copies **only** `qualia_core_db_*` → `qualia.*`
  (legacy `qualia_wasm.js` was overwriting `qualia.js` and breaking WASM init).
  **Next (MC3d):** parse `rope_theta` from GGUF KV; audit tokenizer decode; try 64–128 tok budget.

- **2026-06-18 (n)** — **MC3b CLOSED: Pre-Norm RMSNorm on WASM CPU path (architect directive).**
  Implemented zero-heap Pre-Norm residuals in `gguf_bridge.rs`:
  - `rms_norm_inplace`, `dequant_norm_row_into`, `prepare_pre_norm_input` (stack `[f32; 4096]`)
  - **Attention:** `attn_norm.weight` applied before K/V/Q GEMM (prefill batch + decode)
  - **FFN:** `dispatch_ffn_block_pre_norm` — `ffn_norm.weight` before gate/up/down, residual add
  - **Final:** `apply_output_norm_inplace` (`output_norm.weight`) before vocab projection;
    wired in `llm_agent.rs` (wasm32 decode paths)
  - `gguf_sharder.rs`: `LayerTensors.attn_norm` / `ffn_norm`; `GgufTensorIndex.output_norm`
  **Harness (32-token budget, SmolLM2-360M Q4_K_M):**
  - `[MC3] Final Logits Probe: contains_nan=false` on all 32 decode steps ✅
  - Argmax returns real IDs (e.g. `best_token_id=10989 max_logit=16.33`) — no token-0 spam ✅
  - TTFT **~47–50 s**; 32 tok in **~85–88 s** (0.4 tok/s) ✅
  - Output still **garbled** (`westernphansthelessivisticadium…`) — not coherent English;
    L0 post-FFN variance still spikes (`h[0]≈48`) but deeper layers + `output_norm` keep
    logits finite. **Suspect:** FFN uses `relu_inplace` but SmolLM2 is **SwiGLU/SiLU** → MC3c.
  **Deploy fix:** `package-qualia-wasm.ps1` map order — legacy `qualia_wasm_bg.wasm` was
  overwriting fresh `qualia_core_db_bg.wasm` → stale `qualia_bg.wasm` caused init trap.
  Harness fix: removed early-exit on `layer=31 COMPLETE` (wait for `DONE:` only).

- **2026-06-18 (m)** — **MC3 OPEN: NaN / logit probes (architect directive).**
  Hypothesis: `<|endoftext|>` spam = argmax stuck at token 0 due to all-`NaN` logits.
  Probes added (`#[cfg(wasm32)]`): `[MC3] Final Logits Probe` before argmax loop;
  `[MC3] Attn Out[0]` after SDPA V-sum; `[MC3] hidden pre/post-attn` at L0/L31;
  `[MC3] Argmax result`. **Step 3 finding:** `LayerTensors` / CPU path has **no
  `attn_norm` / `ffn_norm` (RMSNorm)** — weights exist in GGUF but are never applied.
  **Harness trace (entry n):** Step 1 **CONFIRMED** — `[MC3] Final Logits Probe:
  contains_nan=true l[0]=NaN l[1]=NaN` (all 32 decode steps). Step 2 — SDPA L0
  prefill/decode clean (`Attn Out[0]≈0.03`, `has_nan=false`); **L31 prefill SDPA
  all-NaN** (`pos=0..36`); decode tok=37: L0 post-ffn `h[0]=45.8` (no norm → blow-up),
  L31 pre-attn already `NaN`. Step 3 — **`attn_norm`/`ffn_norm` not in `LayerTensors`
  or CPU path** (primary suspect). Argmax returns `None` → fallback `top_i=0` → token 0
  (`<|endoftext|>`). **Next fix:** CPU RMSNorm before attn + FFN.

- **2026-06-18 (l)** — **MC2b E2E VALIDATION (prefill + SDPA pass; coherence partial).**
  Headless harness (`wasm-mc2-test.mjs`) with wasm `DECODE_TOKEN_BUDGET=32`:
  - `[kv_cache] OK` at init ✅
  - `[prefill_layer] K/V passes OK` all layers; `layer=31 COMPLETE` ✅
  - `[MC2] SDPA L1` non-zero (e.g. `l1=51.49` at layer=0 pos=0) ✅
  - TTFT **56317 ms** (~56 s) — real CPU SDPA, not sub-600 ms ✅
  - No `UnsupportedType`, no `PREFILL chunk FAILED`, no guard trips ✅
  - Output: **32 × `<|endoftext|>`** (not `"firehose"`); 0.3 tok/s decode — **coherence
    gate not met** (likely argmax/EOS or RoPE/weight alignment — MC3).
  Prior 48-min run (2048-token budget) exited -1 with no captured logs; fixed harness
  streams console to `agent-tools/wasm-mc2-test-console.log`.

- **2026-06-18 (k)** — **MC2b PREFILL ROOT CAUSE IDENTIFIED + FIXED (Q5_0).**
  - Harness after F5/KV fix still logged `[prefill_layer] FAILED fetch attn_k bytes:
    UnsupportedType` at layer 0 — **before** any K/V projection ran.
  - **Exact cause:** SmolLM2 `Q4_K_M` stores **176 tensors** (all `attn_q`, `attn_k`,
    `attn_output` weights) as **GGML type 6 = Q5_0**. `ggml_quants.rs` had no Q5_0 in
    `ggml_block_layout` / `ggml_row_bytes` / `dequantize_row_into` → `tensor_byte_len`
    returned `None` → `fetch_tensor_bytes` failed.
  - **Fix:** Added `GGML_TYPE_Q5_0` (22-byte blocks, 32 elems), `dequant_q5_0` (current
    llama.cpp `dequantize_row_q5_0` — `uint32_t qh` + `xh_1` at `j+12`), wired into
    `ggml_gpu_quant_supported` on wasm. Cross-validated vs gguf-py on `blk.0.attn_k.weight`.
  - **Verified:** `ggml_row_bytes(Q5_0, 960) = 660`; full `blk.0.attn_k.weight` = 211200 B;
    `cargo test -p qualia-core-db --lib ggml_quants` passes; wasm rebuilt/deployed.
  - **Re-test:** Harness loads OK; full inference with CPU SDPA + 2048-token decode budget
    exceeds practical headless timeout (expected — TTFT should be **seconds** once prefill
    runs). Next harness run should show `[prefill_layer] K/V passes OK` not `UnsupportedType`.

- **2026-06-18 (j)** — **ARCHITECT DIRECTIVES MC2b (approved).**
  - **`adopt_resident_mmap`:** Approved permanent for wasm init. `initialize_webgpu_engine`
    must call it (not `gguf_mmap = Some` alone). Fail init if KV layout/CPU mirror missing.
  - **RoPE:** **Revert to NEOX** (split-half). SmolLM2/Llama requires NEOX against raw GGUF;
    `fused_attention.wgsl` consecutive-pair is a known native/WGSL divergence (MC3).
  - **Prefill root cause (suspected):** `ensure_kv_cache` gated on native `VramLedger`
    `can_allocate_in_universe` — on wasm the ledger never receives adapter budget, so
    allocation could silently no-op → `kv_layout` stays `None` → prefill dies at first
    `dispatch_prefill_layer_batch` check (before K/V logs). **Fix:** wasm bypass of ledger
    gate in `ensure_kv_cache`; `[kv_cache] OK` / `[prefill_layer] FAILED kv_layout is None`
    diagnostics added.
  - **Next validation:** harness must show `[kv_cache] OK` at init, `[prefill_layer] K/V passes OK`,
    `[MC2] SDPA L1` non-zero, TTFT **seconds** (CPU SDPA acceptable for Phase 2A).

- **2026-06-18 (i)** — **MC2 SDPA CODE LANDED; PREFILL STILL BLOCKED (F5/F6).**
  Implemented full SDPA in `cpu_attention_pass` proj_kind=0: GQA via `h.q_heads_per_kv()`,
  scaled dot-product over `0..=pos`, numerically-stable softmax, V-weighted sum into
  `readback_out` (then existing `attn_output` wo `stack_gemm_quant`). Moved `att_scores`
  inside Q arm only. KV access via raw pointer + `from_raw_parts(_mut)` (avoids
  `invalid_reference_casting` with `mut readback_out`). Discovered **F5:** wasm
  `initialize_webgpu_engine` never called `adopt_resident_mmap` → no KV/GEMM arenas;
  fixed to `engine.adopt_resident_mmap(gguf_data)?`. Discovered **F6:** `dispatch_prefill_chunk`
  return ignored in `llm_agent.rs`; added `[llm] PREFILL chunk FAILED` log. Harness after F5
  fix: init OK, but `[llm] PREFILL chunk FAILED pos=0 n=37`, TTFT ~560 ms, output still
  `"firehose"` × 2048, no OOB, no `[cpu_attn]` guard trips. `[MC2] SDPA L1` diagnostic never
  fired → Q SDPA not reached (prefill dies before q_ffn). Changed `rope_inplace` to
  consecutive-pair (WGSL-aligned); revert/test NEOX in MC3 if needed. Headless test harness:
  `agent-tools/wasm-mc2-test.mjs` (Playwright + local static server).
  ➡️ **MC2b next:** read `[attn_pass] GUARD` / `[prefill_layer] K|V pass FAILED` on next
  harness run → fix layer-0 prefill → re-test SDPA → capital-of-France coherence gate.

- **2026-06-18 (h)** — **MC2 session started (architect spec: SDPA + softmax + V-sum).**
  Scope: replace Q zero-stub with real attention; route through `attn_output` wo; build +
  harness verify. See entry (i) for outcome.

- **2026-06-18 (g)** — **PHASE 2A MICRO-COMMIT 1 DONE (CPU K/V projection + RoPE + KV
  write).** Added `cpu_attention_pass` (`#[cfg(wasm32)]`, in `gguf_bridge.rs`):
  `dispatch_attention_pass` now early-returns into it on wasm; the GPU dispatch/`map_async`
  body is wrapped in `#[cfg(not(wasm32))]`. CPU path projects each token's hidden→Q/K/V via
  `stack_gemm_quant`, applies NEOX RoPE (`rope_inplace`, base `ROPE_FREQ_BASE=10000`), and
  writes K/V into `kv_cache_cpu` using `layout.k_index`/`v_index` (mutated via a sound
  single-thread unsafe `&mut`). Q output **stubbed to zero** for MC1. Verified on harness:
  `SmolLM2-360M` ran prefill(37 tok)+decode(295 tok), **no OOB, no `[cpu_attn]` guard
  trips** → KV index math aligned. Output still `"firehose"` (Q stubbed) as expected.
  ➡️ **MC2 next:** SDPA over cached K (scaled dot-product) + numerically-stable softmax +
  V-weighted sum per head → write real Q output, then route through `attn_output` projection.
  Expect output to flip to coherent English. RoPE convention (NEOX vs NORM) + rope base to be
  confirmed against `fused_attention.wgsl` if output is incoherent.

- **2026-06-18 (f)** — **PHASE 1 COMPLETE (OOB resolved, stable).** Re-ran with the noisy
  per-GEMM logs removed (far fewer allocations) → still runs (**290 tok, TTFT 145 ms**),
  same garbage output. So **not a heisenbug** — the real fix is the defensive bounds guards
  added at the entry of `dispatch_gemm_into` (`n_in > input.len() || n_out > out.len()`) and
  `stack_gemm_quant` (`n_in > MAX_STACK_GEMM_IN`). These were missing, so a bad
  dimension fed an out-of-range slice/`write_buffer`. **Keep these guards permanently.**
  Remaining diagnostic `wlog` structural logs can be trimmed before final commit.
  ➡️ **Now Phase 2 Option A (CPU attention)** to replace the dead wasm GPU attention and fix
  the garbage output.
- **2026-06-18 (e)** — **PHASE 1 BREAKTHROUGH: inference no longer traps; it runs
  end-to-end.** With the instrumented build, `SmolLM2-360M-Q4_K_M` generated **267 tokens,
  TTFT 176 ms**. Trace confirms the wasm CPU path: `gemm_into n_in=960 n_out=2560` (FFN) +
  `stack_gemm n_in=960 n_out=8192` (vocab projection) fire thousands of times — **FFN +
  output projection compute correctly on CPU**. BUT output is **garbage/repetitive**
  (`"firehosefirehose…"`) — exactly the **F1 symptom: attention is skipped on wasm**
  (`dispatch_attention_pass` Q-readback is `#[cfg(not(wasm32))]` → returns false → attention
  no-op). So the model = embeddings + FFN + projection, no attention → repetitive nonsense.
  - *OOB status:* the ~10 ms trap stopped once instrumentation landed. All observed
    `stack_gemm`/`gemm_into` dims are within caps (`n_in=960`), so either the added
    `n_in > MAX_STACK_GEMM_IN` / `n_in > input.len()` guards caught an edge call, or the
    trap was a layout-sensitive heisenbug in the **dead GPU attention path** (`map_async`
    on wasm). **Either way it lives in the attention path we are about to replace** — so
    Phase 2 Option A (CPU attention) should both fix output AND remove the OOB source.
  - *Next:* implement Phase 2 Option A (CPU attention: RoPE + KV write to `kv_cache_cpu` +
    SDPA + softmax; reuse `stack_gemm_quant` for Q/K/V/output projections), gated wasm-only.
- **2026-06-18 (d)** — Baseline committed `7304a52e` on branch `wasm-llm-inference-fixes`
  (0.0.18). **Phase 1 in progress:** added `web-sys` `console` feature + a `wlog()` helper
  (`gguf_bridge.rs`, cfg-gated) and entry/boundary logs across the prefill path:
  `llm_agent` PREFILL/DECODE phase markers → `dispatch_prefill_layer_batch` (entry, post-K/V,
  per-token, complete) → `dispatch_attention_q_ffn_token` (entry) → `dispatch_attention_pass`
  (entry w/ `proj_kind`) → `dispatch_gemm_into` / `stack_gemm_quant` (entry w/ dims; added a
  `n_in > MAX_STACK_GEMM_IN` guard). Rebuilt instrumented wasm; running harness to capture
  the last log before the trap. **All instrumentation is temporary — remove once fixed.**
- **2026-06-18 (c)** — Decisions recorded (above). Bumped `qualia-core-db` → **0.0.18**,
  rebuilt/redeployed wasm, committed the clean baseline on a feature branch.
- **2026-06-18 (b)** — Verified fixes complete (see STATUS table): init hang, init OOB
  (wasm-opt off), memory layout (8 MB stack / 4 GB max), LoRA confirmed, harness + shim +
  diagnosis docs. Inference still traps (`memory access out of bounds` ~10 ms in).
- **2026-06-18 (a)** — Root-caused trap is **not** stack/quant/config; attention has no CPU
  fallback on wasm (F1); raw trap likely a bytemuck/wgpu/dimension issue in
  `dispatch_attention_pass`/`dispatch_prefill_layer_batch` (F2).

---

## 🧭 RECONCILIATION WITH BROADER PLAN

Snapshot for aligning this doc with workspace-level roadmaps (`0.0.17`/`0.0.18`, Compute
Universe / Track B3, `WASM_LLM_INFERENCE_DIAGNOSIS.md`):

| Track | Item | This plan | State |
|-------|------|-----------|-------|
| **Stability** | OOB trap | Phase 1 | ✅ Done — guards permanent |
| **Stability** | Init hang / init OOB / wasm-opt / stack | STATUS table | ✅ Done — do not redo |
| **Correctness** | CPU attention stent (Option A) | Phase 2A MC1–MC2 | 🟡 MC1 ✅, MC2 code ✅, **validation blocked on prefill** |
| **Correctness** | WASM init → KV + GEMM arenas | F5/F6 | ✅ GPU prefill OK |
| **Correctness** | Coherent browser tokens | Phase 2A success gate | ✅ CPU `inferWasmStreaming` naked `Paris.` |
| **Correctness** | Coherent GPU async (`inferWasmAsync`) | Phase 2B MC8 Part 3n | ✅ `Paris.` (Part 3l–3n); **3o regression — bisect** |
| **Performance** | Async GPU decode (Option B) | Phase 2B MC8 Part 3o | 🟡 TTFT **~6s** (↓ from ~7.9s); target ~4s |
| **Correctness** | WGSL RoPE ↔ CPU NEOX | MC8 pt2 | ✅ Closed |
| **Correctness** | GPU prefill manifold unified | MC8 pt3 | ✅ CPU fallback blocked |
| **Correctness** | K/V `gemm_weight_buf` flush | MC8 pt3c | ✅ L31 272→21 |
| **Correctness** | Batched prefill RoPE/RMSNorm | MC8 pt3d | ✅ per-row; not flat-space bug |
| **Correctness** | L0 `h[0]` @ step 1 (true GPU) | MC8 pt3d | ❌ **0.423** (pt3 `1.011` was artefact) |
| **Correctness** | L31 `h[0]` @ step 1 | MC8 pt3i | ✅ **0.841** (gate ~1.09) |
| **UX** | OPFS model cache | Phase 3 | ⬜ Parallel track, independent |
| **Architecture** | GGUF → `.q42` AOT | Phase 4 | ⬜ Horizon |

**Blockers:**

- **Resolved (pt3i):** Prefill `token_hidden`/`work_buf` alias + missing writeback flush.
- **Resolved (Part 3k):** Isolation proved decode manifold innocent; batched vocab GEMM was the regression source.
- **Resolved (Part 3l):** Per-chunk `dispatch_gemm_raw_into_async` + CPU argmax restores `Paris.` on `WASM_ASYNC=1`.
- **Resolved (Part 3n):** Fused prefill coherent — Q SDPA `attention_params` race fixed with per-token `mc8_flush`.
- **Resolved (Part 3o):** Per-Q flush eliminated — batched Q dispatch + batched masks; TTFT **~6s** (−17–27%).
- **Active (Part 3o):** Coherence regression — restore `Paris.` (bisect 3n baseline vs 3o); then batched GEMM $M&gt;1$ for ~4s TTFT.

**Branch:** `0.0.18` (ahead of origin).

**MC8 commits:** pt1 `60ba2451` · pt2 `4f506932` · pt3 `de499715` · pt3b `ba357389` · pt3c `64194425` · pt3d `3613b2e4`

---

## ▶️ 8. RESUME HERE (if context/tokens are lost — start at this section)

1. Read **§0** (directives), **§0b** (F1–F6), **§RECONCILIATION**, **MC8 Part 3–3l snapshot**.
2. **Current task = Phase 2B MC8 Part 3o (TTFT Collapse + coherence gate):**
   - **Done (pt3):** GPU prefill sole path; false L0 lock @ 1.011.
   - **Done (pt3b):** FFN residual audit; L1 jump bisect (1.01→-1.66).
   - **Done (pt3c):** KV indexing ✅; **K/V weight-buffer race** fixed; L31 **271→21**.
   - **Done (pt3d):** batched prefill RoPE/RMSNorm ✅; `attn_input` handoff; harness stable @ L31=20.87.
   - **Done (pt3e):** L0 Q/K/Attn_Out diff ✅ all match; KV `COPY_SRC` probe fix; causal mask ruled out.
   - **Done (pt3f):** o_proj + post-attn residual + ffn_norm ✅ all match; pristine residual trap ruled out.
   - **Done (pt3g):** FFN gate/up/SwiGLU/down/residual ✅ all bit-exact; L0=0.423 is correct unified exit.
   - **Done (pt3h):** embed+KV ✅; **L1_input_hidden ❌ first divergence** (batched prefill hidden handoff).
   - **Done (pt3i):** offset/flush audit ✅; `work_aliases_hidden` snapshot + writeback flush; **L1 bit-exact**, L31 **0.841**, coherent `Paris.` (harness `WASM_ASYNC=0` / `inferWasmStreaming` CPU stack).
   - **Endgame (2026-06-19):** see **Part 3j** — TTFT ~7.6s ✅; argmax fused wired; probes removed ✅; **`inferWasmAsync` incoherent** ❌ (fused argmax)
   - **Part 3k (2026-06-19):** CPU argmax gate → **`WASM_ASYNC=1` `Paris.`** ✅; GPU decode manifold cleared
   - **Part 3l (2026-06-19):** No WGSL argmax shader; batched vocab GEMM race fixed → per-chunk async GEMM + CPU argmax; **`Paris.`** ✅ @ ~7.5s TTFT
   - **Part 3m (2026-06-19):** `prefill_work_buf_A`/`_B` + ping-pong — validation ✅; numerics ❌
   - **Part 3n (2026-06-19):** Q SDPA params race fixed — **`Paris.`** ✅ on fused @ ~7.9s TTFT
   - **Part 3o (2026-06-19):** Zero-flush batched Q — TTFT **~6s** ✅; **`Paris.`** ❌ on current rebuild (bisect)
   - **Next:** coherence bisect → batched GEMM $M$ → TTFT ~4s → MC7 regression
3. **Build rule:** `scripts/package-qualia-wasm.ps1` or §BUILD `RUSTFLAGS` (8 MB stack) — mandatory.
4. **Harness:** `agent-tools/wasm-mc2-test.mjs` — `WASM_ASYNC=1`, `WASM_NAKED_PROMPT=1`,
   `WASM_MODEL=models/SmolLM2-360M-Instruct-Q4_K_M.gguf`.
5. **Pass criteria:** post-L31 `h[0]` within 5% of ~1.09; naked `Paris.` on `WASM_ASYNC=1`; TTFT → ~4s.
6. **Regression:** MC7 = `Paris is the capital of France.`, ~11s TTFT.
7. Phase 3 (OPFS) parallel.
8. Update **§7 Progress Log** after each micro-commit.

### Advisor feedback (MC8 pt2 → pt3) — **ANSWERED**

| # | Question | Architect decision | Part 3 outcome |
|---|----------|-------------------|----------------|
| A | CPU prefill + GPU decode split? | **Mandate full GPU prefill** | ✅ `dispatch_prefill_chunk_async_mc8_gpu` wired; CPU fallback blocked |
| B | L0 gate on decode step 1? | **Yes — step 1 only** | ⚠️ pt3 `1.011` was artefact; true L0=**0.423** post-pt3c |
| C | Elementwise vs prefill first? | **Prefill GPU K/V first** | ✅ done; elementwise audit deferred to pt3b (depth blow-up) |
| D | Argmax fusion? | **Gated** | ✅ sync argmax retained |

### Advisor feedback (MC8 pt3b) — **ANSWERED**

| # | Question | Architect decision | Part 3b outcome |
|---|----------|-------------------|-----------------|
| E | Bisect attention KV vs FFN first? | **FFN elementwise chain first** | FFN audit + flushes applied; L31 unchanged; **L1 jump** implicates cross-layer/KV next |

### Advisor feedback (MC8 pt3c) — **ANSWERED**

| # | Question | Architect decision | Part 3c outcome |
|---|----------|-------------------|-----------------|
| F | CPU/GPU diff vs KV indexing first? | **KV layout & layer indexing audit first** | Indexing ✅; found K/V weight-buffer race; flush fix landed |

### Advisor feedback (MC8 pt3d) — **ANSWERED**

| # | Question | Architect decision | Part 3d outcome |
|---|----------|-------------------|-----------------|
| G | Q/K diff vs prefill `token_idx` audit? | **Prefill batched-attn audit first** | K/V RoPE already per `wg_id`; RMSNorm per-row ✅; `attn_input` handoff + Q `abs_pos` patched |

### Advisor feedback (MC8 pt3e) — **ANSWERED**

| # | Question | Architect decision | Part 3e outcome |
|---|----------|-------------------|-----------------|
| H | Q/K diff @ L1 vs prefill causal audit? | **L0 Q/K/V/SDPA diff first** — divergence starts at L0 (0.423), not L1 | attn_rmsnorm/K/Attn_Out **bit-exact**; mask OK; gap is **post-SDPA** (o_proj/FFN) |

### Advisor feedback (MC8 pt3f) — **ANSWERED**

| # | Question | Architect decision | Part 3f outcome |
|---|----------|-------------------|-----------------|
| I | `o_proj`+residual vs FFN chain first? | **`o_proj` + attn-residual first** | o_proj, pristine residual, post-attn residual, ffn_norm **all bit-exact**; gap is **after ffn_norm** |

### Advisor feedback (MC8 pt3g) — **ANSWERED**

| # | Question | Architect decision | Part 3g outcome |
|---|----------|-------------------|-----------------|
| J | gate/up flush audit vs SiLU/down diff first? | **Audit flush/aliases first, then diff** | Audit ✅; gate/up/swiglu/down/ffn_residual **all bit-exact**; `first_divergence=none` |

### Advisor feedback (MC8 pt3h) — **ANSWERED**

| # | Question | Architect decision | Part 3h outcome |
|---|----------|-------------------|-----------------|
| H | Prefill/embedding vs L1–L31 bisect? | **Prefill/embedding reconciliation first** | embed+KV ✅; **L1_input_hidden ❌** @ batched prefill |

### Advisor feedback (MC8 pt3i) — **ANSWERED**

| # | Question | Architect decision | Part 3i outcome |
|---|----------|-------------------|-----------------|
| I | Per-token tail diff vs `batch_buf`/`token_buf` offset audit? | **Offset/flush audit first** | Offsets ✅; missing writeback flush + `token_hidden`/`work_buf` alias **fixed** |

**Harness verify (pt3i):** `L1_input_hidden` **bit-exact** (cpu/gpu `0.179094`, err `0.000017`); `first_divergence(0.01)=none`; L31 `h[0]=0.841` (was `20.87`); naked output **coherent** (`Paris. The capital of France…`); TTFT ~13.3s at pt3i session.

### Advisor feedback (MC8 Endgame) — **ANSWERED**

| # | Question | Architect decision | Endgame outcome |
|---|----------|-------------------|-----------------|
| K | Coherence achieved — TTFT or argmax first? | **TTFT first** | TTFT ~13.3s → **~7.6s**; coherence **lost** on current `inferWasmAsync` harness |
| L | Stride strategy (batched scratch, drop per-token flush)? | **Approved** | Infrastructure landed; fused path disabled pending disjoint-buffer audit |
| M | Argmax fusion after TTFT? | **Approved** | Wired; gated on coherence re-verify |
| N | Phase 2B Complete? | **Gated** | **NOT CLOSED** — `WASM_ASYNC=1` must return `Paris.` + TTFT &lt;4s |

### Advisor feedback (MC8 Part 3k) — **ANSWERED**

| # | Question | Architect decision | Part 3k outcome |
|---|----------|-------------------|-----------------|
| O | Argmax isolation first? | **Gate fused argmax → CPU fallback** | ✅ `Paris.` restored on `WASM_ASYNC=1`; decode manifold innocent |
| P | If still garbled — `ffn_buf` / flush audit? | **Audit decode path** | ⏭️ skipped — CPU argmax fixed output; `ffn_buf` not implicated |
| Q | TTFT ping-pong now? | **Blocked until `Paris.` with CPU argmax** | ✅ gate met; ping-pong is **next** |

### Advisor feedback (MC8 Part 3l) — **ANSWERED**

| # | Question | Architect decision | Part 3l outcome |
|---|----------|-------------------|-----------------|
| R | WGSL argmax OOB / barrier failure? | **Audit shader + readback** | **No WGSL argmax shader exists** — regression was batched vocab GEMM `gemm_weight_buf` race |
| S | 256-byte readback / chunk offset? | **Audit Rust readback** | Linear buffer map OK; chunk offset math correct |
| T | Re-enable `_mc8_fused`? | **Yes after fix** | ✅ per-chunk `dispatch_gemm_raw_into_async`; `Paris.` @ ~7.5s TTFT |

### Advisor feedback (MC8 Part 3o) — **ANSWERED**

| # | Question | Architect decision | Part 3o outcome |
|---|----------|-------------------|-----------------|
| U | Eliminate per-Q `mc8_flush` without uniform race? | **Parameter array OR batched uniform + mask slab** | ✅ batched uniform + `n_tokens` mask upload; per-Q flush removed |
| V | Expand `PREFILL_CHUNK_SIZE` to 32/64? | **Approved** | Already **64**; harness still `n_tokens=4` (short naked prompt, not cap-limited) |
| W | Batched GEMM $M>1$ now? | **Audit first, implement if shader supports** | Deferred — WGSL GEMM is $M=1$; per-token loops retained |
| X | Phase 2B complete after TTFT drop? | **Gated on coherence** | **NOT CLOSED** — TTFT ~6s ✅; `Paris.` ❌ pending bisect |
