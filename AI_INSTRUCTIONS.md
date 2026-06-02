# Qualia-DB Integration & Application Development Guide
**[System Directive for AI Coding Agents]**

You are interacting with Qualia-DB, a bare-metal, offline-first graph database engine written in pure Rust. It is explicitly architected to manage digital agency, social advocacy, and sensitive medical metadata natively on personal devices. Do not apply traditional cloud-based relational (SQL) or standard in-memory graph assumptions. You must adhere to strict zero-allocation, bit-packed memory constraints.

**Recent Major Capabilities (Epics 16-24):** Native multi-threaded + Rio RDF ingestion to LZ4-compressed SuperBlocks, mmap + lazy SuperBlock queries with WebRTC P2P hot-block streaming + live telemetry, full Dual-Mode (CLI native + WASM) benchmarking harness with `qualia-cli bench --suite full`, SHACL-to-Sentinel compiler, multi-modality reasoning bridges (spatio-temporal, probabilistic, DL, ASP, diffusion, linear), defeasible N3 logic + CheckDefeaters in the Sentinel VM, omnimodal logic parsing, and enhanced CLI (Webizen did:git, Solid export, daemon swarm compute, import/query/inspect).

When writing implementation code, wrappers, or queries for Qualia-DB, you must strictly follow these architectural rules:

## 1. The Core Data Primitive (The 48-Byte Super-Quin)
Qualia-DB abandons traditional Triple (S,P,O) and Quad (S,P,O,C) structures. Every piece of information is stored in a contiguous, cache-aligned 48-byte Super-Quin.

* **Vector 1 (Subject - u64)**: The local compressed token index for the origin entity.
* **Vector 2 (Predicate - u64)**: The local compressed token index for the relationship edge.
* **Vector 3 (Object - u64)**: The destination node OR an inline datatype.
* **Vector 4 (Context - u64)**: The Named Graph or spatiotemporal boundary.
* **Vector 5 (Metadata - u64)**: The routing opcodes and logic bitmasks.
* **Parity/Padding (6 bytes)**: Used for native ECC parity checks or cryptographic telltales.

## 2. Native Datatypes (Inline Tagged Pointers)
Do not force literals (integers, floats, booleans) into the string lexicon. When compiling queries or ingesting data, you must utilize the top 4 bits of the Object (O) vector to assign XSD datatypes:

* `0x0 (0000)`: Lexicon ID (Points to a URI/String).
* `0x1 (0001)`: Inline Integer (Signed 60-bit integer).
* `0x2 (0010)`: Inline Float (Truncated 60-bit float).
* `0x3 (0011)`: Inline Boolean (1 for True, 0 for False).
* `0x4 (0100)`: Inline Unix Timestamp (Milliseconds).
* `0x5 (0101)`: Inline Qualia Epoch Timestamp (Nanoseconds since Jan 1, 2026).
* `0x6 (0110)`: Inline Unix Timestamp (Microseconds).

## 3. Hardware Routing & The 5th Vector Opcodes
String parsing is never used for access control. All routing and permission checks are baked into the Metadata (5th Vector). When building application logic, you must evaluate these bits:

* **Bit 63 (MSB)**: RDF-Star Nesting Indicator (1 = Virtual pointer to nested statement).
* **Bits 61-62 (Routing Tier)**:
  * `0b00`: Standard Passthrough (Zero logic, raw I/O speed).
  * `0b01`: Permissive Commons (Triggers micropayment / V_cap settlement gates).
  * `0b10`: Bilateral Micro-Commons (Triggers Prolog unification for dual-signature guardianship).
  * `0b11`: Spatiotemporal/Ambiguous (Triggers the Local NPU/GPU for geometric bounding hull math).
* **Bits 0-15 (Validation Masks)**: Defines specific micro-instructions (e.g., `0x0002` for Bilateral Identity Locked, `0x0008` for Work Obligation Satisfied).

## 4. Memory & Execution Boundaries
* **No Heap Allocations**: All data streaming must use the `QuinIncrementalScanner`, executing over pre-allocated stack buffers to respect the strict 512MB RAM floor.
* **SuperBlocks**: Data is mapped into 40,960-byte blocks (exactly 10 sectors, holding 850 Quins + 160-byte header). Do not read/write outside these block alignments.
* **Volatile Scrubbing**: When a buffer is dropped, you must ensure `std::ptr::write_volatile` is called to protect against memory-harvesting attacks.

