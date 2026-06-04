# Qualia-DB Architecture

> The 3-Core Triad, Webizen VM, Rights Ontology, and the Principal-Agent Ecosystem.

Qualia-DB abandons traditional cloud-centric, string-heavy JVM architectures in favour of a specialised 3-Core Triad built with ruthless mechanical sympathy (512MB RAM floor). Raw multi-modal data (audio, camera feeds) would immediately breach this floor, so the ecosystem forces an **Orchestration Sieve**: the Primary Agent must coordinate deterministic tools (OpenCV, Audio DSP) to strip noise, extract contours, and build optimised files *before* handing them to the local LLM or the database.

---

## The 3-Core Triad

### 1. Zero-Allocation Ingestion
CBOR-LD gatekeeping and WASM OPFS bridging bypass heap-saturation attacks, writing natively to disk. The `qualia-cli ingest` pipeline uses Rio multi-thread streaming, sorting Quins by subject before writing LZ4-compressed SuperBlocks, so the resulting `.q42` file supports O(1) block-range lookups via a companion `.q42.bidx` index.

### 2. GPU Sieve (Geometric Pruning)
Graph nodes are mapped into Minkowski space within continuous 128KB memory-mapped `QualiaSuperBlocks`. The GPU calculates bounding-hull collisions to retrieve data at sub-microsecond speeds without loading unrelated blocks.

### 3. The Webizen VM (Logic Unification + Advanced Compilation)
Data filtering is not enough — human-centric databases must execute logic. Nested N3 implication rules, SHACL shapes, and defeasible logic are compiled by the `WebizenCompiler` (and a dedicated `shacl_compiler`) into compact L1-cache bytecodes. The VM supports:

- Omnimodal surface syntaxes
- 6+ modality bridges (spatio-temporal, probabilistic, description logic, ASP, etc.)
- O(1) termination guarantees on highly cyclic social and legal graphs
- Rights Ontology and structural constraint enforcement at query time
- **Native Hard Science SHACL Extensions**: Custom `qualia:` properties map directly to pure-Rust hardware-accelerated math solvers (`NativeThermodynamics`, `NativeOdeSolver`, `NativeQuantumDft`, `NativeBioinformatics`). This allows the VM to transparently step out of logic resolution into zero-allocation continuous dynamics or SIMD alignment off-heap.

---

## Lazy SuperBlocks, LZ4 Compression & Massive Datasets

Core data lives in 40,960-byte SuperBlocks (exactly 10 disk sectors) with high-density LZ4 compression. The engine lazily scans only 16-byte headers and seeks over irrelevant blocks in O(1) time, decompressing on demand. "Missing" local blocks can be streamed from peers via WebRTC DataChannel. This lets 50GB+ semantic ledgers run comfortably inside the 512MB floor.

Real-world example: WordNet (523MB RDF) → 74.6MB `.q42` · 5.56M quins · 6.5ms first-query latency via demand-paging with no full load.

---

## Fractal Sharding & Swarm AI Compute

While Qualia-DB rigorously enforces the 512MB floor, it is capable of extreme horizontal scale on high-end hardware. Rather than bloating a single instance, it uses **Fractal Sharding**: on a rig with 64GB RAM and 12GB+ GPU, the daemon detects surplus hardware and dynamically spins up dozens of parallel, mathematically isolated 512MB worker cells.

```bash
qualia-cli daemon --workers 100 --compute-swarm
```

This Swarm Orchestration enables massive parallel execution, deep neural-network offloading, and background Sleep-Cycle AI Compute without compromising core mechanical sympathy.

---

## The Rights Ontology & Semantic Adjudicator

Qualia-DB natively encodes a **Rights Ontology** directly into the Webizen VM (with SHACL compilation, defeasible rules, and modality bridges).

- **Linguistic Plurality & Multi-Modal Semantics** — Binary CBOR-LD indexing natively supports mother tongues, languages of prayer, and non-written formats (verbal, ceremonial, heraldry, symbolic SVGs). A Semantic Quin maps a concept regardless of cultural format.
- **The Knowledge Axiom Predicate** — Rights to knowledge and fundamental shared learnings are mathematically un-propertisable. The Webizen VM automatically dismisses any attempt to enclose a Knowledge Axiom as intellectual property.
- **Proportional Escrow (Relational Assertion)** — When a dispute involves a specific Application or Invention, the N3Logic VM analyses the `.q42` Provenance DAGs of both parties, calculates the exact percentage of derivation, and automatically splits ILP Escrow funds proportionally.
- **SHACL & Structural Enforcement** — SHACL shapes are compiled into the same Webizen bytecode used for N3, enabling zero-allocation validation as part of query execution.

---

## Intentional Computing (Anti-Usury Architecture)

Qualia-DB is a framework for **Intentional Computing** — computing that strictly honours the intent, sovereignty, and Duty of Care of the natural person (the Principal).

