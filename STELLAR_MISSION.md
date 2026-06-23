# QualiaDB — The Mission to Stellar (Phenomenal Outcomes)

> A comprehensive mission + roadmap update, synthesised 2026-06-19 from `advanced-q42-llm-ideas.md`,
> `qualia-llm-future-updates.md`, the draft standards suite (`docs/manuals/standards/init-draft-standards-wip-main/`),
> the current engine state, and the full philosophical stack established with Timothy Holborn.
>
> "Stellar" is not a tok/s number. It is the **phenomenal outcome**: ordinary people — especially the
> dispossessed — able to understand themselves and one another, hold their own life-record, seek justice
> and remedy, and live in peace, using science and technology as a *right* (UDHR Art 27 / ICESCR Art 15),
> on hardware they control. The performance work serves that; it is not the point.

---

## 0. North Star (the WHY every line below answers to)

The dependency chain, root to peak (see `memory/project_foundational_ontology.md`):

**interdependence → foundational supports → selfhood → personhood → custody → agency → rights → civics /
democracy / rule-of-law (over man AND bot) → access to justice & remedy → PEACE.**

Operative thesis: the ecosystem is built for *non-human* entities (devices, platforms, legal persons);
the **missing foundation is the support structure for natural human beings.** Give natural people
tech-leverage to seek justice/remedy/dignity and **the terms of trade change** — impunity loses the
asymmetry it feeds on. QualiaDB is the infrastructure that makes the *honourable* version real, as
opposed to the extractive "SSI-wallet / person-as-issued-root-key" counterfeit.

### Non-negotiable governance rails (constraints on EVERY phase, not features)
- **Deontic gate is mandatory** — "served, not consumed by artificium." Every inference is intent-gated
  pre-flight and provenance-gated post-flight (`orchestrate_inference`). Never bypass/stub.
- **Agency, not sovereignty; identifier, not identity** (`memory/feedback_terminology_agency_not_sovereign.md`).
  Meaning is resolved against the manifold + context; the token is never the self.
- **Fiction/non-fiction discipline** — provenance-typed claims; reject ungrounded output. (Today's models,
  incl. LLMs, fail this by default; CML + the deontic gate are the cure.)
- **Asymmetric accountability** — transparency UP (public power/funds), privacy DOWN (persons, esp. vulnerable).
- **Universality** — rights/support for ALL, incl. offenders/prisoners; accountability ≠ exclusion; never impunity.
- **Resilient relational identity** — a *fabric* of identifiers + social-graph recovery, NOT a single key.
- **Self-custody, never hold the user's keys.** Sensitive data lives in the credential-gated vault keyed to them.

---

## 1. Where we are now — V1 "First Light" (ACHIEVED ✅)

- **Native browser LLM**: in-process Rust→WASM+WebGPU decode at **~5.9 tok/s** (SmolLM2-360M, coherent),
  no Ollama/llama.cpp/Python. Single-submit forward, resident weights/logits/norms, parallel Q/K/V GEMM.
- **`.q42` weight container** (`Q42W` magic): 16 KB-page-aligned weight blobs + hyperparams + tokenizer,
  CRC-32C, version-gated; AOT **GGUF → .q42** compiler (`compileGgufToQ42`); **OPFS model cache**
  (`loadOrCompileQ42`, zero-parse warm boot).
- **In-browser 3D manifold renderer** (added 2026-06-23): `spatial.html` now drives the real `PortalGpu`
  WebGPU path — depth-tested 3D, Kawase bloom, 10D-tensor → node projection, orbit camera — live in Chrome,
  not the canvas2d fallback. The first concrete slice of §E (the manifold renderer). See
  `RENDERER_IMPLEMENTATION_PLAN.md` (Phases 0.1 / 0.3 / 1.0) + `memory/project-portal-gpu-dawn-strictness.md`.
