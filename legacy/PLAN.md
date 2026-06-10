# QualiaDB — Full Implementation Plan
_Last audited: 2026-06-05. Based on complete source read of all 55 engine modules + desktop + UI._

---

## Orientation

QualiaDB is a three-tier system:

```
qualia-client (React/Vite)
      ↕  Tauri IPC
qualia-desktop (Rust shell — 37 commands in main.rs)
      ↕  crate dependency
qualia-core-db (Engine — 55 modules, largely real implementations)
```

The engine is substantially more complete than the desktop exposes. The plan below works
from the inside out: structural cleanup first, then engine wiring, then features.

---

## Phase 0 — Structural Refactor  (1–2 days)
**Do this before any feature work. Every subsequent task is faster once main.rs is split.**

`qualia-desktop/src/main.rs` is 1,335 lines handling Tauri setup, all 37 commands, system
tray, URI scheme, download engine, BIP39, token registry, ILP — everything in one file.
This makes every audit, change, and AI-assisted session miss things.

### Target layout

```
qualia-desktop/src/
├── main.rs                   (~80 lines — Tauri builder only)
├── state.rs                  (AppState + initialization + helpers)
├── tray.rs                   (build_tray, handle_tray_event, toggle_window)
├── commands/
│   ├── mod.rs                (single invoke_handler! registration point)
│   ├── config.rs             (get_config, save_config, is_first_run, run_engine_command)
│   ├── identity.rs           (BIP39, wallet derivation, front door DID, imported accounts)
│   ├── wallet.rs             (balances, tx history, tokens CRUD, Nym/STARK toggles,
│   │                          dispatch_tax_payment, get/save_tax_suite)
│   ├── models.rs             (discover_models, run_agent_inference, get/set_active_model)
│   ├── ontology.rs           (ingest_ontology, ingest_literature, export_to_solid,
│   │                          upsert_cmld_definition)
│   ├── assets.rs             (ingest_image, ingest_image_async, fetch_torrent_telemetry)
│   ├── qapps.rs              (list_installed_qapps, qapp credential VC, launch_installed_qapp)
│   ├── daemon.rs             (query_daemon HTTP proxy, check_daemon_health)
│   ├── downloads.rs          (download_model, download_and_vectorize, cancel_download,
│   │                          get_active_downloads, fetch_remote_manifest)
│   └── hardware.rs           (get_hardware_status, profile_energy_circumstance,
│                               check_ollama_status)
├── engine/
│   ├── mod.rs
│   ├── ingest.rs             (shim → qualia_core_db::ingest::streaming_import_rdf)
│   ├── q42.rs                (shim → SuperBlockWriter + new SuperBlockReader)
│   ├── agreement.rs          (shim → AgreementDID, SlgArena, SuspendedTransactionQueue)
│   └── orchestrator.rs       (shim → TaskOrchestrator)
└── llm_offload.rs            (keep, migrate to use engine/orchestrator.rs)
```

The `engine/` directory is the explicit wiring manifest: if a capability has no file here,
it is not exposed. Gaps are immediately visible.

### Also in Phase 0
- Fix daemon auth: pass `dev: true` when starting from desktop (`start_local_daemon_with_options(4242, true)`). The current `dev: false` means every POST /query returns 401.
- Fix path traversal: already done (2026-06-05). Verify it survived the refactor.
- Push `manifests/` to GitHub main branch.

---

## Phase 1 — Data Foundation  (3–4 days)
**Unblocks everything. The daemon, ontology parsing, and RAG all depend on this.**

### 1.1  Wire `streaming_import_rdf` into the desktop

`qualia-desktop/src/ingestion.rs::process_ontology()` returns mock bookmarks.
`qualia_core_db::ingest::streaming_import_rdf()` is a real multi-threaded pipeline using
`rio_xml::RdfXmlParser`, `rio_turtle::TurtleParser`, `NTriplesParser`, and `n3_parser::N3Parser`.

In `engine/ingest.rs`:
```rust
pub fn ingest_file(input: &str, output_q42: &str) -> std::io::Result<()> {
    qualia_core_db::ingest::streaming_import_rdf(input, output_q42)
}
```

`commands/ontology.rs::ingest_ontology` calls this. One-line fix.

### 1.2  Unify the Q42 write format