- **First-Class Agency** — No admin superuser supersedes the Principal. Cryptographic keys are the absolute root of trust for your data boundary.
- **WebRTC CRDT Mesh & M:N Guardianship** — Distributed consensus isn't achieved through public blockchains, but via a local WebRTC Mesh. `did:q42` Webizens form an $M:N$ gossip network using `Automerge` CRDTs. When a child attempts a high-risk operation, the local Webizen halts the VM, packages the query as a `QuorumRequest`, and broadcasts it to the $N$ Guardian Webizens. Once $M$ cryptographic ratifications return, the local VM unblocks.
- **Desktop Network Webizen (`libpcap`)** — The local daemon acts as an active Wireshark-like firewall, monitoring all outbound network egress. If unauthorised spyware or telemetry SDKs attempt to exfiltrate N3Logic data, the Webizen severs the socket connection.
- **Decentralised Threat Intelligence** — When the Webizen encounters an unknown network pattern, it queries a peer-to-peer network of `.q42` reputation DAGs. If the graph flags the behaviour as spyware, the Webizen generates a new N3Logic permission rule and saves the event to a cryptographic audit chain.

---

## DID:GIT & Staged Axiomatic Evolution

Data projects in this ecosystem possess **Temporal Self-Governance**.

- Through the `did:git` Permissive Commons Profile, every project initialises a DOAP (Description of a Project) as its Genesis Block.
- To evolve a project to its next stage (e.g., changing its licence or logic), the proposed `git` commit must be mathematically validated by the N3Logic Webizen VM against the *former* axioms of the previous stage.
- If valid, the transition is anchored globally to the Bitcoin blockchain via `gitmark`.

---

## The ILP Economic Shift Engine

Qualia-DB explicitly rejects the infinite rent-seeking paradigm of the legacy web.

- Creators define an exact **Obligation Cost** using N3Logic Risk-Compounding algorithms (base rate × risk multiplier × temporal compounding).
- As Interledger Protocol (ILP) Web Monetisation streams flow in, the Daemon tracks the running balance.
- Once the exact mathematical threshold is met, the **Threshold Shift Licence (TSL)** automatically fires, irreversibly shifting the asset from *Commercial Gating* to the *Permissive Commons*.

---

## The Consumer Packaging (Qualia Native Vault)

Qualia-DB ships with two tightly-bound consumer interfaces:

1. **Qualia Mobile Vault (Android Jetpack Compose)** — The authoritative node. Handles high-frequency wearable ingestion (native C++ FFT Photoplethysmography), Verifiable Communications, and the **3D Biometric Holograph** (mapping valence/arousal telemetry to Minkowski space). Implements **Sanctuary Modes** with mathematically isolated PBKDF2 DB Lanes for vulnerable populations.

2. **Qualia Desktop Terminal (Tauri)** — A stateless desktop extension that pairs to the Mobile Vault via WebRTC (VC-8 Semantic Handshake). Features the **Semantic Library** (ingest PDFs via Edge VLM, or raw `.rdf`/`.owl` ontologies into binary `.q42` graphs) and acts as a heavy compute offload target for the Webizen Agent (local LLMs via Ollama).

---

## W3C Solid Interoperability Bridge (Allocation Firewall)

Qualia-DB operates natively on `.q42` CBOR-LD binary graphs with strict zero-allocation limits. To provide global backward compatibility with legacy W3C Solid apps, the ecosystem exposes the **Solid Interoperability Bridge** (`qualia-solid-bridge`). 

This module spins up a `warp`/`tokio` server that translates incoming heavy text protocols (HTTP REST, JSON-LD, standard Turtle) into minimal 64-bit Quin hashes via the `ldp_translator.rs`. 
Crucially, this acts as an **Allocation Firewall**: the multi-threaded `tokio` runtime is strictly sandboxed to the network boundary. No string allocations or heap thrashing can bleed into the core 512MB RAM floor of the native Webizen engine.

> **Roadmap (WAC ACLs):** W3C Solid uses static Web Access Control (`.acl` files). Qualia-DB uses the dynamic N3Logic Webizen VM. The proxy currently translates N3 logic into restrictive **Private (`acl:Control`)** policies on the fly. Future iterations will compile bounded N3 rulesets into fully compliant static ACL groups.

---

## Architectural Decision Records

Detailed rationale for specific design choices is in [adr/](adr/).

- [ADR 0001 — The 48-byte Qualia Quin Alignment](adr/0001-the-48-byte-qualia-quin-alignment.md)
- [ADR 0002 — Zero-Allocation Query Compiler](adr/0002-zero-allocation-query-compiler.md)
- [ADR 0003 — Permissive Commons Billing Gates](adr/0003-permissive-commons-billing-gates.md)
- [ADR 0004 — Sentinel to Webizen Terminology Rebrand](adr/0004-sentinel-to-webizen-rebrand.md)
- [ADR 0005 — DNS Frontdoor and HCAI Agreements](adr/0005-dns-frontdoor-and-hcai-agreements.md)
- [ADR 0006 — Zero-Allocation Solid Bridge Isolation](adr/0006-zero-allocation-solid-bridge.md)
