# Qualia WASM, WebGPU, Mobile LLM, and Anatomy Review

**Date:** 2026-08-02

**Branch reviewed:** `0.0.29-dev`; published for feedback on `0.0.29-moredev`

**Status:** Review and recommendations; no implementation authorised by this document
**Primary physical device:** Pixel 10 Pro XL, Android 17, Chrome 150, PowerVR D-Series DXT-48-1536

## Purpose

This report records the current failure modes and performance regressions affecting Qualia's
browser-WASM LLM and Anatomy experience, particularly on mobile. It is intended for technical
review and feedback before the next repair programme is implemented.

The proposed direction preserves these constraints:

- Qualia remains a first-party Rust/WASM inference system. `wllama` may be used as an external
  benchmark reference, but must not be introduced as a Qualia runtime dependency.
- Browser-WASM, native GPU, and native accelerator paths remain separate implementations behind
  shared contracts.
- LLM model weights, KV cache, and inference working memory are not constrained by the 42 MiB
  semantic/SLG Sentinel arena. The Sentinel remains applicable to the semantic execution passes it
  was designed to bound.
- Hot execution must remain allocation-stable and caller-buffered. Cold model construction may
  use bounded allocations outside the Sentinel arena.
- New implementation should be directory-backed and split by lifecycle and responsibility rather
  than added to existing monolithic files.

## Executive summary

The Pixel is not missing suitable graphics hardware. Chrome detects both Vulkan and OpenGL ES
PowerVR adapters internally, but disables WebGPU at the browser feature gate. Consequently,
`navigator.gpu` exists while every `requestAdapter()` attempt returns `null`.

This exposes two different Qualia shortcomings:

1. The LLM now selects an uncommitted CPU-WASM fallback. It produces correct output, but its
   single-threaded, serial implementation is much slower than the WebGPU path. The physical-phone
   trace measured a 37.5 second time to first token and 42.3 seconds for seven generated tokens.
2. Anatomy has a nominal Canvas2D fallback, but decoded anatomy meshes are uploaded only when a
   WebGPU device exists. Without a GPU, the loader discards the decoded mesh and still reports
   success. The current LAN workspace also lacks the `.hmc` filenames requested by the page.

Separately, the working WebGPU LLM remains materially below the comparison implementation. Recent
exact-token measurements place Qualia at 3.71–4.89 tokens/s and wllama at 9.31–9.36 tokens/s on the
repair machine. The largest visible decode bottleneck is the full-vocabulary GPU-to-CPU readback
and CPU argmax performed for every generated token.

## Evidence reviewed

### Physical-phone GPU report

The supplied Chrome GPU report records:

- Chrome 150.0.7871.186 on Android 17.
- Pixel 10 Pro XL with Imagination Technologies PowerVR D-Series DXT-48-1536, driver 25.3.
- Hardware-accelerated Canvas, Vulkan, OpenGL, and WebGL.
- `WebGPU: Disabled` and `WebNN: Disabled`.
- `WebGPU has been disabled via blocklist or the command line`.
- The applicable `Disable webgpu on vk via gl interop` rule.
- GPU process crash count `0`.
- Dawn reports both the OpenGL ES compatibility adapter and Vulkan PowerVR adapter as
  `Available` internally.

Local evidence: `C:\Users\Admin\Downloads\about-gpu-2026-08-02T04-05-28-654Z.txt.phps`.

### Physical-phone Qualia telemetry

The HTTPS LAN trace records:

- secure context and cross-origin isolation were both active;
- `navigator.gpu` was present;
- the adapter was `null` after compatibility, core, low-power, and software requests;
- Qualia selected `cpu-wasm`;
- SmolLM2-360M Q8_0 loaded successfully with a 49,152-token vocabulary;
- model load completed in 1.51 seconds;
- time to first token was 37.524 seconds;
- seven correct tokens completed in 42.331 seconds.

Local evidence: `.qualia/mobile-wasm-lab/secure-phone-20260802-123000/events.jsonl`, particularly
events 162–176.

### Browser and Chromium sources

