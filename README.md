# Qualia-DB

> Peace infrastructure for the natural person — running on your hardware, connected to the world on your terms.

[Watch: *The Untransferable Code*](https://www.youtube.com/watch?v=HJJs-Ve-Dhg) — the philosophical foundation of this work.

Qualia-DB is a zero-allocation, 5-vector heterogeneous semantic graph engine built for personal devices. It enforces a strict **Principal-Agent Duty of Care**: the software is an Agent acting exclusively on behalf of the Natural Person (the Principal), never a platform extracting value for a third party.

**512MB RAM floor · 48-byte Super-Quins · Webizen VM with N3/SHACL/Defeasible logic · ILP-native economics**

---

## Try It

**No install required:** [Live Playground →](https://mediaprophet.github.io/qualiaDB/playground/index.html)

**Native CLI:**
```bash
cargo build --release -p qualia-cli
cargo run --release -p qualia-cli -- bench --suite full
```

**Desktop app:** Download from [Releases](https://github.com/mediaprophet/qualiaDB/releases) (Windows, macOS, Linux, Android).

---

## What it is

Qualia-DB is three things at once:

1. **A zero-allocation semantic graph engine** — binary `.q42` ledgers with 48-byte Super-Quins, LZ4 SuperBlocks, and microsecond memory-mapped queries. WordNet (523MB RDF) compresses to 74.6MB and streams with 6.5ms first-query latency via demand-paging.

2. **A Webizen VM** — an N3Logic + SHACL + defeasible + omnimodal compiler that evaluates Rights Ontology rules, escrow adjudication, and structural constraints at query time with O(1) termination guarantees.

3. **A Principal-Agent ecosystem** — DID:GIT staged axiomatic evolution, ILP Threshold Shift Licensing, decentralized threat intelligence, and a native Cooperative Workspace for shared projects.

---

## v0.0.5 Highlights

- **Multi-Seed Credential Architecture**: Standalone external account imports for Bitcoin (BTC), eCash (XEC), Nym (Nyx), Ethereum (EVM), and Monero (XMR).
- **Semantic Typology Routing**: Direct integration with LLaVA/Minkowski engines utilizing Typology Lenses (Meme Engine, Heraldry Engine) to dynamically shape the RDF payloads upon ingestion.
- **Hardware Orchestration Dashboard**: Explicit real-time WASM boundary visualization exposing atomic background memory backpressure (`nym-telemetry`) and out-of-core disk paging thresholds (`stark-telemetry`).

Full release notes: [docs/manuals/RELEASE_NOTES_v0.0.5.md](docs/manuals/RELEASE_NOTES_v0.0.5.md)

---

## v0.0.4 Highlights

- **Webizen Rebrand**: The legacy "Sentinel VM" architecture is now fully rebranded to "Webizen" across the codebase to accurately reflect sovereign agency.
- **W3C Solid Interoperability Bridge**: A sandboxed `tokio` Allocation Firewall allows seamless HTTP REST export and import into legacy Solid Pods without breaking the 512MB RAM native floor.
- **Native "Hard Science" SHACL Extensions**: Pure-Rust continuous dynamics, quantum DFT bounding, thermodynamics, and SIMD bioinformatics can now be natively invoked via `qualia:` semantic extensions off-heap.
- **Desktop KaTeX Integration**: The Neuro-Chat UI automatically identifies mathematical offloads and renders beautiful LaTeX equations directly into the chat bubbles.
- **HCAI DNS Frontdoor**: `qualia-cli webizen dns-frontdoor` generates zero-permission W3C `did:web` and DNS TXT records to allow discovery without compromising the offline-first vault.

Full release notes: [docs/manuals/RELEASE_NOTES_v0.0.4.md](docs/manuals/RELEASE_NOTES_v0.0.4.md)

## Documentation

| Document | Purpose |
|----------|---------|
| [Architecture](docs/manuals/ARCHITECTURE.md) | 3-Core Triad, Webizen VM, Rights Ontology, ILP engine, DID:GIT, Fractal Sharding |
| [Development Guide](docs/manuals/DEVELOPMENT.md) | Build from source, CLI reference, benchmarks, cross-compilation |
| [Developer Guide](docs/manuals/developer-guide.md) | API reference and integration patterns |
| [Developing Qualia Apps](docs/manuals/developing-apps.md) | Building UI pipelines, Tauri integrations, and hardware orchestration |
| [ADRs](docs/manuals/adr/) | Architectural Decision Records (48-byte Quins, zero-alloc compiler, governance) |
| [Glossary](docs/manuals/glossary.md) | Terms and concepts |
| [AI Instructions](AI_INSTRUCTIONS.md) | Guidance for AI agents working on this codebase |

---

## License

[Creative Commons Attribution-NonCommercial-NoDerivatives 4.0 International](LICENSE)

For commercial licensing, enterprise integration, or consulting on Intentional Computing:
**Timothy Charles Holborn** · [LinkedIn](https://www.linkedin.com/in/ubiquitous/)

---

*Built to guarantee first-class digital agency.*
