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

---

## 2026-07-09 — Model helper is CBOR-LD, not JSON (Grok)

### Status
**done.**

### What was built
- NEW `q42/model_helper.rs`: typed `ModelHelper` + `ModelHelperTokenizer`
- Encode: **self-describe CBOR** (tag `0xd9d9f7`) + embedded `@context` / `@type` (CBOR-LD document shape)
- Extension: **`.q42.cbor-ld`** (no `.json`)
- CLI convert writes helper via `write_beside_p64`; round-trip self-check on convert
- Load API: `ModelHelper::load_beside_p64` for engine activation later

### Measured
- Unit: `model_helper` **3/3** passed

### ⚑ Human
None. Old `.q42.json` files from prior converts (if any) are obsolete — re-convert.

---

## 2026-07-09 — Vault prefers p64 + helper stops on activate (Grok)

### Status
**done.**

### What was built
| Item | Detail |
|------|--------|
| Vault scan | Lists `.p64` + `.gguf`; **hides GGUF when same-stem `.p64` exists** |
| Resolve | Stem / gguf name → sibling `.p64` preferred |
| Activate | `activate_vault_gguf` + `mount_resident_gguf` accept p64 (format-neutral mount) |
| Discover (app) | `discover_models` lists p64; GGUF with sibling p64 surfaces as p64 |
| Helper on decode | `apply_model_helper_stops` loads `.q42.cbor-ld` beside model path and merges stop ids |
| CLI list | Shows KIND column (`p64` / `gguf`) |

### Measured
- `vault_scan_prefers_p64_over_gguf_same_stem` **ok**
- `merge_stop_token_ids_from_helper` **ok**

### ⚑ Human
None.

### Next
1. Measure f16 vs quant tok/s **using** `a0_decode_profile` + passport (not ad-hoc)
2. CUDA/WMMA on prefill product path **via** forge `gemm_f32_tc` / certify — not a new dispatch stack
3. SoA Q4_K convert layout
4. Use full toolkit map in remediation plan §0-A

---

## 2026-07-09 — Principal steer: use the in-repo optimization toolkit (Grok)

### Status
**noted and written into the plan** (no new kernel this entry).

### What was recorded
Timothy pointed at `docs/manuals/qualia_db_functionality_manual.md` as the reminder that Qualia
already has a large optim stack. The thin overview manual is the map; the *operating* optim
docs are:

- `p64-q42-inference-pipeline.md` — P64 path + **Forge boundary** (certify ≠ own decode)
- `wgsl-forge.md` — generate / validate / certify / tune / auto-tune-all
- `inference-tuning.md` — resident decode/weights, FFN fusion, coop GEMV, GPU top-k, smoke gates
- `model-compression.md` — PTQ/prune independent of GGUF
- `acceleration-integration-map.md` + `WGPU_UPSTREAM_TRACKING.md` — migration + tensor cores

Remediation plan gained **§0-A Optimization toolkit** with a composition rule:
measure with harness + passport; certify with forge; convert-time layouts; engine owns decode
until a toggle-swapped certified kernel lands.

### ⚑ Human
None — steer locked.

### Next execution slice (when go)
1. `llm passport` + `a0_decode_profile` on **p64** smollm2 (and f16 if present) for honest baseline
2. Wire one prefill matmul through `dispatch::gemm_f32_tc` behind toggle, certify on A2000
3. Only then SoA Q4_K layout work

---

## 2026-07-09 — Stage-by-stage toolkit probe + two library fixes (Grok)

### Status
**done** — probe suite + CRC table optim + CUDA soft-fail.

### Method
For each inference-pipeline stage, exercise **existing library functions** with simple
tests (not full decode), print timings, then improve from findings.