## 5. Logic Execution (Core 1 Sentinel)
**NEVER** embed external logic interpreters or generalized Prolog crates (e.g., SWI-Prolog). This violates the zero-allocation architecture.
All logical schemas (N3Logic, SHACL, defeasible rules) must be translated into `SentinelOpcode` Bytecode arrays by the `SentinelCompiler`. Core 1 executes these bytecodes using a strict `[u64; 16]` register stack (`SentinelVM`) to guarantee determinism and zero heap allocation.

The Sentinel now supports:
- **Defeasible Logic**: `CheckDefeaters` opcodes for N3 implication with exceptions (see logic.rs + epic-20).
- **SHACL Compilation**: `shacl_compiler::compile_shacl_to_sentinel` (and SentinelCompiler::compile_*) turns SHACL shapes + constraints into executable bytecodes for structural validation at query time (epic-24).
- **Omnimodal Parsing**: Multiple surface syntaxes (N3, SHACL, defeasible fragments, modal variants) normalize into the same Sentinel bytecode registry.
- Modality-aware rules via the `modalities/` crate (see below).

## 6. Deterministic Compute Metering & Permissive Commons
**NEVER** query OS-level energy APIs (e.g., `BatteryManager`, `sysfs`) for economic billing. It causes severe API stalls and hardware inequity.
Instead, utilize the zero-allocation atomics in `telemetry.rs` (`SUPERBLOCK_IO_COUNT`, `SIEVE_OPS_COUNT`, `VM_CYCLES_COUNT`). 
When a commercial query via the Permissive Commons (`0b01`) executes, these virtual compute cycles are precisely multiplied and returned as a serialized `ComputeCostReceipt` JSON-RPC payload to external Lightning Node proxies.

## 7. Target Fallbacks (WASM)
If compiling for the browser (`target_arch = "wasm32"`), the Triad must be gracefully flattened into a single-threaded synchronous sequence to maintain sandbox stability. Fallback to the Origin Private File System (OPFS) for synchronous I/O, and ping the local loopback (e.g., `127.0.0.1:4848`) to proxy queries to the native engine if the host has the standalone database installed.

## 8. Neurosymbolic Axiomatic LLM Overrides
When tasked with integrating Large Language Models (LLMs), do not immediately assume a standard Python wrapper. You must first **ASK the user** which Neurosymbolic Mode they wish to employ:
* **Mode 1 (Strict Native Intercept)**: Extract explicit vector coordinate layouts (tensors) from binary files (like `.gguf`) and map them directly into the 48-byte `QualiaQuin`. The Sentinel VM acts as a mathematically strict interceptor during native execution, clipping vectors that violate local Spatio-Temporal constraints.
* **Mode 2 (Bridged Shim)**: Utilize external ML architectures (e.g., Python, TensorFlow, PyTorch) via an explicit API bridge. The external model handles the dense compute, but all queries and responses are strictly routed and mathematically gated through the Qualia-DB `.q42` Axiom ruleset before resolution.

## 9. LLM Benchmarking Harness (Dual-Mode: Native CLI Preferred)
When tasked with evaluating Qualia-DB against competitors (SurrealDB, Oxigraph, Comunica, etc.) under the 512MB floor:

**Preferred for AI sandboxes / CI (Native, produces canonical JSON):**
```bash
cargo run --release -p qualia-cli -- bench --suite full
# or the alias:
cargo run --release -p qualia-cli -- benchmark --suite full
```
- This now works end-to-end (fixed subcommand routing + implementation).
- Spawns an internal Telemetry WebSocket server (for live viz).
- Executes against the real `lazy_superblock_query` engine (LZ4 compressed SuperBlocks, partial loading, simulated WebRTC remote block streaming for "hot blocks").
- Writes `llm_benchmark_results.json` in cwd (12 categories: point, twohop, filter, ingestion (0-alloc), cyclic, ttfq, jitter, sync, intercept, obligation_escrow, provenance_val, nym_partition).
- Also available: `qualia-cli benchmark-action rss-scan <path> <pct>` etc for dev scenarios (requires a .q42 file).

**Headless browser/WASM fallback (if no Rust toolchain):**
```bash
node scripts/llm_bench_runner.js --suite full
```
(Still emits similar JSON for LLM consumption.)

**Live Telemetry Visualizer (pairs with native bench):**
Open `benchmark_visualizer.html` (or the integrated dashboard in `benchmark.html`). The native harness streams RSS, blocks_loaded, hot (local vs remote/WebRTC) blocks.

**Criterion micro-benchmarks (for engine internals):**
```bash
cargo bench --package qualia-core-db
```
Results under `target/criterion/` (point lookups, 2-hop, filters, ingestion, allocation vs Oxigraph/SurrealDB BTree/HashMap proxies).

