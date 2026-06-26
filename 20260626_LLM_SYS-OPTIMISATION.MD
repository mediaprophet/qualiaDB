# QualiaDB Native LLM / wgpu System Optimisation Plan

Date: 2026-06-26
Branch reviewed: `codex/llm-native-flight-60toc`
Scope: native LLM inference path, wgpu 29, GPU residency, pipeline modularisation, and iGPU/dGPU policy.

## Direct Answers

### Are wgpu groups and advanced features being used in an optimised manner?

Partially, but not yet.

The native path is using workgroups in a few sensible places:

- `fused_transformer.wgsl::coop_gemv` uses a cooperative workgroup reduction with `@workgroup_size(256)`, one workgroup per output row.
- `fused_attention.wgsl` uses `@workgroup_size(64)` plus workgroup-local arrays for attention max/sum/accumulator reductions.
- `topk_reduction.wgsl` uses block-local reduction, which is the right general shape for GPU-side top-k.

However, the current native path is not yet taking full advantage of the newer wgpu/native capability surface:

- The negotiated optional feature set is minimal. It currently appears to request `TIMESTAMP_QUERY` when available, but not shader f16, subgroup operations, push constants/immediate constants, native pipeline cache, or related native-only features where the adapter supports them.
- Most native pipelines use auto pipeline layouts (`layout: None`) rather than explicit bind group layouts with stable contracts, min binding sizes, and dynamic-offset uniform arenas.
- Workgroup sizes are mostly fixed at 64 or 256 rather than autotuned per adapter, model shape, quant type, and kernel.
- There is no subgroup-accelerated path for reductions, broadcast, softmax, or GEMV accumulation.
- Native still has too many CPU/GPU synchronization points and queue waits per token.
- The wasm MC8 path has a more advanced "single encoder / dynamic offset / super-arena" structure than the native path in several places. Native should inherit that architecture, then go further with native-only features.

Short version: the kernels are not naive, but the overall native execution graph is still not optimized enough for a 60+ TOC/s target.

### Are the libraries optimally modularised?

No. The codebase is partially modularised, but not optimally for high-performance native LLM work.

The `gguf_bridge` module is split into useful files such as `attention.rs`, `ffn.rs`, `output.rs`, `gemm.rs`, `forward.rs`, `init.rs`, and `quant_support.rs`. That is a good start. But `QTensorEngine` still acts as a large stateful owner of device state, pipelines, buffers, model layout, benchmark switches, and execution policy. Native and wasm paths are also interleaved with many `#[cfg]` branches, which makes optimization work harder to reason about.

The target structure should separate:

- Device and adapter capability discovery.
- Pipeline and shader registry.
- Bind group and layout contracts.
- Resident model weight store.
- GPU buffer arena and staging rings.
- Decode scheduler.
- Kernel dispatch implementations.
- Profiler and benchmark harness.
- Native and wasm execution backends.

## Current Native Configuration Observed

### wgpu

- Cargo dependency: `wgpu = 29.0.3`.
- Cargo feature: `webgpu`.
- Shader frontend: `naga = 29.0.3`, `wgsl-in`.
- Native shared GPU setup requests a high-performance adapter.
- Required features currently appear to be limited to `TIMESTAMP_QUERY` if supported.
- Required limits raise `max_buffer_size` and `max_storage_buffer_binding_size` to adapter limits, but otherwise stay close to defaults.
- The queue-lane abstraction currently maps logical lanes to one physical wgpu queue.

### Current Native Performance Context

Recent local branch measurements after output-side improvements showed approximately:

- Around 13 TOC/s.
- Around 76.9 ms/token.
- Attention around 52.2 ms/token.
- FFN around 21.7 ms/token.
- Output around 2.2 ms/token.
- Wait count around 107 waits/token.

These numbers are branch-local measurements, not a final baseline. They are still useful because they show that output projection is no longer the dominant bottleneck. Attention, FFN, and synchronization now dominate.

## iGPU / dGPU Policy

