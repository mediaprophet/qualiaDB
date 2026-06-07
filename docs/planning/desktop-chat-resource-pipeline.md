# Desktop Chat + Resource Catalog — Implementation Plan

**Branch target:** `0.0.8-dev`  
**Version:** Engine `0.0.8`, docs track `0.0.8-dev`  
**Last updated:** 2026-06-07  
**Policy:** **No mocks.** Delete or replace every stub, simulated bookmark, padded fake `.q42`, println-only pipeline, and hardcoded graph scope. All user-visible “import / download / chat” paths must call real `qualia-core-db` infrastructure.

---

## 1. Scope

This plan covers the full desktop (Flutter + `qualia-client-core`) pipeline the user requested:

| # | Capability | Current state |
|---|------------|---------------|
| A | Unified resource **catalog** (LLMs, ontologies, SPARQL) | YAML exists; **three loaders disagree**; hubs often see **empty lists** |
| B | Ontology **download → real `.q42`** | Flutter uses `process_ontology()` stub; CLI path is closer but not wired to UI |
| C | LLM **download → GGUF shard map → lifecycle → inference** | Flutter downloads raw `.gguf` only; CLI has full pipeline; chat ignores lifecycle |
| D | **Vector / graph binding** (ontology terms ↔ LLM context) | Missing |
| E | **Chat sessions** (create, browse, resume) | Missing — in-memory only |
| F | **Chat environment** (ontologies, model, prior chat refs) | Missing |
| G | **Chat persistence as `.q42` / WAL** | Missing |
| H | **Anatomy qapp** bundled + chat handoff | Partial (seed path started; not in scope detail here — see §8) |

**Out of scope for this plan:** Ollama, external LLM HTTP servers, mock inference strings, simulated PDF/VLM bookmarks in `ingestion.rs`.

---

## 2. Non-mock inventory — delete or replace

| File / symbol | Problem | Replacement |
|---------------|---------|-------------|
| `qualia-client-core/src/engine/ingestion.rs` — `process_ontology`, `process_pdf` | Simulated bookmarks + println | Call `qualia_core_db::ingest::streaming_import_rdf` + real WAL append; PDF path uses real extractors or is disabled until implemented |
| `qualia-client-core/src/api.rs` — `ingest_ontology()` | Writes zero-padded fake `*.q42.bidx` | Remove from hot path; delegate to `ResourceImportService` (§4) |
| `qualia-client-core/src/api.rs` — `download_and_vectorize()` tail | Calls `process_ontology` stub | Call real ingest + index registration (§4) |
| `qualia-flutter/rust/src/api/qualia_api.rs` — `run_inference_stream` | Post-hoc word split, not token stream | Wire Phase 8 `run_agent_inference` via FRB or true stream sink |
| `qualia-flutter/rust/src/api/qualia_api.rs` — `requested_graph_scope: [q_hash("user:chat_context")]` | Hardcoded | Build from `ChatEnvironment` (§6) |
| `qualia-core-db/src/ingest.rs` — `deterministic_hash` placeholder in worker | Not `q_hash()` / lexicon | Use `q_hash()` + `resolver.rs` type tags; parity XOR fold |
| `fetch_*_catalog()` GitHub JSON manifests | Orphan duplicate of YAML | Deprecate; single YAML catalog via `ResourceCatalog::load()` |
| `asset_library_screen.dart` demo assets | Hardcoded heraldry rows | Keep as visual demo OR gate behind dev flag; not part of catalog pipeline |

---

## 3. Phase 0 — Catalog unification (blocker)

**Goal:** One canonical loader used by CLI, Flutter FRB, qapp readiness, and WASM tests.

### 3.1 Fix YAML shapes

Current files use **wrapped keys**; loaders expect wrong shapes:

```yaml
# resources/catalog.yaml — root is `catalog:`, not flat `sources:`
# resources/llms.yaml — root is `llms:`, not bare array
# resources/ontologies.yaml — root is `ontologies:`
# resources/sparql_endpoints.yaml — root is `sparql_endpoints:`
```

**Action:**

1. Add `crates/qualia-core-db/src/resource_catalog.rs`:
   - `pub fn load_from_dir(dir: &Path) -> Result<ResourceCatalog, CatalogError>`
   - Parse `catalog.yaml` as `{ catalog: { sources: { llms, ontologies, sparql_endpoints } } }`
   - Parse child files as `{ llms: Vec<LLMResource> }`, etc.
   - Resolve `DownloadInfo` for `huggingface`, `direct`, `github` (add `github` raw URL builder — schema.org entry uses `repo` + `path` today; extend `DownloadInfo`).

