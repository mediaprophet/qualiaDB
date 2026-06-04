# Qualia-DB v0.0.3-dev Release Notes

## Highlights
- **Dual-Mode Benchmarking Suite fully operational**: `cargo run --release -p qualia-cli -- bench --suite full` (and `benchmark` alias) now parses and executes correctly. Invokes real `lazy_superblock_query` (LZ4-compressed 40,960-byte SuperBlocks), WebRTC P2P remote block simulation, sysinfo RSS telemetry, and writes `llm_benchmark_results.json`. Spawns WS server at 127.0.0.1:9090 for live visualizers.
- Comprehensive documentation, site, playground, AI_INSTRUCTIONS.md, and benchmark pages refreshed to showcase Epics 16–24 capabilities.

## New / Enhanced Capabilities
- **SHACL-to-Webizen Compiler** (`shacl_compiler.rs`, integrated in WebizenCompiler): Compile shapes/constraints directly to deterministic Webizen bytecodes for zero-alloc structural validation.
- **Modality Bridges** (`crates/qualia-core-db/src/modalities/`): spatio_temporal, probabilistic, diffusion, dl (Description Logic), asp, linear — normalized into the Webizen registry + lexicon tokenization.
- **Defeasible Logic & Omnimodal** (epic-20/21): CheckDefeaters, multi-surface syntax (N3 + SHACL + defeasible) feeding one VM. Omnimodal parsing + registry.
- **Lazy SuperBlocks + WebRTC Telemetry** (epic-23): Header-only scans, O(1) seeks, partial decompress, local vs remote (P2P) hot blocks. Full TelemetryHook + live dashboard support.
- **High-Density LZ4 SuperBlock Compression** (epic-22): 40KB blocks (850 Quins) with strong compression for massive datasets under 512MB floor.
- **Ingestion**: Rio streaming parser (true semantic RDF), multi-threaded paths, CBOR-LD + LZ4 output. `qualia-cli import`.
- **CLI Surface**: Complete set (inspect, dump, daemon --workers N --compute-swarm, webizen {init,ingest,...}, export-solid, bench/benchmark, import, query). Telemetry server module.
- **Query/MMap**: `lazy_superblock_query` + legacy `mmap_query_subject` for 10s-of-GB files.
- Criterion benches + constrained harness scripts updated.

## Version Bumps
- qualia-core-db: 0.0.3
- qualia-desktop: 0.0.3 (tauri.conf 0.0.3)
- qualia-cli: 0.1.1
- Android gradle: 0.0.3-dev
- releases/latest.json: 0.0.3-dev with full notes.

## Documentation & Showcase
- AI_INSTRUCTIONS.md: New sections 10-14 covering native bench (preferred), SHACL/modality, lazy+telemetry, ingestion, full CLI inventory.
- README.md, index.html, developer-guide.md, benchmark*.html, playground/*, etc.: Prominent calls to new harness, epics, CLI commands, visualizers.
- llm_benchmark_results.json refreshed + annotated.

## How to Try the New Harness
```powershell
cargo run --release -p qualia-cli -- bench --suite full
# Then open benchmark_visualizer.html (auto-connects to WS) or inspect the emitted JSON.
```

See git log for individual epic commits (c6ad834 ...).

This release focuses on making the "Dual-Mode" promise real for AI agents and power users while expanding the logic surface (SHACL, defeasible, modalities) without compromising the 512MB zero-alloc contract.