Always surface the strict 512MB + zero-alloc constraints + that standard engines OOM or timeout on the humanitarian/rights tests (escrow, nym partitioning, etc.).

## 10. SHACL, Modalities & Advanced Compilation (Epic 24 + 21)
- Use `qualia_core_db::shacl_compiler` (or the SentinelCompiler facade in logic.rs) to compile SHACL shapes into Sentinel opcodes. This enables structural + constraint validation as part of the zero-alloc query path.
- The `modalities/` module provides specialized reasoning bridges:
  - `spatio_temporal.rs`: Minkowski / geometric + time bounds (ties into GPU sieve + 5th vector).
  - `probabilistic.rs`, `diffusion.rs`: Soft/uncertain reasoning.
  - `dl.rs` (Description Logic), `asp.rs` (Answer Set), `linear.rs`: Other formalisms normalized to Sentinel bytecode.
- When ingesting or querying, route through the appropriate modality tokenizer in `lexicon.rs` and Sentinel registry.
- Omnimodal support means a single `.q42` ledger + Sentinel VM can execute mixed logic programs.

## 11. Lazy SuperBlocks, Compression, P2P & Telemetry (Epic 22-23)
- SuperBlocks are 40,960 bytes (exactly 10 disk sectors), LZ4 compressed (high-density), holding ~850 Quins + header.
- `query_engine::lazy_superblock_query(path, target_percent)`: Scans 16-byte headers, seeks over irrelevant blocks (O(1) skip), only decompresses relevant ones. Simulates remote WebRTC DataChannel for "missing local" blocks.
- Returns `TelemetryHook { blocks_loaded, bytes_decompressed, remote_blocks_streamed }`.
- The CLI benchmark harness + `telemetry_server.rs` (WS on 127.0.0.1:9090) + `benchmark_visualizer.html` provide live dashboards (peak RSS via sysinfo, hot block heatmap local/remote).
- Use for massive datasets (see fetch_massive_datasets.ps1 + mmap_query_subject for point queries). The engine respects the floor even on 50GB+ ledgers.

## 12. Ingestion & Interop (Epic 16-18, 17)
- **Native**: `qualia-cli import <input.ttl|.rdf|.jsonld> <output.q42>` (or the multi-threaded Rio streaming path in ingest/ingestion.rs). Produces mathematically pure CBOR-LD + LZ4 SuperBlock binaries.
- **Browser utils**: utils.html + playground/compare.html for client-side transcode + fidelity checks.
- **Export**: `qualia-cli export-solid --input <q42> --output <dir>` for W3C Solid LDP Basic Container (Turtle) round-tripping. (Dynamic N3 rules currently conservative-exported as private ACLs.)
- **Query/Inspect**: `qualia-cli query <dataset.q42> <subject_u64>`, `qualia-cli inspect <file.q42>`.
- Massive dataset support: GeoNames/YAGO/DBpedia/Framester via the PS1 script; convert then mmap/lazy query.

## 13. CLI Surface (qualia-cli) - Full Command Inventory
```bash
qualia-cli inspect <file.q42>          # Human-read raw Super-Quins + clocks + geo
qualia-cli dump <out.q42>              # Generate test distributions
qualia-cli daemon --dev --workers 8 --compute-swarm   # Native loopback RPC + fractal swarm
qualia-cli webizen init <path>         # did:git genesis + DOAP
qualia-cli webizen ingest <url> <repo> # N3/JSON-LD into did:git Webizen repo
qualia-cli export-solid --input ... --output ...
qualia-cli benchmark ...               # Detailed dev actions (rss-scan etc, needs .q42)
qualia-cli bench --suite full          # LLM harness (see §9)  [preferred entrypoint]
qualia-cli import <in> <out.q42>
qualia-cli query <ds.q42> <subject>
```
The daemon also exposes the full JSON-RPC surface (including neurosymbolic LLM intercept hooks).

## 14. Other Runtime Notes
- **Fractal Sharding / Swarm**: `qualia-cli daemon --workers N --compute-swarm` spins isolated 512MB cells on big iron for parallel LLM offload / deep inference without violating per-cell floors.
- **Desktop Sentinel (libpcap)**: network_sentinel.html + qualia-desktop shows active telemetry blocking + P2P reputation DAGs.
- **Wellfair / Mobile / Desktop**: See crates/qualia-android, qualia-desktop (Tauri), and the wellfair/ Python+web monorepo for the consumer "Native Vault" (holograph, verifiable comms, Sanctuary Lanes, etc.).
- When contributing or scripting, run `cargo check -p qualia-core-db -p qualia-cli` and the native bench to keep the harness green.
