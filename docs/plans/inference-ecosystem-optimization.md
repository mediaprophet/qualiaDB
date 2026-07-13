# Inference Ecosystem Optimization — Master Plan

**Owner:** Claude (Fable 5). **Started:** 2026-07-05. **Status:** living plan (updated as workstreams land).
**Lane:** `crates/qualia-core-db/src/{inference,gguf_bridge,shaders,wgsl_forge-decode-surface}`, claimed in
`coordination/NOTICES.md`. **Progress log:** [`INFERENCE_OPTIMIZATION_PROGRESS_LOG.md`](../../INFERENCE_OPTIMIZATION_PROGRESS_LOG.md)
(per CLAUDE.md §9 — every step appends a dated, honest entry).
**Companion docs:** [`inference-decode-resident-fastpath.md`](inference-decode-resident-fastpath.md) (W1 detail +
the seeAlso disposition of the draft research notes), the draft notes themselves
(`QualiaDB Inference Pipeline Improvement Report.md`), `DEPENDENCY_MODERNIZATION.md`,
`docs/WGPU_UPSTREAM_TRACKING.md`.

## 0. Goal, baseline, and measurement discipline

Goal: raise **capability** (what decode can do), **functionality** (what callers can ask for), and
**tok/s** (decode + prefill throughput) of the native in-process engine — GGUF/p64 → wgpu → autoregressive
decode — without ever bypassing the Webizen gates or the measurement-honesty rules.

**Measured baseline (2026-07-05, SmolLM2-360M Q8_0, RTX A2000 12 GB, Vulkan, this machine):**

| Metric | Value |
|---|---|
| Decode | **19.89 tok/s** (50.3 ms/tok) |
| Forward (32 layers) | 47.9 ms/tok (95%) — attention 30.3 ms (63%), FFN 17.5 ms (37%) |
| Output projection | 1.9 ms/tok (GPU top-1 path) |
| GPU `submit→poll(wait)` round-trips | **107 / token** |
| Empty round-trip | 0.112 ms → **~12 ms/tok (24%) pure fence overhead** |

**Expanded framing (Timothy, 2026-07-05):** establishing hardware benchmarks on a fixed discrete
GPU (the A2000) is what makes differential testing meaningful when the same workloads later scale
to unified-memory architectures (e.g. Mac Studio). W7's thermal/power governance is motivated by
real deployment targets: off-grid, DC-to-DC + lithium-battery + Starlink-Mini-class nodes, where
efficiency-curve operation is a hard requirement, not a nicety. W10 builds on the standing mandate:
royalty-free open standards, verifiable credentials, decentralized-web interoperability.
**Step-by-step executable version of this plan (for any implementing agent):**
[`inference-ecosystem-optimization-EXECUTION.md`](inference-ecosystem-optimization-EXECUTION.md).

Rules that bind every workstream:
- **Before/after numbers from the same harness** (`llm_bench_a0`, `a0_decode_profile`), same machine,
  same model, same backend. Kernel numbers are never extrapolated to end-to-end.
- **Token-identity or PPL gates** for anything that touches numerics (differential tests vs the legacy
  path; ΔPPL ≤ 5% gate for lossy compression, as established by the AWQ/ternary work).
- **The legacy path is never deleted** while a fast path is new: every fast path is a toggle with
  automatic per-model fallback, so a regression is a flag-flip away from recovery.
- Webizen `validate_intent`/`validate_output` and the Phase-8 Sentinel bifurcation are load-bearing
  governance — optimizations must not move them off the loop.

## W1 — Resident-token decode: one fence per token  *(IN FLIGHT)*

