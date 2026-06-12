# Release Targets — Feature Matrix

_Branch: `0.0.11-dev` | Updated: 2026-06-11_

Five release artefacts are built or planned from this repository:

| Artefact | Crate / Path | Delivery |
|---|---|---|
| **WASM (Browser)** | `qualia-core-wasm` (`wasm_bridge.rs`, `wasm_edge.rs`) | `qualia-core-wasm.tar.gz` — drop into any web project |
| **WASM (Mobile PWA)** | `crates/qualia-mobile-harness/` (Dioxus WASM) | PWA — installed via "Add to Home Screen" on any mobile browser; QR-scan bootstrap from desktop |
| **CLI** | `crates/qualia-cli/` | Binary: `qualia-cli`; built via `cargo build --release -p qualia-cli` |
| **Desktop — Qualia Studio** | `crates/qualia-studio/` (Dioxus 0.5 + Shoelace) | Installer: Windows / macOS / Linux via GitHub Releases |
| **Mobile Native** | TBA (likely Flutter mobile) | TBA — iOS / Android; planned for a future milestone |

> **Note — WASM (Mobile PWA):** This is a Dioxus WASM thin-client PWA that runs on a mobile phone and connects back to the user's personal Webizen desktop daemon via WebSocket (port 4242). It does **not** run the graph engine locally. It provides pane-based UI rendering, QR-scan bootstrap, and DID challenge-response pairing. Core graph, inference, and storage features are executed by the daemon and streamed to the mobile UI. Phase C of the Mindware Studio plan (`webizen-platform-plan.md`).

> **Note on Legacy Desktop Prototypes** (`crates/qualia-desktop/` and `crates/qualia-flutter/`): The Tauri/React/NodeJS prototypes are retained in-tree for reference only. The Flutter application is deprecated. All active desktop work has transitioned to the native Dioxus 0.5 / Shoelace target in `crates/qualia-studio/`.

---

## Key

| Symbol | Meaning |
|---|---|
| ✅ | Fully implemented and available in this target |
| ⚠️ | Partial — works but with noted constraints |
| ❌ | Not available in this target |
| 🚧 | Planned / TBA |

---

## Storage Engine

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| 48-byte NQuin semantic data model | ✅ | ❌ | ✅ | ✅ | 🚧 |
| `.q42` v3 volume format (read + write) | ⚠️ Read via OPFS | ❌ | ✅ | ✅ | 🚧 |
| LZ4 SuperBlocks (850 Quins / 40 KB block) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Header-first boot (skip irrelevant blocks) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| `.q42.bidx` demand-paging sidecar | ✅ | ❌ | ✅ | ✅ | 🚧 |
| OPFS auto-cache (daemon blocks → browser cache) | ✅ | ❌ | ❌ | ❌ | 🚧 |
| Write-Ahead Log (WAL, Ed25519-signed mutations) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| WAL → Merkle-DAG checkpoint (`checkpoint_to_dag`) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Merkle-DAG (`DagNode`, `DagStore`, `merge_node`) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| `nodes_as_of(ms)` assertion-time snapshot | ❌ | ❌ | ✅ | ✅ | 🚧 |
| v2 → v3 migration (`migrate_v2_to_v3`) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Memory-mapped query (`mmap_query_subject`) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| In-memory QuinIndex (subject / predicate / object / context) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Volatile buffer scrubbing (`write_volatile` on eviction) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Temporal index fields (`temporal_index_offset/length`) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Merkle root + assertion timestamp in v3 header | ❌ | ❌ | ✅ | ✅ | 🚧 |

---

## Ingest Pipeline

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| Turtle (`.ttl`) | ⚠️ URL/file input | ❌ | ✅ | ✅ Via Ontology Hub | 🚧 |
| N-Triples / N-Quads | ⚠️ URL/file input | ❌ | ✅ | ✅ Via Ontology Hub | 🚧 |
| JSON-LD | ⚠️ URL/file input | ❌ | ✅ | ✅ Via Ontology Hub | 🚧 |
| RDF/XML | ⚠️ URL/file input | ❌ | ✅ | ✅ Via Ontology Hub | 🚧 |
| N3 / N3-Star | ⚠️ URL/file input | ❌ | ✅ | ✅ Via Ontology Hub | 🚧 |
| CBOR-LD (zero-alloc, Q42 lexicon) | ⚠️ URL/file input | ❌ | ✅ | ✅ Via Ontology Hub | 🚧 |
| CogAI `.chk` chunks-and-rules (W3C CG format) | ⚠️ URL/file input | ❌ | ✅ | ✅ Via Ontology Hub | 🚧 |
| RDF-Star / SPARQL-Star embedded triples | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Sort-first ingestor (BIDX-indexable output) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Multi-pass external sorter (datasets > RAM) | ❌ | ❌ | ✅ | ❌ | 🚧 |
| Streaming ingest v3 DAG generation (`DagStore`) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Profile-bound ingest (`--profile <file>.qchk`) | ❌ | ❌ | ✅ | ✅ Via Credential Manager | 🚧 |
| KML geometry ingest → NQuin spatial predicates | ❌ | ❌ | ✅ | ✅ | 🚧 |
| FNV-1a zero-alloc URI hashing (`q_hash`) | ✅ | ❌ | ✅ | ✅ | 🚧 |

---

