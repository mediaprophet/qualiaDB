# QualiaDB — Handover Document

---

## Session 2026-06-06 — v0.0.6 release (App Vault, WASM, CI rebuild)

**Branch:** `main` | **Tag:** `v0.0.6` | **Last commit:** `8579f22`

### Completed this session
- **App Vault** (Flutter): renamed from AppStore, FRB-wired (`list_installed_apps`,
  `launch_installed_app`, `generate_app_credential`), system browser launch via
  `url_launcher`, directory picker for install via `file_picker`, `dev_port` in manifest
- **FRB**: fixed `flutter_rust_bridge.yaml` (was pointing at deleted `api.rs`),
  regenerated all bindings including new `resource_catalog.dart`
- **WASM**: rebuilt `qualia_core_db_bg.wasm` (465KB, 29 exports, was stale 46KB),
  `pages.yml` now rebuilds WASM on every deploy
- **CI**: `release.yml` rewritten (Flutter + WASM + CLI, Tauri removed),
  `permissions: contents: write` added (was causing 403 on all uploads)
- **Docs**: `app-vault-developer-guide.md`, `BUILD_ERRORS_V0-0-6.md`, README warning
- **Cleanup**: cooperative/wellfair/social → `app-development/` (gitignored),
  migration scripts → `scratch/` (gitignored), node.zip deleted, 1601 social files
  removed from tracking, broken Unicode-named directory deleted

### Immediate next tasks
1. **Flutter Linux/macOS platforms**: `flutter create --platforms=linux,macos .`
   in `crates/qualia-flutter/` then commit. Blocks those release jobs.
2. **DirectML SDK**: see `docs/BUILD_ERRORS_V0-0-6.md` §Error 3
3. **benchmark.html**: wire to actual `qualia_core_db.js` WASM, not JS simulation
4. **WellFair app**: `app-development/wellfair/app.json` exists, needs `index.html`
   + daemon query JS (port 5173, CORS-allowed in dev mode)
5. **Node.js 24**: add `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true` to `release.yml`

### Key files changed
`crates/qualia-flutter/lib/main.dart`, `lib/screens/app_vault_screen.dart`,
`lib/screens/wallet_screen.dart`, `lib/src/rust/api/qualia_api.dart`,
`lib/src/rust/api/resource_catalog.dart` (new), `rust/src/api/qualia_api.rs`,
`rust/src/api/resource_catalog.rs`, `flutter_rust_bridge.yaml`, `pubspec.yaml`,
`pubspec.lock`, `frb_generated.{dart,rs}`, `crates/qualia-client-core/src/api.rs`,
`crates/qualia-client-core/src/app_registry.rs`, `.github/workflows/release.yml`,
`.github/workflows/pages.yml`, `.gitignore`, `README.md`,
`docs/playground/qualia_core_db{.js,_bg.wasm}` (rebuilt)

---

## Session 2026-06-05 — Full codebase audit
_Full codebase audit completed: 2026-06-05._
_Covers qualia-core-db (55 modules), qualia-desktop, and qualia-client._

---

## 1. What Was Fixed in This Sprint

| # | Fix | Files |
|---|-----|-------|
| 1 | Path traversal in `qualia://` scheme handler fixed with `Path::components()` filter | `main.rs:1205` |
| 2 | Tauri updater set to `"active": false` — placeholder pubkey was crashing update check on every boot | `tauri.conf.json` |
| 3 | Chat drag-drop uses `tauri://file-drop` (absolute paths) | `Chat.tsx` |
| 4 | Wallet Send: in-modal status replaces `window.alert()` | `Wallet.tsx` |
| 5 | Nym/STARK toggles return new bool state | `main.rs`, `Wallet.tsx` |
| 6 | Imported accounts persist to `imported_accounts.json` | `main.rs`, `CredentialManager.tsx` |
| 7 | Settings Tax Router loads from `get_tax_suite` | `Settings.tsx` |
| 8 | App Manager wired: list, launch, sign, generate VC | `main.rs`, `AppStore.tsx` |
| 9 | Dashboard buttons wired to `run_engine_command`; live terminal log | `main.rs`, `Dashboard.tsx` |
| 10 | Address Book Connect button wired (session-persistent) | `AddressBook.tsx` |
| 11 | LLM Hub: remote manifest fetch, download-state persists page nav, Load/Active button | `main.rs`, `LLMHub.tsx` |
| 12 | Ontology Hub: same remote manifest + persistence pattern | `main.rs`, `OntologyHub.tsx` |
| 13 | Chat: shows active model name, blocks Execute if none loaded | `Chat.tsx` |
| 14 | `manifests/models.json` + `manifests/ontologies.json` created | `manifests/` |