The attached hardware notes identify the Intel HD Graphics 530 as a Skylake Gen9 iGPU with roughly 24 EUs and old integrated graphics capabilities. It can be useful for:

- QuickSync media decode/encode.
- Display composition.
- Light UMA preprocessing if explicitly measured and proven useful.

It should not be treated as a primary native LLM compute target:

- It lacks modern AI matrix engines.
- It lacks XMX-style acceleration.
- It is old enough that CPU AVX2 may beat it for branchy low-latency work.
- Multi-adapter tensor sharing through wgpu is likely to add synchronization and copy overhead that overwhelms any benefit.

Policy recommendation:

- Use the NVIDIA dGPU for heavy LLM compute.
- Do not offload attention, FFN, or logits to the HD 530.
- Consider the HD 530 only for media pipelines or explicitly isolated, low-priority preprocessing tasks.
- Add adapter pinning so the LLM path cannot silently land on the iGPU.

## High-Level Native Optimisation Strategy

The target is not just "faster kernels." The target is a better native decode graph:

1. Keep weights resident.
2. Keep hidden state resident.
3. Keep KV cache resident.
4. Submit fewer command buffers.
5. Read back only the final token id unless logits are explicitly needed.
6. Use explicit bind group layouts and stable pipeline contracts.
7. Gate native-only features by adapter capability.
8. Autotune workgroup sizes and kernel variants.
9. Preserve coherence with strict correctness tests after every optimization.

## Comprehensive To-Do List

### P0 - Baseline, Evidence, and Safety Rails

- [ ] Add a native LLM perf profile command that emits JSON and CSV with:
  - Adapter name.
  - Vendor ID.
  - Device ID.
  - Backend.
  - Driver info if exposed.
  - wgpu version.
  - Full adapter feature bitset.
  - Full adapter limits.
  - Model path.
  - Quant type.
  - Context length.
  - Prompt token count.
  - Generated token count.
  - ms/token.
  - TOC/s.
  - Queue submit count.
  - `poll_wait()` count.
  - CPU readback count.
  - Per-phase timings.

- [ ] Standardise a reproducible benchmark matrix:
  - Fixed tiny coherence prompt.
  - Fixed short generation prompt.
  - Fixed long-context decode prompt.
  - Fixed prefill prompt.
  - Cold run and warm run.
  - Resident weights on/off.
  - Native fused FFN on/off.
  - Native fused attention/O path on/off.
  - Output top1 path on/off.
  - Preproject attention on/off.

- [ ] Add correctness gates that run after every speed change:
  - Token identity against a known deterministic baseline where possible.
  - Coherence smoke test.
  - Max absolute error tests for attention.
  - Max absolute error tests for FFN.
  - Max absolute error tests for logits/output projection.
  - No NaN/Inf assertion in GPU output buffers.

- [ ] Add a "perf regression budget" to CI or local bench scripts:
  - Fail if TOC/s drops by more than a configured threshold.
  - Fail if wait count rises unexpectedly.
  - Fail if output projection regresses after recent gains.

- [ ] Save benchmark environment metadata with every result:
  - Git branch.
  - Git commit if available.
  - Dirty worktree flag.
  - Date/time.
  - OS.
  - GPU driver version if available.
  - Active power profile if available.

### P0 - Adapter Selection and Capability Policy

- [ ] Extend `SharedGpuContext` with a durable `GpuAdapterCaps` struct:
  - Adapter name.
  - Backend.
  - Vendor ID.
  - Device ID.
  - Device type.
  - Dedicated vs integrated hint.
  - Feature bitset.
  - Limits.
  - Timestamp support.
  - Timestamp period if available.
  - Subgroup support if available.
  - f16 shader support if available.

- [ ] Add explicit adapter selection controls:
  - `QUALIA_WGPU_ADAPTER`.
  - `QUALIA_WGPU_BACKEND`.
  - `QUALIA_LLM_FORCE_DGPU=1`.
  - `QUALIA_LLM_ALLOW_IGPU=0/1`.