## SPARQL Engine (138 tests)

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| SELECT / ASK / CONSTRUCT / DESCRIBE | ✅ | ⚠️ Via daemon WS | ✅ | ✅ | 🚧 |
| FILTER / aggregates (COUNT, SUM, AVG, MIN, MAX) | ✅ | ⚠️ Via daemon WS | ✅ | ✅ | 🚧 |
| GROUP BY / HAVING / DISTINCT / LIMIT / OFFSET / ORDER BY | ✅ | ⚠️ Via daemon WS | ✅ | ✅ | 🚧 |
| OPTIONAL / UNION / GRAPH | ✅ | ⚠️ Via daemon WS | ✅ | ✅ | 🚧 |
| Property Paths (7 types: `/`, `\|`, `+`, `*`, `?`, `^`, `!`) | ✅ | ⚠️ Via daemon WS | ✅ | ✅ | 🚧 |
| Subqueries | ✅ | ⚠️ Via daemon WS | ✅ | ✅ | 🚧 |
| SPARQL-Star / RDF-Star (`<< >>` embedded triples) | ✅ | ⚠️ Via daemon WS | ✅ | ✅ | 🚧 |
| SPARQL 1.1 UPDATE (INSERT / DELETE / LOAD / CLEAR) | ⚠️ In-memory only | ❌ | ✅ | ✅ | 🚧 |
| Federated Query (`SERVICE`) | ⚠️ CORS-constrained | ❌ | ✅ | ✅ | 🚧 |
| GeoSPARQL (OGC) spatial functions | ✅ | ⚠️ Via daemon WS | ✅ | ✅ | 🚧 |
| SHACL-SPARQL (`sh:sparql` constraint blocks) | ✅ | ⚠️ Via daemon WS | ✅ | ✅ | 🚧 |
| SPARQL-MM multimedia / time-series windows | ✅ | ⚠️ Via daemon WS | ✅ | ✅ | 🚧 |
| `AS OF "<date>"^^xsd:dateTime` (assertion-time snapshot) | ✅ | ⚠️ Via daemon WS | ✅ | ✅ | 🚧 |
| `AT TIME <ms>` (valid-time point query) | ✅ | ⚠️ Via daemon WS | ✅ | ✅ | 🚧 |
| DID-authenticated federated queries (`sparql_did.rs`) | ⚠️ CORS-constrained | ❌ | ✅ | ✅ | 🚧 |
| SPARQL 1.1 HTTP Protocol endpoint (port 4242) | ❌ Connects to daemon | ✅ Via daemon WS | ✅ Via daemon | ✅ Via daemon | 🚧 |
| SPARQL WebSocket subscription | ⚠️ Connects to daemon | ✅ Via daemon WS | ✅ Via daemon | ✅ Via daemon | 🚧 |
| Zero-allocation query budget (~35 KB per query) | ✅ | ❌ | ✅ | ✅ | 🚧 |

---

