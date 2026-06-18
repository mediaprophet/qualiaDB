# WASM LLM Inference — Diagnosis & Status

**Date:** 2026-06-18
**Scope:** Make `docs/online-llm-demo.html` run real LLM inference in the browser through
Qualia's **own native WASM GGUF + WebGPU pipeline** (not wllama). wllama was dropped in
commit `3e6ca33b`.

**One-line status:** Model **download + cache + engine init now work**; **token generation
still traps** with `RuntimeError: memory access out of bounds` ~10 ms into
`inferWasmStreaming`. The trap is a real bug in the wasm compute kernels (CPU-fallback
GEMM / attention path), not a build-config or stack-size issue.

---

## 1. Architecture constraints (must not be violated)

- **Zero-heap is a hard invariant** elsewhere: 48-byte `NQuin`, 42 MB `SlgArena`, no
  `Vec`/`String`/`Box` in hot paths (CLAUDE.md §6).
- **LLM loading is the ONLY permitted heap exception**, and any memory solution for it
  **must stay quarantined to the wasm LLM path** (`#[cfg(target_arch = "wasm32")]`) and
  must not alter native or shared code. The native cooperative GPU-memory framework
  (`tensor/volume_gpu.rs` `TensorVolumeGpu`, engine `gemm_*_staging` buffers) is
  `#[cfg(not(target_arch = "wasm32"))]` — native-only by design.
- Practical consequence: fixes must be cfg-gated to wasm; do **not** heap-convert the
  shared stack-buffer GEMM design. Prefer raising the wasm stack or resident/fixed-capacity
  buffers over per-call heap churn.

---

## 2. Inference call path (browser)

```
JS: initialize_webgpu_engine(Uint8Array)   docs/pkg/qualia/qualia.js  → wasm
      └─ wasm_llm.rs::initialize_webgpu_engine            (crates/qualia-core-db/src/wasm_llm.rs:63)
           └─ gguf_bridge::initialize_webgpu_engine        (gguf_bridge.rs:2961)  [WASM]
                └─ QTensorEngine::try_new().await           (gguf_bridge.rs:401)   ← WebGPU device/pipelines
                └─ engine.gguf_mmap = Some(Arc<[u8]>)       (stores model bytes; NO parse here)

JS: inferWasmStreaming(prompt, on_token)
      └─ wasm_llm.rs::infer_wasm_streaming                 (wasm_llm.rs:96)
           └─ run_inference_streaming                       (wasm_llm.rs:36)
                └─ LocalLlmAgent::infer_local_model_streaming (llm_agent.rs:654; WASM branch starts ~1126)
                     ├─ GgufTokenizer::from_gguf / encode
                     ├─ GgufTensorIndex::from_gguf
                     ├─ PREFILL:  engine.dispatch_prefill_chunk        (gguf_bridge.rs:1965)
                     │              └─ dispatch_prefill_layer_batch  (per layer)
                     └─ DECODE loop (≤2048 tok):
                            dispatch_transformer_forward             (gguf_bridge.rs:2095)
                              └─ dispatch_transformer_layer          (gguf_bridge.rs:2003)
                                   ├─ dispatch_attention_layer       (KV cache; kv_cache_cpu: Box<[f32]>)
                                   └─ dispatch_gemm_into → dispatch_gemm_raw_into (gguf_bridge.rs:~1230)
                            dispatch_output_argmax_chunked            (gguf_bridge.rs:1331)
```

### Critical wasm vs native difference — GPU readback
`dispatch_gemm_raw_into` (gguf_bridge.rs ~1284–1304):
```rust
let slice = staging.slice(..out_bytes);
slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
self.gpu_device().poll(wgpu::Maintain::Wait);       // no-op on wasm
#[cfg(not(target_arch = "wasm32"))]                  // ← GPU readback NATIVE-ONLY
if let Ok(handle) = tokio::runtime::Handle::try_current() {
    if handle.block_on(rx) ... { out[..n_out].copy_from_slice(&floats[..n_out]); return true; }
}
let _ = staging.unmap();                             // wasm: unmap a never-awaited map
}
stack_gemm_quant(raw, info, input, out, n_in, n_out) // ← wasm path: CPU GEMM fallback (gguf_bridge.rs:294)
```
**On wasm, every GEMM result is computed by the CPU fallback `stack_gemm_quant`** (the GPU
result is unreadable synchronously in a browser — WebGPU map is async-only). So the wasm
inference is effectively a **CPU compute path** that still *encodes* GPU work (buffer
creation / `write_buffer` / dispatch) before falling back.