- [ ] Add fail-loud logging when the LLM path selects an integrated GPU.

- [ ] Add a hardware policy table:
  - NVIDIA dGPU: primary LLM compute.
  - Intel HD 530: media/display/light preprocessing only.
  - CPU AVX2: fallback for tiny branchy work and emergency path.

- [ ] Add startup telemetry that prints:
  - Selected LLM adapter.
  - Rejected adapters and reason.
  - Enabled optional GPU features.
  - Disabled optional GPU features.

### P0 - Native Execution Graph

- [ ] Port the wasm MC8 "single encoder / super-arena" architecture to native.

- [ ] Create a native per-token command encoder that can record:
  - Embedding lookup.
  - Layer attention.
  - Attention output projection.
  - Residual add.
  - RMSNorm.
  - FFN up/gate/down.
  - Final norm.
  - Output top1.

- [ ] Reduce native submissions per token:
  - Target P0: fewer than 16 submits/token.
  - Target P1: fewer than 4 submits/token.
  - Target P2: 1 submit/token for steady-state decode where practical.

- [ ] Reduce CPU readbacks:
  - Target P0: no readback inside layer loop.
  - Target P1: one readback per token for token id only.
  - Target P2: optional no-readback speculative loop for short bursts.

- [ ] Keep hidden state GPU-resident through the entire layer stack.

- [ ] Keep post-attention and post-FFN residual results GPU-resident.

- [ ] Move any remaining CPU residual add in the native sync path onto GPU.

- [ ] Move any remaining CPU RMSNorm in the native sync path onto GPU.

- [ ] Replace layer-by-layer host orchestration with a decode scheduler that records the whole token graph.

- [ ] Use WebGPU pass ordering as the synchronization primitive instead of host waits.

### P0 - Bind Groups, Pipeline Layouts, and Uniforms

- [ ] Replace native `layout: None` pipeline creation with explicit pipeline layouts.

- [ ] Define one binding contract per shader family:
  - Attention.
  - Attention output projection.
  - FFN.
  - RMSNorm.
  - Embedding.
  - Output projection.
  - Top-k/top1.
  - Sieve/constrained decoding.

- [ ] Add `min_binding_size` for storage and uniform buffers where possible.

- [ ] Introduce a native dynamic uniform arena:
  - One buffer per token or frame.
  - Dynamic offsets per layer/pass.
  - No small uniform allocation per dispatch.
  - No avoidable `queue.write_buffer` per dispatch.

- [ ] Cache bind groups for resident weights:
  - Per layer.
  - Per tensor role.
  - Per pipeline family.
  - Invalidated only when model residency changes.

- [ ] Cache bind groups for stable buffers:
  - KV cache.
  - Hidden state buffers.
  - RoPE tables.
  - Output/logit buffers.
  - Sieve buffers.

- [ ] Move transient bind group creation out of the inner decode loop.

- [ ] Batch uniform writes:
  - One write per token for all layer params where possible.
  - One write per changed runtime scalar group.

- [ ] Add a bind group allocation counter to the profiler.

### P0 - Attention Pipeline

- [ ] Precompute RoPE tables:
  - Inverse frequency.
  - Sin table.
  - Cos table.
  - Per model/head layout.

- [ ] Remove repeated shader-side `pow`, `sin`, and `cos` from the hot decode path.

- [ ] Split attention shaders by use case:
  - Decode, single token.
  - Prefill, many tokens.
  - Long-context decode.
  - Small context fast path.

- [ ] Split attention shaders by model shape where useful:
  - Head dimension 64.
  - Head dimension 80.
  - Head dimension 96.
  - Head dimension 128.

- [ ] Add subgroup attention variant where supported:
  - Subgroup max reduction.
  - Subgroup sum reduction.
  - Subgroup accumulator reduction.
  - Fallback to current workgroup path.

- [ ] Autotune attention workgroup size:
  - 32.
  - 64.
  - 128.
  - 256 if register pressure allows.

