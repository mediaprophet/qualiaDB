# Qualia-DB Developer Guide

_Branch: `0.0.6-dev` | Last updated: 2026-06-06_

Qualia-DB is a bare-metal semantic graph database designed specifically for constrained personal environments (mobile devices, IoT, browsers). It enforces a strict 512 MB RAM floor and operates with absolute zero dynamic heap allocation during execution, making garbage-collection pauses mathematically impossible.

---

## The 3-Core Triad Architecture

Qualia-DB splits its workload across three highly specialized, isolated cores:

### Core 1: The Webizen (Logic VM)
A Warren Abstract Machine (WAM) variant written in `#![no_std]` Rust.
- **SLG Tabling**: Uses a 42 MB static ring-buffer (`SlgArena`) to memoize cyclic graph traversals. This prevents infinite stack overflows when querying complex recursive social networks.
- **8 Modality Bridges**: Spatio-temporal, probabilistic, description logic (DL), answer set programming (ASP), linear logic, epistemic logic, paraconsistent logic, and linear temporal logic (LTL). All fully implemented and tested.
- **Native Scientific Primitives**: `NativeThermodynamics`, `NativeOdeSolver`, `NativeQuantumDft`, `NativeBioinformatics`, `NativeClinicalRisk`, `NativeChemicalSynthesis` — all wired to real Rust implementations, not stubs.

### Core 2: The GPU Sieve
Instead of relying on heavy CPU indexing (like B-Trees), Qualia-DB leverages the device GPU (or WebGPU in the browser) to perform massive bitwise SIMD scans over the raw data buffer in parallel.

The WGSL compute shader (`shaders/fused_tensor_contraction.wgsl`) runs 64 threads/workgroup with 4096 FMA ops per thread, across DirectML / Vulkan / Metal / WebGPU via `wgpu`. This is also the shader used for LLM inference — the same GPU path handles both graph sieving and transformer block dispatch.

### Core 3: Ingestion & I/O
- **Strict Binary (CBOR-LD)**: Qualia-DB rejects text-based JSON-LD or Turtle in hot paths. Values are hashed directly into Quins using FNV-1a; no intermediate string representation enters the engine. The CLI's `ingest` command accepts Turtle/N-Triples/RDF-XML via Rio streaming, but converts immediately to the 48-byte Quin format.
- **Zero-Copy IPC**: When running in the browser, the WASM engine is locked to a `SharedArrayBuffer`, allowing the UI thread to read query results without copying memory.

---

## Cryptographic Human Agency Records

Instead of "Data Sovereignty" (which implies server control), Qualia-DB secures **Cryptographic Human Agency Records**.
- The system uses `ed25519-dalek` to enforce Bilateral Micro-Commons rules.
- **Author-Scoped Merkle Signatures**: A user never signs the entire database state. They only cryptographically endorse the specific Merkle sub-roots containing the Quins they personally authored.

---

## The 48-Byte Super-Quin

Every statement in Qualia-DB is mapped to a strict 48-byte hardware struct:

```
[Subject (u64)] [Predicate (u64)] [Object (u64)] [Context (u64)] [Metadata (u64)] [Parity (u64)]
```

All semantic meaning is encoded via bit-packing. See `AGENTS.md §1` for the authoritative bit layout. The full spec is also in the root `ARCHITECTURE.md §2`.

Key invariant: `q_hash()` (FNV-1a) at compile time for all IRIs. No runtime string allocation in the engine core.

---

## The LLM Inference Stack

Qualia-DB runs LLM inference entirely in-process. There is **no Ollama**, no Python runtime, no external HTTP server for models.

1. `gguf_sharder.rs` — parses GGUF header, generates `QualiaQuin` pointer map
2. `gguf_bridge.rs` — maps model weights into OS page cache via `memmap2` (zero heap allocation); dispatches fused transformer blocks to the GPU
3. `shaders/fused_tensor_contraction.wgsl` — WGSL compute shader via `wgpu`
4. `llm_agent.rs` — `LocalLlmAgent` orchestrates Phase 8 bifurcated compute
5. `orchestrator.rs` — `TaskOrchestrator` manages `ModelLifecycle` and `ThermalGovernor`

**Phase 8 Bifurcated Compute**: Token generation uses two wait-free SPSC ring buffers (`rtrb`). The `LogitStream` carries logit vectors from the LLM engine thread to the Webizen Sentinel thread; the `ControlStream` carries `DenyRollback` signals back. The Sentinel can interrupt generation mid-token if it detects an anomaly.