2. **Delete duplicate loaders:**
   - `qualia-flutter/rust/src/api/resource_catalog.rs` — thin FRB wrapper calling `qualia_core_db::resource_catalog::load_from_dir`
   - `qualia-cli/src/resources.rs` — call shared `load_from_dir`
   - `qualia-client-core/src/api.rs` — `workspace_resources_dir()` + ad-hoc structs → shared loader

3. Ship `resources/` next to desktop binary (`bundled/resources/`) with same resolution order as qapps:
   - `{exe}/bundled/resources/`
   - `QUALIA_RESOURCES_DIR` env
   - Dev: `CARGO_MANIFEST_DIR/../../resources`

### 3.2 FRB surface (catalog)

| Function | Returns |
|----------|---------|
| `load_resource_catalog()` | Full catalog summary JSON or typed structs |
| `list_catalog_llms()` | `Vec<LLMResource>` |
| `list_catalog_ontologies()` | `Vec<OntologyResource>` |
| `list_catalog_sparql_endpoints()` | `Vec<SPARQLResource>` |
| `find_catalog_llm(id)` | `Option<LLMResource>` |
| `find_catalog_ontology(id)` | `Option<OntologyResource>` |

### 3.3 Flutter UI

| Screen | Change |
|--------|--------|
| `ontology_hub_screen.dart` | Load via unified API; show error if catalog empty |
| `llm_hub_screen.dart` | Same |
| **New:** `sparql_hub_screen.dart` (optional P0.5) | Browse endpoints; probe via existing `resolve_sparql_endpoint_from_catalog` |
| `qapp_vault_screen.dart` | Readiness already uses catalog — auto-fix when loader unified |

### 3.4 Acceptance tests

- `cargo test -p qualia-core-db resource_catalog::loads_all_entries`
- Count: ≥3 LLMs, ≥5 ontologies, ≥3 SPARQL from committed YAML
- Headless: `docs/tests/suites/wasm-resources.js` once WASM exports wired to shared loader

---

## 4. Phase 1 — Real ontology → `.q42` pipeline

**Goal:** Ontology Hub “Import” produces queryable graph artifacts under `{storage}/Index/`, not fake bookmarks.

### 4.1 New module: `qualia-client-core/src/resource_import.rs`

```rust
pub struct OntologyImportResult {
    pub ontology_id: String,
    pub source_path: PathBuf,      // raw download
    pub q42_path: PathBuf,         // compiled block file
    pub wal_path: PathBuf,         // qualia_global.wal or ontologies.wal
    pub quin_count: u64,
    pub catalog_quins: usize,
}

pub async fn import_catalog_ontology(
    catalog: &ResourceCatalog,
    id: &str,
    storage_root: &Path,
) -> Result<OntologyImportResult, ImportError>;
```

**Steps (mirror `qualia-cli resources import-ontology`, no subprocess):**

1. Resolve entry from catalog; `download.resolved_url()` — fail if missing (no silent skip).
2. Stream to `{storage}/Index/{id}.{format}` (reuse `download_and_vectorize` HTTP loop).
3. `qualia_core_db::ingest::streaming_import_rdf(in, out)` → `{storage}/Index/{id}.q42`
   - **Upgrade ingest worker** to use `q_hash()` + proper `QualiaQuin` parity (§2).
4. Append provenance Quins: `OntologyResource::provenance_quin`, `to_quins()` → `WriteAheadLog`
5. Register in `daemon_graph` / mmap index so `/query` and scoped qapp queries see triples.
6. Write sidecar `{id}.q42.meta.json` (quin count, sha256, imported_at) for UI.

### 4.2 Remove stubs from Flutter path

- `ontology_hub_screen._importOntology()` → `importCatalogOntology(id)` FRB (not `downloadAndVectorize` + stub).
- `catalog.importOntology()` CLI subprocess → **fallback only** when `qualia-cli` on PATH and in-process fails.

### 4.3 Delete

- `ingestion::process_ontology` — replace callers with `resource_import`
- `api::ingest_ontology` mock bidx writer — remove or gate behind `#[cfg(test)]` fixture helper renamed `write_test_bidx_fixture`

### 4.4 Acceptance

- Import `foaf` (~100 KB) → non-zero `.q42` size, WAL mutations, daemon SPARQL returns ≥1 triple
- `inspect_installed_qapp_readiness("Anatomy")` marks `snomedct-us` / `q42:anatomy` correctly

---

## 5. Phase 2 — Real LLM package lifecycle

**Goal:** LLM Hub “Download + Activate” runs the same pipeline as `qualia-cli resources download`.

