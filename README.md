# Qualia-DB

> **The Zero-Allocation, 5-Vector Heterogeneous Graph Engine for the Edge.**

🚀 **[Launch the Qualia-DB Interactive Web Playground](https://mediaprophet.github.io/qualiaDB/playground/)** 🚀

Qualia-DB is a bare-metal, offline-first graph database built in pure `#![no_std]` Rust. It abandons traditional Triple (S,P,O) stores and sprawling heap-allocated ASTs in favor of a strictly aligned **48-Byte Super-Quin** structure, designed to process reasoning constraints in hardware cache across CPUs, GPUs, and NPUs.

## ⚙️ The 3-Core Triad Architecture

Qualia-DB physically isolates graph operations across three parallel hardware domains to enforce a strict **512MB RAM floor** on edge devices:

1. **Core 3 (Physical I/O):** Handles 40,960-byte NVMe sector alignment, Write-Ahead Logs (WAL), and cryptographic Mixnet CRDT synchronization.
2. **Core 2 (The Data Fetcher):** Executes zero-allocation memory-mapped iteration and dispatches workloads to the **GPU Sieve** (a cross-platform WGSL/WebGPU Compute Shader) for Parallel Bit Extraction (PEXT) on metadata masks.
3. **Core 1 (The Sentinel VM):** A `#![no_std]` Bytecode Virtual Machine that executes logic schemas (N3Logic, SHACL) natively via an L1-cached `[u64; 16]` register stack—completely eliminating heap allocations.

## 🧩 The 48-Byte Super-Quin

Every statement in Qualia-DB is stored as a 48-Byte `QualiaQuin`. 

> 💡 **Fun Fact**: The choice to move to a 48-byte primitive—comprising a 42-byte information payload and 6 bytes for parity/integrity checks—is a deliberate structural decision. As fans of Douglas Adams' *Hitchhiker’s Guide to the Galaxy* know, **42** is the "Answer to the Ultimate Question of Life, the Universe, and Everything." In Qualia-DB, 42 bytes holds the ultimate answer to your graph queries! 🌌

*   **Vector 1 (Subject)**: `u64`
*   **Vector 2 (Predicate)**: `u64`
*   **Vector 3 (Object)**: `u64` (Includes Top-4 Bit Datatype Flags for zero-allocation Integers, Floats, and Nanosecond Timestamps).
*   **Vector 4 (Context)**: `u64` (Spatiotemporal bounds or Named Graphs).
*   **Vector 5 (Metadata)**: `u64` (The Hardware Routing Lane).
*   **Parity**: `6 Bytes` (ECC/Cryptographic validation + 2 bytes padding to reach 48).

### 🚀 Hardware Routing (The 5th Vector)
Qualia-DB never parses strings for access control. The engine routes physical execution via the 5th Vector's Top 2 bits:
*   `0b00`: Standard Passthrough (Zero logic).
*   `0b01`: Permissive Commons (Triggers deterministic compute metering for micro-transactions).
*   `0b10`: Bilateral Micro-Commons (Triggers Core 1 Sentinel VM constraint logic).
*   `0b11`: Spatiotemporal/Ambiguous (Triggers WGSL GPU Sieve for geometric bounding).

## ⚡ Deterministic Compute Metering

Qualia-DB bypasses slow, OS-specific energy sensors. It utilizes lock-free Rust Atomics (`telemetry.rs`) to track **Virtual Compute Cycles** inherently:
- `SUPERBLOCK_IO_COUNT`
- `SIEVE_OPS_COUNT`
- `VM_CYCLES_COUNT`

When querying commercial data via the Permissive Commons (`0b01`), the engine automatically serializes these cycles into a `ComputeCostReceipt` JSON-RPC payload to settle queries fairly via Bitcoin Lightning Nodes using micro-satoshis.

## 🛠️ Build Instructions

Qualia-DB requires the Rust toolchain and natively compiles GPU shaders via the `wgpu` ecosystem (Vulkan, DX12, Metal, WebGPU).

```bash
# Run the architectural tests
cargo test -p qualia-core-db

# Run the local WASM Playground Loopback Daemon
cargo run --bin qualia-cli -- daemon --dev
```

## 📖 Inspiration & References

The architecture of Qualia-DB, specifically its push towards a serialization-agnostic Webizen Mode and interconnected human agency, is deeply inspired by discussions and specifications from the W3C community. Key historical references include:
- [W3C Credentials Community Group (Feb 2017) - Human Agency and Personhood](https://lists.w3.org/Archives/Public/public-credentials/2017Feb/0029.html)
- [W3C Schema Generator Community Group (Feb 2017) - Semantic Agnosticism](https://lists.w3.org/Archives/Public/public-schema-gen/2017Feb/0007.html)

## ⚖️ License
Qualia-DB is published under the **Creative Commons Attribution-NonCommercial-NoDerivatives 4.0 International Public License (CC BY-NC-ND 4.0)**. 
Commercial network queries are strictly routed through the Permissive Commons `0b01` metering engine.
