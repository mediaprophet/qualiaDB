# QualiaDB `.q42` Architecture Review: The Stellar Synthesis

**Date:** 2026-06-21
**Purpose:** A definitive architectural synthesis and strategic vector for the QualiaDB ecosystem. This document reconciles the "First Light" empirical achievements of the WASM/native inference engine with the "Stellar" neuro-symbolic and multi-modal physics endgame. It serves as the ultimate blueprint for realizing exceptional, human-centric computational sovereignty.

---

## 1. The Architectural Reality: Mechanical Sympathy Achieved
The most critical finding from the codebase and empirical diagnostics (`WASM_LLM_ENDGAME`, `WASM_LLM_ROADMAP`) is that **the zero-heap, 48-byte Super-Quin architecture is mathematically and practically sound.** 

*   **The WASM-LLM is Not a Dead End:** It is a proven, mathematical triumph of mechanical sympathy. The assumption that browser-based WebGPU inference cannot scale is structurally false for this engine. By resolving the IPC queue-submit bottleneck (routing Q/K/V projection through the parallel GEMM), the engine achieved **5.9 tok/s** and a Time-To-First-Token (TTFT) of **~3.9s** on SmolLM2-360M. It achieved this without relying on external libraries (like Ollama or `candle`), operating strictly within the 42MB `SlgArena` and `wgpu` boundaries.
*   **The True Blocker is an Index Math Defect:** The current `RuntimeError: memory access out of bounds` during prefill (`WASM_LLM_INFERENCE_DIAGNOSIS`) is an isolated index-math defect (likely in `dispatch_attention_layer`), not a fundamental WebGPU or Wasm limit. 
*   **OPFS Stream-Caching Over V8 Parsing:** To permanently solve frontend crashing on massive models, the browser must never attempt to parse a 2GB GGUF file as a JavaScript string or heap object. It must be chunked directly into the Origin Private File System (OPFS), allowing the engine to execute zero-allocation block reads.

**Strategic Imperative: Embrace the Symbiosis of WASM and Swarm capacities.** Retain the custom `QTensorEngine` and strictly enforce the **bifurcated compute model**:
*   **The WASM Layer (Universal Gateway & Edge Sovereign):** Operates as the "Allocation Firewall" and logic sieve. The engine relies on a **Bifurcated Bootloader Pipeline** (JS Feature-Detection Sieve) to dynamically target the host's memory capabilities:
    *   **Wasm64 (The Unchained Sandbox):** On modern engines supporting the experimental `memory64` proposal, the WASM client shares the exact 64-bit mathematical topology as the Native Rust Daemon. It maps massive `<8B` `.q42` tensor files straight into linear memory, drastically reducing I/O bottlenecks for blazing-fast WebGPU kernel feeds.
    *   **Wasm32 (The 512MB Floor / Demand Paging):** On standard browsers bound by the 4GB ceiling, the engine gracefully falls back to 32-bit execution. Here, the engine leverages **OPFS Demand Paging** (rigorously aligning `.q42` opaque blocks to OS 4KB/16KB virtual memory pages) to sequentially stream tensor blocks into the 42MB `SlgArena` at the precise microsecond of use.
    *   *Mechanical Sympathy Warning:* The availability of Wasm64's "larger desk" is not permission for sloppy code. The zero-heap invariants, explicit vector alignments, and 48-byte Quins remain absolute constraints across both paths.
*   **The Native "Swarm" Daemon (Unbound Physics & Compute):** The extreme heavy lifting of Large Physics Models, unbounded continuous mathematical solvers, and massive-scale inference (>8B parameters) is exclusively routed to the native desktop/edge Swarm install. Here, the engine bypasses the browser sandbox entirely, utilizing `memmap2` for zero-copy NVMe-to-VRAM streaming and distributing logic natively across the CPU, GPU, and NPU.

## 2. The Semantic Brainstem: Neuro-Symbolic Binding
The engine’s unique value proposition is not generating text—it is grounding statistical computation in formal logic. The transition from a text-generator to a reasoning engine hinges on the `.q42` Ahead-of-Time (AOT) ingestion compiler.

*   **Lexicon-Bound Decoding:** The flat tokenizer vocabulary must be ingested and compiled into a **CBOR-LD Ontological Header** embedded directly within the `.q42` file. 
*   **Hardware-Level Deontic Taint Masking:** By binding tokens (e.g., SNOMED, FIBO) directly to the 48-byte Super-Quin metadata fields (e.g., `Q42_META_DEONTIC_TAINT`), the WGSL shaders enforce ODRL/SHACL compliance *at the silicon level*. If the statistical sequence violates a user's spatio-temporal constraints or Guardianship contracts, the tensor's weight is mathematically driven to zero in the shader before it ever becomes text. No heap allocations (`Vec`, `String`) are permitted in this hot path.

