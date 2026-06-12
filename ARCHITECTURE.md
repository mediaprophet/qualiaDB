# QualiaDB Architecture

_Branch: `0.0.11` | Last updated: 2026-06-12_

QualiaDB is a zero-allocation, mechanically sympathetic semantic database and multi-agent collaboration ecosystem. It bridges the string-heavy reality of the Semantic Web with hardware-aligned execution paths, enforcing strict constraints to ensure bounded memory and deterministic performance.

---

## 1. Core Principles & Constraints

| Principle | Detail |
|-----------|--------|
| **Zero-Heap in Hot Paths** | No `Vec`, `String`, or `Box` inside evaluator loops. Callers supply fixed-size output buffers (`&mut [T]`) or `[T; N]` stack arrays for local state. |
| **48-Byte Super-Quin** | Every semantic datum fits in a `NQuin` (6 × `u64`: subject, predicate, object, context, metadata, parity). Hashes and bit-packing replace string pointers entirely. |
| **42 MB Prolog Sentinel** | Any single execution pass must stay within 42 × 1024 × 1024 bytes. `SlgArena` enforces this structurally (917,504 Quin slots). |
| **512 MB Edge Floor** | Total system — graph engine, Webizen VM, LLM runtime, and all caches — must stay within 512 MB. Hard design target for personal-device deployment. |
| **Deterministic, Non-Recursive** | No unbounded recursion. LTL/ASP evaluators iterate over slices; they never call themselves. |
| **q_hash for all URIs** | All string IRIs are FNV-1a–hashed at compile time via `q_hash()` or `q_turtle!`. No runtime string allocation in the engine core. |
| **Opcode Partitioning** | `mini_parser.rs` owns `0x00–0x04`. All modality opcodes start at `0x10+`. See §4 for the full opcode table. |
| **Mechanical Sympathy** | Data layouts are cache-line aligned. Evaluation paths avoid pointer chasing and random memory access. |

---

## 2. Universal Quin Bit Layout

Every `NQuin` is exactly 48 bytes — six `u64` fields. All semantic meaning is encoded via bit-packing; no pointers, no heap references.

```
Field      Bits      Meaning
─────────────────────────────────────────────────────────────────────────────
subject    [63]      MSB flag (interpretation depends on modality)
           [0..62]   Entity / agent DID hash (q_hash of IRI)

predicate  [63]      Modality MSB sentinel (e.g. DEFEATER_BIT in deontic)
           [8..62]   Property-path / property IRI hash (q_hash, shifted left 8)
           [0..7]    u8 opcode (0x10+ for all new modalities)

object     [63]      MSB=1 → did:q42 topological pointer (resolver.rs, identifier.rs)
           [60..62]  Inline type tag when MSB=0 (authoritative: resolver.rs):
                       0b000 = IRI/blank-node (FNV-1a hash in bits 0-59)
                       0b001 = xsd:integer    (value in bits 0-59)
                       0b010 = xsd:decimal    (value × 10⁶ in bits 0-59)
                       0b011 = xsd:boolean    (0 or 1 in bit 0)
                       0b100–111 = reserved
           [0..59]   Payload (IRI hash, integer value, or decimal value)
           ⚠ KNOWN INCONSISTENCY: logic.rs::extract_float uses 0b001<<60 as
             an f32 tag with f32 bits in [0..31]. This conflicts with
             resolver.rs (authoritative). New modules must use resolver.rs
             convention. See §9-A.

context    [56..63]  Sensitivity class (PUBLIC=0, RESTRICTED=1, CLASSIFIED=2)
           [0..55]   Contract / graph / world DID hash

metadata   [61..62]  PermissiveRoutingLane (00=Passthrough, 01=Commons,
                       10=Bilateral, 11=Spatial)
           [32..60]  Lamport logical clock (29 bits, wraps at 0x1FFF_FFFF)
           [0..31]   Modality payload (expiry epoch, confidence weight, etc.)
           [59]      CONSUMED_BIT — linear.rs resource tombstone
           [58]      SYNTHESIZED_BIT — dialectical.rs synthesis marker

parity     [0..63]   XOR fold: subject ^ predicate ^ object ^ context ^ metadata (implemented — NQuin::calculate_parity)
```

**Note on `lexicon.rs`:** `generate_60bit_token` masks hashes to 60 bits (`& 0x0FFF_FFFF_FFFF_FFFF`), explicitly reserving bits 60-63 for type tags. All new modality object values must respect this mask.

---

## 3. Storage Engine

### SuperBlocks (`storage.rs`, `wal.rs`)

The fundamental on-disk unit is the **SuperBlock**: exactly 40,960 bytes (10 × 4,096-byte disk sectors), holding 850 `NQuin`s plus a 160-byte header. SuperBlocks are LZ4-compressed for density.

- **Header-first boot**: On startup only block headers are read. The engine maps the full `.q42` file via `memmap2` and services queries by seeking directly to relevant blocks — non-relevant blocks are never decompressed.
- **WAL (`wal.rs`)**: All mutations are append-only to the Write-Ahead Log before being compacted into SuperBlocks. Conduct violation Quins are written here and signed with Ed25519.
- **Volatile scrubbing**: When a buffer is evicted, `std::ptr::write_volatile` zeroes it to prevent memory-harvesting attacks.

### BIDX Demand-Paging (`.q42.bidx`)

A `.q42.bidx` sidecar file records the subject-hash range covered by each SuperBlock. This enables **O(1) block skip** — the query engine binary-searches the BIDX to identify which blocks could contain a target subject.

- **VFS `BlockOffsetMap`**: In-memory structure mapping block index → byte offset, populated from the BIDX at open time.
- **OPFS auto-cache** (WASM target): Blocks fetched from the native daemon are cached in the Origin Private File System for subsequent local queries.

### Key Storage Types

| Type | File | Purpose |
|------|------|---------|
| `NQuin` | `lib.rs` | 48-byte semantic datum |
| `QualiaSuperBlock` | `storage.rs` | 850-Quin compressed block |
| `QuinIncrementalScanner` | `storage.rs` | Zero-alloc streaming cursor |
| `BlockOffsetMap` | `indexing.rs` | BIDX-backed demand-paging index |

---

## 4. Ingestion Pipeline (`ingest.rs`, `qualia-cli`)

The CLI is the entry point for human-centric, agency-preserving data ingestion
into `.q42` vaults.

- **Formats**: CogAI Cognitive AI Chunks (`.chk` text — W3C CG chunks-and-rules format), CBOR-LD (`.cbor` / `.cbor-ld`), N-Triples, Turtle, JSON-LD, RDF/XML via the Rio streaming parser.
- **Profile-bound ingestion**: `qualia ingest --profile <file>.qchk` binds a `CapabilityProfile` for the ingest session, restricting available opcodes and ontologies.

> ⚠ **Capability envelope migration**: CogAI Chunks remain `.chk` text inputs. QCHK capability envelopes are migrating to `.qchk`; legacy `.chk` QCHK files are compatibility-only and should be renamed over time. The `QCHK` magic bytes (`0x51 0x43 0x48 0x4B`) still identify the binary profile payload.
- **Zero-Allocation Parsing**: Values are hashed directly into Quins using FNV-1a; no intermediate string representation enters the engine.
- **Sort-First Ingestor**: Before SuperBlock emission, Quins are sorted by subject hash for BIDX-indexable output.
- **Multi-Pass External Sorter**: Handles datasets larger than RAM by buffering ~50 MB chunks, sorting, and flushing to disk. A K-way merge emits the final sorted `.q42` stream.
- **BIDX Indexing**: A `.q42.bidx` sidecar is generated alongside every `.q42` file.

---

## 5. Webizen VM & Modality Logics

The Webizen VM interprets SHACL constraints and N3Logic rules through integrated reasoning modalities, all executing within the 42 MB `SlgArena` envelope.

### Opcode Allocation

| Range | Owner |
|-------|-------|
| `0x00–0x04` | `mini_parser.rs` — reserved |
| `0x10–0x12` | Deontic Logic |
| `0x20–0x22` | Epistemic Logic |
| `0x30–0x32` | Paraconsistent Logic |
| `0x40–0x44` | Linear Temporal Logic (LTL) |

### N3Logic Rule Types (`n3_parser.rs`)

| Arrow | `RuleType` | Semantics |
|-------|-----------|-----------|
| `=>` | `Strict` | Classical modus ponens — forward chaining |
| `~>` | `Defeasible` | Can be overridden by a Defeater |
| `^>` | `Defeater` | Overtly defeats a matching Defeasible rule; maps to `DEFEATER_BIT` in deontic Quins |
| `-o` | `Linear` | Linear logic: premise is consumed on firing |

Also parses: `#asp {}` blocks → `N3Event::AspBlock`, `qualia:diffuse {}` → `N3Event::DiffuseBlock`. Rule weights: optional float prefix `(0.8) { premise } ~> { conclusion }`.

### Modality Logic Modules

#### Deontic Logic (`deontic_logic.rs`) — ✅ Complete (10/10 tests)
Obligations, permissions, prohibitions, and defeaters.

```
OP_OBLIGATE  = 0x10
OP_PERMIT    = 0x11
OP_FORBID    = 0x12
DEFEATER_BIT = 1u64 << 63   (in predicate field)
```

**N3 → Deontic Bridge** (`compile_n3_rule_to_norm`): Compiles N3 `Rule` structs into norm Quins. `^>` (Defeater) rules set `DEFEATER_BIT`. Opcode selection:

| N3 Rule Type | Predicate keyword | Opcode |
|---|---|---|
| `Strict` | `obligate/must/shall` | `OP_OBLIGATE` |
| `Defeasible` | `permit/may/can` | `OP_PERMIT` |
| `Defeasible` | `forbid/not/prohibit` | `OP_FORBID` |
| `Defeater` | any | `OP_PERMIT` + `DEFEATER_BIT` |
| `Linear` | `obligate` | `OP_OBLIGATE` |

#### Epistemic Logic (`modalities/epistemic.rs`) — ✅ Complete
Knowledge and belief states with certainty scoring and RDF-Star nesting depth.

```
OP_KNOWS            = 0x20
OP_BELIEVES         = 0x21
OP_COMMON_KNOWLEDGE = 0x22
```

