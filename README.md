# QualiaDB / Webizen

> Peace infrastructure for the natural person — running on your hardware, connected to the world on your terms.

[Watch: *The Untransferable Code*](https://www.youtube.com/watch?v=HJJs-Ve-Dhg) — the philosophical foundation of this work.

> **⚠️ Pre-release — active development.**
> This is v0.0.x software. APIs, binary formats, wire protocols, and `.q42` storage layout change without notice until v0.1.0. Do not use in production.

---

## The problem

Every major platform today treats your data, your identity, and your relationships as assets it owns. AI systems compound this: they act on behalf of whoever controls the infrastructure, not on behalf of you.

QualiaDB is built on a different premise. Software running on your device has a fiduciary obligation to *you* — the natural person who owns the hardware. It acts as your agent, not as a data pipeline for a third party.

---

## What it is

**QualiaDB** (desktop application: **Webizen**) is a local-first semantic database and
computational platform for personal AI, knowledge graphs, governed cooperation, and
human-controlled applications.

### Semantic graph and query engine

- Compact 48-byte `NQuin` records and memory-mapped **unified Q42 v3** volumes
  (`Q42\0`, 256-byte header) provide deterministic, bounded graph storage. Lexicon,
  object-range BIDX, optional field-range/postings indexes, and LZ4 SuperBlocks live
  **inside** the single `.q42` file. There is no sibling `.c.q42` or `.q42.lex` on new
  writes. Public magnets fail closed unless the volume is affirmatively Permissive
  Commons; Sanctuary / personal / medical volumes cannot mint a public hash address.
- The live SPARQL engine supports SELECT, ASK, CONSTRUCT, DESCRIBE, named graphs,
  SPARQL-Star, OPTIONAL, UNION, MINUS, BIND, aggregates, sorting, full transitive property
  paths, governed UPDATE, and local or explicitly requested HTTP federation.
- GeoSPARQL executes real WKT distance and topology operations. QISP adds typed Tensor10D
  values, dense-asset references, a typed extension registry, and inline tensor-distance
  and tensor-within predicates.
- RDF and result serializers cover SPARQL JSON/XML/CSV/TSV plus
  N-Triples/N-Quads/Turtle/TriG/N3/JSON-LD/CBOR-LD representations.

### Local inference and GPU compute

- Native GGUF/P64 inference runs in-process with real autoregressive decode—no Python or
  separate model server is required for the local path. Browser builds provide Rust→WASM
  WebGPU decode with OPFS-cached model containers.
- The workspace uses wgpu/naga 30 across core inference, extensions, rendering, and the
  simulation runtime. Device capabilities are negotiated per adapter rather than assumed.
- WGSL Forge provides typed kernel and schedule IR, deterministic shader generation,
  Naga validation, CPU differential oracles, bounded autotuning, pipeline caching, and
  WGSL/HLSL/MSL/PTX/CUDA-C/SPIR-V targets.
- Cooperative-matrix and ray-query paths are explicit experimental opt-ins. A runtime
  oracle prevents a broken driver primitive from returning corrupt inference output;
  f32 workloads retain an exact GPU/CPU fallback, while reduced-precision CUDA WMMA is a
  separately named choice.
- P64 supports real GGUF conversion, role-tagged tensors, CRC-32C integrity, layer-packed
  alignment, zero-copy reading, and Forge execution over resident model weights.

### Logic, governance, and privacy

- The Webizen VM provides bounded N3Logic execution, SHACL validation, and modality
  evaluators for deontic, epistemic, paraconsistent, temporal, description, and other
  logic families.
- The native governed-inference path applies intent validation, N3 output gating,
  grounding/provenance checks, and commit-on-permit sequencing.
- The privacy engine includes real BFV homomorphic encryption, caller-buffered calibrated
  differential privacy, composition accounting, and fixed-size external ciphertext
  references so large cryptographic objects do not enter the 48-byte semantic ABI.

### Geometry, simulation, and visualization

- The computational-geometry library includes adaptive exact predicates, exact
  constructions, 2D/3D hulls and Delaunay methods, mesh generation/refinement/repair,
  boolean operations, topology, BVH and point location, reconstruction, remeshing,
  decimation, motion planning, and persistent-homology tooling.
- The numerical libraries provide real linear algebra, statistics, optimization,
  economics, engineering, physics, chemistry, bioinformatics, and clinical reference
  computations, shared by native, MCP, CLI, and WASM surfaces where exposed.
- `webizen-render` is a wgpu 30 N-dimensional renderer with Tensor10D projection,
  depth/bloom, meshes, picking, CPU readback, and native surface presentation.
  `webizen-runtime` supplies a fixed-timestep GPU diffusion kernel with deterministic
  snapshots and ledger integration.

### Identity, cooperation, and applications

- DID-based identity, Verifiable Credentials, delegated access, W3C Solid/LDP interop,
  and SocialWebNet peer connectivity support user-controlled exchange across devices and
  institutions.
- **WellFair** is a personal health and welfare vault with anatomy visualization,
  medication, clinical, finance, consent, guardianship, communication, and cooperative
  support records built on the same semantic and provenance substrate.
- Webizen Desktop and Studio expose the graph, inference, rendering, QApp, governance,
  and WellFair capabilities through native Tauri commands and browser-compatible UI
  contracts.

---

## Who it is for

| | |
|---|---|
| **Individuals** | Own your AI agent and your data. No platform intermediary, no surveillance. |
| **Developers** | Build Webizen qapps on a SPARQL + semantic graph API with full fiduciary guarantees baked in. |
| **Institutions** | Interoperate with Webizen users via W3C Solid Protocol and WebID without adopting the full stack. |

---

## Get started

**Live playground (no install):** [mediaprophet.github.io/qualiaDB/playground/](https://mediaprophet.github.io/qualiaDB/playground/index.html)

**Desktop app (Webizen — Windows, macOS, Linux; Tauri + Dioxus, native GPU dispatch, signed installer/updater):** Download from [Releases](https://github.com/mediaprophet/qualiaDB/releases).

**CLI:**
```bash
cargo build --release -p qualia-cli
./target/release/qualia --help
```

**WASM:** `qualia-core-wasm.tar.gz` in [Releases](https://github.com/mediaprophet/qualiaDB/releases) — embed in any web project.

Full build instructions, CLI reference, and benchmark guide: [docs/manuals/DEVELOPMENT.md](docs/manuals/DEVELOPMENT.md).

---

## Current status

**0.0.30 (active branch)** — active development, pre-release. Unified Q42 v3 is the
only new-write graph container.

Recent verification of the implemented surfaces includes:

- **SPARQL/QISP:** 299 passed, 0 failed; one real-network integration test is gated.
- **Core library acceptance:** the most recent full recorded run passed 5,365 tests with
  no failures; subsequent focused query and GPU-selector suites also pass.
- **Computational geometry:** 1,517 passed, 0 failed, 0 ignored.
- **Renderer/runtime:** 48 renderer tests and 2 runtime tests passed.
- **Real-model Forge inference:** SmolLM2-360M P64 layer-0 GPU execution matched its CPU
  oracle at `max_rel=3.28e-6`.
- **Cross-target builds:** core, extensions, renderer, runtime, CLI, the device-benchmark
  worker, and the WASM LLM feature profile compile on their relevant targets.

The browser LLM engine is pure Rust→WASM. Live demos:
[`online-llm-demo.html`](https://mediaprophet.github.io/qualiaDB/online-llm-demo.html) ·
[`llmdemo`](https://mediaprophet.github.io/qualiaDB/llmdemo/).

For the precise boundary between implemented functionality, intentionally gated external
capabilities, and remaining pre-v0.1 work, see the
[functionality manual](docs/manuals/qualia_db_functionality_manual.md). Full release history:
[CHANGELOG.md](CHANGELOG.md).

---

## Documentation

| Document | Purpose |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Full technical architecture — Quin bit layout, all modalities, inference stack, every module |
| [docs/manuals/qualia_db_functionality_manual.md](docs/manuals/qualia_db_functionality_manual.md) | Per-crate functionality manual — what each part of the workspace actually does today |
| [docs/manuals/DEVELOPMENT.md](docs/manuals/DEVELOPMENT.md) | Build, test, benchmark, CLI reference, cross-compilation |
| [docs/progress-0.0.30.html](docs/progress-0.0.30.html) | 0.0.30 progress — Q42 v3 volumes, Pages, desktop |
| [docs/manuals/standards/q42-format-internal-draft.md](docs/manuals/standards/q42-format-internal-draft.md) | Canonical Q42 v3 physical layout (48-byte NQuin, 40,960-byte SuperBlock, 256-byte header) |
| [docs/manuals/qapps_specification.md](docs/manuals/qapps_specification.md) | QApp manifest spec — build apps for the Webizen platform |
| [CHANGELOG.md](CHANGELOG.md) | Release history |
| [AGENTS.md](AGENTS.md) / [CLAUDE.md](CLAUDE.md) | AI agent orientation for contributors |

---

## License

[Creative Commons Attribution-NonCommercial-NoDerivatives 4.0 International](LICENSE)

For commercial licensing, enterprise integration, or consulting on Intentional Computing:
**Timothy Charles Holborn** · [LinkedIn](https://www.linkedin.com/in/ubiquitous/)

---

*Built to guarantee first-class digital agency.*
