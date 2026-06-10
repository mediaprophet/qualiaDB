# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.0.10] — 2026-06-10

### Summary

v0.0.10 resolves all build errors (82 → 0), fixes tokio runtime nesting issues, and documents critical implementation gaps including security stubs and the wgpu/Vulkan mock pipeline issue.

### Fixed — Build System

- **All 82 build errors resolved**: Project now compiles successfully with 0 errors
- **Tokio runtime fixes**: Fixed Handle::current() calls with try_current fallback for wgpu async operations
- **Module reorganization**: Completed all path references and imports
- **Test count**: 539 test functions in qualia-core-db (updated from incorrect 271 count)

### Added — Documentation

- **Claude review analysis**: Comprehensive validation of 2026-06-10 code review findings
- **Mock pipeline documentation**: Detailed analysis of wgpu/Vulkan placeholder shader issue
- **CUDA support status**: Documented CUDA GPUDirect Storage (for calculus only, not LLM)
- **Implementation task files**: 9 detailed task files for security, functionality, and documentation fixes
- **BUILD_ISSUES.md**: Updated to reflect resolved build status

### Known Limitations (Documented)

- **Security stubs**: zk_proofs, fiduciary_crypto, ML-DSA, ECC parity require real implementation
- **Query layer**: mmap_query_subject, lazy_superblock_query, indexing need real implementation
- **Linux LLM inference**: wgpu/Vulkan path uses mock_pipeline (placeholder shader)


### Added � SPARQL Engine (7,074 lines across 16 modules)

- **Complete SPARQL 1.1/1.2 implementation**: Zero-allocation architecture with index-based AST
- **Core features**: SELECT, ASK, CONSTRUCT, DESCRIBE, FILTER, aggregates, GROUP BY, HAVING, DISTINCT, LIMIT/OFFSET, ORDER BY
- **Advanced patterns**: OPTIONAL, UNION, GRAPH, Property Paths (7 types), Subqueries
- **SPARQL-Star**: Embedded triples with provenance tracking, Virtual ID Hash strategy
- **W3C Extensions**: SPARQL Update, SHACL-SPARQL, GeoSPARQL, SPARQL-MM, C2PA
- **DID Integration**: SPARQL-DID specification, federated queries with DID authentication, CORS handling
- **Zero-allocation**: Fixed-size arrays, no Vec/String/Box in hot paths, ~35KB per query
- **Testing**: 45+ comprehensive tests covering all modules
- **Documentation**: SPARQL-DID Integration Specification (399-line respec document)

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

- **`chat_agents.rs`**: Sub-agent DID derivation (`did:qualia:subagent:…`), `OutcomeSharingPolicy`, cooperative peer context for multi-LLM inference.
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

Phase 6 completes the core logic modality stack, adds fiduciary mediation between LLM agents and the graph engine, introduces capability profiles with a binary QCHK format, and ships the resource catalog download pipeline. Test count: **195/195** ✅.

---

### Added — Logic Modalities

- **Epistemic Logic** (`modalities/epistemic.rs`): `OP_KNOWS=0x20`, `OP_BELIEVES=0x21`, `OP_COMMON_KNOWLEDGE=0x22`. `EpistemicVerdict` with certainty u8 and nesting depth u4. `evaluate_epistemic_frame` with agent and world filtering. Five tests passing.

- **Linear Temporal Logic** (`modalities/temporal_ltl.rs`): Correct LTL trace evaluator (`evaluate_ltl_trace`). Operators: `Globally` (0x40), `Finally` (0x41), `Next` (0x42), `Until` (0x43), `Release` (0x44). Distinguishes from the float-threshold `Always/Eventually/Next` opcodes in `logic.rs` which remain for backward compatibility. Seven tests passing.

- **Paraconsistent Logic** (`modalities/paraconsistent.rs`): `OP_ISOLATE=0x30`, `OP_CONTRADICTION_SCORE=0x31`, `OP_PARACONSISTENT_MERGE=0x32`. `route_paraconsistent` partitions Quins into consistent and isolated output buffers without halting on contradiction. Isolated context = `q_hash("q42:isolated") ^ original_context`. Wired to `EnforceBilateralMicroCommons` routing lane. Five tests passing.

- **Dialectical Logic** (`modalities/dialectical.rs`): `synthesize_dialectical(thesis, antithesis)` produces a synthesis Quin with `SYNTHESIZED_BIT` (bit 58) set and context = `thesis_context ^ antithesis_context`. Built on top of ASP stable-model pairs.

- **N3 → Deontic Bridge** (`deontic_logic.rs::compile_n3_rule_to_norm`): Compiles N3 `Rule` structs (from `n3_parser.rs`) into deontic norm Quins. Handles `Strict/Defeasible/Defeater/Linear` rule types. `^>` (Defeater) rules produce Quins with `DEFEATER_BIT` set. Returns `None` for malformed rules. Five tests passing.

### Added — Modality Promotions (stubs → real implementations)

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