Quin layout for epistemic Quins:
- `predicate[0..7]` — opcode (0x20–0x22)
- `predicate[8..15]` — certainty u8 (0–255 maps to 0.0–1.0)
- `predicate[16..19]` — nesting_depth u4 (RDF-Star depth)
- `object` — claim_fingerprint (subject^predicate^object of nested claim)
- `context` — q_hash(epistemic_world_did) — which possible world

Key function:
```rust
pub fn evaluate_epistemic_frame(
    quins: &[NQuin],
    agent_did_hash: u64,    // 0 = accept all agents
    world_hash: u64,        // 0 = accept all worlds
    out: &mut [EpistemicVerdict],
) -> Result<usize, EpistemicError>
```

#### Linear Temporal Logic (`modalities/temporal_ltl.rs`) — ✅ Complete
Correct LTL trace evaluation (not the float-threshold opcodes in `logic.rs`).

```
OP_LTL_GLOBALLY = 0x40   // G(φ) — φ at every position
OP_LTL_FINALLY  = 0x41   // F(φ) — φ at some position
OP_LTL_NEXT     = 0x42   // X(φ) — φ at position i+1
OP_LTL_UNTIL    = 0x43   // φ U ψ — φ holds until ψ
OP_LTL_RELEASE  = 0x44   // φ R ψ — ψ holds unless φ releases it
```

⚠ **Important**: The `Always/Eventually/Next` opcodes in `logic.rs` compare a float threshold on a *single Quin's object field* — they are NOT real LTL operators. Use `evaluate_ltl_trace` from `temporal_ltl.rs` for temporal reasoning.

#### Paraconsistent Logic (`modalities/paraconsistent.rs`) — ✅ Complete
Routes contradictions to isolated sub-contexts without halting the system.

```
OP_ISOLATE              = 0x30
OP_CONTRADICTION_SCORE  = 0x31
OP_PARACONSISTENT_MERGE = 0x32
ISOLATED_CONTEXT_PREFIX = q_hash("q42:isolated")
```

Contradiction detection: Two Quins in the same `context` are contradictory if they share the same `subject` + `predicate` but have different `object` values. The second-arriving Quin is re-contextualized to `ISOLATED_CONTEXT_PREFIX ^ original_context`.

Wired to `PermissiveRoutingLane::EnforceBilateralMicroCommons` for vulnerable-user intake paths.

#### Answer Set Programming (`modalities/asp.rs`) — ✅ Complete
Stable model enumeration (up to 8 worlds), zero-alloc.

```rust
pub const MAX_STABLE_MODELS: usize = 8;
pub fn enumerate_stable_models(
    base: &NQuin,
    rules: &[NQuin],
    out_worlds: &mut [u64; MAX_STABLE_MODELS],
) -> usize
```

Worlds encoded as context-hash variants: `world_i_context = base_context ^ (i as u64)`.

#### Description Logic (`modalities/dl.rs`) — ✅ Complete
Subsumption check against a TBox stored in a Quin slice.

```rust
pub fn check_subsumption_quin(
    sub_class_hash: u64,
    super_class_hash: u64,
    tbox: &[NQuin],   // Quins with predicate = q_hash("rdfs:subClassOf")
) -> bool
```

#### Linear Logic (`modalities/linear.rs`) — ✅ Complete
Resource consumption via tombstone mechanism (no heap allocation).

```rust
pub const CONSUMED_BIT: u64 = 1u64 << 59;
pub fn consume_quin(q: &mut NQuin)
pub fn is_consumed(q: &NQuin) -> bool
```

#### Dialectical Logic (`modalities/dialectical.rs`) — ✅ Complete
Thesis/antithesis/synthesis over ASP stable-model pairs.

```rust
pub const SYNTHESIZED_BIT: u64 = 1u64 << 58;
pub fn synthesize_dialectical(
    thesis: &NQuin,
    antithesis: &NQuin,
) -> Option<NQuin>
```

Synthesis context = `thesis_context ^ antithesis_context`; `SYNTHESIZED_BIT` set in metadata.

#### Spatio-Temporal Logic (`modalities/spatio_temporal.rs`) — ✅ Complete
Allen Interval Algebra for time-span intersections: Before, Meets, Overlaps, Starts, During, Finishes, Equals (7 relations).

