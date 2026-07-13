# Qualia-DB Architecture

> The 3-Core Triad, Webizen VM, Rights Ontology, and the Principal-Agent Ecosystem.
> _Branch: `0.0.24` | Last updated: 2026-07-13_

Qualia-DB abandons traditional cloud-centric, string-heavy JVM architectures in favour of a specialised 3-Core Triad built with ruthless mechanical sympathy (512 MB RAM floor). Raw multi-modal data (audio, camera feeds) would immediately breach this floor, so the ecosystem forces an **Orchestration Sieve**: the Primary Agent must coordinate deterministic tools (OpenCV, Audio DSP) to strip noise, extract contours, and build optimised files *before* handing them to the local LLM or the database.

---

## The 3-Core Triad

### 1. Zero-Allocation Ingestion
CBOR-LD gatekeeping and WASM OPFS bridging bypass heap-saturation attacks, writing natively to disk. The `qualia-cli ingest` pipeline uses Rio multi-thread streaming, sorting Quins by subject before writing LZ4-compressed SuperBlocks, so the resulting `.q42` file supports O(1) block-range lookups via a companion `.q42.bidx` index.

Supported ingest formats: CogAI Cognitive AI Chunks (`.chk` text — W3C CG ACT-R chunks-and-rules), CBOR-LD, N-Triples, N-Quads, Turtle, TriG, N3, JSON-LD, RDF/XML.

Ingest is **honest and lossless by intent**: `IngestMode::Complete` interns every literal (with language / datatype tags preserved), while `IngestMode::StripLiterals` is an explicit, reversibility-aware reduction — never silently discarded and never described as "compression" unless it round-trips. CBOR-LD is the native serialisation of the graph.

> ⚠ **Capability envelope migration**: CogAI Chunks remain `.chk` text files. QCHK capability envelopes are migrating to `.qchk`; legacy `.chk` QCHK files are compatibility-only. Use the `QCHK` magic bytes to detect old profile files during migration.

### 2. GPU Sieve (Geometric Pruning)
Graph nodes are mapped into Minkowski space within continuous 128 KB memory-mapped `QualiaSuperBlocks`. The GPU calculates bounding-hull collisions to retrieve data at sub-microsecond speeds without loading unrelated blocks. The WGSL compute shader (`shaders/fused_tensor_contraction.wgsl`) runs 64 threads/workgroup across DirectML / Vulkan / Metal / WebGPU via **`wgpu` 30** (migrated from 29; `naga` 30, `PollType` API, and the wgpu-30 `RequestAdapterOptions` surface).


### 2.5. 10D Volumetric Tensor System (Zero-Heap Geometric Processing)
The 10D tensor system [q, v, w, x, y, z, t, α, μ, σ] provides absolute mechanical sympathy across heterogeneous hardware (edge phones to A2000 GPUs to scarce QPUs). It maps neuro-symbolic human-centric logic into raw geometric physics simulations executable via SIMD, GPU texture units, or asynchronous Ground-State Resolvers.

**Coordinate System:**
- **q (Quantum Context)**: Manages epistemic superposition (q=0 ground truth, q>0 parallel contexts)
- **v (Topological Class)**: Dynamic distance metrics (Euclidean, Cyclic, Hyperbolic, Boundary Cliques)
- **w (Manifold Index)**: Multi-head attention for knowledge universe bifurcation (Medical, Legal, Personal, etc.)
- **x, y, z (Semantic Topology)**: 3D spatial coordinates for semantic clustering
- **t (Temporal State)**: Provenance ledger for immutable historical queries
- **α, μ, σ (Spectral-Logical Payload)**: EM spectrum foundation (Amplitude, Modulation, Spectral Signature). In the **Qualia WASM portal**, σ projects to both vision (CIE XYZ, U2) and hearing (Hz, U3) from the same `fract(σ)` — see [`standards/q42-10d-tensor-standard.md`](standards/q42-10d-tensor-standard.md) §1.3 and [`qualia-wasm-portal.md`](qualia-wasm-portal.md).

### 2.6. Qualia WASM portal & U3 AcousticPlane

Browser and edge embeds ship as one module (`docs/pkg/qualia/qualia_bg.wasm`): engine + wgpu viewport + **U3 symbolic audio**. JavaScript is glue only (`QualiaPortal::tick`). Audio is **spectral-first**: STFT sidecars + 64-bit Sonic Tokens + parametric DSP — never LLM PCM. Operator manual: [`qualia-wasm-portal.md`](qualia-wasm-portal.md). ADR: [`adr/0007-u3-acoustic-plane-symbolic-audio.md`](adr/0007-u3-acoustic-plane-symbolic-audio.md).

