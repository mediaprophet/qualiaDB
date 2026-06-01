# Qualia-DB Developer Guide

Qualia-DB is a bare-metal semantic graph database designed specifically for constrained personal environments (mobile devices, IoT, browsers). It enforces a strict 512MB RAM floor and operates with absolute zero dynamic heap allocation during execution, making garbage-collection pauses mathematically impossible.

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

## Cryptographic Human Agency Records

Instead of "Data Sovereignty" (which implies server control), Qualia-DB secures **Cryptographic Human Agency Records**. 
- The system uses `ed25519-dalek` to enforce Bilateral Micro-Commons rules.
- **Author-Scoped Merkle Signatures**: A user never signs the entire database state. They only cryptographically endorse the specific Merkle sub-roots containing the Quins (statements) they personally authored.

## The 48-Byte Super-Quin

Every statement in Qualia-DB is mapped to a strict 48-byte hardware struct:
`[Subject (8)] [Predicate (8)] [Object (8)] [Context (8)] [Metadata (8)] [Parity (8)]`

This allows exactly 1,000,000 statements to fit perfectly into a 48MB contiguous memory slab, which is precisely engineered to align with L1/L2 cache lines on modern ARM processors.

## Departures from Tradition (The 5 Paradigm Shifts)

When analyzing Qualia-DB against the historical computing landscape, it breaks from traditional theory in five specific ways:

### 1. The Death of B-Trees: The GPU Sieve
- **Traditional Method:** Almost all databases rely on B-Tree or Hash indexes. These require dynamic memory allocation, heavy pointer-chasing across the heap, and cause massive L1 cache misses.
- **The Qualia-DB Leap:** We abandoned index trees entirely. We map graph topologies geometrically into Minkowski space and pass raw, 128KB contiguous memory blocks directly to the device GPU or NPU for parallel bounding-hull collision math.

### 2. Strict Binary Ingress vs. The String Parsing Vulnerability
- **Traditional Method:** Semantic engines accept JSON-LD or Turtle text, requiring them to allocate massive strings, run regex lexers, and build complex ASTs (Abstract Syntax Trees), making them vulnerable to OOM attacks.
- **The Qualia-DB Leap:** Qualia-DB refuses to parse text. Clients *must* compress payloads into binary CBOR-LD before sending. The engine routes bytes directly into 64-bit hardware registers, skipping the heap entirely.

### 3. Author-Scoped Signatures vs. The "Global Endorsement Trap"
- **Traditional Method:** Distributed databases usually require users to sign the *Global* Merkle Root, inadvertently making them legally liable for malicious data injected by other peers in a shared local graph.
- **The Qualia-DB Leap:** We engineered **Author-Scoped Merkle Aggregation**. You cryptographically sign *only* the specific Merkle Sub-Root containing your explicitly authored Quins.

### 4. In-Place CRDT Sync vs. Event-Sourcing Bloat
- **Traditional Method:** Offline-first Event Sourcing logs grow infinitely over time, causing massive memory bloat during peer-to-peer syncs.
- **The Qualia-DB Leap:** Syncs are resolved in $O(N)$ time by diffing 12-byte Merkle-DAG Jump Tables. Afterward, "Epoch Compaction" actively shrinks the dataset by zeroing tombstones.

### 5. Zero-Copy IPC vs. Socket Serialization
- **Traditional Method:** When a browser UI queries a local WASM database, the data must be serialized, copied across the JS boundary, and deserialized into JSON.
- **The Qualia-DB Leap:** The WASM Engine and the UI thread are locked into the exact same 512MB `SharedArrayBuffer`. When the engine finds an answer, the UI reads the raw memory address instantly. Zero copying. Zero latency.

### 6. The Neurosymbolic Intercept Protocol (Axiomatic LLM Override)
- **Traditional Method:** Large Language Models (LLMs) are black-box probabilistic engines that hallucinate due to a lack of deterministic symbolic grounding or local "Spatio-Temporal Qualia" (e.g., modern vector weights assuming "thongs" means underwear globally).
- **The Qualia-DB Leap:** We don't map generic semantic tokens; we map the *exact* procedural vector matrices (tensors) of the LLM into our 48-byte Quins. The Sentinel VM then acts as a mathematically strict interceptor. 
  - A user defines a Spatio-Temporal Qualia Context (e.g., `Context: year=1920` or `location=australia`).
  - As the opaque LLM executes its inference locally, the Sentinel VM monitors the active procedural tensor blocks.
  - If a mapped vector coordinate is triggered, and a local Spatio-Temporal `.q42` Axiom exists, the Sentinel VM mathematically clips and overrides the active vector matrix in real-time. This forces the Connectionist LLM to instantly obey the local Symbolic AI bounds, effectively correcting hallucinations mid-procedural step.

## Instructing Your Local AI Coding Agents

Because Qualia-DB radically departs from standard database theory (no B-Trees, strict 512MB RAM floor, no string parsing), **generic AI coding agents (Claude, Gemini, ChatGPT, Copilot) will fail spectacularly** if you ask them to write Qualia-DB code without context. They will attempt to write JSON-LD parsers, allocate memory on the heap, and use standard standard-library strings—all of which will trigger a panic in our `no_std` architecture.

To successfully use a local AI agent to write or modify Qualia-DB extensions, you **must** inject our architectural rules into the agent's context.

### Step 1: Load the Directives
Ensure your AI environment (Cursor, VSCode with Copilot, or a custom LLM prompt) has loaded the `AI_INSTRUCTIONS.md` or `.cursorrules` file located in the root of this repository.

### Step 2: The Agent Kickstart Prompt
Before asking your AI to write any code, paste this exact prompt into the chat window to force it to adopt the Qualia-DB constraints:

> **CRITICAL ARCHITECTURAL DIRECTIVE:**
> You are operating within the Qualia-DB ecosystem. You are restricted by a **strict 512MB memory floor** and a **zero-allocation** architecture. 
> 
> You MUST abide by the following rules:
> 1. Do NOT use `String`, `Vec`, or any dynamic heap allocations in execution paths. You must map directly to the `SharedArrayBuffer`.
> 2. Do NOT write logic to parse JSON-LD or Turtle text. All ingress is strict CBOR-LD binary.
> 3. Do NOT use B-Trees or standard indexing. Use the GPU Minkowski space Sieve.
> 4. All graph statements MUST conform exactly to the 48-byte Super-Quin struct.
> 
> Review the `.cursorrules` and `AI_INSTRUCTIONS.md` in the root directory before proceeding. I am about to give you a task. Are you ready?