### 5.1 New module: `qualia-client-core/src/model_lifecycle.rs`

```rust
pub struct ModelInstallResult {
    pub model_id: String,
    pub gguf_path: PathBuf,
    pub profile_id: u64,
    pub pointer_quin_count: usize,
    pub lifecycle_state: ModelLifecycle,
}

pub async fn install_catalog_llm(id: &str, storage_root: &Path) -> Result<ModelInstallResult, ModelError>;
pub fn activate_model(profile_id: u64) -> Result<(), ModelError>;
pub fn get_active_model_status() -> ModelStatus;
```

**Steps (in-process, from `qualia-cli/src/resources.rs::cmd_download`):**

1. Stream GGUF → `{storage}/Models/{filename}`
2. `GGufSharder::generate_bidx_pointer_map()` → pointer Quins
3. WAL: provenance + `LLMResource::to_quins()` + pointers → `{storage}/Models/models.wal`
4. `CapabilityProfile` via `LLMResource::to_capability_profile`
5. `TaskOrchestrator` / `ModelLifecycle`: `Discovered → MappedToDisk → StreamingVRAM → Active`
6. Persist `active_model.json`: `{ id, path, profile_id, quantization }` (replace bare filename `active_model.txt`)

### 5.2 Flutter wiring

| Location | Change |
|----------|--------|
| `llm_hub_screen._downloadModel()` | Call `installCatalogLlm` not raw `downloadModel` only |
| `init_core()` | `load_active_model_from_disk()` → `activate_model` if file exists |
| `chat_screen` | Refuse infer if lifecycle ≠ `Active`; show “activate model in LLM Hub” |
| Show lifecycle chip on LLM Hub cards | Discovered / Active / Scrubbing |

### 5.3 Inference

- Expose `run_agent_inference` (Phase 8 bifurcated) via FRB **or** extend `run_inference_stream` to call `LocalLlmAgent::infer` with real token chunks on `StreamSink`
- Pass `CapabilityProfile.profile_id` and graph scope from chat environment (§6)

### 5.4 Acceptance

- Download `phi-3-mini-4k-instruct-q4km` (or small test GGUF in CI) → WAL quins > 0, profile_id stable
- Chat produces non-empty grounded output with ≥1 provenance citation (orchestrator post-flight)

---

## 6. Phase 3 — Ontology ↔ LLM graph binding (“vector mapping”)

**Goal:** Chat inference scope is derived from **installed ontologies** and **active model**, not a constant hash.

> **Clarification:** Real GGUF embedding lookup (`token_embd.weight`) is HANDOVER milestone #1. Until then, binding means **graph-scope Quins + lexicon hashes** compiled from installed `.q42` indexes, not fake float vectors.

### 6.1 New module: `qualia-client-core/src/context_binding.rs`

```rust
pub struct ChatEnvironment {
    pub session_id: String,
    pub active_model_profile_id: u64,
    pub ontology_ids: Vec<String>,           // catalog ids, installed
    pub prior_session_ids: Vec<String>,     // optional cross-session refs
    pub graph_scope_hashes: Vec<u64>,       // q_hash scopes for orchestrator
    pub lexicon_prefixes: Vec<u64>,         // top-N class/property hashes from ontologies
}

pub fn compile_chat_environment(
    storage: &Path,
    catalog: &ResourceCatalog,
    config: &ChatEnvironmentConfig,
) -> Result<ChatEnvironment, BindError>;
```

**Algorithm:**

1. For each selected installed ontology, mmap `.q42` / read WAL summary → collect subject/predicate frequency (cap N=256 per ontology, stack buffers in hot path for compile step only).
2. Merge with `CapabilityProfile` tensor modality flag quins for active model.
3. Emit `graph_scope_hashes` = `{ q_hash("chat:session:{id}"), q_hash("ont:{id}"), ... }`
4. Store compiled environment to `{storage}/Chats/{session_id}/environment.json` + optional binary `environment.q42` (16 Quins max for scope manifest).

### 6.2 Future: true embedding alignment

When `GgufTensorIndex` lands:

- Add `bind_lexicon_to_embedding_table(model, ontology_lexicon) -> BindingTable`
- Store binding table mmap path in `ChatEnvironment`
- **Still no mocks** — if embeddings unavailable, fail with clear error, don’t sin-based pseudo-embed in production path (keep pseudo only for `cfg(test)` / WASM playground).

---

## 7. Phase 4 — Chat sessions + `.q42` persistence

**Goal:** Multi-chat history browsable in UI; messages durable; citations groundable.

### 7.1 Storage layout

