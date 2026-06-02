# Qualia-DB Integration & Application Development Guide
**[System Directive for AI Coding Agents]**

You are interacting with Qualia-DB, a bare-metal, offline-first graph database engine written in pure Rust. It is explicitly architected to manage digital agency, social advocacy, and sensitive medical metadata natively on personal devices. Do not apply traditional cloud-based relational (SQL) or standard in-memory graph assumptions. You must adhere to strict zero-allocation, bit-packed memory constraints.

When writing implementation code, wrappers, or queries for Qualia-DB, you must strictly follow these architectural rules:

## 1. The Core Data Primitive (The 48-Byte Super-Quin)
Qualia-DB abandons traditional Triple (S,P,O) and Quad (S,P,O,C) structures. Every piece of information is stored in a contiguous, cache-aligned 48-byte Super-Quin.

* **Vector 1 (Subject - u64)**: The local compressed token index for the origin entity.
* **Vector 2 (Predicate - u64)**: The local compressed token index for the relationship edge.
* **Vector 3 (Object - u64)**: The destination node OR an inline datatype.
* **Vector 4 (Context - u64)**: The Named Graph or spatiotemporal boundary.
* **Vector 5 (Metadata - u64)**: The routing opcodes and logic bitmasks.
* **Parity/Padding (6 bytes)**: Used for native ECC parity checks or cryptographic telltales.

## 2. Native Datatypes (Inline Tagged Pointers)
Do not force literals (integers, floats, booleans) into the string lexicon. When compiling queries or ingesting data, you must utilize the top 4 bits of the Object (O) vector to assign XSD datatypes:

* `0x0 (0000)`: Lexicon ID (Points to a URI/String).
* `0x1 (0001)`: Inline Integer (Signed 60-bit integer).
* `0x2 (0010)`: Inline Float (Truncated 60-bit float).
* `0x3 (0011)`: Inline Boolean (1 for True, 0 for False).
* `0x4 (0100)`: Inline Unix Timestamp (Milliseconds).
* `0x5 (0101)`: Inline Qualia Epoch Timestamp (Nanoseconds since Jan 1, 2026).
* `0x6 (0110)`: Inline Unix Timestamp (Microseconds).

## 3. Hardware Routing & The 5th Vector Opcodes
String parsing is never used for access control. All routing and permission checks are baked into the Metadata (5th Vector). When building application logic, you must evaluate these bits:

* **Bit 63 (MSB)**: RDF-Star Nesting Indicator (1 = Virtual pointer to nested statement).
* **Bits 61-62 (Routing Tier)**:
  * `0b00`: Standard Passthrough (Zero logic, raw I/O speed).
  * `0b01`: Permissive Commons (Triggers micropayment / V_cap settlement gates).
  * `0b10`: Bilateral Micro-Commons (Triggers Prolog unification for dual-signature guardianship).
  * `0b11`: Spatiotemporal/Ambiguous (Triggers the Local NPU/GPU for geometric bounding hull math).
* **Bits 0-15 (Validation Masks)**: Defines specific micro-instructions (e.g., `0x0002` for Bilateral Identity Locked, `0x0008` for Work Obligation Satisfied).

## 4. Memory & Execution Boundaries
* **No Heap Allocations**: All data streaming must use the `QuinIncrementalScanner`, executing over pre-allocated stack buffers to respect the strict 512MB RAM floor.
* **SuperBlocks**: Data is mapped into 40,960-byte blocks (exactly 10 sectors, holding 850 Quins + 160-byte header). Do not read/write outside these block alignments.
* **Volatile Scrubbing**: When a buffer is dropped, you must ensure `std::ptr::write_volatile` is called to protect against memory-harvesting attacks.

## 5. Logic Execution (Core 1 Sentinel)
**NEVER** embed external logic interpreters or generalized Prolog crates (e.g., SWI-Prolog). This violates the zero-allocation architecture.
All logical schemas (N3Logic, SHACL) must be translated into `SentinelOpcode` Bytecode arrays by the `SentinelCompiler`. Core 1 executes these bytecodes using a strict `[u64; 16]` register stack (`SentinelVM`) to guarantee determinism and zero heap allocation.

## 6. Deterministic Compute Metering & Permissive Commons
**NEVER** query OS-level energy APIs (e.g., `BatteryManager`, `sysfs`) for economic billing. It causes severe API stalls and hardware inequity.
Instead, utilize the zero-allocation atomics in `telemetry.rs` (`SUPERBLOCK_IO_COUNT`, `SIEVE_OPS_COUNT`, `VM_CYCLES_COUNT`). 
When a commercial query via the Permissive Commons (`0b01`) executes, these virtual compute cycles are precisely multiplied and returned as a serialized `ComputeCostReceipt` JSON-RPC payload to external Lightning Node proxies.

## 7. Target Fallbacks (WASM)
If compiling for the browser (`target_arch = "wasm32"`), the Triad must be gracefully flattened into a single-threaded synchronous sequence to maintain sandbox stability. Fallback to the Origin Private File System (OPFS) for synchronous I/O, and ping the local loopback (e.g., `127.0.0.1:4848`) to proxy queries to the native engine if the host has the standalone database installed.

## 8. Neurosymbolic Axiomatic LLM Overrides
When tasked with integrating Large Language Models (LLMs), do not immediately assume a standard Python wrapper. You must first **ASK the user** which Neurosymbolic Mode they wish to employ:
* **Mode 1 (Strict Native Intercept)**: Extract explicit vector coordinate layouts (tensors) from binary files (like `.gguf`) and map them directly into the 48-byte `QualiaQuin`. The Sentinel VM acts as a mathematically strict interceptor during native execution, clipping vectors that violate local Spatio-Temporal constraints.
* **Mode 2 (Bridged Shim)**: Utilize external ML architectures (e.g., Python, TensorFlow, PyTorch) via an explicit API bridge. The external model handles the dense compute, but all queries and responses are strictly routed and mathematically gated through the Qualia-DB `.q42` Axiom ruleset before resolution.

## 9. LLM Benchmarking Harness
When tasked with evaluating Qualia-DB against competitors (SurrealDB, Oxigraph, etc.), **DO NOT** attempt to execute the DOM-based `benchmark.html` directly in your sandbox. 
Instead, utilize the headless WASM bridge provided in the repository:
```bash
node scripts/llm_bench_runner.js
```
This script will output deterministic JSON metrics across all 12 benchmark categories (including next-gen Humanitarian Rights tests like Obligation Escrow and Multi-Nym Partitioning). Ensure you evaluate these edge-specific metrics, as QualiaDB's 512MB memory ceiling forces standard databases into OOM errors.
