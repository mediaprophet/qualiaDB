# Cooperative Projects + Qualia Ecosystem — Project State

**Date:** June 2026  
**Branch:** `0.0.6-dev` (commit `0e4997a`)  
**Purpose:** Context export for new chat sessions

---

## 1. Overall Direction

The goal is to build a human-centric, relational, logic-driven system for cooperative work that properly supports both humans and software agents while keeping legal and moral responsibility with human Principals.

Key themes:
- **Agency & Personhood First**
- **Relational & Social** modeling (not isolated "self-sovereign" individuals)
- **Explicit, opt-in** inheritance and propagation
- **CBOR-LD** as the primary runtime serialization format
- **Webizen logic** (N3Logic + SHACL + full modality stack) as the enforcement layer
- Strong protection of personal boundaries and consent

---

## 2. Phase Completion Status

| Phase | Description | Status |
|-------|-------------|--------|
| Phase 0 | Structural split: `main.rs` into `commands/` + `engine/` shims | ✅ Complete |
| Phase 1 | Data: `streaming_import_rdf`, Q42 format unification, live daemon index | ✅ Complete |
| Phase 2 | LLM: `TaskOrchestrator`, WebizenVM SPSC intercept | ✅ Complete |
| Phase 3 | Agreements: `AgreementDID` + CRDT consent flow | ✅ Complete |
| Phase 4 | Wallet: BIP32/BIP44, ILP audit trail | ⚠️ Partially deferred |
| Phase 5 | P2P: librqbit, LLaVA, CRDT sync, GPU sieve | ⚠️ Partially deferred |
| **Phase 6** | **MCP fiduciary mediation, capability profiles, resource catalog** | ✅ **Complete** |
| Phase 7 | WASM profile loading, ZK-STARK, Nym, TEE, CI/CD signing | 🔲 Next |

---

## 3. Phase 6 — What Was Built

### Core Engine Additions (195/195 tests passing)

#### New Modalities

| Module | Opcodes | Status |
|--------|---------|--------|
| `modalities/epistemic.rs` | OP_KNOWS=0x20, OP_BELIEVES=0x21, OP_COMMON_KNOWLEDGE=0x22 | ✅ 5 tests |
| `modalities/temporal_ltl.rs` | G=0x40, F=0x41, X=0x42, U=0x43, R=0x44 | ✅ 7 tests |
| `modalities/paraconsistent.rs` | OP_ISOLATE=0x30, CONTRADICTION_SCORE=0x31, MERGE=0x32 | ✅ 5 tests |
| `modalities/dialectical.rs` | synthesize_dialectical, SYNTHESIZED_BIT | ✅ |
| `deontic_logic.rs` | compile_n3_rule_to_norm (N3→Deontic bridge) | ✅ 5 tests |

#### Promoted from Stubs

| Module | What changed |
|--------|-------------|
| `modalities/asp.rs` | enumerate_stable_models — zero-alloc, up to 8 worlds |
| `modalities/dl.rs` | check_subsumption_quin — TBox slice traversal |
| `modalities/linear.rs` | consume_quin / is_consumed — CONSUMED_BIT tombstone |

#### SHACL Compiler Extensions

- Deontic constraints: `DeonticObligate`, `DeonticPermit`, `DeonticForbid`, `DeonticNotExpired`
- Epistemic constraints: `EpistemicKnowledge`, `EpistemicBelief`, `CommonKnowledge`
- New SlgOpcode variants: `NativeDeonticEval`, `NativeEpistemicEval(u8)`

#### MCP Fiduciary Mediation Layer

- `McpIntentFrame` struct: purpose_hash, deontic_constraints, profile_id, sanctuary_override
- `enforce_fiduciary_tool_dispatch`: zero-allocation byte-level JSON dispatch
- Sanctuary gate: `query_graph` blocked without cryptographic override; conduct Quin written to WAL
- Buffer scrubbing via `write_volatile`

#### LLM Agent Fiduciary Rules

- `AgentIntent` + `WebizenVerdict` (5 outcomes)
- 7 fiduciary rules with test coverage
- Adversarial conduct → DID-associated Quin on WAL ledger (cryptographically auditable)

#### Capability Profiles

- `CapabilityProfile` struct with engine allow-list, ontology namespaces, backend preference
- QCHK binary format (magic + profile_id + payload_len + JSON-LD)
- 6 named profiles: general, health, chemistry, research, legal, financial
- CLI: `profile compile/list/inspect`
- `ingest --profile <file>.chk` binding

#### Resource Catalog

- `LLMResource`, `OntologyResource`, `SPARQLResource` types with `to_quins()`, WAL integration
- YAML registries: `resources/llms.yaml`, `resources/ontologies.yaml`, `resources/sparql_endpoints.yaml`
- Full download pipeline: YAML → reqwest stream → GGufSharder → WAL
- CLI: `resources list/show/download/import-ontology`