---

## 2. Critical — Must Fix Before Shipping

### 2-A  Daemon auth blocks all queries (NEW — found in full audit)
**File:** `qualia-desktop/src/main.rs` — `start_local_daemon(4242)` call

The daemon starts with `dev: false`. All POST /query calls require `X-Qualia-Token` matching
`$QUALIA_TOKEN` env var. This env var is never set. Result: every query returns 401.
Fix: call `start_local_daemon_with_options(4242, true)` for local desktop use.

### 2-B  Updater pubkey is a placeholder
**File:** `crates/qualia-desktop/tauri.conf.json:24`
`"active": false` is the current workaround. Before shipping: run `tauri signer generate`,
add real pubkey, re-enable. Add private key as `TAURI_PRIVATE_KEY` GitHub secret.

### 2-C  Path traversal in `qualia://` handler
**Status:** FIXED (2026-06-05). `Path::components()` filter now in place.

---

## 3. Engine Capability Inventory

### Tier 1 — Real, production-ready, tested

| Module | What it does |
|--------|-------------|
| `ingest.rs` | Multi-threaded RDF/XML, Turtle, N-Triples, N3 pipeline via `rio_xml`/`rio_turtle`. Hardware-scales to 80% CPU. LZ4-compressed output. **NOT called by desktop.** |
| `logic.rs` | 18-opcode Webizen bytecode VM: `MatchSubject/Predicate/Object`, `EvalMetadataMask`, `LessThan/GreaterThan/LessOrEqual/GreaterOrEqual`, `Always/Eventually/Next` (LTL), `EmitQuin`, `YieldConfidence`, `LoadModel/EvictModel`. Defeasible claim pruning via `prune_defeasible_claims()`. VM suspension via `flatten_to_suspended()`. |
| `webizen_bytecode.rs` | Binary bytecode executor. MSB dispatch: `did:q42` topological pointers vs FNV-1a hashes. WASM SIMD variant (`execute_program_simd`) loads 48-byte Quins into 3×v128 registers. Reports `X-Qualia-Compute-Cost` header. |
| `mini_parser.rs` | Zero-allocation N-Triples pattern compiler. 5-opcode binary encoding. `hash_token()` routes `did:q42:` URIs through `identifier.rs`. Used by daemon POST /query. |
| `webizen.rs` | `SlgArena` — 42MB ring buffer (917,504 Quin slots), O(1) hash-addressed sub-goal tabling. `SlgOpcode` WAM variant with `NativeThermodynamics`, `NativeOdeSolver`, `NativeQuantumDft`, `NativeBioinformatics` opcodes. `AgreementDID::compile_to_super_quins()` — ratified agreements produce 16 Quins in `EnforceBilateralMicroCommons` lane. **Not exposed via any Tauri command.** |
| `crdt.rs` | LWW CRDT with Lamport clocks. `SuspendedTransactionQueue` (32-slot fixed array). `apply_consensus_token()` resumes suspended VM execution on M-of-N signatures. Tested. |
| `sync.rs` | `MerkleCrdtSynchronizer::diff_jump_tables()` — O(N) P2P block-level diff. Epoch compaction. Conflict resolution with tombstone detection. |
| `agency.rs` | Author-scoped Merkle root (Sha256 over Quins filtered by `context == author_did`). Ed25519 sign/verify (`sign_agency_root`, `verify_human_agency`). Deniable encryption PIN → AES-256-GCM key via `derive_lane_key()`. |
| `resolver.rs` | Zero-allocation N-Triples formatter. Inline type detection: `xsd:integer`, `xsd:decimal`, `xsd:boolean`. `did:q42` topological pointer rendering. Demo lexicon (15 entries); production needs mmap'd `.q42.dict`. |
| `archive.rs` | `Q42Archive` — memmap2-based zero-deserialization reader. 64-byte preamble, 4-tier jump tables, Zstd dictionary decompression. |
| `storage.rs` | `SuperBlockWriter` — page-aligned 40,960-byte block writes (OS-specific). `VirtualFileSystem` trait: `NativeVfs` (real), `OpfsVfs` (scaffolded, TODO). |
| `wal.rs` | Append-only WAL with sync_data, recovery from partial writes, truncation after commit. |
| `daemon.rs` | Warp HTTP server: GET /health, POST /query (N-Triples → bytecode → VM → N-Triples/JSON-LD), POST /cache, WebSocket /qualia-bridge. Compile-cost telemetry header. **Daemon executes against empty slice — see Critical Issue 2-A.** |
| `orchestrator.rs` | `TaskOrchestrator`: model lifecycle (`Discovered→MappedToDisk→StreamingVRAM→Active→Scrubbing`). `ThermalGovernor` trait. Async memory scrubbing. `orchestrate_inference()`: validate_intent → infer → validate_output. Tested. **Not used by `llm_offload.rs`.** |
| `llm_agent.rs` | `AgentRuntime` trait. `LocalLlmAgent` with 5 Webizen rules (outbound telemetry, sanctuary scope, provenance required, token budget, remote consent). 128 MB memory budget. `infer_local_model()` is stub. |
| `n3_parser.rs` | Streaming N3 parser: triples, implications (`=>`/`~>`/`^>`/`-o`), weighted rules, ASP blocks (`#asp {}`), diffuse blocks (`qualia:diffuse {}`). Registered into `SlgArena`. |
| `ilp_dispatcher.rs` | `IlpTransport` trait + `HttpIlpTransport` (logs, needs ILP connector sidecar for full STREAM). `resolve_payment_pointer()` — `$host/path` → HTTPS. `generate_energy_of_logic_invoice()` reads `ATOMIC_FLOPS_COUNT`. Offline queue. 5 integration tests pass. |
| `rpc.rs` | `TaxRecipientSuite::default_cooperative()` — 12% split. `route_tax_payment()`. `ComputeCostReceipt::generate()` from telemetry. |
| `npu_ffi.rs` | `SieveOrchestrator::execute_sieve()` dispatches WGPU compute shaders, returns 27-u32 bitmask over 850 Quins. C-ABI FFI: `nets_map_lorentz()`, `nets_tropical_voronoi_route()`. Pure Rust: SMILES parsing, valency calculation. |
| `bioinformatics.rs` | Smith-Waterman alignment with SIMD dispatch: AVX2 (x86_64), NEON (aarch64), scalar fallback. |
| `thermodynamics.rs` | Metropolis-Hastings MCMC. Boltzmann acceptance. Gibbs free energy. |
| `ode_solver.rs` | Runge-Kutta 4th-order integrator (`rk4_step`). |
| `quantum_dft.rs` | `ElectronDensity` grid. Kohn-Sham ground state energy. PINN receptor binding affinity. |
| `geometric.rs` | Lorentz vector mapping. Tropical (Min-Plus) distance. `HomologicalSieve::evaluate_topology_tick()`. |
| `cbor_compiler.rs` | Strict binary gatekeeper (rejects JSON/XML/Turtle). CBOR-LD → Quin parser. Variable-length integer reading. Tested. **Not reachable from any Tauri command.** |
| `shacl_compiler.rs` | SHACL constraint → `SlgOpcode` compiler. Custom constraints: `qualia:thermoMetropolisStep` → `NativeThermodynamics`, `qualia:solveOdeDynamics` → `NativeOdeSolver`, `qualia:dftGroundState` → `NativeQuantumDft`, `qualia:bioSequenceAlignment` → `NativeBioinformatics`. |
| `wasm_bridge.rs` | `execute_ntriples_query()` — REAL (mini_parser + webizen_bytecode). `QualiaWasmBridge::dispatch_query()` — stub. |
| `wasm_edge.rs` | `FederatedNodeManager` (WebRTC offloading). `intercept_computational_opcode()`. `webizen_propose_agreement/poll/sign()`. `serialize_float_array/64()` for IEEE-754 safe boundary crossing. All `#[wasm_bindgen]` annotated. |
| `gguf_bridge.rs` | `QTensor::map_from_pointer()` extracts tensor from Quin 60-bit pointer. `QTensorEngine::decode_lexicon_bound()` — **Lexicon-Bound Decoding**: forces LLM to only output tokens present in `.q42.bidx` lexicon. Logit masking is mocked; interface is correct. |