#### Cognitive AI / ACT-R (`shacl_compiler.rs`, `webizen.rs`) — ⚠ Partial
Opcodes for the [W3C CogAI Community Group](https://github.com/w3c-cg/cogai) ACT-R-inspired chunks-and-rules model:

```
NativeRetrieveByActivation  — stochastic weighted chunk retrieval
NativeDecayMetadata         — activation decay over time
NativeUnless                — non-monotonic default logic
```

SHACL properties: `qualia:retrieveByActivation`, `qualia:decayMetadata`. These opcodes are compiled by `shacl_compiler.rs` and wired in the `execute_vm_frame` dispatch. They execute inline on Core 1 as ACT-R activation and decay operations. The CogAI `.chk` chunks-and-rules text format is a supported ingest source; see §4 for the `.chk` extension disambiguation.

### SHACL Compiler (`shacl_compiler.rs`)

`compile_shacl_to_webizen` translates SHACL shapes into `WebizenOpcode` bytecode arrays. Supports the full standard vocabulary plus domain-specific extensions:

| SHACL Constraint | Compiled to |
|---|---|
| `qualia:thermoMetropolisStep` | `NativeThermodynamics` |
| `qualia:solveOdeDynamics` | `NativeOdeSolver` |
| `qualia:dftGroundState` | `NativeQuantumDft` |
| `qualia:bioSequenceAlignment` | `NativeBioinformatics` |
| `DeonticObligate/Permit/Forbid` | `NativeDeonticEval` |
| `DeonticNotExpired { now_unix }` | `NativeDeonticEval` with expiry check |
| `EpistemicKnowledge { min_certainty }` | `NativeEpistemicEval(u8)` |
| `EpistemicBelief { min_certainty }` | `NativeEpistemicEval(u8)` |
| `CommonKnowledge` | `NativeEpistemicEval(0xFF)` |

### SlgArena (`webizen.rs`)

42 MB ring buffer with 917,504 Quin slots. O(1) hash-addressed sub-goal tabling. All `SlgOpcode::Native*` variants are fully wired — not stubs:

- `NativeThermodynamics` → `thermodynamics.rs` MCMC
- `NativeOdeSolver` → `ode_solver.rs` RK4
- `NativeQuantumDft` → `quantum_dft.rs` DFT ground state + PINN
- `NativeBioinformatics` → `bioinformatics.rs` Smith-Waterman + SIMD
- `NativeClinicalRisk` → `clinical_engine.rs` risk scoring
- `NativeChemicalSynthesis` → `organic_chemistry.rs` SMILES/InChI
- `NativeLipinski` → `organic_chemistry.rs` ADMET filters
- `NativeEpistemicEval` → `epistemic.rs` belief evaluation
- `NativeDeonticEval` → `deontic_logic.rs` norm evaluation

---

## 6. Native Scientific Primitives

All scientific modules are fully wired native engines called from `webizen.rs::execute_vm_frame`. **These are not stubs.**

| Module | File | Capabilities |
|--------|------|-------------|
| Clinical Engine | `clinical_engine.rs` | Framingham risk, CHA₂DS₂-VASc, SCORE2, eGFR, CrCl, pharmacokinetics, SOFA, FHIR/LOINC/RxNorm, drug interactions, contraindications |
| Bioinformatics | `bioinformatics.rs` | Smith-Waterman alignment (AVX2/NEON/scalar), SIMD FASTA, DNA→protein, isoelectric point, peptide cleavage, k-mer hashing, Tanimoto similarity |
| Organic Chemistry | `organic_chemistry.rs` | SMILES/InChI parsing, MW/LogP/TPSA, Lipinski/Veber/Ghose/Egan/pKa filters, Morgan fingerprint, Arrhenius/Gibbs/Henderson-Hasselbalch, isotope distribution, E-factor, atom economy |
| Thermodynamics | `thermodynamics.rs` | MCMC Metropolis–Hastings, Boltzmann acceptance, Gibbs free energy |
| Quantum DFT | `quantum_dft.rs` | DFT ground-state energy estimation, PINN receptor binding affinity |
| ODE Solver | `ode_solver.rs` | Runge-Kutta 4th-order integrator |
| Geometric | `geometric.rs` | Lorentz vector mapping, tropical (Min-Plus) distance, homological sieve |
| Spatial Sieve | `spatial_sieve.rs` | GPU-accelerated Quin filtering via NETS (Non-Euclidean Tropical Sieve), returns 27-u32 bitmask over 850 Quins |

---

## 7. MCP Intent Frame Mediation Layer

### MCP Server (`mcp_server.rs`)

The Model Context Protocol server exposes the graph engine to AI agent tools via a zero-deserialization byte-level JSON dispatcher.

**State machine**: `HandshakePhase → AllocationFirewallActive → SanctuaryGated`

**`McpIntentFrame`** — carried with every tool call:
- `purpose_hash: u64` — FNV-1a hash of declared intent
- `active_deontic_constraints: [u64; 4]` — up to 4 constraint Quin hashes
- `active_profile_id: u64` — `q_hash("profile:*")` of bound `CapabilityProfile`
- `sanctuary_override: Option<[u8; 32]>` — cryptographic override token

**`enforce_fiduciary_tool_dispatch`** — zero-allocation byte-level dispatch using raw byte matching over incoming JSON (no serde). Currently wired tools:
- `query_graph` — requires valid `sanctuary_override` token; blocked without one
- `inject_test_quin` — routes directly to paraconsistent isolation lane

**Sanctuary gate**: If `query_graph` is called without a valid override token, a conduct violation `NQuin` is immediately written to the WAL and signed. Buffer scrubbing via `write_volatile` after each dispatch.

---

## 8. LLM Agent Layer (`llm_agent.rs`)

The LLM engine is a **native, in-process GGUF inference stack** — not Ollama, not a llama.cpp HTTP server, not any external daemon. No Python runtime involved.

### Agent Intent & Verdict

**`AgentIntent`** carries:
- `intent_predicate: u64` — hash of declared intent
- `requested_graph_scope: u64` — target graph context hash
- `requires_network: bool`
- `mcp_intent_frame_hash: u64` — must match an active `McpIntentFrame`
- `active_profile: u64` — bound `CapabilityProfile` ID

**`WebizenVerdict`** — five outcomes:
```rust
Permit
Deny
DenyWithExplanation(u64)      // explanation Quin hash
RequireReconfirmation
Sanitised                      // output was modified to remove unsafe content
```

### Seven Fiduciary Rules

| Rule | Enforcement |
|------|------------|
| No outbound (local mode) | `requires_network == true` in `Local` backend → `Deny` |
| No sanctuary access | `requested_graph_scope` intersects sanctuary context → `Deny` |
| Token cost guard | Accumulated FLOPS exceeds budget → `Deny` |
| Remote consent | `Remote` backend requires signed VC from Principal → `RequireReconfirmation` |
| Adversarial conduct | Detected → conduct Quin written to WAL ledger → `DenyWithExplanation` |
| Intent frame alignment | `mcp_intent_frame_hash` must match active frame → `Deny` |
| Profile masking | Output tokens not in profile's lexicon are masked → `Sanitised` |

### Backend Modes

- **`Local`** — GGUF on-disk, wgpu inference, no outbound traffic, 128 MB RAM cap.
- **`Remote`** — API call routed through Nym mixnet, ILP micropayment metered, requires signed VC.
- **`Hybrid`** — local-first, graceful fallback to Remote with explicit consent.

### Bifurcated Compute (Phase 8)

During token generation two wait-free SPSC ring buffers (`rtrb`) run in parallel — an LLM Engine thread streaming logits and a Webizen Sentinel thread that can inject a `DenyRollback` mid-generation at the token level. This is the implemented governance architecture; it is not optional middleware and must not be replaced with a simple `generate() -> String` wrapper.

### Native-First WASM-LLM Offloading

When `qualia-core-db` is compiled to the `wasm32-unknown-unknown` target, `infer_local_model_streaming` acts as an interceptor. Instead of running synchronous inference inside the single-threaded WASM sandbox (which would block the UI event loop), it probes for the Native Qualia Daemon via the `ExtensionBus` (`ws://127.0.0.1:4242`). 
- **Non-Blocking Execution**: If the daemon accepts the `did:q42` handshake, the intent is forwarded, and the synchronous trait returns immediately. 
- **Asynchronous Token Routing**: Inference tokens are asynchronously streamed back into the Dioxus or generic UI state via the captured `F: 'static` closure.
- **WebGPU Fallback**: If the daemon is unreachable, the execution falls back to the in-browser WebGPU engine (subject to RAM constraints).

---

## 9. Capability Profiles (`profiles.rs`, `resource_catalog.rs`)

### CapabilityProfile

Provides a declarative allow-list constraining what the LLM and Webizen VM are permitted to do within a session.

```rust
pub struct CapabilityProfile {
    pub profile_id: u64,                  // q_hash("profile:*")
    pub active_engines: &'static [SlgOpcode],  // allow-list mask
    pub loaded_ontologies: &'static [u64],     // namespace hashes
    pub preferred_backend: AgentBackend,
    pub permitted_intent_frames: &'static [u64],
}
```

### Known Profile IDs

| Profile | `q_hash(...)` key | Permitted Engines |
|---------|-------------------|-------------------|
| `profile:general` | — | No restriction |
| `profile:health` | — | `NativeClinicalRisk`, `NativeBioAlignment` |
| `profile:chemistry` | — | `NativeChemicalSynthesis`, `NativeLipinski` |
| `profile:research` | — | All scientific, no financial |
| `profile:legal` | — | `OP_OBLIGATE`, `OP_FORBID`, `OP_PERMIT` |
| `profile:financial` | — | ILP opcodes, tax schema, audit trail |

### QCHK Binary Format

Capability profiles are compiled to `.qchk` files for efficient loading:

```
Offset  Size  Field
0       4     Magic: 0x51 0x43 0x48 0x4B ("QCHK")
4       8     profile_id (u64 little-endian)
12      4     payload_len (u32 little-endian)
16      N     JSON-LD payload (UTF-8, payload_len bytes)
```

### Resource Catalog (`resource_catalog.rs`, `resources/`)

Three resource types, each serializable to Quins via `to_quins()`:

- **`LLMResource`** — GGUF models with provenance, size, quantization metadata
- **`OntologyResource`** — RDF namespaces with SHACL validation hooks
- **`SPARQLResource`** — federated query endpoints with reliability metadata

YAML catalogs:
- `resources/catalog.yaml` — master index
- `resources/llms.yaml` — Phi-3-mini, Gemma 2, Qwen2.5, Llama 3.2, Mistral, DeepSeek, CodeGemma + others
- `resources/ontologies.yaml` — PROV-O, SNOMED CT, MeSH, OBO, Schema.org, Dublin Core, SKOS, Wikidata, DBpedia + domain-specific
- `resources/sparql_endpoints.yaml` — Wikidata, DBpedia, Bio2RDF, UniProt

Download pipeline: YAML catalog → `reqwest` stream → `GGufSharder` → WAL ingest.

---

## 10. Orchestrator (`orchestrator.rs`)

`TaskOrchestrator` provides a sieve around all inference calls:

1. **Pre-validate intent** via `validate_intent(intent)` — Webizen rules check
2. **Execute inference** — dispatches to `AgentRuntime` backend
3. **Post-validate output** — `validate_output(result)` checks provenance grounding
4. **Handle verdicts** — `DenyWithExplanation` logs to WAL; `RequireReconfirmation` suspends frame

Model lifecycle state machine: `Discovered → MappedToDisk → StreamingVRAM → Active → Scrubbing`

`ThermalGovernor` trait: `Cool/Warm/Critical` states — controls 3-core triad parallelism budget. `NullThermalGovernor` always returns `Cool` (real governor not yet wired).

---

## 11. CRDT & Multi-Party Contracts (`crdt.rs`)

Three components for deontic multi-party contract support:

1. **`CrdtResolver::resolve_lww`** — Lamport clock tie-breaking. Concurrent mutations resolved by `object` magnitude. Pure, zero-alloc over `&NQuin`.

2. **`CrdtResolver::verify_delegation`** — Temporal expiry + context-bound check on `DelegatedAccess` grants.

3. **`SuspendedTransactionQueue`** — Fixed 32-slot array. Holds flattened WebizenVM frames waiting for M:N signatures. `apply_consensus_token(quin)` wakes suspended execution when `collected_signatures >= threshold`. This is the mechanism for multi-party deontic contract ratification (e.g. Guardianship consent requiring 2-of-3 parties).

### AgreementDID (`webizen.rs`)

`AgreementDID::compile_to_super_quins()` produces 16 Quins in `EnforceBilateralMicroCommons` routing lane (metadata bit pattern `0x4000_0000_0000_0002`). Uses predicates:
- `q42:hasGuardian` — party → agent relationship
- `q42:hasDomainScope` — agreement → domain
- `q42:requiresConsensus` — M-of-N threshold

This encodes agreement *structure*, not norms. The bridge from `AgreementDID` Quins → deontic norm Quins is handled by `compile_n3_rule_to_norm` in `deontic_logic.rs`.

---

## 12. Agency, Identifiers, Verifiable Credentials & Decentralised Identifiers (`agency.rs`, `identifier.rs`)

W3C standards: [Decentralised Identifiers (DIDs) v1.0](https://www.w3.org/TR/did-core/), [Verifiable Credentials Data Model](https://www.w3.org/TR/vc-data-model/).

- **`compute_scoped_merkle_root(frame, author_did_hash)`** — SHA256 over Quins where `quin.context == author_did`. Zero-alloc via `bytemuck::cast_ref`.
- **`derive_lane_key(pin, salt)`** — currently SHA256-based. Production needs PBKDF2 with ≥ 310,000 iterations (known gap for Sanctuary Mode).
- **`did:q42` pointers** (`identifier.rs`) — `parse_did_q42(b"did:q42:...")` → `u64` with bit 63 always set. FNV-1a over payload then `| (1u64 << 63)`. Routes through MSB dispatch in bytecode VM.
- **`did:web` resolution** — resolves `did:web:domain.example` by fetching the DID Document from `https://domain.example/.well-known/did.json`. Used by `webizen dns-frontdoor` to bind a Webizen identity to a domain name.
- **Verifiable Credentials (VCs)** — credentials are encoded as NQuin graphs: the subject field holds the holder DID hash, the predicate encodes the claim type (`q_hash("vc:claim/…")`), the object holds the claim value hash, and the context field holds the issuer DID hash. The `metadata` 5th vector carries the Ed25519 proof signature anchor (first 8 bytes of the signature, with the full signature stored in an adjacent WAL entry).
- **Verifiable Claims** — individual claims within a VC map directly to subject/predicate/object triples in the Quin graph. A VC is a named graph (context field = issuer DID) containing one or more claim Quins.
- **VC issuance** — `fiduciary_crypto.rs` (§37) `sign_operation(quin, OperationType::VerifiableCredential)` produces the Ed25519 proof. ML-DSA post-quantum issuance is planned (§37).
- **VC presentation + verification** — `verify_operation(proof_index)` checks the stored signature. Presentation proofs (holder binding) are a known gap — planned alongside ML-DSA.
- **QCHK capability profiles** (`profiles.rs`) — binary capability grants that are themselves VC-shaped: a signed assertion from a Principal that a named agent may exercise a named set of capabilities.
- **WebID interoperability** (`webizen_identifiers.rs`) — each `WebizenId` struct carries a `webid_hash: u64` (FNV-1a of the WebID URI string). The `IdentityRegistry` maps `webid_hash → WebizenId`, enabling lookup of a Webizen identitifier from a legacy HTTP/FOAF WebID profile URI. SocialWebNet (§38) supersedes WebID-TLS at the transport layer; WebID profile document discovery remains a complementary application-layer mechanism alongside `did:web`.
- **W3C Solid Protocol interoperability** (`solid_ldp.rs`) — `SolidExporter::export_to_solid_pod(input_q42_path, output_dir_path)` translates a `.q42` vault into a W3C Solid LDP Basic Container. Output: `data.ttl` (Turtle RDF serialisation of Quins via `rio_turtle`) and `data.ttl.acl` (Web Access Control rules). NQuin routing lane `EnforcePermissiveCommons` maps to a `acl:mode acl:Read` public-read ACL. Invoked via the `export-solid` CLI command. Inbound Solid Pod import (LDP container → `.q42` ingest) is planned. `SolidLdpFacade::serialize_to_rdf_star(quin)` is a backward-compatibility stub for RDF-Star serialisation.

---

## 13. CLI (`crates/qualia-cli/`)

### Command Reference

| Command | Description |
|---------|-------------|
| `bench` / `benchmark` | Full benchmark suite |
| `inspect` | Decode and display Quin fields from a `.q42` file |
| `dump` | Stream-dump raw Quins |
| `daemon` | Start/stop the Warp HTTP daemon (port 4242) |
| `ingest [--profile <file>.qchk]` | Ingest RDF/N3/CBOR-LD into a `.q42` vault; profile-bound if `--profile` given |
| `import` | Import from external sources |
| `query` | Execute SPARQL-like queries |
| `compress` | Compress existing `.q42` file |
| `export-solid` | Export graph to W3C Solid Pod |
| `resources list llms` | List available LLM resources |
| `resources list ontologies` | List available ontology resources |
| `resources list sparql` | List available SPARQL endpoints |
| `resources show <id>` | Show resource details |
| `resources download <id>` | Download a resource (streams → GGufSharder → WAL) |
| `resources import-ontology <id>` | Download + SHACL-validate + ingest ontology |
| `profile compile <input.jsonld> <output.qchk>` | Compile JSON-LD profile to QCHK binary |
| `profile list` | List known profiles with q_hash IDs |
| `profile inspect <file.qchk>` | Decode and display a QCHK profile |
| `webizen init` | Initialize Webizen identifier material |
| `webizen ingest` | Ingest via Webizen pipeline |
| `webizen validate-gitmark` | Validate git commit mark |
| `webizen publish-ipfs` | Publish to IPFS |
| `webizen seed-webtorrent` | Seed via WebTorrent |
| `webizen dns-frontdoor` | Generate `did:web` + DNS TXT records |
| `evaluate <modality>` | Run logic/reasoning evaluation against a `.q42` vault (16 modalities) |
| `solve <group>` | Mathematical solver dispatch (5 groups: `linalg`, `optimize`, `ode`, `quantum`, `symbolic`) |
| `science <domain>` | Scientific computation runners (7 domains, 23 runners) |
| `qpu` *(requires `--enable-qpu`)* | QPU provider config and job submission (8 providers) |

**`evaluate` modalities** (pass as positional arg): `propositional`, `predicate`, `modal`, `temporal`, `deontic`, `fuzzy`, `paraconsistent`, `relevance`, `intuitionistic`, `linear`, `abductive`, `causal`, `probabilistic`, `defeasible`, `epistemic`, `neuro_symbolic`

**`solve` groups:**
- `linalg` — matrix ops, LU decomposition, eigensolvers
- `optimize` — Nelder-Mead, Newton-Raphson, Levenberg-Marquardt
- `ode` — Runge-Kutta 4, shooting-method BVP, Simpson integrator
- `quantum` — QAOA angle optimiser, SPSA gradient estimator
- `symbolic` — forward-chaining defeasible reasoner, bounded SAT solver

**`science` domains:** `chem`, `bio`, `geo`, `thermo`, `geometric`, `clinical`, `economics` (23 runners total)

**`qpu` subcommands** (all require `--enable-qpu` global flag):

| Subcommand | Description |
|---|---|
| `qpu list-providers` | List all 8 supported QPU providers with required credential fields |
| `qpu configure <provider>` | Write/update provider API credentials in `$QUALIA_DATA_DIR/qpu_config.json` |
| `qpu show [<provider>]` | Show stored credentials (keys are masked) |
| `qpu clear <provider>` | Remove stored credentials for a provider |
| `qpu test-connection <provider>` | Validate connectivity and credentials |
| `qpu submit <provider>` | Submit a QPU job (local `FallbackHandler` simulation if daemon unavailable) |

**Supported QPU providers:** IBM Quantum, D-Wave Leap, IonQ, Rigetti QCS, Azure Quantum, AWS Braket, Google Quantum AI, Quantinuum. Credentials stored as JSON in `$QUALIA_DATA_DIR/qpu_config.json` (defaults to `./qpu_config.json`).

### Profile CLI Workflow

```bash
# Compile a JSON-LD capability profile to QCHK binary
qualia profile compile profile-health.jsonld health.qchk

# List known profiles and their q_hash IDs
qualia profile list

# Inspect a compiled profile
qualia profile inspect health.qchk

# Bind profile during ingest
qualia ingest --profile health.qchk data.ttl output.q42
```

---

## 14. Deployment Targets

| Target | Files | Notes |
|--------|-------|-------|
| **Native CLI** | `crates/qualia-cli/` | Full feature set, 512 MB budget |
| **Desktop — Tauri/React** | `crates/qualia-desktop/` + `crates/qualia-client/` | Legacy desktop prototype retained in-tree for reference. Not the primary shipped desktop target. |
| **Desktop — Flutter** | `crates/qualia-flutter/` | Primary shipped desktop target; LLM Hub, Ontology Hub, Qapp Vault, and FRB bridge to CLI subprocess |
| **WASM (Browser)** | `wasm_bridge.rs`, `wasm_edge.rs` | SIMD variant, OPFS auto-cache, `#[wasm_bindgen]` |
| **Edge Native** | `npu_ffi.rs`, `tee_ffi.rs` | NPU sieve dispatch, TEE C-ABI declarations |
| **P2P / Federated** | `wasm_edge.rs`, `nym_adapter.rs`, `p2p/*` | Implemented daemon sync currently uses libp2p request-response over TCP + Noise + Yamux; broader WebRTC and Nym profiles remain adjacent or evolving layers |

### Flutter Desktop (`crates/qualia-flutter/`) — **shipped desktop target**

Built and released via `.github/workflows/release.yml` (Windows, macOS, Linux). Backed by `qualia_flutter_rust` (flutter_rust_bridge) calling `qualia-client-core`.

Screens: Dashboard, Chat, Wallet, Address Book, Ontology Hub, Asset Library, **Qapp Vault**, Credential Manager, LLM Hub, Spatial Physics, Settings.

**Qapp Vault** (`QappVaultScreen`, nav index 6): Lists qapps in `{data_dir}/Qapps/`, installs from a directory picker, launches via loopback HTTP in `QualiaQappWebView`, and supports chat → qapp handoff (`launchInstalledQappWithContext`). Qapps declare `required_shapes` in `qapp.json`.

- **LLM Hub** — grid/list view, bulk actions, download state persists across navigation, detail panel
- **Ontology Hub** — browse, import, namespace view
- **FRB bridge** (`rust/src/api/qualia_api.rs`) — inference, qapp vault, resource catalog, daemon control

### Tauri/React (`crates/qualia-desktop/` + `crates/qualia-client/`) — **legacy**

Early desktop prototype. **Not built or released by CI** (Tauri removed from `release.yml` in v0.0.6). Retained in-tree for reference; do not treat as the active desktop shell. New UI work belongs in Flutter.

### CLI (`crates/qualia-cli/`) and WASM (`qualia-core-wasm`)

- **CLI** — engine operations, benchmarks, profile compile, resource ingest, daemon control
- **WASM** — browser playground and edge-native engine builds (`qualia-core-wasm.tar.gz` in Releases)

---

## 15. Known Bugs & Correctness Issues

### 15-A Object Field Type-Tag Conflict

`resolver.rs` (authoritative) defines `0b001 << 60` as `xsd:integer` with integer value in bits 0-59.
`logic.rs::extract_float` uses the same bit pattern as an f32 tag with f32 bits in bits 0-31.

**Do not fix unilaterally** — requires alignment across both systems and the ingest layer. New modules must use the `resolver.rs` convention.

### 15-B LTL Opcodes in `logic.rs`

`WebizenOpcode::Always/Eventually/Next` compare a float threshold on a single Quin's object field — they are **not** LTL temporal operators. Existing tests depend on this behavior. Use `evaluate_ltl_trace` from `temporal_ltl.rs` for real temporal reasoning.

### 15-C `prune_defeasible_claims` Uses Heap

`WebizenVM::prune_defeasible_claims` takes `&mut Vec<NQuin>` and uses `HashSet`. This violates the zero-heap mandate. The zero-alloc replacement signature is:

```rust
pub fn partition_defeasible(
    quins: &[NQuin],
    out_hard: &mut [NQuin],
    out_defeasible: &mut [NQuin],
) -> (usize, usize)
```

### 15-D `derive_lane_key` Uses SHA256 (not PBKDF2)

`agency.rs::derive_lane_key` currently uses SHA256. Production Sanctuary Mode requires PBKDF2 with ≥ 310,000 iterations.

### 15-E Legacy `.q42` Write Formats

- `storage.rs::SuperBlockWriter` — legacy raw 40,960-byte `QualiaSuperBlock` structs
- `ingest.rs::streaming_import_rdf` — (Upgraded to v3 `Q42Volume` format with DagStore and block directory)
- `archive.rs::Q42Archive` — legacy reader expecting Zstd + 64-byte preamble + jump tables

The ingest pipeline (`streaming_import_rdf`) has been successfully migrated to the unified v3 `Q42Volume` format. `SuperBlockWriter` and `Q42Archive` will be deprecated or updated to align with v3 in the future.

### 15-F Legacy Tauri Desktop (not shipped)

`qualia-desktop` / `qualia-client` remain in-tree but are not release targets. Qapp Vault for end users is **Flutter only** (`QappVaultScreen` + `QualiaQappWebView` + FRB → `qualia-client-core`). Loopback qapp assets are served by `qapps_protocol.rs`, started from Flutter at init.

---

## 16. Test Status

**640+ tests** as of `0.0.11-dev` (138 SPARQL, 149 SHACL extensions, 8 git_bridge, remainder across core, domains, CLI).

---

## 17. Domain Crates (`domains/`)

All domain crates live under `crates/qualia-core-db/src/domains/` and are wired into the SlgArena via `SlgOpcode::Native*` variants.

| Sub-path | File | Capabilities |
|---|---|---|
| `biological/` | `bioinformatics.rs` | Smith-Waterman alignment (AVX2/NEON/scalar SIMD), FASTA streaming, DNA→protein translation, isoelectric point, peptide cleavage, k-mer hashing, Tanimoto similarity |
| `chemical/` | `organic_chemistry.rs` | SMILES/InChI parsing, MW/LogP/TPSA, Lipinski/Veber/Ghose/Egan/pKa filters, Morgan fingerprint, Arrhenius/Gibbs/Henderson-Hasselbalch, isotope distribution, E-factor, atom economy |
| `physical/` | `thermodynamics.rs` | MCMC Metropolis–Hastings, Boltzmann acceptance, Gibbs free energy |
| `mathematical/` | `geometric.rs` | Lorentz vector mapping, tropical (Min-Plus) distance, homological sieve |
| `financial/` | `economics.rs` | Time-value of money, portfolio risk metrics, ILP micropayment primitives |
| `financial/` | `tax_schema.rs` | Tax dispatch plan structures, FHIR/LOINC finance schema shapes |
| `geospatial/` | `spatial.rs` | GeoSPARQL extension functions, WKT geometry, spatial predicate resolution |

> **Note**: `ARCHITECTURE.md §6` previously listed these files at the source root. They now live in the `domains/` sub-tree; all SlgOpcode wiring is unchanged.

---

## 18. Extended Modality Modules (`modalities/`)

Beyond the nine core modalities documented in §5, the following sub-modules are also compiled and wired:

### Argumentation (`modalities/argumentation.rs`)

Dung-style Abstract Argumentation Framework. `ARGUMENT_BIT` (bit 55), `ATTACK_BIT` (bit 54), `DEFENSE_BIT` (bit 53) in predicate field. `Argument` structs carry `premise_quins` + `conclusion_quin`. Used by the dialectical synthesis pipeline for formal debate resolution.

### Calculus (`modalities/calculus/`)

Zero-heap numerical computation split across a host/GPU/CUDA triad:

| File | Contents |
|---|---|
| `host.rs` | `MmapGridManager` — memory-mapped grid data; feeds chunked slices to GPU kernel |
| `gpu.rs` | WGSL compute dispatch for numerical integration |
| `cuda_bridge.rs` | CUDA GPUDirect Storage bridge (Linux + NVIDIA only; feature `cuda_gds`) |
| `ode_solver.rs` | RK4 step function operating on mmap'd grid slices |
| `tensor_provenance.rs` | Provenance NQuin annotation for tensor computation results |

### Control Feedback (`modalities/control_feedback.rs`)

PID control theory for self-stabilising agents. `ControlState` tracks setpoint, process variable, error integral and derivative. `CONTROL_BIT` (bit 52), `FEEDBACK_BIT` (bit 51), `STABILIZATION_BIT` (bit 50). Used for power system management and Sanctuary Mode stability guarantees.

### Diffusion (`modalities/diffusion.rs`)

Discrete diffusion logic — `trigger_diffusion(graph_id)` schedules a GPU cellular-automaton pass over the Vulkan sieve to iteratively resolve missing edges. Currently a dispatch stub targeting the `fused_tensor_contraction.wgsl` pipeline.

### Graph Theory (`modalities/graph_theory.rs`)

`QualiaGraph` — built from NQuin relation slices. Algorithms: degree centrality, betweenness centrality, Louvain community detection, motif frequency counting, shortest-path BFS. Zero-alloc output via caller-supplied buffers.

### Interval Reasoning (`modalities/interval_reasoning.rs`)

Allen Interval Algebra (13 relations) extended with constraint propagation and temporal planning. `TemporalInterval` structs; `propagate_interval_constraints` eliminates infeasible assignments. Extends `spatio_temporal.rs` (§5) for multi-interval networks.

### Probabilistic (`modalities/probabilistic.rs`)

O(1) weight-threshold evaluation: `evaluate_threshold(weight: f32, threshold: f32) -> bool`. Reads the probabilistic weight stored in the `metadata` 5th vector of the NQuin. Minimal implementation — full Bayesian network integration is planned.

### Logic Sub-modules (`modalities/logic/`)

| File | Contents |
|---|---|
| `n3_parser.rs` | N3/Turtle rule tokeniser → `Rule` structs (Strict / Defeasible / Defeater / Linear); weighted rules (`(0.8) { … } ~> { … }`) |
| `n3_compiler.rs` | Lowers N3 `Rule` structs to WebizenOpcode bytecode; bridges into `deontic_logic.rs` |
| `shacl.rs` | Core SHACL node/property shape compiler → `WebizenOpcode` arrays |
| `shacl_extensions.rs` | Domain-specific SHACL extension predicates (`qualia:*`) |
| `core_modalities_shacl.rs` | SHACL shapes for all nine core modalities |
| `infrastructure_shacl.rs` | SHACL shapes for storage, WAL, and MCP infrastructure invariants |
| `specialized_libs_shacl.rs` | SHACL shapes for specialized library domains |
| `deontic.rs` | Deontic norm evaluation helpers (used by `deontic_logic.rs`) |
| `owl.rs` | OWL → SHACL converter for RadLex and DICOM healthcare ontologies; lowers `owl:Class`, `owl:equivalentClass`, `owl:restriction` axioms into `sh:NodeShape` graphs. Preserves the Principal invariant: `q42:Principal` may have possessions but is not itself a `q42:Thing`. |
| `qubo.rs` | Semantic-to-QUBO compiler — strips DIDs/URIs into ephemeral local integer indices, emits linear biases and quadratic coupler weights (`MAX_QUBO_VARS = 64`, `MAX_COUPLERS = 512`), re-hydrates binary QPU solutions back to Quins. Opcodes: `OP_EMIT_WEIGHT = 0x50`. |
| `rules.rs` | Rule priority and conflict resolution helpers |
| `core.rs` | Shared logic evaluation primitives |

---

## 19. Geometric Algebra (`geometric_algebra/`)

`geometric_algebra/simd_kernel.rs` — high-performance geometric algebra with runtime SIMD detection:

| Type | Description |
|---|---|
| `GeometricAlgebraSIMD` | Dispatcher: selects AVX2 / NEON / scalar path at runtime |
| `Multivector` | Full multivector in G(3,0,1) conformal model |
| `Rotor` | Even sub-algebra element for rotations |
| `Translator` | Null-plane translator for rigid displacements |
| `Grade` | Grade-0 through Grade-4 enumeration |

Key functions: `geometric_product`, `outer_product`, `rotor_from_angle_axis`, `apply_rotor`, `translator_from_displacement`, `apply_translator`. Used by the `spatial_sieve.rs` NETS pipeline for non-Euclidean geometry operations.

---

## 20. Zero-Copy LoRA Multiplexing (`lora/`)

Context-driven neural adapter selection layered on top of the base GGUF model. Extra footprint ≤ 15 MB per cached adapter; context switching < 10 ms.

| File | Contents |
|---|---|
| `adapter_manager.rs` | `LoraAdapter` — rank-r weight delta pairs (A, B) with alpha scaling; `LoraGuard` RAII for clean unload |
| `context_detector.rs` | `ContextDetector` — reads `NQuin` metadata bits 63–48 to select adapter ID; maps context to `ContextType` (Medical / Legal / Chemical / General / …) |
| `webgpu_lora.rs` | `lora_projection.wgsl` dispatch — fused A×B + base weight, 64 threads/workgroup; binds adapter weight buffers to the GPU pipeline |
| `mod.rs` | `LoraMux` — multiplexer table, up to 16 concurrent adapters; integrates with `LocalLlmAgent::infer()` |

NQuin routing: adapter ID is encoded in bits 63–48 of the `metadata` field. `LocalLlmAgent` reads these bits before each forward pass and selects the adapter via `LoraMux::select`.

---

## 21. Solver Library (`solvers/`)

Zero-allocation mathematical solvers operating on fixed-size stack arrays. All sub-modules use `crate::solvers::SolversError` as the unified error type.

### QPU Dispatch (`solvers/qpu/`) — ✅ Active

See §8 (LLM Agent Layer) for the full QPU provider table. The QPU sub-module here handles:

- `dispatcher.rs` — `QpuDispatcher` trait + 8-provider concrete implementations + `FallbackHandler` (classical simulation)
- `pre_solver.rs` — classical pre-processing (graph reduction, variable elimination) before QPU submission
- `mod.rs` — `QpuJob` queue, `ProblemType` (`Annealing | GateModel | Vqe | Qaoa`), provider selection from `QpuConfig` or Principal VC

**CLI integration** — Provider credentials are managed at runtime via the `qualia-cli qpu` subcommand group (requires `--enable-qpu` global flag). Credentials are stored in `$QUALIA_DATA_DIR/qpu_config.json`. The compile-time `qpu_internal` feature flag previously used to gate this surface has been replaced by the runtime flag. See §13 for the full `qpu` subcommand reference.

### Calculus Solvers (`solvers/calculus/`) — ✅ Active

| Type | Description |
|---|---|
| `RungeKutta4Static` | 4th-order RK integrator, fixed 4-element state vector |
| `ShootingMethodBVP` | Shooting-method boundary value problem solver; 100-step trajectory buffer |
| `SimpsonsIntegratorChunked` | 12-point chunked Simpson's rule integrator |
| `ODEState` / `BVPState` / `IntegralChunk` | State and chunk types |

### Linear Algebra (`solvers/linear_algebra/`) — ✅ Active

| Type | Description |
|---|---|
| `Matrix4x4` | Row-major `[[f64;4];4]` with LU decomposition, determinant, inverse |
| `Vector4` | 4-element vector with dot product, norm, normalise |
| `Tensor3x3x3` | 27-element rank-3 tensor contraction |
| `FixedLanczosEigensolver` | Lanczos iteration for dominant eigenvalues, 4×4 systems |
| `StaticLuDecomposition` | In-place LU factorisation with partial pivoting |
| `ConstTensorContractor` | Compile-time–dimensioned tensor contraction |

### Optimization & Root Finding (`solvers/optimization/`) — ✅ Active

| Type | Description |
|---|---|
| `NelderMeadSimplex` | Nelder-Mead downhill simplex, 4D, 5-vertex |
| `BoundedNewtonRaphson` | Newton-Raphson root finding with bracket enforcement |
| `LevenbergMarquardtStack` | Levenberg-Marquardt curve fitting, stack-only Jacobian |

### Hybrid Quantum Optimizers (`solvers/quantum_optimizers/`) — ✅ Active

| Type | Description |
|---|---|
| `QAOAAngleOptimizer` | QAOA β/γ angle optimiser for quantum approximate optimisation |
| `SpsaOptimizer` | Simultaneous Perturbation Stochastic Approximation gradient estimator |

### Symbolic Logic (`solvers/symbolic_logic/`) — ✅ Active

| Type | Description |
|---|---|
| `ForwardChainingDefeasible` | Forward-chaining defeasible reasoner; 100-rule base, 50-fact store |
| `BoundedSatSolver` | DPLL-style SAT solver; bounded clause and variable arrays |

### Unified Error Type

```rust
pub enum SolversError {
    CapacityExceeded, SingularMatrix, InvalidParameters,
    ConvergenceFailed, InvalidDimension, ComputationError,
    QuantumError(u32), OutOfMemory, Unsatisfiable,
}
```

---

## 22. Obfuscation Module (`obfuscation/`)

Semantic-stripping pipeline for safe QPU offloading. All operations maintain zero-allocation invariants and map directly onto the 48-byte NQuin payload.

| File | Contents |
|---|---|
| `semantic_stripper.rs` | Strips human-readable DID/URI context from Quins, replacing with ephemeral local integer indices for anonymous QPU jobs |
| `polynomial_obfuscator.rs` | Encodes stripped indices as polynomial coefficients before transmission |
| `domain_transformer.rs` | Maps domain-specific predicates to anonymous coupling weights |
| `hybrid_state_manager.rs` | Tracks the mapping between obfuscated QPU problem and original semantic graph; re-hydrates results after QPU returns |

---

## 23. DICOM Integration (`dicom.rs`, `dicom_ingest.rs`)

Medical imaging pipeline — heap allocation is permitted here (ingest path, not evaluator hot path).

- **`dicom.rs`** — DICOM Part 10 metadata parser: transfer syntax detection (`IMPLICIT_VR_LITTLE_ENDIAN`, `EXPLICIT_VR_LITTLE_ENDIAN`, `JPEG2000_LOSSLESS`), tag extraction into `BTreeMap<u32, DicomTag>`, anatomy overlay spec generation.
- **`dicom_ingest.rs`** — Ingest pipeline: DICOM file → metadata Quins via `q_hash` predicate mapping → WAL append. Wired to the OWL→SHACL pipeline in `modalities/logic/owl.rs` for RadLex/DICOM shape validation.

---

## 24. ILP Micropayment Dispatcher (`ilp_dispatcher.rs`)

Executes a `TaxDispatchPlan` as a sequence of ILP STREAM micropayments.

**Transport stack (preference order):**
1. **SPSP / ILP-over-HTTP** — resolves `$pointer` → HTTPS endpoint, opens a STREAM connection, sends the exact µ-cent amount, collects a receipt.
2. **Nym mixnet proxy** — when `instruction.use_nym == true`, wraps the Sphinx packet through the Nym gateway before hitting the ILP endpoint.

Each instruction targets a designated ILP Payment Pointer. Receipts are written as provenance Quins to the WAL.

---

## 25. Neuro-Symbolic Sieve (`neuro_symbolic_sieve.rs`)

Grammar-constrained FSM sieve applied to LLM token output — zero-heap hot path.

```rust
pub const MAX_SIEVE_ALLOW: usize = 16;  // max token IDs per FSM state

pub struct SieveSlot { pub token_id: u32 }
```

The sieve enforces a lexicon-bound finite state machine over the token stream during generation. FSM states carry a `[SieveSlot; MAX_SIEVE_ALLOW]` allow-mask. Tokens not in the mask for the current state are rejected before they enter the Webizen Sentinel ring buffer. This is applied before (not instead of) the Phase 8 Sentinel rollback.

---

## 26. Acoustic & BLE Mesh (`acoustic_ble_mesh.rs`)

Zero-infrastructure delay-tolerant networking for crisis / emergency scenarios. Platform: non-WASM native only (`#[cfg(not(target_arch = "wasm32"))]`).

**Acoustic layer**: `AcousticNetwork` with channel manager, modem controller, protocol stack (Physical → Data Link → Network → Transport), error correction, `AcousticChannelManager` with dynamic allocation strategies.

**BLE Mesh layer**: `BleNetwork` / `BleMeshManager` — Bluetooth Mesh profiles, `CompositionData`, `MeshModel` publication/subscription, node feature negotiation.

**Routing**: `MeshRouter` with DTN (Delay-Tolerant Networking) store-and-forward; `MessagePriority` queue (`Critical` / `High` / `Normal` / `Low` / `Background`).

Key entry point: `MeshNetworkManager::new()` → `initialize()` → `discover_nodes()` → `send_message(dest, payload, priority)`.

---

## 27. Ambient Sub-Threshold Orchestration (`ambient_orchestration.rs`)

Power-efficient edge orchestration via NNAPI (Android) and CoreML (iOS/macOS). Manages mobile scientific compute jobs below the user-perceptible threshold. `AmbientOrchestrationManager` schedules solver sub-tasks to NNAPI/CoreML delegates when device is idle, charging, or on Wi-Fi. Integrates with `ThermalGovernor` to pause when in `Critical` state.

---

## 28. Webizen Identifiers (`webizen_identifiers.rs`)

Sovereign actor identifier layer for SPARQL-Star provenance.

```rust
pub type WebizenId = u64;
pub const TAG_WEBIZEN: u64 = 0x8000_0000_0000_0000;  // high bit set
```

Webizen IDs occupy reserved high-prefix lexicon slots. `verify_webizen_signature(id, sig, msg)` — ed25519 signature check against the public key encoded in the Webizen ID. Used to authenticate provenance Quins in SPARQL-Star embedded triples.

---

## 29. Web Civics / Cryptokey Routing (`web_civics.rs`)

Derives a Unique Local IPv6 address (ULA, `fd00::/8`) from an Ed25519 Webizen public key:

```rust
pub fn derive_webizen_ipv6(public_key: &VerifyingKey) -> Ipv6Addr
```

SHA-256 of the public key bytes → first 15 bytes mapped into the `fd__:____:____:____:____/48` ULA block. This enforces **Cryptokey Routing**: the DID is mathematically synonymous with the IPv6 address, making network identity and semantic identity the same object.

---

## 30. Specialized Libraries (`specialized_libs/`)

High-performance domain libraries targeting Phase 2 architectural enhancements. Most sub-modules are currently disabled pending build-error resolution; only `qpu_bridge` is active.

| File | Status | Contents |
|---|---|---|
| `qpu_bridge.rs` | ✅ Active | Bridge between `solvers/qpu/` and external QPU providers; classical problem formulation → QPU submission |
| `linear_algebra.rs` | ⚠️ Disabled | Dense/sparse matrix ops, BLAS-level routines |
| `statistical_computing.rs` | ⚠️ Disabled | Bayesian inference, Monte Carlo sampling |
| `cryptographic_library.rs` | ⚠️ Disabled | Advanced cryptographic primitives beyond `fiduciary_crypto.rs` |
| `engineering_analysis.rs` | ⚠️ Disabled | FEA / FEM structural analysis |
| `financial_modeling.rs` | ⚠️ Disabled | Derivatives pricing, risk models (complements `domains/financial/`) |
| `machine_learning.rs` | ⚠️ Disabled | Classical ML (SVM, decision trees) as complement to LLM inference |
| `medical_computing.rs` | ⚠️ Disabled | Clinical decision support beyond `clinical_engine.rs` |
| `physics_simulation.rs` | ⚠️ Disabled | N-body, rigid body, fluid simulation |
| `quantum_biology.rs` | ⚠️ Disabled | Photosynthesis coherence models, enzyme tunneling |

---

## 31. P2P / Federated Sync (`p2p/`)

libp2p-based peer networking stack for graph replication:

| File | Contents |
|---|---|
| `protocol.rs` | Custom request-response protocol codec over Yamux streams; message framing for Quin sync payloads |
| `routing.rs` | Kademlia DHT routing table management; peer discovery and record lookup |
| `swarm.rs` | `QualiaSwarm` — assembles Transport (TCP + Noise + Yamux) + Kademlia + RequestResponse behaviours; drives the libp2p event loop |
| `mod.rs` | `P2pConfig`, `SyncRequest` / `SyncResponse` types |

Transport: TCP with Noise protocol encryption and Yamux multiplexing. WebRTC and Nym profiles are adjacent/evolving layers.

---

## 32. Hardware-Sympathetic Storage (`zns_storage.rs`, `csd_storage.rs`)

Platform: non-WASM native only.

**`zns_storage.rs` — NVMe Zoned Namespace (ZNS)**: `ZnsZoneManager` manages sequential-write and random-write zones. `ZnsIoScheduler` batches writes to respect zone write-pointer alignment. `ZnsDeviceInfo` exposes zone capacity, LBA range, and zone state (`Empty / ImplicitOpen / ExplicitOpen / Full / Closed / Offline`). Designed for zero-amplification WAL compaction on ZNS-capable NVMe drives.

**`csd_storage.rs` — Computational Storage Device (CSD)**: `CsdStorageManager` offloads filter/aggregate operations to CSD firmware. Pushes SPARQL filter predicates as CSD programs; results arrive as pre-filtered Quin streams, bypassing the host CPU for large-volume scans.

---

## 33. eBPF Allocation Firewall (`ebpf_firewall.rs`)

Platform: non-WASM native only. Linux eBPF support at runtime (compiled on all platforms; eBPF syscalls fail gracefully on non-Linux).

`EbpfFirewall` manages loaded eBPF programs, attached sockets, and firewall rules. Program types: `SocketFilter`, `Xdp` (high-performance packet processing), `TrafficControl`. `FirewallRule` structs specify allowed/denied source IPs, port ranges, and protocol types. Used to enforce the allocation firewall boundary in §7 (MCP Intent Frame Mediation) at the kernel level.

---

## 34. Comorbidity Evaluation (`comorbidity_eval.rs`)

Clinical comorbidity scoring complement to `clinical_engine.rs` (§6). Evaluates ICD-10 comorbidity indices (Charlson, Elixhauser) over a patient's condition Quins. Zero-alloc over caller-supplied condition slices; returns a packed `u32` score for WAL logging.

---

## 35. Webizen Bytecode (`webizen_bytecode.rs`)

Low-level bytecode definitions for the Webizen VM. Defines the `WebizenInstruction` binary encoding that `shacl_compiler.rs` (§5) emits and `webizen.rs::execute_vm_frame` interprets. Each instruction is a `u64`-aligned word with a 4-bit opcode prefix and 60-bit payload. This file is the authoritative encoding reference — `webizen.rs` and `webizen_bytecode.rs` must stay in sync.

Coverage includes: all modality evaluators, MCP fiduciary gate, SHACL constraint compilation, organic chemistry (including isotope distribution), VM safety, SIMD integration, CRDT consensus, and agency signing.

---

## 36. Zero-Knowledge Semantic Proofs (`zk_proofs.rs`)

Privacy-preserving proof layer on top of the Quin graph. Current state: **structural validation only** (Pedersen commitment check). Full zk-SNARK backend (Halo2) is planned.

### Implemented

- **`ZkProofEngine`** — wraps a `ZkConfig` (curve parameters, generator point `G`, blinding factor `H`) and a `Vec<ZkProof>` store (capped at 256 proofs).
- **`ZkProof` struct** — commitment `C` (32 bytes), value commitment `v_c` (32 bytes), blinding factor `r` (32 bytes), `is_valid: bool`.
- **`generate_semantic_proof(quin)`** — derives a synthetic value from `quin.object ^ quin.predicate`, computes the Pedersen commitment `C = v·G + r·H`, validates `C != [0; 32]`, and records the proof in the store. Returns `ZkError::InvalidQuin` if the commitment is degenerate.
- **`verify_semantic_proof(proof_index)`** — re-validates commitment integrity from the stored `ZkProof`. Used by `sparql_filter.rs` for PROV-O privacy-preserving constraint checks.

### Planned (Halo2 / zk-SNARKs)

Full zero-knowledge backend requires arithmetic circuit compilation. Planned components (per `local/architectural-enhancements/Zero_Knowledge_Semantic_Proofs_Implementation_Spec.md`):

- **Semantic circuit** — Quin predicate relationships encoded as R1CS / PLONK constraints
- **Proof generation** — succinct proofs of semantic inference correctness without data disclosure
- **Verification engine** — sub-millisecond proof verification without revealing the underlying Quin values
- **Privacy-preserving inference proofs** — prove LLM output fell within a valid range without disclosing the output

### Integration points

- `sparql_filter.rs` — calls `verify_semantic_proof()` for PROV-O privacy constraints
- `fiduciary_crypto.rs` (§37) — shares the `ZkConfig` curve parameters for combined ML-DSA + ZK proofs
- WAL — `ZkProof` structs serialised into the WAL alongside the Quins they attest

---

## 37. Fiduciary Cryptography (`fiduciary_crypto.rs`)

Cryptographic signing layer that enforces non-repudiation for fiduciary operations. Current state: **Ed25519 implemented; ML-DSA (FIPS 204) partial**.

### Implemented

- **`FiduciaryCrypto`** — holds an `ed25519_dalek::SigningKey` and a proof store (`Vec<FiduciaryProof>`, capped at 256).
- **`sign_operation(quin, operation_type)`** — produces an Ed25519 signature over `quin.subject ‖ quin.predicate ‖ quin.object` via SHA-256. Records the proof with `operation_type` (e.g. `MedicalDirective`, `LegalContract`, `GuardianshipConsent`, `DataTransfer`) and a Unix-timestamp.
- **`verify_operation(proof_index)`** — verifies the stored Ed25519 signature. Returns `FiduciaryError::InvalidSignature` if tampered.
- **`get_audit_trail()`** — returns a slice of all stored `FiduciaryProof` entries for WAL checkpointing.
- ECC parity (real P-256 scalar validation via `p256` crate) wired to the Sentinel pre-flight gate.

### Planned (ML-DSA / FIPS 204)

Full post-quantum hardening requires the Module Lattice-Based Digital Signature Algorithm (per `local/architectural-enhancements/Fiduciary_Cryptography_Implementation_Spec.md`):

- **ML-DSA-87** — 2560-byte public key, 4627-byte signatures; quantum-resistant against Shor's algorithm
- Key sizes do not fit in a single NQuin field — storage strategy: Merkle-tree of signature fragments encoded across multiple linked Quins
- Drop-in replacement for all `sign_operation` / `verify_operation` call sites; Ed25519 remains the intermediate until FIPS 204 is stable in `pqcrypto` crate ecosystem

### Integration points

- `orchestrator.rs` — `validate_intent` pre-flight calls `sign_operation` for conduct-violation Quins
- `wal.rs` — all WAL entries include a fiduciary signature header
- `zk_proofs.rs` (§36) — shares curve parameters for combined proof artefacts

---

## 38. DNSSEC → SocialWebNet Bootstrapping (`daemon_swarm.rs`)

Zero-allocation decentralized networking that moves semantic identity from the Application Layer into the Transport / Network Routing Layers. Replaces traditional HTTP/FOAF WebID-TLS.

### Architecture

The pipeline has three stages:

1. **DNSSEC resolution** — `resolve_dnssec_peer(domain)` issues a DNS TXT/CERT query for the target DID domain. The response is a CBOR-LD `DnssecSemanticPayload` (`did`, `wireguard_pubkey [u8; 32]`, `ipv6_address [u8; 16]`, `service_endpoints[]`). Validated by the DNSSEC chain of trust — no CA required.

2. **WireGuard tunnel establishment** — `establish_wireguard_tunnel(peer_payload, endpoint, port)` calls the `wireguard-rs` userspace library using the DNSSEC-resolved pubkey. A `SocialWebNetPeer` is inserted into `SocialWebNetInterface::active_peers` (keyed by DID hash).

3. **Userspace proxy** — `init_wireguard_interface(name, port)` brings up the interface and listens on `127.0.0.1:1080` (SOCKS5). All traffic to social peers flows through this proxy.

### Key types

| Type | Description |
|---|---|
| `SocialWebNetPeer` | `did_hash: u64`, `pubkey: [u8; 32]`, `preshared_key: [u8; 32]`, `endpoint: [u8; 16]` (IPv6), `port: u16` |
| `SocialWebNetInterface` | `interface_name`, `local_port`, `active_peers: HashMap<u64, SocialWebNetPeer>` |
| `DnssecSemanticPayload` | CBOR-LD zero-allocation struct embedding DID + WG pubkey + service endpoints |

### Invariants

- WireGuard pubkeys are derived from the DID keypair — connecting to a peer = knowing their DID
- No out-of-band key exchange required; the social graph defines the ACL
- `active_peers` is bounded at 256 entries to preserve the 42 MB arena ceiling
- All peer insertions write a provenance Quin to the WAL (signed by `FiduciaryCrypto`)

### Integration points

- `daemon.rs` — Webizen Civics Userspace WireGuard Proxy on `127.0.0.1:1080` is started by the daemon at boot
- `web_civics.rs` (§29) — DID → IPv6 address derivation (`derive_webizen_ipv6`) feeds into interface routing table
- `nym_adapter.rs` — traffic that cannot use WireGuard falls back to Nym mixnet

---

## 39. TEE / Unforgeable Agency (`tee_ffi.rs`)

C-ABI FFI declarations for Trusted Execution Environment hardware: Intel SGX, ARM TrustZone, AMD SEV. Currently provides **FFI groundwork only** — enclave-resident key operations and biometric binding are planned Phase 3 work.

### Implemented

`tee_ffi.rs` declares the extern `"C"` symbol table used on platforms that have a TEE:

```rust
extern "C" {
    fn tee_generate_keypair(pubkey_out: *mut u8) -> i32;
    fn tee_sign(data: *const u8, len: usize, sig_out: *mut u8) -> i32;
    fn tee_verify(data: *const u8, len: usize, sig: *const u8) -> i32;
    fn tee_seal(plaintext: *const u8, len: usize, sealed_out: *mut u8) -> i32;
    fn tee_unseal(sealed: *const u8, len: usize, out: *mut u8) -> i32;
}
```

On platforms without a TEE these resolve to no-op stubs that return `-1`.

### Planned (Unforgeable Agency — Phase 3)

Per `local/architectural-enhancements/Unforgeable_Agency_Implementation_Spec.md`:

- **Biometric-cryptographic anchor** — fingerprint / face / voice template hashed inside the enclave; resulting key never leaves the TEE
- **Enclave-resident DID key operations** — `did:q42` signing moves inside SGX/TrustZone so private keys are never accessible from the host OS
- **Non-repudiation proof generation** — signed attestation quote (`tee_seal`) of every fiduciary action, verifiable by third parties without TEE hardware
- **Continuous ambient attestation** — time-bound proofs (re-signed every N seconds) prevent replay attacks on agency claims
- **Integration with `fiduciary_crypto.rs`** (§37) — TEE-generated ML-DSA keys replace the host-side Ed25519 signing path

---

## 40. Planned Enhancements (Spec / Design Phase)

The following enhancements have detailed implementation specifications in `local/architectural-enhancements/` but do not yet have corresponding source files. They are listed here so the architecture document reflects the full design intent.

### 40-A. WASI Component Model (Phase 1 — highest priority)

Capability-based sandboxing for third-party Qapp code via WebAssembly Component Model (WASI Preview 3). NQuins are passed across the WASM boundary as 48-byte `capability-handle` values; the host enforces expiry, usage limits, and Ed25519 signatures on every handle. Enables a Qapp marketplace where untrusted components are mathematically isolated.

Key planned files: `wasi/component_manager.rs`, `wasi/memory_limiter.rs`, `wasi/security_auditor.rs`, `qualia-component.wit`.

### 40-B. O(1) Memory CRDTs — DVV + Epoch-Based Anti-Entropy (Phase 1)

Addresses the tombstone problem in `crdt.rs` (§11): current OR-Set tombstones grow without bound, violating the 512 MB constraint over long-running nodes. Fix: Dotted Version Vectors (DVVs) track causal history without persistent tombstones; Epoch-Based Anti-Entropy (EAE) seals confirmed tombstones into LZ4-compressed, Ed25519-signed epochs stored in `.q42` SuperBlocks and reclaims the RAM.

Memory model: `Active CRDT Memory = Base + Current-Epoch Tombstones` (constant regardless of total operation count).

Key planned files: `crdt/dotted_version_vector.rs`, `crdt/epoch_manager.rs`, `crdt/memory_controller.rs`.

### 40-C. Spatiotemporal Fractal Indexing — Z-Order Morton Codes (Phase 1)

Replaces heap-allocating R-Tree / KD-Tree spatial indices with a zero-allocation sorted array of 64-bit Morton codes. Encodes (latitude: 21 bits, longitude: 21 bits, timestamp: 22 bits) into a single `u64` by bit-interleaving; BMI2 `_pdep_u64` on x86_64 accelerates encoding to O(1). A bounding-box range query becomes a binary search over the sorted array — no pointer chasing, no rebalancing, cache-friendly sequential access.

Key planned files: `spatiotemporal/morton.rs`, `spatiotemporal/index.rs`.

### 40-D. Cryptographic Halo — FHE over WebGPU (Phase 3)

Fully Homomorphic Encryption (BFV/BGV scheme) accelerated by `wgpu` compute shaders, enabling QualiaDB to perform operations on encrypted Quins without decryption. A semantic query compiles to an FHE arithmetic circuit; the WebGPU kernel evaluates it over ciphertexts; the result decrypts only at the authorised endpoint. Enables blind fiduciary compute and secure multi-party graph analytics.

Prerequisite: wgpu pipeline already exists (`fused_tensor_contraction.wgsl`); FHE shader kernels extend it.

### 40-E. Intermittent Computing — Microsecond NVM Snapshots (Phase 4)

Power-loss interrupt handler captures the entire 42 MB `SlgArena` + CPU register file to Non-Volatile Memory (NVM / MRAM) in under 150 µs. On reboot, the snapshot is checksum-verified and execution resumes at the exact interrupted bytecode position in the Sentinel VM. Enables fiduciary data survival during device power loss in crisis scenarios.

Key design constraint: the 42 MB arena ceiling is the reason this is feasible — a snapshot is a bounded, deterministic DMA transfer.

### 40-F. Spatial Web Anchoring — UWB & VPS (Phase 4)

Physical-space cryptography: a 3D point-cloud hash of a room's geometry (captured via camera VPS + UWB ranging) becomes the AES-256-GCM key for a set of Quins. Those Quins can only be decrypted when an authorised user physically occupies that location. Creates GPS-free, offline "digital dead drops" for crisis / safehouse scenarios without any external infrastructure dependency.

### 40-G. Formal Safety Verification — Coq / LEAN (Phase 4)

Machine-checked mathematical proofs of the Sentinel VM's state-transition function. Target theorems: (1) a `SENSITIVITY_CLASSIFIED` Quin is never routed to the Public Commons lane; (2) no hot-path code path allocates heap memory; (3) the 42 MB arena ceiling is never exceeded; (4) all fiduciary deontic rules halt. Output: a formal certification artefact suitable for legal recognition of QualiaDB as an automated fiduciary guardian.

---

## 41. W3C Solid Protocol & WebID Interoperability (`solid_ldp.rs`)

CG specifications: [Solid Protocol v0.11](https://solidproject.org/TR/protocol) · [Web Access Control](https://solidproject.org/TR/wac) · [Solid-OIDC](https://solidproject.org/TR/oidc) · [Solid WebID Profile](https://solid.github.io/webid-profile/) · [Solid Notifications Protocol](https://solidproject.org/TR/notifications-protocol) · [Solid Application Interoperability](https://solidproject.org/TR/sai) · [Solid DID Method](https://solid.github.io/did-method-solid/) · [WebID 1.0](https://www.w3.org/2005/Incubator/webid/spec/identity/)

### Design intent

Solid support is a **backwards-compatibility and data-portability layer**, not a goal in itself. Three use cases drive it:

- **Ecosystem federation** — Webizen users can exchange data with people at institutions (universities, enterprises, public bodies) that deploy Solid. Those Solid users interact with Webizen data as standard Turtle/RDF + WAC — they get the common baseline but not Qualia's full capabilities (SPARQL-Star provenance, governance VM, NQuin semantics, SocialWebNet routing, LLM inference). Conversely, a Webizen user reading a Solid pod receives a flat RDF graph that is ingested as Quins, after which all Qualia query and inference capabilities apply.
- **Institutional reach** — large organisations are more likely to adopt Solid than Webizen. Solid interop means Webizen participants can address them as first-class nodes in the same semantic web without requiring institutional change.
- **User exit rights / data portability** — a user who chooses to stop using Webizen/QualiaDB can export their semantic graph to any W3C Solid pod provider and continue with standard Solid tooling. No proprietary lock-in.

### 41-A. SolidExporter — `.q42` → LDP Basic Container (Implemented)

```rust
pub struct SolidExporter;
impl SolidExporter {
    pub fn export_to_solid_pod(input_q42_path: &str, output_dir_path: &str) -> std::io::Result<()>
}
```

Translates a `.q42` vault into a W3C Solid LDP Basic Container directory:

| Output file | Contents | Spec reference |
|---|---|---|
| `data.ttl` | Turtle RDF serialisation of all Quins (`rio_turtle`) | Solid Protocol §RDF Sources |
| `data.ttl.acl` | WAC authorization rules derived from NQuin routing lanes | WAC §Authorization |

**WAC ACL model (per [WAC spec](https://solidproject.org/TR/wac)):** Each `acl:Authorization` instance requires at least one `acl:accessTo` or `acl:default`, at least one `acl:mode`, and at least one access subject. The Solid Protocol requires ACL resources to be discoverable via `Link: <...>; rel="acl"` HTTP response headers — this is relevant for a future Solid HTTP server mode.

**Lane → ACL mapping:**

| NQuin routing lane | Generated WAC rule | Access subject |
|---|---|---|
| `EnforcePermissiveCommons` | `acl:mode acl:Read` | `acl:agentClass foaf:Agent` (public) |
| All other lanes | `acl:mode acl:Read, acl:Write, acl:Control` | Owner DID URI only |

Note: `foaf:Agent` means any agent including unauthenticated; `acl:AuthenticatedAgent` would restrict to authenticated users only. The public-read mapping uses `foaf:Agent` per the WAC spec's intent for open data.

**CLI:** `qualia export-solid <input.q42> <output-dir>`

### 41-B. SolidLdpFacade (backward-compatibility stub)

```rust
pub struct SolidLdpFacade;
impl SolidLdpFacade {
    pub fn serialize_to_rdf_star(quin: &NQuin) -> String  // stub
}
```

Retained for earlier API consumers; not used by the export pipeline.

### 41-C. WebID Interoperability (see also §12, §28)

[WebID 1.0](https://www.w3.org/2005/Incubator/webid/spec/identity/) defines an HTTP URI (`https://example.org/profile/card#me`) that dereferences to an RDF profile document asserting `rdf:type foaf:Agent`. The [Solid WebID Profile](https://solid.github.io/webid-profile/) spec extends this with required predicates (`pim:preferencesFile`) and optional predicates (`pim:storage`, `solid:oidcIssuer`, `ldp:inbox`, `cert:key`).

Current Qualia implementation:
- `WebizenId.webid_hash: u64` — FNV-1a of the WebID URI string. Stored alongside the Ed25519 public key.
- `IdentityRegistry::get_webizen_by_webid(webid_hash)` — reverse-lookup: legacy WebID URI → sovereign Webizen ID.
- **Transport replacement:** SocialWebNet (§38) supersedes WebID-TLS (the legacy mechanism that bound a TLS client certificate to a WebID). Qualia uses DID challenge-response (Ed25519) at the application layer, not WebID-TLS.

Known gap: Qualia does not yet generate WebID Profile documents (the RDF resource at the WebID URI). A full profile would include `pim:preferencesFile`, `solid:oidcIssuer`, `pim:storage` pointing to the Qualia daemon, and `cert:key` for legacy WebID-TLS compatibility.

### 41-D. Solid DID Method (`did:solid`)

[`did:solid`](https://solid.github.io/did-method-solid/) is a specialisation of `did:web` that uses a Solid server as the verifiable data registry. Format: `did:solid:server.example` → `https://server.example/.well-known/did.json`. CRUD via HTTP PUT/GET/DELETE; DID documents use `application/did+ld+json`.

The `did:web` resolution already implemented in `identifier.rs` (§12) provides the direct foundation for `did:solid` resolution — the mapping algorithm is identical. Planned: explicit `did:solid` resolver variant that verifies the Solid server's storage description resource.

### 41-E. Solid-OIDC Authentication (Planned)

[Solid-OIDC](https://solidproject.org/TR/oidc) extends OpenID Connect for decentralised environments:
- Authorization Code Flow + PKCE (no pre-existing trust between client and OP)
- DPoP-bound ID tokens (`cnf.jkt` claim = JWK Thumbprint of client's ephemeral key)
- `webid` scope: ID token carries `webid` URI claim
- WebID Profile verification: server checks `?webid solid:oidcIssuer ?iss` triple in the profile document

Current Qualia authentication: DID challenge-response (Ed25519 `sign_operation` via `fiduciary_crypto.rs`) and SocialWebNet DNSSEC-verified tunnels. Solid-OIDC is planned for federation with third-party Solid servers and standard-conformant Solid client apps.

### 41-F. Solid Notifications Protocol (Planned)

[Solid Notifications](https://solidproject.org/TR/notifications-protocol) defines a subscription model for real-time resource change events:
- Discovery: resource servers advertise description resources via `Link: <...>; rel="describedby"`
- Subscription: `POST` to subscription service → channel connection details
- Channel types: `WebSocketChannel2023`, `EventSourceChannel2023`, `WebhookChannel2023`
- Messages: Activity Streams 2.0 JSON-LD

Qualia's existing WebSocket at port 4242 (`/chat/publish`, `/chat/pull`) is a custom protocol, not Solid Notifications conformant. Planned: expose graph change events via Solid Notifications `WebSocketChannel2023` subscriptions so standard Solid client apps can receive live updates.

### 41-G. Solid Application Interoperability — SAI (Planned)

[SAI](https://solidproject.org/TR/sai) defines how applications and social agents securely share data across Solid Pods:
- **Data Registries** — organise data by shape tree type in separate `DataRegistration` containers
- **Access Grants** — represent approved access (agent, data registration, access modes: read/create/update/delete)
- **Authorization Agent** — designated application that manages access decisions for a social agent
- **Access Need Groups** — applications express required data types and access modes before being granted access

SAI is planned as the primary mechanism for Qualia-to-Qualia and Qualia-to-Solid-app data sharing. The NQuin ODRL rights vocabulary and QCHK capability profiles are conceptually aligned with SAI Access Grants (both are signed authorization assertions).

### 41-H. Planned: Inbound Solid Pod Import

Reading an LDP container from a Solid server (HTTP `GET` with `text/turtle`, parse via `rio_turtle`, ingest Quins into `.q42`) is the symmetric counterpart to the existing export. The `rio_turtle` parser dependency is already present in `Cargo.toml`. Requires Solid-OIDC or DID-challenge-response authentication to access protected containers.