- **Deontic / provenance gate** wired (intent pre-flight, provenance-citation post-flight).
- **Provenance vault** (`provenance/`, gitignored + out-of-repo for sensitive) — the first real dataset is
  Timothy's own life-record (see `memory/project_provenance_vault.md`, `project_wellfair_purpose.md`).
- **Draft standards suite** — CML, CMLD, DOA, DOE, HCAIO, DigitalBirthRecord, rights, ulem, etc.

### Honest open V1 defects (fix before calling V1 done)
1. ~~**`wgpu 0.19.4` sends `maxInterStageShaderComponents`** → `requestDevice` fails on recent Chrome.~~
   **RESOLVED 2026-06-23:** wgpu 0.19→0.20 (1160 lib tests green) + `webgpu-limits-shim.js` strips the
   removed limit before `requestDevice`; the 3D viewport renders in Chrome. (The browser-LLM prefill path —
   defect #2 — is a separate bind-group bug still open.)
2. **1B+ model prefill crash** — `dispatch_prefill_chunk` fails → legacy `dispatch_fused_transformer_block`
   binds a 32-byte `TransformerParams` against `MC8GemmBGL`'s 256-min dynamic uniform → invalid bind group.
   SmolLM2-360M is the only verified config.
3. **`.q42`/ingest stores metadata-index; full text truncated** — large literals live in `.nt`, not the `.q42`.
4. **OCR** (image-only diagrams/PDFs) — `tesseract` not installed; diagram corpus unsearchable.

---

## 2. The roadmap to Stellar (workstreams)

### A. Performance & compression — `.q42` v2 ("designed-for-qualia format")
*Goal: 3–6× TPS + 100k-token context on consumer/edge silicon, from a one-time AOT compile.*
- **Ternary (BitNet 1.58b) packing** of non-critical FFN layers (weights ∈ {-1,0,1}); keep attention at Q4.
  Eliminates FMA → hardware adds/subs in the WGSL kernels. (Reported 3–6× speedups, ~70% energy cut.)
  **✅ codec + transcode + GEMM kernel landed 2026-06-23** — `ternary.rs`: absmean-scale quantize +
  5-trits/byte base-3 pack (≈1.6 bits/weight) + zero-heap dequant; `transcode_safetensor_to_q42_ternary`
  applies it *during* transcode (valid Q42W, round-trips, ~8× smaller than F16); **`ternary_gemm.wgsl`** —
  the BitNet GEMM (weight = **add/subtract only**, one end-scale; naga-validated) with a byte-exact CPU
  oracle `ternary_gemm_cpu` (parity-tested == dense matmul of the dequantized weights). 11 tests.
  Plus **name→role mapping + policy transcode** (`tensor_roles.rs`: GGUF + HF naming → engine roles;
  `transcode_safetensor_to_q42_ffn_ternary` ternaries **only** the FFN projections and keeps
  attention/norms/embeddings verbatim — the real §A policy — populating engine roles in the manifest).
  Plus **native GPU dispatch + on-device parity** (`ternary_gpu.rs` runs the kernels on a real wgpu
  device; parity passed **on an NVIDIA A2000** == the CPU oracle). **2-bit branchless variant**
  (external-review-driven): `ternary_gemm_2bit.wgsl` + `pack_trits_2bit` + `ternary_gemm_cpu_2bit` —
  4 trits/byte, shift/mask unpack (no base-3 `/`,`%`), `acc += (f32(c==1)-f32(c==2))*x` (no warp
  divergence); also on-device-verified on the A2000. **Measured on the A2000** (persistent
  pipeline, 4096² batch-1 GEMV): F16 0.963 ms · base-3 ternary **1.140 ms (0.85× — slower than F16!**
  the `/3`,`%3` unpack costs more than it saves) · **2-bit branchless 0.544 ms (1.77× vs F16, 2.10× vs
  base-3)**. The win is only 1.77× (not the ~8× the 16→2-bit ratio implies) ⇒ the naive
  one-output-per-thread GEMV isn't yet bandwidth-bound; **tiled/coalesced GEMV + subgroups is the next
  lever**. End-to-end tok/s still needs the FFN-loop splice + in-loop timestamp queries. **Remaining (task #12):** splice the dispatcher into the live FFN layer loop
  (`gguf_bridge::dispatch_transformer_forward` / `encode_fused_ffn_expansion` — branch on
  `ggml_type == TERNARY`, resident + streaming modes) and a real-model end-to-end run + tok/s vs the
  F16 baseline; then the other §A compressions (KIVI, W4A4/AWQ, spec-decode).
- **KIVI asymmetric KV-cache**: Key cache per-channel 2-bit, Value cache per-token 4-bit → 100k+ context in
  consumer VRAM via a WGPU ring-buffer.
- **W4A4 + Activation-Aware (AWQ)**: a calibration "Concentration-Alignment Transform" over the high-fidelity
  source, scaling factors baked into the CBOR-LD header; runtime scales activations down / weights up →
  Q8-equivalent math at 4-bit speed.
- **Speculative decoding** via zero-copy mmap: map a ~100M draft + the target model; draft guesses 4–5 tokens,
  target verifies in one pass; zero heap penalty swapping → 2–3× perceived TPS.
- **AOT ingest from high-fidelity source** (Q8_0/F16, not pre-lossy Q4): the engine down-samples pathways
  itself. Stream in ~256 MB chunks across Web Workers (avoid main-thread block); flush page-aligned blocks to
  OPFS immediately. (Times: 360M ~2–5 s; 3–4B ~25–45 s; 8B ~1.5–3 min.)
- **Demand-paged mmap** — run models exceeding physical RAM (page layers in at microsecond of use).
- **Transcode → manifold-native, not an opaque blob** (the singular-pipeline payoff): GGUF / **safetensor**
  (add safetensor to `ingest/detect.rs` — currently absent) → the native format encodes weights as
  `Tensor10D` SOA on the **shared resident substrate** (graph–tensor duality, `compute_universe.rs`), so the
  model becomes part of the *same* manifold the logic / render / audio operate on — directly **GPU-enumerable,
  fused-kernel-ready (§F), zero-heap** — with the §A compression (ternary / KIVI / W4A4) applied *during*
  transcode and the lexicon / CBOR-LD header (the wave-coordinate substrate, §D) carried natively. The model
  stops being a blob you query and becomes substrate the whole pipeline computes over.
- **Distribution**: pre-compile `.q42` and host on Hugging Face / WebTorrent swarm → end-user TTFT ≈ 0.

### B. Neuro-symbolic binding — tokenizer → ontology (the differentiator)
*Goal: tokens that mean something; logic before the math finishes.*
- **Semantic Binding Pass** at AOT: extract the flat tokenizer vocab (tokens/scores/merges) and rewrite each
  token as a CBOR-LD node, e.g. `{"@id":"wordnet:synset:apple.n.01","lexical_value":" apple","domain":"botany"}`.
  Cross-reference WordNet/FIBO/SOSA/RadLex/etc.; write a **CBOR-LD semantic header** at the top of the `.q42`.
- **The neuro-symbolic sieve**: WGSL shaders reason over the ontology graph *before* the probability math
  completes → **deontic token masking** (ODRL/SHACL gate sets forbidden token IDs to probability zero at the
  hardware level — e.g. "no clinical advice" masks SNOMED/RadLex tokens).
- **Personal / human-centric ontologies** — the binding differs per person (language, field, "personal
  ontology"); "ground truth" (loud/red/dangerous) is relative to the user's local ontological boundaries.

### C. File-format v2 — the 10D → bifurcated 5D NQuin
*Goal: model physics, space, time, and law in one predictable memory stride.*
- A monolithic LLM `[batch,seq,d_model]` expands into a **10D volumetric tensor**, folded into the fixed
  48-byte **5-element Quin**: ⟨Subject, Predicate, Object, Context, **Manifold-Coordinate**⟩.
- The **bifurcation** is the 5th element: Subject/Predicate/Object/Context = discrete semantic RDF scaffold;
  the **Manifold-Coordinate** is the dimensional gateway holding **temporal asymmetry (t∆)**, **deontic
  state (D)** (active ODRL permissions for the agent), and **physical momentum (P)** (sensor-derived kinetics).
- **Engineering challenge to solve**: structure the NQuin manifest so a "semantic-weight" pointer vs a
  "geometric-multivector" pointer are differentiated **without slowing the hot loop** (overflow buffer for
  non-conforming data; keep the 48-byte stride sacred).
- Embed a **provenance graph** in the CBOR-LD header (per-weight DID provenance — see H).

### D. Multimodal as physics (NOT tokenised images/audio)
*Goal: ingest the universe as it behaves, not as vocabulary. "Large Physics Models," not VLM parlor tricks.*
- **Acoustic manifolds**: bake raw audio into tensor dims as STFT/CQT (phase-aligned signal processing in WGSL),
  never tokenised.
- **Spectral tensors over RGB**: vision/LIDAR/optical as true spectral tensors (thermodynamic/optical physics).
- **Full EMF spectrum**: anchor to W3C **SOSA/SSN** ontologies; treat LIDAR, thermal, IR-depth, RF, and
  **Wi-Fi CSI** (sense through walls / vitals) as structural vectors. Intercept *raw* telemetry before the OS
  down-samples (e.g. 96/192 kHz audio past Nyquist; NIR past the IR-cut filter).
- **Eulerian Video Magnification / phase-based motion** → a "motion microscope": `portal_spectral.wgsl`
  (Steerable Pyramid / State-Space-Model decomposition) amplifies sub-pixel micro-motion → multivectors.
  Refs to track: GeoMag (geometric-aware VMM/SSM), GeoDiffMM (geometry-guided diffusion), Caltech sub-pixel
  (1/500 px) bio-telemetry. Domains to pilot first: **structural-mechanical monitoring** and **human
  bio-telemetry** (bind aberrations to RadLex; runs on local silicon, biometrics never leave the device).

**The unifying substrate — wave coordinates in fixed 10D dims (THE zero-heap mechanism).** EMF (light /
thermal / RF / X-ray), acoustic (sound / ultrasound), and other sensor signals all reduce to a common **wave
coordinate** — frequency/wavelength · amplitude · phase (+ modulation μ, signature σ) — which is exactly
`SpectralDecomposition{amplitude,phase,frequency}` and the 10D spectral axes (α/μ/σ). Because these are
**fixed tensor dimensions**, not token arrays or sample buffers, the representation is **zero-heap for *any*
modality**: no `Vec` of samples, no per-pixel RGB objects, no `String` — one fixed stride (the same invariant
as the 48-byte NQuin). This is *why* multimodal-as-physics is the design: it makes representation, enumeration,
and complex evaluation all zero-heap on one substrate.
  - **Percepts are *enumerated*, not stored.** Colour, pitch, brightness, timbre, heat are **pure functions
    over the fixed coordinates within a band** — colour = f(wavelength, amplitude) in the visible-EMF band;
    pitch = f(frequency) in the audible-acoustic band (CQT). Computed on the fly from fixed dims → no
    allocation. ("Enumerate down to colour" = a projection of the manifold, exactly like 3D from 10D.)
  - **Complex eval is zero-heap** — multimodal reasoning runs as tensor ops on the fixed manifold (the
    cross-manifold fused kernel §F, masking on dims): fixed strides, no heap. Tokens/buffers are
    variable-length/heap; wave-physics in fixed 10D dims is representation **+** enumeration **+** evaluation
    in one zero-heap substrate.
  - **EMF ≠ acoustic — don't flatten.** They share the *wave abstraction* but are physically distinct kinds
    (EM transverse waves, `c`, vacuum vs mechanical pressure waves, medium-dependent `v`). The kind is a
    tagged band parameter so enumeration + propagation physics respect it (colour from EMF-visible, pitch from
    acoustic-audible) — the same anti-flattening discipline as the man-made/natural boundary.
  - **Sensor-extensible** — a new modality = a new band/projection of the same wave substrate (SOSA/SSN
    observation + the spectral dims). The manifold renderer (§E) draws colour from the spectral *signature*,
    not an RGB literal: **spectral → percept → render** is one chain.

### E. The manifold renderer — 3D / geometric / CAD / photogrammetry (projections of the 10D foundation)
*Goal: constraint-satisfying geometric modelling, not probabilistic 3D "guessing." The renderer is a
**projection of the 10D manifold**, not a bolt-on 3D engine: 2D screen, 3D scene, and 4D spacetime views are
**enumerated from the same 10D structure** — the reason 10D is the foundation (lower dimensionalities derive
from it). Artefacts carry their **physics** and their **place/space/time** situation, not merely geometry.*
- **Projective Geometric Algebra (PGA)** multivectors `M = α + v + B + T` bound to `kinematics.wgsl`; the
  geometry **refuses to contract** when a suggested action violates the physical bounding box (deterministic
  prevention).
- **CAD as a constraint system**: the attention/deontic layer verifies watertight + structurally sound +
  printable (overhang/wall-thickness ontology) **before** tensor reduction.
- **Photogrammetry = inverse physics**: 2D sequences → SDF / point cloud, stored as 5D NQuins so the object is
  semantically "known," not pixels.
- **Direct-load** `.obj` / `.stl` / **OpenUSD**; assets = *physical manifold + kinematic multivector* (joints
  as multivectors in the 5th dimension).

**Current state (honest, 2026-06-22) — the gap to close FIRST.** The *data* is genuinely 10D with real 3D
(`Tensor10D{q,v,w,x,y,z,t,α,μ,σ}`, `SpacetimeCoord{x,y,z,t}`), and `webizen-render` has 3D scaffolding (a 4×4
`view_projection` matrix + look-at `SceneCamera`, PGA math, z-depth scaling). **But the implemented output is
the ~2.5D ambient particle field** (50k points, screen-space positions + z-for-depth) — there is **no
depth-stencil buffer, no mesh vertex/index geometry, and no `.obj`/`.stl`/OpenUSD import**. So **3D *assets*
are not yet rendered**; the renderer projects a point/particle field, not a world-space scene. An
output/renderer gap on a sound foundation, *not* a dimensional limit.

**Closing it — the manifold renderer (elevated to near-term, Timothy 2026-06-22):**
1. **World-space 3D scene** — reuse the existing `view_projection` matrix + `SceneCamera` + PGA; add a
   **depth-stencil buffer** (occlusion), **mesh vertex/index buffers** (geometry), and **asset import**
   (`.obj`/`.stl`/OpenUSD).
2. **Physics of artefacts** — each asset = *physical manifold + kinematic multivector* (mass/material/momentum
   `P` in the Manifold-Coordinate §C; `specialized_libs/physics_simulation`; PGA "refuses to contract" on a
   bounding-box violation). An artefact *behaves* physically, it is not just a shape.
3. **Place / space / time (spatio-temporal binding)** — `x,y,z` (space) + `t` (time → temporal evolution /
   animation) + **place/jurisdiction** (GeoSPARQL) evaluated by native **RCC-8** (`spatio_temporal.rs`) and
   Allen/LTL temporal reasoning. An artefact is situated in space **and** time **and** place, and that
   spatio-temporal logic is queryable by the *same modalities* the values layer uses.
4. **One projection, many views** — a single `project: 10D → target` enumerates the 2D / 3D / 4D views from the
   manifold (the volume metric in `tensor/volume_gpu.rs` / `manifold.rs`). The CML Studio 2D canvas and a 3D
   scene are the *same manifold* at different projections.

Leverages: `tensor/` (coordinate · spacetime · manifold · volume_gpu · spectral · topology), `webizen-render`
scaffolding (view-proj · camera · PGA), `specialized_libs/physics_simulation`, `spatio_temporal` (RCC-8),
`interval_reasoning` / `temporal_ltl`. (`PendingImplementation`.)

### F. The cross-manifold fused kernel
- Upgrade `fused_attention.wgsl` / `tensor_volume.wgsl` to **parallel dot-product contraction across all
  orthogonal manifolds** (semantic Quins, spectral tensors, acoustic wave-functions, PGA multivectors)
  simultaneously, **phase-aligned on time (t∆)** at microsecond precision.
- **"Attention" redefined**: not next-token prediction — a **phase-alignment / constraint-satisfaction /
  deontic gateway** that reduces the 10D manifold to what's critical for the current Quin and discards
  irrelevant/forbidden sensor data at the hardware bus (ultimate efficiency).

**The fabric this runs on already exists (honest, 2026-06-22) — `compute_universe.rs`.** This is *why* the
LLM, display, and audio were brought *into* the engine rather than federated as separate processes: a **single
pipeline manifold**. One physical `wgpu::Device` (`gpu_context::shared_gpu`); the semantic `NQuin` graph and
the `Tensor10D` SOA **share one resident substrate** (graph–tensor duality); compute runs as coordinated
**universes** — U0 (LLM) · U1 (tensor) · Sentinel (governance) — over lock-free SPSC rings (the Phase-8
bifurcation), under one `VramLedger`. The 10D manifold *metric* is already GPU-resident (`tensor_volume.wgsl`
ports `Tensor10D::full_distance`). So data is encoded **once** on the shared substrate and the GPU
**enumerates** it for every consumer — no inter-engine marshalling, no heap copies between an LLM process and a
renderer process. *That* is the performance argument: separate engines each copy/convert (heap + latency); one
manifold + one device + GPU enumeration serves LLM inference, logic-modality eval, render, and audio
**simultaneously**, zero-heap.

**Remaining for §F:** (a) bring the **render (`viewport/`) and audio (`spectral` / STFT-CQT)** functions fully
*under* the universe orchestration (they live in the same engine on the shared device today, but as distinct
pipelines, not yet formal universes in the fabric); (b) the **single fused pass** that reduces *all* orthogonal
manifolds at once (above) — currently separate wgsl shaders (`fused_attention`, `tensor_volume`, `ambient`,
`sieve`, …) dispatched on the one device, not yet one cross-manifold kernel.

### G. Heterogeneous compute — CPU + GPU + NPU (+ QPU hooks): the underlying core
*Goal: route each math to the silicon built for it (WebNN + WebGPU + WASM). This is the **bedrock under the
`compute_universe` fabric (§F)** — the universes dispatch each operation to the right processor over the one
shared resident substrate.*
- **NPU (WebNN)** — tensor contraction as a primitive: multi-way relational/dot-product reductions on PGA /
  10D volumetric tensors **without flattening** the geometry; power-efficient.
- **GPU (WebGPU)** — continuous physics & spatial dataflow (`kinematics.wgsl`, `tensor_volume.wgsl`).
- **CPU (WASM)** — deterministic logic & the deontic/DID gatekeeper (`shacl_compiler.rs`, `n3_parser.rs`):
  short-circuits the bus before an unlawful/unsafe vector is ever dispatched.
- **QPU (hooks — future / rare, part of the q42 design)** — `qpu_ingress` + `qubo_compiler` *formulate* a
  QUBO / circuit job; **classical solve by default**, with optional **external-provider dispatch** (8
  providers: IBM / D-Wave / IonQ / Rigetti / Azure / Braket / Google / Quantinuum) gated by `qpu_enabled` +
  a signed commitment. MCP: `qpu_optimize` / `qpu_dft` / `qpu_status`. Hooks now; rare/optional quantum
  offload later — **never on the hot path**.

### H. Federated, energy-opportunistic, provenanced training
*Goal: turn excess (e.g. solar) energy into collaborative, trustworthy model improvement — without a cloud.*
- **Federated LoRA + CRDT**: frozen `.q42` base; nodes train tiny (15–50 MB) LoRA adapters in-shader; deltas
  gossiped P2P asynchronously (`crdt.rs`). `lora_apply.wgsl` treats adapters as **commutative geometric
  layers** folded into the base manifold on the fly (16 KB-aligned, zero-copy mmap over the base).
- **Energy-aware dispatcher** (`qpu_dispatcher.rs` / `daemon_swarm.rs` telemetry): Deficit (battery → 1.58b
  inference, training suspended) / Equilibrium (plugged → Q4 inference) / Surplus (solar/grid peak → wake,
  pull task, saturate WGPU for LoRA gradients). Must pause/resume without memory corruption.
- **Guild provenance**: each LoRA delta is **DID-signed**; the `.q42` provenance graph + ODRL/SHACL gate
  verifies the signer belongs to the trusted guild **before** the weight can influence compute.

### I. Distribution, access & the human payload
- **Credential-gated personal vault** = the wellfair life-record (shield + testament): completeness,
  tamper-evidence, self-custody, **succession / posthumous release** (the key missing piece), selective
  disclosure. Keyed to the user; never hold their keys.
- **CML inline-markup tooling** + the **hypermedia pipeline**: e.g. grab WordNet → define an ontology →
  emit denoted CML inline mark-up across a corpus (the W3C-mail archive, then PDFs) → semantically rich,
  agent-parsable "hypermedia" (Ted Nelson → TimBL lineage). [Future task, noted.]

---

## 3. Sequencing — what makes it Stellar, in order

> Timothy's stated priority: **get V1 done first.** Design the advanced layers *now* (so the foundation
> isn't gutted later) but build them after First Light is robust.

1. **Harden V1** (the four open defects in §1) — esp. the `wgpu` device-limit fix; this unblocks the browser
   LLM for most users. *Highest leverage.*
2. **`.q42` v2 perf (§A)** — ternary FFN + KIVI KV-cache + speculative decode → the big TPS/context jump.
3. **Neuro-symbolic binding (§B)** — tokenizer→ontology + deontic masking → the *true* differentiator
   (this is what no Big-Tech stack has; it's where "human-centric" stops being a slogan).
4. **10D→5D file-format v2 (§C)** — the substrate that lets D/E/F exist.
5. **Multimodal-as-physics + 3D/PGA + fused kernel + heterogeneous compute (§D–G)** — "Large Physics Model."
6. **Federated training + distribution + the credential-gated vault (§H–I)** — the human payload at scale.

**Definition of "Stellar / phenomenal":** not merely fast — *trustworthy, physics-grounded, self-owned,
governed, and in the hands of the people who need it.* Every milestone is judged against §0's governance rails
and the North Star, not just the benchmark.

---

## 4. Honest framing
- **Proven** = §1 (V1 First Light, with its named defects). **Roadmap/aspirational** = §2 (sourced from the
  advanced-ideas conversations; sound in principle, unbuilt; some cite mid-2026 research to verify before relying).
- Built by **one person + AI leverage** under severe resource constraint — so sequencing favours the highest-
  leverage, foundation-preserving steps; the advanced layers are designed-now / built-later.
- The hard problems are named, not hidden (e.g. social-recovery coercion, W4A4 coherence, hot-loop pointer
  striding, federated-trust). That honesty *is* part of the method (fiction/non-fiction discipline applied to
  our own roadmap).