### Tier 2 — Stubs / Partial

| Module | Status |
|--------|--------|
| `ingestion.rs` (desktop) | Stub. `process_ontology()` has comment "would use rio_turtle" — engine already does. Call `qualia_core_db::ingest::streaming_import_rdf()`. |
| `q42_compiler.rs` (desktop) | Complete stub. Should convert VLM graph → CBOR-LD → SuperBlocks → embed HCAI agreements. |
| `spatial.rs` | `SpatiotemporalQuadTree::query_region()` returns empty. `embed_h3_context()` is passthrough. |
| `query_compiler.rs` | Regex heuristic routing only. Not full SPARQL. |
| `modalities/asp.rs` | `generate_stable_models()` returns two hardcoded worlds. |
| `modalities/dl.rs` | `check_subsumption()` always returns false. |
| `modalities/linear.rs` | `consume_resource()` prints and returns true. |
| `modalities/probabilistic.rs` | `evaluate_threshold()` is O(1) comparison. |
| `solid_ldp.rs::quin_to_turtle()` | Emits `urn:qualia:node:X` pseudo-URIs. Should call `resolver::format_ntriples_to()`. |
| `gguf_sharder.rs` | `extract_ontology_to_superblock()` returns zeroed block. |
| `nym_adapter.rs` | Mock Sphinx dispatch. |
| `tee_ffi.rs` | C-ABI declarations only. |
| `indexing.rs` | Empty file. |

