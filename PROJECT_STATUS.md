# QualiaDB — Project Status & Decision Brief

*Date: 2026-06-23. Branch: `0.0.19` (pushed to `github.com/mediaprophet/qualiaDB`). Purpose: a
single self-contained snapshot of where the project is, written so an external reviewer can read it
cold and advise on the **implementation path** before it's chosen. Authorship: Timothy C. Holborn /
WebCivics; drafted with AI tooling as an instrument (not an author). Status is reported honestly —
"done", "measured", "pending", and "proposed" are kept distinct.*

---

## 0. What QualiaDB is

A **sovereign, edge-native, neuro-symbolic engine**: a person-controlled semantic graph + governance
VM + in-process LLM, written in **Rust + WGSL (wgpu)**, that runs on hardware ordinary people own.
The mission is human-centric: the system serves the person and is bound by human-ratified values, not
the other way round. It is **not** an LLM wrapper — it has its own weight format, kernels, and
governance substrate.

**Non-negotiable constraints (every design + every external suggestion must respect these):**
- **No external inference/runtime libraries** — no llama.cpp / ggml / ONNXRuntime / PyTorch / Ollama
  at runtime. Pure in-process Rust + WGSL. (Architectural *and* governance reasons.)
- **Affordability** — must run on consumer/low-power hardware (dev card: NVIDIA A2000, 12 GB). No
  "rent a datacenter GPU" answers; the value is sovereignty/provenance/locality, not raw throughput.
- **Zero-heap / fixed-budget hot paths** — built around a 48-byte record (`NQuin`) and bounded
  arenas; the LLM weight mmap is the one sanctioned large allocation.
- **Governance: machine proposes, human ratifies** — automated systems may propose; only a
  cryptographically-signed human action ratifies a binding norm (the Curation Prime Directive).
  Wisdom (the final "ought") stays out-of-band, with the person (DIKW). Hot-path enforcement may apply
  only **attested** baselines, never machine-authored ones.
- **Authorship** — the human is the author; AI tooling is an instrument, never credited as author.

---

## 1. Architecture at a glance

- **Runtime:** single Rust process; the graph engine and the LLM share **one wgpu device**.
- **GPU:** `wgpu` 0.20 — native DX12 (Windows) / Metal / Vulkan, and **WebGPU** for the `wasm32`
  browser build. Same WGSL everywhere. NPU bridges (DirectML/CoreML/NNAPI) partial.
- **The record:** `NQuin` — 48 bytes, six `u64` `[subject, predicate, object, context, metadata,
  parity]`. Everything (semantic data, weight pointers, governance) is bit-packed into this.
- **Weight format:** `.q42` / `Q42W` — header → tensor manifest (role, layer, ggml-type, dims, blob
  offset/len, CRC) → page-aligned blobs; front-of-file lexicon (`Q42LEX`) + block index for a
  two-step range fetch. Memory-mapped (`memmap2`); the OS demand-pages disk→RAM.
- **GPU residency:** two modes — **resident** (all weights uploaded once; fastest; needs fit in VRAM)
  and **streaming** (one layer in VRAM at a time; runs models > VRAM at a PCIe cost). On a 12 GB
  A2000, compression is also a *residency* lever (a compressed 7B can move from streaming→resident).
- **Targets:** native desktop/mobile + `wasm32` WebGPU (primary web); `wasm64` decided, unbuilt.

---

## 2. Status by workstream

### 2.1 Renderer / engine — Phases 1–6 complete
The renderer arc is done and test-verified (native `cargo test --lib` ≈ **1233 passing / 0 failing**).
- **Phase 1** — world-space 3D scene (live WebGPU viewport in `spatial.html`).
- **Phase 2** — physics of artefacts (bbox admission, kinematic joints, deterministic refusal); made
  visible in the viewport; fixed an elapsed-time joint bug.
- **Phase 3** — place/space/time: one artefact NQuin queried by **both** the spatio-temporal modality
  (RCC-8 / Allen) **and** the deontic modality (`render/place_time.rs`).