**Hardware-Tier Dispatching:**
- **Tier 0 (Edge)**: SIMD-only execution (ARM NEON/x86 AVX2), aggressive quantization, <5W power
- **Tier 1 (Mainstream)**: Hybrid CPU/NPU, minor heap buffering permitted
- **Tier 2 (High-Performance)**: GPU VRAM mapping, parallel Texture Mapping Units
- **Tier 3 (QPU)**: Asynchronous quantum context resolution via Ground-State Resolvers

**Zero-Heap Guarantees:**
- Stack-allocated Tensor10D structure (40 bytes, Pod, Zeroable)
- Caller-supplied buffers for all hot-path operations
- No Vec/HashMap/Box allocations in execution paths
- O(1) lookups via pre-computed topology and geometric distance calculations

**Ground-State Resolver (GSR):**
- Async QPU communication for quantum context resolution
- Classical exhaustion fallback (exhaustive search n≤16, greedy for larger)
- Proof-of-demand mesh aggregation and axiom caching
- Epistemic frame evolution with TTL-based cache cleanup


### 3. The Webizen VM (Logic Unification + Advanced Compilation)
Data filtering is not enough — human-centric databases must execute logic. Nested N3 implication rules, SHACL shapes, and defeasible logic are compiled by the `WebizenCompiler` (and a dedicated `shacl_compiler`) into compact L1-cache bytecodes. The VM supports:

- Omnimodal surface syntaxes
- 8 modality bridges (spatio-temporal, probabilistic, description logic, ASP, linear, epistemic, paraconsistent, linear-temporal LTL, dialectical)
- O(1) termination guarantees on highly cyclic social and legal graphs
- Rights Ontology and structural constraint enforcement at query time
- **Native Hard Science SHACL Extensions**: Custom `qualia:` properties map directly to pure-Rust hardware-accelerated math solvers (`NativeThermodynamics`, `NativeOdeSolver`, `NativeQuantumDft`, `NativeBioinformatics`, `NativeClinicalRisk`, `NativeChemicalSynthesis`). This allows the VM to transparently step out of logic resolution into zero-allocation continuous dynamics or SIMD alignment off-heap.

---

## The Query Engine (SPARQL 1.1 over quins)

Alongside the Webizen VM's logic resolution, Qualia-DB runs a from-scratch,
zero-allocation **SPARQL 1.1 engine** directly over the packed `NQuin` store
(`crates/qualia-core-db/src/sparql_library/`). Parser → planner → physical
executor is one in-process path shared by the daemon HTTP/WebSocket services,
the MCP server, the CLI, and the desktop shell.

- **Query forms** — `SELECT`, `ASK`, `CONSTRUCT` (template instantiation),
  `DESCRIBE` (Concise Bounded Description).
- **Algebra** — basic graph patterns, `OPTIONAL` (left-join), `UNION`, `MINUS`
  (anti-join), correlated `FILTER (NOT) EXISTS`, sub-`SELECT` (evaluated
  independently and joined on its projected variables), `BIND`/Extend, `GRAPH`
  (named-graph enumeration + variable binding), grouping / aggregates /
  `HAVING`, `DISTINCT`/`REDUCED`, `ORDER BY`, `LIMIT`/`OFFSET`, property paths
  (`*`/`+` compute a bounded full transitive closure), SPARQL-Star quoted
  triples, and `AS OF` / `AT TIME` temporal snapshots.
- **FILTER builtins** — an exhaustive dispatcher: string predicates/producers,
  `REGEX` with flags, numeric/boolean operators with a real `f64` channel and
  deterministic total ordering, date/time accessors, query-stable
  `RAND`/`UUID`/`BNODE`/`IRI`, and language/datatype-aware `LANG` · `LANGMATCHES`
  · `DATATYPE` · `STRLANG` · `STRDT`. GeoSPARQL WKT distance/topology via
  `geosparql.rs`.
- **Serialisation** — valid output in every standard format: SPARQL results as
  JSON / XML / CSV / TSV, and RDF as N-Triples / N-Quads / Turtle / TriG / N3 /
  JSON-LD / **CBOR-LD**, resolving term hashes to real IRIs and typed literals
  rather than opaque placeholders.
- **Federation & identity** — unauthenticated HTTP(S) `SERVICE` performs real
  SPARQL 1.1 Protocol requests and parses SPARQL-Results JSON into binding rows;
  `did:resolve(?did)` resolves through the DID resolver, while signing /
  authentication / verification / permission deliberately **fail closed** to the
  key-vault + governance layer rather than fabricating query-layer authority.