| Stage | Library surface | Result |
|-------|-----------------|--------|
| 1 Convert | `compile_gguf_to_p64_with_layout` Verbatim/F16Expand | ~1–4 ms synthetic; works |
| 2 Helper | `ModelHelper` CBOR-LD + `apply_stops_to_tokenizer` | 611 B, merge stops ok |
| 3 Dequant/GEMV | `stack_gemm_quant` ≡ substrate `matvec` | max_err 9.5e-7; stack path **0.5 ms** vs naive dequant+matvec **1.5 s** (f64 matvec cold) |
| 4 Forge GEMM | `gemm_f32` / `gemm_f32_tc` | **bug found**: missing NVRTC panics; fixed soft-fail → floor |
| 5 Top-k | `topk_cpu` | k=8/4096 ok |
| 6 Ternary | `ternary_blob` / `ternary_gemm_cpu` | **ratio ~0.05** vs f32 (real novel rep; product via p64 ternary FFN) |
| 7 Passport | `benchmark_devices` + `load_or_probe` | A2000 Vulkan microbench wins at n=256; cached passport still Dx12@0.11ms |
| 8 Live p64 | `P64TensorIndex::from_p64` on SmolLM2 | **was ~42.5 s**, after CRC table **~3.0 s** (~14×); helper still None (re-convert) |

### Improvements shipped from findings
1. **CRC-32C slice table** (`container_10d/crc32c.rs`) — bit-identical to table-less; p64 validate much faster.
2. **`gemm_f32_tc` soft-fail** — `catch_unwind` around CUDA/NVRTC so missing toolkit does not abort decode/tooling.

### Novel representation opportunities (not yet product-wired)
| Idea | Evidence | Next |
|------|----------|------|
| Ternary FFN p64 | 5% of f32 bytes, GEMV real in-tree | `llm convert` ternary policy + `QUALIA_LLM_TERNARY_FFN` + ΔPPL gate |
| F16 expand | GPU unpack path ready; synthetic F32→F16 size flat (source already dense) | Measure on Q4/Q8 360M/3B |
| Skip re-CRC on trusted activate | 3s still in validate | parse-once + optional `--trust-crc` after convert |
| Passport microbench vs decode | Vulkan faster on small GEMV; Dx12 better for large decode | decode-proxy microbench in passport |
| Forge TC on product path | floor works; CUDA needs NVRTC on PATH | install toolkit or ship runtime; then wire prefill |

### How to re-run
```powershell
cargo test -p qualia-core-db --lib toolkit_probe -- --nocapture
```

### ⚑ Human
- Re-run `llm convert` on smollm2 so `.q42.cbor-ld` attaches (probe saw helper=None).
- Optional: install CUDA NVRTC so forge TC can actually run (today soft-falls to f32).

---

## 2026-07-09 — Plan continue: integrity modes, single parse, reconvert (Grok)

### Status
**done** for this slice.

### What was built
| Item | Detail |
|------|--------|
| **IntegrityMode** | `Full` / `Metadata` / `Structure` via `QUALIA_P64_INTEGRITY` |
| **Measured** | SmolLM2 from_p64: **Full 2410 ms** vs **Metadata 9.0 ms** (~268×) |
| **Single parse** | Decode path: one `from_p64` for tokenizer + tensor index (was 2× full CRC) |
| **Passport** | Stores `preferred_inference_backend` + `probe_gemv_n`; env hint also suggests `QUALIA_P64_INTEGRITY=metadata` |
| **Re-convert** | smollm2 → `.p64` + **`.q42.cbor-ld`** (680 B, ChatMl, 3.9 s) |

### Operator knobs (plan toolkit)
```powershell
$env:QUALIA_P64_INTEGRITY='metadata'   # fast activate after trusted convert
$env:QUALIA_WGPU_BACKEND='dx12'        # or from llm passport --apply-env-hint
```

### Still open in plan
1. Wire forge TC into prefill product path (needs NVRTC for real CUDA win)
2. SoA Q4_K convert layout
3. Ternary FFN productize + ΔPPL
4. Decode-proxy passport (rank by short tok/s, not only GEMV)
5. f16 layout A/B measure on real Q4/Q8 models

### ⚑ Human
None this step. For everyday use after convert: `QUALIA_P64_INTEGRITY=metadata`.

---

## 2026-07-09 — Make it remarkable (Grok)

### Status
**done** — product path is now a composed toolkit story, not a pile of half-wires.

