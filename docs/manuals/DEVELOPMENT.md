# Development Guide

Build, test, benchmark, and contribute to QualiaDB / Webizen.

_Branch: `0.0.29` | Last updated: 2026-08-02_

---

## Prerequisites

| Tool | Required for | Notes |
|---|---|---|
| [Rust stable](https://rustup.rs/) | Everything | `rustup update stable` |
| [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) | WASM browser build | |
| [Dioxus CLI](https://dioxuslabs.com/learn/0.5/getting_started) | Webizen Studio | Primary shipped desktop target (`cargo binstall dioxus-cli`) |
| Node.js ≥ 18 | Docs test suite, API explorer | `docs/tests/run-local.ps1` |
| [Tauri CLI v1.x](https://tauri.app/v1/guides/getting-started/prerequisites/) | Legacy desktop only | `qualia-desktop` crate — not in release CI |

---

## Build from Source

### Native CLI (all platforms)

```bash
cargo build --release -p qualia-cli
./target/release/qualia --help
```

### Webizen Studio desktop app (primary shipped desktop target)

```bash
cd crates/webizen-studio
dx build --release

# Or to run in development mode:
dx serve --platform desktop
```

### WASM browser module

**Main portal + LLM engine** (`docs/pkg/qualia/` — spatial demo, U3 acoustic, **and the WASM
WebGPU LLM**). Use the script — it sets the SIMD/8 MB-stack/4 GB-memory RUSTFLAGS the LLM call
tree needs and publishes the canonical `qualia.*` names:

```powershell
./scripts/package-qualia-wasm.ps1
# = wasm-pack build … --no-default-features --features portal,wasm-llm,wasm-logic,wasm-scientific
```

**Full playground** (the 6 science evaluators + API explorer + the LLM/q42 exports). This is the
shared `docs/playground/qualia_core_db.*` artifact imported by `science-playground.html`,
`benchmark.html`, `llmdemo`, etc. — keep it a strict **superset** (no science export dropped):

```powershell
$env:RUSTFLAGS = "-C target-feature=+simd128 -C link-arg=-zstack-size=8388608 -C link-arg=--max-memory=4294967296"
wasm-pack build crates/qualia-core-db --target web --out-dir pkg-playground --release -- `
  --no-default-features --features portal,wasm-llm,wasm-logic,wasm-scientific,wasm-playground
cp crates/qualia-core-db/pkg-playground/qualia_core_db.{js,d.ts} docs/playground/
cp crates/qualia-core-db/pkg-playground/qualia_core_db_bg.wasm{,.d.ts} docs/playground/
```

#### Browser LLM (P64 AOT)

The browser compiles GGUF to canonical P64 v3 through
`compileGgufToP64`, caches it with `loadOrCompileP64`, and boots the WebGPU
engine from the validated container. GGUF remains a direct-load fallback.
Historical Q42-named exports are retained as P64 aliases.

The WASM target and full playground bundle build successfully. The remaining
real-model WebGPU release checks are tracked in
[`p64-q42-inference-pipeline.md`](p64-q42-inference-pipeline.md).

Key sources are `gguf_bridge/`, `q42/p64_weight.rs`,
`shaders/fused_attention.wgsl`, `shaders/fused_transformer.wgsl`, and
`shaders/fused_ffn.wgsl`.

**Headless verification harnesses** (Playwright + Chrome WebGPU):

```bash
node agent-tools/wasm-mc2-test.mjs          # decode coherence + tok/s (wasm-llm-test.html)
WASM_MODEL=models/smollm2-360m-instruct-q8_0.gguf node agent-tools/wasm-mc2-test.mjs  # quant check
node agent-tools/llmdemo-test.mjs           # intended end-to-end gate: GGUF→P64→OPFS→generate
node agent-tools/gguf-types.mjs <model.gguf>  # dump per-tensor quant types
```

Manual: [`qualia-wasm-portal.md`](qualia-wasm-portal.md). Verify: `node docs/tests/phenomenal-verify.mjs`.


### Feature Flags

The 10D tensor system and related components can be enabled via Cargo feature flags:

```bash
# Enable 10D tensor coordinate system
cargo build --features tensor-10d

# Enable GPU acceleration (CUDA/Metal/Vulkan)
cargo build --features tensor-gpu

# Enable NPU acceleration (Neural Engine)
cargo build --features tensor-npu

# Enable all tensor features
cargo build --features tensor-10d,tensor-gpu,tensor-npu

# Enable sanctuary cryptography
cargo build --features sanctuary-crypto
```

**Feature Descriptions:**
- `tensor-10d`: Enables the 10D tensor coordinate system [q, v, w, x, y, z, t, α, μ, σ] and all tensor operations
- `tensor-gpu`: Enables GPU acceleration for tensor operations via CUDA, Metal, or Vulkan
- `tensor-npu`: Enables NPU acceleration for tensor operations (Apple Neural Engine, etc.)
- `sanctuary-crypto`: Enables sanctuary lane cryptography with PBKDF2 key derivation and AEAD ciphers


### Cross-platform CI builds (recommended for releases)

GitHub Actions (`.github/workflows/release.yml`) builds on tag push:

- `qualia-cli` — Windows, macOS (Intel + Apple Silicon), Linux x86_64
- Flutter desktop bundles — `.dmg` (macOS), AppImage + `.deb` (Linux), `.exe` + `.msi` (Windows)

```bash
git tag v0.0.29
git push origin v0.0.29
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

Model weights are loaded through a resident GGUF or explicitly mounted P64 mmap. The `LocalLlmAgent` runs a Phase 8 bifurcated autoregressive loop with a mid-generation Webizen Sentinel rollback channel. See [`p64-q42-inference-pipeline.md`](p64-q42-inference-pipeline.md) for the full inference pipeline.

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

## Known Build Issues (v0.0.29)

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

- **Current branch:** `0.0.29`
- **Release config:** `release.toml` (cargo-release)
- **Release notes:** [CHANGELOG.md](../../CHANGELOG.md)
- **CI:** `.github/workflows/release.yml` — builds on tag push (Windows, macOS, Linux)

To cut a release:

```bash
git tag v0.0.29
git push origin v0.0.29
```

ADRs (Architectural Decision Records): [`docs/manuals/adr/`](adr/)