Three incompatible write formats exist today:
- `storage.rs::SuperBlockWriter` — raw 40,960-byte aligned blocks (no header)
- `ingest.rs::streaming_import_rdf` — LZ4-compressed variable blocks
- `archive.rs::Q42Archive` — Zstd + 64-byte preamble + jump tables (read-only today)

**Decision:** make `SuperBlockWriter` the canonical on-disk format for the index directory.
Modify `streaming_import_rdf`'s writer thread to accumulate 850 Quins, then call
`SuperBlockWriter::flush_block()` instead of the custom LZ4 writer.

Add `SuperBlockReader` to `storage.rs` — the symmetric read path using `QuinIncrementalScanner`.

Keep `Q42Archive` for distribution/archival exports only (compressed bundles for download).

### 1.3  Wire the daemon's live index

In `daemon.rs:292`:
```rust
let current_database_state: &[crate::QualiaQuin] = &[];  // ← ALWAYS EMPTY
```

At daemon startup, scan `%APPDATA%\QualiaData\Index\` for `*.q42` files. Load them into a
`Arc<RwLock<Vec<QualiaQuin>>>` held in a `lazy_static`. Every POST /query executes against
this live slice. Add a `reload_index()` endpoint so the daemon picks up newly indexed files
without restart.

### 1.4  Persist N3Logic rules from ingestion

In `ingest.rs`, N3 rules are registered into a stack-local `SlgArena` and then dropped.
Fix: write each `N3Event::LogicRule` to a `.q42.rules` NDJSON sidecar alongside the `.q42`
file during ingestion. On daemon startup, load all sidecars and call `arena.register_rule()`
for each one. The daemon's 60-second periodic task can then run `execute_differential_diagnostics()`
over the live index instead of just printing.

### 1.5  Add `query_daemon` Tauri command

Proxy wrapper in `commands/daemon.rs`:
```rust
#[tauri::command]
async fn query_daemon(pattern: String) -> Result<String, String> {
    reqwest::Client::new()
        .post("http://127.0.0.1:4242/query")
        .json(&serde_json::json!({ "query": pattern, "format": "n-triples" }))
        .send().await?.text().await.map_err(|e| e.to_string())
}
```
With `dev: true` on the daemon, no auth header is needed for local calls.

---

## Phase 2 — Real LLM via TaskOrchestrator  (4–5 days)
**Depends on Phase 1 (daemon queries real data for RAG context).**

### 2.1  Ollama streaming in `llm_offload.rs`

Replace the hardcoded word-loop with:
```rust
let client = reqwest::Client::new();
let mut stream = client.post("http://localhost:11434/api/generate")
    .json(&json!({ "model": model_name, "prompt": prompt, "stream": true }))
    .send().await?.bytes_stream();