```
{storage}/
  Chats/
    {session_uuid}/
      session.json          # metadata, title, created, updated, environment refs
      environment.json      # ChatEnvironment snapshot
      messages.jsonl        # append-only UX log (role, content, ts) — optional mirror
      chat.wal              # QualiaQuin mutations (messages as quins)
      chat.q42              # periodic compaction of WAL (ingest.rs block format)
      citations.q42         # provenance quins from orchestrator post-flight
```

### 7.2 Quin encoding for messages

| Field | Mapping |
|-------|---------|
| subject | `q_hash("chat:session:{uuid}")` |
| predicate | `q_hash("chat:hasMessage")` |
| object | `q_hash("msg:{lamport}")` MSB nested bit for RDF-star depth if needed |
| context | `q_hash("chat:role:{user\|agent}")` |
| metadata | Lamport clock + message index |

Agent messages with provenance: additional quins `q42:groundedBy` → citation quin hashes (orchestrator output).

### 7.3 New module: `qualia-client-core/src/chat_session.rs`

```rust
pub fn create_session(title: Option<String>, env: ChatEnvironment) -> Result<String, ChatError>;
pub fn list_sessions() -> Result<Vec<ChatSessionSummary>, ChatError>;
pub fn load_session(id: &str) -> Result<ChatSession, ChatError>;
pub fn append_message(id: &str, role: Role, content: &str) -> Result<u64, ChatError>;
pub fn compact_session_to_q42(id: &str) -> Result<PathBuf, ChatError>;
pub fn delete_session(id: &str) -> Result<(), ChatError>;
```

**Hot path:** `append_message` appends to WAL only (zero heap in append after UTF-8 scan — caller passes `&[u8]` for content hash, store full text in sidecar chunk outside hot path if needed).

### 7.4 Flutter UI

| Component | Description |
|-----------|-------------|
| `chat_history_drawer.dart` | Sidebar: list sessions, new chat, delete |
| `chat_screen.dart` | Load session on open; save each message; show session title |
| `chat_environment_sheet.dart` | Bottom sheet: pick ontologies (installed), model, link prior sessions |
| `main.dart` | Persist `last_session_id` in config |

### 7.5 FRB

Expose all `chat_session::*` + `compile_chat_environment` to Dart.

---

## 8. Phase 5 — End-to-end chat integration

**Single send flow (no mocks):**

```
User sends prompt
  → load ChatEnvironment for session
  → compile / refresh graph_scope_hashes
  → optional: query installed ontologies (scoped SPARQL) for retrieval context
  → orchestrate_inference(LocalLlmAgent, prompt, graph_context, intent)
  → post-flight provenance check (reject ungrounded — show error in UI)
  → append user + agent quins to session WAL
  → stream tokens to UI (real stream)
  → update session.json updated_at
```

**Files to modify:**

- `qualia-flutter/rust/src/api/qualia_api.rs` — `run_inference_stream(prompt, model_path, session_id, sink)`
- `qualia-client-core/src/api.rs` — `handle_engine_chat_command` receives session + environment
- `chat_screen.dart` — full wiring

---

## 9. Phase 6 — Anatomy qapp + catalog cross-links

(Completes parallel work; depends on Phase 0–1.)

- `bundled_qapps.rs` seeds Anatomy; requires `snomedct-us`, `shacl`, etc. in catalog
- Chat “Open in Anatomy” uses real `build_anatomy_graph_context_json_with_dicom` + daemon query counts
- `submit_dicom_ingest` → poll `dicom_ingest_status` → optional `execute_dicom_volume_query` (FRB exists; wire UI poll)
- Qapp readiness uses unified catalog loader

---

## 10. Implementation order (sprints)

| Sprint | Deliverable | Blocks |
|--------|-------------|--------|
| **S0** | Phase 0 catalog loader + bundled resources + hub empty-state fix | Everything |
| **S1** | Phase 1 ontology import in-process + ingest `q_hash` fix | Chat binding |
| **S2** | Phase 2 LLM install/activate + init_core restore | Chat infer |
| **S3** | Phase 4 session WAL + history UI | Chat UX |
| **S4** | Phase 3 environment compiler + settings sheet | Grounded infer |
| **S5** | Phase 5 integrated send + provenance enforcement | Done |
| **S6** | Phase 6 Anatomy/DICOM polish | Optional parallel |

Estimated: **S0–S2 ≈ 3–4 sessions**, **S3–S5 ≈ 3–4 sessions**.

---

## 11. Testing strategy (no mock data in integration tests)

