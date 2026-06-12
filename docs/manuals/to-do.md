# Protocol & Architecture To-Do

_Branch: `0.0.8-dev` | Last updated: 2026-06-07_

This document tracks foundational architectural components that are pending integration. See `docs/PROJECT_STATE.md` for the full phase completion table.

---

## Phase 7 — Completed (v0.0.8)

- [x] **Group chat sub-agent hierarchy** — `chat_agents.rs`, outcome sharing policy, cooperative multi-LLM context
- [x] **Daemon chat relay** — `/chat/publish` + `/chat/pull`, `syncChatRelay()` FRB
- [x] **Qualia-native WebTorrent seeder** — daemon HTTP web seeds for `.c.q42`, magnet `ws=` parameter
- [x] **Ontology Workbench** — URI import, compression, seeding, share cards by contact/session DID
- [x] **Chat graph UX** — fragments, reactions, file attachments, graph panel

---

## Phase 7 — Known Gaps (Next Milestone)

The following are confirmed by code inspection:

- [ ] **`derive_lane_key` uses SHA256, not PBKDF2** — `agency.rs`. Production Sanctuary Mode needs ≥ 310,000 PBKDF2 iterations.
- [ ] **Three incompatible `.q42` write formats** — `storage.rs`, `ingest.rs`, `archive.rs`. `SuperBlockWriter` should become the canonical path.
- [ ] **`prune_defeasible_claims` uses `Vec`/`HashSet`** — `logic.rs`. Violates the zero-heap mandate.
- [ ] **`logic.rs::extract_float` conflicts with `resolver.rs` type tags** — `0b001<<60` used for different purposes in two modules. Do not fix unilaterally; see `AGENTS.md §4-D`.
- [ ] **WASM OPFS bindings** — `wasm_bridge.rs`. Scaffolded; two TODOs remaining for full block caching.
- [ ] **`sanctuary_purge` not implemented** — `mcp_server.rs`. Required for full Sanctuary lifecycle.
- [ ] **CogAI `.chk` ingestion pipeline** — `ingest.rs`. The CogAI Cognitive AI Chunks text format (W3C CG chunks-and-rules) ingestion path is not yet wired end-to-end through the `ExternalSorter`. Note: this gap is about CogAI text chunks — not the QCHK binary Capability Profile format, which is a separate system.
- [ ] **`NullThermalGovernor` always returns `Cool`** — `orchestrator.rs`. Real thermal governor not yet wired.
- [ ] **WASM profile loading** — `wasm_bridge.rs`. QCHK profiles not yet loadable in browser.
- [x] **Qapp Vault (Flutter)** — `listInstalledQapps`, `launchInstalledQapp`, `generateQappCredential`, `verifyAndInstallQapp` via FRB; embedded `QualiaQappWebView`.
- [ ] **Legacy Tauri desktop** — `qualia-desktop` / `qualia-client` not in release CI; freeze or remove when convenient.
- [ ] **`window.webizen` provider API** — Defined in the Webizen Protocol RFC but not implemented in the desktop shell. Phase 7 work.

---

## The Permissive Commons Framework

The underlying cryptographic and economic architecture of the Permissive Commons (Hybrid `.q42` ledgers, WebTorrent sync, Lightning RPC) has been partially implemented. The legal and operational rules remain pending:

- [ ] **Ramifications of Works**: Define the strict legal and computational consequences when an actor utilises an inference from the Commons. How are these ramifications enforced by the Webizen VM?
- [ ] **Supports and Entitlements**: Detail the mechanics of how micropayments, algorithmic proof-of-work, or verifiable credential presentations are mathematically gated before access is granted to shared data.
- [ ] **Revocation & Epoch Compaction**: Outline the protocol for when an author revokes consent to a previously shared subjective inference. How does the Permissive Commons dictate the physical erasure of those Quins via Epoch Compaction across the decentralized community network?
- [ ] **Derivative Works & Licensing**: Establish how legacy Permissive Commons licensing rules are encoded directly into the 48-byte Quin metadata to prevent unauthorized derivative logic execution.

---

## Completed (Phase 6)

- [x] MCP Fiduciary Mediation Layer (`mcp_server.rs`, `McpIntentFrame`, sanctuary gate)
- [x] LLM Agent fiduciary rules (`AgentIntent`, `WebizenVerdict`, 7 rules)
- [x] Capability Profiles (QCHK format, 6 named profiles, CLI `profile compile/list/inspect`)
- [x] Resource Catalog (LLMs, ontologies, SPARQL; download pipeline; CLI `resources`)
- [x] New modalities: Epistemic (`0x20–0x22`), Paraconsistent (`0x30–0x32`), LTL (`0x40–0x44`), Dialectical, promoted ASP/DL/Linear
- [x] SHACL compiler extensions: DeonticObligate/Permit/Forbid/NotExpired, EpistemicKnowledge/Belief/CommonKnowledge
- [x] Scientific primitives fully wired: NativeClinicalRisk, NativeChemicalSynthesis, NativeLipinski
- [x] Phase 8 bifurcated compute scaffolded (LogitStream + ControlStream SPSC, WebizenSentinel, DenyRollback)