- [ ] Record attention occupancy proxies:
  - Workgroups dispatched.
  - Context length.
  - Head count.
  - KV head count.
  - Approximate bytes read.
  - Approximate FLOPs.

- [ ] Re-evaluate native preproject Q/K/V only if it stays GPU-resident.

- [ ] Do not reintroduce a preproject path that reads Q/K/V back to CPU.

- [ ] Fuse attention output projection with residual write when numerically safe.

- [ ] Add a dedicated MQA/GQA path:
  - Avoid repeated KV reads across query heads.
  - Share loaded K/V data where possible.

- [ ] Add causal mask fast path for decode:
  - Single new token does not need full prefill mask logic.

### P0 - FFN / GEMV Pipeline

- [ ] Specialise kernels by quant type:
  - Q4 variants.
  - Q5 variants.
  - Q8 variants.
  - F16/BF16 if present.
  - F32 fallback.

- [ ] Remove runtime `ggml_type` branching from hot shader loops where practical.

- [ ] Specialise by common model dimensions:
  - Hidden dimension.
  - Intermediate dimension.
  - Block size.
  - Quant group size.

- [ ] Add f16 activation path where adapter supports shader f16:
  - Hidden state f16.
  - Intermediate activation f16.
  - Accumulate in f32 where needed.

- [ ] Add packed activation path for memory bandwidth reduction.

- [ ] Tune cooperative GEMV:
  - Workgroup size 128 vs 256 vs 512 where supported.
  - Rows per workgroup.
  - Elements per lane.
  - Vectorized loads.

- [ ] Evaluate multi-row tiled GEMV:
  - More reuse of input hidden vector.
  - Less repeated hidden reads.

- [ ] Fuse FFN up/gate/down more aggressively:
  - Keep intermediate on GPU.
  - Avoid writing large intermediate buffers unless needed.
  - Use tiled partial accumulation for down projection.

- [ ] Move residual add after FFN fully onto GPU.

- [ ] Add a per-layer FFN timing counter separate from attention.

### P0 - Output Projection and Sampling

- [ ] Preserve the recent output projection improvements as a regression-protected path.

- [ ] Replace chunked host-mediated output top1 with full GPU global top1 reduction.

- [ ] Read back only:
  - Token id.
  - Optional score.
  - Optional small diagnostic summary.

- [ ] Keep full logits on GPU unless explicitly requested.

- [ ] Add GPU-side repetition penalty support.

- [ ] Add GPU-side temperature scaling.

- [ ] Add GPU-side top-k.

- [ ] Add GPU-side top-p if required.

- [ ] Integrate constrained decoding/sieve mask before top1/top-k:
  - Mask invalid token ids in GPU memory.
  - Avoid CPU-side logits scan.
  - Maintain deterministic fallback path.

- [ ] Add one correctness test for masked output projection:
  - Forbidden token has high raw logit.
  - Masked sampler chooses the next legal token.

### P1 - Advanced wgpu Native Features

- [ ] Add optional shader f16 feature negotiation:
  - Enable only if adapter supports it.
  - Keep f32 fallback.
  - Compare numerical drift.

- [ ] Add optional subgroup feature negotiation:
  - Enable only if adapter supports it.
  - Keep workgroup fallback.
  - Add shader variants behind capability flags.

- [ ] Add push constants/immediate constants where supported:
  - Layer index.
  - Head counts.
  - Context length.
  - Small scalar params.
  - Avoid tiny uniform writes.

- [ ] Investigate native pipeline cache support in wgpu 29:
  - Cache compute pipelines.
  - Cache shader modules.
  - Avoid repeated pipeline creation.
  - Persist cache only if API/backend makes it safe.

- [ ] Investigate timestamp query inside passes if available:
  - More accurate per-pass GPU timing.
  - Keep fallback to CPU-side timing.

- [ ] Investigate adapter-specific limits:
  - Max compute workgroup storage size.
  - Max invocations per workgroup.
  - Max bindings per bind group.
  - Max dynamic uniform/storage offsets.