---

## 4. Known Format Inconsistency

Three incompatible `.q42` write formats exist:
- `storage.rs::SuperBlockWriter` — raw 40,960-byte `QualiaSuperBlock` structs (no header)
- `ingest.rs::streaming_import_rdf` — LZ4-compressed variable blocks with `block_id+len` header
- `archive.rs::Q42Archive` — reader expecting Zstd + 64-byte preamble + jump tables

**None of the writers produce what the archive reader expects.** See Phase 1 of PLAN.md
for the unification approach: `SuperBlockWriter` as canonical on-disk format.

---

## 5. Architecture Notes

### IPC Pattern
Frontend `invoke('command_name', { camelCaseArgs })` → Rust `snake_case` params via serde.
Events back: `app.emit_all("event-name", payload)` → `listen('event-name', handler)`.

### Storage Layout (Windows)
- Config + identity: `%APPDATA%\Qualia\` (fixed)
- Models / indexes / apps: `%APPDATA%\QualiaData\` (user-configurable in Settings)
- Active model: `%APPDATA%\Qualia\active_model.txt`

### Key Engine Integration Points Not Yet Used
- `qualia_core_db::ingest::streaming_import_rdf()` — call from `ingestion.rs`
- `qualia_core_db::orchestrator::TaskOrchestrator::orchestrate_inference()` — use in `llm_offload.rs`
- `qualia_core_db::webizen::AgreementDID::compile_to_super_quins()` — expose via Tauri
- `qualia_core_db::webizen::SlgArena::execute_vm_frame()` — use in daemon for N3Logic
- `qualia_core_db::cbor_compiler::ingest_network_payload()` — add POST /ingest-cbor endpoint
- `qualia_core_db::ilp_dispatcher::generate_energy_of_logic_invoice()` — wire to inference
- `qualia_core_db::agency::sign_agency_root()` — use for app credentials + Front Door DIDs
- `qualia_core_db::npu_ffi::SieveOrchestrator::execute_sieve()` — pre-filter in daemon query
- `qualia_core_db::sync::MerkleCrdtSynchronizer::diff_jump_tables()` — P2P sync

### Daemon Token Auth
The daemon requires `X-Qualia-Token` matching `$QUALIA_TOKEN` env var when `dev: false`.
The desktop starts the daemon without setting this. Fix: `start_local_daemon_with_options(4242, true)`.

### Remote Manifests
`fetch_remote_manifest(url)` runs in Rust (reqwest), bypassing frontend CSP.
`manifests/models.json` and `manifests/ontologies.json` must be on GitHub `main` branch.
Until pushed, both Hubs silently fall back to bundled lists.

### Active Model Sync
Stored in `active_model.txt`, cached in `AppState::active_model`. The `active-model-changed`
Tauri event keeps LLM Hub and Chat in sync.

---

## 6. Deployment Checklist

- [ ] Fix daemon auth: `start_local_daemon_with_options(4242, true)` in main.rs
- [ ] `tauri signer generate` → replace updater pubkey → re-enable `"active": true`
- [ ] Push `manifests/` to GitHub main
- [ ] Generate real Tauri icons (current may be placeholders)
- [ ] Wire `releases/latest.json` to GitHub Pages for updater
- [ ] Code-sign: Windows (SmartScreen EV cert), macOS (Apple Developer ID + notarization)

---

## 7. Implementation Roadmap

See [PLAN.md](PLAN.md) for the full 7-phase implementation plan.

**Phase 0** (structural): Split `main.rs` into `commands/` modules + `engine/` shims.
**Phase 1** (data): Wire `streaming_import_rdf`, unify Q42 format, fix daemon live index.
**Phase 2** (LLM): Wire `TaskOrchestrator`, Ollama streaming, real WebizenVM SPSC intercept.
**Phase 3** (agreements): Surface `AgreementDID` + CRDT consent flow in AddressBook.
**Phase 4** (wallet): BIP32/BIP44, real balances, ILP audit trail.
**Phase 5** (P2P): librqbit, LLaVA, SPARQL-MM, CRDT sync, GPU sieve.
**Phase 6** (WASM): wasm-pack build, OPFS, SIMD, zero-copy WebGL.
**Phase 7** (production): ZK-STARK, Nym, Gun.eco, TEE, CI/CD signing.

---

## 8. Intentionally Deferred (Not Bugs)

- LLM token streaming — hardcoded output in `llm_offload.rs`
- Nym relay packet routing — simulated loop
- ZK-STARK proof generation — simulated loop
- Wallet coin balances and tx history — hardcoded
- Front Door DID uniqueness — hardcoded string
- BIP32 HD key derivation — hex-slice mock
- Ontology RDF parsing — mock bookmarks (engine has it, desktop doesn't call it)
- Torrent swarm telemetry — hardcoded stats
- Asset library LLaVA extraction — 3 s sleep + hardcoded JSON
- ALP token minting — hardcoded token ID
- ILP STREAM payments — SPSP resolution works, full STREAM needs connector sidecar
- WASM OPFS bindings — scaffolded, two TODOs
