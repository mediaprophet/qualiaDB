# Qualia WASM LLM Inference — The Endgame

**Date:** 2026-06-19 · **Owner:** Qualia
**Companion doc:** `WASM_LLM_INFERENCE_DIAGNOSIS.md` (Legacy: `wasm_llm_planning.md`)

This document supersedes `wasm_llm_planning.md` for the final push of Phase 2B and beyond. It tracks the terminal optimizations required to achieve real-time WebGPU inference and the immediate roadmap for deployment.

---

## ✅ 1. THE PHASE 2B FINAL GATE — **CLOSED (2026-06-19, Part 3y)**

**Closure Criteria:** TTFT reliably below 4.5 s while maintaining absolute coherence. **MET.**

**Result (3-run, naked SmolLM2-360M-Q4_K_M, `WASM_ASYNC=1`, headless Chrome / NVIDIA Ampere):**
* **Coherence:** Locked — `"Paris. The capital of France…"` (3/3 runs).
* **Validation:** Clean. No WebGPU guard trips or OOB traps.
* **TTFT:** **3905 / 3968 / 3997 ms (avg 3957 ms)** — **< 4500 ms** ✅.
* **Flush budget:** decode **2 submits/layer** (was 13).

**How it was won (the profiling-driven path, Parts 3w→3y):**
1. **Profile (3w):** attributed TTFT — decode forward = 416 submits (13/layer) but submit overhead was overlapped; readback negligible. Submits were *not* the bottleneck.
2. **Decode super-arena (3w):** decode → 2 submits/layer by reusing the prefill super-arena at `n_tokens=1`. Correct, but TTFT flat → not submit-bound.
3. **Resident weights (3x):** the real tax was re-uploading ~208 MB of weights **every forward**. Uploaded once into 7 per-role buffers (`stride×n_layer`), bound per-layer sub-ranges. Per-forward upload → 0.
4. **Eager upload (3y):** moved the one-time 219 MB resident upload from the lazy first-prefill (inside the TTFT window) to model-init (`adopt_resident_mmap`, before the clock). `prefill 2409 → 35 ms`; **TTFT 6118 → 3957 ms**. Gate closed.

**Remaining (post-gate, NOT gate blockers):**
* **Throughput:** ~0.6 tok/s (~1700 ms/token). The decode forward is now **GPU-execution-bound** (~400 small `M=1` dispatches/forward on real Ampere; invariant to submits/upload). Next lever: GPU **timestamp profiling** → dispatch **fusion** (Part 3y Phase 2/3, deferred — gate already met).
* **Instrumentation:** `ttft_profile` (wasm-only) still in tree — strip after milestone sign-off.
* **MC7 ChatML regression** (`WASM_NAKED_PROMPT=0`) — next milestone.
4. **Flush Purge:** Delete the `mc8_flush()` separating the K/V block from the tail.
5. **Target:** Exactly **1 submit per layer** (32 total submits for the prefill chunk).

---

## 🗺️ 3. THE REMAINING ROADMAP

### MC7: The ChatML "Regression" — ✅ CLOSED (2026-06-19): Expected Model Behavior, engine parity proven
* **Verdict:** **Not an engine bug.** The 22-token ChatML prompt yields a *fragmentary* greedy
  completion (`The capital of France<|im_end|>` — stops before "Paris"), but this is an alignment
  artifact of the heavily-quantized 360M instruct model under pure argmax, not a defect in Qualia's
  tokenizer, masking, or tensor manifold.
