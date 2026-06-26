# Native LLM Performance Review - Codex - 2026-06-25

## Executive summary

The native LLM path has made real progress: the checked-in benchmark history shows SmolLM2-360M moving from roughly 0.45-0.49 tok/s to roughly 3.2-3.8 tok/s on the current A0/A1b runs. That is still much too slow for a 360M local model, and the remaining bottleneck appears structural rather than a single bad quantization kernel.

The main problem is CPU/GPU ping-pong. Native decode still runs a sequential layer loop where attention, output projection, and FFN sub-ops submit GPU work and read results back to CPU repeatedly. The WASM path contains more ambitious MC8 residency/fusion work, but much of that is behind `#[cfg(target_arch = "wasm32")]`. Native currently keeps the GGUF mmap resident, not the compiled engine, not all layer weights, and not a fused decode graph.

There is also a user-visible runaway behavior: production native decode defaults to `MAX_OUTPUT_TOKENS = 2048`, and the 30 second timeout is checked only after the blocking inference call returns. At the current measured 3.2 tok/s, a no-EOS response can take about 10.7 minutes. At the older 0.48 tok/s baseline, it could take more than an hour. That alone can make the library feel frozen.

## Evidence reviewed

- Current A0 CSV: `benchmarks/results/llm_a0_baseline.csv`
  - SmolLM2-360M Q8: 3.19 decode tok/s, 4.72 prefill tok/s, 1.58s warm TTFT, 7.55s warm total for 20 output tokens.
  - SmolLM2-360M Q4_K_M: 3.63 decode tok/s, 5.43 prefill tok/s, 1.34s warm TTFT, 18.76s warm total for 64 output tokens.
- Older A0 log: `benchmarks/results/_a0_remeasure_step2.log`
  - Q8 around 0.49 tok/s and Q4_K_M around 0.45 tok/s; single benchmark run took about 19 minutes.
- A1b ternary log: `benchmarks/results/_a1b_inc4_mvpp2.log`
  - Ternary GPU FFN reached 5.01 tok/s, but output quality collapsed into repetition.
  - Q8 GGUF baseline in that run was 3.75 tok/s.
- Main code paths reviewed:
  - `crates/qualia-core-db/src/llm_agent.rs`
  - `crates/qualia-core-db/src/gguf_bridge.rs`
  - `crates/qualia-core-db/src/gguf_sharder.rs`
  - `crates/qualia-core-db/src/directml_bridge.rs`
  - `crates/qualia-core-db/src/resident_model.rs`
  - `crates/qualia-core-db/src/llm_bench.rs`
  - `crates/qualia-client-core/src/chat_inference.rs`
  - `crates/qualia-client-core/src/model_lifecycle.rs`

I did not rerun long benchmarks; the repo already contains multi-minute and multi-hour benchmark artifacts, and this review was focused on implementation diagnosis.

## Highest priority findings

### P0 - Timeout and cancellation do not stop the native decode loop

`MAX_OUTPUT_TOKENS` is 2048 in production native builds (`llm_agent.rs:31`, `llm_agent.rs:40`). `INFERENCE_TIMEOUT_MS` is 30 seconds (`llm_agent.rs:55`), but `infer()` calls `infer_local_model()` synchronously and checks the deadline after the call returns (`llm_agent.rs:1799-1816`). The streaming chat path calls `infer_local_model_streaming()` directly (`chat_inference.rs:255-265`), and cancellation only suppresses streamed deltas or checks after completion (`chat_inference.rs:296-298`).

Why this matters: at 3.2 tok/s, 2048 tokens is roughly 10.7 minutes. At the older 0.48 tok/s baseline, it is roughly 71 minutes. If EOS is not reached, the app can appear hung.

Recommendation:

