# New Session Briefing — LLM Token Generation

> Copy everything below this line into the first message of a new Claude Code session
> opened against `C:\Projects\qualiaDB`.

---

I'm continuing development on QualiaDB (`C:\Projects\qualiaDB`), a Rust/Flutter semantic
graph engine with in-process LLM inference.  Read `HANDOVER.md` (top section) and
`CLAUDE.md` before doing anything — CLAUDE.md has hard rules about the inference stack
that must not be violated.

**Branch:** `0.0.6-dev` (or `main` — same content, pushed together).

## What was done last session (do not redo)

The GPU inference layer is now wired:
- `vendor/directml/` — DirectML 1.15 SDK checked in; `build.rs` links it on Windows.
- `src/directml_bridge.rs` — real D3D12 + DirectML GEMM operator with Q4_K dequant.
- `src/metal_bridge.rs` — Accelerate BLAS (`cblas_sgemm`) for macOS Apple Silicon (AMX).
- `gguf_bridge.rs::QTensorEngine` — `load_gguf(path)` memory-maps the file;
  `dispatch_fused_transformer_block()` tries DirectML → Accelerate → wgpu/WGSL in order.
- Linux/NVIDIA: wgpu picks the system Vulkan ICD automatically; no code change needed.

## The task for this session

`infer_local_model()` in `crates/qualia-core-db/src/llm_agent.rs` still outputs **mocked
text** via a hardcoded string fed through the Phase 8 SPSC ring buffers.  The GPU compute
path exists but is never called from the inference function.

Wire real autoregressive token generation:

### Step 1 — Tokenizer (read from GGUF)
`crates/qualia-core-db/src/gguf_sharder.rs::extract_ontology_to_superblock()` currently
returns zeroed bytes.  Parse the GGUF KV metadata section to extract:
- `tokenizer.ggml.tokens` — the vocabulary list
- `tokenizer.ggml.merges` — BPE merge rules
- `tokenizer.ggml.bos_token_id` / `eos_token_id`

Store these in a new `GgufTokenizer` struct.  The GGUF spec for the KV section is in
`vendor/directml/include/DirectML.h` — no, that's wrong, the GGUF spec is at
https://github.com/ggerganov/ggml/blob/master/docs/gguf.md — read it from the Rust
source already in `gguf_sharder.rs` which reads the magic bytes and tensor count.

### Step 2 — Prompt tokenization
Implement `GgufTokenizer::encode(text: &str) -> Vec<u32>` using the BPE merges.
For the initial version a whitespace-split byte-fallback tokenizer is acceptable as
a placeholder that can be replaced.

### Step 3 — Forward pass per token
In `infer_local_model`, for each token position:
1. Embed the token ID as a f32 vector (from `token_embeddings` tensor at the GGUF offset
   stored in the `generate_bidx_pointer_map()` Quin map).
2. Call `QTensorEngine::dispatch_fused_transformer_block()` with the embedding.
3. The output logit vector is sent over the existing `LogitStream` SPSC channel to the
   Webizen Sentinel thread (this part is already wired — keep the Phase 8 bifurcation).
4. The Sentinel reads logits, checks for anomaly signatures, optionally injects
   `DenyRollback` (keep existing logic), then argmax/top-p sample the next token.
5. Push the new token into `final_text`; stop at EOS token or `MAX_OUTPUT_TOKENS`.

### Step 4 — Detokenize
Implement `GgufTokenizer::decode(token_ids: &[u32]) -> String` — map IDs back to
vocabulary strings, join with space or BPE merge rules.

## Key constraints (from CLAUDE.md — mandatory)
- No `Vec`/`String`/`Box` in hot paths — the SPSC ring already uses fixed arrays.
- The Sentinel intercept (`0x99` anachronism check) must remain in the logit-stream loop.
- `orchestrate_inference()` in `orchestrator.rs` gates every call: validate_intent →
  infer → validate_output.  Do not bypass this.
- `infer_local_model` is called from `LocalLlmAgent::infer()` — the memory budget
  guard and timeout guard that wrap it must stay.
- The GGUF model path comes from `AgentBackend::Local { model_path, .. }` — read it
  from there, do not hardcode paths.

## Files to read first
1. `HANDOVER.md` — top section (this session's work)
2. `CLAUDE.md` — hard rules
3. `crates/qualia-core-db/src/llm_agent.rs` — current mock + Phase 8 SPSC
4. `crates/qualia-core-db/src/gguf_sharder.rs` — tokenizer stub
5. `crates/qualia-core-db/src/gguf_bridge.rs` — `load_gguf` + dispatch
6. `crates/qualia-core-db/src/orchestrator.rs` — inference gating

## Secondary tasks (if time permits)
- Add `DirectML.dll` copy step to `.github/workflows/release.yml` Windows Flutter job
  so the release artifact is runnable (currently the DLL is in vendor/ but not copied
  to the artifact directory).
- Wire `release.yml` to also push `vendor/directml/bin/x64-win/DirectML.dll` alongside
  `qualia-cli-windows-x86_64.exe`.
- CUDA path stub: add `cudarc = "0.11"` to non-wasm deps behind `cfg(feature="cuda")`,
  create `src/cuda_bridge.rs` mirroring `directml_bridge.rs` (device init + cuBLAS SGEMM).
  Gate behind `QUALIA_CUDA=1` env var (build.rs already emits the cfg).
