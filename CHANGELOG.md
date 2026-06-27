# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.0.21-la] — 2026-06-27

The **mathematical / statistical substrate** push on branch `0.0.21-la`: one engine is the
source of truth, specialized libraries become composition callers, and every new capability is a
categorised library that reuses the foundation (no duplicated math), fails closed (real result /
`NotImplemented` / `InsufficientData` — never a fabricated number), and is dispatch-ready (§13:
clear kernel-class boundary + an always-present CPU reference). Engine `--lib` suite green
throughout, **1905 tests** by series end (authoritative build: `--manifest-path` the worktree).

### Added — Hardware-backend bridge (P1–P3)
- `platform/compute_bridge/`: an **open `ProbeableBackend` registry** + 8-class `KernelClass`
  taxonomy + per-class `ComputePolicy::select → Plan`, built on the existing
  `device_benchmark`/`hetero_dispatch`/`HardwarePassport`. CPU path always present and never
  hard-fails; GPU/NPU/vendor paths are correctness-gated against the CPU reference before they may
  be the default. Headless-testable (synthetic matrices).

### Added — Statistics & probability (real, honest)
- Root-caused the prior statistics weakness as **fabricated p-values** (the `|t|>1.96 ⇒ p=0.05`
  hack) and replaced it: `solvers/statistics/distributions/` (special functions — `ln_gamma`,
  regularized incomplete gamma/beta, `erf`; Normal/Student-t/χ²/F/MVN pdf/cdf/quantile,
  table-validated). Real p-values end-to-end across hypothesis tests (one/paired/two-sample t,
  one-way ANOVA, χ² GoF/independence, McNemar/Friedman/Iman-Davenport).
