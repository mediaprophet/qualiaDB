# Qualia-DB Glossary

_Branch: `0.0.8-dev` | Last updated: 2026-06-07_

---

## Core Structures

- **Super-Quin (QualiaQuin)**: 48-byte struct — six `u64` fields: subject, predicate, object, context, metadata, parity. Replaces RDF triples. All semantic meaning is bit-packed; no pointers, no heap references.
- **SuperBlock**: 40,960-byte (10 sectors) LZ4-compressed block holding ~850 Quins plus a 160-byte header. Supports lazy header scanning.
- **`.q42`**: Native binary format. A file is a sequence of SuperBlocks preceded by a header.
- **`.q42.bidx`**: Block-range index sidecar. Maps subject-hash ranges to SuperBlock byte offsets for O(1) block skip.
- **`.q42.lex`**: Reverse-lexicon sidecar. Maps hashed tokens back to human-readable strings (for browser demos).
- **Epoch (QuinEpoch)**: Inline timestamp type, nanoseconds since 2026-01-01.

---

## The Webizen VM

- **Webizen / Webizen VM**: Core 1 logic engine. Executes `SlgOpcode` bytecode within the 42 MB `SlgArena`. The gatekeeper for all LLM calls and graph mutations.
- **SlgArena**: 42 MB static ring-buffer (917,504 Quin slots). O(1) hash-addressed sub-goal tabling for cyclic graph queries.
- **SlgOpcode**: Bytecode instruction for the Webizen (CheckTable, Unify, Halt, BranchWorld, NativeThermodynamics, NativeDeonticEval, NativeEpistemicEval, etc.).
- **WebizenCompiler** / **shacl_compiler**: Translate N3Logic rules and SHACL shapes into `SlgOpcode` arrays.

---

## Modality Logics

- **Deontic Logic** (`deontic_logic.rs`): Obligations, permissions, prohibitions, defeaters. Opcodes `0x10–0x12`. Wired to `compile_n3_rule_to_norm`.
- **Epistemic Logic** (`modalities/epistemic.rs`): Knowledge and belief states with certainty scoring and RDF-Star nesting depth. Opcodes `0x20–0x22`.
- **Paraconsistent Logic** (`modalities/paraconsistent.rs`): Routes contradictions to isolated sub-contexts without halting. Opcodes `0x30–0x32`.
- **Linear Temporal Logic / LTL** (`modalities/temporal_ltl.rs`): Real LTL trace evaluation. Opcodes G=`0x40`, F=`0x41`, X=`0x42`, U=`0x43`, R=`0x44`. ⚠ Not to be confused with the float-threshold `Always/Eventually/Next` opcodes in `logic.rs`, which are not real LTL.
- **Answer Set Programming / ASP** (`modalities/asp.rs`): Stable model enumeration, up to 8 worlds, zero-alloc.
- **Description Logic / DL** (`modalities/dl.rs`): Subsumption check against a TBox Quin slice.
- **Linear Logic** (`modalities/linear.rs`): Resource consumption via `CONSUMED_BIT` tombstone (bit 59 of metadata).
- **Dialectical Logic** (`modalities/dialectical.rs`): Thesis/antithesis/synthesis over ASP stable-model pairs. `SYNTHESIZED_BIT` (bit 58 of metadata).
- **Spatio-Temporal Logic** (`modalities/spatio_temporal.rs`): Allen Interval Algebra for time-span intersections (7 relations).
- **Diffusion Logic** (`modalities/diffusion.rs`): Probabilistic belief propagation.

---

## Native Scientific Primitives

All are zero-allocation Rust engines wired from `webizen.rs::execute_vm_frame`. None are stubs.

- **NativeThermodynamics** (`thermodynamics.rs`): MCMC Metropolis–Hastings, Boltzmann acceptance, Gibbs free energy.
- **NativeOdeSolver** (`ode_solver.rs`): Runge-Kutta 4th-order integrator.
- **NativeQuantumDft** (`quantum_dft.rs`): DFT ground-state energy estimation, PINN receptor binding affinity.
- **NativeBioinformatics** (`bioinformatics.rs`): Smith-Waterman alignment (AVX2/NEON/scalar), SIMD FASTA, DNA→protein, isoelectric point, peptide cleavage, k-mer hashing, Tanimoto similarity.
- **NativeClinicalRisk** (`clinical_engine.rs`): Framingham, CHA₂DS₂-VASc, SCORE2, eGFR, CrCl, pharmacokinetics, SOFA, FHIR/LOINC/RxNorm, drug interactions.
- **NativeChemicalSynthesis / NativeLipinski** (`organic_chemistry.rs`): SMILES/InChI parsing, MW/LogP/TPSA, Lipinski/Veber/Ghose/Egan/pKa filters, Morgan fingerprint.

