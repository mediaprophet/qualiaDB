# Qualia-DB 
<div align="center">
**The Human-Centric Semantic Engine**

Qualia-DB is a bare-metal, strictly-constrained, offline-first semantic graph database. Designed explicitly for personal computing environments, it operates under a ruthless **512MB RAM floor**, enforcing mechanical sympathy at every layer to guarantee human agency, battery life, and deterministic logic execution for the Wellfair architecture.

## 🚀 The Three-Core Architecture

Qualia-DB abandons traditional cloud-centric, string-heavy JVM architectures in favor of a specialized 3-Core Triad.

### Core 3: Zero-Allocation Ingestion & IO
The boundary between the network and the disk is a strict binary fortress.
- **CBOR-LD Gatekeeper**: Explicitly rejects JSON-LD or text-based payloads to prevent heap-saturation attacks. It ingests variables via a binary compression dictionary, constructing 64-bit Lexicon indices without allocating a single `String` or `Vec`.
- **WASM OPFS Bridge**: In the browser, Qualia bypasses IndexedDB and dynamically allocates a strict 512MB `SharedArrayBuffer`, writing natively to the disk via the `createSyncAccessHandle` Origin Private File System API.

### Core 2: The GPU Sieve (Geometric Pruning)
Instead of relying on slow, pointer-chasing B-Trees, Qualia-DB maps graph nodes into continuous, 128KB memory-mapped `QualiaSuperBlocks`.
- **The Lorentz Hyperboloid**: Relationships are mapped geometrically into Minkowski space.
- **Vulkan / WebGPU Compute**: The CPU offloads the 128KB frame to the GPU/NPU, which executes highly parallel bounding-hull collision math to rapidly isolate the 48-byte `SuperQuins` you need.

### Core 1: The Prolog Sentinel (Logic Unification)
Data filtering is not enough; a human-centric database must execute logic. Qualitative data—social agency, bilateral medical access, property delegation—requires mathematical verification of rules. Qualia-DB intercepts the graph and executes the logic natively via the Sentinel Virtual Machine. Sentinel mathematically guarantees $O(1)$ termination on highly cyclic social and legal graphs, eliminating stack-overflow vulnerabilities.
- **Native N3Logic Execution**: Nested N3 implication rules (`{...} => {...}`) are compiled directly into Sentinel Bytecodes. By resolving inferences strictly over flattened 64-bit hardware identifiers instead of text strings, N3 inference operates at sub-microsecond speeds entirely within the L1/L2 cache.

## 🛡️ Cryptographic Proof of Human Agency

We strictly reject the "Global Endorsement Trap". In a shared Bilateral Micro-Commons, you only sign what you author.
- **Author-Scoped Merkle Aggregation**: The engine isolates the 48-byte Quins belonging strictly to a user's DID and calculates a Merkle Sub-Root hash.
- **Ed25519 Signatures**: A single 64-byte signature mathematically guarantees the author's claims without forcing them to endorse third-party injected data.
- **Zero-Allocation CRDT Sync**: 12-bit Lamport clocks resolve edge-device conflicts natively within the 128KB static buffers, triggering *Epoch Compactions* that actively shrink the dataset by zeroing out Tombstone arrays.

## ⚡ Spectacle Demo

We have built a glassmorphic, interactive dashboard to visually demonstrate the WASM Engine boot sequence, CBOR-LD ingestion, the SLG VM, and Ed25519 Cryptographic Agency verification.

**[Launch the Sentinel Environment Demo](https://mediaprophet.github.io/qualiaDB/)** *(Requires a browser with `SharedArrayBuffer` COOP/COEP support).*

## 📊 Benchmarks

*Powered by `cargo bench` + `criterion` on bare metal:*

- **Quin Allocation Constraint**: `~3.5 ns` per 48-byte Struct.
- **CBOR-LD Dictionary Ingestion**: `~18.4 ns` per SuperQuin construction (Zero Heap Allocations).
- **GeoSPARQL-Star Subset Compilation**: `~69.1 ns` directly into native hardware 64-bit opcodes.
- **Legacy String Pipeline Ingestion (1,000 Lines)**: `~89.0 µs` (Highly optimized text path, yet still vastly slower than the 18ns CBOR binary path).

## 🛠️ Build Instructions

```bash
# Compile native binary (Daemon)
cargo build --release

# Compile WebWorker WASM Bridge
cd crates/qualia-core-db
wasm-pack build --target no-modules --out-dir ../qualia-client/pkg
```

---
*Built with ruthless mechanical sympathy.*