- Add a cancellation/deadline callback or atomic flag into `infer_local_model_inner()` and check it inside `for step in 0..gen_budget` (`llm_agent.rs:963`, `llm_agent.rs:1434`).
- Cap default chat generation to a much smaller value, such as 128 or 256, until decode throughput is fixed.
- Make timeout cooperative, not post-hoc. The decode loop should break before the wall clock deadline.

### P0 - Native decode is dominated by repeated GPU submit/readback synchronization

The native forward path is sequential: `dispatch_transformer_forward()` loops over every layer (`gguf_bridge.rs:6092-6149`). Each layer calls attention and FFN code that repeatedly submits GPU work and maps a staging buffer back to CPU.

Examples:

- GEMM writes input/weights/params, creates a bind group, submits, copies to staging, maps, waits, and copies to CPU (`gguf_bridge.rs:4162-4271`).
- Attention does the same pattern when readback is requested (`gguf_bridge.rs:4963-5213`).
- FFN performs gate GEMM, up GEMM, CPU SiLU/multiply, then down GEMM (`gguf_bridge.rs:4703-4797`).
- Output argmax loops vocabulary chunks and dispatches a GEMM/readback per chunk (`gguf_bridge.rs:4364-4435`).
- `poll_wait()` counts the blocking GPU waits (`gguf_bridge.rs:3720-3728`), which confirms this is a known instrumentation point.

Recommendation:

- Treat "fewer submit/readback boundaries" as the primary native performance lever.
- Keep hidden state, attention output, FFN intermediates, and logits on GPU across a layer or full token.
- Read back only the final selected token or top-k candidates.
- First practical milestone: fuse gate/up/SwiGLU/down enough to avoid CPU readback between FFN sub-ops.
- Second milestone: encode an entire layer, then multiple layers, into one command buffer with resident weights.

### P0 - The default output projection path does not use the resident/top-k fast path

Native has a GPU top-k path (`dispatch_output_topk_chunked`, `gguf_bridge.rs:4492-4665`) and native can upload the output projection once (`mc8_upload_resident_logits`, `gguf_bridge.rs:3673-3718`). But the decode loop only uses top-k when `gpu_topk_enabled()` is true (`llm_agent.rs:951-1059`), and that flag is default off (`llm_bench.rs:57-73`).

When top-k is off, decode falls back to `dispatch_output_argmax_chunked()` (`llm_agent.rs:1063-1070`), which loops through vocab chunks and calls `dispatch_gemm_raw_into()` on each chunk (`gguf_bridge.rs:4393-4413`). That fallback fetches chunk bytes and writes them to the staging weight buffer; it does not use the resident logits buffer.

Recommendation:

- Enable GPU top-k by default when no sieve mask is active.
- Make argmax use the resident logits buffer too, or retire the full-logit readback fallback from normal operation.
- Add a benchmark column/counter for "topk path used" and "argmax fallback used" so regressions are visible.
- Implement sieve-compatible top-k masking so safety mode does not permanently force the slow path.

### P1 - "Resident model" keeps bytes resident, but not the execution engine

`mount_resident_gguf()` creates a `QTensorEngine`, loads the GGUF, then stores only the mmap and report in `ResidentModelSlot` (`resident_model.rs:24-47`). On each inference, `llm_agent.rs` creates a fresh `QTensorEngine::new()` inside a spawned thread (`llm_agent.rs:771-804`). That recreates pipelines, reparses tokenizer metadata, reparses tensor metadata, and redoes buffer setup.

The benchmark docs acknowledge this: warm TTFT includes pipelines being rebuilt per call (`llm_bench.rs:12-16`). The current CSV still shows 220-329 ms of warm "load" per call.

Recommendation:

- Introduce a persistent native LLM worker per resident model.
- Keep `QTensorEngine`, `GgufTensorIndex`, `GgufTokenizer`, resident logits, and later resident layer weights alive with the model.
- If wgpu/DirectML handles are awkward to move, keep them on the worker thread and send inference jobs over channels.
- Separate "model is mapped" from "model is execution-ready" in telemetry.

