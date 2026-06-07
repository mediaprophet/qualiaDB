# Qualia-DB Architecture

> The 3-Core Triad, Webizen VM, Rights Ontology, and the Principal-Agent Ecosystem.
> _Branch: `0.0.8-dev` | Last updated: 2026-06-07_

Qualia-DB abandons traditional cloud-centric, string-heavy JVM architectures in favour of a specialised 3-Core Triad built with ruthless mechanical sympathy (512 MB RAM floor). Raw multi-modal data (audio, camera feeds) would immediately breach this floor, so the ecosystem forces an **Orchestration Sieve**: the Primary Agent must coordinate deterministic tools (OpenCV, Audio DSP) to strip noise, extract contours, and build optimised files *before* handing them to the local LLM or the database.

---

## The 3-Core Triad

### 1. Zero-Allocation Ingestion
CBOR-LD gatekeeping and WASM OPFS bridging bypass heap-saturation attacks, writing natively to disk. The `qualia-cli ingest` pipeline uses Rio multi-thread streaming, sorting Quins by subject before writing LZ4-compressed SuperBlocks, so the resulting `.q42` file supports O(1) block-range lookups via a companion `.q42.bidx` index.

Supported ingest formats: CogAI Cognitive AI Chunks (`.chk` text — W3C CG ACT-R chunks-and-rules), CBOR-LD, N-Triples, Turtle, JSON-LD, RDF/XML.

> ⚠ **Capability envelope migration**: CogAI Chunks remain `.chk` text files. QCHK capability envelopes are migrating to `.qchk`; legacy `.chk` QCHK files are compatibility-only. Use the `QCHK` magic bytes to detect old profile files during migration.

### 2. GPU Sieve (Geometric Pruning)
Graph nodes are mapped into Minkowski space within continuous 128 KB memory-mapped `QualiaSuperBlocks`. The GPU calculates bounding-hull collisions to retrieve data at sub-microsecond speeds without loading unrelated blocks. The WGSL compute shader (`shaders/fused_tensor_contraction.wgsl`) runs 64 threads/workgroup across DirectML / Vulkan / Metal / WebGPU via `wgpu`.

### 3. The Webizen VM (Logic Unification + Advanced Compilation)
Data filtering is not enough — human-centric databases must execute logic. Nested N3 implication rules, SHACL shapes, and defeasible logic are compiled by the `WebizenCompiler` (and a dedicated `shacl_compiler`) into compact L1-cache bytecodes. The VM supports:

- Omnimodal surface syntaxes
- 8 modality bridges (spatio-temporal, probabilistic, description logic, ASP, linear, epistemic, paraconsistent, linear-temporal LTL, dialectical)
- O(1) termination guarantees on highly cyclic social and legal graphs
- Rights Ontology and structural constraint enforcement at query time
- **Native Hard Science SHACL Extensions**: Custom `qualia:` properties map directly to pure-Rust hardware-accelerated math solvers (`NativeThermodynamics`, `NativeOdeSolver`, `NativeQuantumDft`, `NativeBioinformatics`, `NativeClinicalRisk`, `NativeChemicalSynthesis`). This allows the VM to transparently step out of logic resolution into zero-allocation continuous dynamics or SIMD alignment off-heap.

---

## Lazy SuperBlocks, LZ4 Compression & Massive Datasets

Core data lives in 40,960-byte SuperBlocks (exactly 10 disk sectors) with high-density LZ4 compression. The engine lazily scans only 16-byte headers and seeks over irrelevant blocks in O(1) time, decompressing on demand. "Missing" local blocks can be streamed from peers via WebRTC DataChannel. This lets 50 GB+ semantic ledgers run comfortably inside the 512 MB floor.

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

Qualia-DB runs LLM inference entirely in-process. There is no Ollama, no Python runtime, no HTTP server for models.