| Layer | Test |
|-------|------|
| Rust unit | `resource_catalog::load_from_dir` counts; `compile_chat_environment` scope hashes deterministic |
| Rust integration | Import `foaf` fixture TTL → query daemon; install tiny GGUF fixture → lifecycle Active |
| FRB | Round-trip `list_sessions` / `append_message` |
| Flutter widget | Chat drawer lists seeded sessions from test storage dir |
| Browser | `wasm-resources` against unified catalog export (WASM reads JSON snapshot, not duplicate YAML parser) |
| CI gate | **`#[ignore]` mocks removed** — grep CI fails on `simulated_bookmarks`, `deterministic_hash` in ingest hot path |

Fixtures: commit small `tests/fixtures/foaf.ttl`, `tests/fixtures/tiny.gguf` (or download in CI job from catalog id).

---

## 12. File touch map

### New files

| Path | Purpose |
|------|---------|
| `crates/qualia-core-db/src/resource_catalog/load.rs` | Unified YAML loader (or extend existing module) |
| `crates/qualia-client-core/src/resource_import.rs` | Ontology download + ingest |
| `crates/qualia-client-core/src/model_lifecycle.rs` | GGUF install + activate |
| `crates/qualia-client-core/src/context_binding.rs` | ChatEnvironment compiler |
| `crates/qualia-client-core/src/chat_session.rs` | Session WAL + q42 |
| `crates/qualia-flutter/lib/screens/chat_history_drawer.dart` | History UI |
| `crates/qualia-flutter/lib/screens/chat_environment_sheet.dart` | Env settings |
| `scripts/copy-bundled-resources.ps1` | Stage YAML with desktop dist |

### Major edits

| Path | Change |
|------|--------|
| `crates/qualia-core-db/src/ingest.rs` | Real `q_hash` quins, LZ4 block write per existing format |
| `crates/qualia-client-core/src/engine/ingestion.rs` | Delete or redirect to `resource_import` |
| `crates/qualia-client-core/src/api.rs` | Remove mock ingest; add session/import/lifecycle delegates |
| `crates/qualia-flutter/rust/src/api/resource_catalog.rs` | Thin wrapper only |
| `crates/qualia-flutter/rust/src/api/qualia_api.rs` | Session-aware inference |
| `crates/qualia-cli/src/resources.rs` | Use shared loader + shared import/install fns |
| `ontology_hub_screen.dart`, `llm_hub_screen.dart`, `chat_screen.dart` | Wire new APIs |
| `HANDOVER.md` | Mark milestones complete as sprints land |

### Deprecate

- `fetch_ontology_catalog` / `fetch_model_catalog` (GitHub JSON)
- `ingest_ontology` mock bidx API (after migration window)
- `download_and_vectorize` name → `import_catalog_ontology` (alias deprecated one release)

---

## 13. Acceptance criteria (definition of done)

- [ ] Ontology Hub lists all YAML entries (not empty) in dev and packaged desktop build
- [ ] Import ontology → real `.q42` + WAL; daemon query returns triples
- [ ] LLM Hub download → GGUF + pointer quins + `CapabilityProfile`; Activate → `ModelLifecycle::Active`
- [ ] Chat:create session → send message → restart app → history restored
- [ ] Chat environment selects ≥1 installed ontology; inference scope quins change when selection changes
- [ ] Agent reply rejected by UI if orchestrator post-flight finds zero provenance quins
- [ ] No production code path calls `process_ontology`, `process_pdf` simulated bookmarks, or fake bidx padding
- [ ] `cargo test --workspace` and headless `docs/tests/run-headless.mjs` pass

---

## 14. References

| Doc | Relevance |
|-----|-----------|
| `CLAUDE.md` | Native LLM stack, no Ollama |
| `AGENTS.md` | Zero-heap hot paths for evaluators; chat WAL compile step may alloc |
| `crates/qualia-cli/src/resources.rs` | **Canonical** download/import pipelines to lift in-process |
| `crates/qualia-core-db/src/resource_catalog.rs` | Canonical types + `to_quins()` |
| `crates/qualia-core-db/src/orchestrator.rs` | `orchestrate_inference` governance |
| `app-development/Anatomy/qapp.json` | Required ontologies for readiness checks |
| `docs/planning/desktop-chat-resource-pipeline.md` | This document |

---

## 15. Session handoff note

When starting implementation, **begin with S0 (catalog loader)**. Until hubs show real entries, every downstream UI test looks like a Flutter bug when it is actually a YAML parse mismatch between `resource_catalog.rs` (Flutter), `api.rs` (readiness), and `qualia-cli/resources.rs` (CLI).

**First PR title suggestion:** `fix(catalog): unify YAML resource loader and bundle resources in desktop dist`
