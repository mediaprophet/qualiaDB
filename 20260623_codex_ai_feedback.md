# 20260623 Codex AI Feedback

Date: 2026-06-23

Reviewed inputs:

- `PROJECT_STATUS.md`
- `STELLAR_PHENOMENAL_PLAN.md`
- Prior performance-feedback context from `PERFORMANCE_BRIEF.md` and `PERFORMANCE_FEEDBACK_COMPILATION.md`

Purpose: provide complete implementation-path feedback for the current QualiaDB performance and neuro-symbolic push, with emphasis on what to build first, what to measure, what to avoid overclaiming, and where the architecture needs guardrails.

## Executive Assessment

The project status and STELLAR / Phenomenal plan are substantially stronger than the earlier performance brief. They now contain the key missing pieces:

- Real A2000 kernel benchmark data.
- A clear GPU-specific reframe for ternary inference.
- A runnable ternary-FFN container path.
- Acknowledgement that no end-to-end ternary tok/s exists yet.
- Recognition that the existing `neuro_symbolic_sieve` is the real seed of "ontology in the primitive handler."
- Clear separation between text-only LLM today and multimodal substrate/renderer work.

My overall feedback: the chosen direction is sound, but the next implementation phase should be split into instrumented, independently measurable steps. The current plan's A1 combines two major changes, FFN ternary splice and GPU top-k. Both are good, but merging them into a single proof point risks measurement ambiguity. The engineering discipline shown so far should continue: every performance change should have an independent on/off switch and its own before/after row.

Recommended first sequence:

1. A0 native benchmark harness.
2. A1a GPU top-k reduction, measured alone against current output argmax.
3. A1b FFN-loop ternary splice, measured with top-k off and then top-k on.
4. A2 tiled/coalesced GEMV.
5. KIVI and double-buffered streaming.
6. Compiled ontology control surface.

If implementation time is scarce, do A0 then FFN splice first. If correctness and governance seams matter equally, do top-k immediately after A0 because it reduces readback and becomes the integration point for the sentinel and compiled semantic control surface.

## What Is Strong

### 1. The GPU Ternary Reframe Is Correct

The A2000 data is the most important new evidence:

| Kernel | ms/dispatch | vs F16 |
|--------|------------:|-------:|
| F16 baseline | 0.963 | 1.00x |
| ternary base-3 | 1.140 | 0.85x |
| ternary 2-bit branchless | 0.544 | 1.77x |

This validates the core architectural correction:

- Base-3 is a good storage/distribution encoding.
- Base-3 is not a good GPU hot-path encoding when it requires division, modulo, and branchy unpack.
- 2-bit branchless is the correct runtime layout.
- The GPU win is bandwidth, coalescing, and uniform execution, not literal removal of multiply.

This should be treated as a locked design decision unless a future benchmark disproves it.

### 2. The Plan Is Honest About End-To-End Gaps

The documents correctly avoid claiming:

- End-to-end ternary tok/s.
- Fresh native F16/Q8 tok/s.
- TTFT cold/warm numbers.
- Real multimodal generation.
- WASM/mobile/NPU parity.

This honesty matters. It makes the project credible to external reviewers and prevents phantom speedups from shaping implementation priorities.

### 3. The Existing `neuro_symbolic_sieve` Is The Right Seed

The plan's recognition of `neuro_symbolic_sieve` is important. The current code already contains a bounded token-mask mechanism:

- `token_id -> Q42LEX hash`
- stack-bounded masks
- argmax-time sieve
- zero-heap hot-path intent

That is the right foundation. The next step should generalize it carefully, not replace it with a full ontology engine in the decode loop.

### 4. The Multimodal Honesty Is Good

The docs correctly state:

- Generative VLM is not present.
- The current LLM path is text-only.
- Spatial/geometry tensors should remain F16/BF16.
- Analyze-before-generate is the right sequence.
- CML consent gates should precede large hardware/capability shifts.

This prevents external reviewers from assuming capabilities that are not yet implemented.

## Highest Priority Recommendation

Build the next phase as toggled experiments, not as one bundled leap.

The first proof point should produce a table like this:

| Build | FFN ternary | GPU top-k | Tiled GEMV | Decode tok/s | TTFT warm | Notes |
|-------|-------------|-----------|------------|-------------:|----------:|-------|
| Baseline | off | off | off | TBD | TBD | current live path |
| Top-k only | off | on | off | TBD | TBD | isolates readback win |
| Ternary only | on | off | off | TBD | TBD | isolates FFN splice |
| Ternary + top-k | on | on | off | TBD | TBD | combined A1 result |
| Ternary + top-k + tiled | on | on | on | TBD | TBD | A2 result |

Why this matters:

- If top-k and FFN ternary land together, a speedup cannot be attributed cleanly.
- If a regression appears, debugging will be harder.
- If the model output changes, it will be unclear whether the issue is quantization, sampling, masking, or readback.

Keep each feature behind a runtime flag, env var, or benchmark config:

- `QUALIA_LLM_TERNARY_FFN=0/1`
- `QUALIA_LLM_GPU_TOPK=0/1`
- `QUALIA_LLM_TILED_GEMV=0/1`
- `QUALIA_LLM_SIEVE=0/1`

Names can differ, but toggles are important.

## Feedback On The Sequenced Plan

### A0 - Instrumentation Harness

Strongly agree. This should be the first merged piece.

Required outputs:

- JSON and CSV.
- Model ID and exact file path/hash.
- Backend: DX12, Vulkan, Metal, WebGPU.
- GPU name and adapter limits.
- Weight policy: F16/BF16, Q8, ternary FFN, mixed.
- Residency: resident, streaming, double-buffer streaming.
- TTFT cold broken into components.
- TTFT warm.
- Prefill tok/s.
- Decode tok/s.
- Kernel-only timing where timestamp queries are available.
- Host wall-clock timing for end-to-end reality.

Suggested JSON shape:

```json
{
  "date": "2026-06-23",
  "backend": "dx12",
  "adapter": "NVIDIA RTX A2000 12GB",
  "model": "SmolLM2-360M-Instruct",
  "weight_policy": "q8_baseline",
  "residency": "resident",
  "features": {
    "ternary_ffn": false,
    "gpu_topk": false,
    "tiled_gemv": false,
    "sieve": false
  },
  "ttft_ms": {
    "cold_total": 0.0,
    "warm_total": 0.0,
    "mmap": 0.0,
    "manifest_parse": 0.0,
    "pipeline_create": 0.0,
    "first_upload": 0.0,
    "prefill": 0.0,
    "first_decode": 0.0
  },
  "throughput": {
    "prefill_tok_s": 0.0,
    "decode_tok_s": 0.0
  },
  "gpu_kernel_ms": {
    "ffn": 0.0,
    "attention": 0.0,
    "logits_projection": 0.0,
    "topk": 0.0
  }
}
```

Important measurement note:

Timestamp readback should not be part of the per-token kernel time. For first-pass measurement, synchronous polling is acceptable if it is explicitly separated. Production telemetry should use a background/ring path.

### A1 - FFN-Loop Splice Plus GPU Top-K

I agree with both parts, but recommend splitting implementation:

#### A1a - GPU Top-K

This is lower-risk than the FFN splice and immediately removes a known readback issue.

Current problem:

- Full vocab logits are about 49,152 x f32 = about 196 KB per token.
- The current chunked path updates argmax on CPU.
- The wasm async code explicitly says there is no WGSL argmax reduction shader.

Recommended design:

- WGSL multi-pass block reduction.
- Do not bitonic-sort the full vocab.
- First pass: per-workgroup local top-K over chunks.
- Second pass: merge workgroup candidates.
- Read back only `(token_id, logit)` pairs.
- CPU applies temperature and sampling over the returned candidate set.
- If sampling requires probability, compute softmax over top-K on CPU.

Return logits, not full probabilities, unless the kernel already has a cheap stable softmax. The CPU can softmax the top-K.

Top-K details:

- Start with `K = 32` or `K = 64`.
- Support top-1/argmax as a special cheap mode.
- Preserve deterministic tie-breaking: lower token ID wins or stable order wins.
- Handle NaN as disallowed or negative infinity.
- Integrate sieve/bias before comparison where possible.

Test cases:

- CPU top-K parity on random logits.
- Deterministic ties.
- NaN handling.
- Masked token never appears.
- All masked tokens returns an explicit "no valid token" error.
- K > vocab handled safely.
- K small paths: K=1, K=4, K=32.

Governance note:

Top-K is the right seam for Phase-8 sentinel visibility. It should eventually return candidate flags alongside token IDs:

```text
token_id, logit, concept_id_or_0, control_flags
```

But the first version should keep semantics optional to avoid coupling the top-K kernel to unfinished ontology work.

#### A1b - FFN Ternary Splice

This is the main model-performance proof point.

Key requirements:

- Persistent pipeline created once.
- Runtime branch only on tensor role/format, not on every weight.
- 2-bit GPU-resident layout.
- Per-tensor scale handling verified.
- F16/Q8 fallback untouched.
- Parity test for a representative real model tensor shape, not only synthetic 4096 x 4096.
- End-to-end prompt sanity test.

Important caveat:

The 4096^2 benchmark is valuable, but actual model shapes may differ:

- SmolLM2-360M has `n_embd = 960`.
- FFN dimensions may not align with the benchmark's ideal shape.
- Real row/column layout, cache behavior, and bind overhead can change the ratio.

Do not extrapolate the 1.77x kernel win directly to end-to-end decode. Measure it.

Recommended live measurements:

- Baseline F16/Q8 current path.
- Ternary FFN path with CPU argmax unchanged.
- Ternary FFN path with GPU top-K.
- Ternary FFN path with sieve disabled and enabled.

### A2 - Tiled / Coalesced GEMV

Strongly agree that this is the next kernel lever.

The 1.77x result is good, but the gap between 1.77x and the byte-ratio fantasy indicates that the kernel is not yet saturating memory bandwidth.

Try:

- Workgroup activation tile staging.
- Multiple output rows per workgroup.
- Multiple outputs per thread where register pressure allows.
- Workgroup sizes: 64, 128, 256.
- Vectorized loads where WGSL/backend permits.
- Subgroup reductions where available, but keep fallback paths because WebGPU subgroup support is not universal.

Avoid:

- Optimizing only for 4096 x 4096.
- Assuming browser WebGPU has the same subgroup/f16 behavior as native DX12.
- Making the kernel so specialized it cannot handle SmolLM/Qwen dimensions.

### A3 - KIVI KV Cache

Agree with priority after A1/A2.

KIVI is the correct first KV-cache compression for a single-user edge runtime:

- It directly attacks long-context VRAM pressure.
- It is more relevant than PagedAttention for local single-stream use.
- It complements resident-model goals.

Pass/fail should include:

- VRAM savings.
- Long-context max length improvement.
- Decode tok/s at long context.
- Quality drop below agreed threshold.
- Sentinel/refusal behavior unchanged.

The `<1% quality drop` target is useful, but define the metric:

- Perplexity delta?
- Accuracy on a small eval set?
- Governance prompt behavior?
- Human-rated response acceptability?

Recommendation: use both perplexity delta and a small curated governance/persona regression suite.

### A4 - Double-Buffered Streaming

Agree, but keep it behind the resident-path proof point.

This is critical for the 7B-on-12GB demonstrator, but it is easier to debug once resident small-model decode is measurable.

Needed details:

- Explicit state machine for upload buffer A/B.
- Clear ownership of staging buffers.
- No hidden per-token allocation.
- Backpressure when upload cannot keep up.
- Separate timing for upload, compute, and synchronization stalls.

Important risk:

`wgpu` abstraction may not expose copy/compute overlap equally across all backends. Measure per backend. DX12 may behave differently from Metal and WebGPU.

### A5 - Attention Quantization And W4A4

Agree with order:

1. FFN ternary.
2. Attention weights Q4/Q5/Q8 or AWQ-style.
3. KIVI.
4. W4A4 only after quality harness exists.

W4A4 is not just another compression step. Activation quantization changes runtime numerics and can create subtle quality failures. It needs a real quality gate.

### A6 - Compiled Ontology Control Surface

The plan's framing is correct: compile an ontology control surface, do not run live ontology reasoning in the hot loop.

Recommended record design:

```rust
#[repr(C)]
pub struct TokenControlSlot {
    pub token_id: u32,
    pub concept_id: u32,
    pub bias_q8: i8,
    pub flags: u8,
    pub route_id: u16,
    pub attestation_hash_lo: u64,
}
```

This is illustrative, not a final ABI. The important idea is fixed-size records, sorted by token ID or concept ID, mmap-safe, and bounded.

Recommended flag classes:

- `SOFT_BIAS_POSITIVE`
- `SOFT_BIAS_NEGATIVE`
- `HARD_VETO`
- `REQUIRES_CONSENT`
- `ROUTE_TO_LOGIC`
- `SENSITIVE_DOMAIN`
- `ATTESTED`

Critical governance rule:

Only attested entries can hard-veto or route execution in the hot path. Machine-proposed mappings may inform a background suggestion layer, but must not become hot-path enforcement until ratified.

Do not start with trigger-node-to-WASM handoff in the first version. Start with:

1. Bias.
2. Veto.
3. Audit event.
4. Then route flags.

Reason: routing to logic extensions introduces scheduling, context extraction, result injection, and user-consent complexity. Bias/veto proves the control surface first.

### A7 - TTFT Pass

Agree, but some TTFT work can happen earlier if cheap:

- Pipeline prebuild can be measured during A0.
- Manifest parse persistence can be added early.
- First-layer lazy upload can be measured after A1.

Avoid making TTFT optimization block the live ternary proof point.

## Feedback On The Primitive Ontology Idea

The answer is yes, with boundaries.

It is possible and desirable to put semantic weights into the primitive handler, if "semantic weights" means compiled, attested, fixed-layout control records.

It is not desirable to put rich ontology traversal, `skos:closeMatch`, OWL reasoning, or graph search into the token hot path.

Use three layers:

### Layer 1 - Hot Path Control

Runs per token or per top-K candidate.

Allowed operations:

- Token ID lookup.
- Concept/control ID lookup.
- Bias add.
- Hard mask/veto.
- Small route flag emission.
- Attestation check by flag/hash.

Must be:

- Fixed budget.
- No heap.
- No string parsing.
- No unbounded traversal.
- Optional in benchmark toggles.

### Layer 2 - Async Semantic Sieve

Runs behind the decode loop.

Allowed operations:

- Detokenize spans.
- Resolve phrases to Q42LEX/CML concepts.
- Run `skos:closeMatch` or equivalent mapping.
- Detect semantic drift.
- Propose updated bias/mask state for future tokens.

Can allocate if outside hot path, but should still be bounded for edge devices.

### Layer 3 - Logic Extension Routing

Runs only at safe boundaries:

- End of phrase.
- Tool-call-like trigger.
- Sentence boundary.
- Explicit CML capability gate.

Allowed operations:

- CAS.
- Deontic evaluator.
- SHACL validator.
- Domain-specific specialized library.
- WASM extension.

Should not interrupt the GPU loop every token.

## BPE Token Versus Concept: Keep The Boundary Bright

The docs correctly state that a BPE token is not a concept.

The current `neuro_symbolic_sieve` maps labels to first token IDs. That is useful for grammar-constrained emission and anchors, but it is not general semantic understanding.

Risks if overstated:

- False positives from partial tokens.
- Concepts split across multiple tokens.
- Different tokenizations of the same surface form.
- Prefix/suffix tokens with no standalone meaning.
- Language and morphology issues.

Recommended next bridge:

- Build a token-span trie or finite-state phrase matcher from Q42LEX labels.
- Match spans over decoded tokens.
- Resolve only at word/phrase boundaries.
- Feed concept hits into the async semantic sieve.
- Promote high-confidence, attested single-token anchors into the hot control table.

## Documentation Feedback

### Project Status Document

Strong points:

- Clear done/pending distinction.
- Honest multimodal status.
- Clear model inventory.
- Real A2000 benchmark data.
- Good reviewer question framing.

Suggested improvements:

1. Add a one-page "What should be reviewed?" section at the top.
2. Split "done and measured" from "built but not integrated" even more sharply.
3. Include a small glossary for external reviewers:
   - NQuin
   - Q42W
   - Q42LEX
   - CML
   - Phase-8 sentinel
   - resident vs streaming
4. Add exact command lines for reproducing the A2000 kernel benchmark.
5. Add a "claims we are not making" box:
   - no end-to-end ternary speedup yet
   - no VLM generation yet
   - no native tok/s baseline yet
   - no TTFT baseline yet

### STELLAR / Phenomenal Plan

Strong points:

- The decisions table is excellent.
- The phase ordering is mostly sound.
- The ontology-in-primitive-handler framing is now disciplined.
- It carries governance constraints into the performance plan.

Suggested improvements:

1. Split A1 into A1a top-K and A1b FFN splice, or at least require separate benchmark toggles.
2. Add a "measurement attribution" requirement for every phase.
3. Add "rollback criteria" for each optimization.
4. Add backend portability notes for WebGPU/subgroups/f16.
5. Add a "minimum viable proof point" section:

```text
MVPP: SmolLM2-360M on A2000, resident mode, Q8 baseline vs ternary-FFN, same prompt set, measured native decode tok/s, TTFT warm, and output sanity.
```

## Specific Technical Recommendations

### Top-K Kernel

Recommended algorithm:

1. Each workgroup scans a contiguous vocab slice.
2. Each workgroup keeps local top-K in workgroup memory.
3. Write partial candidates to a small candidate buffer.
4. Second pass merges partial candidates.
5. CPU reads final K candidates.

Avoid full-vocab bitonic sort. It solves a bigger problem than needed.

Top-K candidate struct:

```rust
#[repr(C)]
pub struct TopKCandidate {
    pub token_id: u32,
    pub logit: f32,
    pub flags: u32,
    pub concept_id: u32,
}
```

First implementation can set `flags = 0` and `concept_id = 0`.

### Sampling

Keep tokenization and detokenization on CPU.

For sampling:

- GPU returns top-K logits.
- CPU applies temperature, repetition penalty, top-k/top-p policy over candidates.
- Sentinel inspects candidates.
- CPU chooses token.

If exact top-p over the full distribution is required, top-K is approximate unless K is large enough. That is acceptable if documented. For governance and performance, top-K is the right first step.

### Sieve Integration

The current CPU `update_streaming_argmax_sieved` masks during CPU argmax. The GPU top-K path should eventually accept:

- Optional token allow mask.
- Optional bias table.
- Optional hard veto table.

Do this incrementally:

1. Plain top-K.
2. Top-K with hard token mask.
3. Top-K with bias.
4. Top-K with concept flags.

### Ternary FFN

Checklist:

- Q42 manifest identifies ternary FFN tensors unambiguously.
- Runtime does not reconstruct GPU layout per token.
- Scale application is tested against CPU oracle.
- Shape-specific tests include SmolLM2 and Qwen FFN dimensions.
- F16/Q8 fallback is always available.
- Failure to bind ternary path falls back or errors explicitly, not silently.

### Tiled GEMV

Benchmark shapes should include:

- Synthetic 4096 x 4096.
- SmolLM2 real FFN shapes.
- Qwen 1.5B real FFN shapes.
- Qwen 7B real FFN shapes if available.

Report:

- achieved bandwidth estimate
- occupancy
- register pressure if available
- dispatch time
- end-to-end decode tok/s impact

### KIVI

Do not treat "less than 1% quality drop" as self-evident. Define the eval.

Recommended quality gate:

- PPL delta on a fixed small corpus.
- 20 to 50 governance/persona prompts.
- refusal/sentinel regression.
- long-context retrieval prompt.

### Streaming

Before implementing double-buffering, add counters:

- bytes uploaded per token
- upload wait time
- compute wait time
- buffer swap count
- stall reason

Otherwise you will not know whether double-buffering worked.

## Risk Register

| Risk | Severity | Why it matters | Mitigation |
|------|----------|----------------|------------|
| Bundled A1 changes blur attribution | High | Speedup/regression source unclear | Feature toggles and separate benchmark rows |
| End-to-end ternary speedup smaller than kernel speedup | High | Other decode costs may dominate | Measure baseline, top-k, splice separately |
| Top-K changes sampling semantics | Medium | Exact full-vocab top-p is different | Document top-k sampling mode, keep fallback |
| Ontology hot path becomes too rich | High | TPS collapses | Hot path only O(1) bias/mask/route |
| BPE token treated as concept | High | Semantic false positives | Span resolver and phrase trie |
| Over-veto collapses generation diversity | Medium | Model becomes brittle | Use graded bias, hard veto only attested |
| WebGPU subgroup/f16 assumptions fail | Medium | Browser path regresses | Feature-detect and keep fallback kernels |
| Streaming overlap unavailable on some backends | Medium | wgpu backend variance | Measure per backend, keep sequential fallback |
| W4A4 quality loss | Medium | Hard to debug semantic degradation | Delay until quality harness exists |
| Governance mappings become machine-authored enforcement | Critical | Violates project rail | Enforce only attested baselines |

## Recommended Issue Breakdown

### Issue 1 - Native Benchmark Harness