| Step | Component | Detail |
|------|-----------|--------|
| 1 | `gguf_sharder.rs` · `GgufTokenizer` | Parses GGUF v2/v3 KV section: extracts vocabulary (`tokenizer.ggml.tokens`), BOS/EOS IDs. Greedy longest-match `encode()`; SentencePiece `▁`-aware `decode()`. Falls back to 256-entry byte-level tokeniser when no model is loaded. |
| 2 | `gguf_sharder.rs` · `GGufSharder` | Parses GGUF header magic + tensor count; generates `QualiaQuin` pointer map (byte offsets in object field, upper 4 bits = modality flag `0b1001`). |
| 3 | `gguf_bridge.rs` · `QTensorEngine` | `load_gguf(path)` memory-maps weights via `memmap2` (zero heap). `dispatch_fused_transformer_block()` tries DirectML → Accelerate → wgpu/WGSL in order. |
| 4 | `shaders/fused_tensor_contraction.wgsl` | WGSL compute shader, 64 threads/workgroup, 4096 FMA ops per thread; backend via DirectML 1.15 / Vulkan / Metal / WebGPU. |
| 5 | `llm_agent.rs` · `LocalLlmAgent` | `infer_local_model()` runs the Phase 8 autoregressive decode loop: tokenise prompt → per-step GPU dispatch → SPSC logit stream → sentinel rollback check → argmax sample → EOS detection → detokenise. |
| 6 | `orchestrator.rs` · `TaskOrchestrator` | `orchestrate_inference()` gates every call: `validate_intent` → `infer` → `validate_output`. Manages `ModelLifecycle` state machine and `ThermalGovernor`. |

### Platform GPU Priority

| Platform | Primary path | Fallback |
|---|---|---|
| Windows x64 | DirectML 1.15 (D3D12, hardware-vendor kernels) | wgpu / D3D12 |
| macOS Apple Silicon | Accelerate `cblas_sgemm` (AMX coprocessor) | wgpu / Metal |
| Linux (NVIDIA/AMD) | wgpu / Vulkan (system ICD) | — |
| WASM | Mock path (GPU not accessible from browser) | — |

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
5. Loop ends at EOS token or `MAX_OUTPUT_TOKENS` (2048).

> **Note — real embedding lookup deferred.** The current decode loop uses a deterministic pseudo-embedding (sin-based from token ID) because the GGUF tensor-info section parser (tensor names → byte offsets) is not yet implemented. The GPU compute, SPSC ring, governance pipeline, and tokeniser are all real. See the `HANDOVER.md` "Immediate next tasks" for the embedding lookup work item.

### AgentBackend Variants

```rust
Local   // GGUF on-disk → wgpu → in-process. No outbound traffic. 128 MB RAM cap.
Remote  // API call → Nym mixnet → ILP metered. Requires signed VC from Principal.
Hybrid  // Local-first. Falls back to Remote only with explicit Principal consent.
```

---

## The MCP Fiduciary Mediation Layer

`orchestrate_inference()` in `orchestrator.rs` always runs three gates:

1. `validate_intent(intent)` — pre-flight. Checks N3Logic Rights Ontology rules. If `Deny`, writes a conduct violation Quin to the WAL (signed with Ed25519) and aborts. The model is never invoked.
2. `agent.infer(prompt, graph_context)` — the actual GPU inference.
3. `validate_output(output)` — post-flight. Output must have ≥ 1 provenance `QualiaQuin` citation. Ungrounded output is rejected.

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

Qualia-DB is a framework for **Intentional Computing** — computing that strictly honours the intent, sovereignty, and Duty of Care of the natural person (the Principal).

- **First-Class Agency** — No admin superuser supersedes the Principal. Cryptographic keys are the absolute root of trust.
- **WebRTC CRDT Mesh & M:N Guardianship** — Distributed consensus via a local WebRTC Mesh. `did:q42` Webizens form an M:N gossip network using `Automerge` CRDTs. High-risk operations are packaged as `QuorumRequest`s broadcast to N Guardian Webizens; M ratifications required to proceed.
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

## The Consumer Packaging

Qualia-DB ships with three tightly-bound consumer interfaces:

1. **Qualia Flutter App (`crates/qualia-flutter/`)** — Primary desktop shell (Windows, macOS, Linux) via flutter_rust_bridge. Provides cooperative group chat with sub-agent hierarchy, ontology branches, chat graph panel, LLM Hub, Ontology Workbench (URI import → `.c.q42` → magnet sharing), Qapp Vault (embedded WebView), Wallet, Address Book, and Spatial Physics. The Qualia daemon on loopback also serves chat relay (`/chat/publish`, `/chat/pull`) and HTTP web seeds for ontology artifacts (`/torrent/webseed/{hash}`). See [Flutter API Reference](flutter-api-reference.md) and [RELEASE_NOTES_v0.0.8.md](RELEASE_NOTES_v0.0.8.md).

2. **Qualia CLI (`crates/qualia-cli`)** — The primary toolchain for data ingestion, benchmarking, daemon management, capability profile compilation, and resource catalog operations.

3. **WASM Bridge (`crates/qualia-core-db`, WASM target)** — Builds to `docs/playground/` for the browser-based demo (GitHub Pages). Uses OPFS for block caching and SharedArrayBuffer for zero-copy IPC. Note: LLM inference is not available in the WASM target (GPU paths require native OS APIs); the WASM path falls through to the mock ring-buffer.

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