### P1 - Native does not have the WASM path's full residency/fusion work

The codebase contains substantial MC8 work for resident all-layer weights, resident norms, prefill staging, and chunked encoder submission, but much of it is `#[cfg(target_arch = "wasm32")]`. On native, `adopt_resident_mmap()` uploads only resident logits (`gguf_bridge.rs:3611-3644`). The WASM adoption path additionally calls `mc8_upload_all_resident_weights()` and `mc8_upload_resident_norms()` (`gguf_bridge.rs:3777-3789`).

The native layer path still calls `write_weight_words()` for every GEMM or attention projection (`gguf_bridge.rs:4151-4159`, `gguf_bridge.rs:4201-4205`, `gguf_bridge.rs:5108`). That means the same weights are streamed repeatedly during decode.

Recommendation:

- Port resident all-layer weight arenas to native.
- Start with SmolLM2 dimensions and quant types already in the benchmarks.
- Keep VRAM accounting explicit: current SmolLM2 Q8 is about 386 MB mapped plus 80 MB KV cache, which is small enough for an A2000-class GPU if staged sensibly.
- Make fallback counters visible when residency is skipped.

### P1 - DirectML availability is reported as if it means DirectML inference

`GgufLoadReport.directml_enabled` is set from `self.dml.is_some()` (`gguf_bridge.rs:3475-3484`, `gguf_bridge.rs:3654-3663`). But the real GGUF layer path uses `dispatch_gemm_raw_into()` and wgpu shaders; the DirectML helper is in `dispatch_fused_transformer_block()` (`gguf_bridge.rs:3970-4149`), which is not the main real transformer path. The DirectML bridge itself dequantizes Q4_K into a fresh `Vec<f32>` (`directml_bridge.rs:268-281`) and compiles/binds DirectML operator state inside each `execute()` call (`directml_bridge.rs:590-742`).

Recommendation:

- Rename the report field to `directml_device_available` unless real DirectML kernels are used by the hot path.
- Add backend counters: `wgpu_gemm_calls`, `dml_gemm_calls`, `cpu_gemm_fallback_calls`, `cpu_attention_calls`, `topk_calls`.
- Either build a cached DirectML graph/operator path or remove DirectML from the performance-critical plan for now. The current implementation is not a credible low-latency GEMM path.

### P1 - Prefill is still mostly serial

Native prefill batches K/V, but then loops over each token for Q+FFN (`gguf_bridge.rs:5660-5804`). Inside that per-token path it performs attention readback, output projection, FFN gate/up/down, and residual work. Current prefill is only around 4.7-5.4 tok/s in the A0 CSV.

Recommendation:

- Port the more batched MC8 prefill design to native.
- At minimum, keep Q+FFN for a prefill chunk on GPU rather than walking tokens on CPU.
- Benchmark prefill separately from decode; slow prefill becomes very visible once chat prompts include retrieved context.

### P2 - Tokenizer and tensor metadata are reparsed and some tokenizer operations are asymptotically expensive

Every native inference rebuilds the tokenizer and tensor index from the mmap (`llm_agent.rs:814-838`). `GgufTokenizer::from_gguf()` builds vocab maps from GGUF string arrays (`gguf_sharder.rs:856-950`). The BPE merge lookup does a linear `.position()` over merge pairs (`gguf_sharder.rs:1223-1227`), which can become expensive for long augmented prompts.

Streaming decode also calls `tok.decode(&out_ids)` repeatedly and diffs the full string (`llm_agent.rs:1174-1180`), making streaming text assembly O(tokens squared).

Recommendation:

- Cache tokenizer and tensor index with the resident model.
- Replace BPE merge lookup with a `HashMap<(token_id_or_string, token_id_or_string), rank>` or equivalent ranked pair map.
- Add an incremental decoder for streaming deltas.

### P2 - LoRA path allocates in the per-token loop

