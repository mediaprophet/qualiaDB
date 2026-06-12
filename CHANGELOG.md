# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.0.11-dev] - 2026-06-12 (in progress)

### Added
- Integrated native async `QTensorEngine` for WASM execution over WebGPU
- Deployed locally hosted GGUF model downloader via `coi-serviceworker`
- Exported WASM WebGPU module via `wasm-pack` into `docs/llmdemo/pkg`

---

## [0.0.10] - 2026-06-11

### Summary

v0.0.10 resolves all build errors (82 -> 0), ships a complete SPARQL 1.1/1.2 engine (138 tests),
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

## [0.0.9] — 2026-06-09

### Summary

v0.0.9 addressed initial build error fixing phase, resolving 38 of 82 errors through straightforward corrections and module reorganization.

### Fixed — Build Errors (Partial)

- **38 build errors fixed**: Type mismatches, API usage, module imports
- **qualia-extensions rewired**: Now uses native Qualia LLM pipeline instead of Candle
- **q42_lexicon.rs**: Implemented properly with all required types and methods
- **Module reorganization**: Fixed imports across webizen.rs and related files

### Remaining (Resolved in v0.0.10)

- 44 build errors required architectural fixes (all resolved in v0.0.10)

---

## [0.0.8] — 2026-06-07

### Summary

v0.0.8 ships cooperative group chat with sub-agent hierarchy, daemon-backed chat relay, Qualia-native WebTorrent HTTP web-seeding for ontology artifacts, and the Ontology Workbench import/share pipeline. Flutter desktop is the primary shipped shell.

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

- API Explorer (`docs/api-explorer/`) updated for v0.0.8: chat relay, WebTorrent, Desktop Chat, and Ontology Workbench catalog entries.
- Manuals and LLM helper docs refreshed for current inference stack and Flutter FRB surface.

---

## [0.0.6-dev] — 2026-06-06

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
