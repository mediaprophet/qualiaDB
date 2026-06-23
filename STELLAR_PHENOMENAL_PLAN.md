# STELLAR / Phenomenal — Synthesised Implementation Plan

*Date: 2026-06-23. Branch: `0.0.19`. This is the **best-available implementation plan** for the
performance + neuro-symbolic push, synthesised from all external review (Reviews 1–2, Grok, Codex,
the feedback compilation) **and** direct code inspection. It makes the contested calls explicitly so
the next implementation pass is unambiguous. It is a **proposal for ratification** (machine proposes,
human ratifies) — not unilateral doctrine. Companion docs: `PROJECT_STATUS.md` (snapshot),
`PERFORMANCE_BRIEF.md`, `PERFORMANCE_FEEDBACK_COMPILATION.md`, `STELLAR_MISSION.md` (§A vision).*

---

## 0. The thesis (locked)
Decode is **memory-bandwidth-bound**; prefill is **compute/occupancy-bound**. The win on consumer
hardware (dev: NVIDIA A2000, 12 GB) is **fewer weight/KV bytes + better occupancy + hidden PCIe**,
*not* avoiding arithmetic. Compression is also a **residency lever** (a compressed 7B moves from
streaming → resident). Governance is first-class: the hot path enforces only **attested** human
baselines; rich reasoning stays off the critical loop.

## 1. Design decisions (DECIDED — the "best available" calls)

| # | Decision | Rationale / evidence |
|---|---|---|
| D1 | **Ternary runtime layout = 2-bit branchless; base-3 = archive/distribution only** | Measured on A2000: base-3 **0.85×** (slower than F16, the `/3`,`%3` unpack costs more than it saves), 2-bit branchless **1.77× vs F16 / 2.10× vs base-3**. |
| D2 | **Ternary inner loop = branchless FMA** `acc += (f32(c==1)-f32(c==2))*x` | On GPU the multiply is a free FMA; warp divergence + integer unpack are the real costs. "No-multiply" is a CPU/ASIC argument, not GPU. |
| D3 | **Quant policy: ternary the FFN only; attention/norms/embeddings/spatial stay high-fidelity** | FFN dominates param bytes + is least precision-sensitive; coordinates/norms collapse under ternary. Already implemented in `tensor_roles::ternary_eligible`. |
| D4 | **Sampling: GPU top-k reduction; read back only top-k `(id,prob)`** (~hundreds of bytes, not the 196 KB full-logit readback/token) | Decode is bandwidth-bound; the sentinel only needs the high-probability mass, not the 49 k near-zero tail. |
| D5 | **Top-k kernel = multi-pass block reduction**, NOT bitonic sort | We need top-k, not a fully sorted 49 k vocab; per-workgroup partial top-k → merge is simpler + cheaper for k≪N. |
| D6 | **Tokenize / detokenize = host CPU (Rust)** | Branchy BPE string ops on tiny data; no GPU upside. |
| D7 | **FFN splice = additive branch** on `ggml_type == TERNARY` in `encode_fused_ffn_expansion`; **persistent pipeline + buffer reuse**; F16/Q8 path untouched (fallback) | The earlier 1.02× microbench failure was per-call pipeline rebuild; persistence is the fix. Keep working inference intact. |
| D8 | **Decode = GEMV** (tiled, activation-tile in workgroup memory, multi-output-per-thread, subgroup reductions, test wg 64/128/256); **prefill = register-blocked tiled GEMM** | The 1.77× (not ~8×) shows the naive one-output-per-thread GEMV isn't bandwidth-bound yet — ~4.5× headroom in coalescing. |
| D9 | **Neuro-symbolic = generalise the *existing* `neuro_symbolic_sieve` into a compiled, attested control surface** (`token/concept → {bias, veto, route_flags, concept_id, deontic_flags}`), applied at the top-k step | The sieve already does a zero-heap `token_id → Q42LEX-hash` mask at argmax — this is "ontology in the primitive handler" in embryo. Generalise, don't greenfield. |
| D10 | **Strict layering**: hot path = `O(1)` bias/mask/route only; span→concept resolution = **async** sliding-window; trigger-nodes → logic extensions **off** the critical loop | Rich `skos:closeMatch` per token would obliterate TPS. A BPE token is an anchor, not a concept. |
| D11 | **Governance: hot-path surface enforces only `cml:Attested` baselines** (Curation Prime Directive); steering ≠ veto | Applies *encoded prior human wisdom*; never authors new norms. DIKW: wisdom stays out-of-band. |
| D12 | **KV-cache = KIVI** (2-bit K/channel, 4-bit V/token) + high-fidelity fallback; KIVI before PagedAttention | Single-user edge; KIVI attacks the KV bandwidth wall directly; paged matters for multi-request serving. |
| D13 | **Streaming (>VRAM) = double-buffered ping-pong async upload** (compute N while uploading N+1) + mmap `madvise(WILLNEED)`/sequential hints | The A2000 runs a 7B only via streaming unless compressed; overlap hides the PCIe cost. |
| D14 | **Attention quant order: Q4/Q5/Q8 (AWQ-style) *before* W4A4** | W4A4 activation quant is the hard, quality-risky one; do the safer residency win first. |
| D15 | **TTFT: pre-build pipelines/bind-groups at startup; pre-compiled `.q42` distribution; lazy first-layer upload + background residency; warm KV at model-select** | First-token cost is load + pipeline-create + prefill; pre-build + distribute removes runtime transcode. |
| D16 | **Multi-modal: analyze-before-generate; spatial = F16; reserved tokens as spatial anchors; CML consent gates reuse Phase-4/5 rails** | Lock zero-copy bindings before output mutation; geometry needs precision; control via the existing consent substrate. |
| D17 | **Instrumentation first-class: a native A2000 benchmark harness** (cold/warm TTFT, prefill/decode tok/s, residency, policy) emitting JSON/CSV; GPU **timestamp queries** (begin/end-of-pass) isolate kernel time | "Measure native before optimizing." Timestamps measure GPU-timeline, independent of host submit/readback. |

