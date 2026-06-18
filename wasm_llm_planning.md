# Qualia WASM LLM Inference — Planning & Agent Task Specification

**Date:** 2026-06-18 · **Owner:** Qualia · **Companion doc:** [`WASM_LLM_INFERENCE_DIAGNOSIS.md`](WASM_LLM_INFERENCE_DIAGNOSIS.md)

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
(`wasm_llm.rs`). **Remaining gap:** prefill K/V still CPU-only; fused GPU prefill
(`dispatch_prefill_chunk_async_mc8_gpu`) is dead_code until numerically validated.

**F5 — WASM init skipped KV/GEMM arena setup.** *(Discovered MC2 session; fix landed,
validation pending.)* `initialize_webgpu_engine` (wasm, `gguf_bridge.rs:3242`) previously
set only `engine.gguf_mmap = Some(data)` and **did not** call `adopt_resident_mmap`, which
is the sole path that invokes `ensure_kv_cache()` + `ensure_gemm_buffers()`. Without that,
`dispatch_attention_pass` guard-trips on `kv_cache_gpu.is_none()` / missing GEMM buffers →
prefill K/V never run → decode proceeds with an empty KV mirror → F1 garbage output even
after SDPA lands. **Fix:** init now calls `engine.adopt_resident_mmap(gguf_data)?`.

**F6 — Prefill failure was silently ignored.** `llm_agent.rs` used `let _ =
dispatch_prefill_chunk(...)` (return value discarded). A failed prefill looked like
"inference works" (2048 tok, ~8 tok/s) but TTFT stayed **<600 ms** — impossibly fast for
real CPU attention across 32 layers × 37 prompt tokens. **Fix landed:** log
`[llm] PREFILL chunk FAILED pos=… n=…` on failure. **Still open:** layer-0 prefill fails
even after F5 fix (`[llm] PREFILL chunk FAILED pos=0 n=37` in harness console); root cause
is the next debug target (`[attn_pass] GUARD …` / `[prefill_layer] K|V pass FAILED` logs
added in `dispatch_attention_pass` / `dispatch_prefill_layer_batch`).

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
| WASM init arenas | `initialize_webgpu_engine` → `adopt_resident_mmap` (F5 fix); prefill still fails (F6). |

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

### Phase 2B — Async WebGPU compute (Option B) — 🟡 IN PROGRESS

| Micro-commit | Scope | Status |
|--------------|-------|--------|
| **MC5** | `dispatch_attention_pass_async` + `dispatch_transformer_forward_async` plumbing | ✅ Closed — wired via `inferWasmAsync` (`wasm_llm.rs`) |
| **MC6** | `inferWasmAsync` + JS `Promise` bridge; `_async` dispatch loop | ✅ Closed — naked ` Paris.` TTFT ~9s (vs ~47s CPU) |
| **MC7** | WGSL `Q5_0`/`Q8_0` dequant; gate removal; full GPU offload | ✅ Closed — `Paris is the capital of France`; TTFT ~11s |
| **MC8** | Pipeline fusing — single `CommandEncoder`, one `map_async` per forward/argmax | 🟡 Part 3 in flight — L0@L0 locked; L31 blow-up open |

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
| `dispatch_prefill_chunk_async_mc8_gpu` | GPU batched prefill (dead_code — delegates to CPU) |
| `dispatch_output_argmax_chunked_async_mc8_fused` | Fused multi-chunk argmax (dead_code — delegates to sync) |

**Current hot-path routing (post Part 3 — GPU manifold unified):**

- Prefill → `dispatch_prefill_chunk_async_mc8_gpu` (GPU batched K/V + per-token Q/FFN; **no CPU fallback**)
- Decode forward → `dispatch_transformer_forward_async` → `encode_transformer_layer_gpu` (fused GPU: attn + `wasm_elementwise` RMSNorm/SiLU/residual)
- Argmax → `dispatch_output_argmax_chunked` (sync — argmax fusion **gated** per architect)

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

**Part 3 (🟡 active — manifold unification + depth blow-up)**

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
| L0 @ layer 0 (step 1) | **1.011** | ✅ (~7% from ~1.09 target) |
| L0 @ layer 31 (step 1, post-full forward) | **271.7** | ❌ **depth blow-up** |
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