### What shipped
| Change | Why it's remarkable |
|--------|---------------------|
| **Default integrity = Metadata** | Activate ~9 ms vs ~2.4 s Full CRC on SmolLM2 (still bounds-check) |
| **Engine caches P64 + tensor index** | Adopt once → decode never re-parses |
| **`llm convert --layout auto`** (default) | Picks F16Expand when it fits 12 GiB budget |
| **`llm optimize`** | Passport optional + auto convert + prints activate knobs |
| **Live f16 package** | smollm2 Q8 → **693 MiB F16Expand p64 in 19 s** + CBOR-LD helper |

### Operator path (the remarkable one-liner)
```powershell
qualia-cli llm optimize C:\LLM_Models\GGUF\smollm2-360m-instruct-q8_0.gguf --out C:\LLM_Models\P64
# → .f16.p64 + .q42.cbor-ld
# activate with default metadata integrity (or set QUALIA_P64_INTEGRITY=full for audit)
```

### Measured
| Metric | Value |
|--------|-------|
| F16Expand convert | 19.0 s, 693.3 MiB, 290 tensors |
| Full vs Metadata from_p64 | ~2.4 s vs ~10 ms |
| Helper present | F16Expand / ChatMl |

### Still open (honest, for next go)
1. Forge TC on prefill (NVRTC)
2. SoA Q4_K for models too big for f16
3. Ternary FFN productize + ΔPPL
4. Decode-proxy passport ranking
5. Chat end stop still eos-only on this SmolLM vocab probe (`stops=[2]`) — dig vocab specials further

### ⚑ Human
None — try `llm optimize` on a model and chat on the `.f16.p64`.

---

## 2026-07-09 — Gemma-4 E2B convert + BF16 + arch gate (Grok)

### Status
**partial (honest)** — convert/product path done for Gemma-4; coherent decode **not** (architecture is not Llama-shaped). Bigger **working** model proved on Llama-3.2-3B p64.

### What shipped
| Change | Detail |
|--------|--------|
| **BF16 (ggml type 30)** | CPU dequant + GEMM/attention WGSL + quant gates — unblocks Gemma GGUF convert |
| **Gemma-4 → p64** | `gemma-4-E2B-it-Q4_K_M.p64` 3269.9 MiB, 601 tensors, 18–45 s; helper **chat_family=Gemma4** stop_ids `[1,106,212]` |
| **Gemma4 chat template** | `<\|turn>…<turn\|>` family (not classic `<start_of_turn>`) |
| **Arch hyperparams in P64** | head_dim / head_dim_swa / SWA / shared_kv / softcap / architecture / arch_flags |
| **Fail-closed gemma4** | Activate refuses with PLE+SWA+shared-KV+QK-norm missing list (override `QUALIA_LLM_FORCE_UNSUPPORTED_ARCH=1`) |
| **Llama-3.2-3B p64 smoke** | convert 13.6 s → 1924.9 MiB; "capital of France" → **Paris** + `<\|eot_id\|>` |

### Measured (RTX A2000 12GB, DX12)
| Run | Result |
|-----|--------|
| Gemma convert | Verbatim layout (too large for f16 expand on 12 GiB budget) |
| Gemma decode (pre-gate) | ~2.6 tok/s wall, **garbage** multilingual (wrong head_dim/PLE/SWA) |
| Gemma activate (post-gate) | **ERR** architecture gemma4 not supported — correct |
| Llama-3.2-3B known fact | **"The capital of France is Paris."** load 3.8 s |
| Llama open prompts | Still quiz-attractor (known sampling/template issue on bare CLI) |

### Gemma-4 arch facts (why it is not "just bigger SmolLM")
- `general.architecture=gemma4`, n_embd=1536, n_head=8, n_kv=1
- head_dim **512** global / **256** SWA (not n_embd/n_head=192)
- PLE: `per_layer_token_embd` (~half params), inp_gate/proj per layer
- Shared KV last 20 layers; variable FFN 6144 vs 12288; QK-norm; logit softcap 30

### Next (gemma4 graph workstream — not optional polish)
1. Dual head_dim per-layer SWA pattern + RoPE bases
2. PLE inject (per_layer_token_embd + proj/inp_gate)
3. Shared KV reuse for last N layers
4. QK-norm + post_attention/post_ffw norms
5. Variable FFN width in resident plan
6. Softcap logits

