# QualiaDB Architecture

_Branch: `0.0.6-dev` | Last updated: 2026-06-06_

QualiaDB is a zero-allocation, mechanically sympathetic semantic database and multi-agent collaboration ecosystem. It bridges the string-heavy reality of the Semantic Web with hardware-aligned execution paths, enforcing strict constraints to ensure bounded memory and deterministic performance.

---

## 1. Core Principles & Constraints

| Principle | Detail |
|-----------|--------|
| **Zero-Heap in Hot Paths** | No `Vec`, `String`, or `Box` inside evaluator loops. Callers supply fixed-size output buffers (`&mut [T]`) or `[T; N]` stack arrays for local state. |
| **48-Byte Super-Quin** | Every semantic datum fits in a `QualiaQuin` (6 × `u64`: subject, predicate, object, context, metadata, parity). Hashes and bit-packing replace string pointers entirely. |
| **42 MB Prolog Sentinel** | Any single execution pass must stay within 42 × 1024 × 1024 bytes. `SlgArena` enforces this structurally (917,504 Quin slots). |
| **512 MB Edge Floor** | Total system — graph engine, Webizen VM, LLM runtime, and all caches — must stay within 512 MB. Hard design target for personal-device deployment. |
| **Deterministic, Non-Recursive** | No unbounded recursion. LTL/ASP evaluators iterate over slices; they never call themselves. |
| **q_hash for all URIs** | All string IRIs are FNV-1a–hashed at compile time via `q_hash()` or `q_turtle!`. No runtime string allocation in the engine core. |
| **Opcode Partitioning** | `mini_parser.rs` owns `0x00–0x04`. All modality opcodes start at `0x10+`. See §4 for the full opcode table. |
| **Mechanical Sympathy** | Data layouts are cache-line aligned. Evaluation paths avoid pointer chasing and random memory access. |

---

## 2. Universal Quin Bit Layout

Every `QualiaQuin` is exactly 48 bytes — six `u64` fields. All semantic meaning is encoded via bit-packing; no pointers, no heap references.

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

parity     [0..63]   XOR fold: subject ^ predicate ^ object ^ context (ECC stub)
```

**Note on `lexicon.rs`:** `generate_60bit_token` masks hashes to 60 bits (`& 0x0FFF_FFFF_FFFF_FFFF`), explicitly reserving bits 60-63 for type tags. All new modality object values must respect this mask.

---

## 3. Storage Engine

### SuperBlocks (`storage.rs`, `wal.rs`)

The fundamental on-disk unit is the **SuperBlock**: exactly 40,960 bytes (10 × 4,096-byte disk sectors), holding 850 `QualiaQuin`s plus a 160-byte header. SuperBlocks are LZ4-compressed for density.

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
| `QualiaQuin` | `lib.rs` | 48-byte semantic datum |
| `QualiaSuperBlock` | `storage.rs` | 850-Quin compressed block |
| `QuinIncrementalScanner` | `storage.rs` | Zero-alloc streaming cursor |
| `BlockOffsetMap` | `indexing.rs` | BIDX-backed demand-paging index |

---

## 4. Ingestion Pipeline (`ingest.rs`, `qualia-cli`)

The CLI is the entry point for sovereign data ingestion into `.q42` vaults.

- **Formats**: CogAI Cognitive AI Chunks (`.chk` text — W3C CG chunks-and-rules format), CBOR-LD (`.cbor` / `.cbor-ld`), N-Triples, Turtle, JSON-LD, RDF/XML via the Rio streaming parser.
- **Profile-bound ingestion**: `qualia ingest --profile <file>.chk` binds a `CapabilityProfile` for the ingest session, restricting available opcodes and ontologies.

> ⚠ **`.chk` extension collision**: Two distinct formats share this extension. The QCHK magic bytes (`0x51 0x43 0x48 0x4B` = "QCHK") at offset 0 identify a binary Capability Profile. A `.chk` file without that magic is a CogAI Cognitive AI Chunks text file (human-readable ACT-R chunks-and-rules). The ingest pipeline reads CogAI text chunks; the profile system reads QCHK binaries. Never conflate the two.
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
    quins: &[QualiaQuin],
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
    base: &QualiaQuin,
    rules: &[QualiaQuin],
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
    tbox: &[QualiaQuin],   // Quins with predicate = q_hash("rdfs:subClassOf")
) -> bool
```

#### Linear Logic (`modalities/linear.rs`) — ✅ Complete
Resource consumption via tombstone mechanism (no heap allocation).

```rust
pub const CONSUMED_BIT: u64 = 1u64 << 59;
pub fn consume_quin(q: &mut QualiaQuin)
pub fn is_consumed(q: &QualiaQuin) -> bool
```

#### Dialectical Logic (`modalities/dialectical.rs`) — ✅ Complete
Thesis/antithesis/synthesis over ASP stable-model pairs.

