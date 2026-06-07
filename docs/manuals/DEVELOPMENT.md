# Development Guide

Build, test, benchmark, and contribute to Qualia-DB.
_Branch: `0.0.8-dev` | Last updated: 2026-06-07_

---

## Prerequisites

- [Rust stable](https://rustup.rs/) (`rustup update stable`)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) (for WASM browser builds)
- [Flutter SDK](https://docs.flutter.dev/get-started/install) ≥ 3.16 (primary desktop app — `crates/qualia-flutter/`)
- [Tauri CLI v1.x](https://tauri.app/v1/guides/getting-started/prerequisites/) (legacy `qualia-desktop` — not in release CI)
- Node.js ≥ 18 (for browser benchmark runner and docs test suite)

---

## Build from Source

```bash
# Core native engine + CLI (current platform)
cargo build --release -p qualia-cli

# WebWorker WASM Bridge (for browser playground)
cd crates/qualia-core-db
wasm-pack build --target no-modules --out-dir ../../docs/playground

# Desktop Frontend (Vite/React — must be built before the Tauri shell)
cd crates/qualia-client
npm install
npm run build      # outputs to dist/

# Desktop Shell (Tauri — picks up the qualia-client dist/ automatically)
cd crates/qualia-desktop
cargo tauri build   # or `cargo build --release` for the Rust side only
```

### Cross-platform binaries (recommended)

GitHub Actions (`.github/workflows/release.yml`) automatically builds on tag push:

- `qualia-cli` for Windows, macOS (Intel + Apple Silicon), Linux (x86_64)
- Full desktop bundles: `.dmg` (macOS), AppImage/deb (Linux), `.exe`/`.msi` (Windows)

To trigger official builds:
```bash
git commit -m "Bump to 0.0.6"
git tag v0.0.6
git push origin v0.0.6
```

Local cross-compilation of full Tauri desktop apps from Windows is not straightforward (Tauri bundlers are platform-specific). Use CI for macOS/Linux desktop releases.

### Flutter desktop (primary shipped shell, v0.0.8)

```bash
cd crates/qualia-flutter
flutter pub get
flutter run -d windows   # or macos / linux

# After editing rust/src/api/*.rs:
flutter_rust_bridge_codegen generate
```

Serve local docs (API Explorer at `http://localhost:8765/api-explorer/`):

```powershell
.\docs\tests\run-local.ps1 -Serve -Port 8765
```

### Cross-compiling the CLI locally (Windows → Linux)

A helper using the LLVM tools on this machine is in `scripts/cross-linux/`:

```powershell
cd scripts/cross-linux
.\build-linux.ps1
```

Output: `target/x86_64-unknown-linux-gnu/release/qualia-cli`

---

## The `qualia-cli` Command Reference

```bash
# LLM / Agent Benchmark Harness
cargo run --release -p qualia-cli -- bench --suite full
# (alias: benchmark --suite full)

# Ingest real semantic data into native .q42 (Rio streaming + LZ4 SuperBlocks)
qualia-cli ingest --input ./data/something.ttl --output ./data/out.q42

# Ingest a CogAI Chunks file (W3C CG ACT-R format)
qualia-cli ingest --input ./data/knowledge.chk --output ./data/out.q42

# Ingest with a bound Capability Profile (QCHK binary — different from CogAI .chk)
qualia-cli ingest --input ./data/something.ttl --output ./data/out.q42 --profile health.chk

# Compress a .q42 file (LZ4 block-stream)
qualia-cli compress --input ./data/out.q42 --output ./data/out.c.q42

# Memory-mapped / lazy query against huge ledgers (microseconds, low RAM)
qualia-cli query ./data/out.q42 123456

# Inspect raw Super-Quins
qualia-cli inspect ./data/out.q42

# Start the full daemon
qualia-cli daemon --dev --workers 8 --compute-swarm

# Capability Profiles
qualia-cli profile compile health.jsonld --out health.chk
qualia-cli profile list
qualia-cli profile inspect health.chk

# Resource Catalog (LLMs, ontologies, SPARQL endpoints)
qualia-cli resources list llms
qualia-cli resources list ontologies
qualia-cli resources show <id>
qualia-cli resources download <llm-id>
qualia-cli resources import-ontology <ont-id>

# Webizen / did:git workflows
qualia-cli webizen init ./my-agency
qualia-cli webizen ingest https://example.org/ontology.n3 ./my-agency

# Export to W3C Solid LDP (for backup / interop)
qualia-cli export-solid --input ./data/out.q42 --output ./solid-pod/

# SHACL extensions
qualia-cli shacl --list-extensions

# Detailed dev benchmarks (require a .q42)
qualia-cli benchmark-action rss-scan ./data/out.q42 10
```

Full subcommand list: `qualia-cli --help`

---

## Benchmarking

### Native harness (authoritative)

```bash
cargo run --release -p qualia-cli -- bench --suite full
```

- Drives Lazy SuperBlock scans (LZ4 compressed 40 KB blocks), WebRTC-mocked P2P streaming, and live sysinfo RSS + hot-block telemetry (WebSocket on `:9090`).
- Produces `docs/llm_benchmark_results.json` (12 categories including rights/escrow/nym tests).
- Open `docs/benchmark.html` or `docs/benchmark_visualizer.html` alongside for the live block heatmap + dashboard.

### Criterion micro-benchmarks

```bash
cargo bench -p qualia-core-db
```

### Browser fallback

```bash
node scripts/llm_bench_runner.js --suite full
```

### Testing with massive datasets

1. **Download datasets (2 GB – 12 GB range):**
   ```powershell
   ./scripts/fetch_massive_datasets.ps1
   ```

2. **Convert to native `.q42`:**
   ```bash
   qualia-cli ingest --input ./data/mappingbased-objects.ttl.bz2 --output ./data/dbpedia.q42
   ```

3. **Execute a memory-mapped query:**
   ```bash
   qualia-cli query ./data/dbpedia.q42 --subject 123456
   ```

### Building the WordNet demo dataset

```bash
bash scripts/fetch_wordnet.sh --subset 100000
```

Outputs `docs/playground/wordnet.q42`, `wordnet.q42.lex`, `wordnet.q42.bidx`, `wordnet.c.q42`, and `wordnet.q42.lex.lz4`. Then rebuild the WASM module:

```bash
wasm-pack build crates/qualia-core-db --target web --out-dir ../../docs/playground --no-typescript
```

Then commit `docs/playground/` artefacts for the GitHub Pages deploy.

---

## Running the Daemon Locally

The native daemon listens on `http://localhost:4242`. Start it with:

```bash
cargo run --release -p qualia-cli -- daemon --dev
```

The desktop app and browser playground both poll this endpoint; the UI connection badge turns green when it is reachable.

---

## AI Agent Orientation

Agent orientation files (required reading before writing any code):

- `CLAUDE.md` — Primary orientation for Claude Code sessions. Covers the LLM inference stack, backend modes, bifurcated compute, the Webizen VM gates, daemon port, and core invariants.
- `AGENTS.md` — Multi-agent coordination layer. Covers the immovable rules, Quin bit layout, known inconsistencies, and per-module guidance.

These files supersede the older `AI_INSTRUCTIONS.md`. Both `CLAUDE.md` and `AGENTS.md` are checked in and kept current.

---

## Releases & Versioning

- Current: Core/Desktop/CLI **0.0.8-dev**
- Release notes: see `CHANGELOG.md` and `docs/manuals/RELEASE_NOTES_v0.0.4.md`
- Release config: `release.toml` (cargo-release)
- Next milestone (Phase 7): WASM profile loading, ZK-STARK, Nym integration, TEE, CI/CD signing