## Webizen VM — Logic Modalities

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| SlgArena (42 MB, 917,504 Quin slots) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| WASM SIMD vectorised execution (`wasm_simd`) | ✅ | ❌ | ❌ N/A | ❌ N/A | 🚧 |
| Deontic Logic — Obligate / Permit / Forbid (0x10–0x12) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Deontic Defeater rules (`^>`, `DEFEATER_BIT`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| ODRL policy evaluation (Permission / Prohibition quins) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Epistemic Logic — Knows / Believes / Common Knowledge (0x20–0x22) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Paraconsistent Logic — isolation without system halt (0x30–0x32) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Linear Temporal Logic — G / F / X / U / R trace evaluation (0x40–0x44) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Answer Set Programming — up to 8 stable models (zero-alloc) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Description Logic — `rdfs:subClassOf` TBox subsumption | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Linear Logic — resource consumption tombstone (`CONSUMED_BIT`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Dialectical Logic — thesis / antithesis / synthesis (`SYNTHESIZED_BIT`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Spatio-Temporal Logic — Allen Interval Algebra (7 relations) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| N3 → Deontic Bridge (`compile_n3_rule_to_norm`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| SHACL compiler → WebizenOpcode bytecode | ✅ | ❌ | ✅ | ✅ | 🚧 |
| CRDT — Lamport LWW conflict resolution | ✅ | ❌ | ✅ | ✅ | 🚧 |
| CRDT — M:N deontic contract ratification queue (32 slots) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Bi-temporal graph (`temporal_graph.rs`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| PROV-O provenance quins (`provenance.rs`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Credential-gated subgraphs (AES-256-GCM + HKDF + X25519) | ⚠️ In-memory | ❌ | ✅ | ✅ | 🚧 |
| CogAI / ACT-R opcodes (retrieve, decay, unless) | ✅ Complete | ✅ | ✅ Complete | ✅ Complete | ✅ |

---

## SHACL Extension Modules (149 tests)

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| Deontic SHACL constraints (Obligate / Permit / Forbid / NotExpired) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Epistemic SHACL constraints (Knowledge / Belief / CommonKnowledge) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Biosciences engine — gene ontology, sequence annotation | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Biomedical engine — SNOMED CT, MeSH, ICD-10 | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Organic chemistry — SMILES/InChI, Lipinski/Veber/Ghose/Egan/pKa | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Bioinformatics — Smith-Waterman (AVX2/NEON/scalar), k-mer, Tanimoto | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Clinical engine — Framingham, CHA₂DS₂-VASc, SOFA, FHIR/LOINC/RxNorm | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Thermodynamics — MCMC Metropolis–Hastings, Gibbs, Boltzmann | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Quantum DFT — ground-state energy, PINN receptor binding affinity | ✅ | ❌ | ✅ | ✅ | 🚧 |
| ODE Solver — Runge-Kutta 4th-order | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Geometric / Geometric Algebra — Lorentz, tropical distance, SIMD kernel | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Spatial Sieve (NETS — Non-Euclidean Tropical Sieve, GPU-accelerated) | ⚠️ CPU fallback | ❌ | ✅ | ✅ | 🚧 |
| Financial — time-value of money, portfolio risk | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Geospatial — GeoSPARQL extension functions, WKT geometry | ✅ | ❌ | ✅ | ✅ | 🚧 |

---

## LLM Inference

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| GGUF v2/v3 model loading (`gguf_bridge.rs`, `gguf_sharder.rs`) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| `GgufTokenizer` — greedy longest-match encode, SentencePiece decode | ❌ | ❌ | ✅ | ✅ | 🚧 |
| DirectML 1.15 GPU inference (Windows, Q4_K GEMM) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Accelerate / AMX inference (macOS Apple Silicon) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| wgpu / Vulkan inference — `fused_tensor_contraction.wgsl` (Linux) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| `infer_local_model()` — real Phase 8 autoregressive decode loop | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Phase 8 bifurcated compute (SPSC ring buffers, Sentinel mid-generation rollback) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Zero-Copy LoRA Multiplexing (up to 16 concurrent adapters) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| LoRA GPU shader (`lora_projection.wgsl`) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Local / Remote / Hybrid backend modes | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Remote inference via Nym mixnet (ILP metered) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Resource catalog — LLM model download pipeline (YAML → WAL) | ❌ | ❌ | ✅ | ✅ Via LLM Hub | 🚧 |
| LLM Hub UI (grid/list, bulk actions, download state) | ❌ | ❌ | ❌ | ✅ | 🚧 |
| Inference results streamed to mobile PWA via WebSocket | ❌ | ✅ Streamed | ❌ | ✅ Daemon | 🚧 |
| WASM mock inference path (ring-buffer stub) | ✅ | ❌ | ❌ | ❌ | 🚧 |

---

## Fiduciary / Governance Layer

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| `AgentIntent` + `WebizenVerdict` (5 outcomes) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Seven LLM fiduciary rules | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Adversarial conduct WAL log (DID-associated, Ed25519 auditable) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| `McpIntentFrame` mediation + sanctuary gate | ✅ | ❌ | ✅ | ✅ | 🚧 |
| MCP `enforce_fiduciary_tool_dispatch` | ✅ | ❌ | ✅ | ✅ | 🚧 |
| `TaskOrchestrator` (pre-validate → infer → post-validate) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| N3 Rights Ontology pre-flight (`validate_intent`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Capability Profiles (QCHK binary, 6 named profiles) | ✅ | ⚠️ Enforced by daemon | ✅ | ✅ | 🚧 |
| `profile compile / list / inspect` | ❌ | ❌ | ✅ | ✅ Via Credential Manager | 🚧 |
| ECC parity (real P-256 scalar validation) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| FiduciaryCrypto sign / verify (ed25519-dalek) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| ZK structural validation (Pedersen commitment check) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Full ZK proof backend (bellman / arkworks) | ❌ Pending | ❌ | ❌ Pending | ❌ Pending | 🚧 |
| ML-DSA (FIPS 204 compliant) | ❌ Pending | ❌ | ❌ Pending | ❌ Pending | 🚧 |

---

## Identity Credentials, Verifiable Credentials, Verifiable Claims (VCs) & Decentralised Identifiers (DIDs)

W3C standards: [Decentralised Identifiers (DIDs) v1.0](https://www.w3.org/TR/did-core/), [Verifiable Credentials Data Model](https://www.w3.org/TR/vc-data-model/), [Verifiable Claims](https://www.w3.org/TR/verifiable-claims-data-model/).

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| `did:q42` identifier parsing + topological pointers | ✅ | ❌ | ✅ | ✅ | 🚧 |
| `did:q42` resolution (`resolver.rs`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| `did:web` resolution | ✅ | ❌ | ✅ | ✅ | 🚧 |
| `did:key` resolution | ⚠️ Partial | ❌ | ⚠️ Partial | ⚠️ Partial | 🚧 |
| DID Document (DDO) generation + serialisation | ❌ | ❌ | ✅ | ✅ | 🚧 |
| DID:GIT staged axiomatic evolution | ❌ | ❌ | ✅ | ✅ | 🚧 |
| `webizen init` — generate Webizen identifier material | ❌ | ❌ | ✅ | ✅ | 🚧 |
| `webizen validate-gitmark` | ❌ | ❌ | ✅ | ✅ | 🚧 |
| `webizen dns-frontdoor` — `did:web` + DNS TXT records | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Verifiable Credentials (W3C VC Data Model v2) — Principal-signed | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Verifiable Claims — claims encoded as NQuin subject/predicate/object triples | ✅ | ❌ | ✅ | ✅ | 🚧 |
| VC issuance — Ed25519 proof suite (`fiduciary_crypto.rs`) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| VC issuance — ML-DSA post-quantum proof suite | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| VC presentation + verification | ❌ | ❌ | ✅ | ✅ | 🚧 |
| VC selective disclosure (ZK proof over claims) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Credential Manager UI | ❌ | ❌ | ❌ | ✅ | 🚧 |
| Sub-agent DID derivation (`did:qualia:subagent:…`) | ❌ | ❌ | ✅ | ✅ Via Chat | 🚧 |
| Multi-seed identity credentials (BTC, XEC, Nym, EVM, XMR) | ❌ | ❌ | ✅ | ✅ Via Wallet | 🚧 |
| QCHK capability profile — VC-bound capability grants | ✅ | ⚠️ Enforced by daemon | ✅ | ✅ | 🚧 |
| `derive_lane_key` (SHA256; PBKDF2 planned) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Merkle root over agent-scoped Quins | ✅ | ❌ | ✅ | ✅ | 🚧 |
| QR-scan bootstrap + DID challenge-response pairing | ❌ | ✅ | ❌ | ✅ Generates QR | 🚧 |

### W3C Solid Protocol & WebID Interoperability

Solid support in Qualia/Webizen is a **backwards-compatibility and data-portability layer**, not a goal in itself. Three use cases drive it:

1. **Ecosystem federation** — Webizen users can network with people whose organisations use Solid (enterprises, universities, and public institutions are more likely to deploy Solid than Webizen). Solid users can exchange data with Webizen users, though they will not have access to Qualia's full capability set (native graph inference, SPARQL-Star provenance, governance VM, SocialWebNet routing, etc.).
2. **Institutional reach** — large organisations deploying Solid pods can be addressed as participants in the same semantic web without needing to adopt Webizen. The common substrate is standard RDF, WebID, and Linked Data.
3. **User data-portability and exit rights** — a user who chooses to stop using Webizen/QualiaDB can export their semantic graph to any W3C Solid pod provider and continue from there with standard Solid tooling. No data lock-in.

CG specifications: [Solid Protocol v0.11](https://solidproject.org/TR/protocol) · [Web Access Control](https://solidproject.org/TR/wac) · [Solid-OIDC](https://solidproject.org/TR/oidc) · [Solid WebID Profile](https://solid.github.io/webid-profile/) · [Solid Notifications Protocol](https://solidproject.org/TR/notifications-protocol) · [Solid Application Interoperability](https://solidproject.org/TR/sai) · [Solid DID Method](https://solid.github.io/did-method-solid/) · [WebID 1.0](https://www.w3.org/2005/Incubator/webid/spec/identity/)

| Feature | Use case | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|---|:---:|:---:|:---:|:---:|:---:|
| Solid LDP Basic Container export — `data.ttl` + `data.ttl.acl` (`solid_ldp.rs`, `export-solid`) | Exit / portability | ❌ | ❌ | ✅ | ⚠️ Future | 🚧 |
| WAC `.acl` rules from NQuin routing lanes (public / owner-only) | Exit / portability | ❌ | ❌ | ✅ | ⚠️ Future | 🚧 |
| Inbound Solid Pod import (LDP Turtle → `.q42` ingest) | Federation / entry | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| WebID profile URI → `webid_hash` FNV-1a in `WebizenId` (`webizen_identifiers.rs`) | Identity bridge | ✅ | ❌ | ✅ | ✅ | 🚧 |
| `IdentityRegistry` WebID URI → WebizenId reverse-lookup | Identity bridge | ✅ | ❌ | ✅ | ✅ | 🚧 |
| WebID Profile document generation (`foaf:Agent`, `pim:storage`, `solid:oidcIssuer`) | Identity bridge | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| `did:solid` resolution (specialisation of `did:web` via Solid server registry) | Identity bridge | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Solid-OIDC authentication (Auth Code + PKCE + DPoP tokens + `webid` scope) | Federation auth | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Solid Notifications subscription (receive updates from a Solid pod) | Federation / live sync | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| LDN (Linked Data Notifications) receiver — `ldp:inbox` | Federation / messaging | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Solid Application Interoperability (SAI) — access grants across pods | Federation (advanced) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| WebID-TLS (legacy; transport superseded by SocialWebNet) | Legacy compat | ⚠️ | ❌ | ⚠️ | ⚠️ | 🚧 |

> **Functionality asymmetry:** A Solid user connecting to a Webizen user's data will receive standard RDF (Turtle) with WAC access control — the common Solid baseline. They will not have access to Qualia's SPARQL-Star provenance, governance VM, NQuin semantic bit-packing, SocialWebNet routing, or LLM inference. A Webizen user reading a Solid pod receives a flat RDF graph which is ingested as Quins — full Qualia query and inference capabilities apply to that data once imported.
>
> **Currently implemented:** `SolidExporter::export_to_solid_pod()` (`solid_ldp.rs`) — file-based export to LDP Basic Container (`data.ttl` via `rio_turtle` + `data.ttl.acl` WAC rules). Invoked via `qualia export-solid`. Qualia does not yet act as a live Solid HTTP server.

---

## Ontology & Vocabulary

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| Resource catalog — ontology download + SHACL-validate + ingest | ❌ | ❌ | ✅ | ✅ Via Ontology Hub | 🚧 |
| Ontology Hub UI (browse, import, namespace view) | ❌ | ❌ | ❌ | ✅ | 🚧 |
| WebTorrent seeding of ontology artefacts | ❌ | ❌ | ✅ `webizen seed-webtorrent` | ✅ Via Ontology Hub | 🚧 |
| Magnet URI builder for `.c.q42` files | ❌ | ❌ | ✅ | ✅ | 🚧 |
| SKOS concept scheme quins | ✅ | ❌ | ✅ | ✅ | 🚧 |
| PROV-O temporal / provenance vocabulary | ✅ | ❌ | ✅ | ✅ | 🚧 |
| GeoSPARQL + KML spatial vocabulary | ✅ | ❌ | ✅ | ✅ | 🚧 |
| ODRL rights vocabulary | ✅ | ❌ | ✅ | ✅ | 🚧 |
| W3C CogAI CG agent-structure vocabulary | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Dublin Core terms | ✅ | ❌ | ✅ | ✅ | 🚧 |
| CBOR-LD with embedded Q42 lexicon (zero-alloc, offline) | ✅ | ❌ | ✅ | ✅ | 🚧 |

---

## QPU Dispatch

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| IBM Quantum | ❌ | ❌ | ✅ | ✅ | 🚧 |
| D-Wave | ❌ | ❌ | ✅ | ✅ | 🚧 |
| IonQ | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Rigetti | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Azure Quantum | ❌ | ❌ | ✅ | ✅ | 🚧 |
| AWS Braket | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Google Quantum AI | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Quantinuum | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Principal consent commitment activation | ❌ | ❌ | ✅ | ✅ | 🚧 |
| QPU job provenance quins (WAL-logged) | ❌ | ❌ | ✅ | ✅ | 🚧 |

---

## Networking & P2P

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| Daemon HTTP API (port 4242 — `/health`, `/query`, SPARQL endpoint) | ❌ Connects to daemon | ✅ Via WebSocket | ✅ daemon start/stop | ✅ Auto-started | 🚧 |
| `/chat/publish` + `/chat/pull` relay | ❌ | ✅ Via WebSocket | ✅ | ✅ | 🚧 |
| libp2p sync (TCP + Noise + Yamux) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Nym mixnet adapter (`nym_adapter.rs`) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| IPFS publish (`webizen publish-ipfs`) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| WebTorrent HTTP web-seed server | ❌ | ❌ | ✅ | ✅ | 🚧 |
| ILP micropayment metering (remote inference) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| HCAI DNS Frontdoor | ❌ | ❌ | ✅ | ✅ | 🚧 |
| **SocialWebNet** — DNSSEC → WireGuard peer bootstrap (`daemon_swarm.rs`) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| SocialWebNet tunnel establishment (DID-derived WG pubkeys) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| DID → WireGuard pubkey resolution via DNSSEC TXT/CERT records | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Userspace WireGuard proxy (SOCKS5 on 127.0.0.1:1080) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| QR-scan bootstrap + Private Network Access headers | ❌ | ✅ | ❌ | ✅ Serves QR | 🚧 |
| PWA manifest + service worker generation (offline mobile) | ❌ | ✅ | ❌ | ✅ Generates | 🚧 |

---

## Solver Library (`solvers/`)

Zero-allocation, fixed-size stack solvers for physics, math, and AI applications.

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| Runge-Kutta 4th-order ODE solver (`RungeKutta4Static`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Shooting method BVP solver (`ShootingMethodBVP`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Chunked Simpson's integrator (`SimpsonsIntegratorChunked`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Fixed Lanczos eigensolver (`FixedLanczosEigensolver`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Static LU decomposition (`StaticLuDecomposition`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Fixed 4×4 tensor contraction (`ConstTensorContractor`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Nelder-Mead simplex optimizer (`NelderMeadSimplex`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Newton-Raphson root finder (`BoundedNewtonRaphson`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Levenberg-Marquardt curve fitter (`LevenbergMarquardtStack`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| QAOA angle optimizer (`QAOAAngleOptimizer`) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| SPSA gradient estimator (`SpsaOptimizer`) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Defeasible forward chaining (`ForwardChainingDefeasible`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| DPLL-based bounded SAT solver (`BoundedSatSolver`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| QPU job formulation + dispatch queue (8 providers) | ❌ | ❌ | ✅ | ✅ | 🚧 |

---

## Geometric Algebra (`geometric_algebra/`)

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| Multivector operations (grade 0–4) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Geometric / outer / inner products | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Reverse / grade involution / conjugate | ✅ | ❌ | ✅ | ✅ | 🚧 |
| SIMD kernel (AVX2/NEON/WASM-SIMD) — `simd_kernel.rs` | ⚠️ WASM-SIMD | ❌ | ✅ | ✅ | 🚧 |
| Lorentz boost, spacetime norm | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Tropical distance metric (NETS integration) | ✅ | ❌ | ✅ | ✅ | 🚧 |

---

## Hardware-Sympathetic Storage

Low-level storage backends for NVMe and computational storage devices.

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| ZNS zone allocation / write / read / flush / reset (`zns_storage.rs`) | ❌ | ❌ | ✅ Linux | ✅ Linux | 🚧 |
| Zero-copy zone buffer (`ZeroCopyBuffer`) | ❌ | ❌ | ✅ Linux | ✅ Linux | 🚧 |
| CSD task dispatch (`csd_storage.rs`) | ❌ | ❌ | ✅ Linux | ✅ Linux | 🚧 |
| IO scheduler with priority queue (`IoScheduler`) | ❌ | ❌ | ✅ | ✅ | 🚧 |

---

## eBPF Allocation Firewall (`ebpf_firewall.rs`)

Kernel-level socket policy enforcement (Linux only).

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| eBPF program load / attach / detach | ❌ | ❌ | ✅ Linux | ✅ Linux | 🚧 |
| Socket filter rules (Allow / Deny / Redirect / Modify / Log) | ❌ | ❌ | ✅ Linux | ✅ Linux | 🚧 |
| Traffic analysis + anomaly detection | ❌ | ❌ | ✅ Linux | ✅ Linux | 🚧 |
| Rate limiting per socket | ❌ | ❌ | ✅ Linux | ✅ Linux | 🚧 |
| Program metrics (execution time, packet counts) | ❌ | ❌ | ✅ Linux | ✅ Linux | 🚧 |

---

## Acoustic & BLE Mesh (`acoustic_ble_mesh.rs`)

Off-grid, air-gap-crossing communication via audio and BLE.

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| Acoustic protocol stack (physical / data-link / network / transport) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Acoustic modulation (OOK / FSK / CHIRP / Ultrasonic / Subsonic) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Multi-hop acoustic mesh routing | ❌ | ❌ | ✅ | ✅ | 🚧 |
| BLE mesh manager (provisioning / configuration / messaging) | ❌ | ⚠️ Via phone BLE | ✅ | ✅ | 🚧 |
| BLE mesh network formation + message relay | ❌ | ⚠️ Via phone BLE | ✅ | ✅ | 🚧 |
| Message priority queue (per `MessagePriority`) | ❌ | ❌ | ✅ | ✅ | 🚧 |

---

## Ambient Sub-Threshold Orchestration (`ambient_orchestration.rs`)

Background orchestration that operates below user-perceptible compute thresholds.

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| Task scheduling with priority levels | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Thermal / CPU headroom governor integration | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Daemon-linked background inference tasks | ❌ | ❌ | ✅ | ✅ | 🚧 |

---

## Neuro-Symbolic Sieve

Grammar-constrained FSM token-filtering over LLM logit output.

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| Neuro-symbolic sieve (`neurosymbolic_sieve.rs`) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Grammar → FSM compilation (zero-heap) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Live token filtering during Phase 8 decode | ❌ | ❌ | ✅ | ✅ | 🚧 |

---

## Webizen Identity & Cryptokey Routing

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| `webizen_identifiers.rs` — Webizen identity lifecycle | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Ed25519 key generation + signing | ✅ | ❌ | ✅ | ✅ | 🚧 |
| `derive_webizen_ipv6` — DID → IPv6 (Ed25519 key-derived) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Cryptokey routing via IPv6 address space (`web_civics.rs`) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Webizen bytecode VM (`webizen_bytecode.rs`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Principal credential validation + consent flag | ✅ | ⚠️ Delegated to daemon | ✅ | ✅ | 🚧 |

---

## Obfuscation Module (`obfuscation/`)

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| Identifier / literal obfuscation | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Reversible obfuscation with key material | ✅ | ❌ | ✅ | ✅ | 🚧 |

---

## DICOM Integration

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| DICOM tag parsing → NQuin triples | ⚠️ In-memory | ❌ | ✅ | ✅ | 🚧 |
| DICOM study / series / instance graph | ⚠️ In-memory | ❌ | ✅ | ✅ | 🚧 |
| DICOM → FHIR bridge (imaging study resource) | ❌ | ❌ | ✅ | ✅ | 🚧 |

---

## Comorbidity Evaluation

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| Multi-condition interaction scoring | ✅ | ❌ | ✅ | ✅ | 🚧 |
| ICD-10 comorbidity graph queries | ✅ | ❌ | ✅ | ✅ | 🚧 |
| CHA₂DS₂-VASc + SOFA integration | ✅ | ❌ | ✅ | ✅ | 🚧 |

---

## Specialized Libraries (`specialized_libs/`)

These libraries have additional build dependencies and are conditionally compiled.

| Library | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native | Status |
|---|:---:|:---:|:---:|:---:|:---:|---|
| `bioinformatics` — Smith-Waterman, k-mer, Tanimoto | ✅ | ❌ | ✅ | ✅ | 🚧 | Compiled via SHACL engine |
| `organic_chemistry` — SMILES/InChI, Lipinski/Veber | ✅ | ❌ | ✅ | ✅ | 🚧 | Compiled via SHACL engine |
| `thermodynamics` — MCMC, Metropolis–Hastings | ✅ | ❌ | ✅ | ✅ | 🚧 | Compiled via SHACL engine |
| `financial` — TVM, portfolio risk | ✅ | ❌ | ✅ | ✅ | 🚧 | Compiled via SHACL engine |
| `geospatial` — WKT, GeoSPARQL extensions | ✅ | ❌ | ✅ | ✅ | 🚧 | Compiled via SHACL engine |
| `geometric` — SIMD geometric algebra kernel | ✅ | ❌ | ✅ | ✅ | 🚧 | Compiled via domains |
| Extended biosciences / biomedical | ✅ | ❌ | ✅ | ✅ | 🚧 | Via `specialized_libs/` (some deps pending) |

---

## Desktop-Only Features (Webizen)

These features exist exclusively in the Flutter desktop release:

| Feature | Notes |
|---|---|
| Dashboard screen | Usage overview, daemon health, active models |
| Chat screen (Neuro-Chat) | LLM inference with live loading indicator, full `TaskOrchestrator` pipeline |
| Group Chat + Sub-agents | Cooperative multi-LLM sessions; participant management; session DIDs |
| Chat → Qapp handoff | `launchInstalledQappWithContext` |
| Wallet screen | Multi-seed credential management (BTC, XEC, Nym, EVM, XMR) |
| Address Book | Contact / DID management |
| LLM Hub | Download, activate, manage GGUF models; bulk actions; persistent state |
| Ontology Hub | Browse, import, share, WebTorrent seed ontologies |
| Qapp Vault | Install, list, launch qapps; loopback HTTP serving; `QualiaQappWebView` |
| Credential Manager | QCHK capability profile binding; DID session management |
| Spatial Physics screen | Spatial / physics visualisation |
| Settings | Daemon config, GPU backend, Principal identity, network preferences |
| KaTeX mathematical rendering | LaTeX in chat responses |
| FRB bridge (`qualia_api.rs`) | Direct Rust ↔ Flutter bindings for inference, vault, daemon, resources |
| QR code generation (`GET /mobile/qr`) | Serves pairing QR for mobile PWA bootstrap |
| Mindware Studio canvas | Dioxus WASM pane-composer served on port 8080; SSE telemetry; manifest deploy |

---

## Mobile PWA Features (WASM Mobile)

These features exist in the `qualia-mobile-harness` PWA target; all graph/inference execution is delegated to the desktop daemon:

| Feature | Notes |
|---|---|
| QR-scan bootstrap | Camera access via `web_sys`; scans desktop QR, redirects to LAN daemon address |
| DID challenge-response pairing | Desktop sends challenge; phone signs with mobile DID (WebAuthn/Passkeys); pipeline unlocked on success |
| "Add to Home Screen" PWA install | Dynamically generated `manifest.json` + `sw.js`; offline-capable shell |
| Pane canvas rendering | Dioxus WASM UI; panes map to `rdf:type` via `PaneRegistry` |
| WebSocket data pipe (port 4242) | Live streaming of SPARQL results, LLM inference tokens, telemetry from daemon |
| Private Network Access headers | `Access-Control-Allow-Private-Network: true`; enables mobile browser → local daemon requests |
| Sensitivity gate | `0x02` Classified data never forwarded without explicit Guardianship override (enforced by daemon egress gatekeeper) |

---

## CLI-Only Features

These features exist exclusively in the CLI release:

| Command | Notes |
|---|---|
| `bench` / `benchmark` | Full benchmark suite; point / two-hop / filter latency |
| `dump` | Stream-dump raw Quins from a `.q42` file |
| `compress` | Compress an existing `.q42` file |
| `export-solid` | Export graph to W3C Solid Pod |
| `webizen validate-gitmark` | Validate git commit cryptographic mark |
| `webizen dns-frontdoor` | Generate `did:web` + DNS TXT zone records |
| `resources import-ontology <id>` | Multi-step download + validate + ingest pipeline |
| Multi-pass external sorter | For datasets that exceed available RAM |

---

## SocialWebNet — Socially-Defined Networking Protocol

`daemon_swarm.rs` implements a bootstrapping pipeline that derives encrypted peer-to-peer tunnels from a user's social graph and DID infrastructure, rather than from manual key exchange:

1. **DNSSEC resolution** — Each peer publishes their WireGuard public key as a DNSSEC-signed TXT/CERT record associated with their DID domain
2. **Semantic payload** — The DNSSEC record contains a CBOR-LD encoded `DnssecSemanticPayload` (`did`, `wireguard_pubkey [32]`, `ipv6_address`, `service_endpoints[]`)
3. **Tunnel establishment** — `establish_wireguard_tunnel(peer_payload, endpoint, port)` configures a WireGuard peer using the socially-resolved key — no out-of-band key sharing required
4. **Userspace proxy** — A SOCKS5 proxy on `127.0.0.1:1080` routes traffic through the WireGuard interface
5. **DID-locked access** — All peer connections are gated by DID challenge-response; the social graph defines who can connect, not a static ACL

The result: connectivity to a peer = knowing their DID. Trust is anchored in the same decentralised identity fabric that governs the rest of the Webizen platform.

---

## WASM (Browser) Constraints Summary

The WASM (Browser) target runs entirely in-browser with no daemon dependency when used standalone. Key constraints versus the CLI/Desktop targets:

- **No GPU inference** — GGUF model weights cannot be memory-mapped or dispatched to a GPU from a browser sandbox; the mock ring-buffer path is used
- **No WAL writes** — the WAL requires a writable filesystem; WASM queries operate on read-only OPFS-cached blocks
- **No Merkle-DAG** — DAG operations require WAL; unavailable in WASM
- **No file system access** — ingest is limited to URL fetch or `<input type="file">` uploads
- **Federated queries (SERVICE) CORS-constrained** — remote SPARQL endpoints must set `Access-Control-Allow-Origin`; DID-authenticated federation similarly limited
- **271-test browser suite** — the WASM build ships a full test harness covering WASM / Native / Both execution modes (see `docs/api-explorer/`)
- **WASM SIMD** — compiled with `-C target-feature=+simd128` when the `wasm_simd` feature is enabled; provides vectorised Webizen VM execution paths

## WASM (Mobile PWA) Constraints Summary

The mobile PWA is a **thin UI client** — it does not bundle `qualia-core-db`. All graph engine, inference, and storage features are executed by the Webizen desktop daemon and streamed to the phone over a DID-authenticated WebSocket. Key constraints:

- **No local graph engine** — no NQuin processing, no SPARQL evaluation, no Webizen VM on the phone
- **No local LLM inference** — token generation runs on the desktop GPU; tokens are streamed to the mobile UI
- **Daemon required** — the phone must be on the same local network as the desktop, or reach it via SocialWebNet tunnel
- **Sensitivity enforcement is remote** — the desktop daemon's egress gatekeeper (`context >> 56`) enforces classification; `0x02` Classified data is never forwarded to the phone without Guardianship override
- **Phase C work** — `qualia-mobile-harness` crate is in planning/early implementation; some features listed as ✅ are specified but not yet fully built

---

## Test Coverage by Target

| Suite | Count | Primary Target |
|---|---|---|
| SPARQL engine | 138 | All (WASM Browser + CLI + Desktop via shared `qualia-core-db`) |
| SHACL extension modules | 149 | All |
| git_bridge / Merkle-DAG | 8 | CLI + Desktop |
| Browser test suite | 271 | WASM Browser |
| Core + domains + other | ~74 | All |
| **Total** | **640+** | |

---

## Distributed Data Consistency (CRDTs)

`crdt.rs` provides the base CRDT types. The Dotted Version Vector + Epoch-Based Anti-Entropy (DVV+EAE) enhancement that guarantees O(1) memory regardless of runtime is specified and planned.

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| OR-Set CRDT, LWW map, 2P-Set (existing `crdt.rs`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Lamport LWW conflict resolution (M:N deontic contracts) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Nym mixnet CRDT sync (`webizen_sync.rs`) | ❌ | ❌ | ✅ | ✅ | 🚧 |
| Dotted Version Vectors (DVV) — O(1) tombstone tracking | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Epoch-Based Anti-Entropy (EAE) — tombstone GC + sealed epochs | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| CRDT memory controller (512MB budget enforcement) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Sentinel CRDT gate (anomaly detection on sync operations) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |

---

## Spatiotemporal Fractal Indexing

`modalities/spatio_temporal.rs` implements Allen Interval Algebra relations. The Z-Order Morton Code indexing for zero-allocation multidimensional spatial-temporal queries is specified and planned.

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| Allen Interval Algebra (7 relations) via `spatio_temporal.rs` | ✅ | ❌ | ✅ | ✅ | 🚧 |
| GeoSPARQL / KML spatial predicates | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Z-Order Morton Code encoder/decoder (21-bit lat/lon + 22-bit time) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| BMI2 SIMD-accelerated Morton encoding (`_pdep_u64`) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| `SpatiotemporalIndex` — zero-allocation sorted Morton array | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Range query via Morton code binary search | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Nearest-neighbour query (Haversine + time distance) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| `.bidx` block-level Morton sampling (fast range prefetch) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |

---

## WASI Extension Ecosystem (Planned)

Capability-based sandboxing via WebAssembly Component Model (WASI Preview 3) for third-party Qapp development. No implementation yet — spec at `local/architectural-enhancements/WASI_Component_Model_Implementation_Spec.md`.

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| WIT interface types for NQuin / capability handles | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| `ComponentManager` — WASM component load / instantiate | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Capability registry — time-limited, usage-limited, Ed25519-signed | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Per-component 100 MB memory limit + `MemoryGuard` | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| `SecurityAuditor` — capability / network / file violation detection | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| `QappManifest` format (required capabilities, security level) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Domain-specific component interfaces (anatomy, math, physics) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |

---

## Post-Quantum Cryptography

`fiduciary_crypto.rs` has the existing FiduciaryCrypto sign/verify (Ed25519). ML-DSA (FIPS 204) and full Halo2 zk-SNARKs are partially implemented or planned.

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| Ed25519 sign / verify (`fiduciary_crypto.rs`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| Pedersen commitment ZK validation (`zk_proofs.rs`) | ✅ | ❌ | ✅ | ✅ | 🚧 |
| ML-DSA (Module Lattice-Based Digital Signature, FIPS 204) | ⚠️ Partial | ❌ | ⚠️ Partial | ⚠️ Partial | 🚧 |
| ML-DSA: 2560-byte public key / 4627-byte signature storage in Quins | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| zk-SNARK semantic proofs (`zk_proofs.rs` Halo2 backend) | ⚠️ Structural only | ❌ | ⚠️ Structural only | ⚠️ Structural only | 🚧 |
| zk-SNARK circuit compiler (Quin predicate → arithmetic circuit) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Privacy-preserving inference proofs (ZK proof of LLM output range) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |

---

## Cryptographic Halo — FHE over WebGPU (Phase 3, Planned)

Fully Homomorphic Encryption (BFV/BGV scheme) GPU-accelerated via `wgpu` compute shaders, enabling computation on encrypted Quins without decryption. Spec at `local/architectural-enhancements/Cryptographic_Halo_Implementation_Spec.md`.

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| FHE key generation (BFV/BGV) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Encrypted Quin arithmetic (add / multiply on ciphertexts) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| WebGPU FHE compute shader (100-1000x vs CPU) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| FHE circuit compiler (semantic query → encrypted computation) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Blind fiduciary compute (reasoning without data exposure) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |

---

## Unforgeable Agency — TEE Biometric (Phase 3, Partial Groundwork)

`tee_ffi.rs` provides the FFI bindings to Intel SGX / ARM TrustZone / AMD SEV. Biometric-cryptographic binding and enclave-resident identity verification are planned.

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| TEE FFI bindings (SGX / TrustZone / SEV) — `tee_ffi.rs` | ❌ | ❌ | ⚠️ FFI only | ⚠️ FFI only | 🚧 |
| Secure enclave key generation + signing | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Biometric template → cryptographic anchor binding | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| TEE-resident DID key operations (fingerprint / face / voice) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Non-repudiation proof generation in enclave | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Continuous ambient attestation (time-bound proofs) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |

---

## Intermittent Computing — NVM Snapshots (Phase 4, Planned)

Microsecond volatile-to-NVM snapshots of the entire 42 MB `SlgArena` + CPU register state triggered on power-loss interrupt, enabling exact instruction resumption. Spec at `local/architectural-enhancements/Intermittent_Computing_Implementation_Spec.md`.

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| Power-loss interrupt handler (`SnapshotEngine`) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| CPU register capture (`CpuRegisters` struct, <1 µs) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| 42 MB arena snapshot to NVM/MRAM (<100 µs) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Checksum-verified state restoration on boot | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Sentinel VM exact bytecode resumption | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| `CrisisLogger` emergency buffer with protection levels | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Battery / thermal / impact monitoring triggers | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |

---

## Spatial Web Anchoring — UWB & VPS (Phase 4, Planned)

Physical-space cryptography: a 3D point-cloud hash of a room becomes the encryption key for Quins, creating "digital dead drops" that can only be decrypted when an authorized user physically occupies the location. Spec at `local/architectural-enhancements/Spatial_Web_Anchoring_Implementation_Spec.md`.

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| UWB ranging → spatial anchor coordinates | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| VPS point-cloud feature extraction (camera input) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Spatial hash generation (room geometry → 256-bit key) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Spatially-gated Quin encryption / decryption | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| GPS-free offline anchoring (no external infrastructure) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Digital dead drop creation / retrieval | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |

---

## Formal Safety Verification (Phase 4, Planned)

Machine-checked mathematical proofs (Coq / LEAN theorem provers) of Sentinel VM state-transition correctness and zero-allocation invariants. Spec at `local/architectural-enhancements/Formal_Verification_Implementation_Spec.md`.

| Feature | WASM (Browser) | WASM (Mobile PWA) | CLI | Desktop (Webizen) | Mobile Native |
|---|:---:|:---:|:---:|:---:|:---:|
| Coq/LEAN model of Sentinel VM state machine | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Proof: SENSITIVITY_CLASSIFIED Quins never route to Public Commons | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Proof: zero-allocation invariant (no heap in hot paths) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Proof: 42 MB arena ceiling never exceeded | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Proof: fiduciary deontic rules always halt | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |
| Legal recognition artefact (formal certification document) | 🚧 | ❌ | 🚧 | 🚧 | 🚧 |

---

## Architectural Enhancement Roadmap

Full 16-enhancement roadmap from `local/architectural-enhancements/`. Each enhancement is categorised by phase, implementation status in the current codebase, and primary release targets.

| ID | Enhancement | Phase | Code Status | Primary File(s) | Target |
|---|---|:---:|---|---|---|
| 128 | Zero-Copy LoRA Multiplexing | 1 | ✅ Implemented | `lora/mod.rs`, `lora/webgpu_lora.rs` | CLI, Desktop |
| 119 | O(1) Memory CRDTs (base) | 1 | ⚠️ Partial | `crdt.rs` (OR-Set / LWW); DVV+EAE planned | All |
| 125 | Spatiotemporal Fractal Indexing (base) | 1 | ⚠️ Partial | `modalities/spatio_temporal.rs`; Morton codes planned | All |
| 133 | WASI Component Model | 1 | 🚧 Spec only | — | WASM, Desktop |
| 120 | Hardware-Sympathetic Storage (ZNS) | 2 | ✅ Implemented | `zns_storage.rs` | CLI, Desktop (Linux) |
| 127 | NVMe Computational Storage Pushdown | 2 | ✅ Implemented | `csd_storage.rs` | CLI, Desktop |
| 126 | Allocation Firewall (eBPF) | 2 | ✅ Implemented | `ebpf_firewall.rs` | CLI, Desktop (Linux) |
| 124 | Zero-Infrastructure Acoustic & BLE Mesh | 2 | ✅ Implemented | `acoustic_ble_mesh.rs` | CLI, Desktop |
| 122 | Ambient Sub-Threshold Orchestration | 2 | ✅ Implemented | `ambient_orchestration.rs` | CLI, Desktop |
| 121 | Fiduciary Cryptography (ML-DSA) | 2 | ⚠️ Partial | `fiduciary_crypto.rs` (Ed25519 done; ML-DSA planned) | All |
| 123 | Zero-Knowledge Semantic Proofs | 2 | ⚠️ Partial | `zk_proofs.rs` (Pedersen done; Halo2 planned) | CLI, Desktop |
| — | DNSSEC → SocialWebNet | 2 | ✅ Implemented | `daemon_swarm.rs` | CLI, Desktop |
| 129 | Cryptographic Halo (FHE over WebGPU) | 3 | 🚧 Spec only | — | Desktop |
| 130 | Unforgeable Agency (TEE Biometric) | 3 | ⚠️ FFI only | `tee_ffi.rs` | CLI, Desktop |
| 131 | Intermittent Computing (NVM Snapshots) | 4 | 🚧 Spec only | — | Desktop |
| 132 | Spatial Web Anchoring (UWB + VPS) | 4 | 🚧 Spec only | — | Desktop, Mobile Native |
| 134 | Formal Verification (Coq/LEAN) | 4 | 🚧 Spec only | — | All |

> Full implementation specifications are in `local/architectural-enhancements/` — one `.md` per enhancement. The `Phase_2_Implementation_Completion_Summary.md` and `Architectural_Enhancement_Roadmap.md` provide the integration strategy and dependency graph.