```rust
pub const SYNTHESIZED_BIT: u64 = 1u64 << 58;
pub fn synthesize_dialectical(
    thesis: &QualiaQuin,
    antithesis: &QualiaQuin,
) -> Option<QualiaQuin>
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

SHACL properties: `qualia:retrieveByActivation`, `qualia:decayMetadata`. These opcodes are compiled by `shacl_compiler.rs` and wired in the `execute_vm_frame` dispatch, but ACT-R activation/decay ops currently **yield to Core 2 GPU Sieve** (return `None`) rather than executing inline — full implementation is a Phase 7 gap. The CogAI `.chk` chunks-and-rules text format is a supported ingest source; see §4 for the `.chk` extension disambiguation.

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

**Sanctuary gate**: If `query_graph` is called without a valid override token, a conduct violation `QualiaQuin` is immediately written to the WAL and signed. Buffer scrubbing via `write_volatile` after each dispatch.

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

Capability profiles are compiled to `.chk` files for efficient loading:

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

1. **`CrdtResolver::resolve_lww`** — Lamport clock tie-breaking. Concurrent mutations resolved by `object` magnitude. Pure, zero-alloc over `&QualiaQuin`.

2. **`CrdtResolver::verify_delegation`** — Temporal expiry + context-bound check on `DelegatedAccess` grants.

3. **`SuspendedTransactionQueue`** — Fixed 32-slot array. Holds flattened WebizenVM frames waiting for M:N signatures. `apply_consensus_token(quin)` wakes suspended execution when `collected_signatures >= threshold`. This is the mechanism for multi-party deontic contract ratification (e.g. Guardianship consent requiring 2-of-3 parties).

### AgreementDID (`webizen.rs`)

`AgreementDID::compile_to_super_quins()` produces 16 Quins in `EnforceBilateralMicroCommons` routing lane (metadata bit pattern `0x4000_0000_0000_0002`). Uses predicates:
- `q42:hasGuardian` — party → agent relationship
- `q42:hasDomainScope` — agreement → domain
- `q42:requiresConsensus` — M-of-N threshold

This encodes agreement *structure*, not norms. The bridge from `AgreementDID` Quins → deontic norm Quins is handled by `compile_n3_rule_to_norm` in `deontic_logic.rs`.

---

## 12. Agency & Identity (`agency.rs`, `identifier.rs`)

- **`compute_scoped_merkle_root(frame, author_did_hash)`** — SHA256 over Quins where `quin.context == author_did`. Zero-alloc via `bytemuck::cast_ref`.
- **`derive_lane_key(pin, salt)`** — currently SHA256-based. Production needs PBKDF2 with ≥ 310,000 iterations (known gap for Sanctuary Mode).
- **`did:q42` pointers** (`identifier.rs`) — `parse_did_q42(b"did:q42:...")` → `u64` with bit 63 always set. FNV-1a over payload then `| (1u64 << 63)`. Routes through MSB dispatch in bytecode VM.

---

## 13. CLI (`crates/qualia-cli/`)

### Command Reference

| Command | Description |
|---------|-------------|
| `bench` / `benchmark` | Full benchmark suite |
| `inspect` | Decode and display Quin fields from a `.q42` file |
| `dump` | Stream-dump raw Quins |
| `daemon` | Start/stop the Warp HTTP daemon (port 4242) |
| `ingest [--profile <file>.chk]` | Ingest RDF/N3/CBOR-LD into a `.q42` vault; profile-bound if `--profile` given |
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
| `profile compile <input.jsonld> <output.chk>` | Compile JSON-LD profile to QCHK binary |
| `profile list` | List known profiles with q_hash IDs |
| `profile inspect <file.chk>` | Decode and display a QCHK profile |
| `webizen init` | Initialize Webizen identity |
| `webizen ingest` | Ingest via Webizen pipeline |
| `webizen validate-gitmark` | Validate git commit mark |
| `webizen publish-ipfs` | Publish to IPFS |
| `webizen seed-webtorrent` | Seed via WebTorrent |
| `webizen dns-frontdoor` | Generate `did:web` + DNS TXT records |

### Profile CLI Workflow

```bash
# Compile a JSON-LD capability profile to QCHK binary
qualia profile compile profile-health.jsonld health.chk

# List known profiles and their q_hash IDs
qualia profile list

# Inspect a compiled profile
qualia profile inspect health.chk