- Descriptive completeness (covariance/skewness/kurtosis/quantiles), Spearman + significance, OLS
  regression with inference; resampling (cross-validation, bootstrap SE + percentile/BCa CIs,
  permutation tests); robust/EDA (trimmed mean, MAD, IQR/winsorisation); information theory
  (entropy/KL/cross-entropy/mutual information); experiment design (power/sample-size, A/B
  two-proportion, bandits ε-greedy/UCB1/Thompson); and **anomaly detection** (z-score, robust
  modified-z, Tukey fences, Grubbs' test, multivariate Mahalanobis χ² gate).

### Added — Statistical learning (ISL + PRML) — `solvers/learning/`
- The full *Introduction to Statistical Learning* surface (ch 2–13; ch 10 deep learning
  deliberately deferred to the native LLM stack) and the *PRML* Bayesian/temporal/graphical spine:
  OLS/ridge/lasso/PCR/PLS, logistic/Poisson/multinomial GLM (IRLS), LDA/QDA/NB/KNN/SVM(+RBF,
  +multiclass), CART/random-forest/boosting/BART, PCA/k-means/GMM-EM/hierarchical/SOM,
  splines/GAM/smoothing-splines, survival (Kaplan-Meier/Cox), multiple-testing corrections, metrics;
  plus MVN, Bayesian linear regression, Gaussian process, HMM, MCMC, Kalman, belief-propagation
  (sum-product), mean-field variational inference. ~50 estimators, all reusing
  `linear_algebra` (cholesky/gemm/eigen/qr/svd) and the distributions library.
- **Active learning** (`learning/active/`): frugal human-attestation ranking — uncertainty
  sampling, query-by-committee (vote/consensus entropy, average-KL disagreement), information
  density. Pure ranking over existing estimators' predictions; surfaces the *few* items most worth
  a human's judgement instead of demanding mass labelling.
- **KG embedding** (`learning/kg_embedding/`, affordability-gated): TransE/DistMult/ComplEx/RotatE
  scoring + link-prediction (MRR/Hits@k) as the always-on cheap path; an SGD trainer (margin +
  logistic, negative sampling, deterministic) as the **heavy run-once artifact producer** — never
  on a user's critical path.

### Added — Cross-plan capabilities (the book-capability plans)
- Metaheuristic optimizers (hill-climbing/simulated-annealing/ABC, generic over state);
  **ontology alignment as optimization** (`solvers/ontology_align/`) that structurally can only
  emit `closeMatch → RequiresHumanReview` — never an asserted `exactMatch`; type-2 / interval
  type-2 fuzzy (`modalities/fuzzy_type2`); fuzzy graph-match (similarity + approximate);
  Zadeh fuzzy quantifiers; graded fuzzy-RDF-schema entailment; spreading activation
  (`solvers/graph_opt/`) for the relevance router; fractal/hierarchical shortest-path
  decomposition (cell-distributable); **constructibility** decision in the CAS
  (`specialized_libs/constructibility.rs` — Wantzel degree criterion, Gauss-Wantzel polygons, the
  classical impossibilities).

### Added — Likeliness modality (`modalities/likeliness/`) — a third uncertainty calculus
- A **qualitative, ordinal** calculus of expectation (Vector Semantics §4.2; Kornai's "naive"
  inference), built as its **own modality** rather than folded into `defeasible`/`fuzzy` (Timothy's
  call: indecision about the fold is itself the signal it is distinct). A 7-level scale
  (`Impossible … Even … Certain`) with a **Kleene / De Morgan** lattice (`not` involution, `and`
  meet, `or` join) — no excluded middle and no contradiction collapse, which is exactly what makes
  it *not* probability (no normalisation, non-additive) and *not* fuzzy membership. Naive inference
  on top: weakest-link modus ponens, chain attenuation (longer defeasible chains weaken), rebuttal,
  and defeasible revision. Kernel-class `ElementwiseMap`.

### Added — KG↔LLM live-system integrations
- **f-SPARQL** (`solvers/fuzzy_query/`): a degree algebra over the engine's own `BindingRow`
  solutions (join=t-norm, union=t-conorm, projection, negation, α-cut, top-k) + FILTER membership
  functions + an annotate/collect hook over the crisp executor stream. No fork of the SPARQL parser.
- **KG↔LLM grounding gate** (`solvers/grounding/` + `inference/orchestrator.rs`): deepens the
  post-flight from "a citation exists" to "the claim is *supported* by its cited facts." A resolver
  turns provenance hashes into facts; weak grounding routes to human review, ungrounded is blocked —
  both **before** WAL commit. The plain entrypoint is unchanged (no resolver → no-op; zero
  regression).

### Added — The Swarm: verify-before-pay distributed jobs (`services/swarm/`)
- A paid swarm is dual-use, so **payment is impossible without independent result verification**:
  content-addressed `JobSpec` (Personal/Collaborative/Paid), a `JobExecutor` trait + real local
  executor, **Freivalds' O(n²) probabilistic matrix-product verification** + embedding-artifact
  ranking reproduction, and a dispatch path where no `Rejected` verdict can reach a payment.
- Settlement is an **escrow state machine over `value_flow`** (ROI-capped pricing via `commons_cost`;
  `eroi_viable` = the solar-excess thermodynamic gate). On `Verified` it emits a
  `MicropaymentInstruction` for the existing `ilp_dispatcher` rail; on `Rejected`, a refund. **It
  never moves funds itself.**
- Isolate B in `daemon_swarm.rs` **de-mocked** — the former constant-`999` fabrication replaced by a
  real deterministic computation through the swarm executor.

### Changed — Honesty pass across specialized libraries
- Neutralised fabricated achieved-quality metrics surfaced to clients (medical diagnosis confidence,
  structural safety factor, trade fills, compliance passes, ML/physics/chemistry placeholder
  outputs) — now real computation or a fail-closed `NotImplemented`/`InsufficientData`, with tests
  asserting the honest behaviour. Numeric duplication consolidated: the dense-LA core (gemm/qr/
  cholesky/eigen/lu/svd/spectral/polynomial) lives once in `solvers/linear_algebra`; the LLM forward
  pass is grounded as named STEM math (GEMM/activations/softmax/norm/attention/RoPE/FFN), each proven
  equal to the real kernel.

---

## [0.0.19] — 2026-06-22

A large release centred on the **human-rights / values-credentials subsystem** and a complete
**computational legal-logic engine**, plus the algebra/CAS/ZK and hybrid-modality groundwork
that preceded it. Engine `--lib` suite green throughout (1153–1160 tests by series end).

### Added — Computational legal-logic stack (`legal_logic.md` §1–§30; plan: `DEONTIC_LOGIC_PLAN.md`)
All §1–§30 logic operators implemented as tested `modalities/*.rs` (only the two heavy
*substrates* remain — GPU 10D renderer #11–13, binary carrier codecs #9):
- **SDL⁺ core** (`deontic.rs`): O/P/F + Optionality/Gratuitousness, the full lifecycle
  (Pending/Active/Violated/Defeated/Discharged/Expired), rebutting **and** undercutting
  defeaters (`DefeatKind`), first-class dyadic `O(q|p)` (CTD generalised).
- **Hohfeldian jural square** (`jural.rs`): 8 correlative positions, correlativity, unmet-duty
  legibility, personhood category-error; ontology `core-ontologies/jural.n3`.
- **STIT agency** (`stit.rs`): duty-bearer vs bystander, omission, joint/shared liability.
- **Compositions** (`deontic_compose.rs`): deontic × temporal (`O(Gφ)`/`O(φ U ψ)`), × epistemic
  (**mens rea**), × spatial (jurisdictional subsumption), × linear (discharge), × argumentation
  (Dung grounded extension), × fuzzy/probabilistic, × ASP/abductive.
- **Causal** (`causal.rs`): but-for causation, root-node dependency cascade, overdetermination.
- **Capacity/delegation/contract** (`capacity.rs`, `delegation.rs`, `contract.rs`): duress →
  voidable-at-election (confirmed by Timothy), guardianship, posthumous standing, delegation +
  revocation cascade, Offer→Assent→Binding.
- **Economic/identity** (`value_flow.rs`, `capability_gap.rs`, `identity_fabric.rs`): Permissive
  Commons capped-ROI threshold discharge, RPL capability gap, k-of-n resilient identity.
- **Meta-deontic / governance** (`meta_deontic.rs`, `interaction_governance.rs`,
  `responsibility.rs`): provenance-anchored court-admissible WAL BreachRecords with ed25519
  endorsement; verdict → PolicyMode (PreventiveBlock/PermissiveAudit/Prioritize/Interactive);
  allegation→adjudication; systemic meta-guards (rule-of-law asymmetry, overreach, accountability
  vacuum). Composition wires for ZK-gate, proportionality (CAS), sense-translation, consensus,
  wave→fact, content-addressed carriers.

### Added — Values-credentials subsystem (`core-ontologies/`)
- 101 UN/IHL instruments lifted into a **CML 3-layer concept graph** (TEXT→CONCEPT→LOGIC):
  3,518 concepts / 3,619 deontic norms, all `cml:Proposed` pending human attestation.
- `values.n3` Agent lattice, `agency.n3` grounding, `jurisdiction.n3` (ratification + AU pilot),
  `modal-junctures.n3` (illocutionary force + epistemic strength bands), "person" as
  **frame-relative** (per-document reading, not global).
- **BCP-47 `@en` language tags** across the corpus (multilingual foundation).
- 220 round-trip-verified `.q42` volumes; `docs/values-credentials.html` demo over real data.
- **CML library-upgrade protocol**: `SCHEMA_VERSION` + `cml:schemaVersion` stamps,
  `tools/reprocess_library.py` (idempotent, `--check` staleness gate), `CML_UPGRADE.md` +
  `CML_VERSIONS.md` — regeneration preserves human curation (SOURCE/GENERATED split).

### Added — MCP cooperation & governance surface
- **Agent-cooperation gate** (`mcp_cooperation.rs`): verified + grounded (agency.n3 G1') caller
  standpoint → deontic gate; **flag-gated dispatch enforcement** (`QUALIA_MCP_ENFORCE`, default
  off) so every call can be required to carry a standpoint.
- MCP tools: `values_check`, `values_evaluate`, `jural_correlate`, `deontic_govern`,
  `mcp_cooperate`, `graph_resolve`.
- Native **verifiable credentials** (`verifiable_credential.rs`): issue/verify (ed25519) +
  issuer grounding. Runtime **agent-type resolution** (`agent.rs`).

### Added — Algebra, symbolic CAS, ZK, manifold (earlier 0.0.19)
- Polynomial roots, determinant, symmetric + general eigensolvers, SVD; a symbolic CAS
  (`Expr`/simplify/differentiate/parse/expand/factor, Expr↔NQuin); MCP algebra/CAS tools.
- Real **R1CS zero-knowledge** proof of matrix multiplication (A·B=C), Groth16 (arkworks).
- Manifold GPU/CPU volume-metric unification + CPU reference search + metric-parity audit;
  **RCC-8** spatial topology (zero-heap, full-polygon).

### Added — Hybrid-modality engine (#22) + FrameLayout ABI
- Zero-alloc `QuinIndex` accessors, revision-cached per-cell index, `modal_kind` resolution,
  unified resolver + `graph_resolve`, collision-aware lexicon interner (the lexicon backstop).
- `frame_layout.rs` as the canonical ABI for the NQuin's 6 computational bytes.

### Added — SHACL coverage
- `logic_modalities_shacl.rs` now carries **42** `q42:<Name>ConfigurationShape`s (the whole
  legal-logic surface) with engine↔SHACL completeness tests; epistemic shape fully fleshed out.

### Added — Surface
- Modalities Observatory: **38** live demo cards (incl. the full §1–§30 legal-logic stack);
  enriched epistemic + deontic demos; CML Studio linked into the nav.

### Changed
- **One 60-bit hash space**: `q_hash` ≡ `generate_60bit_token` (top 4 bits = type/modality tag).
- `FrameLayout`: `0b101` formally allocated as inline `xsd:float` (resolver float-tag fix).

### Fixed
- **Non-derogable immunity** blocks the Exemptive override (ICCPR Art. 4(2)) — a derogation can
  no longer defeat the right to life, etc.
- GPU `count_buf` `COPY_DST` wgpu validation error (was misreported as "no GPU").

---

## [0.0.18] — 2026-06-19

### Added — Browser-native WASM + WebGPU LLM inference (Phase 2B → Phase 5)

- **In-browser decode at ~5.9 tok/s** on SmolLM2-360M (Q4_K_M), coherent, zero-heap hot loop,
  on a stock NVIDIA Ampere via Chrome WebGPU — up from 0.6 tok/s. Generalised across quant
  (Q8_0 verified). The whole engine is native Rust→WASM (no Ollama / llama.cpp / Python).
- **`.q42` AOT weight container** (`q42_weight.rs`): compile a GGUF once → a self-contained,
  16 KB-page-aligned container (weight blobs + hyperparams + tokenizer, `Q42W` magic, CRC-32C).
  `compileGgufToQ42` + `q42FormatVersion` WASM exports; cached in OPFS; warm boots are a
  zero-parse `.q42` load. All inference runs from the `.q42` thereafter.
- **OPFS model cache** (`docs/js/opfs-model-cache.js`): `loadGgufCached` + `loadOrCompileQ42`
  stream models to disk (no whole-model JS-heap blob) and serve instantly on repeat loads.

### Changed — Phase 5 decode throughput (the ~10× arc)

- **Parallel Q/K/V projection (Phase 5.5, the win):** `fused_attention.wgsl` was
  `@workgroup_size(1)` — one thread per head doing the entire projection GEMM serially (~15
  threads on a 5000-core GPU). Routed Q/K/V projection through the parallel
  `fused_transformer.wgsl` GEMM (dedicated proj buffers + `proj_row_stride`), leaving the
  attention kernel SDPA-only. forward 52437→4452 ms (~12×); GPU drain 1577→114 ms/token.
- **Single-submit forward (Phase 5.4):** one `CommandEncoder` + monotonic uniform cursors
  across all layers → 64 submits/token → ~2 (per-layer `mc8_flush` removed; KV visibility via
  WebGPU intra-encoder barriers).
- **Resident weights / logits / norms:** layer weights, the tied `token_embd` output/logits
  projection (~50 MB), and per-layer attn/ffn norms uploaded to VRAM once at init — no
  per-token/per-layer `write_buffer` re-uploads.
- **Modular fused FFN (Phase 5.0):** `gate·SiLU·up` collapsed into one compute pass, composed
  from `math_core`/`fused_ffn` WGSL fragments in Rust, with a const Phase-6 deontic-taint seam.

### Changed — Productization

- **Playground WASM refreshed** to the superset (`portal,wasm-llm,wasm-logic,wasm-scientific,
  wasm-playground`): 75 exports, **0 dropped** (science exports retained) + the q42/async-LLM
  exports — brings the Phase 5 engine to the shared `docs/playground` artifact.
- **`llmdemo` + `online-llm-demo`** run the `.q42` AOT/OPFS pipeline end-to-end at ~5.9 tok/s.

---

## [0.0.17] — 2026-06-17

### Added — U3 AcousticPlane (symbolic audio on Pages)

- **Universe U3 (`AcousticPlane`)**: Sonic Token SPSC ring, parametric DSP kernel, and `AcousticUniform` (328 B / 82 floats) wired through `QualiaPortal` WASM exports — no PCM from U0.
- **Spectral-first playback**: Cold STFT bake (`stft_bake.rs`) and **CQT bake** (`cqt_bake.rs`) with `Q4AU` sidecar header (`SIDECAR_KIND_STFT` / `SIDECAR_KIND_CQT`); `audio_sidecar_link.rs` hashes frames, emits `q42:hasSpectralSheet` quins, and writes `spectral/audio/{hash:016x}.bin` (native) or pins frames in-portal (WASM).
- **Binaural HRTF**: Analytic fallback plus **KemarLite** embedded 8-azimuth ITD/ILD profile (v0.1 default); camera yaw rotates pan in `hrtf.rs`.
- **σ phenomenal parity** (`portal_acoustic.rs`): shared `Tensor10D.σ` projects to CIE λ (400–700 nm) on U2 and Hz (1760–110) on U3.
- **Zero-copy browser handoff**: 1024 B `Q3AS` SharedArrayBuffer (`acoustic_sab.rs`) with COOP/COEP bootstrap (`qualia-coi.js`, `coi-serviceworker.js`); **AudioWorklet** stereo grains with overlap-add (`qualia-audio-worklet.js`, `qualia-shell.js` `mountAcousticPlane`).
- **Portal exports**: `set_acoustic_enabled`, `acoustic_uniform_floats`, `create_acoustic_sab`, `publish_acoustic_sab`, `drain_sonic_tokens`, `bake_stft_sidecar_demo`, `bake_cqt_sidecar_demo`, `acoustic_sidecar_pinned`.
- **CI**: 28 `audio::` unit tests; `phenomenal_hrtf` + `phenomenal_sigma_visual_audio_parity`; `phenomenal-verify.mjs` acoustic API oracles in Pages workflow.

### Added — Compute Universe / Phase 8 parity (Track B3)

- **`sentinel_allows_topology_draft()`**: Phase-8 gate on U1→U0 draft batches (0x99 anachronism byte) before `verify_topology_draft_batch`.
- **Shared decode path**: `try_accept_topology_draft()` + `drain_tensor_context_inject()` in `llm_agent.rs` — native threaded and WASM synchronous decode paths now parity-wired (producer start, context inject drain, speculative accept, decode hints).

### Added — Documentation

- **Manual**: [`docs/manuals/qualia-wasm-portal.md`](docs/manuals/qualia-wasm-portal.md) — WASM portal T2 phenomenal path.
- **Standard draft**: [`docs/manuals/standards/q42-acoustic-plane-draft.md`](docs/manuals/standards/q42-acoustic-plane-draft.md) (v0.1 internal).
- **ADR**: [`docs/manuals/adr/0007-u3-acoustic-plane-symbolic-audio.md`](docs/manuals/adr/0007-u3-acoustic-plane-symbolic-audio.md).
- **Test vectors**: [`docs/manuals/standards/vectors/acoustic-plane-v0.1.json`](docs/manuals/standards/vectors/acoustic-plane-v0.1.json).
- **Tracking**: [`docs/plans/AUDIO_PROJECT_STATUS.md`](docs/plans/AUDIO_PROJECT_STATUS.md); migration plan Track B5 / P-F1–F2 marked complete on Pages.

### Fixed

- **SAB header parse**: Read 28-byte `Pod` layout, not 32-byte slot — fixes `pod_read_unaligned` `SizeMismatch` on publish.
- **HRTF test**: Corrected `yaw_rotates_pan` assertion logic.
- **`bake_stft_sidecar_demo`**: Return `Uint8Array::from(&buf[..])` for WASM byte handoff.
- **`AcousticUniform` contract**: Size oracle updated to 328 B in phenomenal contract tests.

### Changed

- Workspace version bump to **0.0.17** (`qualia-core-db`, `qualia-cli`, `qualia-client-core`, sibling crates).
- `publish_acoustic_sab` / `build_acoustic_uniform` now take `&mut self` for rotating sidecar frame playback.

---

## [0.0.13] - 2026-06-16

### Added — 10D Volumetric Tensor System with Zero-Heap Guarantees
- **10D Tensor Coordinate System [q, v, w, x, y, z, t, α, μ, σ]**: Implemented complete 10-dimensional volumetric tensor architecture for Q42 volumes. Zero-heap compatible, stack-allocated structure using fixed-size f32 values for GPU/SIMD compatibility and quantization.
- **Quantum Context (q)**: Manages epistemic superposition with q=0 for ground truth and q>0 for parallel contexts, pending GSR resolutions.
- **Topological Classes (v)**: Implements dynamic distance metrics (Euclidean, Cyclic/Toroidal, Hyperbolic/Tree, Boundary Cliques) for geometric "physics rules" in different regions of the manifold.
- **Manifold Bifurcation (w)**: Multi-head attention identifier isolating separate knowledge universes (Medical, Legal, Personal, Environmental, Socioeconomic) in same memory block for parallel bifurcation and cross-manifold correlation.
- **Spacetime Dimensions (x, y, z, t)**: 3D semantic topology coordinates with temporal state/provenance ledger for immutable historical queries.
- **Spectral-Logical Payload [α, μ, σ]**: EM spectrum foundation replacing simple RGB color space with Amplitude (confidence weight), Modulation (phase/metadata carrier), and Spectral Signature (logical class index).
- **Ground-State Resolver (GSR) Integration**: Async QPU communication for quantum context resolution with classical exhaustion fallback (exhaustive search n≤16, greedy for larger problems), proof-of-demand mesh aggregation, and axiom caching for epistemic frame evolution.
- **Hardware-Tier Dispatching**: Telemetry-aware routing across 4 capability tiers (Edge, Mainstream, High-Performance, QPU) with power-aware and thermal-aware throttling, supporting SIMD-only, Hybrid CPU/NPU, GPU VRAM, and QPU async execution strategies.
- **Q42 Volume Integration**: Bridge layer between NQuin (48-byte) and Tensor10D (40-byte) systems with semantic mapping, tensor search, temporal queries, and manifold queries.

### Changed — Zero-Heap Guarantees
- **Sanctuary Crypto Refactoring**: Eliminated Vec allocations in `encrypt_sanctuary_chunk()` and `decrypt_sanctuary_chunk()` by changing from returning `Vec<u8>` to accepting caller-supplied `&mut [u8]` buffers. Return type changed to `Result<usize, String>` for bytes written.
- **Stack-Allocated Buffers**: Implemented 4KB stack-allocated buffers for typical chunk sizes in cryptographic operations.
- **Zero-Heap API Pattern**: Caller-supplied fixed-capacity buffers prevent dynamic memory allocation in hot paths.

### Added — Cryptographic Infrastructure
- **48-byte PBKDF2 Key Derivation**: Enhanced key derivation splitting into 32-byte cipher key and 16-byte volume root tweak.
- **Implicit Domain-Separated Nonce Derivation**: XOR-based nonce derivation using volume root tweak and chunk index/offset.
- **AEAD Cipher Integration**: Integrated AES-256-GCM with zero-heap guarantees for sanctuary lane cryptography.

### Added — Feature Flags
- **tensor-10d**: Enables 10D tensor coordinate system and all tensor operations
- **tensor-gpu**: Enables GPU acceleration (CUDA/Metal/Vulkan) for tensor operations
- **tensor-npu**: Enables NPU acceleration (Neural Engine) for tensor operations

### Added — Documentation
- **Q42_PIPELINE_CONTAINER_SPEC.md**: Comprehensive architectural specification defining 10D tensor system, pipeline-to-container architecture, hardware capability tiers, zero-heap execution constraints, and implementation phases.

---
## [0.0.12] - 2026-06-14

### Added — Surgical Patch Plan: Solver Unification & v3 Persistence
- **Zero-allocation Solver Infrastructure**: Migrated SolverState and SolverConfig to use packed [u64; 4] payload arrays. Added .cost_value(), .satisfiable(), and .quantum_calls() accessors to strictly enforce 0-allocation bounds in qualia-core-db solvers without relying on brittle structure-size checks.
- **v3 Streaming Volume Persistence (q42_volume.rs)**: Implemented a native StreamingVolumeAppender tailored to extreme memory constraints (off-grid architectures). It successfully replaces the unified volume builder's full-memory load by streaming directly to disk while natively maintaining v3 block indexing (idx, lock_dir, DAG Merkle bytes) incrementally at the end of the payload.
- **Persistence Worker Wire-up**: Re-wired spawn_persistence_worker in webizen-desktop to stream simulation blocks via sector-aligned QUINS_PER_BLOCK chunks instead of monolithic buffers, strictly securing the legacy-free v3 architecture across local Desktop environments.
- **Legacy Purge**: Eliminated trailing v2 scaffolding across the workspace, replacing is_v2_volume detection heuristics with is_unified_volume flags and terminating unsupported configurations.


### Added
- Integrated native async `QTensorEngine` for WASM execution over WebGPU
- Deployed locally hosted GGUF model downloader via `coi-serviceworker`
- Exported WASM WebGPU module via `wasm-pack` into `docs/llmdemo/pkg`
- **Zero-allocation payload packing**: `xsd:integer`, `xsd:decimal`, and `xsd:boolean` datatypes are now intercepted during semantic parsing (`qualia-core-db` & `qualia-cli`) and packed natively into the 60-bit payload, enforcing two's complement and fixed-point scale boundaries without intermediate `String` allocations.

### Fixed
- Addressed `mcp_server.rs` build failure caused by missing `.to_string()` conversions for integers.
- **resolver.rs**: Corrected `write_object_term` to properly sign-extend 60-bit `xsd:integer` and `xsd:decimal` payloads on display, preventing negative numbers from appearing as large unsigned positive values.

### Added — CLI ETL Pipeline & Full RDF/SPARQL Exposure

**Format auto-detection** (`crates/qualia-cli/src/ingest/detect.rs`):
- `SemanticFormat` enum covering all 16 supported formats
- `detect_format(path)` — two-stage: file-extension hint + 16-byte magic-byte probe (Q42 `b"Q42"`, CBOR-LD `0xd9 0xd9 0xf7`, XML envelope, JSON envelope, QCHK)

**Ingest pipeline — allocation-safe rewrite**:
- `ingest_ntriples` and `ingest_rdf_xml` rewritten to use `ExternalSorter` (out-of-core K-way merge, ≤48 MB peak); eliminates the unbounded `Vec<NQuin>` + `HashMap<u64,String>` that OOM'd on any non-trivial file
- `ingest_cbor` / `ingest_kml` — 256 MB file-size guard; `ingest_kml` now uses `memmap2::Mmap` instead of `read_to_end` (eliminates heap copy of raw bytes)
- Query vault load — `std::fs::read(&vault)` replaced with `memmap2::Mmap::map` throughout the `query` command

**All RDF/RDF-Star/N3 parsers wired to `ingest semantic <file>`** (`ingest/mod.rs`):
- `stream_ingest!` macro generates zero-boilerplate streaming wrappers for all parser variants
- New CLI ingest functions: `ingest_ntriples_star`, `ingest_nquads`, `ingest_nquads_star`, `ingest_turtle`, `ingest_trig`, `ingest_trig_star`, `ingest_n3`, `ingest_json_ld`, `ingest_json_ld_star`
- `ingest_auto(input, output)` — top-level dispatcher: runs `detect_format`, routes to correct function
- `IngestFormat::Semantic { file }` handler uses `ingest_auto`; prints detected format, triple/block counts, output path
- Added `parse_n3_star_stream` to `n3_star.rs` in `qualia-core-db` (was the only parser missing a streaming entry point)

**SPARQL query command — native engine**:
- `Commands::Query { Sparql | SparqlStar }` now runs the full native pipeline: `parse_sparql` → `QueryPlanner::plan` → `memmap2` vault → `QueryExecutor::execute`
- `run_sparql_query(vault, qs)` helper in `main.rs`; results streamed to stdout row-by-row
- Both `sparql` and `sparql-star` dialects use the same engine (the native parser already handles RDF-Star embedded triples)

**SHACL mapping compiler** (`ingest/mapper.rs`):
- `compile_shacl_mapping(path)` — boot-phase lightweight Turtle parser; extracts `@prefix` declarations, `sh:targetClass`, and all `sh:property [...]` blocks containing `qext:sourceColumn` or `qext:sourceJsonKey`
- Maps `sh:datatype` → `TargetDatatype::{Integer,Float,DateTime,StringRef}`
- Replaces the previous `unimplemented!()` stub; CSV and JSON ingest now fully operational

**CSV/JSON streamer — all datatype arms complete** (`csv_mapper.rs`, `json_mapper.rs`):
- `DateTime` arm: parses RFC3339 / ISO8601 / date-only strings via `chrono`; stores as `(0b011u64 << 60) | unix_millis` (XSD dateTime inline tag)
- `StringRef` arm: hashes string via `hash_token` (FNV-1a); stores object hash (consistent with IRI hashing elsewhere)

**Bug fix** (`json_ld_stream.rs`):
- `hash_str` previously used `DefaultHasher` (non-deterministic across process restarts, incompatible with rest of engine); replaced with `hash_token` (FNV-1a)

### Tests
- 16 new unit tests in `ingest/detect.rs` and `ingest/mapper.rs`; all passing
- Covers: extension-based detection, magic-byte probing (Q42, QCHK, CBOR-LD, XML, KML, JSON), SHACL field extraction, datatype mapping, predicate hash consistency, error cases

### Added — SocialWebNet Fiduciary Supremacy & Fifth Vector Routing (2026-06-12)

**Fiduciary Supremacy & Sanctuary Mode**:
- `webizen_server.rs`: Added the RPC bridge (`window.webizen`) intercept point ensuring edge devices (mobile/WASM apps) must use the native node.
- Enforced **Sanctuary Mode**: `POST /api/v1/webizen/rpc` will return `423 Locked` and sever access to `/sovereign` domains when the daemon is in Sanctuary mode.

**Fifth Vector Handshakes (`daemon_swarm.rs`)**:
- Implemented `DnssecSemanticPayload` serialization and parsing over CBOR-LD/DNSSEC TXT records.
- WireGuard tunnels are strictly gated by bitmask evaluation (`routing_mask`) against the local `CompiledPermission`. The SocialWebNet WireGuard stack runs *exclusively* on native installs.

**CRDT Bifurcation (`crdt.rs`)**:
- Sovereign `wf:` (WellFair) domains are explicitly exempted from automated LWW merges. These require Tri-Party Access and manual user authorization.
- Only Commons domains (`qp:`) undergo Lamport clock-based LWW consensus.

### Added — CLI Logic Modality & Science Surface (2026-06-12)

**`qualia-cli evaluate <modality>` — 16 subcommands (15 modalities + neuro-symbolic)**:
- `ltl`, `asp`, `dl`, `probabilistic`, `linear-logic`, `dialectical`, `diffusion`, `spatio-temporal`, `interval`, `graph-topology`, `argumentation`, `control-feedback` (12 new in previous session)
- `neuro-symbolic` — demonstrates `SieveLexSpec::fever_observation()` + `graph_mutation_default()`, showing token-level SHACL constraint masks and their FNV-1a hashes

**`qualia-cli solve <group>` — extended with 3 new groups**:
- `solve ode rk4|harmonic|bvp|quantum-spectrum` — `Rk4Solver`, `ShootingMethod`, `QuantizationMapper`
- `solve quantum qaoa|spsa` — `QAOAAngleOptimizer`, `SpsaOptimizer` with `DemoQaoa`/`DemoSpsa` adapters
- `solve symbolic defeasible|sat` — `ForwardChainingDefeasible`, `BoundedSatSolver`

**`qualia-cli science <domain>` — new top-level command (7 subdomains, 23 runners)**:
- `chem smiles|thermo|drug-like|pka` — SMILES descriptors, Arrhenius/Gibbs/HH, Lipinski/Veber/Ghose/Egan/BBB, ionisation fraction
- `bio align|kmer|translate|isoelectric|jaccard|minhash` — Smith-Waterman/Needleman-Wunsch, k-mer frequencies, DNA→protein, pI, MinHash Jaccard
- `geo embed-h3` — H3 index → NQuin context hash
- `thermo gibbs|anneal` — `ThermodynamicSampler` Gibbs free energy and Metropolis-Hastings acceptance
- `geometric cross|angle` — `geometric_algebra::utils::{cross_product, dot_product, angle_between_vectors, rad_to_deg}`
- `clinical framingham|sofa|ckd|pk|drug-interactions` — Framingham CVD risk, SOFA score, CKD-EPI eGFR + Cockcroft-Gault, 1-compartment IV PK, DDI screening
- `economics gbm|var|macro` — GBM path simulation, Monte Carlo 95% VaR, macroeconomic MV=PQ flow

**MCP surface extension (`mcp_server.rs`) — 6 new tools**:
- `evaluate_modality` — routes `ltl|asp|probabilistic|argumentation` to Webizen VM evaluators
- `bioinformatics_align` — `align_nucleotide`/`align_protein` dispatch
- `chemical_descriptors` — SMILES → `parse_smiles` + `compute_descriptors`, returns MW proxy
- `clinical_risk` — Framingham 10-year CVD risk score dispatch
- `symbolic_logic_infer` — `ForwardChainingDefeasible` or `BoundedSatSolver` dispatch
- `geometric_algebra_op` — `cross_product` or `angle_between_vectors` dispatch

**New source files**: `crates/qualia-cli/src/solve.rs` (extended), `crates/qualia-cli/src/science.rs` (new)
**Modified source files**: `evaluate.rs` (+`run_neuro_symbolic`), `main.rs` (+`Commands::Science`, +`SolveAction::{Ode,Quantum,Symbolic}`, +`EvaluateModality::NeuroSymbolic`, +all dispatch arms), `mcp_server.rs` (+6 tool handlers)

### Added — QPU Provider Configuration (2026-06-12)

**`qualia-cli --enable-qpu qpu <subcommand>`** — runtime-gated QPU management:
- `--enable-qpu` global flag on `Cli` struct (replaces compile-time `#[cfg(feature = "qpu_internal")]` stub)
- Credentials stored in `$QUALIA_DATA_DIR/qpu_config.json` (JSON, keys masked on display)

**Supported providers (8 total)**:

| ID | Provider | Problem types | Key credentials |
|----|----------|--------------|----------------|
| `ibm` | IBM Quantum | gate-model, vqe, qaoa | `api_key`, `hub`, `group`, `project` |
| `dwave` | D-Wave Leap | annealing (QUBO) | `api_key` |
| `ionq` | IonQ | gate-model | `api_key`, `backend` |
| `rigetti` | Rigetti QCS | gate-model, vqe, qaoa | `api_key`, `user_id`, `qpu_id` |
| `azure` | Azure Quantum | gate-model, annealing | `subscription_id`, `resource_group`, `workspace`, `location` |
| `braket` | AWS Braket | gate-model, annealing | `access_key_id`, `secret_access_key`, `region` |
| `google` | Google Quantum AI | gate-model | `project_id`, `processor_id` |
| `quantinuum` | Quantinuum | gate-model | `api_key`, `machine` |

**Subcommands**:
- `list-providers` — show all 8 providers with required fields and docs links
- `configure <provider> [--field value ...]` — set/update credentials (partial updates supported; existing fields not overwritten by omission)
- `show [--provider <id>]` — display config for all or one provider (API keys masked)
- `clear <provider>` — remove a provider's stored credentials
- `test-connection <provider>` — validate required fields are present; print endpoint and auth method
- `submit <provider> [--problem-type annealing|gate-model|vqe|qaoa] [--qubits N] [--shots N]` — local classical simulation via `FallbackHandler`; live dispatch via `qualia-cli daemon`

**New source file**: `crates/qualia-cli/src/qpu.rs`
**Modified**: `crates/qualia-cli/src/main.rs` — `Commands::Qpu` now always compiled, gated at runtime; `QpuAction` fully implemented

### Added — Specialized Libraries: All 9 Enabled (2026-06-12)

All `specialized_libs/` modules are now fully compiled and tested (79/79 tests passing). Previously blocked by build errors from prior sessions; all remaining stubs replaced with real implementations:

- **`cryptographic_library`** — real AES-256-GCM (`aes-gcm 0.10`) + Ed25519 (`ed25519-dalek 2.1`); `generate_iv()` and `rotate_key()` use `rand::random::<[u8; N]>()` (rand 0.10 API)
- **`linear_algebra`** — LU decomposition, matrix multiply, SVD, eigenvalue routines; ZK proof pipeline via `zk_proofs.rs`
- **`statistical_computing`** — descriptive stats, regression, hypothesis testing, distribution samplers
- **`physics_simulation`** — Burgers CFD solver (3 output fields per node: velocity / pressure / temperature), distributed simulation, wave propagation
- **`machine_learning`** — gradient descent, neural network training, decision tree, clustering
- **`financial_modeling`** — TVM, Black-Scholes, Monte Carlo VaR, portfolio optimisation
- **`chemistry_modeling`** — SMILES descriptors, reaction simulation, molecular dynamics
- **`medical_computing`** — Framingham / SOFA / CHA₂DS₂-VASc scoring, clinical decision support
- **`engineering_analysis`** — FEA, structural analysis, thermal analysis, CFD coupling

**Bug fixed** (`zk_proofs.rs`): `ZkProof.verification_key_id` was being set to the proving key ID (`"pk_circuit_..."`) but the verifying key was stored under `circuit_id`. Fixed to use `circuit_id.to_string()` — resolves `KeyNotFound` panic in `linear_algebra::private_matrix_multiply` test.

### Added — Cross-Platform Abstraction Layer (2026-06-12)

Three new modules added to `qualia-core-db` providing real platform-native implementations for storage, thread scheduling, and network filtering. Driven by `local/resolving_platform_issues.md`.

**`storage_driver.rs`** — `StorageDriver` trait with four platform-specific backends:
- `MmapDriver` — fully file-backed via `memmap2`; portable across all platforms; real file snapshot (per-file copy)
- `MmapApfsDriver` (macOS) — `madvise(MADV_WILLNEED/FREE)` async prefetch/release; `clonefile(2)` O(1) APFS snapshot (zero extra disk); `fcntl(F_NOCACHE=48)` bypass for WAL sequential writes; `F_FULLFSYNC` flush-through Apple ANS write queue — all via libc behind `cfg!(target_os = "macos")`
- `WinNvmeDriver` (Windows) — `CreateFileW("\\\\.\\\PhysicalDriveN")` + `DeviceIoControl(IOCTL_STORAGE_QUERY_PROPERTY = 0x002D_1400)` to probe NVMe hardware across drives 0–7; falls back to `MmapDriver` without admin privilege
- `ZnsDriver` (Linux) — wraps existing `ZnsZoneManager` with zone-append + file overlay
- `running_under_wsl2()` — detects WSL2 via `/proc/version` contains "microsoft"; `log_startup_diagnostics()` emits actionable warning for `networkingMode=mirrored` (required for port 4242 in WSL2)
- `open_storage(data_dir)` — platform factory: Linux → ZNS or Mmap, Windows → WinNvme, macOS → MmapApfs, else → Mmap
- `NetworkFilter` trait + `NoopFilter` + `open_network_filter()` delegating to `ebpf_filter`
- 10 unit tests, all passing

**`platform_scheduler.rs`** — `QosClass` enum + `bind_current_thread(class)` for thread placement:
- macOS — `pthread_set_qos_class_self_np(qos_class, 0)` via custom FFI; maps `UserInteractive` → P-cores (efficiency), `Background` → E-cores (efficiency); correct for Apple Silicon AMP
- Linux — `core_affinity` P/E-core split (lower-numbered = P-cores heuristic) + `libc::setpriority` nice levels (-10 for UserInteractive, 19 for Background)
- Windows — `SetThreadPriority` via `windows` crate (HIGHEST → IDLE)
- Convenience wrappers: `bind_inference_thread()` → UserInteractive; `bind_background_thread()` → Background
- 3 unit tests, all passing

**`ebpf_filter.rs`** — `NetworkFilter` trait with real per-platform implementations:
- `EbpfLinuxFilter` (Linux) — real `bpf(SYS_bpf, BPF_PROG_LOAD)` syscall; cBPF pass-all bytecode; WSL2 note logged; `Drop` closes fd
- `WfpFilter` (Windows) — `FwpmEngineOpen0` / `FwpmFilterAdd0` / `FwpmEngineClose0` WFP BFE session; handle stored as `isize` (Send+Sync safe); `FwpmEngineOpen0` returns `u32` DWORD (ERROR_SUCCESS = 0)
- `MacNetworkExtFilter` (macOS) — `xpc_connection_create_mach_service("com.qualiadb.netfilter")`; degrades to noop gracefully when Apple Network Extension entitlement not installed; `Drop` calls `xpc_release`
- `AndroidVpnFilter` (Android) — VpnService TUN fd bridge; noop when fd=-1
- `open_platform_filter()` factory dispatches to the correct implementation per target
- 6 unit tests, all passing

**`ARCHITECTURE.md §43`** added: full cross-platform documentation — storage driver capability matrix, thread QoS usage conventions, network filter per-platform notes, mobile platform matrix (iOS/Android).

---

## [0.0.13] - 2026-06-16

### Added — 10D Volumetric Tensor System with Zero-Heap Guarantees
- **10D Tensor Coordinate System [q, v, w, x, y, z, t, α, μ, σ]**: Implemented complete 10-dimensional volumetric tensor architecture for Q42 volumes. Zero-heap compatible, stack-allocated structure using fixed-size f32 values for GPU/SIMD compatibility and quantization.
- **Quantum Context (q)**: Manages epistemic superposition with q=0 for ground truth and q>0 for parallel contexts, pending GSR resolutions.
- **Topological Classes (v)**: Implements dynamic distance metrics (Euclidean, Cyclic/Toroidal, Hyperbolic/Tree, Boundary Cliques) for geometric "physics rules" in different regions of the manifold.
- **Manifold Bifurcation (w)**: Multi-head attention identifier isolating separate knowledge universes (Medical, Legal, Personal, Environmental, Socioeconomic) in same memory block for parallel bifurcation and cross-manifold correlation.
- **Spacetime Dimensions (x, y, z, t)**: 3D semantic topology coordinates with temporal state/provenance ledger for immutable historical queries.
- **Spectral-Logical Payload [α, μ, σ]**: EM spectrum foundation replacing simple RGB color space with Amplitude (confidence weight), Modulation (phase/metadata carrier), and Spectral Signature (logical class index).
- **Ground-State Resolver (GSR) Integration**: Async QPU communication for quantum context resolution with classical exhaustion fallback (exhaustive search n≤16, greedy for larger problems), proof-of-demand mesh aggregation, and axiom caching for epistemic frame evolution.
- **Hardware-Tier Dispatching**: Telemetry-aware routing across 4 capability tiers (Edge, Mainstream, High-Performance, QPU) with power-aware and thermal-aware throttling, supporting SIMD-only, Hybrid CPU/NPU, GPU VRAM, and QPU async execution strategies.
- **Q42 Volume Integration**: Bridge layer between NQuin (48-byte) and Tensor10D (40-byte) systems with semantic mapping, tensor search, temporal queries, and manifold queries.

### Changed — Zero-Heap Guarantees
- **Sanctuary Crypto Refactoring**: Eliminated Vec allocations in `encrypt_sanctuary_chunk()` and `decrypt_sanctuary_chunk()` by changing from returning `Vec<u8>` to accepting caller-supplied `&mut [u8]` buffers. Return type changed to `Result<usize, String>` for bytes written.
- **Stack-Allocated Buffers**: Implemented 4KB stack-allocated buffers for typical chunk sizes in cryptographic operations.
- **Zero-Heap API Pattern**: Caller-supplied fixed-capacity buffers prevent dynamic memory allocation in hot paths.

### Added — Cryptographic Infrastructure
- **48-byte PBKDF2 Key Derivation**: Enhanced key derivation splitting into 32-byte cipher key and 16-byte volume root tweak.
- **Implicit Domain-Separated Nonce Derivation**: XOR-based nonce derivation using volume root tweak and chunk index/offset.
- **AEAD Cipher Integration**: Integrated AES-256-GCM with zero-heap guarantees for sanctuary lane cryptography.

### Added — Feature Flags
- **tensor-10d**: Enables 10D tensor coordinate system and all tensor operations
- **tensor-gpu**: Enables GPU acceleration (CUDA/Metal/Vulkan) for tensor operations
- **tensor-npu**: Enables NPU acceleration (Neural Engine) for tensor operations

### Added — Documentation
- **Q42_PIPELINE_CONTAINER_SPEC.md**: Comprehensive architectural specification defining 10D tensor system, pipeline-to-container architecture, hardware capability tiers, zero-heap execution constraints, and implementation phases.

---
## [0.0.12] - 2026-06-11

### Summary

v0.0.12 resolves all build errors (82 -> 0), ships a complete SPARQL 1.1/1.2 engine (138 tests),
 (82 → 0), ships a complete SPARQL 1.1/1.2 engine (138 tests),
implements the Q42 v3 format with Merkle-DAG and temporal SPARQL extensions (Phases 1–4),
adds Zero-Copy LoRA Multiplexing, 8-provider QPU dispatch, platform-native GPU inference pipelines,
SHACL bioscience/biomedical/organic-chemistry extensions, credential-gated subgraphs, and
real implementations for previously-stubbed security and query primitives.

---

### Fixed — Build System

- **All 82 build errors resolved**: Project compiles with 0 errors on all target platforms
- **Tokio runtime nesting**: Fixed `Handle::current()` calls with `try_current` fallback for wgpu async
- **Module reorganization**: Completed all path references and imports
- **SPARQL engine (64 additional errors)**: Resolved type mismatches, lifetime issues, missing impls across 16 SPARQL modules post-initial-ship

---

### Added — SPARQL Engine (7,074 lines across 16 modules)

- **Complete SPARQL 1.1/1.2**: Zero-allocation architecture with index-based AST; fixed-size arrays, no `Vec`/`String`/`Box` in hot paths, ~35 KB per query budget
- **Core**: SELECT, ASK, CONSTRUCT, DESCRIBE, FILTER, aggregates (COUNT/SUM/AVG/MIN/MAX), GROUP BY, HAVING, DISTINCT, LIMIT/OFFSET, ORDER BY
- **Patterns**: OPTIONAL, UNION, GRAPH, Property Paths (7 types), Subqueries
- **SPARQL-Star / RDF-Star**: Embedded triples with provenance tracking, Virtual ID Hash strategy
- **W3C extensions**: SPARQL Update, SHACL-SPARQL, GeoSPARQL (OGC), SPARQL-MM, Federated Query (`SERVICE`)
- **DID integration**: `sparql_did.rs` — federated queries with DID authentication, CORS handling; 399-line ReSpec specification
- **WebSocket endpoint**: `sparql_websocket.rs` — live SPARQL subscription over WebSocket
- **HTTP endpoint**: `sparql_endpoint.rs` — SPARQL 1.1 Protocol-compliant HTTP endpoint
- **Testing**: 138 tests passing (up from 45 at initial ship)

### Added — SPARQL Temporal Extension (`AS OF` / `AT TIME`) — Phase 4

- **`TemporalMode` enum** (`sparql_ast.rs`): `AsOf = 0` (assertion-time), `AtTime = 1` (valid-time); `#[repr(u8)]` + `Copy`
- **`Pattern::AsOf`** variant: wraps any inner pattern with `timestamp_ms: u64` + `TemporalMode`
- **`PhysicalOperatorType::AsOf`** (`sparql_planner.rs`): plans the temporal filter in the physical plan
- **`execute_as_of` + `check_temporal_constraint`** (`sparql_executor.rs`): filters candidates via `T_CONTEXT` PROV-O quins (`generatedAtTime` / `startedAtTime` / `endedAtTime`); open-world (no annotation = include)
- **Parser** (`sparql_parser.rs`): recognises `AS OF` and `AT TIME` after the closing `}` of the WHERE clause; `parse_temporal_literal` accepts integer ms or `"YYYY-MM-DD"^^xsd:dateTime`
- **Syntax**: `SELECT ... WHERE { ... } AS OF "2024-06-01"^^xsd:dateTime` or `... AT TIME 1717286400000`

---

### Added — Q42 v3 Format

- **v3 header** (`q42_volume.rs`): `temporal_index_offset/length`, `merkle_root [u8;32]`, `assertion_timestamp`, `dag_root_offset/length` carved from the former `_reserved` region `[88..256]`
- **v2 hard-rejection**: `verify_version()` requires version == 3; old v2 files fail with a descriptive error
- **`migrate_v2_to_v3()`**: in-place one-pass upgrade populating new header fields with zero/default sentinels
- **NQuin v3 bit-layout**: bits 63–48 of the metadata field reserved for LoRA adapter routing (see LoRA section)
- **Ingest Pipeline DAG wiring**: `streaming_import_rdf` in `ingest.rs` upgraded to generate full v3 unified `Q42Volume` formats (with valid V3 headers, Block Directory, and DagStore serialization) instead of legacy `.c.q42` stream format.

### Added — Merkle-DAG (`git_bridge.rs`) — Phases 1 & 4

- **`DagNode`** / **`DagStore`**: content-addressed 32-byte SHA-256 hash nodes, flat on-disk slab
- **`MERGE_SECONDARY: u32 = 0x0008`**: flag for secondary-parent back-links in merge commits
- **`merge_node(primary, secondary, quins, author_did, timestamp_ms, message)`**: creates two linked `DagNode`s (primary commit + secondary back-link); returns `(primary_hash, secondary_hash)`
- **`nodes_as_of(ms: u64)`**: assertion-time snapshot filter — returns all node hashes with `timestamp <= ms`
- **WAL → DagStore linking** (`wal.rs`): 32-byte header extended with `prev_dag_hash`; `checkpoint_to_dag()` commits WAL segments to DAG; `buffered_count()` for backpressure

### Added — Temporal Graph & Provenance — Phase 2

- **`temporal_graph.rs`**: `TemporalGraph` — assertion-time and valid-time edges, bi-temporal indexing
- **`provenance.rs`**: PROV-O `Entity`/`Activity`/`Agent` quins; `CONTEST_CONTEXT` for contested-provenance tracking
- **`spatial_sieve.rs`**: upgraded from stub to real GeoSPARQL quins using `kml_bridge::T_CONTEXT`
- **`kml_bridge.rs`**: KML geometry ingest to NQuin spatial predicates; `T_CONTEXT = q_hash("urn:qualia:context:temporal")` public const
- **CogAI orchestrator pre-flight** (`orchestrator.rs`): W3C CogAI CG agent-structure SHACL validation before every inference call

### Added — Credential-Gated Subgraphs — Phase 3

- **`SubgraphLayer` / `SubgraphKey`** (`rdf_star.rs` / `sentinel.rs`): AES-256-GCM encrypted subgraphs with HKDF-derived per-layer keys
- **X25519 ECDH encapsulation**: ephemeral key exchange for subgraph key delivery
- **ODRL policy evaluation** (`deontic_logic.rs`): `odrl:Permission` / `odrl:Prohibition` quins gate subgraph access
- **PROV-O provenance filter** (`sparql_filter.rs`): `prov_predicates` module — `GENERATED_AT_TIME`, `STARTED_AT_TIME`, `ENDED_AT_TIME` as compiled constants

### Added — Ontology Vocabulary Integration

- **Temporal**: PROV-O (`prov:generatedAtTime`, `prov:startedAtTime`, `prov:endedAtTime`) + DC Terms
- **Spatial**: GeoSPARQL + KML geometry bridge
- **Rights**: ODRL (`odrl:Permission`, `odrl:Prohibition`) + SKOS concept schemes
- **Agent structure**: W3C CogAI CG vocabulary + SHACL conformance profiles

---

### Added — Native-First WASM-LLM Offloading

- **`extension_bus::wasm_bus`**: Implemented `did:q42` WebSocket handshake and event routing for WASM targets (`qualia-core-db/src/extension_bus.rs`).
- **Zero-Allocation Sync/Async Bridge**: Refactored `llm_agent.rs` WASM execution path to intercept inference requests and cleanly pipe synchronous LLM traits into the non-blocking Dioxus `on_token` event loop callback.
- **WASM Extension Fallback**: The WASM inference pipeline now intelligently escalates to the Qualia Native Daemon (port 4242) if installed, or gracefully falls back to the in-browser WebGPU engine if not.

---

### Added — Zero-Copy LoRA Multiplexing

- **`lora/` module**: `LoraAdapter` (rank-r weight deltas, alpha scaling), `LoraMux` (mux table, up to 16 concurrent adapters)
- **GPU shader** (`shaders/wgsl/lora_projection.wgsl`): fused LoRA projection compute shader (A x B + base weight), 64 threads/workgroup
- **NQuin routing** (`gguf_bridge.rs`): bits 63–48 of metadata field encode adapter ID; `LocalLlmAgent::infer()` selects adapter from `NQuin` context before each forward pass
- **Zero heap**: adapter weights mapped via `memmap2`, no heap copy; `LoraGuard` RAII ensures clean unload

---

### Added — QPU Dispatch (`solvers/qpu/`)

- **8 providers**: IBM Quantum, D-Wave, IonQ, Rigetti, Azure Quantum, AWS Braket, Google Quantum AI, Quantinuum
- **`QpuDispatcher`**: provider-agnostic trait; selects provider from `QpuConfig` or session Principal VC
- **Commitment activation** (Tauri desktop): `activate_qpu_commitment()` FRB binding — prompts Principal consent before any QPU job submission
- **WAL logging**: QPU job submissions and results recorded as provenance quins

---

### Added — GPU Inference Pipelines (Platform-Native)

- **Windows — DirectML 1.15**: `wgpu` backend targeting DirectML; real quantized GEMM via `fused_transformer.wgsl`
- **macOS — Accelerate / AMX**: `cblas_sgemm` via `Accelerate.framework`; AMX matrix engine enabled for Apple Silicon
- **Linux — wgpu / Vulkan**: real `fused_tensor_contraction.wgsl` pipeline (replaces `mock_pipeline`); 64 threads/workgroup, 4096 FMA ops per thread
- **`infer_local_model()`**: Phase 8 bifurcated autoregressive loop (LLM engine thread <-> Webizen Sentinel thread via SPSC ring buffers) now runs the real pipeline on all host targets; WASM retains mock path

---

### Added — SHACL Extension Modules

- **Biosciences** (`shacl/biosciences.rs`): gene ontology constraints, sequence annotation shapes
- **Biomedical** (`shacl/biomedical.rs`): SNOMED CT, MeSH, ICD-10 constraint validation
- **Organic chemistry** (`shacl/organic_chemistry.rs`): SMILES/InChI structural constraints, isotope rules
- **SlgOpcode wiring**: new `NativeBiosciencesEval`, `NativeBiomedicalEval`, `NativeOrganicChemEval` opcodes
- **WASM exposure**: all three engines callable from the browser test suite
- **149 tests** for SHACL extension modules

### Added — Domain Crates (6 compiled)

- `domains/bioinformatics` — sequence alignment, phylogenetic distance
- `domains/organic_chemistry` — reaction SMILES, isotope distribution (fixed mass accumulation bug)
- `domains/thermodynamics` — Gibbs energy, entropy calculations
- `domains/geometric` — geometric algebra SIMD kernel (real SIMD lanes, no scalar fallback)
- `domains/financial` — time-value of money, portfolio risk metrics
- `domains/geospatial` — GeoSPARQL extension functions, WKT geometry

---

### Fixed — Security & Query Stubs Replaced with Real Implementations

- **ECC parity check** (`q42_lex.rs`): real P-256 scalar validation; replaces always-true stub
- **`FiduciaryCrypto::sign()` / `verify()`** (`fiduciary_crypto.rs`): wired to `ed25519-dalek`; replaces `unimplemented!()`
- **ZK structural validation** (`zk_proofs.rs`): Pedersen commitment structure check; placeholder proof rejected
- **`mmap_query_subject`** (`q42_volume.rs`): real mmap scan over SuperBlock index; replaces empty-return stub
- **`QuinIndex::lookup()`** (`lexicon.rs`): B-tree subject index; replaces always-miss stub
- **wgpu real pipeline** (`gguf_bridge.rs`): `build_real_pipeline()` replaces `mock_pipeline` on all host targets

---

### Added — Test Infrastructure

- **271-test browser suite** (`docs/api-explorer/`): WASM / Native / Both execution modes; interactive per-test log viewer
- **Interactive API Explorer**: live query execution against the running daemon; endpoint catalog with copy-to-clipboard
- **Total test count**: 640+ across all crates (138 SPARQL, 149 SHACL extensions, 8 git_bridge, remainder spread across core, domains, CLI)

---

## [0.0.13] - 2026-06-16

### Added — 10D Volumetric Tensor System with Zero-Heap Guarantees
- **10D Tensor Coordinate System [q, v, w, x, y, z, t, α, μ, σ]**: Implemented complete 10-dimensional volumetric tensor architecture for Q42 volumes. Zero-heap compatible, stack-allocated structure using fixed-size f32 values for GPU/SIMD compatibility and quantization.
- **Quantum Context (q)**: Manages epistemic superposition with q=0 for ground truth and q>0 for parallel contexts, pending GSR resolutions.
- **Topological Classes (v)**: Implements dynamic distance metrics (Euclidean, Cyclic/Toroidal, Hyperbolic/Tree, Boundary Cliques) for geometric "physics rules" in different regions of the manifold.
- **Manifold Bifurcation (w)**: Multi-head attention identifier isolating separate knowledge universes (Medical, Legal, Personal, Environmental, Socioeconomic) in same memory block for parallel bifurcation and cross-manifold correlation.
- **Spacetime Dimensions (x, y, z, t)**: 3D semantic topology coordinates with temporal state/provenance ledger for immutable historical queries.
- **Spectral-Logical Payload [α, μ, σ]**: EM spectrum foundation replacing simple RGB color space with Amplitude (confidence weight), Modulation (phase/metadata carrier), and Spectral Signature (logical class index).
- **Ground-State Resolver (GSR) Integration**: Async QPU communication for quantum context resolution with classical exhaustion fallback (exhaustive search n≤16, greedy for larger problems), proof-of-demand mesh aggregation, and axiom caching for epistemic frame evolution.
- **Hardware-Tier Dispatching**: Telemetry-aware routing across 4 capability tiers (Edge, Mainstream, High-Performance, QPU) with power-aware and thermal-aware throttling, supporting SIMD-only, Hybrid CPU/NPU, GPU VRAM, and QPU async execution strategies.
- **Q42 Volume Integration**: Bridge layer between NQuin (48-byte) and Tensor10D (40-byte) systems with semantic mapping, tensor search, temporal queries, and manifold queries.

### Changed — Zero-Heap Guarantees
- **Sanctuary Crypto Refactoring**: Eliminated Vec allocations in `encrypt_sanctuary_chunk()` and `decrypt_sanctuary_chunk()` by changing from returning `Vec<u8>` to accepting caller-supplied `&mut [u8]` buffers. Return type changed to `Result<usize, String>` for bytes written.
- **Stack-Allocated Buffers**: Implemented 4KB stack-allocated buffers for typical chunk sizes in cryptographic operations.
- **Zero-Heap API Pattern**: Caller-supplied fixed-capacity buffers prevent dynamic memory allocation in hot paths.

### Added — Cryptographic Infrastructure
- **48-byte PBKDF2 Key Derivation**: Enhanced key derivation splitting into 32-byte cipher key and 16-byte volume root tweak.
- **Implicit Domain-Separated Nonce Derivation**: XOR-based nonce derivation using volume root tweak and chunk index/offset.
- **AEAD Cipher Integration**: Integrated AES-256-GCM with zero-heap guarantees for sanctuary lane cryptography.

### Added — Feature Flags
- **tensor-10d**: Enables 10D tensor coordinate system and all tensor operations
- **tensor-gpu**: Enables GPU acceleration (CUDA/Metal/Vulkan) for tensor operations
- **tensor-npu**: Enables NPU acceleration (Neural Engine) for tensor operations

### Added — Documentation
- **Q42_PIPELINE_CONTAINER_SPEC.md**: Comprehensive architectural specification defining 10D tensor system, pipeline-to-container architecture, hardware capability tiers, zero-heap execution constraints, and implementation phases.

---
## [0.0.12] — 2026-06-09

### Summary

v0.0.12 addressed initial build error fixing phase, resolving 38 of 82 errors through straightforward corrections and module reorganization.

### Fixed — Build Errors (Partial)

- **38 build errors fixed**: Type mismatches, API usage, module imports
- **qualia-extensions rewired**: Now uses native Qualia LLM pipeline instead of Candle
- **q42_lexicon.rs**: Implemented properly with all required types and methods
- **Module reorganization**: Fixed imports across webizen.rs and related files

### Remaining (Resolved in v0.0.12)

- 44 build errors required architectural fixes (all resolved in v0.0.12)

---

## [0.0.13] - 2026-06-16

### Added — 10D Volumetric Tensor System with Zero-Heap Guarantees
- **10D Tensor Coordinate System [q, v, w, x, y, z, t, α, μ, σ]**: Implemented complete 10-dimensional volumetric tensor architecture for Q42 volumes. Zero-heap compatible, stack-allocated structure using fixed-size f32 values for GPU/SIMD compatibility and quantization.
- **Quantum Context (q)**: Manages epistemic superposition with q=0 for ground truth and q>0 for parallel contexts, pending GSR resolutions.
- **Topological Classes (v)**: Implements dynamic distance metrics (Euclidean, Cyclic/Toroidal, Hyperbolic/Tree, Boundary Cliques) for geometric "physics rules" in different regions of the manifold.
- **Manifold Bifurcation (w)**: Multi-head attention identifier isolating separate knowledge universes (Medical, Legal, Personal, Environmental, Socioeconomic) in same memory block for parallel bifurcation and cross-manifold correlation.
- **Spacetime Dimensions (x, y, z, t)**: 3D semantic topology coordinates with temporal state/provenance ledger for immutable historical queries.
- **Spectral-Logical Payload [α, μ, σ]**: EM spectrum foundation replacing simple RGB color space with Amplitude (confidence weight), Modulation (phase/metadata carrier), and Spectral Signature (logical class index).
- **Ground-State Resolver (GSR) Integration**: Async QPU communication for quantum context resolution with classical exhaustion fallback (exhaustive search n≤16, greedy for larger problems), proof-of-demand mesh aggregation, and axiom caching for epistemic frame evolution.
- **Hardware-Tier Dispatching**: Telemetry-aware routing across 4 capability tiers (Edge, Mainstream, High-Performance, QPU) with power-aware and thermal-aware throttling, supporting SIMD-only, Hybrid CPU/NPU, GPU VRAM, and QPU async execution strategies.
- **Q42 Volume Integration**: Bridge layer between NQuin (48-byte) and Tensor10D (40-byte) systems with semantic mapping, tensor search, temporal queries, and manifold queries.

### Changed — Zero-Heap Guarantees
- **Sanctuary Crypto Refactoring**: Eliminated Vec allocations in `encrypt_sanctuary_chunk()` and `decrypt_sanctuary_chunk()` by changing from returning `Vec<u8>` to accepting caller-supplied `&mut [u8]` buffers. Return type changed to `Result<usize, String>` for bytes written.
- **Stack-Allocated Buffers**: Implemented 4KB stack-allocated buffers for typical chunk sizes in cryptographic operations.
- **Zero-Heap API Pattern**: Caller-supplied fixed-capacity buffers prevent dynamic memory allocation in hot paths.

### Added — Cryptographic Infrastructure
- **48-byte PBKDF2 Key Derivation**: Enhanced key derivation splitting into 32-byte cipher key and 16-byte volume root tweak.
- **Implicit Domain-Separated Nonce Derivation**: XOR-based nonce derivation using volume root tweak and chunk index/offset.
- **AEAD Cipher Integration**: Integrated AES-256-GCM with zero-heap guarantees for sanctuary lane cryptography.

### Added — Feature Flags
- **tensor-10d**: Enables 10D tensor coordinate system and all tensor operations
- **tensor-gpu**: Enables GPU acceleration (CUDA/Metal/Vulkan) for tensor operations
- **tensor-npu**: Enables NPU acceleration (Neural Engine) for tensor operations

### Added — Documentation
- **Q42_PIPELINE_CONTAINER_SPEC.md**: Comprehensive architectural specification defining 10D tensor system, pipeline-to-container architecture, hardware capability tiers, zero-heap execution constraints, and implementation phases.

---
## [0.0.12] — 2026-06-07

### Summary

v0.0.12 ships cooperative group chat with sub-agent hierarchy, daemon-backed chat relay, Qualia-native WebTorrent HTTP web-seeding for ontology artifacts, and the Ontology Workbench import/share pipeline. Flutter desktop is the primary shipped shell.

### Added — Group Chat & Sub-Agents

- **`chat_agents.rs`**: Sub-agent DID derivation (`did:qualia:subagent:...`), `OutcomeSharingPolicy`, cooperative peer context for multi-LLM inference.
- **Chat relay**: `POST /chat/publish` + `GET /chat/pull` on the Qualia daemon; `syncChatRelay()` FRB binding.
- **Chat graph**: Fragment replies, branch types, reactions, file attachments with sharing policy.
- **Group sessions**: `createGroupChatSession`, participant management, session DIDs for ontology sharing.

### Added — WebTorrent Seeder (Daemon)

- **`webtorrent_seeder.rs`** + **`webtorrent_routes.rs`**: In-process HTTP web-seed for `.c.q42` files; magnet builder with `ws=` parameter; upload telemetry (`seeder: "qualia-daemon"`).
- Daemon boot syncs active seeds from `{storage}/Index/workbench.jsonl`.
- Flutter syncs workbench seeds ~2s after daemon start.

### Added — Ontology Workbench

- URI import → `.c.q42` compression → SHA-1 info hash → magnet URI.
- Per-ontology torrent policy (audience, contact/session DIDs, bandwidth limits).
- Share cards for contacts and chat session DIDs.

### Changed

- API Explorer (`docs/api-explorer/`) updated for v0.0.12: chat relay, WebTorrent, Desktop Chat, and Ontology Workbench catalog entries.
- Manuals and LLM helper docs refreshed for current inference stack and Flutter FRB surface.

---

## [0.0.12-dev] — 2026-06-06

### Summary

Phase 6 completes the core logic modality stack, adds fiduciary mediation between LLM agents and the graph engine, introduces capability profiles with a binary QCHK format, and ships the resource catalog download pipeline. Test count: **195/195** pass.

---

### Added — Logic Modalities

- **Epistemic Logic** (`modalities/epistemic.rs`): `OP_KNOWS=0x20`, `OP_BELIEVES=0x21`, `OP_COMMON_KNOWLEDGE=0x22`. `EpistemicVerdict` with certainty u8 and nesting depth u4. `evaluate_epistemic_frame` with agent and world filtering. Five tests passing.

- **Linear Temporal Logic** (`modalities/temporal_ltl.rs`): Correct LTL trace evaluator (`evaluate_ltl_trace`). Operators: `Globally` (0x40), `Finally` (0x41), `Next` (0x42), `Until` (0x43), `Release` (0x44). Distinguishes from the float-threshold `Always/Eventually/Next` opcodes in `logic.rs` which remain for backward compatibility. Seven tests passing.

- **Paraconsistent Logic** (`modalities/paraconsistent.rs`): `OP_ISOLATE=0x30`, `OP_CONTRADICTION_SCORE=0x31`, `OP_PARACONSISTENT_MERGE=0x32`. `route_paraconsistent` partitions Quins into consistent and isolated output buffers without halting on contradiction. Isolated context = `q_hash("q42:isolated") ^ original_context`. Wired to `EnforceBilateralMicroCommons` routing lane. Five tests passing.

- **Dialectical Logic** (`modalities/dialectical.rs`): `synthesize_dialectical(thesis, antithesis)` produces a synthesis Quin with `SYNTHESIZED_BIT` (bit 58) set and context = `thesis_context ^ antithesis_context`. Built on top of ASP stable-model pairs.

- **N3 → Deontic Bridge** (`deontic_logic.rs::compile_n3_rule_to_norm`): Compiles N3 `Rule` structs (from `n3_parser.rs`) into deontic norm Quins. Handles `Strict/Defeasible/Defeater/Linear` rule types. `^>` (Defeater) rules produce Quins with `DEFEATER_BIT` set. Returns `None` for malformed rules. Five tests passing.

### Added — Modality Promotions (stubs to real implementations)

- **ASP (`modalities/asp.rs`)**: Replaced `generate_stable_models()` stub with zero-alloc `enumerate_stable_models`. Up to `MAX_STABLE_MODELS = 8` worlds encoded as context-hash variants.

- **Description Logic (`modalities/dl.rs`)**: Replaced always-false stub with `check_subsumption_quin` operating over a TBox Quin slice, checking `predicate = q_hash("rdfs:subClassOf")` chains.

- **Linear Logic (`modalities/linear.rs`)**: Replaced println stub with tombstone mechanism. `consume_quin` sets `CONSUMED_BIT` (metadata bit 59). `is_consumed` reads it. Zero allocation.

### Added — SHACL Compiler Extensions

- **Deontic constraints**: `DeonticObligate`, `DeonticPermit`, `DeonticForbid`, `DeonticNotExpired { now_unix: u32 }` — validated against active deontic Quins.

- **Epistemic constraints**: `EpistemicKnowledge { min_certainty: u8 }`, `EpistemicBelief { min_certainty: u8 }`, `CommonKnowledge` — delegate to `NativeEpistemicEval(u8)` opcode.

- **New SlgOpcode variants** (`webizen.rs`): `NativeDeonticEval`, `NativeEpistemicEval(u8)`.

### Added — MCP Intent Frame Mediation

- **`McpIntentFrame`** (`mcp_server.rs`): Struct carrying `purpose_hash`, `active_deontic_constraints: [u64; 4]`, `active_profile_id`, and `sanctuary_override: Option<[u8; 32]>`.

- **`enforce_fiduciary_tool_dispatch`**: Zero-allocation byte-level dispatch using raw byte matching over incoming JSON (no serde). Tools: `query_graph` (sanctuary-gated), `inject_test_quin` (paraconsistent isolation lane).

- **Sanctuary gate**: `query_graph` without a valid override token writes a conduct violation Quin to WAL and returns blocked. Tested: sanctuary override binding, extraction helpers.

- **Buffer scrubbing**: Transient MCP buffers zeroed via `write_volatile` after each dispatch.

### Added — LLM Agent Fiduciary Rules

- **`AgentIntent`** (`llm_agent.rs`): `intent_predicate`, `requested_graph_scope`, `requires_network`, `mcp_intent_frame_hash`, `active_profile`.

- **`WebizenVerdict`**: Five outcomes — `Permit`, `Deny`, `DenyWithExplanation(u64)`, `RequireReconfirmation`, `Sanitised`.

- **Seven fiduciary rules**: no outbound (local), no sanctuary access, token cost guard, remote consent, adversarial conduct → conduct Quin to ledger, intent frame alignment, profile masking.

- **Tests**: Frame violation, profile masking, adversarial conduct (3 tests).

### Added — Capability Profiles

- **`CapabilityProfile`** (`profiles.rs`): `profile_id`, `active_engines` (SlgOpcode allow-list), `loaded_ontologies`, `preferred_backend`, `permitted_intent_frames`.

- **QCHK binary format**: 4-byte magic `QCHK` + 8-byte profile_id + 4-byte payload_len + JSON-LD payload.

- **CLI `profile` subcommand**: `compile` (.jsonld → .chk), `list` (known profiles), `inspect` (.chk decoder).

- **`ingest --profile <file>.chk`**: Binds a CapabilityProfile for the ingest session.

- **Six named profiles**: `profile:general`, `profile:health`, `profile:chemistry`, `profile:research`, `profile:legal`, `profile:financial`.

### Added — Resource Catalog

- **`resource_catalog.rs`**: `LLMResource`, `OntologyResource`, `SPARQLResource` types with `to_quins()`, `provenance_quin()`, `source_url_quin()`, `to_capability_profile()`. WAL integration.

- **YAML catalogs**: `resources/catalog.yaml`, `resources/llms.yaml` (Phi-3-mini, Gemma 2, Qwen2.5, Llama 3.2, Mistral, DeepSeek, CodeGemma + others), `resources/ontologies.yaml` (PROV-O, SNOMED CT, MeSH, OBO, Schema.org, Dublin Core, SKOS, Wikidata, DBpedia + domain-specific), `resources/sparql_endpoints.yaml` (Wikidata, DBpedia, Bio2RDF, UniProt).

- **CLI `resources` subcommand**: `list llms/ontologies/sparql`, `show <id>`, `download <id>`, `import-ontology <id>`. Download pipeline: YAML catalog → reqwest stream → GGufSharder → WAL.

### Added — Orchestrator Hardening

- **`TaskOrchestrator`** (`orchestrator.rs`): Pre-validates intent, post-validates output grounding, handles `DenyWithExplanation` (WAL log) and `RequireReconfirmation` (frame suspension).

### Fixed — Organic Chemistry

- **Isotope distribution calculation**: Fixed incorrect mass accumulation in multi-isotope compounds.

---

## [Unreleased] — 2026-06-05

### Added

- **Cooperative Conduct Policy**: Strict policy against adversarial, manipulative, and/or dishonest conduct by AI agents. Any such conduct will be noted in the permanent record of the project's development.
- **`AdversarialConductRecord` and `LLM_RULE_NO_ADVERSARIAL_CONDUCT`** (`llm_agent.rs`): Tracks and permanently logs any violations to WAL. Behavior associated with the commanding natural person's DID (`principal_did`). Cryptographic provenance for tamper-proof auditing.
- **DID Association & Court-Auditable Liability Graphs**: Conduct log incorporates cryptographic provenance to serve as evidence for court-of-law auditing, mapping violations to insurance liability graphs.

---

## [0.0.5] — Prior Release

- Multi-Seed Credential Architecture: Bitcoin (BTC), eCash (XEC), Nym (Nyx), Ethereum (EVM), Monero (XMR) imports.
- Semantic Typology Routing: LLaVA/Minkowski integration with Typology Lenses.
- Hardware Orchestration Dashboard: Real-time WASM boundary visualization, memory backpressure, disk paging thresholds.

## [0.0.4] — Prior Release

- Webizen Rebrand: "Sentinel VM" fully rebranded to "Webizen".
- W3C Solid Interoperability Bridge: Sandboxed `tokio` Allocation Firewall for Solid Pod HTTP REST export/import.
- Native "Hard Science" SHACL Extensions: thermodynamics, quantum DFT, bioinformatics via `qualia:` semantic extensions.
- Desktop KaTeX Integration: Mathematical LaTeX rendering in Neuro-Chat.
- HCAI DNS Frontdoor: `qualia-cli webizen dns-frontdoor` generates `did:web` + DNS TXT records.