- Chrome initially enabled Android WebGPU for Android 12+ devices using Qualcomm and ARM GPUs and
  described support for other configurations as a gradual rollout:
  <https://developer.chrome.com/blog/new-in-webgpu-121?hl=en>
- Chrome documents a `null` adapter together with `WebGPU has been disabled via blocklist` as a
  browser/GPU blocklist outcome:
  <https://developer.chrome.com/docs/web-platform/webgpu/troubleshooting-tips?hl=en>
- Chromium added intended Imagination Technologies support on Android 16+ in May 2025:
  <https://chromium.googlesource.com/chromium/src/%2B/472e65391d0736097e9aa13f0cce7849876abe2a%5E%21/>
- Chromium's current software-rendering list contains the Vulkan/GL interop WebGPU block and a
  PowerVR Graphite block for rendering glitches:
  <https://chromium.googlesource.com/chromium/src/gpu/%2B/refs/heads/main/config/software_rendering_list.json>
- The WebGPU API states that `powerPreference` is a selection hint and must not influence whether
  an adapter is returned:
  <https://gpuweb.github.io/types/interfaces/GPURequestAdapterOptions.html>
- Chrome 146 still documented WebNN as an origin trial. The physical phone also reports WebNN as
  disabled, so the NPU is not presently a dependable browser backend:
  <https://developer.chrome.com/release-notes/146?hl=en>

## Findings

### F1 — The Pixel WebGPU failure is a Chrome/driver gate, not absent hardware

**Severity:** External blocker with high product impact

**Confidence:** High

The PowerVR GPU and its Vulkan/OpenGL ES implementations are visible to Chrome's Dawn layer, but
Chrome suppresses WebGPU before page code can obtain an adapter. Qualia cannot bypass this from a
webpage, and changing Qualia's `powerPreference` cannot make a blocklisted adapter available.

The Android compatibility-first shim is sensible adapter ordering, but it cannot override the
browser feature gate. Requiring people to change Chrome flags is not an acceptable product
solution.

### F2 — Anatomy falsely succeeds without rendering a body

**Severity:** P0 correctness defect

**Confidence:** High

`docs/playground/anatomy.js` checks `navigator.gpu`, which is insufficient because the property can
exist while `requestAdapter()` returns `null`.

The portal subsequently constructs its Canvas2D fallback, but
`render/portal/mod.rs::finish_body_mesh_upload` performs the body upload only when `self.gpu` is
present. When it is absent, the decoded `BodyMeshAccum` is dropped and the method still returns an
object reporting organ and triangle counts. The UI can therefore say the body loaded while showing
no body.

The fallback paints background, ambient field, tensor projection, and HUD; it does not retain or
rasterise the decoded anatomy mesh.

### F3 — The local Anatomy assets do not match the requested names

**Severity:** P0 for LAN testing; deployment risk

**Confidence:** High

The page requests:

- `docs/playground/anatomy-male.hmc`
- `docs/playground/anatomy-female.hmc`

Those paths are absent in the workspace. The available files are:

- `anatomy-male.qualia` — 97,088,460 bytes
- `anatomy-female.qualia` — 122,573,659 bytes

Both available files begin with the current `QBDL` bundle magic used by `.hmc`. The LAN harness
serves `docs/` but does not execute the release-asset fetch script, whereas the Pages workflow does.
Consequently, local phone testing reaches a missing asset after the renderer issue is repaired.

### F4 — CPU-WASM is correct but not yet a performant mobile backend

**Severity:** P0 performance limitation for blocklisted browsers

**Confidence:** High

The new CPU-WASM implementation is first-party and modular under `gguf_bridge/wasm_cpu/`. It is a
useful correctness floor and correctly keeps inference memory outside the 42 MiB Sentinel.

Its present execution plan is nevertheless serial:

- the engine is stored in a browser-thread `thread_local`;
- prompt prefill evaluates one token at a time;
- each token evaluates every transformer layer serially;
- the generic quantized GEMV dequantizes each complete weight row into scratch and then computes a
  dot product;
- the full vocabulary projection and CPU argmax run every decode step;
- browser yielding happens between complete tokens, not within expensive token work;
- no Web Worker pool or WASM thread pool is used, despite cross-origin isolation being available.