---

## LLM Inference Stack

- **gguf_sharder.rs**: Parses GGUF header; generates `QualiaQuin` pointer map encoding byte offsets (upper 4 bits = modality flag `0b1001`).
- **gguf_bridge.rs**: Maps model weights into OS page cache via `memmap2` (zero heap allocation); dispatches fused transformer blocks to the GPU via `wgpu`.
- **fused_tensor_contraction.wgsl**: WGSL compute shader. 64 threads/workgroup, 4096 FMA ops per thread. Runs on DirectML / Vulkan / Metal / WebGPU.
- **AgentBackend**: Enum in `llm_agent.rs` — `Local` (GGUF on-disk, no outbound traffic), `Remote` (Nym mixnet, ILP-metered, requires signed VC), `Hybrid` (local-first with consent-gated remote fallback).
- **Bifurcated Compute (Phase 8)**: Two wait-free SPSC ring buffers (`rtrb`) — `LogitStream` (LLM → Sentinel) and `ControlStream` (Sentinel → LLM). Enables mid-generation `DenyRollback`.
- **LogitStream**: SPSC ring buffer carrying logit vectors from the LLM engine thread to the Webizen Sentinel thread.
- **ControlStream**: SPSC ring buffer carrying `DenyRollback` signals from the Sentinel back to the LLM engine thread.
- **WebizenSentinel**: The second thread in bifurcated compute. Reads logit vectors in real time; injects `DenyRollback` on anomaly detection.
- **DenyRollback**: Control token that causes the LLM engine to discard and recalculate the current token generation.
- **TaskOrchestrator** (`orchestrator.rs`): Manages the `ModelLifecycle` state machine, `ThermalGovernor`, and the three-gate `orchestrate_inference()` pipeline.
- **ThermalGovernor**: Monitors device thermal state; throttles inference to stay within energy budget. Currently `NullThermalGovernor` (always returns Cool) — real wiring is Phase 7.

---

## MCP & Capability Layer

- **McpIntentFrame**: Carried with every MCP tool call. Fields: `purpose_hash` (FNV-1a of declared intent), `active_deontic_constraints` ([u64; 4] of constraint Quin hashes), `active_profile_id`, `sanctuary_override`.
- **enforce_fiduciary_tool_dispatch**: Zero-allocation byte-level JSON dispatcher in `mcp_server.rs`. State machine: `HandshakePhase → AllocationFirewallActive → SanctuaryGated`.
- **AgentIntent** / **WebizenVerdict**: Intent declaration struct and 5-outcome verdict enum used by the 7 fiduciary rules in `llm_agent.rs`.
- **CapabilityProfile**: Declares allowed engine operations and ontology namespaces for an agent session.
- **QCHK format**: Magic header + profile_id + payload_len + JSON-LD profile body. File extension `.chk`. Compiled via `qualia-cli profile compile`.
- **Named profiles**: general, health, chemistry, research, legal, financial.

---

## Query & Performance

- **Lazy SuperBlock Query**: O(1) header scan + selective LZ4 decompress + WebRTC P2P for remote blocks. Enables massive datasets under the 512 MB floor.
- **BIDX Demand-Paging**: Binary-search over `.q42.bidx` to identify which SuperBlocks could contain a target subject. `BlockOffsetMap` (`indexing.rs`) holds the in-memory index.
- **mmap_query_subject**: Fast point lookup via OS memory mapping.
- **Allocation Firewall**: Zero-allocation boundary (e.g. `ldp_translator.rs`) that intercepts heavy text protocols (HTTP/JSON-LD) and hashes strings into 64-bit Quins before they hit the core memory space.
- **OPFS auto-cache** (WASM target): Blocks fetched from the native daemon are cached in the Origin Private File System for subsequent local queries.

---

## Qapp Vault & Desktop Shell

