# QualiaDB Architecture

_Branch: `0.0.6-dev`_

QualiaDB is a zero-allocation, mechanically sympathetic semantic database and multi-agent collaboration ecosystem. It bridges the string-heavy reality of the Semantic Web with hardware-aligned execution paths, enforcing strict constraints to ensure bounded memory and deterministic performance.

---

## 1. Core Principles & Constraints

- **Zero-Heap in Hot Paths**: No `Vec`, `String`, or `Box` allocations inside evaluator loops. Callers supply fixed-size output buffers (`&mut [T]`) or use `[T; N]` stack arrays.
- **42 MB Prolog Sentinel & SlgArena**: The Webizen VM executes within a strictly bounded 42 MB memory envelope. `SlgArena` manages state structurally without dynamic growth (917,504 Quin slots).
- **The 48-Byte Super-Quin**: Every semantic datum fits into a `QualiaQuin` (6 × `u64`: subject, predicate, object, context, metadata, parity). Hashes and bit-packing replace string pointers entirely.
- **512 MB Edge Floor**: The total system — graph engine, Webizen VM, LLM runtime, and all caches — must stay within 512 MB. This is a hard design target for personal-device deployment.
- **Mechanical Sympathy**: Data layouts and evaluation paths are designed for maximum CPU cache efficiency, avoiding pointer chasing and random memory access.
- **q_hash for all URIs**: All string IRIs are FNV-1a–hashed at compile time via `q_hash()` or `q_turtle!`. No runtime string allocation in the engine core.

---

## 2. Storage Engine

### SuperBlocks (`storage.rs`, `wal.rs`, `archive.rs`)

The fundamental on-disk unit is the **SuperBlock**: exactly 40,960 bytes (10 × 4,096-byte disk sectors), holding 850 `QualiaQuin`s plus a 160-byte header. SuperBlocks are LZ4-compressed for density.

- **Header-first boot**: On startup, only block headers are read. The engine maps the full `.q42` file via `memmap2` and services queries by seeking directly to relevant blocks — non-relevant blocks are never decompressed.
- **WAL (`wal.rs`)**: All mutations are append-only to the Write-Ahead Log before being compacted into SuperBlocks. Conduct violation Quins are written here and signed with ed25519.
- **Volatile scrubbing**: When a buffer is evicted, `std::ptr::write_volatile` zeroes it to prevent memory-harvesting attacks.

### BIDX Demand-Paging (`.q42.bidx`)

A `.q42.bidx` sidecar file records the subject-hash range covered by each SuperBlock. This enables **O(1) block skip** — the query engine binary-searches the BIDX to identify which blocks could contain a target subject, then seeks directly to those blocks without scanning the file.

- **VFS `BlockOffsetMap`**: An in-memory structure mapping block index → byte offset, populated from the BIDX at open time. Queries never touch blocks outside the relevant range.
- **OPFS auto-cache** (WASM target): When running in the browser, blocks fetched from the native daemon are cached in the Origin Private File System so subsequent queries are local.

### Key Types

| Type | File | Purpose |
|------|------|---------|
| `QualiaQuin` | `lib.rs` | 48-byte semantic datum |
| `QualiaSuperBlock` | `storage.rs` | 850-Quin compressed block |
| `QuinIncrementalScanner` | `storage.rs` | Zero-alloc streaming cursor |
| `BlockOffsetMap` | `indexing.rs` | BIDX-backed demand-paging index |

---

## 3. Universal Translator: CLI Ingestion Pipeline

The `qualia-cli` is the entry point for sovereign data ingestion into `.q42` vaults.

- **Formats**: Prioritizes Cognitive AI Chunks (`.chk`) and CBOR-LD (`.cbor` / `.cbor-ld`); also accepts N-Triples, Turtle, JSON-LD, and RDF/XML via the Rio streaming parser.
- **Zero-Allocation Parsing**: Data is pull-parsed sequentially. Values are hashed directly into Quins using FNV-1a; no intermediate string representation enters the engine.
- **Sort-First Ingestor**: Before SuperBlock emission, the ingestor sorts Quins by subject hash. This makes the output BIDX-indexable and enables O(1) block skipping at query time.
- **Multi-Pass External Sorter**: Handles datasets larger than RAM by buffering up to ~50 MB chunks, sorting by object hash, and flushing to disk. A K-way merge then emits the final sorted `.q42` stream.
- **BIDX Indexing**: A `.q42.bidx` sidecar is generated alongside every `.q42` file, recording the subject-hash range of each SuperBlock.