The build enables `simd128`, allowing compiler vectorisation, but there is no dedicated
WASM-SIMD Q8 kernel or deterministic parallel reduction plan comparable to mature CPU runtimes.

### F5 — WebGPU decode still performs a full-vocabulary readback per token

**Severity:** P1 performance defect

**Confidence:** High

`gguf_bridge/forward.rs::dispatch_forward_and_argmax_fused_async` performs the transformer forward
and logits projection in one command encoder, but then copies all generated logits into a staging
buffer and maps them to the CPU. For the current 49,152-token vocabulary this is 196,608 bytes of
GPU-to-CPU data plus a synchronization fence for every generated token. The final argmax is then
performed on the CPU.

This path is better than the earlier two-round-trip implementation, but “fused argmax” is an
inaccurate description: the logits computation is fused into the encoder; the reduction is not on
the GPU.

An earlier attempt to insert the existing top-K reduction into the same browser command stream
caused command-stream corruption and was correctly removed. The needed repair is a separately
validated browser reduction design, not restoration of that experiment.

### F6 — Residency can silently demote performance

**Severity:** P1 performance-contract defect

**Confidence:** High

Full eager residency was restored after the deferred-residency regression. Clean logs now show
approximately 318.8 MB resident across 32 layers.

However, model loading still treats failed weight, logits, or norm residency as a warning and
continues into per-forward, per-token, or per-layer upload fallbacks. That can recreate the former
~0.5 token/s class without failing model load or clearly changing the reported backend.

A performance-capable load needs an explicit receipt stating which resources are resident. A
benchmark or “accelerated” mode should fail closed if the declared execution plan cannot be
constructed.

### F7 — Historical and current performance numbers are not directly comparable

**Severity:** Measurement-quality issue

**Confidence:** High

The historical browser lab displayed 21.84 tokens/s for Qualia and 27.67 for wllama, but its Qualia
counter used approximate whitespace pieces while the engine internally decoded a fixed number of
model tokens.

More reliable comparisons are:

| Measurement | Qualia | Reference | Notes |
|---|---:|---:|---|
| Historical Phase 5 | ~5.9 tok/s | — | SmolLM2-360M Q4_K_M, different machine/adapter |
| Rebuilt resident path | 4.6 tok/s | — | Q8_0, 128-token run, RTX A2000 repair machine |
| Fresh exact-token run | 4.89 tok/s | 9.31 tok/s | Q8_0, 64 generated model tokens |
| Stressed-process exact run | 3.71 tok/s | 9.36 tok/s | Treat as a floor, not a clean attribution |
| Pixel CPU-WASM | 37.5 s TTFT | — | Seven tokens in 42.3 s end-to-end |

The currently verified Q8_0 model is approximately 369 MB. The historical 5.9 token/s path used
Q4_K_M, while current Q4 browser artefacts are withheld because they are not semantically reliable.
Part of the bandwidth and memory regression is therefore a correctness-motivated model change, but
it does not explain the full gap to the comparison implementation.

### F8 — WASM/native separation is underway but incomplete

**Severity:** P2 maintainability and regression risk

**Confidence:** High

Positive structure already exists:

- browser-specific shaders under `src/shaders/wasm/`;
- browser GPU concerns under `gguf_bridge/mc8_wasm/`;
- browser CPU concerns under `gguf_bridge/wasm_cpu/`;
- native prepared decode under `gguf_bridge/resident_decode.rs`.

Remaining issues:

- `gguf_bridge/forward.rs` is approximately 2,059 lines and continues to own multiple native and
  browser execution lifecycles;
- `wasm_llm.rs` is approximately 631 lines and still mixes binding, lifecycle, token streaming,
  benchmark, and diagnostic concerns;
- `anatomy.js` is approximately 1,006 lines and mixes UI, download, OPFS, decoding, mixer,
  interaction, and render-loop concerns;
- root and `shaders/wasm/` copies coexist, with some deliberately divergent and some identical;
- several browser shaders are still selected from root paths, making ownership less obvious;
- `webgpu-limits-shim.js` still describes wgpu 0.19 even though the crate uses wgpu 30.