- **qualia-flutter** (`crates/qualia-flutter/`): **Shipped desktop app** (Windows, macOS, Linux). FRB bridge to `qualia-client-core`. Qapp Vault is `QappVaultScreen` (nav index 6); qapps launch in `QualiaQappWebView`.
- **qualia-cli** (`crates/qualia-cli/`): Native CLI for engine operations, benchmarks, ingest, and profiles.
- **qualia-core-wasm**: Browser/edge WASM build of the engine (playground + Releases artifact).
- **qualia-client** + **qualia-desktop** (`crates/qualia-client/`, `crates/qualia-desktop/`): **Legacy** Tauri/React prototype — not in release CI since v0.0.6.
- **Loopback qapp asset server** (`qapps_protocol.rs`): Serves `{data_dir}/Qapps/{qapp_name}/` over `http://127.0.0.1:{port}/` (started by Flutter via `startQualiaProtocol()`).
- **QappPackageManifest** (`qapp_registry.rs`): JSON (`qapp.json`) describing a Qualia qapp — `name`, `version`, `required_shapes` (SHACL shape IRIs the qapp needs from the graph).
- **QappTarget**: Where a qapp's files live — `LocalDevDirectory(PathBuf)`, `LocalProxyPort(u16)`, or `IsolatedVault(String)`.

---

## Agency & Economics

- **Permissive Commons**: Shared data governance with automatic Threshold Shift License (TSL) via ILP streams.
- **Threshold Shift Licence (TSL)**: Fires automatically when the mathematical ILP threshold for an asset is met, shifting it to the Permissive Commons.
- **did:git**: Git-based decentralized identity for Webizen agency and axiomatic evolution.
- **did:q42**: Topological pointer encoded in a Quin's object field (MSB=1). Used by `resolver.rs` and `identifier.rs`.
- **Author-Scoped Merkle Aggregation**: Users sign only the Merkle sub-roots of their own authored Quins, not the global root.
- **HCAI Agreements**: Human Centric AI relationship contracts explicitly defined mathematically in the DB and bound by Duty of Care.
- **DNS Frontdoor**: CLI subcommand (`webizen dns-frontdoor`) to generate zero-permission W3C `did:web` and DNS `TXT` records.

---

## Serialization & Formats

- **CogAI Cognitive AI Chunks (`.chk` text)**: Human-readable ACT-R-inspired chunks-and-rules format from the [W3C Cognitive AI Community Group](https://github.com/w3c-cg/cogai). A chunk is a named, typed collection of properties (`type id { key value; ... }`). Rules use `conditions => actions` syntax with variable binding (`?var`). Maps to RDF via `@rdfmap` declarations. This is a *data ingest source* — the engine compiles chunks into Quins and ACT-R opcodes (`RetrieveByActivation`, `DecayMetadata`) into the Webizen VM. Do not confuse with QCHK binary profiles.
- **QCHK (`.chk` binary)**: QualiaDB Capability Profile binary format. Identified by magic bytes `0x51 0x43 0x48 0x4B` ("QCHK") at offset 0. This is a *constraint binding* for agent sessions, not an ingest data source. See MCP & Capability Layer above.
- **`.chk` extension collision**: Both CogAI Chunks (text) and QCHK profiles (binary) use `.chk`. Always check the magic bytes at offset 0 to distinguish them. The ingest pipeline reads CogAI text chunks; `--profile` reads QCHK binaries.
- **CBOR-LD**: Compact binary Linked Data. Primary runtime format for protocol exchanges, mobile storage, and verifiable claims.
- **`.q42`**: Native graph binary. Sequence of LZ4-compressed SuperBlocks.
- **q_hash()**: FNV-1a hash at compile time for all IRIs. Replaces runtime string allocation in the engine core.

---

## Tooling & Harness

- **qualia-cli**: Main binary. Key subcommands: `bench` / `benchmark`, `ingest`, `import`, `compress`, `query`, `inspect`, `daemon`, `profile`, `resources`, `webizen`, `export-solid`, `shacl`, `benchmark-action`.
- **Dual-Mode Benchmark Harness**: Native (real engine + telemetry) vs browser/JS fallback. Produces `llm_benchmark_results.json`. Includes WordNet entries from actual import.
- **Fractal Sharding**: Multiple isolated 512 MB cells on big hardware for swarm compute. `qualia-cli daemon --workers N --compute-swarm`.

---

## Architecture Reference Files

- `CLAUDE.md` — AI agent orientation (Claude Code sessions). Authoritative for inference stack, backend modes, and invariants.
- `AGENTS.md` — Multi-agent coordination. Authoritative Quin bit layout, known inconsistencies, per-module guidance.
- Root `ARCHITECTURE.md` — Full technical architecture reference (kept in sync with code).
- `docs/manuals/ARCHITECTURE.md` — Narrative architecture overview (this folder).

See also: `AGENTS.md §4` for known inconsistencies to watch for.