The engine carries 304 in-crate tests. The **QISP** immersive profile
(`immersive/`) layers versioned `webizen.org` IRIs, Tensor10D geo/spectral
predicates, and a typed function registry on top of this surface. See
[`query-engine/`](query-engine/) for the extension and DID-integration manuals.

---

## Lazy SuperBlocks, LZ4 Compression & Massive Datasets

Core data lives in 40,960-byte SuperBlocks (exactly 10 disk sectors) with
high-density LZ4 compression. The engine lazily scans only 16-byte headers and
seeks over irrelevant blocks in O(1) time, decompressing on demand. "Missing"
local blocks can be streamed from peers through the sync layer; the currently
implemented daemon path is libp2p request-response over TCP + Noise + Yamux,
while broader WebRTC transport language elsewhere in the repo remains a future
or adjacent profile. This lets 50 GB+ semantic ledgers run comfortably inside
the 512 MB floor.

Persistence is behind a cross-platform driver abstraction — `open_storage(data_dir)` in `storage_driver.rs` selects the backend for the host: ZNS NVMe zone-append, APFS clonefile, WinNVMe, or a portable `Mmap` fallback (and a real OPFS-backed `wasm_storage` counterpart on the browser target). Every write goes through a tamper-evident, Ed25519-signed Write-Ahead Log (`wal.rs`) that also records Webizen VM conduct violations.

Real-world example: WordNet (523 MB RDF) → 74.6 MB `.q42` · 5.56 M quins · 6.5 ms first-query latency via demand-paging with no full load.

---

## Fractal Sharding & Swarm AI Compute

While Qualia-DB rigorously enforces the 512 MB floor, it is capable of extreme horizontal scale on high-end hardware. Rather than bloating a single instance, it uses **Fractal Sharding**: on a rig with 64 GB RAM and 12 GB+ GPU, the daemon detects surplus hardware and dynamically spins up dozens of parallel, mathematically isolated 512 MB worker cells.

```bash
qualia-cli daemon --workers 100 --compute-swarm
```

This Swarm Orchestration enables massive parallel execution, deep neural-network offloading, and background Sleep-Cycle AI Compute without compromising core mechanical sympathy.

---

## The LLM Inference Stack (native, no Ollama)

For the byte-to-runtime path, P64 compatibility aliases, native/WASM
differences, governance boundary, and current implementation gaps, see the
[Q42/P64 Inference Pipeline](p64-q42-inference-pipeline.md). The table below is
a compact component summary.

Qualia-DB runs LLM inference entirely in-process. There is no Ollama, no Python runtime, no HTTP server for models.

| Step | Component | Detail |
|------|-----------|--------|
| 1 | `gguf_sharder.rs` · `GgufTokenizer` | Reads tokenizer metadata from GGUF or the embedded P64 `Q42T` section. Greedy longest-match `encode()`; SentencePiece `▁`-aware `decode()`. |
| 2 | `p64_weight.rs` · `P64TensorIndex` | Validates P64 v3, exposes role-tagged tensor descriptors, and builds the synthetic `GgufTensorIndex` compatibility view used by the engine. |
| 3 | `gguf_bridge` · `QTensorEngine` | Adopts an explicitly mounted P64 mmap or loads GGUF, reserves GEMM/KV arenas, and runs the production transformer-forward path. |
| 4 | `shaders/fused_tensor_contraction.wgsl` | WGSL compute shader, 64 threads/workgroup, 4096 FMA ops per thread; backend via DirectML 1.15 / Vulkan / Metal / WebGPU on **`wgpu` 30**. |
| 5 | `llm_agent.rs` · `LocalLlmAgent` | `infer_local_model()` runs the Phase 8 autoregressive decode loop: tokenise prompt → per-step GPU dispatch → SPSC logit stream → sentinel rollback check → argmax sample → EOS detection → detokenise. |
| 6 | `orchestrator.rs` · `TaskOrchestrator` | `orchestrate_inference()` gates every call: `validate_intent` → `infer` → `validate_output`. Manages `ModelLifecycle` state machine and `ThermalGovernor`. |

### Platform GPU Priority

| Platform | Primary path | Fallback |
|---|---|---|
| Windows x64 | DirectML 1.15 (D3D12, hardware-vendor kernels) | wgpu / D3D12 |
| macOS Apple Silicon | Accelerate `cblas_sgemm` (AMX coprocessor) | wgpu / Metal |
| Linux (NVIDIA/AMD) | wgpu / Vulkan (system ICD) | — |
| WASM | Browser WebGPU with P64 or GGUF | Model-backed P64 browser release run remains tracked in the pipeline manual |

### Phase 8 Bifurcated Compute

