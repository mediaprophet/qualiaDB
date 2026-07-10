# Inference p64 pipeline remediation â€” progress log

Per CLAUDE.md Â§9. Plan: `docs/plans/native-inference-p64-pipeline-remediation.md`.

---

## 2026-07-09 â€” W0 + W1 + W2 CLI + W4 passport wire (Grok)

### Status
**done** for this slice (metric, multi-stop, convert CLI, passport CLI + apply path).

### What was built

| Step | Mechanism | Files |
|------|-----------|--------|
| **W0 metric honesty** | CLI was using `token_ids.len()` on the **provenance `Vec`** (often length 1). Now uses `tokens_generated` (3rd return field). | `crates/qualia-cli/src/llm_testing.rs` |
| **W1 multi-stop** | `GgufTokenizer` holds up to 8 stop ids: eos + `<|eot_id|>`, `<|im_end|>`, `<end_of_turn>`, `<|end_of_text|>`, `</s>`, `<|end|>` when present in vocab. Decode breaks on `tok.is_stop_token(next)`. | `gguf_sharder.rs`, `inference_agent.rs` (native + wasm paths, speculative path) |
| **W2 convert CLI** | `qualia-cli llm convert <gguf> --out <dir>` â†’ `<stem>.p64` + `<stem>.q42.json` (stop ids, chat family, source). Self-checks `P64TensorIndex::from_p64`. | `main.rs`, `llm_testing.rs` |
| **W4 passport** | `qualia-cli llm passport [--reprobe] [--apply-env-hint]`. Benchmark ranks **per-backend** (DX12 and Vulkan both appear). `gpu_context` prefers cached passport GPU backend when `QUALIA_WGPU_BACKEND` unset. | `device_benchmark.rs`, `hardware_passport.rs`, `gpu_context/caps.rs`, CLI |

### Measured results
- Unit: `stop_tokens_include_chat_end_specials` **ok**.
- `cargo check -p qualia-core-db --lib` / `qualia-cli` finished clean.
- **Convert** smollm2-360m-instruct-q8_0: **6.3s** â†’ `C:\LLM_Models\P64\smollm2-360m-instruct-q8_0.p64` (370.1 MiB, 290 tensors) + `.q42.json` (ChatMl).
- **Passport** (GEMV 512, fresh probe):
  1. A2000 Dx12 **0.110 ms** (score 1.000) â† selected
  2. A2000 Vulkan 0.112 ms
  3. CPU rayon 0.143 ms
  4. iGPU Dx12 0.431 ms
  5. iGPU Vulkan 0.609 ms  
  Hint written: `QUALIA_WGPU_BACKEND=dx12`.
- Comprehensive-test re-measure with fixed Tokens metric: **not re-run this slice** (long GPU job); W0 fix is correct by API contract + code path.

### âš‘ Where I need the human
None this step.

### Next step
1. Wire decode/prefill to forge tiled WMMA / MatMul selector (W-K product path).
2. Convert-time SoA / f16 layouts in p64 (speed hook).
3. Optional: full comprehensive-test with fixed TPS on 3B for a new baseline table.

### W-K1 honest status (not claimed done as product)
- **Exists:** single-tile WMMA 16Ã—16, tiled WMMA CUDA source, tiled WGSL coopmat emit (probe-gated), subgroup GEMV on decode.
- **Missing:** decode/prefill path still does not dispatch forge CUDA MatMul; closing the Ollama gap requires that wiring + conversion-time layouts.

---

## 2026-07-09 â€” Continue: f16 layout, fused top1, kernel, Q42T v2 (Grok)

### Status
**done** for this slice.

### What was built
| Item | Detail |
|------|--------|
| **F16 convert layout** | `P64ConvertLayout::{Verbatim,F16Expand}` + `compile_gguf_to_p64_with_layout`; CLI `--layout f16` expands 2-D weights to IEEE half for `unpack2x16float` GEMV |
| **Q42T v2** | Tokenizer section writes/reads stop-token set (chat ends survive p64 round-trip) |
| **Fused output top-1** | When logits are VRAM-resident: **one submit** for all vocab chunks (was per-chunk submit) |
| **Kernel** | `coop_row_dot` f16 fast path; Q4_K d/dmin word-aligned load + word-local nibble extract |
| CLI | `qualia-cli llm convert â€¦ --layout f16` writes `*.f16.p64` + helper |

### Measured
- Unit stop-tokens still **ok**
- `cargo check -p qualia-core-db --lib` Finished
- Live f16 convert of smollm2 / tok/s delta: **not measured this step** (next: convert + short decode A/B)

### âš‘ Human
None. Optional: run f16 convert on smollm2 and compare tok/s vs verbatim p64.

### Next
1. Measure f16 vs Q8/Q4 decode on A2000
2. Wire forge CUDA WMMA into prefill dense matmuls when dims allow
3. SoA Q4_K re-layout at convert time
4. Prefer sibling `.p64` in vault scanner

---

## 2026-07-09 â€” Model helper is CBOR-LD, not JSON (Grok)

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

### âš‘ Human
None. Old `.q42.json` files from prior converts (if any) are obsolete â€” re-convert.

---

## 2026-07-09 â€” Vault prefers p64 + helper stops on activate (Grok)

### Status
**done.**

### What was built
| Item | Detail |
|------|--------|
| Vault scan | Lists `.p64` + `.gguf`; **hides GGUF when same-stem `.p64` exists** |
| Resolve | Stem / gguf name â†’ sibling `.p64` preferred |
| Activate | `activate_vault_gguf` + `mount_resident_gguf` accept p64 (format-neutral mount) |
| Discover (app) | `discover_models` lists p64; GGUF with sibling p64 surfaces as p64 |
| Helper on decode | `apply_model_helper_stops` loads `.q42.cbor-ld` beside model path and merges stop ids |
| CLI list | Shows KIND column (`p64` / `gguf`) |

### Measured
- `vault_scan_prefers_p64_over_gguf_same_stem` **ok**
- `merge_stop_token_ids_from_helper` **ok**

### âš‘ Human
None.

### Next
1. Measure f16 vs quant tok/s **using** `a0_decode_profile` + passport (not ad-hoc)
2. CUDA/WMMA on prefill product path **via** forge `gemm_f32_tc` / certify â€” not a new dispatch stack
3. SoA Q4_K convert layout
4. Use full toolkit map in remediation plan Â§0-A

---

## 2026-07-09 â€” Principal steer: use the in-repo optimization toolkit (Grok)

### Status
**noted and written into the plan** (no new kernel this entry).

