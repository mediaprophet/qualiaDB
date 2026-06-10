# Qualia-DB

> Peace infrastructure for the natural person — running on your hardware, connected to the world on your terms.

[Watch: *The Untransferable Code*](https://www.youtube.com/watch?v=HJJs-Ve-Dhg) — the philosophical foundation of this work.

> **⚠️ Pre-release — active development.**
> This is v0.0.x software. Breaking changes to the API, binary formats, wire protocols, and `.q42` storage layout occur without deprecation notices. Nothing is stable until v0.1.0 at minimum. Do not use in production.

Qualia-DB is a zero-allocation, 5-vector heterogeneous semantic graph engine built for personal devices. It enforces a strict **Principal-Agent Duty of Care**: the software is an Agent acting exclusively on behalf of the Natural Person (the Principal), never a platform extracting value for a third party.

**512MB RAM floor · 48-byte Super-Quins · Webizen VM with N3/SHACL/Defeasible logic · MCP fiduciary mediation · ILP-native economics**

---

## Try It

**No install required:** [Live Playground →](https://mediaprophet.github.io/qualiaDB/playground/index.html)

**Native CLI:**
```bash
cargo build --release -p qualia-cli
cargo run --release -p qualia-cli -- bench --suite full
```

**Desktop app (Flutter):** Download from [Releases](https://github.com/mediaprophet/qualiaDB/releases) (Windows, macOS, Linux).

**WASM:** `qualia-core-wasm.tar.gz` in [Releases](https://github.com/mediaprophet/qualiaDB/releases) — drop into any web project.

---

## What It Is

Qualia-DB is four things at once:

1. **A zero-allocation semantic graph engine** — binary `.q42` ledgers with 48-byte Super-Quins, LZ4 SuperBlocks, and memory-mapped queries (query layer under development - see to-do/005). WordNet (523MB RDF) compresses to 74.6MB and streams with 6.5ms first-query latency via demand-paging.

2. **A Webizen VM** — an N3Logic + SHACL + full modality logic compiler that evaluates deontic norms, epistemic claims, temporal traces, paraconsistent contradictions, Rights Ontology rules, escrow adjudication, and structural constraints at query time with O(1) termination guarantees.

3. **A fiduciary AI layer** — MCP Intent Frame mediation, capability profiles, and seven LLM fiduciary rules ensure every AI agent call is pre- and post-validated against the Principal's declared rights, with conduct violations written to a WAL with conduct logging (ECC parity under development - see to-do/004).

4. **A Principal-Agent ecosystem** — DID:GIT staged axiomatic evolution, ILP Threshold Shift Licensing, decentralized threat intelligence, and a native Cooperative Workspace for shared projects.

---

## v0.0.10-dev Highlights (current branch)

- **In-process LLM inference**: `GgufTokenizer` parses the GGUF v2/v3 KV section (vocabulary, BOS/EOS IDs); greedy longest-match encode; SentencePiece-aware decode. `infer_local_model()` runs a real autoregressive decode loop — GPU dispatch via DirectML 1.15 (Windows) / Accelerate AMX (macOS) / wgpu/Vulkan (Linux) — with Phase 8 SPSC Webizen Sentinel mid-generation rollback. No Ollama, no Python.
- **Flutter Chat UI wired**: `runInference(prompt, modelPath)` exported via flutter_rust_bridge. The Chat screen calls the full `TaskOrchestrator` governance pipeline (intent validation → Phase 8 GPU loop → provenance grounding) and shows a live loading indicator.
- **GPU compute layer**: DirectML 1.15 SDK (`vendor/directml/`), `directml_bridge.rs` (real D3D12 + Q4_K GEMM), `metal_bridge.rs` (Accelerate `cblas_sgemm`, runs on Apple AMX), `gguf_bridge.rs` (`load_gguf` memory-maps weights via `memmap2`; `dispatch_fused_transformer_block` tries DirectML → Accelerate → wgpu in order).
- **Full Modality Stack**: Epistemic logic (OP_KNOWS/BELIEVES/COMMON_KNOWLEDGE), LTL trace evaluation (G/F/X/U/R), Paraconsistent routing (contradiction isolation without system halt), Dialectical synthesis (thesis-antithesis-synthesis over ASP stable models).
- **N3 → Deontic Bridge**: N3 rule parser now compiles directly to norm Quins. `^>` (Defeater) rules set `DEFEATER_BIT`. Round-trip tested.
- **MCP Fiduciary Mediation**: `McpIntentFrame` + `enforce_fiduciary_tool_dispatch` + sanctuary gate with WAL conduct logging.
- **LLM Agent Rules**: `AgentIntent` + `WebizenVerdict` + seven fiduciary rules including adversarial conduct tracking (DID-associated, cryptographically auditable).
- **Capability Profiles**: QCHK binary format, six named profiles, profile-bound `ingest`, `profile compile/list/inspect` CLI.
- **Resource Catalog**: Full download pipeline (YAML → reqwest → GGufSharder → WAL). `resources` CLI subcommand live. LLM, Ontology, and SPARQL endpoint registries.
- **539 test functions in qualia-core-db alone** (browser suite + unit suite).

**Build Status**: ✅ Compiles successfully (0 errors, all 82 build errors resolved)

**⚠️ Known Limitations** (see [to-do/](to-do/) for implementation tasks):
- Query layer stubs: mmap_query_subject, lazy_superblock_query need real implementation
- Security stubs: zk_proofs, fiduciary_crypto, ML-DSA, ECC parity need real implementation  
- Linux LLM inference: wgpu/Vulkan path uses mock pipeline (placeholder shader)

Full changelog: [CHANGELOG.md](CHANGELOG.md)

---

## v0.0.5 Highlights

- **Multi-Seed Credential Architecture**: Standalone external account imports for Bitcoin (BTC), eCash (XEC), Nym (Nyx), Ethereum (EVM), and Monero (XMR).
- **Semantic Typology Routing**: Direct integration with LLaVA/Minkowski engines utilizing Typology Lenses (Meme Engine, Heraldry Engine) to dynamically shape the RDF payloads upon ingestion.
- **Hardware Orchestration Dashboard**: Explicit real-time WASM boundary visualization exposing atomic background memory backpressure (`nym-telemetry`) and out-of-core disk paging thresholds (`stark-telemetry`).

Full release notes: [docs/manuals/RELEASE_NOTES_v0.0.5.md](docs/manuals/RELEASE_NOTES_v0.0.5.md)

---

## Quick-Start CLI Examples

### Ingest RDF data with a capability profile
```bash
# Compile a JSON-LD profile to binary QCHK
qualia profile compile profiles/health.jsonld health.qchk

# Ingest Turtle file bound to the health profile
qualia ingest --profile health.qchk data/patient-graph.ttl output.q42
```

### Browse and download resources
```bash
# List available LLM models
qualia resources list llms

# Show details of a specific resource
qualia resources show phi3-mini

# Download and ingest an ontology
qualia resources import-ontology snomed-ct

# Download an LLM to local vault
qualia resources download gemma2-2b-q4
```

### Profile management
```bash
# List known profiles with their q_hash IDs
qualia profile list

# Inspect a compiled QCHK profile
qualia profile inspect health.qchk
```

### Query and inspect
```bash
# Run the full benchmark suite
qualia bench --suite full

# Inspect Quin fields from a .q42 file
qualia inspect output.q42

# Start the local daemon
qualia daemon start
```

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     Principal (Natural Person)                  │
└────────────────────────────┬────────────────────────────────────┘
                             │ Capability Profile (QCHK)
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                MCP Intent Frame Mediation Layer                 │
│         enforce_fiduciary_tool_dispatch + sanctuary gate        │
└────────────┬───────────────────────────────────────────┬────────┘
             │                                           │
             ▼                                           ▼
┌────────────────────────┐              ┌────────────────────────┐
│     LLM Agent Layer    │              │    Query Engine         │
│  AgentIntent +         │              │  SPARQL-like + N3Logic  │
│  WebizenVerdict        │              │  mini_parser.rs         │
│  7 fiduciary rules     │              │                         │
└────────────┬───────────┘              └────────────┬───────────┘
             │                                       │
             ▼                                       ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Webizen VM (SlgArena 42MB)                    │
│                                                                 │
│  Deontic    Epistemic    LTL    Paraconsistent    ASP/DL        │
│  0x10-12    0x20-22   0x40-44    0x30-32       modalities/      │
│                                                                 │
│  SHACL Compiler → WebizenOpcode bytecode                       │
│  N3 Parser → Rule types → compile_n3_rule_to_norm              │
│  Native Scientific: Clinical, Bioinformatics, Chemistry,        │
│                     ODE, DFT, Thermodynamics, Geometry          │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Storage Engine (.q42)                         │
│   SuperBlocks (LZ4) + BIDX demand-paging + WAL (Ed25519)       │
│   48-byte QualiaQuins | FNV-1a hashed IRIs | zero heap          │
└─────────────────────────────────────────────────────────────────┘
```

Full architecture documentation: [ARCHITECTURE.md](ARCHITECTURE.md)

---

## Documentation

| Document | Purpose |
|----------|---------| 
| [ARCHITECTURE.md](ARCHITECTURE.md) | Full layered architecture: Quin bit layout, all modalities, MCP mediation, LLM fiduciary rules, capability profiles, scientific engines, CLI |
| [HANDOVER.md](HANDOVER.md) | Session handover for next AI agent — current state, known gaps, suggested tasks |
| [CHANGELOG.md](CHANGELOG.md) | Release history |
| [TODO.md](TODO.md) | Remaining work and known gaps |
| [docs/PROJECT_STATE.md](docs/PROJECT_STATE.md) | Phase completion status |
| [docs/RESOURCE_CATALOG.md](docs/RESOURCE_CATALOG.md) | Resource catalog format, QCHK spec, CLI workflow |
| [docs/manuals/DEVELOPMENT.md](docs/manuals/DEVELOPMENT.md) | Build from source, CLI reference, benchmarks, cross-compilation |
| [docs/manuals/flutter-api-reference.md](docs/manuals/flutter-api-reference.md) | Flutter FRB API reference — all exported functions, data types, inference usage |
| [ADRs](docs/manuals/adr/) | Architectural Decision Records |
| [Qapp Vault Developer Guide](docs/manuals/qapp-vault-developer-guide.md) | Build web qapps for the Qapp Vault — manifest spec, daemon API, auth, CORS |
| [AI Instructions](AGENTS.md) | Guidance for AI agents working on this codebase |

---

## License

[Creative Commons Attribution-NonCommercial-NoDerivatives 4.0 International](LICENSE)

For commercial licensing, enterprise integration, or consulting on Intentional Computing:
**Timothy Charles Holborn** · [LinkedIn](https://www.linkedin.com/in/ubiquitous/)

---

*Built to guarantee first-class digital agency.*