**Next (Part 3c):**
1. **CPU vs GPU per-layer diff** on decode step 1 @ L0/L1 (same embedding) — localize first divergent op.
2. **Attention KV read path** — `encode_transformer_layer_gpu` writes K/V then `encode_attn_ffn_tail_gpu` runs Q+attn; audit KV layout indexing for `layer > 0` and prefill→decode handoff.
3. **Argmax:** remain sync until post-L31 `h[0]` stable.

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
| MC8 RoPE (WGSL) | `fused_attention.wgsl::apply_rope_neox`; `AttentionGpuParams.rope_theta_base` + `rope_scale` |
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
| **Correctness** | WASM init → KV + GEMM arenas | F5 | 🟡 Fix landed; prefill still fails (F6) |
| **Correctness** | Coherent browser tokens | Phase 2A success gate | 🟡 MC7 coherent on CPU elem; MC8 fused garbled |
| **Performance** | Async GPU decode (Option B) | Phase 2B MC5–MC8 | 🟡 Fused decode ~9s TTFT; target ~4s |
| **Correctness** | WGSL RoPE ↔ CPU NEOX | MC8 pt2 | ✅ Closed |
| **Correctness** | GPU prefill manifold unified | MC8 pt3 | ✅ CPU fallback blocked |
| **Correctness** | L0 post-FFN GPU @ L0 (step 1) | MC8 pt3 | ✅ `h[0]=1.011` |
| **Correctness** | L0 post-FFN GPU @ L31 (step 1) | MC8 pt3b | ❌ `h[0]=271.7` blow-up (FFN audit unchanged) |
| **Correctness** | Depth bisect L0–L3 (step 1) | MC8 pt3b | L0=1.01 ✅; L1=-1.66 ❌ jump at L1 |
| **UX** | OPFS model cache | Phase 3 | ⬜ Parallel track, independent |
| **Architecture** | GGUF → `.q42` AOT | Phase 4 | ⬜ Horizon |

**Single blocker for broader "WASM LLM works" claim (updated MC8 pt2):** fused GPU decode
runs without WebGPU errors and beats MC7 TTFT, but **output coherence regressed** vs MC7
(`Paris is the capital of France` → garbled repetition). Advisors should prioritize:
**(1)** CPU/GPU L0 parity on decode step 1, **(2)** CPU-prefill vs GPU-decode KV/RoPE split,
**(3)** elementwise residual routing before argmax fusion.

**Branch:** `0.0.18` (ahead of origin). MC8 pt1 `60ba2451`, MC8 pt2 `4f506932`.

---

## ▶️ 8. RESUME HERE (if context/tokens are lost — start at this section)

1. Read **§0** (directives), **§0b** (F1–F6), **§6** (decisions), **§RECONCILIATION**, **MC8 §Part 2/3**.
2. **Current task = Phase 2B MC8 Part 3c (L1 jump — not FFN residual):**
   - **Done (pt3):** GPU prefill manifold unified; L0@L0 step1 = **1.011** (gate met).
   - **Done (pt3b):** FFN/residual routing audit + depth bisect L0–L3; `add_residual_main` OK; extra flushes; scratch isolation — **post-L31 still 271.7**.
   - **Bisect:** L0=1.011 → L1=-1.662 → L2=-7.337 → L3=-0.595 → L31=271.7. **L1 jump** ⇒ cross-layer / KV / layer-block fault, not FFN creep.
   - **Next:** CPU vs GPU per-layer diff @ L0/L1; attention KV read path + prefill→decode handoff.
3. **Build rule:** must use 8 MB stack `RUSTFLAGS` (§BUILD/DEPLOY/TEST) — omitting → instant OOB trap.
4. **Harness:** `agent-tools/wasm-mc2-test.mjs` with `WASM_ASYNC=1`, `WASM_NAKED_PROMPT=1`,
   `WASM_MODEL=models/SmolLM2-360M-Instruct-Q4_K_M.gguf`.
5. **Pass criteria (Part 3b):** post-L31 `h[0]` within 5% of CPU (~1.09 band); then naked ` Paris.`; TTFT trending toward ~4s.
6. **Regression reference:** MC7 (`3d39868c`) = coherent output, ~11s TTFT, CPU elementwise + GPU attention.
7. Phase 3 (OPFS) can run in parallel.
8. Keep **§7 Progress Log** updated as you go.

### Advisor feedback (MC8 pt2 → pt3) — **ANSWERED**

| # | Question | Architect decision | Part 3 outcome |
|---|----------|-------------------|----------------|
| A | CPU prefill + GPU decode split? | **Mandate full GPU prefill** | ✅ `dispatch_prefill_chunk_async_mc8_gpu` wired; CPU fallback blocked |
| B | L0 gate on decode step 1? | **Yes — step 1 only** | ✅ `h[0]=1.011` @ L0; step 0 not probed |
| C | Elementwise vs prefill first? | **Prefill GPU K/V first** | ✅ done; elementwise audit deferred to pt3b (depth blow-up) |
| D | Argmax fusion? | **Gated** | ✅ sync argmax retained |

### Advisor feedback (MC8 pt3b) — **ANSWERED**

| # | Question | Architect decision | Part 3b outcome |
|---|----------|-------------------|-----------------|
| E | Bisect attention KV vs FFN first? | **FFN elementwise chain first** | FFN audit + flushes applied; L31 unchanged; **L1 jump** implicates cross-layer/KV next |

**New advisor question (pt3c):** L1 jumps 1.01 → -1.66 after FFN routing fix. Prioritize **CPU vs GPU per-layer diff** or **KV cache layout / layer indexing audit** first?
