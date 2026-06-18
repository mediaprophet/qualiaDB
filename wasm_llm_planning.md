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

**F1 — Attention has NO CPU fallback on wasm; only GEMM does.**
`dispatch_gemm_raw_into` (gguf_bridge.rs ~1230) falls through to `stack_gemm_quant`
(gguf_bridge.rs:294) on wasm — a real CPU GEMM. But `dispatch_attention_pass`
(gguf_bridge.rs:1460), which does RoPE + KV-cache write + scaled-dot-product attention,
uses the same `map_async`/`poll(Wait)`/`#[cfg(not(wasm32))]` readback pattern **with no
CPU equivalent**. On wasm it issues GPU work whose result is unreadable (WebGPU map is
async-only) and returns without computing real attention.
➡️ **Consequence:** "route the compute into `stack_gemm_quant`" (spec Phase 2) is
insufficient — `stack_gemm_quant` is matmul only. Correct wasm inference needs **either**
(A) a new CPU attention kernel for wasm, **or** (B) a true async-WebGPU decode path. See §6 Q1.

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

**F4 — Async dispatch variants are partial.** `dispatch_prefill_chunk_async`
(gguf_bridge.rs:2672) and `dispatch_output_argmax_chunked_async` (:2881) exist;
`dispatch_gemm_into_async` (:2371) exists; **there is no `dispatch_transformer_forward_async`
and no async attention.** A true async-GPU decode (option B) requires writing these.

---

## ✅ STATUS: Already fixed & verified (do not redo)

| Fix | Detail |
|-----|--------|
| Init hang | `docs/js/webgpu-limits-shim.js` strips removed WebGPU limits; `gguf_bridge.rs::initialize_webgpu_engine` (wasm) uses `try_new().await?` not `new_async()`. Engine init verified (~440 ms / 258 MB). |
| Init OOB | `Cargo.toml` `wasm-opt = false` (was `-Oz --enable-bulk-memory`, miscompiled the model `to_vec` copy). |
| Memory layout | `scripts/package-qualia-wasm.ps1` RUSTFLAGS: `-zstack-size=8388608` + `--max-memory=4294967296` → binary `min=131p(8MB)`, `max=65536p(4GB)`. (Did not fix the inference trap → not a stack overflow.) |
| LoRA | `context_detector` + `adapter_manager` compile for wasm; `webgpu_lora` is native-only; CPU apply wired at `llm_agent.rs:1379`. |
| Harness | `docs/wasm-llm-test.html` (model dropdown, cache toggle, panic hook, on-page log). |

---

## 🐞 PHASE 1 — Pinpoint & squash the OOB trap

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

1. **Short-circuit GEMM to CPU on wasm:** in `dispatch_gemm_raw_into` /`dispatch_gemm_into`,
   gate the entire GPU encode+map block under `#[cfg(not(target_arch="wasm32"))]` and on
   wasm call `stack_gemm_quant` directly — **do not create/write GPU buffers or `map_async`
   on wasm at all** (prevents wgpu state churn and the wasted-work OOB surface, F3).
2. **Attention (the hard part, F1):** decide per §6 Q1:
   - **Option A (CPU attention, simpler):** implement `#[cfg(target_arch="wasm32")]` CPU
     attention in/around `dispatch_attention_pass`: RoPE on Q/K, write K/V into the resident
     `kv_cache_cpu: Box<[f32]>`, scaled-dot-product over cached positions, softmax, weighted
     sum into `scratch_b`. Reuse `stack_gemm_quant` for the QKV and output projections. Use
     fixed-capacity buffers (no per-call heap).
   - **Option B (async WebGPU, faster, bigger):** make the wasm decode path async — add
     `dispatch_transformer_forward_async` + async attention using `JsFuture` on
     `map_async`, and make `infer_wasm_streaming` await per-step. Keeps GPU compute.
3. Verify `inferWasmStreaming` returns coherent tokens for a known prompt (capital-of-France
   test) on `SmolLM2-360M` before moving on.

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
| wasm init | `gguf_bridge.rs:2961` (`initialize_webgpu_engine`), `:401` (`try_new`) |
| decode (wasm branch) | `llm_agent.rs:654` / wasm region from ~`:1126` |
| prefill | `dispatch_prefill_chunk` `gguf_bridge.rs:1965` → `dispatch_prefill_layer_batch` |
| transformer layer | `dispatch_transformer_layer` `gguf_bridge.rs:2003`; forward `:2095` |
| attention | `dispatch_attention_layer` `:1617` → `dispatch_attention_pass` `:1460` (**no CPU fallback**) |
| GEMM + CPU fallback | `dispatch_gemm_into` `:1308`, readback `~:1284`, `stack_gemm_quant` `:294` |
| argmax | `dispatch_output_argmax_chunked` `:1331` |
| async variants | `dispatch_gemm_into_async` `:2371`, `dispatch_prefill_chunk_async` `:2672`, `dispatch_output_argmax_chunked_async` `:2881` |
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

- **2026-06-18 (c)** — Decisions recorded (above). Bumping `qualia-core-db` → **0.0.18**,
  rebuilding/redeploying wasm, and committing the clean baseline on a feature branch.
  *Next:* Phase 1 instrumentation.
- **2026-06-18 (b)** — Verified fixes complete (see STATUS table): init hang, init OOB
  (wasm-opt off), memory layout (8 MB stack / 4 GB max), LoRA confirmed, harness + shim +
  diagnosis docs. Inference still traps (`memory access out of bounds` ~10 ms in).
- **2026-06-18 (a)** — Root-caused trap is **not** stack/quant/config; attention has no CPU
  fallback on wasm (F1); raw trap likely a bytemuck/wgpu/dimension issue in
  `dispatch_attention_pass`/`dispatch_prefill_layer_batch` (F2).

---

## ▶️ 8. RESUME HERE (if context/tokens are lost — start at this section)

1. Read **§0** (directives), **§0b** (critical findings F1–F4), **§6** (decisions).
2. **Current task = Phase 1:** add `#[cfg(target_arch="wasm32")]` `web_sys::console::log_1`
   + bounds guards in `dispatch_attention_pass` (`gguf_bridge.rs:1460`) and
   `dispatch_prefill_layer_batch`, printing every slice/`bytemuck::cast_slice` byte-len vs
   buffer capacity and parsed dims. Rebuild (wasm-opt **off**), run
   `docs/wasm-llm-test.html` (served :8788) with `SmolLM2-360M-Instruct-Q4_K_M.gguf`, and
   read the **last log line before the trap** → that's the faulting access. Fix the
   dimension/index math.
3. Then **Phase 2 Option A** (CPU attention: RoPE + KV write into `kv_cache_cpu` + SDPA +
   softmax, fixed-capacity buffers, reuse `stack_gemm_quant` for projections) → verify
   coherent tokens → then **Option B** (async WebGPU). Run **Phase 3 (OPFS)** in parallel.
4. Build/deploy/test commands: **§BUILD/DEPLOY/TEST**. Code map: **§KEY CODE REFERENCES**.
5. Keep **§7 Progress Log** updated as you go.