This structure increases the chance that a mobile repair changes native behaviour, or that source
and shipped WASM artefacts drift.

### F9 — Current tests do not exercise the failing product contracts

**Severity:** P1 coverage gap

**Confidence:** High

Existing browser tests verify that adapter attempts occur in the desired order and that CPU-WASM
fallback strings/exports are wired. They do not verify:

- a real `navigator.gpu` with all adapter requests returning `null`;
- Anatomy rendering or honest failure without a GPU;
- local `.hmc` asset availability;
- resident-resource receipts;
- clean physical-device prefill/decode performance budgets;
- device-loss visibility rather than swallowed render-loop errors.

## Recommended repair programme

### Workstream A — Mobile capability contract and Anatomy correctness

**Priority:** P0

1. Add one shared browser capability probe that returns a structured result:
   API present, adapter obtained, adapter type, feature level, limits, device obtained, and failure
   reason.
2. Make both LLM and Anatomy consume that result rather than testing `navigator.gpu` independently.
3. Change Anatomy mesh loading so one of these outcomes is mandatory:
   - WebGPU body rendered;
   - real fallback body rendered;
   - explicit unsupported/error result.
   Returning success after discarding the mesh must be impossible.
4. Prefer a WebGL2 anatomy fallback because the phone report confirms hardware-accelerated WebGL.
   The bundled Babylon Anatomy implementation is a useful design/reference source, but the shared
   pack reader and Qualia renderer contracts should remain authoritative.
5. Make the LAN server validate/stage the canonical `.hmc` assets before displaying a QR code.
6. Preserve mobile DPR caps and system muting, but measure peak memory through fetch, WASM transfer,
   decode, accumulator construction, and GPU upload.
7. Surface render-loop/device-loss errors in telemetry and stop swallowing them.

### Workstream B — CPU-WASM performance floor

**Priority:** P0/P1

1. Move inference off the UI thread into a dedicated Web Worker.
2. Add an optional fixed worker pool when `crossOriginIsolated` and `SharedArrayBuffer` are
   available.
3. Implement dedicated WASM-SIMD Q8/Q4 dot/GEMV kernels without per-row full-f32 materialisation.
4. Partition output rows deterministically across workers, with fixed-order reduction.
5. Add batched/parallel prompt prefill and yield at bounded time intervals rather than only between
   full tokens.
6. Keep KV cache and scratch in a separately budgeted inference workspace, outside `SlgArena`.
7. Report CPU backend details honestly: threads, SIMD availability, context allocation, model
   quantisation, TTFT, prefill rate, and decode rate.
8. Treat WebNN as an optional future backend only after stable availability and real device
   receipts. Do not claim NPU use when the browser reports WebNN disabled.

### Workstream C — WebGPU decode performance

**Priority:** P1

1. Build a browser-specific GPU top-1 reduction that consumes resident logits and emits a compact
   `{token_id, score}` result.
2. Validate it as an independent pipeline/pass before integrating it into the full token graph.
3. Read back only the compact result rather than approximately 196 KB of logits per token.
4. Add GPU/CPU oracle parity over adversarial logits: ties, NaN, infinities, negative-only values,
   partial vocabulary chunks, and non-power-of-two sizes.
5. Record command validation errors and device loss in the lab receipt.
6. Continue reducing per-layer passes and bind-group churn after the output fence is removed.
7. Do not reintroduce deferred residency.

### Workstream D — Residency and performance receipts

**Priority:** P1

Define a browser inference receipt containing at least:

- engine and artefact versions;
- adapter vendor/architecture/description and feature level;
- model hash, format, quantisation, layers, vocabulary, and context;
- resident layer-weight bytes and coverage;
- resident logits/norm/KV status;
- fallback counts and reasons;
- prompt tokens and generated model tokens;
- load, compile, upload, TTFT, prefill, and steady-decode timings;
- GPU device-loss and validation messages;
- CPU-WASM worker/SIMD configuration where applicable.

Accelerated benchmark mode should reject partial residency. Normal product mode may allow a
fallback, but must label it and estimate the expected performance class before generation begins.

