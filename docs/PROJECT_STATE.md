# Cooperative Projects + Qualia Ecosystem — Project State

**Date:** 2026-06-10 (Updated)  
**Original Date:** 2026-06-06  
**Branch:** `0.0.10-dev`  
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

## 2. Current Build Status

**Build:** ✅ Compiling successfully (0 errors, all 82 errors resolved)  
**Test Count:** 539 test functions in qualia-core-db  
**Version:** 0.0.10-dev

---

## 3. Phase Completion Status

| Phase | Description | Status |
|-------|-------------|--------|
| Phase 0 | Structural split: `main.rs` into `commands/` + `engine/` shims | ✅ Complete |
| Phase 1 | Data: `streaming_import_rdf`, Q42 format unification, live daemon index | ✅ Complete |
| Phase 2 | LLM: `TaskOrchestrator`, WebizenVM SPSC intercept | ✅ Complete |
| Phase 3 | Agreements: `AgreementDID` + CRDT consent flow | ✅ Complete |
| Phase 4 | Wallet: BIP32/BIP44, ILP audit trail | ⚠️ Partially deferred |
| Phase 5 | P2P: librqbit, LLaVA, CRDT sync, GPU sieve | ⚠️ Partially deferred |
| Phase 6 | MCP fiduciary mediation, capability profiles, resource catalog | ✅ Complete |
| Phase 8 | GPU inference layer + autoregressive decode + Flutter chat UI | ✅ Complete |
| Phase 9 | Real embedding lookup (tensor-info parser), `modelPath` state in Flutter nav | ✅ Complete (GgufTensorIndex implemented) |
| Phase 7 | WASM profile loading, ZK-STARK, Nym, TEE, CI/CD signing | 🔲 Queued |

---

## 4. Recent Build Fixes (2026-06-10)

### Build Error Resolution
- All 82 build errors resolved
- Tokio runtime nesting issues fixed
- Module reorganization completed
- Test count: 539 functions in qualia-core-db

### Critical Implementation Gaps (Documented in to-do/)

**Security Stubs** (Tasks 001-004):
- zk_proofs.rs: verify_proof returns true unconditionally
- fiduciary_crypto.rs: signature verification ignored
- ML-DSA: Hand-rolled, not FIPS 204 compliant
- ECC parity: Mock implementation

**Query Layer Stubs** (Task 005):
- mmap_query_subject: Returns empty vector
- lazy_superblock_query: Fabricates results
- indexing.rs: Empty file

**LLM Inference** (Task 006):
- wgpu/Vulkan path uses mock_pipeline (placeholder shader)
- Real fused_transformer.wgsl exists but unused

---

## 5. For Detailed Historical Information

See [CHANGELOG.md](../CHANGELOG.md) for detailed release notes through v0.0.8.

See [to-do/](../to-do/) for current implementation tasks and priorities.

See [BUILD_ISSUES.md](../BUILD_ISSUES.md) for build error resolution history.

---

## 6. Key Files

### Engine
- `crates/qualia-core-db/src/lib.rs` — QualiaQuin, core types
- `crates/qualia-core-db/src/webizen.rs` — SlgArena, SlgOpcode dispatch
- `crates/qualia-core-db/src/deontic_logic.rs` — Deontic norms + N3 bridge
- `crates/qualia-core-db/src/modalities/` — All logic modality implementations
- `crates/qualia-core-db/src/mcp_server.rs` — MCP mediation layer
- `crates/qualia-core-db/src/llm_agent.rs` — Agent fiduciary rules
- `crates/qualia-core-db/src/profiles.rs` — Capability profiles
- `crates/qualia-core-db/src/resource_catalog.rs` — Resource types
- `crates/qualia-core-db/src/gguf_sharder.rs` — GGUF parser + GgufTensorIndex (Phase 9 complete)

### CLI
- `crates/qualia-cli/src/main.rs` — All CLI commands
- `resources/` — YAML catalogs

### Ontology
- `ontology/cooperative-projects.ttl` — Main ontology + Agent Framework
- `assets/icons/` — Icon assets