Token generation uses two wait-free SPSC ring buffers (`rtrb`) keeping the governance intercept off the critical allocation path:

```
LLM Engine thread  ──LogitSummary──►  LogitStream  ──►  Webizen Sentinel (calling thread)
                   ◄──DenyRollback──  ControlStream ◄──  (checks anomaly byte; injects rollback)
```

Per decode step:
1. LLM thread embeds the current token and calls `dispatch_fused_transformer_block()`.
2. Argmax + anomaly flag are packed into a fixed-size `LogitSummary` (no heap) and pushed to `LogitStream`.
3. Sentinel reads the summary. If `anomaly_byte == 0x99` (anachronism signature), it pushes `DenyRollback` to `ControlStream`.
4. On the next step, the LLM thread pops `ControlStream`. If a rollback is pending, it substitutes a safe neighbour token instead of the argmax.
5. Loop ends at EOS, the release decode budget (256), the 30-second cooperative deadline, or the absolute `MAX_OUTPUT_TOKENS` ceiling (2048).

> **Note — embedding lookup fully implemented.** The decode loop uses real token embeddings via `GgufTensorIndex::dequantize_token_embedding_into()` which parses the GGUF tensor-info section and dequantizes per-token embeddings into caller-supplied buffers. The GPU compute, SPSC ring, governance pipeline, and tokeniser are all fully functional.

### AgentBackend Variants

```rust
Local   // Explicitly mounted P64 or GGUF → wgpu → in-process. No outbound traffic.
Remote  // API call → Nym mixnet → ILP metered. Requires signed VC from Principal.
Hybrid  // Local-first. Falls back to Remote only with explicit Principal consent.
```

---

## The MCP Fiduciary Mediation Layer

`orchestrate_inference()` in `orchestrator.rs` always runs three gates:

1. `validate_intent(intent)` — pre-flight. Checks N3Logic Rights Ontology rules. If `Deny`, writes a conduct violation Quin to the WAL (signed with Ed25519) and aborts. The model is never invoked.
2. `agent.infer(prompt, graph_context)` — the actual GPU inference.
3. `validate_output(output)` — post-flight. Output must have ≥ 1 provenance `NQuin` citation. Ungrounded output is rejected.

The MCP server (`mcp_server.rs`) exposes the graph engine via `McpIntentFrame` (purpose_hash, deontic_constraints, profile_id, sanctuary_override). The state machine progresses: `HandshakePhase → AllocationFirewallActive → SanctuaryGated`.

---

## The Rights Ontology & Semantic Adjudicator

Qualia-DB natively encodes a **Rights Ontology** directly into the Webizen VM (with SHACL compilation, defeasible rules, and modality bridges).

- **Linguistic Plurality & Multi-Modal Semantics** — Binary CBOR-LD indexing natively supports mother tongues, languages of prayer, and non-written formats.
- **The Knowledge Axiom Predicate** — Rights to knowledge and fundamental shared learnings are mathematically un-propertisable.
- **Proportional Escrow (Relational Assertion)** — The N3Logic VM analyses `.q42` Provenance DAGs of both parties, calculates the exact percentage of derivation, and splits ILP Escrow funds proportionally.
- **SHACL & Structural Enforcement** — SHACL shapes are compiled into the same Webizen bytecode used for N3, enabling zero-allocation validation as part of query execution.

---

## Intentional Computing (Anti-Usury Architecture)

Qualia-DB is a framework for **Intentional Computing** — computing that
strictly honours the intent, agency, and Duty of Care of the natural person
(the Principal).

- **First-Class Agency** — No admin superuser supersedes the Principal. Cryptographic keys are the absolute root of trust; identity/fiduciary signatures use post-quantum **ML-DSA-65 (FIPS-204, via the `fips204` crate)** alongside Ed25519, with AEAD (AES-256-GCM / ChaCha20-Poly1305 / XChaCha20-Poly1305), HKDF-SHA256, and BLAKE3 / SHA-2 in `crypto/`. `crypto/zk_proofs.rs` provides real Groth16 over BLS12-381 (arkworks 0.6).
- **Sync Mesh & M:N Guardianship** — Distributed consensus remains the broader
  architectural goal. The currently implemented daemon sync profile uses a
  libp2p request-response path, while WebRTC mesh language in older docs
  describes an adjacent or future-facing transport profile. High-risk
  operations are packaged as `QuorumRequest`s broadcast to N Guardian
  Webizens; M ratifications required to proceed.
- **Capability Profiles** — `.qchk` (QCHK) binary bundles declare the allowed engine operations and ontology namespaces for an agent session. Six named profiles: general, health, chemistry, research, legal, financial.

---