## 3. The Phenomenal Horizon: From LLMs to Large Physics Models
The true differentiation of the Qualia architecture (`advanced-q42-llm-ideas.md`, `qualia-llm-future-updates.md`) is its rejection of the "Vision-Language Model" paradigm, which flattens physical reality into vocabulary. The Qualia engine is a **Constraint-Satisfying Geometric Solver**.

*   **10D Volumetric Tensors into 5D NQuins:** A monolithic LLM expands into a 10D phase space, which Qualia elegantly folds into the fixed 48-byte stride: `⟨Subject, Predicate, Object, Context, Manifold-Coordinate⟩`. The Manifold-Coordinate natively hosts temporal asymmetry ($t\Delta$), physical momentum ($P$), and Deontic states ($D$).
*   **Multimodality as Physics:** Audio is not tokenized; it is ingested as acoustic manifolds (STFT/CQT). Visual data (camera, LIDAR, Wi-Fi CSI) is ingested as thermodynamic and spatial gradients. Through **Eulerian Video Magnification (EVM)**, the engine amplifies sub-pixel phase shifts to detect micro-vibrations, pulse rates, and structural integrity directly at the hardware layer.
*   **Projective Geometric Algebra (PGA):** CAD, 3D printing, and photogrammetry assets are mapped as multivectors ($\alpha+v+B+T$). The engine does not guess geometry probabilistically; it mathematically verifies structural tolerance and kinematic constraints before generating output.

## 4. Federated Energy-Opportunism & Epistemic Provenance
To support multi-agent collaboration (e.g., decentralized biology experts or edge communities), the engine must view energy and trust as fundamental computational primitives.

*   **LoRA-CRDT over WebTorrent:** Large synchronized training is impossible at the edge. The system will use frozen `.q42` base models and gossip tiny (15–50MB) LoRA adapters as Conflict-Free Replicated Data Types (CRDTs). The WebGPU shader is designed to mathematically fold these commutative geometric layers into the base manifold on the fly.
*   **Epistemic Provenance (Guild Collaboration):** To prevent statistical poisoning, every LoRA delta is cryptographically signed with the author's W3C Decentralized Identifier (DID). Before the shader applies a weight update, the Prolog Sentinel processes an ODRL/SHACL logic gate verifying the author's standing in the expert guild. Users do not just download math; they download cryptographically proven cognitive provenance.
*   **Hardware Telemetry State Machine:** The dispatcher (`daemon_swarm.rs`) operates a hardware abstraction layer acting as a smart DC-DC converter. During *Deficit* (battery power), background training halts and inference drops to 1.58b. During *Equilibrium*, standard inference resumes. During *Surplus* (off-grid solar peak), the daemon silently awakens to compute LoRA gradients natively in the WebGPU shader, transforming excess kinetic energy into verified cognitive infrastructure.

## 5. Execution Strategy: The Path to "Stellar"
To realize this exceptional vision, development must strictly follow this sequence:

1.  **V1 First Light (Immediate):** Debug the `memory access out of bounds` in the WASM prefill chunk and correct the HuggingFace deployment 404s. Prove the zero-heap architecture publicly with a working, coherent Lane A (WASM) demo in the browser.
2.  **Parallel AOT Ingestion & OPFS Distribution:** Build the WASM pipeline to ingest high-fidelity (Q8_0 or F16) files via chunked Web Workers. By centrally hosting these pre-compiled, OS-paged `.q42` files, end-users can stream them directly into their Origin Private File System (OPFS), dropping their Time-To-First-Token (TTFT) to effectively zero.
3.  **AOT Compiler Evolution (Performance):** Upgrade the `.q42` compiler to perform the **Concentration-Alignment Transform** (embedding activation scaling factors for lossless W4A4 execution), down-sampling non-critical pathways into **Ternary (BitNet 1.58b)** FFN matrices and **KIVI asymmetric KV-caches**. Include **Zero-Copy Speculative Decoding** mapping a draft and target model simultaneously via `mmap` for extreme TPS amplification natively on the Swarm daemon.
4.  **Neuro-Symbolic Gateways:** Deploy the CBOR-LD ontological tokenizer mapping to execute hardware-level deontic masking. **Crucially, this includes the Egress Gatekeeper: the native daemon inspects the `webizen:SensitivityLabel` bit-mask on every 48-byte Quin. Any data tagged `0x02` (Classified/Private) is mathematically severed from the Nym mixnet and WebTorrent egress bridges, fulfilling the project's promise of an impenetrable Information Fiduciary.**
5.  **Heterogeneous Compute & Physics Integration:** Evolve the `fused_attention.wgsl` into the **Cross-Manifold Fused Kernel**. Solidify the bifurcated framework: the WASM (CPU) handles the deterministic logic, **CRDT suspension queues for multi-agent guardianship workflows**, and Sieve. The Native Swarm daemon leverages WebGPU (GPU) for **Projective Geometric Algebra (PGA) and acoustic/EMF wave-function physics**, and WebNN (NPU) for dense tensor contractions.