- [ ] Create feature-gated shader modules:
  - Portable WGSL.
  - f16 WGSL.
  - subgroup WGSL.
  - native-fast WGSL.

### P1 - Memory Residency and Buffer Arena

- [ ] Build a native `GpuBufferArena`:
  - Long-lived resident buffers.
  - Per-token transient buffers.
  - Staging upload ring.
  - Readback ring.
  - Alignment helpers.

- [ ] Build a `ResidentWeightStore`:
  - Stable handles for every tensor.
  - Layer-indexed arrays instead of hot-path hash lookups.
  - Tensor role enum.
  - Byte offsets and quant metadata.

- [ ] Eliminate hot-path allocations:
  - No `Vec` growth in decode loop.
  - No transient String formatting in decode loop.
  - No transient bind group vectors in decode loop.

- [ ] Add a VRAM ledger:
  - Resident model weights.
  - KV cache.
  - Scratch arena.
  - Staging/readback.
  - Fragmentation estimate.

- [ ] Add eviction policy:
  - Keep active model resident.
  - Evict inactive model weights first.
  - Evict nonessential scratch buffers next.
  - Never evict current token buffers mid-decode.

- [ ] Add KV cache layout review:
  - Coalesced read pattern for decode.
  - Separate K and V or interleaved layout benchmark.
  - Head-major vs token-major benchmark.
  - GQA/MQA optimized layout.

### P1 - Native Scheduling and Async

- [ ] Replace sync wait-heavy decode path with an async scheduler:
  - Submit token work.
  - Prepare next CPU-side metadata while GPU runs.
  - Read back token id only when required.

- [ ] Add a small command encoder pool or builder abstraction.

- [ ] Add a staging readback ring:
  - Avoid blocking on the same buffer every token.
  - Allow N-token delayed diagnostics when text streaming permits.

- [ ] Add backpressure policy:
  - UI streaming cadence.
  - Thermal governor.
  - Queue depth.
  - Memory pressure.

- [ ] Keep deterministic mode:
  - Blocking one-token-at-a-time path for tests.
  - Async fast path for production.

### P1 - Native/Wasm Split

- [ ] Define a `DecodeBackend` trait:
  - `prepare_model`.
  - `prepare_prompt`.
  - `decode_one`.
  - `decode_many`.
  - `collect_token`.
  - `shutdown`.

- [ ] Implement `NativeWgpuDecodeBackend`.

- [ ] Implement `WasmWgpuDecodeBackend`.

- [ ] Move MC8 wasm-specific code behind the wasm backend.

- [ ] Move native super-encoder code behind the native backend.

- [ ] Keep shared shader contracts in one registry.

- [ ] Keep shared quant math in a backend-neutral module.

- [ ] Keep benchmark toggles in a runtime config struct rather than scattered environment lookups.

### P1 - Shader Registry and Contracts

- [ ] Add a central shader registry:
  - Shader name.
  - Entry point.
  - Required features.
  - Required limits.
  - Bind group layout contract.
  - Workgroup defaults.
  - Tunable workgroup range.

- [ ] Add generated or checked binding constants:
  - Avoid binding index drift between Rust and WGSL.
  - Add tests that inspect expected binding layout metadata where possible.

- [ ] Document every compute shader:
  - Inputs.
  - Outputs.
  - Workgroup layout.
  - Memory layout.
  - Numerical precision.
  - Feature gates.
  - Fallback path.

- [ ] Add shader-specific tests:
  - Tiny hand-verifiable input.
  - Random small input.
  - Edge dimensions.
  - Quant edge cases.

### P1 - Autotuning

- [ ] Add an autotune cache keyed by:
  - Adapter vendor/device.
  - Backend.
  - Driver version if available.
  - Model architecture.
  - Quant type.
  - Context length bucket.

- [ ] Autotune attention workgroup sizes.

- [ ] Autotune GEMV workgroup sizes.

- [ ] Autotune output top1 block sizes.