---

## 3. What is FIXED (verified in the harness)

| # | Symptom | Root cause | Fix | Evidence |
|---|---------|-----------|-----|----------|
| 1 | Page stuck on "Initialising Qualia WebGPU engine…" (hang, never resolves) | wgpu **0.19.4** serializes the spec-removed `maxInterStageShaderComponents` limit into `GPUAdapter.requestDevice`; current Chrome rejects it → `QTensorEngine::new_async().expect()` **panics inside the async init future** → wasm aborts, promise pends forever | (a) `docs/js/webgpu-limits-shim.js` strips limits not in `adapter.limits` before `requestDevice` (works against the committed binary); (b) `gguf_bridge.rs:2961` `initialize_webgpu_engine` now uses `QTensorEngine::try_new().await?` instead of `new_async()` so a bad adapter rejects cleanly | init resolves; `isWebgpuEngineReady()===true` |
| 2 | `initialize_webgpu_engine` traps `memory access out of bounds` for any real model (≥~50 MB), but works for a 64-byte buffer | **wasm-opt `-Oz --enable-bulk-memory`** miscompiled the large `vec![0; n]` / `memory.fill` for the model `to_vec()` copy | `crates/qualia-core-db/Cargo.toml:228` `[package.metadata.wasm-pack.profile.release] wasm-opt = false` | 258 MB model loads; **engine ready in ~440 ms** |
| 3 | (precaution) wasm stack vs native | native call tree assumes 8 MB stack | `scripts/package-qualia-wasm.ps1` RUSTFLAGS `-C link-arg=-zstack-size=8388608 -C link-arg=--max-memory=4294967296` | binary memory section: `min=131p (8 MB)`, `max=65536p (4 GB)`. **Did NOT fix the inference trap** → trap is not a stack overflow |
| 4 | LoRA | — | confirmed intact for wasm | `lora/mod.rs`: `adapter_manager` + `context_detector` compiled for all targets; `webgpu_lora` is `#[cfg(not(wasm32))]`; CPU apply wired at `llm_agent.rs:1379` `adapter.apply_cpu(...)` |

### Harness evidence (latest build: wasm-opt off, 8 MB stack, 4 GB max)
```
[..] WASM module initialised
[..] cache MISS models/SmolLM2-360M-Instruct-Q4_K_M.gguf — fetching…
[..] fetched (cache.put failed: ...Unexpected internal error.) 258.1 MB, 2889 ms
[..] initialize_webgpu_engine…
[..] engine ready=true (init 439 ms)          ← INIT OK
[..] inferWasmStreaming…
[..] WINDOW ERROR: Uncaught RuntimeError: memory access out of bounds   ← TRAP ~10 ms in
```

---

## 4. The REMAINING bug (the blocker)

**Symptom:** `inferWasmStreaming` throws `Uncaught RuntimeError: memory access out of
bounds` ~10 ms after start. The streaming Promise never resolves (a wasm trap does not
reject the Promise), so the UI sits on "Generating…".

**Established facts:**
- **Not** the init path (init succeeds, engine ready).
- **Not** quant-specific: traps with Q4_K_M (258 MB) and Q8_0 (386 MB); gemma-3-1b q4_0
  (569 MB) reached the same trap.
- **Not** a stack overflow: an **8 MB** stack (verified in the binary) did not change it.
- **Not a Rust panic** (no panic-hook message, even with `init_panic_hook()` installed) →
  it is a **raw wasm trap**: an unchecked/raw-pointer/SIMD (`+simd128`) memory access, a
  `bytemuck` cast over a too-short slice, or a wgpu-wasm buffer op with a bad size.