## 6. Fiduciary Developer Protocols (The MCP Firewall)
To allow external systems (and coding agents) to interact with the engine safely, the Model Context Protocol (MCP) Server operates behind a strict Allocation Firewall. For complete architectural details on how multi-agent interaction is governed—including identity, token circuit breakers, and semantic graph updates—see the definitive [Human-Centric Multi-Agent Coordination Specification](docs/manuals/standards/MULTI_AGENT_PROTOCOL.md).
*   **Dual-Strap Stdio:** The MCP server communicates exclusively over stdio multiplexed behind compilation flags.
*   **Zero-Heap Parsing:** JSON-RPC requests are parsed using zero-allocation, byte-slice manipulation.
*   **Absolute Deontic Halts:** If a developer query encounters data tagged `0x02` (Classified) via the `SensitivityLabel`, the engine hard-errors (`ERROR_FIDUCIARY_BOUND_VIOLATION`) or completely strips the quins. There is no "dev override" for classified human data.

## 7. Crisis Realities: Thermal & Intermittent Sympathy
Operating at the edge means surviving hostile physical environments.
*   **Thermal Management:** The native daemon monitors OS thermal states. If device thermals enter a "Warning" state, the Swarm dynamically throttles dense tensor contractions and limits WebGPU thread concurrency, preserving the user's battery and device lifespan.
*   **Intermittent Computing:** The engine must survive sudden power-loss. The Write-Ahead Log (WAL) execution frame utilizes `core::ptr::write_volatile` to flush uncommitted, flattened graph mutations directly to Non-Volatile Memory (NVM) in microseconds.

## 8. Native Cryptographic Identity (`did:q42`)
Identity in QualiaDB is not a String-based HTTP WebID. The network layer relies on `did:q42`, resolving peer identities directly via DNSSEC-backed WireGuard tunnels. Inside the engine, identities are parsed into fixed-size byte arrays (`[u32; 12]`) utilizing `#![no_std]` crates (e.g., `ed25519-dalek`), entirely eliminating string parsing from the unification loop.

**Conclusion:** The QualiaDB architecture is fundamentally sound and mathematically revolutionary. By enforcing the strict bifurcation between the universally accessible, limited WASM sieve and the unbounded, native Swarm compute daemon—treating AI not as a stochastic chatbot, but as a rigid, heterogeneous, neuro-symbolic physics engine bound to human rights—the project achieves the "Stellar" mandate: returning computational sovereignty to the natural person.

---

### System Directive: The Stellar Synthesis (Phase 2B/3)
*(Immediate Execution Mandate for Local Agent / Antigravity)*

> "The WASM-LLM zero-heap architecture is proven. Stop all attempts to bypass or 'mock' the WebGPU inference layer.
> 1. Target `dispatch_attention_layer` and fix the index-math defect causing the OOB trap.
> 2. Implement the Bifurcated Bootloader Pipeline (`memory64` probe) in the frontend. Fall back to OPFS stream-caching (Demand Paging) when Wasm32 is strictly required.
> 3. Ensure the CBOR-LD tokenizer binds directly to the 48-byte Super-Quin metadata fields to enable hardware-level Deontic Taint Masking. No heap allocations are permitted in this hot path.
> **4. Enforce the Egress Gatekeeper in `daemon_swarm.rs`: verify the `SensitivityLabel` byte before any `nym_adapter` or `webtorrent` transmission.**
> **5. Refactor the Sentinel VM to safely push yielding Webizen transactions to the `SuspendedTransactionQueue` as flat byte-arrays when awaiting Guardianship CRDT signatures.**
> **6. Enforce the Allocation Firewall: Ensure all PRs and agent-generated code pass `dhat-rs` micro-benchmarks. Any logic added to `qualia-core-db` that utilizes dynamic heap allocations (`Vec`, `String`, `Box`) will be categorically rejected.**
> **7. Respect the 8MB WASM Stack Limit: When writing logic for the Webizen VM or WASM fallback, all local arrays and arenas must be statically sized to fit safely within the 8MB stack allocation configuration, operating independently of the 4GB linear memory ceiling.**"