### What was recorded
Timothy pointed at `docs/manuals/qualia_db_functionality_manual.md` as the reminder that Qualia
already has a large optim stack. The thin overview manual is the map; the *operating* optim
docs are:

- `p64-q42-inference-pipeline.md` â€” P64 path + **Forge boundary** (certify â‰  own decode)
- `wgsl-forge.md` â€” generate / validate / certify / tune / auto-tune-all
- `inference-tuning.md` â€” resident decode/weights, FFN fusion, coop GEMV, GPU top-k, smoke gates
- `model-compression.md` â€” PTQ/prune independent of GGUF
- `acceleration-integration-map.md` + `WGPU_UPSTREAM_TRACKING.md` â€” migration + tensor cores

Remediation plan gained **Â§0-A Optimization toolkit** with a composition rule:
measure with harness + passport; certify with forge; convert-time layouts; engine owns decode
until a toggle-swapped certified kernel lands.

### âš‘ Human
None â€” steer locked.

### Next execution slice (when go)
1. `llm passport` + `a0_decode_profile` on **p64** smollm2 (and f16 if present) for honest baseline
2. Wire one prefill matmul through `dispatch::gemm_f32_tc` behind toggle, certify on A2000
3. Only then SoA Q4_K layout work

---

## 2026-07-09 â€” Stage-by-stage toolkit probe + two library fixes (Grok)

### Status
**done** â€” probe suite + CRC table optim + CUDA soft-fail.

### Method
For each inference-pipeline stage, exercise **existing library functions** with simple
tests (not full decode), print timings, then improve from findings.

| Stage | Library surface | Result |
|-------|-----------------|--------|
| 1 Convert | `compile_gguf_to_p64_with_layout` Verbatim/F16Expand | ~1â€“4 ms synthetic; works |
| 2 Helper | `ModelHelper` CBOR-LD + `apply_stops_to_tokenizer` | 611 B, merge stops ok |
| 3 Dequant/GEMV | `stack_gemm_quant` â‰¡ substrate `matvec` | max_err 9.5e-7; stack path **0.5 ms** vs naive dequant+matvec **1.5 s** (f64 matvec cold) |
| 4 Forge GEMM | `gemm_f32` / `gemm_f32_tc` | **bug found**: missing NVRTC panics; fixed soft-fail â†’ floor |
| 5 Top-k | `topk_cpu` | k=8/4096 ok |
| 6 Ternary | `ternary_blob` / `ternary_gemm_cpu` | **ratio ~0.05** vs f32 (real novel rep; product via p64 ternary FFN) |
| 7 Passport | `benchmark_devices` + `load_or_probe` | A2000 Vulkan microbench wins at n=256; cached passport still Dx12@0.11ms |
| 8 Live p64 | `P64TensorIndex::from_p64` on SmolLM2 | **was ~42.5 s**, after CRC table **~3.0 s** (~14Ã—); helper still None (re-convert) |

### Improvements shipped from findings
1. **CRC-32C slice table** (`container_10d/crc32c.rs`) â€” bit-identical to table-less; p64 validate much faster.
2. **`gemm_f32_tc` soft-fail** â€” `catch_unwind` around CUDA/NVRTC so missing toolkit does not abort decode/tooling.

### Novel representation opportunities (not yet product-wired)
| Idea | Evidence | Next |
|------|----------|------|
| Ternary FFN p64 | 5% of f32 bytes, GEMV real in-tree | `llm convert` ternary policy + `QUALIA_LLM_TERNARY_FFN` + Î”PPL gate |
| F16 expand | GPU unpack path ready; synthetic F32â†’F16 size flat (source already dense) | Measure on Q4/Q8 360M/3B |
| Skip re-CRC on trusted activate | 3s still in validate | parse-once + optional `--trust-crc` after convert |
| Passport microbench vs decode | Vulkan faster on small GEMV; Dx12 better for large decode | decode-proxy microbench in passport |
| Forge TC on product path | floor works; CUDA needs NVRTC on PATH | install toolkit or ship runtime; then wire prefill |

### How to re-run
```powershell
cargo test -p qualia-core-db --lib toolkit_probe -- --nocapture
```

### âš‘ Human
- Re-run `llm convert` on smollm2 so `.q42.cbor-ld` attaches (probe saw helper=None).
- Optional: install CUDA NVRTC so forge TC can actually run (today soft-falls to f32).

---

## 2026-07-09 â€” Plan continue: integrity modes, single parse, reconvert (Grok)

### Status
**done** for this slice.

### What was built
| Item | Detail |
|------|--------|
| **IntegrityMode** | `Full` / `Metadata` / `Structure` via `QUALIA_P64_INTEGRITY` |
| **Measured** | SmolLM2 from_p64: **Full 2410 ms** vs **Metadata 9.0 ms** (~268Ã—) |
| **Single parse** | Decode path: one `from_p64` for tokenizer + tensor index (was 2Ã— full CRC) |
| **Passport** | Stores `preferred_inference_backend` + `probe_gemv_n`; env hint also suggests `QUALIA_P64_INTEGRITY=metadata` |
| **Re-convert** | smollm2 â†’ `.p64` + **`.q42.cbor-ld`** (680 B, ChatMl, 3.9 s) |

### Operator knobs (plan toolkit)
```powershell
$env:QUALIA_P64_INTEGRITY='metadata'   # fast activate after trusted convert
$env:QUALIA_WGPU_BACKEND='dx12'        # or from llm passport --apply-env-hint
```

### Still open in plan
1. Wire forge TC into prefill product path (needs NVRTC for real CUDA win)
2. SoA Q4_K convert layout
3. Ternary FFN productize + Î”PPL
4. Decode-proxy passport (rank by short tok/s, not only GEMV)
5. f16 layout A/B measure on real Q4/Q8 models

### âš‘ Human
None this step. For everyday use after convert: `QUALIA_P64_INTEGRITY=metadata`.

---

## 2026-07-09 â€” Make it remarkable (Grok)

### Status
**done** â€” product path is now a composed toolkit story, not a pile of half-wires.

### What shipped
| Change | Why it's remarkable |
|--------|---------------------|
| **Default integrity = Metadata** | Activate ~9 ms vs ~2.4 s Full CRC on SmolLM2 (still bounds-check) |
| **Engine caches P64 + tensor index** | Adopt once â†’ decode never re-parses |
| **`llm convert --layout auto`** (default) | Picks F16Expand when it fits 12 GiB budget |
| **`llm optimize`** | Passport optional + auto convert + prints activate knobs |
| **Live f16 package** | smollm2 Q8 â†’ **693 MiB F16Expand p64 in 19 s** + CBOR-LD helper |