**AgentBackend** (`llm_agent.rs`) has exactly three variants: `Local`, `Remote`, `Hybrid`. Do not add an Ollama backend.

---

## Departures from Tradition (The Architecture Leaps)

### 1. The Death of B-Trees: The GPU Sieve
Graph topologies are mapped geometrically into Minkowski space. Raw 128 KB contiguous memory blocks are passed directly to the device GPU for parallel bounding-hull collision math. No B-Tree, no pointer chasing.

### 2. Strict Binary Ingress vs. The String Parsing Vulnerability
Values are FNV-1a hashed into 64-bit Quin fields at ingest time. The engine routes bytes directly into hardware registers, skipping the heap. The CogAI Chunks (`.chk`) text format is the deliberate exception — it is a human-authored cognitive knowledge format (W3C CG ACT-R chunks-and-rules) that is parsed into Quins at ingest time, not presented to the VM as raw text.

### 3. Author-Scoped Signatures vs. The "Global Endorsement Trap"
**Author-Scoped Merkle Aggregation** — you sign only the specific Merkle Sub-Root containing your explicitly authored Quins.

### 4. In-Place CRDT Sync vs. Event-Sourcing Bloat
Syncs are resolved in O(N) time by diffing 12-byte Merkle-DAG Jump Tables. Epoch Compaction shrinks the dataset by zeroing tombstones.

### 5. Zero-Copy IPC vs. Socket Serialization
The WASM Engine and the UI thread share the same 512 MB `SharedArrayBuffer`. No serialisation boundary.

### 6. The Neurosymbolic Intercept Protocol (Bifurcated Compute)
The Webizen Sentinel reads logit vectors in real time as the LLM generates tokens. If a mapped vector coordinate triggers a local Spatio-Temporal Axiom violation, the Sentinel injects a `DenyRollback` and the LLM recalculates — mid-generation, not post-hoc. This is implemented via the two SPSC ring buffers in Phase 8.

### 7. Lazy SuperBlocks + On-Demand P2P (Zero-Heap Massive Graphs)
40,960-byte SuperBlocks (10 sectors) with high-density LZ4. `lazy_superblock_query` does pure header scans + O(1) seeks. Missing local blocks are streamed from peers (WebRTC DataChannel). Enables 50 GB+ ledgers on 512 MB devices.

### 8. SHACL / Defeasible / Omnimodal Webizen Compilation (Core 1 Evolution)
`WebizenCompiler` + `shacl_compiler` translate SHACL shapes + N3 defeasible rules into the same compact `[u64; 16]` bytecode the VM executes. Eight modality bridges normalize into the registry for unified reasoning.

### 9. MCP Fiduciary Mediation Layer
Every LLM call passes through `orchestrate_inference()`: pre-flight `validate_intent()` (N3Logic Rights Ontology check), actual inference, post-flight `validate_output()` (≥ 1 provenance Quin required). The `McpIntentFrame` carries purpose_hash, deontic_constraints, and the active `CapabilityProfile` ID with every tool call.

### 10. Capability Profiles
QCHK (`.chk`) binary bundles declare the allowed engine operations and ontology namespaces for an agent session. Six named profiles: general, health, chemistry, research, legal, financial. Compiled via `qualia-cli profile compile`.

---

## Instructing Your Local AI Coding Agents

Because Qualia-DB radically departs from standard database theory, **generic AI coding agents will fail spectacularly** if given no context. They will attempt to write JSON-LD parsers, allocate memory on the heap, and use standard-library strings — all of which will trigger a panic.

**Load the orientation files first:**

- `CLAUDE.md` — primary orientation for Claude Code sessions (start here)
- `AGENTS.md` — multi-agent coordination layer with Quin bit layout and known inconsistencies

These files are the authoritative agent kickstart. They supersede the older `AI_INSTRUCTIONS.md` and `.cursorrules` files.

### Critical invariants for agents

1. No `Vec`, `String`, or `Box` in hot paths.
2. All graph statements conform to the 48-byte Super-Quin.
3. All string IRIs are `q_hash()`-ed, never stored as strings.
4. Opcodes `0x10+` for all new modalities (`mini_parser.rs` owns `0x00–0x04`).
5. The LLM backend is `gguf_bridge.rs` + `wgpu`. Do not add Ollama or external HTTP model clients.
6. The Webizen VM gates every LLM call. Do not bypass `orchestrate_inference()`.