If a LoRA adapter is active, decode clones the current embedding into a `Vec<f32>` for every token (`llm_agent.rs:1021-1026`). This is not the main bottleneck today, but it violates the intended hot-path discipline and will become noticeable after GPU sync is reduced.

Recommendation:

- Use a caller-supplied fixed scratch buffer for the LoRA input snapshot.
- Or adjust `apply_cpu()` so it can safely read from one stack slice and write to another without heap allocation.

### P2 - Fallback behavior can hide failed output projection

If output argmax/top-k fails, the code falls back to folding over the hidden embedding vector as if it were logits (`llm_agent.rs:1077-1088`). That returns an index in `0..emb_dim`, not a true vocab token distribution. This can mask serious decode-path failures and generate misleading output.

Recommendation:

- Treat output projection failure as a hard inference error in normal mode.
- Keep the pseudo fallback only behind an explicit test or diagnostic flag.

### P2 - Memory budget accounting is not connected to actual resident memory

`LLM_MEMORY_BUDGET_BYTES` is 128 MB (`llm_agent.rs:27`), but current models map 270-386 MB plus an 80 MB KV cache. The guard checks `LocalLlmAgent.memory_used_bytes` (`llm_agent.rs:1802-1808`), but that field appears initialized to 0 and not updated in the core inference path.

Recommendation:

- Reconcile the budget with actual mapped bytes, KV bytes, and GPU buffers.
- Use the resident model report rather than an agent-local counter that stays zero.
- Avoid calling a 386 MB mmap compliant with a 128 MB budget unless the budget explicitly excludes mmapped model bytes.

## Recommended order of attack

1. Enforce cancellation, deadline, and a smaller default max-new-token cap inside the decode loop. This is the fastest way to stop perceived hangs.
2. Flip GPU top-k on by default for non-sieve decode and record whether it actually ran.
3. Add fallback counters and include them in `llm_a0_baseline.json`: GPU waits, forward ms/token, output ms/token, attention ms/token, FFN ms/token, top-k hit count, argmax fallback count, CPU fallback count.
4. Make the resident model slot own or address a persistent execution worker, not just an mmap.
5. Port native resident layer weights and norms. Stop re-uploading the same Q/K/V/O/gate/up/down weights per token.
6. Fuse native FFN first, then full layer, then full token. The goal is one readback per generated token, ideally only top-k candidates.
7. Decide whether DirectML is strategic. If yes, build cached operators/bindings and resident buffers. If not, report it as device availability only and optimize the wgpu path.
8. After throughput stabilizes, revisit ternary FFN. The 5 tok/s result is promising, but current output quality is explicitly degenerate in the benchmark log.

## Suggested near-term benchmark matrix

Run the same prompt and decode count for each row, in release mode:

| Case | Env/settings | Purpose |
|---|---|---|
| Baseline current | default | Guardrail for regressions |
| Top-k enabled | `QUALIA_LLM_GPU_TOPK=1` | Measure output projection improvement |
| Decode profile | `QUALIA_LLM_PROFILE_DECODE=1` | Attribute forward/output/waits |
| CPU attention | `QUALIA_LLM_CPU_ATTENTION=1` | Correctness comparison, expected slower |
| Sieve on/off | same prompt, same budget | Measure safety-mode cost |
| Persistent worker prototype | no env | Measure warm TTFT/load slice removal |

Key success metrics:

- Warm TTFT under 500 ms for short prompts.
- Decode above 10 tok/s before deeper quantization work.
- GPU wait count reduced materially from the current per-sub-op pattern.
- Top-k path used by default, with full argmax fallback near zero for normal free-text decode.

## Bottom line

The native library is not slow because SmolLM2-360M is inherently too big. It is slow because the implementation still behaves like a correctness-first GPU debugger: submit a small op, read it back, do CPU work, submit another small op, repeat. The next performance win is not another week of isolated kernels; it is reducing synchronization boundaries, keeping execution state resident, and enforcing practical runtime limits so the app never feels stuck while that work lands.