- Occurs in **prefill or the first decode step** (≈10 ms), i.e. inside
  `dispatch_prefill_chunk` → `dispatch_prefill_layer_batch` → (`dispatch_attention_layer` /
  `dispatch_gemm_into` / `stack_gemm_quant`).

**Ranked hypotheses to investigate:**
1. **`dispatch_attention_layer` + KV cache** (`kv_cache_cpu: Box<[f32]>`, `gguf_bridge.rs`
   struct field :358). Position/token-index math (`token_idx`, `batch_start_token_idx`)
   indexing the CPU KV mirror out of range, or a `bytemuck::cast_slice` over a buffer sized
   from wrong dims. This runs first in prefill and is the most index-heavy. **Primary suspect.**
2. **`matmul_dims()` mis-parsing GGUF tensor shapes** → `n_in`/`n_out` wrong → a
   `write_buffer`/`copy_buffer_to_buffer`/`get_mapped_range` with a size exceeding the
   reused `gemm_*` buffer capacity (sized by `ensure_gemm_buffers` for `MAX_STACK_GEMM_DIM
   = 10240`). SmolLM2-360M dims (hidden 960, FFN 2560) are within caps, so a *wrong parse*
   is the risk, not the true dims.
3. **`stack_gemm_quant`** (gguf_bridge.rs:294): `row = [0f32; MAX_STACK_GEMM_IN=10240]`,
   `row[..n_in]`. If `n_in > 10240` this is a Rust panic (would show via hook) — so only a
   suspect if a SIMD/unchecked variant is used. Bounds guard at the top looks correct.
4. **wgpu-wasm `map_async` + immediate `unmap` without await** (lines ~1286–1301): on wasm
   the read map is started, `poll(Wait)` is a no-op, then `unmap()` is called before the
   map resolves. May corrupt wgpu state / trap on a subsequent op. Lower probability but
   wasm-specific.
5. The native sync-readback architecture being fundamentally wrong for the browser: the
   **async** dispatch variants exist only partially — `dispatch_prefill_chunk_async`
   (gguf_bridge.rs:2672) and `dispatch_output_argmax_chunked_async` (:2881) exist, but
   there is **no `dispatch_transformer_forward_async`**. The streaming decode calls the
   **sync** variants. Even after the OOB is fixed, output correctness on wasm depends
   entirely on the CPU fallback being complete for *attention* (verify `dispatch_attention_layer`
   has a real wasm CPU path, not just GEMM).

**Recommended next diagnostic step:** add explicit `if idx >= buf.len() { return false; }`
guards (and `web_sys::console::log_1`) at each slice/cast/index site in
`dispatch_attention_layer`, `dispatch_gemm_into`, `dispatch_gemm_raw_into`, and
`dispatch_prefill_layer_batch`, rebuild, and run the harness — the first guard that fires
(or the last log before the trap) pinpoints the faulting access. Then fix the dimension /
index math. Keep all changes cfg-aware so native is untouched.

---

## 5. Build, deploy, and test setup (reproduction)

### Build (wasm)
```bash
cd crates/qualia-core-db
RUSTFLAGS="-C target-feature=+simd128 -C link-arg=-zstack-size=8388608 -C link-arg=--max-memory=4294967296" \
  wasm-pack build --target web --out-dir pkg-qualia --release -- \
  --no-default-features --features portal,wasm-llm,wasm-logic,wasm-scientific
```
- `wasm-opt` is **disabled** via `Cargo.toml` metadata (do not re-enable `-Oz` without
  re-testing the init copy — it miscompiles bulk-memory).
- The PowerShell wrapper `scripts/package-qualia-wasm.ps1` does the same but **aborts on
  wasm-pack's stderr `[INFO]` lines** under PowerShell (`$ErrorActionPreference=Stop`); run
  the `wasm-pack` command from Git Bash instead, or fix the script to not treat stderr as
  fatal.

