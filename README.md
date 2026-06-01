# Qualia-DB: The Human-Centric Semantic Ecosystem
<div align="center">
**Peace Infrastructure for the Natural Person**

Qualia-DB has evolved far beyond a bare-metal semantic graph database. It is a strictly constrained, mathematically verifiable **Ecosystem** designed to guarantee human agency, protect knowledge from proprietary enclosure, and dismantle infinite rent-seeking via native micro-economics.

It operates under a rigorous **Principal-Agent Duty of Care**: the software acts exclusively as the Agent on behalf of the Natural Person (the Principal).

## 🚀 The Three-Core Database Engine & The Orchestration Sieve

Qualia-DB abandons traditional cloud-centric, string-heavy JVM architectures in favor of a specialized 3-Core Triad built with ruthless mechanical sympathy (512MB RAM floor). 
However, raw multi-modal data (audio, camera feeds) would immediately crash this floor. To prevent this, the ecosystem forces an **Orchestration Sieve**: the Primary Agent must coordinate deterministic tools (OpenCV, Audio DSP) to strip noise, extract contours, and build optimized files *before* handing them to the local LLM or the database.

1. **Zero-Allocation Ingestion**: CBOR-LD gatekeeping and WASM OPFS bridging bypass heap-saturation attacks, writing natively to disk.
2. **GPU Sieve (Geometric Pruning)**: Graph nodes are mapped into Minkowski space within continuous 128KB memory-mapped `QualiaSuperBlocks`. The GPU calculates bounding-hull collisions to retrieve data at sub-microsecond speeds.
3. **The Prolog Sentinel (Logic Unification)**: Data filtering is not enough; human-centric databases must execute logic. Nested N3 implication rules are compiled into L1-cache bytecodes, guaranteeing $O(1)$ termination on highly cyclic social and legal graphs.

## ⚖️ The Rights Ontology & Semantic Adjudicator

Qualia-DB natively encodes a **Rights Ontology** directly into the N3Logic Sentinel VM.
- **Linguistic Plurality & Multi-Modal Semantics**: We reject the assumption that knowledge is exclusively bound to written Unicode strings. By utilizing binary CBOR-LD indexing, the ecosystem inherently supports "mother tongues", "languages of prayer", and non-written formats (verbal, ceremonial, heraldry, symbolic SVGs). A Semantic Quin maps a concept natively, regardless of the cultural format.
- **The Knowledge Axiom Predicate**: Rights to knowledge and fundamental shared learnings are mathematically un-propertizeable. If a semantic dispute arises, the Sentinel VM automatically dismisses any attempt to extract or enclose a Knowledge Axiom as intellectual property.
- **Proportional Escrow (Relational Assertion)**: When a dispute involves a specific *Application* or *Invention*, the N3Logic VM analyzes the `.q42` Provenance DAGs of both parties. It mathematically calculates the exact percentage of derivation and automatically splits incoming ILP Escrow funds based on absolute truth, stripping away false claims of originality.

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

## 📱 The Consumer Packaging

This backend ecosystem binds directly to two critical consumer interfaces:
1. **[WellFair](https://github.com/mediaprophet/wellfair/) (The Primary Mobile Agent)**: An independent mobile application structured around pluralistic foundational needs (Maslow, Systems of Faith). It binds to the Qualia-DB daemon to provide Sanctuary Modes, Duress Decoys, and Nym Mixnet anonymous routing.
2. **Cooperative Workspace**: A packaged desktop toolkit embedded in the ecosystem for managing collaborative `did:git` projects, visualizing DOAP stages, and organizing team Verifiable Credentials.

## 🛠️ Build Instructions

```bash
# Compile native binary (Daemon)
cargo build --release

# Compile WebWorker WASM Bridge
cd crates/qualia-core-db
wasm-pack build --target no-modules --out-dir ../qualia-client/pkg
```

---
*Built to guarantee first-class digital agency.*