Deliverable:

- `cargo` or CLI command that emits JSON/CSV for native A2000 baseline.
- Supports feature toggles.
- Separates setup, GPU kernel, and end-to-end timing.

Acceptance:

- Produces F16/Q8 SmolLM2-360M resident baseline.

### Issue 2 - GPU Top-K Reduction

Deliverable:

- `fused_top_k_reduction.wgsl`
- CPU parity tests.
- Integrated optional path for output projection.

Acceptance:

- Reduces logit readback from full vocab to K candidates.
- Same argmax as CPU path for K=1 or includes CPU argmax token in K.

### Issue 3 - FFN Ternary Live Splice

Deliverable:

- Ternary FFN path inside live decode loop.
- Persistent pipeline and buffer reuse.
- Fallback path untouched.

Acceptance:

- End-to-end ternary decode tok/s measured versus baseline.
- Output is sane on a fixed prompt set.

### Issue 4 - Attribution Benchmark Matrix

Deliverable:

- Runs baseline, top-k only, ternary only, combined.

Acceptance:

- Results table can be pasted into `PROJECT_STATUS.md`.

### Issue 5 - Tiled GEMV

Deliverable:

- New or revised GEMV kernel with activation tiling and backend fallbacks.

Acceptance:

- Higher decode tok/s or achieved bandwidth on at least SmolLM2 and one Qwen shape.

### Issue 6 - KIVI KV Cache

Deliverable:

- K 2-bit/channel and V 4-bit/token cache mode.

Acceptance:

- Longer context in 12 GB resident mode with defined quality threshold.

### Issue 7 - Compiled Control Surface V1

Deliverable:

- Fixed-record control table loaded from attested shard.
- Bias/veto only.
- No logic routing yet.

Acceptance:

- Attested hard veto masks candidate at top-K step.
- Machine-proposed mapping cannot hard-veto.

### Issue 8 - Control Surface V2 Routing

Deliverable:

- Route flags to logic extension at safe boundary.

Acceptance:

- Deterministic extension result can be injected without per-token TPS hit.

## Suggested Changes To The Open Decision

The current open question asks whether to do live splice, top-k, or tiled GEMV first.

My answer:

1. Do A0 first.
2. Do top-k first if it can be completed in a small isolated patch.
3. Do FFN splice immediately after.
4. Measure top-k only, ternary only, and combined.
5. Do tiled GEMV after the live splice because real end-to-end data will tell you where tiling matters most.

Rationale:

- Top-k is a clean low-risk bandwidth win and creates the governance seam.
- FFN splice is the main proof point.
- Tiled GEMV is important, but without live decode attribution it may optimize a kernel while another part dominates.

If there is only time for one implementation step after A0, choose the FFN splice. But do not let top-k drift far behind; it is both a performance win and the right place to attach the future compiled ontology control surface.

## Feedback On External Reviewer Framing

For external review, ask sharper questions:

1. Given the measured 2-bit 1.77x kernel win, what GEMV tiling pattern would you try for SmolLM/Qwen FFN shapes?
2. For top-K over 49k logits in WGSL, would you use per-workgroup fixed-K insertion, heap-like local selection, or another method?
3. How would you structure readback to avoid blocking the next decode step?
4. What top-K value is sufficient for governance visibility without approximating too aggressively?
5. How should KIVI quality be measured for a local human-centric assistant?
6. What is the safest ABI for compiled ontology control records?

Avoid asking broad "how do we make it faster?" questions now. The plan is specific enough to ask implementation-level questions.

## Final Verdict

The plan is credible, technically grounded, and unusually honest. The A2000 data justifies the 2-bit branchless decision. The existing `neuro_symbolic_sieve` justifies treating compiled ontology control as an extension of current architecture, not a speculative bolt-on. The main thing to protect now is measurement clarity.

Recommended immediate path:

1. Build A0.
2. Land GPU top-K as an isolated measured optimization.
3. Land FFN ternary splice as the main proof point.
4. Measure combined effect.
5. Then close kernel headroom with tiled GEMV.
6. Only then push deeper into KIVI, double-buffer streaming, and compiled ontology controls.

The project should keep saying, plainly: "We have a fast isolated ternary kernel and a runnable ternary container. We do not yet have end-to-end ternary TPS." That sentence is not a weakness. It is the discipline that will make the eventual number trustworthy.