### Deploy to the demo's package dir
```bash
SRC=crates/qualia-core-db/pkg-qualia DOCS=docs/pkg/qualia
cp -f $SRC/qualia_core_db.js          $DOCS/qualia.js
cp -f $SRC/qualia_core_db_bg.wasm     $DOCS/qualia_bg.wasm
cp -f $SRC/qualia_core_db.d.ts        $DOCS/qualia.d.ts
cp -f $SRC/qualia_core_db_bg.wasm.d.ts $DOCS/qualia_bg.wasm.d.ts
sed -i 's/qualia_core_db_bg\.wasm/qualia_bg.wasm/g' $DOCS/qualia.js
```

### Test harness
- **`docs/wasm-llm-test.html`** — controllable page: model dropdown (local `/models/*.gguf`
  with chat template), Cache toggle, Load + Run buttons, on-page log capturing panics /
  window errors / timings / tokens. Installs the limits shim + `init_panic_hook`.
- **Models** (gitignored, in `docs/models/`): `SmolLM2-360M-Instruct-Q4_K_M.gguf` (chatml,
  ~258 MB), `smollm2-360m-instruct-q8_0.gguf` (~386 MB), `gemma-3-1b-it-q4_0.gguf` (gemma3,
  ~569 MB). `.gitignore` has `docs/models/` and `models/`.
- **Servers:** docs served by `.claude/serve_docs.py` on **8788**; an optional CORS+Range
  model server `.claude/serve_models.py` on **8799** (for serving models from
  `C:/LLM_Models/GGUF` cross-origin). The harness fetches models **same-origin** from
  `/models/`, so 8788 alone is enough.

### Environment notes
- Chrome-based; WebGPU adapter present; **`crossOriginIsolated === false`** in the preview
  (COI service worker registered but isolation not applied) — multi-thread/SharedArrayBuffer
  features unavailable; single-thread wasm only.
- Quant support confirmed real: `ggml_quants.rs` dequantizes F32/F16/Q4_0/Q8_0/Q4_K/Q6_K.

---

## 6. Secondary issue — browser model caching (user requirement)

Goal: cache downloaded GGUF in browser storage so models aren't re-downloaded.
- Current harness uses **Cache Storage** best-effort; `cache.put` **fails on the 258 MB
  entry**: `Failed to execute 'put' on 'Cache': Unexpected internal error.` (large-entry
  limit and/or COI service-worker interference). It's wrapped in try/catch so it does not
  block loading.
- **Recommended fix:** use **OPFS** (Origin Private File System) for large GGUF caching —
  robust for hundreds of MB; matches the project's existing OPFS auto-cache notes
  (`wasm_storage.rs`, `.q42.bidx` demand-paging). Add a "cache in browser" toggle / prompt
  in the production demo.

---

## 7. Files changed this session

| File | Change | Status |
|------|--------|--------|
| `docs/js/webgpu-limits-shim.js` | NEW — strips removed WebGPU device limits | keep |
| `docs/online-llm-demo.html` | include shim `<script>` | keep |
| `docs/llmdemo/index.html` | include shim `<script>` (`../js/...`) | keep |
| `crates/qualia-core-db/src/gguf_bridge.rs` | wasm `initialize_webgpu_engine` uses `try_new().await?` (no panic-hang) | keep |
| `crates/qualia-core-db/Cargo.toml` | `wasm-opt = false` | keep (until a safe opt set is found) |
| `scripts/package-qualia-wasm.ps1` | RUSTFLAGS: 8 MB stack + 4 GB max-memory | keep |
| `crates/qualia-core-db/src/llm_agent.rs` | reverted (prefill_chunk back to stack array — heap hack removed) | clean |
| `docs/wasm-llm-test.html` | NEW — test harness + caching | keep (dev) |
| `.claude/serve_models.py` | NEW — CORS+Range model server (dev) | keep (dev) |
| `.gitignore` | add `docs/models/`, `models/` | keep |

`docs/pkg/qualia/qualia.js` + `qualia_bg.wasm` are the deployed rebuilt artifacts (wasm-opt
off, 8 MB stack). `scripts/prepare_schemaorg_benchmark.ps1` in `git diff` is pre-existing,
not from this work.