## 2. Non-negotiable rails (carried into every phase)
No external inference libs · consumer hardware (no datacenter answers) · zero-heap / fixed-budget hot
paths · machine-proposes-human-ratifies (attested-only enforcement) · DIKW wisdom out-of-band ·
affordability · human authorship (AI as instrument). Rich reasoning **never** on the GPU critical
loop; the WGSL stays lightweight (bias/mask/route + early-exit flags only).

## 3. Sequenced plan (phases, with pass/fail)

> Ordering reflects the cross-review consensus: instrument → splice+top-k (the real number) →
> close kernel headroom → KV/streaming → deeper quant → the ontology control surface → TTFT.

### A0 — Instrumentation harness *(prerequisite)*
- **Build:** a native bench harness driving the live decode/prefill path; fields = cold/warm TTFT,
  prefill tok/s, decode tok/s, residency mode, weight policy; JSON/CSV out. GPU `TIMESTAMP_QUERY`
  device feature (fallback to wall-clock if absent), begin/end-of-pass `timestamp_writes`, resolve →
  small readback. Browser/WASM row separate from native.
- **Pass/fail:** produces a trustworthy **native F16/Q8 SmolLM2-360M baseline** (the missing number),
  with setup time separated from kernel time.

### A1 — FFN-loop splice + GPU top-k *(the main event)*
- **Build:** persistent ternary pipeline in the bridge struct; branch `encode_fused_ffn_expansion`
  on `ggml_type == TERNARY` → bind 2-bit weights → 2-bit branchless kernel; F16/Q8 fallback intact.
  Add `fused_top_k_reduction.wgsl` (multi-pass block reduction) → read back top-k `(id,prob)`.
  Drive with the **runnable ternary container** (`compile_gguf_to_q42_ternary_ffn`, already built) on
  the q8 SmolLM2-360M; in-loop timestamp queries (synchronous readback for the first pass, excluded
  from per-token timing).
- **Pass/fail:** real **end-to-end ternary decode tok/s vs the A0 F16/Q8 baseline** on the A2000;
  output quality sane (no garbage); existing F16/Q8 path unregressed.

### A2 — Decode GEMV tiling *(close the kernel headroom)*
- **Build:** tiled GEMV — stage activation tile in `workgroup` memory, multiple output features per
  thread, subgroup reductions where available; sweep wg 64/128/256. Profile achieved bandwidth
  (Nsight/PIX).
- **Pass/fail:** measurable decode-TPS gain over A1 and/or a clear rise in achieved memory bandwidth
  (target: move from 1.77× toward the ~8× weight-byte ratio).

### A3 — KIVI KV-cache
- **Build:** K 2-bit/channel, V 4-bit/token; high-fidelity fallback mode.
- **Pass/fail:** longer context fits resident on 12 GB with <1% quality drop on chosen evals.

### A4 — Double-buffered streaming *(the 7B-on-12 GB demonstrator)*
- **Build:** ping-pong VRAM buffers + async copy in `dispatch_transformer_forward`; mmap prefetch
  hints; bind-group reuse; no readback/timestamp poll inside the critical loop.
- **Pass/fail:** Qwen2.5-7B runs; streaming decode TPS improves vs the sequential path; upload stalls
  drop.

### A5 — Attention + deeper quant
- **Build:** attention projections → Q4/Q5/Q8 (AWQ-style scales in header); then a W4A4 prototype
  behind a quality harness (perplexity + governance/persona prompts + refusal/sentinel regression).
- **Pass/fail:** acceptable quality at lower resident VRAM; W4A4 produces usable trade-off data.

