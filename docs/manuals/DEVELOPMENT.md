# Development Guide

Build, test, benchmark, and contribute to Qualia-DB.

---

## Prerequisites

- [Rust stable](https://rustup.rs/) (`rustup update stable`)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) (for WASM browser builds)
- [Tauri CLI v1.x](https://tauri.app/v1/guides/getting-started/prerequisites/) (for the desktop app)
- Node.js ≥ 18 (for the browser benchmark runner)

---

## Build from Source

```bash
# Core native engine + CLI (current platform)
cargo build --release -p qualia-cli

# WebWorker WASM Bridge (for browser playground)
cd crates/qualia-core-db
wasm-pack build --target no-modules --out-dir ../../docs/playground

# Desktop Terminal (Tauri, current platform)
cd crates/qualia-desktop
cargo tauri build   # or `cargo build --release` for the Rust side only
```

### Cross-platform binaries (recommended)

GitHub Actions (`.github/workflows/release.yml`) automatically builds on tag push:

- `qualia-cli` for Windows, macOS (Intel + Apple Silicon), Linux (x86_64)
- Full desktop bundles: `.dmg` (macOS), AppImage/deb (Linux), `.exe`/`.msi` (Windows)
- Android APK

To trigger official macOS and Linux builds:
```bash
git commit -m "Bump to 0.0.5"
git tag v0.0.5
git push origin v0.0.5
```

Local cross-compilation of full Tauri desktop apps from Windows is not straightforward (Tauri bundlers are platform-specific). Use CI for macOS/Linux desktop releases.

### Cross-compiling the CLI locally (Windows → Linux)

A helper using the LLVM tools on this machine is in `scripts/cross-linux/`:

```powershell
cd scripts/cross-linux
.\build-linux.ps1
```

Output: `target/x86_64-unknown-linux-gnu/release/qualia-cli`

See `scripts/cross-linux/README.md` for details (renames `clang` to the name the build system expects for the `linux-gnu` target). For `aarch64` a matching aarch64-linux `clang` is required. For macOS targets from Windows, use a full osx-cross setup or let CI do it.

---

## The `qualia-cli` Command Reference

```bash
# LLM / Agent Benchmark Harness
cargo run --release -p qualia-cli -- bench --suite full
# (alias: benchmark --suite full)

# Ingest real semantic data into native .q42 (Rio streaming + LZ4 SuperBlocks)
qualia-cli ingest --input ./data/something.ttl --output ./data/out.q42

# Compress a .q42 file (LZ4 block-stream)
qualia-cli compress --input ./data/out.q42 --output ./data/out.c.q42

# Memory-mapped / lazy query against huge ledgers (microseconds, low RAM)
qualia-cli query ./data/out.q42 123456

# Inspect raw Super-Quins
qualia-cli inspect ./data/out.q42

# Start the full daemon with fractal sharding + swarm compute
qualia-cli daemon --dev --workers 8 --compute-swarm

# Webizen / did:git workflows
qualia-cli webizen init ./my-agency
qualia-cli webizen ingest https://example.org/ontology.n3 ./my-agency

# Export to W3C Solid LDP (for backup / interop)
qualia-cli export-solid --input ./data/out.q42 --output ./solid-pod/

# Generate DNS Frontdoor records (HCAI Discovery)
qualia-cli webizen dns-frontdoor qualia.org ./my-agency

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

- Drives Lazy SuperBlock scans (LZ4 compressed 40KB blocks), WebRTC-mocked P2P streaming, and live sysinfo RSS + hot-block telemetry (WebSocket on `:9090`).
- Produces `docs/llm_benchmark_results.json` (12 categories including rights/escrow/nym tests: Obligation Escrow, Provenance, Multi-Nym Partitioning).
- Open `docs/benchmark.html` or `docs/benchmark_visualizer.html` alongside for the live block heatmap + dashboard.

### Criterion micro-benchmarks

```bash
cargo bench -p qualia-core-db
```

Runs comparison benchmarks against Oxigraph/SurrealDB-class proxies.

### Profiling with `dhat-rs`

To ensure zero-allocation guarantees are met, the Solid Bridge module can be profiled with `dhat-rs` enabled:
```bash
cargo run --features dhat-heap -p qualia-solid-bridge
```
This will dump a `dhat-heap.json` file which can be viewed in the [DHAT Viewer](https://nnethercote.github.io/dh_view/dh_view.html) to strictly enforce the Allocation Firewall.

### Browser fallback

```bash
node scripts/llm_bench_runner.js --suite full
```

### Testing with massive datasets

To prove zero-allocation architecture and microsecond `mmap` read speeds:

1. **Download datasets (2GB – 12GB range):**
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
   The 50GB binary is memory-mapped; Quins are fetched in microseconds without heap allocation.

### Building the WordNet demo dataset

```bash
bash scripts/fetch_wordnet.sh --subset 100000
```

Outputs `docs/playground/wordnet.q42`, `wordnet.q42.lex`, `wordnet.q42.bidx`, `wordnet.c.q42`, and `wordnet.q42.lex.lz4`. Then rebuild the WASM module:

```bash
wasm-pack build crates/qualia-core-db --target web --out-dir ../../docs/playground --no-typescript
```

Then commit `docs/playground/` artefacts for the GitHub Pages deploy.

See also [AI_INSTRUCTIONS.md](../../AI_INSTRUCTIONS.md) §9 for agent-specific guidance.

---

## Running the Daemon Locally

The native daemon listens on `http://localhost:4242`. Start it with:

```bash
cargo run --release -p qualia-cli -- daemon --dev
```

The desktop app and browser playground both poll this endpoint; the UI connection badge turns green when it is reachable.

---

## Releases & Versioning

- Current: Core/Desktop/CLI **0.0.5**
- Next planned: 0.0.6 (WebRTC multi-peer, full sparse queries)
- Release notes: [RELEASE_NOTES_v0.0.5.md](RELEASE_NOTES_v0.0.5.md) (coming soon)
- Release config: `release.toml` (cargo-release)