- [ ] Autotune KV cache layout if multiple layouts are available.

- [ ] Save winning parameters to a local config file.

- [ ] Provide a safe default if autotune has not run.

- [ ] Provide `QUALIA_LLM_DISABLE_AUTOTUNE=1`.

### P1 - DirectML / Vendor Backend Evaluation

- [ ] Audit whether DirectML is currently only used for adapter memory telemetry or also for compute.

- [ ] Build a tiny DirectML benchmark for:
  - GEMV.
  - Matmul.
  - RMSNorm.
  - Softmax.
  - Attention-like batched matmul.

- [ ] Compare DirectML against current wgpu WGSL on the target machine.

- [ ] If DirectML wins materially:
  - Consider DirectML backend for native Windows.
  - Keep wgpu as portable fallback.
  - Keep the same `DecodeBackend` interface.

- [ ] Consider llama.cpp/ggml interop only as an explicit backend option:
  - Useful if 60+ TOC/s is required sooner than the WGSL stack can reach it.
  - Must preserve QualiaDB coherence and safety checks around the backend.

### P2 - iGPU and Media-Specific Work

- [ ] Keep HD 530 out of LLM compute by default.

- [ ] Add optional QuickSync media path:
  - Video decode.
  - Video encode.
  - Transcription preprocessing if future multimodal path needs it.

- [ ] Add explicit measurement before any iGPU preprocessing is enabled:
  - CPU baseline.
  - dGPU baseline.
  - iGPU path.
  - Copy overhead.
  - End-to-end latency.

- [ ] Avoid multi-adapter tensor ping-pong:
  - No layer split across iGPU/dGPU.
  - No logits split across iGPU/dGPU.
  - No KV cache split across iGPU/dGPU.

### P2 - Thermal and Power Governance

- [ ] Wire the real thermal governor into native decode.

- [ ] Add performance modes:
  - Quiet.
  - Balanced.
  - Fast.
  - Benchmark.

- [ ] Adjust scheduling based on:
  - GPU temperature.
  - CPU temperature.
  - Laptop/desktop power state.
  - VRAM pressure.
  - UI responsiveness.

- [ ] Surface current mode in telemetry.

- [ ] Ensure benchmark mode pins the fastest safe configuration.

### P2 - Packaging and Runtime Controls

- [ ] Add a single LLM runtime config object:
  - Backend.
  - Adapter preference.
  - Feature flags.
  - Autotune settings.
  - Memory budget.
  - Perf mode.
  - Correctness mode.

- [ ] Replace scattered environment variables with config-backed settings.

- [ ] Keep environment variables as override/debug hooks.

- [ ] Add a diagnostics export command:
  - Adapter caps.
  - Current config.
  - Last benchmark.
  - Last model load.
  - Last error.

## Suggested Module Layout

Target native LLM module split:

```text
crates/qualia-core-db/src/gguf_bridge/
  backend/
    mod.rs
    native_wgpu.rs
    wasm_wgpu.rs
    cpu_fallback.rs
  device/
    caps.rs
    context.rs
    adapter_select.rs
  pipelines/
    registry.rs
    attention.rs
    ffn.rs
    output.rs
    norm.rs
    embedding.rs
    topk.rs
  shaders/
    contracts.rs
    bindings.rs
  memory/
    arena.rs
    resident_weights.rs
    kv_cache.rs
    staging.rs
  scheduler/
    decode.rs
    prefill.rs
    command_graph.rs
  profiling/
    counters.rs
    trace.rs
    reports.rs
  correctness/
    oracle.rs
    tolerances.rs
```

This does not need to land as one giant refactor. The safest path is to introduce these boundaries around new work, then migrate old code module by module.

## Immediate Next Tickets

### Ticket 1 - Native Adapter Capability Dump

Goal: make the runtime prove which GPU and features it is actually using.

Tasks:

- Add `GpuAdapterCaps`.
- Emit caps at LLM startup.
- Add caps to benchmark JSON.
- Add adapter override env vars.
- Add warning when selected adapter is integrated graphics.

