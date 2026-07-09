# Inference p64 pipeline remediation — progress log

Per CLAUDE.md §9. Plan: `docs/plans/native-inference-p64-pipeline-remediation.md`.

---

## 2026-07-09 — W0 + W1 + W2 CLI + W4 passport wire (Grok)

### Status
**done** for this slice (metric, multi-stop, convert CLI, passport CLI + apply path).

### What was built

| Step | Mechanism | Files |
|------|-----------|--------|
| **W0 metric honesty** | CLI was using `token_ids.len()` on the **provenance `Vec`** (often length 1). Now uses `tokens_generated` (3rd return field). | `crates/qualia-cli/src/llm_testing.rs` |
| **W1 multi-stop** | `GgufTokenizer` holds up to 8 stop ids: eos + `<|eot_id|>`, `<|im_end|>`, `<end_of_turn>`, `<|end_of_text|>`, `</s>`, `<|end|>` when present in vocab. Decode breaks on `tok.is_stop_token(next)`. | `gguf_sharder.rs`, `inference_agent.rs` (native + wasm paths, speculative path) |
| **W2 convert CLI** | `qualia-cli llm convert <gguf> --out <dir>` → `<stem>.p64` + `<stem>.q42.json` (stop ids, chat family, source). Self-checks `P64TensorIndex::from_p64`. | `main.rs`, `llm_testing.rs` |
| **W4 passport** | `qualia-cli llm passport [--reprobe] [--apply-env-hint]`. Benchmark ranks **per-backend** (DX12 and Vulkan both appear). `gpu_context` prefers cached passport GPU backend when `QUALIA_WGPU_BACKEND` unset. | `device_benchmark.rs`, `hardware_passport.rs`, `gpu_context/caps.rs`, CLI |

### Measured results
- Unit: `stop_tokens_include_chat_end_specials` **ok**.
- `cargo check -p qualia-core-db --lib` / `qualia-cli` finished clean.
- **Convert** smollm2-360m-instruct-q8_0: **6.3s** → `C:\LLM_Models\P64\smollm2-360m-instruct-q8_0.p64` (370.1 MiB, 290 tensors) + `.q42.json` (ChatMl).
- **Passport** (GEMV 512, fresh probe):
  1. A2000 Dx12 **0.110 ms** (score 1.000) ← selected
  2. A2000 Vulkan 0.112 ms
  3. CPU rayon 0.143 ms
  4. iGPU Dx12 0.431 ms
  5. iGPU Vulkan 0.609 ms  
  Hint written: `QUALIA_WGPU_BACKEND=dx12`.
- Comprehensive-test re-measure with fixed Tokens metric: **not re-run this slice** (long GPU job); W0 fix is correct by API contract + code path.

### ⚑ Where I need the human
None this step.

### Next step
1. Wire decode/prefill to forge tiled WMMA / MatMul selector (W-K product path).
2. Convert-time SoA / f16 layouts in p64 (speed hook).
3. Optional: full comprehensive-test with fixed TPS on 3B for a new baseline table.

### W-K1 honest status (not claimed done as product)
- **Exists:** single-tile WMMA 16×16, tiled WMMA CUDA source, tiled WGSL coopmat emit (probe-gated), subgroup GEMV on decode.
- **Missing:** decode/prefill path still does not dispatch forge CUDA MatMul; closing the Ollama gap requires that wiring + conversion-time layouts.

---

## 2026-07-09 — Continue: f16 layout, fused top1, kernel, Q42T v2 (Grok)

### Status
**done** for this slice.

### What was built
| Item | Detail |
|------|--------|
| **F16 convert layout** | `P64ConvertLayout::{Verbatim,F16Expand}` + `compile_gguf_to_p64_with_layout`; CLI `--layout f16` expands 2-D weights to IEEE half for `unpack2x16float` GEMV |
| **Q42T v2** | Tokenizer section writes/reads stop-token set (chat ends survive p64 round-trip) |
| **Fused output top-1** | When logits are VRAM-resident: **one submit** for all vocab chunks (was per-chunk submit) |
| **Kernel** | `coop_row_dot` f16 fast path; Q4_K d/dmin word-aligned load + word-local nibble extract |
| CLI | `qualia-cli llm convert … --layout f16` writes `*.f16.p64` + helper |

### Measured
- Unit stop-tokens still **ok**
- `cargo check -p qualia-core-db --lib` Finished
- Live f16 convert of smollm2 / tok/s delta: **not measured this step** (next: convert + short decode A/B)

### ⚑ Human
None. Optional: run f16 convert on smollm2 and compare tok/s vs verbatim p64.

### Next
1. Measure f16 vs Q8/Q4 decode on A2000
2. Wire forge CUDA WMMA into prefill dense matmuls when dims allow
3. SoA Q4_K re-layout at convert time
4. Prefer sibling `.p64` in vault scanner