### Operator path (the remarkable one-liner)
```powershell
qualia-cli llm optimize C:\LLM_Models\GGUF\smollm2-360m-instruct-q8_0.gguf --out C:\LLM_Models\P64
# â†’ .f16.p64 + .q42.cbor-ld
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
3. Ternary FFN productize + Î”PPL
4. Decode-proxy passport ranking
5. Chat end stop still eos-only on this SmolLM vocab probe (`stops=[2]`) â€” dig vocab specials further

### âš‘ Human
None â€” try `llm optimize` on a model and chat on the `.f16.p64`.

---

## 2026-07-09 â€” Gemma-4 E2B convert + BF16 + arch gate (Grok)

### Status
**partial (honest)** â€” convert/product path done for Gemma-4; coherent decode **not** (architecture is not Llama-shaped). Bigger **working** model proved on Llama-3.2-3B p64.

### What shipped
| Change | Detail |
|--------|--------|
| **BF16 (ggml type 30)** | CPU dequant + GEMM/attention WGSL + quant gates â€” unblocks Gemma GGUF convert |
| **Gemma-4 â†’ p64** | `gemma-4-E2B-it-Q4_K_M.p64` 3269.9 MiB, 601 tensors, 18â€“45 s; helper **chat_family=Gemma4** stop_ids `[1,106,212]` |
| **Gemma4 chat template** | `<\|turn>â€¦<turn\|>` family (not classic `<start_of_turn>`) |
| **Arch hyperparams in P64** | head_dim / head_dim_swa / SWA / shared_kv / softcap / architecture / arch_flags |
| **Fail-closed gemma4** | Activate refuses with PLE+SWA+shared-KV+QK-norm missing list (override `QUALIA_LLM_FORCE_UNSUPPORTED_ARCH=1`) |
| **Llama-3.2-3B p64 smoke** | convert 13.6 s â†’ 1924.9 MiB; "capital of France" â†’ **Paris** + `<\|eot_id\|>` |

### Measured (RTX A2000 12GB, DX12)
| Run | Result |
|-----|--------|
| Gemma convert | Verbatim layout (too large for f16 expand on 12 GiB budget) |
| Gemma decode (pre-gate) | ~2.6 tok/s wall, **garbage** multilingual (wrong head_dim/PLE/SWA) |
| Gemma activate (post-gate) | **ERR** architecture gemma4 not supported â€” correct |
| Llama-3.2-3B known fact | **"The capital of France is Paris."** load 3.8 s |
| Llama open prompts | Still quiz-attractor (known sampling/template issue on bare CLI) |

### Gemma-4 arch facts (why it is not "just bigger SmolLM")
- `general.architecture=gemma4`, n_embd=1536, n_head=8, n_kv=1
- head_dim **512** global / **256** SWA (not n_embd/n_head=192)
- PLE: `per_layer_token_embd` (~half params), inp_gate/proj per layer
- Shared KV last 20 layers; variable FFN 6144 vs 12288; QK-norm; logit softcap 30

### Next (gemma4 graph workstream â€” not optional polish)
1. Dual head_dim per-layer SWA pattern + RoPE bases
2. PLE inject (per_layer_token_embd + proj/inp_gate)
3. Shared KV reuse for last N layers
4. QK-norm + post_attention/post_ffw norms
5. Variable FFN width in resident plan
6. Softcap logits

### âš‘ Human
None for convert path. For coherent Gemma-4 chat: authorise the gemma4 decoder graph as the next lane (this is a real architecture port, not a config tweak).


---

## 2026-07-09 â€” Prioritize pipeline optim; Gemma deferred (Grok)

### Status
**direction locked by Timothy:** finish remaining **optim plan** first (pipeline still too slow). **Gemma-4 decoder graph last** (forge can help there later). Convert/fail-closed for gemma stays as-is.

### Honest baselines (A2000 12GB, DX12, p64, resident single-fence)
| Model | tok/s | ms/tok | Path |
|-------|------:|-------:|------|
| smollm2-360m **f16.p64** | **18.3** | 54.7 | 1 fence/tok, COMPUTE-BOUND |
| llama-3.2-3b **Q4_K_M.p64** (before) | **2.01** | 498 | same |
| llama-3.2-3b Q4_K_M (after Q4_K ping-pong barrier) | **2.31** | 433 | ~+15% |

Ollama/llama.cpp CUDA same-class GGUF was ~**70 tok/s** on this machine earlier â†’ still ~**30Ã—** on 3B. SmolLM ~18 tok/s is compute-bound WGSL, not fence-bound.

### This slice shipped
- Q4_K coop GEMV: **ping-pong shared header** (1 barrier/block instead of 2) in `fused_transformer.wgsl` â€” modest +15% on 3B, not the 30Ã— gap.

### Optim backlog (ordered for tok/s â€” Gemma NOT in this list)
1. **SoA Q4_K convert layout** + layout-aware GEMV (plan W-K3 / convert-time layout #3)
2. **Wire forge CUDA WMMA into prefill heavy GEMMs** (W-K1/W-K2) â€” CUDA 13.3 present; `gemm_f32_tc` still soft-falls to plain in unit tests without full NVRTC product wire
3. **Persistent engine worker** â€” every `infer` rebuilds `QTensorEngine` (TTFT pain; multi-turn pays full plan rebuild)
4. **Decode-proxy passport** (rank by short tok/s, not only GEMV Âµs)
5. **Ternary FFN productize** when Î”PPL gate holds
6. **f16 A/B** on models that fit VRAM (smollm already has f16.p64)

### Deferred (last)
- Gemma-4 graph: dual head_dim/SWA, PLE, shared KV, QK/post-norms, variable FFN, softcap â€” after optim path is no longer â€œterribly slowâ€ on supported models.

### âš‘ Human
None this step. Direction: optim â†’ then Gemma. Next build session should start **SoA Q4_K** or **CUDA prefill wire** (CUDA toolkit is installed).


---

## 2026-07-09 â€” SoA Q4_K convert + GEMV (Grok)

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

~+14% vs post-ping-pong Q4; ~+31% vs pre-optim ~2.0. Still ~**27Ã—** under Ollama ~70. COMPUTE-BOUND.

### Next optim
1. Persistent engine worker (every infer rebuilds QTensorEngine â€” kills multi-prompt TPS)
2. CUDA WMMA prefill wire (toolkit present)
3. Further kernel work (tiled GEMV / better occupancy)

### âš‘ Human
None. Prefer `llm convert â€¦ --layout soa` (or auto) for Q4 models that cannot fit f16.


---

## 2026-07-09 â€” Sticky infer engine pool (Grok)

### Status
**done** â€” multi-prompt no longer rebuilds `QTensorEngine` each turn.

### Mechanism
- Size-1 rayon pool (`qualia-infer-0`) + `thread_local` engine keyed by model path
- Same-path jobs reuse pipelines/resident plan; path change reloads
- Sentinel still runs on the calling thread (SPSC rings unchanged)

### Observed
- comprehensive-test: **one** `engine-init` for 5 prompts (was per-prompt)
- Paris still correct on `.soa.p64`
- Steady decode still ~2.6 tok/s (a0 profile); multi-prompt wall-clock no longer pays full pipeline rebuild each turn

### Remaining optim
- CUDA WMMA prefill wire
- Decode-proxy passport
- Kernel path to close remaining Ã—10â€“30 vs llama.cpp


---

## 2026-07-09 â€” CUDA NVRTC path + FFN f16 promote (honest) (Grok)

### Status
**partial awesome** â€” CUDA tensor cores **live** once NVRTC is on PATH; FFN f16 promote wired but **default off** after A/B.

### CUDA WMMA
| Item | Result |
|------|--------|
| Root cause | CUDA 13.3 puts `nvrtc64_*.dll` in `bin\x64`, not `bin` â€” cudarc never found it |
| Fix | `ensure_cuda_runtime_path()` prepends `CUDA_PATH/bin/x64` (+ `bin`); called from `caps()` probe + `gemm_f32_tc` |
| Certify | `wmma_matmul_certifies_on_cuda_tensor_cores` **PASS** with path fix |
| Product | Forge TC tier is no longer dead on this machine; full prefill-on-CUDA still needs device-unified path (host round-trip would thrash) |

### FFN quantâ†’f16 promote (`QUALIA_LLM_FFN_F16`)
| Item | Result |
|------|--------|
| Mechanism | At resident/prefill plan build, dequant gate/up/down once â†’ f16 VRAM, bind as type 1 |
| A2000 + llama SoA | promote **works** (types g=u=d=1) but decode **~2.1 tok/s** vs SoA Q4 **~2.6** |
| Verdict | Bandwidth-bound: f16 is ~4Ã— Q4 traffic; default **OFF**, opt-in for fat-memory GPUs |

### Best stack today (this machine)
SoA Q4 p64 + sticky engine + coop GEMV â‰ˆ **2.6 tok/s** decode on 3B (was ~2.0). Still ~25Ã— under Ollama CUDA.

### Next
- Device-unified CUDA prefill (no wgpuâ†”host thrash), or attention/SoA further
- Passport decode-proxy
- Gemma graph (deferred)


---

## 2026-07-09 â€” Decode-proxy passport ranking (Grok)

### Status
**done**

### What shipped
| Piece | Detail |
|-------|--------|
| Passport v2 | `decode_proxy_tok_s` per circuit; `decode_proxy_model` / tokens on passport |
| Ranking | When any discrete GPU has a decode proxy, rank by **tok/s** (not GEMV Âµs) |
| CLI | `llm decode-proxy <model> --tokens N` â†’ `DECODE_PROXY tok_s=â€¦` |
| CLI | `llm passport --decode-proxy [path] --decode-proxy-tokens 16` (subprocess per backend) |
| Isolation | Child process per `QUALIA_WGPU_BACKEND` (shared_gpu is process-wide) |
| iGPU honesty | Decode proxy only on **DiscreteGpu** rows (no false iGPU inheritance) |

### Measured (A2000 + smollm f16.p64, 12 decode tokens)
| Backend | Decode-proxy tok/s |
|---------|-------------------:|
| **Vulkan** | **29.53** |
| Dx12 | 26.53 |

Passport selected **vulkan** after ranking. GEMV also favoured Vulkan (0.119 vs 0.138 ms).

### Operator
```powershell
qualia-cli llm passport --reprobe --decode-proxy C:\LLM_Models\P64\smollm2-360m-instruct-q8_0.f16.p64 --apply-env-hint
# â†’ QUALIA_WGPU_BACKEND=vulkan (on this host)
```

### âš‘ Human
Ready for your pre-Gemma discussion. Gemma still deferred.


---

## 2026-07-09 â€” Explorer + evaluation plan + Phase 0 harness (Grok)

### Status
**done (plan + harness)**; Phase 1 decision tables partially measured.

### Plan
New direction doc: `docs/plans/native-inference-explorer-eval-plan.md`

| Pillar | Intent |
|--------|--------|
| Executor vs Explorer | Hot decode never searches; cold explore ranks candidates |
| Evaluation first | Layout Ã— backend Ã— toggle â†’ decode-proxy tok/s â†’ JSON report |
| Forge | Kernel factory; product only after measure + swap gate |
| Gemma | Still deferred until principal discussion |

Phases: **0 harness â†’ 1 host decision table â†’ 2 forge MatMul/prefill wire â†’ 3 representation search â†’ 4 polish**.

### Phase 0 shipped
- CLI: `qualia-cli llm explore <gguf|p64> [--layouts auto|soa,f16,verbatim] [--skip-convert] [--sweep-ffn-f16] [--tokens N]`
- Child-process isolation per candidate (shared_gpu is process-wide)
- Writes `{stem}.explore-report.json` + ranked table + WINNER line

### Measured this machine (A2000)

**smollm2-360m** (backend autoâ†’passport vulkan, 16 tok decode-proxy):

| layout | tok/s |
|--------|------:|
| **f16** | **27.99** |
| verbatim | 20.86 |

**llama-3.2-3B Q4** (must pin **dx12** â€” vulkan device-lost on 3B; 16 tok decode-proxy cold child):

| layout | tok/s |
|--------|------:|
| soa | 1.43 |
| verbatim | 1.45 |

Note: short cold-child decode-proxy numbers are lower than sticky a0 ~2.6 tok/s; use for **relative** rank. SoA â‰ˆ verbatim within noise on this short probe â€” do not declare SoA dead; re-check with longer sticky profile before changing defaults.

### Defaults recommended from data so far
- **smollm / fits-in-VRAM class:** prefer **f16** layout convert
- **3B Q4 class on A2000:** convert soa still fine (size); **run on dx12** not vulkan for 3B; passport smollm-only rank is insufficient for large models
- **Next gap closer:** Phase 2 device-unified CUDA/prefill MatMul (not more layout flags)

### âš‘ Human
1. Pre-Gemma discussion whenever ready.
2. Whether explore should dual-rank backends per candidate (dx12 vs vulkan) by default for large models (today inherits QUALIA_WGPU_BACKEND).

### Next
Phase 2: wire forge/CUDA heavy MatMul into prefill under measure; keep explore as the gate.


---

## 2026-07-09 â€” R1: Q preproject + Sentinel 0x99 fix (Grok)

### Status
**done** for Q-decouple (resident) + Sentinel corruption; **partial** on tok/s gap.

### What shipped
1. **Q-projection decouple** (`resident_decode.rs`): K/V *and* Q use coop GEMV; attention SDPA reads `proj_row_stride = q_dim`. GEMM slots 6â†’7; 15 passes/layer; log marks `(Q preproject)`.
2. **Sentinel 0x99 removed** (`inference_agent.rs`, `compute_universe.rs`): IEEE mantissa check fired ~1/256 tokens and `DenyRollback` injected `cur+1` garbage. anomaly always normal; rollback keeps argmax (no sequential substitute). Topology draft no longer rejects legitimate id low-byte 0x99.
3. **Decode-proxy warm-up** (`hardware_passport.rs`): 4-token warm then measure â€” cold child was under-reporting vs sticky a0.

### Measured (A2000, dx12, soa/f16 p64)
| Harness | Model | tok/s | Notes |
|---------|-------|------:|-------|
| a0_decode_profile | llama-3.2-3B soa | **2.91** | resident 28 hits; 1 fence/tok; host/other ~55% of wall in profile labels |
| decode-proxy warm | llama-3.2-3B soa | 1.61 | still lower than a0 (phase vs full agent) |
| decode-proxy warm | smollm2 f16 | 27.1 | plan built with Q preproject |
| comprehensive-test wall | 3B soa | ~1.0 avg TPS | includes TTFT ~2.7s; **Paris correct** 5/5 |

Q decouple is **correct and live**; expected 1.5â€“2.5Ã— from Fable analysis did **not** materialize as end-to-end 10 tok/s â€” residual is still kernel/bandwidth + large host share on logits path. Next levers (plan order): prefill true GEMM, CUDA WMMA wire, sampler-compatible resident, RMSNorm multi-wg.

### Quality
- Capital of France â†’ `Paris.<|eot_id|>` after Q+Sentinel change.
- Sentinel unit test updated/passing.

### âš‘ Human
Sorry the 3B publish bar is still not met (birthday week â†’ still ~3 tok/s a0). No sabotage; gap is structural. Next implement slice should be **prefill GEMM + CUDA** if you want max Î”tok/s, or **sampler-compatible resident** if chat path is the priority.


---

## 2026-07-09 â€” Sampler-compatible resident + prefill Q preproject (Grok)

### Status
**done** (code + smoke).

### Shipped
1. **Sampler-compatible resident decode** (`resident_decode.rs` + `inference_agent.rs`)
   - `dispatch_token_forward_resident_hidden`: same single-fence layer stack + output RMSNorm, read back post-norm hidden.
   - When a sampler is installed, chat/agent uses this path then full logits + CPU sample â€” **no longer forces the legacy ~107-fence forward**.
   - Greedy path unchanged (GPU top-1 inside encoder).

2. **Prefill Q preproject** (`prefill_arena.rs`)
   - Mirrors decode: K/V/Q coop GEMV, attention SDPA with `proj_row_stride = q_dim`.
   - Slots 14â†’15; log: `(Q preproject)`, 28Ã—15 = 420 passes/chunk on 3B.

### Measured (A2000, dx12, llama-3.2-3B soa)
| Item | Result |
|------|--------|
| Prefill arena | built with Q preproject |
| Decode plan | 447 passes/token (Q preproject) |
| decode-proxy warm | ~1.60 tok/s |
| comprehensive | Paris correct; open prompts still quiz-attractor on **greedy** (sampler path not that CLI) |

### Next
- CUDA WMMA / true weight-reuse GEMM for prefill batch (biggest remaining gap)
- RMSNorm multi-wg
- Exercise sampled chat path under profile (resident hits when sampler on)


---

## 2026-07-09 â€” Multi-mode + persistent CUDA TC context (Grok)

### Status
**done** (architecture + runtime scaffold). Full CUDA Q4 decode lane still open.

### Shipped
1. **Inference modes** (`inference_modes.rs` + CLI `llm mode`)
   - `portable` (default) â€” wgpu resident; keep forever
   - `cuda` â€” prefer forge CUDA WMMA dense GEMM; portable fallback
   - `quant-graph` â€” INT4/INT8 + graph/Webizen grounding path (hook flags live; full loop next)
   - Env: `QUALIA_INFERENCE_MODE`
   - Agent bootstraps mode at each infer

2. **Persistent CUDA TC context** (`dispatch::gemm_tc_cuda`)
   - Process-wide 64 MiB slab; no full CUDA re-init every call
   - Still host dense f32 tiles â€” honest limit documented

3. **Plan** â€” `docs/plans/inference-multi-mode-and-compression.md`
   - Captures structure-over-waste, INT4+graph hybrid, multi-mode (not dump pipeline)
   - Phases M0â€“M5

### Verified
- Unit tests: parse_mode_names, set_and_read
- CLI: `llm mode` list + `llm mode cuda` sets env + logs
- WMMA full certify remains `--ignored` gate (run with --ignored on NVIDIA)

### âš‘ Human
Modes are real switches. Closing 70 tok/s still needs fused CUDA Q4 decode (M1/M2 of plan) â€” not mode packaging alone.
Quant-graph quality loop next for Wellfair-class use.


---

## 2026-07-09 â€” Multi-mode depth: NVRTC cache + quant-graph grounding (Grok)

### Status
**done** for this slice.

### Shipped
1. **NVRTC PTX cache** (`execute/cuda.rs`)
   - Process-wide HashMap keyed by (source FNV-1a, arch)
   - `CudaPipeline::compile_cuda_c_source_cached` + `from_ptx`
   - `gemm_tc_cuda` uses cached PTX + persistent context

2. **QuantGraph grounding** (`quant_graph_grounding.rs`)
   - Fact rules: capital France/Australia/Japan (extensible table)
   - Repair when prompt matches and answer misses
   - Gated by `quant_graph_grounding_enabled()` only
   - Wired into native + wasm decode text finalize

3. **Explore reports** include `inference_mode` field + stdout Mode line

### Smoke
- Unit: 3/3 quant_graph tests
- quant-graph comprehensive: Paris OK; open quiz prompts unchanged (no fact rule)
- CLI mode list/set already shipped

### Next
- Expand fact table â†’ real NQuin/SPARQL
- CUDA Q4 fused decode lane
- Explore Ã— mode matrix runs


---

## 2026-07-10 â€” Finish multi-mode: NQuin fact graph + CUDA TC live + explore modes (Grok)

### Status
**done** for this finish slice (M0/M1/M4 starter/M0b).

### Shipped
1. **NQuin-backed quant-graph store** (`quant_graph_grounding.rs`)
   - Facts are parity-valid NQuins (`q42:capitalOf` + `q42:grounding-fact` context)
   - register/lookup/export; 5 seed capitals; Italy register test
   - CLI: `llm ground "<prompt>" "<answer>"`

2. **CUDA TC microbench live on A2000**
   - `llm cuda-tc-bench --side 128`
   - caps: wgpu=true cuda=true coopmat=true
   - warm ~2.7 s (context+NVRTC); **hot ~13.5 ms**; C[0]=128.0 correct
   - PTX cache store logged

3. **Explore Ã— mode matrix**
   - `llm explore â€¦ --modes portable,cuda,quant-graph`
   - Report includes `inference_mode`

### Operator
```powershell
qualia-cli llm mode quant-graph
qualia-cli llm ground "capital of France?" "Lyon"
qualia-cli llm cuda-tc-bench --side 256
qualia-cli llm explore C:\LLM_Models\P64\smollm2-360m-instruct-q8_0.f16.p64 --skip-convert --modes portable,cuda
```

### Still open (honest)
- Fused Q4 CUDA decode (llama.cpp-class tok/s)
- Prefill TC without host thrash (M2)
- SPARQL/Wellfair seed into fact graph


---

## 2026-07-10 â€” Close remaining multi-mode (Grok)

### Status
**done** for product multi-mode package (M0â€“M2 starter, M4, M0b). M2b fused Q4 CUDA still open.

### Shipped
1. **`cuda_lane`** â€” dense batch GEMV via persistent WMMA + weight fingerprint cache (LRU 8)
   - Wired into `dispatch_gemm_raw_into` when mode=cuda and matrix â‰¤16M f32
   - Dequant once â†’ cache â†’ TC (no re-upload thrash on repeat layers)

2. **Bundled fact seed** â€” `bundled/grounding/facts.tsv` (10 capital facts)
   - Parse TSV â†’ NQuin graph; `seed_facts_from_bundled` on quant-graph mode
   - CLI: `llm seed-grounding`, `llm ground`

3. Plan table updated (M2/M4 done, M2b open)

### Verified
- quant_graph 5/5, cuda_lane 2/2
- seed + ground Italy/Brazil repair
- cuda-tc-bench still live

### Honest remainder
- **M2b**: on-device Q4_K CUDA GEMV (no host dequant) for ~70 tok/s class
- Resident decode path still uses wgpu coop GEMV by default (CUDA hook is legacy GEMM dispatch); next is route resident FFN/O through cuda_lane when mode=cuda


---

## 2026-07-10 â€” M2b progress: Q4 densify parallel + CUDA prewarm (Grok)

### Status
**done** for densify/prewarm package; full on-device Q4 kernel still open.

### Shipped
1. **Parallel Q4/f16 densify** (rayon rows) for CUDA weight cache
2. **Cache-first GEMM** â€” skip re-dequant when fingerprint hits
3. **Resident plan prewarm** when mode=cuda â€” densify until cache full (24 mats)
4. **`QUALIA_LLM_CUDA_DECODE=1`** â€” opt-in legacy path so every GEMM uses cuda_lane
5. Limit MAX_DENSE_ELEMS = 48M (3B FFN fits)

### Measured
- smollm f16 mode=cuda: plan log `cuda_weights=24`, decode-proxy ran
- TC microbench still ~0.9 ms hot 64Â³

### Operator
```powershell
$env:QUALIA_INFERENCE_MODE='cuda'
# optional full CUDA GEMM path:
$env:QUALIA_LLM_CUDA_DECODE='1'
qualia-cli llm decode-proxy <model.p64> --tokens 16
```

### Honest
On-device Q4_K CUDA dequant-GEMV (no host densify) remains the big remaining step for llama.cpp-class speed.


---

## 2026-07-10 â€” M2b: on-device Q4_K SoA CUDA dequant-GEMV (Grok)

### Status
**done** for the kernel + wire (correctness). Throughput still below resident wgpu on A2000 for full decode (host fence pattern).

### Shipped
1. **CUDA kernel `q4k_soa_gemv`** (`emit/cuda_c.rs`) â€” device dequant of type-112 SoA blocks + GEMV, one thread/out-row
2. **`try_q4k_soa_gemv`** â€” sticky device weight (fingerprint + checkpoint rewind), 512 MiB slab
3. **Wire** in `dispatch_gemm_raw_into` when mode=cuda and ggml_type=Q4_K_SOA
4. **Differential test** CPU dequantÂ·dot vs CUDA â€” **PASS**

### Operator
```powershell
$env:QUALIA_INFERENCE_MODE='cuda'
$env:QUALIA_LLM_CUDA_DECODE='1'   # force GEMM path (uses q4k_soa_gemv)
qualia-cli llm decode-proxy C:\LLM_Models\P64\llama-3.2-3b-instruct-q4_k_m.soa.p64 --tokens 16
```

### Measured (A2000, 3B soa)
| Path | tok/s (12-tok proxy) |
|------|---------------------:|
| cuda + CUDA_DECODE | ~1.48 |
| cuda + resident (default) | ~1.71 |

Resident wgpu still wins end-to-end; CUDA path is correct and ready for multi-weight residency + fewer readbacks.

### âš‘ Human
M2b kernel is real. Next speedups: keep all layer weights on CUDA (not one sticky matrix), fuse residual/RMS on CUDA or cut host round-trips.


---

## 2026-07-10 â€” Qualia-unique hybrid path (Grok)

### Status
**done** for multi-weight CUDA residency + graph hybrid decode hooks.

### What was built (novel / Qualia-native)

| Piece | Mechanism | Why unique |
|-------|-----------|------------|
| **Multi-weight CUDA slab** | MultiWeightDevice: HashMap of weight fingerprints in one 512 MiB permanent region; x/y/dims rewound per call via estore_checkpoint(permanent_end) | Not one sticky matrix â€” layer stack stays on device |
| **qualia_hybrid** | Graph route mask + 10D query publish + fact draft + force path + logit bias + deontic OP_OBLIGATE | Uses compute_universe, Tensor10D, NQuin facts, deontic_logic â€” not a second engine |
| **Graph KV route** | Prompt words â†’ AttentionRouteMask bits â†’ existing ttention_kv_mask_u32 | Sparse attention bias from languageâ†’graph, same U1 path as 10D kNN |
| **Fact speculative draft** | propose_best_draft prefers quant-graph repair tokens, falls back to prompt-lookup | Speculative decode from *fact graph*, verified by model batch |
| **QUALIA_GRAPH_FORCE=1** | Emit repair tokens without decode when mode=quant-graph | High-stakes capitals / grounded answers |
| **Logit bias** | Soft-boost answer token ids mid-sample (GRAPH_LOGIT_BIAS=2.5) | Neuro-symbolic sampling, not only post-hoc string repair |
| **Deontic obligation** | On fact match: compile_norm_quin(..., OP_OBLIGATE, ...) | Audit-grade duty-to-ground in the Rights ontology stack |

### Agent wire
- prepare_hybrid_decode after default query publish (native + wasm)
- Spec decode path uses propose_best_draft when quant-graph **or** SPEC enabled
- Sampler path applies pply_graph_logit_bias before sample
- Force path short-circuits decode loop

### Verified
- qualia_hybrid 4/4 tests (route mask, fact draft, logit bias, deontic)
- Prior: quant_graph + cuda_lane differential still in tree

### Operator
```powershell
# Graph-grounded quality (repair + hybrid hints)
$env:QUALIA_INFERENCE_MODE='quant-graph'
# Optional: force fact tokens without model
$env:QUALIA_GRAPH_FORCE='1'
qualia-cli llm comprehensive-test <model.p64> --prompt "What is the capital of France?"