---

## 4. Webizen VM & Modality Logics

The Webizen VM natively interprets SHACL constraints and N3Logic rules through integrated reasoning modalities, all executing within the 42 MB `SlgArena` envelope.

### N3Logic Rule Types (`n3_parser.rs`)

| Arrow | `RuleType` | Semantics |
|-------|-----------|-----------|
| `=>` | `Strict` | Classical modus ponens — forward chaining |
| `~>` | `Defeasible` | Can be overridden by a Defeater |
| `^>` | `Defeater` | Overtly defeats a matching Defeasible rule; maps to `DEFEATER_BIT` in deontic Quins |
| `-o` | `Linear` | Linear logic: premise is consumed on firing |

### Modality Logics

- **Deontic Logic** (`deontic_logic.rs`): Obligations, permissions, prohibitions, and defeaters (`OP_OBLIGATE=0x10`, `OP_PERMIT=0x11`, `OP_FORBID=0x12`). `DEFEATER_BIT = 1u64 << 63` in the predicate field.
- **Epistemic Logic** (`epistemic.rs`): Knowledge and belief states with certainty scoring (`OP_KNOWS=0x20`, `OP_BELIEVES=0x21`, `OP_COMMON_KNOWLEDGE=0x22`).
- **Temporal Logic / LTL** (`temporal_ltl.rs`): Linear Temporal Logic trace evaluation — `Globally`, `Finally`, `Next`, `Until`, `Release`. ⚠ The `Always/Eventually/Next` opcodes in `logic.rs` are *not* real LTL — use `evaluate_ltl_trace` from this module instead.
- **Paraconsistent Logic** (`paraconsistent.rs`): Routes contradictions to an isolated sub-context (`q_hash("q42:isolated") ^ original_context`) without halting the system.
- **Dialectical Logic** (`dialectical.rs`): Thesis / antithesis → synthesis over ASP stable-model pairs.
- **Spatio-Temporal Logic** (`modalities/spatio_temporal.rs`): Allen Interval algebra for time-span intersections; ties into the GPU sieve and 5th-vector routing.

### Opcode Allocation

`mini_parser.rs` owns `0x00–0x04`. All modality opcodes start at `0x10`:

| Range | Owner |
|-------|-------|
| `0x00–0x04` | `mini_parser.rs` — reserved |
| `0x10–0x12` | Deontic |
| `0x20–0x22` | Epistemic |
| `0x30–0x32` | Paraconsistent |
| `0x40–0x44` | LTL |

### SHACL Compiler (`shacl_compiler.rs`)

`compile_shacl_to_webizen` translates SHACL shapes and constraints into `WebizenOpcode` bytecode arrays. This enables structural + constraint validation as part of the zero-alloc query path. Native "hard science" SHACL extensions (e.g. `qualia:thermoMetropolisStep`, `qualia:dftGroundState`) map directly to `SlgOpcode::Native*` dispatch in `webizen.rs`.

---

## 5. Native Scientific Primitives

The engine includes native `SlgOpcode` hardware-aligned instructions for scientific evaluation, wired directly in `webizen.rs::execute_vm_frame`. **These are not stubs** — each calls real implementations.

| Module | File | Capabilities |
|--------|------|-------------|
| Clinical Engine | `clinical_engine.rs` | Risk scoring (Framingham, CHA₂DS₂-VASc, SCORE2), eGFR, CrCl, pharmacokinetics, SOFA, FHIR/LOINC/RxNorm, drug interactions |
| Bioinformatics | `bioinformatics.rs` | Smith-Waterman alignment, DNA→protein translation, isoelectric point, peptide cleavage, k-mer hashing, Tanimoto similarity |
| Organic Chemistry | `organic_chemistry.rs` | SMILES/InChI parsing, ADMET heuristics (BBB permeation), LogP, TPSA, Lipinski/Veber/Ghose/Egan filters, pKa, Morgan fingerprint, Arrhenius, Gibbs, atom economy |
| Thermodynamics | `thermodynamics.rs` | MCMC Metropolis–Hastings sampling |
| Quantum DFT | `quantum_dft.rs` | DFT ground-state energy estimation |
| ODE Solver | `ode_solver.rs` | RK4 numerical integration |
| Geometric | `geometric.rs` | Spatial geometry and bounding-hull operations |
| Spatial Sieve | `spatial_sieve.rs` | GPU-accelerated Quin filtering via NETS (Non-Euclidean Tropical Sieve) |

