# Qualia-DB Developer Guide

Qualia-DB is a bare-metal semantic graph database designed specifically for constrained edge environments (mobile devices, IoT, browsers). It enforces a strict 512MB RAM floor and operates with absolute zero dynamic heap allocation during execution, making garbage-collection pauses mathematically impossible.

## The 3-Core Triad Architecture

Qualia-DB splits its workload across three highly specialized, isolated cores:

### Core 1: The Sentinel (Logic VM)
A Warren Abstract Machine (WAM) variant written in `#![no_std]` Rust.
- **SLG Tabling**: Uses a 42MB static ring-buffer (`SlgArena`) to memoize cyclic graph traversals. This prevents infinite stack overflows when querying complex recursive social networks.

### Core 2: The GPU Sieve
Instead of relying on heavy CPU indexing (like B-Trees), Qualia-DB leverages the device GPU (or WebGPU in the browser) to perform massive bitwise SIMD scans over the raw data buffer in parallel.

### Core 3: Ingestion & I/O
- **Strict Binary (CBOR-LD)**: Qualia-DB violently rejects text-based JSON-LD or Turtle. To prevent CPU string-parsing bottlenecks, the client must compress data into CBOR-LD *before* sending it.
- **Zero-Copy IPC**: When running in the browser, the WASM engine is locked to a `SharedArrayBuffer`, allowing the UI thread to read query results synchronously without copying memory.

## Cryptographic Proof of Human Agency

Instead of "Data Sovereignty" (which implies server control), Qualia-DB enforces **Human Agency**. 
- The system uses `ed25519-dalek` to enforce Bilateral Micro-Commons rules.
- **Author-Scoped Merkle Signatures**: A user never signs the entire database state. They only cryptographically endorse the specific Merkle sub-roots containing the Quins (statements) they personally authored.

## The 48-Byte Super-Quin

Every statement in Qualia-DB is mapped to a strict 48-byte hardware struct:
`[Subject (8)] [Predicate (8)] [Object (8)] [Context (8)] [Metadata (8)] [Parity (8)]`

This allows exactly 1,000,000 statements to fit perfectly into a 48MB contiguous memory slab, which is precisely engineered to align with L1/L2 cache lines on modern ARM processors.