# Bind profile during ingest
qualia ingest --profile health.chk data.ttl output.q42
```

---

## 14. Deployment Targets

| Target | Files | Notes |
|--------|-------|-------|
| **Native CLI** | `crates/qualia-cli/` | Full feature set, 512 MB budget |
| **Desktop — Tauri/React** | `crates/qualia-desktop/` + `crates/qualia-client/` | Active primary desktop. Vite/React frontend, Tauri shell, `qualia://` URI scheme for sandboxed apps |
| **Desktop — Flutter** | `crates/qualia-flutter/` | Alternative desktop target; LLM Hub, Ontology Hub, FRB bridge to CLI subprocess |
| **WASM (Browser)** | `wasm_bridge.rs`, `wasm_edge.rs` | SIMD variant, OPFS auto-cache, `#[wasm_bindgen]` |
| **Edge Native** | `npu_ffi.rs`, `tee_ffi.rs` | NPU sieve dispatch, TEE C-ABI declarations |
| **P2P / Federated** | `wasm_edge.rs`, `nym_adapter.rs` | WebRTC offloading, Nym mixnet, federated node manager |

### Tauri/React Desktop (`crates/qualia-desktop/` + `crates/qualia-client/`)

The primary active desktop target. `qualia-desktop` is the Tauri shell (Rust); `qualia-client` is a Vite/React frontend.

Pages: Dashboard, LLM Hub, Ontology Hub, App Manager, Wallet, Address Book, Credential Manager, Asset Library, Physics Engine, Chat, Settings.

**App Manager** (`/apps` route): Lists apps installed in `{data_dir}/Apps/`, launches them in a webview via the `qualia://localhost/{app-name}/` URI scheme, and issues developer VCs. Apps declare SHACL `required_shapes` in an `app.json` manifest — these map to `target_shapes` in the P2P sync protocol.

⚠ The Tauri command handler (`commands/mod.rs`) currently has `generate_handler![]` empty. `list_installed_apps`, `launch_installed_app`, and `generate_app_credential` exist in `qualia-client-core/src/api.rs` but are not yet registered as `#[tauri::command]`. See §15-F.

### Flutter Desktop (`crates/qualia-flutter/`)

- **LLM Hub** — grid/list view, bulk actions, download state persists across navigation, detail panel
- **Ontology Hub** — browse, import, namespace view
- **FRB bridge** (`rust/src/api.rs`) — `download_llm`, `import_ontology` delegate to `qualia-cli` subprocess

---

## 15. Known Bugs & Correctness Issues

### 15-A Object Field Type-Tag Conflict

`resolver.rs` (authoritative) defines `0b001 << 60` as `xsd:integer` with integer value in bits 0-59.
`logic.rs::extract_float` uses the same bit pattern as an f32 tag with f32 bits in bits 0-31.

**Do not fix unilaterally** — requires alignment across both systems and the ingest layer. New modules must use the `resolver.rs` convention.

### 15-B LTL Opcodes in `logic.rs`

`WebizenOpcode::Always/Eventually/Next` compare a float threshold on a single Quin's object field — they are **not** LTL temporal operators. Existing tests depend on this behavior. Use `evaluate_ltl_trace` from `temporal_ltl.rs` for real temporal reasoning.

### 15-C `prune_defeasible_claims` Uses Heap

`WebizenVM::prune_defeasible_claims` takes `&mut Vec<QualiaQuin>` and uses `HashSet`. This violates the zero-heap mandate. The zero-alloc replacement signature is:

```rust
pub fn partition_defeasible(
    quins: &[QualiaQuin],
    out_hard: &mut [QualiaQuin],
    out_defeasible: &mut [QualiaQuin],
) -> (usize, usize)
```

### 15-D `derive_lane_key` Uses SHA256 (not PBKDF2)

`agency.rs::derive_lane_key` currently uses SHA256. Production Sanctuary Mode requires PBKDF2 with ≥ 310,000 iterations.

### 15-E Three Incompatible `.q42` Write Formats

- `storage.rs::SuperBlockWriter` — raw 40,960-byte `QualiaSuperBlock` structs
- `ingest.rs::streaming_import_rdf` — LZ4-compressed variable blocks with `block_id+len` header
- `archive.rs::Q42Archive` — reader expecting Zstd + 64-byte preamble + jump tables

None of the writers produce what the archive reader expects. `SuperBlockWriter` should become the canonical on-disk format.

### 15-F App Manager Tauri Commands Not Registered

`crates/qualia-desktop/src/commands/mod.rs` contains `tauri::generate_handler![]` — an empty handler list. The functions `list_installed_apps`, `launch_installed_app`, and `generate_app_credential` exist in `qualia-client-core/src/api.rs` but lack `#[tauri::command]` attributes and are not included in the handler. The UI (`AppStore.tsx`) calls `invoke('list_installed_apps')` etc. — these calls currently return errors. Additionally, `launch_installed_app` has no implementation that opens a Tauri webview at `qualia://localhost/{app_name}/index.html`.

---

## 16. Test Status

**195/195 tests passing** as of commit `0e4997a` on `0.0.6-dev`.

Coverage includes: all modality evaluators, MCP fiduciary gate, SHACL constraint compilation, organic chemistry (including isotope distribution), VM safety, SIMD integration, CRDT consensus, and agency signing.