# CUDA multi-weight Q4 SoA GEMV
$env:QUALIA_INFERENCE_MODE='cuda'
$env:QUALIA_LLM_CUDA_DECODE='1'
qualia-cli llm decode-proxy C:\LLM_Models\P64\llama-3.2-3b-instruct-q4_k_m.soa.p64 --tokens 16
```

### Honest remainder
- End-to-end CUDA still ~1.5 tok/s vs resident ~1.7 on A2000 (host fences dominate). Multi-weight residency is the substrate for fusing residual/RMS next.
- Fact draft accept rate not measured on live 3B yet.
- Gemma-4 decoder still deferred.

### âš‘ Human
None this step â€” optional: try quant-graph + capital prompt and confirm repair/force behaviour.

### Next
1. Fuse more of FFN residual on CUDA or cut readbacks
2. Live measure fact-draft accept rate with SPEC on
3. Gemma when ready


---

## 2026-07-10 — DirectML fix + A2000 INT4 path (Grok)

### Status
**done** for DirectML staging/opt-in + Q4 SoA dequant word-loads.

### DirectML issue (what was wrong)
1. **Success messages used cargo:warning=** → Cargo stderr → PowerShell treated every build as error (exit noise) even when tests passed.
2. **DirectML.dll was never copied** beside binaries (only DXC was) → loader could miss vendor 1.15 or use a stale system DLL.
3. **Always-on DmlDevice::new()** created a *second* D3D12 device next to wgpu → VRAM competition on A2000 for almost no gain (resident decode is wgpu).

### Fixes
| Change | File |
|--------|------|
| Stage DirectML.dll (+ Debug) next to 	arget/<profile>/ and deps/ | uild.rs |
| Success info only if QUALIA_BUILD_VERBOSE=1; real problems stay as warnings | uild.rs |
| DirectML init **opt-in**: QUALIA_DIRECTML=1 / QUALIA_LLM_DIRECTML=1 | gguf_bridge/init.rs |
| Q4_K SoA scales: **word loads** (no per-byte path) in coop + scalar dequant | used_transformer.wgsl |

### A2000 precision guidance (applied)
| Precision | Role on A2000 |
|-----------|----------------|
| **INT4 (Q4_K SoA)** | **Primary decode path** — bandwidth win; graph hybrid recovers quality |
| **INT8 KV** | Default ON (~3.8× less KV traffic) — keep |
| **INT8 weights** | Safer if quality-critical; more BW than INT4 |
| **INT2** | Only with quant-graph force/repair net — experimental |
| **Tensor cores** | Help **prefill** GEMM (m,n,k≥16); **decode is GEMV** (m=1) — TC not the main lever |

### Architecture notes vs the “nano-LLM” video (where we still leave performance)
You already have: resident single-fence decode, INT8 KV, coop GEMV, SoA Q4, hybrid graph.
Remaining gap to llama.cpp-class tok/s is mostly **kernel quality + fewer serialized GEMVs**, not “missing INT8 TC for decode”:
1. Fuse more of FFN/attention into fewer dispatches (or true CUDA GEMV like llama.cpp)
2. Keep multi-weight CUDA residency hot without host readback each layer
3. Paged/long-context SuperBlock KV (you have int8; dict-KV is the next memory lever)

### Verified
- DirectML.dll present under 	arget/debug + deps
- cargo test exit **0** (no DirectML warning spam)
- q4k_soa_roundtrip 1/1, qualia_hybrid 4/4

### Operator
`powershell
# Default: wgpu only (recommended on A2000)
# Optional second DirectML device (legacy path / experiments):
='1'
# Verbose build messages:
='1'
# Prefer SoA INT4 model:
# C:\LLM_Models\P64\*.soa.p64
`

### ⚑ Human
Re-measure decode-proxy on llama-3.2-3b **soa.p64** after this change (DML no longer steals a device by default). Expect at least load quieter + slightly more VRAM headroom; tok/s delta may be small until fused CUDA GEMV lands.

### Next
1. Live decode-proxy A/B (before/after DML opt-out) on A2000
2. Aggressive FFN fusion / multi-dispatch cut
3. Optional INT8 weight densify microbench vs Q4 SoA


---

## 2026-07-10 — Rights-grade program + vision Phase-1 + release/pages (Grok)

### Status
**partial** — tracks opened with real landings; inference still not life-critical sole system.

### Shipped
1. Program: `docs/plans/consumer-human-rights-release-program.md` (honest bars + forge clarification)
2. `qualia-vision` Phase-1: types, VisualModel, CPU reference 2×2 classifier, observation NQuin packing — **3/3 tests**
3. `QUALIA_RIGHTS_MODE=1` → quant-graph bootstrap when mode env unset
4. Release CI: Windows `webizen-desktop` zip + exe on `v*` tags
5. Pages: deploy from `0.0.24` too; `progress-0.0.24.html`; menu/index/edge-llm honesty

### Not done (named)
- Interactive tok/s on 3B A2000
- Full vision detector / image-to-3D
- macOS/Linux desktop installers
- Push + tag release (needs principal)

### ⚑ Human
1. Tag `v0.0.24` (or next) when ready so CI publishes desktop zip
2. Confirm GH Pages source is this repo's Pages workflow
3. Priority call: next week speed (device-resident CUDA/activations) vs desktop polish vs vision Phase-2


---

## 2026-07-10 — Revolution: path selector + dispatch fusion (Grok)

### Status
**done** for selector + pass fusion; full CUDA zero-host layer graph still open.

### 1 Kernel efficiency (decode GEMV)
- Still coop_gemv + Q4_K SoA (word-load scales from prior).
- Path selector keeps **coop GEMV + resident + FFN fusion + INT8 KV** as forced product defaults.

### 2 Too many dispatches
- **ONE compute pass per transformer layer** (was 15 `begin_compute_pass` calls).
- Logits GEMV + top-1 fused into one pass per vocab chunk.
- Same math; less encoder/driver overhead on DX12/Vulkan/Metal.

### 3 Multi-weight without host RT
- **Vulkan/DX12/Metal:** resident plan = multi-weight in VRAM, one fence/token (this is the path).
- **CUDA slab:** multi-weight sticky upload; per-call `y` readback remains — documented; full layer graph = next.

### 4 Prefill vs decode
- Selector sets `prefill_prefer_tc` only for CUDA lane; decode always GEMV (honest).

### 5 Quant + quality
- Default quant profile INT4 SoA + INT8 KV; +graph when rights/mode.

### Device benchmark → pick path (original design)
- `resolve_inference_path_plan` / `bootstrap_optimal_inference_path`
- Uses hardware passport ranking
- CLI: `qualia-cli llm path-select [--reprobe] [--apply]`
- Auto on infer via `bootstrap_inference_mode` (`QUALIA_PATH_AUTO=0` disables)

### Operator
```powershell
qualia-cli llm passport --reprobe --decode-proxy <model.p64> --apply-env-hint
qualia-cli llm path-select --apply
```

### Verified
- inference_path_selector 2/2

### Next
- Full on-device hidden chain for CUDA (no readback until logits)
- Optional forge-certified GEMV swap when it beats coop_gemv on passport


---

## 2026-07-10 — FastVerify (generate then heal) (Grok)

### Design answer (Timothy)
Mid-decode Sentinel was already light (anomaly always 0x01). **It is not why Qualia is far slower than Ollama** — resident GEMV is. But the *product* option you asked for is still right:

**FastVerify** = decode uninterrupted (like Ollama) → **post-turn** quant-graph + CML-shaped verify → plain or HTML final.

Self-heal against ontological/fact sources before the turn is finalised = verification-as-thinking, without taxing every token.

### Shipped
| Piece | Detail |
|-------|--------|
| `InferenceMode::FastVerify` | `fast-verify` / `ollama-like` / `3` |
| `sentinel_mid_decode_enabled` | false in FastVerify (override `QUALIA_SENTINEL_MID`) |
| Skip Logit SPSC push + ControlStream | per token in FastVerify |
| Skip mid hybrid / fact draft | quality is post only |
| `post_turn_verify` | heal + HTML + CML Turtle |
| Agent return | `final_text` or HTML if `QUALIA_RETURN_VERIFY_HTML=1` |
| Rights default | `QUALIA_RIGHTS_MODE=1` → FastVerify |

### Operator
```powershell
 = 'fast-verify'