All scientific modules are SHACL-validated at the constraint layer; SHACL shapes for each domain are compiled to `SlgOpcode` calls by `shacl_compiler.rs`.

---

## 6. Agent & LLM Layer

The LLM engine is a **native, in-process GGUF inference stack** — not Ollama, not a llama.cpp HTTP server, not any external daemon. There is no Python runtime involved.

- **Model format**: Quantized GGUF files (Q4_K_M, Q8_0). Loaded via `memmap2` directly into the OS page cache — zero heap allocation for model weights.
- **Inference backend**: `wgpu` compute shaders (DirectML / Vulkan / Metal / WebGPU). Fused transformer blocks are dispatched to the GPU via WGSL. See `gguf_bridge.rs` and `shaders/fused_tensor_contraction.wgsl`.
- **Three `AgentBackend` modes** (`llm_agent.rs`):
  - `Local` — GGUF on-disk, wgpu inference, no outbound traffic, 128 MB RAM cap.
  - `Remote` — API call routed through Nym mixnet, ILP micropayment metered, requires a signed VC from the Principal.
  - `Hybrid` — local-first, graceful fallback to Remote with explicit consent.
- **Phase 8 bifurcated compute**: During token generation two wait-free SPSC ring buffers (`rtrb`) run in parallel — an LLM Engine thread streaming logits and a Webizen Sentinel thread that can inject a `DenyRollback` mid-generation at the token level.
- **Agent Intent Validation**: Every LLM call must pass Webizen pre-flight (`validate_intent`) and post-flight (`validate_output`) checks against the Rights Ontology. This is mandatory, not optional.
- **Provenance Citation**: All LLM outputs must cite a specific `QualiaQuin` provenance hash. Ungrounded outputs are rejected.
- **Memory Budgeting**: The LLM runtime operates within a strict 128 MB ceiling, reserving the rest of the 512 MB edge budget for the Webizen VM and semantic graph.

---

## 7. MCP Server & Capability Profiles

### MCP Server (`mcp_server.rs`)

The Model Context Protocol server exposes the graph engine to AI agent tools via a zero-deserialization byte-level JSON dispatcher.

- **`McpRuntimeState`**: `HandshakePhase → AllocationFirewallActive → SanctuaryGated` — explicit state machine governing what operations are permitted at each phase.
- **`McpIntentFrame`**: Every tool call carries a purpose hash, active deontic constraints, an optional `CapabilityProfile` ID, and a session nonce. The intent frame is validated against the Rights Ontology before any graph traversal.
- **`enforce_fiduciary_tool_dispatch`**: Zero-allocation tool dispatch using raw byte matching over the incoming JSON payload (no serde). Currently wired tools: `query_graph` (requires cryptographic sanctuary override), `inject_test_quin` (routes to paraconsistent isolation lane).
- **Sanctuary gate**: If `query_graph` is called without a valid sanctuary override token, a conduct violation `QualiaQuin` is immediately written to the WAL and signed.
- **Buffer scrubbing**: Transient MCP buffers are zeroed via `write_volatile` after each dispatch to prevent data harvesting.

### Capability Profiles (`profiles.rs`)

`CapabilityProfile` provides a declarative allow-list that constrains what the LLM and Webizen VM are permitted to do within a given session.

- **`active_engines`**: If non-empty, acts as an allow-list mask over the `CAPABILITY_REGISTRY` — only the listed `SlgOpcode` variants can be dispatched.
- **`loaded_ontologies`**: Namespace hashes (e.g. `q_hash("namespace:Bio2RDF")`) actively mapped into the LLM context window.
- **`preferred_backend`**: Specifies `AgentBackend::Local`, `Remote`, or `Hybrid` for this profile.
- **`permitted_intent_frames`**: If non-empty, any LLM intent declared outside this set is instantly denied by the Webizen VM.

Profiles are loaded from external `.chk` files or constructed natively, and are referenced by hash (`profile_id = q_hash("profile:health")`) in `McpIntentFrame.active_profile_id`.