- **Phase 4** — sense path: mic PCM → forward DFT → `∫Ψ>τ→Fact` bridge → a discrete Fact NQuin, under
  a fail-closed consent gate (`render/sense.rs`).
- **Phase 5** — authoring vocabulary (`render/authoring.rs`): a qapp declares 3D + 2D views over **one
  manifold**; the planner enforces attestation gates, rights-bounded contexts, and budget-driven
  3D→2D degradation (the affordability rail, shown in the viewer).
- **Phase 6** — model-as-substrate (`render/model_substrate.rs`): one buffer holds a renderable
  manifold + transcoded weights; the renderer projects the manifold while the weights are co-resident.
- **Pending:** Phase 0.2b (lift `render/` into a standalone `qualia-render` crate); a small
  optimization backlog (bloom composite, DPR resize).

### 2.2 LLM performance / STELLAR §A (the current active focus)
Compression-led, because **decode is memory-bandwidth-bound** (each token streams ~all weights).

**Done + measured (on the A2000, real):**
- **BitNet-1.58b ternary codec** (`ternary.rs`) — quantize + base-3 (1.6-bit) *and* 2-bit packings;
  zero-heap dequant.
- **GPU kernels** — `ternary_gemm.wgsl` (base-3) + `ternary_gemm_2bit.wgsl` (branchless) + an
  `f16_gemv.wgsl` baseline; native dispatch + **on-device parity verified on the A2000**.
- **Transcode** — safetensor → Q42W (verbatim / ternary / FFN-policy) and a **runnable** GGUF →
  ternary-FFN container (`compile_gguf_to_q42_ternary_ffn`) that preserves hyperparams + tokenizer.