* **Internal proof of correctness (no external LLM reference used — Prime Directive #4):**
  1. **CPU path ≡ GPU path** — byte-identical fragment ⇒ the GPU manifold (decode super-arena /
     resident weights, Parts 3w–3y) is innocent.
  2. **Tokenizer HF-parity** — `smollm_tokenizer_audit_vs_hf_reference` asserts the exact 22 IDs;
     BOS handling correct (`<|im_start|>`=BOS=1, no double-BOS).
  3. **In-context knowledge intact** — priming the assistant turn with "…The capital of France is"
     deterministically yields **" Paris.<|im_end|>"**. The model knows the fact; greedy decode just
     emits the turn-end `<|im_end|>` one token early from a bare `assistant\n` turn.
* **Consequence / follow-up:** ChatML demos on small models need a proper **sampler
  (Temperature / Top-K / Top-P) in the agent layer — *outside* the core engine** (the deterministic
  tensor core must emit the highest-logit token, EOS included). The core engine ships greedy/argmax
  for reproducible CI gates.

### Phase 3: OPFS Robust Model Caching — ✅ DONE in harness (2026-06-19), verified
* **Goal:** Bypass browser heap limits and `Cache Storage` failures for >250MB files.
* **Delivered:** `docs/js/opfs-model-cache.js` — `loadGgufCached(url, name, expectedSize, onProgress)`.
  Streams the `fetch` response straight to OPFS via `FileSystemWritableFileStream`
  (`pipeThrough(progressCounter).pipeTo(writable)`) — no >250MB JS-heap blob. Atomic `.part` →
  `move()` promotion gated on streamed-bytes == Content-Length. Best-effort: any OPFS failure
  (quota/unsupported/interrupted) falls back to a plain buffered fetch (loading never blocks).
  Wired into `docs/wasm-llm-test.html` `getModelBytes()` with progress in `#loadstat`; "Clear cache"
  now also purges OPFS (`clearAllOpfsModels`). Engine contract unchanged (`Uint8Array`).
* **Verified (headless Chrome):** miss → streamed 258.1 MB → boot (1 `.gguf` request); reload →
  **OPFS HIT in 246 ms, 0 `.gguf` requests** → boot. No `cache.put` failure (old Cache Storage bug
  eliminated). JS-only — no wasm rebuild.
* **Pending:** port to `online-llm-demo.html` + `llmdemo/index.html` (architect gated until harness
  proven — now proven). Chunked OPFS→wasm mmap (avoid the one full `arrayBuffer()` read) is Phase 4.

### Phase 4: AOT `.q42` Compilation (Horizon) — DRAFT PLAN (2026-06-19)
* **Goal:** Compile GGUF → a Qualia-native `.q42` **weight container** ahead of time so the runtime
  skips GGUF parsing and binds page-aligned tensor slices directly (toward mmap/zero-copy).

* **Reconciliation (important):** the existing `.q42` (`tensor/q42_integration.rs`, `bake_pipeline.rs`,
  the schemaorg `.q42`) is the **semantic graph** format (`NQuin → Tensor10D` volumes). Phase 4 is a
  **new sibling section in the q42 family** — an LLM-weight container — not a change to the semantic
  format. Use a distinct section magic so they never collide.

* **Layout (per §6-Q2: contiguous strided blobs + 48-byte Quin manifest; weights are NOT Quins):**
  ```
  [ Q42 container header ]            existing v3 header + a pointer to the weight section
  [ Q42WeightHeader      ]            magic b"Q42W", version, page_log2 (12=4K|14=16K), n_tensors,
                                      n_layers, manifest_offset, blob_offset, arch scaffold NQuin
  [ manifest: Q42TensorEntry[] ]      one per tensor — role, layer, ggml_type, dim0/1, blob_offset,
                                      byte_len, + a 48-byte scaffold NQuin (ontology/lexical binding)
  [ pad → page boundary  ]
  [ tensor blob region   ]            quantized bytes, per-role grouped + per-layer strided, each
                                      tensor START page-aligned (4K/16K) for single-fetch mmap; the
                                      intra-role stride mirrors the 256-aligned bind ranges from 3x.
  ```
  Rust (`#[repr(C, align(16))]`, padding-free, little-endian on the wire) — **v2** (128B header):
  ```rust
  struct Q42WeightHeader { magic:[u8;4]=b"Q42W", version:u16=2, page_log2:u16, n_tensors:u32,
      n_layers:u32, n_embd:u32, n_head:u32, n_kv_head:u32, vocab_size:u32, rope_freq_base:f32,
      rope_scale:f32, manifest_offset:u64, blob_offset:u64, cold_offset:u64, cold_len:u64,
      header_crc:u32, format_flags:u32, arch_quin: NQuin /*48B*/ }      // 128 B
  struct Q42TensorEntry  { role:u16, layer:u16, ggml_type:u32, dim0:u32, dim1:u32,
      blob_offset:u64, byte_len:u64, scaffold_quin: NQuin /*48B*/ }     // 80 B
  ```
  **Integrity (in-band, zero-copy-safe):** `header_crc` = CRC-32C over header+manifest (boot check,
  µs); each entry's `NQuin.parity` = CRC-32C of its 32 functional bytes (offset/len corruption →
  caught before any GPU bind, avoiding OOB traps). `NQuin.metadata` bitfield reserved for
  sparsity/quant/deontic-taint hints. Blob bit-rot deferred (lazy/sampled). See `WASM_LLM_TTFT_QUESTIONS`
  follow-up + the q42 integrity discussion.