The measured killer: the hidden state round-trips CPU↔GPU twice per layer (~107 fences/token + GPU
idle during every CPU turnaround). Fix: whole token — 32×(RMSNorm → K/V preproject+cache-write →
Q-SDPA → O-proj → residual → RMSNorm → gate/up → SiLU·mul → down → residual) + output norm + chunked
logits top-1 — encoded into **one command submit, one fence, ~400 B readback**, hidden state resident
in VRAM. Native mirror of the proven wasm MC8 fused-encoder design; same kernels as the legacy path;
GPU RMSNorm reduces in the same sequential order as the CPU reference.

Built: `gguf_bridge/resident_decode.rs` (per-model plan: pre-created bind groups, static uniform
arena, per-token dynamic attention-param arena, ping-pong hidden buffers), toggle
`QUALIA_LLM_RESIDENT_DECODE` (default ON, auto-fallback), decode-loop wiring.
Remaining: compile-fix → differential test (resident vs legacy text identity on the bench prompts,
`a1d`) → re-profile.

**Acceptance:** identical decode text vs legacy on the bench prompts; `a1a`/`a1c` still green;
waits/token ≈ 1–2 in the profile; honest before/after tok/s in the log.
**Expected (not promised):** removes ~12 ms/tok fence + a real share of the turnaround idle currently
booked as attention/FFN time. The measured number decides.

## W2 — Real sampler + generation quality  *(capability/functionality)*

Decode is pure greedy argmax; repetition collapse is documented in-tree; the sampler/penalty gap was
already called out in the 0.0.23 handover. Build the exact (not top-K-approximated) chain:

- When sampling is requested: read back **full logits** for the token (196 KB — acceptable at one
  fence/token; W1's encoder gains an optional full-logits staging copy) and run on CPU:
  **temperature → repetition/frequency/presence penalties (over the real context) → top-k → top-p →
  seeded RNG draw**. Greedy stays on the GPU top-1 fast path, bit-identical to today.
- Deterministic under a fixed seed (`ChaCha`-class PRNG, seed in the agent config; no `rand` in wasm
  paths that forbid it). Config surfaced through `AgentConfig`/MCP so callers can actually use it.
- **Acceptance:** seeded runs reproduce exactly; greedy path unchanged (a1a still byte-identical);
  a repetition-prone prompt demonstrably de-loops with penalties on (reported, not asserted — quality
  claims stay honest).

## W3 — Prefill / TTFT

Prefill is already chunk-batched (~64 tok/chunk) but still submits per layer per chunk with the
shared-uniform race forcing serialization. Port the W1 param-arena + pre-built-bind-group pattern to
`dispatch_prefill_chunk`: one submit per chunk (all layers), KV writes chained, no per-layer fences.
Also: keep the existing prefix-cache (CPU KV mirror) working with the arena path.
**Acceptance:** prefill tok/s + cold/warm TTFT before/after in `a0_native_llm_baseline`; token-identity
of the first decoded token vs legacy prefill.

## W4 — DX12 decode deadlock: root-cause and fix  *(backend robustness)*

Settled finding: DX12 device initializes but decode hangs (35 min observed, 0.0.23). Vulkan is the
only working GPU decode path. Hypotheses to test IN ORDER on this machine (A2000 does DX12):
1. W1 first — 107 fences/token → 1 changes the fence pattern entirely; re-test DX12 with the resident
   path (the hang may be a per-fence `poll(wait_indefinitely)` vs map_async-callback ordering issue
   that simply stops being hit).
2. If still hung: bisect with the existing probes — empty-round-trip bench (`bench_empty_submit_roundtrip`)
   on DX12, then single-layer dispatch, then output top-1 — to localize which primitive deadlocks.
3. Known wgpu-on-DX12 patterns to check: `poll(wait)` on a queue with an unsignaled fence when the
   map_async callback was registered after submit; staging-belt/mapped-buffer reuse across submits.
   Fix in our dispatch pattern if it's ours; upstream issue with minimal repro if it's wgpu's.
**Acceptance:** either DX12 decode completes the bench (number recorded) or a minimal repro + issue
reference documented in the log — no silent "still broken".

## W5 — KV cache: memory + long context

Two stages, honestly separated:
- **W5a (implement now): int8 KV cache.** Quantize K/V slots to int8 + per-head/per-slot f16 scale at
  write, dequant in the attention shader read path. Halves KV memory (80 MiB → ~40 MiB at ctx 1024;
  the win scales with context). Toggle + fallback; **gate: ΔPPL ≤ 5% vs f32 KV on the eval corpus +
  coherent decode**; measured tok/s effect reported (memory-bandwidth-bound attention may speed up).
- **W5b (research-gated): Lexico/Top-K-SAE sparse-dictionary compression** (H1 of the draft notes —
  the real long-context lever). Needs trained per-layer dictionaries + OMP encode; this is a
  D20-class eval task. The training/certification automation is **W10 (Calibration Forge)** below;
  **not** scaffolded as fake-done. ⚑ Timothy: eval-corpus curation when W5b starts.

## W6 — Speculative decoding

- **W6a (implement now): prompt-lookup / n-gram speculation.** Draft K tokens by matching the current
  suffix against the prompt+generated context (no draft model needed), verify with ONE batched
  forward (the prefill batch path), accept the agreeing prefix. Exact-output property: final text is
  bit-identical to greedy decode — makes it safe to default ON for greedy. Big wins on quoting/
  repetitive workloads; ~neutral elsewhere; measured honestly.
  Integrates with the existing `try_accept_topology_draft` machinery (a draft-acceptance seam already
  in the decode loop) rather than adding a second competing mechanism.
- **W6b (model-gated): two-model speculative decode** (SmolLM2-135M drafting for 360M/3B). Needs the
  draft model present + the sampler (W2) for acceptance sampling under non-greedy decode.
  ⚑ Timothy: whether to ship a draft model alongside.

## W7 — Thermal / power governance  *(H2 of the draft notes, made real)*

`ThermalGovernor` exists (orchestrator state machine) but reads no real sensor. Build:
- **Telemetry:** NVML (`nvml-wrapper`) on NVIDIA — GPU temp, power draw, clocks, throttle reasons;
  WMI thermal zone fallback on Windows; graceful "no sensor" degradation. Feature-gated, native-only.
- **Governor wiring:** the decode loop already calls `record_llm_decode_step()`; the governor gains a
  real input stream and its states (Nominal/Warm/Hot) modulate *our own* pacing (inter-token yield /
  batch sizing) — the efficiency-curve behaviour the draft notes argue for.
- **Honest privilege boundary:** setting a hardware TDP cap needs admin/driver rights; we DETECT and
  RECOMMEND (log + MCP-visible telemetry), we do not silently escalate. Documented.
**Acceptance:** telemetry visibly correct vs `nvidia-smi` on this machine; a sustained-decode run
(≥5 min) shows the governor transitioning + pacing instead of raw throttle cliffs; all numbers logged.

## W8 — Tensor-core matmuls  *(gated upstream, scoped honestly)*

CUDA WMMA kernels exist + are certified but off the decode path; wgpu cooperative-matrix needs the
upstream fix (#9741, merged but unreleased — see `docs/WGPU_UPSTREAM_TRACKING.md`). Plan: keep the
soft-fork test-patch path for validation only; wire coopmat into the decode GEMV/GEMM selection
behind a default-OFF feature the moment the wgpu release lands. No decode-path integration before
that release (a soft-fork on the shipping path violates the dependency-modernization rule).
**Acceptance now:** selection seam exists + is tested with the kernel-level cert; decode integration
acceptance defined (token-identity + tok/s) for when the gate opens.

## W10 — Forge upgrade: calibration/adaptation pipeline  *(the training-process automation; answers the W5b "training harness" line)*

**Decision (Timothy, 2026-07-05): this is an UPGRADE of the existing forge, not a new one.** The
forge is already the inference-optimization producer — it certifies kernels (`ForgeGraphExecutor`
vs CPU oracle) and transcodes GGUF→p64 — and the calibration pipeline is the same
produce-and-certify pattern with a new artifact class. The programme's training-shaped processes —
AWQ activation scales (exists), W5a int8-KV scale calibration, W5b sparse dictionaries / Top-K
SAEs, any future PTQ variant — land as new forge capabilities (a `calibration` concern inside the
forge, per §11 split-as-you-go), keeping the settled framing intact: **the forge PRODUCES +
CERTIFIES artifacts; the engine RUNS them.** Stages 4–5 below are the forge's existing muscles;
the upgrade adds stages 1–3:

**Pipeline (5 stages, each automatable):**
1. **Corpus** — assemble/expand the calibration+eval corpus. *Local Ollama is a legitimate resource
   HERE*: synthesizing domain-diverse calibration text offline, no cloud. (⚑ the *eval* half stays
   curation-grade — Timothy's existing W5b ask.)
2. **Capture** — run OUR engine over the corpus with instrumentation on: existing `llm_awq` hooks
   for FFN activations; NEW KV-capture hooks (per-layer K/V tensors) for W5a/W5b. This CANNOT come
   from Ollama — the artifacts compress our engine's own tensors (GQA layout, RoPE convention,
   layer shapes are engine-specific), and Ollama's API exposes neither internal activations nor
   full logits.
3. **Learn** — the artifact trainer: AWQ scale fold (exists), int8-KV scale fit (W5a), dictionary
   learning / Top-K SAE (W5b — the genuinely new component; runs on the existing solver/GPU
   substrate).
4. **Certify** — the ΔPPL ≤ 5% gate via the existing `perplexity_eval_blocking` oracle + coherence
   metrics + (cross-check) an independent llama.cpp-lineage reference via local Ollama, formalizing
   what the bug-hunt probes already do by hand.
5. **Package** — certified artifact + provenance (corpus hash, engine version, gate numbers) into a
   p64/q42 sidecar section, so the engine can refuse uncertified artifacts.

**Status:** ~60% of the skeleton exists IN the forge/bench layer already (capture hooks, cert
oracle, sweep harness, p64 packaging) — the upgrade formalizes them under one forge entry point
(certify-calibration-artifact alongside certify-kernel and transcode) instead of ad-hoc test
harnesses. Build order: the cheap part (KV-capture hooks) lands WITH W5a so the seam exists; the
learner + orchestration land when W5b opens. Ollama stays strictly on the forge side of the line —
it never appears in the inference runtime (CLAUDE.md §1 unchanged).

## W9 — Harness + sustaining

- Profile prints waits/token, per-phase split, and **which path ran** (resident/legacy/sampler/spec) —
  path-visibility counters already exist for top-k vs argmax; extend to the new paths.
- A/B toggles documented in one place (they now number ~10).
- A fast smoke gate (`a1a` + `a1d` + 8-token decode) runnable without the full release build for
  pre-push checks.

## Sequencing

W1 (in flight) → W2 (sampler; reuses W1's readback seam) → W3 (prefill arena; same pattern) →
W4 (DX12 re-test is cheap once W1 lands; root-cause if still hung) → W5a (int8 KV, + the W10
KV-capture seam) → W6a (prompt-lookup) → W7 (thermal telemetry) → W9 continuously; W5b (+ the W10
learner/orchestration) / W6b / W8 open when their gates (corpus / draft model / wgpu release) open.
Each lands with tests green + a log entry before the next starts.

## ⚑ Where Timothy is needed (concrete asks, none blocking W1–W4)

1. **W5b** eval-corpus curation call when sparse-dictionary KV work starts (not yet).
2. **W6b** decision: ship a draft model (135M) alongside the main model?
3. **W7** whether an *opt-in* admin-privileged TDP-cap helper is wanted at all, or detect+recommend only.
4. **W8** confirm the no-soft-fork-on-shipping-path stance holds until wgpu releases the coopmat fix.
