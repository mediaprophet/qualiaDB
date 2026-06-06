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

### Components

| # | File | Role |
|---|------|------|
| 1 | `gguf_sharder.rs` · `GgufTokenizer` | Parses the GGUF v2/v3 KV metadata section to extract the full vocabulary list, BOS token ID, and EOS token ID. `encode(text)` uses greedy longest-match tokenisation with a single-byte fallback. `decode(ids)` handles SentencePiece `▁` prefixes and `<0x##>` byte-literal tokens. Defaults to a 256-entry byte-level tokeniser when the model file is absent. |
| 2 | `gguf_sharder.rs` · `GGufSharder` | Parses GGUF magic + tensor count; generates `QualiaQuin` pointer maps (byte offsets, modality flag `0b1001` in upper 4 bits of the object field). |
| 3 | `gguf_bridge.rs` · `QTensorEngine` | `load_gguf(path)` memory-maps the GGUF file via `memmap2`. `dispatch_fused_transformer_block(tensor, activations)` dispatches GEMM to DirectML 1.15 (Windows), Accelerate `cblas_sgemm` (macOS AMX), or the wgpu/WGSL fallback. All three produce a `Vec<f32>` logit vector. |
| 4 | `llm_agent.rs` · `LocalLlmAgent::infer_local_model` | Autoregressive decode loop. Initialises `QTensorEngine` + `GgufTokenizer` inside the spawned LLM thread. Per step: pseudo-embedding from token ID → `dispatch_fused_transformer_block` → argmax logit → `LogitSummary` pushed to SPSC ring → Sentinel anomaly check → optional `DenyRollback` → sample next token → accumulate → stop at EOS / `MAX_OUTPUT_TOKENS`. |
| 5 | `orchestrator.rs` · `TaskOrchestrator::orchestrate_inference` | Mandatory governance gate: `validate_intent` (Rights Ontology pre-flight) → `LocalLlmAgent::infer` → `validate_output` (≥ 1 provenance Quin required). |

### Phase 8 Bifurcated Compute

Two wait-free SPSC ring buffers (`rtrb`) decouple generation from governance:

```
LLM Engine thread  ──LogitSummary──►  LogitStream (cap 1024)  ──►  Webizen Sentinel
                   ◄──DenyRollback──  ControlStream (cap 16)  ◄──  (calling thread)
```

The `LogitSummary` struct is fixed-size (two `u32` + one `u8`) — no heap allocation in the hot path. If `anomaly_byte == 0x99`, the Sentinel injects `DenyRollback`; the LLM thread substitutes a safe neighbour token on the next step.

### Known limitation — pseudo-embeddings

The current loop generates a deterministic sin-based embedding from the token ID rather than reading the real `token_embd.weight` tensor from the GGUF. The GGUF KV section (vocabulary) is parsed; the tensor-info section (tensor names + byte offsets) is the next milestone. Until that lands, the GPU dispatch runs against real weight matrices but from a synthetic embedding, so output text is incoherent.

### AgentBackend variants

```rust
Local   // GGUF on-disk → GPU dispatch → in-process. No outbound traffic. 128 MB RAM cap.
Remote  // API call → Nym mixnet → ILP metered. Requires signed VC from Principal.
Hybrid  // Local-first; falls back to Remote only with explicit Principal consent.
```

Do not add an Ollama backend or any external HTTP model client.

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

## Flutter Desktop App & FRB API

The `crates/qualia-flutter/` package is a Flutter desktop app (Windows, macOS, Linux) that calls the Rust engine via [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge) (FRB).

### Architecture

```
Flutter (Dart)
  └── lib/src/rust/api/   ← generated FRB bindings (do not edit directly)
        ├── qualia_api.dart      ← main API surface
        └── resource_catalog.dart ← LLM / ontology catalog

      ↕  FFI (cdylib on desktop, staticlib on iOS)

crates/qualia-flutter/rust/src/api/
  ├── qualia_api.rs         ← FRB-exported functions (edit here)
  └── resource_catalog.rs   ← catalog functions
  └── depends on: qualia-core-db, qualia-client-core
```

### Regenerating bindings

After editing any `pub fn` in `rust/src/api/`:

```bash
cd crates/qualia-flutter
flutter_rust_bridge_codegen generate
```

This overwrites `lib/src/rust/` — commit the generated files.

### Key inference function

```dart
import 'package:qualia_flutter/src/rust/api/qualia_api.dart' as api;

// Runs the full Webizen-gated inference pipeline:
//   validate_intent → Phase 8 SPSC GPU loop → validate_output
final response = await api.runInference(
  prompt: 'What is the thermodynamic entropy of water at 300 K?',
  modelPath: '/home/user/.qualia/models/phi3-mini-q4km.gguf',
);
```

`modelPath` must be the absolute path to a `.gguf` file already downloaded to the device. Pass an empty string to receive the "no model loaded" sentinel message.

See the full API reference at [flutter-api-reference.md](flutter-api-reference.md).

### Screen → API mapping

| Screen | Primary API calls |
|---|---|
| Chat | `runInference(prompt, modelPath)` |
| LLM Hub | `loadLlmResources()`, `downloadLlm(id)`, `downloadModel(url, filename, modelId)` |
| Ontology Hub | `loadOntologyResources()`, `importOntology(id)` |
| App Vault | `listInstalledApps()`, `launchInstalledApp(appName)`, `verifyAndInstallApp(zipPath, credentialSig)` |
| Wallet | `getCoinBalances()`, `deriveWalletsFromSeed(seed)`, `generateBip39Seed()` |
| Settings | `getConfig()`, `saveConfig(newConfig)` |
| Dashboard | `getHardwareStatus()`, `daemonStatus()`, `startDaemon()` |

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