* **`compile_gguf_to_q42(input: &[u8]) -> Vec<u8>` (wasm export):**
  1. Parse GGUF via `GgufTensorIndex::from_gguf` (reuse).
  2. Reuse the per-role/per-layer page-aligned layout already computed in
     `mc8_upload_all_resident_weights` (3x) — it is the AOT layout.
  3. Emit header + manifest (entries + scaffold Quins) + page-aligned blob region; stream output to
     OPFS via the Phase 3 writer. Optionally fan out per-tensor packing to Web Workers (the architect's
     "compiler farm").
  4. Runtime: read header+manifest (small), bind each blob slice by pre-baked offset — zero GGUF parse;
     the resident-weight upload becomes a direct copy (or mmap view) from the `.q42`.

* **Decisions (architect, 2026-06-19):** (1) `page_log2` header field, **default 16K**; (2) raw NQuin
  hot manifest **+** optional cold CBOR-LD section (`cold_offset`/`cold_len`, reserved in v1);
  (3) include ALL tensors (`token_embd`/`output`/norms via `layer=0xFFFF` sentinel); (4) enforce
  **little-endian** emission now + `version` gate.

* **v2 COMPILER + READER + BOOT GATE DONE (2026-06-19):** `src/q42_weight.rs` —
  - `compile_gguf_to_q42(input, page_log2) -> Vec<u8>` (native, explicit LE); writes the v2 header
    (hyperparams + `header_crc` + per-entry `parity` CRC) + manifest + 16K-page blobs.
  - `Q42TensorIndex::from_q42(&[u8])` runtime reader — validates magic/version, **verifies header +
    every entry CRC** (rejects corruption before any bind), reconstructs `GgufHyperparams`,
    zero-copy `blob()` view.
  - wasm export `compileGgufToQ42(Uint8Array, page_log2)`.
  - **Dual-format boot gate** in `initialize_webgpu_engine` (`gguf_bridge.rs`): first 4 bytes →
    `b"GGUF"` (legacy `adopt_resident_mmap`) or `b"Q42W"` (`adopt_resident_q42`). The q42 path
    validates the container, reserves GEMM/KV arenas from the header hyperparams, and **maps the
    page-aligned GEMM-role blobs straight into `Mc8WeightArenaBufs`** (`mc8_upload_resident_from_q42`).
  - **Verified:** native test PASSES — `290 tensors, 32 layers, blob@32768`, reader round-trip +
    hyperparams + **CRC tamper-detection** (flipped manifest byte rejected). wasm build compiles clean.
* **Remaining Phase 4 (next):** (a) **inference-from-`.q42` hot path** — source per-tensor params
  (dims/ggml_type/len) from the manifest instead of `GgufTensorIndex`, so a q42 boot can actually
  run decode (today it structurally maps weights but the hot path still expects a GGUF index);
  (b) JS **ingest pipeline** — `compileGgufToQ42` on first load → stream `.q42` to OPFS (Phase 3
  writer) → boot from `.q42` thereafter; (c) cold CBOR-LD ontology section + `NQuin.metadata` flag
  consumption in-shader; (d) optional single-buffer zero-copy bind (bind q42 blobs directly, skip the
  arena copy) + Web-Worker compiler farm. wasm export not yet rebuilt/deployed (no consumer until the
  JS ingest pipeline lands).

---

## 🏗️ 4. ARCHITECTURAL REFERENCE (THE HOT PATH)

Do not violate these established WebGPU manifold boundaries.

### The Fused Layer (`encode_prefill_q_ffn_tail_fused`)
The authoritative fused WebGPU manifold. All operations (Q-SDPA, o_proj, FFN chain, and elementwise residuals) must queue into a single `CommandEncoder` and submit exactly once at the end of the layer.

### The Disjoint Weight Arena (`Mc8WeightArenaBufs`)
Weights are segregated into 7 disjoint `STORAGE` buffers (`qkv_k`, `qkv_v`, `qkv_q`, `o_proj`, `gate`, `up`, `down`). This prevents asynchronous `queue.write_buffer` calls from clobbering in-flight weight reads. 

### Dynamic Offsets (`has_dynamic_offset: true`)
Parameter buffers (`gemm_params`, `elem_params`, `attn_params`) utilize dynamic offsets aligned to 256 bytes (`MC8_UNIFORM_ALIGN`). This allows multiple dispatches in the same encoder to share a single bind group without pipeline flushes.

### Batched Q-SDPA & Masking
* **Prefill (Dense):** Uses `mask_active = 0` (no sparse slab upload). The causal loop `logical <= abs_pos` is authoritative.
* **Decode (Sparse):** U1 sparse slabs are uploaded and active **exclusively** during $M=1$ decode via `encode_attention_pass_gpu` to prevent OR-poisoning across batched rows.

### Batched GEMM ($M > 1$)
Matrix-Matrix multiplication is enabled. Dispatches use a 2D grid `(⌈N/64⌉, M, 1)` where `global_id.y` represents the token index $M$.