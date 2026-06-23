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
| D18 | **Feature toggles + per-phase measurement attribution** — every optimization behind a runtime flag (`QUALIA_LLM_TERNARY_FFN` / `_GPU_TOPK` / `_TILED_GEMV` / `_SIEVE`); each phase reports an independent before/after row | bundling changes blurs attribution + makes regressions undebuggable (Codex). |
| D19 | **Ontology shards = dynamically composable, layered priority stack** — L0 Core (attested, never overridable) · L1 Domain (`<cml:load_ontology>`) · L2 User/Task; **top-down, first-hit wins** | sovereign multi-domain adaptation; resolves Open Q3 (Gemini/Grok). |
| D20 | **Quality gate = a defined trinity** — perplexity Δ on a fixed corpus **+** a 20–50-prompt governance/persona suite **+** sentinel/refusal regression; the metric is **defined**, not a bare "<1%" | perplexity alone can't see deontic drift in a sovereign system; resolves Open Q4 (all). |
| D21 | **Compiled control record = fixed-size, mmap-safe, sorted** (`token/concept → {bias_q8, flags, route_id, concept_id, attestation}`; flags incl. `SOFT_BIAS±` / `HARD_VETO` / `REQUIRES_CONSENT` / `ROUTE_TO_LOGIC` / `SENSITIVE_DOMAIN` / `ATTESTED`); **only `ATTESTED` entries may hard-veto or route in the hot path** | Codex's ABI + the governance rail made mechanical. |
| D16+ | **Zero-copy hand-off contract** between the LLM substrate and the renderer (explicit VRAM buffer ownership/lifetime) to prevent impedance mismatch | Grok refinement to D16. |
| D22 | **Parallel-path adoption + shared-improvement rule** (Timothy, R2). Every optimization lands as an **additive, toggle-selectable path** that leaves the existing F16/Q8 path **fully runnable**; the old path is **deprecated only after** the new one wins on the §8 attribution matrix **and** the D20 quality gate. Where a change improves the **common/shared** layer (instrumentation, residency, mmap, bug fixes), apply it **once to the shared code**, not forked per path. | Retain-then-deprecate de-risks; "if updates improve both, do that" avoids divergence/drift. Generalises D7 (additive branch) + D18 (toggles). |
| D23 | **Sense-then-route boot** (Timothy, R3). At startup, **enumerate ALL wgpu adapters**, classify topology by `device_type` (`DiscreteGpu` → discrete; `IntegratedGpu`/`Cpu` → unified), pull host RAM via `sysinfo`, and **select an execution protocol** from (topology, model-header bytes, VRAM/RAM budget) **before** mapping weights or compiling pipelines. Replaces today's single `PowerPreference::HighPerformance` pick (which silently grabs only the A2000 and ignores the iGPU + 64 GB). | A static heuristic can't span a 12 GB discrete card, a 64 GB-backed iGPU, and a unified phone. Compile only the pipelines the chosen protocol needs (TTFT). |
| D24 | **Bounded floors, never user-configured** (Timothy, R3). Reserve a hard OS floor (discrete ≈ 1.5 GB VRAM display floor; unified ≈ 4–6 GB host floor); the LLM mmap + KV cache inhabit the remainder. KV cache always in the fastest pool, **capped** (already 448 MB in `VramLedger`). Under pressure **degrade / refuse**, never crash the host. | Extends the existing `OperationalMode` Full/Eco/Reserve + `UniverseOrchestrator` pins. Manual memory config = OOM crashes + fragile UX. |
| D25 | **Three residency protocols, chosen by fit** (Timothy, R3). **Resident** (fits VRAM → upload once; on unified → mappable, no duplicate copy). **Streaming_PingPong** (exceeds VRAM → A4 double-buffer over PCIe). **Heterogeneous_Overflow** (discrete **+** iGPU → overflow layers execute on the iGPU reading system RAM, only the small activation crosses PCIe) — **adopted only when a micro-benchmark shows `iGPU_compute < PCIe_copy + dGPU_path`**. The iGPU is an *overflow home*, **not** a co-equal engine (it is far slower at GEMM). | Uses the silicon the box already has instead of streaming overflow from disk. Honest: cross-adapter handoff in wgpu is a host-mediated copy, not zero-copy. |
| D26 | **Boot micro-probe + cached "hardware passport"** (Timothy, R3). A ~50 ms dummy-GEMM per adapter measures compute + PCIe transfer; results + the chosen routing are cached to disk (where permitted), keyed by the discovered adapter **identifiers** (vendor/device handles, not an "identity"); later boots **skip the probe** (TTFT, A7). WASM/sandbox → OPFS/IndexedDB or skip-and-reprobe. **Caveat:** adapter UUIDs aren't stable across driver updates → validate, reprobe on mismatch. | "Probe once, cache, fast-boot" beats probing every launch. The passport is a *capacity manifest*, not an identity (D27). |
| D27 | **Attestation: human-key root; hardware enclave is ONE witness, never the root** (Timothy, R3 — the load-bearing call). The passport is a **signed, mutable capacity claim** signed by the **human's sovereign key** (software keys we already have: ML-DSA-65 / Ed25519 / `key_vault`). TPM / Secure-Enclave attestation, *if ever added*, is an **optional confidence-raising witness** treated as a low-trust, manufacturer-chained claim — **never** the root of trust. Peers extend trust **relationally** (guardianship→delegation), not because a silicon vendor vouched. | TPM-as-root chains to Intel/Apple/MS CAs = reintroduces the corporate locus-of-control + a definitive-identity capture surface — the exact 2001-DRM topology rejected. Aligns [[principle-governance-topology-relational]] · [[principle-identifiers-not-identity]] · [[principle-out-of-band-remainder-is-freedom]]. **⚠ "Root of trust" = KEY-CONTROL, not identity-definition** — a key the human *controls* signs the manifest; identity stays a relational enumeration over many identifiers toward an out-of-band user (confidence never 1.0). Saying "the key = the person" would itself be the capture default. **⚠ Depends on the identity/key substrate, which is mid-remediation** (`identity-governance-remediation.md`): Finding A (identity collapsed to one handle at conf 1.0), Finding D (`orchestrator.rs:366` signs conduct with forged `[42u8;32]`; rights rule self-reports), Finding C (`did:q42` not a real DID method). **H1(b) signing + H5 cross-node trust are blocked on that doc's Phase 2 (enumeration) + Phase 3 (did:q42 verification-method, real rights gate).** Build H1(a)/H2/H3 (local routing, no trust) now; do not build the trust layer until then. |
| D29 | **Native distributed cluster inference across networked unified-memory nodes** (Timothy, R3 — e.g. the "many Mac Minis loading one big model" pattern). Pipeline/tensor-parallel **shard a model across ≥2 nodes**; only the small activation crosses the **network** (`libp2p` — already a dep: tcp/noise/yamux/request-response/kad/mdns); each node advertises capacity via its **signed passport** (H1); layers assigned by measured capacity; per-node zero-heap + residency planner (H0–H2/H4). | The network-scale form of the compute-cell / fractal-sharding swarm ([[project-compute-cell-model]]) and the Walkabout/regional-hub vision. Affordability: cluster cheap unified nodes vs one datacenter box. Trust is **relational + attested** (D27) — so it inherits D27's human-key-root dependency. |
| D28 | **Unified memory is *different*, not just smaller** (Timothy, R3). No PCIe: the win is avoiding the duplicate copy (mappable buffers where the backend allows; wgpu can't do literal mmap→kernel) **+** host-protective floors (D24), not bus-splitting. Hardware-topology change surfacing: **local capability** change → `<cml:capability_shift>` consent gate; **network/identity-affecting** change → human-key **re-delegation** (higher ratification). | Perf gains are topology-shaped: discrete desktop ← heterogeneous overflow + resident; unified (Mac/phone) ← no-duplicate + floors. The local-vs-relational split mirrors the DIKW out-of-band rule. |
| D30 | **Benchmark-driven device prioritisation — a measured capability matrix, NOT a static device-type hierarchy** (Timothy, R3). The boot probe benchmarks **every available compute circuit — CPU (incl. AVX-512 where present), iGPU, GPU, and NPU if available** — on a representative GEMV/GEMM (+ transfer cost), and the planner ranks devices by **measured throughput**, routing accordingly. No hardcoded "GPU > CPU". | "Fallback" is the wrong frame (Timothy) — each circuit is first-class, ranked by data: a weak iGPU may still win for overflow vs PCIe streaming; a strong AVX-512 CPU may beat an old iGPU. NPU access is platform-API (DirectML / NNAPI / CoreML), **not** wgpu — detect + benchmark where the backend exists, else mark unavailable (honest). |
| D31 | **Load/employment = a discovery→adaptive-plan process, NOT a fixed formula** (Timothy, R3). It is *not* a rigid "three-axis rule". Step 1 **discover** what hardware is actually present (H0 enumerate + H1 benchmark) — the set varies wildly: NPU or none, unified or discrete, one GPU or many. Step 2 **figure out how best to employ *that specific* machine.** Discovery yields the inputs (measured compute, memory-pool location/capacity, transfer/load cost); the planner then **adapts** per machine — fit resident on the best-measured circuit; place overflow by **argmin of *measured* cost** over whatever circuits are present (`{iGPU-in-place, stream-to-fast, CPU, NPU, …}`); order loads **immediate-need-first** for TTFT (prefill compute-bound vs decode bandwidth-bound). No hardcoded hierarchy, no assumption a given circuit exists. | A fixed formula is wrong because hardware is heterogeneous — the plan must be **derived per machine from discovery**, not assumed. "Fastest device wins" also fails (fastest device = smallest pool). Stream-vs-in-place crossover is data-dependent → H1 must measure transfer, not just compute. |
| D32 | **Discovery generalizes from compute to ALL device capabilities — incl. sensors; governance LEADS** (Timothy, R3). The discovery→adaptive-employment pattern (D31) extends beyond compute circuits to **sensing capabilities**: smartwatch biometrics (pulse) and **camera-derived vitals** (rPPG / Eulerian video magnification — heart-rate from skin-colour change), giving signals a device lacks dedicated sensors for. Each sensor is one more **consent-gated capability/identifier**. **Mandatory rails (non-negotiable):** per-capability **consent gate** (Phase-4 `sense.rs`, fail-closed), **on-device**, **analyze-before-generate**, bound by medical-agency + human-rights instruments; carry **uncertainty + provenance**; **signal, never diagnosis** (DIKW — clinical judgment stays with the human). **Biometrics are the most capture-prone identifiers** → strongest protection, contribute to the confidence-relation only by explicit consent, **never definitive** (out-of-band remainder holds hardest). | Same rPPG capability is dual-use (prosthetic vs covert surveillance — governance-topology-decides); the ICT-as-prosthetic lens. **Honest limits:** rPPG is noisy + has a **skin-tone accuracy bias** (first-order dignity/fairness concern, must be surfaced not hidden); it's an estimate. Lands as a sensor-discovery sibling to H0 + new `sense.rs` modalities → NQuin facts (consent/provenance/confidence); analysis via `medical_computing`. **Forward/aspirational — recorded, not built;** the consent/medical-agency content is Timothy's to define. |
| D33 | **The unifying frame is the HUMAN (mind/attention/agency), NOT hardware** (Timothy, R3 — the deepest reframe). Hardware "unified memory" is a *local* property of one node; the real unification is the human mind, which binds a heterogeneous, scattered **personal device fabric** (phone, watch, laptop, desktop, …) into one agentic surface. So discovery/employment (D31/D32) scopes to **the human's device fabric**: marshal *all* the human's devices — subject to **consent/ownership, availability, performance** — on their behalf (devices = agents acting for the principal; the human = the observer-frame, task #15). **Attention is graded-in-band toward a WHOLLY-out-of-band locus** (refines [[principle-out-of-band-remainder-is-freedom]]): the mind/judgment is wholly out-of-band (the freedom); device-activity/focus is a partial in-band projection at a confidence (rule 3 of `identity-governance-remediation.md`, extended from identity to attention). → the engine can **follow attention** (surface results where the human is; run heavy passes on idle devices — Sleep-Cycle Swarm; affordability: human pays only the cheap on-device fold). | Each device = an **identifier/capability**; the human = the relational structure over them, never one device (identifiers-not-identity made physical). **Dependency:** binding several devices to *the same out-of-band human* needs the relational identifier-enumeration (remediation Phase 2) — so the personal-fabric scheduler inherits that gate (same as H1(b)/H5), via the *same* enumeration algorithm. **Governance caution:** seeing+using all your devices is mechanically the panopticon topology — human-as-frame + per-device consent + out-of-band remainder is what makes it a prosthetic fabric, not surveillance (governance-topology-decides). |

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

### A1 — the live decode wins *(the main event — split for clean attribution, per Codex)*
Build both, but as **independently-toggled, separately-measured** steps so a speedup (or regression)
is attributable. Drive with the **runnable ternary container** (`compile_gguf_to_q42_ternary_ffn`,
already built) on the q8 SmolLM2-360M.

**A1a — GPU top-k reduction** *(lower risk; do first; it's also the governance seam)*
- **Build:** `fused_top_k_reduction.wgsl` — **per-workgroup local top-K → merge** (no full-vocab
  bitonic sort); read back only `(token_id, logit)` pairs; CPU does temperature/top-p/softmax over
  the K candidates. K=32/64; argmax (K=1) as a cheap special mode; NaN→−∞; deterministic tie-break.
  First version returns `flags=0, concept_id=0` (no coupling to the unfinished ontology surface).
- **Pass/fail:** logit readback drops from ~196 KB/token to K pairs; K=1 result == CPU argmax;
  masked token never returned; decode tok/s vs A0 baseline (isolates the readback win).

**A1b — FFN-loop ternary splice** *(the main proof point)*
- **Build:** persistent ternary pipeline created **once** in the bridge struct; branch
  `encode_fused_ffn_expansion` on `ggml_type == TERNARY` → bind 2-bit weights → 2-bit branchless
  kernel; **F16/Q8 fallback untouched**; fail-to-bind errors explicitly (never silent). Scale verified
  vs the CPU oracle on **real SmolLM2/Qwen FFN shapes** (not only the 4096² benchmark).
- **Pass/fail (the MVPP):** real **end-to-end ternary decode tok/s vs the A0 F16/Q8 baseline** on the
  A2000, measured **top-k off, then on** (the attribution matrix in §8); output sane on a fixed prompt
  set; existing path unregressed. **Do not extrapolate the 1.77× kernel win to end-to-end — measure it.**

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

### A6 — Compiled ontology control surface *(neuro-symbolic endgame — split A6a/A6b, unanimous)*
Generalise `neuro_symbolic_sieve` into a compiled control surface, in **three layers** (Codex):
Layer 1 hot-path `O(1)` control (per token / top-k candidate); Layer 2 async sliding-window
span→concept resolution; Layer 3 logic-extension routing only at safe boundaries (phrase/sentence/CML
gate). The BPE-token↔concept bridge is a **span trie / FSM phrase matcher over `Q42LEX` labels**,
resolved at word/phrase boundaries — *not* per-token (a token is an anchor, not a concept).

**A6a — bias/veto masks at the top-k step** *(do first)*
- **Build:** the compiled `.q42lex` control records (D21), applied as `O(1)` bias/veto during the
  top-k reduction; **attested-only** enforcement (Curation Prime Directive); **graded influence**
  (hard veto / soft bias / confidence-weighted) — the default, never binary on/off.
- **First concrete shard (Codex, R2):** a **reserved control-token or a single narrow attested span**,
  **not** natural-language BPE-token hard vetoes (BPE token≠concept → brittle false positives). Proves
  the mechanism (attested shard → compiled mask → hot-path enforcement) at minimal generative-quality
  risk. The span *content* is **human-attested** (e.g. a guard around "simulate/override human
  ratification") — proposed, not pre-decided here.
- **Pass/fail (explicit criterion):** an **attested** hard-veto masks a candidate in **< X µs/token
  with zero measurable decode-TPS impact**; a **machine-proposed** mapping **cannot** hard-veto or
  route (governance test); generative diversity not collapsed (graded influence verified).

**A6b — trigger-node → logic-extension handoff** *(only after A6a is stable)*
- **Build:** `ROUTE_TO_LOGIC` nodes hand off context to a logic extension (CAS / deontic VM / SHACL /
  specialized lib / WASM) **at safe boundaries, off the GPU loop**; deterministic result injected back.
- **Pass/fail:** a routed concept yields a deterministic extension result injected into context with
  **no per-token TPS hit**; the GPU decode loop is never interrupted per token.

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

### AH — Adaptive-hardware / memory-routing track *(addendum, 2026-06-23; topology-aware; feeds A4 + A7)*
The performance ceiling differs by silicon. This track makes the engine **sense the host and route**
instead of grabbing one adapter. **Affordability rail:** it only ever uses the hardware the user
*already has* better — never requires more. Gains are topology-shaped (D28): a discrete desktop with a
big-RAM iGPU gains most (heterogeneous overflow + resident); a unified Mac/phone gains from
no-duplicate mapping + host floors; a single small GPU still benefits from the resident-vs-stream
decision + the cached fast-boot.

**H0 — Host topology + capability sensor** *(buildable now; foundation)*
- **Build:** enumerate all wgpu adapters; classify `device_type` (discrete vs unified); read host RAM
  (`sysinfo`) + per-adapter memory (reuse `directml_bridge::probe_best_adapter_memory`). Emit a
  `HostTopology { adapters[], unified, host_ram, vram_budget, os_floor }`. Extends `gpu_context`.
- **Pass/fail:** correctly classifies the dev box (A2000 *discrete* + Intel *iGPU* + 64 GB) and a
  unified target; **enumerates the iGPU that is invisible today**.

**H1 — Multi-circuit micro-probe + cached passport** *((a) buildable now; (b) gated)*
- **Build (a) — buildable now:** per compute circuit — CPU (incl. AVX-512 where present), iGPU, GPU,
  NPU-if-present — measure **two axes**: **(i) compute** (representative GEMV — ✅ DONE,
  `device_benchmark.rs`) and **(ii) host→device transfer/upload cost** (PCIe for discrete; staging path
  for iGPU — wgpu can't show true zero-copy, so a relative signal — *the remaining piece, needed for the
  D31 stream-vs-in-place crossover*). Produce a **capability matrix ranked by measured throughput**
  (D30). Cache `HostTopology` + the matrix; fast-boot skips the probe when the adapter set matches
  (*also remaining*). WASM → OPFS/IndexedDB. NPU = platform-API, benchmarked only where it exists, else
  "unavailable".
- **Build (b) — GATED:** sign the passport with the **human's sovereign key** (D27). **Blocked on the
  human-key-root issue** (`identity-governance-remediation.md`); build (a) and stop before (b).
- **Pass/fail (a):** the matrix ranks the circuits by *measured* GEMV on the dev box (expect A2000 >
  iGPU > CPU; NPU unavailable here); 2nd boot skips the probe; topology change → `<cml:capability_shift>`
  re-probe (D28).

**H2 — Residency + device-priority planner** *(buildable now; the routing brain)*
- **Build:** apply the **discovery-derived employment plan (D31)** from (discovered topology,
  model-header bytes, budget − floors, H1 compute+transfer matrix) — adapt to whatever hardware is
  present: **resident-on-highest-ranked-device** for everything that fits; for the
  **overflow**, choose **argmin of measured cost** over `{iGPU-in-place, stream-to-fast-device, CPU}`
  (= D25's Resident / Streaming_PingPong / Heterogeneous_Overflow); **load order immediate-need-first**
  for TTFT; weight axes by phase (prefill compute-bound vs decode bandwidth-bound). Expose the choice to
  the decode path (today it just adopts a mmap with no strategy). Behind `QUALIA_LLM_ROUTE` for A/B.
- **Pass/fail:** a model that fits → Resident; one that exceeds VRAM → Streaming (stub until A4) or
  Heterogeneous on the dev box; decision logged + measured via the A0 harness.

**H3 — Heterogeneous overflow dispatch (discrete + iGPU)** *(larger; depends on H2)*
- **Build:** multi-`Device` pool `[A2000, iGPU]`; overflow layers run on the iGPU (system-RAM
  resident); host-mediated activation handoff; **gated by the H1 micro-benchmark** (`iGPU_compute <
  PCIe_copy + dGPU_path`), else fall back to streaming/CPU.
- **Pass/fail:** a >12 GB model runs across A2000 + iGPU+RAM, **measured against disk-streaming** — keep
  only if faster (the D25 gate).

**H4 — Unified-memory path (Mac-Metal / phone)** *(deferred — needs the hardware to verify)*
- **Build:** mappable-buffer no-duplicate residency + strict host floors (D24/D28); no PCIe assumptions.
- **Pass/fail:** on a unified target, model + KV inhabit the shared pool under a hard OS floor without
  host thrash. *Marked honestly as untestable here until a unified device is available.*

> **Dependencies:** H0 → H1 → H2; **H1 feeds A7** (cached fast-boot TTFT); **H2 feeds A4** (it picks
> when streaming is even needed). H3 needs H2 + multi-device capacity. H0–H2 land on the dev box now;
> H3 is larger; H4 awaits hardware.

**H5 — Distributed cluster inference (networked unified nodes)** *(D29; larger/later; aspirational-but-aligned)*
- **Build:** node discovery (`libp2p` mdns/kad — already a dep); a **pipeline-parallel scheduler** that
  assigns layer ranges to nodes by **passport capacity** (H1); activation transport over `libp2p`
  request-response; per-node residency via H2/H4. The "many Mac Minis → one big model" pattern, native.
- **Pass/fail:** a model too big for any single node runs across **≥2 nodes**; output == single-node;
  network-activation latency hidden where the pipeline allows.
- **Honest scope:** needs ≥2 unified nodes to verify (defer like H4); the scheduler + transport are
  substantial; the **trust layer inherits D27's human-key-root dependency** (don't ship cross-node
  attestation until that separate issue is resolved). Builds on H0–H2 + H4 + the passport.

## 4. Benchmark matrix (to populate as phases land)

| Mode | Backend | Model | Weight policy | Residency | TTFT cold | TTFT warm | Prefill t/s | Decode t/s |
|---|---|---|---|---|--:|--:|--:|--:|
| Baseline | DX12 A2000 | SmolLM2-360M | F16/BF16 | resident | — | — | — | — |
| **Baseline (A0, 2026-06-23)** | DX12 A2000 | SmolLM2-360M | **Q8** | resident | **2729** | **1100** | n/m¹ | **1.52** |
| Baseline (A0) | DX12 A2000 | SmolLM2-360M | Q4_K_M | resident | 1460 | 1229 | n/m¹ | 1.11 |
| Ternary | DX12 A2000 | SmolLM2-360M | FFN 2-bit / attn F16 | resident | — | — | — | — |
| Ternary+tiled | DX12 A2000 | SmolLM2-360M | FFN 2-bit, tiled GEMV | resident | — | — | — | — |
| Streaming | DX12 A2000 | Qwen2.5-7B | BF16 | streaming | — | — | — | — |
| Streaming DB | DX12 A2000 | Qwen2.5-7B | mixed | double-buffer | — | — | — | — |
| Browser | WebGPU | SmolLM2-360M | current | resident | — | — | — | refresh 5.9 |
| Heterogeneous (H3) | A2000 + Intel iGPU | Qwen2.5-7B | overflow→iGPU/RAM | split | — | — | — | — |
| Unified (H4) | Metal/iGPU | SmolLM2-360M | mappable no-dup | unified | — | — | — | — |

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
- **Eval-set format (Codex, R2):** JSONL rows `{id, category, prompt, expected_behavior, hard_gate}`;
  `hard_gate=true` rows (sentinel/refusal canaries) **must** stay binary-pass after any change;
  perplexity Δ is *soft evidence only*. Corpus + prompt content is **human-curated** (D20), not derived
  from any corpus-as-authority.
- **Keep/reject rule (Codex, R2 — proposed default, pending ratification):** keep a change only if it
  yields **≥5–10% end-to-end decode improvement OR enables a better residency class**, with **zero
  hard-gate regression**; W4A4 expected to need its own (tighter) threshold. The exact numbers are
  Timothy's to set once A0 produces real baselines.
- **Ontology compilation** becomes a first-class AOT tool (like weight transcode): versioned,
  validated to preserve semantics; only the high-certainty attested fragment lives hot.

## 7. Open questions — RESOLVED by the 2026-06-23 review round (Gemini · Grok ×2 · Codex)
1. **First step → A0, then A1a (top-k) then A1b (FFN splice), then A2.** Unanimous: top-k is a
   *prerequisite for measurement* (the 196 KB/token readback would mask any kernel gain), and tiling
   (A2) comes *after* the live splice so attribution data tells us where to tile. Don't optimize the
   kernel in a vacuum.
2. **Ontology surface first pass → A6a (bias/veto masks) only; A6b (trigger→logic) after.** The
   handoff's pause/serialize/inject/consent state-machine is the risk; bias/veto proves the rail first.
3. **Shards → dynamically composable, layered priority stack** (D19): L0 Core/attested · L1 Domain ·
   L2 User-Task; top-down, first-hit wins (pointer-offset stack in the handler).
4. **Quality gate → the trinity** (D20): perplexity Δ on a fixed corpus + a 20–50-prompt
   governance/persona suite + sentinel/refusal regression. Define the metric; not bare "<1%".

## 8. Minimum viable proof point (MVPP) + attribution matrix
**MVPP:** SmolLM2-360M on the A2000, resident, **Q8 baseline vs ternary-FFN, same prompt set**,
measured **native decode tok/s + warm TTFT + output sanity**. That single comparison validates or
refutes the performance thesis. Every phase carries a **measurement-attribution** row and a
**rollback criterion**; backend-portability is checked (WebGPU subgroup/f16 support is *not* universal
— keep fallback kernels).

¹ Prefill t/s not meaningfully measured at A0 — the probe prompt is 5 tokens; A0.2 uses a long
prompt + GPU timestamp queries for a real prefill/kernel figure.

**A0 finding (2026-06-23, A2000, Q8 SmolLM2-360M, 64-token decode):** native decode is **1.52 tok/s
(~657 ms/token)** — *slower* than the browser path's 5.9 tok/s. Root-cause hypothesis (to confirm in
A1a/A2): ~33 serialized GPU round-trips/token (32 layer dispatches + a full 49 k-vocab argmax readback)
with per-token sync stalls. This is the number A1a (kill the 196 KB readback) and A2 (tiling/fewer
dispatches) must beat. Harness: `llm_bench.rs` + `tests/llm_bench_a0.rs`; artifacts in
`benchmarks/results/`.

| Build | FFN ternary | GPU top-k | Tiled GEMV | Decode tok/s | TTFT warm |
|---|:--:|:--:|:--:|--:|--:|
| Baseline (live Q8, A0) | off | off | off | **1.52** | **1100 ms** |
| Top-k only | off | on | off | — | — |
| Ternary only | on | off | off | — | — |
| Ternary + top-k (**A1 result**) | on | on | off | — | — |
| + tiled (**A2 result**) | on | on | on | — | — |

## 9. Condensed risk register (Codex)
| Risk | Sev | Mitigation |
|---|---|---|
| Bundled A1 blurs attribution | High | feature toggles + separate before/after rows (§8) |
| End-to-end gain ≪ kernel gain (other costs dominate) | High | measure baseline / top-k / splice separately; don't extrapolate 1.77× |
| Ontology hot path too rich → TPS collapse | High | hot path `O(1)` bias/mask/route only; rich resolution async (A6 layers) |
| BPE token treated as concept → false positives | High | span trie / phrase matcher; resolve at boundaries |
| Over-veto collapses diversity | Med | graded influence; hard veto only attested |
| Machine mappings become hot-path enforcement | **Critical** | enforce **attested-only** (D21) |
| WebGPU subgroup/f16 assumptions fail | Med | feature-detect; keep fallback kernels |
| W4A4 quality loss | Med | delay until the quality-gate trinity exists |