## DID:GIT & Staged Axiomatic Evolution

Data projects in this ecosystem possess **Temporal Self-Governance**.

- Through the `did:git` Permissive Commons Profile, every project initialises a DOAP (Description of a Project) as its Genesis Block.
- To evolve a project to its next stage, the proposed `git` commit must be mathematically validated by the N3Logic Webizen VM against the *former* axioms.
- If valid, the transition is anchored globally to the Bitcoin blockchain via `gitmark`.

---

## The ILP Economic Shift Engine

Qualia-DB explicitly rejects the infinite rent-seeking paradigm of the legacy web.

- Creators define an exact **Obligation Cost** using N3Logic Risk-Compounding algorithms.
- As Interledger Protocol (ILP) Web Monetisation streams flow in, the Daemon tracks the running balance.
- Once the mathematical threshold is met, the **Threshold Shift Licence (TSL)** automatically fires, shifting the asset to the *Permissive Commons*.

---

## Human-Facing Packaging

Qualia-DB ships with three tightly-bound human-facing interfaces:

1. **Webizen Studio (`crates/webizen-studio/`) + Webizen Desktop (`crates/webizen-desktop/`)** — The Webizen environment (Windows, macOS, Linux). The UI is built in Rust with **Dioxus 0.8.0-alpha.0** (compiled to web assets and hosted inside the `webizen-desktop` **Tauri 2** shell), bypassing legacy Node.js/React overhead. It provides a flexible pane registry housing a Neuro-Symbolic Chat, an LLM Engine harness, a Vital Monitor, and an Ontology Builder designed to let people define personal ontological axioms via natural language, plus a large registry of domain "qapps". The desktop shell ships signed, self-updating release artifacts (minisign updater; desktop / CLI / WASM release CI). Webizen Studio replaces the older Node.js/React prototypes and the Flutter desktop application.

2. **Qualia CLI (`crates/qualia-cli`)** — The primary toolchain for data ingestion, benchmarking, daemon management, capability profile compilation, and resource catalog operations.

3. **WASM Bridge (`crates/qualia-core-db`, WASM target)** — Builds to `docs/playground/` for the browser demo (GitHub Pages), with feature-gated profiles (`wasm-logic`, `wasm-scientific`, `wasm-llm`, `wasm-ontology`; all wgpu/GPU code is gated behind `gpu-runtime`). Uses OPFS for block caching and SharedArrayBuffer for zero-copy IPC. The SPARQL query engine and the compute-engine primitives run in WASM; the **autoregressive LLM decode loop still uses the mock ring-buffer path** on WASM — real GPU decode requires native OS APIs.

---

## W3C Solid Interoperability Bridge (Allocation Firewall)

Qualia-DB operates natively on `.q42` CBOR-LD binary graphs with strict zero-allocation limits. The `qualia-solid-bridge` crate exposes a `warp`/`tokio` server translating incoming HTTP REST / JSON-LD / Turtle into minimal 64-bit Quin hashes via `ldp_translator.rs`. The multi-threaded `tokio` runtime is sandboxed to the network boundary — no string allocations bleed into the 512 MB floor.

---

## Architectural Decision Records

Detailed rationale for specific design choices is in [adr/](adr/).

- [ADR 0001 — The 48-byte Qualia Quin Alignment](adr/0001-the-48-byte-qualia-quin-alignment.md)
- [ADR 0002 — Zero-Allocation Query Compiler](adr/0002-zero-allocation-query-compiler.md)
- [ADR 0003 — Permissive Commons Billing Gates](adr/0003-permissive-commons-billing-gates.md)
- [ADR 0004 — Sentinel to Webizen Terminology Rebrand](adr/0004-sentinel-to-webizen-rebrand.md)
- [ADR 0005 — DNS Frontdoor and HCAI Agreements](adr/0005-dns-frontdoor-and-hcai-agreements.md)
- [ADR 0006 — Zero-Allocation Solid Bridge Isolation](adr/0006-zero-allocation-solid-bridge.md)
- [ADR 0007 — U3 Acoustic Plane: Symbolic Audio](adr/0007-u3-acoustic-plane-symbolic-audio.md)
- [ADR 0008 — FrameLayout ABI for the NQuin's Computational Bytes](adr/0008-frame-layout-abi-and-inline-tags.md)
- [ADR 0009 — ShEx scoped alongside SHACL](adr/0009-shex-scoped-alongside-shacl.md)
- [ADR 0010 — Speculative decode default-on](adr/0010-speculative-decode-default-on.md)
- [ADR 0011 — Human-centric consent, accountability & disposition](adr/0011-human-centric-consent-accountability-and-disposition.md)