### Workstream E — Library and artefact separation

**Priority:** P2, performed alongside relevant P0/P1 changes

Suggested target structure:

```text
gguf_bridge/
  browser/
    mod.rs
    capability.rs
    lifecycle.rs
    receipt.rs
    webgpu/
      mod.rs
      plan.rs
      residency.rs
      prefill.rs
      decode.rs
      output_reduction.rs
      readback.rs
    cpu/
      mod.rs
      plan.rs
      kernels_q8.rs
      kernels_q4.rs
      prefill.rs
      decode.rs
      worker_protocol.rs
  native/
    ...

render/anatomy/
  mod.rs
  pack.rs
  mixer.rs
  webgpu.rs
  webgl.rs
  receipt.rs
```

The existing modules should be migrated incrementally, preserving behaviour and tests at each
step. Shader selection should be routed through one explicit target-specific module so root/native
and browser variants cannot be confused.

## Proposed acceptance criteria

### Mobile LLM

- Model load never remains indefinitely in a “loading” state.
- Backend selection explicitly reports `webgpu`, `cpu-wasm`, or unsupported, with a reason.
- No claim of GPU or NPU use without a real adapter/device receipt.
- CPU-WASM runs outside the UI thread and the page remains interactive during prefill.
- A fixed Pixel test records TTFT, prefill rate, and decode rate with exact model tokens.
- Performance thresholds are agreed before promotion; results below threshold remain labelled
  experimental rather than silently shipping as equivalent acceleration.

### WebGPU LLM

- Clean startup proves full weight/logits/norm residency or fails accelerated mode.
- GPU top-1 matches the CPU oracle for the complete test corpus.
- Per-token readback is a compact result, not the full vocabulary.
- No uncaptured validation errors, device loss, or resident fallbacks during the benchmark.
- A clean exact-token comparison is run against the historical Qualia build and external reference
  on the same adapter, model, prompt, token count, and browser process.

### Anatomy

- With WebGPU: male and female packs render, orbit, pinch, mixer, and resize on the physical phone.
- Without WebGPU but with WebGL2: a real body renders through the fallback.
- Without either renderer: the UI returns an explicit unsupported result and never says the body
  loaded.
- LAN startup verifies both canonical `.hmc` paths and pack magic before producing the QR code.
- Success, error, device-loss, and out-of-memory paths emit bounded telemetry.
- Peak memory is measured for both packs and stays within an agreed mobile budget.

## Feedback requested

Reviewers are asked to comment on these decisions:

1. **Anatomy fallback:** Should WebGL2 be the required mobile fallback, or is an explicit
   unsupported result acceptable where WebGPU is blocklisted?
2. **CPU-WASM product status:** Should the current correct-but-slow backend remain automatically
   selected, or require an explicit “CPU mode may be slow” confirmation until worker/SIMD work is
   complete?
3. **Performance targets:** What minimum TTFT and steady decode rate should gate promotion for the
   Pixel-class CPU fallback and desktop WebGPU respectively?
4. **Model baseline:** Should the repair target Q8_0 first for correctness, or include restoration of
   a coherent Q4_K_M browser path in the same programme?
5. **Residency policy:** Should any resident-resource failure abort accelerated model load, or may
   normal product mode continue with a clearly labelled slower plan?
6. **Asset naming:** Can `.hmc` become the only canonical public Anatomy extension, with old
   `.qualia` bundles migrated during local setup?
7. **Browser scope:** Is Chrome/Android the initial supported mobile matrix, or must Safari/iOS and
   Firefox be included before the fallback design is accepted?
8. **Architecture:** Is the proposed browser/native module boundary appropriate, or should rendering
   and LLM capability discovery live in a separate shared browser-runtime crate?

## Recommended immediate decision

Approve Workstream A first, because it converts the present misleading Anatomy success into a real
render or honest failure and makes LAN testing deterministic. In parallel, specify the receipt and
benchmark contract from Workstream D. Then optimise CPU-WASM and WebGPU against those receipts.

This sequence fixes product correctness and measurement integrity before making further performance
claims.