### A6 — Compiled ontology control surface *(neuro-symbolic endgame)*
- **Build:** generalise `neuro_symbolic_sieve` → compiled `.q42lex` shard
  (`token/concept → {bias, veto, route_flags, concept_id, deontic_flags}`), **attested-only**,
  applied at the top-k step; async sliding-window token-span → `Q42LEX`/CML concept resolution
  (`skos:closeMatch`); trigger-nodes hand off to logic extensions (modalities / CAS / specialized
  libs / WASM) off the loop; hot-swappable via `<cml:load_ontology>`.
- **Pass/fail:** an attested veto zeroes a token at the top-k step with no measurable TPS hit; a
  trigger-node routes to a logic extension and injects a deterministic result; machine-proposed
  mappings are *not* enforced in the hot path (governance test).

### A7 — TTFT pass
- **Build:** pre-create pipelines/bind-groups at startup; pre-compiled `.q42` distribution
  (HF/WebTorrent); lazy first-layer upload + background residency; warm KV at model-select; browser
  Cache API/IndexedDB + HTTP-range.
- **Pass/fail:** cold TTFT decomposed (mmap/parse/pipeline/upload/prefill/first-decode) and reduced;
  warm decode TPS unregressed.

### MM — Multi-modal track *(parallel, after A1)*
- **Build:** `fused_spatial_encoder.wgsl` ingesting F16 geometry (point cloud / SH) → tokens
  cross-attended into the context; reserved-token spatial anchors; zero-copy VRAM → render hand-off;
  CML `<cml:capability_shift>` consent gates (Phase-4/5 rails) before VRAM allocation. **Analyze**
  first; **generate** later.
- **Pass/fail:** a mapped 3D asset is ingested + cross-attended (analyze) under a consent gate, with
  geometry kept F16 and zero CPU round-trip to the renderer.

## 4. Benchmark matrix (to populate as phases land)

| Mode | Backend | Model | Weight policy | Residency | TTFT cold | TTFT warm | Prefill t/s | Decode t/s |
|---|---|---|---|---|--:|--:|--:|--:|
| Baseline | DX12 A2000 | SmolLM2-360M | F16/BF16 | resident | — | — | — | — |
| Baseline | DX12 A2000 | SmolLM2-360M | Q8 | resident | — | — | — | — |
| Ternary | DX12 A2000 | SmolLM2-360M | FFN 2-bit / attn F16 | resident | — | — | — | — |
| Ternary+tiled | DX12 A2000 | SmolLM2-360M | FFN 2-bit, tiled GEMV | resident | — | — | — | — |
| Streaming | DX12 A2000 | Qwen2.5-7B | BF16 | streaming | — | — | — | — |
| Streaming DB | DX12 A2000 | Qwen2.5-7B | mixed | double-buffer | — | — | — | — |
| Browser | WebGPU | SmolLM2-360M | current | resident | — | — | — | refresh 5.9 |

## 5. What's already in place (the plan starts from reality)
- 2-bit branchless kernel + base-3 kernel + F16 baseline, **on-device-verified (A2000)**; CPU oracles.
- Transcode: safetensor (verbatim/ternary/FFN-policy) + **runnable** GGUF→ternary-FFN container
  (hyperparams + tokenizer preserved). Name→role policy. The persistent-pipeline bench harness.
- `neuro_symbolic_sieve` (the control-surface foundation); the deontic VM + Q42LEX + the 14 logic
  modalities + CAS + specialized libs (the logic-extension targets). Resident + streaming residency
  modes; chunked host argmax (the seam for top-k + the sieve).
- Models on disk: SmolLM2-360M, Qwen2.5-1.5B, Qwen2.5-7B; q8 SmolLM2 GGUF.

## 6. Risk & discipline notes
- **The discipline that decides success:** ruthless separation of the `O(1)` hot path from
  async/cold/WASM layers. Every extra per-token cycle compounds on a bandwidth-bound loop.
- **Over-constraint risk:** graded influence (hard veto / soft bias / confidence-weighted), not
  binary on/off, or generative diversity collapses.
- **Quality gating:** W4A4 + ternary adoption gate on perplexity **and** governance/persona prompts
  **and** refusal/sentinel regression — not perplexity alone.
- **Ontology compilation** becomes a first-class AOT tool (like weight transcode): versioned,
  validated to preserve semantics; only the high-certainty attested fragment lives hot.

## 7. Open questions for the next feedback round
1. First implementation step: **A1 (splice + top-k) for the real number**, or **A2 (tiled GEMV) to
   close the kernel headroom first**? (Consensus leans A1; A2 is lower-risk + fully measurable now.)
2. Ontology control surface first pass: bias/veto masks only, or also trigger-node → WASM handoff?
3. Ontology shards: global-per-model or dynamically composable with priority/override?
4. Quality benchmark to gate ternary FFN adoption: perplexity, governance prompts, or both?