// emit each `response` field chunk as "llm-token" event
```
`check_ollama_status()` is the gate. The `active_model.txt` path is already wired.

### 2.2  Use `TaskOrchestrator::orchestrate_inference()`

Replace raw thread spawning in `execute_agent_inference` with:
```rust
let orch = TaskOrchestrator::new(Box::new(SysinfoThermalGovernor::new()));
match orch.orchestrate_inference(&agent, &prompt, &graph_context, intent) {
    OrchestrationResult::Committed { text, .. } => { /* stream text */ }
    OrchestrationResult::Blocked { reason, .. } => { /* emit shield alert */ }
    OrchestrationResult::Failed(e) => { /* emit error */ }
}
```

Implement `SysinfoThermalGovernor` using the existing `sysinfo` dependency to return
`ThermalStatus::Critical` when CPU temperature exceeds threshold.

### 2.3  RAG context from daemon query

Before passing the prompt to the LLM, extract key terms and query the daemon:
```rust
let pattern = format!("?s ?p <{}> .", q_hash_uri(&key_term));
let context_triples = query_daemon(pattern).await?;
// pass context_triples as graph_context to orchestrate_inference
```
`format_ntriples_to()` in `resolver.rs` produces the output; `compile_ntriples_to_bytecode()`
in `mini_parser.rs` compiles the pattern. Both are real and tested.

### 2.4  Replace magic-byte SPSC sentinel with `WebizenVM`

In `llm_offload.rs` sentinel thread, replace `bytes[0] == 0x99` with:
```rust
use qualia_core_db::logic::{WebizenVM, WebizenOpcode};
let mut vm = WebizenVM::new();
// compile temporal bound program once at inference start
let program = vec![
    WebizenOpcode::LessThan { vector_id: 3, value: temporal_end as f32 },
    WebizenOpcode::HaltIfFalse,
    WebizenOpcode::GreaterThan { vector_id: 3, value: temporal_start as f32 },
    WebizenOpcode::HaltIfFalse,
];
vm.load_bytecode(&program);
// in the hot loop: vm.execute_constraint(&token_quin)
```
The VM has `LessThan/GreaterThan` opcodes and `extract_float()` — all real.

### 2.5  Wire thermodynamics into the sentinel

When `strictEnergy` flag is set, add a `NativeThermodynamics` check via the `SlgArena`
`execute_vm_frame()` path. The `ThermodynamicSampler::metropolis_step()` validates proposed
energy states inline.

### 2.6  Wire `QTensorEngine::decode_lexicon_bound()` to real logit masking

Once Ollama streaming provides actual logit probabilities (the streaming API includes them
with the `logprobs` parameter), pass them through `decode_lexicon_bound(logits, valid_ids)`
where `valid_ids` come from the live lexicon dictionary. This forces the LLM to only emit
tokens that exist in the `.q42.bidx` index.

---

## Phase 3 — Agreement Protocol UI  (4–5 days)
**The most unique capability in the codebase. No other system has this.**

### 3.1  Expose AgreementDID via Tauri commands

Add to `commands/identity.rs`:
```rust
#[tauri::command]
fn propose_agreement(principal: u64, agents: Vec<u64>, domain: String, threshold: u8)
    -> Result<u64, String>  // returns agreement_id

#[tauri::command]
fn ratify_agreement(agreement_id: u64, signing_key_hex: String)
    -> Result<Vec<[u8;48]>, String>  // returns compiled Quins as bytes

#[tauri::command]
fn get_pending_agreements() -> Vec<serde_json::Value>
```

### 3.2  Write ratified agreements to the `.q42` index

`AgreementDID::compile_to_super_quins()` produces 16 Quins with `EnforceBilateralMicroCommons`
routing lane. Write these via `SuperBlockWriter` + WAL into the user's personal `.q42` file.
The data is now in the graph and queryable via the daemon.

### 3.3  Wire CRDT consent token flow

When a remote peer sends a `q42:issuesConsentToken` Quin over the daemon WebSocket,
call `SuspendedTransactionQueue::apply_consensus_token()`. If M-of-N threshold is met,
the suspended VM resumes. Surface this in the AddressBook page as a consent notification badge.

### 3.4  Wire `wasm_edge.rs` agreement functions into the browser build

The WASM module exports `webizen_propose_agreement()`, `webizen_poll_agreements()`, and
`webizen_sign_agreement()`. Once the WASM build exists (Phase 6), these are callable directly
from the AddressBook page without Tauri IPC.

---

## Phase 4 — Identity & Wallet  (4–5 days)

### 4.1  Real BIP32/BIP44 derivation

Add to `qualia-desktop/Cargo.toml`:
```toml
bitcoin = "0.31"
k256 = { version = "0.13", features = ["ecdh"] }
```

Replace the hex-slice mock in `derive_wallets_from_seed` with proper child key derivation:
- BTC: `m/84'/0'/0'/0/0` (BIP84 native SegWit)
- XEC: `m/44'/145'/0'/0/0`
- ETH: `m/44'/60'/0'/0/0` + keccak256 checksum address
- NYM: `m/44'/118'/0'/0/0` (Cosmos)
- Qualia root: keep ed25519 from `agency.rs`

### 4.2  Real Front Door DID via `agency.rs`

Generate a fresh `ed25519_dalek::SigningKey` per invite. Derive a `did:q42:` identifier
using `identifier.rs::parse_did_q42()`. Sign a minimal DID Document. Discard private key
after handshake. Replace the hardcoded `"did:qualia:frontdoor:88f72a-connect"`.

### 4.3  Real app credential VC via `agency.rs`

`generate_qapp_credential` signs `(qapp_name + timestamp)` with the master identity key
using `agency.rs::sign_agency_root()`. `verify_and_install_qapp` calls `verify_human_agency()`
instead of the prefix-string check.