### ⚑ Human
None for convert path. For coherent Gemma-4 chat: authorise the gemma4 decoder graph as the next lane (this is a real architecture port, not a config tweak).


---

## 2026-07-09 — Prioritize pipeline optim; Gemma deferred (Grok)

### Status
**direction locked by Timothy:** finish remaining **optim plan** first (pipeline still too slow). **Gemma-4 decoder graph last** (forge can help there later). Convert/fail-closed for gemma stays as-is.

### Honest baselines (A2000 12GB, DX12, p64, resident single-fence)
| Model | tok/s | ms/tok | Path |
|-------|------:|-------:|------|
| smollm2-360m **f16.p64** | **18.3** | 54.7 | 1 fence/tok, COMPUTE-BOUND |
| llama-3.2-3b **Q4_K_M.p64** (before) | **2.01** | 498 | same |
| llama-3.2-3b Q4_K_M (after Q4_K ping-pong barrier) | **2.31** | 433 | ~+15% |

Ollama/llama.cpp CUDA same-class GGUF was ~**70 tok/s** on this machine earlier → still ~**30×** on 3B. SmolLM ~18 tok/s is compute-bound WGSL, not fence-bound.

### This slice shipped
- Q4_K coop GEMV: **ping-pong shared header** (1 barrier/block instead of 2) in `fused_transformer.wgsl` — modest +15% on 3B, not the 30× gap.

### Optim backlog (ordered for tok/s — Gemma NOT in this list)
1. **SoA Q4_K convert layout** + layout-aware GEMV (plan W-K3 / convert-time layout #3)
2. **Wire forge CUDA WMMA into prefill heavy GEMMs** (W-K1/W-K2) — CUDA 13.3 present; `gemm_f32_tc` still soft-falls to plain in unit tests without full NVRTC product wire
3. **Persistent engine worker** — every `infer` rebuilds `QTensorEngine` (TTFT pain; multi-turn pays full plan rebuild)
4. **Decode-proxy passport** (rank by short tok/s, not only GEMV µs)
5. **Ternary FFN productize** when ΔPPL gate holds
6. **f16 A/B** on models that fit VRAM (smollm already has f16.p64)

### Deferred (last)
- Gemma-4 graph: dual head_dim/SWA, PLE, shared KV, QK/post-norms, variable FFN, softcap — after optim path is no longer “terribly slow” on supported models.

### ⚑ Human
None this step. Direction: optim → then Gemma. Next build session should start **SoA Q4_K** or **CUDA prefill wire** (CUDA toolkit is installed).


---

## 2026-07-09 — SoA Q4_K convert + GEMV (Grok)

### Status
**done** for SoA layout product path.

### What shipped
| Piece | Detail |
|-------|--------|
| `GGML_TYPE_Q4_K_SOA=112` | 160 B/superblock: qs[128] + f16 d_sub[8] + f16 m_sub[8] |
| Convert | `P64ConvertLayout::Q4kSoa`; CLI `--layout soa`; **auto** picks SoA when f16 does not fit and source >256 MiB |
| WGSL | Barrier-free coop GEMV SoA path + attention/generic dequant |
| CPU dequant | Round-trip test vs stock Q4_K |

### Measured (A2000 DX12, resident 1-fence)
| Container | tok/s | size |
|-----------|------:|-----:|
| llama-3.2-3b Q4 verbatim p64 | ~2.3 | 1925 MiB |
| llama-3.2-3b **.soa.p64** | **~2.63** | 2069 MiB (~+7.5%) |
| Quality | **Paris** + eot | coherent |

~+14% vs post-ping-pong Q4; ~+31% vs pre-optim ~2.0. Still ~**27×** under Ollama ~70. COMPUTE-BOUND.

### Next optim
1. Persistent engine worker (every infer rebuilds QTensorEngine — kills multi-prompt TPS)
2. CUDA WMMA prefill wire (toolkit present)
3. Further kernel work (tiled GEMV / better occupancy)

### ⚑ Human
None. Prefer `llm convert … --layout soa` (or auto) for Q4 models that cannot fit f16.