---

## 4. Ontology State (cooperative-projects.ttl)

### Core Concepts

- `qp:Project`, `qp:Subproject` relationships
- `qp:subProjectOf` / `qp:hasSubproject`
- `qp:inheritsGovernanceFrom` (explicit, opt-in)
- `qp:propagatesObligationToParent` (boolean, defaults to protecting agency)
- `qp:graduatedFrom` — allows a subproject to become independent
- `qp:dependsOn` — general many-to-many dependency (not just hierarchy)
- `qp:ContextualConsent`, `qp:RelationshipContext`, `qp:RelationshipRole`
- Dynamic Equity / Stewardship Shares (`qp:Slice`)
- Contracts, Verifiable Claims, Tokenized Shares, Cash-Out logic

### Key Logic Patterns

- Inheritance and obligation roll-up are **explicit and conditional**
- Personal data and `ContextualConsent` are **never automatically lifted** to parent projects
- Subprojects can maintain independent Dynamic Equity / Stewardship Shares
- Credential requirements can cascade when governance is inherited
- Governance inheritance is additive (subprojects can add local rules)

---

## 5. UI Progress

### Desktop (Flutter — `crates/qualia-flutter/`)

- **LLM Hub** — grid/list view, bulk actions, download state persists navigation, detail panel
- **Ontology Hub** — browse, import, namespace view
- **FRB bridge** (`rust/src/api.rs`) — `download_llm`, `import_ontology` delegate to `qualia-cli` subprocess

### Web (docs/)

- `docs/project-detail.html` — Subprojects section + Create Subproject modal
- `docs/cooperative.html` — Hierarchy badges and parent/subproject indicators
- `docs/kanban.html` — Hierarchy filter, color-coded badges, dynamic filtering
- `docs/roadmap.html` — Hierarchy-aware phases with filtering and badges

---

## 6. Key Files

### Engine

- `crates/qualia-core-db/src/lib.rs` — `QualiaQuin`, core types
- `crates/qualia-core-db/src/webizen.rs` — SlgArena, SlgOpcode dispatch
- `crates/qualia-core-db/src/deontic_logic.rs` — Deontic norms + N3 bridge
- `crates/qualia-core-db/src/modalities/` — All logic modality implementations
- `crates/qualia-core-db/src/mcp_server.rs` — MCP mediation layer
- `crates/qualia-core-db/src/llm_agent.rs` — Agent fiduciary rules
- `crates/qualia-core-db/src/profiles.rs` — Capability profiles
- `crates/qualia-core-db/src/resource_catalog.rs` — Resource types

### CLI

- `crates/qualia-cli/src/main.rs` — All CLI commands
- `resources/` — YAML catalogs

### Ontology

- `ontology/cooperative-projects.ttl` — Main ontology + Agent Framework
- `assets/icons/` — Icon assets

---

## 7. Known Gaps (Phase 7 scope)

| Gap | File | Notes |
|-----|------|-------|
| `derive_lane_key` uses SHA256, not PBKDF2 | `agency.rs` | Production Sanctuary Mode needs ≥310,000 PBKDF2 iterations |
| Three incompatible `.q42` write formats | `storage.rs`, `ingest.rs`, `archive.rs` | `SuperBlockWriter` should become canonical |
| `prune_defeasible_claims` uses `Vec`/`HashSet` | `logic.rs` | Violates zero-heap mandate |
| `logic.rs::extract_float` conflicts with `resolver.rs` type tags | `logic.rs` | `0b001<<60` used for different purposes |
| WASM OPFS bindings | `wasm_bridge.rs` | Scaffolded, two TODOs remaining |
| `sanctuary_purge` not implemented | `mcp_server.rs` | Required for full sanctuary lifecycle |
| `.chk` ingestion pipeline via `ExternalSorter` | `ingest.rs` | Not yet wired end-to-end |
| `NullThermalGovernor` always returns `Cool` | `orchestrator.rs` | Real thermal governor not yet wired |
| WASM profile loading | `wasm_bridge.rs` | QCHK profiles not yet loadable in browser |

---

## 8. Design Principles to Maintain

- Everything is **Principal-centered**
- Inheritance and propagation are **explicit and reversible**
- Personal boundaries and consent are protected by default
- Logic (Webizen) enforces boundaries — not advisory but enforced
- CBOR-LD is the runtime format
- Zero-heap in all hot paths
- 42 MB sentinel is structurally enforced

---

*Updated: June 2026 – Phase 6 (MCP Fiduciary Mediation + Capability Profiles + Resource Catalog) Complete*