### 4.4  Real chain balance queries

Add a user-triggered `refresh_balances` command. Query per-chain:
- XEC: `https://chronik.be.cash/xec/script/p2pkh/{hash}/utxos`
- BTC: `https://blockstream.info/api/address/{addr}`
- ETH: Etherscan/Alchemy free tier
- NYM: Nyx Chain RPC (Cosmos-compatible)
- XMR: local `monero-wallet-rpc` (show "daemon required" if absent)

### 4.5  ILP compute cost audit trail

Wire `generate_energy_of_logic_invoice()` into `orchestrate_inference()` so every inference
call generates an invoice. Write the resulting payment as N-Quads to the `.q42` graph
(as documented in `ilp_dispatcher.rs` module docstring). The `ATOMIC_FLOPS_COUNT` telemetry
counter is already incremented by the VM — just wire the chain.

---

## Phase 5 — P2P, Multimodal & GPU Sieve  (5–7 days)

### 5.1  librqbit BitTorrent daemon

```toml
librqbit = "7"
```
Spawn a `librqbit::Session` at app startup. Expose `seed_file(path) -> magnet_uri`,
`fetch_via_torrent(magnet, dest)`, real `fetch_torrent_telemetry()`. Bind the returned
Magnet URI into the Quin's `metadata` slot when indexing an asset.

### 5.2  LLaVA vision extraction via Ollama

Replace `ingest_image_async`'s 3-second sleep with a real POST to `llava` model in Ollama.
Parse response for semantic facets. Write facets via `streaming_import_rdf` writer path.

### 5.3  SPARQL-MM temporal queries via daemon

The daemon POST /query accepts N-Triples patterns compiled by `mini_parser`. Wire the
Asset Library search input to `query_daemon` with temporal bounds as `LessThan/GreaterThan`
constraints in the bytecode.

### 5.4  CRDT P2P sync via daemon WebSocket

When two nodes connect via `/qualia-bridge`, use `MerkleCrdtSynchronizer::diff_jump_tables()`
to compute block-level diffs. Resolve conflicts with `CrdtResolver` (LWW + Lamport clocks).
Both are fully implemented in `sync.rs` and `crdt.rs`.

### 5.5  GPU sieve via `SieveOrchestrator`

`npu_ffi.rs::SieveOrchestrator::execute_sieve()` dispatches WGPU compute shaders to filter
Quins by metadata bitmask. Wire as a pre-filter step in the daemon query path before
`execute_program()`. WGPU is already a dependency on non-WASM targets.

### 5.6  SHACL shapes file for physical constraints

Create `qualia_shapes.ttl` in the Index directory with constraints like:
```turtle
qualia:chemicalPathway a sh:NodeShape ;
  sh:property [ sh:path qualia:energyYield ;
                qualia:thermoMetropolisStep true ] .
```
`shacl_compiler.rs::ShaclCompiler::compile_shape()` already handles
`"qualia:thermoMetropolisStep"` → `SlgOpcode::NativeThermodynamics`. Parse this file at
daemon startup and register the compiled bytecode in the `SlgArena`.

---

## Phase 6 — WASM Edge & Offline Sovereignty  (5–7 days)

### 6.1  Compile qualia-core-db to WASM

```bash
wasm-pack build crates/qualia-core-db --target bundler --features wasm_simd
```

`wasm_bridge.rs::execute_ntriples_query()` is real (calls `mini_parser` +
`webizen_bytecode`). `wasm_edge.rs` exports `FederatedNodeManager`, agreement functions,
and float serialization — all `#[wasm_bindgen]` annotated. The WASM build exists in theory;
it just needs to be built and embedded in the Vite asset pipeline.

### 6.2  Implement `OpfsVfs`

Fill in the two `// TODO` methods in `storage.rs::OpfsVfs` using
`FileSystemSyncAccessHandle`. Must run in a `SharedWorker` (OPFS sync API is Worker-only).
Set up the Worker in the Vite build.

### 6.3  LRU cache for .bidx chunks

```toml
lru = "0.12"   # add to qualia-core-db
```
Wrap `OpfsVfs` in a `CachedVfs<V: VirtualFileSystem>`. When the native daemon is
unreachable, the LRU cache continues serving recent `.q42` page reads for offline inference.

### 6.4  Enable WASM SIMD