Acceptance:

- Benchmark output shows adapter and enabled features.
- User can force the dGPU.
- No LLM run silently uses HD 530 unless explicitly allowed.

### Ticket 2 - Native Bind Group and Uniform Arena

Goal: remove avoidable per-dispatch bind group and uniform churn.

Tasks:

- Add explicit bind group layouts for native attention, FFN, output, and top1.
- Add dynamic uniform arena for per-layer params.
- Cache resident weight bind groups.
- Add counters for bind group creation and uniform writes.

Acceptance:

- Fewer bind groups allocated during steady-state decode.
- Fewer `queue.write_buffer` calls per token.
- No coherence regression.

### Ticket 3 - Native Single-Encoder Decode Prototype

Goal: remove layer-loop CPU/GPU waits.

Tasks:

- Record attention, FFN, norm, residual, and output top1 into one native command encoder for one token.
- Keep hidden state GPU-resident.
- Read back token id once.
- Keep old path as correctness fallback.

Acceptance:

- Wait count/token drops materially.
- Attention and FFN still produce acceptable numerical error.
- Generated text smoke test remains coherent.

### Ticket 4 - RoPE Table Precompute

Goal: stop paying trig/pow cost inside attention decode.

Tasks:

- Allocate resident RoPE sin/cos tables.
- Bind RoPE table to attention shader.
- Add fallback shader path for old behavior.
- Validate numerical equivalence within tolerance.

Acceptance:

- Attention ms/token improves or stays neutral.
- No visible quality regression.

### Ticket 5 - GPU Global Top1 and Sieve Integration

Goal: keep logits on GPU and return only the selected token.

Tasks:

- Add global top1 reduction.
- Add optional GPU-side mask/sieve buffer.
- Read back token id and score only.
- Add masked-token correctness test.

Acceptance:

- Output remains near current improved timing or faster.
- Invalid tokens cannot be selected when sieve is active.

## 60+ TOC/s Reality Check

The 60+ TOC/s target is plausible only if most of the following become true:

- Native decode has very few queue submissions per token.
- Hidden state never comes back to CPU inside the layer stack.
- Output projection reads back only token id.
- Attention avoids shader-side trig in the hot path.
- FFN is quant-specialized and avoids large intermediate round trips.
- Workgroup sizes are tuned for the actual dGPU.
- Optional native features such as f16 and subgroups are used where they win.
- The model/quant size is compatible with the target hardware.

If wgpu WGSL cannot reach the target on the available hardware, the project should evaluate a backend split:

- wgpu native portable backend.
- DirectML Windows backend.
- llama.cpp/ggml-compatible backend.
- CPU fallback backend.

The scheduler and coherence/sentinel layers should sit above these backends so QualiaDB semantics remain stable.

## Risk Register

- Subgroup and f16 support differs sharply across adapters and backends.
- HD 530 feature limits may accidentally constrain the selected adapter if adapter selection is wrong.
- Reducing waits can hide synchronization bugs.
- f16 can introduce numerical drift.
- Shader specialization can multiply maintenance cost.
- Pipeline layout changes can break wasm if contracts are not separated cleanly.
- Resident buffers can increase VRAM pressure.
- Autotune can overfit one driver version.
- DirectML or vendor backend integration can complicate reproducibility.

## Definition of Done for Native Optimisation Sprint

- LLM adapter is explicitly identified and controlled.
- Integrated GPU is not used for LLM compute unless explicitly allowed.
- Benchmark JSON captures timing, waits, features, and adapter.
- Native steady-state decode has no per-layer CPU readback.
- Output path reads back token id only by default.
- Attention uses precomputed RoPE tables.
- Bind groups and uniform buffers are cached/arena-backed.
- Correctness tests pass after every optimization.
- Coherence smoke test passes.
- Native path reaches the best measured TOC/s target for the selected model/hardware, with 60+ TOC/s as the stretch target.

