# Development Guide

Build, test, benchmark, and contribute to QualiaDB / Webizen.

_Branch: `0.0.10-dev` | Last updated: 2026-06-11_

---

## Prerequisites

| Tool | Required for | Notes |
|---|---|---|
| [Rust stable](https://rustup.rs/) | Everything | `rustup update stable` |
| [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) | WASM browser build | |
| [Flutter SDK](https://docs.flutter.dev/get-started/install) ≥ 3.16 | Desktop app | Primary shipped desktop target |
| [flutter_rust_bridge_codegen](https://cjycode.com/flutter_rust_bridge/) | Flutter FRB API changes | Run after editing `crates/qualia-flutter/rust/src/api/` |
| Node.js ≥ 18 | Docs test suite, API explorer | `docs/tests/run-local.ps1` |
| [Tauri CLI v1.x](https://tauri.app/v1/guides/getting-started/prerequisites/) | Legacy desktop only | `qualia-desktop` crate — not in release CI |

---

## Build from Source

### Native CLI (all platforms)

```bash
cargo build --release -p qualia-cli
./target/release/qualia --help
```

### Flutter desktop app (primary shipped desktop target)

```bash
cd crates/qualia-flutter
flutter pub get
flutter run -d windows   # or: -d macos / -d linux

# After editing crates/qualia-flutter/rust/src/api/*.rs:
flutter_rust_bridge_codegen generate
```

### WASM browser module

```bash
cd crates/qualia-core-db
wasm-pack build --target no-modules --out-dir ../../docs/playground
```

For embedding in an external web project, use `--target web` instead.

### Cross-platform CI builds (recommended for releases)

GitHub Actions (`.github/workflows/release.yml`) builds on tag push:

- `qualia-cli` — Windows, macOS (Intel + Apple Silicon), Linux x86_64
- Flutter desktop bundles — `.dmg` (macOS), AppImage + `.deb` (Linux), `.exe` + `.msi` (Windows)

```bash
git tag v0.0.10
git push origin v0.0.10
```

### Cross-compiling the CLI locally (Windows → Linux)

```powershell
cd scripts/cross-linux
.\build-linux.ps1
# Output: target/x86_64-unknown-linux-gnu/release/qualia
```

### Serve the local docs / API explorer

```powershell
.\docs\tests\run-local.ps1 -Serve -Port 8765
# API Explorer: http://localhost:8765/api-explorer/
```

---

## CLI Command Reference

```bash
# ── Ingestion ──────────────────────────────────────────────────────────
qualia ingest data.ttl output.q42
qualia ingest --profile health.qchk data.ttl output.q42   # profile-bound

# ── Inspection & export ────────────────────────────────────────────────
qualia inspect output.q42                 # decode and display Quin fields
qualia dump output.q42                    # stream-dump raw Quins
qualia compress output.q42 output.c.q42  # LZ4 SuperBlock compress
qualia export-solid output.q42 ./solid-pod/   # W3C Solid LDP export

# ── Querying ───────────────────────────────────────────────────────────
qualia query output.q42                  # interactive SPARQL-like query
qualia import                            # import from external source

# ── Daemon ─────────────────────────────────────────────────────────────
qualia daemon start                      # start on http://localhost:4242
qualia daemon stop

# ── Capability profiles ────────────────────────────────────────────────
qualia profile compile profile.jsonld profile.qchk
qualia profile list
qualia profile inspect profile.qchk

# ── Resource catalog (LLMs, ontologies, SPARQL endpoints) ─────────────
qualia resources list llms
qualia resources list ontologies
qualia resources list sparql
qualia resources show <id>
qualia resources download <id>           # streams → GGufSharder → WAL
qualia resources import-ontology <id>   # download + SHACL-validate + ingest

# ── Webizen / identity workflows ──────────────────────────────────────
qualia webizen init
qualia webizen ingest
qualia webizen validate-gitmark
qualia webizen publish-ipfs
qualia webizen seed-webtorrent
qualia webizen dns-frontdoor             # generate did:web + DNS TXT records

# ── Benchmarks ────────────────────────────────────────────────────────
qualia bench --suite full
qualia benchmark --suite full            # alias
```

Full subcommand list: `qualia --help`

---

## Testing

### Run the full test suite

```bash
cargo test -p qualia-core-db
```

The `qualia-core-db` crate contains 539+ test functions covering SPARQL, SHACL, biosciences/biomedical/chemistry engines, SPARQL-Star, temporal graph queries, WAL/DAG linking, and WASM bridge paths.

### Run SPARQL-specific tests

```bash
cargo test -p qualia-core-db sparql
```

### Run the browser test suite

```powershell
.\docs\tests\run-local.ps1 -Serve -Port 8765
# Open http://localhost:8765/tests/ — 271-test suite (WASM/Native/Both modes)
```

### Run Criterion micro-benchmarks

```bash
cargo bench -p qualia-core-db
```

---

## Benchmarking

### Native harness (authoritative)

```bash
qualia bench --suite full
```

- Exercises: Lazy SuperBlock scans (LZ4 40 KB blocks), mmap point queries, two-hop graph traversal, filter queries, and live sysinfo RSS telemetry (WebSocket on `:9090`).
- Output: `docs/llm_benchmark_results.json` (12 categories including rights, escrow, and Nym tests).
- Visualisation: open `docs/benchmark.html` or `docs/benchmark_visualizer.html` for the live block heatmap and dashboard.

### Testing with large datasets

```powershell
# Download reference datasets (2 GB – 12 GB):
./scripts/fetch_massive_datasets.ps1
```

```bash
# Ingest DBpedia:
qualia ingest ./data/mappingbased-objects.ttl.bz2 ./data/dbpedia.q42

# Memory-mapped query:
qualia query ./data/dbpedia.q42
```

### Building the WordNet playground dataset

```bash
bash scripts/fetch_wordnet.sh --subset 100000
# Outputs: docs/playground/wordnet.q42 + .lex + .bidx + .c.q42 + .lex.lz4
```

Rebuild the WASM module after updating the dataset:

```bash
wasm-pack build crates/qualia-core-db --target web \
  --out-dir ../../docs/playground --no-typescript
```

Commit `docs/playground/` artefacts to trigger a GitHub Pages deploy.

---

## Running the Daemon Locally

The native daemon listens on `http://localhost:4242`. Endpoints: `/health`, `/query` (SPARQL), `/chat/publish`, `/chat/pull`, WebTorrent routes.

```bash
cargo run --release -p qualia-cli -- daemon start
```

The Flutter desktop app and browser playground both connect to this endpoint. The UI connection badge turns green when the daemon is reachable.

---

## GPU Inference

In-process LLM inference uses a platform-specific GPU backend selected at startup:

| Platform | Backend | Notes |
|---|---|---|
| Windows x86_64 | DirectML 1.15 | `directml_bridge.rs`; requires D3D12-capable GPU |
| macOS (Apple Silicon) | Accelerate / AMX | `metal_bridge.rs`; `cblas_sgemm` via Accelerate framework |
| Linux / all others | wgpu / Vulkan | `gguf_bridge.rs` + `fused_transformer.wgsl` shader |
| WASM | Mock ring-buffer | GPU path not available in browser; mock path used |

The backend selection is automatic and falls through in priority order: DirectML → Accelerate → wgpu. No configuration required.

Model weights are loaded via `memmap2` zero-copy from a GGUF file on disk. The `LocalLlmAgent` runs a Phase 8 bifurcated autoregressive loop with a mid-generation Webizen Sentinel rollback channel. See `ARCHITECTURE.md §3` for the full inference pipeline description.

---

## SPARQL Development

The SPARQL engine lives in `crates/qualia-core-db/src/sparql_*.rs`. Key modules:

| Module | Purpose |
|---|---|
| `sparql_parser.rs` | SPARQL 1.1 + RDF-Star parser |
| `sparql_ast.rs` | AST types |
| `sparql_planner.rs` | Query planner |
| `sparql_executor.rs` | Executor (joins, filters, aggregates) |
| `sparql_aggregates.rs` | GROUP BY / aggregate functions |
| `sparql_filter.rs` | FILTER expression evaluation |
| `sparql_update.rs` | SPARQL Update (INSERT/DELETE DATA) |
| `sparql_endpoint.rs` | HTTP SPARQL endpoint (port 4242 `/query`) |
| `sparql_did.rs` | DID-authenticated federation |
| `sparql_federated.rs` | SERVICE clause federation |
| `sparql_results.rs` | SPARQL JSON / XML result serialisation |
| `sparql_extensions.rs` | Qualia-specific extension functions |
| `sparql_mm.rs` | Multimedia / modality extensions |
| `sparql_websocket.rs` | WebSocket-based live SPARQL subscriptions |
| `sparql_shacl.rs` | SHACL validation integrated into query |

SPARQL-Star tests: `crates/qualia-core-db/tests/sparql_star_tests.rs`

---

## RDF Parsers

Supported input formats for `qualia ingest`:

| Format | Module | Notes |
|---|---|---|
| Turtle / Turtle-Star | `turtle_star.rs` | Default RDF format |
| N-Triples / N-Triples-Star | `ntriples_star.rs` | |
| N-Quads / N-Quads-Star | `nquads_star.rs` | Named graphs |
| TriG / TriG-Star | `trig_star.rs` | Named graphs + RDF-Star |
| N3 | `n3_star.rs` | N3Logic rules |
| JSON-LD | `json_ld_stream.rs` | Streaming |
| CBOR-LD | `cbor_parser.rs` | Zero-alloc, offline |

---

## Known Build Issues (v0.0.10-dev)

All crates compile cleanly except where noted:

| Crate / module | Status | Notes |
|---|---|---|
| `qualia-core-db` — SPARQL modules | ⚠️ Build errors under resolution | `sparql_executor`, `sparql_endpoint`, `sparql_extensions`, `sparql_mm`, `sparql_websocket` |
| All other crates | ✅ Clean | |

Tracking: [BUILD_ERRORS_TRACKING.md](../../BUILD_ERRORS_TRACKING.md)

---

## AI Agent Orientation

Required reading before modifying any code:

- [`CLAUDE.md`](../../CLAUDE.md) — primary orientation for Claude Code. Covers the LLM inference stack, backend modes, bifurcated compute, Webizen VM gates, daemon port, and core invariants.
- [`AGENTS.md`](../../AGENTS.md) — multi-agent coordination. Covers immovable rules, Quin bit layout, known inconsistencies, and per-module guidance.

These supersede the older `AI_INSTRUCTIONS.md`.

---

## Releases & Versioning

- **Current branch:** `0.0.10-dev`
- **Release config:** `release.toml` (cargo-release)
- **Release notes:** [CHANGELOG.md](../../CHANGELOG.md)
- **CI:** `.github/workflows/release.yml` — builds on tag push (Windows, macOS, Linux)

To cut a release:

```bash
git tag v0.0.10
git push origin v0.0.10
```

ADRs (Architectural Decision Records): [`docs/manuals/adr/`](adr/)