`webizen_bytecode.rs::execute_program_simd()` loads each 48-byte Quin into three `v128`
registers (subject+predicate, object+context, metadata+parity). The `wasm_simd` feature
flag is declared in `Cargo.toml`. Pass `--features wasm_simd` to `wasm-pack`.

### 6.5  Zero-copy WebGL

Replace `SpatialPhysics.tsx` hardcoded `Float32Array` vertices with:
```ts
const positions = new Float32Array(wasm.memory.buffer, vertexOffset, vertexCount * 3);
geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
```
`wasm_edge.rs::serialize_float_array()` is the Rust-side of this boundary.

---

## Phase 7 — ZK, Nym & Production  (5–7 days)

### 7.1  ZK-STARK prover

```toml
winterfell = "0.9"
```
Replace simulated STARK loop in `toggle_stark_prover` with real Winterfell execution traces
over SHACL validation runs. Solar-wattage threshold gating already correct.

### 7.2  Real Nym integration

`nym_adapter.rs::route_through_mixnet()` is the right call site. Wire to a Nym client
subprocess or `nym-sdk`. Replace the simulated packet counter in `toggle_nym_relay`.

### 7.3  Gun.eco WebSocket bridge

The daemon has a placeholder `tokio::spawn` for Gun.eco. Wire it to actually connect to a
Gun relay peer and sync Quin hashes as a secondary P2P discovery mechanism alongside librqbit.

### 7.4  `tee_ffi.rs` platform implementations

C-ABI declarations exist for Android KeyStore and Apple Secure Enclave. Implement glue:
- Android: call via `jni_bridge.rs` (already exists)
- Apple: `core-foundation` bindings

### 7.5  Real VRAM detection

Replace `vram_estimated_gb: 16.0`:
- Windows: `nvml-wrapper` (NVIDIA) or DXGI adapter query via `winapi`
- macOS: `IOKit` via `core-foundation`
- Linux: `std::fs::read_to_string("/sys/class/drm/card0/device/mem_info_vram_total")`

### 7.6  CI/CD and release pipeline

1. Run `tauri signer generate` — add private key as `TAURI_PRIVATE_KEY` GitHub secret
2. Re-enable `"active": true` in updater block (currently disabled as a fix)
3. Generate `releases/latest.json` on each tag push to GitHub Pages
4. Code-sign: Windows SmartScreen (EV cert), macOS notarization (Apple Developer ID)
5. Verify `https://mediaprophet.github.io/qualiaDB/benchmark.html` and `/playground/index.html`

---

## Dependency Graph

```
Phase 0 (structural refactor)
    │
    ├──► Phase 1 (data foundation — streaming_import_rdf, daemon live index)
    │         │
    │         ├──► Phase 2 (LLM via orchestrator + real RAG)
    │         │         │
    │         │         ├──► Phase 5 (P2P + multimodal + GPU sieve)
    │         │         └──► Phase 6 (WASM edge + OPFS)
    │         │
    │         └──► Phase 3 (agreement protocol UI)
    │
    ├──► Phase 4 (identity + wallet — independent of data stack)
    │
    └────────────────────────────────────────────► Phase 7 (ZK + production)
```

## Effort Summary

| Phase | Description                                      | Est. Days |
|-------|--------------------------------------------------|-----------|
| 0     | Structural refactor — split main.rs              | 1–2       |
| 1     | Data foundation — real parsing, live daemon      | 3–4       |
| 2     | LLM via orchestrator + real neurosymbolic RAG    | 4–5       |
| 3     | Agreement protocol UI (AgreementDID + CRDT)      | 4–5       |
| 4     | Identity + wallet (BIP32 + chain APIs + ILP)     | 4–5       |
| 5     | P2P + multimodal + SHACL shapes + GPU sieve      | 5–7       |
| 6     | WASM edge + OPFS + SIMD + zero-copy WebGL        | 5–7       |
| 7     | ZK-STARK + Nym + Gun.eco + TEE + CI/CD           | 5–7       |

**Total: ~31–42 working days.**

Phase 0 is the prerequisite that makes all other phases faster to execute and audit.
Phases 1–3 together deliver: real ontology indexing, real LLM inference with neurosymbolic
intercept, real RAG from the graph, and real M:N consent agreements — the core differentiating
loop of the system.
