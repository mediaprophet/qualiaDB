# Qualia-DB: The Human-Centric Semantic Ecosystem
<div align="center">
**Peace infrastructure for the natural person — running on your hardware, connected to the world on your terms.**

See: [*The Untransferable Code*](https://www.youtube.com/watch?v=HJJs-Ve-Dhg) — the philosophical foundation of this work.

Qualia-DB has evolved far beyond a bare-metal semantic graph database. It is a strictly constrained, mathematically verifiable **Ecosystem** designed to guarantee human agency, protect knowledge from proprietary enclosure, and dismantle infinite rent-seeking via native micro-economics.

It operates under a rigorous **Principal-Agent Duty of Care**: the software acts exclusively as the Agent on behalf of the Natural Person (the Principal).

**v0.0.3 Highlights**: Fully operational native `qualia-cli bench --suite full` harness (Lazy SuperBlocks + LZ4 + WebRTC telemetry + live visualizer), SHACL-to-Sentinel compiler, 6 modality bridges, defeasible + omnimodal logic in the Sentinel VM, high-density LZ4 SuperBlocks, and major ingestion/query improvements for massive real-world datasets. See [RELEASE_NOTES_v0.0.3.md](RELEASE_NOTES_v0.0.3.md).

## 🚀 The Three-Core Database Engine & The Orchestration Sieve

Qualia-DB abandons traditional cloud-centric, string-heavy JVM architectures in favor of a specialized 3-Core Triad built with ruthless mechanical sympathy (512MB RAM floor). 
However, raw multi-modal data (audio, camera feeds) would immediately crash this floor. To prevent this, the ecosystem forces an **Orchestration Sieve**: the Primary Agent must coordinate deterministic tools (OpenCV, Audio DSP) to strip noise, extract contours, and build optimized files *before* handing them to the local LLM or the database.

1. **Zero-Allocation Ingestion**: CBOR-LD gatekeeping and WASM OPFS bridging bypass heap-saturation attacks, writing natively to disk.
2. **GPU Sieve (Geometric Pruning)**: Graph nodes are mapped into Minkowski space within continuous 128KB memory-mapped `QualiaSuperBlocks`. The GPU calculates bounding-hull collisions to retrieve data at sub-microsecond speeds.
3. **The Sentinel (Logic Unification + Advanced Compilation)**: Data filtering is not enough; human-centric databases must execute logic. Nested N3 implication rules, SHACL shapes, and defeasible logic are compiled by the `SentinelCompiler` (and dedicated `shacl_compiler`) into compact L1-cache bytecodes. The VM supports omnimodal surface syntaxes and 6+ modality bridges (spatio-temporal, probabilistic, description logic, ASP, etc.). Guarantees $O(1)$ termination on highly cyclic social and legal graphs while enforcing Rights Ontology and structural constraints at query time.

### Fractal Sharding & Swarm AI Compute
While Qualia-DB rigorously enforces the 512MB floor to guarantee universal access, it is capable of extreme horizontal scale on high-end hardware. Rather than bloating a single instance into a massive JVM heap, it employs **Fractal Sharding**.
If installed on a powerhouse rig (e.g., 64GB RAM, 12GB+ GPU), the daemon detects the hardware surplus and dynamically spins up dozens of parallel, mathematically isolated 512MB worker cells:
```bash
qualia-cli daemon --workers 100 --compute-swarm
```
This Swarm Orchestration enables massive parallel execution, deep neural-network offloading, and background **Sleep-Cycle AI Compute** without ever compromising the pristine mechanical sympathy of the core architecture.

### Lazy SuperBlocks, LZ4 Compression & Massive Datasets
Core data lives in 40,960-byte SuperBlocks (exactly 10 disk sectors) with high-density LZ4 compression. The engine can lazily scan only 16-byte headers and seek over irrelevant blocks in O(1) time, decompressing on-demand. "Missing" local blocks can be streamed from peers (WebRTC DataChannel simulation in the harness). This lets 50GB+ semantic ledgers run comfortably inside the 512MB floor.

See:
- `qualia-cli import` + `scripts/fetch_massive_datasets.ps1` for turning real-world RDF (DBpedia, YAGO, Framester, GeoNames) into `.q42`
- `qualia-cli query` and `lazy_superblock_query` for microsecond mmap / lazy access
- Live telemetry dashboards in `benchmark_visualizer.html` (pairs with the native harness) showing RSS, blocks loaded, and local vs. remote hot blocks.

## ⚖️ The Rights Ontology & Semantic Adjudicator

Qualia-DB natively encodes a **Rights Ontology** directly into the Sentinel VM (now with SHACL compilation, defeasible rules, and modality bridges).
- **Linguistic Plurality & Multi-Modal Semantics**: We reject the assumption that knowledge is exclusively bound to written Unicode strings. By utilizing binary CBOR-LD indexing, the ecosystem inherently supports "mother tongues", "languages of prayer", and non-written formats (verbal, ceremonial, heraldry, symbolic SVGs). A Semantic Quin maps a concept natively, regardless of the cultural format.
- **The Knowledge Axiom Predicate**: Rights to knowledge and fundamental shared learnings are mathematically un-propertizeable. If a semantic dispute arises, the Sentinel VM automatically dismisses any attempt to extract or enclose a Knowledge Axiom as intellectual property.
- **Proportional Escrow (Relational Assertion)**: When a dispute involves a specific *Application* or *Invention*, the N3Logic VM analyzes the `.q42` Provenance DAGs of both parties. It mathematically calculates the exact percentage of derivation and automatically splits incoming ILP Escrow funds based on absolute truth, stripping away false claims of originality.
- **SHACL & Structural Enforcement**: SHACL shapes are compiled into the same Sentinel bytecode used for N3, enabling zero-allocation validation of data shapes and constraints as part of query execution (new in v0.0.3).

## 🛡️ Intentional Computing (Anti-Usury Architecture)

Modern computing often exploits the user (usury) through unconsented telemetry and enclosed architectures. Qualia-DB rejects this. It is a framework for **Intentional Computing**—computing that strictly honors the intent, sovereignty, and Duty of Care of the natural person (the Principal).

- **First-Class Agency**: There is no "admin" superuser that supersedes you. Your cryptographic keys are the absolute root of trust for your data boundary.
- **Desktop Network Sentinel (`libpcap`)**: For desktop and laptop environments, the local daemon acts as an active Wireshark-like firewall. It monitors all outbound network egress. If unauthorized spyware or telemetry SDKs attempt to exfiltrate your private N3Logic data, the Sentinel actively severs the socket connection to neutralize the malware.
- **Decentralized Threat Intelligence**: When the Sentinel encounters an *unknown* network pattern, it dynamically queries a peer-to-peer network of `.q42` reputation DAGs. If the graph flags the behavior as spyware, the Sentinel dynamically generates a new N3Logic permission rule to block it, saving the event to a cryptographic audit chain.

## 🧬 DID:GIT & Staged Axiomatic Evolution

Data projects in this ecosystem are not static—they possess **Temporal Self-Governance**.
- Through the `did:git` Permissive Commons Profile, every project initializes a DOAP (Description of a Project) as its Genesis Block.
- To evolve a project to its next stage (e.g., changing its license or logic), the proposed `git` commit must be mathematically validated by the N3Logic Sentinel VM against the *former* axioms of the previous stage. 
- If valid, the transition is anchored globally to the Bitcoin blockchain via `gitmark`.

## 💸 The ILP Economic Shift Engine

We explicitly reject the infinite rent-seeking paradigm of the legacy web.
- Creators define an exact **Obligation Cost** using N3Logic Risk-Compounding algorithms (factoring base rate, risk multiplier, and temporal compounding).
- As Interledger Protocol (ILP) Web Monetization streams flow in, the Daemon tracks the balance.
- Once the exact mathematical threshold is met, the **Threshold Shift License (TSL)** automatically fires, irreversibly shifting the asset from *Commercial Gating* to the *Permissive Commons*.

## 📱 The Consumer Packaging (Qualia Native Vault)

Qualia-DB natively ships with two critical consumer interfaces tightly bound to the zero-copy SLG Arena:
1. **The Qualia Mobile Vault (Android Jetpack Compose)**: The mobile app serves as the authoritative node. It handles high-frequency wearable ingestion (via native C++ FFT Photoplethysmography), Verifiable Communications, and powers the **3D Biometric Holograph** (mapping valence and arousal telemetry to Minkowski space). It implements native **Sanctuary Modes** with mathematically isolated PBKDF2 DB Lanes for vulnerable populations.
2. **Qualia Desktop Terminal (Tauri)**: A stateless desktop extension that pairs to the Mobile Vault via WebRTC (VC-8 Semantic Handshake). It features the **Semantic Library**, which natively ingests massive document libraries (PDFs via Edge VLM, or raw `.rdf` / `.owl` ontologies) into binary `.q42` graphs. It acts as a heavy compute offload target for the **Webizen Agent**, executing local LLMs (Ollama) while streaming live telemetry back to the mobile vault.

## 🧪 LLM Benchmarking Suite (Dual-Mode, Now Fully Operational)

Qualia-DB features a headless benchmarking suite explicitly designed for AI Agents (Claude, Grok, ChatGPT) to evaluate the engine's performance under strict human-centric constraints (512MB RAM floor). For the natural person, their device isn't "the edge"—it is the **heart** of the network. 

**Native (authoritative, real engine + telemetry + JSON artifact):**
```bash
cargo run --release -p qualia-cli -- bench --suite full
# alias also works:
cargo run --release -p qualia-cli -- benchmark --suite full
```
- Drives Lazy SuperBlock scans (LZ4 compressed 40KB blocks), WebRTC-mocked P2P streaming, live sysinfo RSS + hot-block telemetry (WS :9090).
- Produces `llm_benchmark_results.json` (12 categories incl. next-gen rights tests: Obligation Escrow, Provenance, Multi-Nym Partitioning).
- Visualizer: open `benchmark_visualizer.html` (or root `benchmark.html`).
- Micro-benches: `cargo bench -p qualia-core-db` (Criterion, vs Oxigraph/SurrealDB class proxies).
- Browser fallback: `node scripts/llm_bench_runner.js --suite full`.

See also AI_INSTRUCTIONS.md §9 for agents.

### Testing with Massive Datasets (Epic 17)

To prove the zero-allocation architecture and microsecond `mmap` read speeds, we have provided a script to download massive real-world semantic datasets (like GeoNames, YAGO Tiny, DBpedia subsets, and Framester). 

1. **Download the datasets (2GB - 12GB range):**
   ```powershell
   ./scripts/fetch_massive_datasets.ps1
   ```
2. **Convert to .q42 Native Binary:**
   ```bash
   qualia-cli import ./data/mappingbased-objects.ttl.bz2 ./data/dbpedia.q42
   ```
3. **Execute an OS-level Memory-Mapped Query:**
   ```bash
   qualia-cli query ./data/dbpedia.q42 --subject 123456
   ```
   *The 50GB binary will be memory-mapped and the Quins will be fetched in microseconds without touching RAM heap allocation.*

## 🌐 W3C Solid Interoperability Bridge

Qualia-DB operates entirely natively on `.q42` CBOR-LD binary graphs to bypass the string-parsing bloat of traditional Semantic Web DBs. However, to guarantee global backward compatibility, it features a native **Solid Exporter**. 

This exporter acts as a one-way bridge: it compiles the highly constrained 48-byte `Super-Quins` back into standard W3C Turtle (`.ttl`) format and generates LDP Basic Containers, allowing you to instantly back up your encrypted vault to any standard W3C Solid Pod (like Inrupt or CSS).

> **To-Do / Roadmap (WAC ACLs):** W3C Solid uses static Web Access Control (`.acl` files) to govern data sharing. Qualia-DB uses the dynamic N3Logic Sentinel VM (evaluating things like "Is the principal's biometric stress level currently safe enough to authorize this?"). Currently, the exporter conservatively defaults complex dynamic N3 rules to **Private (`acl:Control`)** during export to prevent data leakage. Future iterations will aim to compile bounded N3 rulesets into static ACL groups.

## 🛠️ Getting Started, CLI & Tooling

### Build from Source
```bash
# Core native engine + CLI (current platform)
cargo build --release -p qualia-cli

# WebWorker WASM Bridge (for browser playground)
cd crates/qualia-core-db
wasm-pack build --target no-modules --out-dir ../qualia-client/pkg

# Desktop Terminal (Tauri, current platform)
cd crates/qualia-desktop
cargo tauri build   # or cargo build --release for the rust side only
```

**Cross-platform binaries (recommended):**

The project uses GitHub Actions (`.github/workflows/release.yml`) to automatically build:

- qualia-cli for Windows, macOS (Intel + Apple Silicon), Linux (x86_64)
- Full desktop bundles (.dmg for macOS, AppImage/deb for Linux, .exe/.msi for Windows)
- Android APK

To trigger official macOS and Linux builds:
```bash
git tag v0.0.4
git push origin v0.0.4
```

Then download the platform-specific artifacts from the GitHub Release page and place them in `releases/` if desired for local distribution.

Local cross-compilation of the full Tauri desktop apps from Windows is not straightforward (Tauri bundlers are platform-specific); use the CI for macOS/Linux desktop releases. For the pure-Rust `qualia-cli` you can add targets with `rustup target add ...` and build with `--target`, but some native dependencies (ring, etc.) require a matching cross-compiler toolchain on the host.
```

### The `qualia-cli` Swiss Army Knife (v0.1.1)
The native CLI is the primary way to exercise the full engine, including the now-fully-operational Dual-Mode benchmark harness:

```bash
# LLM / Agent Benchmark Harness (produces llm_benchmark_results.json + live telemetry)
cargo run --release -p qualia-cli -- bench --suite full
# (also works as `benchmark --suite full`)

# Run the live block-level telemetry visualizer alongside it
# (opens benchmark_visualizer.html and connects to WS telemetry on :9090)

# Ingest real semantic data into native .q42 (Rio streaming + LZ4 SuperBlocks)
qualia-cli import ./data/something.ttl ./data/out.q42

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

# Detailed dev benchmarks (require a .q42)
qualia-cli benchmark-action rss-scan ./data/out.q42 10
```

See the full subcommand list via `qualia-cli --help`. The native `bench` command is the recommended path for AI agents and CI (see AI_INSTRUCTIONS.md §9).

### Releases & Versioning
- Current: Core/Desktop **0.0.3-dev**, CLI **0.1.1**
- See [RELEASE_NOTES_v0.0.3.md](RELEASE_NOTES_v0.0.3.md) for the full list of new capabilities (SHACL compiler, modalities, Lazy SuperBlocks + WebRTC telemetry, LZ4, defeasible/omnimodal logic, working native harness, etc.).
- Prebuilts: Windows installer, Android APK, and desktop bundles are in the `releases/` directory (GitHub Releases for signed artifacts).

## 📝 Licensing & Commercial Inquiries

Qualia-DB is currently released under the **Creative Commons Attribution-NonCommercial-NoDerivatives 4.0 International License**.

For any commercial licensing inquiries, custom enterprise integration, or consulting regarding Intentional Computing, please reach out to the creator directly:
- **Timothy Charles Holborn**
- [LinkedIn Profile](https://www.linkedin.com/in/ubiquitous/)

---
*Built to guarantee first-class digital agency.*

For the full list of new features in this release (SHACL compiler, Lazy SuperBlocks + WebRTC telemetry, native benchmark harness, modalities, LZ4, defeasible/omnimodal logic, etc.) see [RELEASE_NOTES_v0.0.3.md](RELEASE_NOTES_v0.0.3.md). The canonical reference for AI agents is [AI_INSTRUCTIONS.md](AI_INSTRUCTIONS.md). A glossary of terms is in [docs/glossary.md](docs/glossary.md).
