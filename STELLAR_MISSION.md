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
- **Deontic / provenance gate** wired (intent pre-flight, provenance-citation post-flight).
- **Provenance vault** (`provenance/`, gitignored + out-of-repo for sensitive) — the first real dataset is
  Timothy's own life-record (see `memory/project_provenance_vault.md`, `project_wellfair_purpose.md`).
- **Draft standards suite** — CML, CMLD, DOA, DOE, HCAIO, DigitalBirthRecord, rights, ulem, etc.

### Honest open V1 defects (fix before calling V1 done)
1. **`wgpu 0.19.4` sends `maxInterStageShaderComponents`** → `requestDevice` fails on recent Chrome (the
   browser LLM won't init for many users). Needs wgpu upgrade (0.20+) + full GPU regression test.
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

### E. 3D / geometric / CAD / photogrammetry
*Goal: constraint-satisfying geometric modelling, not probabilistic 3D "guessing."*
- **Projective Geometric Algebra (PGA)** multivectors `M = α + v + B + T` bound to `kinematics.wgsl`; the
  geometry **refuses to contract** when a suggested action violates the physical bounding box (deterministic
  prevention).
- **CAD as a constraint system**: the attention/deontic layer verifies watertight + structurally sound +
  printable (overhang/wall-thickness ontology) **before** tensor reduction.
- **Photogrammetry = inverse physics**: 2D sequences → SDF / point cloud, stored as 5D NQuins so the object is
  semantically "known," not pixels.
- **Direct-load** `.obj` / `.stl` / **OpenUSD**; assets = *physical manifold + kinematic multivector* (joints
  as multivectors in the 5th dimension).

### F. The cross-manifold fused kernel
- Upgrade `fused_attention.wgsl` / `tensor_volume.wgsl` to **parallel dot-product contraction across all
  orthogonal manifolds** (semantic Quins, spectral tensors, acoustic wave-functions, PGA multivectors)
  simultaneously, **phase-aligned on time (t∆)** at microsecond precision.
- **"Attention" redefined**: not next-token prediction — a **phase-alignment / constraint-satisfaction /
  deontic gateway** that reduces the 10D manifold to what's critical for the current Quin and discards
  irrelevant/forbidden sensor data at the hardware bus (ultimate efficiency).

### G. Heterogeneous compute — the CPU + GPU + NPU trinity
*Goal: route each math to the silicon designed for it (via WebNN + WebGPU + WASM).*
- **NPU (WebNN)** — tensor contraction as a primitive: multi-way relational/dot-product reductions on PGA /
  10D volumetric tensors **without flattening** the geometry; power-efficient.
- **GPU (WebGPU)** — continuous physics & spatial dataflow (`kinematics.wgsl`, `tensor_volume.wgsl`).
- **CPU (WASM)** — deterministic logic & the deontic/DID gatekeeper (`shacl_compiler.rs`, `n3_parser.rs`):
  short-circuits the bus before an unlawful/unsafe vector is ever dispatched.

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