- **Name→role policy** (`tensor_roles.rs`) — GGUF + HF names → engine roles; ternary the FFN only.
- **Kernel benchmark (4096² batch-1 GEMV, persistent pipeline):**

  | Kernel | ms/dispatch | vs F16 |
  |---|---|---|
  | F16 baseline | 0.963 | 1.00× |
  | ternary **base-3** (int `/3`,`%3` + branch) | 1.140 | **0.85× — slower than F16** |
  | ternary **2-bit branchless** (shift/mask + FMA) | **0.544** | **1.77× vs F16, 2.10× vs base-3** |

  Findings: (1) **base-3 on GPU is a net loss** (unpack costs more than the bandwidth saved);
  (2) **2-bit branchless is the win**; (3) it's only 1.77×, not the ~8× the byte-ratio implies → the
  naive one-output-per-thread GEMV **isn't bandwidth-bound yet** → tiled/coalesced GEMV is the next
  lever. *Key reframe:* on GPU the multiply is a free FMA — ternary's GPU win is **bandwidth +
  occupancy**, not "no-multiply" (that's a CPU/ASIC argument).
- **Real-model transcode (measured):** SmolLM2-360M BF16 723.7 MB → 302.5 MB (2.39×, FFN ~10×);
  Q8 GGUF 386.4 MB → 185.2 MB (2.09×, runnable: 32 layers / n_embd 960 / vocab 49152 / 1.3 MB
  tokenizer preserved).

**Pending (the §A roadmap, in consensus order):**
- **FFN-loop splice** — route ternary FFN tensors through the 2-bit kernel inside the live decode loop
  (`gguf_bridge::encode_fused_ffn_expansion`), persistent pipeline + buffer reuse + in-loop GPU
  timestamp queries → the **end-to-end tok/s** (the real proof point; not yet measured).
- **GPU top-k reduction** — argmax/top-k on-device, read back only top-k `(id, prob)` (~hundreds of
  bytes) instead of the 196 KB full-logit readback per token; preserves the Phase-8 sentinel's view.
- **Tiled/coalesced GEMV + subgroup reductions** (decode), register-blocked tiled GEMM (prefill).
- **KIVI KV-cache** (2-bit K / 4-bit V), **W4A4+AWQ** (attention), **speculative decoding**.
- **Double-buffered streaming** (ping-pong upload of layer N+1 while computing N) for > VRAM models.
- **TTFT:** pre-built pipelines, pre-compiled `.q42` distribution, lazy first-layer upload.
- **No clean native F16/Q8 baseline tok/s yet** (the only end-to-end number, ~5.9 tok/s SmolLM2-360M,
  is old + browser/WASM).

### 2.3 Semantic & governance layer
- **CML (Context Markup Language)** — a concept **is** a context hash (`q_hash(IRI)` → NQuin `context`
  field); TEXT→CONCEPT→LOGIC three-layer model; `cml.n3` axioms; the Curation Prime Directive enforced
  by a SHACL firewall. CML/CMLD draft standards rewritten to the implemented design.
- **Deontic VM** — `compile_norm_quin` / `evaluate_deontic_contract`, masked on the concept's context
  hash; the values-credentials corpus (101 instruments → 3,518 concepts) drives it.
- **`neuro_symbolic_sieve.rs`** — a **zero-heap, FSM grammar/token-mask sieve applied during the
  chunked argmax**, binding `token_id → Q42LEX hash`, built from the mmap'd `.q42.lex` + tokenizer.
  *This is the embryonic form of "semantic control in the primitive handler"* (see §4).
- **Namespace migrated** `ns.webcivics.org → ns.webcivics.net` (now live) across 249 files;
  regenerated + all gates green. Published schema-definition docs at `sdo.webcivics.org`.

### 2.4 Multi-modality (honest current state)
- **Generative VLM: not present.** The LLM path is **text-only**. "Vision" is a filename heuristic
  (`mmproj` → a tensor tag) + the inference bridge ignores non-LLM tensors. No vision encoder.
- **Substrate + sense: multi-modal by design** — modality-flagged tensors, the 10D tensor
  (spatial/temporal/spectral), the audio sense path (Phase 4), the renderer's 3D/2D/spectral views.
- **Decisions made:** spatial/geometry tensors stay **high-fidelity F16** (never ternarised —
  coordinate collapse); **analyze before generate**; **CML consent gates** for capability/hardware
  shifts (reuse Phase-4/5 gates). True multimodal generation (a vision tower → the LLM) is roadmap.

---

## 3. Models on hand (downloaded, gitignored)
- `SmolLM2-360M-Instruct` (BF16, 694 MB) — verified config; transcode validated.
- `Qwen2.5-1.5B-Instruct` (2.9 GB) — second arch, fits resident.
- `Qwen2.5-7B-Instruct` (7.0 GB BF16 ≈ 14 GB → **exceeds 12 GB VRAM** → the streaming + ternary-fits
  demonstrator).
- Plus existing GGUFs: `smollm2-360m-instruct-q8_0.gguf` (used for the runnable ternary container).

---

## 4. The neuro-symbolic question: ontology in the primitive handler
A live design thread (with external review) concluded: **yes, pursue it — but as a *compiled control
surface*, not live ontology reasoning in the hot loop.** Crucially, a **primitive form already
exists** (`neuro_symbolic_sieve`: a zero-heap `token_id → lexicon-hash` mask applied at argmax). The
proposal is to **generalise it**:
- Extend the sieve's per-token slot from a binary allow-mask to a compiled
  `token/concept → {bias, veto, route_flags, concept_id, deontic_flags}` array, sourced from
  **attested** ontology shards (`.q42lex`), hot-swappable via a CML capability gate.
- **Layering discipline (load-bearing):** hot path = `O(1)` bias/mask/route lookup only; rich
  span→concept resolution (`skos:closeMatch`) is **cold/async** (a sliding-window sieve); trigger
  nodes hand off to **logic-engine extensions** (the 14 modalities / CAS / specialized libs / WASM)
  **off** the critical loop.
- **Governance rail:** the hot-path surface enforces only `cml:Attested` baselines (Curation Prime
  Directive) — it applies *encoded prior human wisdom*, never authors new norms.
- **Why it helps:** speed (avoid wasted generation + full-logit readback; `O(1)` checks), accuracy
  (deterministic fallback to real logic extensions where probabilistic text fails), flexibility
  (compiled domain ontologies — SNOMED/ISO — as swappable shards). It does **not** speed the GEMMs.
- **Caveat:** a BPE token is not a concept; single-token entries are good for anchors/control/domain
  terms, general meaning needs spans. This is the END-STATE; it plugs into the top-k step, so it comes
  *after* the top-k reduction + FFN splice.

---

## 5. The open decision — the implementation path (for reviewer input)
The consensus-synthesised sequence (see `PERFORMANCE_FEEDBACK_COMPILATION.md`) is: brief ✓ →
kernel-level measurement ✓ → **FFN-loop splice** → measure → then GEMV tiling / KIVI / streaming.
The genuine choices to weigh:

1. **FFN-loop splice first** (reviewer #1): edits the live decode loop → the real end-to-end ternary
   tok/s. Highest-value, highest-risk; the prerequisite (a runnable ternary model) is now built.
2. **GPU top-k reduction**: immediate, lower-risk TPS win (kills the 196 KB/token readback) *and* the
   seam the sentinel + ontology-control-surface plug into. Arguably do this *with* the splice.
3. **Tiled/coalesced GEMV** before the splice: the benchmark shows ~4.5× headroom in the kernel
   itself; low-risk, fully measurable now — but doesn't give an end-to-end number on its own.
4. **The compiled ontology control surface** (generalise `neuro_symbolic_sieve`): the neuro-symbolic
   endgame; depends on the top-k step existing first.

**The question for reviewers:** given decode is memory-bound and the kernel is *not yet*
bandwidth-bound (1.77×, not ~8×), what is the right *first* implementation step — the live splice (for
a real number), the top-k reduction (de-risk + the integration seam), or the tiled GEMV (close the
kernel's own headroom)? And how aggressively to pursue the compiled ontology surface vs. landing raw
TPS first?

---

## 6. Honest gaps (what is NOT done / not measured)
- **No end-to-end ternary tok/s** — the kernel is fast in isolation; not yet in the live loop.
- **No fresh native baseline tok/s** for a full F16/Q8 model (only an old browser number).
- **No TTFT measurements** (cold/warm) captured.
- **Ternary inference on a real model not yet run** (the runnable container exists; the dispatch
  branch does not).
- **No multimodal generation** (text-only LLM today).
- **`wasm64`, mobile parity, NPU paths** — designed/partial, not release-verified.
- The kernel benchmark is a standalone GEMV, not the live decode path.

---

## 7. Pointers
- **Detailed docs:** `PERFORMANCE_BRIEF.md` (perf deep-dive + reviewer questions),
  `PERFORMANCE_FEEDBACK_COMPILATION.md` (consolidated external feedback + experiment table),
  `STELLAR_MISSION.md` (roadmap, §A), `RENDERER_IMPLEMENTATION_PLAN.md` (renderer phases),
  `docs/manuals/standards/.../cml`, `.../CMLD` (the CML/CMLD standards),
  `core-ontologies/CML_CONCEPT_GRAPH.md` (the concept-graph architecture).
- **Key source:** `crates/qualia-core-db/src/` — `ternary.rs`, `ternary_gpu.rs`,
  `shaders/{ternary_gemm,ternary_gemm_2bit,f16_gemv}.wgsl`, `q42_weight.rs`, `safetensor.rs`,
  `tensor_roles.rs`, `neuro_symbolic_sieve.rs`, `gguf_bridge.rs`
  (`dispatch_transformer_forward` / `encode_fused_ffn_expansion`), `gpu_context.rs`,
  `modalities/logic/deontic.rs`, `render/`.
- **Verification:** native `cargo test -p qualia-core-db --lib`; on-device kernel tests in
  `ternary_gpu.rs` (run on a machine with a GPU); real-model transcode in
  `tests/transcode_real_model.rs` (needs the gitignored models).