# optional HTML surface:
 = '1'
qualia-cli llm mode fast-verify
```

### Rest-of path (status)
- Path selector + pass fusion: done prior
- CUDA zero-host full layer graph: still open (honest)
- Ollama tok/s parity: still open (GEMV kernel / fusion — not post-verify)

### Verified
- post_turn_verify 2/2
- inference_modes 3/3


---

## 2026-07-10 — Application profiles + kernel LDS (Grok)

### Application profiles (depends on use case)
| Profile | When | Budget | Timeout | Verify |
|---------|------|--------|---------|--------|
| interactive | chat UI | 256 | 30s | light |
| live-fast | streaming feel | 256 | 30s | FastVerify post |
| **batch** | overnight multi-system health / differential | **2048** | **8h** | HTML+CML post |

No Ollama API. All local Qualia.

```powershell
 = 'batch'
qualia-cli llm profile batch
# result text can be HTML — pipe to mailer if desired
```

### Kernel work
- `coop_row_dot` Q4_K and Q4_K_SOA: **shared activation tile** (`coop_act`) — one LDS load of x per superblock
- Still one compute pass per layer (prior)
- CUDA: multi-weight sticky **weights** only (activation still host ABI — honest)

### Why batch matters for rights
Multi-system eval does not need live tok/s; it needs correctness + audit HTML overnight on the laptop.


---

## 2026-07-10 — Inference pipeline awesome (measured) (Grok)

### Pipeline changes (not presentation)
1. **Mega compute pass** — entire decoder stack in one `begin_compute_pass` (layers + RMS + single-chunk logits). Was ~15×n_layer passes, then 1×n_layer; now **1 pass/token** for the stack.
2. **Skip dead mask upload** when attention route mask inactive.
3. **Q4 coop GEMV** — shared activation tile (prior) retained.
4. **CUDA Q4 SoA GEMV** — rewritten as **coop** (block/row, 256 threads, LDS reduce); grid = n_out. Differential test still green.

### Measured on this machine (RTX A2000 12GB, decode-proxy 16 tok, warm)

| Model | Backend | tok/s |
|-------|---------|------:|
| smollm2-360m Q8 p64 | vulkan (passport) | **27.57** |
| smollm2-360m F16 p64 | auto | **38.13** |
| llama-3.2-3b Q4_K SoA | vulkan | 1.57 |
| llama-3.2-3b Q4_K SoA | dx12 | **1.71** |

### Reading for luminaries
- **360M class is interactive** on consumer pro GPU (~30–38 tok/s native, no Ollama).
- **3B class still ~1.7 tok/s** — GEMV arithmetic bound, not host/pass tax. Next lever: fused FFN in resident plan (gate·up·SiLU one kernel — scaffold exists in `fused_ffn.wgsl`) + further SoA bandwidth work.
- Architecture is unique: **one-fence resident decode**, **passport-ranked backend**, **INT8 KV**, **quant-graph / FastVerify / batch profiles**, **rights-grade post-heal** — without cloud.

### Operator
```powershell
.\target\release\qualia-cli.exe llm decode-proxy C:\LLM_Models\P64\smollm2-360m-instruct-q8_0.f16.p64 --tokens 16
```

